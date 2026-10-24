/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Orders own their contiguous storage, lifecycle and timetable state. C++
//! placement-constructs typed objects before exposing facade aliases. All accesses
//! to aliased fields are raw, field-sized reads/writes: no Rust references to live
//! C++ Order objects survive external services, mutation or reentry. Panics abort.
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
use crate::order_type::{
    OLFB_FULL_LOAD, OLFB_FULL_LOAD_ANY, OLFB_NO_LOAD, OUFB_TRANSFER, OUFB_UNLOAD,
};
use crate::vehicle::{VehicleRef, VehicleServices, maintenance};
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
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            unsafe { (*self.vector).length as u8 }
        }
    }
    fn manual(&self) -> u8 {
        if self.list.is_null() {
            0
        } else {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            unsafe { (*self.list).manual }
        }
    }
    fn order(&self, index: u8) -> Option<Order> {
        if index >= self.count() {
            None
        } else {
            // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
            unsafe { Some((*self.vector).data().add(usize::from(index)).read()) }
        }
    }
    fn kind(&self, index: usize) -> u8 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        (unsafe { (*(*self.vector).data().add(index)).kind }) & 15
    }
    fn update_real(&self, state: *mut Consist) {
        let count = self.count();
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if unsafe { (*state).real >= count } {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).real = 0;
            }
        }
        if self.manual() > 0 {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            while self.kind(usize::from(unsafe { (*state).real })) == 8 {
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*state).real = (*state).real.wrapping_add(1);
                    if (*state).real >= count {
                        (*state).real = 0;
                    }
                }
            }
        } else {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).real = 0;
            }
        }
    }
    fn valid(&self) -> bool {
        !self.vector.is_null()
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            && (0..unsafe { (*self.vector).length })
                .any(|index| matches!(self.kind(index), 1 | 2 | 6))
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
        // SAFETY: Indices are within the live vector's length/capacity; Order is Copy and no reference survives.
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_reset(p: *mut Consist) {
    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
    unsafe {
        (*p).last_departure = 0;
        (*p).next_departure = 0;
        (*p).round_trip = 0;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_copy(p: *mut Consist, src: *const Consist) {
    if ptr::eq(p, src) {
        return;
    }
    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
    unsafe {
        (*p).time = (*src).time;
        (*p).lateness = (*src).lateness;
        (*p).start = (*src).start;
        (*p).real = (*src).real;
        (*p).implicit = (*src).implicit;
        (*p).flags |= (*src).flags & ((1 << 3) | (1 << 4) | (1 << 5) | (1 << 8));
        (*p).flags = ((*p).flags & !(1 << 9)) | ((*src).flags & (1 << 9));
    }
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
    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
    unsafe { (*p).length }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_data(p: *const Vector) -> *mut Order {
    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
    unsafe { (*p).data() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_resize(p: *mut Vector, size: usize) {
    // SAFETY: Indices are within the live vector's length/capacity; Order is Copy and no reference survives.
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
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let order = unsafe { src.read() };
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    let v = unsafe { &mut *p };
    let index = index.min(v.length);
    // SAFETY: Indices are within the live vector's length/capacity; Order is Copy and no reference survives.
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
    // SAFETY: Indices are within the live vector's length/capacity; Order is Copy and no reference survives.
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
    // SAFETY: Indices are within the live vector's length/capacity; Order is Copy and no reference survives.
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
    // SAFETY: Indices are within the live vector's length/capacity; Order is Copy and no reference survives.
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
/// precondition. These safety contracts do not change the C ABI or table layout.
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
    pub first_vehicle: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    pub last_station: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub ownerless_station: unsafe extern "C" fn(station: u16, if_valid: u8) -> u64,
    pub vehicle_type: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub vehicle_status: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub tick_counter: extern "C" fn() -> u64,
    pub primary_vehicle: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub vehicle_ownership: unsafe extern "C" fn(ctx: *mut c_void, result: *mut c_void) -> u64,
    pub ticks_per_second: extern "C" fn() -> u64,
    pub unit_number: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub economy_date: extern "C" fn() -> u64,
    pub economy_fraction: extern "C" fn() -> u64,
    pub maximum_date: extern "C" fn() -> u64,
    pub timetable_year_limit: extern "C" fn() -> u64,
    pub stopped_or_crashed: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub allocate_list: extern "C" fn() -> *mut c_void,
    pub suppress_implicit: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub shared_window: unsafe extern "C" fn(ctx: *mut c_void, first: *mut c_void) -> u64,
    pub vehicle_id: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub reliability: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub display_speed: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub age_years: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub needs_service: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub remaining_years: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub airport_tile: unsafe extern "C" fn(destination: u16) -> u64,
    pub base_station_tile: unsafe extern "C" fn(destination: u16) -> u64,
    pub station_tile: unsafe extern "C" fn(destination: u16) -> u64,
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
    pub owner_check: unsafe extern "C" fn(owner: u8, result: *mut c_void) -> u64,
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
    pub bus: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub backup_capacity: extern "C" fn() -> u64,
    pub create_backup: unsafe extern "C" fn(ctx: *mut c_void, user: u32) -> *mut c_void,
    pub networking: extern "C" fn() -> u64,
    pub network_server: extern "C" fn() -> u64,
    pub network_client: extern "C" fn() -> u64,
    pub server_client: extern "C" fn() -> u64,
    pub default_group: extern "C" fn() -> u64,
    pub vehicle_tile: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub vehicle_group: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub unique_backup_name: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub review_setting: extern "C" fn() -> u64,
    pub local_owner: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub fast_aircraft: unsafe extern "C" fn(ctx: *mut c_void) -> u64,
    pub no_jet_crash: extern "C" fn() -> u64,
    pub append_station: unsafe extern "C" fn(ctx: *mut c_void, station: u16),
    pub invalidate_station_list: extern "C" fn(),
    pub command_error: unsafe extern "C" fn(ctx: *mut c_void, error: u32, detail: u32),
    pub timetable_dirty: unsafe extern "C" fn(ctx: *mut c_void),
    pub invalidate_order: unsafe extern "C" fn(ctx: *mut c_void, data: i32),
    pub delete_order_news: unsafe extern "C" fn(ctx: *mut c_void),
    pub suppress_implicit_write: unsafe extern "C" fn(ctx: *mut c_void, suppress: u8),
    pub invalidate_vehicle_list: unsafe extern "C" fn(ctx: *mut c_void),
    pub close_shared_window: unsafe extern "C" fn(ctx: *mut c_void, window: u32),
    pub invalidate_shared_window: unsafe extern "C" fn(ctx: *mut c_void, window: u32, data: i32),
    pub last_station_write: unsafe extern "C" fn(ctx: *mut c_void, station: u16),
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
fn native_vehicle(v: *mut c_void) -> VehicleRef {
    // SAFETY: Callers inherit their entry's live, non-null Vehicle contract.
    VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(v.cast()) })
}

impl Game {
    fn maintenance(self) -> &'static maintenance::Services {
        // SAFETY: The immutable base table supplies its process-wide maintenance table.
        unsafe { &*self.0.base.maintenance }
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
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let shell = unsafe { (*vehicle).orders };
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
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.vehicle(v)).orders }
    }
    fn next(self, v: *mut c_void) -> *mut c_void {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.vehicle(v)).next }
    }
    fn previous(self, v: *mut c_void) -> *mut c_void {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.vehicle(v)).previous }
    }
    fn first(self, v: *mut c_void) -> *mut c_void {
        let list = self.orders(v);
        if list.is_null() {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe { (self.0.first_vehicle)(v) }
        } else {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            unsafe { (*self.list(list)).first }
        }
    }
    fn size(self, list: *mut c_void) -> u8 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.vector(list)).length as u8 }
    }
    fn count(self, v: *mut c_void) -> u8 {
        let list = self.orders(v);
        if list.is_null() { 0 } else { self.size(list) }
    }
    fn stored_count(self, list: *mut c_void) -> usize {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.vector(list)).length }
    }
    fn stored_order(self, list: *mut c_void, index: usize) -> Order {
        // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
        unsafe { (*self.vector(list)).data().add(index).read() }
    }
    fn put_stored(self, list: *mut c_void, index: usize, order: Order) {
        // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
        unsafe {
            (*self.vector(list)).data().add(index).write(order);
        }
    }
    fn order(self, list: *mut c_void, index: u8) -> Option<Order> {
        if index >= self.size(list) {
            None
        } else {
            // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
            unsafe { Some((*self.vector(list)).data().add(usize::from(index)).read()) }
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
        // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
        unsafe {
            (*self.vector(list))
                .data()
                .add(usize::from(index))
                .write(order);
        }
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
        if order.kind() == 7 {
            if order.variable() != 5 {
                return next;
            }
            return self.decision(list, order.flags, hops.wrapping_add(1));
        }
        if order.kind() == 2 {
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
    fn stopping(
        self,
        list: *mut c_void,
        v: *mut c_void,
        first: u8,
        mut hops: u32,
        output: *mut c_void,
    ) {
        let mut next = first;
        if first == u8::MAX {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            next = unsafe { (*self.consist(v)).implicit };
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
            while next != u8::MAX && self.order(list, next).unwrap().kind() == 7 {
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
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            let last = (unsafe { (self.0.last_station)(v) }) as u16;
            if (order.kind() == 1 || order.kind() == 8)
                && order.destination == last
                && matches!(order.unload(), 1 | 2)
            {
                return;
            }
            if order.kind() == 2 || order.destination == last {
                continue;
            }
            // SAFETY: Output is the entry's live native station vector; no borrow crosses its mutation.
            unsafe {
                (self.0.append_station)(output, order.destination);
            }
            return;
        }
    }
}
impl Order {
    pub(crate) fn kind(&self) -> u8 {
        self.kind & 15
    }
    pub(crate) fn non_stop(self) -> u8 {
        self.kind >> 6
    }
    pub(crate) fn load(self) -> u8 {
        (self.flags >> 4) & 7
    }
    pub(crate) fn unload(self) -> u8 {
        self.flags & 7
    }
    pub(crate) fn action(&self) -> u8 {
        (self.flags >> 3) & 15
    }
    pub(crate) fn depot_type(self) -> u8 {
        self.flags & 7
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
    pub(crate) fn is_refit(self) -> bool {
        self.refit < 64 || self.refit == 0xfd
    }
    /// `Order::GetConditionSkipToOrder`.
    pub(crate) fn condition_skip_to(self) -> u8 {
        self.flags
    }
    /// `Order::CanLeaveWithCargo`.
    pub(crate) fn can_leave_with_cargo(self, has_cargo: bool) -> bool {
        self.load() != OLFB_NO_LOAD
            || (has_cargo && self.unload() != OUFB_UNLOAD && self.unload() != OUFB_TRANSFER)
    }
    /// `Order::IsFullLoadOrder`.
    pub(crate) fn is_full_load(self) -> bool {
        matches!(self.load(), OLFB_FULL_LOAD | OLFB_FULL_LOAD_ANY)
    }
    fn wait_timetabled(self) -> bool {
        if self.kind() == 7 {
            self.wait > 0
        } else {
            self.flags & 8 != 0
        }
    }
    fn travel_timetabled(self) -> bool {
        if self.kind() == 7 {
            self.travel > 0
        } else {
            self.flags & 128 != 0
        }
    }
    pub(crate) fn timetabled_wait(self) -> u16 {
        if self.wait_timetabled() { self.wait } else { 0 }
    }
    fn timetabled_travel(self) -> u16 {
        if self.travel_timetabled() {
            self.travel
        } else {
            0
        }
    }
    fn can_load_unload(self) -> bool {
        if (self.kind() != 1 && self.kind() != 8) || self.non_stop() & 2 != 0 {
            return false;
        }
        !(self.load() == 4 && self.unload() == 4)
    }
    fn complete(self) -> bool {
        if !self.travel_timetabled() && self.kind() != 7 {
            return false;
        }
        !(!self.wait_timetabled() && self.kind() == 1 && self.non_stop() & 2 == 0)
    }
    fn equals(self, other: Self) -> bool {
        if self.kind() == 2 && self.kind == other.kind && (self.action() | other.action()) & 2 != 0
        {
            return self.depot_type() == other.depot_type()
                && self.action() & !2 == other.action() & !2;
        }
        self.kind == other.kind
            && self.flags == other.flags
            && self.destination == other.destination
    }
    fn set_wait_timetabled(&mut self, b: bool) {
        if self.kind() != 7 {
            self.flags = (self.flags & !8) | if b { 8 } else { 0 };
        }
    }
    fn set_travel_timetabled(&mut self, b: bool) {
        if self.kind() != 7 {
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
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*state).first = v;
            (*state).manual = 0;
            (*state).vehicles = 1;
            (*state).timetable = 0;
        }
        for index in 0..g.stored_count(list) {
            let order = g.stored_order(list, index);
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                if order.kind() != 8 {
                    (*state).manual = (*state).manual.wrapping_add(1);
                }
                (*state).total = (*state)
                    .total
                    .wrapping_add(i32::from(order.wait) + i32::from(order.travel));
            }
        }
        recalculate(g, list);
        let mut u = g.previous(v);
        while !u.is_null() {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).vehicles = (*state).vehicles.wrapping_add(1);
                (*state).first = u;
            }
            u = g.previous(u);
        }
        u = g.next(v);
        while !u.is_null() {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).vehicles = (*state).vehicles.wrapping_add(1);
            }
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
            if matches!(order.kind(), 1 | 6)
                // SAFETY: This mode uses GetIfValid and accepts missing stations.
                && unsafe { (g.0.ownerless_station)(order.destination, 1) } != 0
            {
                (g.0.invalidate_station_list)();
                break;
            }
        }
        if keep == 0 {
            return 1;
        }
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_order_vector_resize(g.vector(list), 0);
            (*state).manual = 0;
            (*state).timetable = 0;
        }
    };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_decision(
    list: *mut c_void,
    next: u8,
    hops: u32,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    u32::from(g.decision(list, next, hops))
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
    g.stopping(list, v, first, hops, output);
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
        // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
        let order = unsafe { input.read() };
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_order_vector_insert(g.vector(list), index as usize, input);
            if order.kind() != 8 {
                (*state).manual = (*state).manual.wrapping_add(1);
            }
            (*state).timetable = (*state).timetable.wrapping_add(
                i32::from(order.timetabled_wait()) + i32::from(order.timetabled_travel()),
            );
            (*state).total = (*state)
                .total
                .wrapping_add(i32::from(order.wait) + i32::from(order.travel));
        }
        if matches!(order.kind(), 1 | 6)
            // SAFETY: List insertion supplies a live station/waypoint for these order kinds.
            && unsafe { (g.0.ownerless_station)(order.destination, 0) } != 0
        {
            (g.0.invalidate_station_list)();
        }
    };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_delete(
    list: *mut c_void,
    index: u32,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    let state = g.list(list);
    if (index as usize) < g.stored_count(list) {
        let order = g.stored_order(list, index as usize);
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            if order.kind() != 8 {
                (*state).manual = (*state).manual.wrapping_sub(1);
            }
            (*state).timetable = (*state).timetable.wrapping_sub(
                i32::from(order.timetabled_wait()) + i32::from(order.timetabled_travel()),
            );
            (*state).total = (*state)
                .total
                .wrapping_sub(i32::from(order.wait) + i32::from(order.travel));
            openttd_rust_order_vector_erase(g.vector(list), index as usize);
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_move(
    list: *mut c_void,
    from: u8,
    to: u8,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    if from < g.size(list) && to < g.size(list) {
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_order_vector_move(g.vector(list), from as usize, to as usize);
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_list_remove_vehicle(
    list: *mut c_void,
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    let state = g.list(list);
    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
    unsafe {
        (*state).vehicles = (*state).vehicles.wrapping_sub(1);
        if (*state).first == v {
            (*state).first = g.next(v);
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
            order.kind() == 8 || order.complete()
        }))
    }
}

fn recalculate(g: Game, list: *mut c_void) {
    let state = g.list(list);
    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
    unsafe {
        (*state).timetable = 0;
    }
    for i in 0..g.stored_count(list) {
        let order = g.stored_order(list, i);
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*state).timetable = (*state).timetable.wrapping_add(
                i32::from(order.timetabled_wait()) + i32::from(order.timetabled_travel()),
            );
        }
    }
}

