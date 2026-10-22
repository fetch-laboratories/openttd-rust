/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Road stops (`roadstop.cpp`) own bay admission, entrance status and the
//! drive-through entry topology and occupancy. Each native `RoadStop` shell owns
//! one Rust `State`; the `BaseEntry` stop of a drive-through line owns the shared
//! `Entries`, which every stop of the line points at. Native shells, the pool,
//! station stop lists, map tiles and the vehicle hash stay native services.
//! Stores go through the raw owner pointers, so no reference into a state or
//! entry survives a service call, and services never reenter road stops.
#![allow(unsafe_code, clippy::many_single_char_names)]
use crate::direction_func::dir_to_diag_dir;
use crate::direction_type::{DIAGDIR_NE, DIAGDIR_NW, DIAGDIR_SE, DIAGDIR_SW};
use crate::road::State as RoadState;
use crate::services::{Map, RoadStopType, Services};
use crate::station::BaseStation;
use crate::tile_type::{MP_STATION, TILE_SIZE};
use crate::vehicle::{GroundServices, VS_CRASHED, VehicleRef, VehicleServices, VehicleType};

/// `RoadStopStatusFlag::Bay0Free`.
const BAY0_FREE: u8 = 1 << 0;
/// `RoadStopStatusFlag::Bay1Free`.
const BAY1_FREE: u8 = 1 << 1;
/// `RoadStopStatusFlag::BaseEntry`.
const BASE_ENTRY: u8 = 1 << 6;
/// `RoadStopStatusFlag::EntryBusy`.
const ENTRY_BUSY: u8 = 1 << 7;
/// `RVS_USING_SECOND_BAY`.
const USING_SECOND_BAY: u8 = 1;
/// `RVSB_IN_ROAD_STOP`, also `1 << RVS_IN_ROAD_STOP`.
const IN_ROAD_STOP: u8 = 1 << 5;
/// `1 << RVS_IN_DT_ROAD_STOP`.
const IN_DT_ROAD_STOP: u8 = 1 << 6;
/// `TILE_SIZE` in the `uint16_t` entry arithmetic.
#[allow(clippy::cast_possible_truncation)] // TILE_SIZE is 16.
const TILE_UNITS: u16 = TILE_SIZE as u16;

/// `RoadStop::Entry`: one drive-through entry point in tile units.
#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Entry {
    pub length: u16,
    pub occupied: u16,
}
/// `RoadStop::Entries`: both entry points of one drive-through line.
#[repr(C)]
#[derive(Default)]
pub struct Entries {
    pub east: Entry,
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
/// Drive-through entry inputs of a road vehicle.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DriveRead {
    pub total_length: u16,
    pub direction: u8,
}
/// Rebuild inputs of one vehicle on a stop tile; road fields are zero for
/// other vehicle types.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RebuildRead {
    pub road: *mut RoadState,
    pub total_length: u16,
    pub vehicle_type: VehicleType,
    pub direction: u8,
    pub status: u8,
    pub primary: bool,
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
    /// `RoadStop::next`.
    pub next: extern "C" fn(*const Stop) -> *mut Stop,
    /// `RoadStop::xy`.
    pub tile: extern "C" fn(*const Stop) -> u32,
    /// The shell's Rust owner.
    pub state: extern "C" fn(*const Stop) -> *mut State,
    /// First live `RoadStop::Iterate(index)` item.
    pub pool_next: extern "C" fn(u32) -> PoolStop,
    /// Drive-through `Enter`/`Leave` read of a road vehicle.
    pub drive_read: extern "C" fn(VehicleRef) -> DriveRead,
    /// Rebuild filter read of any vehicle in the tile hash.
    pub rebuild_read: extern "C" fn(VehicleRef) -> RebuildRead,
}

/// Map dimensions read at the entry's use.
#[allow(clippy::cast_possible_truncation)] // Map::LogX() is at most 12.
fn map(s: &Services) -> Map {
    Map {
        log_x: (s.map_log_x)() as u8,
    }
}

fn table(s: &Services) -> &RoadStopServices {
    // SAFETY: The shared table returns a static immutable table.
    unsafe { &*(s.road_stops)() }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_road_stop_new() -> *mut State {
    Box::into_raw(Box::new(State {
        status: BAY0_FREE | BAY1_FREE,
        entries: std::ptr::null_mut(),
    }))
}
/// `RoadStop::~RoadStop`: the head frees the shared entries.
/// # Safety
/// One native shell transfers its live owner exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_stop_destroy(state: *mut State) {
    // SAFETY: Caller transfers the owner; `delete nullptr` is a no-op.
    unsafe {
        let state = Box::from_raw(state);
        if state.status & BASE_ENTRY != 0 && !state.entries.is_null() {
            drop(Box::from_raw(state.entries));
        }
    }
}

