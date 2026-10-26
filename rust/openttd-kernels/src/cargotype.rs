/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Town cargo acceptance effects from `cargotype.h`.
pub const TAE_WATER: u8 = 4;
pub const TAE_FOOD: u8 = 5;
/// `CargoClass::Passengers` mask.
pub const CC_PASSENGERS: u16 = 1 << 0;
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
