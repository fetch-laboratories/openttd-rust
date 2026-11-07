/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `water_type.h` values used by shared water-class services.

/// `WaterClass::Canal`; native compilation asserts the value.
pub const WATER_CLASS_CANAL: WaterClass = WaterClass::Canal;

/// `WaterClass::Sea`.
pub const WATER_CLASS_SEA: WaterClass = WaterClass::Sea;

/// `WaterClass::River`.
pub const WATER_CLASS_RIVER: WaterClass = WaterClass::River;

/// Compatibility import; the original definition belongs to `water_map.h`.
pub use crate::water_map::WaterClass;
