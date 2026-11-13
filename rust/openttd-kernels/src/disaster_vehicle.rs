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

/// `ST_ZEPPELINER`.
pub const ST_ZEPPELINER: u8 = DisasterSubType::ST_ZEPPELINER.0;
/// `ST_ZEPPELINER_SHADOW`.
pub const ST_ZEPPELINER_SHADOW: u8 = DisasterSubType::ST_ZEPPELINER_SHADOW.0;
/// `ST_SMALL_UFO`.
pub const ST_SMALL_UFO: u8 = DisasterSubType::ST_SMALL_UFO.0;
/// `ST_SMALL_UFO_SHADOW`.
pub const ST_SMALL_UFO_SHADOW: u8 = DisasterSubType::ST_SMALL_UFO_SHADOW.0;
/// `ST_AIRPLANE`.
pub const ST_AIRPLANE: u8 = DisasterSubType::ST_AIRPLANE.0;
/// `ST_AIRPLANE_SHADOW`.
pub const ST_AIRPLANE_SHADOW: u8 = DisasterSubType::ST_AIRPLANE_SHADOW.0;
/// `ST_HELICOPTER`.
pub const ST_HELICOPTER: u8 = DisasterSubType::ST_HELICOPTER.0;
/// `ST_HELICOPTER_SHADOW`.
pub const ST_HELICOPTER_SHADOW: u8 = DisasterSubType::ST_HELICOPTER_SHADOW.0;
/// `ST_HELICOPTER_ROTORS`.
pub const ST_HELICOPTER_ROTORS: u8 = DisasterSubType::ST_HELICOPTER_ROTORS.0;
/// `ST_BIG_UFO`.
pub const ST_BIG_UFO: u8 = DisasterSubType::ST_BIG_UFO.0;
/// `ST_BIG_UFO_SHADOW`.
pub const ST_BIG_UFO_SHADOW: u8 = DisasterSubType::ST_BIG_UFO_SHADOW.0;
/// `ST_BIG_UFO_DESTROYER`.
pub const ST_BIG_UFO_DESTROYER: u8 = DisasterSubType::ST_BIG_UFO_DESTROYER.0;
/// `ST_BIG_UFO_DESTROYER_SHADOW`.
pub const ST_BIG_UFO_DESTROYER_SHADOW: u8 = DisasterSubType::ST_BIG_UFO_DESTROYER_SHADOW.0;
/// `ST_SMALL_SUBMARINE`.
pub const ST_SMALL_SUBMARINE: u8 = DisasterSubType::ST_SMALL_SUBMARINE.0;
/// `ST_BIG_SUBMARINE`.
pub const ST_BIG_SUBMARINE: u8 = DisasterSubType::ST_BIG_SUBMARINE.0;
