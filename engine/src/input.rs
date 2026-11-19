//! The engine's own keys (§ 8 input abstraction): apps say `Key::Space`, never a
//! `winit` key code, so a `winit` update only changes `from_winit` below.

use std::collections::HashSet;
use std::path::PathBuf;

use winit::keyboard::KeyCode;

/// The keys the engine passes on to apps. Physical positions (as on a US
/// keyboard), so WASD stays in the same place on any layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Escape,
    Space,
    Enter,
    Tab,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Plus,
    Minus,
    PageUp,
    PageDown,
    LeftShift,
    RightShift,
}

impl Key {
    /// Translates a `winit` key; `None` for keys the engine doesn't pass on.
    pub(crate) fn from_winit(code: KeyCode) -> Option<Key> {
        use KeyCode as W;
        Some(match code {
            W::Escape => Key::Escape,
            W::Space => Key::Space,
            W::Enter => Key::Enter,
            W::Tab => Key::Tab,
            W::Backspace => Key::Backspace,
            W::ArrowUp => Key::Up,
            W::ArrowDown => Key::Down,
            W::ArrowLeft => Key::Left,
            W::ArrowRight => Key::Right,
            W::KeyA => Key::A,
            W::KeyB => Key::B,
            W::KeyC => Key::C,
            W::KeyD => Key::D,
            W::KeyE => Key::E,
            W::KeyF => Key::F,
            W::KeyG => Key::G,
            W::KeyH => Key::H,
            W::KeyI => Key::I,
            W::KeyJ => Key::J,
            W::KeyK => Key::K,
            W::KeyL => Key::L,
            W::KeyM => Key::M,
            W::KeyN => Key::N,
            W::KeyO => Key::O,
            W::KeyP => Key::P,
            W::KeyQ => Key::Q,
            W::KeyR => Key::R,
            W::KeyS => Key::S,
            W::KeyT => Key::T,
            W::KeyU => Key::U,
            W::KeyV => Key::V,
            W::KeyW => Key::W,
            W::KeyX => Key::X,
            W::KeyY => Key::Y,
            W::KeyZ => Key::Z,
            W::Equal | W::NumpadAdd => Key::Plus, // the key with + on it
            W::Minus | W::NumpadSubtract => Key::Minus,
            W::PageUp => Key::PageUp,
            W::PageDown => Key::PageDown,
            W::ShiftLeft => Key::LeftShift,
            W::ShiftRight => Key::RightShift,
            _ => return None,
        })
    }
}

/// The mouse buttons the engine passes on to apps (§ 20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

impl MouseButton {
    /// Translates a `winit` button; `None` for extra buttons the engine ignores.
    pub(crate) fn from_winit(button: winit::event::MouseButton) -> Option<MouseButton> {
        use winit::event::MouseButton as W;
        match button {
            W::Left => Some(MouseButton::Left),
            W::Right => Some(MouseButton::Right),
            W::Middle => Some(MouseButton::Middle),
            _ => None,
        }
    }
}

/// How many trackpad pixels count as one mouse-wheel notch (§ 18: *scroll delta*).
/// Mice report notches ("lines"); trackpads report pixels, many per gesture.
const PIXELS_PER_LINE: f32 = 40.0;

/// Keyboard and mouse state for one frame.
#[derive(Debug, Default)]
pub struct Input {
    held: HashSet<Key>,
    pressed: HashSet<Key>,
    /// Wheel movement since the last frame, in notches.
    scroll: f32,
    /// Files dropped on the window since the last frame (§ 19).
    dropped: Vec<PathBuf>,
    /// Mouse buttons down right now (§ 20).
    buttons: HashSet<MouseButton>,
    /// Where the cursor was last seen, in window pixels; `None` = outside the window.
    cursor: Option<(f32, f32)>,
    /// How far the cursor moved since the last frame, in pixels.
    motion: (f32, f32),
}

impl Input {
    /// Is `key` down right now? (For continuous actions: keep rotating while held.)
    pub fn held(&self, key: Key) -> bool {
        self.held.contains(&key)
    }

    /// Did `key` go down since the last frame? Once per press, never for auto-repeat.
    /// (For toggles: the texture on/off key in § 15.)
    pub fn pressed(&self, key: Key) -> bool {
        self.pressed.contains(&key)
    }

    /// How far the mouse wheel moved since the last frame, in notches (§ 18).
    /// Positive = away from you (up); 0 when it didn't move. Trackpads are converted
    /// to notches too, so one number works for both.
    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    /// Is mouse `button` down right now? (§ 20: drag = held + `mouse_delta`.)
    pub fn mouse_held(&self, button: MouseButton) -> bool {
        self.buttons.contains(&button)
    }

