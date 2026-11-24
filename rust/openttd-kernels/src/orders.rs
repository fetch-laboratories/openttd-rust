/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Orders own their contiguous storage, lifecycle and timetable state; C++
//! placement-constructs typed objects before exposing facade aliases. Record pointers
//! come from the entry and stay live and serialized for the call. Each access to an
//! aliased field is a field-sized raw copy, and no Rust reference to a C++ Order, List or
//! Consist survives a service call, mutation or reentry. Panics abort.
// Native-width casts and branch structure reproduce the original order commands.
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::if_not_else,
    clippy::collapsible_match
)]
use crate::bitmath_func::{assign_bit16, gb_u8, has_bit};
use crate::cargo_type::{CARGO_AUTO_REFIT, CargoType};
use crate::company_type::OWNER_NONE;
use crate::depot_type::DepotID;
use crate::ground_vehicle::GVF_SUPPRESS_IMPLICIT_ORDERS;
use crate::order_type::{
    CloneOptions, OT_CONDITIONAL, OT_DUMMY, OT_GOTO_DEPOT, OT_GOTO_STATION, OT_GOTO_WAYPOINT,
    OT_IMPLICIT, OT_LEAVESTATION, OT_LOADING, OT_NOTHING, OrderType, VehicleOrderID,
};
use crate::raw_field::{read_field, write_field};
use crate::station_type::StationID;
use crate::strings_type::StringID;
use crate::table::strings::{
    STR_ERROR_AIRCRAFT_NOT_ENOUGH_RANGE, STR_ERROR_CAN_T_ADD_ORDER,
    STR_ERROR_CAN_T_ADD_ORDER_SHARED, STR_ERROR_CAN_T_COPY_SHARE_ORDER, STR_ERROR_NO_BUOY,
    STR_ERROR_NO_MORE_SPACE_FOR_ORDERS, STR_ERROR_NO_RAIL_WAYPOINT, STR_ERROR_NO_ROAD_WAYPOINT,
    STR_ERROR_TIMETABLE_INCOMPLETE, STR_ERROR_TIMETABLE_NOT_STARTED,
    STR_ERROR_TIMETABLE_NOT_STOPPING_HERE, STR_ERROR_TIMETABLE_ONLY_WAIT_AT_STATIONS,
    STR_ERROR_TOO_MANY_ORDERS, STR_ERROR_UNBUNCHING_NO_CONDITIONAL,
    STR_ERROR_UNBUNCHING_NO_FULL_LOAD, STR_ERROR_UNBUNCHING_NO_UNBUNCHING_CONDITIONAL,
    STR_ERROR_UNBUNCHING_NO_UNBUNCHING_FULL_LOAD, STR_ERROR_UNBUNCHING_ONLY_ONE_ALLOWED,
};
use crate::tile_type::TileIndex;
use crate::timer_game_common::DAYS_IN_LEAP_YEAR;
use crate::timer_game_tick::DAY_TICKS;
use crate::vehicle::{VehicleRef, VehicleServices, maintenance};
use crate::vehicle_base::{VS_CRASHED, VS_STOPPED};
use crate::vehicle_type::{VEH_AIRCRAFT, VEH_ROAD, VEH_SHIP, VEH_TRAIN, VehicleType};
use std::ffi::c_void;

/// Opaque live native `OrderList`; never dereferenced by Rust.
#[repr(C)]
pub struct NativeOrderList {
    _private: [u8; 0],
}
use std::mem::MaybeUninit;
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Order {
    pub kind: u8,
    pub flags: u8,
    pub destination: u16,
    pub refit: u8,
    pub wait: u16,
    pub travel: u16,
    pub speed: u16,
}
#[repr(C)]
pub struct Consist {
    pub time: i32,
    pub lateness: i32,
    pub start: u64,
    pub last_departure: u64,
    pub next_departure: u64,
    pub round_trip: i32,
    pub real: u8,
    pub implicit: u8,
    pub flags: u16,
}
#[repr(C)]
pub struct VehicleOrders {
    pub current: MaybeUninit<Order>,
    pub orders: *mut c_void,
    pub next: *mut c_void,
    pub previous: *mut c_void,
}
#[repr(C)]
pub struct List {
    pub manual: u8,
    pub vehicles: u32,
    pub first: *mut c_void,
    pub timetable: i32,
    pub total: i32,
}
#[repr(C)]
pub struct Backup {
    pub user: u32,
    pub tile: u32,
    pub group: u16,
    pub clone: *const c_void,
}

/// Copy the whole Order at `order`.
fn load(order: *const Order) -> Order {
    // SAFETY: The module contract keeps the Order live; the copy creates no reference.
    unsafe { order.read() }
}
/// Store a whole Order at `order`.
fn store(order: *mut Order, value: Order) {
    // SAFETY: The module contract keeps the Order live and writable; no reference is created.
    unsafe { order.write(value) }
}
/// The Vehicle's `current_order`.
fn current_order(vehicle: *mut VehicleOrders) -> *mut Order {
    // SAFETY: The module contract keeps the shell live; this takes the field address only.
    unsafe { (&raw mut (*vehicle).current).cast() }
}
/// The storage of a live order vector, null while it has no capacity.
fn data(vector: *const Vector) -> *mut Order {
    // SAFETY: The module contract keeps the vector live; `data` reads its storage pointer.
    unsafe { (*vector).data() }
}
/// The order slot `index` of `orders`, which the caller keeps within its capacity.
fn at(orders: *mut Order, index: usize) -> *mut Order {
    // SAFETY: Callers index within the allocation, as the original's span indexing does.
    unsafe { orders.add(index) }
}
/// The order slot `index` of a live vector.
fn slot(vector: *const Vector, index: usize) -> *mut Order {
    at(data(vector), index)
}
/// Call an export of this module from inside an entry with that entry's live shells,
/// outputs and static table.
macro_rules! reenter {
    ($export:ident($($arg:expr),* $(,)?)) => {
        // SAFETY: The export's pointer contract is the calling entry's own.
        unsafe { $export($($arg),*) }
    };
}

// Entry-local raw handles. No owner reference or cached vector data survives a
// callback that can mutate/reenter orders; callers reacquire after such calls.
struct OrderAccess {
    list: *mut List,
    vector: *mut Vector,
}
impl OrderAccess {
    fn count(&self) -> u8 {
        if self.vector.is_null() {
            0
        } else {
            read_field!(self.vector, length) as u8
        }
    }
    fn manual(&self) -> u8 {
        if self.list.is_null() {
            0
        } else {
            read_field!(self.list, manual)
        }
    }
    fn order_ptr(&self, index: u8) -> Option<*const Order> {
        if index >= self.count() {
            None
        } else {
            // The handle must not survive list mutation.
            Some(slot(self.vector, usize::from(index)).cast_const())
        }
    }
    fn kind(&self, index: usize) -> u8 {
        read_field!(slot(self.vector, index), kind) & 15
    }
    fn update_real(&self, state: *mut Consist) {
        let count = self.count();
        if read_field!(state, real) >= count {
            write_field!(state, real, 0);
        }
        if self.manual() > 0 {
            while self.kind(usize::from(read_field!(state, real))) == OT_IMPLICIT {
                write_field!(state, real, read_field!(state, real).wrapping_add(1));
                if read_field!(state, real) >= count {
                    write_field!(state, real, 0);
                }
            }
        } else {
            write_field!(state, real, 0);
        }
    }
    fn valid(&self) -> bool {
        !self.vector.is_null()
            && (0..read_field!(self.vector, length)).any(|index| {
                matches!(
                    self.kind(index),
                    OT_GOTO_STATION | OT_GOTO_DEPOT | OT_GOTO_WAYPOINT
                )
            })
    }
}

pub struct Vector {
    storage: Box<[MaybeUninit<Order>]>,
    length: usize,
    construct: unsafe extern "C" fn(*mut c_void, usize),
    destroy: unsafe extern "C" fn(*mut c_void, usize),
}
impl Vector {
    fn new(
        construct: unsafe extern "C" fn(*mut c_void, usize),
        destroy: unsafe extern "C" fn(*mut c_void, usize),
    ) -> Self {
        Self {
            storage: Box::new([]),
            length: 0,
            construct,
            destroy,
        }
    }
    fn data(&self) -> *mut Order {
        if self.storage.is_empty() {
            ptr::null_mut()
        } else {
            self.storage.as_ptr().cast_mut().cast()
        }
    }
    unsafe fn reserve(&mut self, required: usize) {
        if required <= self.storage.len() {
            return;
        }
        // Match the original standard-library growth policy on supported targets.
        #[cfg(target_env = "msvc")]
        let capacity = required.max(self.storage.len() + self.storage.len() / 2);
        #[cfg(not(target_env = "msvc"))]
        let capacity = required.max(self.length + self.length.max(1));
        let mut storage = Box::<[Order]>::new_uninit_slice(capacity);
        // SAFETY: New aligned storage is exclusive; the leaf begins C++ Order
        // lifetimes, including each DestinationID, without running simulation.
        unsafe {
            (self.construct)(storage.as_mut_ptr().cast(), capacity);
        }
        // SAFETY: Both ranges contain live trivially copyable C++ Order objects.
        // Existing values are copied before their source lifetimes end.
        unsafe {
            if self.length != 0 {
                ptr::copy_nonoverlapping(self.data(), storage.as_mut_ptr().cast(), self.length);
            }
            (self.destroy)(self.data().cast(), self.storage.len());
        }
        self.storage = storage;
    }
    unsafe fn resize(&mut self, size: usize) {
        // SAFETY: This vector is live and exclusively borrowed for the resize.
        unsafe {
            self.reserve(size);
        }
        if size > self.length {
            // Capacity slots stay live after logical erasure. The original's
            // DeleteUnreachedImplicitOrders keeps a span across erase and reads
            // the retained last slot on wraparound; preserve those bytes until
            // a subsequent resize exposes the slot with its default value.
            // Order destruction/construction is trivial and cannot reenter.
            // SAFETY: The C++ hooks construct/destroy trivial Orders in owned storage and cannot reenter.
            unsafe {
                (self.destroy)(self.data().add(self.length).cast(), size - self.length);
            }
            // SAFETY: The C++ hooks construct/destroy trivial Orders in owned storage and cannot reenter.
            unsafe {
                (self.construct)(self.data().add(self.length).cast(), size - self.length);
            }
        }
        self.length = size;
    }
}
impl Drop for Vector {
    fn drop(&mut self) {
        // SAFETY: No access remains when the shell destroys its unique owner.
        unsafe {
            (self.destroy)(self.data().cast(), self.storage.len());
        }
    }
}

