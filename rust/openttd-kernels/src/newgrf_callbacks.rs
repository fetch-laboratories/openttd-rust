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

pub const CBID_INDUSTRY_PROBABILITY: u16 = 0x22;
pub const CBID_INDUSTRY_LOCATION: u16 = 0x28;
pub const CBID_INDUSTRY_REFUSE_CARGO: u16 = 0x3d;
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

pub const CBID_HOUSE_ALLOW_CONSTRUCTION: CallbackID = CallbackID(0x17);
pub const CBID_HOUSE_DRAW_FOUNDATIONS: CallbackID = CallbackID(0x14E);

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

pub const CBID_VEHICLE_REFIT_COST: CallbackID = CallbackID(0x15e);
pub const CBID_VEHICLE_LOAD_AMOUNT: CallbackID = CallbackID(0x12);

/// Masks from `newgrf_callbacks.h::IndustryCallbackMask`.
pub const INDUSTRY_PROBABILITY: u16 = 1 << 0;
pub const INDUSTRY_PRODUCTION_CARGO_ARRIVAL: u16 = 1 << 1;
pub const INDUSTRY_PRODUCTION_256_TICKS: u16 = 1 << 2;
pub const INDUSTRY_REFUSE_CARGO: u16 = 1 << 10;

/// `CBID_STATION_LAND_SLOPE_CHECK`.
pub const CBID_STATION_LAND_SLOPE_CHECK: CallbackID = CallbackID(0x149);
/// `CBID_STATION_AVAILABILITY`.
pub const CBID_STATION_AVAILABILITY: u16 = 0x13;

pub const CBID_GENERIC_AI_PURCHASE_SELECTION: CallbackID = CallbackID(0x18);
pub const CBID_SOUNDS_AMBIENT_EFFECT: CallbackID = CallbackID(0x144);
/// Object land slope check callback.
pub const CBID_OBJECT_LAND_SLOPE_CHECK: CallbackID = CallbackID(0x157);
/// Object colour callback.
pub const CBID_OBJECT_COLOUR: CallbackID = CallbackID(0x15b);

/// Engine loading custom refit callback.
pub const CBID_VEHICLE_CUSTOM_REFIT: CallbackID = CallbackID(0x163);
/// `CBID_VEHICLE_VISUAL_EFFECT`.
pub const CBID_VEHICLE_VISUAL_EFFECT: CallbackID = CallbackID(0x10);
/// Cargo payment callback from `newgrf_callbacks.h`.
pub const CBID_CARGO_PROFIT_CALC: CallbackID = CallbackID(0x39);

/// Station cargo rating callback from `newgrf_callbacks.h`.
pub const CBID_CARGO_STATION_RATING_CALC: CallbackID = CallbackID(0x145);

pub const CBID_VEHICLE_REFIT_CAPACITY: CallbackID = CallbackID(0x15);
pub const CBID_VEHICLE_ARTIC_ENGINE: CallbackID = CallbackID(0x16);
pub const VEHICLE_REFIT_CAPACITY: u16 = 1 << 3;
pub const VEHICLE_ARTIC_ENGINE: u16 = 1 << 4;
pub const VEHICLE_CARGO_SUFFIX: u16 = 1 << 5;
