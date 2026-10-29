/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `openttd.h` modes and pause reasons.
#![allow(missing_docs)]
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

impl GameMode {
    /// Ordinary play.
    pub const NORMAL: Self = Self(1);
    /// Startup before assets are available.
    pub const BOOTSTRAP: Self = Self(3);
}
impl SwitchMode {
    /// No pending switch.
    pub const NONE: Self = Self(0);
    pub const NEWGAME: Self = Self(1);
    pub const RESTARTGAME: Self = Self(2);
    pub const RELOADGAME: Self = Self(3);
    pub const LOAD_GAME: Self = Self(5);
    pub const SAVE_GAME: Self = Self(7);
    pub const SAVE_HEIGHTMAP: Self = Self(8);
    pub const GENRANDLAND: Self = Self(9);
    pub const LOAD_SCENARIO: Self = Self(10);
    pub const START_HEIGHTMAP: Self = Self(11);
    pub const LOAD_HEIGHTMAP: Self = Self(12);
    pub const RESTART_HEIGHTMAP: Self = Self(13);
    pub const JOIN_GAME: Self = Self(14);
}
/// `PauseMode : uint8_t`, including invalid command inputs.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PauseMode(pub u8);
impl PauseMode {
    pub const NORMAL: Self = Self(0);
    pub const SAVE_LOAD: Self = Self(1);
    pub const JOIN: Self = Self(2);
    pub const ERROR: Self = Self(3);
    pub const ACTIVE_CLIENTS: Self = Self(4);
    pub const GAME_SCRIPT: Self = Self(5);
    pub const LINK_GRAPH: Self = Self(6);
    pub const COMMAND_DURING_PAUSE: Self = Self(7);
    pub const fn bit(self) -> u8 {
        1 << self.0
    }
}
