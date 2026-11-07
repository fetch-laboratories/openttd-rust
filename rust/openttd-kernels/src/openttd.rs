/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `openttd.h` modes and pause reasons.
#![allow(missing_docs)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GameMode(pub u8);
impl GameMode {
    /// Main menu mode.
    pub const MENU: Self = Self(0);
    /// Scenario editor mode.
    pub const EDITOR: Self = Self(2);
}

/// Native mode-switch request, including its full underlying byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PauseMode(pub u8);
impl PauseMode {
    pub const NORMAL: Self = Self::Normal;
    pub const SAVE_LOAD: Self = Self::SaveLoad;
    pub const JOIN: Self = Self::Join;
    pub const ERROR: Self = Self::Error;
    pub const ACTIVE_CLIENTS: Self = Self::ActiveClients;
    pub const GAME_SCRIPT: Self = Self::GameScript;
    pub const LINK_GRAPH: Self = Self::LinkGraph;
    pub const COMMAND_DURING_PAUSE: Self = Self::CommandDuringPause;
    #[must_use]
    pub const fn bit(self) -> u8 {
        1 << self.0
    }
}

#[allow(non_upper_case_globals)]
impl PauseMode {
    /// A game normally paused
    pub const Normal: Self = Self(0);
    /// A game paused for saving/loading
    pub const SaveLoad: Self = Self(1);
    /// A game paused for '`pause_on_join`'
    pub const Join: Self = Self(2);
    /// A game paused because a (critical) error
    pub const Error: Self = Self(3);
    /// A game paused for '`min_active_clients`'
    pub const ActiveClients: Self = Self(4);
    /// A game paused by a game script
    pub const GameScript: Self = Self(5);
    /// A game paused due to the link graph schedule lagging
    pub const LinkGraph: Self = Self(6);
    /// A game paused, and a command executed during the pause; resets on autosave
    pub const CommandDuringPause: Self = Self(7);
}

/// Native `PauseModes`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PauseModes(pub u8);

impl PauseModes {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: PauseMode) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

impl PauseModes {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: PauseMode) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(PauseModes);
