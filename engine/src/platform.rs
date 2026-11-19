//! Window and event handling (the only module that talks to `winit`), and the
//! engine's main loop: it owns the loop and calls the app (§ 8).

use std::ffi::CString;
use std::fmt;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::error::{EventLoopError, OsError};
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::app::{App, Flow, Frame, Gfx};
use crate::gpu::{FrameStatus, GpuContext, GpuError};
use crate::input::{Input, Key};

/// What the app asks for when opening a window.
pub struct WindowConfig {
    pub title: String,
    /// Size in logical points (the physical size is 2× on a Retina screen).
    pub width: u32,
    pub height: u32,
}

/// Why `run` stopped with an error. Opaque on purpose: apps print it, they don't
/// need (or get) the `winit` and Vulkan types inside.
#[derive(Debug)]
pub struct RunError(ErrorKind);

#[derive(Debug)]
enum ErrorKind {
    EventLoop(EventLoopError),
    Window(OsError),
    Gpu(GpuError),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ErrorKind::EventLoop(e) => write!(f, "event loop error: {e}"),
            ErrorKind::Window(e) => write!(f, "could not create window: {e}"),
            ErrorKind::Gpu(e) => write!(f, "GPU error: {e}"),
        }
    }
}

impl std::error::Error for RunError {}

impl From<EventLoopError> for RunError {
    fn from(e: EventLoopError) -> Self {
        RunError(ErrorKind::EventLoop(e))
    }
}

impl From<OsError> for RunError {
    fn from(e: OsError) -> Self {
        RunError(ErrorKind::Window(e))
    }
}

impl From<GpuError> for RunError {
    fn from(e: GpuError) -> Self {
        RunError(ErrorKind::Gpu(e))
    }
}

/// Opens a window, sets up the GPU, then runs `app` until it returns `Flow::Exit`
/// or the window is closed: `setup` once, then `update` and `draw` every frame.
pub fn run<A: App>(config: WindowConfig, app: A) -> Result<(), RunError> {
    let event_loop = EventLoop::new()?;
    let mut runner = Runner {
        config,
        app,
        input: Input::default(),
        last_frame: Instant::now(),
        minimised: false,
        needs_rebuild: false,
        graphics: None,
        error: None,
    };
    event_loop.run_app(&mut runner)?;
    match runner.error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// The engine's side of the loop: `winit` calls this, and this calls the app.
struct Runner<A: App> {
    config: WindowConfig,
    app: A,
    input: Input,
    last_frame: Instant,
    minimised: bool,
    /// The swapchain no longer matches the window: rebuild before the next frame (§ 16).
    needs_rebuild: bool,
    graphics: Option<Graphics>,
    error: Option<RunError>,
}

/// The window and the GPU context made from it, kept together so the order they
/// are dropped in is fixed.
struct Graphics {
    // Rust drops fields top to bottom: the GPU context (and its surface) goes
    // first, *then* the window the surface points at. Don't reorder these.
    gpu: GpuContext,
    window: Window,
}

impl<A: App> Runner<A> {
    /// Records the error and stops the event loop; `run` returns it.
    fn fail(&mut self, event_loop: &ActiveEventLoop, error: RunError) {
        self.error = Some(error);
        event_loop.exit();
    }

    /// One frame: let the app update, collect its draw list, draw it, ask for the
    /// next frame. While minimised it does nothing, so the loop pauses.
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        if self.minimised {
            return;
        }
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        let flow = self.app.update(&self.input, dt);
        self.input.end_frame();
        if flow == Flow::Exit {
            event_loop.exit();
            return;
        }

        let Some(graphics) = &mut self.graphics else {
            return;
        };

        // § 16: the window changed size (or the last frame said "out of date"):
        // rebuild the swapchain and depth image at the current size first.
        if self.needs_rebuild {
            let size = graphics.window.inner_size();
            if size.width == 0 || size.height == 0 {
                return; // minimised: nothing to build for; the Resized event restarts us
            }
            if let Err(e) = graphics.gpu.rebuild_for_window((size.width, size.height)) {
                return self.fail(event_loop, e.into());
            }
            self.needs_rebuild = false;
        }

        // The aspect of the image this frame will be drawn into, fresh every frame.
        let mut frame = Frame::new(graphics.gpu.aspect());
        self.app.draw(&mut frame);

        match graphics.gpu.draw_frame(&frame) {
            Ok(status) => {
                // OutOfDate (frame skipped) or Suboptimal (shown, but mismatched):
                // the swapchain no longer fits the window — rebuild before the next one.
                if status != FrameStatus::Presented {
                    self.needs_rebuild = true;
                }
                graphics.window.request_redraw();
            }
            Err(e) => self.fail(event_loop, e.into()),
        }
    }
}

impl<A: App> ApplicationHandler for Runner<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.config.title)
            .with_inner_size(LogicalSize::new(self.config.width, self.config.height));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => window,
            Err(e) => return self.fail(event_loop, e.into()),
        };

        // Vulkan setup needs the window, so it happens here, not before the loop.
        // `inner_size` is in physical pixels (2× the logical size on Retina).
        let size = window.inner_size();
        let app_name = CString::new(self.config.title.as_str()).unwrap_or_default();
        let mut gpu = match GpuContext::new(&app_name, &window, (size.width, size.height)) {
            Ok(gpu) => gpu,
            Err(e) => return self.fail(event_loop, e.into()),
        };

        // The app's turn: create its pipelines and meshes.
        if let Err(e) = self.app.setup(&mut Gfx { gpu: &mut gpu }) {
            return self.fail(event_loop, e);
        }

        self.last_frame = Instant::now();
        window.request_redraw(); // start the frame loop
        self.graphics = Some(Graphics { gpu, window });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.frame(event_loop),
            WindowEvent::Resized(size) => {
                // Minimised (on Linux/Windows) = 0×0, and a 0×0 swapchain is invalid:
                // stop drawing until the window has a real size again.
                let was_minimised = self.minimised;
                self.minimised = size.width == 0 || size.height == 0;
                // Any real size change: the swapchain must follow (§ 16).
                if !self.minimised {
                    self.needs_rebuild = true;
                }
                let restored = was_minimised && !self.minimised;
                if let Some(graphics) = self.graphics.as_ref().filter(|_| restored) {
                    graphics.window.request_redraw(); // restart the frame loop
                }
            }
            // § 18: the mouse wheel (or a two-finger trackpad swipe). Only the
            // vertical part is used.
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(_, lines) => self.input.scroll_lines(lines),
                MouseScrollDelta::PixelDelta(pixels) => self.input.scroll_pixels(pixels.y as f32),
            },
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => {
                if let Some(key) = Key::from_winit(code) {
                    match state {
                        ElementState::Pressed => self.input.key_down(key),
                        ElementState::Released => self.input.key_up(key),
                    }
                }
            }
            _ => {}
        }
    }
}
