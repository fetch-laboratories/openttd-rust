/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Road stops (`roadstop.cpp`): bay admission, entrance status and drive-through entry
//! topology and occupancy. Each `RoadStop` shell owns one `State`; the `BaseEntry` stop of a
//! drive-through line owns the `Entries` every stop of the line points at. The pool, stop
//! lists, map and vehicle hash stay native.
#![allow(unsafe_code, clippy::many_single_char_names)]
use crate::bitmath_func::{has_bit, sb, set_bit};
use crate::direction_func::dir_to_diag_dir;
use crate::direction_type::{Axis, DiagDirection, Direction};
use crate::map_func::World;
use crate::road::State as RoadState;
use crate::roadstop_base::{RoadStopStatusFlag as StatusFlag, RoadStopStatusFlags as StatusFlags};
use crate::roadveh::{
    RVS_IN_DT_ROAD_STOP, RVS_IN_ROAD_STOP, RVS_USING_SECOND_BAY, RVSB_IN_ROAD_STOP,
};
use crate::services::{Map, Services};
use crate::station::BaseStation;
use crate::station_type::{RoadStopID, RoadStopType};
use crate::tile_type::{MP_STATION, TILE_SIZE, TileIndex};
use crate::vehicle::{GroundServices, VehicleRef, VehicleServices, VehicleType};
use crate::vehicle_base::VS_CRASHED;

/// `TILE_SIZE` in the `uint16_t` entry arithmetic.
#[allow(clippy::cast_possible_truncation)] // TILE_SIZE is 16.
const TILE_UNITS: u16 = TILE_SIZE as u16;

/// `RoadStop::Entry`: one drive-through entry point in tile units.
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Entry {
    /// The length of the stop in tile units.
    pub length: u16,
    /// The amount of occupied stop in tile units.
    pub occupied: u16,
}
/// `RoadStop::Entries`: both entry points of one drive-through line.
#[repr(C)]
#[derive(Default)]
pub struct Entries {
    /// Vehicles that entered from the east.
    pub east: Entry,
    /// Vehicles that entered from the west.
    pub west: Entry,
}
/// One allocation per native shell: the saved status byte and the shared
/// entries pointer, which the `BaseEntry` holder owns.
#[repr(C)]
pub struct State {
    pub status: u8,
    pub entries: *mut Entries,
}
/// Opaque native `RoadStop` shell; never dereferenced in Rust.
#[repr(C)]
pub struct Stop {
    _private: [u8; 0],
}
/// The next live pool item at or after an index; `state` is null at the end.
#[repr(C)]
pub struct PoolStop {
    pub state: *mut State,
    pub xy: u32,
    pub index: u32,
}
/// `GroundVehicleCache::cached_total_length` of a live road record.
fn total_length(v: VehicleRef) -> u16 {
    crate::raw_field::read_field!(v.ground_cache(), cached_total_length)
}

/// Native road stop services, reached through the shared table.
#[repr(C)]
pub struct RoadStopServices {
    /// Shared vehicle table; Rebuild uses its tile hash traversal.
    pub vehicle: *const VehicleServices,
    /// Shared road ground vehicle table.
    pub ground: *const GroundServices,
    /// `Station::GetByTile(tile)->GetPrimaryRoadStop(type)`.
    pub primary: extern "C" fn(u32, RoadStopType) -> *mut Stop,
    /// The station's bus or truck stop list head.
    pub head: extern "C" fn(*mut BaseStation, bool) -> *mut Stop,
    /// `RoadVehicle::IsBus`, preserving its front-engine assertion.
    pub is_bus: extern "C" fn(VehicleRef) -> bool,
    /// `RoadStop::next`.
    pub next: extern "C" fn(*const Stop) -> *mut Stop,
    /// `RoadStop::xy`.
    pub tile: extern "C" fn(*const Stop) -> u32,
    /// The shell's Rust owner.
    pub state: extern "C" fn(*const Stop) -> *mut State,
    /// First live `RoadStop::Iterate(index)` item.
    pub pool_next: extern "C" fn(u32) -> PoolStop,
}

/// Map dimensions read at the entry's use.
fn map(s: &Services) -> Map {
    s.map()
}

fn table(s: &Services) -> &RoadStopServices {
    // SAFETY: The shared table returns a static immutable table.
    unsafe { &*(s.road_stops)() }
}

