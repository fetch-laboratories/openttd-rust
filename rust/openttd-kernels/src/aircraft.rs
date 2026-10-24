/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
//! Complete aircraft controllers and canonical private/airport-block ownership.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::struct_field_names,
    clippy::verbose_bit_mask,
    clippy::if_not_else, // Preserve the reference controller branch order.
    clippy::manual_midpoint // Source int bounds are finite map heights.
)]
#[repr(C)]
pub struct State {
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
/// `AIR_SHADOW` from `aircraft.h`.
pub(crate) const AIR_SHADOW: u8 = 4;

use crate::airport::{
    AT_OILRIG, ENDLANDING, ENDTAKEOFF, FLYING, HANGAR, HELIENDLANDING, HELILANDING, HELITAKEOFF,
    LANDING, STARTTAKEOFF, TAKEOFF,
};
use crate::company_type::OWNER_NONE;
use crate::newgrf_sound::VSE_START;
use crate::order_type::{
    OT_CONDITIONAL, OT_GOTO_DEPOT, OT_GOTO_STATION, OT_LEAVESTATION, OT_LOADING, OT_NOTHING,
};
use crate::random_func::chance16_i;
use crate::services::{Map, Services};
use crate::station::Station;
use crate::station_type::INVALID_STATION;
use crate::tile_type::INVALID_TILE;
use crate::vehicle::PROP_AIRCRAFT_SPEED;
use crate::vehicle::maintenance;
use crate::vehicle::{
    AltitudeInputs, Coordinates, DayCost, DirectionProgress, MovementObservations, OpenTTDVehicle,
    PositionObservations, SpeedInputs, TickStatus, VehicleRef, VehicleServices, XY, YEAR_TICKS,
};
use crate::vehicle_base::{VS_AIRCRAFT_BROKEN, VS_CRASHED, VS_HIDDEN, VS_STOPPED};
use std::cell::Cell;
use std::ffi::c_void;
use std::ptr::NonNull;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vehicle {
    pub handle: *mut OpenTTDVehicle,
    pub state: *mut State,
    pub id: u32,
}
// SAFETY for these named loads: descriptors name live aligned canonical members
// under the game-state mutex. Each read ends here, before any native callback.
impl PositionObservations {
    fn subtype(&self) -> i64 {
        i64::from(unsafe { self.subtype.read() })
    }
    fn z(&self) -> i64 {
        i64::from(unsafe { self.z.read() })
    }
}
impl MovementObservations {
    fn x(&self) -> i64 {
        i64::from(unsafe { self.x.read() })
    }
    fn y(&self) -> i64 {
        i64::from(unsafe { self.y.read() })
    }
    fn direction(&self) -> i64 {
        i64::from(unsafe { self.direction.read().0 })
    }
    fn speed(&self) -> i64 {
        i64::from(unsafe { self.speed.read() })
    }
    fn xy(&self) -> XY {
        XY {
            x: self.x() as i32,
            y: self.y() as i32,
        }
    }
}
impl Vehicle {
    fn base(self) -> VehicleRef {
        // SAFETY: Original live-handle precondition holds for every native service.
        VehicleRef(unsafe { NonNull::new_unchecked(self.handle) })
    }
    fn invalid() -> Self {
        Self {
            handle: std::ptr::null_mut(),
            state: std::ptr::null_mut(),
            id: 1_048_575,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Node {
    pub next: *const c_void,
    pub blocks: u64,
    pub position: u8,
    pub next_position: u8,
    pub heading: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Moving {
    pub x: i16,
    pub y: i16,
    pub flags: u16,
    pub direction: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockNode {
    pub blocks: u64,
    pub position: u8,
    pub next_position: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RouteNode {
    pub next: *const c_void,
    pub next_position: u8,
    pub heading: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockChoice {
    pub next: *const c_void,
    pub blocks: u64,
    pub heading: u8,
}
struct Movement {
    x: i64,
    y: i64,
    flags: i64,
    direction: i64,
}
struct NewPosition {
    x: i64,
    y: i64,
    tile: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Geometry {
    pub tile: u32,
    pub rotation: u8,
    pub width: u16,
    pub height: u16,
    pub airport: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LayoutRoute {
    pub node: *const c_void,
    pub next: *const c_void,
    pub next_position: u8,
    pub heading: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HangarCandidate {
    pub owner: u8,
    pub airport: bool,
    pub hangar: bool,
}
#[repr(C)]
pub struct Leaves {
    pub geometry: unsafe extern "C" fn(*const Station) -> Geometry,
    pub layout_blocks: unsafe extern "C" fn(*const c_void, u8) -> u64,
    pub layout_heading: unsafe extern "C" fn(*const c_void, u8) -> u8,
    pub layout_next_position: unsafe extern "C" fn(*const c_void, u8) -> u8,
    pub layout_next: unsafe extern "C" fn(*const c_void, u8) -> *const c_void,
    pub layout_route: unsafe extern "C" fn(*const c_void, u8) -> LayoutRoute,
    pub vehicle: *const VehicleServices,
    pub shared: *const Services,
    pub next: unsafe extern "C" fn(Vehicle) -> Vehicle,
    pub station: unsafe extern "C" fn(u16) -> *const Station,
    pub airport_tile: unsafe extern "C" fn(*const Station) -> u32,
    pub station_tile: unsafe extern "C" fn(*const Station) -> u32,
    pub rotation: unsafe extern "C" fn(*const Station) -> u8,
    pub airport_type: unsafe extern "C" fn(*const Station) -> u8,
    pub station_owner: unsafe extern "C" fn(*const Station) -> u8,
    pub airport_blocks: unsafe extern "C" fn(*const Station) -> *mut u64,
    pub had_vehicle: unsafe extern "C" fn(*const Station) -> u8,
    pub airport_elements: unsafe extern "C" fn(*const c_void) -> u8,
    pub helipads: unsafe extern "C" fn(*const c_void) -> u8,
    pub airport_flags: unsafe extern "C" fn(*const c_void) -> u8,
    pub airport_delta_z: unsafe extern "C" fn(*const c_void) -> u8,
    pub node: unsafe extern "C" fn(*const c_void, u8) -> *const c_void,
    pub fta: unsafe extern "C" fn(*const c_void) -> Node,
    pub moving: unsafe extern "C" fn(*const c_void, u8) -> Moving,
    pub engine_speed: unsafe extern "C" fn(Vehicle) -> u16,
    pub engine_subtype: unsafe extern "C" fn(Vehicle) -> u8,
    pub engine_sound: unsafe extern "C" fn(Vehicle) -> u16,
    pub airport_entry: unsafe extern "C" fn(*const c_void, u8) -> u8,
    pub direction_towards: unsafe extern "C" fn(Vehicle, i32, i32) -> u8,
    pub hangar_height:
        unsafe extern "C" fn(*const Station, *const crate::station_cmd::AirportSpec) -> i32,
    pub terminal_count: unsafe extern "C" fn(*const c_void, u32) -> u8,
    pub hangar_exit:
        unsafe extern "C" fn(*const Station, u32, *const crate::station_cmd::AirportSpec) -> u8,
    pub service_interval: unsafe extern "C" fn(Vehicle) -> u16,
    pub chain_in_depot: unsafe extern "C" fn(Vehicle) -> bool,
    pub nearest_depot_order: unsafe extern "C" fn(Vehicle) -> bool,
    pub part_of_orders: unsafe extern "C" fn(Vehicle) -> bool,
    pub next_aircraft: unsafe extern "C" fn(u32) -> Vehicle,
    pub viewport: unsafe extern "C" fn(Vehicle),
    pub rotor_image: unsafe extern "C" fn(Vehicle),
    pub copy_sprite: unsafe extern "C" fn(VehicleRef, VehicleRef),
    pub create_effect: unsafe extern "C" fn(Vehicle, u8, i32, i32, i32),
    pub truncate_cargo: unsafe extern "C" fn(Vehicle),
    pub crash_news: unsafe extern "C" fn(Vehicle, u16, u32),
    pub station_rating: unsafe extern "C" fn(u32, u8, i32, u32),
    pub landing_rating: unsafe extern "C" fn(u16, u8),
    pub dirty_depot: unsafe extern "C" fn(Vehicle),
    pub first_arrival: unsafe extern "C" fn(Vehicle, u16),
    pub update_delta: unsafe extern "C" fn(Vehicle),

    pub destination_too_far: unsafe extern "C" fn(Vehicle),
    pub delete_range_news: unsafe extern "C" fn(Vehicle),
    pub service_order: unsafe extern "C" fn(Vehicle, u16),
    pub running_cost: unsafe extern "C" fn(Vehicle) -> i64,
    pub subtract_cost: unsafe extern "C" fn(Vehicle, i64),
    pub dirty_lists: unsafe extern "C" fn(Vehicle),
    pub next_stopping_station: unsafe extern "C" fn(Vehicle) -> u16,
    pub remove_depot_orders: unsafe extern "C" fn(u16),
    pub assert_flying: unsafe extern "C" fn(Vehicle),
    pub invalid_movement: unsafe extern "C" fn(Vehicle),
    pub invalid_position: unsafe extern "C" fn(Vehicle, *const c_void),
    pub invalid_scheme: unsafe extern "C" fn(Vehicle),
    pub unreachable: unsafe extern "C" fn(Vehicle),
    pub cargo_age_default: unsafe extern "C" fn(Vehicle) -> u16,
    pub range_default: unsafe extern "C" fn(Vehicle) -> u16,
    pub rotor_image_if_changed: unsafe extern "C" fn(Vehicle) -> bool,
    pub update_rotor_image: unsafe extern "C" fn(Vehicle),
    pub vehicle_crash: unsafe extern "C" fn(Vehicle, bool) -> u32,
    pub send_to_depot: unsafe extern "C" fn(Vehicle, bool) -> bool,
    pub fta_next_position: unsafe extern "C" fn(*const c_void) -> u8,
    pub block_node: unsafe extern "C" fn(*const c_void) -> BlockNode,
    pub route_node: unsafe extern "C" fn(*const c_void) -> RouteNode,
    pub block_choice: unsafe extern "C" fn(*const c_void) -> BlockChoice,
}
struct AirportAccess {
    station: *const Station,
    blocks: Cell<Option<*mut u64>>,
}
impl AirportAccess {
    fn new(station: *const Station) -> Self {
        Self {
            station,
            blocks: Cell::new(None),
        }
    }
    fn pointer<const COUNTED: bool>(&self, world: &World<'_, COUNTED>) -> *mut u64 {
        self.blocks.get().unwrap_or_else(|| {
            let pointer = world.airport_blocks(self.station);
            self.blocks.set(Some(pointer));
            pointer
        })
    }
    fn read<const COUNTED: bool>(&self, world: &World<'_, COUNTED>) -> u64 {
        // SAFETY: One field-sized access to the live canonical mask, ending
        // before any native callback. Reentry observes/mutates the same scalar.
        unsafe { self.pointer(world).read() }
    }
    fn reserve<const COUNTED: bool>(&self, world: &World<'_, COUNTED>, bits: u64) {
        let pointer = self.pointer(world);
        // SAFETY: Serial game-state interval; no Rust world reference is formed.
        unsafe {
            pointer.write(pointer.read() | bits);
        }
    }
    fn release<const COUNTED: bool>(&self, world: &World<'_, COUNTED>, bits: u64) {
        let pointer = self.pointer(world);
        // SAFETY: Same short canonical scalar access as reserve.
        unsafe {
            pointer.write(pointer.read() & !bits);
        }
    }
}
struct World<'a, const COUNTED: bool> {
    leaves: &'a Leaves,
    vs: &'a VehicleServices,
    ss: &'a Services,
    movement: crate::movement::CachedTable<'a>,
    map: Cell<Option<Map>>,
    map_bounds: Cell<Option<(u32, u32)>>,
}
const SAMPLE_COUNT: i64 = 73;
const HELICOPTER_SOUND: i64 = 22;
const EXPLOSION_SOUND: i64 = 16;
const SKID_SOUND: i64 = 21;
/// `AircraftSubType::AIR_HELICOPTER` from `aircraft.h`.
pub const AIR_HELICOPTER: u8 = 0;
/// `AircraftSubType::AIR_AIRCRAFT` from `aircraft.h`.
pub const AIR_AIRCRAFT: u8 = 2;
const NOTHING: u64 = 1 << 30;
const ZEPPELIN: u64 = 1 << 62;
const CLOSED: u64 = 1 << 63;
const NO_CLAMP: i64 = 1;
const TAKING_OFF: i64 = 2;
const SLOW_TURN: i64 = 4;
const LAND: i64 = 8;
const EXACT: i64 = 16;
const BRAKE: i64 = 32;
const HELI_RAISE: i64 = 64;
const HELI_LOWER: i64 = 128;
const HOLD: i64 = 256;
macro_rules! private_fields {
    ($(($get:ident,$set:ident,$field:ident,$type:ty)),* $(,)?)=>{$(
        fn $get(&self,id:Vehicle)->$type {
            crate::raw_field::read_field!(id.state, $field)
        }
        fn $set(&self,id:Vehicle,value:$type) {
            crate::raw_field::write_field!(id.state, $field, value);
        }
    )*};
}
/* These macros emit one named synchronous function for each typed callback.
 * Widening preserves C++ integer promotions; narrowing occurs at field writes.
 * Neither macro selects an operation at runtime or copies the callback table. */
macro_rules! scalar_services {
    ($($name:ident($receiver:ident; $($arg:ident : $ty:ty),*) => $call:expr;)*) => {$(
        fn $name(&$receiver, $($arg:$ty),*) -> i64 {
            // SAFETY: No canonical owner borrow survives this noexcept callback.
            let result = $call;
            Self::service_witness(stringify!($name), std::mem::size_of_val(&result));
            i64::from(result)
        }
    )*};
}
macro_rules! direct_services {
    ($($name:ident($receiver:ident; $($arg:ident : $ty:ty),*) -> $ret:ty => $call:expr;)*) => {$(
        fn $name(&$receiver, $($arg:$ty),*) -> $ret {
            // SAFETY: No canonical owner borrow survives this noexcept callback.
            let result = $call;
            Self::service_witness(stringify!($name), std::mem::size_of_val(&result));
            result
        }
    )*};
}
fn can_use_station(v: Vehicle, st: *const Station) -> i64 {
    i64::from(unsafe {
        crate::vehicle_func::can_vehicle_use_station(
            v.handle,
            st.cast(),
            crate::newgrf_airport::runtime(),
        )
    })
}
impl<const COUNTED: bool> World<'_, COUNTED> {
    /// Count the actual boundary return type before integer promotion. The only
    /// production enable call is `WitnessWriter`'s process-start constructor;
    /// callbacks cannot change it during this synchronous World invocation.
    #[inline]
    fn service_witness(name: &str, bytes: usize) {
        use crate::witness;
        if !COUNTED {
            return;
        }
        witness::hit(witness::AIRCRAFT_SERVICES);
        witness::add(witness::AIRCRAFT_BYTES, bytes as u64);
        if matches!(
            name,
            "station" | "hangar_height" | "hangar_exit" | "landing_rating" | "first_arrival"
        ) {
            witness::hit(witness::AIRCRAFT_STATIONS);
        }
    }
    unsafe fn new(leaves: *const Leaves) -> Self {
        // SAFETY: Immutable tables outlive this synchronous entry. No copied
        // vehicle/station observations or owner borrows survive callbacks.
        let leaves = unsafe { &*leaves };
        Self {
            leaves,
            vs: unsafe { &*leaves.vehicle },
            ss: unsafe { &*leaves.shared },
            movement: crate::movement::CachedTable::new(unsafe { &*leaves.shared }),
            map: Cell::new(None),
            map_bounds: Cell::new(None),
        }
    }
    // Dimensions are runtime observations captured once when this entry first
    // consumes them. Game-state serialization and these callbacks exclude map
    // reset; nested entries obtain their own dimensions. No world state is cached.
    fn map(&self) -> Map {
        self.map.get().unwrap_or_else(|| {
            let log_x = (self.ss.map_log_x)();
            Self::service_witness("map_log_x", std::mem::size_of_val(&log_x));
            let map = Map::from_log_x(log_x);
            self.map.set(Some(map));
            map
        })
    }
    fn map_bounds(&self) -> (u32, u32) {
        self.map_bounds.get().unwrap_or_else(|| {
            let bounds = (self.ss.map_max_x)();
            Self::service_witness("map_max_x", 4);
            let y = (self.ss.map_max_y)();
            Self::service_witness("map_max_y", 4);
            self.map_bounds.set(Some((bounds, y)));
            (bounds, y)
        })
    }
    fn map_max_x(&self) -> i64 {
        i64::from(self.map_bounds().0)
    }
    fn map_max_y(&self) -> i64 {
        i64::from(self.map_bounds().1)
    }
    fn maintenance(&self) -> &maintenance::Services {
        // SAFETY: The base vehicle table names its immutable maintenance table.
        unsafe { &*self.vs.maintenance }
    }
    // Direct Rust maintenance; no owner borrow spans these calls.
    fn handle_breakdown(&self, v: Vehicle) -> i64 {
        i64::from(unsafe { maintenance::handle_breakdown_status(v.base(), self.vs) }.status)
    }
    fn needs_service(&self, v: Vehicle) -> i64 {
        i64::from(unsafe { maintenance::needs_automatic_servicing(v.base(), self.vs) })
    }
    fn service_in_depot(&self, v: Vehicle) {
        unsafe { maintenance::service_in_depot(v.base(), self.vs) };
    }
    fn age_vehicle(&self, v: Vehicle) {
        unsafe { maintenance::age_vehicle(v.base(), self.vs) };
    }
    fn decrease_value(&self, v: Vehicle) {
        unsafe { maintenance::decrease_vehicle_value(v.base(), self.vs) };
    }
    fn check_breakdown(&self, v: Vehicle) {
        unsafe { maintenance::check_vehicle_breakdown(v.base(), self.vs) };
    }
    fn process_orders(&self, v: Vehicle) {
        let orders = self.vs.orders();
        Self::service_witness(
            "orders",
            std::mem::size_of::<*const crate::orders::Leaves>(),
        );
        // SAFETY: The original live shell survives orders; no state borrow spans reentry.
        unsafe { crate::orders::openttd_rust_process_orders(v.handle.cast(), orders) };
    }
    fn random(&self) -> u32 {
        let word = self.ss.random();
        Self::service_witness("random", 4);
        word
    }

    scalar_services! {
        subtype(self; v: Vehicle) => (self.vs.subtype)(v.base());
        x(self; v: Vehicle) => (self.vs.x)(v.base());
        y(self; v: Vehicle) => (self.vs.y)(v.base());
        z(self; v: Vehicle) => (self.vs.z)(v.base());
        tile(self; v: Vehicle) => (self.vs.tile)(v.base());
        direction(self; v: Vehicle) => (self.vs.direction)(v.base());
        tick_counter(self; v: Vehicle) => (self.vs.tick)(v.base());
        owner(self; v: Vehicle) => (self.vs.owner)(v.base());
        vehicle_status(self; v: Vehicle) => (self.vs.status)(v.base());
        current_speed(self; v: Vehicle) => (self.vs.speed)(v.base());
        subspeed(self; v: Vehicle) => (self.vs.subspeed)(v.base());
        order_type(self; v: Vehicle) => (self.vs.order_type)(v.base());
        order_destination(self; v: Vehicle) => (self.vs.order_destination)(v.base());
        running_ticks(self; v: Vehicle) => (self.vs.running)(v.base());
        order_time(self; v: Vehicle) => (self.vs.order_time)(v.base());
        last_station(self; v: Vehicle) => (self.vs.last_station)(v.base());
        plane_speed(self; ) => (self.ss.plane_speed)();
        no_jetcrash(self; ) => (self.ss.no_jetcrash)();
        plane_crashes(self; ) => (self.ss.plane_crashes)();
        service_at_helipad(self; ) => (self.ss.service_at_helipad)();
        disaster_sound(self; ) => (self.ss.disaster_sound)();
        economy_date(self; ) => (self.ss.economy_date)();
        calendar_date(self; ) => (self.ss.calendar_date)();
        airport_tile(self; s: *const Station) => unsafe { (self.leaves.airport_tile)(s) };
        station_tile(self; s: *const Station) => unsafe { (self.leaves.station_tile)(s) };
        rotation(self; s: *const Station) => unsafe { (self.leaves.rotation)(s) };
        airport_type(self; s: *const Station) => unsafe { (self.leaves.airport_type)(s) };
        station_owner(self; s: *const Station) => unsafe { (self.leaves.station_owner)(s) };
        had_vehicle(self; s: *const Station) => unsafe { (self.leaves.had_vehicle)(s) };
        airport_elements(self; airport: *const c_void) => unsafe { (self.leaves.airport_elements)(airport) };
        helipads(self; airport: *const c_void) => unsafe { (self.leaves.helipads)(airport) };
        airport_flags(self; airport: *const c_void) => unsafe { (self.leaves.airport_flags)(airport) };
        airport_delta_z(self; airport: *const c_void) => unsafe { (self.leaves.airport_delta_z)(airport) };
        engine_speed(self; v: Vehicle) => unsafe { (self.leaves.engine_speed)(v) };
        engine_subtype(self; v: Vehicle) => unsafe { (self.leaves.engine_subtype)(v) };
        engine_sound(self; v: Vehicle) => unsafe { (self.leaves.engine_sound)(v) };
        engine_reliability(self; v: Vehicle) => (self.maintenance().engine_reliability)(v.base());
        vehicle_type(self; v: Vehicle) => (self.vs.vehicle_type)(v.base());
        slope(self; x: i64, y: i64) => self.movement.get().pixel_z_at(x as i32,y as i32,false);
        // Original unsigned TileHeight * TILE_HEIGHT narrows to int before widening.
        tile_height(self; tile: i64) => (self.ss.tile_height)(tile as u32).wrapping_mul(crate::tile_type::TILE_HEIGHT) as i32;
        airport_entry(self; ap: *const c_void, direction: i64) => unsafe { (self.leaves.airport_entry)(ap, direction as u8) };
        direction_towards(self; v: Vehicle, x: i64, y: i64) => unsafe { (self.leaves.direction_towards)(v, x as i32, y as i32) };
        terminal_count(self; ap: *const c_void, index: i64) => unsafe { (self.leaves.terminal_count)(ap, index as u32) };
        service_interval(self; v: Vehicle) => unsafe { (self.leaves.service_interval)(v) };
        chain_in_depot(self; v: Vehicle) => unsafe { (self.leaves.chain_in_depot)(v) };
        waiting_unbunching(self; v: Vehicle) => (self.vs.waiting_for_unbunching)(v.base());
        nearest_depot_order(self; v: Vehicle) => unsafe { (self.leaves.nearest_depot_order)(v) };
        part_of_orders(self; v: Vehicle) => unsafe { (self.leaves.part_of_orders)(v) };
        next_stopping_station(self; vehicle: Vehicle) => unsafe { (self.leaves.next_stopping_station)(vehicle) };
        speed_property(self; v: Vehicle) => (self.vs.property)((self.vs.newgrf)(), v.base(), PROP_AIRCRAFT_SPEED, 0) as u32;
        cargo_age_property(self; v: Vehicle) => (self.vs.property)((self.vs.newgrf)(), v.base(), crate::newgrf_properties::PROP_AIRCRAFT_CARGO_AGE_PERIOD, i32::from(unsafe { (self.leaves.cargo_age_default)(v) })) as u32;
        range_property(self; v: Vehicle) => (self.vs.property)((self.vs.newgrf)(), v.base(), crate::newgrf_properties::PROP_AIRCRAFT_RANGE, i32::from(unsafe { (self.leaves.range_default)(v) })) as u32;
        start_sound(self; v: Vehicle) => (self.vs.play_vehicle_sound)(v.base(), VSE_START);
        rotor_image_if_changed(self; v: Vehicle) => unsafe { (self.leaves.rotor_image_if_changed)(v) };
        vehicle_crash(self; v: Vehicle, flooded: i64) => unsafe { (self.leaves.vehicle_crash)(v, flooded != 0) };
        send_to_depot(self; v: Vehicle, service: i64) => unsafe { (self.leaves.send_to_depot)(v, service != 0) };
    }
    direct_services! {
        movement_observations(self; v: Vehicle) -> MovementObservations => (self.vs.movement_observations)(v.base());
        position_observations(self; v: Vehicle) -> PositionObservations => (self.vs.position_observations)(v.base());
        maximum_speed(self; v: Vehicle) -> u16 => (self.vs.max_speed)(v.base());
        altitude_inputs(self; v: Vehicle) -> AltitudeInputs => (self.vs.altitude_inputs)(v.base());
        geometry(self; s: *const Station) -> Geometry => unsafe { (self.leaves.geometry)(s) };
        layout_blocks(self; ap: *const c_void, pos: u8) -> u64 => unsafe { (self.leaves.layout_blocks)(ap, pos) };
        layout_heading(self; ap: *const c_void, pos: u8) -> u8 => unsafe { (self.leaves.layout_heading)(ap, pos) };
        layout_next_position(self; ap: *const c_void, pos: u8) -> u8 => unsafe { (self.leaves.layout_next_position)(ap, pos) };
        layout_next(self; ap: *const c_void, pos: u8) -> *const c_void => unsafe { (self.leaves.layout_next)(ap, pos) };
        layout_route(self; ap: *const c_void, pos: u8) -> LayoutRoute => unsafe { (self.leaves.layout_route)(ap, pos) };
        write_coordinates(self; v: Vehicle, x: i64, y: i64, z: i64) -> () => (self.vs.write_coordinates)(v.base(), x as i32, y as i32, z as i32);
        coordinates(self; v: Vehicle) -> Coordinates => (self.vs.coordinates)(v.base());
        xy(self; v: Vehicle) -> XY => (self.vs.xy)(v.base());
        speed_inputs(self; v: Vehicle) -> SpeedInputs => (self.vs.speed_inputs)(v.base());
        direction_progress(self; v: Vehicle) -> DirectionProgress => (self.vs.direction_progress)(v.base());
        tick_status(self; v: Vehicle) -> TickStatus => (self.vs.tick_status)(v.base());
        day_cost(self; v: Vehicle) -> DayCost => (self.vs.day_cost)(v.base());

        set_x(self; v: VehicleRef, value: i64) -> () => (self.vs.write_x)(v, value as i32);
        set_y(self; v: VehicleRef, value: i64) -> () => (self.vs.write_y)(v, value as i32);
        set_base_z(self; v: VehicleRef, value: i64) -> () => (self.vs.write_z)(v, value as i32);
        set_z(self; v: Vehicle, value: i64) -> () => (self.vs.write_z)(v.base(), value as i32);
        set_tile(self; v: Vehicle, value: i64) -> () => (self.vs.write_tile)(v.base(), value as u32);
        set_direction(self; v: Vehicle, value: i64) -> () => (self.vs.write_direction)(v.base(), value as u8);
        set_tick_counter(self; v: Vehicle, value: i64) -> () => (self.vs.write_tick)(v.base(), value as u8);
        set_vehicle_status(self; v: Vehicle, value: i64) -> () => (self.vs.write_status)(v.base(), value as u8);
        set_current_speed(self; v: Vehicle, value: i64) -> () => (self.vs.write_speed)(v.base(), value as u16);
        set_subspeed(self; v: Vehicle, value: i64) -> () => (self.vs.write_subspeed)(v.base(), value as u8);
        set_progress(self; v: Vehicle, value: i64) -> () => (self.vs.write_progress)(v.base(), value as u8);
        set_maximum_speed(self; v: Vehicle, value: i64) -> () => (self.vs.write_max_speed)(v.base(), value as u16);
        set_breakdown_counter(self; v: Vehicle, value: i64) -> () => (self.vs.write_breakdown_counter)(v.base(), value as u8);
        set_running_ticks(self; v: Vehicle, value: i64) -> () => (self.vs.write_running)(v.base(), value as u8);
        set_order_time(self; v: Vehicle, value: i64) -> () => (self.vs.write_order_time)(v.base(), value as i32);
        set_day_counter(self; v: Vehicle, value: i64) -> () => (self.vs.write_day)(v.base(), value as u8);
        set_profit(self; v: Vehicle, value: i64) -> () => (self.vs.write_profit)(v.base(), value as i64);
        set_last_station(self; v: Vehicle, value: i64) -> () => (self.vs.write_last_station)(v.base(), value as u16);
        set_economy_service(self; v: Vehicle, value: i64) -> () => (self.vs.write_economy_service)(v.base(), value as i32);
        set_calendar_service(self; v: Vehicle, value: i64) -> () => (self.vs.write_calendar_service)(v.base(), value as i32);
        set_breakdowns(self; v: Vehicle, value: i64) -> () => (self.vs.write_breakdowns)(v.base(), value as u8);
        set_reliability(self; v: Vehicle, value: i64) -> () => (self.vs.write_reliability)(v.base(), value as u16);
        set_cargo_age(self; v: Vehicle, value: i64) -> () => (self.vs.write_cargo_age)(v.base(), value as u16);
        airport_blocks(self; s: *const Station) -> *mut u64 => unsafe { (self.leaves.airport_blocks)(s) };
        node(self; airport: *const c_void, pos: u8) -> *const c_void => unsafe { (self.leaves.node)(airport, pos) };
        fta(self; node: *const c_void) -> Node => unsafe { (self.leaves.fta)(node) };
        block_choice(self; node: *const c_void) -> BlockChoice => unsafe { (self.leaves.block_choice)(node) };
        route_node(self; node: *const c_void) -> RouteNode => unsafe { (self.leaves.route_node)(node) };
        block_node(self; node: *const c_void) -> BlockNode => unsafe { (self.leaves.block_node)(node) };
        fta_next_position(self; node: *const c_void) -> u8 => unsafe { (self.leaves.fta_next_position)(node) };
        update_position(self; vehicle: Vehicle) -> () => unsafe { crate::vehicle::occupancy::update_position(self.vs, vehicle.base(), self.map()); (self.leaves.viewport)(vehicle); };
        rotor_image(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.rotor_image)(vehicle) };
        copy_sprite(self; vehicle: VehicleRef, source: VehicleRef) -> () => unsafe { (self.leaves.copy_sprite)(vehicle, source) };
        position_next(self; vehicle: VehicleRef) -> Option<VehicleRef> => (self.vs.next)(vehicle);
        position_y(self; vehicle: VehicleRef) -> i32 => (self.vs.y)(vehicle);
        position_coordinates(self; vehicle: VehicleRef, x: i64, y: i64, z: i64) -> () => (self.vs.write_coordinates)(vehicle, x as i32, y as i32, z as i32);
        position_viewport(self; vehicle: VehicleRef) -> () => unsafe { crate::vehicle::occupancy::update_position(self.vs, vehicle, self.map()); (self.vs.base_viewport)(vehicle); };
        create_effect(self; vehicle: Vehicle, kind: i64, x: i64, y: i64, z: i64) -> () => unsafe { (self.leaves.create_effect)(vehicle, kind as u8, x as i32, y as i32, z as i32) };
        dirty_start_stop(self; vehicle: Vehicle) -> () => (self.vs.start_stop_dirty)(vehicle.base());
        play_sound(self; vehicle: Vehicle, sound: i64) -> () => (self.vs.play_sound)(vehicle.base(), sound as u16);
        truncate_cargo(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.truncate_cargo)(vehicle) };
        crash_news(self; vehicle: Vehicle, station: i64, victims: i64) -> () => unsafe { (self.leaves.crash_news)(vehicle, station as u16, victims as u32) };
        landing_rating(self; station_id: i64, cargo: i64) -> () => unsafe { (self.leaves.landing_rating)(station_id as u16, cargo as u8) };
        free_order(self; vehicle: Vehicle) -> () => (self.vs.order_free)(vehicle.base());
        leave_unbunching(self; vehicle: Vehicle) -> () => (self.vs.leave_unbunching_depot)(vehicle.base());
        dirty_depot(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.dirty_depot)(vehicle) };
        first_arrival(self; vehicle: Vehicle, station: i64) -> () => unsafe { (self.leaves.first_arrival)(vehicle, station as u16) };
        begin_loading(self; vehicle: Vehicle) -> () => crate::vehicle::visits::begin_loading(vehicle.base(), self.vs.visits());
        dirty_details(self; vehicle: Vehicle) -> () => (self.maintenance().details_dirty)(vehicle.base());
        update_delta(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.update_delta)(vehicle) };

        destination_too_far(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.destination_too_far)(vehicle) };
        delete_range_news(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.delete_range_news)(vehicle) };
        handle_loading(self; vehicle: Vehicle, second: i64) -> () => self.vs.handle_loading(vehicle.base(), second != 0);
        service_order(self; vehicle: Vehicle, station: i64) -> () => unsafe { (self.leaves.service_order)(vehicle, station as u16) };
        dummy_order(self; vehicle: Vehicle) -> () => (self.vs.make_dummy)(vehicle.base());
        check_orders(self; vehicle: Vehicle) -> () => (self.vs.check_orders_only)(vehicle.base());
        running_cost(self; vehicle: Vehicle) -> i64 => unsafe { (self.leaves.running_cost)(vehicle) };
        subtract_cost(self; vehicle: Vehicle, cost: i64) -> () => unsafe { (self.leaves.subtract_cost)(vehicle, cost) };
        dirty_lists(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.dirty_lists)(vehicle) };
        remove_depot_orders(self; station_id: i64) -> () => unsafe { (self.leaves.remove_depot_orders)(station_id as u16) };
        assert_flying(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.assert_flying)(vehicle) };
        invalid_movement(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.invalid_movement)(vehicle) };
        invalid_position(self; vehicle: Vehicle, ap: *const c_void) -> () => unsafe { (self.leaves.invalid_position)(vehicle, ap) };
        invalid_scheme(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.invalid_scheme)(vehicle) };
        unreachable(self; vehicle: Vehicle) -> () => unsafe { (self.leaves.unreachable)(vehicle) };
        update_rotor_image(self; v: Vehicle) -> () => unsafe { (self.leaves.update_rotor_image)(v) };
        enter_depot(self; v: Vehicle) -> () => self.vs.enter_depot(v.base());
        delete_aircraft(self; v: Vehicle) -> () => (self.vs.destroy)(v.base());
    }

    fn next(&self, v: Vehicle) -> Vehicle {
        {
            let result = unsafe { (self.leaves.next)(v) };
            Self::service_witness("next", std::mem::size_of_val(&result));
            result
        }
    }
    fn station(&self, id: u32) -> *const Station {
        {
            let result = unsafe { (self.leaves.station)(id as u16) };
            Self::service_witness("station", std::mem::size_of_val(&result));
            result
        }
    }
    fn moving(&self, airport: *const c_void, pos: i64) -> Movement {
        // SAFETY: Synchronous noexcept service; no owner borrow spans this call.
        {
            let m = unsafe { (self.leaves.moving)(airport, pos as u8) };
            Self::service_witness("moving", std::mem::size_of_val(&m));
            Movement {
                x: i64::from(m.x),
                y: i64::from(m.y),
                flags: i64::from(m.flags),
                direction: i64::from(m.direction),
            }
        }
    }
    fn new_position(&self, v: Vehicle) -> NewPosition {
        // SAFETY: Synchronous noexcept service; no owner borrow spans this call.
        {
            let p = (self.vs.new_position)(v.base());
            Self::service_witness("new_position", std::mem::size_of_val(&p));
            NewPosition {
                x: i64::from(p.x),
                y: i64::from(p.y),
                tile: p.tile,
            }
        }
    }
    fn next_aircraft(&self, last: u32) -> Vehicle {
        {
            let result = unsafe { (self.leaves.next_aircraft)(last) };
            Self::service_witness("next_aircraft", std::mem::size_of_val(&result));
            result
        }
    }
    fn station_rating(&self, tile: i64, owner: i64, amount: i64, radius: i64) {
        Self::service_witness("station_rating", 0);
        // SAFETY: Synchronous noexcept service; no owner borrow spans this call.
        unsafe {
            (self.leaves.station_rating)(tile as u32, owner as u8, amount as i32, radius as u32);
        }
    }

    fn status(&self, id: Vehicle, mask: i64, on: bool) {
        let old = self.vehicle_status(id);
        self.set_vehicle_status(id, if on { old | mask } else { old & !mask });
    }
    private_fields!(
        (pos, set_pos, pos, u8),
        (previous, set_previous, previous_pos, u8),
        (state, set_state, state, u8),
        (target, set_target, targetairport, u16),
        (lastdir, set_lastdir, last_direction, u8),
        (turns, set_turns, number_consecutive_turns, u8),
        (turn, set_turn, turn_counter, u8),
        (flags, set_flags, flags, u8),
        (crashed, set_crashed, crashed_counter, u16),
        (range, set_range, cached_max_range, u16),
        (range_sqr, set_range_sqr, cached_max_range_sqr, u32)
    );
    fn flag(&self, id: Vehicle, bit: u8, on: bool) {
        let f = self.flags(id);
        self.set_flags(id, if on { f | (1 << bit) } else { f & !(1 << bit) });
    }
    fn target_station(&self, id: Vehicle) -> *const Station {
        self.station(u32::from(self.target(id)))
    }
    fn airport_spec(&self, st: *const Station) -> *const crate::station_cmd::AirportSpec {
        let tile = unsafe { (self.leaves.airport_tile)(st) };
        if tile == INVALID_TILE {
            crate::newgrf_airport::runtime().dummy_spec
        } else {
            crate::station_base::airport_get_spec(
                tile,
                unsafe { (self.leaves.airport_type)(st) },
                crate::newgrf_airport::runtime(),
            )
        }
    }
    fn airport_fta(&self, st: *const Station) -> *const c_void {
        (crate::newgrf_airport::runtime().fta)(self.airport_spec(st))
    }
    fn dummy_airport() -> *const c_void {
        crate::airport::get_airport(crate::airport::AT_DUMMY, crate::newgrf_airport::runtime())
    }
    fn has_hangar(&self, st: *const Station) -> i64 {
        i64::from((crate::newgrf_airport::runtime().has_hangar)(
            st.cast(),
            self.airport_spec(st),
        ))
    }
    fn hangar_candidate(&self, st: *const Station, vehicle_owner: u8) -> HangarCandidate {
        let owner = self.station_owner(st) as u8;
        let runtime = crate::newgrf_airport::runtime();
        let airport = owner == vehicle_owner
            && unsafe { ((*runtime.commands().station).facilities)(st.cast_mut().cast()) }
                & crate::station_type::FACILITY_AIRPORT
                != 0;
        HangarCandidate {
            owner,
            airport,
            hangar: airport && self.has_hangar(st) != 0,
        }
    }
    fn hangar_height(&self, station: i64) -> i64 {
        let st =
            crate::newgrf_airport::station_by_id(station as u16, crate::newgrf_airport::runtime())
                .cast();
        let result = unsafe { (self.leaves.hangar_height)(st, self.airport_spec(st)) };
        Self::service_witness("hangar_height", std::mem::size_of_val(&result));
        i64::from(result)
    }
    fn hangar_exit(&self, v: Vehicle) -> i64 {
        let tile = (self.vs.tile)(v.base());
        let st = self.station(u32::from((self.ss.station_index)(tile)));
        let result = unsafe { (self.leaves.hangar_exit)(st, tile, self.airport_spec(st)) };
        Self::service_witness("hangar_exit", std::mem::size_of_val(&result));
        i64::from(result)
    }
    fn valid_airport(&self, id: Vehicle) -> bool {
        let s = self.target_station(id);
        !s.is_null() && self.airport_tile(s) as u32 != INVALID_TILE
    }
    fn airport(&self, id: Vehicle) -> *const c_void {
        let s = self.target_station(id);
        if s.is_null() {
            Self::dummy_airport()
        } else {
            self.airport_fta(s)
        }
    }
    fn block_owner(&self, station: u32) -> AirportAccess {
        AirportAccess::new(self.station(station))
    }
    fn blocks(&self, station: u32) -> u64 {
        self.block_owner(station).read(self)
    }
    fn tile_x(&self, t: u32) -> i64 {
        i64::from(self.map().tile_x(t))
    }
    fn tile_y(&self, t: u32) -> i64 {
        i64::from(self.map().tile_y(t))
    }
    fn tile_virt(&self, x: i64, y: i64) -> u32 {
        self.map().tile_virt_xy(x as i32 as u32, y as i32 as u32)
    }

    fn height(&self, id: Vehicle, observed: Option<&MovementObservations>) -> i64 {
        let point = observed.map_or_else(|| self.xy(id), MovementObservations::xy);
        let x = i64::from(point.x).clamp(0, self.map_max_x() * 16);
        let y = i64::from(point.y).clamp(0, self.map_max_y() * 16);
        self.tile_height(i64::from(self.tile_virt(x, y)))
    }
    fn bounds(&self, id: Vehicle, observed: Option<&MovementObservations>) -> (i64, i64) {
        let mut base = self.height(id, observed);
        if observed.map_or_else(|| self.subtype(id), |o| o.position.subtype()) == 0
            && self.vehicle_type(id) == 3
        {
            base += 34;
        }
        let max_speed = if let Some(o) = observed {
            if o.direction() <= 3 {
                base += 10;
            }
            self.maximum_speed(id)
        } else {
            let inputs = self.altitude_inputs(id);
            if inputs.direction <= 3 {
                base += 10;
            }
            inputs.max_speed
        };
        // C++ cached_max_speed is uint16; arithmetic promotes to signed int.
        base += (20 * (i64::from(max_speed) / 200) - 90).min(0);
        (base + 120, base + 360)
    }
    fn flight_flags(
        &self,
        id: Vehicle,
        ptr: *mut u8,
        takeoff: bool,
        observed: Option<&MovementObservations>,
    ) -> i64 {
        let (min, max) = self.bounds(id, observed);
        let middle = (min + max) / 2;
        let mut z = observed.map_or_else(|| self.z(id), |o| o.position.z());
        // SAFETY: Scalar pointer belongs to the live aircraft/disaster owner for this synchronous access.
        let mut flags = unsafe { ptr.read() };
        if z < min || (flags & 4 != 0 && z < middle) {
            flags |= 4;
            z += if takeoff { 2 } else { 1 };
        } else if !takeoff && (z > max || (flags & 2 != 0 && z > middle)) {
            flags |= 2;
            z -= 1;
        } else if flags & 4 != 0 && z >= middle {
            flags &= !4;
        } else if flags & 2 != 0 && z <= middle {
            flags &= !2;
        }
        // SAFETY: All shared callbacks above completed before this raw scalar write.
        unsafe {
            ptr.write(flags);
        }
        z
    }
    fn flight(&self, id: Vehicle, takeoff: bool, observed: &MovementObservations) -> i64 {
        let ptr = id.state;
        // SAFETY: Computing a raw field address creates no reference or access scope.
        self.flight_flags(
            id,
            unsafe { &raw mut (*ptr).flags },
            takeoff,
            Some(observed),
        )
    }
    fn effect(&self, id: Vehicle, x: i64, y: i64, z: i64, kind: u32) {
        self.create_effect(id, i64::from(kind), x, y, z);
    }
    fn position(&self, id: Vehicle, x: i64, y: i64, z: i64) {
        let observed = self.position_observations(id);
        self.position_observed(id, &observed, x, y, z);
    }
    fn position_observed(
        &self,
        id: Vehicle,
        observed: &PositionObservations,
        x: i64,
        y: i64,
        z: i64,
    ) {
        self.write_coordinates(id, x, y, z);
        self.update_position(id);
        if observed.subtype() == 0 {
            self.rotor_image(id);
        }
        // Positioning consumes base shell services only. Do not resolve an
        // aircraft owner or ID for the shadow/rotor successors.
        // SAFETY: The original unconditionally dereferences the live shadow.
        let shadow = unsafe { self.position_next(id.base()).unwrap_unchecked() };
        let safe_x = x.clamp(0, self.map_max_x() * 16);
        let mut safe_y = (y - 1).clamp(0, self.map_max_y() * 16);
        self.set_x(shadow, x);
        self.set_y(
            shadow,
            y - ((observed.z() - self.slope(safe_x, safe_y)) >> 3),
        );
        safe_y = i64::from(self.position_y(shadow)).clamp(0, self.map_max_y() * 16);
        self.set_base_z(shadow, self.slope(safe_x, safe_y));
        self.copy_sprite(shadow, id.base());
        self.position_viewport(shadow);
        if let Some(rotor) = self.position_next(shadow) {
            self.position_coordinates(rotor, x, y, z + 5);
            self.position_viewport(rotor);
        }
    }
    fn position_current(&self, id: Vehicle, observed: Option<&MovementObservations>) {
        if let Some(o) = observed {
            self.position_observed(id, &o.position, o.x(), o.y(), o.position.z());
        } else {
            let p = self.coordinates(id);
            self.position(id, i64::from(p.x), i64::from(p.y), i64::from(p.z));
        }
    }
    fn enter_hangar(&self, id: Vehicle) {
        self.set_subspeed(id, 0);
        self.set_progress(id, 0);
        let shadow = self.next(id);
        self.status(shadow, i64::from(VS_HIDDEN), true);
        let rotor = self.next(shadow);
        if !rotor.handle.is_null() {
            self.status(rotor, i64::from(VS_HIDDEN), true);
            self.set_current_speed(rotor, 0);
        }
        self.position_current(id, None);
    }
    fn cache(&self, id: Vehicle, update_range: bool) {
        let speed = self.speed_property(id) as u32;
        self.set_maximum_speed(
            id,
            if speed != 0 {
                i64::from(speed.wrapping_mul(128) / 10)
            } else {
                self.engine_speed(id)
            },
        );
        self.set_cargo_age(id, self.cargo_age_property(id));
        let shadow = self.next(id);
        self.set_cargo_age(shadow, self.cargo_age_property(shadow));
        if update_range {
            let range = self.range_property(id) as u16;
            self.set_range(id, range);
            self.set_range_sqr(
                id,
                u32::from(self.range(id)).wrapping_mul(u32::from(self.range(id))),
            );
        }
    }
    fn helicopter_tick(&self, id: Vehicle) {
        let rotor = self.next(self.next(id));
        if self.vehicle_status(rotor) & i64::from(VS_HIDDEN) != 0 {
            return;
        }
        let mut speed = self.current_speed(rotor);
        if self.order_type(id) == i64::from(OT_LOADING)
            || self.vehicle_status(id) & i64::from(VS_STOPPED) != 0
        {
            if speed != 0 {
                speed = i64::from((speed as u16).wrapping_add(1));
                self.set_current_speed(rotor, speed);
                if speed >= 128 && self.state(rotor) == 3 {
                    speed = 0;
                    self.set_current_speed(rotor, speed);
                }
            }
        } else {
            if speed == 0 {
                speed = 112;
                self.set_current_speed(rotor, speed);
            }
            if speed >= 80 {
                speed -= 1;
                self.set_current_speed(rotor, speed);
            }
        }
        let tick = (self.tick_counter(rotor) as u8).wrapping_add(1);
        self.set_tick_counter(rotor, i64::from(tick));
        let speed = speed >> 4;
        if speed == 0 {
            self.set_state(rotor, 0);
            if self.rotor_image_if_changed(id) != 0 {
                return;
            }
        } else if i64::from(tick) >= speed {
            self.set_tick_counter(rotor, 0);
            let state = self.state(rotor).wrapping_add(1);
            self.set_state(rotor, if state > 3 { 1 } else { state });
            self.update_rotor_image(id);
        } else {
            return;
        }
        self.position_viewport(rotor.base());
    }
    fn speed(&self, id: Vehicle, mut limit: u32, mut hard: bool) -> u32 {
        let input = self.speed_inputs(id);
        let mut spd = u32::from(input.acceleration).wrapping_mul(77);
        let factor = self.plane_speed() as u32;
        limit = limit.wrapping_mul(factor);
        if i64::from(input.status) & i64::from(VS_AIRCRAFT_BROKEN) != 0 {
            if 320 < limit {
                hard = false;
            }
            limit = limit.min(320);
        }
        if (i64::from(input.max_speed) as u32) < limit {
            if (i64::from(input.speed) as u32) < limit {
                hard = false;
            }
            limit = i64::from(input.max_speed) as u32;
        }
        let old_sub = i64::from(input.subspeed) as u8;
        let sub = old_sub.wrapping_add(spd as u8);
        self.set_subspeed(id, i64::from(sub));
        let cur = i64::from(input.speed) as u32;
        if !hard && cur > limit {
            let reduction = ((cur as i32).wrapping_mul(cur as i32) / 16384) / factor as i32;
            limit = cur.wrapping_sub(reduction.max(1) as u32);
        }
        spd = (cur + (spd >> 8) + u32::from(sub < old_sub)).min(limit);
        if spd != cur {
            self.set_current_speed(id, i64::from(spd));
            self.dirty_start_stop(id);
        }
        if factor > 1 {
            spd /= factor;
        }
        let motion = self.direction_progress(id);
        if motion.direction & 1 == 0 {
            spd = spd.wrapping_mul(3) / 4;
        }
        spd = spd.wrapping_add(u32::from(motion.progress));
        self.set_progress(id, i64::from(spd as u8));
        spd >> 8
    }
    fn entry_tile(&self, station: *const Station) -> u32 {
        if station.is_null() {
            return 0;
        }
        let tile = self.airport_tile(station) as u32;
        if tile == INVALID_TILE {
            self.station_tile(station) as u32
        } else {
            tile
        }
    }
    fn entry_point(&self, id: Vehicle, ap: *const c_void, rotation: i64, tile: u32) -> u8 {
        let point = self.xy(id);
        let dx = i64::from(point.x) - self.tile_x(tile) * 16;
        let dy = i64::from(point.y) - self.tile_y(tile) * 16;
        let dir = if dy.abs() < dx.abs() {
            if dx < 0 { 0 } else { 2 }
        } else if dy < 0 {
            3
        } else {
            1
        };
        let rotated = (dir + (0_u8.wrapping_sub(rotation as u8 >> 1) & 3)) & 3;
        self.airport_entry(ap, i64::from(rotated)) as u8
    }
    fn next_airport(&self, id: Vehicle) {
        if matches!(self.order_type(id), kind if kind == i64::from(OT_GOTO_STATION) || kind == i64::from(OT_GOTO_DEPOT))
        {
            self.set_target(id, self.order_destination(id) as u16);
        }
        let s = self.target_station(id);
        let airport_tile = if s.is_null() {
            INVALID_TILE
        } else {
            self.airport_tile(s) as u32
        };
        let valid = airport_tile != INVALID_TILE;
        let tile = if s.is_null() {
            0
        } else if valid {
            airport_tile
        } else {
            self.station_tile(s) as u32
        };
        let ap = if valid {
            self.airport_fta(s)
        } else {
            Self::dummy_airport()
        };
        let rotation = if valid { self.rotation(s) } else { 0 };
        let pos = self.entry_point(id, ap, rotation, tile);
        self.set_pos(id, pos);
        self.set_previous(id, pos);
    }
    fn controller(&self, id: Vehicle) -> bool {
        // Acquired after event's orders/loading, never in deleting crashed_tick.
        // Geometry/layout/map, speed writes/dirtying, direction/new-position,
        // cache/NewGRF property, sound, position/viewport/rotor and MaybeCrash
        // callbacks preserve this shell and chain. They may reenter or allocate
        // other vehicles; pool growth does not move existing shells. MaybeCrash
        // marks a crash, but destruction occurs on a later crashed_tick only.
        // This descriptor ends before airport_next's state-handler/depot work;
        // no load/reset, replacement command or deleting callback is reachable.
        let observed = self.movement_observations(id);
        let s = self.target_station(id);
        let valid = !s.is_null();
        let mut tile = INVALID_TILE;
        let mut rotation = 0;
        let mut sx = 1;
        let mut sy = 1;
        let mut has_airport = false;
        if valid {
            let geometry = self.geometry(s);
            tile = geometry.tile;
            rotation = i64::from(geometry.rotation);
            sx = i64::from(geometry.width);
            sy = i64::from(geometry.height);
            has_airport = geometry.airport;
        }
        let ap = if tile == INVALID_TILE {
            Self::dummy_airport()
        } else {
            self.airport_fta(s)
        };
        if !has_airport {
            if i64::from(self.pos(id)) >= self.airport_elements(ap) {
                let pos = self.entry_point(id, ap, 0, if valid { tile } else { 0 });
                self.set_pos(id, pos);
                self.set_previous(id, pos);
            } else if i64::from(self.target(id)) != self.order_destination(id) {
                self.set_state(id, FLYING);
                self.cache(id, false);
                self.next_airport(id);
                self.position_observed(
                    id,
                    &observed.position,
                    observed.x(),
                    observed.y(),
                    self.flight(id, false, &observed),
                );
                return false;
            }
        }
        let mut amd = self.moving(ap, i64::from(self.pos(id)));
        let ox = amd.x;
        let oy = amd.y;
        amd.direction = (amd.direction + rotation) & 7;
        match rotation {
            0 => {}
            2 => {
                amd.x = oy;
                amd.y = i64::from((sy * 16 - ox - 1) as i16);
            }
            4 => {
                amd.x = i64::from((sx * 16 - ox - 1) as i16);
                amd.y = i64::from((sy * 16 - oy - 1) as i16);
            }
            6 => {
                amd.x = i64::from((sx * 16 - oy - 1) as i16);
                amd.y = ox;
            }
            _ => {
                self.unreachable(id);
            }
        }
        let flags = amd.flags;
        let mut x = self.tile_x(tile) * 16;
        let mut y = self.tile_y(tile) * 16;
        if flags & HELI_RAISE != 0 {
            let rotor = self.next(self.next(id));
            if self.current_speed(rotor) > 32 {
                self.set_current_speed(id, 0);
                self.set_current_speed(rotor, self.current_speed(rotor) - 1);
                if self.current_speed(rotor) == 32 && self.start_sound(id) == 0 {
                    let sound = self.engine_sound(id);
                    self.play_sound(
                        id,
                        if sound < SAMPLE_COUNT {
                            HELICOPTER_SOUND
                        } else {
                            sound
                        },
                    );
                }
            } else {
                self.set_current_speed(rotor, 32);
                let count = self.speed(id, 65535, true);
                if count > 0 {
                    self.set_tile(id, 0);
                    let dest = self.bounds(id, Some(&observed)).0;
                    if observed.position.z() >= dest {
                        self.set_current_speed(id, 0);
                        return true;
                    }
                    self.position_observed(
                        id,
                        &observed.position,
                        observed.x(),
                        observed.y(),
                        (observed.position.z() + i64::from(count)).min(dest),
                    );
                }
            }
            return false;
        }
        if flags & HELI_LOWER != 0 {
            self.flag(id, 3, true);
            if !valid {
                self.set_state(id, FLYING);
                self.cache(id, false);
                self.next_airport(id);
                return false;
            }
            if self.airport_type(s) != i64::from(AT_OILRIG) {
                x = observed.x();
                y = observed.y();
                tile = self.tile_virt(x, y);
            }
            self.set_tile(id, i64::from(tile));
            let z = self.slope(x, y) + 1 + self.airport_delta_z(ap);
            if z == observed.position.z() {
                let rotor = self.next(self.next(id));
                if self.current_speed(rotor) >= 80 {
                    self.flag(id, 3, false);
                    return true;
                }
                self.set_current_speed(rotor, self.current_speed(rotor) + 4);
            } else {
                let count = self.speed(id, 65535, true);
                if count > 0 {
                    let current = observed.position.z();
                    let dest = if current > z {
                        (current - i64::from(count)).max(z)
                    } else {
                        (current + i64::from(count)).min(z)
                    };
                    self.position_observed(
                        id,
                        &observed.position,
                        observed.x(),
                        observed.y(),
                        dest,
                    );
                }
            }
            return false;
        }
        let point = observed.xy();
        let dist = ((x + amd.x - i64::from(point.x)).abs() + (y + amd.y - i64::from(point.y)).abs())
            as u32;
        if flags & EXACT == 0 && dist <= if flags & SLOW_TURN != 0 { 8 } else { 4 } {
            return true;
        }
        if dist == 0 {
            let difference = (amd.direction - observed.direction()) & 7;
            if difference == 0 {
                self.set_current_speed(id, 0);
                return true;
            }
            if self.speed(id, 50, true) == 0 {
                return false;
            }
            self.set_direction(
                id,
                (observed.direction() + if difference > 4 { 7 } else { 1 }) & 7,
            );
            self.set_current_speed(id, observed.speed() >> 1);
            self.position_current(id, Some(&observed));
            return false;
        }
        if flags & BRAKE != 0 && observed.speed() > 50 * self.plane_speed() {
            self.maybe_crash(id);
            if self.vehicle_status(id) & i64::from(VS_CRASHED) != 0 {
                return false;
            }
        }
        let mut limit = 50;
        let mut hard = true;
        if flags & NO_CLAMP != 0 {
            limit = 65535;
        }
        if flags & HOLD != 0 {
            limit = 425;
            hard = false;
        }
        if flags & LAND != 0 {
            limit = 230;
            hard = false;
        }
        if flags & BRAKE != 0 {
            limit = 50;
            hard = false;
        }
        let mut count = self.speed(id, limit, hard);
        if count == 0 {
            return false;
        }
        let nudge = count + 3 > dist;
        if self.turn(id) != 0 {
            self.set_turn(id, self.turn(id) - 1);
        }
        loop {
            let px;
            let py;
            let next_tile;
            if nudge || flags & LAND != 0 {
                let point = observed.xy();
                let vx = i64::from(point.x);
                let vy = i64::from(point.y);
                px = vx + (x + amd.x - vx).signum();
                py = vy + (y + amd.y - vy).signum();
                next_tile = if self.airport_type(s) == i64::from(AT_OILRIG) {
                    self.airport_tile(s) as u32
                } else {
                    self.tile_virt(px, py)
                };
            } else {
                let newdir = self.direction_towards(id, x + amd.x, y + amd.y);
                if newdir != observed.direction() {
                    if flags & SLOW_TURN != 0
                        && self.turns(id) < 8
                        && observed.position.subtype() == 2
                    {
                        if self.turn(id) == 0 || newdir == i64::from(self.lastdir(id)) {
                            if newdir == i64::from(self.lastdir(id)) {
                                self.set_turns(id, 0);
                            } else {
                                self.set_turns(id, self.turns(id).wrapping_add(1));
                            }
                            self.set_turn(id, (2 * self.plane_speed()) as u8);
                            self.set_lastdir(id, observed.direction() as u8);
                            self.set_direction(id, newdir);
                        }
                        let gp = self.new_position(id);
                        px = gp.x;
                        py = gp.y;
                        next_tile = gp.tile;
                    } else {
                        self.set_current_speed(id, observed.speed() >> 1);
                        self.set_direction(id, newdir);
                        px = observed.x();
                        py = observed.y();
                        next_tile = self.tile(id) as u32;
                    }
                } else {
                    self.set_turns(id, 0);
                    let gp = self.new_position(id);
                    px = gp.x;
                    py = gp.y;
                    next_tile = gp.tile;
                }
            }
            self.set_tile(id, i64::from(next_tile));
            if flags & (TAKING_OFF | SLOW_TURN | LAND) != 0 {
                self.set_tile(id, 0);
            }
            let mut z = observed.position.z();
            if flags & TAKING_OFF != 0 {
                z = self.flight(id, true, &observed);
            } else if flags & HOLD != 0 {
                if z > self.height(id, Some(&observed))
                    + if observed.position.subtype() == 0 {
                        184
                    } else {
                        150
                    }
                {
                    z -= 1;
                }
            } else if flags & (SLOW_TURN | NO_CLAMP) == SLOW_TURN | NO_CLAMP {
                z = self.flight(id, false, &observed);
            }
            let mut airport_z = observed.position.z();
            if flags & (LAND | BRAKE) != 0 && valid {
                airport_z = self.hangar_height(i64::from(self.target(id))) + 1;
            }
            if flags & LAND != 0 {
                if self.blocks(u32::from(self.target(id))) & ZEPPELIN != 0 {
                    self.set_state(id, FLYING);
                    self.cache(id, false);
                    self.position_observed(
                        id,
                        &observed.position,
                        px,
                        py,
                        self.flight(id, false, &observed),
                    );
                    self.set_pos(id, self.previous(id));
                    count -= 1;
                    if count == 0 {
                        break;
                    }
                    continue;
                }
                if self.airport_tile(s) as u32 == INVALID_TILE {
                    self.set_state(id, FLYING);
                    self.cache(id, false);
                    self.next_airport(id);
                    self.position_observed(
                        id,
                        &observed.position,
                        px,
                        py,
                        self.flight(id, false, &observed),
                    );
                    count -= 1;
                    if count == 0 {
                        break;
                    }
                    continue;
                }
                let t = i64::from(dist.wrapping_sub(4).max(1));
                let delta = z - airport_z;
                if delta >= t {
                    z -= (z - airport_z + t - 1) / t;
                }
                if z < airport_z {
                    z = airport_z;
                }
            }
            if flags & BRAKE != 0 {
                z += (airport_z - z).signum();
            }
            self.position_observed(id, &observed.position, px, py, z);
            count -= 1;
            if count == 0 {
                break;
            }
        }
        false
    }
    fn crashed_tick(&self, id: Vehicle) -> bool {
        self.set_crashed(id, self.crashed(id).wrapping_add(3));
        let station = self.target_station(id);
        let valid = !station.is_null() && self.airport_tile(station) as u32 != INVALID_TILE;
        if self.crashed(id) < 500 && !valid && self.crashed(id).is_multiple_of(3) {
            let z = self.slope(
                self.x(id).clamp(0, self.map_max_x() * 16),
                self.y(id).clamp(0, self.map_max_y() * 16),
            );
            self.set_z(id, self.z(id) - 1);
            if self.z(id) <= z {
                self.set_crashed(id, 500);
                self.set_z(id, z + 1);
            } else {
                self.set_crashed(id, 0);
            }
            self.position_current(id, None);
        }
        if self.crashed(id) < 650 {
            let r = self.random();
            if chance16_i(1, 32, r) {
                let delta = [7, 0, 0, 1][((r >> 16) & 3) as usize];
                self.set_direction(id, (self.direction(id) + delta) & 7);
                self.position_current(id, None);
                let r = self.random();
                self.effect(
                    id,
                    i64::from((r & 15).wrapping_sub(4) as i32),
                    i64::from(((r >> 4) & 15).wrapping_sub(4) as i32),
                    i64::from((r >> 8) & 15),
                    7,
                );
            }
        } else if self.crashed(id) >= 10000 {
            if valid {
                AirportAccess::new(station).release(self, (1 << 8) | (1 << 29));
            }
            self.delete_aircraft(id);
            return false;
        }
        true
    }
    fn smoke(&self, id: Vehicle, mode: bool, status: i64) {
        if status & i64::from(VS_AIRCRAFT_BROKEN) == 0 {
            return;
        }
        if self.current_speed(id) < 10 {
            self.status(id, i64::from(VS_AIRCRAFT_BROKEN), false);
            self.set_breakdown_counter(id, 0);
            return;
        }
        if !mode && self.tick_counter(id) & 15 == 0 {
            let (x, y) = [
                (5, 5),
                (6, 0),
                (5, -5),
                (0, -6),
                (-5, -5),
                (-6, 0),
                (-5, 5),
                (0, 6),
            ][self.direction(id) as usize];
            self.effect(id, x, y, 2, 10);
        }
    }
    fn crash(&self, id: Vehicle, flooded: bool) -> u32 {
        let victims = (self.vehicle_crash(id, i64::from(flooded)) as u32).wrapping_add(2);
        self.set_crashed(id, if flooded { 9000 } else { 0 });
        victims
    }
    fn crash_airplane(&self, id: Vehicle) {
        self.effect(id, 4, 4, 8, 5);
        let victims = self.crash(id, false);
        self.truncate_cargo(id);
        self.truncate_cargo(self.next(id));
        let station = if self.valid_airport(id) {
            u32::from(self.target(id))
        } else {
            u32::from(INVALID_STATION)
        };
        self.crash_news(id, i64::from(station), i64::from(victims));
        let tile = self.tile_virt(
            self.x(id).clamp(0, self.map_max_x() * 16),
            self.y(id).clamp(0, self.map_max_y() * 16),
        );
        self.station_rating(i64::from(tile), self.owner(id), -160, 30);
        if self.disaster_sound() != 0 {
            self.play_sound(id, EXPLOSION_SOUND);
        }
    }
    fn maybe_crash(&self, id: Vehicle) {
        let ap = self.airport(id);
        let prob = if self.airport_flags(ap) & 4 != 0
            && self.engine_subtype(id) & 2 != 0
            && self.no_jetcrash() == 0
        {
            3276
        } else {
            let setting = self.plane_crashes();
            if setting == 0 {
                return;
            }
            (0x4000_u32.wrapping_shl(setting as u32)) / 1500
        };
        if self.random() & ((1 << 22) - 1) > prob {
            return;
        }
        for cargo in 0..i64::from(crate::cargo_type::NUM_CARGO) {
            self.landing_rating(i64::from(self.target(id)), cargo);
        }
        self.crash_airplane(id);
    }
    fn missing_orders(&self, id: Vehicle) {
        if !self.valid_airport(id) {
            // The source passes DepotCommandFlag{}, which is DepotCommandFlag::Service.
            if self.send_to_depot(id, 1) != 0 {
                self.crash_airplane(id);
            }
        } else if self.order_type(id) != i64::from(OT_GOTO_DEPOT) {
            self.free_order(id);
        }
    }
    fn leave_hangar(&self, id: Vehicle, direction: i64) {
        self.set_current_speed(id, 0);
        self.set_subspeed(id, 0);
        self.set_progress(id, 0);
        self.set_direction(id, direction);
        self.status(id, i64::from(VS_HIDDEN), false);
        let shadow = self.next(id);
        self.status(shadow, i64::from(VS_HIDDEN), false);
        let rotor = self.next(shadow);
        if !rotor.handle.is_null() {
            self.status(rotor, i64::from(VS_HIDDEN), false);
            self.set_current_speed(rotor, 80);
        }
        self.service_in_depot(id);
        self.leave_unbunching(id);
        self.position_current(id, None);
        self.dirty_depot(id);
    }
    fn terminal(&self, id: Vehicle) {
        if self.order_type(id) == i64::from(OT_GOTO_DEPOT) {
            return;
        }
        let station = u32::from(self.target(id));
        self.set_last_station(id, i64::from(station));
        if self.had_vehicle(self.station(station)) & 16 == 0 {
            self.first_arrival(id, i64::from(station));
        }
        self.begin_loading(id);
    }
    fn has_block(
        &self,
        id: Vehicle,
        current: *const c_void,
        ap: *const c_void,
        airport: &AirportAccess,
    ) -> bool {
        let reference = self.node(ap, self.pos(id));
        let c = self.block_node(current);
        let next = self.layout_blocks(ap, c.next_position);
        if self.layout_blocks(ap, c.position) != next {
            let mut blocks = next;
            if current != reference && c.blocks != NOTHING {
                blocks |= c.blocks;
            }
            if airport.read(self) & blocks != 0 {
                self.set_current_speed(id, 0);
                self.set_subspeed(id, 0);
                return true;
            }
        }
        false
    }
    fn set_block(
        &self,
        id: Vehicle,
        current: *const c_void,
        ap: *const c_void,
        airport: &AirportAccess,
    ) -> bool {
        let c = self.fta(current);
        let next = self.layout_blocks(ap, c.next_position);
        let reference = self.node(ap, self.pos(id));
        if (self.layout_blocks(ap, c.position) & next) != next {
            let mut blocks = next;
            let mut cursor = if current == reference {
                c.next
            } else {
                current
            };
            while !cursor.is_null() {
                let node = self.block_choice(cursor);
                if node.heading == c.heading && node.blocks != 0 {
                    blocks |= node.blocks;
                    break;
                }
                cursor = node.next;
            }
            if c.blocks == next {
                blocks ^= next;
            }
            if airport.read(self) & blocks != 0 {
                self.set_current_speed(id, 0);
                self.set_subspeed(id, 0);
                return false;
            }
            if next != NOTHING {
                airport.reserve(self, blocks);
            }
        }
        true
    }
    fn free_terminal(&self, id: Vehicle, start: u8, end: u8, airport: &AirportAccess) -> bool {
        let states = [2, 3, 4, 5, 6, 7, 19, 20, 8, 9, 21];
        let bits = [0, 1, 2, 3, 4, 5, 22, 23, 6, 7, 24];
        for i in start..end {
            let block = 1 << bits[usize::from(i)];
            if airport.read(self) & block == 0 {
                self.set_state(id, states[usize::from(i)]);
                airport.reserve(self, block);
                return true;
            }
        }
        false
    }
    fn find_terminal(&self, id: Vehicle, ap: *const c_void, airport: &AirportAccess) -> bool {
        let groups = self.terminal_count(ap, 0);
        if groups > 1 {
            let mut cursor = self.layout_next(ap, self.pos(id));
            while !cursor.is_null() {
                let node = self.block_choice(cursor);
                if node.heading != 255 {
                    return false;
                }
                if airport.read(self) & node.blocks == 0 {
                    let group = u32::from(self.fta_next_position(cursor)) + 1;
                    let mut start = 0;
                    for i in 1..group {
                        start += self.terminal_count(ap, i64::from(i));
                    }
                    let end = start + self.terminal_count(ap, i64::from(group));
                    if self.free_terminal(id, start as u8, end as u8, airport) {
                        return true;
                    }
                }
                cursor = node.next;
            }
        }
        let mut total = 0;
        for i in (1..=groups).rev() {
            total += self.terminal_count(ap, i);
        }
        self.free_terminal(id, 0, total as u8, airport)
    }
    fn find_helipad(&self, id: Vehicle, ap: *const c_void, airport: &AirportAccess) -> bool {
        let pads = self.helipads(ap) as u8;
        if pads == 0 {
            self.find_terminal(id, ap, airport)
        } else {
            self.free_terminal(id, 8, pads.wrapping_add(8), airport)
        }
    }
    fn state_handler(&self, id: Vehicle, ap: *const c_void, airport: &AirportAccess) {
        match self.state(id) {
            HANGAR => {
                if self.previous(id) != self.pos(id) {
                    self.enter_depot(id);
                    self.set_state(id, self.layout_heading(ap, self.pos(id)));
                    return;
                }
                if self.order_type(id) == i64::from(OT_GOTO_DEPOT)
                    && self.vehicle_status(id) & i64::from(VS_STOPPED) != 0
                {
                    self.free_order(id);
                    return;
                }
                if self.waiting_unbunching(id) != 0 {
                    return;
                }
                if !matches!(self.order_type(id), kind if kind == i64::from(OT_GOTO_STATION) || kind == i64::from(OT_GOTO_DEPOT))
                {
                    return;
                }
                if self.order_type(id) == i64::from(OT_GOTO_DEPOT)
                    && self.order_destination(id) == i64::from(self.target(id))
                {
                    self.enter_depot(id);
                    return;
                }
                if self.has_block(id, self.node(ap, self.pos(id)), ap, airport) {
                    return;
                }
                if self.order_destination(id) == i64::from(self.target(id)) {
                    if self.subtype(id) == 0 {
                        if !self.find_helipad(id, ap, airport) {
                            return;
                        }
                    } else if !self.find_terminal(id, ap, airport) {
                        return;
                    }
                } else {
                    self.set_state(
                        id,
                        if self.subtype(id) == 0 {
                            HELITAKEOFF
                        } else {
                            TAKEOFF
                        },
                    );
                }
                let dir = self.hangar_exit(id);
                self.leave_hangar(id, dir);
                self.airport_move(id, ap, &self.block_owner(u32::from(self.target(id))));
            }
            2..=9 | 19..=21 => {
                if self.previous(id) != self.pos(id) {
                    self.terminal(id);
                    self.set_state(id, self.layout_heading(ap, self.pos(id)));
                    if self.service_at_helipad() != 0
                        && self.subtype(id) == 0
                        && self.helipads(ap) > 0
                    {
                        self.set_economy_service(id, self.economy_date());
                        self.set_calendar_service(id, self.calendar_date());
                        self.set_breakdowns(id, 0);
                        self.set_reliability(id, self.engine_reliability(id));
                        self.dirty_details(id);
                    }
                    return;
                }
                if self.order_type(id) == i64::from(OT_NOTHING) {
                    return;
                }
                if self.has_block(id, self.node(ap, self.pos(id)), ap, airport) {
                    return;
                }
                let hangar = match self.order_type(id) {
                    kind if kind == i64::from(OT_GOTO_STATION) => false,
                    kind if kind == i64::from(OT_GOTO_DEPOT) => {
                        self.order_destination(id) == i64::from(self.target(id))
                    }
                    kind if kind == i64::from(OT_CONDITIONAL) => return,
                    _ => {
                        self.free_order(id);
                        true
                    }
                };
                self.set_state(
                    id,
                    if hangar && self.has_hangar(self.target_station(id)) != 0 {
                        HANGAR
                    } else if self.subtype(id) == 0 {
                        HELITAKEOFF
                    } else {
                        TAKEOFF
                    },
                );
                self.airport_move(id, ap, &self.block_owner(u32::from(self.target(id))));
            }
            TAKEOFF => {
                if self.start_sound(id) == 0 {
                    self.play_sound(id, self.engine_sound(id));
                }
                self.set_state(id, STARTTAKEOFF);
            }
            STARTTAKEOFF => {
                self.set_state(id, ENDTAKEOFF);
                self.update_delta(id);
            }
            ENDTAKEOFF => {
                self.set_state(id, FLYING);
                self.next_airport(id);
            }
            HELITAKEOFF => {
                self.set_state(id, FLYING);
                self.update_delta(id);
                self.next_airport(id);
                if self.needs_service(id) != 0 {
                    self.send_to_depot(id, 1);
                }
            }
            FLYING => {
                let s = airport.station;
                if can_use_station(id, s) != 0
                    && (self.station_owner(s) == i64::from(OWNER_NONE)
                        || self.station_owner(s) == self.owner(id))
                    && airport.read(self) & CLOSED == 0
                {
                    let landing = if self.subtype(id) == 0 {
                        HELILANDING
                    } else {
                        LANDING
                    };
                    let mut cursor = self.layout_next(ap, self.pos(id));
                    while !cursor.is_null() {
                        let node = self.route_node(cursor);
                        if node.heading == landing {
                            let speed = self.current_speed(id);
                            let sub = self.subspeed(id);
                            if !self.has_block(id, cursor, ap, airport) {
                                self.set_state(id, landing);
                                if landing == HELILANDING {
                                    self.flag(id, 3, true);
                                }
                                self.set_pos(id, node.next_position);
                                airport.reserve(self, self.layout_blocks(ap, self.pos(id)));
                                return;
                            }
                            self.set_current_speed(id, speed);
                            self.set_subspeed(id, sub);
                        }
                        cursor = node.next;
                    }
                }
                self.set_state(id, FLYING);
                self.set_pos(id, self.layout_next_position(ap, self.pos(id)));
            }
            LANDING => {
                self.set_state(id, ENDLANDING);
                self.update_delta(id);
                let point = self.xy(id);
                let tile = self.tile_virt(i64::from(point.x), i64::from(point.y));
                crate::animated_tile::openttd_rust_trigger_airport_tile(
                    (crate::animated_tile::services().as_station)(
                        crate::animated_tile::station_base(self.target(id)),
                    ),
                    tile,
                    crate::station_type::AAT_AIRPLANE_TOUCHDOWN,
                );
                if !(self.vs.play_vehicle_sound)(id.base(), crate::newgrf_sound::VSE_TOUCHDOWN) {
                    self.play_sound(id, SKID_SOUND);
                }
                if self.needs_service(id) != 0 {
                    self.send_to_depot(id, 1);
                }
            }
            HELILANDING => {
                self.set_state(id, HELIENDLANDING);
                self.update_delta(id);
            }
            ENDLANDING => {
                if self.has_block(id, self.node(ap, self.pos(id)), ap, airport) {
                    return;
                }
                if self.order_type(id) == i64::from(OT_GOTO_STATION)
                    && self.find_terminal(id, ap, airport)
                {
                    return;
                }
                self.set_state(id, HANGAR);
            }
            HELIENDLANDING => {
                if self.has_block(id, self.node(ap, self.pos(id)), ap, airport) {
                    return;
                }
                if self.order_type(id) == i64::from(OT_GOTO_STATION)
                    && self.find_helipad(id, ap, airport)
                {
                    return;
                }
                self.set_state(
                    id,
                    if self.has_hangar(self.target_station(id)) != 0 {
                        HANGAR
                    } else {
                        HELITAKEOFF
                    },
                );
            }
            _ => {
                self.invalid_scheme(id);
            }
        }
    }
    fn airport_move(&self, id: Vehicle, ap: *const c_void, airport: &AirportAccess) -> bool {
        if i64::from(self.pos(id)) >= self.airport_elements(ap) {
            self.invalid_position(id, ap);
        }
        let current = self.layout_route(ap, self.pos(id));
        let mut cursor = current.node;
        if current.heading == self.state(id) {
            let previous = self.pos(id);
            let state = self.state(id);
            self.state_handler(id, ap, airport);
            if self.state(id) != FLYING {
                self.set_previous(id, previous);
            }
            if self.state(id) != state || self.pos(id) != previous {
                self.cache(id, false);
            }
            return true;
        }
        self.set_previous(id, self.pos(id));
        if current.next.is_null() {
            if self.set_block(id, cursor, ap, airport) {
                self.set_pos(id, current.next_position);
                self.cache(id, false);
            }
            return false;
        }
        loop {
            let node = self.route_node(cursor);
            if node.heading == self.state(id) || node.heading == 0 {
                if self.set_block(id, cursor, ap, airport) {
                    self.set_pos(id, node.next_position);
                    self.cache(id, false);
                }
                return false;
            }
            cursor = node.next;
            if cursor.is_null() {
                break;
            }
        }
        self.invalid_movement(id);
        false
    }
    fn airport_next(&self, id: Vehicle) {
        if !self.controller(id) {
            return;
        }
        let station = self.target_station(id);
        let ap = if station.is_null() {
            Self::dummy_airport()
        } else {
            self.airport_fta(station)
        };
        let airport = AirportAccess::new(station);
        let previous = self.layout_blocks(ap, self.previous(id));
        let current = self.layout_blocks(ap, self.pos(id));
        if previous != current && !(airport.read(self) & ZEPPELIN != 0 && previous == 1 << 8) {
            airport.release(self, previous);
        }
        self.airport_move(id, ap, &airport);
    }
    fn too_far(&self, id: Vehicle, too_far: bool) {
        let has = self.flags(id) & 1 != 0;
        if too_far {
            if !has {
                self.flag(id, 0, true);
                self.dirty_start_stop(id);
                self.destination_too_far(id);
            }
            return;
        }
        if has {
            self.flag(id, 0, false);
            self.dirty_start_stop(id);
            self.delete_range_news(id);
        }
    }
    fn distance(&self, a: u32, b: u32) -> u32 {
        let dx = (self.tile_x(a) - self.tile_x(b)) as i32;
        let dy = (self.tile_y(a) - self.tile_y(b)) as i32;
        dx.wrapping_mul(dx).wrapping_add(dy.wrapping_mul(dy)) as u32
    }
    fn event(&self, id: Vehicle, pass: u32) -> bool {
        let status = self.vehicle_status(id);
        if status & i64::from(VS_CRASHED) != 0 {
            return self.crashed_tick(id);
        }
        if status & i64::from(VS_STOPPED) != 0 {
            return true;
        }
        let status = self.handle_breakdown(id);
        self.smoke(id, pass != 0, status);
        self.process_orders(id);
        self.handle_loading(id, i64::from(pass));
        let order_type = self.order_type(id);
        if order_type == i64::from(OT_LOADING) || order_type == i64::from(OT_LEAVESTATION) {
            return true;
        }
        if (ENDTAKEOFF..=HELIENDLANDING).contains(&self.state(id)) {
            self.too_far(id, false);
        } else if self.range_sqr(id) != 0 {
            let current = self.target_station(id);
            let next = if order_type == i64::from(OT_GOTO_STATION)
                || order_type == i64::from(OT_GOTO_DEPOT)
            {
                self.station(self.order_destination(id) as u32)
            } else {
                std::ptr::null()
            };
            if !current.is_null()
                && self.airport_tile(current) as u32 != INVALID_TILE
                && !next.is_null()
                && self.airport_tile(next) as u32 != INVALID_TILE
            {
                self.too_far(
                    id,
                    self.distance(
                        self.airport_tile(current) as u32,
                        self.airport_tile(next) as u32,
                    ) > self.range_sqr(id),
                );
            }
        }
        if self.flags(id) & 1 == 0 {
            self.airport_next(id);
        }
        true
    }
    fn tick(&self, id: Vehicle) -> bool {
        let subtype = self.subtype(id);
        if subtype > 2 {
            return true;
        }
        crate::witness::hit(crate::witness::AIRCRAFT_TICKS);
        let gate = self.tick_status(id);
        self.set_tick_counter(id, i64::from(gate.tick.wrapping_add(1)));
        if i64::from(gate.status) & i64::from(VS_STOPPED) == 0 {
            self.set_running_ticks(
                id,
                i64::from((self.running_ticks(id) as u8).wrapping_add(1)),
            );
        }
        if subtype == 0 {
            self.helicopter_tick(id);
        }
        self.set_order_time(id, i64::from((self.order_time(id) as i32).wrapping_add(1)));
        for pass in 0..2 {
            if !self.event(id, pass) {
                return false;
            }
        }
        true
    }
    fn service_needed(&self, id: Vehicle) {
        if self.service_interval(id) == 0 || self.needs_service(id) == 0 {
            return;
        }
        if self.chain_in_depot(id) != 0 {
            self.service_in_depot(id);
            return;
        }
        if !matches!(self.order_type(id), kind if kind == i64::from(OT_GOTO_STATION) || kind == i64::from(OT_GOTO_DEPOT))
        {
            return;
        }
        let station = self.order_destination(id) as u32;
        let st = self.station(station);
        if self.has_hangar(st) != 0 && can_use_station(id, st) != 0 {
            self.service_order(id, i64::from(station));
            self.dirty_start_stop(id);
        } else if self.order_type(id) == i64::from(OT_GOTO_DEPOT) {
            self.dummy_order(id);
            self.dirty_start_stop(id);
        }
    }
    fn day(&self, id: Vehicle, calendar: bool) {
        if self.subtype(id) > 2 {
            return;
        }
        if calendar {
            self.age_vehicle(id);
            return;
        }
        unsafe { maintenance::economy_age_vehicle(id.base(), self.vs) };
        let day = (self.vs.day)(id.base());
        Self::service_witness("day", 1);
        let day = day.wrapping_add(1);
        self.set_day_counter(id, i64::from(day));
        if day & 7 == 0 {
            self.decrease_value(id);
        }
        self.check_orders(id);
        self.check_breakdown(id);
        self.service_needed(id);
        if self.running_ticks(id) == 0 {
            return;
        }
        let cost = self.running_cost(id);
        let inputs = self.day_cost(id);
        let ticks = i64::from(inputs.running);
        let divisor = YEAR_TICKS;
        // Money's saturating multiplication is provided by the shared Money service.
        let cost = cost.saturating_mul(ticks) / divisor;
        self.set_profit(id, inputs.profit.saturating_sub(cost));
        self.set_running_ticks(id, 0);
        self.subtract_cost(id, cost);
        self.dirty_lists(id);
    }
    fn nearest_hangar(&self, id: Vehicle) -> u32 {
        let vtile = self.tile_virt(
            self.x(id).clamp(0, self.map_max_x() * 16),
            self.y(id).clamp(0, self.map_max_y() * 16),
        );
        let mut best = 0;
        let mut result = u32::from(INVALID_STATION);
        let range = self.range_sqr(id);
        let mut last = u32::from(INVALID_STATION);
        let mut next = u32::from(INVALID_STATION);
        if range != 0 {
            if self.order_type(id) == i64::from(OT_GOTO_STATION)
                || (self.order_type(id) == i64::from(OT_GOTO_DEPOT)
                    && self.nearest_depot_order(id) == 0)
            {
                last = self.last_station(id) as u32;
                next = self.order_destination(id) as u32;
            } else {
                if self.valid_airport(id) {
                    last = u32::from(self.target(id));
                }
                next = self.next_stopping_station(id) as u32;
            }
        }
        let last = self.station(last);
        let next = self.station(next);
        let engine_subtype = self.engine_subtype(id);
        let owner = self.owner(id) as u8;
        Self::service_witness("for_each_station", 0);
        self.ss.for_each_station(|st, _, cursor| {
            let cursor = u32::from(cursor);
            let candidate = self.hangar_candidate(st, owner);
            if candidate.owner != owner || !candidate.airport || !candidate.hangar {
                return;
            }
            let ap = self.airport_fta(st);
            if self.airport_flags(ap) & 4 != 0 && engine_subtype & 2 != 0 && self.no_jetcrash() == 0
            {
                return;
            }
            if self.airport_flags(ap) & 1 == 0 && engine_subtype & 1 != 0 {
                return;
            }
            if range != 0 {
                let ld = if !last.is_null() && self.airport_tile(last) as u32 != INVALID_TILE {
                    self.distance(self.airport_tile(st) as u32, self.airport_tile(last) as u32)
                } else {
                    0
                };
                let nd = if !next.is_null() && self.airport_tile(next) as u32 != INVALID_TILE {
                    self.distance(self.airport_tile(st) as u32, self.airport_tile(next) as u32)
                } else {
                    0
                };
                if ld > range || nd > range {
                    return;
                }
            }
            let distance = self.distance(vtile, self.airport_tile(st) as u32);
            if distance < best || result == u32::from(INVALID_STATION) {
                best = distance;
                result = cursor;
            }
        });
        result
    }
    fn replacement(&self, station: u32) {
        let st = self.station(station);
        let ap = self.airport_fta(st);
        let rotation = if self.airport_tile(st) as u32 == INVALID_TILE {
            0
        } else {
            self.rotation(st)
        };
        let mut cursor = Vehicle::invalid();
        loop {
            cursor = self.next_aircraft(cursor.id);
            if cursor.handle.is_null() {
                break;
            }
            if self.subtype(cursor) > 2 || u32::from(self.target(cursor)) != station {
                continue;
            }
            self.assert_flying(cursor);
            if self.order_type(cursor) == i64::from(OT_GOTO_DEPOT)
                && self.part_of_orders(cursor) == 0
                && self.order_destination(cursor) == i64::from(station)
                && (self.has_hangar(st) == 0 || can_use_station(cursor, st) == 0)
            {
                self.dummy_order(cursor);
                self.dirty_start_stop(cursor);
            }
            let pos = self.entry_point(
                cursor,
                ap,
                rotation,
                self.entry_tile(self.target_station(cursor)),
            );
            self.set_pos(cursor, pos);
            self.set_previous(cursor, pos);
            self.cache(cursor, false);
        }
        if self.has_hangar(st) == 0 {
            self.remove_depot_orders(i64::from(station));
        }
    }
}
/// Allocate the canonical private state once for every ordinary/indexed aircraft shell.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_aircraft_state_new() -> *mut State {
    Box::into_raw(Box::new(State {
        cached_max_range_sqr: 0,
        cached_max_range: 0,
        cache_padding: 0,
        crashed_counter: 0,
        targetairport: 65535,
        pos: 0,
        previous_pos: 0,
        state: 0,
        last_direction: 255,
        number_consecutive_turns: 0,
        turn_counter: 0,
        flags: 0,
    }))
}
/// Destroy the sole owner after the C++ shell's `PreDestructor` and scalar aliases end.
/// # Safety
/// The allocation is live, uniquely owned, and no scalar access remains active.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_state_destroy(state: *mut State) {
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// Allocate the station airport's canonical mask, independently of service ownership.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_blocks_new() -> *mut u64 {
    Box::into_raw(Box::new(0))
}
/// Release exactly once after the enclosing station stops using the scalar.
/// # Safety
/// Pointer is the live sole airport allocation with no active scalar access.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_blocks_destroy(blocks: *mut u64) {
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
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_tick(leaves: *const Leaves, id: Vehicle) -> bool {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.tick(id) })
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_calendar_day(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.day(id, true);
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_economy_day(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.day(id, false);
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
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
        w.position(id, i64::from(x), i64::from(y), i64::from(z));
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
pub(crate) unsafe fn enter_hangar(leaves: &Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.enter_hangar(id);
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_cache(
    leaves: *const Leaves,
    id: Vehicle,
    range: bool,
) {
    crate::witness::hit(crate::witness::AIRCRAFT_CACHE_ENTRIES);
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.cache(id, range);
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
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
        {
            let (lo, hi) = w.bounds(id, None);
            if !min.is_null() {
                unsafe {
                    min.write(lo as i32);
                }
            }
            if !max.is_null() {
                unsafe {
                    max.write(hi as i32);
                }
            }
        }
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
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
        w.flight_flags(id, flags, takeoff, None) as i32
    })
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_height(leaves: *const Leaves, id: Vehicle) -> i32 {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.height(id, None) as i32 })
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_hold_altitude(
    leaves: *const Leaves,
    id: Vehicle,
) -> i32 {
    if id.state.is_null() {
        crate::witness::hit(crate::witness::AIRCRAFT_DISASTER_ENTRIES);
    }
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        (w.height(id, None) + if w.subtype(id) == 0 { 184 } else { 150 }) as i32
    })
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_missing_orders(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.missing_orders(id);
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_crash(
    leaves: *const Leaves,
    id: Vehicle,
    flooded: bool,
) -> u32 {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.crash(id, flooded) })
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_next_airport(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.next_airport(id);
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_leave_hangar(
    leaves: *const Leaves,
    id: Vehicle,
    direction: u8,
) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.leave_hangar(id, i64::from(direction));
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_order_location(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        if w.state(id) == FLYING {
            w.next_airport(id);
        }
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_replacement(leaves: *const Leaves, station: u16) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        w.replacement(u32::from(station));
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_closest_depot(
    leaves: *const Leaves,
    id: Vehicle,
) -> u16 {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        {
            let st = w.target_station(id);
            if !st.is_null()
                && w.airport_tile(st) as u32 != INVALID_TILE
                && can_use_station(id, st) != 0
                && w.has_hangar(st) != 0
            {
                w.target(id)
            } else {
                w.nearest_hangar(id) as u16
            }
        }
    })
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_event(
    leaves: *const Leaves,
    id: Vehicle,
    pass: bool,
) -> bool {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| { w.event(id, u32::from(pass)) })
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_release_blocks(leaves: *const Leaves, id: Vehicle) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        let station = w.target_station(id);
        if !station.is_null() && w.airport_tile(station) as u32 != INVALID_TILE {
            let ap = w.airport_fta(station);
            let bits = w.layout_blocks(ap, w.previous(id)) | w.layout_blocks(ap, w.pos(id));
            AirportAccess::new(station).release(&w, bits);
        }
    });
}
/// Run the original controller synchronously over live handles.
/// # Safety
/// Handles/state and immutable services satisfy the source lifetime preconditions.
/// Raw scalar accesses end before noexcept callbacks; no borrow survives reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_aircraft_invalidate_target(
    leaves: *const Leaves,
    station: u16,
) {
    crate::witness::hit(crate::witness::AIRCRAFT_ENTRIES);
    with_world!(leaves, |w| {
        {
            let mut cursor = Vehicle::invalid();
            loop {
                cursor = w.next_aircraft(cursor.id);
                if cursor.handle.is_null() {
                    break;
                }
                if w.subtype(cursor) <= 2 && w.target(cursor) == station {
                    w.set_target(cursor, 65535);
                }
            }
        }
    });
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAircraftState" ["probe"] => State {
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
    "OpenTTDAircraftNode" ["probe"] => Node {
        next,
        blocks,
        position,
        next_position,
        heading,
    };
    "OpenTTDAircraftMoving" ["probe"] => Moving {
        x,
        y,
        flags,
        direction,
    };
    "OpenTTDAircraftLeaves" ["probe"] => Leaves {
        geometry,
        layout_blocks,
        layout_heading,
        layout_next_position,
        layout_next,
        layout_route,
        vehicle,
        shared,
        next,
        station,
        airport_tile,
        station_tile,
        rotation,
        airport_type,
        station_owner,
        airport_blocks,
        had_vehicle,
        airport_elements,
        helipads,
        airport_flags,
        airport_delta_z,
        node,
        fta,
        moving,
        engine_speed,
        engine_subtype,
        engine_sound,
        airport_entry,
        direction_towards,
        hangar_height,
        terminal_count,
        hangar_exit,
        service_interval,
        chain_in_depot,
        nearest_depot_order,
        part_of_orders,
        next_aircraft,
        viewport,
        rotor_image,
        copy_sprite,
        create_effect,
        truncate_cargo,
        crash_news,
        station_rating,
        landing_rating,
        dirty_depot,
        first_arrival,
        update_delta,

        destination_too_far,
        delete_range_news,
        service_order,
        running_cost,
        subtract_cost,
        dirty_lists,
        next_stopping_station,
        remove_depot_orders,
        assert_flying,
        invalid_movement,
        invalid_position,
        invalid_scheme,
        unreachable,
        cargo_age_default,
        range_default,
        rotor_image_if_changed,
        update_rotor_image,
        vehicle_crash,
        send_to_depot,
        fta_next_position,
        block_node,
        route_node,
        block_choice,
    };
    "OpenTTDAircraftBlockNode" ["probe"] => BlockNode {
        blocks,
        position,
        next_position,
    };
    "OpenTTDAircraftRouteNode" ["probe"] => RouteNode {
        next,
        next_position,
        heading,
    };
    "OpenTTDAircraftBlockChoice" ["probe"] => BlockChoice {
        next,
        blocks,
        heading,
    };
    "OpenTTDAircraftGeometry" ["probe"] => Geometry {
        tile,
        rotation,
        width,
        height,
        airport,
    };
    "OpenTTDAircraftLayoutRoute" ["probe"] => LayoutRoute {
        node,
        next,
        next_position,
        heading,
    };
}

/// `Aircraft::GetSpeedOldUnits` applied to `cached_max_speed`.
pub(crate) fn speed_old_units(speed: u32) -> u32 {
    speed * 10 / 128
}

/// `Aircraft::IsNormalAircraft`.
pub(crate) const fn is_normal_aircraft(subtype: u8) -> bool {
    subtype <= AIR_AIRCRAFT
}