impl Game {
    fn current(self, v: *mut c_void) -> Order {
        // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
        unsafe { (*self.vehicle(v)).current.as_ptr().read() }
    }
    fn put_current(self, v: *mut c_void, o: Order) {
        // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
        unsafe {
            (*self.vehicle(v)).current.as_mut_ptr().write(o);
        }
    }
    fn complete(self, list: *mut c_void) -> bool {
        (0..self.stored_count(list)).all(|i| {
            let o = self.stored_order(list, i);
            o.kind() == 8 || o.complete()
        })
    }
    fn duration(self, list: *mut c_void) -> i32 {
        if self.complete(list) {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            unsafe { (*self.list(list)).timetable }
        } else {
            -1
        }
    }
    fn reset(self, v: *mut c_void) {
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_consist_reset(self.consist(v));
        }
    }
    fn dirty(self, v: *mut c_void) {
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (self.0.timetable_dirty)(v);
        }
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
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*owner).total = (*owner).total.wrapping_add(total_delta);
            (*owner).timetable = (*owner).timetable.wrapping_add(timetable_delta);
        }
        let mut u = self.first(v);
        while !u.is_null() {
            let state = self.consist(u);
            let mut current = self.current(u);
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if unsafe { (*state).real == index } && current.equals(order) {
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
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let time = unsafe { (*state).time };
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*state).time = 0;
        }
        if self.current(v).kind() == 8 {
            return;
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let real = unsafe { (*state).real };
        if real >= self.count(v) {
            return;
        }
        let mut order = self.vehicle_order(v, real).unwrap();
        let mut first = 0u8;
        let list = self.orders(v);
        for i in 0..self.stored_count(list) {
            if self.stored_order(list, i).kind() != 8 {
                break;
            }
            first = first.wrapping_add(1);
        }
        let mut just_started = false;
        if real == first && travelling {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            just_started = unsafe { (*state).flags & (1 << 3) == 0 };
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if unsafe { (*state).start != 0 } {
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*state).lateness = (self.0.tick_counter)().wrapping_sub((*state).start) as i32;
                    (*state).start = 0;
                }
            }
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).flags |= 1 << 3;
            }
            self.dirty(v);
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if unsafe { (*state).flags & (1 << 3) == 0 } {
            return;
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let autofill = unsafe { (*state).flags & (1 << 4) != 0 };
        let remeasure =
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            !order.wait_timetabled() || (autofill && unsafe { (*state).flags & (1 << 5) == 0 });
        if travelling && remeasure {
            let mut current = self.current(v);
            current.wait = 0;
            self.put_current(v, current);
        }
        if just_started {
            return;
        }
        if order.kind() != 7 && (travelling || time > i32::from(order.wait) || remeasure) {
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
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).flags &= !((1 << 4) | (1 << 5));
            }
        }
        if autofill {
            return;
        }
        let timetable = i32::from(if travelling {
            order.timetabled_travel()
        } else {
            order.timetabled_wait()
        });
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if timetable == 0 && (travelling || unsafe { (*state).lateness >= 0 }) {
            return;
        }
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*state).lateness = (*state).lateness.wrapping_sub(timetable.wrapping_sub(time));
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if unsafe { (*state).lateness > timetable } {
            let cycle = self.duration(self.orders(v));
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if cycle != -1 && unsafe { (*state).lateness > cycle } {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                unsafe {
                    (*state).lateness %= cycle;
                }
            }
        }
        let mut u = self.first(v);
        while !u.is_null() {
            self.dirty(u);
            u = self.next(u);
        }
    }
    fn owner_check(self, v: *mut c_void, output: *mut c_void) -> bool {
        // SAFETY: The vehicle is live and output is the entry's writable native CommandCost.
        (unsafe { (self.0.vehicle_ownership)(v, output) }) != 0
    }
    fn command_error(self, output: *mut c_void, error: u32) -> u32 {
        // SAFETY: Output is the entry's live, writable native CommandCost.
        unsafe {
            (self.0.command_error)(output, error, 0);
        }
        0
    }
    fn primary(self, v: *mut c_void) -> bool {
        // SAFETY: The entry supplies a live shell or null; short-circuiting excludes null.
        !v.is_null() && unsafe { (self.0.primary_vehicle)(v) } != 0
    }
    fn timetable_sort(self, a: *mut c_void, b: *mut c_void) -> std::cmp::Ordering {
        let sa = self.consist(a);
        let sb = self.consist(b);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let mut ai = unsafe { (*sa).real };
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let mut bi = unsafe { (*sb).real };
        let raw = i32::from(bi) - i32::from(ai);
        let ao = self.current(a);
        let bo = self.current(b);
        if ao.kind() != 3 || ao.non_stop() == 0 {
            ai = ai.wrapping_sub(1);
        }
        if bo.kind() != 3 || bo.non_stop() == 0 {
            bi = bi.wrapping_sub(1);
        }
        let index = i32::from(bi) - i32::from(ai);
        if index != 0 {
            return index.cmp(&0);
        }
        if raw != 0 {
            return raw.cmp(&0);
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let time = unsafe { (*sb).time.wrapping_sub((*sa).time) };
        if time != 0 {
            return time.cmp(&0);
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe { (self.0.unit_number)(b) }.cmp(&unsafe { (self.0.unit_number)(a) })
    }
}

/// Timetable command validation and execution. Ownership is a shared company
/// service. Error IDs are mapped to unchanged strings by the C++ command facade.
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
        return g.command_error(output, 0);
    }
    if !g.owner_check(v, output) {
        return 0;
    }
    {
        let Some(order) = g.vehicle_order(v, index) else {
            return g.command_error(output, 0);
        };
        if order.kind() == 8 || field >= 3 {
            return g.command_error(output, 0);
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
                1 => {
                    if order.non_stop() & 2 != 0 {
                        return g.command_error(output, 1);
                    }
                }
                7 => {}
                _ => return g.command_error(output, 2),
            }
        }
        if travel != order.travel && order.kind() == 7 {
            return g.command_error(output, 0);
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        if speed != order.speed && (order.kind() == 7 || unsafe { (g.0.vehicle_type)(v) } == 3) {
            return g.command_error(output, 0);
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
        return g.command_error(output, 0);
    }
    if !g.owner_check(v, output) {
        return 0;
    }
    if field >= 3 || g.count(v) == 0 {
        return g.command_error(output, 0);
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
        return g.command_error(output, 0);
    }
    if g.orders(v).is_null() {
        return g.command_error(output, 0);
    }
    let state = g.consist(v);
    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
    if apply_group == 0 && unsafe { (*state).flags & (1 << 3) == 0 } {
        return g.command_error(output, 3);
    }
    if !g.owner_check(v, output) {
        return 0;
    }
    if execute != 0 {
        if apply_group != 0 {
            let mut late = 0;
            let mut u = g.first(v);
            while !u.is_null() {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if unsafe { (*state).flags & (1 << 3) != 0 } {
                    let us = g.consist(u);
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    late = late.max(unsafe { (*us).lateness });
                    g.reset(u);
                }
                u = g.next(u);
            }
            if late > 0 {
                u = g.first(v);
                while !u.is_null() {
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    if unsafe { (*state).flags & (1 << 3) != 0 } {
                        let us = g.consist(u);
                        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                        unsafe {
                            (*us).lateness = (*us).lateness.wrapping_sub(late);
                        }
                        g.dirty(u);
                    }
                    u = g.next(u);
                }
            }
        } else {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).lateness = 0;
            }
            g.reset(v);
            g.dirty(v);
        }
    }
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
    let g = Game(leaves);
    if !g.primary(v) {
        return g.command_error(output, 0);
    }
    if g.orders(v).is_null() {
        return g.command_error(output, 0);
    }
    if !g.owner_check(v, output) {
        return 0;
    }
    {
        let list = g.orders(v);
        let total = g.duration(list);
        let date = (g.0.economy_date)() as i32;
        let start_date = date.wrapping_add(
            (start.wrapping_sub((g.0.tick_counter)()) as i32)
                .wrapping_add((g.0.economy_fraction)() as i32)
                / 74,
        );
        let max_date = (g.0.maximum_date)() as i32;
        if start_date < 0 || start_date > max_date {
            return g.command_error(output, 0);
        }
        if start_date.wrapping_sub(date) > (g.0.timetable_year_limit)() as i32
            || date.wrapping_sub(start_date) > 366
        {
            return g.command_error(output, 0);
        }
        if all != 0 && !g.complete(list) {
            return g.command_error(output, 4);
        }
        if all != 0 && start_date.wrapping_add(total / 74) > max_date {
            return g.command_error(output, 0);
        }
        if execute != 0 {
            let mut vehicles = Vec::new();
            if all != 0 {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                let mut u = unsafe { (*g.list(list)).first };
                while !u.is_null() {
                    vehicles.push(u);
                    u = g.next(u);
                }
            } else {
                vehicles.push(v);
            }
            if vehicles.len() >= 2 {
                vehicles.sort_unstable_by(|all, start| g.timetable_sort(*all, *start));
            }
            let count = vehicles.len() as i32;
            for (index, u) in vehicles.into_iter().enumerate() {
                let us = g.consist(u);
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*us).lateness = 0;
                    (*us).flags &= !(1 << 3);
                    (*us).start =
                        start.wrapping_add(((index as i32).wrapping_mul(total) / count) as u64);
                }
                // Preserve the original v (rather than u) reset quirk.
                g.reset(v);
                g.dirty(u);
            }
        }
    };
    1
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
        return g.command_error(output, 0);
    }
    if g.orders(v).is_null() {
        return g.command_error(output, 0);
    }
    let state = g.consist(v);
    if !g.owner_check(v, output) {
        return 0;
    }
    if execute != 0 {
        if autofill != 0 {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).flags |= 1 << 4;
                (*state).flags &= !(1 << 3);
                if preserve_wait != 0 {
                    (*state).flags |= 1 << 5;
                }
                (*state).start = 0;
                (*state).lateness = 0;
            }
        } else {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).flags &= !((1 << 4) | (1 << 5));
            }
        }
        let mut u = g.first(v);
        while !u.is_null() {
            if u != v {
                let us = g.consist(u);
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*us).flags &= !((1 << 4) | (1 << 5));
                }
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
            .wrapping_mul(74)
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
    date.wrapping_add((start.wrapping_sub(counter) as i32).wrapping_add(i32::from(fract)) / 74)
}

