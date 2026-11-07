//! Window and event handling (the only module that talks to `winit`).
//! Also decides *when* the GPU is set up: right after the window exists.

use std::ffi::CString;
use std::fmt;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::gpu::{GpuContext, GpuError};

/// What the app asks for when opening a window.
pub struct WindowConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

/// Everything that can go wrong while opening or running the window.
#[derive(Debug)]
pub enum PlatformError {
    EventLoop(EventLoopError),
    Window(OsError),
    Gpu(GpuError),
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::EventLoop(e) => write!(f, "event loop error: {e}"),
            PlatformError::Window(e) => write!(f, "could not create window: {e}"),
            PlatformError::Gpu(e) => write!(f, "GPU setup failed: {e}"),
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<EventLoopError> for PlatformError {
    fn from(e: EventLoopError) -> Self {
        PlatformError::EventLoop(e)
    }
}

impl From<GpuError> for PlatformError {
    fn from(e: GpuError) -> Self {
        PlatformError::Gpu(e)
    }
}

/// Opens a window, sets up the GPU for it, calls `on_ready` once both exist,
/// then draws frames until the user quits. Each frame is cleared to
/// `clear_colour(seconds since start)`.
pub fn run(
    config: WindowConfig,
    on_ready: fn(&GpuContext),
    clear_colour: fn(f32) -> [f32; 4],
) -> Result<(), PlatformError> {
    let event_loop = EventLoop::new()?;
    let mut app = App {
        config,
        on_ready,
        clear_colour,
        start: Instant::now(),
        minimised: false,
        graphics: None,
        error: None,
    };
    event_loop.run_app(&mut app)?;
    match app.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// State the event loop calls back into.
struct App {
    config: WindowConfig,
    on_ready: fn(&GpuContext),
    clear_colour: fn(f32) -> [f32; 4],
    start: Instant,
    minimised: bool,
    graphics: Option<Graphics>,
    error: Option<PlatformError>,
}

/// The window and the GPU context made from it, kept together so the order they
/// are dropped in is fixed.
struct Graphics {
    // Rust drops fields top to bottom: the GPU context (and its surface) goes
    // first, *then* the window the surface points at. Don't reorder these.
    gpu: GpuContext,
    window: Window,
}

impl App {
    /// Records the error and stops the event loop; `run` returns it.
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: PlatformError) {
        self.error = Some(error);
        event_loop.exit();
    }

    /// Draws one frame, then asks for the next one. While minimised it does neither,
    /// so the loop pauses until a resize restarts it.
    fn draw(&mut self, event_loop: &ActiveEventLoop) {
        if self.minimised {
            return;
        }
        let Some(graphics) = &mut self.graphics else {
            return;
        };
        let colour = (self.clear_colour)(self.start.elapsed().as_secs_f32());
        match graphics.gpu.draw_frame(colour) {
            // OutOfDate/Suboptimal: the swapchain no longer matches the window.
            // Rebuilding it is § 16; until then frames may be skipped after a resize.
            Ok(_status) => graphics.window.request_redraw(),
            Err(e) => self.fail(event_loop, e.into()),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => window,
            Err(e) => return self.fail(event_loop, PlatformError::Window(e)),
        };

        // Vulkan setup needs the window, so it happens here, not before the loop.
        // `inner_size` is in physical pixels (2× the logical size on Retina).
        let size = window.inner_size();
        let app_name = CString::new(self.config.title.as_str()).unwrap_or_default();
        let gpu = match GpuContext::new(&app_name, &window, (size.width, size.height)) {
            Ok(gpu) => gpu,
            Err(e) => return self.fail(event_loop, e.into()),
        };
        (self.on_ready)(&gpu);
        window.request_redraw(); // start the frame loop
        self.graphics = Some(Graphics { gpu, window });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.draw(event_loop),
            WindowEvent::Resized(size) if size.width == 0 || size.height == 0 => {
                // Minimised (on Linux/Windows): a 0×0 swapchain is invalid, so stop
                // drawing until the window has a real size again.
                self.minimised = true;
                println!("minimised: rendering paused");
            }
            WindowEvent::Resized(size) => {
                println!("resized: {}x{}", size.width, size.height);
                if self.minimised {
                    self.minimised = false;
                    if let Some(graphics) = &self.graphics {
                        graphics.window.request_redraw(); // restart the frame loop
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                if logical_key == Key::Named(NamedKey::Escape) {
                    event_loop.exit();
                } else {
                    println!("key: {logical_key:?}");
                }
            }
            _ => {}
        }
    }
}
