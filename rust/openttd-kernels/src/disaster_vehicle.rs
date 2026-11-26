/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `disaster_vehicle.h` subtype identifiers.

/// Native `DisasterSubType`: Different sub types of disaster vehicles.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DisasterSubType(pub u8);
impl DisasterSubType {
    /// Zeppelin, crashes at airports.
    pub const ST_ZEPPELINER: Self = Self(0);
    /// Shadow of the zeppelin.
    pub const ST_ZEPPELINER_SHADOW: Self = Self(1);
    /// Small UFO, tries to find a road vehicle to destroy.
    pub const ST_SMALL_UFO: Self = Self(2);
    /// Shadow of small UFO
    pub const ST_SMALL_UFO_SHADOW: Self = Self(3);
    /// Airplane destroying an oil refinery
    pub const ST_AIRPLANE: Self = Self(4);
    /// Shadow of airplane
    pub const ST_AIRPLANE_SHADOW: Self = Self(5);
    /// Helicopter destroying a factory.
    pub const ST_HELICOPTER: Self = Self(6);
    /// Shadow of helicopter.
    pub const ST_HELICOPTER_SHADOW: Self = Self(7);
    /// Rotors of helicopter.
    pub const ST_HELICOPTER_ROTORS: Self = Self(8);
    /// Big UFO, finds a piece of railroad to "park" on
    pub const ST_BIG_UFO: Self = Self(9);
    /// Shadow of the big UFO
    pub const ST_BIG_UFO_SHADOW: Self = Self(10);
    /// Aircraft the will bomb the big UFO
    pub const ST_BIG_UFO_DESTROYER: Self = Self(11);
    /// Shadow of the aircraft.
    pub const ST_BIG_UFO_DESTROYER_SHADOW: Self = Self(12);
    /// Small submarine, pops up in the oceans but doesn't do anything
    pub const ST_SMALL_SUBMARINE: Self = Self(13);
    /// Big submarine, pops up in the oceans but doesn't do anything
    pub const ST_BIG_SUBMARINE: Self = Self(14);
}
