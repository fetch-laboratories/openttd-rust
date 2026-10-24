/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `airport.h` values. `services.cpp` asserts every value.

/// `HANGAR`.
pub const HANGAR: u8 = 1;
/// `AT_OILRIG`.
pub const AT_OILRIG: u8 = 9;

pub const TERM1: u8 = 2;
pub const TERM2: u8 = 3;
pub const TERM3: u8 = 4;
pub const TERM4: u8 = 5;
pub const TERM5: u8 = 6;
pub const TERM6: u8 = 7;
pub const HELIPAD1: u8 = 8;
pub const HELIPAD2: u8 = 9;
pub const TAKEOFF: u8 = 10;
pub const STARTTAKEOFF: u8 = 11;
pub const ENDTAKEOFF: u8 = 12;
pub const HELITAKEOFF: u8 = 13;
pub const FLYING: u8 = 14;
pub const LANDING: u8 = 15;
pub const ENDLANDING: u8 = 16;
pub const HELILANDING: u8 = 17;
pub const HELIENDLANDING: u8 = 18;
pub const TERM7: u8 = 19;
pub const TERM8: u8 = 20;
pub const HELIPAD3: u8 = 21;
pub const AMD_SLOW_TURN: u8 = 2;
pub const AMD_EXACT_POSITION: u8 = 4;
pub const AMD_BRAKE: u8 = 5;
pub const AMD_HELI_RAISE: u8 = 6;
pub const AMD_HELI_LOWER: u8 = 7;
pub const AMD_HOLD: u8 = 8;
pub const ATP_TTDP_LARGE: u8 = 1;

/// First custom airport.
pub const NEW_AIRPORT_OFFSET: u8 = 10;
/// Invalid airport type.
pub const AT_INVALID: u16 = 254;
/// First custom airport tile.
pub const NEW_AIRPORTTILE_OFFSET: u8 = 74;
/// Invalid airport tile spec.
pub const INVALID_AIRPORTTILE: u16 = 256;

/// Dummy airport type with its separate finite-state machine.
pub const AT_DUMMY: u8 = 255;

/// `AirportFTAClass::Flag` bit masks.
pub const FTA_AIRPLANES: u8 = 1;
pub const FTA_HELICOPTERS: u8 = 2;
pub const FTA_SHORT_STRIP: u8 = 4;
/// `GetAirport`, preserving the distinct dummy finite-state machine.
pub(crate) fn get_airport(
    kind: u8,
    s: &crate::newgrf_airport::Services,
) -> *const std::ffi::c_void {
    if kind == AT_DUMMY {
        s.dummy_fta
    } else {
        (s.fta)(crate::newgrf_airport::get(kind, s))
    }
}
/// Original `GetVehiclePosOnBuild`; the named hangar operation consumes the
/// selected spec directly, keeping its native scan/rotation implementation.
pub(crate) fn get_vehicle_pos_on_build(tile: u32, s: &crate::newgrf_airport::Services) -> u8 {
    let st = crate::station_map::station_by_tile(tile, s);
    let spec = crate::newgrf_airport::station_spec(st, s);
    let fta = (s.fta)(spec);
    let mut i = 0_u32;
    loop {
        if (s.hangar_tile)(st, i, spec) == tile {
            (s.check)((s.fta_heading)(fta, i) == HANGAR);
            return (s.fta_position)(fta, i);
        }
        i = i.wrapping_add(1);
    }
}
