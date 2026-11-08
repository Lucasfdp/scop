//! The engine's own keys (§ 8 input abstraction): apps say `Key::Space`, never a
//! `winit` key code, so a `winit` update only changes `from_winit` below.

use std::collections::HashSet;

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

/// Keyboard state for one frame.
#[derive(Debug, Default)]
pub struct Input {
    held: HashSet<Key>,
    pressed: HashSet<Key>,
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

    pub(crate) fn key_down(&mut self, key: Key) {
        // `insert` returns false if the key was already held: that's auto-repeat.
        if self.held.insert(key) {
            self.pressed.insert(key);
        }
    }

    pub(crate) fn key_up(&mut self, key: Key) {
        self.held.remove(&key);
    }

    /// Called after the app's `update`: "pressed" only lasts one frame.
    pub(crate) fn end_frame(&mut self) {
        self.pressed.clear();
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
}
