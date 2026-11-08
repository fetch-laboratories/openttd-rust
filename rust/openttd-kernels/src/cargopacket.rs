/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `cargopacket.h` constants shared by simulation owners.

/// `VehicleCargoList::MTA_LOAD`: temporary raw adapter for `vehicle/visits.rs`.
pub const MTA_LOAD: u8 = MoveToAction::MTA_LOAD.0;

/// Native `CargoPacketID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoPacketID(pub u32);

impl CargoPacketID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(0x00FF_F000);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(0x00FF_FFFF);
}

/// Native `CargoList::MoveToAction`, shared with `VehicleCargoList`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveToAction(pub u8);
impl MoveToAction {
    /// First movement action.
    pub const MTA_BEGIN: Self = Self(0);
    /// Transfer the cargo to the station.
    pub const MTA_TRANSFER: Self = Self(0);
    /// Deliver the cargo to some town or industry.
    pub const MTA_DELIVER: Self = Self(1);
    /// Keep the cargo in the vehicle.
    pub const MTA_KEEP: Self = Self(2);
    /// Load the cargo from the station.
    pub const MTA_LOAD: Self = Self(3);
    /// End of the movement action range.
    pub const MTA_END: Self = Self(4);
    /// Number of movement actions.
    pub const NUM_MOVE_TO_ACTION: Self = Self::MTA_END;
}

/// The existing cargo packet record, with vocabulary from `cargopacket.h`.
pub(crate) use crate::cargo_storage::Packet as CargoPacket;
impl CargoPacket {
    /// Maximum number of items in one cargo packet.
    pub const MAX_COUNT: u16 = u16::MAX;
}
