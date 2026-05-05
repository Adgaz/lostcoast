use anyhow::{Context, Result};
use egui::ViewportId;
use gfx::overlay::OverlayDraw;
use gfx::AppGfx;
use glam::Vec3;
use lostcoast_core::scene::Scene;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::path::PathBuf;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{CursorGrabMode, Window, WindowId};

mod input;
mod stats;
use input::FlyCam;
use stats::Stats;

fn parse_args() -> (PathBuf, String) {
    let mut args = std::env::args().skip(1);
    let mut scene_path = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--scene" => scene_path = args.next().map(PathBuf::from),
            other => eprintln!("ignoring unknown arg {other}"),
        }
    }
    let path = scene_path.unwrap_or_else(|| PathBuf::from("assets/scenes/clear.json"));
    let label = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("scene")
        .to_string();
    (path, label)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let (scene_path, scene_label) = parse_args();
    let scene = assets::sidecar::load_scene(&scene_path)
        .with_context(|| format!("loading scene {}", scene_path.display()))?;

    let event_loop = EventLoop::new().context("event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(scene, scene_label);
    event_loop.run_app(&mut app).context("run_app")?;
    Ok(())
}

struct App {
    scene: Scene,
    scene_label: String,
    window: Option<Window>,
    gfx: Option<AppGfx>,
    cam: FlyCam,
    start: Instant,
    last_frame: Instant,
    cursor_grabbed: bool,
    stats: Stats,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    overlay_visible: bool,
    last_log_write: Instant,
    log_path: Option<PathBuf>,
}

impl App {
    fn new(scene: Scene, scene_label: String) -> Self {
        let cam = match &scene {
            Scene::Cornell { .. } => {
                FlyCam::new(Vec3::new(0.0, 1.0, 2.5), Vec3::new(0.0, 1.0, -0.5))
            }
            _ => FlyCam::new(Vec3::new(3.0, 3.0, 3.0), Vec3::ZERO),
        };
        let now = Instant::now();
        Self {
            scene,
            scene_label,
            window: None,
            gfx: None,
            cam,
            start: now,
            last_frame: now,
            cursor_grabbed: false,
            stats: Stats::new(120),
            egui_ctx: egui::Context::default(),
            egui_state: None,
            overlay_visible: true,
            last_log_write: now,
            log_path: std::env::var_os("LOSTCOAST_STATS_LOG")
                .map(PathBuf::from)
                .or_else(|| Some(PathBuf::from("/tmp/lostcoast-stats.log"))),
        }
    }

    fn write_stats_line(&self) {
        let Some(path) = self.log_path.as_ref() else {
            return;
        };
        let line = format!(
            "t={:.2}s fps={:.1} avg={:.2}ms min={:.2}ms max={:.2}ms cam=({:.2},{:.2},{:.2}) yaw={:.1} pitch={:.1}\n",
            self.start.elapsed().as_secs_f32(),
            self.stats.fps(),
            self.stats.avg_ms(),
            self.stats.min_ms(),
            self.stats.max_ms(),
            self.cam.pos.x,
            self.cam.pos.y,
            self.cam.pos.z,
            self.cam.yaw.to_degrees(),
            self.cam.pitch.to_degrees(),
        );
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            use std::io::Write;
            let _ = f.write_all(line.as_bytes());
        }
    }

    fn try_grab_cursor(&mut self) {
        if let Some(w) = &self.window {
            let modes = [CursorGrabMode::Locked, CursorGrabMode::Confined];
            for m in modes {
                if w.set_cursor_grab(m).is_ok() {
                    w.set_cursor_visible(false);
                    self.cursor_grabbed = true;
                    return;
                }
            }
        }
    }

    fn release_cursor(&mut self) {
        if let Some(w) = &self.window {
            let _ = w.set_cursor_grab(CursorGrabMode::None);
            w.set_cursor_visible(true);
            self.cursor_grabbed = false;
        }
    }
}

struct UiSnapshot<'a> {
    scene_label: &'a str,
    fps: f32,
    avg_ms: f32,
    min_ms: f32,
    max_ms: f32,
    cam_pos: Vec3,
    yaw: f32,
    pitch: f32,
}

