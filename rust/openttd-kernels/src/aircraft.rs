/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
//! `aircraft.h` and `aircraft_cmd.cpp`: aircraft and airport movement, over the
//! Rust-owned aircraft state and airport block mask.
//!
//! Every entry runs synchronously over live handles. Raw field accesses end before each
//! `noexcept` callback, and no Rust borrow of an owner survives a callback, including
//! reentry and `PreDestructor` deletion. Station and `AirportFTAClass` records stay in
//! C++ and are read through one typed leaf per field.
#![allow(unsafe_code)]

use crate::airport::{
    AT_DUMMY, AT_OILRIG, AirportBlock, AirportBlocks, AirportFTAClassFlag, AirportMovementStates,
    AirportMovingData, AirportMovingDataFlag, ENDLANDING, ENDTAKEOFF, FLYING, HANGAR,
    HELIENDLANDING, HELILANDING, HELIPAD1, HELIPAD2, HELIPAD3, HELITAKEOFF, LANDING, MAX_TERMINALS,
    STARTTAKEOFF, TAKEOFF, TERM1, TERM2, TERM3, TERM4, TERM5, TERM6, TERM7, TERM8,
};
use crate::bitmath_func::{clr_bit8, gb, has_bit, set_bit8};
use crate::company_type::Owner;
use crate::direction_func::{
    change_diag_dir, change_dir, diag_dir_difference, dir_difference, dir_to_diag_dir,
};
use crate::direction_type::{DiagDirection, DirDiff, Direction};
use crate::effectvehicle_func::EffectVehicleType;
use crate::engine_type::{AIR_CTOL, AIR_FAST};
use crate::map_func::Map;
use crate::newgrf_properties::{
    PROP_AIRCRAFT_CARGO_AGE_PERIOD, PROP_AIRCRAFT_RANGE, PROP_AIRCRAFT_SPEED,
};
use crate::newgrf_sound::{VSE_START, VSE_TOUCHDOWN};
use crate::order_type::{ODATF_NEAREST_DEPOT, ODTF_PART_OF_ORDERS, OrderType};
use crate::services::Services;
use crate::sound_type::{ORIGINAL_SAMPLE_COUNT, SoundFx, SoundID};
use crate::station::Station;
use crate::station_type::{AirportAnimationTrigger, StationID};
use crate::tile_type::{TILE_HEIGHT, TILE_SIZE, TileIndex};
use crate::vehicle::maintenance;
use crate::vehicle::{OpenTTDVehicle, VehicleRef, VehicleServices, YEAR_TICKS};
use crate::vehicle_base::{VehState, VehStates};
use crate::vehicle_type::{DepotCommandFlag, DepotCommandFlags, VehicleID, VehicleType};
use std::ffi::c_void;
use std::ptr::NonNull;

#[cfg(test)]
mod tests;

/// Base values for flight levels above ground level for 'normal' flight and holding patterns.
/// Due to speed and direction, the actual flight level may be higher.
/// `AIRCRAFT_MIN_FLYING_ALTITUDE`: Minimum flying altitude above tile.
pub(crate) const AIRCRAFT_MIN_FLYING_ALTITUDE: i32 = 120;
/// `AIRCRAFT_MAX_FLYING_ALTITUDE`: Maximum flying altitude above tile.
pub(crate) const AIRCRAFT_MAX_FLYING_ALTITUDE: i32 = 360;
/// `PLANE_HOLD_MAX_FLYING_ALTITUDE`: holding flying altitude above tile of planes.
pub(crate) const PLANE_HOLD_MAX_FLYING_ALTITUDE: i32 = 150;
/// `HELICOPTER_HOLD_MAX_FLYING_ALTITUDE`: holding flying altitude above tile of helicopters.
pub(crate) const HELICOPTER_HOLD_MAX_FLYING_ALTITUDE: i32 = 184;

/// Native `AircraftSubType`: an aircraft can be one of those types.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AircraftSubType(pub u8);
impl AircraftSubType {
    /// an helicopter
    pub const AIR_HELICOPTER: Self = Self(0);
    /// an airplane
    pub const AIR_AIRCRAFT: Self = Self(2);
    /// shadow of the aircraft
    pub const AIR_SHADOW: Self = Self(4);
}
/// `AIR_HELICOPTER`.
pub const AIR_HELICOPTER: u8 = AircraftSubType::AIR_HELICOPTER.0;
/// `AIR_AIRCRAFT`.
pub const AIR_AIRCRAFT: u8 = AircraftSubType::AIR_AIRCRAFT.0;
/// `AIR_SHADOW`.
pub(crate) const AIR_SHADOW: u8 = AircraftSubType::AIR_SHADOW.0;

/// Native `AirVehicleFlags`: flags for air vehicles; shared with disaster vehicles.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirVehicleFlags(pub u8);
impl AirVehicleFlags {
    /// Next destination is too far away.
    pub const VAF_DEST_TOO_FAR: Self = Self(0);

    /* The next two flags are to prevent stair climbing of the aircraft. The idea is that the aircraft
     * will ascend or descend multiple flight levels at a time instead of following the contours of the
     * landscape at a fixed altitude. This only has effect when there are more than 15 height levels. */
    /// The vehicle is currently lowering its altitude because it hit the upper bound.
    pub const VAF_IN_MAX_HEIGHT_CORRECTION: Self = Self(1);
    /// The vehicle is currently raising its altitude because it hit the lower bound.
    pub const VAF_IN_MIN_HEIGHT_CORRECTION: Self = Self(2);

    /// The helicopter is descending directly at its destination (helipad or in front of hangar)
    pub const VAF_HELI_DIRECT_DESCENT: Self = Self(3);
}

/// `ROTOR_Z_OFFSET`: Z Offset between helicopter- and rotorsprite.
pub(crate) const ROTOR_Z_OFFSET: i32 = 5;

/// Native `HelicopterRotorStates`: Helicopter rotor animation states
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct HelicopterRotorStates(u8);
impl HelicopterRotorStates {
    const HRS_ROTOR_STOPPED: Self = Self(0);
    const HRS_ROTOR_MOVING_1: Self = Self(1);
    const HRS_ROTOR_MOVING_3: Self = Self(3);
}

/* Special velocities for aircraft */
/// `SPEED_LIMIT_TAXI`: Maximum speed of an aircraft while taxiing
const SPEED_LIMIT_TAXI: u16 = 50;
/// `SPEED_LIMIT_APPROACH`: Maximum speed of an aircraft on finals
const SPEED_LIMIT_APPROACH: u16 = 230;
/// `SPEED_LIMIT_BROKEN`: Maximum speed of an aircraft that is broken
const SPEED_LIMIT_BROKEN: u16 = 320;
/// `SPEED_LIMIT_HOLD`: Maximum speed of an aircraft that flies the holding pattern
const SPEED_LIMIT_HOLD: u16 = 425;
/// `SPEED_LIMIT_NONE`: No environmental speed limit. Speed limit is type dependent
const SPEED_LIMIT_NONE: u16 = u16::MAX;

/// Native `MovementTerminalMapping`: combination of aircraft state for going to a certain
/// terminal and the airport flag for that terminal block.
struct MovementTerminalMapping {
    /// Aircraft movement state when going to this terminal.
    state: u8,
    /// Bitmask in the airport flags that need to be free for this terminal.
    blocks: AirportBlock,
}

/// `_airport_terminal_mapping`: A list of all valid terminals and their associated blocks.
const AIRPORT_TERMINAL_MAPPING: [MovementTerminalMapping; 11] = [
    MovementTerminalMapping {
        state: TERM1,
        blocks: AirportBlock::TERM1,
    },
    MovementTerminalMapping {
        state: TERM2,
        blocks: AirportBlock::TERM2,
    },
    MovementTerminalMapping {
        state: TERM3,
        blocks: AirportBlock::TERM3,
    },
    MovementTerminalMapping {
        state: TERM4,
        blocks: AirportBlock::TERM4,
    },
    MovementTerminalMapping {
        state: TERM5,
        blocks: AirportBlock::TERM5,
    },
    MovementTerminalMapping {
        state: TERM6,
        blocks: AirportBlock::TERM6,
    },
    MovementTerminalMapping {
        state: TERM7,
        blocks: AirportBlock::TERM7,
    },
    MovementTerminalMapping {
        state: TERM8,
        blocks: AirportBlock::TERM8,
    },
    MovementTerminalMapping {
        state: HELIPAD1,
        blocks: AirportBlock::HELIPAD1,
    },
    MovementTerminalMapping {
        state: HELIPAD2,
        blocks: AirportBlock::HELIPAD2,
    },
    MovementTerminalMapping {
        state: HELIPAD3,
        blocks: AirportBlock::HELIPAD3,
    },
];

/// The Rust-owned fields of `Aircraft`, at their original widths. `cache_padding` keeps the
/// layout of `AircraftCache` before the scalar fields.
#[repr(C)]
pub struct AircraftState {
    pub cached_max_range_sqr: u32,
    pub cached_max_range: u16,
    pub cache_padding: u16,
    pub crashed_counter: u16,
    pub targetairport: u16,
    pub pos: u8,
    pub previous_pos: u8,
    pub state: u8,
    pub last_direction: u8,
    pub number_consecutive_turns: u8,
    pub turn_counter: u8,
    pub flags: u8,
}

/// A live aircraft handle: its vehicle record, its aircraft state (null for a disaster
/// vehicle in the flight helpers) and, for a front aircraft, its `VehicleID`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vehicle {
    pub handle: *mut OpenTTDVehicle,
    pub state: *mut AircraftState,
    pub id: u32,
}

