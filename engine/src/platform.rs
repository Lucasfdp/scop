//! Window and event handling (the only module that talks to `winit`).
//! Also decides *when* the GPU is set up: right after the window exists.

use std::ffi::CString;
use std::fmt;

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
/// and runs the event loop until the user quits.
pub fn run(config: WindowConfig, on_ready: fn(&GpuContext)) -> Result<(), PlatformError> {
    let event_loop = EventLoop::new()?;
    let mut app = App {
        config,
        on_ready,
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
    graphics: Option<Graphics>,
    error: Option<PlatformError>,
}

/// The window and the GPU context made from it, kept together so the order they
/// are dropped in is fixed.
#[expect(
    dead_code,
    reason = "owned to control drop order; read from § 5 on, when frames are drawn"
)]
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
        self.graphics = Some(Graphics { gpu, window });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) if size.width == 0 || size.height == 0 => {
                // Minimised: a 0×0 swapchain is invalid. Once frames are drawn (§ 5),
                // skip drawing and rebuilding until the size is non-zero again.
                println!("minimised: rendering paused");
            }
            WindowEvent::Resized(size) => println!("resized: {}x{}", size.width, size.height),
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