/// Canonical `RoadVehicle::IsBus` observation.
pub(crate) fn is_bus(s: &Services, v: VehicleRef) -> bool {
    (table(s).is_bus)(v)
}
/// `Station::GetPrimaryRoadStop(const RoadVehicle *)`.
pub(crate) fn primary_for_vehicle(
    s: &Services,
    station: *mut BaseStation,
    v: VehicleRef,
) -> *mut Stop {
    let stops = table(s);
    let head = (stops.head)(station, !is_bus(s, v));
    accessible(s, head, v)
}
/// `RoadStop::GetNextRoadStop`: get the next road stop accessible by this
/// vehicle, retaining the native linked-list order.
pub(crate) fn next_for_vehicle(s: &Services, stop: *const Stop, v: VehicleRef) -> *mut Stop {
    accessible(s, (table(s).next)(stop), v)
}
/// The first stop from `stop` onward that the vehicle can use.
fn accessible(s: &Services, mut stop: *mut Stop, v: VehicleRef) -> *mut Stop {
    let stops = table(s);
    let movement = crate::movement::table(s);
    let ground = unsafe { &*stops.ground };
    let world = crate::map_func::World::new(s);
    while !stop.is_null() {
        let tile = TileIndex((stops.tile)(stop));
        // The vehicle cannot go to this roadstop (different roadtype)
        if !crate::road_map::has_tile_any_road_type(&world, tile.0, (movement.road_compatible)(v)) {
            stop = (stops.next)(stop);
            continue;
        }
        // The vehicle is articulated and can therefore not go to a standard road stop.
        if world.is_bay_road_stop_tile(tile) && (ground.has_articulated)(v) {
            stop = (stops.next)(stop);
            continue;
        }

        // The vehicle can actually go to this road stop. So, return it!
        return stop;
    }
    std::ptr::null_mut()
}
/// `RoadStop::RoadStop`: the Rust-owned part of a new road stop, with both bays free.
fn create() -> *mut State {
    Box::into_raw(Box::new(State {
        status: StatusFlags::from_element(StatusFlag::Bay0Free)
            .with(StatusFlag::Bay1Free)
            .0,
        entries: std::ptr::null_mut(),
    }))
}
/// `RoadStop::~RoadStop`: de-initializes road stops.
/// # Safety
/// The caller transfers a live owner from `create` exactly once.
unsafe fn destroy(state: *mut State) {
    // SAFETY: Caller transfers the owner; `delete nullptr` is a no-op.
    unsafe {
        let state = Box::from_raw(state);
        // When we are the head we need to free the entries
        if StatusFlags(state.status).test(StatusFlag::BaseEntry) && !state.entries.is_null() {
            drop(Box::from_raw(state.entries));
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_stop_new() -> *mut State {
    create()
}
/// # Safety
/// One native shell transfers its live owner exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_stop_destroy(state: *mut State) {
    unsafe { destroy(state) };
}

fn status(state: *mut State) -> StatusFlags {
    // SAFETY: Live owner; the load ends before any service.
    unsafe { StatusFlags((*state).status) }
}
fn set_status(state: *mut State, status: StatusFlags) {
    // SAFETY: Live owner, serialized game thread.
    unsafe { (*state).status = status.0 }
}
fn entries(state: *mut State) -> *mut Entries {
    // SAFETY: Live owner; the load ends before any service.
    unsafe { (*state).entries }
}
fn set_entries(state: *mut State, entries: *mut Entries) {
    // SAFETY: Live owner, serialized game thread.
    unsafe { (*state).entries = entries }
}
/// `&entries->east` or `&entries->west`.
fn side(entries: *mut Entries, east: bool) -> *mut Entry {
    // SAFETY: Live shared entries; only the place is computed.
    unsafe {
        if east {
            &raw mut (*entries).east
        } else {
            &raw mut (*entries).west
        }
    }
}
/// `RoadStop::GetEntry(dir)`: get the drive through road stop entry struct
/// for the given direction.
fn entry(state: *mut State, dir: DiagDirection) -> *mut Entry {
    side(entries(state), dir < DiagDirection::DIAGDIR_SW)
}
fn add_length(entries: *mut Entries, amount: u16) {
    // SAFETY: Live shared entries; uint16_t arithmetic.
    unsafe {
        (*entries).east.length = (*entries).east.length.wrapping_add(amount);
        (*entries).west.length = (*entries).west.length.wrapping_add(amount);
    }
}
fn sub_length(entries: *mut Entries, amount: u16) {
    // SAFETY: Live shared entries; uint16_t arithmetic.
    unsafe {
        (*entries).east.length = (*entries).east.length.wrapping_sub(amount);
        (*entries).west.length = (*entries).west.length.wrapping_sub(amount);
    }
}

/// `RoadStop::IsDriveThroughRoadStopContinuation`: checks whether the `next`
/// tile is still part of the same drive through stop `rs` in the same
/// direction for the same vehicle.
pub(crate) fn is_drive_through_road_stop_continuation(
    s: &Services,
    rs: TileIndex,
    next: TileIndex,
) -> bool {
    (s.tile_type)(next.0) == MP_STATION
        && (s.station_index)(next.0) == (s.station_index)(rs.0)
        && (s.station_type)(next.0) == (s.station_type)(rs.0)
        && crate::map_func::World::new(s).is_drive_through_stop_tile(next)
        && crate::map_func::World::new(s).get_drive_through_stop_axis(next)
            == crate::map_func::World::new(s).get_drive_through_stop_axis(rs)
}
/// Temporary raw adapter for `road.rs`.
pub(crate) fn continuation(s: &Services, rs: u32, next: u32) -> bool {
    is_drive_through_road_stop_continuation(s, TileIndex(rs), TileIndex(next))
}

/// `RoadStop::GetByTile`: find a roadstop at given tile, returning the shell
/// and its owner.
pub(crate) fn get_by_tile(
    s: &Services,
    tile: TileIndex,
    kind: RoadStopType,
) -> (*mut Stop, *mut State) {
    let t = table(s);
    let mut rs = (t.primary)(tile.0, kind);
    loop {
        if (t.tile)(rs) == tile.0 {
            return (rs, (t.state)(rs));
        }
        let next = (t.next)(rs);
        World::new(s).assert_with(|| !next.is_null());
        rs = next;
    }
}
/// Temporary raw adapter for `road.rs`.
pub(crate) fn by_tile_raw(s: &Services, tile: u32, kind: RoadStopType) -> (*mut Stop, *mut State) {
    get_by_tile(s, TileIndex(tile), kind)
}
/// `RoadStop::GetByTile(tile, GetRoadStopType(tile))`.
pub(crate) fn at(s: &Services, tile: TileIndex) -> *mut State {
    get_by_tile(
        s,
        tile,
        crate::map_func::World::new(s).get_road_stop_type(tile),
    )
    .1
}
/// Temporary raw adapter for `road.rs`.
pub(crate) fn at_raw(s: &Services, tile: u32) -> *mut State {
    at(s, TileIndex(tile))
}

/// `rs->GetEntry(dir)`, copied for a native-equivalent read.
pub(crate) fn entry_read(state: *mut State, dir: DiagDirection) -> Entry {
    // SAFETY: Live shared entries of a drive-through stop; the copy ends the load.
    unsafe { *entry(state, dir) }
}
/// `RoadStop::HasFreeBay`: checks whether there is a free bay in this road
/// stop.
pub(crate) fn has_free_bay(state: *mut State) -> bool {
    status(state).any(StatusFlags::from_element(StatusFlag::Bay0Free).with(StatusFlag::Bay1Free))
}
/// `RoadStop::IsFreeBay`: checks whether the given bay is free in this road
/// stop.
pub(crate) fn is_free_bay(state: *mut State, nr: u8) -> bool {
    match nr {
        0 => status(state).test(StatusFlag::Bay0Free),
        1 => status(state).test(StatusFlag::Bay1Free),
        _ => unreachable!("original IsFreeBay NOT_REACHED"),
    }
}
/// `RoadStop::IsEntranceBusy`: checks whether the entrance of the road stop
/// is occupied by a vehicle.
pub(crate) fn is_entrance_busy(state: *mut State) -> bool {
    status(state).test(StatusFlag::EntryBusy)
}
/// `RoadStop::SetEntranceBusy`: makes an entrance occupied or free.
pub(crate) fn set_entrance_busy(state: *mut State, busy: bool) {
    let status = status(state);
    set_status(
        state,
        if busy {
            status.with(StatusFlag::EntryBusy)
        } else {
            status.without(StatusFlag::EntryBusy)
        },
    );
}
/// `RoadStop::AllocateBay`: allocates a bay and returns its number.
fn allocate_bay(s: &Services, state: *mut State) -> u8 {
    World::new(s).assert_with(|| has_free_bay(state));
    // Find the first free bay.
    let mut bay_nr = 0;
    while !is_free_bay(state, bay_nr) {
        bay_nr += 1;
    }
    allocate_drive_through_bay(state, bay_nr);
    bay_nr
}
/// `RoadStop::AllocateDriveThroughBay`: allocates a bay in a drive-through
/// road stop.
fn allocate_drive_through_bay(state: *mut State, nr: u8) {
    set_status(
        state,
        status(state).without(match nr {
            0 => StatusFlag::Bay0Free,
            1 => StatusFlag::Bay1Free,
            _ => unreachable!("original AllocateDriveThroughBay NOT_REACHED"),
        }),
    );
}
/// `RoadStop::FreeBay`: frees the given bay.
fn free_bay(state: *mut State, nr: u8) {
    set_status(
        state,
        status(state).with(match nr {
            0 => StatusFlag::Bay0Free,
            1 => StatusFlag::Bay1Free,
            _ => unreachable!("original FreeBay NOT_REACHED"),
        }),
    );
}

/// `RoadStop::Entry::Enter`: enter the road stop.
fn enter_entry(entry: *mut Entry, total_length: u16) {
    // we cannot assert on this->occupied < this->length because of the
    // remote possibility that RVs are running through each other when
    // trying to prevention an infinite jam.
    // SAFETY: Live shared entry; uint16_t arithmetic.
    unsafe { (*entry).occupied = (*entry).occupied.wrapping_add(total_length) };
}
/// `RoadStop::Entry::Leave`: leave the road stop.
fn leave_entry(s: &Services, entry: *mut Entry, total_length: u16) {
    // SAFETY: Live shared entry; the copy ends the load before the native assertion.
    World::new(s).assert_with(|| unsafe { (*entry).occupied } >= total_length);
    // SAFETY: Live shared entry; uint16_t arithmetic.
    unsafe { (*entry).occupied = (*entry).occupied.wrapping_sub(total_length) };
}

/// `RoadStop::Enter`: enter the road stop at `xy`.
/// # Safety
/// Live serialized stop and road owners of the front vehicle `v`.
pub(crate) unsafe fn enter(
    s: &Services,
    state: *mut State,
    xy: TileIndex,
    v: VehicleRef,
    road: *mut RoadState,
) -> bool {
    let t = table(s);
    if crate::map_func::World::new(s).is_bay_road_stop_tile(xy) {
        // SAFETY: The shared road ground table is static.
        let ground = unsafe { &*t.ground };
        // For normal (non drive-through) road stops
        // Check if station is busy or if there are no free bays or whether it is a articulated vehicle.
        if is_entrance_busy(state) || !has_free_bay(state) || (ground.has_articulated)(v) {
            crate::witness::hit(crate::witness::ROAD_STOP_ENTER_REFUSED);
            return false;
        }
        // SAFETY: Caller's live road owner; no service runs during these stores.
        unsafe {
            let mut rv_state = u32::from(crate::road::get_state(road));
            set_bit(&mut rv_state, RVS_IN_ROAD_STOP);
            crate::road::set_state(road, state_byte(rv_state));
            // Allocate a bay and update the road state
            let bay_nr = allocate_bay(s, state);
            crate::witness::hit(if bay_nr == 0 {
                crate::witness::ROAD_STOP_ENTER_BAY0
            } else {
                crate::witness::ROAD_STOP_ENTER_BAY1
            });
            let mut rv_state = u32::from(crate::road::get_state(road));
            sb(&mut rv_state, RVS_USING_SECOND_BAY, 1, u32::from(bay_nr));
            crate::road::set_state(road, state_byte(rv_state));
        }
        // Mark the station entrance as busy
        set_entrance_busy(state, true);
        return true;
    }
    crate::witness::hit(crate::witness::ROAD_STOP_ENTER_DRIVE);
    // Vehicles entering a drive-through stop from the 'normal' side use first bay (bay 0).
    let e = entry(
        state,
        dir_to_diag_dir(Direction(v.direction()), |ok| {
            crate::movement::native_assert(s, ok);
        }),
    );
    enter_entry(e, total_length(v));
    // Indicate a drive-through stop
    // SAFETY: Caller's live road owner; no service runs during the store.
    unsafe {
        let mut rv_state = u32::from(crate::road::get_state(road));
        set_bit(&mut rv_state, RVS_IN_DT_ROAD_STOP);
        crate::road::set_state(road, state_byte(rv_state));
    }
    true
}

/// The `uint8_t RoadVehicle::state` store after `SetBit`/`SB`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "SetBit and SB store back into the original uint8_t RoadVehicle::state"
)]
fn state_byte(value: u32) -> u8 {
    value as u8
}