/// One accessor pair for each field of `AircraftState`.
macro_rules! aircraft_fields {
    ($($(#[$doc:meta])* $field:ident, $set:ident: $ty:ty => $wrap:expr, $raw:expr;)*) => {
        impl Vehicle {
            $(
                $(#[$doc])*
                fn $field(self) -> $ty {
                    let raw = crate::raw_field::read_field!(self.state, $field);
                    $wrap(raw)
                }
                fn $set(self, value: $ty) {
                    crate::raw_field::write_field!(self.state, $field, $raw(value));
                }
            )*
        }
    };
}
aircraft_fields! {
    /// `pos`: next desired position of the aircraft.
    pos, set_pos: u8 => std::convert::identity, std::convert::identity;
    /// `previous_pos`: previous desired position of the aircraft.
    previous_pos, set_previous_pos: u8 => std::convert::identity, std::convert::identity;
    /// `state`: state of the airport, see `AirportMovementStates`.
    state, set_state: u8 => std::convert::identity, std::convert::identity;
    /// `targetairport`: airport to go to next.
    targetairport, set_targetairport: StationID => StationID, |s: StationID| s.0;
    /// `last_direction`.
    last_direction, set_last_direction: Direction => Direction, |d: Direction| d.0;
    /// `number_consecutive_turns`: protection to prevent the aircraft of making a lot of
    /// turns in order to reach a specific point.
    number_consecutive_turns, set_number_consecutive_turns: u8 => std::convert::identity, std::convert::identity;
    /// `turn_counter`: ticks between each turn to prevent > 45 degree turns.
    turn_counter, set_turn_counter: u8 => std::convert::identity, std::convert::identity;
    /// `flags`: aircraft flags, see `AirVehicleFlags`.
    flags, set_flags: u8 => std::convert::identity, std::convert::identity;
    /// `crashed_counter`: timer for handling crash animations.
    crashed_counter, set_crashed_counter: u16 => std::convert::identity, std::convert::identity;
    /// `acache.cached_max_range`: cached maximum range.
    cached_max_range, set_cached_max_range: u16 => std::convert::identity, std::convert::identity;
    /// `acache.cached_max_range_sqr`: cached squared maximum range.
    cached_max_range_sqr, set_cached_max_range_sqr: u32 => std::convert::identity, std::convert::identity;
}
impl Vehicle {
    fn base(self) -> VehicleRef {
        // SAFETY: Every entry receives a live, non-null vehicle record.
        VehicleRef(unsafe { NonNull::new_unchecked(self.handle) })
    }
    /// `v->index`.
    fn index(self) -> VehicleID {
        VehicleID(self.id)
    }
    /// The aircraft that starts an iteration before the pool's first.
    fn invalid() -> Self {
        Self {
            handle: std::ptr::null_mut(),
            state: std::ptr::null_mut(),
            id: VehicleID::INVALID.0,
        }
    }
}

/// `v->Next()`, which the original dereferences without a test: the shadow of every
/// aircraft, and the rotor of a helicopter's shadow.
fn next_part(v: VehicleRef) -> VehicleRef {
    // SAFETY: The original unconditionally dereferences the live shadow and rotor.
    unsafe { v.next().unwrap_unchecked() }
}
/// `v->vehstatus.Test(state)`.
fn vehstatus_test(v: VehicleRef, state: VehState) -> bool {
    v.status() & VehStates::from_element(state).0 != 0
}
/// `v->vehstatus.Set(state)` or `Reset(state)`.
fn vehstatus_assign(v: VehicleRef, state: VehState, on: bool) {
    let mask = VehStates::from_element(state).0;
    let status = v.status();
    v.write_status(if on { status | mask } else { status & !mask });
}
/// `Aircraft::state` of a rotor, which stores its `HelicopterRotorStates` there.
fn rotor_state(u: VehicleRef) -> HelicopterRotorStates {
    HelicopterRotorStates(crate::raw_field::read_field!(u.aircraft_state(), state))
}
fn set_rotor_state(u: VehicleRef, state: HelicopterRotorStates) {
    crate::raw_field::write_field!(u.aircraft_state(), state, state.0);
}

/// The C++ leaves of the aircraft controller: shared operations and the fields of the
/// station and `AirportFTAClass` records, which C++ stores.
#[repr(C)]
pub struct Leaves {
    /// `&st->airport.blocks`, the Rust allocation the station holds.
    pub airport_blocks: unsafe extern "C" fn(*const Station) -> *mut AirportBlocks,
    /// `AirportFTAClass::delta_z`.
    pub airport_delta_z: unsafe extern "C" fn(*const c_void) -> u8,
    /// `AirportFTAClass::nofelements`.
    pub airport_elements: unsafe extern "C" fn(*const c_void) -> u8,
    /// `AirportFTAClass::entry_points[dir]`.
    pub airport_entry: unsafe extern "C" fn(*const c_void, u8) -> u8,
    /// `AirportFTAClass::flags`.
    pub airport_flags: unsafe extern "C" fn(*const c_void) -> u8,
    /// `st->airport.h`.
    pub airport_height: unsafe extern "C" fn(*const Station) -> u8,
    /// `st->airport.tile`.
    pub airport_tile: unsafe extern "C" fn(*const Station) -> u32,
    /// `st->airport.type`.
    pub airport_type: unsafe extern "C" fn(*const Station) -> u8,
    /// `st->airport.w`.
    pub airport_width: unsafe extern "C" fn(*const Station) -> u8,
    /// `u->sprite_cache.sprite_seq.CopyWithoutPalette(v->sprite_cache.sprite_seq)`.
    pub copy_sprite: unsafe extern "C" fn(VehicleRef, VehicleRef),
    /// `CrashAirplane`'s news and script events.
    pub crash_news: unsafe extern "C" fn(Vehicle, u16, u32),
    /// `DeleteVehicleNews(v->index, AdviceType::AircraftDestinationTooFar)`.
    pub delete_range_news: unsafe extern "C" fn(Vehicle),
    /// `AircraftHandleDestTooFar`'s script event and advice news.
    pub destination_too_far: unsafe extern "C" fn(Vehicle),
    /// `GetDirectionTowards`.
    pub direction_towards: unsafe extern "C" fn(Vehicle, i32, i32) -> u8,
    /// `AircraftLeaveHangar`'s depot and aircraft list windows.
    pub dirty_depot: unsafe extern "C" fn(Vehicle),
    /// `OnNewEconomyDay`'s details and aircraft list windows.
    pub dirty_lists: unsafe extern "C" fn(Vehicle),
    /// `AircraftEntersTerminal`'s first-arrival news and script events.
    pub first_arrival: unsafe extern "C" fn(Vehicle, u16),
    /// `AirportFTA::blocks`.
    pub fta_blocks: unsafe extern "C" fn(*const c_void) -> AirportBlocks,
    /// `AirportFTA::heading`.
    pub fta_heading: unsafe extern "C" fn(*const c_void) -> u8,
    /// `AirportFTA::next`, or null.
    pub fta_next: unsafe extern "C" fn(*const c_void) -> *const c_void,
    /// `AirportFTA::next_position`.
    pub fta_next_position: unsafe extern "C" fn(*const c_void) -> u8,
    /// `AirportFTA::position`.
    pub fta_position: unsafe extern "C" fn(*const c_void) -> u8,
    /// `st->had_vehicle_of_type`, for `NewGRF` station variable 0x8A.
    pub had_vehicle: unsafe extern "C" fn(*const Station) -> u8,
    /// `Airport::GetHangarExitDirection(tile)`.
    pub hangar_exit:
        unsafe extern "C" fn(*const Station, u32, *const crate::station_cmd::AirportSpec) -> u8,
    /// `AirportFTAClass::num_helipads`.
    pub helipads: unsafe extern "C" fn(*const c_void) -> u8,
    /// `AircraftEventHandler_General`'s `FatalError`.
    pub invalid_scheme: unsafe extern "C" fn(Vehicle),
    /// `MaybeCrashAirplane`'s goods entry reset for one cargo.
    pub landing_rating: unsafe extern "C" fn(u16, u8),
    /// The depot command's services, for `CMD_SEND_VEHICLE_TO_DEPOT`.
    pub mass: *const crate::vehicle_transactions::mass::Services,
    /// `AirportFTAClass::MovingData(position)`, with its assertion.
    pub moving: unsafe extern "C" fn(*const c_void, u8) -> AirportMovingData,
    /// The next aircraft of the pool after a `VehicleID`.
    pub next_aircraft: unsafe extern "C" fn(u32) -> Vehicle,
    /// `&apc->layout[position]`.
    pub node: unsafe extern "C" fn(*const c_void, u8) -> *const c_void,
    /// `st->airport.rotation`.
    pub rotation: unsafe extern "C" fn(*const Station) -> u8,
    /// `GetRotorImage` into the rotor's sprite.
    pub rotor_image: unsafe extern "C" fn(Vehicle),
    /// `GetRotorImage`; true when the rotor's sprite is unchanged.
    pub rotor_image_if_changed: unsafe extern "C" fn(Vehicle) -> bool,
    /// `Company::Get(v->owner)->settings.vehicle.servint_aircraft`.
    pub service_interval: unsafe extern "C" fn(Vehicle) -> u16,
    /// `v->current_order.MakeGoToDepot(station, OrderDepotTypeFlag::Service)`.
    pub service_order: unsafe extern "C" fn(Vehicle, u16),
    pub shared: *const Services,
    /// `Station::GetIfValid`.
    pub station: unsafe extern "C" fn(u16) -> *const Station,
    /// `st->owner`.
    pub station_owner: unsafe extern "C" fn(*const Station) -> u8,
    /// `st->xy`.
    pub station_tile: unsafe extern "C" fn(*const Station) -> u32,
    /// `SubtractMoneyFromCompanyFract(v->owner, cost)`.
    pub subtract_cost: unsafe extern "C" fn(Vehicle, i64),
    /// `AirportFTAClass::terminals[index]`.
    pub terminal_count: unsafe extern "C" fn(*const c_void, u32) -> u8,
    /// `v->cargo.Truncate()`.
    pub truncate_cargo: unsafe extern "C" fn(VehicleRef),
    /// `GetRotorImage` into a new rotor sprite.
    pub update_rotor_image: unsafe extern "C" fn(Vehicle),
    pub vehicle: *const VehicleServices,
    /// `Vehicle::Crash(flooded)`.
    pub vehicle_crash: unsafe extern "C" fn(Vehicle, bool) -> u32,
    /// `v->UpdateViewport(true, false)`.
    pub viewport: unsafe extern "C" fn(Vehicle),
}

/// The tables and entry-local map descriptor the helpers share.
struct World<'a, const COUNTED: bool> {
    leaves: &'a Leaves,
    vs: &'a VehicleServices,
    ss: &'a Services,
    movement: crate::movement::CachedTable<'a>,
    map: crate::map_func::World<'a>,
}

impl<const COUNTED: bool> World<'_, COUNTED> {
    // SAFETY: Immutable tables outlive this synchronous entry.
    unsafe fn new(leaves: *const Leaves) -> Self {
        let leaves = unsafe { &*leaves };
        let ss = unsafe { &*leaves.shared };
        Self {
            leaves,
            vs: unsafe { &*leaves.vehicle },
            ss,
            movement: crate::movement::CachedTable::new(ss),
            map: crate::map_func::World::new(ss),
        }
    }

    /// Count one outgoing call and the size of its result. The only production enable
    /// call is `WitnessWriter`'s process-start constructor.
    #[inline]
    fn call<T>(call: impl FnOnce() -> T) -> T {
        let result = call();
        if COUNTED {
            crate::witness::hit(crate::witness::AIRCRAFT_SERVICES);
            crate::witness::add(
                crate::witness::AIRCRAFT_BYTES,
                std::mem::size_of::<T>() as u64,
            );
        }
        result
    }
    /// Count one outgoing call that resolves a station.
    #[inline]
    fn station_call<T>(call: impl FnOnce() -> T) -> T {
        let result = Self::call(call);
        if COUNTED {
            crate::witness::hit(crate::witness::AIRCRAFT_STATIONS);
        }
        result
    }
    /// The native assertion.
    #[inline]
    fn check(&self, condition: bool) {
        self.map.check(condition);
    }
    fn dims(&self) -> Map {
        self.map.dimensions
    }
    /// `Map::MaxX() * TILE_SIZE` and `Map::MaxY() * TILE_SIZE`, as the `int` arguments of
    /// `Clamp`.
    fn map_max_xy(&self) -> (i32, i32) {
        (
            self.dims().max_x().wrapping_mul(TILE_SIZE).cast_signed(),
            Map::max_y(self.map.size_y())
                .wrapping_mul(TILE_SIZE)
                .cast_signed(),
        )
    }
    fn clamp(&self, a: i32, min: i32, max: i32) -> i32 {
        crate::math::clamp(a, min, max, |c| self.check(c))
    }
    /// `TileVirtXY(x, y)` of `int` coordinates.
    fn tile_virt_xy(&self, x: i32, y: i32) -> TileIndex {
        self.dims()
            .tile_virt_xy(x.cast_unsigned(), y.cast_unsigned())
    }
    fn get_slope_pixel_z(&self, x: i32, y: i32) -> i32 {
        self.movement.get().get_slope_pixel_z(x, y, false)
    }

    /* Station fields. */
    /// `Station::GetIfValid(id)`.
    fn station_get_if_valid(&self, id: StationID) -> *const Station {
        Self::station_call(|| unsafe { (self.leaves.station)(id.0) })
    }
    /// `Station::Get(id)`, with its pool assertion.
    fn station_get(id: StationID) -> *const Station {
        Self::station_call(|| {
            crate::newgrf_airport::station_by_id(id.0, crate::newgrf_airport::runtime())
                .cast_const()
                .cast()
        })
    }
    fn airport_tile(&self, st: *const Station) -> TileIndex {
        TileIndex(Self::call(|| unsafe { (self.leaves.airport_tile)(st) }))
    }
    fn station_xy(&self, st: *const Station) -> TileIndex {
        TileIndex(Self::call(|| unsafe { (self.leaves.station_tile)(st) }))
    }
    fn airport_rotation(&self, st: *const Station) -> Direction {
        Direction(Self::call(|| unsafe { (self.leaves.rotation)(st) }))
    }
    fn airport_type(&self, st: *const Station) -> u8 {
        Self::call(|| unsafe { (self.leaves.airport_type)(st) })
    }
    fn airport_w(&self, st: *const Station) -> u8 {
        Self::call(|| unsafe { (self.leaves.airport_width)(st) })
    }
    fn airport_h(&self, st: *const Station) -> u8 {
        Self::call(|| unsafe { (self.leaves.airport_height)(st) })
    }
    fn station_owner(&self, st: *const Station) -> Owner {
        crate::company_type::CompanyID(Self::call(|| unsafe { (self.leaves.station_owner)(st) }))
    }
    /// `st->airport.blocks`, read in place.
    fn airport_blocks(&self, st: *const Station) -> AirportBlocks {
        let blocks = Self::call(|| unsafe { (self.leaves.airport_blocks)(st) });
        // SAFETY: One field-sized load of the live mask, ending before any callback.
        unsafe { blocks.read() }
    }
    /// `st->airport.blocks.Set(set)` or `Reset(set)` in place.
    fn airport_blocks_update(&self, st: *const Station, update: impl FnOnce(&mut AirportBlocks)) {
        let blocks = Self::call(|| unsafe { (self.leaves.airport_blocks)(st) });
        // SAFETY: One field-sized load and store of the live mask, with no callback between.
        unsafe {
            let mut value = blocks.read();
            update(&mut value);
            blocks.write(value);
        }
    }
    /// `st->airport.GetSpec()`.
    fn airport_spec(st: *const Station) -> *const crate::station_cmd::AirportSpec {
        Self::call(|| {
            crate::newgrf_airport::station_spec(
                st.cast_mut().cast(),
                crate::newgrf_airport::runtime(),
            )
        })
    }
    /// `st->airport.GetFTA()`.
    fn airport_get_fta(st: *const Station) -> *const c_void {
        Self::call(|| {
            crate::newgrf_airport::station_fta(
                st.cast_mut().cast(),
                crate::newgrf_airport::runtime(),
            )
        })
    }
    /// `st->airport.HasHangar()`.
    fn airport_has_hangar(st: *const Station) -> bool {
        Self::call(|| {
            crate::station_base::airport_has_hangar(st.cast(), crate::newgrf_airport::runtime())
        })
    }
    /// `st->facilities.Test(StationFacility::Airport)`.
    fn station_has_airport(st: *const Station) -> bool {
        let runtime = crate::newgrf_airport::runtime();
        // SAFETY: The station command table names its immutable station table.
        let facilities = Self::call(|| unsafe {
            ((*runtime.commands().station).facilities)(st.cast_mut().cast())
        });
        facilities & crate::station_type::FACILITY_AIRPORT != 0
    }
    /// `CanVehicleUseStation(v, st)`.
    fn can_vehicle_use_station(v: Vehicle, st: *const Station) -> bool {
        // SAFETY: A live vehicle and station for this synchronous call.
        unsafe {
            crate::vehicle_func::can_vehicle_use_station(
                v.handle,
                st.cast(),
                crate::newgrf_airport::runtime(),
            )
        }
    }

    /* AirportFTAClass fields. */
    fn nofelements(&self, apc: *const c_void) -> u8 {
        Self::call(|| unsafe { (self.leaves.airport_elements)(apc) })
    }
    fn num_helipads(&self, apc: *const c_void) -> u8 {
        Self::call(|| unsafe { (self.leaves.helipads)(apc) })
    }
    fn fta_flags(&self, apc: *const c_void) -> u8 {
        Self::call(|| unsafe { (self.leaves.airport_flags)(apc) })
    }
    fn fta_flags_test(flags: u8, flag: AirportFTAClassFlag) -> bool {
        flags & (1 << flag.0) != 0
    }
    fn delta_z(&self, apc: *const c_void) -> u8 {
        Self::call(|| unsafe { (self.leaves.airport_delta_z)(apc) })
    }
    fn terminals(&self, apc: *const c_void, index: u32) -> u8 {
        Self::call(|| unsafe { (self.leaves.terminal_count)(apc, index) })
    }
    fn entry_points(&self, apc: *const c_void, dir: DiagDirection) -> u8 {
        Self::call(|| unsafe { (self.leaves.airport_entry)(apc, dir.0) })
    }
    fn moving_data(&self, apc: *const c_void, position: u8) -> AirportMovingData {
        Self::call(|| unsafe { (self.leaves.moving)(apc, position) })
    }
    /// `&apc->layout[position]`.
    fn layout(&self, apc: *const c_void, position: u8) -> *const c_void {
        Self::call(|| unsafe { (self.leaves.node)(apc, position) })
    }

    /* AirportFTA fields. */
    fn next(&self, node: *const c_void) -> *const c_void {
        Self::call(|| unsafe { (self.leaves.fta_next)(node) })
    }
    fn blocks(&self, node: *const c_void) -> AirportBlocks {
        Self::call(|| unsafe { (self.leaves.fta_blocks)(node) })
    }
    fn position(&self, node: *const c_void) -> u8 {
        Self::call(|| unsafe { (self.leaves.fta_position)(node) })
    }
    fn next_position(&self, node: *const c_void) -> u8 {
        Self::call(|| unsafe { (self.leaves.fta_next_position)(node) })
    }
    fn heading(&self, node: *const c_void) -> u8 {
        Self::call(|| unsafe { (self.leaves.fta_heading)(node) })
    }

    /* Vehicle services. */
    fn order_type(&self, v: Vehicle) -> OrderType {
        OrderType(Self::call(|| (self.vs.order_type)(v.base())))
    }
    /// `v->current_order.IsType(order_type)`.
    fn current_order_is_type(&self, v: Vehicle, order_type: OrderType) -> bool {
        self.order_type(v) == order_type
    }
    /// `v->current_order.GetDestination()`.
    fn order_destination(&self, v: Vehicle) -> StationID {
        StationID(Self::call(|| (self.vs.order_destination)(v.base())))
    }
    fn plane_speed(&self) -> u8 {
        Self::call(|| (self.ss.plane_speed)())
    }
    /// `v->NeedsAutomaticServicing()`.
    fn needs_automatic_servicing(&self, v: Vehicle) -> bool {
        // SAFETY: A live vehicle and its immutable maintenance table.
        unsafe { maintenance::needs_automatic_servicing(v.base(), self.vs) }
    }
    /// `VehicleServiceInDepot(v)`.
    fn service_in_depot(&self, v: Vehicle) {
        // SAFETY: A live vehicle and its immutable maintenance table.
        unsafe { maintenance::service_in_depot(v.base(), self.vs) };
    }
    /// `SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP)`.
    fn start_stop_dirty(&self, v: Vehicle) {
        Self::call(|| (self.vs.start_stop_dirty)(v.base()));
    }
    /// `PlayVehicleSound(v, event)`.
    fn vehicle_sound(&self, v: Vehicle, event: crate::newgrf_sound::VehicleSoundEvent) -> bool {
        Self::call(|| (self.vs.play_vehicle_sound)(v.base(), event))
    }
    /// `SndPlayVehicleFx(sound, v)`.
    fn snd_play_vehicle_fx(&self, sound: SoundID, v: Vehicle) {
        Self::call(|| (self.vs.play_sound)(v.base(), sound.0));
    }
    /// `AircraftVehInfo(v->engine_type)->sfx`.
    fn engine_sfx(&self, v: Vehicle) -> SoundID {
        SoundID(Self::call(|| (self.vs.engine)(v.base())).aircraft_sound())
    }
    /// `AircraftVehInfo(v->engine_type)->subtype`.
    fn engine_subtype(&self, v: Vehicle) -> u8 {
        Self::call(|| (self.vs.engine)(v.base())).aircraft_subtype()
    }
    /// `GetVehicleProperty(v, property, fallback)`.
    fn get_vehicle_property(&self, v: VehicleRef, property: u8, fallback: i32) -> i32 {
        Self::call(|| (self.vs.property)((self.vs.newgrf)(), v, property, fallback))
    }
    /// `GetNewVehiclePos(v)`.
    fn new_vehicle_pos(&self, v: Vehicle) -> crate::vehicle::NewPosition {
        Self::call(|| (self.vs.new_position)(v.base()))
    }
    /// `GetDirectionTowards(v, x, y)`.
    fn get_direction_towards(&self, v: Vehicle, x: i32, y: i32) -> Direction {
        Direction(Self::call(|| unsafe {
            (self.leaves.direction_towards)(v, x, y)
        }))
    }
    fn random(&self) -> u32 {
        Self::call(|| self.ss.random())
    }
    /// `CreateEffectVehicleRel(v, x, y, z, type)`.
    fn create_effect_vehicle_rel(v: Vehicle, x: i32, y: i32, z: i32, kind: EffectVehicleType) {
        crate::effect::create_effect_vehicle_rel(v.base(), x, y, z, kind);
    }
    /// `v->UpdatePosition()` and `v->UpdateViewport(true, false)`.
    fn update_position_and_full_viewport(&self, v: Vehicle) {
        // SAFETY: A live vehicle; the tile hash ends before the viewport callback.
        unsafe { crate::vehicle::occupancy::update_position(self.vs, v.base(), self.dims()) };
        Self::call(|| unsafe { (self.leaves.viewport)(v) });
    }
    /// `u->UpdatePositionAndViewport()`.
    fn update_position_and_viewport(&self, u: VehicleRef) {
        // SAFETY: A live vehicle; the tile hash ends before the viewport callback.
        unsafe { crate::vehicle::occupancy::update_position(self.vs, u, self.dims()) };
        Self::call(|| (self.vs.base_viewport)(u));
    }
    /// `v->UpdateDeltaXY()`.
    fn update_delta_xy(&self, v: Vehicle) {
        Self::call(|| (self.vs.update_delta)(v.base()));
    }

    /// `Aircraft::IsInDepot()`, which is `IsChainInDepot()` for an aircraft.
    fn is_in_depot(&self, v: Vehicle) -> bool {
        let b = v.base();
        self.check(is_primary_vehicle(b.subtype()));
        vehstatus_test(b, VehState::Hidden)
            && crate::station_map::is_hangar_tile(
                TileIndex(b.tile()),
                crate::newgrf_airport::runtime(),
            )
    }

    /// `Command<CMD_SEND_VEHICLE_TO_DEPOT>::Do(DoCommandFlag::Execute, v->index, depot_cmd, {})`
    /// under `Backup<CompanyID> cur_company(_current_company, v->owner)`; true when the
    /// command failed.
    fn send_to_depot(&self, v: Vehicle, depot_cmd: DepotCommandFlags) -> bool {
        // SAFETY: The table names the static depot command services.
        let mass = unsafe { &*self.leaves.mass };
        let base = mass.common.base;
        let finance = base.commands.finance;
        let old = Self::call(|| (finance.current_company)());
        Self::call(|| (finance.set_current_company)(v.base().owner()));
        // SAFETY: Both native results live on this stack for the synchronous command.
        let failed = unsafe {
            crate::command::with_cost(base.cost, |out| {
                crate::command::with_cost(base.cost, |item| {
                    crate::command::nested(
                        base.commands,
                        base.shared,
                        out,
                        0,
                        crate::command_type::DoCommandFlag::EXECUTE,
                        |flags| {
                            crate::vehicle_transactions::mass::cmd_send_vehicle_to_depot(
                                mass,
                                out.0,
                                item.0,
                                crate::command_type::DoCommandFlags(flags),
                                crate::vehicle_transactions::mass::CmdSendVehicleToDepot {
                                    veh_id: v.index(),
                                    depot_cmd,
                                    vli: crate::vehiclelist::VehicleListIdentifier {
                                        list_type: crate::vehiclelist::VehicleListType(0),
                                        vtype: VehicleType(0),
                                        company: crate::company_type::CompanyID(0),
                                        index: 0,
                                    },
                                },
                            );
                        },
                    );
                    !(base.cost.success)(out.0)
                })
            })
        };
        Self::call(|| (finance.set_current_company)(old));
        failed
    }

    /// Original `FindNearestHangar`: find the nearest hangar to v.
    /// `StationID::Invalid()` is returned, if the company does not have any suitable
    /// airports (like helipads only)
    #[allow(
        clippy::similar_names,
        reason = "FindNearestHangar's last_dest, next_dest, last_dist and next_dist"
    )]
    fn find_nearest_hangar(&self, v: Vehicle) -> StationID {
        let b = v.base();
        let mut best: u32 = 0;
        let mut index = StationID::INVALID;
        let vtile = self.map.tile_virt_xy_clamped_to_map(b.x(), b.y());
        let avi_subtype = self.engine_subtype(v);
        let max_range = v.cached_max_range_sqr();

        /* Determine destinations where it's coming from and where it's heading to */
        let mut last_dest: *const Station = std::ptr::null();
        let mut next_dest: *const Station = std::ptr::null();
        if max_range != 0 {
            if self.current_order_is_type(v, OrderType::OT_GOTO_STATION)
                || (self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
                    && b.current_order().depot_action_flags() & ODATF_NEAREST_DEPOT == 0)
            {
                last_dest = self.station_get_if_valid(StationID(b.last_station()));
                next_dest = self.station_get_if_valid(self.order_destination(v));
            } else {
                last_dest = self.get_target_airport_if_valid(v);
                let mut next_station = None;
                crate::orders::get_next_stopping_station(b, self.vs.orders(), &mut |station| {
                    next_station = Some(station);
                });
                if let Some(station) = next_station {
                    next_dest = self.station_get_if_valid(station);
                }
            }
        }

        let no_jetcrash = Self::call(|| (self.ss.no_jetcrash)());
        let owner = b.owner();
        self.ss.for_each_station(|st, _, station_index| {
            if self.station_owner(st).0 != owner
                || !Self::station_has_airport(st)
                || !Self::airport_has_hangar(st)
            {
                return;
            }

            let afc = Self::airport_get_fta(st);
            let flags = self.fta_flags(afc);

            /* don't crash the plane if we know it can't land at the airport */
            if Self::fta_flags_test(flags, AirportFTAClassFlag::SHORT_STRIP)
                && avi_subtype & AIR_FAST != 0
                && !no_jetcrash
            {
                return;
            }

            /* the plane won't land at any helicopter station */
            if !Self::fta_flags_test(flags, AirportFTAClassFlag::AIRPLANES)
                && avi_subtype & AIR_CTOL != 0
            {
                return;
            }

            /* Check if our last and next destinations can be reached from the depot airport. */
            if max_range != 0 {
                let last_dist =
                    if !last_dest.is_null() && self.airport_tile(last_dest) != TileIndex::INVALID {
                        self.dims()
                            .distance_square(self.airport_tile(st), self.airport_tile(last_dest))
                    } else {
                        0
                    };
                let next_dist =
                    if !next_dest.is_null() && self.airport_tile(next_dest) != TileIndex::INVALID {
                        self.dims()
                            .distance_square(self.airport_tile(st), self.airport_tile(next_dest))
                    } else {
                        0
                    };
                if last_dist > max_range || next_dist > max_range {
                    return;
                }
            }

            /* v->tile can't be used here, when aircraft is flying v->tile is set to 0 */
            let distance = self.dims().distance_square(vtile, self.airport_tile(st));
            if distance < best || index == StationID::INVALID {
                best = distance;
                index = StationID(station_index);
            }
        });
        index
    }

    /// Original `Aircraft::FindClosestDepot`: the station of the closest hangar, which the
    /// shim turns into a `ClosestDepot`.
    fn find_closest_depot(&self, v: Vehicle) -> StationID {
        let st = self.get_target_airport_if_valid(v);
        /* If the station is not a valid airport or if it has no hangars */
        if st.is_null() || !Self::can_vehicle_use_station(v, st) || !Self::airport_has_hangar(st) {
            /* the aircraft has to search for a hangar on its own */
            return self.find_nearest_hangar(v);
        }
        v.targetairport()
    }

    /// Original `CheckIfAircraftNeedsService`.
    fn check_if_aircraft_needs_service(&self, v: Vehicle) {
        if Self::call(|| unsafe { (self.leaves.service_interval)(v) }) == 0
            || !self.needs_automatic_servicing(v)
        {
            return;
        }
        if self.is_in_depot(v) {
            self.service_in_depot(v);
            return;
        }

        /* When we're parsing conditional orders and the like
         * we don't want to consider going to a depot too. */
        if !self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
            && !self.current_order_is_type(v, OrderType::OT_GOTO_STATION)
        {
            return;
        }

        let destination = self.order_destination(v);
        let st = Self::station_get(destination);

        self.check(!st.is_null());

        /* only goto depot if the target airport has a depot */
        if Self::airport_has_hangar(st) && Self::can_vehicle_use_station(v, st) {
            Self::call(|| unsafe { (self.leaves.service_order)(v, destination.0) });
            self.start_stop_dirty(v);
        } else if self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT) {
            Self::call(|| (self.vs.make_dummy)(v.base()));
            self.start_stop_dirty(v);
        }
    }

    /// Original `Aircraft::OnNewCalendarDay`: calendar day handler.
    fn on_new_calendar_day(&self, v: Vehicle) {
        if !is_normal_aircraft(v.base().subtype()) {
            return;
        }
        // SAFETY: A live vehicle and its immutable maintenance table.
        unsafe { maintenance::age_vehicle(v.base(), self.vs) };
    }

    /// Original `Aircraft::OnNewEconomyDay`: economy day handler.
    #[allow(
        clippy::verbose_bit_mask,
        reason = "OnNewEconomyDay's (++day_counter & 7) == 0"
    )]
    fn on_new_economy_day(&self, v: Vehicle) {
        let b = v.base();
        if !is_normal_aircraft(b.subtype()) {
            return;
        }
        // SAFETY: A live vehicle and its immutable maintenance table.
        unsafe { maintenance::economy_age_vehicle(b, self.vs) };

        let day_counter = b.day().wrapping_add(1);
        b.write_day(day_counter);
        if day_counter & 7 == 0 {
            // SAFETY: As above.
            unsafe { maintenance::decrease_vehicle_value(b, self.vs) };
        }

        Self::call(|| (self.vs.check_orders_only)(b));

        // SAFETY: As above.
        unsafe { maintenance::check_vehicle_breakdown(b, self.vs) };
        self.check_if_aircraft_needs_service(v);

        if b.running() == 0 {
            return;
        }

        // SAFETY: A live vehicle and its static NewGRF tables.
        let running_cost = Self::call(|| unsafe { self.vs.running_cost(b) });
        /* Money's OverflowSafeInt multiplication saturates; the division does not. */
        let cost = running_cost.saturating_mul(i64::from(b.running())) / YEAR_TICKS;

        let profit = Self::call(|| (self.vs.profit)(b));
        Self::call(|| (self.vs.write_profit)(b, profit.saturating_sub(cost)));
        b.write_running(0);

        Self::call(|| unsafe { (self.leaves.subtract_cost)(v, cost) });

        Self::call(|| unsafe { (self.leaves.dirty_lists)(v) });
    }

    /// Original `HelicopterTickHandler`.
    fn helicopter_tick_handler(&self, v: Vehicle) {
        let u = next_part(next_part(v.base()));

        if vehstatus_test(u, VehState::Hidden) {
            return;
        }

        /* if true, helicopter rotors do not rotate. This should only be the case if a helicopter is
         * loading/unloading at a terminal or stopped */
        if self.current_order_is_type(v, OrderType::OT_LOADING)
            || vehstatus_test(v.base(), VehState::Stopped)
        {
            if u.speed() != 0 {
                u.write_speed(u.speed().wrapping_add(1));
                if u.speed() >= 0x80 && rotor_state(u) == HelicopterRotorStates::HRS_ROTOR_MOVING_3
                {
                    u.write_speed(0);
                }
            }
        } else {
            if u.speed() == 0 {
                u.write_speed(0x70);
            }
            if u.speed() >= 0x50 {
                u.write_speed(u.speed() - 1);
            }
        }

        let tick = u.tick().wrapping_add(1);
        u.write_tick(tick);
        let tick = i32::from(tick);
        let spd = i32::from(u.speed() >> 4);

        if spd == 0 {
            set_rotor_state(u, HelicopterRotorStates::HRS_ROTOR_STOPPED);
            if Self::call(|| unsafe { (self.leaves.rotor_image_if_changed)(v) }) {
                return;
            }
        } else if tick >= spd {
            u.write_tick(0);
            set_rotor_state(u, HelicopterRotorStates(rotor_state(u).0.wrapping_add(1)));
            if rotor_state(u) > HelicopterRotorStates::HRS_ROTOR_MOVING_3 {
                set_rotor_state(u, HelicopterRotorStates::HRS_ROTOR_MOVING_1);
            }
            Self::call(|| unsafe { (self.leaves.update_rotor_image)(v) });
        } else {
            return;
        }

        self.update_position_and_viewport(u);
    }

    /// Original `SetAircraftPosition`: set aircraft position.
    #[allow(
        clippy::many_single_char_names,
        reason = "SetAircraftPosition's v, u, x, y and z"
    )]
    fn set_aircraft_position(&self, v: Vehicle, x: i32, y: i32, z: i32) {
        let b = v.base();
        b.write_coordinates(x, y, z);

        self.update_position_and_full_viewport(v);
        if b.subtype() == AIR_HELICOPTER {
            Self::call(|| unsafe { (self.leaves.rotor_image)(v) });
        }

        let u = next_part(b);

        let (max_x, max_y) = self.map_max_xy();
        let safe_x = self.clamp(x, 0, max_x);
        let mut safe_y = self.clamp(y.wrapping_sub(1), 0, max_y);
        u.write_x(x);
        u.write_y(y.wrapping_sub(b.z().wrapping_sub(self.get_slope_pixel_z(safe_x, safe_y)) >> 3));

        safe_y = self.clamp(u.y(), 0, max_y);
        u.write_z(self.get_slope_pixel_z(safe_x, safe_y));
        Self::call(|| unsafe { (self.leaves.copy_sprite)(u, b) }); // the shadow is never coloured

        self.update_position_and_viewport(u);

        if let Some(u) = u.next() {
            u.write_x(x);
            u.write_y(y);
            u.write_z(z.wrapping_add(ROTOR_Z_OFFSET));

            self.update_position_and_viewport(u);
        }
    }

    /// Original `HandleAircraftEnterHangar`: handle Aircraft specific tasks when an Aircraft
    /// enters a hangar.
    fn handle_aircraft_enter_hangar(&self, v: Vehicle) {
        let b = v.base();
        b.write_subspeed(0);
        b.write_progress(0);

        let u = next_part(b);
        vehstatus_assign(u, VehState::Hidden, true);
        if let Some(u) = u.next() {
            vehstatus_assign(u, VehState::Hidden, true);
            u.write_speed(0);
        }

        self.set_aircraft_position(v, b.x(), b.y(), b.z());
    }

    /// Original `PlayAircraftSound`.
    fn play_aircraft_sound(&self, v: Vehicle) {
        if !self.vehicle_sound(v, VSE_START) {
            self.snd_play_vehicle_fx(self.engine_sfx(v), v);
        }
    }

    /// Original `UpdateAircraftCache`: update cached values of an aircraft.
    /// Currently caches callback 36 max speed.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "UpdateAircraftCache stores its uint results in the uint16_t cached_max_speed, cached_cargo_age_period and cached_max_range"
    )]
    fn update_aircraft_cache(&self, v: Vehicle, update_range: bool) {
        let b = v.base();
        let mut max_speed = self
            .get_vehicle_property(b, PROP_AIRCRAFT_SPEED, 0)
            .cast_unsigned();
        if max_speed != 0 {
            /* Convert from original units to km-ish/h */
            max_speed = max_speed.wrapping_mul(128) / 10;

            b.write_max_speed(max_speed as u16);
        } else {
            /* Use the default max speed of the vehicle. */
            b.write_max_speed(Self::call(|| (self.vs.engine)(b)).aircraft_speed());
        }

        /* Update cargo aging period. */
        let cargo_age = Self::call(|| (self.vs.engine)(b)).cargo_age();
        b.write_cargo_age(self.get_vehicle_property(
            b,
            PROP_AIRCRAFT_CARGO_AGE_PERIOD,
            i32::from(cargo_age),
        ) as u16);
        let u = next_part(b); // Shadow for mail
        let cargo_age = Self::call(|| (self.vs.engine)(u)).cargo_age();
        u.write_cargo_age(self.get_vehicle_property(
            u,
            PROP_AIRCRAFT_CARGO_AGE_PERIOD,
            i32::from(cargo_age),
        ) as u16);

        /* Update aircraft range. */
        if update_range {
            let max_range = Self::call(|| (self.vs.engine)(b)).aircraft_range();
            v.set_cached_max_range(self.get_vehicle_property(
                b,
                PROP_AIRCRAFT_RANGE,
                i32::from(max_range),
            ) as u16);
            /* Squared it now so we don't have to do it later all the time. */
            let range = u32::from(v.cached_max_range());
            v.set_cached_max_range_sqr(range.wrapping_mul(range));
        }
    }

    /// Original `UpdateAircraftSpeed`: sets the new speed for an aircraft. Returns the
    /// number of position updates needed within the tick.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "UpdateAircraftSpeed's (uint8_t)spd, its uint16_t cur_speed store and its (uint8_t) progress store"
    )]
    fn update_aircraft_speed(&self, v: Vehicle, mut speed_limit: u32, mut hard_limit: bool) -> i32 {
        let b = v.base();
        /*
         * 'acceleration' has the unit 3/8 mph/tick. This function is called twice per tick.
         * So the speed amount we need to accelerate is:
         *     acceleration * 3 / 16 mph = acceleration * 3 / 16 * 16 / 10 km-ish/h
         *                               = acceleration * 3 / 10 * 256 * (km-ish/h / 256)
         *                               ~ acceleration * 77 (km-ish/h / 256)
         */
        let mut spd = u32::from(b.acceleration()).wrapping_mul(77);
        let plane_speed = self.plane_speed();

        /* Adjust speed limits by plane speed factor to prevent taxiing
         * and take-off speeds being too low. */
        speed_limit = speed_limit.wrapping_mul(u32::from(plane_speed));

        /* adjust speed for broken vehicles */
        if vehstatus_test(b, VehState::AircraftBroken) {
            if u32::from(SPEED_LIMIT_BROKEN) < speed_limit {
                hard_limit = false;
            }
            speed_limit = speed_limit.min(u32::from(SPEED_LIMIT_BROKEN));
        }

        if u32::from(b.max_speed()) < speed_limit {
            if u32::from(b.speed()) < speed_limit {
                hard_limit = false;
            }
            speed_limit = u32::from(b.max_speed());
        }

        let t = b.subspeed();
        b.write_subspeed(t.wrapping_add(spd as u8));

        /* Aircraft's current speed is used twice so that very fast planes are
         * forced to slow down rapidly in the short distance needed. The magic
         * value 16384 was determined to give similar results to the old speed/48
         * method at slower speeds. This also results in less reduction at slow
         * speeds to that aircraft do not get to taxi speed straight after
         * touchdown. */
        let cur_speed = i32::from(b.speed());
        if !hard_limit && cur_speed.cast_unsigned() > speed_limit {
            speed_limit = cur_speed
                .wrapping_sub(
                    1.max((cur_speed.wrapping_mul(cur_speed) / 16384) / i32::from(plane_speed)),
                )
                .cast_unsigned();
        }

        spd = (u32::from(b.speed()) + (spd >> 8) + u32::from(b.subspeed() < t)).min(speed_limit);

        /* updates statusbar only if speed have changed to save CPU time */
        if spd != u32::from(b.speed()) {
            b.write_speed(spd as u16);
            self.start_stop_dirty(v);
        }

        /* Adjust distance moved by plane speed setting */
        if plane_speed > 1 {
            spd /= u32::from(plane_speed);
        }

        /* Convert direction-independent speed into direction-dependent speed. (old movement method) */
        spd = get_old_advance_speed(b, spd);

        spd = spd.wrapping_add(u32::from(b.progress()));
        b.write_progress(spd as u8);
        (spd >> 8).cast_signed()
    }

    /// Original `GetTileHeightBelowAircraft`: get the tile height below the aircraft.
    /// This function is needed because aircraft can leave the mapborders.
    fn get_tile_height_below_aircraft(&self, v: VehicleRef) -> i32 {
        let (max_x, max_y) = self.map_max_xy();
        let safe_x = self.clamp(v.x(), 0, max_x);
        let safe_y = self.clamp(v.y(), 0, max_y);
        crate::tile_map::tile_height_in(&self.map, self.tile_virt_xy(safe_x, safe_y))
            .wrapping_mul(TILE_HEIGHT)
            .cast_signed()
    }

    /// Original `GetAircraftFlightLevelBounds`: get the 'flight level' bounds, in pixels
    /// from `z_pos` 0 for a particular vehicle for normal flight situation.
    /// When the maximum is reached the vehicle should consider descending.
    /// When the minimum is reached the vehicle should consider ascending.
    fn get_aircraft_flight_level_bounds(
        &self,
        v: VehicleRef,
        min_level: Option<&mut i32>,
        max_level: Option<&mut i32>,
    ) {
        let mut base_altitude = self.get_tile_height_below_aircraft(v);
        if VehicleType(v.vehicle_type()) == VehicleType::VEH_AIRCRAFT
            && v.subtype() == AIR_HELICOPTER
        {
            base_altitude += HELICOPTER_HOLD_MAX_FLYING_ALTITUDE - PLANE_HOLD_MAX_FLYING_ALTITUDE;
        }

        /* Make sure eastbound and westbound planes do not "crash" into each
         * other by providing them with vertical separation
         */
        match Direction(v.direction()) {
            Direction::DIR_N | Direction::DIR_NE | Direction::DIR_E | Direction::DIR_SE => {
                base_altitude += 10;
            }
            _ => {}
        }

        /* Make faster planes fly higher so that they can overtake slower ones */
        base_altitude += (20 * (i32::from(v.max_speed()) / 200) - 90).min(0);

        if let Some(min_level) = min_level {
            *min_level = base_altitude + AIRCRAFT_MIN_FLYING_ALTITUDE;
        }
        if let Some(max_level) = max_level {
            *max_level = base_altitude + AIRCRAFT_MAX_FLYING_ALTITUDE;
        }
    }

    /// Original `GetAircraftHoldMaxAltitude`: gets the maximum 'flight level' for the
    /// holding pattern of the aircraft, in pixels `z_pos` 0, depending on terrain below.
    fn get_aircraft_hold_max_altitude(&self, v: VehicleRef) -> i32 {
        let tile_height = self.get_tile_height_below_aircraft(v);

        tile_height
            + if v.subtype() == AIR_HELICOPTER {
                HELICOPTER_HOLD_MAX_FLYING_ALTITUDE
            } else {
                PLANE_HOLD_MAX_FLYING_ALTITUDE
            }
    }

    /// Original `GetAircraftFlightLevel<T>`, over the flags of an aircraft or a disaster
    /// vehicle.
    #[allow(
        clippy::manual_midpoint,
        reason = "GetAircraftFlightLevel's int (min + max) / 2 truncates toward zero"
    )]
    fn get_aircraft_flight_level(&self, v: VehicleRef, flags: *mut u8, takeoff: bool) -> i32 {
        /* Aircraft is in flight. We want to enforce it being somewhere
         * between the minimum and the maximum allowed altitude. */
        let mut aircraft_min_altitude = 0;
        let mut aircraft_max_altitude = 0;
        self.get_aircraft_flight_level_bounds(
            v,
            Some(&mut aircraft_min_altitude),
            Some(&mut aircraft_max_altitude),
        );
        let aircraft_middle_altitude = (aircraft_min_altitude + aircraft_max_altitude) / 2;

        /* If those assumptions would be violated, aircraft would behave fairly strange. */
        self.check(aircraft_min_altitude < aircraft_middle_altitude);
        self.check(aircraft_middle_altitude < aircraft_max_altitude);

        // SAFETY: The flags belong to the live aircraft or disaster owner; no callback
        // runs between this load and the store below.
        let mut f = unsafe { flags.read() };
        let mut z = v.z();
        if z < aircraft_min_altitude
            || (has_bit(f, AirVehicleFlags::VAF_IN_MIN_HEIGHT_CORRECTION.0)
                && z < aircraft_middle_altitude)
        {
            /* Ascend. And don't fly into that mountain right ahead.
             * And avoid our aircraft become a stairclimber, so if we start
             * correcting altitude, then we stop correction not too early. */
            set_bit8(&mut f, AirVehicleFlags::VAF_IN_MIN_HEIGHT_CORRECTION.0);
            z += if takeoff { 2 } else { 1 };
        } else if !takeoff
            && (z > aircraft_max_altitude
                || (has_bit(f, AirVehicleFlags::VAF_IN_MAX_HEIGHT_CORRECTION.0)
                    && z > aircraft_middle_altitude))
        {
            /* Descend lower. You are an aircraft, not an space ship.
             * And again, don't stop correcting altitude too early. */
            set_bit8(&mut f, AirVehicleFlags::VAF_IN_MAX_HEIGHT_CORRECTION.0);
            z -= 1;
        } else if has_bit(f, AirVehicleFlags::VAF_IN_MIN_HEIGHT_CORRECTION.0)
            && z >= aircraft_middle_altitude
        {
            /* Now, we have corrected altitude enough. */
            clr_bit8(&mut f, AirVehicleFlags::VAF_IN_MIN_HEIGHT_CORRECTION.0);
        } else if has_bit(f, AirVehicleFlags::VAF_IN_MAX_HEIGHT_CORRECTION.0)
            && z <= aircraft_middle_altitude
        {
            /* Now, we have corrected altitude enough. */
            clr_bit8(&mut f, AirVehicleFlags::VAF_IN_MAX_HEIGHT_CORRECTION.0);
        }
        // SAFETY: As above.
        unsafe { flags.write(f) };

        z
    }
    /// `GetAircraftFlightLevel(v, takeoff)` of an aircraft.
    fn aircraft_flight_level(&self, v: Vehicle, takeoff: bool) -> i32 {
        // SAFETY: Computing a raw field address creates no reference.
        self.get_aircraft_flight_level(v.base(), unsafe { &raw mut (*v.state).flags }, takeoff)
    }

    /// Original `AircraftGetEntryPoint`: find the entry point to an airport depending on
    /// direction which the airport is being approached from. Each airport can have up to
    /// four entry points for its approach system so that approaching aircraft do not fly
    /// through each other or are forced to do 180 degree turns during the approach. The
    /// arrivals are grouped into four sectors dependent on the `DiagDirection` from which
    /// the airport is approached.
    fn aircraft_get_entry_point(&self, v: Vehicle, apc: *const c_void, rotation: Direction) -> u8 {
        /* assert(v != nullptr) holds by the type of v. */
        self.check(!apc.is_null());

        /* In the case the station doesn't exit anymore, set target tile 0.
         * It doesn't hurt much, aircraft will go to next order, nearest hangar
         * or it will simply crash in next tick */
        let mut tile = TileIndex(0);

        let st = self.station_get_if_valid(v.targetairport());
        if !st.is_null() {
            /* Make sure we don't go to INVALID_TILE if the airport has been removed. */
            let airport_tile = self.airport_tile(st);
            tile = if airport_tile == TileIndex::INVALID {
                self.station_xy(st)
            } else {
                airport_tile
            };
        }

        let b = v.base();
        let delta_x = b.x().wrapping_sub(
            self.dims()
                .tile_x(tile)
                .wrapping_mul(TILE_SIZE)
                .cast_signed(),
        );
        let delta_y = b.y().wrapping_sub(
            self.dims()
                .tile_y(tile)
                .wrapping_mul(TILE_SIZE)
                .cast_signed(),
        );

        let mut dir = if delta_y.wrapping_abs() < delta_x.wrapping_abs() {
            /* We are northeast or southwest of the airport */
            if delta_x < 0 {
                DiagDirection::DIAGDIR_NE
            } else {
                DiagDirection::DIAGDIR_SW
            }
        } else {
            /* We are northwest or southeast of the airport */
            if delta_y < 0 {
                DiagDirection::DIAGDIR_NW
            } else {
                DiagDirection::DIAGDIR_SE
            }
        };
        let check = |c| self.check(c);
        dir = change_diag_dir(
            dir,
            diag_dir_difference(
                DiagDirection::DIAGDIR_NE,
                dir_to_diag_dir(rotation, check),
                check,
            ),
            check,
        );
        self.entry_points(apc, dir)
    }

    /// Original `AircraftController`: controls the movement of an aircraft. This function
    /// actually moves the vehicle on the map and takes care of minor things like sound
    /// playback. Returns whether the position requested by the State Machine has been
    /// reached.
    #[allow(
        clippy::too_many_lines,
        reason = "the one original AircraftController body"
    )]
    #[allow(
        clippy::many_single_char_names,
        reason = "AircraftController's v, u, x, y, z and t"
    )]
    #[allow(clippy::if_not_else, reason = "AircraftController's branch order")]
    fn aircraft_controller(&self, v: Vehicle) -> bool {
        let b = v.base();
        /* nullptr if station is invalid */
        let st = self.station_get_if_valid(v.targetairport());
        /* INVALID_TILE if there is no station */
        let mut tile = TileIndex::INVALID;
        let mut rotation = Direction::DIR_N;
        let mut size_x: u32 = 1;
        let mut size_y: u32 = 1;
        /* st->airport.tile, read once for both of the original's tests below. */
        let mut airport_tile = TileIndex::INVALID;
        if !st.is_null() {
            airport_tile = self.airport_tile(st);
            if airport_tile != TileIndex::INVALID {
                tile = airport_tile;
                rotation = self.airport_rotation(st);
                size_x = u32::from(self.airport_w(st));
                size_y = u32::from(self.airport_h(st));
            } else {
                tile = self.station_xy(st);
            }
        }
        /* DUMMY if there is no station or no airport */
        let afc = if tile == TileIndex::INVALID {
            crate::airport::get_airport(AT_DUMMY, crate::newgrf_airport::runtime())
        } else {
            Self::airport_get_fta(st)
        };

        /* prevent going to INVALID_TILE if airport is deleted. */
        if st.is_null() || airport_tile == TileIndex::INVALID {
            /* Jump into our "holding pattern" state machine if possible */
            if v.pos() >= self.nofelements(afc) {
                let pos = self.aircraft_get_entry_point(v, afc, Direction::DIR_N);
                v.set_previous_pos(pos);
                v.set_pos(pos);
            } else if v.targetairport() != self.order_destination(v) {
                /* If not possible, just get out of here fast */
                v.set_state(FLYING);
                self.update_aircraft_cache(v, false);
                self.aircraft_next_airport_pos_and_order(v);
                /* get aircraft back on running altitude */
                let z = self.aircraft_flight_level(v, false);
                self.set_aircraft_position(v, b.x(), b.y(), z);
                return false;
            }
        }

        /*  get airport moving data */
        let amd = crate::airport::rotate_airport_moving_data(
            self.moving_data(afc, v.pos()),
            rotation,
            size_x,
            size_y,
            |c| self.check(c),
        );

        let x = self
            .dims()
            .tile_x(tile)
            .wrapping_mul(TILE_SIZE)
            .cast_signed();
        let y = self
            .dims()
            .tile_y(tile)
            .wrapping_mul(TILE_SIZE)
            .cast_signed();

        /* Helicopter raise */
        if amd.flags.test(AirportMovingDataFlag::HELI_RAISE) {
            let u = next_part(next_part(b));

            /* Make sure the rotors don't rotate too fast */
            if u.speed() > 32 {
                b.write_speed(0);
                u.write_speed(u.speed() - 1);
                if u.speed() == 32 && !self.vehicle_sound(v, VSE_START) {
                    let mut sfx = self.engine_sfx(v);
                    /* For compatibility with old NewGRF we ignore the sfx property, unless a NewGRF-defined sound is used.
                     * The baseset has only one helicopter sound, so this only limits using plane or cow sounds. */
                    if u32::from(sfx.0) < ORIGINAL_SAMPLE_COUNT {
                        sfx = SoundID::from(SoundFx::SND_18_TAKEOFF_HELICOPTER);
                    }
                    self.snd_play_vehicle_fx(sfx, v);
                }
            } else {
                u.write_speed(32);
                let count = self.update_aircraft_speed(v, u32::from(SPEED_LIMIT_NONE), true);
                if count > 0 {
                    b.write_tile(0);

                    let mut z_dest = 0;
                    self.get_aircraft_flight_level_bounds(b, Some(&mut z_dest), None);

                    /* Reached altitude? */
                    if b.z() >= z_dest {
                        b.write_speed(0);
                        return true;
                    }
                    self.set_aircraft_position(v, b.x(), b.y(), (b.z() + count).min(z_dest));
                }
            }
            return false;
        }

        /* Helicopter landing. */
        if amd.flags.test(AirportMovingDataFlag::HELI_LOWER) {
            let mut flags = v.flags();
            set_bit8(&mut flags, AirVehicleFlags::VAF_HELI_DIRECT_DESCENT.0);
            v.set_flags(flags);

            if st.is_null() {
                v.set_state(FLYING);
                self.update_aircraft_cache(v, false);
                self.aircraft_next_airport_pos_and_order(v);
                return false;
            }

            /* Vehicle is now at the airport.
             * Helicopter has arrived at the target landing pad, so the current position is also where it should land.
             * Except for Oilrigs which are special due to being a 1x1 station, and helicopters land outside it. */
            let (mut x, mut y) = (x, y);
            if self.airport_type(st) != AT_OILRIG {
                x = b.x();
                y = b.y();
                tile = self.tile_virt_xy(x, y);
            }
            b.write_tile(tile.0);

            /* Find altitude of landing position. */
            let z = self.get_slope_pixel_z(x, y) + 1 + i32::from(self.delta_z(afc));

            if z == b.z() {
                let u = next_part(next_part(b));

                /*  Increase speed of rotors. When speed is 80, we've landed. */
                if u.speed() >= 80 {
                    let mut flags = v.flags();
                    clr_bit8(&mut flags, AirVehicleFlags::VAF_HELI_DIRECT_DESCENT.0);
                    v.set_flags(flags);
                    return true;
                }
                u.write_speed(u.speed().wrapping_add(4));
            } else {
                let count = self.update_aircraft_speed(v, u32::from(SPEED_LIMIT_NONE), true);
                if count > 0 {
                    if b.z() > z {
                        self.set_aircraft_position(v, b.x(), b.y(), (b.z() - count).max(z));
                    } else {
                        self.set_aircraft_position(v, b.x(), b.y(), (b.z() + count).min(z));
                    }
                }
            }
            return false;
        }

        /* Get distance from destination pos to current pos. */
        let dist = ((x + i32::from(amd.x) - b.x()).abs() + (y + i32::from(amd.y) - b.y()).abs())
            .cast_unsigned();

        /* Need exact position? */
        if !amd.flags.test(AirportMovingDataFlag::EXACT_POSITION)
            && dist
                <= if amd.flags.test(AirportMovingDataFlag::SLOW_TURN) {
                    8
                } else {
                    4
                }
        {
            return true;
        }

        let check = |c| self.check(c);

        /* At final pos? */
        if dist == 0 {
            /* Change direction smoothly to final direction. */
            let dirdiff = dir_difference(amd.direction, Direction(b.direction()), check);
            /* if distance is 0, and plane points in right direction, no point in calling
             * UpdateAircraftSpeed(). So do it only afterwards */
            if dirdiff == DirDiff::DIRDIFF_SAME {
                b.write_speed(0);
                return true;
            }

            if self.update_aircraft_speed(v, u32::from(SPEED_LIMIT_TAXI), true) == 0 {
                return false;
            }

            b.write_direction(
                change_dir(
                    Direction(b.direction()),
                    if dirdiff > DirDiff::DIRDIFF_REVERSE {
                        DirDiff::DIRDIFF_45LEFT
                    } else {
                        DirDiff::DIRDIFF_45RIGHT
                    },
                    check,
                )
                .0,
            );
            b.write_speed(b.speed() >> 1);

            self.set_aircraft_position(v, b.x(), b.y(), b.z());
            return false;
        }

        if amd.flags.test(AirportMovingDataFlag::BRAKE)
            && i32::from(b.speed()) > i32::from(SPEED_LIMIT_TAXI) * i32::from(self.plane_speed())
        {
            self.maybe_crash_airplane(v);
            if vehstatus_test(b, VehState::Crashed) {
                return false;
            }
        }

        let mut speed_limit = u32::from(SPEED_LIMIT_TAXI);
        let mut hard_limit = true;

        if amd.flags.test(AirportMovingDataFlag::NO_SPEED_CLAMP) {
            speed_limit = u32::from(SPEED_LIMIT_NONE);
        }
        if amd.flags.test(AirportMovingDataFlag::HOLD) {
            speed_limit = u32::from(SPEED_LIMIT_HOLD);
            hard_limit = false;
        }
        if amd.flags.test(AirportMovingDataFlag::LAND) {
            speed_limit = u32::from(SPEED_LIMIT_APPROACH);
            hard_limit = false;
        }
        if amd.flags.test(AirportMovingDataFlag::BRAKE) {
            speed_limit = u32::from(SPEED_LIMIT_TAXI);
            hard_limit = false;
        }

        let mut count = self.update_aircraft_speed(v, speed_limit, hard_limit);
        if count == 0 {
            return false;
        }

        /* If the plane will be a few subpixels away from the destination after
         * this movement loop, start nudging it towards the exact position for
         * the whole loop. Otherwise, heavily depending on the speed of the plane,
         * it is possible we totally overshoot the target, causing the plane to
         * make a loop, and trying again, and again, and again .. */
        let nudge_towards_target = count.cast_unsigned().wrapping_add(3) > dist;

        if v.turn_counter() != 0 {
            v.set_turn_counter(v.turn_counter() - 1);
        }

        loop {
            'body: {
                let gp_x;
                let gp_y;
                let gp_new_tile;

                if nudge_towards_target || amd.flags.test(AirportMovingDataFlag::LAND) {
                    /* move vehicle one pixel towards target */
                    let target_x = x + i32::from(amd.x);
                    let target_y = y + i32::from(amd.y);
                    gp_x = if b.x() != target_x {
                        b.x() + if target_x > b.x() { 1 } else { -1 }
                    } else {
                        b.x()
                    };
                    gp_y = if b.y() != target_y {
                        b.y() + if target_y > b.y() { 1 } else { -1 }
                    } else {
                        b.y()
                    };

                    /* Oilrigs must keep v->tile as st->airport.tile, since the landing pad is in a non-airport tile */
                    gp_new_tile = if self.airport_type(st) == AT_OILRIG {
                        self.airport_tile(st)
                    } else {
                        self.tile_virt_xy(gp_x, gp_y)
                    };
                } else {
                    /* Turn. Do it slowly if in the air. */
                    let newdir =
                        self.get_direction_towards(v, x + i32::from(amd.x), y + i32::from(amd.y));
                    if newdir != Direction(b.direction()) {
                        if amd.flags.test(AirportMovingDataFlag::SLOW_TURN)
                            && v.number_consecutive_turns() < 8
                            && b.subtype() == AIR_AIRCRAFT
                        {
                            if v.turn_counter() == 0 || newdir == v.last_direction() {
                                if newdir == v.last_direction() {
                                    v.set_number_consecutive_turns(0);
                                } else {
                                    v.set_number_consecutive_turns(
                                        v.number_consecutive_turns().wrapping_add(1),
                                    );
                                }
                                v.set_turn_counter(2_u8.wrapping_mul(self.plane_speed()));
                                v.set_last_direction(Direction(b.direction()));
                                b.write_direction(newdir.0);
                            }

                            /* Move vehicle. */
                            let gp = self.new_vehicle_pos(v);
                            gp_x = gp.x;
                            gp_y = gp.y;
                            gp_new_tile = TileIndex(gp.tile);
                        } else {
                            b.write_speed(b.speed() >> 1);
                            b.write_direction(newdir.0);

                            /* When leaving a terminal an aircraft often goes to a position
                             * directly in front of it. If it would move while turning it
                             * would need an two extra turns to end up at the correct position.
                             * To make it easier just disallow all moving while turning as
                             * long as an aircraft is on the ground. */
                            gp_x = b.x();
                            gp_y = b.y();
                            gp_new_tile = TileIndex(b.tile());
                        }
                    } else {
                        v.set_number_consecutive_turns(0);
                        /* Move vehicle. */
                        let gp = self.new_vehicle_pos(v);
                        gp_x = gp.x;
                        gp_y = gp.y;
                        gp_new_tile = TileIndex(gp.tile);
                    }
                }

                b.write_tile(gp_new_tile.0);
                /* If vehicle is in the air, use tile coordinate 0. */
                if amd.flags.any(&[
                    AirportMovingDataFlag::TAKEOFF,
                    AirportMovingDataFlag::SLOW_TURN,
                    AirportMovingDataFlag::LAND,
                ]) {
                    b.write_tile(0);
                }

                /* Adjust Z for land or takeoff? */
                let mut z = b.z();

                if amd.flags.test(AirportMovingDataFlag::TAKEOFF) {
                    z = self.aircraft_flight_level(v, true);
                } else if amd.flags.test(AirportMovingDataFlag::HOLD) {
                    /* Let the plane drop from normal flight altitude to holding pattern altitude */
                    if z > self.get_aircraft_hold_max_altitude(b) {
                        z -= 1;
                    }
                } else if amd.flags.all(&[
                    AirportMovingDataFlag::SLOW_TURN,
                    AirportMovingDataFlag::NO_SPEED_CLAMP,
                ]) {
                    z = self.aircraft_flight_level(v, false);
                }

                /* NewGRF airports (like a rotated intercontinental from OpenGFX+Airports) can be non-rectangular
                 * and their primary (north-most) tile does not have to be part of the airport.
                 * As such, the height of the primary tile can be different from the rest of the airport.
                 * Given we are landing/breaking, and as such are not a helicopter, we know that there has to be a hangar.
                 * We also know that the airport itself has to be completely flat (otherwise it is not a valid airport).
                 * Therefore, use the height of this hangar to calculate our z-value. */
                let mut airport_z = b.z();
                if amd
                    .flags
                    .any(&[AirportMovingDataFlag::LAND, AirportMovingDataFlag::BRAKE])
                    && !st.is_null()
                {
                    self.check(Self::airport_has_hangar(st));
                    let runtime = crate::newgrf_airport::runtime();
                    let hangar_tile = TileIndex(Self::call(|| {
                        (runtime.hangar_tile)(st.cast(), 0, Self::airport_spec(st))
                    }));
                    airport_z = self.map.tile_max_pixel_z(hangar_tile) + 1; // To avoid clashing with the shadow
                }

                if amd.flags.test(AirportMovingDataFlag::LAND) {
                    if self.airport_blocks(st).test(AirportBlock::ZEPPELINER) {
                        /* Zeppeliner blocked the runway, abort landing */
                        v.set_state(FLYING);
                        self.update_aircraft_cache(v, false);
                        let z = self.aircraft_flight_level(v, false);
                        self.set_aircraft_position(v, gp_x, gp_y, z);
                        v.set_pos(v.previous_pos());
                        break 'body;
                    }

                    if self.airport_tile(st) == TileIndex::INVALID {
                        /* Airport has been removed, abort the landing procedure */
                        v.set_state(FLYING);
                        self.update_aircraft_cache(v, false);
                        self.aircraft_next_airport_pos_and_order(v);
                        /* get aircraft back on running altitude */
                        let z = self.aircraft_flight_level(v, false);
                        self.set_aircraft_position(v, gp_x, gp_y, z);
                        break 'body;
                    }

                    /* We're not flying below our destination, right? */
                    self.check(airport_z <= z);
                    let t = 1_u32.max(dist.wrapping_sub(4)).cast_signed();
                    let delta = z - airport_z;

                    /* Only start lowering when we're sufficiently close for a 1:1 glide */
                    if delta >= t {
                        z = z
                            .cast_unsigned()
                            .wrapping_sub(crate::math::ceil_div(
                                (z - airport_z).cast_unsigned(),
                                t.cast_unsigned(),
                            ))
                            .cast_signed();
                    }
                    if z < airport_z {
                        z = airport_z;
                    }
                }

                /* We've landed. Decrease speed when we're reaching end of runway. */
                if amd.flags.test(AirportMovingDataFlag::BRAKE) {
                    if z > airport_z {
                        z -= 1;
                    } else if z < airport_z {
                        z += 1;
                    }
                }

                self.set_aircraft_position(v, gp_x, gp_y, z);
            }
            count -= 1;
            if count == 0 {
                break;
            }
        }
        false
    }

    /// Original `HandleCrashedAircraft`: handle crashed aircraft `v`.
    fn handle_crashed_aircraft(&self, v: Vehicle) -> bool {
        let b = v.base();
        v.set_crashed_counter(v.crashed_counter().wrapping_add(3));

        let st = self.get_target_airport_if_valid(v);

        /* make aircraft crash down to the ground */
        if v.crashed_counter() < 500 && st.is_null() && v.crashed_counter().is_multiple_of(3) {
            let (max_x, max_y) = self.map_max_xy();
            let z =
                self.get_slope_pixel_z(self.clamp(b.x(), 0, max_x), self.clamp(b.y(), 0, max_y));
            b.write_z(b.z() - 1);
            if b.z() <= z {
                v.set_crashed_counter(500);
                b.write_z(z + 1);
            } else {
                v.set_crashed_counter(0);
            }
            self.set_aircraft_position(v, b.x(), b.y(), b.z());
        }

        if v.crashed_counter() < 650 {
            let mut r = 0;
            if crate::random_func::chance16r(self.ss, 1, 32, &mut r) {
                const DELTA: [DirDiff; 4] = [
                    DirDiff::DIRDIFF_45LEFT,
                    DirDiff::DIRDIFF_SAME,
                    DirDiff::DIRDIFF_SAME,
                    DirDiff::DIRDIFF_45RIGHT,
                ];

                b.write_direction(
                    change_dir(
                        Direction(b.direction()),
                        DELTA[gb(r, 16, 2) as usize],
                        |c| {
                            self.check(c);
                        },
                    )
                    .0,
                );
                self.set_aircraft_position(v, b.x(), b.y(), b.z());
                r = self.random();
                Self::create_effect_vehicle_rel(
                    v,
                    gb(r, 0, 4).wrapping_sub(4).cast_signed(),
                    gb(r, 4, 4).wrapping_sub(4).cast_signed(),
                    gb(r, 8, 4).cast_signed(),
                    EffectVehicleType::EV_EXPLOSION_SMALL,
                );
            }
        } else if v.crashed_counter() >= 10000 {
            /*  remove rubble of crashed airplane */

            /* clear runway-in on all airports, set by crashing plane
             * small airports use AIRPORT_BUSY, city airports use AirportBlock::RunwayInOut, etc.
             * but they all share the same number */
            if !st.is_null() {
                self.airport_blocks_update(st, |blocks| {
                    blocks.reset(AirportBlocks::from_element(AirportBlock::RUNWAY_IN));
                    blocks.reset(AirportBlocks::from_element(AirportBlock::RUNWAY_IN_OUT)); // commuter airport
                    blocks.reset(AirportBlocks::from_element(AirportBlock::RUNWAY_IN2)); // intercontinental
                });
            }

            Self::call(|| (self.vs.destroy)(b));

            return false;
        }

        true
    }

    /// Original `HandleAircraftSmoke`: handle smoke of broken aircraft. `mode` is whether
    /// this is the non-first call for this vehicle in this tick.
    #[allow(
        clippy::verbose_bit_mask,
        reason = "HandleAircraftSmoke's (tick_counter & 0x0F) == 0"
    )]
    fn handle_aircraft_smoke(v: Vehicle, mode: bool) {
        const SMOKE_POS: [(i8, i8); 8] = [
            (5, 5),
            (6, 0),
            (5, -5),
            (0, -6),
            (-5, -5),
            (-6, 0),
            (-5, 5),
            (0, 6),
        ];

        let b = v.base();
        if !vehstatus_test(b, VehState::AircraftBroken) {
            return;
        }

        /* Stop smoking when landed */
        if b.speed() < 10 {
            vehstatus_assign(b, VehState::AircraftBroken, false);
            b.write_breakdown_counter(0);
            return;
        }

        /* Spawn effect et most once per Tick, i.e. !mode */
        if !mode && b.tick() & 0x0F == 0 {
            let (x, y) = SMOKE_POS[usize::from(b.direction())];
            Self::create_effect_vehicle_rel(
                v,
                i32::from(x),
                i32::from(y),
                2,
                EffectVehicleType::EV_BREAKDOWN_SMOKE_AIRCRAFT,
            );
        }
    }

    /// Original `HandleMissingAircraftOrders`.
    fn handle_missing_aircraft_orders(&self, v: Vehicle) {
        /*
         * We do not have an order. This can be divided into two cases:
         * 1) we are heading to an invalid station. In this case we must
         *    find another airport to go to. If there is nowhere to go,
         *    we will destroy the aircraft as it otherwise will enter
         *    the holding pattern for the first airport, which can cause
         *    the plane to go into an undefined state when building an
         *    airport with the same StationID.
         * 2) we are (still) heading to a (still) valid airport, then we
         *    can continue going there. This can happen when you are
         *    changing the aircraft's orders while in-flight or in for
         *    example a depot. However, when we have a current order to
         *    go to a depot, we have to keep that order so the aircraft
         *    actually stops.
         */
        let st = self.get_target_airport_if_valid(v);
        if st.is_null() {
            /* DepotCommandFlag{} is the flag DepotCommandFlag::Service. */
            let failed = self.send_to_depot(
                v,
                DepotCommandFlags::from_element(DepotCommandFlag::Service),
            );

            if failed {
                self.crash_airplane(v);
            }
        } else if !self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT) {
            Self::call(|| (self.vs.order_free)(v.base()));
        }
    }

    /// Original `Aircraft::GetOrderStationLocation`.
    fn get_order_station_location(&self, v: Vehicle) {
        /* Orders are changed in flight, ensure going to the right station. */
        if v.state() == FLYING {
            self.aircraft_next_airport_pos_and_order(v);
        }

        /* Aircraft do not use dest-tile */
    }

    /// Original `Aircraft::Crash`.
    fn crash(&self, v: Vehicle, flooded: bool) -> u32 {
        let victims =
            Self::call(|| unsafe { (self.leaves.vehicle_crash)(v, flooded) }).wrapping_add(2); // pilots
        v.set_crashed_counter(if flooded { 9000 } else { 0 }); // max 10000, disappear pretty fast when flooded

        victims
    }

    /// Original `CrashAirplane`: bring the aircraft in a crashed state, create the
    /// explosion animation, and create a news item about the crash.
    fn crash_airplane(&self, v: Vehicle) {
        let b = v.base();
        Self::create_effect_vehicle_rel(v, 4, 4, 8, EffectVehicleType::EV_EXPLOSION_LARGE);

        let victims = self.crash(v, false);

        Self::call(|| unsafe { (self.leaves.truncate_cargo)(b) });
        Self::call(|| unsafe { (self.leaves.truncate_cargo)(next_part(b)) });
        let st = self.get_target_airport_if_valid(v);
        let vt = self.map.tile_virt_xy_clamped_to_map(b.x(), b.y());

        let station = if st.is_null() {
            StationID::INVALID
        } else {
            v.targetairport()
        };
        Self::call(|| unsafe { (self.leaves.crash_news)(v, station.0, victims) });

        crate::station::spatial::modify_station_rating_around(
            self.ss,
            vt,
            crate::company_type::CompanyID(b.owner()),
            -160,
            30,
        );
        if Self::call(|| (self.ss.disaster_sound)()) {
            self.snd_play_vehicle_fx(SoundID::from(SoundFx::SND_12_EXPLOSION), v);
        }
    }

    /// Original `MaybeCrashAirplane`: decide whether aircraft `v` should crash.
    fn maybe_crash_airplane(&self, v: Vehicle) {
        let st = Self::station_get(v.targetairport());

        let prob: u32 = if Self::fta_flags_test(
            self.fta_flags(Self::airport_get_fta(st)),
            AirportFTAClassFlag::SHORT_STRIP,
        ) && self.engine_subtype(v) & AIR_FAST != 0
            && !Self::call(|| (self.ss.no_jetcrash)())
        {
            3276
        } else {
            let plane_crashes = Self::call(|| (self.ss.plane_crashes)());
            if plane_crashes == 0 {
                return;
            }
            0x4000_u32.wrapping_shl(u32::from(plane_crashes)) / 1500
        };

        if gb(self.random(), 0, 22) > prob {
            return;
        }

        /* Crash the airplane. Remove all goods stored at the station. */
        for cargo in 0..crate::cargo_type::NUM_CARGO {
            Self::station_call(|| unsafe {
                (self.leaves.landing_rating)(v.targetairport().0, cargo);
            });
        }

        self.crash_airplane(v);
    }

    /// Original `AircraftEntersTerminal`: aircraft arrives at a terminal. If it is the
    /// first aircraft, throw a party. Start loading cargo.
    fn aircraft_enters_terminal(&self, v: Vehicle) {
        if self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT) {
            return;
        }

        /* Station::Get(v->targetairport) runs inside the had_vehicle_of_type services. */
        let st = v.targetairport();
        v.base().write_last_station(st.0);

        let hvot_aircraft = crate::station_type::StationHadVehicleOfType::HVOT_AIRCRAFT.0;
        /* Check if station was ever visited before */
        if Self::station_call(|| (self.ss.station_had_vehicle_of_type)(st.0)) & hvot_aircraft == 0 {
            Self::call(|| (self.ss.station_add_vehicle_of_type)(st.0, hvot_aircraft));
            /* show newsitem of celebrating citizens */
            Self::station_call(|| unsafe { (self.leaves.first_arrival)(v, st.0) });
        }

        crate::vehicle::visits::begin_loading(v.base(), self.vs.visits());
    }

    /// Original `AircraftLandAirplane`: aircraft touched down at the landing strip.
    fn aircraft_land_airplane(&self, v: Vehicle) {
        let st = Self::station_get(v.targetairport());

        let b = v.base();
        let vt = self.tile_virt_xy(b.x(), b.y());

        self.update_delta_xy(v);

        crate::animated_tile::trigger_airport_tile_animation(
            st.cast_mut().cast(),
            vt,
            AirportAnimationTrigger::AirplaneTouchdown,
        );

        if !self.vehicle_sound(v, VSE_TOUCHDOWN) {
            self.snd_play_vehicle_fx(SoundID::from(SoundFx::SND_17_SKID_PLANE), v);
        }
    }

    /// Original `AircraftNextAirportPos_and_Order`: set the right pos when heading to
    /// other airports after takeoff.
    fn aircraft_next_airport_pos_and_order(&self, v: Vehicle) {
        if self.current_order_is_type(v, OrderType::OT_GOTO_STATION)
            || self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
        {
            v.set_targetairport(self.order_destination(v));
        }

        let st = self.get_target_airport_if_valid(v);
        let apc = if st.is_null() {
            crate::airport::get_airport(AT_DUMMY, crate::newgrf_airport::runtime())
        } else {
            Self::airport_get_fta(st)
        };
        let rotation = if st.is_null() {
            Direction::DIR_N
        } else {
            self.airport_rotation(st)
        };
        let pos = self.aircraft_get_entry_point(v, apc, rotation);
        v.set_previous_pos(pos);
        v.set_pos(pos);
    }

    /// Original `AircraftLeaveHangar`: aircraft is about to leave the hangar. This is
    /// called in `AfterLoadGame` for old savegames, so it relies on no data being valid,
    /// especially not on the vehicle being on the ground inside a depot.
    fn aircraft_leave_hangar(&self, v: Vehicle, exit_dir: Direction) {
        let b = v.base();
        b.write_speed(0);
        b.write_subspeed(0);
        b.write_progress(0);
        b.write_direction(exit_dir.0);
        vehstatus_assign(b, VehState::Hidden, false);
        {
            let u = next_part(b);
            vehstatus_assign(u, VehState::Hidden, false);

            /* Rotor blades */
            if let Some(u) = u.next() {
                vehstatus_assign(u, VehState::Hidden, false);
                u.write_speed(80);
            }
        }

        self.service_in_depot(v);
        Self::call(|| (self.vs.leave_unbunching_depot)(b));
        self.set_aircraft_position(v, b.x(), b.y(), b.z());
        Self::call(|| unsafe { (self.leaves.dirty_depot)(v) });
    }

    /* AIRCRAFT MOVEMENT SCHEME */

    /// Original `AircraftEventHandler_EnterTerminal`.
    fn aircraft_event_handler_enter_terminal(&self, v: Vehicle, apc: *const c_void) {
        self.aircraft_enters_terminal(v);
        v.set_state(self.heading(self.layout(apc, v.pos())));
    }

    /// Original `AircraftEventHandler_EnterHangar`: aircraft arrived in an airport hangar.
    fn aircraft_event_handler_enter_hangar(&self, v: Vehicle, apc: *const c_void) {
        Self::call(|| self.vs.enter_depot(v.base()));
        v.set_state(self.heading(self.layout(apc, v.pos())));
    }

    /// Original `AircraftEventHandler_InHangar`: handle aircraft movement/decision making
    /// in an airport hangar.
    fn aircraft_event_handler_in_hangar(&self, v: Vehicle, apc: *const c_void) {
        /* if we just arrived, execute EnterHangar first */
        if v.previous_pos() != v.pos() {
            self.aircraft_event_handler_enter_hangar(v, apc);
            return;
        }

        /* if we were sent to the depot, stay there */
        if self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
            && vehstatus_test(v.base(), VehState::Stopped)
        {
            Self::call(|| (self.vs.order_free)(v.base()));
            return;
        }

        /* Check if we should wait here for unbunching. */
        if Self::call(|| (self.vs.waiting_for_unbunching)(v.base())) {
            return;
        }

        if !self.current_order_is_type(v, OrderType::OT_GOTO_STATION)
            && !self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
        {
            return;
        }

        /* We are leaving a hangar, but have to go to the exact same one; re-enter */
        if self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
            && self.order_destination(v) == v.targetairport()
        {
            Self::call(|| self.vs.enter_depot(v.base()));
            return;
        }

        /* if the block of the next position is busy, stay put */
        if self.airport_has_block(v, self.layout(apc, v.pos()), apc) {
            return;
        }

        /* We are already at the target airport, we need to find a terminal */
        if self.order_destination(v) == v.targetairport() {
            /* FindFreeTerminal:
             * 1. Find a free terminal, 2. Occupy it, 3. Set the vehicle's state to that terminal */
            if v.base().subtype() == AIR_HELICOPTER {
                if !self.airport_find_free_helipad(v, apc) {
                    return; // helicopter
                }
            } else if !self.airport_find_free_terminal(v, apc) {
                return; // airplane
            }
        } else {
            // Else prepare for launch.
            /* airplane goto state takeoff, helicopter to helitakeoff */
            v.set_state(if v.base().subtype() == AIR_HELICOPTER {
                HELITAKEOFF
            } else {
                TAKEOFF
            });
        }
        let tile = v.base().tile();
        let st = Self::station_get(StationID(Self::call(|| (self.ss.station_index)(tile))));
        let exit_dir = Direction(Self::station_call(|| unsafe {
            (self.leaves.hangar_exit)(st, tile, Self::airport_spec(st))
        }));
        self.aircraft_leave_hangar(v, exit_dir);
        self.airport_move(v, apc);
    }

    /// Original `AircraftEventHandler_AtTerminal`: at one of the Airport's Terminals.
    fn aircraft_event_handler_at_terminal(&self, v: Vehicle, apc: *const c_void) {
        /* if we just arrived, execute EnterTerminal first */
        if v.previous_pos() != v.pos() {
            self.aircraft_event_handler_enter_terminal(v, apc);
            /* on an airport with helipads, a helicopter will always land there
             * and get serviced at the same time - setting */
            if Self::call(|| (self.ss.service_at_helipad)())
                && v.base().subtype() == AIR_HELICOPTER
                && self.num_helipads(apc) > 0
            {
                let b = v.base();
                /* an excerpt of ServiceAircraft, without the invisibility stuff */
                Self::call(|| {
                    (self.vs.write_economy_service)(b, crate::timer_game_common::economy_date().0);
                });
                Self::call(|| {
                    (self.vs.write_calendar_service)(
                        b,
                        crate::timer_game_common::calendar_date().0,
                    );
                });
                Self::call(|| (self.vs.write_breakdowns)(b, 0));
                let reliability = Self::call(|| (self.vs.engine)(b)).reliability();
                Self::call(|| (self.vs.write_reliability)(b, reliability));
                // SAFETY: The base vehicle table names its immutable maintenance table.
                Self::call(|| (unsafe { &*self.vs.maintenance }.details_dirty)(b));
            }
            return;
        }

        if self.current_order_is_type(v, OrderType::OT_NOTHING) {
            return;
        }

        /* if the block of the next position is busy, stay put */
        if self.airport_has_block(v, self.layout(apc, v.pos()), apc) {
            return;
        }

        /* airport-road is free. We either have to go to another airport, or to the hangar
         * ---> start moving */

        let mut go_to_hangar = false;
        match self.order_type(v) {
            OrderType::OT_GOTO_STATION => {} // ready to fly to another airport
            OrderType::OT_GOTO_DEPOT => {
                // visit hangar for servicing, sale, etc.
                go_to_hangar = self.order_destination(v) == v.targetairport();
            }
            OrderType::OT_CONDITIONAL => {
                /* In case of a conditional order we just have to wait a tick
                 * longer, so the conditional order can actually be processed;
                 * we should not clear the order as that makes us go nowhere. */
                return;
            }
            _ => {
                // orders have been deleted (no orders), goto depot and don't bother us
                Self::call(|| (self.vs.order_free)(v.base()));
                go_to_hangar = true;
            }
        }

        if go_to_hangar && Self::airport_has_hangar(Self::station_get(v.targetairport())) {
            v.set_state(HANGAR);
        } else {
            /* airplane goto state takeoff, helicopter to helitakeoff */
            v.set_state(if v.base().subtype() == AIR_HELICOPTER {
                HELITAKEOFF
            } else {
                TAKEOFF
            });
        }
        self.airport_move(v, apc);
    }

    /// Original `AircraftEventHandler_General`.
    fn aircraft_event_handler_general(&self, v: Vehicle) {
        Self::call(|| unsafe { (self.leaves.invalid_scheme)(v) });
    }

    /// Original `AircraftEventHandler_TakeOff`.
    fn aircraft_event_handler_take_off(&self, v: Vehicle) {
        self.play_aircraft_sound(v); // play takeoffsound for airplanes
        v.set_state(STARTTAKEOFF);
    }

    /// Original `AircraftEventHandler_StartTakeOff`.
    fn aircraft_event_handler_start_take_off(&self, v: Vehicle) {
        v.set_state(ENDTAKEOFF);
        self.update_delta_xy(v);
    }

    /// Original `AircraftEventHandler_EndTakeOff`.
    fn aircraft_event_handler_end_take_off(&self, v: Vehicle) {
        v.set_state(FLYING);
        /* get the next position to go to, differs per airport */
        self.aircraft_next_airport_pos_and_order(v);
    }

    /// Original `AircraftEventHandler_HeliTakeOff`.
    fn aircraft_event_handler_heli_take_off(&self, v: Vehicle) {
        v.set_state(FLYING);
        self.update_delta_xy(v);

        /* get the next position to go to, differs per airport */
        self.aircraft_next_airport_pos_and_order(v);

        /* Send the helicopter to a hangar if needed for replacement */
        if self.needs_automatic_servicing(v) {
            self.send_to_depot(
                v,
                DepotCommandFlags::from_element(DepotCommandFlag::Service),
            );
        }
    }

    /// Original `AircraftEventHandler_Flying`.
    fn aircraft_event_handler_flying(&self, v: Vehicle, apc: *const c_void) {
        let st = Self::station_get(v.targetairport());

        /* Runway busy, not allowed to use this airstation or closed, circle. */
        let owner = self.station_owner(st);
        if Self::can_vehicle_use_station(v, st)
            && (owner == Owner::NONE || owner.0 == v.base().owner())
            && !self.airport_blocks(st).test(AirportBlock::AIRPORT_CLOSED)
        {
            /* {32,FLYING,AirportBlock::Nothing,37}, {32,LANDING,N,33}, {32,HELILANDING,N,41},
             * if it is an airplane, look for LANDING, for helicopter HELILANDING
             * it is possible to choose from multiple landing runways, so loop until a free one is found */
            let landingtype = if v.base().subtype() == AIR_HELICOPTER {
                HELILANDING
            } else {
                LANDING
            };
            let mut current = self.next(self.layout(apc, v.pos()));
            while !current.is_null() {
                if self.heading(current) == landingtype {
                    /* save speed before, since if AirportHasBlock is false, it resets them to 0
                     * we don't want that for plane in air
                     * hack for speed thingy */
                    let tcur_speed = v.base().speed();
                    let tsubspeed = v.base().subspeed();
                    if !self.airport_has_block(v, current, apc) {
                        v.set_state(landingtype); // LANDING / HELILANDING
                        if v.state() == HELILANDING {
                            let mut flags = v.flags();
                            set_bit8(&mut flags, AirVehicleFlags::VAF_HELI_DIRECT_DESCENT.0);
                            v.set_flags(flags);
                        }
                        /* it's a bit dirty, but I need to set position to next position, otherwise
                         * if there are multiple runways, plane won't know which one it took (because
                         * they all have heading LANDING). And also occupy that block! */
                        v.set_pos(self.next_position(current));
                        let blocks = self.blocks(self.layout(apc, v.pos()));
                        self.airport_blocks_update(st, |b| {
                            b.set(blocks);
                        });
                        return;
                    }
                    v.base().write_speed(tcur_speed);
                    v.base().write_subspeed(tsubspeed);
                }
                current = self.next(current);
            }
        }
        v.set_state(FLYING);
        v.set_pos(self.next_position(self.layout(apc, v.pos())));
    }

    /// Original `AircraftEventHandler_Landing`.
    fn aircraft_event_handler_landing(&self, v: Vehicle) {
        v.set_state(ENDLANDING);
        self.aircraft_land_airplane(v); // maybe crash airplane

        /* check if the aircraft needs to be replaced or renewed and send it to a hangar if needed */
        if self.needs_automatic_servicing(v) {
            self.send_to_depot(
                v,
                DepotCommandFlags::from_element(DepotCommandFlag::Service),
            );
        }
    }

    /// Original `AircraftEventHandler_HeliLanding`.
    fn aircraft_event_handler_heli_landing(&self, v: Vehicle) {
        v.set_state(HELIENDLANDING);
        self.update_delta_xy(v);
    }

    /// Original `AircraftEventHandler_EndLanding`.
    fn aircraft_event_handler_end_landing(&self, v: Vehicle, apc: *const c_void) {
        /* next block busy, don't do a thing, just wait */
        if self.airport_has_block(v, self.layout(apc, v.pos()), apc) {
            return;
        }

        /* if going to terminal (OT_GOTO_STATION) choose one
         * 1. in case all terminals are busy AirportFindFreeTerminal() returns false or
         * 2. not going for terminal (but depot, no order),
         * --> get out of the way to the hangar. */
        if self.current_order_is_type(v, OrderType::OT_GOTO_STATION)
            && self.airport_find_free_terminal(v, apc)
        {
            return;
        }
        v.set_state(HANGAR);
    }

    /// Original `AircraftEventHandler_HeliEndLanding`.
    fn aircraft_event_handler_heli_end_landing(&self, v: Vehicle, apc: *const c_void) {
        /*  next block busy, don't do a thing, just wait */
        if self.airport_has_block(v, self.layout(apc, v.pos()), apc) {
            return;
        }

        /* if going to helipad (OT_GOTO_STATION) choose one. If airport doesn't have helipads, choose terminal
         * 1. in case all terminals/helipads are busy (AirportFindFreeHelipad() returns false) or
         * 2. not going for terminal (but depot, no order),
         * --> get out of the way to the hangar IF there are terminals on the airport.
         * --> else TAKEOFF
         * the reason behind this is that if an airport has a terminal, it also has a hangar. Airplanes
         * must go to a hangar. */
        if self.current_order_is_type(v, OrderType::OT_GOTO_STATION)
            && self.airport_find_free_helipad(v, apc)
        {
            return;
        }
        v.set_state(
            if Self::airport_has_hangar(Self::station_get(v.targetairport())) {
                HANGAR
            } else {
                HELITAKEOFF
            },
        );
    }

    /// `_aircraft_state_handlers[v->state](v, apc)`: the handler function for each target
    /// of the aircraft.
    fn aircraft_state_handler(&self, v: Vehicle, apc: *const c_void) {
        match v.state() {
            HANGAR => self.aircraft_event_handler_in_hangar(v, apc),
            TERM1 | TERM2 | TERM3 | TERM4 | TERM5 | TERM6 | HELIPAD1 | HELIPAD2 | TERM7 | TERM8
            | HELIPAD3 => self.aircraft_event_handler_at_terminal(v, apc),
            TAKEOFF => self.aircraft_event_handler_take_off(v),
            STARTTAKEOFF => self.aircraft_event_handler_start_take_off(v),
            ENDTAKEOFF => self.aircraft_event_handler_end_take_off(v),
            HELITAKEOFF => self.aircraft_event_handler_heli_take_off(v),
            FLYING => self.aircraft_event_handler_flying(v, apc),
            LANDING => self.aircraft_event_handler_landing(v),
            ENDLANDING => self.aircraft_event_handler_end_landing(v, apc),
            HELILANDING => self.aircraft_event_handler_heli_landing(v),
            HELIENDLANDING => self.aircraft_event_handler_heli_end_landing(v, apc),
            /* TO_ALL, the table's first entry. */
            _ => self.aircraft_event_handler_general(v),
        }
    }

    /// Original `AirportClearBlock`.
    fn airport_clear_block(&self, v: Vehicle, apc: *const c_void) {
        let previous_blocks = self.blocks(self.layout(apc, v.previous_pos()));
        /* we have left the previous block, and entered the new one. Free the previous block */
        if previous_blocks != self.blocks(self.layout(apc, v.pos())) {
            let st = Self::station_get(v.targetairport());

            if self.airport_blocks(st).test(AirportBlock::ZEPPELINER)
                && previous_blocks == AirportBlocks::from_element(AirportBlock::RUNWAY_IN)
            {
                return;
            }

            self.airport_blocks_update(st, |blocks| {
                blocks.reset(previous_blocks);
            });
        }
    }

    /// Original `AirportGoToNextPosition`.
    fn airport_go_to_next_position(&self, v: Vehicle) {
        /* if aircraft is not in position, wait until it is */
        if !self.aircraft_controller(v) {
            return;
        }

        let apc = Self::airport_get_fta(Self::station_get(v.targetairport()));

        self.airport_clear_block(v, apc);
        self.airport_move(v, apc); // move aircraft to next position
    }

    /// Original `AirportMove`: gets pos from vehicle and next orders.
    fn airport_move(&self, v: Vehicle, apc: *const c_void) -> bool {
        /* error handling */
        let nofelements = self.nofelements(apc);
        if v.pos() >= nofelements {
            crate::debug::debug!(
                misc,
                0,
                "[Ap] position {} is not valid for current airport. Max position is {}",
                v.pos(),
                i32::from(nofelements) - 1
            );
            self.check(v.pos() < nofelements);
        }

        let mut current = self.layout(apc, v.pos());
        /* we have arrived in an important state (eg terminal, hangar, etc.) */
        if self.heading(current) == v.state() {
            let prev_pos = v.pos(); // location could be changed in state, so save it before-hand
            let prev_state = v.state();
            self.aircraft_state_handler(v, apc);
            if v.state() != FLYING {
                v.set_previous_pos(prev_pos);
            }
            if v.state() != prev_state || v.pos() != prev_pos {
                self.update_aircraft_cache(v, false);
            }
            return true;
        }

        v.set_previous_pos(v.pos()); // save previous location

        /* there is only one choice to move to */
        if self.next(current).is_null() {
            if self.airport_set_blocks(v, current, apc) {
                v.set_pos(self.next_position(current));
                self.update_aircraft_cache(v, false);
            } // move to next position
            return false;
        }

        /* there are more choices to choose from, choose the one that
         * matches our heading */
        loop {
            let heading = self.heading(current);
            if v.state() == heading || heading == AirportMovementStates::TO_ALL.0 {
                if self.airport_set_blocks(v, current, apc) {
                    v.set_pos(self.next_position(current));
                    self.update_aircraft_cache(v, false);
                } // move to next position
                return false;
            }
            current = self.next(current);
            if current.is_null() {
                break;
            }
        }

        crate::debug::debug!(
            misc,
            0,
            "[Ap] cannot move further on Airport! (pos {} state {}) for vehicle {}",
            v.pos(),
            v.state(),
            v.index().0
        );
        crate::services_assert::not_reached()
    }

    /// Original `AirportHasBlock`: returns true if the road ahead is busy, eg. you must
    /// wait before proceeding.
    fn airport_has_block(
        &self,
        v: Vehicle,
        current_pos: *const c_void,
        apc: *const c_void,
    ) -> bool {
        let reference = self.layout(apc, v.pos());
        let next = self.layout(apc, self.next_position(current_pos));

        /* same block, then of course we can move */
        let next_blocks = self.blocks(next);
        if self.blocks(self.layout(apc, self.position(current_pos))) != next_blocks {
            let st = Self::station_get(v.targetairport());
            let mut blocks = next_blocks;

            /* check additional possible extra blocks */
            if current_pos != reference {
                let current_blocks = self.blocks(current_pos);
                if current_blocks != AirportBlocks::from_element(AirportBlock::NOTHING) {
                    blocks.set(current_blocks);
                }
            }

            if self.airport_blocks(st).any(blocks) {
                v.base().write_speed(0);
                v.base().write_subspeed(0);
                return true;
            }
        }
        false
    }

    /// Original `AirportSetBlocks`: "reserve" a block for the plane. Returns true on
    /// success. Eg, next block was free and we have occupied it.
    fn airport_set_blocks(
        &self,
        v: Vehicle,
        current_pos: *const c_void,
        apc: *const c_void,
    ) -> bool {
        let next = self.layout(apc, self.next_position(current_pos));
        let reference = self.layout(apc, v.pos());

        /* if the next position is in another block, check it and wait until it is free */
        let next_blocks = self.blocks(next);
        if !self
            .blocks(self.layout(apc, self.position(current_pos)))
            .all(next_blocks)
        {
            let mut blocks = next_blocks;
            /* search for all all elements in the list with the same state, and blocks != N
             * this means more blocks should be checked/set */
            let mut current = current_pos;
            if current == reference {
                current = self.next(current);
            }
            let current_pos_heading = self.heading(current_pos);
            while !current.is_null() {
                if self.heading(current) == current_pos_heading {
                    let current_blocks = self.blocks(current);
                    if current_blocks.is_any() {
                        blocks.set(current_blocks);
                        break;
                    }
                }
                current = self.next(current);
            }

            /* if the block to be checked is in the next position, then exclude that from
             * checking, because it has been set by the airplane before */
            if self.blocks(current_pos) == next_blocks {
                blocks.flip(next_blocks);
            }

            let st = Self::station_get(v.targetairport());
            if self.airport_blocks(st).any(blocks) {
                v.base().write_speed(0);
                v.base().write_subspeed(0);
                return false;
            }

            if next_blocks != AirportBlocks::from_element(AirportBlock::NOTHING) {
                self.airport_blocks_update(st, |b| {
                    b.set(blocks); // occupy next block
                });
            }
        }
        true
    }

    /// Original `FreeTerminal`: find a free terminal or helipad, and if available, assign
    /// it. `i` is the first terminal to examine and `last_terminal` the terminal number to
    /// stop examining. Returns whether a terminal or helipad has been found and assigned.
    fn free_terminal(&self, v: Vehicle, mut i: u8, last_terminal: u8) -> bool {
        self.check(usize::from(last_terminal) <= AIRPORT_TERMINAL_MAPPING.len());
        let st = Self::station_get(v.targetairport());
        while i < last_terminal {
            let mapping = &AIRPORT_TERMINAL_MAPPING[usize::from(i)];
            let blocks = AirportBlocks::from_element(mapping.blocks);
            if !self.airport_blocks(st).any(blocks) {
                /* TERMINAL# HELIPAD# */
                v.set_state(mapping.state); // start moving to that terminal/helipad
                self.airport_blocks_update(st, |b| {
                    b.set(blocks); // occupy terminal/helipad
                });
                return true;
            }
            i += 1;
        }
        false
    }

    /// Original `GetNumTerminals`: get the number of terminals at the airport.
    fn get_num_terminals(&self, apc: *const c_void) -> u32 {
        let mut num: u32 = 0;

        let mut i = u32::from(self.terminals(apc, 0));
        while i > 0 {
            num = num.wrapping_add(u32::from(self.terminals(apc, i)));
            i -= 1;
        }

        num
    }

    /// Original `AirportFindFreeTerminal`: find a free terminal, and assign it if
    /// available. Returns whether a free terminal was found and assigned.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "FreeTerminal's uint8_t parameters take the uint group bounds"
    )]
    fn airport_find_free_terminal(&self, v: Vehicle, apc: *const c_void) -> bool {
        /* example of more terminalgroups
         * {0,HANGAR,AirportBlock::Nothing,1}, {0,TERMGROUP,AirportBlock::TermGroup1,0}, {0,TERMGROUP,TERM_GROUP2_ENTER_block,1}, {0,0,N,1},
         * Heading TERMGROUP denotes a group. We see 2 groups here:
         * 1. group 0 -- AirportBlock::TermGroup1 (check block)
         * 2. group 1 -- TERM_GROUP2_ENTER_block (check block)
         * First in line is checked first, group 0. If the block (AirportBlock::TermGroup1) is free, it
         * looks at the corresponding terminals of that group. If no free ones are found, other
         * possible groups are checked (in this case group 1, since that is after group 0). If that
         * fails, then attempt fails and plane waits
         */
        if self.terminals(apc, 0) > 1 {
            let st = Self::station_get(v.targetairport());
            let mut temp = self.next(self.layout(apc, v.pos()));

            while !temp.is_null() {
                if self.heading(temp) == AirportMovementStates::TERMGROUP.0 {
                    if !self.airport_blocks(st).any(self.blocks(temp)) {
                        /* read which group do we want to go to?
                         * (the first free group) */
                        let target_group = u32::from(self.next_position(temp)) + 1;

                        /* at what terminal does the group start?
                         * that means, sum up all terminals of
                         * groups with lower number */
                        let mut group_start: u32 = 0;
                        for i in 1..target_group {
                            group_start += u32::from(self.terminals(apc, i));
                        }

                        let group_end = group_start + u32::from(self.terminals(apc, target_group));
                        if self.free_terminal(v, group_start as u8, group_end as u8) {
                            return true;
                        }
                    }
                } else {
                    /* once the heading isn't 255, we've exhausted the possible blocks.
                     * So we cannot move */
                    return false;
                }
                temp = self.next(temp);
            }
        }

        /* if there is only 1 terminalgroup, all terminals are checked (starting from 0 to max) */
        self.free_terminal(v, 0, self.get_num_terminals(apc) as u8)
    }

    /// Original `AirportFindFreeHelipad`: find a free helipad, and assign it if available.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "FreeTerminal's uint8_t parameters take the uint group bounds"
    )]
    fn airport_find_free_helipad(&self, v: Vehicle, apc: *const c_void) -> bool {
        /* if an airport doesn't have helipads, use terminals */
        let num_helipads = self.num_helipads(apc);
        if num_helipads == 0 {
            return self.airport_find_free_terminal(v, apc);
        }

        /* only 1 helicoptergroup, check all helipads
         * The blocks for helipads start after the last terminal (MAX_TERMINALS) */
        self.free_terminal(
            v,
            MAX_TERMINALS as u8,
            u32::from(num_helipads).wrapping_add(MAX_TERMINALS) as u8,
        )
    }

    /// Original `AircraftHandleDestTooFar`: handle the 'dest too far' flag and the
    /// corresponding news message for aircraft.
    fn aircraft_handle_dest_too_far(&self, v: Vehicle, too_far: bool) {
        let mut flags = v.flags();
        if too_far {
            if !has_bit(flags, AirVehicleFlags::VAF_DEST_TOO_FAR.0) {
                set_bit8(&mut flags, AirVehicleFlags::VAF_DEST_TOO_FAR.0);
                v.set_flags(flags);
                self.start_stop_dirty(v);
                /* The script event and, for the local company, the advice news. */
                Self::call(|| unsafe { (self.leaves.destination_too_far)(v) });
            }
            return;
        }

        if has_bit(flags, AirVehicleFlags::VAF_DEST_TOO_FAR.0) {
            /* Not too far anymore, clear flag and message. */
            clr_bit8(&mut flags, AirVehicleFlags::VAF_DEST_TOO_FAR.0);
            v.set_flags(flags);
            self.start_stop_dirty(v);
            Self::call(|| unsafe { (self.leaves.delete_range_news)(v) });
        }
    }

    /// Original `AircraftEventHandler`.
    fn aircraft_event_handler(&self, v: Vehicle, r#loop: u32) -> bool {
        let b = v.base();
        if vehstatus_test(b, VehState::Crashed) {
            return self.handle_crashed_aircraft(v);
        }

        if vehstatus_test(b, VehState::Stopped) {
            return true;
        }

        // SAFETY: A live vehicle and its immutable maintenance table.
        unsafe { maintenance::handle_breakdown_status(b, self.vs) };

        Self::handle_aircraft_smoke(v, r#loop != 0);
        let orders = Self::call(|| self.vs.orders());
        crate::orders::process_orders(b, orders);
        Self::call(|| self.vs.handle_loading(b, r#loop != 0, orders));

        if self.current_order_is_type(v, OrderType::OT_LOADING)
            || self.current_order_is_type(v, OrderType::OT_LEAVESTATION)
        {
            return true;
        }

        if v.state() >= ENDTAKEOFF && v.state() <= HELIENDLANDING {
            /* If we are flying, unconditionally clear the 'dest too far' state. */
            self.aircraft_handle_dest_too_far(v, false);
        } else if v.cached_max_range_sqr() != 0 {
            /* Check the distance to the next destination. This code works because the target
             * airport is only updated after take off and not on the ground. */
            let cur_st = self.station_get_if_valid(v.targetairport());
            let next_st = if self.current_order_is_type(v, OrderType::OT_GOTO_STATION)
                || self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
            {
                self.station_get_if_valid(self.order_destination(v))
            } else {
                std::ptr::null()
            };

            if !cur_st.is_null()
                && self.airport_tile(cur_st) != TileIndex::INVALID
                && !next_st.is_null()
                && self.airport_tile(next_st) != TileIndex::INVALID
            {
                let dist = self
                    .dims()
                    .distance_square(self.airport_tile(cur_st), self.airport_tile(next_st));
                self.aircraft_handle_dest_too_far(v, dist > v.cached_max_range_sqr());
            }
        }

        if !has_bit(v.flags(), AirVehicleFlags::VAF_DEST_TOO_FAR.0) {
            self.airport_go_to_next_position(v);
        }

        true
    }

    /// Original `Aircraft::Tick`.
    fn tick(&self, v: Vehicle) -> bool {
        let b = v.base();
        if !is_normal_aircraft(b.subtype()) {
            return true;
        }

        crate::witness::hit(crate::witness::AIRCRAFT_TICKS);

        b.write_tick(b.tick().wrapping_add(1));

        if !vehstatus_test(b, VehState::Stopped) {
            b.write_running(b.running().wrapping_add(1));
        }

        if b.subtype() == AIR_HELICOPTER {
            self.helicopter_tick_handler(v);
        }

        b.write_order_time(b.order_time().wrapping_add(1));

        for i in 0..2 {
            /* stop if the aircraft was deleted */
            if !self.aircraft_event_handler(v, i) {
                return false;
            }
        }

        true
    }

    /// Original `GetTargetAirportIfValid`: returns aircraft's target station if
    /// `v->target_airport` is a valid station with airport, null if invalid.
    fn get_target_airport_if_valid(&self, v: Vehicle) -> *const Station {
        self.check(VehicleType(v.base().vehicle_type()) == VehicleType::VEH_AIRCRAFT);

        let st = self.station_get_if_valid(v.targetairport());
        if st.is_null() {
            return std::ptr::null();
        }

        if self.airport_tile(st) == TileIndex::INVALID {
            std::ptr::null()
        } else {
            st
        }
    }

    /// Original `UpdateAirplanesOnNewStation`: updates the status of the Aircraft heading
    /// or in the station.
    fn update_airplanes_on_new_station(&self, index: StationID) {
        let st = Self::station_get(index);
        /* only 1 station is updated per function call, so it is enough to get entry_point once */
        let ap = Self::airport_get_fta(st);
        let rotation = if self.airport_tile(st) == TileIndex::INVALID {
            Direction::DIR_N
        } else {
            self.airport_rotation(st)
        };

        let mut v = Vehicle::invalid();
        loop {
            v = Self::call(|| unsafe { (self.leaves.next_aircraft)(v.id) });
            if v.handle.is_null() {
                break;
            }
            if !is_normal_aircraft(v.base().subtype()) || v.targetairport() != index {
                continue;
            }
            self.check(v.state() == FLYING);

            /* The aircraft is heading to a hangar, but the new station doesn't have one,
             * or the aircraft can't land on the new station. Cancel current order. */
            if self.current_order_is_type(v, OrderType::OT_GOTO_DEPOT)
                && v.base().current_order().depot_type() & ODTF_PART_OF_ORDERS == 0
                && self.order_destination(v) == index
                && (!Self::airport_has_hangar(st) || !Self::can_vehicle_use_station(v, st))
            {
                Self::call(|| (self.vs.make_dummy)(v.base()));
                self.start_stop_dirty(v);
            }
            let pos = self.aircraft_get_entry_point(v, ap, rotation);
            v.set_previous_pos(pos);
            v.set_pos(pos);
            self.update_aircraft_cache(v, false);
        }

        /* Heliports don't have a hangar. Invalidate all go to hangar orders from all aircraft. */
        if !Self::airport_has_hangar(st) {
            crate::orders::remove_order_from_all_vehicles(
                self.vs.orders(),
                OrderType::OT_GOTO_DEPOT,
                index,
                true,
            );
        }
    }

    /// `Station::~Station`'s loop: normal aircraft targeting the station lose their target.
    fn station_removed(&self, index: StationID) {
        let mut a = Vehicle::invalid();
        loop {
            a = Self::call(|| unsafe { (self.leaves.next_aircraft)(a.id) });
            if a.handle.is_null() {
                break;
            }
            if !is_normal_aircraft(a.base().subtype()) {
                continue;
            }
            if a.targetairport() == index {
                a.set_targetairport(StationID::INVALID);
            }
        }
    }

    /// `Vehicle::PreDestructor`'s aircraft branch: release the blocks of the previous and
    /// current position at a valid target airport.
    fn release_aircraft_airport_blocks(&self, a: Vehicle) {
        let st = self.get_target_airport_if_valid(a);
        if !st.is_null() {
            let layout = Self::airport_get_fta(st);
            let mut blocks = self.blocks(self.layout(layout, a.previous_pos()));
            blocks.set(self.blocks(self.layout(layout, a.pos())));
            self.airport_blocks_update(st, |b| {
                b.reset(blocks);
            });
        }
    }
}

/// `Vehicle::GetOldAdvanceSpeed`: determines the effective direction-specific vehicle
/// movement speed. This method belongs to the old vehicle movement method: a vehicle
/// moves a step every 256 progress units. The vehicle speed is scaled by 3/4 when moving
/// in X or Y direction due to the longer distance.
fn get_old_advance_speed(v: VehicleRef, speed: u32) -> u32 {
    if v.direction() & 1 != 0 {
        speed
    } else {
        speed.wrapping_mul(3) / 4
    }
}

/// `Aircraft::IsPrimaryVehicle`.
const fn is_primary_vehicle(subtype: u8) -> bool {
    is_normal_aircraft(subtype)
}

/// `Aircraft::IsNormalAircraft`: check if the aircraft type is a normal flying device;
/// eg not a rotor or a shadow. Returns true if the aircraft is a helicopter/airplane and
/// false if it is a shadow or a rotor.
pub(crate) const fn is_normal_aircraft(subtype: u8) -> bool {
    /* To be fully correct the commented out functionality is the proper one,
     * but since value can only be 0 or 2, it is sufficient to only check <= 2
     * return (this->subtype == AIR_HELICOPTER) || (this->subtype == AIR_AIRCRAFT); */
    subtype <= AIR_AIRCRAFT
}

/// `Aircraft::GetSpeedOldUnits` applied to `cached_max_speed`, at `int` width.
pub(crate) fn speed_old_units(speed: u32) -> u32 {
    (speed.cast_signed().wrapping_mul(10) / 128).cast_unsigned()
}

/// Allocate the canonical private state once for every ordinary/indexed aircraft shell.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_aircraft_state_new() -> *mut AircraftState {
    Box::into_raw(Box::new(AircraftState {
        cached_max_range_sqr: 0,
        cached_max_range: 0,
        cache_padding: 0,
        crashed_counter: 0,
        targetairport: StationID::INVALID.0,
        pos: 0,
        previous_pos: 0,
        state: 0,
        last_direction: Direction::INVALID_DIR.0,
        number_consecutive_turns: 0,
        turn_counter: 0,
        flags: 0,
    }))
}
/// Destroy the sole owner after the C++ shell's `PreDestructor` and scalar aliases end.
/// # Safety
/// The allocation is live, uniquely owned, and no scalar access remains active.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_state_destroy(state: *mut AircraftState) {
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// Allocate the station airport's canonical mask, independently of service ownership.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_blocks_new() -> *mut AirportBlocks {
    Box::into_raw(Box::new(AirportBlocks(0)))
}
/// Release exactly once after the enclosing station stops using the scalar.
/// # Safety
/// Pointer is the live sole airport allocation with no active scalar access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_blocks_destroy(blocks: *mut AirportBlocks) {
    unsafe {
        drop(Box::from_raw(blocks));
    }
}

