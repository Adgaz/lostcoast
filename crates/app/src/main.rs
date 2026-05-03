use anyhow::{Context, Result};
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
use input::FlyCam;

fn parse_args() -> PathBuf {
    let mut args = std::env::args().skip(1);
    let mut scene_path = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--scene" => scene_path = args.next().map(PathBuf::from),
            other => eprintln!("ignoring unknown arg {other}"),
        }
    }
    scene_path.unwrap_or_else(|| PathBuf::from("assets/scenes/clear.json"))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let scene_path = parse_args();
    let scene = assets::sidecar::load_scene(&scene_path)
        .with_context(|| format!("loading scene {}", scene_path.display()))?;

    let event_loop = EventLoop::new().context("event loop")?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(scene);
    event_loop.run_app(&mut app).context("run_app")?;
    Ok(())
}

struct App {
    scene: Scene,
    window: Option<Window>,
    gfx: Option<AppGfx>,
    cam: FlyCam,
    start: Instant,
    last_frame: Instant,
    cursor_grabbed: bool,
}

impl App {
    fn new(scene: Scene) -> Self {
        let cam = FlyCam::new(Vec3::new(3.0, 3.0, 3.0), Vec3::ZERO);
        let now = Instant::now();
        Self {
            scene,
            window: None,
            gfx: None,
            cam,
            start: now,
            last_frame: now,
            cursor_grabbed: false,
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
        window.request_redraw();
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
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
                    } else {
                        self.cam.on_key(&name, pressed);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_frame).as_secs_f32();
                self.last_frame = now;
                self.cam.update(dt);
                if let Some(g) = self.gfx.as_mut() {
                    g.camera_pos = self.cam.pos;
                    g.camera_target = self.cam.target();
                    let t = self.start.elapsed().as_secs_f32();
                    if let Err(e) = g.render(&self.scene, t) {
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