fn build_ui(ctx: &egui::Context, snap: &UiSnapshot) {
    egui::Window::new("stats")
        .anchor(egui::Align2::LEFT_TOP, [12.0, 12.0])
        .resizable(false)
        .collapsible(false)
        .show(ctx, |ui| {
            ui.label(format!("scene: {}", snap.scene_label));
            ui.separator();
            ui.label(format!("fps: {:.1}", snap.fps));
            ui.label(format!("frame: {:.2} ms (avg)", snap.avg_ms));
            ui.label(format!(
                "min: {:.2} ms  max: {:.2} ms",
                snap.min_ms, snap.max_ms
            ));
            ui.separator();
            ui.label(format!(
                "cam: ({:.2}, {:.2}, {:.2})",
                snap.cam_pos.x, snap.cam_pos.y, snap.cam_pos.z
            ));
            ui.label(format!(
                "yaw: {:.1}°  pitch: {:.1}°",
                snap.yaw.to_degrees(),
                snap.pitch.to_degrees()
            ));
            ui.separator();
            ui.label("tab: toggle cursor   `: hide overlay");
            ui.label("click: grab cursor   esc: release / exit");
        });
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("lostcoast")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("create_window failed: {e}");
                event_loop.exit();
                return;
            }
        };
        let size = window.inner_size();
        let display_handle = match window.display_handle() {
            Ok(h) => h.as_raw(),
            Err(e) => {
                eprintln!("display handle: {e}");
                event_loop.exit();
                return;
            }
        };
        let window_handle = match window.window_handle() {
            Ok(h) => h.as_raw(),
            Err(e) => {
                eprintln!("window handle: {e}");
                event_loop.exit();
                return;
            }
        };
        match AppGfx::new(display_handle, window_handle, (size.width, size.height)) {
            Ok(g) => self.gfx = Some(g),
            Err(e) => {
                eprintln!("gfx init: {e:#}");
                event_loop.exit();
                return;
            }
        }
        let state = egui_winit::State::new(
            self.egui_ctx.clone(),
            ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        self.egui_state = Some(state);
        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if !self.cursor_grabbed {
            if let (Some(state), Some(window)) = (self.egui_state.as_mut(), self.window.as_ref()) {
                let response = state.on_window_event(window, &event);
                if response.consumed {
                    return;
                }
            }
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(g) = self.gfx.as_mut() {
                    g.resize(size.width.max(1), size.height.max(1));
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } if !self.cursor_grabbed => {
                self.try_grab_cursor();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => {
                let pressed = state == ElementState::Pressed;
                if let PhysicalKey::Code(code) = physical_key {
                    let name = format!("{code:?}");
                    if pressed && code == winit::keyboard::KeyCode::Escape {
                        if self.cursor_grabbed {
                            self.release_cursor();
                        } else {
                            event_loop.exit();
                        }
                    } else if pressed && code == winit::keyboard::KeyCode::Tab {
                        if self.cursor_grabbed {
                            self.release_cursor();
                        } else {
                            self.try_grab_cursor();
                        }
                    } else if pressed && code == winit::keyboard::KeyCode::Backquote {
                        self.overlay_visible = !self.overlay_visible;
                    } else if self.cursor_grabbed {
                        self.cam.on_key(&name, pressed);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                self.stats.tick(dt);
                self.cam.update(dt);
                if now.duration_since(self.last_log_write).as_secs_f32() >= 1.0 {
                    self.write_stats_line();
                    self.last_log_write = now;
                }

                let snap = UiSnapshot {
                    scene_label: &self.scene_label,
                    fps: self.stats.fps(),
                    avg_ms: self.stats.avg_ms(),
                    min_ms: self.stats.min_ms(),
                    max_ms: self.stats.max_ms(),
                    cam_pos: self.cam.pos,
                    yaw: self.cam.yaw,
                    pitch: self.cam.pitch,
                };
                let visible = self.overlay_visible;
                let overlay_data = match (self.window.as_ref(), self.egui_state.as_mut()) {
                    (Some(window), Some(state)) => {
                        let raw_input = state.take_egui_input(window);
                        let full_output = self.egui_ctx.run(raw_input, |ctx| {
                            if visible {
                                build_ui(ctx, &snap);
                            }
                        });
                        state.handle_platform_output(window, full_output.platform_output);
                        let pixels_per_point = full_output.pixels_per_point;
                        let primitives = self
                            .egui_ctx
                            .tessellate(full_output.shapes, pixels_per_point);
                        Some(OverlayDraw {
                            primitives,
                            textures_delta: full_output.textures_delta,
                            pixels_per_point,
                        })
                    }
                    _ => None,
                };

                if let Some(g) = self.gfx.as_mut() {
                    g.camera_pos = self.cam.pos;
                    g.camera_target = self.cam.target();
                    let t = self.start.elapsed().as_secs_f32();
                    if let Err(e) = g.render(&self.scene, t, overlay_data) {
                        eprintln!("render: {e:#}");
                        event_loop.exit();
                    }
                }
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.cursor_grabbed {
                self.cam.on_mouse(delta.0 as f32, delta.1 as f32);
            }
        }
    }
}
