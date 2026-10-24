/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared constants and the native callback ID domain from `newgrf_callbacks.h`.

/// Native `CallbackID : uint16_t`, preserving unnamed values too.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CallbackID(pub u16);

pub const CBID_VEHICLE_LENGTH: CallbackID = CallbackID(0x11);
pub const CBID_VEHICLE_32DAY_CALLBACK: CallbackID = CallbackID(0x32);

pub const CBID_INDTILE_ANIMATION_TRIGGER: CallbackID = CallbackID(0x25);
pub const CBID_STATION_ANIMATION_TRIGGER: CallbackID = CallbackID(0x140);
pub const CBID_AIRPTILE_ANIMATION_TRIGGER: CallbackID = CallbackID(0x152);
pub const CBID_OBJECT_ANIMATION_TRIGGER: CallbackID = CallbackID(0x159);

pub const CBID_HOUSE_ANIMATION_NEXT_FRAME: CallbackID = CallbackID(0x1A);
pub const CBID_HOUSE_ANIMATION_SPEED: CallbackID = CallbackID(0x20);
pub const CBID_INDTILE_ANIMATION_NEXT_FRAME: CallbackID = CallbackID(0x26);
pub const CBID_INDTILE_ANIMATION_SPEED: CallbackID = CallbackID(0x27);
pub const CBID_STATION_ANIMATION_NEXT_FRAME: CallbackID = CallbackID(0x141);
pub const CBID_STATION_ANIMATION_SPEED: CallbackID = CallbackID(0x142);
pub const CBID_AIRPTILE_ANIMATION_NEXT_FRAME: CallbackID = CallbackID(0x153);
pub const CBID_AIRPTILE_ANIMATION_SPEED: CallbackID = CallbackID(0x154);
pub const CBID_OBJECT_ANIMATION_NEXT_FRAME: CallbackID = CallbackID(0x158);
pub const CBID_OBJECT_ANIMATION_SPEED: CallbackID = CallbackID(0x15A);

/// `CALLBACK_FAILED`.
pub const CALLBACK_FAILED: u16 = 0xFFFF;

crate::abi::layouts! {
    fn abi_layout;
    "CallbackID" ["maintenance"] => CallbackID {
    };
}

/// `CBID_STATION_LAND_SLOPE_CHECK`.
pub const CBID_STATION_LAND_SLOPE_CHECK: CallbackID = CallbackID(0x149);
