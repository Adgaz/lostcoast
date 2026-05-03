use anyhow::{Context, Result};
use gfx::AppGfx;
use lostcoast_core::scene::Scene;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::path::PathBuf;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

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
    start: Instant,
}

impl App {
    fn new(scene: Scene) -> Self {
        Self {
            scene,
            window: None,
            gfx: None,
            start: Instant::now(),
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
            WindowEvent::RedrawRequested => {
                if let Some(g) = self.gfx.as_mut() {
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
}