// Witness enablement is fixed by the process-start writer. Select once per
// synchronous entry; both variants use the same World and service bodies.
macro_rules! with_world {
    ($leaves:expr, |$world:ident| $body:block) => {{
        if crate::witness::enabled() {
            // SAFETY: The entry supplies live immutable tables for this call.
            let $world = unsafe { World::<true>::new($leaves) };
            $body
        } else {
            // SAFETY: Same tables and lifetime as the counted specialization.
            let $world = unsafe { World::<false>::new($leaves) };
            $body
        }
    }};
}

/// `UpdateAircraftCache`, for Rust callers.
/// # Safety
/// `leaves` is the live immutable table and `v` a live aircraft.
pub(crate) unsafe fn update_aircraft_cache(leaves: *const Leaves, v: Vehicle, update_range: bool) {
    crate::witness::hit(crate::witness::AIRCRAFT_CACHE_ENTRIES);
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.update_aircraft_cache(v, update_range);
    });
}
/// `HandleAircraftEnterHangar`, for Rust callers.
/// # Safety
/// As `update_aircraft_cache`.
pub(crate) unsafe fn handle_aircraft_enter_hangar(leaves: &Leaves, v: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.handle_aircraft_enter_hangar(v);
    });
}
/// `UpdateAirplanesOnNewStation`, for Rust callers.
/// # Safety
/// `leaves` is the live immutable table and `st` a live station.
pub(crate) unsafe fn update_airplanes_on_new_station(leaves: *const Leaves, st: StationID) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.update_airplanes_on_new_station(st);
    });
}
/// `Station::~Station`'s aircraft loop, for Rust callers.
/// # Safety
/// `leaves` is the live immutable table.
pub(crate) unsafe fn invalidate_target(leaves: *const Leaves, station: StationID) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.station_removed(station);
    });
}
/// `AircraftEventHandler`, for the native test entry.
/// # Safety
/// As `update_aircraft_cache`.
#[cfg(feature = "native-test-support")]
pub(crate) unsafe fn event(leaves: *const Leaves, id: Vehicle, pass: bool) -> bool {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.aircraft_event_handler(id, u32::from(pass))
    })
}

