/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `airport.h` and `airport.cpp` values and helpers. `services.cpp` asserts every value.

use crate::tile_type::TileIndex;

/* Some airport-related constants */

/// `NUM_AIRPORTTILES_PER_GRF`: Number of airport tiles per `NewGRF`; limited to 255 to allow
/// extending Action3 with an extended byte later on.
pub const NUM_AIRPORTTILES_PER_GRF: u32 = 255;
/// `NUM_AIRPORTTILES`: Total number of airport tiles.
pub const NUM_AIRPORTTILES: u16 = 256;
/// `NEW_AIRPORTTILE_OFFSET`: offset of first newgrf airport tile
pub const NEW_AIRPORTTILE_OFFSET: u8 = 74;
/// `INVALID_AIRPORTTILE`: id for an invalid airport tile
pub const INVALID_AIRPORTTILE: u16 = NUM_AIRPORTTILES;

/// Native `AirportTypes`: Airport types. The values Rust names.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportTypes(pub u8);
impl AirportTypes {
    /// Small airport.
    pub const AT_SMALL: Self = Self(0);
    /// Large airport.
    pub const AT_LARGE: Self = Self(1);
    /// Oilrig airport.
    pub const AT_OILRIG: Self = Self(9);
    /// Number of the first newgrf airport.
    pub const NEW_AIRPORT_OFFSET: Self = Self(10);
    /// Maximal number of airports per `NewGRF`.
    pub const NUM_AIRPORTS_PER_GRF: Self = Self(128);
    /// Maximal number of airports in total.
    pub const NUM_AIRPORTS: Self = Self(128);
    /// Invalid airport.
    pub const AT_INVALID: Self = Self(254);
    /// Dummy airport.
    pub const AT_DUMMY: Self = Self(255);
}

/// `AT_SMALL`.
pub(crate) const AT_SMALL: u8 = AirportTypes::AT_SMALL.0;
/// `AT_LARGE`.
pub(crate) const AT_LARGE: u8 = AirportTypes::AT_LARGE.0;
/// `AT_OILRIG`.
pub const AT_OILRIG: u8 = AirportTypes::AT_OILRIG.0;
/// `NEW_AIRPORT_OFFSET`.
pub const NEW_AIRPORT_OFFSET: u8 = AirportTypes::NEW_AIRPORT_OFFSET.0;
/// `NUM_AIRPORTS_PER_GRF`, at the width of the `uint` IDs it bounds.
pub const NUM_AIRPORTS_PER_GRF: u32 = AirportTypes::NUM_AIRPORTS_PER_GRF.0 as u32;
/// `NUM_AIRPORTS`, at the width of the airport spec indices it bounds.
pub const NUM_AIRPORTS: u16 = AirportTypes::NUM_AIRPORTS.0 as u16;
/// `AT_INVALID`, at the width of the override manager IDs it is compared with.
pub const AT_INVALID: u16 = AirportTypes::AT_INVALID.0 as u16;
/// `AT_DUMMY`: the dummy airport type with its separate finite-state machine.
pub const AT_DUMMY: u8 = AirportTypes::AT_DUMMY.0;

/// Native `AirportMovingDataFlag`: Flags for airport movement data. The values Rust names.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportMovingDataFlag(pub u8);
impl AirportMovingDataFlag {
    /// Turn slowly (mostly used in the air).
    pub const SLOW_TURN: Self = Self(2);
    /// Go exactly to the destination coordinates.
    pub const EXACT_POSITION: Self = Self(4);
    /// Taxiing at the airport.
    pub const BRAKE: Self = Self(5);
    /// Helicopter take-off.
    pub const HELI_RAISE: Self = Self(6);
    /// Helicopter landing.
    pub const HELI_LOWER: Self = Self(7);
    /// Holding pattern movement (above the airport).
    pub const HOLD: Self = Self(8);
}

