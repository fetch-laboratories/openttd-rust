/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `transport_type.h` values.
/// Railway transport.
pub const TRANSPORT_RAIL: u8 = TransportType::TRANSPORT_RAIL.0;
/// Road transport.
pub const TRANSPORT_ROAD: u8 = TransportType::TRANSPORT_ROAD.0;
/// Water transport.
pub const TRANSPORT_WATER: u8 = TransportType::TRANSPORT_WATER.0;

/// Native `TransportType` with its unchanged uint8 representation.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransportType(pub u8);

impl TransportType {
    /// Begin of the iterator.
    pub const TRANSPORT_BEGIN: Self = Self(0);
    /// Transport by train
    pub const TRANSPORT_RAIL: Self = Self(0);
    /// Transport by road vehicle
    pub const TRANSPORT_ROAD: Self = Self(1);
    /// Transport over water
    pub const TRANSPORT_WATER: Self = Self(2);
    /// Transport through air
    pub const TRANSPORT_AIR: Self = Self(3);
    /// End of iterations.
    pub const TRANSPORT_END: Self = Self(4);
    /// Sentinel for invalid transport types.
    pub const INVALID_TRANSPORT: Self = Self(255);
}