/* The exports below share one contract.
 * # Safety
 * `leaves` is the live immutable table and the handle a live aircraft (or, for the
 * flight helpers, a live disaster vehicle with null state). Raw scalar accesses end
 * before noexcept callbacks; no borrow survives reentry. */

/// `Aircraft::Tick`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_tick_abi(leaves: *const Leaves, id: Vehicle) -> u8 {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    u8::from(with_world!(leaves, |w| { w.tick(id) }))
}
/// `Aircraft::OnNewCalendarDay`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_calendar_day(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.on_new_calendar_day(id);
    });
}
/// `Aircraft::OnNewEconomyDay`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_economy_day(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.on_new_economy_day(id);
    });
}
/// `SetAircraftPosition`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_position(
    leaves: *const Leaves,
    id: Vehicle,
    x: i32,
    y: i32,
    z: i32,
) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.set_aircraft_position(id, x, y, z);
    });
}
/// `UpdateAircraftCache`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_cache(
    leaves: *const Leaves,
    id: Vehicle,
    range: bool,
) {
    unsafe { update_aircraft_cache(leaves, id, range) };
}
/// `GetAircraftFlightLevelBounds`.
/// # Safety
/// See the shared export contract; each non-null bound is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_flight_bounds(
    leaves: *const Leaves,
    id: Vehicle,
    min: *mut i32,
    max: *mut i32,
) {
    if id.state.is_null() {
        crate::witness::hit(crate::witness::AIRCRAFT_DISASTER_ENTRIES);
    }
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        // SAFETY: The caller supplies null or writable bounds.
        w.get_aircraft_flight_level_bounds(id.base(), unsafe { min.as_mut() }, unsafe {
            max.as_mut()
        });
    });
}
/// `GetAircraftFlightLevel`.
/// # Safety
/// See the shared export contract; `flags` is the vehicle's live flags.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_flight_level(
    leaves: *const Leaves,
    id: Vehicle,
    flags: *mut u8,
    takeoff: bool,
) -> i32 {
    if id.state.is_null() {
        crate::witness::hit(crate::witness::AIRCRAFT_DISASTER_ENTRIES);
    }
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.get_aircraft_flight_level(id.base(), flags, takeoff)
    })
}
/// `GetTileHeightBelowAircraft`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_height(leaves: *const Leaves, id: Vehicle) -> i32 {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.get_tile_height_below_aircraft(id.base()) })
}
/// `GetAircraftHoldMaxAltitude`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_hold_altitude(
    leaves: *const Leaves,
    id: Vehicle,
) -> i32 {
    if id.state.is_null() {
        crate::witness::hit(crate::witness::AIRCRAFT_DISASTER_ENTRIES);
    }
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.get_aircraft_hold_max_altitude(id.base()) })
}
/// `HandleMissingAircraftOrders`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_missing_orders(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.handle_missing_aircraft_orders(id);
    });
}
/// `Aircraft::Crash`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_crash(
    leaves: *const Leaves,
    id: Vehicle,
    flooded: bool,
) -> u32 {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.crash(id, flooded) })
}
/// `AircraftNextAirportPos_and_Order`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_next_airport(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.aircraft_next_airport_pos_and_order(id);
    });
}
/// `AircraftLeaveHangar`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_leave_hangar(
    leaves: *const Leaves,
    id: Vehicle,
    direction: u8,
) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.aircraft_leave_hangar(id, Direction(direction));
    });
}
/// `Aircraft::GetOrderStationLocation`.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_order_location(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.get_order_station_location(id);
    });
}
/// `UpdateAirplanesOnNewStation`.
/// # Safety
/// See the shared export contract; `station` is a live station.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_replacement(leaves: *const Leaves, station: u16) {
    unsafe { update_airplanes_on_new_station(leaves, StationID(station)) };
}
/// `Aircraft::FindClosestDepot`'s station.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_closest_depot(
    leaves: *const Leaves,
    id: Vehicle,
) -> u16 {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.find_closest_depot(id).0 })
}
/// `Vehicle::PreDestructor`'s aircraft block release.
/// # Safety
/// See the shared export contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_release_blocks(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.release_aircraft_airport_blocks(id);
    });
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAircraftLeaves" ["probe"] => Leaves {
        airport_blocks,
        airport_delta_z,
        airport_elements,
        airport_entry,
        airport_flags,
        airport_height,
        airport_tile,
        airport_type,
        airport_width,
        copy_sprite,
        crash_news,
        delete_range_news,
        destination_too_far,
        direction_towards,
        dirty_depot,
        dirty_lists,
        first_arrival,
        fta_blocks,
        fta_heading,
        fta_next,
        fta_next_position,
        fta_position,
        had_vehicle,
        hangar_exit,
        helipads,
        invalid_scheme,
        landing_rating,
        mass,
        moving,
        next_aircraft,
        node,
        rotation,
        rotor_image,
        rotor_image_if_changed,
        service_interval,
        service_order,
        shared,
        station,
        station_owner,
        station_tile,
        subtract_cost,
        terminal_count,
        truncate_cargo,
        update_rotor_image,
        vehicle,
        vehicle_crash,
        viewport,
    };
    "OpenTTDAircraftMoving" ["probe"] => AirportMovingData {
        x,
        y,
        flags,
        direction,
    };
    "OpenTTDAircraftState" ["probe"] => AircraftState {
        cached_max_range_sqr,
        cached_max_range,
        cache_padding,
        crashed_counter,
        targetairport,
        pos,
        previous_pos,
        state,
        last_direction,
        number_consecutive_turns,
        turn_counter,
        flags,
    };
    "OpenTTDAircraftVehicle" ["probe"] => Vehicle {
        handle,
        state,
        id,
    };
}