/// All owner handles below are live, uniquely owned shell allocations. C++ does
/// not free Rust allocations; deletion happens once after typed access has ended.
/// Exports never unwind. Scalars have the original defaults and widths.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_consist_new() -> *mut Consist {
    Box::into_raw(Box::new(Consist {
        time: 0,
        lateness: 0,
        start: 0,
        last_departure: 0,
        next_departure: 0,
        round_trip: 0,
        real: 0,
        implicit: 0,
        flags: 0,
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_delete(p: *mut Consist) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
/// `BaseConsist::ResetDepotUnbunching`: reset all depot unbunching times.
///
/// # Safety
/// `p` is a live consist owner of the serialized game state.
unsafe fn reset_depot_unbunching(p: *mut Consist) {
    write_field!(p, last_departure, 0);
    write_field!(p, next_departure, 0);
    write_field!(p, round_trip, 0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_reset(p: *mut Consist) {
    // SAFETY: As `reset_depot_unbunching`.
    unsafe { reset_depot_unbunching(p) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_copy(p: *mut Consist, src: *const Consist) {
    if ptr::eq(p, src) {
        return;
    }
    write_field!(p, time, read_field!(src, time));
    write_field!(p, lateness, read_field!(src, lateness));
    write_field!(p, start, read_field!(src, start));
    write_field!(p, real, read_field!(src, real));
    write_field!(p, implicit, read_field!(src, implicit));
    write_field!(
        p,
        flags,
        read_field!(p, flags)
            | read_field!(src, flags) & ((1 << 3) | (1 << 4) | (1 << 5) | (1 << 8))
    );
    write_field!(
        p,
        flags,
        (read_field!(p, flags) & !(1 << 9)) | (read_field!(src, flags) & (1 << 9))
    );
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_vehicle_orders_new() -> *mut VehicleOrders {
    Box::into_raw(Box::new(VehicleOrders {
        current: MaybeUninit::uninit(),
        orders: ptr::null_mut(),
        next: ptr::null_mut(),
        previous: ptr::null_mut(),
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_orders_delete(p: *mut VehicleOrders) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orderlist_new() -> *mut List {
    Box::into_raw(Box::new(List {
        manual: 0,
        vehicles: 0,
        first: ptr::null_mut(),
        timetable: 0,
        total: 0,
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_orderlist_delete(p: *mut List) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orderbackup_new() -> *mut Backup {
    Box::into_raw(Box::new(Backup {
        user: 0,
        tile: u32::MAX,
        group: u16::MAX,
        clone: ptr::null(),
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_orderbackup_delete(p: *mut Backup) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_vector_new(
    construct: unsafe extern "C" fn(*mut c_void, usize),
    destroy: unsafe extern "C" fn(*mut c_void, usize),
) -> *mut Vector {
    Box::into_raw(Box::new(Vector::new(construct, destroy)))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_delete(p: *mut Vector) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_size(p: *const Vector) -> usize {
    read_field!(p, length)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_data(p: *const Vector) -> *mut Order {
    data(p)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_resize(p: *mut Vector, size: usize) {
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    unsafe {
        (*p).resize(size);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_insert(
    p: *mut Vector,
    index: usize,
    src: *const Order,
) {
    // Copy before reserve: original emplace permits its input to alias a live order.
    let order = load(src);
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    let v = unsafe { &mut *p };
    let index = index.min(v.length);
    // SAFETY: After `reserve`, the shifted range and the inserted slot fit in capacity.
    unsafe {
        v.reserve(v.length + 1);
        ptr::copy(
            v.data().add(index),
            v.data().add(index + 1),
            v.length - index,
        );
        v.data().add(index).write(order);
    }
    v.length += 1;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_erase(p: *mut Vector, index: usize) {
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    let v = unsafe { &mut *p };
    if index >= v.length {
        return;
    }
    // SAFETY: `index` is below the length, so the shifted tail lies within live Orders.
    unsafe {
        ptr::copy(
            v.data().add(index + 1),
            v.data().add(index),
            v.length - index - 1,
        );
        v.resize(v.length - 1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_move(p: *mut Vector, from: usize, to: usize) {
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    let v = unsafe { &mut *p };
    if from == to || from >= v.length || to >= v.length {
        return;
    }
    // SAFETY: `from` and `to` are below the length, so both ranges lie within live Orders.
    unsafe {
        let order = v.data().add(from).read();
        if from < to {
            ptr::copy(v.data().add(from + 1), v.data().add(from), to - from);
        } else {
            ptr::copy(v.data().add(to), v.data().add(to + 1), from - to);
        }
        v.data().add(to).write(order);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_assign(
    p: *mut Vector,
    src: *const Order,
    length: usize,
) {
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    let v = unsafe { &mut *p };
    // SAFETY: C++ supplies `length` live source Orders outside this vector's storage.
    unsafe {
        v.resize(length);
        if length != 0 {
            ptr::copy_nonoverlapping(src, v.data(), length);
        }
    }
}

/// Shared accessors expose typed world shells; all pointed-to fields are accessed
/// only as raw scalars. The immutable C++ table outlives each synchronous entry;
/// callbacks keep no Rust owner reference across reentry.
/// Services are typed noexcept calls; commands and destruction may reenter Rust.
///
/// Pointer-taking callbacks require the live native pointee type used by their
/// C++ body, with writable output storage where applicable. Deletion consumes
/// the detached shell; no access to its owner may remain. Destination lookups
/// also require a live station/depot/waypoint of the source branch's type even
/// though their arguments are scalar IDs. Distance inputs must be live map tiles
/// so the native signed square cannot overflow. No Rust owner borrow spans a callback.
/// Scalar-only settings, checked lookups and pool iteration need no such input
/// precondition.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub vehicle: unsafe extern "C" fn(*mut c_void) -> *mut VehicleOrders,
    pub consist: unsafe extern "C" fn(*mut c_void) -> *mut Consist,
    pub list: unsafe extern "C" fn(*mut c_void) -> *mut List,
    pub vector: unsafe extern "C" fn(*mut c_void) -> *mut Vector,
    pub backup: unsafe extern "C" fn(*mut c_void) -> *mut Backup,
    pub backup_vector: unsafe extern "C" fn(*mut c_void) -> *mut Vector,
    pub backup_consist: unsafe extern "C" fn(*mut c_void) -> *mut Consist,
    pub ownerless_station: unsafe extern "C" fn(station: u16, if_valid: u8) -> u64,
    pub vehicle_type: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub tick_counter: extern "C" fn() -> u64,
    pub primary_vehicle: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub ticks_per_second: extern "C" fn() -> u64,
    pub unit_number: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub maximum_date: extern "C" fn() -> u64,
    pub allocate_list: extern "C" fn() -> *mut c_void,
    pub shared_window: unsafe extern "C" fn(ctx: *mut c_void, first: *mut c_void) -> u64,
    pub vehicle_id: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub reliability: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub display_speed: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub age_years: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub needs_service: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub remaining_years: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub airport_tile: unsafe extern "C" fn(destination: u16) -> u64,
    pub depot_tile: unsafe extern "C" fn(destination: u16) -> u64,
    pub distance: unsafe extern "C" fn(first: u32, second: u32, square: u8) -> u64,
    pub station_location: unsafe extern "C" fn(ctx: *mut c_void, destination: u16) -> u64,
    pub destination_tile: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub aircraft_flying: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub target_airport: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub waypoint_tile: unsafe extern "C" fn(destination: u16) -> u64,
    pub at_station: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub tile_station: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub ship_station_tile: unsafe extern "C" fn(destination: u16) -> u64,
    pub valid_station: extern "C" fn(destination: u16) -> u64,
    pub station_owner: unsafe extern "C" fn(destination: u16) -> u64,
    pub station_error: unsafe extern "C" fn(ctx: *mut c_void, destination: u16) -> u64,
    pub valid_depot: extern "C" fn(destination: u16) -> u64,
    pub depot_owner: unsafe extern "C" fn(destination: u16) -> u64,
    pub rail_depot: unsafe extern "C" fn(destination: u16) -> u64,
    pub road_depot: unsafe extern "C" fn(destination: u16) -> u64,
    pub ship_depot: unsafe extern "C" fn(destination: u16) -> u64,
    pub valid_waypoint: extern "C" fn(destination: u16) -> u64,
    pub waypoint_facilities: unsafe extern "C" fn(destination: u16) -> u64,
    pub waypoint_owner: unsafe extern "C" fn(destination: u16) -> u64,
    pub list_capacity: extern "C" fn() -> u64,
    pub next_backup: unsafe extern "C" fn(cursor: *mut u32) -> *mut c_void,
    pub next_vehicle: extern "C" fn(first: u32) -> *mut c_void,
    pub aircraft_range: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub aircraft_range_square: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub backup_capacity: extern "C" fn() -> u64,
    pub create_backup: unsafe extern "C" fn(ctx: *mut c_void, user: u32) -> *mut c_void,
    pub networking: extern "C" fn() -> u64,
    pub network_server: extern "C" fn() -> bool,
    pub network_client: extern "C" fn() -> u64,
    pub server_client: extern "C" fn() -> u64,
    pub default_group: extern "C" fn() -> u64,
    pub vehicle_group: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub unique_backup_name: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub local_owner: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub no_jet_crash: extern "C" fn() -> u64,
    pub append_station: unsafe extern "C" fn(ctx: *mut c_void, station: u16),
    pub station_commands: *const crate::station_cmd::CommandServicesTable,
    pub timetable_dirty: unsafe extern "C" fn(ctx: *mut c_void),
    pub invalidate_order: unsafe extern "C" fn(ctx: *mut c_void, data: i32),
    pub delete_order_news: unsafe extern "C" fn(ctx: *mut c_void),
    pub invalidate_vehicle_list: unsafe extern "C" fn(ctx: *mut c_void),
    pub close_shared_window: unsafe extern "C" fn(ctx: *mut c_void, window: u32),
    pub invalidate_shared_window: unsafe extern "C" fn(ctx: *mut c_void, window: u32, data: i32),
    pub dirty_vehicle_windows: unsafe extern "C" fn(ctx: *mut c_void),
    pub capture_backup_metadata: unsafe extern "C" fn(ctx: *mut c_void, source: *mut c_void),
    pub clear_backup_name: unsafe extern "C" fn(ctx: *mut c_void),
    pub restore_backup_metadata: unsafe extern "C" fn(ctx: *mut c_void, backup: *mut c_void),
    pub order_news: unsafe extern "C" fn(ctx: *mut c_void, message: u32),
    pub debug_list: unsafe extern "C" fn(ctx: *mut c_void),
    pub assert_departure_range: extern "C" fn(lower: i32, upper: i32),
    pub delete_list: unsafe extern "C" fn(ctx: *mut c_void),
    pub reverse_train: unsafe extern "C" fn(ctx: *mut c_void),
    pub next_airport: unsafe extern "C" fn(ctx: *mut c_void),
    pub set_destination: unsafe extern "C" fn(ctx: *mut c_void, tile: u32),
    pub closest_depot: unsafe extern "C" fn(ctx: *mut c_void, result: *mut Closest),
    pub share_command: unsafe extern "C" fn(ctx: *mut c_void, source: *const c_void),
    pub group_command: unsafe extern "C" fn(ctx: *mut c_void, group: u16),
    pub delete_backup: unsafe extern "C" fn(ctx: *mut c_void),
    pub clear_backup_gui: extern "C" fn(tile: u32, user: u32),
    pub clear_backup_post: extern "C" fn(user: u32),
    pub missing_aircraft_orders: unsafe extern "C" fn(ctx: *mut c_void),
    pub change_timetable_command:
        unsafe extern "C" fn(ctx: *mut c_void, index: u8, field: u8, value: u16),
    pub base: &'static VehicleServices,
}
// Private helpers inherit the native entry's live-shell/output contracts and
// the source branches' vehicle/destination types; they never expose safe calls
// accepting arbitrary native pointers outside this module.
#[derive(Clone, Copy)]
struct Game(&'static Leaves);

/// Convert the live, non-null native Vehicle shell supplied by each entry.
fn native_vehicle(v: *mut c_void, base: &VehicleServices) -> VehicleRef {
    // SAFETY: Callers inherit their entry's live, non-null Vehicle contract.
    unsafe { VehicleRef::from_shell(v, base) }
}

/// Typed calls into the `Leaves` table. Callers pass the live native pointees and
/// destination types the table contract requires.
macro_rules! leaves {
    ($(fn $name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)?;)*) => {
        impl Game {
            $(fn $name(self, $($arg: $ty),*) $(-> $ret)? {
                // SAFETY: The caller meets the `Leaves` contract for this callback; no Rust
                // owner borrow crosses the call.
                unsafe { (self.0.$name)($($arg),*) }
            })*
        }
    };
}
leaves! {
    fn ownerless_station(station: u16, if_valid: u8) -> u64;
    fn vehicle_type(ctx: *mut c_void) -> u64;
    fn primary_vehicle(ctx: *mut c_void) -> u64;
    fn unit_number(ctx: *mut c_void) -> u64;
    fn shared_window(ctx: *mut c_void, first: *mut c_void) -> u64;
    fn vehicle_id(ctx: *mut c_void) -> u64;
    fn reliability(ctx: *mut c_void) -> u64;
    fn display_speed(ctx: *mut c_void) -> u64;
    fn age_years(ctx: *mut c_void) -> u64;
    fn needs_service(ctx: *mut c_void) -> u64;
    fn remaining_years(ctx: *mut c_void) -> u64;
    fn airport_tile(destination: u16) -> u64;
    fn depot_tile(destination: u16) -> u64;
    fn station_location(ctx: *mut c_void, destination: u16) -> u64;
    fn destination_tile(ctx: *mut c_void) -> u64;
    fn aircraft_flying(ctx: *mut c_void) -> u64;
    fn target_airport(ctx: *mut c_void) -> u64;
    fn waypoint_tile(destination: u16) -> u64;
    fn at_station(ctx: *mut c_void) -> u64;
    fn tile_station(ctx: *mut c_void) -> u64;
    fn ship_station_tile(destination: u16) -> u64;
    fn station_owner(destination: u16) -> u64;
    fn station_error(ctx: *mut c_void, destination: u16) -> u64;
    fn depot_owner(destination: u16) -> u64;
    fn rail_depot(destination: u16) -> u64;
    fn road_depot(destination: u16) -> u64;
    fn ship_depot(destination: u16) -> u64;
    fn waypoint_facilities(destination: u16) -> u64;
    fn waypoint_owner(destination: u16) -> u64;
    fn aircraft_range_square(ctx: *mut c_void) -> u64;
    fn create_backup(ctx: *mut c_void, user: u32) -> *mut c_void;
    fn vehicle_group(ctx: *mut c_void) -> u64;
    fn unique_backup_name(ctx: *mut c_void) -> u64;
    fn local_owner(ctx: *mut c_void) -> u64;
    fn append_station(ctx: *mut c_void, station: u16);
    fn timetable_dirty(ctx: *mut c_void);
    fn invalidate_order(ctx: *mut c_void, data: i32);
    fn delete_order_news(ctx: *mut c_void);
    fn invalidate_vehicle_list(ctx: *mut c_void);
    fn close_shared_window(ctx: *mut c_void, window: u32);
    fn invalidate_shared_window(ctx: *mut c_void, window: u32, data: i32);
    fn dirty_vehicle_windows(ctx: *mut c_void);
    fn capture_backup_metadata(ctx: *mut c_void, source: *mut c_void);
    fn clear_backup_name(ctx: *mut c_void);
    fn restore_backup_metadata(ctx: *mut c_void, backup: *mut c_void);
    fn order_news(ctx: *mut c_void, message: u32);
    fn debug_list(ctx: *mut c_void);
    fn delete_list(ctx: *mut c_void);
    fn reverse_train(ctx: *mut c_void);
    fn next_airport(ctx: *mut c_void);
    fn set_destination(ctx: *mut c_void, tile: u32);
    fn closest_depot(ctx: *mut c_void, result: *mut Closest);
    fn share_command(ctx: *mut c_void, source: *const c_void);
    fn group_command(ctx: *mut c_void, group: u16);
    fn delete_backup(ctx: *mut c_void);
    fn missing_aircraft_orders(ctx: *mut c_void);
    fn change_timetable_command(ctx: *mut c_void, index: u8, field: u8, value: u16);
}

/// `HasBit(v->GetGroundVehicleFlags(), GVF_SUPPRESS_IMPLICIT_ORDERS)`.
fn suppress_implicit(v: VehicleRef) -> bool {
    has_bit(v.ground_vehicle_flags(), GVF_SUPPRESS_IMPLICIT_ORDERS)
}
/// `AssignBit(v->GetGroundVehicleFlags(), GVF_SUPPRESS_IMPLICIT_ORDERS, suppress)`.
fn assign_suppress_implicit(v: VehicleRef, suppress: bool) {
    let mut flags = v.ground_vehicle_flags();
    assign_bit16(&mut flags, GVF_SUPPRESS_IMPLICIT_ORDERS, suppress);
    v.set_ground_vehicle_flags(flags);
}
/// `v->GetVehStatus().Any({VehState::Stopped, VehState::Crashed})`.
fn stopped_or_crashed(v: VehicleRef) -> bool {
    v.vehstatus_raw() & (VS_STOPPED | VS_CRASHED) != 0
}

impl Game {
    fn bus(self, vehicle: *mut c_void) -> bool {
        // SAFETY: The immutable maintenance table supplies the shared world services.
        let shared = unsafe { &*self.maintenance().shared };
        crate::road_stop::is_bus(shared, native_vehicle(vehicle, self.0.base))
    }
    fn shared(self) -> &'static crate::services::Services {
        // SAFETY: The immutable maintenance table names the process-wide shared table.
        unsafe { &*self.maintenance().shared }
    }
    fn maintenance(self) -> &'static maintenance::Services {
        // SAFETY: The immutable base table supplies its process-wide maintenance table.
        unsafe { &*self.0.base.maintenance }
    }
    fn invalidate_station_list(self) {
        // SAFETY: The immutable leaves table supplies the process-wide station command table.
        (unsafe { &*self.0.station_commands }.station_list_classes_invalidate)();
    }
    fn vehicle(self, v: *mut c_void) -> *mut VehicleOrders {
        // SAFETY: The entry supplies a live Vehicle shell whose orders owner remains allocated.
        unsafe { (self.0.vehicle)(v) }
    }
    fn consist(self, v: *mut c_void) -> *mut Consist {
        // SAFETY: The entry supplies a live Vehicle shell whose consist owner remains allocated.
        unsafe { (self.0.consist)(v) }
    }
    fn list(self, list: *mut c_void) -> *mut List {
        // SAFETY: The shell is a live OrderList; the callback only returns its owner pointer.
        unsafe { (self.0.list)(list) }
    }
    fn vector(self, list: *mut c_void) -> *mut Vector {
        // SAFETY: The shell is a live OrderList; the callback only returns its storage owner.
        unsafe { (self.0.vector)(list) }
    }
    fn order_access(self, vehicle: *mut VehicleOrders) -> OrderAccess {
        let shell = read_field!(vehicle, orders);
        if shell.is_null() {
            OrderAccess {
                list: ptr::null_mut(),
                vector: ptr::null_mut(),
            }
        } else {
            OrderAccess {
                list: self.list(shell),
                vector: self.vector(shell),
            }
        }
    }
    fn orders(self, v: *mut c_void) -> *mut c_void {
        read_field!(self.vehicle(v), orders)
    }
    fn next(self, v: *mut c_void) -> *mut c_void {
        read_field!(self.vehicle(v), next)
    }
    fn previous(self, v: *mut c_void) -> *mut c_void {
        read_field!(self.vehicle(v), previous)
    }
    fn first(self, v: *mut c_void) -> *mut c_void {
        let list = self.orders(v);
        if list.is_null() {
            native_vehicle(v, self.0.base).first().shell()
        } else {
            read_field!(self.list(list), first)
        }
    }
    fn size(self, list: *mut c_void) -> u8 {
        read_field!(self.vector(list), length) as u8
    }
    fn count(self, v: *mut c_void) -> u8 {
        let list = self.orders(v);
        if list.is_null() { 0 } else { self.size(list) }
    }
    fn stored_count(self, list: *mut c_void) -> usize {
        read_field!(self.vector(list), length)
    }
    fn stored_order(self, list: *mut c_void, index: usize) -> Order {
        load(slot(self.vector(list), index))
    }
    fn put_stored(self, list: *mut c_void, index: usize, order: Order) {
        store(slot(self.vector(list), index), order);
    }
    fn order(self, list: *mut c_void, index: u8) -> Option<Order> {
        if index >= self.size(list) {
            None
        } else {
            Some(load(slot(self.vector(list), usize::from(index))))
        }
    }
    fn vehicle_order(self, v: *mut c_void, index: u8) -> Option<Order> {
        let list = self.orders(v);
        if list.is_null() {
            None
        } else {
            self.order(list, index)
        }
    }
    fn put_order(self, list: *mut c_void, index: u8, order: Order) {
        store(slot(self.vector(list), usize::from(index)), order);
    }
    fn next_index(self, list: *mut c_void, index: u8) -> u8 {
        let count = self.size(list);
        if count == 0 {
            u8::MAX
        } else {
            ((u16::from(index) + 1) % u16::from(count)) as u8
        }
    }
    fn decision(self, list: *mut c_void, next: u8, hops: u32) -> u8 {
        if hops > u32::from(self.size(list)) || next >= self.size(list) {
            return u8::MAX;
        }
        let order = self.order(list, next).unwrap();
        if order.kind() == OT_CONDITIONAL {
            if order.variable() != 5 {
                return next;
            }
            return self.decision(list, order.flags, hops.wrapping_add(1));
        }
        if order.kind() == OT_GOTO_DEPOT {
            if order.action() & 1 != 0 {
                return u8::MAX;
            }
            if order.is_refit() {
                return next;
            }
        }
        if !order.can_load_unload() {
            return self.decision(list, self.next_index(list, next), hops.wrapping_add(1));
        }
        next
    }
    /// `OrderList::GetNextStoppingStation`, appending each station to `output`.
    fn stopping(
        self,
        list: *mut c_void,
        v: *mut c_void,
        first: u8,
        mut hops: u32,
        output: &mut impl FnMut(u16),
    ) {
        let mut next = first;
        if first == u8::MAX {
            next = read_field!(self.consist(v), implicit);
            if next >= self.size(list) {
                next = if self.size(list) == 0 { u8::MAX } else { 0 };
                if next == u8::MAX {
                    return;
                }
            } else {
                next = self.next_index(list, next);
            }
        }
        loop {
            hops = hops.wrapping_add(1);
            next = self.decision(list, next, hops);
            while next != u8::MAX && self.order(list, next).unwrap().kind() == OT_CONDITIONAL {
                let skip = self.decision(list, self.order(list, next).unwrap().flags, hops);
                let advance = self.decision(list, self.next_index(list, next), hops);
                if advance == u8::MAX || advance == first || skip == advance {
                    next = if skip == first { u8::MAX } else { skip };
                } else if skip == u8::MAX || skip == first {
                    next = if advance == first { u8::MAX } else { advance };
                } else {
                    self.stopping(list, v, skip, hops, output);
                    self.stopping(list, v, advance, hops, output);
                    return;
                }
                hops = hops.wrapping_add(1);
            }
            if next == u8::MAX {
                return;
            }
            let order = self.order(list, next).unwrap();
            let last = native_vehicle(v, self.0.base).last_station_visited_raw();
            if (order.kind() == OT_GOTO_STATION || order.kind() == OT_IMPLICIT)
                && order.destination == last
                && matches!(order.unload(), 1 | 2)
            {
                return;
            }
            if order.kind() == OT_GOTO_DEPOT || order.destination == last {
                continue;
            }
            output(order.destination);
            return;
        }
    }
}
/// A live `Order` read in place, such as `Vehicle::current_order`: each getter
/// loads only the fields its `order_base.h` getter reads, when it is called.
/// A view of a borrowed order lives no longer than the borrow.
#[derive(Clone, Copy)]
pub(crate) struct OrderView<'a>(*const Order, std::marker::PhantomData<&'a Order>);

// An `OrderView` names a live order of the serialized game state at every read.
impl OrderView<'_> {
    /// View the order at `order`.
    ///
    /// # Safety
    /// `order` is a live, initialized `Order` of the serialized game state for
    /// every read made through the view.
    pub(crate) unsafe fn of(order: *const Order) -> Self {
        Self(order, std::marker::PhantomData)
    }
    /// `Order::GetType`: get the type of order of this order.
    #[inline]
    pub(crate) fn kind(self) -> crate::order_type::OrderType {
        crate::order_type::OrderType(gb_u8(read_field!(self.0, kind), 0, 4))
    }
    #[inline]
    pub(crate) fn kind_raw(self) -> u8 {
        self.kind().0
    }
    /// `Order::IsType`: check whether this order is of the given type.
    #[inline]
    pub(crate) fn is_type(self, kind: crate::order_type::OrderType) -> bool {
        self.kind() == kind
    }
    /// `Order::GetNonStopType`: at which stations must we stop?
    #[inline]
    pub(crate) fn non_stop(self) -> crate::order_type::OrderNonStopFlags {
        crate::order_type::OrderNonStopFlags(gb_u8(read_field!(self.0, kind), 6, 2))
    }
    #[inline]
    pub(crate) fn non_stop_raw(self) -> u8 {
        self.non_stop().0
    }
    /// `Order::GetLoadType`: how must the consist be loaded?
    #[inline]
    pub(crate) fn load_type(self) -> crate::order_type::OrderLoadType {
        crate::order_type::OrderLoadType(gb_u8(read_field!(self.0, flags), 4, 3))
    }
    /// `Order::GetDepotOrderType`: what caused us going to the depot?
    #[inline]
    pub(crate) fn depot_type(self) -> crate::order_type::OrderDepotTypeFlags {
        crate::order_type::OrderDepotTypeFlags(gb_u8(read_field!(self.0, flags), 0, 3))
    }
    #[inline]
    pub(crate) fn depot_type_raw(self) -> u8 {
        self.depot_type().0
    }
    /// `Order::GetDepotActionType`: what are we going to do when in the depot.
    #[inline]
    pub(crate) fn depot_action_flags(self) -> crate::order_type::OrderDepotActionFlags {
        crate::order_type::OrderDepotActionFlags(gb_u8(read_field!(self.0, flags), 3, 4))
    }
    #[inline]
    pub(crate) fn depot_action_flags_raw(self) -> u8 {
        self.depot_action_flags().0
    }
    /// `Order::GetStopLocation`: where must we stop at the platform?
    #[inline]
    pub(crate) fn stop_location(self) -> crate::order_type::OrderStopLocation {
        crate::order_type::OrderStopLocation(gb_u8(read_field!(self.0, kind), 4, 2))
    }
    /// `Order::GetDestination`: gets the destination of this order.
    ///
    /// Precondition: `IsType(OT_GOTO_WAYPOINT) || IsType(OT_GOTO_DEPOT) ||
    /// IsType(OT_GOTO_STATION)`.
    #[inline]
    pub(crate) fn destination(self) -> crate::order_type::DestinationID {
        crate::order_type::DestinationID(read_field!(self.0, destination))
    }
    #[inline]
    pub(crate) fn destination_raw(self) -> u16 {
        self.destination().0
    }
    /// `Order::GetMaxSpeed`: get the maxmimum speed in km-ish/h a vehicle is
    /// allowed to reach on the way to the destination.
    #[inline]
    pub(crate) fn max_speed(self) -> u16 {
        read_field!(self.0, speed)
    }
    /// `Order::IsFullLoadOrder`: is this order a `OrderLoadType::FullLoad` or
    /// `OrderLoadType::FullLoadAny`?
    #[inline]
    pub(crate) fn is_full_load(self) -> bool {
        let kind = self.load_type();
        kind == crate::order_type::OrderLoadType::FullLoad
            || kind == crate::order_type::OrderLoadType::FullLoadAny
    }
    /// `Order::GetUnloadType`: how must the consist be unloaded?
    #[inline]
    pub(crate) fn unload_type(self) -> crate::order_type::OrderUnloadType {
        crate::order_type::OrderUnloadType(gb_u8(read_field!(self.0, flags), 0, 3))
    }
    /// `Order::IsRefit`: is this order a refit order.
    #[inline]
    pub(crate) fn is_refit(self) -> bool {
        let refit_cargo = read_field!(self.0, refit);
        refit_cargo < crate::cargo_type::NUM_CARGO || refit_cargo == CARGO_AUTO_REFIT
    }
    /// `Order::IsAutoRefit`: is this order an auto-refit order.
    #[inline]
    pub(crate) fn is_auto_refit(self) -> bool {
        read_field!(self.0, refit) == CARGO_AUTO_REFIT
    }
    /// `Order::GetRefitCargo`: get the cargo to refit to.
    #[inline]
    pub(crate) fn refit_cargo(self) -> CargoType {
        CargoType(read_field!(self.0, refit))
    }
    /// `Order::GetConditionSkipToOrder`: get the order to skip to.
    #[inline]
    pub(crate) fn condition_skip_to_order(self) -> VehicleOrderID {
        VehicleOrderID(read_field!(self.0, flags))
    }
    /// `Order::IsWaitTimetabled`: does this order have an explicit wait time set?
    #[inline]
    pub(crate) fn is_wait_timetabled(self) -> bool {
        if self.is_type(OrderType::OT_CONDITIONAL) {
            read_field!(self.0, wait) > 0
        } else {
            has_bit(read_field!(self.0, flags), 3)
        }
    }
    /// `Order::GetTimetabledWait`: get the time in ticks a vehicle should wait at the
    /// destination or 0 if it's not timetabled.
    #[inline]
    pub(crate) fn timetabled_wait(self) -> u16 {
        if self.is_wait_timetabled() {
            read_field!(self.0, wait)
        } else {
            0
        }
    }
    /// `Order::CanLeaveWithCargo`: a vehicle can leave the current station with cargo if it
    /// can load cargo here, or if it could have loaded it before and does not unload all of it.
    pub(crate) fn can_leave_with_cargo(self, has_cargo: bool) -> bool {
        use crate::order_type::{OrderLoadType, OrderUnloadType};
        self.load_type() != OrderLoadType::NoLoad
            || (has_cargo
                && self.unload_type() != OrderUnloadType::Unload
                && self.unload_type() != OrderUnloadType::Transfer)
    }
    /// A copy of the whole order, where the original copies it.
    pub(crate) fn copied(self) -> Order {
        // SAFETY: The view names a live order; the copy creates no reference.
        unsafe { self.0.read() }
    }
}

impl Order {
    /// This order, read through `OrderView` for as long as it is borrowed.
    fn view(&self) -> OrderView<'_> {
        // SAFETY: A reference names a live, initialized order.
        unsafe { OrderView::of(ptr::from_ref(self)) }
    }
    /// `Order::GetType`.
    pub(crate) fn kind(&self) -> u8 {
        self.view().kind_raw()
    }
    /// `Order::GetNonStopType`.
    pub(crate) fn non_stop(self) -> u8 {
        self.view().non_stop_raw()
    }
    /// `Order::GetLoadType`.
    pub(crate) fn load(self) -> u8 {
        self.view().load_type().0
    }
    /// `Order::GetUnloadType`.
    pub(crate) fn unload(self) -> u8 {
        self.view().unload_type().0
    }
    /// `Order::GetDepotActionType`.
    pub(crate) fn action(&self) -> u8 {
        self.view().depot_action_flags_raw()
    }
    /// `Order::GetDepotOrderType`.
    pub(crate) fn depot_type(&self) -> u8 {
        self.view().depot_type_raw()
    }
    fn variable(self) -> u8 {
        (self.destination >> 11) as u8
    }
    fn comparator(self) -> u8 {
        self.kind >> 5
    }
    fn value(self) -> u16 {
        self.destination & 2047
    }
    /// `Order::IsRefit`.
    pub(crate) fn is_refit(self) -> bool {
        self.view().is_refit()
    }
    /// `Order::GetMaxSpeed`.
    pub(crate) fn max_speed(self) -> u16 {
        self.view().max_speed()
    }
    fn wait_timetabled(self) -> bool {
        self.view().is_wait_timetabled()
    }
    fn travel_timetabled(self) -> bool {
        if self.kind() == OT_CONDITIONAL {
            self.travel > 0
        } else {
            self.flags & 128 != 0
        }
    }
    pub(crate) fn timetabled_wait(self) -> u16 {
        self.view().timetabled_wait()
    }
    fn timetabled_travel(self) -> u16 {
        if self.travel_timetabled() {
            self.travel
        } else {
            0
        }
    }
    fn can_load_unload(self) -> bool {
        if (self.kind() != OT_GOTO_STATION && self.kind() != OT_IMPLICIT)
            || self.non_stop() & 2 != 0
        {
            return false;
        }
        !(self.load() == 4 && self.unload() == 4)
    }
    fn complete(self) -> bool {
        if !self.travel_timetabled() && self.kind() != OT_CONDITIONAL {
            return false;
        }
        !(!self.wait_timetabled() && self.kind() == OT_GOTO_STATION && self.non_stop() & 2 == 0)
    }
    fn equals(&self, other: &Self) -> bool {
        if self.kind() == OT_GOTO_DEPOT
            && self.kind == other.kind
            && (self.action() | other.action()) & 2 != 0
        {
            return self.depot_type() == other.depot_type()
                && self.action() & !2 == other.action() & !2;
        }
        self.kind == other.kind
            && self.flags == other.flags
            && self.destination == other.destination
    }
    fn set_wait_timetabled(&mut self, b: bool) {
        if self.kind() != OT_CONDITIONAL {
            self.flags = (self.flags & !8) | if b { 8 } else { 0 };
        }
    }
    fn set_travel_timetabled(&mut self, b: bool) {
        if self.kind() != OT_CONDITIONAL {
            self.flags = (self.flags & !128) | if b { 128 } else { 0 };
        }
    }
}

/// List initialization owns traversal and bookkeeping of canonical storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_init(
    list: *mut c_void,
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    let state = g.list(list);
    {
        write_field!(state, first, v);
        write_field!(state, manual, 0);
        write_field!(state, vehicles, 1);
        write_field!(state, timetable, 0);
        for index in 0..g.stored_count(list) {
            let order = g.stored_order(list, index);
            if order.kind() != OT_IMPLICIT {
                write_field!(state, manual, read_field!(state, manual).wrapping_add(1));
            }
            write_field!(
                state,
                total,
                read_field!(state, total)
                    .wrapping_add(i32::from(order.wait) + i32::from(order.travel))
            );
        }
        recalculate(g, list);
        let mut u = g.previous(v);
        while !u.is_null() {
            write_field!(
                state,
                vehicles,
                read_field!(state, vehicles).wrapping_add(1)
            );
            write_field!(state, first, u);
            u = g.previous(u);
        }
        u = g.next(v);
        while !u.is_null() {
            write_field!(
                state,
                vehicles,
                read_field!(state, vehicles).wrapping_add(1)
            );
            u = g.next(u);
        }
    };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_recalculate(
    list: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    recalculate(g, list);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_free(
    list: *mut c_void,
    keep: u8,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    let state = g.list(list);
    {
        for index in 0..g.stored_count(list) {
            let order = g.stored_order(list, index);
            if matches!(order.kind(), OT_GOTO_STATION | OT_GOTO_WAYPOINT)
                && g.ownerless_station(order.destination, 1) != 0
            {
                g.invalidate_station_list();
                break;
            }
        }
        if keep == 0 {
            return 1;
        }
        reenter!(openttd_rust_order_vector_resize(g.vector(list), 0));
        write_field!(state, manual, 0);
        write_field!(state, timetable, 0);
    };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_stopping(
    list: *mut c_void,
    v: *mut c_void,
    first: u8,
    hops: u32,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    g.stopping(list, v, first, hops, &mut |station| {
        g.append_station(output, station);
    });
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_insert(
    list: *mut c_void,
    index: u32,
    input: *const Order,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    let state = g.list(list);
    {
        let order = load(input);
        reenter!(openttd_rust_order_vector_insert(
            g.vector(list),
            index as usize,
            input
        ));
        if order.kind() != OT_IMPLICIT {
            write_field!(state, manual, read_field!(state, manual).wrapping_add(1));
        }
        write_field!(
            state,
            timetable,
            read_field!(state, timetable).wrapping_add(
                i32::from(order.timetabled_wait()) + i32::from(order.timetabled_travel()),
            )
        );
        write_field!(
            state,
            total,
            read_field!(state, total).wrapping_add(i32::from(order.wait) + i32::from(order.travel))
        );
        if matches!(order.kind(), OT_GOTO_STATION | OT_GOTO_WAYPOINT)
            && g.ownerless_station(order.destination, 0) != 0
        {
            g.invalidate_station_list();
        }
    };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_complete(
    list: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    {
        u32::from((0..g.stored_count(list)).all(|i| {
            let order = g.stored_order(list, i);
            order.kind() == OT_IMPLICIT || order.complete()
        }))
    }
}

fn recalculate(g: Game, list: *mut c_void) {
    let state = g.list(list);
    write_field!(state, timetable, 0);
    for i in 0..g.stored_count(list) {
        let order = g.stored_order(list, i);
        write_field!(
            state,
            timetable,
            read_field!(state, timetable).wrapping_add(
                i32::from(order.timetabled_wait()) + i32::from(order.timetabled_travel()),
            )
        );
    }
}

impl Game {
    /// `OrderList::DeleteOrderAt`: remove an order from the order list and delete it.
    /// `index` is the position of the order which is to be deleted.
    fn delete_order_at(self, list: *mut c_void, index: VehicleOrderID) {
        let index = usize::from(index.0);
        if index >= self.stored_count(list) {
            return;
        }
        let state = self.list(list);
        let order = self.stored_order(list, index);
        if order.kind() != OT_IMPLICIT {
            write_field!(state, manual, read_field!(state, manual).wrapping_sub(1));
        }
        write_field!(
            state,
            timetable,
            read_field!(state, timetable).wrapping_sub(
                i32::from(order.timetabled_wait()) + i32::from(order.timetabled_travel()),
            )
        );
        write_field!(
            state,
            total,
            read_field!(state, total).wrapping_sub(i32::from(order.wait) + i32::from(order.travel))
        );
        reenter!(openttd_rust_order_vector_erase(self.vector(list), index));
    }
    fn current(self, v: *mut c_void) -> Order {
        load(current_order(self.vehicle(v)))
    }
    fn put_current(self, v: *mut c_void, o: Order) {
        store(current_order(self.vehicle(v)), o);
    }
    fn complete(self, list: *mut c_void) -> bool {
        (0..self.stored_count(list)).all(|i| {
            let o = self.stored_order(list, i);
            o.kind() == OT_IMPLICIT || o.complete()
        })
    }
    fn duration(self, list: *mut c_void) -> i32 {
        if self.complete(list) {
            read_field!(self.list(list), timetable)
        } else {
            -1
        }
    }
    fn reset(self, v: *mut c_void) {
        reenter!(openttd_rust_consist_reset(self.consist(v)));
    }
    fn dirty(self, v: *mut c_void) {
        self.timetable_dirty(v);
    }
    fn change_timetable(self, v: *mut c_void, index: u8, val: u16, field: u8, timetabled: bool) {
        let list = self.orders(v);
        let mut order = self.order(list, index).unwrap();
        let (total_delta, timetable_delta) = match field {
            0 => {
                let deltas = (
                    i32::from(val) - i32::from(order.wait),
                    if timetabled { i32::from(val) } else { 0 }
                        - i32::from(order.timetabled_wait()),
                );
                order.wait = val;
                order.set_wait_timetabled(timetabled);
                deltas
            }
            1 => {
                let deltas = (
                    i32::from(val) - i32::from(order.travel),
                    if timetabled { i32::from(val) } else { 0 }
                        - i32::from(order.timetabled_travel()),
                );
                order.travel = val;
                order.set_travel_timetabled(timetabled);
                deltas
            }
            2 => {
                order.speed = val;
                (0, 0)
            }
            _ => unreachable!(),
        };
        self.put_order(list, index, order);
        let owner = self.list(list);
        write_field!(
            owner,
            total,
            read_field!(owner, total).wrapping_add(total_delta)
        );
        write_field!(
            owner,
            timetable,
            read_field!(owner, timetable).wrapping_add(timetable_delta)
        );
        let mut u = self.first(v);
        while !u.is_null() {
            let state = self.consist(u);
            let mut current = self.current(u);
            if read_field!(state, real) == index && current.equals(&order) {
                match field {
                    0 => {
                        current.wait = val;
                        current.set_wait_timetabled(timetabled);
                    }
                    1 => {
                        current.travel = val;
                        current.set_travel_timetabled(timetabled);
                    }
                    2 => current.speed = val,
                    _ => unreachable!(),
                }
                self.put_current(u, current);
            }
            self.dirty(u);
            u = self.next(u);
        }
    }
    fn update_timetable(self, v: *mut c_void, travelling: bool) {
        let state = self.consist(v);
        let time = read_field!(state, time);
        write_field!(state, time, 0);
        if self.current(v).kind() == OT_IMPLICIT {
            return;
        }
        let real = read_field!(state, real);
        if real >= self.count(v) {
            return;
        }
        let mut order = self.vehicle_order(v, real).unwrap();
        let mut first = 0u8;
        let list = self.orders(v);
        for i in 0..self.stored_count(list) {
            if self.stored_order(list, i).kind() != OT_IMPLICIT {
                break;
            }
            first = first.wrapping_add(1);
        }
        let mut just_started = false;
        if real == first && travelling {
            just_started = read_field!(state, flags) & (1 << 3) == 0;
            if read_field!(state, start) != 0 {
                write_field!(
                    state,
                    lateness,
                    (self.0.tick_counter)().wrapping_sub(read_field!(state, start)) as i32
                );
                write_field!(state, start, 0);
            }
            write_field!(state, flags, read_field!(state, flags) | 1 << 3);
            self.dirty(v);
        }
        if read_field!(state, flags) & (1 << 3) == 0 {
            return;
        }
        let autofill = read_field!(state, flags) & (1 << 4) != 0;
        let remeasure =
            !order.wait_timetabled() || (autofill && read_field!(state, flags) & (1 << 5) == 0);
        if travelling && remeasure {
            let mut current = self.current(v);
            current.wait = 0;
            self.put_current(v, current);
        }
        if just_started {
            return;
        }
        if order.kind() != OT_CONDITIONAL
            && (travelling || time > i32::from(order.wait) || remeasure)
        {
            let seconds = (self.0.ticks_per_second)() as i32;
            let value =
                (time.max(1).wrapping_add(seconds - 1) / seconds).wrapping_mul(seconds) as u16;
            if travelling && (autofill || !order.travel_timetabled()) {
                self.change_timetable(v, real, value, 1, autofill);
            } else if !travelling && (autofill || !order.wait_timetabled()) {
                self.change_timetable(v, real, value, 0, autofill);
            }
            order = self.vehicle_order(v, real).unwrap();
        }
        if real == first && travelling {
            write_field!(
                state,
                flags,
                read_field!(state, flags) & !((1 << 4) | (1 << 5))
            );
        }
        if autofill {
            return;
        }
        let timetable = i32::from(if travelling {
            order.timetabled_travel()
        } else {
            order.timetabled_wait()
        });
        if timetable == 0 && (travelling || read_field!(state, lateness) >= 0) {
            return;
        }
        write_field!(
            state,
            lateness,
            read_field!(state, lateness).wrapping_sub(timetable.wrapping_sub(time))
        );
        if read_field!(state, lateness) > timetable {
            let cycle = self.duration(self.orders(v));
            if cycle != -1 && read_field!(state, lateness) > cycle {
                write_field!(state, lateness, read_field!(state, lateness) % cycle);
            }
        }
        let mut u = self.first(v);
        while !u.is_null() {
            self.dirty(u);
            u = self.next(u);
        }
    }
    fn owner_check(self, v: *mut c_void, output: *mut c_void) -> bool {
        let owner = (native_vehicle(v, self.0.base)).owner_raw();
        self.check_owner(u64::from(owner), output)
    }
    /// The entry's native `CommandCost`, written through the shared cost services.
    fn cost(self, output: *mut c_void) -> crate::services::Cost<'static> {
        // SAFETY: The entry supplies immutable owning tables that outlive it, and its live,
        // writable native cost slot.
        let costs = unsafe { &*(*self.0.station_commands).cost };
        crate::services::Cost(output.cast(), costs)
    }
    /// `return CMD_ERROR;`
    fn cmd_error(self, output: *mut c_void) -> u32 {
        crate::command_func::cmd_error(self.cost(output));
        0
    }
    /// `return CommandCost(msg);`
    fn command_cost(self, output: *mut c_void, msg: StringID) -> u32 {
        self.cost(output).error(msg.0);
        0
    }
    /// `return CommandCost(msg, extra_msg);`
    fn command_cost_extra(self, output: *mut c_void, msg: StringID, extra_msg: StringID) -> u32 {
        self.cost(output).error_extra(msg, extra_msg);
        0
    }
    fn primary(self, v: *mut c_void) -> bool {
        !v.is_null() && self.primary_vehicle(v) != 0
    }
    fn timetable_sort(self, a: *mut c_void, b: *mut c_void) -> std::cmp::Ordering {
        let sa = self.consist(a);
        let sb = self.consist(b);
        let mut ai = read_field!(sa, real);
        let mut bi = read_field!(sb, real);
        let raw = i32::from(bi) - i32::from(ai);
        let ao = self.current(a);
        let bo = self.current(b);
        if ao.kind() != OT_LOADING || ao.non_stop() == 0 {
            ai = ai.wrapping_sub(1);
        }
        if bo.kind() != OT_LOADING || bo.non_stop() == 0 {
            bi = bi.wrapping_sub(1);
        }
        let index = i32::from(bi) - i32::from(ai);
        if index != 0 {
            return index.cmp(&0);
        }
        if raw != 0 {
            return raw.cmp(&0);
        }
        let time = read_field!(sb, time).wrapping_sub(read_field!(sa, time));
        if time != 0 {
            return time.cmp(&0);
        }
        self.unit_number(b).cmp(&self.unit_number(a))
    }
}

/// Timetable command validation and execution. Ownership is a shared company
/// service. Each error return writes the `CommandCost` the original returns there.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_timetable_change(
    v: *mut c_void,
    execute: u8,
    index: u8,
    field: u8,
    value: u16,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    if !g.primary(v) {
        return g.cmd_error(output);
    }
    if !g.owner_check(v, output) {
        return 0;
    }
    {
        let Some(order) = g.vehicle_order(v, index) else {
            return g.cmd_error(output);
        };
        if order.kind() == OT_IMPLICIT || field >= 3 {
            return g.cmd_error(output);
        }
        let mut wait = order.wait;
        let mut travel = order.travel;
        let mut speed = order.speed;
        match field {
            0 => wait = value,
            1 => travel = value,
            2 => speed = if value == 0 { u16::MAX } else { value },
            _ => unreachable!(),
        }
        if wait != order.wait {
            match order.kind() {
                OT_GOTO_STATION => {
                    if order.non_stop() & 2 != 0 {
                        return g.command_cost(output, STR_ERROR_TIMETABLE_NOT_STOPPING_HERE);
                    }
                }
                OT_CONDITIONAL => {}
                _ => return g.command_cost(output, STR_ERROR_TIMETABLE_ONLY_WAIT_AT_STATIONS),
            }
        }
        if travel != order.travel && order.kind() == OT_CONDITIONAL {
            return g.cmd_error(output);
        }
        if speed != order.speed
            && (order.kind() == OT_CONDITIONAL || g.vehicle_type(v) == u64::from(VEH_AIRCRAFT))
        {
            return g.cmd_error(output);
        }
        if execute != 0 {
            match field {
                0 => {
                    if wait != order.wait || (wait > 0 && !order.wait_timetabled()) {
                        g.change_timetable(v, index, wait, 0, wait > 0);
                    }
                }
                1 => {
                    if travel != order.travel || (travel > 0 && !order.travel_timetabled()) {
                        g.change_timetable(v, index, travel, 1, travel > 0);
                    }
                }
                2 => {
                    if speed != order.speed {
                        g.change_timetable(v, index, speed, 2, speed != u16::MAX);
                    }
                }
                _ => unreachable!(),
            }
            let mut u = g.first(v);
            while !u.is_null() {
                g.reset(u);
                u = g.next(u);
            }
        }
    };
    1
}

pub unsafe extern "C" fn openttd_rust_timetable_validate_bulk(
    v: *mut c_void,
    field: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    if !g.primary(v) {
        return g.cmd_error(output);
    }
    if !g.owner_check(v, output) {
        return 0;
    }
    if field >= 3 || g.count(v) == 0 {
        return g.cmd_error(output);
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_timetable_on_time(
    v: *mut c_void,
    execute: u8,
    apply_group: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    if !g.primary(v) {
        return g.cmd_error(output);
    }
    if g.orders(v).is_null() {
        return g.cmd_error(output);
    }
    let state = g.consist(v);
    if apply_group == 0 && read_field!(state, flags) & (1 << 3) == 0 {
        return g.command_cost(output, STR_ERROR_TIMETABLE_NOT_STARTED);
    }
    if !g.owner_check(v, output) {
        return 0;
    }
    if execute != 0 {
        if apply_group != 0 {
            let mut late = 0;
            let mut u = g.first(v);
            while !u.is_null() {
                if read_field!(state, flags) & (1 << 3) != 0 {
                    let us = g.consist(u);
                    late = late.max(read_field!(us, lateness));
                    g.reset(u);
                }
                u = g.next(u);
            }
            if late > 0 {
                u = g.first(v);
                while !u.is_null() {
                    if read_field!(state, flags) & (1 << 3) != 0 {
                        let us = g.consist(u);
                        write_field!(us, lateness, read_field!(us, lateness).wrapping_sub(late));
                        g.dirty(u);
                    }
                    u = g.next(u);
                }
            }
        } else {
            write_field!(state, lateness, 0);
            g.reset(v);
            g.dirty(v);
        }
    }
    1
}

/// A command call's context: `DoCommandFlag::Execute` and the `CommandCost` output.
struct CommandCall {
    execute: bool,
    output: *mut c_void,
}

/// `CmdSetTimetableStart`'s arguments, in the original's order.
struct TimetableStart {
    veh: *mut c_void,
    timetable_all: bool,
    start_tick: u64,
}

/// `CmdSetTimetableStart`: set the start date of the timetable.
fn set_timetable_start(g: Game, call: &CommandCall, args: &TimetableStart) -> u32 {
    if !g.primary(args.veh) {
        return g.cmd_error(call.output);
    }
    if g.orders(args.veh).is_null() {
        return g.cmd_error(call.output);
    }
    if !g.owner_check(args.veh, call.output) {
        return 0;
    }
    {
        let list = g.orders(args.veh);
        let total = g.duration(list);
        let date = crate::timer_game_common::economy_date().0;
        let start_date = date.wrapping_add(
            (args.start_tick.wrapping_sub((g.0.tick_counter)()) as i32)
                .wrapping_add(i32::from(crate::timer_game_common::economy_fraction()))
                / DAY_TICKS,
        );
        let max_date = (g.0.maximum_date)() as i32;
        // Don't let a timetable start at an invalid date.
        if start_date < 0 || start_date > max_date {
            return g.cmd_error(call.output);
        }
        // Don't let a timetable start more than 15 years into the future...
        if start_date.wrapping_sub(date)
            > crate::timer_game_common::date_at_start_of_year(
                crate::timetable::MAX_TIMETABLE_START_YEARS,
            )
        {
            return g.cmd_error(call.output);
        }
        // ...or 1 year in the past.
        if date.wrapping_sub(start_date) > DAYS_IN_LEAP_YEAR {
            return g.cmd_error(call.output);
        }
        // If trying to distribute start dates over a shared order group, we need to know the total duration.
        if args.timetable_all && !g.complete(list) {
            return g.command_cost(call.output, STR_ERROR_TIMETABLE_INCOMPLETE);
        }
        // Don't allow invalid start dates for other vehicles in the shared order group.
        if args.timetable_all && start_date.wrapping_add(total / DAY_TICKS) > max_date {
            return g.cmd_error(call.output);
        }
        if call.execute {
            let mut vehicles = Vec::new();
            if args.timetable_all {
                let mut u = read_field!(g.list(list), first);
                while !u.is_null() {
                    vehicles.push(u);
                    u = g.next(u);
                }
            } else {
                vehicles.push(args.veh);
            }
            if vehicles.len() >= 2 {
                vehicles.sort_unstable_by(|all, start| g.timetable_sort(*all, *start));
            }
            let count = vehicles.len() as i32;
            for (index, u) in vehicles.into_iter().enumerate() {
                let us = g.consist(u);
                write_field!(us, lateness, 0);
                write_field!(
                    us,
                    flags,
                    read_field!(us, flags) & !crate::base_consist::VF_TIMETABLE_STARTED
                );
                // Do multiplication, then division to reduce rounding errors.
                write_field!(
                    us,
                    start,
                    args.start_tick
                        .wrapping_add(((index as i32).wrapping_mul(total) / count) as u64)
                );
                // Unbunching data is no longer valid.
                // The original resets v, not w; the quirk is preserved.
                g.reset(args.veh);
                g.dirty(u);
            }
        }
    };
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_timetable_start(
    v: *mut c_void,
    execute: u8,
    all: u8,
    start: u64,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let call = CommandCall {
        execute: execute != 0,
        output,
    };
    let args = TimetableStart {
        veh: v,
        timetable_all: all != 0,
        start_tick: start,
    };
    set_timetable_start(Game(leaves), &call, &args)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_timetable_autofill(
    v: *mut c_void,
    execute: u8,
    autofill: u8,
    preserve_wait: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    if !g.primary(v) {
        return g.cmd_error(output);
    }
    if g.orders(v).is_null() {
        return g.cmd_error(output);
    }
    let state = g.consist(v);
    if !g.owner_check(v, output) {
        return 0;
    }
    if execute != 0 {
        if autofill != 0 {
            write_field!(state, flags, read_field!(state, flags) | 1 << 4);
            write_field!(state, flags, read_field!(state, flags) & !(1 << 3));
            if preserve_wait != 0 {
                write_field!(state, flags, read_field!(state, flags) | 1 << 5);
            }
            write_field!(state, start, 0);
            write_field!(state, lateness, 0);
        } else {
            write_field!(
                state,
                flags,
                read_field!(state, flags) & !((1 << 4) | (1 << 5))
            );
        }
        let mut u = g.first(v);
        while !u.is_null() {
            if u != v {
                let us = g.consist(u);
                write_field!(us, flags, read_field!(us, flags) & !((1 << 4) | (1 << 5)));
            }
            g.dirty(u);
            u = g.next(u);
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_timetable_update(
    v: *mut c_void,
    travelling: u8,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    g.update_timetable(v, travelling != 0);
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orders_start_tick(
    date: i32,
    current_date: i32,
    fract: u16,
    counter: u64,
) -> u64 {
    counter.wrapping_add(
        date.wrapping_sub(current_date)
            .wrapping_mul(DAY_TICKS)
            .wrapping_sub(i32::from(fract)) as u64,
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orders_start_date(
    start: u64,
    date: i32,
    fract: u16,
    counter: u64,
) -> i32 {
    date.wrapping_add(
        (start.wrapping_sub(counter) as i32).wrapping_add(i32::from(fract)) / DAY_TICKS,
    )
}

impl Game {
    fn manual(self, v: *mut c_void) -> u8 {
        let list = self.orders(v);
        if list.is_null() {
            0
        } else {
            read_field!(self.list(list), manual)
        }
    }
    fn update_real(self, v: *mut c_void) {
        let vehicle = self.vehicle(v);
        let state = self.consist(v);
        self.order_access(vehicle).update_real(state);
    }
    fn skip_real(self, v: *mut c_void) {
        let state = self.consist(v);
        if self.manual(v) > 0 {
            loop {
                write_field!(state, real, read_field!(state, real).wrapping_add(1));
                if read_field!(state, real) >= self.count(v) {
                    write_field!(state, real, 0);
                }
                if self
                    .vehicle_order(v, read_field!(state, real))
                    .unwrap()
                    .kind()
                    != 8
                {
                    break;
                }
            }
        } else {
            write_field!(state, real, 0);
        }
    }
    fn increment_implicit(self, v: *mut c_void) {
        let state = self.consist(v);
        if read_field!(state, implicit) == read_field!(state, real) {
            self.skip_real(v);
        }
        loop {
            write_field!(
                state,
                implicit,
                read_field!(state, implicit).wrapping_add(1)
            );
            if read_field!(state, implicit) >= self.count(v) {
                write_field!(state, implicit, 0);
            }
            if read_field!(state, implicit) == read_field!(state, real)
                || self
                    .vehicle_order(v, read_field!(state, implicit))
                    .unwrap()
                    .kind()
                    == 8
            {
                break;
            }
        }
        self.invalidate_order(v, 0);
    }
    fn increment_real(self, v: *mut c_void) {
        let state = self.consist(v);
        if read_field!(state, implicit) == read_field!(state, real) {
            self.increment_implicit(v);
        } else {
            self.skip_real(v);
            self.invalidate_order(v, 0);
        }
    }
    fn any_order(self, v: *mut c_void, predicate: impl Fn(Order) -> bool) -> bool {
        let list = self.orders(v);
        !list.is_null()
            && (0..self.stored_count(list)).any(|i| predicate(self.stored_order(list, i)))
    }
    fn previous_unbunching(self, v: *mut c_void) -> bool {
        self.previous_unbunching_of(self.orders(v), self.consist(v))
    }
    /// `PreviousOrderIsUnbunching` over a vehicle's order list shell (or null) and consist.
    fn previous_unbunching_of(self, list: *mut c_void, consist: *mut Consist) -> bool {
        !list.is_null()
            && previous_order_is_unbunching(
                &OrderAccess {
                    list: self.list(list),
                    vector: self.vector(list),
                },
                consist,
            )
    }
    fn leave_unbunching(self, v: *mut c_void) {
        if !self.previous_unbunching(v) {
            return;
        }
        let state = self.consist(v);
        let tick = (self.0.tick_counter)();
        write_field!(state, last_departure, tick);
        write_field!(state, lateness, 0);
        self.dirty(v);
        let mut count = 0i32;
        let mut time = 0i32;
        let mut u = self.first(v);
        while !u.is_null() {
            if !stopped_or_crashed(native_vehicle(u, self.0.base)) {
                count = count.wrapping_add(1);
                time = time.wrapping_add(read_field!(self.consist(u), round_trip));
            }
            u = self.next(u);
        }
        count = count.max(1);
        let separation = (time / count / count).max(1);
        let departure = tick.wrapping_add(separation as u64);
        u = self.first(v);
        while !u.is_null() {
            let record = native_vehicle(u, self.0.base);
            if !stopped_or_crashed(record) {
                write_field!(self.consist(u), next_departure, departure);
                crate::window_func::set_window_dirty(
                    crate::window_type::WindowClass::WC_VEHICLE_VIEW,
                    crate::window_type::WindowNumber::from(record.index(self.0.base.newgrf())),
                );
            }
            u = self.next(u);
        }
    }
    fn measure_unbunching(self, v: *mut c_void) {
        let state = self.consist(v);
        if self.current(v).action() & 4 == 0 || read_field!(state, last_departure) == 0 {
            return;
        }
        let measured =
            (self.0.tick_counter)().wrapping_sub(read_field!(state, last_departure)) as i32;
        let previous = read_field!(state, round_trip);
        // Original int32 product occurs before ClampTo<int32>; preserve native wrap.
        let result = if previous == 0 {
            measured
        } else {
            {
                let lower = previous / 2;
                let upper = previous.wrapping_mul(2);
                (self.0.assert_departure_range)((lower as u64) as i32, (upper as u64) as i32);
                if measured <= lower {
                    lower
                } else if measured >= upper {
                    upper
                } else {
                    measured
                }
            }
        };
        write_field!(state, round_trip, result);
    }
}
/// Scalar vehicle policy, without any persistent state reference or world borrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_real(v: *mut c_void, leaves: &'static Leaves) -> u32 {
    let g = Game(leaves);
    g.update_real(v);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_skip_real(v: *mut c_void, leaves: &'static Leaves) -> u32 {
    let g = Game(leaves);
    g.skip_real(v);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_increment_implicit(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    g.increment_implicit(v);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_increment_real(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    g.increment_real(v);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_leave_unbunching(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    g.leave_unbunching(v);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_wait_unbunching(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    u32::from(VisitOrders::new(leaves).is_waiting_for_unbunching(native_vehicle(v, leaves.base)))
}

/// `Order::Free`: 'Free' the order.
/// @note ONLY use on "`current_order`" vehicle orders!
pub(crate) fn free(v: VehicleRef) {
    // SAFETY: The live vehicle's current order.
    unsafe { free_order(v.current_order_pointer()) };
}
/// `Order::Free` on `p`.
///
/// # Safety
/// `p` is a live, properly constructed vehicle's `current_order` of the serialized game state.
unsafe fn free_order(p: *mut Order) {
    write_field!(p, kind, OT_NOTHING);
    write_field!(p, flags, 0);
    write_field!(p, destination, 0);
}
/// All fields belong to a live, properly constructed C++ Order, accessed raw.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_free(p: *mut Order) -> u32 {
    // SAFETY: As `free_order`; the native caller keeps the current-order note.
    unsafe { free_order(p) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_station(p: *mut Order, destination: u16) -> u32 {
    let mut order = load(p);
    {
        order.kind = OT_GOTO_STATION;
        order.flags = 0;
        order.destination = destination;
    };
    store(p, order);
    0
}

/// `Order::MakeGoToDepot`: makes this order a Go To Depot order.
///
/// # Safety
/// `p` is a live, properly constructed order of the serialized game state.
pub(crate) unsafe fn make_go_to_depot(
    p: *mut Order,
    destination: crate::order_type::DestinationID,
    order: crate::order_type::OrderDepotTypeFlags,
    non_stop_type: crate::order_type::OrderNonStopFlags,
    action: crate::order_type::OrderDepotActionFlags,
    cargo: crate::cargo_type::CargoType,
) {
    write_field!(p, kind, OT_GOTO_DEPOT);
    set_depot_order_type(p, order);
    set_depot_action_type(p, action);
    set_non_stop_type(p, non_stop_type);
    write_field!(p, destination, destination.0);
    set_refit(p, cargo);
}
/// `Order::SetDepotOrderType`: set the cause to go to the depot.
fn set_depot_order_type(p: *mut Order, depot_order_type: crate::order_type::OrderDepotTypeFlags) {
    let mut flags = u32::from(read_field!(p, flags));
    crate::bitmath_func::sb(&mut flags, 0, 3, u32::from(depot_order_type.0));
    write_field!(p, flags, flags as u8);
}
/// `Order::SetDepotActionType`: set what we are going to do in the depot.
fn set_depot_action_type(
    p: *mut Order,
    depot_service_type: crate::order_type::OrderDepotActionFlags,
) {
    let mut flags = u32::from(read_field!(p, flags));
    crate::bitmath_func::sb(&mut flags, 3, 4, u32::from(depot_service_type.0));
    write_field!(p, flags, flags as u8);
}
/// `Order::SetNonStopType`: set whether we must stop at stations or not.
fn set_non_stop_type(p: *mut Order, non_stop_type: crate::order_type::OrderNonStopFlags) {
    let mut kind = u32::from(read_field!(p, kind));
    crate::bitmath_func::sb(&mut kind, 6, 2, u32::from(non_stop_type.0));
    write_field!(p, kind, kind as u8);
}
/// `Order::SetRefit`: make this depot/station order also a refit order.
fn set_refit(p: *mut Order, cargo: crate::cargo_type::CargoType) {
    write_field!(p, refit, cargo.0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_depot(
    p: *mut Order,
    destination: u16,
    depot_type: u8,
    nonstop: u8,
    action: u8,
    cargo: u8,
) -> u32 {
    // SAFETY: As `make_go_to_depot`.
    unsafe {
        make_go_to_depot(
            p,
            crate::order_type::DestinationID(destination),
            crate::order_type::OrderDepotTypeFlags(depot_type),
            crate::order_type::OrderNonStopFlags(nonstop),
            crate::order_type::OrderDepotActionFlags(action),
            crate::cargo_type::CargoType(cargo),
        );
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_waypoint(p: *mut Order, destination: u16) -> u32 {
    let mut order = load(p);
    {
        order.kind = OT_GOTO_WAYPOINT;
        order.flags = 0;
        order.destination = destination;
    };
    store(p, order);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_loading(p: *mut Order, ordered: u8) -> u32 {
    let mut order = load(p);
    {
        order.kind = OT_LOADING;
        if ordered == 0 {
            order.flags = 0;
        }
    };
    store(p, order);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_leave(p: *mut Order) -> u32 {
    let mut order = load(p);
    {
        order.kind = OT_LEAVESTATION;
        order.flags = 0;
    };
    store(p, order);
    0
}

/// `Order::MakeDummy`: makes this order a Dummy order.
///
/// # Safety
/// `p` is a live, properly constructed order of the serialized game state.
pub(crate) unsafe fn make_dummy(p: *mut Order) {
    write_field!(p, kind, OT_DUMMY);
    write_field!(p, flags, 0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_dummy(p: *mut Order) -> u32 {
    // SAFETY: As `make_dummy`.
    unsafe { make_dummy(p) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_conditional(p: *mut Order, skip: u8) -> u32 {
    let mut order = load(p);
    {
        order.kind = OT_CONDITIONAL;
        order.flags = skip;
        order.destination = 0;
    };
    store(p, order);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_implicit(p: *mut Order, destination: u16) -> u32 {
    let mut order = load(p);
    {
        order.kind = OT_IMPLICIT;
        order.destination = destination;
    };
    store(p, order);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_refit(p: *mut Order, cargo: u8) -> u32 {
    let mut order = load(p);
    order.refit = cargo;
    store(p, order);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_equals(p: *mut Order, q: *const Order) -> u32 {
    // SAFETY: Both orders are live and equality calls no external service or
    // mutation; these shared borrows end before returning to native code.
    u32::from(unsafe { (&*p).equals(&*q) })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_assign(p: *mut Order, q: *const Order) -> u32 {
    let order = load(q);
    store(p, order);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_convert(p: *mut Order) -> u32 {
    let order = load(p);
    {
        let mut packed = u16::from(order.kind());
        match order.kind() {
            OT_GOTO_STATION => {
                if order.unload() == 1 {
                    packed |= 1 << 5;
                }
                if matches!(order.load(), 2 | 3) {
                    packed |= 1 << 6;
                }
                if order.non_stop() & 1 != 0 {
                    packed |= 1 << 7;
                }
                packed |= (order.destination & 255) << 8;
            }
            OT_GOTO_DEPOT => {
                if order.depot_type() & 2 == 0 {
                    packed |= 1 << 6;
                }
                packed |= 1 << 7;
                packed |= (order.destination & 255) << 8;
            }
            OT_LOADING => {
                if matches!(order.load(), 2 | 3) {
                    packed |= 1 << 6;
                }
                if order.load() == 4 && order.unload() == 4 {
                    packed = 0;
                }
            }
            _ => {}
        }
        u32::from(packed)
    }
}

/// `Order::ShouldStopAtStation`: check whether the given vehicle should stop at
/// the given station based on this order and the non-stop settings.
/// `last_station_visited` is the vehicle's `last_station_visited`.
pub(crate) fn should_stop_at_station(
    p: *const Order,
    last_station_visited: StationID,
    station: StationID,
) -> bool {
    use crate::order_type::{
        OrderDepotTypeFlag, OrderDepotTypeFlags, OrderNonStopFlag, OrderNonStopFlags,
    };
    let order = load(p);
    let is_dest_station = order.kind() == OT_GOTO_STATION && order.destination == station.0;

    (order.kind() != OT_GOTO_DEPOT
        || OrderDepotTypeFlags(order.depot_type()).test(OrderDepotTypeFlag::PartOfOrders))
        // Do stop only when we've not just been there
        && last_station_visited != station
        /* Finally do stop when there is no non-stop flag set for this type of station. */
        && !OrderNonStopFlags(order.non_stop()).test(if is_dest_station {
            OrderNonStopFlag::NoDestination
        } else {
            OrderNonStopFlag::NoIntermediate
        })
}

struct Control {
    g: Game,
}
impl Control {
    fn initialize_list(&self, list: *mut c_void, v: *mut c_void) {
        reenter!(openttd_rust_list_init(list, v, self.g.0));
    }
    fn new_list(&self, v: *mut c_void, input: Option<Order>, source: *mut c_void) -> *mut c_void {
        let g = self.g;
        let list = (g.0.allocate_list)();
        if let Some(order) = input {
            reenter!(openttd_rust_order_vector_insert(
                g.vector(list),
                0,
                &raw const order
            ));
        }
        if !source.is_null() {
            let vector = g.vector(source);
            let data = data(vector);
            let count = read_field!(vector, length);
            reenter!(openttd_rust_order_vector_assign(
                g.vector(list),
                data,
                count
            ));
        }
        self.initialize_list(list, v);
        list
    }
    fn insert(&self, v: *mut c_void, order: Order, index: u8) {
        let g = self.g;
        let list = g.orders(v);
        if list.is_null() {
            let list = self.new_list(v, Some(order), ptr::null_mut());
            write_field!(g.vehicle(v), orders, list);
        } else {
            reenter!(openttd_rust_list_insert(
                list,
                u32::from(index),
                &raw const order,
                g.0
            ));
        }
        let mut u = g.first(v);
        g.delete_order_news(u);
        while !u.is_null() {
            let state = g.consist(u);
            if index <= read_field!(state, real) {
                let next = u16::from(read_field!(state, real)) + 1;
                if next < u16::from(g.count(u)) {
                    write_field!(state, real, next as u8);
                }
            }
            if index == read_field!(state, implicit) && g.vehicle_type(u) < u64::from(VEH_SHIP) {
                assign_suppress_implicit(native_vehicle(u, g.0.base), true);
            }
            if index <= read_field!(state, implicit) {
                let next = u16::from(read_field!(state, implicit)) + 1;
                if next < u16::from(g.count(u)) {
                    write_field!(state, implicit, next as u8);
                }
            }
            g.reset(u);
            g.invalidate_order(u, i32::from(u16::from(u8::MAX) | (u16::from(index) << 8)));
            u = g.next(u);
        }
        let list = g.orders(v);
        for i in 0..g.stored_count(list) {
            let mut o = g.stored_order(list, i);
            if o.kind() == OT_CONDITIONAL {
                let target = o.flags;
                if target >= index {
                    o.flags = target.wrapping_add(1);
                }
                if target == i as u8 {
                    o.flags = ((u16::from(target) + 1) % u16::from(g.count(v))) as u8;
                }
                g.put_stored(list, i, o);
            }
        }
        g.invalidate_vehicle_list(v);
    }
    fn cancel_loading(&self, v: *mut c_void) {
        let mut order = self.g.current(v);
        order.kind &= !0xc0;
        if matches!(order.load(), 2 | 3) {
            order.flags &= !0x70;
        }
        self.g.put_current(v, order);
    }
    fn delete(&self, v: *mut c_void, index: u8) {
        let g = self.g;
        g.delete_order_at(g.orders(v), VehicleOrderID(index));
        let mut u = g.first(v);
        g.delete_order_news(u);
        while !u.is_null() {
            let state = g.consist(u);
            if index == read_field!(state, real) && g.current(u).kind() == OT_LOADING {
                self.cancel_loading(u);
            }
            if index < read_field!(state, real) {
                write_field!(state, real, read_field!(state, real).wrapping_sub(1));
            } else if index == read_field!(state, real) {
                g.update_real(u);
            }
            if index < read_field!(state, implicit) {
                write_field!(
                    state,
                    implicit,
                    read_field!(state, implicit).wrapping_sub(1)
                );
            } else if index == read_field!(state, implicit) {
                if read_field!(state, implicit) >= g.count(u) {
                    write_field!(state, implicit, 0);
                }
                while read_field!(state, implicit) != read_field!(state, real)
                    && g.vehicle_order(u, read_field!(state, implicit))
                        .unwrap()
                        .kind()
                        != 8
                {
                    write_field!(
                        state,
                        implicit,
                        read_field!(state, implicit).wrapping_add(1)
                    );
                    if read_field!(state, implicit) >= g.count(u) {
                        write_field!(state, implicit, 0);
                    }
                }
            }
            g.reset(u);
            g.invalidate_order(u, i32::from(u16::from(index) | (u16::from(u8::MAX) << 8)));
            u = g.next(u);
        }
        let list = g.orders(v);
        for i in 0..g.stored_count(list) {
            let mut o = g.stored_order(list, i);
            if o.kind() == OT_CONDITIONAL {
                let mut target = o.flags;
                if target >= index {
                    target = target.saturating_sub(1);
                }
                if target == i as u8 {
                    target = ((u16::from(target) + 1) % u16::from(g.count(v))) as u8;
                }
                o.flags = target;
                g.put_stored(list, i, o);
            }
        }
        g.invalidate_vehicle_list(v);
    }
    fn delete_implicit(&self, v: *mut c_void) {
        let g = self.g;
        let state = g.consist(v);
        let record = native_vehicle(v, g.0.base);
        if g.vehicle_type(v) < u64::from(VEH_SHIP) && suppress_implicit(record) {
            assign_suppress_implicit(record, false);
            write_field!(state, implicit, read_field!(state, real));
            g.invalidate_order(v, 0);
            return;
        }
        let list = g.orders(v);
        let orders = if list.is_null() {
            ptr::null_mut()
        } else {
            data(g.vector(list))
        };
        let mut current = read_field!(state, implicit);
        while current != u8::MAX {
            if read_field!(state, implicit) == read_field!(state, real) {
                break;
            }
            // Match the original captured span and unchanged local index after
            // DeleteOrder. Erasure never reallocates; all capacity slots remain
            // typed/live, including the retained former last order. No reference
            // or borrow crosses the deletion/window callbacks.
            if load(at(orders, usize::from(current))).kind() == OT_IMPLICIT {
                self.delete(v, read_field!(state, implicit));
            } else {
                let next = g.next_index(g.orders(v), current);
                if current < next {
                    write_field!(
                        state,
                        implicit,
                        read_field!(state, implicit).wrapping_add(1)
                    );
                } else {
                    write_field!(state, implicit, 0);
                }
                current = next;
            }
        }
    }
    fn move_order(&self, v: *mut c_void, from: u8, to: u8) {
        let g = self.g;
        let list = g.orders(v);
        reenter!(openttd_rust_order_vector_move(
            g.vector(list),
            usize::from(from),
            usize::from(to)
        ));
        let mut u = g.first(v);
        g.delete_order_news(u);
        fn remap(index: u8, from: u8, to: u8) -> u8 {
            if index == from {
                to
            } else if index > from && index <= to {
                index.wrapping_sub(1)
            } else if index < from && index >= to {
                index.wrapping_add(1)
            } else {
                index
            }
        }
        while !u.is_null() {
            let state = g.consist(u);
            write_field!(state, real, remap(read_field!(state, real), from, to));
            write_field!(
                state,
                implicit,
                remap(read_field!(state, implicit), from, to)
            );
            g.reset(u);
            g.invalidate_order(u, i32::from(u16::from(from) | (u16::from(to) << 8)));
            u = g.next(u);
        }
        for i in 0..g.stored_count(list) {
            let mut o = g.stored_order(list, i);
            if o.kind() == OT_CONDITIONAL {
                o.flags = remap(o.flags, from, to);
                g.put_stored(list, i, o);
            }
        }
        g.invalidate_vehicle_list(v);
    }
    fn add_shared(&self, v: *mut c_void, shared: *mut c_void) {
        let g = self.g;
        if g.orders(shared).is_null() {
            let list = self.new_list(shared, None, ptr::null_mut());
            write_field!(g.vehicle(v), orders, list);
            write_field!(g.vehicle(shared), orders, list);
        }
        let next = g.next(shared);
        write_field!(g.vehicle(v), next, next);
        write_field!(g.vehicle(v), previous, shared);
        write_field!(g.vehicle(shared), next, v);
        if !next.is_null() {
            write_field!(g.vehicle(next), previous, v);
        }
        let list = g.list(g.orders(shared));
        write_field!(list, vehicles, read_field!(list, vehicles).wrapping_add(1));
    }
    fn remove_shared(&self, v: *mut c_void) {
        let g = self.g;
        let first = g.first(v);
        let were_first = first == v;
        // Window number is copied before changing the head, as in the original.
        let identifier = g.shared_window(v, first);
        let list = g.list(g.orders(v));
        write_field!(list, vehicles, read_field!(list, vehicles).wrapping_sub(1));
        if read_field!(list, first) == v {
            write_field!(list, first, g.next(v));
        }
        let previous = g.previous(v);
        let next = g.next(v);
        if !were_first {
            write_field!(g.vehicle(previous), next, next);
        }
        if !next.is_null() {
            write_field!(g.vehicle(next), previous, previous);
        }
        if read_field!(list, vehicles) == 1 {
            g.close_shared_window(v, identifier as u32);
            g.invalidate_order(g.first(v), ((-2i64) as u64) as i32);
        } else if were_first {
            g.invalidate_shared_window(
                v,
                identifier as u32,
                (g.vehicle_id(g.first(v)) | (1 << 31)) as i32,
            );
        }
        write_field!(g.vehicle(v), next, ptr::null_mut());
        write_field!(g.vehicle(v), previous, ptr::null_mut());
    }
    fn delete_vehicle_orders(&self, v: *mut c_void, keep: bool, reset: bool) {
        let g = self.g;
        g.delete_order_news(v);
        let list = g.orders(v);
        if !list.is_null() && read_field!(g.list(list), vehicles) > 1 {
            self.remove_shared(v);
            write_field!(g.vehicle(v), orders, ptr::null_mut());
        } else if !list.is_null() {
            let remove = reenter!(openttd_rust_list_free(list, (u32::from(keep)) as u8, g.0));
            if remove != 0 {
                g.delete_list(list);
            }
            if !keep {
                write_field!(g.vehicle(v), orders, ptr::null_mut());
            }
        }
        g.reset(v);
        if reset {
            let state = g.consist(v);
            write_field!(state, implicit, 0);
            write_field!(state, real, 0);
            if g.current(v).kind() == OT_LOADING {
                self.cancel_loading(v);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_insert_order(
    v: *mut c_void,
    index: u8,
    input: *const Order,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    let input = load(input);
    control.insert(v, input, index);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_delete_vehicle_orders(
    v: *mut c_void,
    keep: u8,
    reset: u8,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.delete_vehicle_orders(v, keep != 0, reset != 0);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_add_shared(
    v: *mut c_void,
    shared: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.add_shared(v, shared);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_delete_implicit(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.delete_implicit(v);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_timetable_bulk(
    v: *mut c_void,
    field: u8,
    value: u16,
    execute: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    let g = control.g;
    {
        let valid = reenter!(openttd_rust_timetable_validate_bulk(v, field, output, g.0));
        if valid == 0 {
            return 0;
        }
        if execute != 0 {
            let mut index = 0u8;
            while index < g.count(v) {
                let order = g.vehicle_order(v, index);
                if order.is_some_and(|o| o.kind() != OT_IMPLICIT) {
                    g.change_timetable_command(v, index, field, value);
                }
                index = index.wrapping_add(1);
            }
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_destination(
    v: *mut c_void,
    depth: i32,
    look_ahead: u8,
    input: *const Order,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    let input = load(input);
    u64::from(control.destination(v, input, depth, look_ahead != 0))
}

/// `ProcessOrders`: handle the orders of a vehicle and determine the next place to go to
/// if needed. Returns true if the order was a new (non-implicit) order.
pub(crate) fn process_orders(v: VehicleRef, leaves: &'static Leaves) -> bool {
    let control = Control { g: Game(leaves) };
    control.process(v.shell())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_process_orders(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    u64::from(control.process(v))
}

/// `Vehicle::GetNextStoppingStation`: the stations the vehicle will stop at next, each
/// passed to `next_station` in the order the original appends them.
pub(crate) fn get_next_stopping_station(
    v: VehicleRef,
    leaves: &'static Leaves,
    next_station: &mut impl FnMut(StationID),
) {
    let g = Game(leaves);
    let list = g.orders(v.shell());
    if list.is_null() {
        return;
    }
    g.stopping(list, v.shell(), u8::MAX, 0, &mut |station| {
        next_station(StationID(station));
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_condition(
    v: *mut c_void,
    input: *const Order,
    leaves: &'static Leaves,
) -> u64 {
    let g = Game(leaves);

    let input = load(input);
    u64::from(g.conditional(input, v))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_location(
    v: *mut c_void,
    airport: u8,
    input: *const Order,
    leaves: &'static Leaves,
) -> u64 {
    let g = Game(leaves);

    let input = load(input);
    u64::from(g.get_location(input, v, airport != 0).0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_distance(
    v: *mut c_void,
    previous: u8,
    current: u8,
    depth: i32,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    let g = control.g;
    u64::from(g.get_order_distance(VehicleOrderID(previous), VehicleOrderID(current), v, depth))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_insert(
    v: *mut c_void,
    index: u8,
    execute: u8,
    input: *const Order,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    let input = load(input);
    control.insert_command(v, index, input, execute != 0, output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_delete(
    v: *mut c_void,
    index: u8,
    execute: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.delete_command(v, index, execute != 0, output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_skip(
    v: *mut c_void,
    index: u8,
    execute: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.skip_command(v, index, execute != 0, output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_move(
    v: *mut c_void,
    from: u8,
    to: u8,
    execute: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.move_command(v, from, to, execute != 0, output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_modify(
    v: *mut c_void,
    index: u8,
    field: u8,
    value: u16,
    execute: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.modify_command(v, index, field, value, execute != 0, output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_refit(
    v: *mut c_void,
    index: u8,
    cargo: u8,
    execute: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.refit_command(v, index, cargo, execute != 0, output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_clone(
    v: *mut c_void,
    source: *mut c_void,
    action: u8,
    execute: u8,
    output: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    // SAFETY: the native command passes its live CommandCost slot and the vehicle
    // shells `Vehicle::GetIfValid` returned.
    unsafe {
        cmd_clone_order(
            leaves,
            output,
            execute != 0,
            CmdCloneOrder {
                action: CloneOptions(action),
                veh_dst: v,
                veh_src: source,
            },
        )
    }
}

/// `CmdCloneOrder`'s arguments, with each vehicle as its shell.
#[derive(Clone, Copy)]
pub(crate) struct CmdCloneOrder {
    /// Action to perform.
    pub(crate) action: CloneOptions,
    /// Destination vehicle to clone orders to.
    pub(crate) veh_dst: *mut c_void,
    /// Source vehicle to clone orders from, if any (none for `CO_UNSHARE`).
    pub(crate) veh_src: *mut c_void,
}

/// `CmdCloneOrder`: clone, share or copy an order list of another vehicle. The cost or
/// error is in `output`.
///
/// # Safety
/// `output` is a live native `CommandCost`, and each vehicle is null or a live vehicle
/// shell, for the whole synchronous call.
pub(crate) unsafe fn cmd_clone_order(
    leaves: &'static Leaves,
    output: *mut c_void,
    execute: bool,
    args: CmdCloneOrder,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.clone_command(args.veh_dst, args.veh_src, args.action, execute, output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_capture_backup(
    v: *mut c_void,
    source: *const c_void,
    user: u32,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.capture_backup(v, source, user)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_backup_create(
    v: *mut c_void,
    user: u32,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.backup_create(v, u64::from(user))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_backup_restore(
    v: *mut c_void,
    user: u32,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.backup_restore(v, u64::from(user))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_backup_clear_user(
    tile: u32,
    user: u32,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.backup_clear_user(u64::from(tile), u64::from(user))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_backup_reset(user: u32, leaves: &'static Leaves) -> u64 {
    let control = Control { g: Game(leaves) };
    control.backup_reset(u64::from(user))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_backup_tile_clear(
    tile: u32,
    from_gui: u8,
    leaves: &'static Leaves,
) -> u64 {
    order_backup_reset(leaves, crate::tile_type::TileIndex(tile), from_gui != 0);
    0
}

/// `OrderBackup::Reset(t, from_gui)`: reset the order backups of the tile from GUI/game
/// logic; `from_gui` says whether the call came from the GUI, i.e. whether it must be
/// synced over the network.
pub(crate) fn order_backup_reset(
    leaves: &'static Leaves,
    t: crate::tile_type::TileIndex,
    from_gui: bool,
) {
    let control = Control { g: Game(leaves) };
    control.backup_tile_clear(t, from_gui);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_backup_group_clear(
    group: u16,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.backup_group_clear(u64::from(group))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_backup_vehicle_clear(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.backup_vehicle_clear(v)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_command_clear_backup(
    tile: u32,
    user: u32,
    execute: u8,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    {
        if execute != 0 {
            control.backup_clear_user(
                u64::from(if tile == 0 { u32::MAX } else { tile }),
                u64::from(user),
            );
        }
        1
    }
}

/// `RemoveOrderFromAllVehicles`: removes an order from all vehicles. Triggers when, say, a
/// station is removed.
pub(crate) fn remove_order_from_all_vehicles(
    leaves: &'static Leaves,
    order_type: OrderType,
    destination: StationID,
    hangar: bool,
) {
    let control = Control { g: Game(leaves) };
    control.remove_destination(order_type.0, destination.0, hangar);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_destination(
    kind: u8,
    destination: u16,
    hangar: u8,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.remove_destination(kind, destination, hangar != 0);
    0
}

/// `CheckOrders`: check the orders of a vehicle, to see if there are invalid orders and stuff.
pub(crate) fn check_orders(v: VehicleRef, leaves: &'static Leaves) {
    let control = Control { g: Game(leaves) };
    control.check_orders(|| v);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_check_orders(v: *mut c_void, leaves: &'static Leaves) -> u64 {
    let control = Control { g: Game(leaves) };
    control.check_orders(|| native_vehicle(v, leaves.base));
    0
}

#[repr(C)]
#[derive(Default)]
pub struct Closest {
    pub tile: u32,
    pub destination: u16,
    pub reverse: u8,
    pub found: u8,
}
impl Game {
    fn conditional(self, order: Order, v: *mut c_void) -> u8 {
        if order.kind() != OT_CONDITIONAL {
            return u8::MAX;
        }
        let variable = match order.variable() {
            0 => i32::from(crate::vehicle::visits::percent_filled(
                native_vehicle(v, self.0.base),
                None,
                self.0.base.visits(),
            )),
            1 => ((self.reliability(v) * 101) >> 16) as i32,
            7 => {
                (i32::from((self.0.base.engine)(native_vehicle(v, self.0.base)).reliability())
                    * 101)
                    >> 16
            }
            2 => (self.display_speed(v) as i32).wrapping_mul(10) / 16,
            3 => self.age_years(v) as i32,
            4 => self.needs_service(v) as i32,
            5 => return order.flags,
            6 => self.remaining_years(v) as i32,
            _ => unreachable!(),
        };
        let value = i32::from(order.value());
        let skip = match order.comparator() {
            0 => variable == value,
            1 => variable != value,
            2 => variable < value,
            3 => variable <= value,
            4 => variable > value,
            5 => variable >= value,
            6 => variable != 0,
            7 => variable == 0,
            _ => unreachable!(),
        };
        if skip { order.flags } else { u8::MAX }
    }
    /// Returns a tile somewhat representing the order destination (not suitable for pathfinding).
    /// `airport` gets the airport tile rather than the station location for aircraft.
    /// Returns the destination of the order, or `INVALID_TILE` if none.
    fn get_location(self, order: Order, v: *mut c_void, airport: bool) -> TileIndex {
        match OrderType(order.kind()) {
            OrderType::OT_GOTO_STATION | OrderType::OT_GOTO_WAYPOINT | OrderType::OT_IMPLICIT => {
                TileIndex(
                    (if airport && self.vehicle_type(v) == u64::from(VehicleType::VEH_AIRCRAFT.0) {
                        self.airport_tile(order.destination)
                    } else {
                        u64::from((self.shared().base_station_xy)(order.destination))
                    }) as u32,
                )
            }
            OrderType::OT_GOTO_DEPOT => {
                if DepotID(order.destination) == DepotID::INVALID {
                    return TileIndex::INVALID;
                }
                TileIndex(
                    (if self.vehicle_type(v) == u64::from(VehicleType::VEH_AIRCRAFT.0) {
                        u64::from((self.shared().station_xy)(order.destination))
                    } else {
                        self.depot_tile(order.destination)
                    }) as u32,
                )
            }
            _ => TileIndex::INVALID,
        }
    }
    /// Get the distance between two orders of a vehicle. Conditional orders are resolved
    /// and the bigger distance of the two order branches is returned.
    fn get_order_distance(
        self,
        previous: VehicleOrderID,
        current: VehicleOrderID,
        v: *mut c_void,
        depth: i32,
    ) -> u32 {
        let list = self.orders(v);
        // This source helper indexes the full stored span, not GetOrderAt's
        // uint8 count. Oversized save-loaded lists can still have conditional
        // destinations beyond their narrowed GetNumOrders value.
        let order = self.stored_order(list, usize::from(current.0));
        if OrderType(order.kind()) == OrderType::OT_CONDITIONAL {
            if depth > i32::from(self.count(v)) {
                return 0;
            }
            let d1 = self.get_order_distance(
                previous,
                VehicleOrderID(order.flags),
                v,
                depth.wrapping_add(1),
            ) as i32;
            let d2 = self.get_order_distance(
                previous,
                VehicleOrderID(self.next_index(list, current.0)),
                v,
                depth.wrapping_add(1),
            ) as i32;
            return d1.max(d2) as u32;
        }
        let prev = self.get_location(self.stored_order(list, usize::from(previous.0)), v, true);
        let cur = self.get_location(order, v, true);
        if prev == TileIndex::INVALID || cur == TileIndex::INVALID {
            return 0;
        }
        // SAFETY: get_location resolved live destination tiles and the invalid sentinel
        // was excluded above. The vehicle shell is live; no owner borrow spans either call.
        unsafe {
            (self.0.distance)(
                prev.0,
                cur.0,
                u8::from(self.vehicle_type(v) == u64::from(VehicleType::VEH_AIRCRAFT.0)),
            ) as u32
        }
    }
}
impl Control {
    fn destination(&self, v: *mut c_void, mut order: Order, mut depth: i32, pbs: bool) -> bool {
        let g = self.g;
        loop {
            if depth > i32::from(g.count(v)) {
                let mut current = g.current(v);
                current.kind = OT_NOTHING;
                current.flags = 0;
                current.destination = 0;
                g.put_current(v, current);
                g.set_destination(v, 0);
                return false;
            }
            match order.kind() {
                OT_GOTO_STATION => {
                    let tile = g.station_location(v, order.destination);
                    g.set_destination(v, tile as u32);
                    return true;
                }
                OT_GOTO_DEPOT => {
                    if order.depot_type() & 1 != 0 && g.needs_service(v) == 0 {
                        g.update_timetable(v, true);
                        g.increment_real(v);
                    } else if g.current(v).action() & 2 != 0 {
                        if g.destination_tile(v) == 0
                            && u64::from(crate::timer_game_common::economy_fraction())
                                != g.vehicle_id(v) % DAY_TICKS as u64
                        {
                        } else {
                            let mut closest = Closest::default();
                            g.closest_depot(v, ptr::addr_of_mut!(closest));
                            if closest.found != 0 {
                                if pbs && closest.reverse != 0 {
                                    return false;
                                }
                                g.set_destination(v, closest.tile);
                                let mut current = g.current(v);
                                current.destination = closest.destination;
                                g.put_current(v, current);
                                if g.vehicle_type(v) == u64::from(VEH_TRAIN) && closest.reverse != 0
                                {
                                    g.reverse_train(v);
                                }
                                if g.vehicle_type(v) == u64::from(VEH_AIRCRAFT)
                                    && g.aircraft_flying(v) != 0
                                    && g.target_airport(v) != u64::from(closest.destination)
                                {
                                    g.next_airport(v);
                                }
                                return true;
                            }
                            if pbs {
                                return false;
                            }
                            g.update_timetable(v, true);
                            g.increment_real(v);
                        }
                    } else {
                        if g.vehicle_type(v) != u64::from(VEH_AIRCRAFT) {
                            let tile = g.depot_tile(order.destination);
                            g.set_destination(v, tile as u32);
                        } else {
                            let destination = g.current(v).destination;
                            if g.target_airport(v) != u64::from(destination) {
                                let tile = g.station_location(v, destination);
                                g.set_destination(v, tile as u32);
                            }
                        }
                        return true;
                    }
                }
                OT_GOTO_WAYPOINT => {
                    let tile = g.waypoint_tile(order.destination);
                    g.set_destination(v, tile as u32);
                    return true;
                }
                OT_CONDITIONAL => {
                    let next = self.g.conditional(order, v);
                    if next != u8::MAX {
                        g.update_timetable(v, false);
                        let state = g.consist(v);
                        write_field!(state, implicit, next);
                        write_field!(state, real, next);
                        g.update_real(v);
                        let travel = g
                            .vehicle_order(v, read_field!(state, real))
                            .unwrap()
                            .timetabled_travel();
                        write_field!(
                            state,
                            time,
                            read_field!(state, time).wrapping_add(i32::from(travel))
                        );
                        if g.vehicle_type(v) < u64::from(VEH_SHIP) {
                            assign_suppress_implicit(native_vehicle(v, g.0.base), true);
                        }
                    } else {
                        g.update_timetable(v, true);
                        g.increment_real(v);
                    }
                }
                _ => {
                    g.set_destination(v, 0);
                    return false;
                }
            }
            let real = read_field!(g.consist(v), real);
            let next = g.vehicle_order(v, real).filter(|o| o.kind() != OT_IMPLICIT);
            let Some(next) = next else {
                let mut current = g.current(v);
                current.kind = OT_NOTHING;
                current.flags = 0;
                current.destination = 0;
                g.put_current(v, current);
                g.set_destination(v, 0);
                return false;
            };
            g.put_current(v, next);
            order = next;
            depth = depth.wrapping_add(1);
        }
    }
    fn process(&self, v: *mut c_void) -> bool {
        let g = self.g;
        let mut vehicle = g.vehicle(v);
        let current_ptr = current_order(vehicle);
        // Match the source's early order-type exits before fetching anything else.
        let order_kind = read_field!(current_ptr, kind) & 15;
        match order_kind {
            OT_GOTO_DEPOT => {
                if (read_field!(current_ptr, flags) & 7) & 2 == 0 {
                    return false;
                }
            }
            OT_LOADING => return false,
            OT_LEAVESTATION => {
                if g.vehicle_type(v) != u64::from(VEH_AIRCRAFT) {
                    return false;
                }
            }
            _ => {}
        }
        let reverse = order_kind == OT_NOTHING;
        let current = load(current_ptr);
        if ((order_kind == OT_GOTO_STATION && current.non_stop() & 2 != 0)
            || order_kind == OT_GOTO_WAYPOINT)
            && g.at_station(v) != 0
            && u64::from(current.destination) == g.tile_station(v)
        {
            self.delete_implicit(v);
            native_vehicle(v, g.0.base).set_last_station_visited_raw(current.destination);
            g.update_timetable(v, true);
            g.increment_implicit(v);
            // Deletion/timetable/index updates may mutate the list or reenter.
            vehicle = g.vehicle(v);
        }
        let state = g.consist(v);
        let access = g.order_access(vehicle);
        access.update_real(state);
        let order = access
            .order_ptr(read_field!(state, real))
            // SAFETY: The handle addresses a live order; this scalar observation
            // ends before the next service call.
            .filter(|o| unsafe { (&**o).kind() != OT_IMPLICIT });
        let kind = g.vehicle_type(v);
        if order.is_none() || (kind == u64::from(VEH_AIRCRAFT) && !access.valid()) {
            if kind == u64::from(VEH_AIRCRAFT) {
                g.missing_aircraft_orders(v);
                return false;
            }
            let mut current = load(current_order(vehicle));
            current.kind = OT_NOTHING;
            current.flags = 0;
            current.destination = 0;
            store(current_order(vehicle), current);
            g.set_destination(v, 0);
            return false;
        }
        let order = order.unwrap();
        // After update_real and order_ptr, valid only inspects the list.
        // From order_ptr to materialization, vehicle_type,
        // destination_tile and ship_station_tile are read-only native leaves;
        // none reenters or mutates orders. Thus the raw handle stays live, with
        // no owner/storage borrow crossing any callback. Earlier implicit-order
        // mutations have already reacquired vehicle and order storage above.
        // SAFETY: Equality only reads the canonical orders and calls no service;
        // both shared borrows end before destination_tile or ship_station_tile.
        if unsafe { (&*order).equals(&*(*vehicle).current.as_ptr()) }
            && (kind == u64::from(VEH_AIRCRAFT) || g.destination_tile(v) != 0)
            && (kind != u64::from(VEH_SHIP)
                // SAFETY: Fresh scalar read of the still-live order after the
                // destination gate, without retaining a borrow across a service.
                || unsafe { (&*order).kind() } != OT_GOTO_STATION
                || g.ship_station_tile(read_field!(order, destination)) != u64::from(u32::MAX))
        {
            return false;
        }
        let order = load(order);
        store(current_order(vehicle), order);
        g.invalidate_order(v, ((-2i64) as u64) as i32);
        if kind == u64::from(VEH_SHIP) || kind == u64::from(VEH_AIRCRAFT) {
            g.dirty_vehicle_windows(v);
        }
        self.destination(v, order, 0, false) && reverse
    }
}

impl Game {
    fn check_owner(self, owner: u64, output: *mut c_void) -> bool {
        // SAFETY: The entry supplies immutable owning tables and a live native cost slot.
        let station = unsafe { &*self.0.station_commands };
        // SAFETY: These canonical command/cost tables outlive the synchronous entry.
        let (costs, commands) = unsafe { (&*station.cost, &*station.command) };
        let result = crate::services::Cost(output.cast(), costs);
        result.ownership(commands, owner as u8, 0);
        result.ok()
    }
}
impl Control {
    fn insert_command(
        &self,
        v: *mut c_void,
        index: u8,
        order: Order,
        execute: bool,
        output: *mut c_void,
    ) -> u64 {
        let g = self.g;
        if !self.g.primary(v) {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if order.refit != 0xfe || order.wait != 0 || order.travel != 0 || order.speed != u16::MAX {
            return u64::from(self.g.cmd_error(output));
        }
        let kind = g.vehicle_type(v);
        let destination = u64::from(order.destination);
        match order.kind() {
            OT_GOTO_STATION => {
                if (g.0.valid_station)(destination as u16) == 0 {
                    return u64::from(self.g.cmd_error(output));
                }
                let owner = g.station_owner(destination as u16);
                if owner != u64::from(OWNER_NONE) && !self.g.check_owner(owner, output) {
                    return 0;
                }
                if order_can_use_station(v, destination as u16) == 0 {
                    return u64::from(self.g.command_cost_extra(
                        output,
                        STR_ERROR_CAN_T_ADD_ORDER,
                        StringID(g.station_error(v, destination as u16) as u32),
                    ));
                }
                let mut u = g.first(v);
                while !u.is_null() {
                    if order_can_use_station(u, destination as u16) == 0 {
                        return u64::from(self.g.command_cost_extra(
                            output,
                            STR_ERROR_CAN_T_ADD_ORDER_SHARED,
                            StringID(g.station_error(u, destination as u16) as u32),
                        ));
                    }
                    u = g.next(u);
                }
                if order.non_stop() != 0 && kind >= u64::from(VEH_SHIP) {
                    return u64::from(self.g.cmd_error(output));
                }
                match order.load() {
                    0 | 4 => {}
                    2 | 3 => {
                        if g.any_order(v, |o| o.kind() == OT_GOTO_DEPOT && o.action() & 4 != 0) {
                            return u64::from(
                                self.g
                                    .command_cost(output, STR_ERROR_UNBUNCHING_NO_FULL_LOAD),
                            );
                        }
                    }
                    _ => return u64::from(self.g.cmd_error(output)),
                }
                if !matches!(order.unload(), 0 | 1 | 2 | 4) {
                    return u64::from(self.g.cmd_error(output));
                }
                match (order.kind >> 4) & 3 {
                    0 | 1 => {
                        if kind != u64::from(VEH_TRAIN) {
                            return u64::from(self.g.cmd_error(output));
                        }
                    }
                    2 => {}
                    _ => return u64::from(self.g.cmd_error(output)),
                }
            }
            OT_GOTO_DEPOT => {
                if order.action() & 2 == 0 {
                    if kind == u64::from(VEH_AIRCRAFT) {
                        if (g.0.valid_station)(destination as u16) == 0 {
                            return u64::from(self.g.cmd_error(output));
                        }
                        if !self
                            .g
                            .check_owner(g.station_owner(destination as u16), output)
                        {
                            return 0;
                        }
                        if order_can_use_station(v, destination as u16) == 0
                            || u64::from(crate::station_base::airport_has_hangar(
                                crate::newgrf_airport::station_by_id(
                                    destination as u16,
                                    crate::newgrf_airport::runtime(),
                                ),
                                crate::newgrf_airport::runtime(),
                            )) == 0
                        {
                            return u64::from(self.g.cmd_error(output));
                        }
                    } else {
                        if (g.0.valid_depot)(destination as u16) == 0 {
                            return u64::from(self.g.cmd_error(output));
                        }
                        if !self
                            .g
                            .check_owner(g.depot_owner(destination as u16), output)
                        {
                            return 0;
                        }
                        if kind > u64::from(VEH_SHIP)
                            || match kind {
                                k if k == u64::from(VEH_TRAIN) => g.rail_depot(destination as u16),
                                k if k == u64::from(VEH_ROAD) => g.road_depot(destination as u16),
                                _ => g.ship_depot(destination as u16),
                            } == 0
                        {
                            return u64::from(self.g.cmd_error(output));
                        }
                    }
                }
                if order.non_stop() != 0 && kind >= u64::from(VEH_SHIP) {
                    return u64::from(self.g.cmd_error(output));
                }
                let mut flags = order.depot_type();
                if flags & 2 != 0 {
                    flags &= !1;
                }
                flags &= !2;
                if flags != 0 || order.action() & !7 != 0 {
                    return u64::from(self.g.cmd_error(output));
                }
                if order.depot_type() & 1 != 0 && order.action() & 5 != 0 {
                    return u64::from(self.g.cmd_error(output));
                }
                if order.action() & 4 != 0 {
                    if g.any_order(v, |o| {
                        o.kind() == OT_GOTO_STATION && matches!(o.load(), 2 | 3)
                    }) {
                        return u64::from(
                            self.g
                                .command_cost(output, STR_ERROR_UNBUNCHING_NO_UNBUNCHING_FULL_LOAD),
                        );
                    }
                    if g.any_order(v, |o| o.kind() == OT_GOTO_DEPOT && o.action() & 4 != 0) {
                        return u64::from(
                            self.g
                                .command_cost(output, STR_ERROR_UNBUNCHING_ONLY_ONE_ALLOWED),
                        );
                    }
                    if g.any_order(v, |o| o.kind() == OT_CONDITIONAL) {
                        return u64::from(
                            self.g.command_cost(
                                output,
                                STR_ERROR_UNBUNCHING_NO_UNBUNCHING_CONDITIONAL,
                            ),
                        );
                    }
                }
            }
            OT_GOTO_WAYPOINT => {
                if (g.0.valid_waypoint)(destination as u16) == 0 {
                    return u64::from(self.g.cmd_error(output));
                }
                let facilities = g.waypoint_facilities(destination as u16);
                match kind {
                    k if k == u64::from(VEH_TRAIN) => {
                        if facilities & 1 == 0 {
                            return u64::from(self.g.command_cost_extra(
                                output,
                                STR_ERROR_CAN_T_ADD_ORDER,
                                STR_ERROR_NO_RAIL_WAYPOINT,
                            ));
                        }
                        if !self
                            .g
                            .check_owner(g.waypoint_owner(destination as u16), output)
                        {
                            return 0;
                        }
                    }
                    k if k == u64::from(VEH_ROAD) => {
                        if facilities & 6 == 0 {
                            return u64::from(self.g.command_cost_extra(
                                output,
                                STR_ERROR_CAN_T_ADD_ORDER,
                                STR_ERROR_NO_ROAD_WAYPOINT,
                            ));
                        }
                        if !self
                            .g
                            .check_owner(g.waypoint_owner(destination as u16), output)
                        {
                            return 0;
                        }
                    }
                    k if k == u64::from(VEH_SHIP) => {
                        if facilities & 16 == 0 {
                            return u64::from(self.g.command_cost_extra(
                                output,
                                STR_ERROR_CAN_T_ADD_ORDER,
                                STR_ERROR_NO_BUOY,
                            ));
                        }
                        let owner = g.waypoint_owner(destination as u16);
                        if owner != u64::from(OWNER_NONE) && !self.g.check_owner(owner, output) {
                            return 0;
                        }
                    }
                    _ => return u64::from(self.g.cmd_error(output)),
                }
                if order.non_stop() != 0 && kind >= u64::from(VEH_SHIP) {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            OT_CONDITIONAL => {
                if order.flags != 0 && order.flags >= g.count(v) {
                    return u64::from(self.g.cmd_error(output));
                }
                if order.variable() >= 8 {
                    return u64::from(self.g.cmd_error(output));
                }
                if g.any_order(v, |o| o.kind() == OT_GOTO_DEPOT && o.action() & 4 != 0) {
                    return u64::from(
                        self.g
                            .command_cost(output, STR_ERROR_UNBUNCHING_NO_CONDITIONAL),
                    );
                }
                let compare = order.comparator();
                if compare >= 8 {
                    return u64::from(self.g.cmd_error(output));
                }
                match order.variable() {
                    4 => {
                        if compare != 6 && compare != 7 {
                            return u64::from(self.g.cmd_error(output));
                        }
                    }
                    5 => {
                        if compare != 0 || order.value() != 0 {
                            return u64::from(self.g.cmd_error(output));
                        }
                    }
                    variable => {
                        if matches!(variable, 0 | 1 | 7) && order.value() > 100 {
                            return u64::from(self.g.cmd_error(output));
                        }
                        if compare == 6 || compare == 7 {
                            return u64::from(self.g.cmd_error(output));
                        }
                    }
                }
            }
            _ => return u64::from(self.g.cmd_error(output)),
        }
        if index > g.count(v) {
            return u64::from(self.g.cmd_error(output));
        }
        if g.count(v) >= 254 {
            return u64::from(self.g.command_cost(output, STR_ERROR_TOO_MANY_ORDERS));
        }
        if g.orders(v).is_null() && (g.0.list_capacity)() == 0 {
            return u64::from(
                self.g
                    .command_cost(output, STR_ERROR_NO_MORE_SPACE_FOR_ORDERS),
            );
        }
        if execute {
            self.insert(v, order, index);
        }
        1
    }
    fn declone(&self, v: *mut c_void, execute: bool) -> u64 {
        if execute {
            self.delete_vehicle_orders(v, false, true);
            self.g.invalidate_order(v, ((-1i64) as u64) as i32);
            self.g.invalidate_vehicle_list(v);
        }
        1
    }
    fn delete_command(&self, v: *mut c_void, index: u8, execute: bool, output: *mut c_void) -> u64 {
        let g = self.g;
        if !self.g.primary(v) {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if index >= g.count(v) {
            return self.declone(v, execute);
        }
        if g.vehicle_order(v, index).is_none() {
            return u64::from(self.g.cmd_error(output));
        }
        if execute {
            self.delete(v, index);
        }
        1
    }
    fn skip_command(&self, v: *mut c_void, index: u8, execute: bool, output: *mut c_void) -> u64 {
        let g = self.g;
        if !self.g.primary(v) {
            return u64::from(self.g.cmd_error(output));
        }
        let state = g.consist(v);
        if index == read_field!(state, implicit) || index >= g.count(v) || g.count(v) < 2 {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if execute {
            if g.current(v).kind() == OT_LOADING {
                let vehicle = native_vehicle(v, self.g.0.base);
                crate::vehicle::visits::leave_station(vehicle, g.0.base.visits());
            }
            write_field!(state, implicit, index);
            write_field!(state, real, index);
            g.update_real(v);
            g.reset(v);
            g.invalidate_order(v, ((-2i64) as u64) as i32);
            if matches!(g.vehicle_type(v), 2 | 3) {
                g.dirty_vehicle_windows(v);
            }
        }
        1
    }
    fn move_command(
        &self,
        v: *mut c_void,
        from: u8,
        to: u8,
        execute: bool,
        output: *mut c_void,
    ) -> u64 {
        let g = self.g;
        if !self.g.primary(v) {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if from >= g.count(v)
            || to >= g.count(v)
            || from == to
            || g.count(v) <= 1
            || g.vehicle_order(v, from).is_none()
        {
            return u64::from(self.g.cmd_error(output));
        }
        if execute {
            self.move_order(v, from, to);
        }
        1
    }
    fn modify_command(
        &self,
        v: *mut c_void,
        index: u8,
        field: u8,
        data: u16,
        execute: bool,
        output: *mut c_void,
    ) -> u64 {
        let g = self.g;
        if field >= 9 {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.primary(v) {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if index >= g.count(v) {
            return u64::from(self.g.cmd_error(output));
        }
        let list = g.orders(v);
        let mut order = g.order(list, index).unwrap();
        match order.kind() {
            OT_GOTO_STATION => {
                if !matches!(field, 0..=3) {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            OT_GOTO_DEPOT => {
                if !matches!(field, 0 | 4) {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            OT_GOTO_WAYPOINT => {
                if field != 0 {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            OT_CONDITIONAL => {
                if !matches!(field, 5..=8) {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            _ => return u64::from(self.g.cmd_error(output)),
        }
        let byte = data as u8;
        match field {
            0 => {
                if g.vehicle_type(v) >= u64::from(VEH_SHIP)
                    || byte == order.non_stop()
                    || byte & !3 != 0
                {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            1 => {
                if g.vehicle_type(v) != u64::from(VEH_TRAIN) || data >= 3 {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            2 => {
                if order.non_stop() & 2 != 0
                    || byte == order.unload()
                    || !matches!(byte, 0 | 1 | 2 | 4)
                {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            3 => {
                if order.non_stop() & 2 != 0 || byte == order.load() {
                    return u64::from(self.g.cmd_error(output));
                }
                match byte {
                    0 | 4 => {}
                    2 | 3 => {
                        if g.any_order(v, |o| o.kind() == OT_GOTO_DEPOT && o.action() & 4 != 0) {
                            return u64::from(
                                self.g
                                    .command_cost(output, STR_ERROR_UNBUNCHING_NO_FULL_LOAD),
                            );
                        }
                    }
                    _ => return u64::from(self.g.cmd_error(output)),
                }
            }
            4 => {
                if byte >= 4 {
                    return u64::from(self.g.cmd_error(output));
                }
                if byte == 3 {
                    if g.any_order(v, |o| o.kind() == OT_GOTO_DEPOT && o.action() & 4 != 0)
                        && order.action() & 4 == 0
                    {
                        return u64::from(
                            self.g
                                .command_cost(output, STR_ERROR_UNBUNCHING_ONLY_ONE_ALLOWED),
                        );
                    }
                    if g.any_order(v, |o| o.kind() == OT_CONDITIONAL) {
                        return u64::from(
                            self.g.command_cost(
                                output,
                                STR_ERROR_UNBUNCHING_NO_UNBUNCHING_CONDITIONAL,
                            ),
                        );
                    }
                    if g.any_order(v, |o| {
                        o.kind() == OT_GOTO_STATION && matches!(o.load(), 2 | 3)
                    }) {
                        return u64::from(
                            self.g
                                .command_cost(output, STR_ERROR_UNBUNCHING_NO_UNBUNCHING_FULL_LOAD),
                        );
                    }
                }
            }
            5 => {
                if byte >= 8 {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            6 => {
                if byte >= 8 {
                    return u64::from(self.g.cmd_error(output));
                }
                match order.variable() {
                    5 => return u64::from(self.g.cmd_error(output)),
                    4 => {
                        if byte != 6 && byte != 7 {
                            return u64::from(self.g.cmd_error(output));
                        }
                    }
                    _ => {
                        if byte == 6 || byte == 7 {
                            return u64::from(self.g.cmd_error(output));
                        }
                    }
                }
            }
            7 => match order.variable() {
                4 | 5 => return u64::from(self.g.cmd_error(output)),
                0 | 1 | 7 => {
                    if data > 100 {
                        return u64::from(self.g.cmd_error(output));
                    }
                }
                _ => {
                    if data > 2047 {
                        return u64::from(self.g.cmd_error(output));
                    }
                }
            },
            8 => {
                if data >= u16::from(g.count(v)) {
                    return u64::from(self.g.cmd_error(output));
                }
            }
            _ => unreachable!(),
        }
        if execute {
            match field {
                0 => {
                    order.kind = (order.kind & !0xc0) | ((byte & 3) << 6);
                    if order.non_stop() & 2 != 0 {
                        order.refit = 0xfe;
                        order.flags &= !0x77;
                    }
                }
                1 => order.kind = (order.kind & !0x30) | ((byte & 3) << 4),
                2 => order.flags = (order.flags & !7) | (byte & 7),
                3 => {
                    order.flags = (order.flags & !0x70) | ((byte & 7) << 4);
                    if order.load() == 4 {
                        order.refit = 0xfe;
                    }
                }
                4 => match byte {
                    0 => {
                        order.flags &= !(1 | 8 | 32);
                    }
                    1 => {
                        order.flags = (order.flags | 1) & !(8 | 32);
                        order.refit = 0xfe;
                    }
                    2 => {
                        order.flags = (order.flags | 8) & !(1 | 32);
                        order.refit = 0xfe;
                    }
                    3 => {
                        order.flags = (order.flags | 32) & !(1 | 8);
                    }
                    _ => unreachable!(),
                },
                5 => {
                    order.destination = (order.destination & 2047) | (u16::from(byte & 31) << 11);
                    let compare = order.comparator();
                    match order.variable() {
                        5 => {
                            order.kind &= !0xe0;
                            order.destination &= !2047;
                        }
                        4 => {
                            if compare != 6 && compare != 7 {
                                order.kind = (order.kind & !0xe0) | (6 << 5);
                            }
                            order.destination &= !2047;
                        }
                        variable => {
                            if matches!(variable, 0 | 1 | 7) && order.value() > 100 {
                                order.destination = (order.destination & !2047) | 0x64;
                            }
                            if compare == 6 || compare == 7 {
                                order.kind &= !0xe0;
                            }
                        }
                    }
                }
                6 => order.kind = (order.kind & !0xe0) | ((byte & 7) << 5),
                7 => order.destination = (order.destination & !2047) | (data & 2047),
                8 => order.flags = byte,
                _ => unreachable!(),
            }
            g.put_order(list, index, order);
            let mut u = g.first(v);
            g.delete_order_news(u);
            while !u.is_null() {
                let state = g.consist(u);
                let mut current = g.current(u);
                if index == read_field!(state, real)
                    && matches!(current.kind(), OT_GOTO_STATION | OT_LOADING)
                    && current.load() != order.load()
                {
                    current.flags = (current.flags & !0x70) | ((order.load() & 7) << 4);
                    g.put_current(u, current);
                }
                g.reset(u);
                g.invalidate_order(u, ((-2i64) as u64) as i32);
                u = g.next(u);
            }
        }
        1
    }
    fn refit_command(
        &self,
        v: *mut c_void,
        index: u8,
        cargo: u8,
        execute: bool,
        output: *mut c_void,
    ) -> u64 {
        let g = self.g;
        if cargo >= 64 && cargo != 0xfe && cargo != 0xfd {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.primary(v) {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        let Some(mut order) = g.vehicle_order(v, index) else {
            return u64::from(self.g.cmd_error(output));
        };
        if cargo == 0xfd && order.kind() != OT_GOTO_STATION {
            return u64::from(self.g.cmd_error(output));
        }
        if order.load() == 4 {
            return u64::from(self.g.cmd_error(output));
        }
        if execute {
            order.refit = cargo;
            if cargo != 0xfe && order.kind() == OT_GOTO_DEPOT {
                order.flags &= !(1 | 8);
            }
            g.put_order(g.orders(v), index, order);
            let mut u = g.first(v);
            while !u.is_null() {
                g.invalidate_order(u, ((-2i64) as u64) as i32);
                if index == read_field!(g.consist(u), real) && g.current(u).depot_type() & 2 != 0 {
                    let mut current = g.current(u);
                    current.refit = cargo;
                    g.put_current(u, current);
                }
                u = g.next(u);
            }
        }
        1
    }
}

impl Vector {
    unsafe fn transfer(&mut self, other: *mut Self) {
        // Both owners are live/disjoint and no typed order pointer is used during
        // transfer. Existing C++ Order lifetimes move with the allocation.
        // SAFETY: `other` is a distinct live vector; the swaps exchange its owned storage.
        unsafe {
            std::mem::swap(&mut self.storage, &mut (*other).storage);
            std::mem::swap(&mut self.length, &mut (*other).length);
        }
    }
}
impl Game {
    fn backup(self, b: *mut c_void) -> *mut Backup {
        // SAFETY: The pool traversal supplies a live OrderBackup shell.
        unsafe { (self.0.backup)(b) }
    }
    fn backup_vector(self, b: *mut c_void) -> *mut Vector {
        // SAFETY: The pool traversal supplies a live OrderBackup shell and its storage owner.
        unsafe { (self.0.backup_vector)(b) }
    }
    fn backup_consist(self, b: *mut c_void) -> *mut Consist {
        // SAFETY: The pool traversal supplies a live OrderBackup shell and its consist owner.
        unsafe { (self.0.backup_consist)(b) }
    }
    fn next_backup(self, cursor: &mut u32) -> *mut c_void {
        // SAFETY: The callback only advances this exclusive live cursor.
        unsafe { (self.0.next_backup)(cursor) }
    }
    fn next_vehicle(self, first: u32) -> *mut c_void {
        (self.0.next_vehicle)(first)
    }
    fn backed_order(self, b: *mut c_void, i: usize) -> Order {
        load(slot(self.backup_vector(b), i))
    }
    fn captured(self, backup: *mut c_void, v: *mut c_void, user: u32) {
        let state = self.backup(backup);
        write_field!(state, user, user);
        write_field!(state, tile, native_vehicle(v, self.0.base).tile_raw());
        write_field!(state, group, self.vehicle_group(v) as u16);
        reenter!(openttd_rust_consist_copy(
            self.backup_consist(backup),
            self.consist(v)
        ));
        self.capture_backup_metadata(backup, v);
        let list = self.orders(v);
        if !list.is_null() && read_field!(self.list(list), vehicles) > 1 {
            let first = self.first(v);
            let clone = if first == v { self.next(v) } else { first };
            write_field!(state, clone, clone);
        } else if !list.is_null() {
            let vector = self.vector(list);
            let data = data(vector);
            let length = read_field!(vector, length);
            reenter!(openttd_rust_order_vector_assign(
                self.backup_vector(backup),
                data,
                length
            ));
        }
    }
    fn aircraft_range(self, dst: *mut c_void, src: *mut c_void) -> bool {
        // SAFETY: The range check receives a live aircraft shell.
        if unsafe { (self.0.aircraft_range)(dst) } == 0 || self.count(src) == 0 {
            return true;
        }
        let list = self.orders(src);
        let max = self.aircraft_range_square(dst) as u32;
        for i in 0..self.size(list) {
            if matches!(
                self.order(list, i).unwrap().kind(),
                OT_GOTO_STATION | OT_GOTO_DEPOT | OT_GOTO_WAYPOINT
            ) && self.get_order_distance(
                VehicleOrderID(i),
                VehicleOrderID(self.next_index(list, i)),
                src,
                0,
            ) > max
            {
                return false;
            }
        }
        true
    }
}
impl Control {
    fn clone_command(
        &self,
        dst: *mut c_void,
        src: *mut c_void,
        action: CloneOptions,
        execute: bool,
        output: *mut c_void,
    ) -> u64 {
        let g = self.g;
        if !self.g.primary(dst) {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(dst, output) {
            return 0;
        }
        if action == CloneOptions::CO_UNSHARE {
            return self.declone(dst, execute);
        }
        if action != CloneOptions::CO_SHARE && action != CloneOptions::CO_COPY {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.primary(src) || g.vehicle_type(dst) != g.vehicle_type(src) || dst == src {
            return u64::from(self.g.cmd_error(output));
        }
        if !self.g.owner_check(src, output) {
            return 0;
        }
        let kind = g.vehicle_type(src);
        if action == CloneOptions::CO_SHARE {
            if kind == u64::from(VEH_ROAD) && g.bus(src) != g.bus(dst) {
                return u64::from(self.g.cmd_error(output));
            }
            if g.first(src) == g.first(dst) {
                return u64::from(self.g.cmd_error(output));
            }
        }
        let list = g.orders(src);
        if !list.is_null() {
            for i in 0..g.stored_count(list) {
                let order = g.stored_order(list, i);
                if order.kind() != OT_GOTO_STATION
                    && !(kind == u64::from(VEH_AIRCRAFT)
                        && order.kind() == OT_GOTO_DEPOT
                        && order.destination != u16::MAX)
                {
                    continue;
                }
                let destination = u64::from(order.destination);
                if (action == CloneOptions::CO_COPY
                    || order_can_use_station(src, destination as u16) != 0)
                    && order_can_use_station(dst, destination as u16) == 0
                {
                    // Both the share and the copy branch of `CmdCloneOrder` return this.
                    return u64::from(self.g.command_cost_extra(
                        output,
                        STR_ERROR_CAN_T_COPY_SHARE_ORDER,
                        StringID(g.station_error(dst, destination as u16) as u32),
                    ));
                }
            }
        }
        if kind == u64::from(VEH_AIRCRAFT) && !g.aircraft_range(dst, src) {
            return u64::from(
                self.g
                    .command_cost(output, STR_ERROR_AIRCRAFT_NOT_ENOUGH_RANGE),
            );
        }
        if (action == CloneOptions::CO_COPY || list.is_null()) && (g.0.list_capacity)() == 0 {
            return u64::from(
                self.g
                    .command_cost(output, STR_ERROR_NO_MORE_SPACE_FOR_ORDERS),
            );
        }
        if execute {
            self.delete_vehicle_orders(
                dst,
                action == CloneOptions::CO_COPY,
                g.count(dst) != g.count(src),
            );
            if action == CloneOptions::CO_SHARE {
                write_field!(g.vehicle(dst), orders, g.orders(src));
                self.add_shared(dst, src);
                g.invalidate_order(dst, ((-1i64) as u64) as i32);
                g.invalidate_order(src, ((-2i64) as u64) as i32);
                g.invalidate_vehicle_list(dst);
            } else {
                // Original copies the source before deleting the kept empty shell.
                // Rust temporary storage is a scalar copy, never a live mirror.
                let mut copied = Vec::new();
                let source = g.orders(src);
                if !source.is_null() {
                    let vector = g.vector(source);
                    for i in 0..read_field!(vector, length) {
                        copied.push(load(slot(vector, i)));
                    }
                }
                let old = g.orders(dst);
                if !old.is_null() {
                    g.delete_list(old);
                }
                let list = self.new_list(dst, None, ptr::null_mut());
                reenter!(openttd_rust_order_vector_assign(
                    g.vector(list),
                    copied.as_ptr(),
                    copied.len()
                ));
                // Initialize after copied orders, retaining original total calculation.
                self.initialize_list(list, dst);
                write_field!(g.vehicle(dst), orders, list);
                g.invalidate_order(dst, ((-1i64) as u64) as i32);
                g.invalidate_vehicle_list(dst);
            }
        }
        1
    }
    fn restore_backup(&self, backup: *mut c_void, v: *mut c_void) {
        let g = self.g;
        let state = g.backup(backup);
        let clone = read_field!(state, clone);
        if !clone.is_null() {
            g.share_command(v, clone);
        } else if read_field!(g.backup_vector(backup), length) > 0 && (g.0.list_capacity)() != 0 {
            let list = (g.0.allocate_list)();
            // SAFETY: The list and backup vectors are distinct live owners.
            unsafe {
                (*g.vector(list)).transfer(g.backup_vector(backup));
            }
            self.initialize_list(list, v);
            write_field!(g.vehicle(v), orders, list);
            g.invalidate_station_list();
        }
        if g.unique_backup_name(backup) == 0 {
            g.clear_backup_name(backup);
        }
        reenter!(openttd_rust_consist_copy(
            g.consist(v),
            g.backup_consist(backup)
        ));
        g.restore_backup_metadata(v, backup);
        g.update_real(v);
        let vs = g.consist(v);
        if read_field!(vs, implicit) >= g.count(v) {
            write_field!(vs, implicit, read_field!(vs, real));
        }
        g.group_command(v, read_field!(state, group));
    }
    fn capture_backup(&self, v: *mut c_void, source: *const c_void, user: u32) -> u64 {
        let g = self.g;
        g.captured(v, source.cast_mut(), user);
        0
    }
    fn backup_create(&self, v: *mut c_void, a: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            if read_field!(g.backup(ob), user) == a as u32 {
                g.delete_backup(ob);
            }
            ob = self.g.next_backup(&mut cursor);
        }
        if (g.0.backup_capacity)() != 0 {
            g.create_backup(v, a as u32);
        }
        0
    }
    fn backup_restore(&self, v: *mut c_void, a: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            if native_vehicle(v, g.0.base).tile_raw() == read_field!(state, tile)
                && read_field!(state, user) == a as u32
            {
                self.restore_backup(ob, v);
                g.delete_backup(ob);
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
    fn backup_reset(&self, a: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            if read_field!(g.backup(ob), user) == a as u32 {
                (g.0.clear_backup_post)(a as u32);
                return 0;
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
    fn backup_clear_user(&self, a: u64, b: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            {
                if read_field!(state, user) == b as u32
                    && (read_field!(state, tile) == a as u32 || a as u32 == u32::MAX)
                {
                    g.delete_backup(ob);
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
    /// `OrderBackup::Reset(t, from_gui)`.
    fn backup_tile_clear(&self, t: crate::tile_type::TileIndex, from_gui: bool) {
        let g = self.g;
        /* The user has CLIENT_ID_SERVER as default when network play is not active,
         * but compiled it. A network client has its own variable for the unique
         * client/user identifier. Finally if networking isn't compiled in the
         * default is just plain and simple: 0. */
        let user = if (g.0.networking)() != 0 && !(g.0.network_server)() {
            (g.0.network_client)() as u32
        } else {
            (g.0.server_client)() as u32
        };
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            let tile = || crate::tile_type::TileIndex(read_field!(state, tile));
            /* If this is a GUI action, and it's not a backup of us, ignore it.
             * If it's not for our chosen tile either, ignore it. */
            if (!from_gui || read_field!(state, user) == user)
                && (t == crate::tile_type::TileIndex::INVALID || t == tile())
            {
                if from_gui {
                    /* We need to circumvent the "prevention" from this command being executed
                     * while the game is paused, so use the internal method. Nor do we want
                     * this command to get its cost estimated when shift is pressed. */
                    (g.0.clear_backup_gui)(tile().0, user);
                } else {
                    /* The command came from the game logic, i.e. the clearing of a tile.
                     * In that case we have no need to actually sync this, just do it. */
                    g.delete_backup(ob);
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
    }
    fn backup_group_clear(&self, a: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            {
                if read_field!(state, group) == a as u16 {
                    write_field!(state, group, (g.0.default_group)() as u16);
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
    fn backup_vehicle_clear(&self, v: *mut c_void) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            {
                if read_field!(state, clone) == v {
                    let first = g.first(v);
                    let clone = if first == v { g.next(v) } else { first };
                    write_field!(state, clone, clone);
                    if clone.is_null() {
                        g.delete_backup(ob);
                    }
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
    fn backup_destination_clear(&self, a: u64, b: u64, c: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            {
                let length = read_field!(g.backup_vector(ob), length);
                for i in 0..length {
                    let order = g.backed_order(ob, i);
                    let mut kind = order.kind();
                    if kind == OT_GOTO_DEPOT && order.action() & 2 != 0 {
                        continue;
                    }
                    let hangar = crate::station_map::is_hangar_tile_raw(
                        read_field!(g.backup(ob), tile),
                        crate::newgrf_airport::runtime(),
                    );
                    if kind == OT_GOTO_DEPOT && c != 0 && !hangar {
                        continue;
                    }
                    if kind == OT_IMPLICIT || (hangar && kind == OT_GOTO_DEPOT && c == 0) {
                        kind = OT_GOTO_STATION;
                    }
                    if u64::from(kind) == a && u64::from(order.destination) == b {
                        g.delete_backup(ob);
                        break;
                    }
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
}

impl Control {
    fn remove_destination(&self, kind: u8, destination: u16, hangar: bool) {
        let g = self.g;
        let mut v = self.g.next_vehicle(0);
        while !v.is_null() {
            let next = (g.vehicle_id(v) as u32).wrapping_add(1);
            let aircraft = g.vehicle_type(v) == u64::from(VEH_AIRCRAFT);
            let mut current = g.current(v);
            let current_kind = if aircraft && current.kind() == OT_GOTO_DEPOT && !hangar {
                OT_GOTO_STATION
            } else {
                current.kind()
            };
            if current_kind == kind && (!hangar || aircraft) && current.destination == destination {
                current.kind = OT_DUMMY;
                current.flags = 0;
                g.put_current(v, current);
                crate::window_func::set_window_dirty(
                    crate::window_type::WindowClass::WC_VEHICLE_VIEW,
                    crate::window_type::WindowNumber::from(
                        native_vehicle(v, g.0.base).index(g.0.base.newgrf()),
                    ),
                );
            }
            if !g.orders(v).is_null() {
                let mut index = 0u8;
                while index < g.count(v) {
                    let mut next = index.wrapping_add(1);
                    let list = g.orders(v);
                    let mut order = g.order(list, index).unwrap();
                    let mut ty = order.kind();
                    if !(ty == OT_GOTO_DEPOT && (order.action() & 2 != 0 || (hangar && !aircraft)))
                    {
                        if ty == OT_IMPLICIT || (aircraft && ty == OT_GOTO_DEPOT && !hangar) {
                            ty = OT_GOTO_STATION;
                        }
                        if ty == kind && order.destination == destination {
                            if order.kind() == OT_IMPLICIT {
                                self.delete(v, index);
                                next = index;
                            } else {
                                let state = g.list(list);
                                write_field!(
                                    state,
                                    total,
                                    read_field!(state, total).wrapping_sub(i32::from(order.wait))
                                );
                                if order.wait_timetabled() {
                                    write_field!(
                                        state,
                                        timetable,
                                        read_field!(state, timetable)
                                            .wrapping_sub(i32::from(order.timetabled_wait()))
                                    );
                                    order.set_wait_timetabled(false);
                                }
                                order.wait = 0;
                                let travel = order.travel_timetabled();
                                order.kind = OT_DUMMY;
                                order.flags = 0;
                                order.set_travel_timetabled(travel);
                                g.put_order(list, index, order);
                                let mut shared = g.first(v);
                                while !shared.is_null() {
                                    let data = u16::from(index) | (u16::from(u8::MAX) << 8);
                                    g.invalidate_order(shared, i32::from(data));
                                    g.invalidate_order(shared, i32::from(data));
                                    shared = g.next(shared);
                                }
                            }
                        }
                    }
                    index = next;
                }
            }
            v = self.g.next_vehicle(next);
        }
        self.backup_destination_clear(u64::from(kind), u64::from(destination), u64::from(hangar));
    }
    /// `CheckOrders`. `vehicle` yields the typed record of `v`; the native entry resolves
    /// its pointer only once the review setting is enabled.
    fn check_orders(&self, vehicle: impl FnOnce() -> VehicleRef) {
        let g = self.g;
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        let review = unsafe { g.shared().settings().order_review_system.read() };
        // Does the user wants us to check things?
        if review == 0 {
            return;
        }
        let record = vehicle();
        let v = record.shell();
        // Do nothing for crashed vehicles
        if record
            .vehstatus()
            .test(crate::vehicle_base::VehState::Crashed)
        {
            return;
        }
        // Do nothing for stopped vehicles if setting is '1'
        if review == 1
            && record
                .vehstatus()
                .test(crate::vehicle_base::VehState::Stopped)
        {
            return;
        }
        // do nothing we we're not the first vehicle in a share-chain
        if g.first(v) != v {
            return;
        }
        // Only check every 20 days, so that we don't flood the message log
        if g.local_owner(v) == 0 || !record.day_counter().is_multiple_of(20) {
            return;
        }
        let mut message = 0u32;
        let mut stations = 0i32;
        let list = g.orders(v);
        if !list.is_null() {
            for i in 0..g.stored_count(list) {
                let order = g.stored_order(list, i);
                if order.kind() == OT_DUMMY {
                    message = 1;
                    break;
                }
                if order.kind() == OT_GOTO_STATION {
                    stations = stations.wrapping_add(1);
                    if order_can_use_station(v, order.destination) == 0 {
                        message = 2;
                    } else if record.vehicle_type()
                        == crate::vehicle_type::VehicleType::VEH_AIRCRAFT
                        && (g.0.base.engine)(record).aircraft_subtype()
                            & crate::engine_type::AIR_FAST
                            != 0
                        && u64::from(
                            (crate::newgrf_airport::runtime().fta_flags)(
                                crate::newgrf_airport::station_fta(
                                    crate::newgrf_airport::station_by_id(
                                        order.destination,
                                        crate::newgrf_airport::runtime(),
                                    ),
                                    crate::newgrf_airport::runtime(),
                                ),
                            ) & crate::airport::FTA_SHORT_STRIP
                                != 0,
                        ) != 0
                        && (g.0.no_jet_crash)() == 0
                        && message == 0
                    {
                        message = 3;
                    }
                }
            }
        }
        if g.count(v) > 1
            && g.stored_order(list, 0)
                .equals(&g.stored_order(list, g.stored_count(list) - 1))
        {
            message = 4;
        }
        if stations < 2 && message == 0 {
            message = 5;
        }
        g.debug_list(v);
        if message != 0 {
            g.order_news(v, message);
        }
    }
}

impl Control {
    fn begin_loading_orders(&self, v: *mut c_void) {
        let g = self.g;
        let current = g.current(v);
        let record = native_vehicle(v, g.0.base);
        let last = record.last_station_visited_raw();
        if current.kind() == OT_GOTO_STATION && current.destination == last {
            self.delete_implicit(v);

            /* Now both order indices point to the destination station, and we can start loading */
            let mut current = g.current(v);
            current.kind = OT_LOADING;
            g.put_current(v, current);
            g.update_timetable(v, true);

            /* Furthermore add the Non Stop flag to mark that this station
             * is the actual destination of the vehicle, which is (for example)
             * necessary to be known for HandleTrainLoading to determine
             * whether the train is lost or not; not marking a train lost
             * that arrives at random stations is bad. */
            let mut current = g.current(v);
            current.kind = (current.kind & !0xc0) | 0xc0;
            g.put_current(v, current);
            return;
        }
        /* We weren't scheduled to stop here. Insert an implicit order
         * to show that we are stopping here.
         * While only groundvehicles have implicit orders, e.g. aircraft might still enter
         * the 'wrong' terminal when skipping orders etc. */
        let state = g.consist(v);
        let implicit = read_field!(state, implicit);
        let in_list = g.vehicle_order(v, implicit);
        if g.vehicle_type(v) < u64::from(VEH_SHIP)
            && !in_list.is_some_and(|o| o.kind() == OT_IMPLICIT && o.destination == last)
        {
            let suppress = suppress_implicit(record);
            /* Do not create consecutive duplicates of implicit orders */
            let previous = if implicit > 0 {
                g.vehicle_order(v, implicit - 1)
            } else if g.count(v) > 1 {
                g.vehicle_order(v, g.count(v) - 1)
            } else {
                None
            };
            if !previous.is_some_and(|o| {
                matches!(o.kind(), OT_GOTO_STATION | OT_IMPLICIT) && o.destination == last
            }) {
                /* Prefer deleting implicit orders instead of inserting new ones,
                 * so test whether the right order follows later. In case of only
                 * implicit orders treat the last order in the list like an
                 * explicit one, except if the overall number of orders surpasses
                 * IMPLICIT_ORDER_ONLY_CAP. */
                let mut target = i32::from(implicit);
                let mut found = false;
                while target != i32::from(read_field!(state, real)) || g.manual(v) == 0 {
                    let Some(order) = g.vehicle_order(v, target as u8) else {
                        break; // No orders.
                    };
                    if order.kind() == OT_IMPLICIT && order.destination == last {
                        found = true;
                        break;
                    }
                    target = target.wrapping_add(1);
                    if target >= i32::from(g.count(v)) {
                        if g.manual(v) == 0 && g.count(v) < 32 {
                            break;
                        }
                        target = 0;
                    }
                    if target == i32::from(read_field!(state, implicit)) {
                        break; // Avoid infinite loop.
                    }
                }
                if found {
                    if suppress {
                        /* Skip to the found order */
                        write_field!(state, implicit, target as u8);
                        g.invalidate_order(v, 0);
                    } else {
                        /* Delete all implicit orders up to the station we just reached */
                        loop {
                            let order = g.vehicle_order(v, read_field!(state, implicit)).unwrap();
                            if order.kind() == OT_IMPLICIT && order.destination == last {
                                break;
                            }
                            if order.kind() == OT_IMPLICIT {
                                self.delete(v, read_field!(state, implicit));
                            } else {
                                /* Skip non-implicit orders, e.g. service-orders */
                                write_field!(
                                    state,
                                    implicit,
                                    read_field!(state, implicit).wrapping_add(1)
                                );
                            }

                            /* Wrapped around. */
                            if g.vehicle_order(v, read_field!(state, implicit)).is_none() {
                                write_field!(state, implicit, 0);
                            }
                        }
                    }
                } else if !suppress
                    && (if g.orders(v).is_null() {
                        (g.0.list_capacity)() != 0
                    } else {
                        g.count(v) < 254
                    })
                {
                    /* Insert new implicit order */
                    let order = Order {
                        kind: OT_IMPLICIT,
                        flags: 0,
                        destination: last,
                        refit: 0xfe,
                        wait: 0,
                        travel: 0,
                        speed: u16::MAX,
                    };
                    self.insert(v, order, read_field!(state, implicit));
                    if read_field!(state, implicit) > 0 {
                        write_field!(
                            state,
                            implicit,
                            read_field!(state, implicit).wrapping_sub(1)
                        );
                    }

                    /* InsertOrder disabled creation of implicit orders for all vehicles with the same implicit order.
                     * Reenable it for this vehicle */
                    assign_suppress_implicit(record, false);
                }
            }
        }
        let mut current = g.current(v);
        current.kind = OT_LOADING;
        current.flags = 0;
        g.put_current(v, current);
    }
}

/// `PreviousOrderIsUnbunching`: check if the previous order is a depot unbunching order, over
/// the vehicle's resolved order list and its consist.
fn previous_order_is_unbunching(orders: &OrderAccess, consist: *mut Consist) -> bool {
    let count = orders.count();
    let implicit = read_field!(consist, implicit);
    /* If we are headed for the first order, we must wrap around back to the last order. */
    let index = if (implicit >= count && count == 0) || implicit == 0 {
        count.wrapping_sub(1)
    } else {
        implicit.wrapping_sub(1)
    };
    let Some(previous_order) = orders.order_ptr(index) else {
        return false;
    };
    // SAFETY: `order_ptr` names one of the list's live, initialised orders.
    let previous_order = unsafe { OrderView::of(previous_order) };
    if !previous_order.is_type(crate::order_type::OrderType::OT_GOTO_DEPOT) {
        return false;
    }
    previous_order
        .depot_action_flags()
        .test(crate::order_type::OrderDepotActionFlag::Unbunch)
}

/// `BaseConsist::ResetDepotUnbunching` on the consist owner of a live vehicle record.
pub(crate) fn reset_unbunching(v: VehicleRef) {
    let consist = read_field!(v.0.as_ptr(), consist);
    // SAFETY: The consist owner the live record stores.
    unsafe { reset_depot_unbunching(consist) };
}

/// Direct access to the canonical orders owner for route prediction.
/// Handles are live for the synchronous entry; no owner borrow crosses a leaf.
#[derive(Clone, Copy)]
pub(crate) struct VisitOrders(Game);
impl VisitOrders {
    pub(crate) fn new(leaves: &'static Leaves) -> Self {
        Self(Game(leaves))
    }
    fn shell(v: VehicleRef) -> *mut c_void {
        v.shell().cast()
    }
    /// `v->orders`, the native `OrderList` shell or null.
    pub(crate) fn list(self, v: VehicleRef) -> *mut c_void {
        self.0.orders(Self::shell(v))
    }
    /// `OrderList::GetNumOrders()`.
    pub(crate) fn count(self, list: *mut c_void) -> VehicleOrderID {
        VehicleOrderID(self.0.size(list))
    }
    /// `orders[index]` for an index below `count`, read in place.
    pub(crate) fn order(self, list: *mut c_void, index: VehicleOrderID) -> OrderView<'static> {
        // SAFETY: An index below `count` names a live stored order, which no caller of a
        // view changes while it reads it.
        unsafe { OrderView::of(slot(self.0.vector(list), usize::from(index.0))) }
    }
    /// Full `OrderList::GetOrders()` span length, without `VehicleOrderID` narrowing.
    pub(crate) fn stored_count(self, list: *mut c_void) -> usize {
        self.0.stored_count(list)
    }
    /// Full stored-span entry, with `index` below `stored_count`.
    pub(crate) fn stored_order(self, list: *mut c_void, index: usize) -> Order {
        self.0.stored_order(list, index)
    }
    /// `OrderList::GetNext`.
    pub(crate) fn next(self, list: *mut c_void, index: VehicleOrderID) -> VehicleOrderID {
        VehicleOrderID(self.0.next_index(list, index.0))
    }
    /// `OrderList::GetNextDecisionNode`.
    pub(crate) fn decision(
        self,
        list: *mut c_void,
        next: VehicleOrderID,
        hops: u32,
    ) -> VehicleOrderID {
        VehicleOrderID(self.0.decision(list, next.0, hops))
    }
    /// `OrderList::GetTotalDuration()`.
    pub(crate) fn total(self, list: *mut c_void) -> i32 {
        read_field!(self.0.list(list), total)
    }
    /// `v->current_order`.
    pub(crate) fn current(self, v: VehicleRef) -> Order {
        self.0.current(Self::shell(v))
    }
    /// `v->GetOrder(index)`: null without a list or beyond its orders, read in place.
    pub(crate) fn get(self, v: VehicleRef, index: VehicleOrderID) -> Option<OrderView<'static>> {
        let list = self.list(v);
        (!list.is_null() && index < self.count(list)).then(|| self.order(list, index))
    }
    /// Virtual `Vehicle::GetDisplayMaxSpeed`: the maximum speed in km-ish/h as displayed
    /// to the player.
    pub(crate) fn get_display_max_speed(self, v: VehicleRef) -> i32 {
        // SAFETY: The live shell; the leaf reads it and retains nothing. The leaf widens the
        // virtual's `int`, which the cast narrows back.
        unsafe { (self.0.0.display_speed)(Self::shell(v)) as i32 }
    }
    /// The implicit-order part of `Vehicle::BeginLoading`.
    pub(crate) fn begin_loading(self, v: VehicleRef) {
        Control { g: self.0 }.begin_loading_orders(Self::shell(v));
    }
    /// `Vehicle::DeleteUnreachedImplicitOrders`.
    pub(crate) fn delete_implicit(self, v: VehicleRef) {
        Control { g: self.0 }.delete_implicit(Self::shell(v));
    }
    /// `Vehicle::IncrementImplicitOrderIndex`.
    pub(crate) fn increment_implicit(self, v: VehicleRef) {
        self.0.increment_implicit(Self::shell(v));
    }
    /// `UpdateVehicleTimetable`.
    pub(crate) fn update_timetable(self, v: VehicleRef, travelling: bool) {
        self.0.update_timetable(Self::shell(v), travelling);
    }
    /// The unbunching round-trip measurement of `VehicleEnterDepot`.
    pub(crate) fn measure_unbunching(self, v: VehicleRef) {
        self.0.measure_unbunching(Self::shell(v));
    }
    /// `Vehicle::LeaveUnbunchingDepot`.
    pub(crate) fn leave_unbunching_depot(self, v: VehicleRef) {
        self.0.leave_unbunching(Self::shell(v));
    }
    /// `Vehicle::HasDepotOrder`: checks if a vehicle has a depot in its order list.
    /// True iff at least one order is a depot order.
    pub(crate) fn has_depot_order(self, v: VehicleRef) -> bool {
        let list = crate::vehicle_base::order_list(v.0.as_ptr());
        if list.is_null() {
            return false;
        }
        // `Vehicle::Orders()`: the list's one stored span, resolved once.
        let vector = self.0.vector(list);
        (0..read_field!(vector, length)).any(|i| {
            // SAFETY: Each index below the span's length names a live stored order.
            unsafe { OrderView::of(slot(vector, i)) }
                .is_type(crate::order_type::OrderType::OT_GOTO_DEPOT)
        })
    }
    /// `Vehicle::IsWaitingForUnbunching` after its `assert(this->IsInDepot())`, which
    /// `vehicle::is_waiting_for_unbunching` makes.
    pub(crate) fn is_waiting_for_unbunching(self, v: VehicleRef) -> bool {
        // `v->orders` and the consist owner, read in place from the live record.
        let list = crate::vehicle_base::order_list(v.0.as_ptr());
        let consist = read_field!(v.0.as_ptr(), consist);
        /* Don't bother if there are no vehicles sharing orders. */
        if list.is_null() {
            return false;
        }
        let state = self.0.list(list);
        if read_field!(state, vehicles) <= 1 {
            return false;
        }
        let orders = OrderAccess {
            list: state,
            vector: self.0.vector(list),
        };

        /* Don't do anything if there aren't enough orders. */
        if orders.count() <= 1 {
            return false;
        }

        /* Don't do anything if this is not our unbunching order. */
        if !previous_order_is_unbunching(&orders, consist) {
            return false;
        }

        read_field!(consist, next_departure) > (self.0.0.tick_counter)()
    }
    /// `v->current_order.MakeLeaveStation()`.
    pub(crate) fn make_leave_station(self, v: VehicleRef) {
        reenter!(openttd_rust_order_leave(current_order(
            self.0.vehicle(Self::shell(v))
        )));
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDOrderFields" ["probe"] => Order {
        kind => type,
        flags,
        destination,
        refit => refit_cargo,
        wait => wait_time,
        travel => travel_time,
        speed => max_speed,
    };
    "OpenTTDConsistState" ["probe"] => Consist {
        time => current_order_time,
        lateness => lateness_counter,
        start => timetable_start,
        last_departure,
        next_departure,
        round_trip => round_trip_time,
        real => real_index,
        implicit => implicit_index,
        flags => vehicle_flags,
    };
    "OpenTTDVehicleOrderState" ["probe"] => VehicleOrders {
        current,
        orders,
        next => next_shared,
        previous => previous_shared,
    };
    "OpenTTDOrderListState" ["probe"] => List {
        manual,
        vehicles,
        first => first_shared,
        timetable => timetable_duration,
        total => total_duration,
    };
    "OpenTTDOrderBackupState" ["probe"] => Backup {
        user,
        tile,
        group,
        clone,
    };
    "OpenTTDOrdersLeaves" ["probe"] => Leaves {
        vehicle,
        consist,
        list,
        vector,
        backup,
        backup_vector,
        backup_consist,
        ownerless_station,
        vehicle_type,
        tick_counter,
        primary_vehicle,
        ticks_per_second,
        unit_number,
        maximum_date,
        allocate_list,
        shared_window,
        vehicle_id,
        reliability,
        display_speed,
        age_years,
        needs_service,
        remaining_years,
        airport_tile,
        depot_tile,
        distance,
        station_location,
        destination_tile,
        aircraft_flying,
        target_airport,
        waypoint_tile,
        at_station,
        tile_station,
        ship_station_tile,
        valid_station,
        station_owner,
        station_error,
        valid_depot,
        depot_owner,
        rail_depot,
        road_depot,
        ship_depot,
        valid_waypoint,
        waypoint_facilities,
        waypoint_owner,
        list_capacity,
        next_backup,
        next_vehicle,
        aircraft_range,
        aircraft_range_square,
        backup_capacity,
        create_backup,
        networking,
        network_server,
        network_client,
        server_client,
        default_group,
        vehicle_group,
        unique_backup_name,
        local_owner,
        no_jet_crash,
        append_station,
        station_commands,
        timetable_dirty,
        invalidate_order,
        delete_order_news,
        invalidate_vehicle_list,
        close_shared_window,
        invalidate_shared_window,
        dirty_vehicle_windows,
        capture_backup_metadata,
        clear_backup_name,
        restore_backup_metadata,
        order_news,
        debug_list,
        assert_departure_range,
        delete_list,
        reverse_train,
        next_airport,
        set_destination,
        closest_depot,
        share_command,
        group_command,
        delete_backup,
        clear_backup_gui,
        clear_backup_post,
        missing_aircraft_orders,
        change_timetable_command,
        base,
    };
    "OpenTTDOrdersClosest" ["probe"] => Closest {
        tile,
        destination,
        reverse,
        found,
    };
}

/// Shared named station eligibility operation, keeping the source vehicle gate.
fn order_can_use_station(vehicle: *mut std::ffi::c_void, destination: u16) -> u64 {
    // SAFETY: The caller supplies live serialized vehicle and destination shells.
    u64::from(unsafe {
        crate::vehicle_func::can_vehicle_use_station(
            native_vehicle(vehicle, &*crate::newgrf_airport::runtime().vehicles),
            crate::newgrf_airport::station_by_id(destination, crate::newgrf_airport::runtime()),
            crate::newgrf_airport::runtime(),
        )
    })
}

/// `Vehicle::IsWaitingForUnbunching` over a live record: a vehicle sharing a two-order list
/// whose previous order is its unbunching depot order waits while its next departure is still
/// ahead, and does not wait once that tick comes or without a shared list. A
/// vehicle outside a depot fails the native depot assertion first.
#[cfg(test)]
mod unbunching_tests {
    use super::{
        Consist, List, Order, Vector, openttd_rust_consist_new, openttd_rust_order_vector_insert,
        openttd_rust_order_vector_new, openttd_rust_orderlist_new, openttd_rust_vehicle_orders_new,
    };
    use crate::order_type::OT_GOTO_DEPOT;
    use crate::vehicle::VehicleRef;
    use crate::vehicle_type::VehicleType;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

    static LIST: AtomicPtr<List> = AtomicPtr::new(std::ptr::null_mut());
    static VECTOR: AtomicPtr<Vector> = AtomicPtr::new(std::ptr::null_mut());
    static FAILED: AtomicU32 = AtomicU32::new(0);
    unsafe extern "C" fn list(_: *mut c_void) -> *mut List {
        LIST.load(Ordering::Relaxed)
    }
    unsafe extern "C" fn vector(_: *mut c_void) -> *mut Vector {
        VECTOR.load(Ordering::Relaxed)
    }
    extern "C" fn tick_counter() -> u64 {
        100
    }
    unsafe extern "C" fn construct(data: *mut c_void, count: usize) {
        for i in 0..count {
            // SAFETY: The vector's storage for `count` orders.
            unsafe { data.cast::<Order>().add(i).write(order(0, 0)) };
        }
    }
    unsafe extern "C" fn destroy(_: *mut c_void, _: usize) {}
    extern "C" fn record_failure(condition: bool) {
        if !condition {
            FAILED.fetch_add(1, Ordering::Relaxed);
        }
    }
    fn order(kind: u8, flags: u8) -> Order {
        Order {
            kind,
            flags,
            destination: 0,
            refit: 0,
            wait: 0,
            travel: 0,
            speed: 0,
        }
    }

    #[test]
    fn is_waiting_for_unbunching_follows_the_shared_list_and_departure_time() {
        if crate::services_assert::isolate_test_check(
            "orders::unbunching_tests::is_waiting_for_unbunching_follows_the_shared_list_and_departure_time",
            record_failure,
        ) {
            return;
        }
        let shared = openttd_rust_orderlist_new();
        let orders = openttd_rust_order_vector_new(construct, destroy);
        LIST.store(shared, Ordering::Relaxed);
        VECTOR.store(orders, Ordering::Relaxed);
        let leaves: &'static super::Leaves = Box::leak(Box::new(super::Leaves {
            list,
            vector,
            tick_counter,
            ..crate::test_tables::ORDERS
        }));
        let consist: *mut Consist = openttd_rust_consist_new();
        let owner = openttd_rust_vehicle_orders_new();
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            VehicleType::VEH_ROAD,
        );
        // SAFETY: The test's fresh owners, which it leaks; any non-null shell names the list.
        unsafe {
            (*owner).orders = shared.cast();
            (*record).orders = owner;
            (*record).consist = consist;
            (*shared).vehicles = 2;
            // The second order is current, so the previous one is the unbunching depot order.
            // The depot action type is bits 3-6 of `flags`.
            let unbunch = crate::order_type::OrderDepotActionFlags::from_element(
                crate::order_type::OrderDepotActionFlag::Unbunch,
            );
            openttd_rust_order_vector_insert(orders, 0, &order(OT_GOTO_DEPOT, unbunch.0 << 3));
            openttd_rust_order_vector_insert(orders, 1, &order(1, 0));
            (*consist).implicit = 1;
            (*consist).next_departure = 101;
        }
        let v = VehicleRef::of(record);
        let waiting = || crate::vehicle::is_waiting_for_unbunching(v, leaves, || true);
        assert!(waiting());
        // SAFETY: Each write below is to the test's leaked owners, with no other access live.
        unsafe { (*consist).next_departure = 100 };
        assert!(!waiting());
        // SAFETY: As above.
        unsafe {
            (*consist).next_departure = 101;
            (*shared).vehicles = 1;
        };
        assert!(!waiting());
        // SAFETY: As above.
        unsafe { (*shared).vehicles = 2 };
        assert!(waiting());
        assert_eq!(FAILED.load(Ordering::Relaxed), 0);
        // Outside a depot the assertion fails before the same tests run.
        assert!(crate::vehicle::is_waiting_for_unbunching(v, leaves, || {
            false
        }));
        assert_eq!(FAILED.load(Ordering::Relaxed), 1);
    }
}

/// `Order::Free` and `Order::MakeDummy` on a vehicle's current order write only the fields
/// `order_cmd.cpp` names: `Free` the type, flags and destination; `MakeDummy` the type and
/// flags. Every field starts with a distinct value, so a whole-order store fails.
/// The `OrderView` getters `vehicle/visits.rs` reads, on orders whose fields are all
/// distinct, against `order_base.h`'s bit layouts: the unload type is bits 0-2 of `flags`, the
/// load type bits 4-6, the wait-timetabled flag bit 3 (`wait_time > 0` for a conditional
/// order), and the skip-to order of a conditional order is the whole `flags` byte.
#[cfg(test)]
mod order_view_getter_tests {
    use super::{Order, OrderView};
    use crate::cargo_type::CargoType;
    use crate::order_type::{OrderLoadType, OrderUnloadType, VehicleOrderID};

    fn order(kind: u8, flags: u8, refit: u8, wait: u16) -> Order {
        Order {
            kind,
            flags,
            destination: 0x1234,
            refit,
            wait,
            travel: 888,
            speed: 999,
        }
    }
    fn view(o: &Order) -> OrderView<'_> {
        // SAFETY: A reference names a live, initialized order.
        unsafe { OrderView::of(o) }
    }

    #[test]
    fn a_station_order_decodes_its_flags_and_refit() {
        // OT_GOTO_STATION with non-stop bits; full load (2 << 4), wait timetabled (bit 3),
        // unload (1); refit to cargo 5.
        let o = order(0x41, 0x29, 5, 777);
        let v = view(&o);
        assert_eq!(v.unload_type(), OrderUnloadType(1));
        assert_eq!(v.load_type(), OrderLoadType(2));
        assert!(v.is_refit());
        assert!(!v.is_auto_refit());
        assert_eq!(v.refit_cargo(), CargoType(5));
        assert_eq!(v.condition_skip_to_order(), VehicleOrderID(0x29));
        assert!(v.is_wait_timetabled());
        assert_eq!(v.timetabled_wait(), 777);
        assert!(v.can_leave_with_cargo(false));
        assert_eq!(o.unload(), 1);
        assert!(o.is_refit());
        assert_eq!(o.timetabled_wait(), 777);
    }

    #[test]
    fn a_conditional_order_takes_its_wait_from_the_wait_time() {
        // OT_CONDITIONAL: bit 3 is part of the skip-to order, not the wait flag.
        let timetabled = order(7, 0x0A, 0xFD, 300);
        assert!(view(&timetabled).is_wait_timetabled());
        assert_eq!(view(&timetabled).timetabled_wait(), 300);
        assert_eq!(
            view(&timetabled).condition_skip_to_order(),
            VehicleOrderID(0x0A)
        );
        let untimed = order(7, 0x0A, 0xFD, 0);
        assert!(!view(&untimed).is_wait_timetabled());
        assert_eq!(view(&untimed).timetabled_wait(), 0);
        // The station order's bit 3 clear: not timetabled, whatever its wait time.
        assert_eq!(view(&order(1, 0x21, 0, 500)).timetabled_wait(), 0);
    }

    #[test]
    fn refit_cargo_values_decode_as_refit_and_auto_refit() {
        for (refit, is_refit, auto) in [
            (0, true, false),
            (63, true, false),
            (64, false, false),
            (0xFD, true, true),
            (0xFE, false, false),
            (0xFF, false, false),
        ] {
            let o = order(1, 0, refit, 0);
            assert_eq!(view(&o).is_refit(), is_refit, "refit cargo {refit}");
            assert_eq!(view(&o).is_auto_refit(), auto, "refit cargo {refit}");
            assert_eq!(view(&o).refit_cargo(), CargoType(refit));
        }
    }

    #[test]
    fn can_leave_with_cargo_follows_the_load_and_unload_types() {
        // (flags, has_cargo, expected): no load is 4 << 4; unload 1, transfer 2, no unload 4.
        for (flags, has_cargo, expected) in [
            (0x00, false, true),
            (0x40, false, false),
            (0x40, true, true),
            (0x41, true, false),
            (0x42, true, false),
            (0x44, true, true),
            (0x21, true, true),
        ] {
            let o = order(1, flags, 0xFE, 0);
            assert_eq!(
                view(&o).can_leave_with_cargo(has_cargo),
                expected,
                "flags {flags:#x}, has cargo {has_cargo}"
            );
        }
    }
}

#[cfg(test)]
mod current_order_write_tests {
    use super::{Order, VehicleOrders, free, make_dummy, openttd_rust_vehicle_orders_new};
    use crate::order_type::{OT_DUMMY, OT_NOTHING};
    use crate::vehicle::VehicleRef;
    use crate::vehicle_type::VehicleType;

    fn fields(o: Order) -> [u32; 7] {
        [
            o.kind.into(),
            o.flags.into(),
            o.destination.into(),
            o.refit.into(),
            o.wait.into(),
            o.travel.into(),
            o.speed.into(),
        ]
    }

    #[test]
    fn free_and_make_dummy_write_only_the_original_fields() {
        let owner: *mut VehicleOrders = openttd_rust_vehicle_orders_new();
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            VehicleType::VEH_TRAIN,
        );
        // SAFETY: The test's fresh record and orders owner.
        unsafe { (*record).orders = owner };
        let v = VehicleRef::of(record);
        let full = Order {
            kind: 0x12,
            flags: 0x34,
            destination: 0x5678,
            refit: 0x9a,
            wait: 0xbcde,
            travel: 0xf012,
            speed: 0x3456,
        };
        // SAFETY: The record's current order, written and read with no other access.
        unsafe {
            v.current_order_pointer().write(full);
            free(v);
            assert_eq!(
                fields(v.current_order_pointer().read()),
                [OT_NOTHING.into(), 0, 0, 0x9a, 0xbcde, 0xf012, 0x3456]
            );
            v.current_order_pointer().write(full);
            make_dummy(v.current_order_pointer());
            assert_eq!(
                fields(v.current_order_pointer().read()),
                [OT_DUMMY.into(), 0, 0x5678, 0x9a, 0xbcde, 0xf012, 0x3456]
            );
            crate::vehicle_base::openttd_rust_vehicle_record_delete(record);
            crate::orders::openttd_rust_vehicle_orders_delete(owner);
        }
    }
}

#[cfg(test)]
mod order_view_tests {
    use super::{Order, OrderView};
    use crate::order_type::{
        OrderDepotActionFlags, OrderDepotTypeFlags, OrderNonStopFlags, OrderType,
    };

    /// `order_base.h` packs `type` as type (bits 0-3), stop location (4-5) and
    /// non-stop flags (6-7), and `flags` as depot order type or unload type
    /// (0-2), depot action type (3-6) and load type (4-6). The expectations are
    /// those fields of distinct stored values, so a getter reading the wrong
    /// field or bits fails.
    #[test]
    fn view_getters_decode_the_original_bit_layout() {
        let order = Order {
            kind: 0b1011_0110,
            flags: 0b1101_1010,
            destination: 0x1234,
            refit: 7,
            wait: 0x2345,
            travel: 0x3456,
            speed: 0x4567,
        };
        // SAFETY: The order lives on this stack through every read.
        let view = unsafe { OrderView::of(&raw const order) };
        assert_eq!(view.kind(), OrderType(6));
        assert_eq!(view.kind_raw(), 6);
        assert_eq!(view.non_stop(), OrderNonStopFlags(2));
        assert_eq!(view.non_stop_raw(), 2);
        assert_eq!(view.depot_type(), OrderDepotTypeFlags(2));
        assert_eq!(view.depot_type_raw(), 2);
        assert_eq!(view.depot_action_flags(), OrderDepotActionFlags(11));
        assert_eq!(view.depot_action_flags_raw(), 11);
        assert_eq!(view.destination(), crate::order_type::DestinationID(0x1234));
        assert_eq!(view.destination_raw(), 0x1234);
        assert_eq!(view.max_speed(), 0x4567);
        // Load type 5 is neither `FullLoad` (2) nor `FullLoadAny` (3).
        assert!(!view.is_full_load());
        let copy = view.copied();
        assert_eq!(
            (
                copy.kind,
                copy.flags,
                copy.destination,
                copy.refit,
                copy.wait,
                copy.travel,
                copy.speed
            ),
            (0b1011_0110, 0b1101_1010, 0x1234, 7, 0x2345, 0x3456, 0x4567)
        );
    }

    #[test]
    fn view_full_load_is_load_type_two_or_three() {
        for (flags, full) in [
            (0x20, true),
            (0x30, true),
            (0x40, false),
            (0x10, false),
            (0x0f, false),
        ] {
            let order = Order {
                kind: 0,
                flags,
                destination: 0,
                refit: 0,
                wait: 0,
                travel: 0,
                speed: 0,
            };
            assert_eq!(
                // SAFETY: The order lives on this stack through the read.
                unsafe { OrderView::of(&raw const order) }.is_full_load(),
                full,
                "flags {flags:#x}"
            );
        }
    }
}

#[cfg(test)]
mod command_cost_tests {
    use super::{
        Consist, Leaves, Order, VehicleOrders, openttd_rust_command_insert,
        openttd_rust_timetable_on_time, openttd_rust_timetable_validate_bulk,
    };
    use crate::services::{CommandCost, CommandCostServices};
    use crate::station_cmd::CommandServicesTable;
    use crate::strings_type::StringID;
    use crate::table::strings::{
        STR_ERROR_CAN_T_ADD_ORDER, STR_ERROR_NO_BUOY, STR_ERROR_NO_RAIL_WAYPOINT,
        STR_ERROR_NO_ROAD_WAYPOINT, STR_ERROR_TIMETABLE_NOT_STARTED,
    };
    use crate::test_tables::{
        unreachable0, unreachable1, unreachable2, unreachable3, unreachable4,
    };
    use crate::vehicle_type::VehicleType;
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::mem::MaybeUninit;
    use std::ptr;

    /// One native `CommandCost` error constructor call.
    #[derive(Debug, PartialEq, Eq)]
    enum Constructed {
        /// `CommandCost(msg)`.
        Error(u32),
        /// `CommandCost(msg, extra_msg)`.
        ErrorExtra(u32, u32),
    }
    /// The fixture vehicle: its order owner, consist and record, reached through its shell.
    struct Fixture {
        orders: VehicleOrders,
        consist: Consist,
        record: *mut crate::vehicle_base::VehicleRecord,
    }
    thread_local! {
        static CONSTRUCTED: RefCell<Vec<Constructed>> = const { RefCell::new(Vec::new()) };
        static KIND: Cell<VehicleType> = const { Cell::new(VehicleType::TRAIN) };
    }
    extern "C" fn error(_: *mut CommandCost, msg: u32) {
        CONSTRUCTED.with(|c| c.borrow_mut().push(Constructed::Error(msg)));
    }
    extern "C" fn error_extra(_: *mut CommandCost, msg: u32, extra_msg: u32) {
        CONSTRUCTED.with(|c| c.borrow_mut().push(Constructed::ErrorExtra(msg, extra_msg)));
    }
    extern "C" fn zero(_: *mut CommandCost) {}
    extern "C" fn success(_: *const CommandCost) -> bool {
        true
    }
    extern "C" fn current_company() -> u8 {
        1
    }
    extern "C" fn company() -> *const crate::company::Services {
        const FINANCE: crate::company::FinanceServices = crate::company::FinanceServices {
            current_company,
            ..crate::test_tables::FINANCE
        };
        const COMPANY: crate::company::Services = crate::company::Services {
            finance: &FINANCE,
            ..crate::test_tables::COMPANY
        };
        &COMPANY
    }
    unsafe extern "C" fn primary_vehicle(_: *mut c_void) -> u64 {
        1
    }
    unsafe extern "C" fn vehicle_type(_: *mut c_void) -> u64 {
        u64::from(KIND.get().0)
    }
    unsafe extern "C" fn vehicle(v: *mut c_void) -> *mut VehicleOrders {
        // SAFETY: Every shell this test passes is its live `Fixture`.
        unsafe { &raw mut (*v.cast::<Fixture>()).orders }
    }
    unsafe extern "C" fn consist(v: *mut c_void) -> *mut Consist {
        // SAFETY: As `vehicle`.
        unsafe { &raw mut (*v.cast::<Fixture>()).consist }
    }
    extern "C" fn record(v: *mut c_void) -> *mut crate::vehicle_base::VehicleRecord {
        // SAFETY: As `vehicle`.
        unsafe { (*v.cast::<Fixture>()).record }
    }
    extern "C" fn valid_waypoint(_: u16) -> u64 {
        1
    }
    unsafe extern "C" fn waypoint_facilities(_: u16) -> u64 {
        0
    }

    /// Order leaves over the fixture vehicle and a recording cost table.
    fn leaves() -> &'static Leaves {
        let cost: &'static CommandCostServices = Box::leak(Box::new(CommandCostServices {
            zero,
            error,
            error_extra,
            success,
            ..crate::test_tables::COMMAND_COSTS
        }));
        let command: &'static crate::services::CommandServices =
            Box::leak(Box::new(crate::services::CommandServices {
                company,
                ..crate::test_tables::COMMANDS
            }));
        let station_commands: *const CommandServicesTable =
            Box::leak(Box::new(CommandServicesTable {
                cost,
                command,
                ..crate::test_tables::STATION_COMMANDS
            }));
        let base: &'static crate::vehicle::VehicleServices =
            Box::leak(Box::new(crate::vehicle::VehicleServices {
                record_from_shell: record,
                ..crate::test_tables::VEHICLE
            }));
        Box::leak(Box::new(Leaves {
            vehicle,
            consist,
            list: unreachable1,
            vector: unreachable1,
            backup: unreachable1,
            backup_vector: unreachable1,
            backup_consist: unreachable1,
            ownerless_station: unreachable2,
            vehicle_type,
            tick_counter: unreachable0,
            primary_vehicle,
            ticks_per_second: unreachable0,
            unit_number: unreachable1,
            maximum_date: unreachable0,
            allocate_list: unreachable0,
            shared_window: unreachable2,
            vehicle_id: unreachable1,
            reliability: unreachable1,
            display_speed: unreachable1,
            age_years: unreachable1,
            needs_service: unreachable1,
            remaining_years: unreachable1,
            airport_tile: unreachable1,
            depot_tile: unreachable1,
            distance: unreachable3,
            station_location: unreachable2,
            destination_tile: unreachable1,
            aircraft_flying: unreachable1,
            target_airport: unreachable1,
            waypoint_tile: unreachable1,
            at_station: unreachable1,
            tile_station: unreachable1,
            ship_station_tile: unreachable1,
            valid_station: unreachable1,
            station_owner: unreachable1,
            station_error: unreachable2,
            valid_depot: unreachable1,
            depot_owner: unreachable1,
            rail_depot: unreachable1,
            road_depot: unreachable1,
            ship_depot: unreachable1,
            valid_waypoint,
            waypoint_facilities,
            waypoint_owner: unreachable1,
            list_capacity: unreachable0,
            next_backup: unreachable1,
            next_vehicle: unreachable1,
            aircraft_range: unreachable1,
            aircraft_range_square: unreachable1,
            backup_capacity: unreachable0,
            create_backup: unreachable2,
            networking: unreachable0,
            network_server: unreachable0,
            network_client: unreachable0,
            server_client: unreachable0,
            default_group: unreachable0,
            vehicle_group: unreachable1,
            unique_backup_name: unreachable1,
            local_owner: unreachable1,
            no_jet_crash: unreachable0,
            append_station: unreachable2,
            station_commands,
            timetable_dirty: unreachable1,
            invalidate_order: unreachable2,
            delete_order_news: unreachable1,
            invalidate_vehicle_list: unreachable1,
            close_shared_window: unreachable2,
            invalidate_shared_window: unreachable3,
            dirty_vehicle_windows: unreachable1,
            capture_backup_metadata: unreachable2,
            clear_backup_name: unreachable1,
            restore_backup_metadata: unreachable2,
            order_news: unreachable2,
            debug_list: unreachable1,
            assert_departure_range: unreachable2,
            delete_list: unreachable1,
            reverse_train: unreachable1,
            next_airport: unreachable1,
            set_destination: unreachable2,
            closest_depot: unreachable2,
            share_command: unreachable2,
            group_command: unreachable2,
            delete_backup: unreachable1,
            clear_backup_gui: unreachable2,
            clear_backup_post: unreachable1,
            missing_aircraft_orders: unreachable1,
            change_timetable_command: unreachable4,
            base,
        }))
    }

    /// Order entries write the original's `CommandCost` at their returns: `CMD_ERROR` as
    /// `CommandCost(INVALID_STRING_ID)` (`CmdBulkChangeTimetable`), a single string
    /// (`CmdSetVehicleOnTime`) and `CmdInsertOrder`'s three waypoint returns, whose message
    /// and extra message take the original's positions.
    #[test]
    fn order_errors_construct_the_original_command_costs() {
        let leaves = leaves();
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            VehicleType::TRAIN,
        );
        // SAFETY: The fresh record is live and exclusively this test's.
        crate::vehicle::VehicleRef(unsafe { ptr::NonNull::new_unchecked(record) })
            .set_owner(crate::company_type::CompanyID(1));
        let mut fixture = Fixture {
            orders: VehicleOrders {
                current: MaybeUninit::zeroed(),
                orders: ptr::NonNull::<u8>::dangling().as_ptr().cast(),
                next: ptr::null_mut(),
                previous: ptr::null_mut(),
            },
            consist: Consist {
                time: 0,
                lateness: 0,
                start: 0,
                last_departure: 0,
                next_departure: 0,
                round_trip: 0,
                real: 0,
                implicit: 0,
                flags: 0,
            },
            record,
        };
        let v = (&raw mut fixture).cast::<c_void>();
        let output = ptr::NonNull::<u8>::dangling().as_ptr().cast();
        let waypoint = Order {
            kind: crate::order_type::OT_GOTO_WAYPOINT,
            flags: 0,
            destination: 0,
            refit: 0xfe,
            wait: 0,
            travel: 0,
            speed: u16::MAX,
        };
        CONSTRUCTED.with(|c| c.borrow_mut().clear());
        // SAFETY: The null vehicle fails the first check; the output is never dereferenced.
        let bulk =
            unsafe { openttd_rust_timetable_validate_bulk(ptr::null_mut(), 0, output, leaves) };
        // SAFETY: The fixture vehicle has orders and a consist whose timetable has not started.
        let on_time = unsafe { openttd_rust_timetable_on_time(v, 0, 0, output, leaves) };
        let mut inserted = Vec::new();
        for kind in [VehicleType::TRAIN, VehicleType::ROAD, VehicleType::SHIP] {
            KIND.set(kind);
            // SAFETY: The fixture vehicle, its record and the order live through the call.
            inserted.push(unsafe {
                openttd_rust_command_insert(v, 0, 0, &raw const waypoint, output, leaves)
            });
        }
        // SAFETY: The test allocated the record and nothing else retains it.
        drop(unsafe { Box::from_raw(record) });
        assert_eq!((bulk, on_time, inserted), (0, 0, vec![0, 0, 0]));
        assert_eq!(
            CONSTRUCTED.with(RefCell::take),
            [
                Constructed::Error(StringID::INVALID_STRING_ID.0),
                Constructed::Error(STR_ERROR_TIMETABLE_NOT_STARTED.0),
                Constructed::ErrorExtra(STR_ERROR_CAN_T_ADD_ORDER.0, STR_ERROR_NO_RAIL_WAYPOINT.0),
                Constructed::ErrorExtra(STR_ERROR_CAN_T_ADD_ORDER.0, STR_ERROR_NO_ROAD_WAYPOINT.0),
                Constructed::ErrorExtra(STR_ERROR_CAN_T_ADD_ORDER.0, STR_ERROR_NO_BUOY.0),
            ]
        );
    }
}
