use std::{sync::atomic::AtomicBool, fmt::Display, sync::atomic::Ordering};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

pub(crate) struct KeyState(i32, AtomicBool);

impl Clone for KeyState {
    fn clone(&self) -> Self {
        KeyState(self.0, AtomicBool::new(self.1.load(Ordering::Relaxed)))
    }
}

impl KeyState {
    pub(crate) fn new(vkey: i32) -> Self {
        KeyState(vkey, AtomicBool::new(unsafe { GetAsyncKeyState(vkey) < 0 }))
    }

    pub(crate) fn keyup(&self) -> bool {
        let (prev_state, state) = self.update();
        prev_state && !state
    }

    pub(crate) fn keydown(&self) -> bool {
        let (prev_state, state) = self.update();
        !prev_state && state
    }

    pub(crate) fn is_key_down(&self) -> bool {
        unsafe { GetAsyncKeyState(self.0) < 0 }
    }

    fn update(&self) -> (bool, bool) {
        let state = self.is_key_down();
        let prev_state = self.1.swap(state, Ordering::SeqCst);
        (prev_state, state)
    }
}
