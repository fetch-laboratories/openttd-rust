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

/// `CALLBACK_FAILED`.
pub const CALLBACK_FAILED: u16 = 0xFFFF;

crate::abi::layouts! {
    fn abi_layout;
    "CallbackID" ["maintenance"] => CallbackID {
    };
}

pub const CBID_VEHICLE_MODIFY_PROPERTY: CallbackID = CallbackID(0x36);
pub const CBID_VEHICLE_BUILD_PROBABILITY: CallbackID = CallbackID(0x162);

pub const CBID_NO_CALLBACK: CallbackID = CallbackID(0x00);
pub const CBID_RANDOM_TRIGGER: CallbackID = CallbackID(0x01);
pub const CBID_TRAIN_ALLOW_WAGON_ATTACH: CallbackID = CallbackID(0x1d);
pub const CBID_VEHICLE_START_STOP_CHECK: CallbackID = CallbackID(0x31);
pub const CBID_VEHICLE_COLOUR_MAPPING: CallbackID = CallbackID(0x2d);
pub const CBID_VEHICLE_SPAWN_VISUAL_EFFECT: CallbackID = CallbackID(0x160);