/// `RoadStop::Leave`: leave the road stop, for a vehicle on tile `tile`.
/// # Safety
/// Live serialized stop and road owners of the front vehicle `v`.
pub(crate) unsafe fn leave(
    s: &Services,
    state: *mut State,
    tile: TileIndex,
    v: VehicleRef,
    road: *mut RoadState,
) {
    if crate::map_func::World::new(s).is_bay_road_stop_tile(tile) {
        crate::witness::hit(crate::witness::ROAD_STOP_LEAVE_BAY);
        // SAFETY: Caller's live road owner, read before any store.
        let rv_state = unsafe { crate::road::get_state(road) };
        // Vehicle is leaving a road stop tile, mark bay as free
        free_bay(state, u8::from(has_bit(rv_state, RVS_USING_SECOND_BAY)));
        set_entrance_busy(state, false);
    } else {
        crate::witness::hit(crate::witness::ROAD_STOP_LEAVE_DRIVE);
        // Otherwise just leave the drive through's entry cache.
        let e = entry(
            state,
            dir_to_diag_dir(Direction(v.direction()), |ok| {
                crate::movement::native_assert(s, ok);
            }),
        );
        leave_entry(s, e, total_length(v));
    }
}
/// `RoadStop::GetByTile(v->tile, GetRoadStopType(v->tile))->Leave(v)`.
/// # Safety
/// Live serialized road owner of the front vehicle `v`.
pub(crate) unsafe fn leave_vehicle(s: &Services, v: VehicleRef, road: *mut RoadState) {
    let tile = TileIndex(v.tile());
    // SAFETY: Caller's contract.
    unsafe { leave(s, at(s, tile), tile, v, road) };
}