    /// How far the cursor moved since the last frame, in pixels: (right, down).
    /// The movement, not the position — so a drag turns things by how far you drag,
    /// wherever on the window it starts (§ 20, *mouse delta*).
    pub fn mouse_delta(&self) -> (f32, f32) {
        self.motion
    }

    pub(crate) fn mouse_down(&mut self, button: MouseButton) {
        self.buttons.insert(button);
    }

    pub(crate) fn mouse_up(&mut self, button: MouseButton) {
        self.buttons.remove(&button);
    }

    /// The cursor is now at (x, y). The movement counts from where it was last seen;
    /// the first position after entering the window only sets the start, so the
    /// camera never jumps.
    pub(crate) fn cursor_moved(&mut self, x: f32, y: f32) {
        if let Some((last_x, last_y)) = self.cursor {
            self.motion.0 += x - last_x;
            self.motion.1 += y - last_y;
        }
        self.cursor = Some((x, y));
    }

    /// The cursor left the window: forget where it was, and release the buttons
    /// (their "up" may happen outside, where the window never hears about it).
    pub(crate) fn cursor_left(&mut self) {
        self.cursor = None;
        self.buttons.clear();
    }

    /// Files dragged onto the window since the last frame, in the order they
    /// arrived (§ 19). Usually empty; one entry per file when several are dropped.
    pub fn dropped_files(&self) -> &[PathBuf] {
        &self.dropped
    }

    pub(crate) fn drop_file(&mut self, path: PathBuf) {
        self.dropped.push(path);
    }

    /// A wheel movement in notches (`lines`), as a mouse reports it.
    pub(crate) fn scroll_lines(&mut self, lines: f32) {
        self.scroll += lines; // several events can arrive between two frames
    }

    /// A wheel movement in pixels, as a trackpad reports it.
    pub(crate) fn scroll_pixels(&mut self, pixels: f32) {
        self.scroll_lines(pixels / PIXELS_PER_LINE);
    }

    pub(crate) fn key_down(&mut self, key: Key) {
        // `insert` returns false if the key was already held: that's auto-repeat.
        if self.held.insert(key) {
            self.pressed.insert(key);
        }
    }

    pub(crate) fn key_up(&mut self, key: Key) {
        self.held.remove(&key);
    }

    /// Called after the app's `update`: "pressed", the scroll and dropped files only
    /// last one frame.
    pub(crate) fn end_frame(&mut self) {
        self.pressed.clear();
        self.scroll = 0.0;
        self.dropped.clear();
        self.motion = (0.0, 0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_counts_once_even_with_auto_repeat() {
        let mut input = Input::default();
        input.key_down(Key::Space);
        input.key_down(Key::Space); // OS auto-repeat
        assert!(input.pressed(Key::Space) && input.held(Key::Space));
        input.end_frame();
        input.key_down(Key::Space); // still held, repeating
        assert!(!input.pressed(Key::Space) && input.held(Key::Space));
        input.key_up(Key::Space);
        assert!(!input.held(Key::Space));
    }

    #[test]
    fn mouse_delta_is_movement_and_never_jumps() {
        let mut input = Input::default();
        input.cursor_moved(100.0, 100.0); // first sight: only sets the start
        assert_eq!(input.mouse_delta(), (0.0, 0.0));
        input.cursor_moved(110.0, 95.0);
        input.cursor_moved(115.0, 95.0); // several moves in one frame add up
        assert_eq!(input.mouse_delta(), (15.0, -5.0));
        input.end_frame();
        assert_eq!(input.mouse_delta(), (0.0, 0.0)); // one frame only
        // Leaves at one side, comes back at the other: no 500-pixel jump.
        input.mouse_down(MouseButton::Left);
        input.cursor_left();
        assert!(!input.mouse_held(MouseButton::Left)); // released when it left
        input.cursor_moved(615.0, 95.0);
        assert_eq!(input.mouse_delta(), (0.0, 0.0));
    }

    #[test]
    fn dropped_files_last_one_frame() {
        let mut input = Input::default();
        input.drop_file(PathBuf::from("a.obj"));
        input.drop_file(PathBuf::from("b.obj"));
        assert_eq!(input.dropped_files().len(), 2);
        input.end_frame();
        assert!(input.dropped_files().is_empty());
    }

    #[test]
    fn scroll_adds_up_within_a_frame_then_resets() {
        let mut input = Input::default();
        input.scroll_lines(1.0);
        input.scroll_lines(1.0);
        input.scroll_pixels(-PIXELS_PER_LINE); // a trackpad: one notch's worth back
        assert_eq!(input.scroll(), 1.0);
        input.end_frame();
        assert_eq!(input.scroll(), 0.0);
    }
}