/// `AirportMovingDataFlag::SlowTurn` bit number.
pub const AMD_SLOW_TURN: u8 = AirportMovingDataFlag::SLOW_TURN.0;
/// `AirportMovingDataFlag::ExactPosition` bit number.
pub const AMD_EXACT_POSITION: u8 = AirportMovingDataFlag::EXACT_POSITION.0;
/// `AirportMovingDataFlag::Brake` bit number.
pub const AMD_BRAKE: u8 = AirportMovingDataFlag::BRAKE.0;
/// `AirportMovingDataFlag::HeliRaise` bit number.
pub const AMD_HELI_RAISE: u8 = AirportMovingDataFlag::HELI_RAISE.0;
/// `AirportMovingDataFlag::HeliLower` bit number.
pub const AMD_HELI_LOWER: u8 = AirportMovingDataFlag::HELI_LOWER.0;
/// `AirportMovingDataFlag::Hold` bit number.
pub const AMD_HOLD: u8 = AirportMovingDataFlag::HOLD.0;

/// Native `AirportMovementStates`: Movement States on Airports (headings target). The values
/// Rust names.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportMovementStates(pub u8);
impl AirportMovementStates {
    /// Heading for hangar.
    pub const HANGAR: Self = Self(1);
    /// Heading for terminal 1.
    pub const TERM1: Self = Self(2);
    /// Heading for terminal 2.
    pub const TERM2: Self = Self(3);
    /// Heading for terminal 3.
    pub const TERM3: Self = Self(4);
    /// Heading for terminal 4.
    pub const TERM4: Self = Self(5);
    /// Heading for terminal 5.
    pub const TERM5: Self = Self(6);
    /// Heading for terminal 6.
    pub const TERM6: Self = Self(7);
    /// Heading for helipad 1.
    pub const HELIPAD1: Self = Self(8);
    /// Heading for helipad 2.
    pub const HELIPAD2: Self = Self(9);
    /// Airplane wants to leave the airport.
    pub const TAKEOFF: Self = Self(10);
    /// Airplane has arrived at a runway for take-off.
    pub const STARTTAKEOFF: Self = Self(11);
    /// Airplane has reached end-point of the take-off runway.
    pub const ENDTAKEOFF: Self = Self(12);
    /// Helicopter wants to leave the airport.
    pub const HELITAKEOFF: Self = Self(13);
    /// Vehicle is flying in the air.
    pub const FLYING: Self = Self(14);
    /// Airplane wants to land.
    pub const LANDING: Self = Self(15);
    /// Airplane wants to finish landing.
    pub const ENDLANDING: Self = Self(16);
    /// Helicopter wants to land.
    pub const HELILANDING: Self = Self(17);
    /// Helicopter wants to finish landing.
    pub const HELIENDLANDING: Self = Self(18);
    /// Heading for terminal 7.
    pub const TERM7: Self = Self(19);
    /// Heading for terminal 8.
    pub const TERM8: Self = Self(20);
    /// Heading for helipad 3.
    pub const HELIPAD3: Self = Self(21);
}

