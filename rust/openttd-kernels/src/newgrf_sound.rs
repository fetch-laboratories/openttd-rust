/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared sound events from `newgrf_sound.h`.

/// `VehicleSoundEvent`: events at which a sound might be played.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VehicleSoundEvent(pub u8);

impl VehicleSoundEvent {
    /// Vehicle starting, i.e. leaving, the station.
    pub const VSE_START: Self = Self(1);
    /// Train entering a tunnel.
    pub const VSE_TUNNEL: Self = Self(2);
    /// Vehicle breaking down.
    pub const VSE_BREAKDOWN: Self = Self(3);
    /// Vehicle running normally.
    pub const VSE_RUNNING: Self = Self(4);
    /// Whenever a plane touches down.
    pub const VSE_TOUCHDOWN: Self = Self(5);
    /// Vehicle visual effect (steam, diesel smoke or electric spark) is shown.
    pub const VSE_VISUAL_EFFECT: Self = Self(6);
    /// Every 16 ticks while the vehicle is running (speed > 0).
    pub const VSE_RUNNING_16: Self = Self(7);
    /// Every 16 ticks while the vehicle is stopped (speed == 0).
    pub const VSE_STOPPED_16: Self = Self(8);
    /// Whenever cargo payment is made for a vehicle.
    pub const VSE_LOAD_UNLOAD: Self = Self(9);
}

crate::abi::layouts! {
    fn abi_layout;
    "VehicleSoundEvent" ["maintenance"] => VehicleSoundEvent {
    };
}

/// `VehicleSoundEvent::VSE_LOAD_UNLOAD`, for `cargo_payment.rs`, which no pass has finished.
pub const VSE_LOAD_UNLOAD: VehicleSoundEvent = VehicleSoundEvent::VSE_LOAD_UNLOAD;
