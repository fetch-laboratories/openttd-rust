/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */


//! `ScriptStationList_Cargo` vocabulary from `script_stationlist.hpp`.

/// `ScriptStationList_Cargo::CargoSelector`: criteria of selecting and
/// grouping cargo at a station.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CargoSelector(pub u8);
impl CargoSelector {
    /// Group by origin station.
    pub const CS_BY_FROM: Self = Self(0);
    /// Select by next hop and group by origin station.
    pub const CS_VIA_BY_FROM: Self = Self(1);
    /// Group by next hop.
    pub const CS_BY_VIA: Self = Self(2);
    /// Select by origin station and group by next hop.
    pub const CS_FROM_BY_VIA: Self = Self(3);
}

/// `ScriptStationList_Cargo::CargoMode`: ways of associating cargo to stations.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CargoMode(pub u8);
impl CargoMode {
    /// Waiting cargo.
    pub const CM_WAITING: Self = Self(0);
    /// Planned cargo.
    pub const CM_PLANNED: Self = Self(1);
}
