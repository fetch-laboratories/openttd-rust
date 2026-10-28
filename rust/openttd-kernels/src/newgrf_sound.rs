/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared sound events from `newgrf_sound.h`.

/// Native `VehicleSoundEvent : uint8_t`, preserving unnamed values too.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VehicleSoundEvent(pub u8);

pub const VSE_TUNNEL: VehicleSoundEvent = VehicleSoundEvent(2);
pub const VSE_START: VehicleSoundEvent = VehicleSoundEvent(1);
pub const VSE_BREAKDOWN: VehicleSoundEvent = VehicleSoundEvent(3);
pub const VSE_TOUCHDOWN: VehicleSoundEvent = VehicleSoundEvent(5);
pub const VSE_RUNNING: VehicleSoundEvent = VehicleSoundEvent(4);
pub const VSE_RUNNING_16: VehicleSoundEvent = VehicleSoundEvent(7);
pub const VSE_STOPPED_16: VehicleSoundEvent = VehicleSoundEvent(8);

crate::abi::layouts! {
    fn abi_layout;
    "VehicleSoundEvent" ["maintenance"] => VehicleSoundEvent {
    };
}

/// `VSE_VISUAL_EFFECT`.
pub const VSE_VISUAL_EFFECT: VehicleSoundEvent = VehicleSoundEvent(6);

/// `VSE_LOAD_UNLOAD`.
pub const VSE_LOAD_UNLOAD: VehicleSoundEvent = VehicleSoundEvent(9);
