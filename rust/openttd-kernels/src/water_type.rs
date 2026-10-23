/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `water_type.h` values used by shared water-class services.

use crate::services::WaterClass;

/// `WaterClass::Canal`; native compilation asserts the value.
pub const WATER_CLASS_CANAL: WaterClass = WaterClass(1);

/// `WaterClass::Sea`.
pub const WATER_CLASS_SEA: WaterClass = WaterClass(0);