/// `GetEntryDirection`: get the `DiagDirection` for entering the drive
/// through stop from the given 'side' (east or west) on the given axis.
fn entry_direction(east: bool, axis: Axis) -> DiagDirection {
    match axis {
        Axis::AXIS_X => {
            if east {
                DiagDirection::DIAGDIR_NE
            } else {
                DiagDirection::DIAGDIR_SW
            }
        }
        Axis::AXIS_Y => {
            if east {
                DiagDirection::DIAGDIR_SE
            } else {
                DiagDirection::DIAGDIR_NW
            }
        }
        _ => unreachable!("original GetEntryDirection NOT_REACHED"),
    }
}

/// `RoadStop::Entry::Rebuild`: rebuild, from scratch, the vehicles and other
/// metadata on this stop, from the base stop at `xy`, into `this`. As the
/// original, the length is written per tile and the occupied space is summed
/// from the vehicles' cached lengths after the traversal.
fn rebuild_entry(s: &Services, xy: TileIndex, state: *mut State, east: bool, this: *mut Entry) {
    World::new(s).assert_with(|| status(state).test(StatusFlag::BaseEntry));
    let t = table(s);
    // SAFETY: The shared vehicle table is static.
    let vehicle = unsafe { &*t.vehicle };
    let axis = crate::map_func::World::new(s).get_drive_through_stop_axis(xy);
    let entry_dir = entry_direction(east, axis);
    let mut vehicles: Vec<VehicleRef> = Vec::new();
    // SAFETY: `this` is the caller's live entry; no service runs during the write.
    unsafe { (*this).length = 0 };
    let offset = map(s)
        .offs_by_axis(axis, |ok| crate::services_assert::check(ok))
        .cast_unsigned();
    let mut tile = xy;
    while is_drive_through_road_stop_continuation(s, xy, tile) {
        // SAFETY: As above; the read and write end before the vehicle traversal.
        unsafe { (*this).length = (*this).length.wrapping_add(TILE_UNITS) };
        vehicle.for_each_on_tile(tile.0, |v| {
            // Not a RV or not in the right direction or crashed :(
            if v.vehicle_type() != VehicleType::ROAD.0
                || dir_to_diag_dir(Direction(v.direction()), |ok| {
                    crate::movement::native_assert(s, ok);
                }) != entry_dir
                || !v.is_front_engine()
                || v.status() & VS_CRASHED != 0
            {
                return;
            }
            // Don't add ones not in a road stop
            // SAFETY: Live road owner of this road vehicle shell.
            let road = crate::raw_field::read_field!(v.0.as_ptr(), road).cast();
            if unsafe { crate::road::get_state(road) } < RVSB_IN_ROAD_STOP {
                return;
            }
            crate::witness::hit(crate::witness::ROAD_STOP_REBUILD_VEHICLE);
            // `include(vehicles, rv)`.
            if !vehicles.contains(&v) {
                vehicles.push(v);
            }
        });
        tile = TileIndex(tile.0.wrapping_add(offset));
    }
    // SAFETY: As above; each write ends before the next vehicle read.
    unsafe { (*this).occupied = 0 };
    for &v in &vehicles {
        let cached_total_length = total_length(v);
        unsafe { (*this).occupied = (*this).occupied.wrapping_add(cached_total_length) };
    }
}
/// `entries->east.Rebuild(rs)` or `entries->west.Rebuild(rs)`.
fn rebuild(s: &Services, xy: TileIndex, state: *mut State, east: bool) {
    crate::witness::hit(crate::witness::ROAD_STOP_REBUILD);
    // Live shared entries of this base stop.
    rebuild_entry(s, xy, state, east, side(entries(state), east));
}

