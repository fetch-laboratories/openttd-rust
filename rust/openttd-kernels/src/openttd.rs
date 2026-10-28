/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `openttd.h` game modes.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GameMode(pub u8);
impl GameMode {
    /// Main menu mode.
    pub const MENU: Self = Self(0);
    /// Scenario editor mode.
    pub const EDITOR: Self = Self(2);
}

/// Native mode-switch request, including its full underlying byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SwitchMode(pub u8);
impl SwitchMode {
    /// `SM_EDITOR`.
    pub const EDITOR: Self = Self(4);
    /// `SM_MENU`.
    pub const MENU: Self = Self(6);
}