fn status(state: *mut State) -> u8 {
    // SAFETY: Live owner; the load ends before any service.
    unsafe { (*state).status }
}
fn set_status(state: *mut State, status: u8) {
    // SAFETY: Live owner, serialized game thread.
    unsafe { (*state).status = status }
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
/// `RoadStop::GetEntry(dir)`.
fn entry(state: *mut State, dir: u8) -> *mut Entry {
    side(entries(state), dir < DIAGDIR_SW)
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

/// `RoadStop::IsDriveThroughRoadStopContinuation`.
pub(crate) fn continuation(s: &Services, rs: u32, next: u32) -> bool {
    (s.tile_type)(next) == MP_STATION
        && (s.station_index)(next) == (s.station_index)(rs)
        && (s.station_type)(next).0 == (s.station_type)(rs).0
        && (s.is_drive_through_stop)(next)
        && (s.drive_through_axis)(next) == (s.drive_through_axis)(rs)
}

/// `RoadStop::GetByTile`: the shell and its owner.
pub(crate) fn get_by_tile(s: &Services, tile: u32, kind: RoadStopType) -> (*mut Stop, *mut State) {
    let t = table(s);
    let mut rs = (t.primary)(tile, kind);
    loop {
        if (t.tile)(rs) == tile {
            return (rs, (t.state)(rs));
        }
        let next = (t.next)(rs);
        debug_assert!(!next.is_null());
        rs = next;
    }
}
/// `RoadStop::GetByTile(tile, GetRoadStopType(tile))`.
pub(crate) fn at(s: &Services, tile: u32) -> *mut State {
    get_by_tile(s, tile, (s.road_stop_type)(tile)).1
}

/// `rs->GetEntry(dir)`, copied for a native-equivalent read.
pub(crate) fn entry_read(state: *mut State, dir: u8) -> Entry {
    // SAFETY: Live shared entries of a drive-through stop; the copy ends the load.
    unsafe { *entry(state, dir) }
}
/// `RoadStop::HasFreeBay`.
pub(crate) fn has_free_bay(state: *mut State) -> bool {
    status(state) & (BAY0_FREE | BAY1_FREE) != 0
}
/// `RoadStop::IsFreeBay`.
pub(crate) fn is_free_bay(state: *mut State, nr: u8) -> bool {
    match nr {
        0 => status(state) & BAY0_FREE != 0,
        1 => status(state) & BAY1_FREE != 0,
        _ => unreachable!("original IsFreeBay NOT_REACHED"),
    }
}
/// `RoadStop::IsEntranceBusy`.
pub(crate) fn is_entrance_busy(state: *mut State) -> bool {
    status(state) & ENTRY_BUSY != 0
}
/// `RoadStop::SetEntranceBusy`.
pub(crate) fn set_entrance_busy(state: *mut State, busy: bool) {
    let status = status(state);
    set_status(
        state,
        if busy {
            status | ENTRY_BUSY
        } else {
            status & !ENTRY_BUSY
        },
    );
}
/// `RoadStop::AllocateBay`.
fn allocate_bay(state: *mut State) -> u8 {
    debug_assert!(has_free_bay(state));
    let mut bay_nr = 0;
    while !is_free_bay(state, bay_nr) {
        bay_nr += 1;
    }
    // AllocateDriveThroughBay(bay_nr)
    set_status(
        state,
        status(state)
            & !match bay_nr {
                0 => BAY0_FREE,
                1 => BAY1_FREE,
                _ => unreachable!("original AllocateDriveThroughBay NOT_REACHED"),
            },
    );
    bay_nr
}
/// `RoadStop::FreeBay`.
fn free_bay(state: *mut State, nr: u8) {
    set_status(
        state,
        status(state)
            | match nr {
                0 => BAY0_FREE,
                1 => BAY1_FREE,
                _ => unreachable!("original FreeBay NOT_REACHED"),
            },
    );
}

/// `RoadStop::Enter` for the stop at `xy`.
/// # Safety
/// Live serialized stop and road owners of the front vehicle `v`.
pub(crate) unsafe fn enter(
    s: &Services,
    state: *mut State,
    xy: u32,
    v: VehicleRef,
    road: *mut RoadState,
) -> bool {
    let t = table(s);
    if (s.is_bay_stop)(xy) {
        // SAFETY: The shared road ground table is static.
        let ground = unsafe { &*t.ground };
        if is_entrance_busy(state) || !has_free_bay(state) || (ground.has_articulated)(v) {
            crate::witness::hit(crate::witness::ROAD_STOP_ENTER_REFUSED);
            return false;
        }
        // SAFETY: Caller's live road owner; no service runs during these stores.
        unsafe {
            crate::road::set_state(road, crate::road::get_state(road) | IN_ROAD_STOP);
            let bay_nr = allocate_bay(state);
            crate::witness::hit(if bay_nr == 0 {
                crate::witness::ROAD_STOP_ENTER_BAY0
            } else {
                crate::witness::ROAD_STOP_ENTER_BAY1
            });
            let rv_state = crate::road::get_state(road);
            crate::road::set_state(
                road,
                (rv_state & !(1 << USING_SECOND_BAY)) | (bay_nr << USING_SECOND_BAY),
            );
        }
        set_entrance_busy(state, true);
        return true;
    }
    crate::witness::hit(crate::witness::ROAD_STOP_ENTER_DRIVE);
    let r = (t.drive_read)(v);
    let e = entry(state, dir_to_diag_dir(r.direction));
    // SAFETY: Live shared entry; RVs may run through each other, so no bound.
    unsafe {
        (*e).occupied = (*e).occupied.wrapping_add(r.total_length);
        crate::road::set_state(road, crate::road::get_state(road) | IN_DT_ROAD_STOP);
    }
    true
}

/// `RoadStop::Leave` for a vehicle on tile `tile`.
/// # Safety
/// Live serialized stop and road owners of the front vehicle `v`.
pub(crate) unsafe fn leave(
    s: &Services,
    state: *mut State,
    tile: u32,
    v: VehicleRef,
    road: *mut RoadState,
) {
    if (s.is_bay_stop)(tile) {
        crate::witness::hit(crate::witness::ROAD_STOP_LEAVE_BAY);
        // SAFETY: Caller's live road owner, read before any store.
        let rv_state = unsafe { crate::road::get_state(road) };
        free_bay(state, (rv_state >> USING_SECOND_BAY) & 1);
        set_entrance_busy(state, false);
    } else {
        crate::witness::hit(crate::witness::ROAD_STOP_LEAVE_DRIVE);
        let r = (table(s).drive_read)(v);
        let e = entry(state, dir_to_diag_dir(r.direction));
        // SAFETY: Live shared entry; uint16_t arithmetic.
        unsafe {
            debug_assert!((*e).occupied >= r.total_length);
            (*e).occupied = (*e).occupied.wrapping_sub(r.total_length);
        }
    }
}
/// `RoadStop::GetByTile(v->tile, GetRoadStopType(v->tile))->Leave(v)`.
/// # Safety
/// Live serialized road owner of the front vehicle `v`.
pub(crate) unsafe fn leave_vehicle(
    s: &Services,
    vehicle: &VehicleServices,
    v: VehicleRef,
    road: *mut RoadState,
) {
    let tile = (vehicle.tile)(v);
    // SAFETY: Caller's contract.
    unsafe { leave(s, at(s, tile), tile, v, road) };
}

/// `GetEntryDirection`.
fn entry_direction(east: bool, axis: u8) -> u8 {
    match axis {
        0 => {
            if east {
                DIAGDIR_NE
            } else {
                DIAGDIR_SW
            }
        }
        1 => {
            if east {
                DIAGDIR_SE
            } else {
                DIAGDIR_NW
            }
        }
        _ => unreachable!("original GetEntryDirection NOT_REACHED"),
    }
}

/// `RoadStop::Entry::Rebuild` from the base stop at `xy`, returning the entry.
fn rebuild_entry(s: &Services, xy: u32, state: *mut State, east: bool) -> Entry {
    debug_assert!(status(state) & BASE_ENTRY != 0);
    let t = table(s);
    // SAFETY: The shared vehicle table is static.
    let vehicle = unsafe { &*t.vehicle };
    let axis = (s.drive_through_axis)(xy);
    let entry_dir = entry_direction(east, axis);
    let mut vehicles: Vec<(VehicleRef, u16)> = Vec::new();
    let mut length: u16 = 0;
    let offset = map(s).offs_by_axis(axis);
    let mut tile = xy;
    while continuation(s, xy, tile) {
        length = length.wrapping_add(TILE_UNITS);
        vehicle.for_each_on_tile(tile, |v| {
            let r = (t.rebuild_read)(v);
            // Not a RV or not in the right direction or crashed :(
            if r.vehicle_type != VehicleType::ROAD
                || dir_to_diag_dir(r.direction) != entry_dir
                || !r.primary
                || r.status & VS_CRASHED != 0
            {
                return;
            }
            // Don't add ones not in a road stop.
            // SAFETY: Live road owner of this road vehicle shell.
            if unsafe { crate::road::get_state(r.road) } < IN_ROAD_STOP {
                return;
            }
            crate::witness::hit(crate::witness::ROAD_STOP_REBUILD_VEHICLE);
            if !vehicles.iter().any(|&(u, _)| u == v) {
                vehicles.push((v, r.total_length));
            }
        });
        tile = tile.wrapping_add(offset);
    }
    let mut occupied: u16 = 0;
    for &(_, total_length) in &vehicles {
        occupied = occupied.wrapping_add(total_length);
    }
    Entry { length, occupied }
}
/// `entries->east.Rebuild(rs)` or `entries->west.Rebuild(rs)`.
fn rebuild(s: &Services, xy: u32, state: *mut State, east: bool) {
    crate::witness::hit(crate::witness::ROAD_STOP_REBUILD);
    let built = rebuild_entry(s, xy, state, east);
    // SAFETY: Live shared entries of this base stop; no service runs here.
    unsafe { *side(entries(state), east) = built };
}

/// `RoadStop::MakeDriveThrough` for the stop at `xy`.
pub(crate) fn make_drive_through(s: &Services, xy: u32, state: *mut State) {
    debug_assert!(entries(state).is_null());
    let rst = (s.road_stop_type)(xy);
    let axis = (s.drive_through_axis)(xy);
    let offset = map(s).offs_by_axis(axis);

    // Information about the tile north of us.
    let north_tile = xy.wrapping_sub(offset);
    let north = continuation(s, xy, north_tile);
    let rs_north = if north {
        get_by_tile(s, north_tile, rst).1
    } else {
        std::ptr::null_mut()
    };
    // Information about the tile south of us.
    let mut south_tile = xy.wrapping_add(offset);
    let south = continuation(s, xy, south_tile);
    let mut rs_south = if south {
        get_by_tile(s, south_tile, rst).1
    } else {
        std::ptr::null_mut()
    };

    // Amount of road stops that will be added to the 'northern' head.
    let mut added: u16 = 1;
    if north && !entries(rs_north).is_null() {
        // There is a more northern one, so this can join them.
        let joined = entries(rs_north);
        set_entries(state, joined);
        if south && !entries(rs_south).is_null() {
            crate::witness::hit(crate::witness::ROAD_STOP_MAKE_MERGE);
            // There more southern tiles too, they must 'join' us too.
            set_status(rs_south, status(rs_south) & !BASE_ENTRY);
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
                // Free the now unneeded entries struct.
                drop(Box::from_raw(southern));
            }
            // Make all 'children' of the southern tile take the new master.
            while continuation(s, xy, south_tile) {
                rs_south = get_by_tile(s, south_tile, rst).1;
                if entries(rs_south).is_null() {
                    break;
                }
                set_entries(rs_south, entries(rs_north));
                added = added.wrapping_add(1);
                south_tile = south_tile.wrapping_add(offset);
            }
        } else {
            crate::witness::hit(crate::witness::ROAD_STOP_MAKE_JOIN);
        }
    } else if south && !entries(rs_south).is_null() {
        crate::witness::hit(crate::witness::ROAD_STOP_MAKE_PARENT);
        // There is one to the south, but not to the north... so we become 'parent'.
        set_entries(state, entries(rs_south));
        set_status(state, status(state) | BASE_ENTRY);
        set_status(rs_south, status(rs_south) & !BASE_ENTRY);
    } else {
        crate::witness::hit(crate::witness::ROAD_STOP_MAKE_ALONE);
        // We are the only... so we are automatically the master.
        set_entries(state, Box::into_raw(Box::default()));
        set_status(state, status(state) | BASE_ENTRY);
    }

    // Now update the lengths.
    added = added.wrapping_mul(TILE_UNITS);
    add_length(entries(state), added);
}

/// `RoadStop::ClearDriveThrough` for the stop at `xy`.
pub(crate) fn clear_drive_through(s: &Services, xy: u32, state: *mut State) {
    debug_assert!(!entries(state).is_null());
    let rst = (s.road_stop_type)(xy);
    let axis = (s.drive_through_axis)(xy);
    let offset = map(s).offs_by_axis(axis);

    // Information about the tile north of us.
    let mut north_tile = xy.wrapping_sub(offset);
    let north = continuation(s, xy, north_tile);
    let mut rs_north = if north {
        get_by_tile(s, north_tile, rst).1
    } else {
        std::ptr::null_mut()
    };
    let mut rs_north_xy = north_tile;
    // Information about the tile south of us.
    let mut south_tile = xy.wrapping_add(offset);
    let south = continuation(s, xy, south_tile);
    let mut rs_south = if south {
        get_by_tile(s, south_tile, rst).1
    } else {
        std::ptr::null_mut()
    };

    // Must only be cleared after we determined which neighbours are
    // part of our little entry 'queue'.
    crate::ground::clear_square(xy, s);

    if north {
        // There is a tile to the north, so we can't clear ourselves.
        if south {
            crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_SPLIT);
            // There are more southern tiles too, they must be split;
            // first make the new southern 'base'.
            set_status(rs_south, status(rs_south) | BASE_ENTRY);
            set_entries(rs_south, Box::into_raw(Box::default()));

            // Keep track of the base because we need it later on.
            let rs_south_base = rs_south;
            let base_tile = south_tile;

            // Make all (even more) southern stops part of the new entry queue.
            south_tile = south_tile.wrapping_add(offset);
            while continuation(s, base_tile, south_tile) {
                rs_south = get_by_tile(s, south_tile, rst).1;
                set_entries(rs_south, entries(rs_south_base));
                south_tile = south_tile.wrapping_add(offset);
            }

            // Find the other end; the northern most tile.
            while continuation(s, base_tile, north_tile) {
                rs_north = get_by_tile(s, north_tile, rst).1;
                rs_north_xy = north_tile;
                north_tile = north_tile.wrapping_sub(offset);
            }

            // Rebuild both parts from scratch instead of maintaining a vehicle list.
            rebuild(s, base_tile, rs_south_base, true);
            rebuild(s, base_tile, rs_south_base, false);

            debug_assert!(status(rs_north) & BASE_ENTRY != 0);
            rebuild(s, rs_north_xy, rs_north, true);
            rebuild(s, rs_north_xy, rs_north, false);
        } else {
            crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_TAIL);
            // Only we left, so simple update the length.
            sub_length(entries(rs_north), TILE_UNITS);
        }
    } else if south {
        crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_HEAD);
        // There is only something to the south. Hand over the base entry.
        set_status(rs_south, status(rs_south) | BASE_ENTRY);
        sub_length(entries(rs_south), TILE_UNITS);
    } else {
        crate::witness::hit(crate::witness::ROAD_STOP_CLEAR_LAST);
        // We were the last.
        // SAFETY: This sole stop of its line owns the entries.
        unsafe { drop(Box::from_raw(entries(state))) };
    }

    // Make sure we don't get used for something 'incorrect'.
    set_status(state, status(state) & !BASE_ENTRY);
    set_entries(state, std::ptr::null_mut());
}