/// `RoadStop::MakeDriveThrough`: join this road stop at `xy` to another
/// 'base' road stop if possible; fill all necessary data to become an actual
/// drive through road stop. Also update the length etc.
pub(crate) fn make_drive_through(s: &Services, xy: TileIndex, state: *mut State) {
    World::new(s).assert_with(|| entries(state).is_null());
    let rst = crate::map_func::World::new(s).get_road_stop_type(xy);
    let axis = crate::map_func::World::new(s).get_drive_through_stop_axis(xy);
    let offset = map(s)
        .offs_by_axis(axis, |ok| crate::services_assert::check(ok))
        .cast_unsigned();

    // Information about the tile north of us
    let north_tile = TileIndex(xy.0.wrapping_sub(offset));
    let north = is_drive_through_road_stop_continuation(s, xy, north_tile);
    let rs_north = if north {
        get_by_tile(s, north_tile, rst).1
    } else {
        std::ptr::null_mut()
    };
    // Information about the tile south of us
    let mut south_tile = TileIndex(xy.0.wrapping_add(offset));
    let south = is_drive_through_road_stop_continuation(s, xy, south_tile);
    let mut rs_south = if south {
        get_by_tile(s, south_tile, rst).1
    } else {
        std::ptr::null_mut()
    };

    // Amount of road stops that will be added to the 'northern' head
    let mut added: u16 = 1;
    if north && !entries(rs_north).is_null() {
        // There is a more northern one, so this can join them
        let joined = entries(rs_north);
        set_entries(state, joined);
        if south && !entries(rs_south).is_null() {
            crate::witness::hit(crate::witness::ROAD_STOP_MAKE_MERGE);
            // There more southern tiles too, they must 'join' us too
            set_status(rs_south, status(rs_south).without(StatusFlag::BaseEntry));
            let southern = entries(rs_south);
            // SAFETY: Distinct live entries; the southern block is freed below.
            unsafe {
                (*joined).east.occupied = (*joined)
                    .east
                    .occupied
                    .wrapping_add((*southern).east.occupied);
                (*joined).west.occupied = (*joined)
                    .west
                    .occupied
                    .wrapping_add((*southern).west.occupied);
                // Free the now unneeded entries struct
                drop(Box::from_raw(southern));
            }
            // Make all 'children' of the southern tile take the new master
            while is_drive_through_road_stop_continuation(s, xy, south_tile) {
                rs_south = get_by_tile(s, south_tile, rst).1;
                if entries(rs_south).is_null() {
                    break;
                }
                set_entries(rs_south, entries(rs_north));
                added = added.wrapping_add(1);
                south_tile = TileIndex(south_tile.0.wrapping_add(offset));
            }
        } else {
            crate::witness::hit(crate::witness::ROAD_STOP_MAKE_JOIN);
        }
    } else if south && !entries(rs_south).is_null() {
        crate::witness::hit(crate::witness::ROAD_STOP_MAKE_PARENT);
        // There is one to the south, but not to the north... so we become 'parent'
        set_entries(state, entries(rs_south));
        set_status(state, status(state).with(StatusFlag::BaseEntry));
        set_status(rs_south, status(rs_south).without(StatusFlag::BaseEntry));
    } else {
        crate::witness::hit(crate::witness::ROAD_STOP_MAKE_ALONE);
        // We are the only... so we are automatically the master
        set_entries(state, Box::into_raw(Box::default()));
        set_status(state, status(state).with(StatusFlag::BaseEntry));
    }

    // Now update the lengths
    added = added.wrapping_mul(TILE_UNITS);
    add_length(entries(state), added);
}

