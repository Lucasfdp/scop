//! Window and event handling (the only module that talks to `winit`).

use std::fmt;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

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
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlatformError::EventLoop(e) => write!(f, "event loop error: {e}"),
            PlatformError::Window(e) => write!(f, "could not create window: {e}"),
        }
    }
}

impl std::error::Error for PlatformError {}

impl From<EventLoopError> for PlatformError {
    fn from(e: EventLoopError) -> Self {
        PlatformError::EventLoop(e)
    }
}

/// Opens a window and runs the event loop until the user quits.
pub fn run(config: WindowConfig) -> Result<(), PlatformError> {
    let event_loop = EventLoop::new()?;
    let mut app = App {
        config,
        window: None,
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
    window: Option<Window>,
    error: Option<PlatformError>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        match event_loop.create_window(attributes) {
            Ok(window) => self.window = Some(window),
            Err(e) => {
                self.error = Some(PlatformError::Window(e));
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
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
