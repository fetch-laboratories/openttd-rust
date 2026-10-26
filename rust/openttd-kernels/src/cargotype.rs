/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Town cargo acceptance effects from `cargotype.h`.
pub const TAE_WATER: u8 = 4;
pub const TAE_FOOD: u8 = 5;

/// `CargoClass::Passengers`.
pub const PASSENGERS: u16 = 1 << 0;
/// `IsCargoInClass` with classes read from the canonical native cargo spec.
pub const fn is_cargo_in_class(classes: u16, requested: u16) -> bool {
    classes & requested != 0
}