/// `RoadStop::ClearDriveThrough`: prepare for removal of the stop at `xy`;
/// update other neighbouring stops if needed. Also update the length etc.
pub(crate) fn clear_drive_through(s: &Services, xy: TileIndex, state: *mut State) {
    World::new(s).assert_with(|| !entries(state).is_null());
    let rst = crate::map_func::World::new(s).get_road_stop_type(xy);
    let axis = crate::map_func::World::new(s).get_drive_through_stop_axis(xy);
    let offset = map(s)
        .offs_by_axis(axis, |ok| crate::services_assert::check(ok))
        .cast_unsigned();

    // Information about the tile north of us
    let mut north_tile = TileIndex(xy.0.wrapping_sub(offset));
    let north = is_drive_through_road_stop_continuation(s, xy, north_tile);
    let mut rs_north = if north {
        get_by_tile(s, north_tile, rst).1
    } else {
        std::ptr::null_mut()
    };
    let mut rs_north_xy = north_tile;
    // Information about the tile south of us
    let mut south_tile = TileIndex(xy.0.wrapping_add(offset));
    let south = is_drive_through_road_stop_continuation(s, xy, south_tile);
    let mut rs_south = if south {
        get_by_tile(s, south_tile, rst).1
    } else {
        std::ptr::null_mut()
    };

    // Must only be cleared after we determined which neighbours are
    // part of our little entry 'queue'
    crate::ground::do_clear_square(xy, s);

    if north {
        // There is a tile to the north, so we can't clear ourselves.
        if south {
            crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_SPLIT);
            // There are more southern tiles too, they must be split;
            // first make the new southern 'base'
            set_status(rs_south, status(rs_south).with(StatusFlag::BaseEntry));
            set_entries(rs_south, Box::into_raw(Box::default()));

            // Keep track of the base because we need it later on
            let rs_south_base = rs_south;
            let base_tile = south_tile;

            // Make all (even more) southern stops part of the new entry queue
            south_tile = TileIndex(south_tile.0.wrapping_add(offset));
            while is_drive_through_road_stop_continuation(s, base_tile, south_tile) {
                rs_south = get_by_tile(s, south_tile, rst).1;
                set_entries(rs_south, entries(rs_south_base));
                south_tile = TileIndex(south_tile.0.wrapping_add(offset));
            }

            // Find the other end; the northern most tile
            while is_drive_through_road_stop_continuation(s, base_tile, north_tile) {
                rs_north = get_by_tile(s, north_tile, rst).1;
                rs_north_xy = north_tile;
                north_tile = TileIndex(north_tile.0.wrapping_sub(offset));
            }

            // We have to rebuild the entries because we cannot easily determine
            // how full each part is. So instead of keeping and maintaining a list
            // of vehicles and using that to 'rebuild' the occupied state we just
            // rebuild it from scratch as that removes lots of maintenance code
            // for the vehicle list and it's faster in real games as long as you
            // do not keep split and merge road stop every tick by the millions.
            rebuild(s, base_tile, rs_south_base, true);
            rebuild(s, base_tile, rs_south_base, false);

            World::new(s).assert_with(|| status(rs_north).test(StatusFlag::BaseEntry));
            rebuild(s, rs_north_xy, rs_north, true);
            rebuild(s, rs_north_xy, rs_north, false);
        } else {
            crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_TAIL);
            // Only we left, so simple update the length.
            sub_length(entries(rs_north), TILE_UNITS);
        }
    } else if south {
        crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_HEAD);
        // There is only something to the south. Hand over the base entry
        set_status(rs_south, status(rs_south).with(StatusFlag::BaseEntry));
        sub_length(entries(rs_south), TILE_UNITS);
    } else {
        crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_LAST);
        // We were the last
        // SAFETY: This sole stop of its line owns the entries.
        unsafe { drop(Box::from_raw(entries(state))) };
    }

    // Make sure we don't get used for something 'incorrect'
    set_status(state, status(state).without(StatusFlag::BaseEntry));
    set_entries(state, std::ptr::null_mut());
}