impl Game {
    fn manual(self, v: *mut c_void) -> u8 {
        let list = self.orders(v);
        if list.is_null() {
            0
        } else {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            unsafe { (*self.list(list)).manual }
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
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*state).real = (*state).real.wrapping_add(1);
                    if (*state).real >= self.count(v) {
                        (*state).real = 0;
                    }
                }
                if self
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    .vehicle_order(v, unsafe { (*state).real })
                    .unwrap()
                    .kind()
                    != 8
                {
                    break;
                }
            }
        } else {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).real = 0;
            }
        }
    }
    fn increment_implicit(self, v: *mut c_void) {
        let state = self.consist(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if unsafe { (*state).implicit == (*state).real } {
            self.skip_real(v);
        }
        loop {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).implicit = (*state).implicit.wrapping_add(1);
                if (*state).implicit >= self.count(v) {
                    (*state).implicit = 0;
                }
            }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if unsafe { (*state).implicit == (*state).real }
                || self
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    .vehicle_order(v, unsafe { (*state).implicit })
                    .unwrap()
                    .kind()
                    == 8
            {
                break;
            }
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (self.0.invalidate_order)(v, 0);
        }
    }
    fn increment_real(self, v: *mut c_void) {
        let state = self.consist(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if unsafe { (*state).implicit == (*state).real } {
            self.increment_implicit(v);
        } else {
            self.skip_real(v);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (self.0.invalidate_order)(v, 0);
            }
        }
    }
    fn any_order(self, v: *mut c_void, predicate: impl Fn(Order) -> bool) -> bool {
        let list = self.orders(v);
        !list.is_null()
            && (0..self.stored_count(list)).any(|i| predicate(self.stored_order(list, i)))
    }
    fn previous_unbunching(self, v: *mut c_void) -> bool {
        let list = self.orders(v);
        if list.is_null() {
            return false;
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let implicit = unsafe { (*self.consist(v)).implicit };
        let index = if (implicit >= self.count(v) && self.count(v) == 0) || implicit == 0 {
            self.count(v).wrapping_sub(1)
        } else {
            implicit.wrapping_sub(1)
        };
        self.vehicle_order(v, index)
            .is_some_and(|o| o.kind() == 2 && o.action() & 4 != 0)
    }
    fn leave_unbunching(self, v: *mut c_void) {
        if !self.previous_unbunching(v) {
            return;
        }
        let state = self.consist(v);
        let tick = (self.0.tick_counter)();
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*state).last_departure = tick;
            (*state).lateness = 0;
        }
        self.dirty(v);
        let mut count = 0i32;
        let mut time = 0i32;
        let mut u = self.first(v);
        while !u.is_null() {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            if unsafe { (self.0.stopped_or_crashed)(u) } == 0 {
                count = count.wrapping_add(1);
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                time = time.wrapping_add(unsafe { (*self.consist(u)).round_trip });
            }
            u = self.next(u);
        }
        count = count.max(1);
        let separation = (time / count / count).max(1);
        let departure = tick.wrapping_add(separation as u64);
        u = self.first(v);
        while !u.is_null() {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            if unsafe { (self.0.stopped_or_crashed)(u) } == 0 {
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*self.consist(u)).next_departure = departure;
                }
                (self.maintenance().view_dirty)(native_vehicle(u));
            }
            u = self.next(u);
        }
    }
    fn measure_unbunching(self, v: *mut c_void) {
        let state = self.consist(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if self.current(v).action() & 4 == 0 || unsafe { (*state).last_departure == 0 } {
            return;
        }
        let measured =
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            (self.0.tick_counter)().wrapping_sub(unsafe { (*state).last_departure }) as i32;
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let previous = unsafe { (*state).round_trip };
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
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*state).round_trip = result;
        }
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
pub unsafe extern "C" fn openttd_rust_has_depot(v: *mut c_void, leaves: &'static Leaves) -> u32 {
    let g = Game(leaves);
    u32::from(g.any_order(v, |o| o.kind() == 2))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_has_full_load(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    u32::from(g.any_order(v, |o| o.kind() == 1 && matches!(o.load(), 2 | 3)))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_has_conditional(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    u32::from(g.any_order(v, |o| o.kind() == 7))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_has_unbunching(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u32 {
    let g = Game(leaves);
    u32::from(g.any_order(v, |o| o.kind() == 2 && o.action() & 4 != 0))
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
    let g = Game(leaves);
    {
        u32::from(
            !g.orders(v).is_null()
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                && unsafe { (*g.list(g.orders(v))).vehicles > 1 }
                && g.count(v) > 1
                && g.previous_unbunching(v)
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                && unsafe { (*g.consist(v)).next_departure > (g.0.tick_counter)() },
        )
    }
}

/// All fields belong to a live, properly constructed C++ Order, accessed raw.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_free(p: *mut Order) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 0;
        order.flags = 0;
        order.destination = 0;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_station(p: *mut Order, destination: u16) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 1;
        order.flags = 0;
        order.destination = destination;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
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
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 2;
        order.flags = (order.flags & !7) | (depot_type & 7);
        order.flags = (order.flags & !0x78) | ((action & 15) << 3);
        order.kind = (order.kind & !0xc0) | ((nonstop & 3) << 6);
        order.destination = destination;
        order.refit = cargo;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_waypoint(p: *mut Order, destination: u16) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 6;
        order.flags = 0;
        order.destination = destination;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_loading(p: *mut Order, ordered: u8) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 3;
        if ordered == 0 {
            order.flags = 0;
        }
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_leave(p: *mut Order) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 4;
        order.flags = 0;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_dummy(p: *mut Order) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 5;
        order.flags = 0;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_conditional(p: *mut Order, skip: u8) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 7;
        order.flags = skip;
        order.destination = 0;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_implicit(p: *mut Order, destination: u16) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    {
        order.kind = 8;
        order.destination = destination;
    };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_refit(p: *mut Order, cargo: u8) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let mut order = unsafe { p.read() };
    order.refit = cargo;
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_equals(p: *mut Order, q: *const Order) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let order = unsafe { p.read() };
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    u32::from(order.equals(unsafe { q.read() }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_assign(p: *mut Order, q: *const Order) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let order = unsafe { q.read() };
    // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
    unsafe {
        p.write(order);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_convert(p: *mut Order) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let order = unsafe { p.read() };
    {
        let mut packed = u16::from(order.kind());
        match order.kind() {
            1 => {
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
            2 => {
                if order.depot_type() & 2 == 0 {
                    packed |= 1 << 6;
                }
                packed |= 1 << 7;
                packed |= (order.destination & 255) << 8;
            }
            3 => {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_can_load_unload(p: *mut Order) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let order = unsafe { p.read() };
    u32::from(order.can_load_unload())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_can_leave(p: *mut Order, has_cargo: u8) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let order = unsafe { p.read() };
    {
        u32::from(
            order.load() != 4 || (has_cargo != 0 && order.unload() != 1 && order.unload() != 2),
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_should_stop(
    p: *mut Order,
    last_station: u16,
    station: u16,
) -> u32 {
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let order = unsafe { p.read() };
    {
        u32::from(
            (order.kind() != 2 || order.depot_type() & 2 != 0)
                && last_station != station
                && order.non_stop()
                    & (if order.kind() == 1 && order.destination == station {
                        2
                    } else {
                        1
                    })
                    == 0,
        )
    }
}

struct Control {
    g: Game,
}
impl Control {
    fn initialize_list(&self, list: *mut c_void, v: *mut c_void) {
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_list_init(list, v, self.g.0);
        }
    }
    fn new_list(&self, v: *mut c_void, input: Option<Order>, source: *mut c_void) -> *mut c_void {
        let g = self.g;
        let list = (g.0.allocate_list)();
        if let Some(order) = input {
            // SAFETY: Reentry with this entry's live shell, outputs and static table.
            unsafe {
                openttd_rust_order_vector_insert(g.vector(list), 0, &raw const order);
            }
        }
        if !source.is_null() {
            let vector = g.vector(source);
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            let data = unsafe { (*vector).data() };
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            let count = unsafe { (*vector).length };
            // SAFETY: Reentry with this entry's live shell, outputs and static table.
            unsafe {
                openttd_rust_order_vector_assign(g.vector(list), data, count);
            }
        }
        self.initialize_list(list, v);
        list
    }
    fn insert(&self, v: *mut c_void, order: Order, index: u8) {
        let g = self.g;
        let list = g.orders(v);
        if list.is_null() {
            let list = self.new_list(v, Some(order), ptr::null_mut());
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*g.vehicle(v)).orders = list;
            }
        } else {
            // SAFETY: Reentry with this entry's live shell, outputs and static table.
            unsafe {
                openttd_rust_list_insert(list, u32::from(index), &raw const order, g.0);
            }
        }
        let mut u = g.first(v);
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.delete_order_news)(u);
        }
        while !u.is_null() {
            let state = g.consist(u);
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if index <= unsafe { (*state).real } {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                let next = u16::from(unsafe { (*state).real }) + 1;
                if next < u16::from(g.count(u)) {
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).real = next as u8;
                    }
                }
            }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            if index == unsafe { (*state).implicit } && unsafe { (g.0.vehicle_type)(u) } < 2 {
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.suppress_implicit_write)(u, 1);
                }
            }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if index <= unsafe { (*state).implicit } {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                let next = u16::from(unsafe { (*state).implicit }) + 1;
                if next < u16::from(g.count(u)) {
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).implicit = next as u8;
                    }
                }
            }
            g.reset(u);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.invalidate_order)(u, i32::from(u16::from(u8::MAX) | (u16::from(index) << 8)));
            }
            u = g.next(u);
        }
        let list = g.orders(v);
        for i in 0..g.stored_count(list) {
            let mut o = g.stored_order(list, i);
            if o.kind() == 7 {
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
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.invalidate_vehicle_list)(v);
        }
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
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_list_delete(g.orders(v), u32::from(index), g.0);
        }
        let mut u = g.first(v);
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.delete_order_news)(u);
        }
        while !u.is_null() {
            let state = g.consist(u);
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if index == unsafe { (*state).real } && g.current(u).kind() == 3 {
                self.cancel_loading(u);
            }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if index < unsafe { (*state).real } {
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*state).real = (*state).real.wrapping_sub(1);
                }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            } else if index == unsafe { (*state).real } {
                g.update_real(u);
            }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if index < unsafe { (*state).implicit } {
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*state).implicit = (*state).implicit.wrapping_sub(1);
                }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            } else if index == unsafe { (*state).implicit } {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if unsafe { (*state).implicit >= g.count(u) } {
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).implicit = 0;
                    }
                }
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                while unsafe { (*state).implicit != (*state).real }
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    && g.vehicle_order(u, unsafe { (*state).implicit })
                        .unwrap()
                        .kind()
                        != 8
                {
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).implicit = (*state).implicit.wrapping_add(1);
                        if (*state).implicit >= g.count(u) {
                            (*state).implicit = 0;
                        }
                    }
                }
            }
            g.reset(u);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.invalidate_order)(u, i32::from(u16::from(index) | (u16::from(u8::MAX) << 8)));
            }
            u = g.next(u);
        }
        let list = g.orders(v);
        for i in 0..g.stored_count(list) {
            let mut o = g.stored_order(list, i);
            if o.kind() == 7 {
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
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.invalidate_vehicle_list)(v);
        }
    }
    fn delete_implicit(&self, v: *mut c_void) {
        let g = self.g;
        let state = g.consist(v);
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        if unsafe { (g.0.vehicle_type)(v) } < 2 && unsafe { (g.0.suppress_implicit)(v) } != 0 {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.suppress_implicit_write)(v, 0);
            }
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).implicit = (*state).real;
            }
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.invalidate_order)(v, 0);
            }
            return;
        }
        let list = g.orders(v);
        let orders = if list.is_null() {
            ptr::null_mut()
        } else {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            unsafe { (*g.vector(list)).data() }
        };
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let mut current = unsafe { (*state).implicit };
        while current != u8::MAX {
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if unsafe { (*state).implicit == (*state).real } {
                break;
            }
            // Match the original captured span and unchanged local index after
            // DeleteOrder. Erasure never reallocates; all capacity slots remain
            // typed/live, including the retained former last order. No reference
            // or borrow crosses the deletion/window callbacks.
            // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
            if unsafe { orders.add(usize::from(current)).read() }.kind() == 8 {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                self.delete(v, unsafe { (*state).implicit });
            } else {
                let next = g.next_index(g.orders(v), current);
                if current < next {
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).implicit = (*state).implicit.wrapping_add(1);
                    }
                } else {
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).implicit = 0;
                    }
                }
                current = next;
            }
        }
    }
    fn move_order(&self, v: *mut c_void, from: u8, to: u8) {
        let g = self.g;
        let list = g.orders(v);
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_order_vector_move(g.vector(list), usize::from(from), usize::from(to));
        }
        let mut u = g.first(v);
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.delete_order_news)(u);
        }
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
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).real = remap((*state).real, from, to);
                (*state).implicit = remap((*state).implicit, from, to);
            }
            g.reset(u);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.invalidate_order)(u, i32::from(u16::from(from) | (u16::from(to) << 8)));
            }
            u = g.next(u);
        }
        for i in 0..g.stored_count(list) {
            let mut o = g.stored_order(list, i);
            if o.kind() == 7 {
                o.flags = remap(o.flags, from, to);
                g.put_stored(list, i, o);
            }
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.invalidate_vehicle_list)(v);
        }
    }
    fn add_shared(&self, v: *mut c_void, shared: *mut c_void) {
        let g = self.g;
        if g.orders(shared).is_null() {
            let list = self.new_list(shared, None, ptr::null_mut());
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*g.vehicle(v)).orders = list;
                (*g.vehicle(shared)).orders = list;
            }
        }
        let next = g.next(shared);
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*g.vehicle(v)).next = next;
            (*g.vehicle(v)).previous = shared;
            (*g.vehicle(shared)).next = v;
            if !next.is_null() {
                (*g.vehicle(next)).previous = v;
            }
        }
        let list = g.list(g.orders(shared));
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*list).vehicles = (*list).vehicles.wrapping_add(1);
        }
    }
    fn remove_shared(&self, v: *mut c_void) {
        let g = self.g;
        let first = g.first(v);
        let were_first = first == v;
        // Window number is copied before changing the head, as in the original.
        // SAFETY: Both vehicles belong to the live shared-order chain.
        let identifier = unsafe { (g.0.shared_window)(v, first) };
        let list = g.list(g.orders(v));
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*list).vehicles = (*list).vehicles.wrapping_sub(1);
            if (*list).first == v {
                (*list).first = g.next(v);
            }
        }
        let previous = g.previous(v);
        let next = g.next(v);
        if !were_first {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*g.vehicle(previous)).next = next;
            }
        }
        if !next.is_null() {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*g.vehicle(next)).previous = previous;
            }
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if unsafe { (*list).vehicles == 1 } {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.close_shared_window)(v, identifier as u32);
            }
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.invalidate_order)(g.first(v), ((-2i64) as u64) as i32);
            }
        } else if were_first {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.invalidate_shared_window)(
                    v,
                    identifier as u32,
                    ((g.0.vehicle_id)(g.first(v)) | (1 << 31)) as i32,
                );
            }
        }
        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
        unsafe {
            (*g.vehicle(v)).next = ptr::null_mut();
            (*g.vehicle(v)).previous = ptr::null_mut();
        }
    }
    fn delete_vehicle_orders(&self, v: *mut c_void, keep: bool, reset: bool) {
        let g = self.g;
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.delete_order_news)(v);
        }
        let list = g.orders(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if !list.is_null() && unsafe { (*g.list(list)).vehicles > 1 } {
            self.remove_shared(v);
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*g.vehicle(v)).orders = ptr::null_mut();
            }
        } else if !list.is_null() {
            // SAFETY: Reentry with this entry's live shell, outputs and static table.
            let remove = unsafe { openttd_rust_list_free(list, (u32::from(keep)) as u8, g.0) };
            if remove != 0 {
                // SAFETY: The detached list is deleted once; no owner borrow or subsequent access remains.
                unsafe {
                    (g.0.delete_list)(list);
                }
            }
            if !keep {
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*g.vehicle(v)).orders = ptr::null_mut();
                }
            }
        }
        g.reset(v);
        if reset {
            let state = g.consist(v);
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).implicit = 0;
                (*state).real = 0;
            }
            if g.current(v).kind() == 3 {
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
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let input = unsafe { input.read() };
    control.insert(v, input, index);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_delete_order(
    v: *mut c_void,
    index: u8,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.delete(v, index);
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
pub unsafe extern "C" fn openttd_rust_remove_shared(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.remove_shared(v);
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
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        let valid = unsafe { openttd_rust_timetable_validate_bulk(v, field, output, g.0) };
        if valid == 0 {
            return 0;
        }
        if execute != 0 {
            let mut index = 0u8;
            while index < g.count(v) {
                let order = g.vehicle_order(v, index);
                if order.is_some_and(|o| o.kind() != 8) {
                    // SAFETY: The vehicle is live; no owner borrow spans the nested timetable command.
                    unsafe {
                        (g.0.change_timetable_command)(v, index, field, value);
                    }
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
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let input = unsafe { input.read() };
    u64::from(control.destination(v, input, depth, look_ahead != 0))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_process_orders(
    v: *mut c_void,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    u64::from(control.process(v))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_condition(
    v: *mut c_void,
    input: *const Order,
    leaves: &'static Leaves,
) -> u64 {
    let g = Game(leaves);

    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let input = unsafe { input.read() };
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

    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let input = unsafe { input.read() };
    u64::from(g.location(input, v, airport != 0))
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
    u64::from(g.distance(previous, current, v, depth))
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
    // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
    let input = unsafe { input.read() };
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
    let control = Control { g: Game(leaves) };
    control.clone_command(v, source, action, execute != 0, output)
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
    let control = Control { g: Game(leaves) };
    control.backup_tile_clear(u64::from(tile), u64::from(from_gui))
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
pub unsafe extern "C" fn openttd_rust_backup_destination_clear(
    kind: u8,
    destination: u16,
    hangar: u8,
    leaves: &'static Leaves,
) -> u64 {
    let control = Control { g: Game(leaves) };
    control.backup_destination_clear(u64::from(kind), u64::from(destination), u64::from(hangar))
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_check_orders(v: *mut c_void, leaves: &'static Leaves) -> u64 {
    let control = Control { g: Game(leaves) };
    control.check_orders(v);
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
        if order.kind() != 7 {
            return u8::MAX;
        }
        let variable = match order.variable() {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            0 => i32::from(crate::vehicle::visits::percent_filled(
                // SAFETY: the entry's live Vehicle shell.
                VehicleRef(unsafe { ptr::NonNull::new_unchecked(v.cast()) }),
                None,
                self.0.base.visits(),
            )),
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            1 => ((unsafe { (self.0.reliability)(v) } * 101) >> 16) as i32,
            7 => {
                (i32::from((self.maintenance().engine_reliability)(native_vehicle(v))) * 101) >> 16
            }
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            2 => ((unsafe { (self.0.display_speed)(v) }) as i32).wrapping_mul(10) / 16,
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            3 => (unsafe { (self.0.age_years)(v) }) as i32,
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            4 => (unsafe { (self.0.needs_service)(v) }) as i32,
            5 => return order.flags,
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            6 => (unsafe { (self.0.remaining_years)(v) }) as i32,
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
    fn location(self, order: Order, v: *mut c_void, airport: bool) -> u32 {
        match order.kind() {
            1 | 6 | 8 => {
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                (if airport && unsafe { (self.0.vehicle_type)(v) } == 3 {
                    // SAFETY: The source branch supplies a live destination of the required pool type.
                    unsafe { (self.0.airport_tile)(order.destination) }
                } else {
                    // SAFETY: The source branch supplies a live destination of the required pool type.
                    unsafe { (self.0.base_station_tile)(order.destination) }
                }) as u32
            }
            2 => {
                if order.destination == u16::MAX {
                    return u32::MAX;
                }
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                (if unsafe { (self.0.vehicle_type)(v) } == 3 {
                    // SAFETY: The source branch supplies a live destination of the required pool type.
                    unsafe { (self.0.station_tile)(order.destination) }
                } else {
                    // SAFETY: The source branch supplies a live destination of the required pool type.
                    unsafe { (self.0.depot_tile)(order.destination) }
                }) as u32
            }
            _ => u32::MAX,
        }
    }
    fn distance(self, previous: u8, current: u8, v: *mut c_void, depth: i32) -> u32 {
        let list = self.orders(v);
        // This source helper indexes the full stored span, not GetOrderAt's
        // uint8 count. Oversized save-loaded lists can still have conditional
        // destinations beyond their narrowed GetNumOrders value.
        let order = self.stored_order(list, usize::from(current));
        if order.kind() == 7 {
            if depth > i32::from(self.count(v)) {
                return 0;
            }
            let d1 = self.distance(previous, order.flags, v, depth.wrapping_add(1)) as i32;
            let d2 = self.distance(
                previous,
                self.next_index(list, current),
                v,
                depth.wrapping_add(1),
            ) as i32;
            return d1.max(d2) as u32;
        }
        let prev = self.location(self.stored_order(list, usize::from(previous)), v, true);
        let cur = self.location(order, v, true);
        if prev == u32::MAX || cur == u32::MAX {
            return 0;
        }
        // SAFETY: location resolved live destination tiles and the invalid sentinel
        // was excluded above. The vehicle shell is live; no owner borrow spans either call.
        unsafe { (self.0.distance)(prev, cur, u8::from((self.0.vehicle_type)(v) == 3)) as u32 }
    }
}
impl Control {
    fn destination(&self, v: *mut c_void, mut order: Order, mut depth: i32, pbs: bool) -> bool {
        let g = self.g;
        loop {
            if depth > i32::from(g.count(v)) {
                let mut current = g.current(v);
                current.kind = 0;
                current.flags = 0;
                current.destination = 0;
                g.put_current(v, current);
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.set_destination)(v, 0);
                }
                return false;
            }
            match order.kind() {
                1 => {
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    let tile = unsafe { (g.0.station_location)(v, order.destination) };
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    unsafe {
                        (g.0.set_destination)(v, tile as u32);
                    }
                    return true;
                }
                2 => {
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    if order.depot_type() & 1 != 0 && unsafe { (g.0.needs_service)(v) } == 0 {
                        g.update_timetable(v, true);
                        g.increment_real(v);
                    } else if g.current(v).action() & 2 != 0 {
                        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                        if unsafe { (g.0.destination_tile)(v) } == 0
                            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                            && (g.0.economy_fraction)() != unsafe { (g.0.vehicle_id)(v) } % 74
                        {
                        } else {
                            let mut closest = Closest::default();
                            // SAFETY: The callback writes one result into this live local.
                            unsafe { (g.0.closest_depot)(v, ptr::addr_of_mut!(closest)) };
                            if closest.found != 0 {
                                if pbs && closest.reverse != 0 {
                                    return false;
                                }
                                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                                unsafe {
                                    (g.0.set_destination)(v, closest.tile);
                                }
                                let mut current = g.current(v);
                                current.destination = closest.destination;
                                g.put_current(v, current);
                                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                                if unsafe { (g.0.vehicle_type)(v) } == 0 && closest.reverse != 0 {
                                    // SAFETY: The vehicle is a live train; no owner borrow spans the nested reverse command.
                                    unsafe {
                                        (g.0.reverse_train)(v);
                                    }
                                }
                                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                                if unsafe { (g.0.vehicle_type)(v) } == 3
                                    // SAFETY: The source branch has established that this live vehicle is an aircraft.
                                    && unsafe { (g.0.aircraft_flying)(v) } != 0
                                    // SAFETY: The source branch has established that this live vehicle is an aircraft.
                                    && unsafe { (g.0.target_airport)(v) } != u64::from(closest.destination)
                                {
                                    // SAFETY: The vehicle is a live aircraft; no owner borrow spans airport movement.
                                    unsafe {
                                        (g.0.next_airport)(v);
                                    }
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
                        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                        if unsafe { (g.0.vehicle_type)(v) } != 3 {
                            // SAFETY: The source branch supplies a live destination of the required pool type.
                            let tile = unsafe { (g.0.depot_tile)(order.destination) };
                            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                            unsafe {
                                (g.0.set_destination)(v, tile as u32);
                            }
                        } else {
                            let destination = g.current(v).destination;
                            // SAFETY: The source branch has established that this live vehicle is an aircraft.
                            if unsafe { (g.0.target_airport)(v) } != u64::from(destination) {
                                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                                let tile = unsafe { (g.0.station_location)(v, destination) };
                                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                                unsafe {
                                    (g.0.set_destination)(v, tile as u32);
                                }
                            }
                        }
                        return true;
                    }
                }
                6 => {
                    // SAFETY: The source branch supplies a live destination of the required pool type.
                    let tile = unsafe { (g.0.waypoint_tile)(order.destination) };
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    unsafe {
                        (g.0.set_destination)(v, tile as u32);
                    }
                    return true;
                }
                7 => {
                    let next = self.g.conditional(order, v);
                    if next != u8::MAX {
                        g.update_timetable(v, false);
                        let state = g.consist(v);
                        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                        unsafe {
                            (*state).implicit = next;
                            (*state).real = next;
                        }
                        g.update_real(v);
                        let travel = g
                            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                            .vehicle_order(v, unsafe { (*state).real })
                            .unwrap()
                            .timetabled_travel();
                        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                        unsafe {
                            (*state).time = (*state).time.wrapping_add(i32::from(travel));
                        }
                        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                        if unsafe { (g.0.vehicle_type)(v) } < 2 {
                            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                            unsafe {
                                (g.0.suppress_implicit_write)(v, 1);
                            }
                        }
                    } else {
                        g.update_timetable(v, true);
                        g.increment_real(v);
                    }
                }
                _ => {
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    unsafe {
                        (g.0.set_destination)(v, 0);
                    }
                    return false;
                }
            }
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            let real = unsafe { (*g.consist(v)).real };
            let next = g.vehicle_order(v, real).filter(|o| o.kind() != 8);
            let Some(next) = next else {
                let mut current = g.current(v);
                current.kind = 0;
                current.flags = 0;
                current.destination = 0;
                g.put_current(v, current);
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.set_destination)(v, 0);
                }
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
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let current_ptr = unsafe { (*vehicle).current.as_ptr() };
        // Match the source's early order-type exits before fetching anything else.
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let order_kind = unsafe { (*current_ptr).kind } & 15;
        match order_kind {
            2 => {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if (unsafe { (*current_ptr).flags } & 7) & 2 == 0 {
                    return false;
                }
            }
            3 => return false,
            4 => {
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                if unsafe { (g.0.vehicle_type)(v) } != 3 {
                    return false;
                }
            }
            _ => {}
        }
        let reverse = order_kind == 0;
        // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
        let current = unsafe { current_ptr.read() };
        if ((order_kind == 1 && current.non_stop() & 2 != 0) || order_kind == 6)
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            && unsafe { (g.0.at_station)(v) } != 0
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            && u64::from(current.destination) == unsafe { (g.0.tile_station)(v) }
        {
            self.delete_implicit(v);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.last_station_write)(v, current.destination);
            }
            g.update_timetable(v, true);
            g.increment_implicit(v);
            // Deletion/timetable/index updates may mutate the list or reenter.
            vehicle = g.vehicle(v);
        }
        let state = g.consist(v);
        let access = g.order_access(vehicle);
        access.update_real(state);
        let order = access
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            .order(unsafe { (*state).real })
            .filter(|o| o.kind() != 8);
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        let kind = unsafe { (g.0.vehicle_type)(v) };
        if order.is_none() || (kind == 3 && !access.valid()) {
            if kind == 3 {
                // SAFETY: The vehicle is a live aircraft; no owner borrow spans order mutation.
                unsafe {
                    (g.0.missing_aircraft_orders)(v);
                }
                return false;
            }
            // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
            let mut current = unsafe { (*vehicle).current.as_ptr().read() };
            current.kind = 0;
            current.flags = 0;
            current.destination = 0;
            // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
            unsafe {
                (*vehicle).current.as_mut_ptr().write(current);
            }
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.set_destination)(v, 0);
            }
            return false;
        }
        let order = order.unwrap();
        // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
        if order.equals(unsafe { (*vehicle).current.as_ptr().read() })
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            && (kind == 3 || unsafe { (g.0.destination_tile)(v) } != 0)
            && (kind != 2
                || order.kind() != 1
                // SAFETY: The source branch supplies a live destination of the required pool type.
                || unsafe { (g.0.ship_station_tile)(order.destination) } != u64::from(u32::MAX))
        {
            return false;
        }
        // SAFETY: Writes one Order into live storage at an in-bounds index; no reference survives.
        unsafe {
            (*vehicle).current.as_mut_ptr().write(order);
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.invalidate_order)(v, ((-2i64) as u64) as i32);
        }
        if kind == 2 || kind == 3 {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.dirty_vehicle_windows)(v);
            }
        }
        self.destination(v, order, 0, false) && reverse
    }
}

impl Game {
    fn error_with(self, output: *mut c_void, error: u32, detail: u64) -> u64 {
        // SAFETY: Output is the entry's live, writable native CommandCost.
        unsafe {
            (self.0.command_error)(output, error, detail as u32);
        }
        0
    }
    fn check_owner(self, owner: u64, output: *mut c_void) -> bool {
        // SAFETY: Output is the entry's live, writable native CommandCost.
        (unsafe { (self.0.owner_check)(owner as u8, output) }) != 0
    }
    fn error(self, output: *mut c_void, error: u32) -> u64 {
        u64::from(self.command_error(output, error))
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
            return self.g.error(output, 0);
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if order.refit != 0xfe || order.wait != 0 || order.travel != 0 || order.speed != u16::MAX {
            return self.g.error(output, 0);
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        let kind = unsafe { (g.0.vehicle_type)(v) };
        let destination = u64::from(order.destination);
        match order.kind() {
            1 => {
                if (g.0.valid_station)(destination as u16) == 0 {
                    return self.g.error(output, 0);
                }
                // SAFETY: The source branch supplies a live destination of the required pool type.
                let owner = unsafe { (g.0.station_owner)(destination as u16) };
                if owner != 0x10 && !self.g.check_owner(owner, output) {
                    return 0;
                }
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                if unsafe { order_can_use_station(v, destination as u16) } == 0 {
                    return self.g.error_with(
                        output,
                        5,
                        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                        unsafe { (g.0.station_error)(v, destination as u16) },
                    );
                }
                let mut u = g.first(v);
                while !u.is_null() {
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    if unsafe { order_can_use_station(u, destination as u16) } == 0 {
                        return self.g.error_with(
                            output,
                            6,
                            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                            unsafe { (g.0.station_error)(u, destination as u16) },
                        );
                    }
                    u = g.next(u);
                }
                if order.non_stop() != 0 && kind >= 2 {
                    return self.g.error(output, 0);
                }
                match order.load() {
                    0 | 4 => {}
                    2 | 3 => {
                        if g.any_order(v, |o| o.kind() == 2 && o.action() & 4 != 0) {
                            return self.g.error(output, 7);
                        }
                    }
                    _ => return self.g.error(output, 0),
                }
                if !matches!(order.unload(), 0 | 1 | 2 | 4) {
                    return self.g.error(output, 0);
                }
                match (order.kind >> 4) & 3 {
                    0 | 1 => {
                        if kind != 0 {
                            return self.g.error(output, 0);
                        }
                    }
                    2 => {}
                    _ => return self.g.error(output, 0),
                }
            }
            2 => {
                if order.action() & 2 == 0 {
                    if kind == 3 {
                        if (g.0.valid_station)(destination as u16) == 0 {
                            return self.g.error(output, 0);
                        }
                        if !self
                            .g
                            // SAFETY: The source branch supplies a live destination of the required pool type.
                            .check_owner(unsafe { (g.0.station_owner)(destination as u16) }, output)
                        {
                            return 0;
                        }
                        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                        if unsafe { order_can_use_station(v, destination as u16) } == 0
                            // SAFETY: The source branch supplies a live destination of the required pool type.
                            || u64::from(crate::station_base::airport_has_hangar(crate::newgrf_airport::station_by_id(destination as u16, crate::newgrf_airport::runtime()), crate::newgrf_airport::runtime())) == 0
                        {
                            return self.g.error(output, 0);
                        }
                    } else {
                        if (g.0.valid_depot)(destination as u16) == 0 {
                            return self.g.error(output, 0);
                        }
                        if !self
                            .g
                            // SAFETY: The source branch supplies a live destination of the required pool type.
                            .check_owner(unsafe { (g.0.depot_owner)(destination as u16) }, output)
                        {
                            return 0;
                        }
                        if kind > 2
                            || match kind {
                                // SAFETY: The source branch supplies a live destination of the required pool type.
                                0 => unsafe { (g.0.rail_depot)(destination as u16) },
                                // SAFETY: The source branch supplies a live destination of the required pool type.
                                1 => unsafe { (g.0.road_depot)(destination as u16) },
                                // SAFETY: The source branch supplies a live destination of the required pool type.
                                _ => unsafe { (g.0.ship_depot)(destination as u16) },
                            } == 0
                        {
                            return self.g.error(output, 0);
                        }
                    }
                }
                if order.non_stop() != 0 && kind >= 2 {
                    return self.g.error(output, 0);
                }
                let mut flags = order.depot_type();
                if flags & 2 != 0 {
                    flags &= !1;
                }
                flags &= !2;
                if flags != 0 || order.action() & !7 != 0 {
                    return self.g.error(output, 0);
                }
                if order.depot_type() & 1 != 0 && order.action() & 5 != 0 {
                    return self.g.error(output, 0);
                }
                if order.action() & 4 != 0 {
                    if g.any_order(v, |o| o.kind() == 1 && matches!(o.load(), 2 | 3)) {
                        return self.g.error(output, 8);
                    }
                    if g.any_order(v, |o| o.kind() == 2 && o.action() & 4 != 0) {
                        return self.g.error(output, 9);
                    }
                    if g.any_order(v, |o| o.kind() == 7) {
                        return self.g.error(output, 10);
                    }
                }
            }
            6 => {
                if (g.0.valid_waypoint)(destination as u16) == 0 {
                    return self.g.error(output, 0);
                }
                // SAFETY: The source branch supplies a live destination of the required pool type.
                let facilities = unsafe { (g.0.waypoint_facilities)(destination as u16) };
                match kind {
                    0 => {
                        if facilities & 1 == 0 {
                            return self.g.error(output, 11);
                        }
                        if !self.g.check_owner(
                            // SAFETY: valid_waypoint above established a live pool entry.
                            unsafe { (g.0.waypoint_owner)(destination as u16) },
                            output,
                        ) {
                            return 0;
                        }
                    }
                    1 => {
                        if facilities & 6 == 0 {
                            return self.g.error(output, 12);
                        }
                        if !self.g.check_owner(
                            // SAFETY: valid_waypoint above established a live pool entry.
                            unsafe { (g.0.waypoint_owner)(destination as u16) },
                            output,
                        ) {
                            return 0;
                        }
                    }
                    2 => {
                        if facilities & 16 == 0 {
                            return self.g.error(output, 13);
                        }
                        // SAFETY: The source branch supplies a live destination of the required pool type.
                        let owner = unsafe { (g.0.waypoint_owner)(destination as u16) };
                        if owner != 0x10 && !self.g.check_owner(owner, output) {
                            return 0;
                        }
                    }
                    _ => return self.g.error(output, 0),
                }
                if order.non_stop() != 0 && kind >= 2 {
                    return self.g.error(output, 0);
                }
            }
            7 => {
                if order.flags != 0 && order.flags >= g.count(v) {
                    return self.g.error(output, 0);
                }
                if order.variable() >= 8 {
                    return self.g.error(output, 0);
                }
                if g.any_order(v, |o| o.kind() == 2 && o.action() & 4 != 0) {
                    return self.g.error(output, 14);
                }
                let compare = order.comparator();
                if compare >= 8 {
                    return self.g.error(output, 0);
                }
                match order.variable() {
                    4 => {
                        if compare != 6 && compare != 7 {
                            return self.g.error(output, 0);
                        }
                    }
                    5 => {
                        if compare != 0 || order.value() != 0 {
                            return self.g.error(output, 0);
                        }
                    }
                    variable => {
                        if matches!(variable, 0 | 1 | 7) && order.value() > 100 {
                            return self.g.error(output, 0);
                        }
                        if compare == 6 || compare == 7 {
                            return self.g.error(output, 0);
                        }
                    }
                }
            }
            _ => return self.g.error(output, 0),
        }
        if index > g.count(v) {
            return self.g.error(output, 0);
        }
        if g.count(v) >= 254 {
            return self.g.error(output, 15);
        }
        if g.orders(v).is_null() && (g.0.list_capacity)() == 0 {
            return self.g.error(output, 16);
        }
        if execute {
            self.insert(v, order, index);
        }
        1
    }
    fn declone(&self, v: *mut c_void, execute: bool) -> u64 {
        if execute {
            self.delete_vehicle_orders(v, false, true);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (self.g.0.invalidate_order)(v, ((-1i64) as u64) as i32);
            }
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (self.g.0.invalidate_vehicle_list)(v);
            }
        }
        1
    }
    fn delete_command(&self, v: *mut c_void, index: u8, execute: bool, output: *mut c_void) -> u64 {
        let g = self.g;
        if !self.g.primary(v) {
            return self.g.error(output, 0);
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if index >= g.count(v) {
            return self.declone(v, execute);
        }
        if g.vehicle_order(v, index).is_none() {
            return self.g.error(output, 0);
        }
        if execute {
            self.delete(v, index);
        }
        1
    }
    fn skip_command(&self, v: *mut c_void, index: u8, execute: bool, output: *mut c_void) -> u64 {
        let g = self.g;
        if !self.g.primary(v) {
            return self.g.error(output, 0);
        }
        let state = g.consist(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if index == unsafe { (*state).implicit } || index >= g.count(v) || g.count(v) < 2 {
            return self.g.error(output, 0);
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if execute {
            if g.current(v).kind() == 3 {
                // SAFETY: The vehicle is live; no owner borrow spans station departure.
                let vehicle = VehicleRef(unsafe { ptr::NonNull::new_unchecked(v.cast()) });
                crate::vehicle::visits::leave_station(vehicle, g.0.base.visits());
            }
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).implicit = index;
                (*state).real = index;
            }
            g.update_real(v);
            g.reset(v);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.invalidate_order)(v, ((-2i64) as u64) as i32);
            }
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            if matches!(unsafe { (g.0.vehicle_type)(v) }, 2 | 3) {
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.dirty_vehicle_windows)(v);
                }
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
            return self.g.error(output, 0);
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
            return self.g.error(output, 0);
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
            return self.g.error(output, 0);
        }
        if !self.g.primary(v) {
            return self.g.error(output, 0);
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        if index >= g.count(v) {
            return self.g.error(output, 0);
        }
        let list = g.orders(v);
        let mut order = g.order(list, index).unwrap();
        match order.kind() {
            1 => {
                if !matches!(field, 0..=3) {
                    return self.g.error(output, 0);
                }
            }
            2 => {
                if !matches!(field, 0 | 4) {
                    return self.g.error(output, 0);
                }
            }
            6 => {
                if field != 0 {
                    return self.g.error(output, 0);
                }
            }
            7 => {
                if !matches!(field, 5..=8) {
                    return self.g.error(output, 0);
                }
            }
            _ => return self.g.error(output, 0),
        }
        let byte = data as u8;
        match field {
            0 => {
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                if unsafe { (g.0.vehicle_type)(v) } >= 2
                    || byte == order.non_stop()
                    || byte & !3 != 0
                {
                    return self.g.error(output, 0);
                }
            }
            1 => {
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                if unsafe { (g.0.vehicle_type)(v) } != 0 || data >= 3 {
                    return self.g.error(output, 0);
                }
            }
            2 => {
                if order.non_stop() & 2 != 0
                    || byte == order.unload()
                    || !matches!(byte, 0 | 1 | 2 | 4)
                {
                    return self.g.error(output, 0);
                }
            }
            3 => {
                if order.non_stop() & 2 != 0 || byte == order.load() {
                    return self.g.error(output, 0);
                }
                match byte {
                    0 | 4 => {}
                    2 | 3 => {
                        if g.any_order(v, |o| o.kind() == 2 && o.action() & 4 != 0) {
                            return self.g.error(output, 7);
                        }
                    }
                    _ => return self.g.error(output, 0),
                }
            }
            4 => {
                if byte >= 4 {
                    return self.g.error(output, 0);
                }
                if byte == 3 {
                    if g.any_order(v, |o| o.kind() == 2 && o.action() & 4 != 0)
                        && order.action() & 4 == 0
                    {
                        return self.g.error(output, 9);
                    }
                    if g.any_order(v, |o| o.kind() == 7) {
                        return self.g.error(output, 10);
                    }
                    if g.any_order(v, |o| o.kind() == 1 && matches!(o.load(), 2 | 3)) {
                        return self.g.error(output, 8);
                    }
                }
            }
            5 => {
                if byte >= 8 {
                    return self.g.error(output, 0);
                }
            }
            6 => {
                if byte >= 8 {
                    return self.g.error(output, 0);
                }
                match order.variable() {
                    5 => return self.g.error(output, 0),
                    4 => {
                        if byte != 6 && byte != 7 {
                            return self.g.error(output, 0);
                        }
                    }
                    _ => {
                        if byte == 6 || byte == 7 {
                            return self.g.error(output, 0);
                        }
                    }
                }
            }
            7 => match order.variable() {
                4 | 5 => return self.g.error(output, 0),
                0 | 1 | 7 => {
                    if data > 100 {
                        return self.g.error(output, 0);
                    }
                }
                _ => {
                    if data > 2047 {
                        return self.g.error(output, 0);
                    }
                }
            },
            8 => {
                if data >= u16::from(g.count(v)) {
                    return self.g.error(output, 0);
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
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.delete_order_news)(u);
            }
            while !u.is_null() {
                let state = g.consist(u);
                let mut current = g.current(u);
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if index == unsafe { (*state).real }
                    && matches!(current.kind(), 1 | 3)
                    && current.load() != order.load()
                {
                    current.flags = (current.flags & !0x70) | ((order.load() & 7) << 4);
                    g.put_current(u, current);
                }
                g.reset(u);
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.invalidate_order)(u, ((-2i64) as u64) as i32);
                }
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
            return self.g.error(output, 0);
        }
        if !self.g.primary(v) {
            return self.g.error(output, 0);
        }
        if !self.g.owner_check(v, output) {
            return 0;
        }
        let Some(mut order) = g.vehicle_order(v, index) else {
            return self.g.error(output, 0);
        };
        if cargo == 0xfd && order.kind() != 1 {
            return self.g.error(output, 0);
        }
        if order.load() == 4 {
            return self.g.error(output, 0);
        }
        if execute {
            order.refit = cargo;
            if cargo != 0xfe && order.kind() == 2 {
                order.flags &= !(1 | 8);
            }
            g.put_order(g.orders(v), index, order);
            let mut u = g.first(v);
            while !u.is_null() {
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.invalidate_order)(u, ((-2i64) as u64) as i32);
                }
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if index == unsafe { (*g.consist(u)).real } && g.current(u).depot_type() & 2 != 0 {
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
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
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
        // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
        unsafe { (*self.backup_vector(b)).data().add(i).read() }
    }
    fn captured(self, backup: *mut c_void, v: *mut c_void, user: u32) {
        let state = self.backup(backup);
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            (*state).user = user;
            (*state).tile = (self.0.vehicle_tile)(v) as u32;
            (*state).group = (self.0.vehicle_group)(v) as u16;
            openttd_rust_consist_copy(self.backup_consist(backup), self.consist(v));
        }
        // SAFETY: The new backup and source vehicle are live; neither owner is borrowed.
        unsafe {
            (self.0.capture_backup_metadata)(backup, v);
        }
        let list = self.orders(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if !list.is_null() && unsafe { (*self.list(list)).vehicles > 1 } {
            let first = self.first(v);
            let clone = if first == v { self.next(v) } else { first };
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*state).clone = clone;
            }
        } else if !list.is_null() {
            let vector = self.vector(list);
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            let data = unsafe { (*vector).data() };
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            let length = unsafe { (*vector).length };
            // SAFETY: Reentry with this entry's live shell, outputs and static table.
            unsafe {
                openttd_rust_order_vector_assign(self.backup_vector(backup), data, length);
            }
        }
    }
    fn aircraft_range(self, dst: *mut c_void, src: *mut c_void) -> bool {
        // SAFETY: The range check receives a live aircraft shell.
        if unsafe { (self.0.aircraft_range)(dst) } == 0 || self.count(src) == 0 {
            return true;
        }
        let list = self.orders(src);
        // SAFETY: The range check receives a live aircraft shell.
        let max = (unsafe { (self.0.aircraft_range_square)(dst) }) as u32;
        for i in 0..self.size(list) {
            if matches!(self.order(list, i).unwrap().kind(), 1 | 2 | 6)
                && self.distance(i, self.next_index(list, i), src, 0) > max
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
        action: u8,
        execute: bool,
        output: *mut c_void,
    ) -> u64 {
        let g = self.g;
        if !self.g.primary(dst) {
            return self.g.error(output, 0);
        }
        if !self.g.owner_check(dst, output) {
            return 0;
        }
        if action == 2 {
            return self.declone(dst, execute);
        }
        if action != 0 && action != 1 {
            return self.g.error(output, 0);
        }
        if !self.g.primary(src)
            // SAFETY: Both primary checks succeeded, so both native vehicle shells are live.
            || unsafe { (g.0.vehicle_type)(dst) } != unsafe { (g.0.vehicle_type)(src) }
            || dst == src
        {
            return self.g.error(output, 0);
        }
        if !self.g.owner_check(src, output) {
            return 0;
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        let kind = unsafe { (g.0.vehicle_type)(src) };
        if action == 0 {
            // SAFETY: The source branch has established that both live vehicles are road vehicles.
            if kind == 1 && unsafe { (g.0.bus)(src) } != unsafe { (g.0.bus)(dst) } {
                return self.g.error(output, 0);
            }
            if g.first(src) == g.first(dst) {
                return self.g.error(output, 0);
            }
        }
        let list = g.orders(src);
        if !list.is_null() {
            for i in 0..g.stored_count(list) {
                let order = g.stored_order(list, i);
                if order.kind() != 1
                    && !(kind == 3 && order.kind() == 2 && order.destination != u16::MAX)
                {
                    continue;
                }
                let destination = u64::from(order.destination);
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                if (action == 1 || unsafe { order_can_use_station(src, destination as u16) } != 0)
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    && unsafe { order_can_use_station(dst, destination as u16) } == 0
                {
                    return self.g.error_with(
                        output,
                        17,
                        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                        unsafe { (g.0.station_error)(dst, destination as u16) },
                    );
                }
            }
        }
        if kind == 3 && !g.aircraft_range(dst, src) {
            return self.g.error(output, 18);
        }
        if (action == 1 || list.is_null()) && (g.0.list_capacity)() == 0 {
            return self.g.error(output, 16);
        }
        if execute {
            self.delete_vehicle_orders(dst, action == 1, g.count(dst) != g.count(src));
            if action == 0 {
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*g.vehicle(dst)).orders = g.orders(src);
                }
                self.add_shared(dst, src);
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.invalidate_order)(dst, ((-1i64) as u64) as i32);
                }
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.invalidate_order)(src, ((-2i64) as u64) as i32);
                }
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.invalidate_vehicle_list)(dst);
                }
            } else {
                // Original copies the source before deleting the kept empty shell.
                // Rust temporary storage is a scalar copy, never a live mirror.
                let mut copied = Vec::new();
                let source = g.orders(src);
                if !source.is_null() {
                    let vector = g.vector(source);
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    for i in 0..unsafe { (*vector).length } {
                        // SAFETY: Reads one Order from live storage; the source is valid and no reference survives.
                        copied.push(unsafe { (*vector).data().add(i).read() });
                    }
                }
                let old = g.orders(dst);
                if !old.is_null() {
                    // SAFETY: The detached list is deleted once; no owner borrow or subsequent access remains.
                    unsafe {
                        (g.0.delete_list)(old);
                    }
                }
                let list = self.new_list(dst, None, ptr::null_mut());
                // SAFETY: Reentry with this entry's live shell, outputs and static table.
                unsafe {
                    openttd_rust_order_vector_assign(g.vector(list), copied.as_ptr(), copied.len());
                }
                // Initialize after copied orders, retaining original total calculation.
                self.initialize_list(list, dst);
                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                unsafe {
                    (*g.vehicle(dst)).orders = list;
                }
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.invalidate_order)(dst, ((-1i64) as u64) as i32);
                }
                // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                unsafe {
                    (g.0.invalidate_vehicle_list)(dst);
                }
            }
        }
        1
    }
    fn restore_backup(&self, backup: *mut c_void, v: *mut c_void) {
        let g = self.g;
        let state = g.backup(backup);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let clone = unsafe { (*state).clone };
        if !clone.is_null() {
            // SAFETY: Both vehicles are live; no owner borrow spans the nested share command.
            unsafe {
                (g.0.share_command)(v, clone);
            }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        } else if unsafe { (*g.backup_vector(backup)).length > 0 } && (g.0.list_capacity)() != 0 {
            let list = (g.0.allocate_list)();
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            unsafe {
                (*g.vector(list)).transfer(g.backup_vector(backup));
            }
            self.initialize_list(list, v);
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*g.vehicle(v)).orders = list;
            }
            (g.0.invalidate_station_list)();
        }
        // SAFETY: The pool traversal supplies a live OrderBackup shell and its native name.
        if unsafe { (g.0.unique_backup_name)(backup) } == 0 {
            // SAFETY: The backup remains allocated throughout this native string mutation.
            unsafe {
                (g.0.clear_backup_name)(backup);
            }
        }
        // SAFETY: Reentry with this entry's live shell, outputs and static table.
        unsafe {
            openttd_rust_consist_copy(g.consist(v), g.backup_consist(backup));
        }
        // SAFETY: The vehicle and backup are live; neither owner is borrowed across the copy.
        unsafe {
            (g.0.restore_backup_metadata)(v, backup);
        }
        g.update_real(v);
        let vs = g.consist(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        if unsafe { (*vs).implicit >= g.count(v) } {
            // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
            unsafe {
                (*vs).implicit = (*vs).real;
            }
        }
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        // SAFETY: The vehicle is live; no owner borrow spans the nested group command.
        unsafe {
            (g.0.group_command)(v, (*state).group);
        }
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
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if unsafe { (*g.backup(ob)).user == a as u32 } {
                // SAFETY: The pool entry is deleted once; no owner borrow or subsequent access remains.
                unsafe {
                    (g.0.delete_backup)(ob);
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
        if (g.0.backup_capacity)() != 0 {
            // SAFETY: The source vehicle is live; no owner borrow spans allocation or reentry.
            unsafe {
                (g.0.create_backup)(v, a as u32);
            }
        }
        0
    }
    fn backup_restore(&self, v: *mut c_void, a: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            if (unsafe { (g.0.vehicle_tile)(v) }) as u32 == unsafe { (*state).tile }
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                && unsafe { (*state).user == a as u32 }
            {
                self.restore_backup(ob, v);
                // SAFETY: The pool entry is deleted once; no owner borrow or subsequent access remains.
                unsafe {
                    (g.0.delete_backup)(ob);
                }
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
            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
            if unsafe { (*g.backup(ob)).user == a as u32 } {
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
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if unsafe {
                    (*state).user == b as u32 && ((*state).tile == a as u32 || a as u32 == u32::MAX)
                } {
                    // SAFETY: The pool entry is deleted once; no owner borrow or subsequent access remains.
                    unsafe {
                        (g.0.delete_backup)(ob);
                    }
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
    fn backup_tile_clear(&self, a: u64, b: u64) -> u64 {
        let g = self.g;
        let user = if (g.0.networking)() != 0 && (g.0.network_server)() == 0 {
            (g.0.network_client)() as u32
        } else {
            (g.0.server_client)() as u32
        };
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if (b == 0 || unsafe { (*state).user == user })
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    && (a as u32 == u32::MAX || a as u32 == unsafe { (*state).tile })
                {
                    if b != 0 {
                        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                        (g.0.clear_backup_gui)(unsafe { (*state).tile }, user);
                    } else {
                        // SAFETY: The pool entry is deleted once; no owner borrow or subsequent access remains.
                        unsafe {
                            (g.0.delete_backup)(ob);
                        }
                    }
                }
            }
            ob = self.g.next_backup(&mut cursor);
        }
        0
    }
    fn backup_group_clear(&self, a: u64) -> u64 {
        let g = self.g;
        let mut cursor = 0;
        let mut ob = self.g.next_backup(&mut cursor);
        while !ob.is_null() {
            let state = g.backup(ob);
            {
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if unsafe { (*state).group == a as u16 } {
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).group = (g.0.default_group)() as u16;
                    }
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
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                if unsafe { (*state).clone == v } {
                    let first = g.first(v);
                    let clone = if first == v { g.next(v) } else { first };
                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                    unsafe {
                        (*state).clone = clone;
                    }
                    if clone.is_null() {
                        // SAFETY: The pool entry is deleted once; no owner borrow or subsequent access remains.
                        unsafe {
                            (g.0.delete_backup)(ob);
                        }
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
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                let length = unsafe { (*g.backup_vector(ob)).length };
                for i in 0..length {
                    let order = g.backed_order(ob, i);
                    let mut kind = order.kind();
                    if kind == 2 && order.action() & 2 != 0 {
                        continue;
                    }
                    // SAFETY: The pool traversal supplies a live OrderBackup shell and its saved tile.
                    let hangar = crate::station_map::is_hangar_tile(
                        unsafe { (*g.backup(ob)).tile },
                        crate::newgrf_airport::runtime(),
                    );
                    if kind == 2 && c != 0 && !hangar {
                        continue;
                    }
                    if kind == 8 || (hangar && kind == 2 && c == 0) {
                        kind = 1;
                    }
                    if u64::from(kind) == a && u64::from(order.destination) == b {
                        // SAFETY: The pool entry is deleted once; no owner borrow or subsequent access remains.
                        unsafe {
                            (g.0.delete_backup)(ob);
                        }
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
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            let next = ((unsafe { (g.0.vehicle_id)(v) }) as u32).wrapping_add(1);
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            let aircraft = unsafe { (g.0.vehicle_type)(v) } == 3;
            let mut current = g.current(v);
            let current_kind = if aircraft && current.kind() == 2 && !hangar {
                1
            } else {
                current.kind()
            };
            if current_kind == kind && (!hangar || aircraft) && current.destination == destination {
                current.kind = 5;
                current.flags = 0;
                g.put_current(v, current);
                (g.maintenance().view_dirty)(native_vehicle(v));
            }
            if !g.orders(v).is_null() {
                let mut index = 0u8;
                while index < g.count(v) {
                    let mut next = index.wrapping_add(1);
                    let list = g.orders(v);
                    let mut order = g.order(list, index).unwrap();
                    let mut ty = order.kind();
                    if !(ty == 2 && (order.action() & 2 != 0 || (hangar && !aircraft))) {
                        if ty == 8 || (aircraft && ty == 2 && !hangar) {
                            ty = 1;
                        }
                        if ty == kind && order.destination == destination {
                            if order.kind() == 8 {
                                self.delete(v, index);
                                next = index;
                            } else {
                                let state = g.list(list);
                                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                                unsafe {
                                    (*state).total =
                                        (*state).total.wrapping_sub(i32::from(order.wait));
                                }
                                if order.wait_timetabled() {
                                    // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                                    unsafe {
                                        (*state).timetable = (*state)
                                            .timetable
                                            .wrapping_sub(i32::from(order.timetabled_wait()));
                                    }
                                    order.set_wait_timetabled(false);
                                }
                                order.wait = 0;
                                let travel = order.travel_timetabled();
                                order.kind = 5;
                                order.flags = 0;
                                order.set_travel_timetabled(travel);
                                g.put_order(list, index, order);
                                let mut shared = g.first(v);
                                while !shared.is_null() {
                                    let data = u16::from(index) | (u16::from(u8::MAX) << 8);
                                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                                    unsafe {
                                        (g.0.invalidate_order)(shared, i32::from(data));
                                    }
                                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                                    unsafe {
                                        (g.0.invalidate_order)(shared, i32::from(data));
                                    }
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
    fn check_orders(&self, v: *mut c_void) {
        let g = self.g;
        let review = (g.0.review_setting)();
        if review == 0
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            || unsafe { (g.0.vehicle_status)(v) } & 128 != 0
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            || (review == 1 && unsafe { (g.0.vehicle_status)(v) } & 2 != 0)
            || g.first(v) != v
        {
            return;
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        if unsafe { (g.0.local_owner)(v) } == 0
            || !(g.0.base.day)(native_vehicle(v)).is_multiple_of(20)
        {
            return;
        }
        let mut message = 0u32;
        let mut stations = 0i32;
        let list = g.orders(v);
        if !list.is_null() {
            for i in 0..g.stored_count(list) {
                let order = g.stored_order(list, i);
                if order.kind() == 5 {
                    message = 1;
                    break;
                }
                if order.kind() == 1 {
                    stations = stations.wrapping_add(1);
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    if unsafe { order_can_use_station(v, order.destination) } == 0 {
                        message = 2;
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    } else if unsafe { (g.0.vehicle_type)(v) } == 3
                        // SAFETY: The source branch has established that this live vehicle is an aircraft.
                        && unsafe { (g.0.fast_aircraft)(v) } != 0
                        // SAFETY: The source branch supplies a live destination of the required pool type.
                        && u64::from((crate::newgrf_airport::runtime().fta_flags)(crate::newgrf_airport::station_fta(crate::newgrf_airport::station_by_id(order.destination, crate::newgrf_airport::runtime()), crate::newgrf_airport::runtime())) & crate::airport::FTA_SHORT_STRIP != 0) != 0
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
                .equals(g.stored_order(list, g.stored_count(list) - 1))
        {
            message = 4;
        }
        if stations < 2 && message == 0 {
            message = 5;
        }
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        unsafe {
            (g.0.debug_list)(v);
        }
        if message != 0 {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            unsafe {
                (g.0.order_news)(v, message);
            }
        }
    }
}

impl Control {
    fn begin_loading_orders(&self, v: *mut c_void) {
        let g = self.g;
        let current = g.current(v);
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        let last = (unsafe { (g.0.last_station)(v) }) as u16;
        if current.kind() == 1 && current.destination == last {
            self.delete_implicit(v);
            let mut current = g.current(v);
            current.kind = 3;
            g.put_current(v, current);
            g.update_timetable(v, true);
            let mut current = g.current(v);
            current.kind = (current.kind & !0xc0) | 0xc0;
            g.put_current(v, current);
            return;
        }
        let state = g.consist(v);
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        let implicit = unsafe { (*state).implicit };
        let in_list = g.vehicle_order(v, implicit);
        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
        if unsafe { (g.0.vehicle_type)(v) } < 2
            && !in_list.is_some_and(|o| o.kind() == 8 && o.destination == last)
        {
            // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
            let suppress = unsafe { (g.0.suppress_implicit)(v) } != 0;
            let previous = if implicit > 0 {
                g.vehicle_order(v, implicit - 1)
            } else if g.count(v) > 1 {
                g.vehicle_order(v, g.count(v) - 1)
            } else {
                None
            };
            if !previous.is_some_and(|o| matches!(o.kind(), 1 | 8) && o.destination == last) {
                let mut target = i32::from(implicit);
                let mut found = false;
                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                while target != i32::from(unsafe { (*state).real }) || g.manual(v) == 0 {
                    let Some(order) = g.vehicle_order(v, target as u8) else {
                        break;
                    };
                    if order.kind() == 8 && order.destination == last {
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
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    if target == i32::from(unsafe { (*state).implicit }) {
                        break;
                    }
                }
                if found {
                    if suppress {
                        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                        unsafe {
                            (*state).implicit = target as u8;
                        }
                        // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                        unsafe {
                            (g.0.invalidate_order)(v, 0);
                        }
                    } else {
                        loop {
                            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                            let order = g.vehicle_order(v, unsafe { (*state).implicit }).unwrap();
                            if order.kind() == 8 && order.destination == last {
                                break;
                            }
                            if order.kind() == 8 {
                                // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                                self.delete(v, unsafe { (*state).implicit });
                            } else {
                                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                                unsafe {
                                    (*state).implicit = (*state).implicit.wrapping_add(1);
                                }
                            }
                            // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                            if g.vehicle_order(v, unsafe { (*state).implicit }).is_none() {
                                // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                                unsafe {
                                    (*state).implicit = 0;
                                }
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
                    let order = Order {
                        kind: 8,
                        flags: 0,
                        destination: last,
                        refit: 0xfe,
                        wait: 0,
                        travel: 0,
                        speed: u16::MAX,
                    };
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    self.insert(v, order, unsafe { (*state).implicit });
                    // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
                    if unsafe { (*state).implicit > 0 } {
                        // SAFETY: Exclusive serialized scalar write to a live owner; no borrow survives the statement.
                        unsafe {
                            (*state).implicit = (*state).implicit.wrapping_sub(1);
                        }
                    }
                    // SAFETY: The entry supplies a live Vehicle shell; no owner borrow crosses this call.
                    unsafe {
                        (g.0.suppress_implicit_write)(v, 0);
                    }
                }
            }
        }
        let mut current = g.current(v);
        current.kind = 3;
        current.flags = 0;
        g.put_current(v, current);
    }
}

/// Short raw access to the canonical aliased Order; never retained across calls.
pub(crate) struct OrderHandle(pub(crate) *const Order);
impl OrderHandle {
    /// # Safety
    /// Handle denotes a live Order under the serial game-state mutex.
    pub(crate) unsafe fn is_type(&self, kind: u8) -> bool {
        // SAFETY: One scalar read from the live canonical record.
        unsafe { (&*self.0).kind() == kind }
    }
    /// # Safety
    /// Handle denotes a live Order under the serial game-state mutex.
    pub(crate) unsafe fn destination(&self) -> u16 {
        // SAFETY: One scalar read from the live canonical record.
        unsafe { ptr::addr_of!((*self.0).destination).read() }
    }
    /// # Safety
    /// Handle denotes a live depot Order under the serial game-state mutex.
    pub(crate) unsafe fn nearest_depot(&self) -> bool {
        // SAFETY: One scalar read; native OrderDepotActionFlag::NearestDepot = 1.
        unsafe { (&*self.0).action() & 2 != 0 }
    }
}

/// Direct access to the canonical orders owner for route prediction (#349).
/// Handles are live for the synchronous entry; no owner borrow crosses a leaf.
#[derive(Clone, Copy)]
pub(crate) struct VisitOrders(Game);
impl VisitOrders {
    pub(crate) fn new(leaves: &'static Leaves) -> Self {
        Self(Game(leaves))
    }
    fn shell(v: VehicleRef) -> *mut c_void {
        v.0.as_ptr().cast()
    }
    /// `v->orders`, the native `OrderList` shell or null.
    pub(crate) fn list(self, v: VehicleRef) -> *mut c_void {
        self.0.orders(Self::shell(v))
    }
    /// `OrderList::GetNumOrders()`.
    pub(crate) fn count(self, list: *mut c_void) -> u8 {
        self.0.size(list)
    }
    /// `orders[index]` for an index below `count`.
    pub(crate) fn order(self, list: *mut c_void, index: u8) -> Order {
        self.0.stored_order(list, usize::from(index))
    }
    /// `OrderList::GetNext`.
    pub(crate) fn next(self, list: *mut c_void, index: u8) -> u8 {
        self.0.next_index(list, index)
    }
    /// `OrderList::GetNextDecisionNode`.
    pub(crate) fn decision(self, list: *mut c_void, next: u8, hops: u32) -> u8 {
        self.0.decision(list, next, hops)
    }
    /// `v->cur_implicit_order_index`.
    pub(crate) fn implicit(self, v: VehicleRef) -> u8 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.0.consist(Self::shell(v))).implicit }
    }
    /// `v->current_order_time`.
    pub(crate) fn time(self, v: VehicleRef) -> i32 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.0.consist(Self::shell(v))).time }
    }
    /// `OrderList::GetTotalDuration()`.
    pub(crate) fn total(self, list: *mut c_void) -> i32 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.0.list(list)).total }
    }
    /// `v->current_order`.
    pub(crate) fn current(self, v: VehicleRef) -> Order {
        self.0.current(Self::shell(v))
    }
    /// `v->GetOrder(index)`: null without a list or beyond its orders.
    pub(crate) fn get(self, v: VehicleRef, index: u8) -> Option<Order> {
        self.0.vehicle_order(Self::shell(v), index)
    }
    /// `v->cur_real_order_index`.
    pub(crate) fn real(self, v: VehicleRef) -> u8 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.0.consist(Self::shell(v))).real }
    }
    /// `v->lateness_counter`.
    pub(crate) fn lateness(self, v: VehicleRef) -> i32 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.0.consist(Self::shell(v))).lateness }
    }
    /// `v->vehicle_flags`.
    pub(crate) fn flags(self, v: VehicleRef) -> u16 {
        // SAFETY: Scalar read of a live serialized owner; no borrow survives the statement.
        unsafe { (*self.0.consist(Self::shell(v))).flags }
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
    /// The vehicle type's list-window invalidation.
    pub(crate) fn dirty_vehicle_windows(self, v: VehicleRef) {
        // SAFETY: the live shell is read synchronously; no owner borrow survives.
        unsafe { (self.0.0.dirty_vehicle_windows)(Self::shell(v)) };
    }
    /// `UpdateVehicleTimetable`.
    pub(crate) fn update_timetable(self, v: VehicleRef, travelling: bool) {
        self.0.update_timetable(Self::shell(v), travelling);
    }
    /// The unbunching round-trip measurement of `VehicleEnterDepot`.
    pub(crate) fn measure_unbunching(self, v: VehicleRef) {
        self.0.measure_unbunching(Self::shell(v));
    }
    /// `BaseConsist::ResetDepotUnbunching`.
    pub(crate) fn reset_unbunching(self, v: VehicleRef) {
        // SAFETY: The live consist owner is written by scalar stores only.
        unsafe { openttd_rust_consist_reset(self.0.consist(Self::shell(v))) };
    }
    /// `v->current_order.MakeLeaveStation()`.
    pub(crate) fn make_leave_station(self, v: VehicleRef) {
        // SAFETY: One Order store into the live current order; no alias survives.
        unsafe { openttd_rust_order_leave((*self.0.vehicle(Self::shell(v))).current.as_mut_ptr()) };
    }
    /// `v->current_order.MakeDummy()`.
    pub(crate) fn make_dummy(self, v: VehicleRef) {
        // SAFETY: One Order store into the live current order; no alias survives.
        unsafe { openttd_rust_order_dummy((*self.0.vehicle(Self::shell(v))).current.as_mut_ptr()) };
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
        first_vehicle,
        last_station,
        ownerless_station,
        vehicle_type,
        vehicle_status,
        tick_counter,
        primary_vehicle,
        vehicle_ownership,
        ticks_per_second,
        unit_number,
        economy_date,
        economy_fraction,
        maximum_date,
        timetable_year_limit,
        stopped_or_crashed,
        allocate_list,
        suppress_implicit,
        shared_window,
        vehicle_id,
        reliability,
        display_speed,
        age_years,
        needs_service,
        remaining_years,
        airport_tile,
        base_station_tile,
        station_tile,
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
        owner_check,
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
        bus,
        backup_capacity,
        create_backup,
        networking,
        network_server,
        network_client,
        server_client,
        default_group,
        vehicle_tile,
        vehicle_group,
        unique_backup_name,
        review_setting,
        local_owner,
        fast_aircraft,
        no_jet_crash,
        append_station,
        invalidate_station_list,
        command_error,
        timetable_dirty,
        invalidate_order,
        delete_order_news,
        suppress_implicit_write,
        invalidate_vehicle_list,
        close_shared_window,
        invalidate_shared_window,
        last_station_write,
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
unsafe fn order_can_use_station(vehicle: *mut std::ffi::c_void, destination: u16) -> u64 {
    // SAFETY: The caller supplies live serialized vehicle and destination shells.
    u64::from(unsafe {
        crate::vehicle_func::can_vehicle_use_station(
            vehicle.cast(),
            crate::newgrf_airport::station_by_id(destination, crate::newgrf_airport::runtime()),
            crate::newgrf_airport::runtime(),
        )
    })
}
