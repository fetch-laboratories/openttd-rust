/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Town cargo acceptance effects from `cargotype.h`.
pub const TAE_WATER: u8 = 4;
pub const TAE_FOOD: u8 = 5;
/// `CargoClass::Mail` mask.
pub const CC_MAIL: u16 = 1 << 1;
/// `CargoClass::Express` mask.
pub const CC_EXPRESS: u16 = 1 << 2;
/// `CargoClass::Armoured` mask.
pub const CC_ARMOURED: u16 = 1 << 3;
/// `CargoClass::Bulk` mask.
pub const CC_BULK: u16 = 1 << 4;
/// `CargoClass::PieceGoods` mask.
pub const CC_PIECEGOODS: u16 = 1 << 5;
/// `CargoClass::Liquid` mask.
pub const CC_LIQUID: u16 = 1 << 6;
/// `CargoClass::Refrigerated` mask.
pub const CC_REFRIGERATED: u16 = 1 << 7;

/// Invalid specification bit number.
pub const INVALID_CARGO_BITNUM: u8 = 255;
/// Town production effect from `cargotype.h`.
pub const TPE_NONE: u8 = 0;
/// Town production effect from `cargotype.h`.
pub const TPE_PASSENGERS: u8 = 1;
/// Town production effect from `cargotype.h`.
pub const TPE_MAIL: u8 = 2;
/// Town production effect from `cargotype.h`.
pub const NUM_TPE: u8 = 3;
/// Town production effect from `cargotype.h`.
pub const INVALID_TPE: u8 = 4;

/// Original `CargoSpec::IsValid`.
pub const fn is_cargo_spec_valid(bitnum: u8) -> bool {
    bitnum != INVALID_CARGO_BITNUM
}

/// Default `CargoSpec::Iterate()` observes each slot when advancing, without retaining a world borrow.
pub(crate) fn iterate<T>(
    mut load: impl FnMut(u8) -> T,
    mut valid: impl FnMut(&T) -> bool,
) -> impl Iterator<Item = (u8, T)> {
    (0..crate::cargo_type::NUM_CARGO)
        .map(move |cargo| (cargo, load(cargo)))
        .filter(move |(_, spec)| valid(spec))
}

/// `CargoClass::Passengers`.
pub const PASSENGERS: u16 = 1 << 0;
/// `IsCargoInClass` with classes read from the canonical native cargo spec.
pub const fn is_cargo_in_class(classes: u16, requested: u16) -> bool {
    classes & requested != 0
}
