/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `openttd.h` game modes.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GameMode(pub u8);
impl GameMode {
    /// Main menu mode.
    pub const MENU: Self = Self(0);
}