/// `RoadStop::Entry::CheckIntegrity` of `rs->GetEntry(dir)`: check the
/// integrity of the data in this struct.
fn check_integrity(s: &Services, xy: TileIndex, state: *mut State, dir: DiagDirection) {
    if !status(state).test(StatusFlag::BaseEntry) {
        return;
    }
    crate::witness::hit(crate::witness::ROAD_STOP_INTEGRITY);
    // The tile 'before' the road stop must not be part of this 'line'
    let world = World::new(s);
    world.assert_with(|| world.is_drive_through_stop_tile(xy));
    world.assert_with(|| {
        !is_drive_through_road_stop_continuation(
            s,
            xy,
            TileIndex(
                xy.0.wrapping_sub(
                    map(s)
                        .offs_by_axis(world.get_drive_through_stop_axis(xy), |ok| {
                            crate::services_assert::check(ok);
                        })
                        .cast_unsigned(),
                ),
            ),
        )
    });
    let east = dir < DiagDirection::DIAGDIR_SW;
    let mut temp = Entry::default();
    rebuild_entry(s, xy, state, east, &raw mut temp);
    // SAFETY: Live shared entries; the copy ends the load.
    let current = unsafe { *side(entries(state), east) };
    if temp != current {
        unreachable!("original CheckIntegrity NOT_REACHED");
    }
}