/// `RoadStop::Entry::CheckIntegrity` of `rs->GetEntry(dir)`.
fn check_integrity(s: &Services, xy: u32, state: *mut State, dir: u8) {
    if status(state) & BASE_ENTRY == 0 {
        return;
    }
    crate::witness::hit(crate::witness::ROAD_STOP_INTEGRITY);
    // The tile 'before' the road stop must not be part of this 'line'.
    debug_assert!((s.is_drive_through_stop)(xy));
    debug_assert!(!continuation(
        s,
        xy,
        xy.wrapping_sub(map(s).offs_by_axis((s.drive_through_axis)(xy)))
    ));
    let east = dir < DIAGDIR_SW;
    let temp = rebuild_entry(s, xy, state, east);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_stop_enter(
    s: *const Services,
    tile: u32,
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
        leave_vehicle(s, &*table(s).vehicle, v, road);
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
    check_integrity(unsafe { &*s }, xy, state, dir);
}
/// `AfterLoadRoadStops`: construct the drive-through entries, then rebuild
/// the data in those entries, each in pool order.
/// # Safety
/// Live serialized shared table after the ROAD chunk pointers are fixed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_road_stop_after_load(s: *const Services) {
    // SAFETY: Caller's contract.
    let s = unsafe { &*s };
    let t = table(s);
    let mut index = 0;
    loop {
        let rs = (t.pool_next)(index);
        if rs.state.is_null() {
            break;
        }
        if (s.is_drive_through_stop)(rs.xy) {
            make_drive_through(s, rs.xy, rs.state);
        }
        index = rs.index + 1;
    }
    index = 0;
    loop {
        let rs = (t.pool_next)(index);
        if rs.state.is_null() {
            break;
        }
        if status(rs.state) & BASE_ENTRY != 0 {
            crate::witness::hit(crate::witness::ROAD_STOP_AFTER_LOAD);
            rebuild(s, rs.xy, rs.state, true);
            rebuild(s, rs.xy, rs.state, false);
        }
        index = rs.index + 1;
    }
}
