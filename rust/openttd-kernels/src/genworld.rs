/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `genworld.h` constants and native enum carriers.
/// `GenWorldProgress : uint8_t`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Progress(pub u8);
/// `GWP_LANDSCAPE`.
pub const GWP_LANDSCAPE: Progress = Progress(1);
/// `GWP_RIVER`.
pub const GWP_RIVER: Progress = Progress(2);
/// `GWM_HEIGHTMAP`.
pub const GWM_HEIGHTMAP: u8 = 3;
/// `LG_ORIGINAL`.
pub const LG_ORIGINAL: u8 = 0;
/// `LG_TERRAGENESIS`.
pub const LG_TERRAGENESIS: u8 = 1;
/// `CUSTOM_SEA_LEVEL_NUMBER_DIFFICULTY`.
pub const CUSTOM_SEA_LEVEL_NUMBER_DIFFICULTY: u8 = 4;