/// `HANGAR`.
pub const HANGAR: u8 = AirportMovementStates::HANGAR.0;
/// `TERM1`.
pub const TERM1: u8 = AirportMovementStates::TERM1.0;
/// `TERM2`.
pub const TERM2: u8 = AirportMovementStates::TERM2.0;
/// `TERM3`.
pub const TERM3: u8 = AirportMovementStates::TERM3.0;
/// `TERM4`.
pub const TERM4: u8 = AirportMovementStates::TERM4.0;
/// `TERM5`.
pub const TERM5: u8 = AirportMovementStates::TERM5.0;
/// `TERM6`.
pub const TERM6: u8 = AirportMovementStates::TERM6.0;
/// `HELIPAD1`.
pub const HELIPAD1: u8 = AirportMovementStates::HELIPAD1.0;
/// `HELIPAD2`.
pub const HELIPAD2: u8 = AirportMovementStates::HELIPAD2.0;
/// `TAKEOFF`.
pub const TAKEOFF: u8 = AirportMovementStates::TAKEOFF.0;
/// `STARTTAKEOFF`.
pub const STARTTAKEOFF: u8 = AirportMovementStates::STARTTAKEOFF.0;
/// `ENDTAKEOFF`.
pub const ENDTAKEOFF: u8 = AirportMovementStates::ENDTAKEOFF.0;
/// `HELITAKEOFF`.
pub const HELITAKEOFF: u8 = AirportMovementStates::HELITAKEOFF.0;
/// `FLYING`.
pub const FLYING: u8 = AirportMovementStates::FLYING.0;
/// `LANDING`.
pub const LANDING: u8 = AirportMovementStates::LANDING.0;
/// `ENDLANDING`.
pub const ENDLANDING: u8 = AirportMovementStates::ENDLANDING.0;
/// `HELILANDING`.
pub const HELILANDING: u8 = AirportMovementStates::HELILANDING.0;
/// `HELIENDLANDING`.
pub const HELIENDLANDING: u8 = AirportMovementStates::HELIENDLANDING.0;
/// `TERM7`.
pub const TERM7: u8 = AirportMovementStates::TERM7.0;
/// `TERM8`.
pub const TERM8: u8 = AirportMovementStates::TERM8.0;
/// `HELIPAD3`.
pub const HELIPAD3: u8 = AirportMovementStates::HELIPAD3.0;

/// Native `AirportFTAClass::Flag`: Bitmask of airport flags.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportFTAClassFlag(pub u8);
impl AirportFTAClassFlag {
    /// Can planes land on this airport type?
    pub const AIRPLANES: Self = Self(0);
    /// Can helicopters land on this airport type?
    pub const HELICOPTERS: Self = Self(1);
    /// This airport has a short landing strip, dangerous for fast aircraft.
    pub const SHORT_STRIP: Self = Self(2);
}

/// `AirportFTAClass::Flags` mask of `Flag::Airplanes`.
pub const FTA_AIRPLANES: u8 = 1 << AirportFTAClassFlag::AIRPLANES.0;
/// `AirportFTAClass::Flags` mask of `Flag::Helicopters`.
pub const FTA_HELICOPTERS: u8 = 1 << AirportFTAClassFlag::HELICOPTERS.0;
/// `AirportFTAClass::Flags` mask of `Flag::ShortStrip`.
pub const FTA_SHORT_STRIP: u8 = 1 << AirportFTAClassFlag::SHORT_STRIP.0;

/// Original `GetAirport`: get the finite state machine of an airport type, preserving the
/// distinct dummy finite-state machine.
pub(crate) fn get_airport(
    airport_type: u8,
    s: &crate::newgrf_airport::Services,
) -> *const std::ffi::c_void {
    if airport_type == AT_DUMMY {
        s.dummy_fta
    } else {
        (s.fta)(crate::newgrf_airport::get(airport_type, s))
    }
}

/// Original `GetVehiclePosOnBuild`: the position (index in airport node array) where an
/// aircraft built on `hangar_tile` ends up. The hangar operation consumes the selected spec
/// directly, keeping its native scan and rotation.
pub(crate) fn get_vehicle_pos_on_build(
    hangar_tile: TileIndex,
    s: &crate::newgrf_airport::Services,
) -> u8 {
    let st = crate::station_map::station_get_by_tile(hangar_tile, s);
    let spec = crate::newgrf_airport::station_spec(st, s);
    let apc = (s.fta)(spec);
    /* When we click on hangar we know the tile it is on. By that we know
     * its position in the array of depots the airport has.....we can search
     * layout for #th position of depot. Since layout must start with a listing
     * of all depots, it is simple */
    let mut i = 0_u32;
    loop {
        if TileIndex((s.hangar_tile)(st, i, spec)) == hangar_tile {
            (s.check)((s.fta_heading)(apc, i) == HANGAR);
            return (s.fta_position)(apc, i);
        }
        i = i.wrapping_add(1);
    }
}