/// `RoadStop::Enter` from native tile entry: `RoadStop::GetByTile(tile,
/// GetRoadStopType(tile))->Enter(rv)`.
/// # Safety
/// Live serialized shared table, front road vehicle and its owner.
pub(crate) unsafe fn enter_by_tile(
    s: *const Services,
    tile: TileIndex,
    v: VehicleRef,
    road: *mut RoadState,
) -> bool {
    // SAFETY: Caller's contract.
    unsafe {
        let s = &*s;
        enter(s, at(s, tile), tile, v, road)
    }
}
/// `RoadStop::GetByTile(v->tile, GetRoadStopType(v->tile))->Leave(v)`.
/// # Safety
/// Live serialized shared table, front road vehicle and its owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_stop_leave(
    s: *const Services,
    v: VehicleRef,
    road: *mut RoadState,
) {
    // SAFETY: Caller's contract; the road stop table's vehicle table is static.
    unsafe {
        let s = &*s;
        leave_vehicle(s, v, road);
    }
}
/// `rs->GetEntry(dir).CheckIntegrity(rs)`.
/// # Safety
/// Live serialized shared table and stop owner at `xy`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_stop_check_integrity(
    s: *const Services,
    xy: u32,
    state: *mut State,
    dir: u8,
) {
    // SAFETY: Caller's contract.
    check_integrity(unsafe { &*s }, TileIndex(xy), state, DiagDirection(dir));
}
/// # Safety
/// Live serialized shared table after the ROAD chunk pointers are fixed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_stop_after_load(s: *const Services) {
    // SAFETY: Caller's contract.
    after_load_road_stops(unsafe { &*s });
}
/// `AfterLoadRoadStops`: construct the drive-through entries, then rebuild
/// the data in those entries, each in pool order.
fn after_load_road_stops(s: &Services) {
    let t = table(s);
    // Construct the drive-through entries before rebuilding their data.
    let mut index = u32::from(RoadStopID::BEGIN.0);
    loop {
        let rs = (t.pool_next)(index);
        if rs.state.is_null() {
            break;
        }
        if crate::map_func::World::new(s).is_drive_through_stop_tile(TileIndex(rs.xy)) {
            make_drive_through(s, TileIndex(rs.xy), rs.state);
        }
        index = rs.index + 1;
    }
    index = u32::from(RoadStopID::BEGIN.0);
    loop {
        let rs = (t.pool_next)(index);
        if rs.state.is_null() {
            break;
        }
        if status(rs.state).test(StatusFlag::BaseEntry) {
            crate::witness::hit(crate::witness::ROAD_STOP_AFTER_LOAD);
            rebuild(s, TileIndex(rs.xy), rs.state, true);
            rebuild(s, TileIndex(rs.xy), rs.state, false);
        }
        index = rs.index + 1;
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadStopEntry" ["probe"] => crate::road_stop::Entry { length, occupied, };
    "OpenTTDRoadStopPoolItem" ["probe"] => crate::road_stop::PoolStop { state, xy, index, };
    "OpenTTDRoadStopEntries" ["probe"] => crate::road_stop::Entries { east, west, };
    "OpenTTDRoadStopState" ["probe"] => crate::road_stop::State { status, entries, };
    "OpenTTDRoadStopServices" ["probe"] => crate::road_stop::RoadStopServices {
        vehicle,
        ground,
        primary,
        head,
        is_bus,
        next,
        tile,
        state,
        pool_next,
    };
}
