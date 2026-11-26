/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Orders, order lists, order backups and timetables.
//!
//! The file holds the bodies of `order_cmd.cpp` and the members of `order_base.h` (orders,
//! order lists, the seven order commands, `ProcessOrders` and `UpdateOrderDest`),
//! `order_backup.cpp`, `timetable_cmd.cpp`, the order and unbunching members of
//! `vehicle.cpp` and `vehicle_base.h`, and `base_consist.cpp`. It also owns the Rust records
//! the C++ shells alias: `Order`, `BaseConsist`'s members (`Consist`), the vehicle's order
//! members (`VehicleOrders`), an `OrderList`'s members (`List`) and its orders (`Vector`),
//! and an `OrderBackup`'s members (`Backup`).
//!
//! Lifetime contract: every record, owner and shell a body reaches is live and serialized
//! by the game-state mutex for the synchronous entry. Each access to an aliased field is a
//! field-sized raw load or store at the original's observation point, and no Rust reference
//! to a native order, list, consist or backup survives a service call, mutation or reentry.
//! A handle that addresses a list's orders is resolved again after any call that can
//! replace the vehicle's list or grow its orders.

use crate::base_consist::{VehicleFlag, VehicleFlags};
use crate::bitmath_func::{assign_bit16, gb_u8, gb_u16, has_bit, sb_u8, sb_u16};
use crate::cargo_type::{CARGO_AUTO_REFIT, CARGO_NO_REFIT, CargoType, NUM_CARGO};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, Owner};
use crate::ground_vehicle::GVF_SUPPRESS_IMPLICIT_ORDERS;
use crate::group_type::GroupID;
use crate::network_type::{CLIENT_ID_SERVER, ClientID};
use crate::news_type::AdviceType;
use crate::order_type::{
    CloneOptions, DestinationID, IMPLICIT_ORDER_ONLY_CAP, MAX_VEH_ORDER_ID, ModifyOrderFlags,
    ModifyTimetableFlags, OrderConditionComparator, OrderConditionVariable, OrderDepotAction,
    OrderDepotActionFlag, OrderDepotActionFlags, OrderDepotTypeFlag, OrderDepotTypeFlags,
    OrderLoadType, OrderNonStopFlag, OrderNonStopFlags, OrderStopLocation, OrderType,
    OrderUnloadType, VehicleOrderID,
};
use crate::raw_field::{read_field, write_field};
use crate::services::{CommandCost, Cost};
use crate::services_assert::{assert_with, not_reached};
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
    STR_NEWS_PLANE_USES_TOO_SHORT_RUNWAY, STR_NEWS_VEHICLE_HAS_DUPLICATE_ENTRY,
    STR_NEWS_VEHICLE_HAS_INVALID_ENTRY, STR_NEWS_VEHICLE_HAS_TOO_FEW_ORDERS,
    STR_NEWS_VEHICLE_HAS_VOID_ORDER,
};
use crate::tile_type::TileIndex;
use crate::timer_game_common::DAYS_IN_LEAP_YEAR;
use crate::timer_game_economy::EconomyDate;
use crate::timer_game_tick::{DAY_TICKS, INVALID_TICKS, TICKS_PER_SECOND};
use crate::vehicle::{VehicleRef, VehicleServices};
use crate::vehicle_base::{VehState, VehStates};
use crate::vehicle_gui::{VehicleInvalidateWindowData, get_window_class_for_vehicle_type};
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::vehiclelist::{VehicleListIdentifier, VehicleListType};
use crate::window_func::{
    close_window_by_id, invalidate_window_classes_data, invalidate_window_data,
    set_window_classes_dirty, set_window_dirty,
};
use crate::window_type::{WindowClass, WindowNumber};
use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::ptr;

/// Opaque live native `OrderList` shell; never dereferenced by Rust.
#[repr(C)]
pub struct NativeOrderList {
    _private: [u8; 0],
}

/// `Order`: the members of one order, in `order_base.h`'s layout.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Order {
    /// The type of order + non-stop flags.
    pub r#type: u8,
    /// Load/unload types, depot order/action types.
    pub flags: u8,
    /// The destination of the order.
    pub dest: u16,
    /// Refit `CargoType`.
    pub refit_cargo: u8,
    /// How long in ticks to wait at the destination.
    pub wait_time: u16,
    /// How long in ticks the journey to this destination should take.
    pub travel_time: u16,
    /// How fast the vehicle may go on the way to the destination.
    pub max_speed: u16,
}

/// `BaseConsist`'s members that Rust reads and writes: the various front vehicle properties
/// that are preserved when autoreplacing, using order-backup or switching front engines
/// within a consist.
#[repr(C)]
pub struct Consist {
    /// How many ticks have passed since this order started.
    pub current_order_time: i32,
    /// How many ticks late (or early if negative) this vehicle is.
    pub lateness_counter: i32,
    /// At what tick of `TimerGameTick::counter` the vehicle should start its timetable.
    pub timetable_start: u64,
    /// When the vehicle last left its unbunching depot.
    pub depot_unbunching_last_departure: u64,
    /// When the vehicle will next try to leave its unbunching depot.
    pub depot_unbunching_next_departure: u64,
    /// How many ticks for a single circumnavigation of the orders.
    pub round_trip_time: i32,
    /// The index to the current real (non-implicit) order.
    pub cur_real_order_index: u8,
    /// The index to the current implicit order.
    pub cur_implicit_order_index: u8,
    /// Used for gradual loading and other miscellaneous things (`VehicleFlags`).
    pub vehicle_flags: u16,
}

/// `Vehicle`'s order members.
#[repr(C)]
pub struct VehicleOrders {
    /// The current order (+ status, like: loading).
    pub current_order: MaybeUninit<Order>,
    /// Pointer to the order list for this vehicle: a native `OrderList` shell or null.
    pub orders: *mut c_void,
    /// Pointer to the next vehicle that shares the order: a native `Vehicle` shell or null.
    pub next_shared: *mut c_void,
    /// Pointer to the previous vehicle that shares the order: a native `Vehicle` shell or null.
    pub previous_shared: *mut c_void,
}

/// `OrderList`'s members, besides its orders.
#[repr(C)]
pub struct List {
    /// NOSAVE: How many manually added orders are there in the list.
    pub num_manual_orders: u8,
    /// NOSAVE: Number of vehicles that share this order list.
    pub num_vehicles: u32,
    /// NOSAVE: Pointer to the first vehicle in the shared order chain, a native shell.
    pub first_shared: *mut c_void,
    /// NOSAVE: Total timetabled duration of the order list.
    pub timetable_duration: i32,
    /// NOSAVE: Total (timetabled or not) duration of the order list.
    pub total_duration: i32,
}

/// `OrderBackup`'s members, besides its orders and its `BaseConsist`.
#[repr(C)]
pub struct Backup {
    /// The user that requested the backup.
    pub user: u32,
    /// Tile of the depot where the order was changed.
    pub tile: u32,
    /// The group the vehicle was part of.
    pub group: u16,
    /// Vehicle this vehicle was a clone of: a native `Vehicle` shell or null.
    pub clone: *const c_void,
}

/// `std::vector<Order>`: an order list's or backup's orders, in storage C++ constructs and
/// destroys through the two hooks.
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
    #[allow(unsafe_code)]
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
    #[allow(unsafe_code)]
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
    /// `std::vector::emplace(begin + index, order)`, with `index` clamped to the end as
    /// `std::ranges::next` clamps it.
    #[allow(unsafe_code)]
    unsafe fn insert(&mut self, index: usize, order: Order) {
        let index = index.min(self.length);
        // SAFETY: After `reserve`, the shifted range and the inserted slot fit in capacity.
        unsafe {
            self.reserve(self.length + 1);
            ptr::copy(
                self.data().add(index),
                self.data().add(index + 1),
                self.length - index,
            );
            self.data().add(index).write(order);
        }
        self.length += 1;
    }
    /// `std::vector::erase(begin + index)` for an index below the length.
    #[allow(unsafe_code)]
    unsafe fn erase(&mut self, index: usize) {
        if index >= self.length {
            return;
        }
        // SAFETY: `index` is below the length, so the shifted tail lies within live Orders.
        unsafe {
            ptr::copy(
                self.data().add(index + 1),
                self.data().add(index),
                self.length - index - 1,
            );
            self.resize(self.length - 1);
        }
    }
    /// `std::rotate` of the order at `from` to `to`, both below the length.
    #[allow(unsafe_code)]
    unsafe fn rotate(&mut self, from: usize, to: usize) {
        if from == to || from >= self.length || to >= self.length {
            return;
        }
        // SAFETY: `from` and `to` are below the length, so both ranges lie within live Orders.
        unsafe {
            let order = self.data().add(from).read();
            if from < to {
                ptr::copy(self.data().add(from + 1), self.data().add(from), to - from);
            } else {
                ptr::copy(self.data().add(to), self.data().add(to + 1), from - to);
            }
            self.data().add(to).write(order);
        }
    }
    /// `assign(first, first + length)` from `length` live orders outside this vector.
    #[allow(unsafe_code)]
    unsafe fn assign(&mut self, src: *const Order, length: usize) {
        // SAFETY: The caller supplies `length` live source Orders outside this storage.
        unsafe {
            self.resize(length);
            if length != 0 {
                ptr::copy_nonoverlapping(src, self.data(), length);
            }
        }
    }
    /// `std::move` of another vector's storage into this empty one.
    #[allow(unsafe_code)]
    unsafe fn take(&mut self, other: *mut Self) {
        // Both owners are live and distinct, and no typed order address is used during the
        // move. Existing C++ Order lifetimes move with the allocation.
        // SAFETY: `other` is a distinct live vector; the swaps exchange its owned storage.
        unsafe {
            std::mem::swap(&mut self.storage, &mut (*other).storage);
            std::mem::swap(&mut self.length, &mut (*other).length);
        }
    }
}
impl Drop for Vector {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: No access remains when the shell destroys its unique owner.
        unsafe {
            (self.destroy)(self.data().cast(), self.storage.len());
        }
    }
}

// The owner exports. Each owner is a live, uniquely owned shell allocation: C++ deletes
// each exactly once, after its last access, and frees no Rust allocation itself.

/// The `BaseConsist` owner, with the members' defaults.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_consist_new() -> *mut Consist {
    Box::into_raw(Box::new(Consist {
        current_order_time: 0,
        lateness_counter: 0,
        timetable_start: 0,
        depot_unbunching_last_departure: 0,
        depot_unbunching_next_departure: 0,
        round_trip_time: 0,
        cur_real_order_index: 0,
        cur_implicit_order_index: 0,
        vehicle_flags: 0,
    }))
}
/// Delete a `BaseConsist` owner.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_delete(p: *mut Consist) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
/// `BaseConsist::ResetDepotUnbunching` for the C++ callers.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_reset(p: *mut Consist) {
    ConsistRef(p).reset_depot_unbunching();
}
/// The consist part of `BaseConsist::CopyConsistPropertiesFrom` for the C++ callers, which
/// copy `name` before it and `service_interval` after it. No member this copies reads or
/// shares storage with `service_interval`, so the later write stores the same values.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_consist_copy(p: *mut Consist, src: *const Consist) {
    ConsistRef(p).copy_consist_properties_from(ConsistRef(src.cast_mut()), || {});
}
/// The vehicle's order members, with the current order uninitialised until C++ constructs it.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_vehicle_orders_new() -> *mut VehicleOrders {
    Box::into_raw(Box::new(VehicleOrders {
        current_order: MaybeUninit::uninit(),
        orders: ptr::null_mut(),
        next_shared: ptr::null_mut(),
        previous_shared: ptr::null_mut(),
    }))
}
/// Delete a vehicle's order members.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_orders_delete(p: *mut VehicleOrders) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
/// An `OrderList`'s members, with their defaults.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orderlist_new() -> *mut List {
    Box::into_raw(Box::new(List {
        num_manual_orders: 0,
        num_vehicles: 0,
        first_shared: ptr::null_mut(),
        timetable_duration: 0,
        total_duration: 0,
    }))
}
/// Delete an `OrderList`'s members.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_orderlist_delete(p: *mut List) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
/// An `OrderBackup`'s members, with their defaults.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orderbackup_new() -> *mut Backup {
    Box::into_raw(Box::new(Backup {
        user: 0,
        tile: TileIndex::INVALID.0,
        group: GroupID::INVALID.0,
        clone: ptr::null(),
    }))
}
/// Delete an `OrderBackup`'s members.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_orderbackup_delete(p: *mut Backup) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
/// An empty `std::vector<Order>` over C++'s construction and destruction hooks.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_vector_new(
    construct: unsafe extern "C" fn(*mut c_void, usize),
    destroy: unsafe extern "C" fn(*mut c_void, usize),
) -> *mut Vector {
    Box::into_raw(Box::new(Vector::new(construct, destroy)))
}
/// Delete a `std::vector<Order>`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_delete(p: *mut Vector) {
    // SAFETY: C++ deletes each Rust-allocated owner exactly once, after its last access.
    unsafe {
        drop(Box::from_raw(p));
    }
}
/// `std::vector::size`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_size(p: *const Vector) -> usize {
    read_field!(p, length)
}
/// `std::vector::data`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_data(p: *const Vector) -> *mut Order {
    // SAFETY: C++ passes a live owner; `data` reads its storage pointer.
    unsafe { (*p).data() }
}
/// `std::vector::resize`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_resize(p: *mut Vector, size: usize) {
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    unsafe {
        (*p).resize(size);
    }
}
/// `std::vector::emplace` of a copy of `src`, which may alias one of its orders.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_insert(
    p: *mut Vector,
    index: usize,
    src: *const Order,
) {
    // Copy before reserve: the original's emplace permits its input to alias a live order.
    // SAFETY: C++ passes a live order.
    let order = unsafe { src.read() };
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    unsafe { (*p).insert(index, order) };
}
/// `std::vector::erase`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_erase(p: *mut Vector, index: usize) {
    // SAFETY: C++ passes the live unique owner for this call; no other reference exists.
    unsafe { (*p).erase(index) };
}
/// `std::vector::assign` from `length` live orders outside the vector.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_order_vector_assign(
    p: *mut Vector,
    src: *const Order,
    length: usize,
) {
    // SAFETY: C++ passes the live unique owner and `length` live source Orders outside it.
    unsafe { (*p).assign(src, length) };
}

/// The orders table: the C++ bindings the bodies call, typed `noexcept` functions over IDs
/// and shells. Owner slots return the Rust owner a live shell holds; pool slots allocate,
/// delete or iterate a pool still stored in C++. No Rust owner borrow spans a call, and a
/// call that deletes a shell consumes it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    // Pool operations and the owners of C++ shells, until the order pools move (#906).
    /// `new OrderList()`.
    pub allocate_list: extern "C" fn() -> *mut c_void,
    /// The `Backup` owner of a live `OrderBackup` shell.
    pub backup: unsafe extern "C" fn(*mut c_void) -> *mut Backup,
    /// `OrderBackup::CanAllocateItem()`.
    pub backup_can_allocate_item: extern "C" fn() -> u8,
    /// The `Consist` owner of a live `OrderBackup` shell.
    pub backup_consist: unsafe extern "C" fn(*mut c_void) -> *mut Consist,
    /// The `Vector` owner of a live `OrderBackup` shell.
    pub backup_vector: unsafe extern "C" fn(*mut c_void) -> *mut Vector,
    /// `new OrderBackup(v, user)`, whose constructor runs this file's body.
    pub create_backup: unsafe extern "C" fn(v: *mut c_void, user: u32) -> *mut c_void,
    /// `delete ob`.
    pub delete_backup: unsafe extern "C" fn(*mut c_void),
    /// `delete list`.
    pub delete_list: unsafe extern "C" fn(*mut c_void),
    /// The `List` owner of a live `OrderList` shell.
    pub list: unsafe extern "C" fn(*mut c_void) -> *mut List,
    /// `OrderList::CanAllocateItem()`.
    pub list_can_allocate_item: extern "C" fn() -> u8,
    /// `OrderBackup::Iterate()`'s next backup at or after `*cursor`, advancing it, or null.
    pub next_backup: unsafe extern "C" fn(cursor: *mut u32) -> *mut c_void,
    /// The `Vector` owner of a live `OrderList` shell.
    pub vector: unsafe extern "C" fn(*mut c_void) -> *mut Vector,

    // Members still stored in C++ shells: one reader or writer each.
    /// `ob->name.clear()`.
    pub backup_clear_name: unsafe extern "C" fn(backup: *mut c_void),
    /// `ob->name`'s bytes.
    pub backup_name: unsafe extern "C" fn(backup: *const c_void) -> crate::services::ByteSpan,
    /// `ob->service_interval`.
    pub backup_service_interval: unsafe extern "C" fn(backup: *const c_void) -> u16,
    /// `ob->name = v->name`.
    pub backup_set_name: unsafe extern "C" fn(backup: *mut c_void, v: *const c_void),
    /// `ob->service_interval = interval`.
    pub backup_set_service_interval: unsafe extern "C" fn(backup: *mut c_void, interval: u16),
    /// `v->name = ob->name`.
    pub vehicle_set_name: unsafe extern "C" fn(v: *mut c_void, backup: *const c_void),
    /// `v->service_interval = interval`.
    pub vehicle_set_service_interval: unsafe extern "C" fn(v: *mut c_void, interval: u16),

    // Lookups still stored in C++ pools (#634, #638).
    /// `BaseStation::GetIfValid(index)`, null where invalid.
    pub base_station_get_if_valid: extern "C" fn(index: u16) -> *mut crate::station::BaseStation,
    /// `Waypoint::GetIfValid(index)`, null where invalid.
    pub waypoint_get_if_valid: extern "C" fn(index: u16) -> *mut crate::station::BaseStation,

    // Unported bodies.
    /// `OrderList::DebugCheckSanity()`, under `WITH_ASSERT`.
    pub debug_check_sanity: unsafe extern "C" fn(list: *mut c_void),
    /// `GetVehicleCannotUseStationReason(v, st)`.
    pub get_vehicle_cannot_use_station_reason:
        unsafe extern "C" fn(v: *mut c_void, st: *const crate::station::BaseStation) -> u32,
    /// `InvalidateVehicleOrder(v, data)`.
    pub invalidate_vehicle_order: unsafe extern "C" fn(v: *mut c_void, data: i32),

    // Commands and the C++ vector the export of `GetNextStoppingStation` fills.
    /// `next_station.push_back(station)` on the export's output vector.
    pub append_station: unsafe extern "C" fn(next_station: *mut c_void, station: u16),
    /// `Command<CMD_CLEAR_ORDER_BACKUP>::Unsafe<CommandCallback>(..)` from the GUI.
    pub clear_order_backup_unsafe: extern "C" fn(tile: u32, user: u32),

    // Tables.
    /// The base vehicle table.
    pub base: &'static VehicleServices,
    /// The depot table, for `Depot::GetIfValid`.
    pub depots: *const crate::depot_cmd::Services,
    /// The vehicle scheduler's table: the controllers' tables and the vehicle pool.
    pub scheduler: *const crate::vehicle::scheduler::SchedulerServices,
    /// The station command table, for its station members and command services.
    pub station_commands: *const crate::station_cmd::CommandServicesTable,
    /// The vehicle transaction table, for `IsUniqueVehicleName`.
    pub vehicle_transactions: *const crate::vehicle_transactions::TransactionServices,
}

/// The process-wide tables a body reads through `Leaves`.
#[allow(unsafe_code)]
impl Leaves {
    /// The shared table.
    pub(crate) fn shared(&self) -> &'static crate::services::Services {
        // SAFETY: The base table names its process-lifetime maintenance and shared tables.
        unsafe { &*(*self.base.maintenance).shared }
    }
    /// The vehicle maintenance table.
    fn maintenance(&self) -> &'static crate::vehicle::maintenance::Services {
        // SAFETY: As `shared`.
        unsafe { &*self.base.maintenance }
    }
    /// The vehicle resolver table, which reads the members still stored in C++.
    fn newgrf(&self) -> &'static crate::newgrf_vehicle::Services {
        self.base.newgrf()
    }
    /// The vehicle scheduler table.
    fn scheduler(&self) -> &'static crate::vehicle::scheduler::SchedulerServices {
        // SAFETY: The table names the process-lifetime scheduler table.
        unsafe { &*self.scheduler }
    }
    /// The station command table.
    fn station_commands(&self) -> &'static crate::station_cmd::CommandServicesTable {
        // SAFETY: The process-lifetime station command table.
        unsafe { &*self.station_commands }
    }
    /// The command services.
    fn commands(&self) -> &'static crate::services::CommandServices {
        // SAFETY: The station command table names the process-lifetime command table.
        unsafe { &*self.station_commands().command }
    }
    /// The native `CommandCost` at `out`, written through the shared cost services.
    fn cost(&self, out: *mut CommandCost) -> Cost<'static> {
        // SAFETY: The station command table names the process-lifetime cost table.
        Cost(out, unsafe { &*self.station_commands().cost })
    }
    /// The depot table.
    fn depots(&self) -> &'static crate::depot_cmd::Services {
        // SAFETY: The table names the process-lifetime depot table.
        unsafe { &*self.depots }
    }
    /// The station catchment table, for `Station::GetIfValid`.
    fn catchment(&self) -> &'static crate::station::CatchmentServices {
        // SAFETY: The shared getter returns the process-lifetime catchment table.
        unsafe { &*(self.shared().station_catchment)() }
    }
    /// The vehicle visit table.
    fn visits(&self) -> &'static crate::vehicle::visits::Services {
        self.base.visits()
    }
}

/// `Vehicle::GetIfValid(index)`.
fn vehicle_get_if_valid(index: VehicleID, s: &Leaves) -> Option<VehicleRef> {
    crate::vehicle_base::get_if_valid(&s.scheduler().pool(), index)
}

/// The record of a live non-null vehicle shell.
#[allow(unsafe_code)]
fn record(shell: *mut c_void, s: &Leaves) -> VehicleRef {
    // SAFETY: Each caller passes a live non-null `Vehicle` shell of the serialized game.
    unsafe { VehicleRef::from_shell(shell, s.base) }
}

/// The record of a vehicle shell, `None` for null.
fn record_if(shell: *mut c_void, s: &Leaves) -> Option<VehicleRef> {
    (!shell.is_null()).then(|| record(shell, s))
}

/// The consist owner of a live vehicle record.
#[allow(unsafe_code)]
pub(crate) fn consist(v: VehicleRef) -> ConsistRef {
    ConsistRef(read_field!(v.0.as_ptr(), consist))
}

/// The order members of a live vehicle record.
#[allow(unsafe_code)]
fn vehicle_orders(v: VehicleRef) -> *mut VehicleOrders {
    read_field!(v.0.as_ptr(), orders)
}

/// `BaseConsist`'s members in a live `Consist` owner, read and written in place.
#[derive(Clone, Copy)]
pub(crate) struct ConsistRef(*mut Consist);

#[allow(unsafe_code)]
impl ConsistRef {
    /// `current_order_time`.
    pub(crate) fn current_order_time(self) -> i32 {
        read_field!(self.0, current_order_time)
    }
    pub(crate) fn set_current_order_time(self, value: i32) {
        write_field!(self.0, current_order_time, value);
    }
    /// `lateness_counter`.
    pub(crate) fn lateness_counter(self) -> i32 {
        read_field!(self.0, lateness_counter)
    }
    pub(crate) fn set_lateness_counter(self, value: i32) {
        write_field!(self.0, lateness_counter, value);
    }
    /// `timetable_start`.
    pub(crate) fn timetable_start(self) -> u64 {
        read_field!(self.0, timetable_start)
    }
    pub(crate) fn set_timetable_start(self, value: u64) {
        write_field!(self.0, timetable_start, value);
    }
    /// `depot_unbunching_last_departure`.
    pub(crate) fn depot_unbunching_last_departure(self) -> u64 {
        read_field!(self.0, depot_unbunching_last_departure)
    }
    pub(crate) fn set_depot_unbunching_last_departure(self, value: u64) {
        write_field!(self.0, depot_unbunching_last_departure, value);
    }
    /// `depot_unbunching_next_departure`.
    pub(crate) fn depot_unbunching_next_departure(self) -> u64 {
        read_field!(self.0, depot_unbunching_next_departure)
    }
    pub(crate) fn set_depot_unbunching_next_departure(self, value: u64) {
        write_field!(self.0, depot_unbunching_next_departure, value);
    }
    /// `round_trip_time`.
    pub(crate) fn round_trip_time(self) -> i32 {
        read_field!(self.0, round_trip_time)
    }
    pub(crate) fn set_round_trip_time(self, value: i32) {
        write_field!(self.0, round_trip_time, value);
    }
    /// `cur_real_order_index`.
    pub(crate) fn cur_real_order_index(self) -> VehicleOrderID {
        VehicleOrderID(read_field!(self.0, cur_real_order_index))
    }
    pub(crate) fn set_cur_real_order_index(self, value: VehicleOrderID) {
        write_field!(self.0, cur_real_order_index, value.0);
    }
    /// `cur_implicit_order_index`.
    pub(crate) fn cur_implicit_order_index(self) -> VehicleOrderID {
        VehicleOrderID(read_field!(self.0, cur_implicit_order_index))
    }
    pub(crate) fn set_cur_implicit_order_index(self, value: VehicleOrderID) {
        write_field!(self.0, cur_implicit_order_index, value.0);
    }
    /// `vehicle_flags`.
    pub(crate) fn vehicle_flags(self) -> VehicleFlags {
        VehicleFlags(read_field!(self.0, vehicle_flags))
    }
    pub(crate) fn set_vehicle_flags(self, value: VehicleFlags) {
        write_field!(self.0, vehicle_flags, value.0);
    }

    /// `BaseConsist::ResetDepotUnbunching`: resets all the data used for depot unbunching.
    pub(crate) fn reset_depot_unbunching(self) {
        self.set_depot_unbunching_last_departure(0);
        self.set_depot_unbunching_next_departure(0);
        self.set_round_trip_time(0);
    }

    /// `BaseConsist::CopyConsistPropertiesFrom`, for the members this owner holds: copy
    /// properties of other `BaseConsist`. The callers copy `name` before these members;
    /// `copy_service_interval` copies `service_interval`, which the caller's record holds,
    /// at the original's point between the timetable members and the order indices.
    pub(crate) fn copy_consist_properties_from(
        self,
        src: Self,
        copy_service_interval: impl FnOnce(),
    ) {
        if ptr::eq(self.0, src.0) {
            return;
        }

        self.set_current_order_time(src.current_order_time());
        self.set_lateness_counter(src.lateness_counter());
        self.set_timetable_start(src.timetable_start());

        copy_service_interval();

        self.set_cur_real_order_index(src.cur_real_order_index());
        self.set_cur_implicit_order_index(src.cur_implicit_order_index());

        if src.vehicle_flags().test(VehicleFlag::TIMETABLE_STARTED) {
            self.set_vehicle_flags(self.vehicle_flags().set(VehicleFlag::TIMETABLE_STARTED));
        }
        if src.vehicle_flags().test(VehicleFlag::AUTOFILL_TIMETABLE) {
            self.set_vehicle_flags(self.vehicle_flags().set(VehicleFlag::AUTOFILL_TIMETABLE));
        }
        if src
            .vehicle_flags()
            .test(VehicleFlag::AUTOFILL_PRESERVE_WAIT_TIME)
        {
            self.set_vehicle_flags(
                self.vehicle_flags()
                    .set(VehicleFlag::AUTOFILL_PRESERVE_WAIT_TIME),
            );
        }
        if src
            .vehicle_flags()
            .test(VehicleFlag::SERVICE_INTERVAL_IS_PERCENT)
            != self
                .vehicle_flags()
                .test(VehicleFlag::SERVICE_INTERVAL_IS_PERCENT)
        {
            self.set_vehicle_flags(
                self.vehicle_flags()
                    .flip(VehicleFlag::SERVICE_INTERVAL_IS_PERCENT),
            );
        }
        if src
            .vehicle_flags()
            .test(VehicleFlag::SERVICE_INTERVAL_IS_CUSTOM)
        {
            self.set_vehicle_flags(
                self.vehicle_flags()
                    .set(VehicleFlag::SERVICE_INTERVAL_IS_CUSTOM),
            );
        }
    }
}

/// `BaseConsist::ResetDepotUnbunching` on a vehicle: resets all the data used for depot
/// unbunching.
pub(crate) fn reset_depot_unbunching(v: VehicleRef) {
    consist(v).reset_depot_unbunching();
}

impl Default for Order {
    /// `Order()`: the members' defaults.
    fn default() -> Self {
        Self {
            r#type: 0,
            flags: 0,
            dest: 0,
            refit_cargo: CARGO_NO_REFIT,
            wait_time: 0,
            travel_time: 0,
            max_speed: u16::MAX,
        }
    }
}

impl Order {
    /// This order, written in place for as long as it is borrowed.
    pub(crate) fn edit(&mut self) -> OrderRef {
        OrderRef(ptr::from_mut(self))
    }
}

/// A live `Order` read in place, such as `Vehicle::current_order`: each getter loads only
/// the members its `order_base.h` getter reads, when it is called. A view of a borrowed
/// order lives no longer than the borrow.
#[derive(Clone, Copy)]
pub(crate) struct OrderView<'a>(*const Order, std::marker::PhantomData<&'a Order>);

#[allow(unsafe_code)]
impl OrderView<'_> {
    /// View the order at `order`.
    ///
    /// # Safety
    /// `order` is a live, initialized `Order` of the serialized game state for every read
    /// made through the view.
    pub(crate) unsafe fn of(order: *const Order) -> Self {
        Self(order, std::marker::PhantomData)
    }
    /// The order's address, for its in-place writers.
    pub(crate) fn as_ptr(self) -> *const Order {
        self.0
    }
    /// `Order::IsType`: check whether this order is of the given type.
    #[inline]
    pub(crate) fn is_type(self, r#type: OrderType) -> bool {
        self.kind() == r#type
    }
    /// `Order::GetType`: get the type of order of this order.
    #[inline]
    pub(crate) fn kind(self) -> OrderType {
        OrderType(gb_u8(read_field!(self.0, r#type), 0, 4))
    }
    /// `Order::IsGotoOrder`: is this a 'goto' order with a real destination? True if the type
    /// is either `OT_GOTO_WAYPOINT`, `OT_GOTO_DEPOT` or `OT_GOTO_STATION`.
    #[inline]
    pub(crate) fn is_goto_order(self) -> bool {
        self.is_type(OrderType::OT_GOTO_WAYPOINT)
            || self.is_type(OrderType::OT_GOTO_DEPOT)
            || self.is_type(OrderType::OT_GOTO_STATION)
    }
    /// `Order::GetDestination`: gets the destination of this order.
    ///
    /// Precondition: `IsType(OT_GOTO_WAYPOINT) || IsType(OT_GOTO_DEPOT) ||
    /// IsType(OT_GOTO_STATION)`.
    #[inline]
    pub(crate) fn destination(self) -> DestinationID {
        DestinationID(read_field!(self.0, dest))
    }
    /// The raw `dest`, for the `NewGRF` variable that returns it as an integer.
    #[inline]
    pub(crate) fn destination_raw(self) -> u16 {
        self.destination().0
    }
    /// `Order::IsRefit`: is this order a refit order.
    #[inline]
    pub(crate) fn is_refit(self) -> bool {
        let refit_cargo = read_field!(self.0, refit_cargo);
        refit_cargo < NUM_CARGO || refit_cargo == CARGO_AUTO_REFIT
    }
    /// `Order::IsAutoRefit`: is this order a auto-refit order.
    #[inline]
    pub(crate) fn is_auto_refit(self) -> bool {
        read_field!(self.0, refit_cargo) == CARGO_AUTO_REFIT
    }
    /// `Order::GetRefitCargo`: get the cargo to to refit to.
    #[inline]
    pub(crate) fn refit_cargo(self) -> CargoType {
        CargoType(read_field!(self.0, refit_cargo))
    }
    /// `Order::IsFullLoadOrder`: is this order a `OrderLoadType::FullLoad` or
    /// `OrderLoadType::FullLoadAny`?
    #[inline]
    pub(crate) fn is_full_load(self) -> bool {
        let r#type = self.load_type();
        r#type == OrderLoadType::FullLoad || r#type == OrderLoadType::FullLoadAny
    }
    /// `Order::GetLoadType`: how must the consist be loaded?
    #[inline]
    pub(crate) fn load_type(self) -> OrderLoadType {
        OrderLoadType(gb_u8(read_field!(self.0, flags), 4, 3))
    }
    /// `Order::GetUnloadType`: how must the consist be unloaded?
    #[inline]
    pub(crate) fn unload_type(self) -> OrderUnloadType {
        OrderUnloadType(gb_u8(read_field!(self.0, flags), 0, 3))
    }
    /// `Order::GetNonStopType`: at which stations must we stop?
    #[inline]
    pub(crate) fn non_stop(self) -> OrderNonStopFlags {
        OrderNonStopFlags(gb_u8(read_field!(self.0, r#type), 6, 2))
    }
    /// `Order::GetStopLocation`: where must we stop at the platform?
    #[inline]
    pub(crate) fn stop_location(self) -> OrderStopLocation {
        OrderStopLocation(gb_u8(read_field!(self.0, r#type), 4, 2))
    }
    /// `Order::GetDepotOrderType`: what caused us going to the depot?
    #[inline]
    pub(crate) fn depot_type(self) -> OrderDepotTypeFlags {
        OrderDepotTypeFlags(gb_u8(read_field!(self.0, flags), 0, 3))
    }
    /// `Order::GetDepotActionType`: what are we going to do when in the depot.
    #[inline]
    pub(crate) fn depot_action_flags(self) -> OrderDepotActionFlags {
        OrderDepotActionFlags(gb_u8(read_field!(self.0, flags), 3, 4))
    }
    /// `Order::GetConditionVariable`: what variable do we have to compare?
    #[inline]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the five bits of `GB(dest, 11, 5)`"
    )]
    pub(crate) fn condition_variable(self) -> OrderConditionVariable {
        // `GB` of the 16-bit destination's top five bits always fits the `uint8_t` enum.
        OrderConditionVariable(gb_u16(read_field!(self.0, dest), 11, 5) as u8)
    }
    /// `Order::GetConditionComparator`: what is the comparator to use?
    #[inline]
    pub(crate) fn condition_comparator(self) -> OrderConditionComparator {
        OrderConditionComparator(gb_u8(read_field!(self.0, r#type), 5, 3))
    }
    /// `Order::GetConditionSkipToOrder`: get the order to skip to.
    #[inline]
    pub(crate) fn condition_skip_to_order(self) -> VehicleOrderID {
        VehicleOrderID(read_field!(self.0, flags))
    }
    /// `Order::GetConditionValue`: get the value to base the skip on.
    #[inline]
    pub(crate) fn condition_value(self) -> u16 {
        gb_u16(read_field!(self.0, dest), 0, 11)
    }
    /// `Order::IsWaitTimetabled`: does this order have an explicit wait time set?
    #[inline]
    pub(crate) fn is_wait_timetabled(self) -> bool {
        if self.is_type(OrderType::OT_CONDITIONAL) {
            read_field!(self.0, wait_time) > 0
        } else {
            has_bit(read_field!(self.0, flags), 3)
        }
    }
    /// `Order::IsTravelTimetabled`: does this order have an explicit travel time set?
    #[inline]
    pub(crate) fn is_travel_timetabled(self) -> bool {
        if self.is_type(OrderType::OT_CONDITIONAL) {
            read_field!(self.0, travel_time) > 0
        } else {
            has_bit(read_field!(self.0, flags), 7)
        }
    }
    /// `Order::GetTimetabledWait`: get the time in ticks a vehicle should wait at the
    /// destination or 0 if it's not timetabled.
    #[inline]
    pub(crate) fn timetabled_wait(self) -> u16 {
        if self.is_wait_timetabled() {
            read_field!(self.0, wait_time)
        } else {
            0
        }
    }
    /// `Order::GetTimetabledTravel`: get the time in ticks a vehicle should take to reach the
    /// destination or 0 if it's not timetabled.
    #[inline]
    pub(crate) fn timetabled_travel(self) -> u16 {
        if self.is_travel_timetabled() {
            read_field!(self.0, travel_time)
        } else {
            0
        }
    }
    /// `Order::GetWaitTime`: get the time in ticks a vehicle will probably wait at the
    /// destination (timetabled or not).
    #[inline]
    pub(crate) fn wait_time(self) -> u16 {
        read_field!(self.0, wait_time)
    }
    /// `Order::GetTravelTime`: get the time in ticks a vehicle will probably take to reach the
    /// destination (timetabled or not).
    #[inline]
    pub(crate) fn travel_time(self) -> u16 {
        read_field!(self.0, travel_time)
    }
    /// `Order::GetMaxSpeed`: get the maxmimum speed in km-ish/h a vehicle is allowed to reach
    /// on the way to the destination.
    #[inline]
    pub(crate) fn max_speed(self) -> u16 {
        read_field!(self.0, max_speed)
    }
    /// `Order::IsCompletelyTimetabled`: checks if `travel_time` and `wait_time` apply to this
    /// order and if they are timetabled.
    pub(crate) fn is_completely_timetabled(self) -> bool {
        if !self.is_travel_timetabled() && !self.is_type(OrderType::OT_CONDITIONAL) {
            return false;
        }
        if !self.is_wait_timetabled()
            && self.is_type(OrderType::OT_GOTO_STATION)
            && !self.non_stop().test(OrderNonStopFlag::NoDestination)
        {
            return false;
        }
        true
    }
    /// `Order::Equals`: does this order have the same type, flags and destination?
    pub(crate) fn equals(self, other: OrderView<'_>) -> bool {
        /* In case of go to nearest depot orders we need "only" compare the flags
         * with the other and not the nearest depot order bit or the actual
         * destination because those get clear/filled in during the order
         * evaluation. If we do not do this the order will continuously be seen as
         * a different order and it will try to find a "nearest depot" every tick. */
        if (self.is_type(OrderType::OT_GOTO_DEPOT)
            && read_field!(self.0, r#type) == read_field!(other.0, r#type))
            && (self
                .depot_action_flags()
                .test(OrderDepotActionFlag::NearestDepot)
                || other
                    .depot_action_flags()
                    .test(OrderDepotActionFlag::NearestDepot))
        {
            return self.depot_type() == other.depot_type()
                && self
                    .depot_action_flags()
                    .reset(OrderDepotActionFlag::NearestDepot)
                    == other
                        .depot_action_flags()
                        .reset(OrderDepotActionFlag::NearestDepot);
        }

        read_field!(self.0, r#type) == read_field!(other.0, r#type)
            && read_field!(self.0, flags) == read_field!(other.0, flags)
            && read_field!(self.0, dest) == read_field!(other.0, dest)
    }
    /// `Order::MapOldOrder`: pack this order into a 16 bits integer as close to the TTD
    /// representation as possible.
    pub(crate) fn map_old_order(self) -> u16 {
        let mut order = u16::from(self.kind().0);
        match self.kind() {
            OrderType::OT_GOTO_STATION => {
                if self.unload_type() == OrderUnloadType::Unload {
                    crate::bitmath_func::set_bit16(&mut order, 5);
                }
                if self.is_full_load() {
                    crate::bitmath_func::set_bit16(&mut order, 6);
                }
                if self.non_stop().test(OrderNonStopFlag::NoIntermediate) {
                    crate::bitmath_func::set_bit16(&mut order, 7);
                }
                order |= gb_u16(self.destination().0, 0, 8) << 8;
            }
            OrderType::OT_GOTO_DEPOT => {
                if !self.depot_type().test(OrderDepotTypeFlag::PartOfOrders) {
                    crate::bitmath_func::set_bit16(&mut order, 6);
                }
                crate::bitmath_func::set_bit16(&mut order, 7);
                order |= gb_u16(self.destination().0, 0, 8) << 8;
            }
            OrderType::OT_LOADING => {
                if self.is_full_load() {
                    crate::bitmath_func::set_bit16(&mut order, 6);
                }
                /* If both "no load" and "no unload" are set, return nothing order instead */
                if self.load_type() == OrderLoadType::NoLoad
                    && self.unload_type() == OrderUnloadType::NoUnload
                {
                    order = u16::from(OrderType::OT_NOTHING.0);
                }
            }
            _ => {}
        }
        order
    }
    /// `Order::ShouldStopAtStation`: check whether the given vehicle should stop at the given
    /// station based on this order and the non-stop settings. `last_station_visited` is the
    /// vehicle's `last_station_visited`.
    pub(crate) fn should_stop_at_station(
        self,
        last_station_visited: StationID,
        station: StationID,
    ) -> bool {
        let is_dest_station =
            self.is_type(OrderType::OT_GOTO_STATION) && read_field!(self.0, dest) == station.0;

        (!self.is_type(OrderType::OT_GOTO_DEPOT)
            || self.depot_type().test(OrderDepotTypeFlag::PartOfOrders))
            // Do stop only when we've not just been there
            && last_station_visited != station
            /* Finally do stop when there is no non-stop flag set for this type of station. */
            && !self.non_stop().test(if is_dest_station {
                OrderNonStopFlag::NoDestination
            } else {
                OrderNonStopFlag::NoIntermediate
            })
    }
    /// `Order::CanLoadOrUnload`.
    pub(crate) fn can_load_or_unload(self) -> bool {
        (self.is_type(OrderType::OT_GOTO_STATION) || self.is_type(OrderType::OT_IMPLICIT))
            && !self.non_stop().test(OrderNonStopFlag::NoDestination)
            && (self.load_type() != OrderLoadType::NoLoad
                || self.unload_type() != OrderUnloadType::NoUnload)
    }
    /// `Order::CanLeaveWithCargo`: a vehicle can leave the current station with cargo if:
    /// 1. it can load cargo here OR
    /// 2. (a) it could leave the last station with cargo AND
    ///    (b) it doesn't have to unload all cargo here.
    pub(crate) fn can_leave_with_cargo(self, has_cargo: bool) -> bool {
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

/// A live `Order` written in place: a list's order or a vehicle's `current_order`. Each
/// setter writes only the members its `order_base.h` or `order_cmd.cpp` setter writes.
#[derive(Clone, Copy)]
pub(crate) struct OrderRef(*mut Order);

#[allow(unsafe_code)]
impl OrderRef {
    /// Write the order at `order` in place.
    ///
    /// # Safety
    /// As [`OrderView::of`], and `order` is writable.
    pub(crate) unsafe fn of(order: *mut Order) -> Self {
        Self(order)
    }
    /// This order, read in place.
    pub(crate) fn view(self) -> OrderView<'static> {
        // SAFETY: The handle names a live order.
        unsafe { OrderView::of(self.0) }
    }
    /// `Order::Free`: 'Free' the order. Only use on `current_order` vehicle orders!
    pub(crate) fn free(self) {
        write_field!(self.0, r#type, OrderType::OT_NOTHING.0);
        write_field!(self.0, flags, 0);
        write_field!(self.0, dest, 0);
    }
    /// `Order::MakeGoToStation`: makes this order a Go To Station order.
    pub(crate) fn make_go_to_station(self, destination: StationID) {
        write_field!(self.0, r#type, OrderType::OT_GOTO_STATION.0);
        write_field!(self.0, flags, 0);
        write_field!(self.0, dest, destination.0);
    }
    /// `Order::MakeGoToDepot`: makes this order a Go To Depot order. `order` says whether it
    /// is a 'default' order or an overridden vehicle order, `non_stop_type` how to get to the
    /// depot, `action` what to do in the depot and `cargo` the cargo type to change to.
    pub(crate) fn make_go_to_depot(
        self,
        destination: DestinationID,
        order: OrderDepotTypeFlags,
        non_stop_type: OrderNonStopFlags,
        action: OrderDepotActionFlags,
        cargo: CargoType,
    ) {
        write_field!(self.0, r#type, OrderType::OT_GOTO_DEPOT.0);
        self.set_depot_order_type(order);
        self.set_depot_action_type(action);
        self.set_non_stop_type(non_stop_type);
        write_field!(self.0, dest, destination.0);
        self.set_refit(cargo);
    }
    /// `Order::MakeGoToWaypoint`: makes this order a Go To Waypoint order.
    pub(crate) fn make_go_to_waypoint(self, destination: StationID) {
        write_field!(self.0, r#type, OrderType::OT_GOTO_WAYPOINT.0);
        write_field!(self.0, flags, 0);
        write_field!(self.0, dest, destination.0);
    }
    /// `Order::MakeLoading`: makes this order a Loading order. `ordered` says whether this is
    /// an ordered stop.
    pub(crate) fn make_loading(self, ordered: bool) {
        write_field!(self.0, r#type, OrderType::OT_LOADING.0);
        if !ordered {
            write_field!(self.0, flags, 0);
        }
    }
    /// `Order::MakeLeaveStation`: makes this order a Leave Station order.
    pub(crate) fn make_leave_station(self) {
        write_field!(self.0, r#type, OrderType::OT_LEAVESTATION.0);
        write_field!(self.0, flags, 0);
    }
    /// `Order::MakeDummy`: makes this order a Dummy order.
    pub(crate) fn make_dummy(self) {
        write_field!(self.0, r#type, OrderType::OT_DUMMY.0);
        write_field!(self.0, flags, 0);
    }
    /// `Order::MakeConditional`: makes this order an conditional order. `order` is the order
    /// to jump to.
    pub(crate) fn make_conditional(self, order: VehicleOrderID) {
        write_field!(self.0, r#type, OrderType::OT_CONDITIONAL.0);
        write_field!(self.0, flags, order.0);
        write_field!(self.0, dest, 0);
    }
    /// `Order::MakeImplicit`: makes this order an implicit order.
    pub(crate) fn make_implicit(self, destination: StationID) {
        write_field!(self.0, r#type, OrderType::OT_IMPLICIT.0);
        write_field!(self.0, dest, destination.0);
    }
    /// `Order::SetDestination`: sets the destination of this order.
    pub(crate) fn set_destination(self, destination: DestinationID) {
        write_field!(self.0, dest, destination.0);
    }
    /// `Order::SetRefit`: make this depot/station order also a refit order.
    pub(crate) fn set_refit(self, cargo: CargoType) {
        write_field!(self.0, refit_cargo, cargo.0);
    }
    /// `Order::SetLoadType`: set how the consist must be loaded.
    pub(crate) fn set_load_type(self, load_type: OrderLoadType) {
        let mut flags = read_field!(self.0, flags);
        sb_u8(&mut flags, 4, 3, load_type.0);
        write_field!(self.0, flags, flags);
    }
    /// `Order::SetUnloadType`: set how the consist must be unloaded.
    pub(crate) fn set_unload_type(self, unload_type: OrderUnloadType) {
        let mut flags = read_field!(self.0, flags);
        sb_u8(&mut flags, 0, 3, unload_type.0);
        write_field!(self.0, flags, flags);
    }
    /// `Order::SetNonStopType`: set whether we must stop at stations or not.
    pub(crate) fn set_non_stop_type(self, non_stop_type: OrderNonStopFlags) {
        let mut r#type = read_field!(self.0, r#type);
        sb_u8(&mut r#type, 6, 2, non_stop_type.0);
        write_field!(self.0, r#type, r#type);
    }
    /// `Order::SetStopLocation`: set where we must stop at the platform.
    pub(crate) fn set_stop_location(self, stop_location: OrderStopLocation) {
        let mut r#type = read_field!(self.0, r#type);
        sb_u8(&mut r#type, 4, 2, stop_location.0);
        write_field!(self.0, r#type, r#type);
    }
    /// `Order::SetDepotOrderType`: set the cause to go to the depot.
    pub(crate) fn set_depot_order_type(self, depot_order_type: OrderDepotTypeFlags) {
        let mut flags = read_field!(self.0, flags);
        sb_u8(&mut flags, 0, 3, depot_order_type.0);
        write_field!(self.0, flags, flags);
    }
    /// `Order::SetDepotActionType`: set what we are going to do in the depot.
    pub(crate) fn set_depot_action_type(self, depot_service_type: OrderDepotActionFlags) {
        let mut flags = read_field!(self.0, flags);
        sb_u8(&mut flags, 3, 4, depot_service_type.0);
        write_field!(self.0, flags, flags);
    }
    /// `Order::SetConditionVariable`: set variable we have to compare.
    pub(crate) fn set_condition_variable(self, condition_variable: OrderConditionVariable) {
        let mut dest = read_field!(self.0, dest);
        sb_u16(&mut dest, 11, 5, u16::from(condition_variable.0));
        write_field!(self.0, dest, dest);
    }
    /// `Order::SetConditionComparator`: set the comparator to use.
    pub(crate) fn set_condition_comparator(self, condition_comparator: OrderConditionComparator) {
        let mut r#type = read_field!(self.0, r#type);
        sb_u8(&mut r#type, 5, 3, condition_comparator.0);
        write_field!(self.0, r#type, r#type);
    }
    /// `Order::SetConditionSkipToOrder`: get the order to skip to.
    pub(crate) fn set_condition_skip_to_order(self, order_id: VehicleOrderID) {
        write_field!(self.0, flags, order_id.0);
    }
    /// `Order::SetConditionValue`: set the value to base the skip on.
    pub(crate) fn set_condition_value(self, value: u16) {
        let mut dest = read_field!(self.0, dest);
        sb_u16(&mut dest, 0, 11, value);
        write_field!(self.0, dest, dest);
    }
    /// `Order::SetWaitTimetabled`: set if the wait time is explicitly timetabled (unless the
    /// order is conditional).
    pub(crate) fn set_wait_timetabled(self, timetabled: bool) {
        if !self.view().is_type(OrderType::OT_CONDITIONAL) {
            let mut flags = read_field!(self.0, flags);
            crate::bitmath_func::assign_bit8(&mut flags, 3, timetabled);
            write_field!(self.0, flags, flags);
        }
    }
    /// `Order::SetTravelTimetabled`: set if the travel time is explicitly timetabled (unless
    /// the order is conditional).
    pub(crate) fn set_travel_timetabled(self, timetabled: bool) {
        if !self.view().is_type(OrderType::OT_CONDITIONAL) {
            let mut flags = read_field!(self.0, flags);
            crate::bitmath_func::assign_bit8(&mut flags, 7, timetabled);
            write_field!(self.0, flags, flags);
        }
    }
    /// `Order::SetWaitTime`: set the time in ticks to wait at the destination.
    pub(crate) fn set_wait_time(self, time: u16) {
        write_field!(self.0, wait_time, time);
    }
    /// `Order::SetTravelTime`: set the time in ticks to take for travelling to the destination.
    pub(crate) fn set_travel_time(self, time: u16) {
        write_field!(self.0, travel_time, time);
    }
    /// `Order::SetMaxSpeed`: set the maxmimum speed in km-ish/h a vehicle is allowed to reach
    /// on the way to the destination.
    pub(crate) fn set_max_speed(self, speed: u16) {
        write_field!(self.0, max_speed, speed);
    }
    /// `Order::AssignOrder`: assign data to an order (from another order), member by member
    /// in the original's order.
    pub(crate) fn assign_order(self, other: OrderView<'_>) {
        let other = other.as_ptr();
        write_field!(self.0, r#type, read_field!(other, r#type));
        write_field!(self.0, flags, read_field!(other, flags));
        write_field!(self.0, dest, read_field!(other, dest));

        write_field!(self.0, refit_cargo, read_field!(other, refit_cargo));

        write_field!(self.0, wait_time, read_field!(other, wait_time));
        write_field!(self.0, travel_time, read_field!(other, travel_time));
        write_field!(self.0, max_speed, read_field!(other, max_speed));
    }
    /// The defaulted copy assignment `*this = other`.
    pub(crate) fn copy_assign(self, other: Order) {
        // SAFETY: The handle names a live, writable order; the store creates no reference.
        unsafe { self.0.write(other) };
    }
}

/// `Order::Free` on `v->current_order`: 'Free' the order.
pub(crate) fn free(v: VehicleRef) {
    current_order(v).free();
}
/// `Order::MakeDummy` on `v->current_order`: makes this order a Dummy order.
pub(crate) fn make_dummy(v: VehicleRef) {
    current_order(v).make_dummy();
}
/// `Order::MakeLeaveStation` on `v->current_order`: makes this order a Leave Station order.
pub(crate) fn make_leave_station(v: VehicleRef) {
    current_order(v).make_leave_station();
}

/// `v->current_order`, written in place.
#[allow(unsafe_code)]
pub(crate) fn current_order(v: VehicleRef) -> OrderRef {
    // SAFETY: Vehicle construction starts the order's lifetime before publication.
    unsafe { OrderRef::of(v.current_order_pointer()) }
}

/// An `OrderList` shell with its two Rust owners, resolved once by an entry and again only
/// after a call that can replace the vehicle's list.
#[derive(Clone, Copy)]
pub(crate) struct OrderListRef {
    shell: *mut c_void,
    list: *mut List,
    vector: *mut Vector,
}

#[allow(unsafe_code)]
impl OrderListRef {
    /// The live `OrderList` shell `shell` and its owners.
    pub(crate) fn of(shell: *mut c_void, s: &Leaves) -> Self {
        // SAFETY: Each caller passes a live non-null `OrderList` shell.
        let (list, vector) = unsafe { ((s.list)(shell), (s.vector)(shell)) };
        Self {
            shell,
            list,
            vector,
        }
    }
    /// The native shell.
    pub(crate) fn shell(self) -> *mut c_void {
        self.shell
    }
    /// The number of stored orders: `GetOrders().size()`, without the `VehicleOrderID`
    /// narrowing of `GetNumOrders`.
    pub(crate) fn len(self) -> usize {
        read_field!(self.vector, length)
    }
    /// `orders[index]` for an index below [`Self::len`], read and written in place.
    pub(crate) fn order(self, index: usize) -> OrderRef {
        // SAFETY: An index below the length names a live stored order of the span.
        unsafe { OrderRef::of((*self.vector).data().add(index)) }
    }
    /// The address of the stored orders, null while the vector has no capacity.
    fn data(self) -> *mut Order {
        // SAFETY: The live vector; `data` reads its storage pointer.
        unsafe { (*self.vector).data() }
    }
    /// `OrderList::GetFirstOrder`: get the first order of the order chain.
    pub(crate) fn get_first_order(self) -> VehicleOrderID {
        if self.len() == 0 {
            VehicleOrderID::INVALID_VEH_ORDER_ID
        } else {
            VehicleOrderID(0)
        }
    }
    /// `OrderList::GetOrderAt`: get a certain order of the order chain, `None` beyond its
    /// orders.
    pub(crate) fn get_order_at(self, index: VehicleOrderID) -> Option<OrderRef> {
        if index >= self.get_num_orders() {
            return None;
        }
        Some(self.order(usize::from(index.0)))
    }
    /// `OrderList::GetLastOrder`: get the last order of the order chain.
    pub(crate) fn get_last_order(self) -> VehicleOrderID {
        if self.len() == 0 {
            VehicleOrderID::INVALID_VEH_ORDER_ID
        } else {
            VehicleOrderID(self.get_num_orders().0.wrapping_sub(1))
        }
    }
    /// `OrderList::GetNext`: get the order after the given one or the first one, if the given
    /// one is the last one.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the original's static_cast"
    )]
    pub(crate) fn get_next(self, cur: VehicleOrderID) -> VehicleOrderID {
        if self.len() == 0 {
            return VehicleOrderID::INVALID_VEH_ORDER_ID;
        }
        // `(cur + 1) % GetNumOrders()` in `int`, narrowed to `VehicleOrderID`.
        VehicleOrderID(((i32::from(cur.0) + 1) % i32::from(self.get_num_orders().0)) as u8)
    }
    /// `OrderList::GetNumOrders`: get number of orders in the order list.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the original's static_cast"
    )]
    pub(crate) fn get_num_orders(self) -> VehicleOrderID {
        // The original's static_cast of `std::size(orders)` to `VehicleOrderID`.
        VehicleOrderID(self.len() as u8)
    }
    /// `OrderList::GetNumManualOrders`: get number of manually added orders in the order list.
    pub(crate) fn get_num_manual_orders(self) -> VehicleOrderID {
        VehicleOrderID(read_field!(self.list, num_manual_orders))
    }
    fn set_num_manual_orders(self, value: VehicleOrderID) {
        write_field!(self.list, num_manual_orders, value.0);
    }
    /// `OrderList::IsShared`: is this a shared order list?
    pub(crate) fn is_shared(self) -> bool {
        read_field!(self.list, num_vehicles) > 1
    }
    /// `OrderList::GetFirstSharedVehicle`: get the first vehicle of this vehicle chain.
    pub(crate) fn get_first_shared_vehicle(self, s: &Leaves) -> Option<VehicleRef> {
        record_if(read_field!(self.list, first_shared), s)
    }
    fn first_shared_shell(self) -> *mut c_void {
        read_field!(self.list, first_shared)
    }
    fn set_first_shared(self, v: *mut c_void) {
        write_field!(self.list, first_shared, v);
    }
    /// `OrderList::GetNumVehicles`: return the number of vehicles that share this orders list.
    pub(crate) fn get_num_vehicles(self) -> u32 {
        read_field!(self.list, num_vehicles)
    }
    fn set_num_vehicles(self, value: u32) {
        write_field!(self.list, num_vehicles, value);
    }
    /// `OrderList::AddVehicle`: adds the given vehicle to this shared order list. This is
    /// supposed to be called after the vehicle has been inserted into the shared vehicle chain.
    fn add_vehicle(self) {
        self.set_num_vehicles(self.get_num_vehicles().wrapping_add(1));
    }
    /// `OrderList::RemoveVehicle`: removes the vehicle from the shared order list. This is
    /// supposed to be called when the vehicle is still in the chain.
    fn remove_vehicle(self, v: VehicleRef) {
        self.set_num_vehicles(self.get_num_vehicles().wrapping_sub(1));
        if v.shell() == self.first_shared_shell() {
            self.set_first_shared(read_field!(vehicle_orders(v), next_shared));
        }
    }
    /// `OrderList::GetTimetableTotalDuration`: gets the total duration of the vehicles
    /// timetable or `Ticks::INVALID_TICKS` is the timetable is not complete.
    pub(crate) fn get_timetable_total_duration(self) -> i32 {
        if self.is_complete_timetable() {
            read_field!(self.list, timetable_duration)
        } else {
            INVALID_TICKS
        }
    }
    /// `OrderList::GetTotalDuration`: gets the known duration of the vehicles orders,
    /// timetabled or not.
    pub(crate) fn get_total_duration(self) -> i32 {
        read_field!(self.list, total_duration)
    }
    /// `OrderList::UpdateTimetableDuration`: must be called if an order's timetable is changed
    /// to update internal book keeping.
    fn update_timetable_duration(self, delta: i32) {
        write_field!(
            self.list,
            timetable_duration,
            read_field!(self.list, timetable_duration).wrapping_add(delta)
        );
    }
    /// `OrderList::UpdateTotalDuration`: must be called if an order's timetable is changed to
    /// update internal book keeping.
    fn update_total_duration(self, delta: i32) {
        write_field!(
            self.list,
            total_duration,
            read_field!(self.list, total_duration).wrapping_add(delta)
        );
    }
}

/// An `OrderBackup` shell with its three Rust owners, resolved once per backup visited.
#[derive(Clone, Copy)]
struct OrderBackupRef {
    shell: *mut c_void,
    backup: *mut Backup,
    vector: *mut Vector,
    consist: ConsistRef,
}

#[allow(unsafe_code)]
impl OrderBackupRef {
    /// The live `OrderBackup` shell `shell` and its owners.
    fn of(shell: *mut c_void, s: &Leaves) -> Self {
        // SAFETY: Each caller passes a live non-null `OrderBackup` shell.
        let (backup, vector, consist) = unsafe {
            (
                (s.backup)(shell),
                (s.backup_vector)(shell),
                (s.backup_consist)(shell),
            )
        };
        Self {
            shell,
            backup,
            vector,
            consist: ConsistRef(consist),
        }
    }
    /// `user`.
    fn user(self) -> u32 {
        read_field!(self.backup, user)
    }
    fn set_user(self, value: u32) {
        write_field!(self.backup, user, value);
    }
    /// `tile`.
    fn tile(self) -> TileIndex {
        TileIndex(read_field!(self.backup, tile))
    }
    fn set_tile(self, value: TileIndex) {
        write_field!(self.backup, tile, value.0);
    }
    /// `group`.
    fn group(self) -> GroupID {
        GroupID(read_field!(self.backup, group))
    }
    fn set_group(self, value: GroupID) {
        write_field!(self.backup, group, value.0);
    }
    /// `clone`, a native `Vehicle` shell or null.
    fn clone_shell(self) -> *mut c_void {
        read_field!(self.backup, clone).cast_mut()
    }
    fn set_clone(self, value: *mut c_void) {
        write_field!(self.backup, clone, value.cast_const());
    }
    /// `orders.size()`.
    fn len(self) -> usize {
        read_field!(self.vector, length)
    }
    /// `orders[index]` for an index below [`Self::len`].
    fn order(self, index: usize) -> OrderView<'static> {
        // SAFETY: An index below the length names a live stored order.
        unsafe { OrderView::of((*self.vector).data().add(index)) }
    }
    /// `delete this`: the shell and its owners are dead afterwards.
    fn delete(self, s: &Leaves) {
        // SAFETY: The live backup shell, whose owners no handle uses after this call.
        unsafe { (s.delete_backup)(self.shell) };
    }
}

/// The next live backup of `OrderBackup::Iterate()` at or after `cursor`, advancing it.
#[allow(unsafe_code)]
fn next_backup(s: &Leaves, cursor: &mut u32) -> Option<OrderBackupRef> {
    // SAFETY: The callback only advances this exclusive live cursor.
    let ob = unsafe { (s.next_backup)(cursor) };
    (!ob.is_null()).then(|| OrderBackupRef::of(ob, s))
}

/// `OrderBackup::Iterate()`: each live backup in pool order, the next one fetched after the
/// body has run on the current one, which it may delete.
fn for_each_backup(s: &Leaves, mut body: impl FnMut(OrderBackupRef)) {
    let mut cursor = 0;
    while let Some(ob) = next_backup(s, &mut cursor) {
        body(ob);
    }
}

/// `BaseStation::owner` of a station a lookup returned.
#[allow(unsafe_code)]
fn base_station_owner(st: *mut crate::station::BaseStation, s: &Leaves) -> Owner {
    // SAFETY: The station command table names the process-lifetime station table.
    let station = unsafe { &*s.station_commands().station };
    CompanyID((station.station_owner)(st))
}

/// `new OrderList()` followed by the constructor's body: `orders` in place and
/// `Initialize(v)`.
fn new_order_list(v: VehicleRef, orders: impl FnOnce(OrderListRef), s: &Leaves) -> OrderListRef {
    let list = OrderListRef::of((s.allocate_list)(), s);
    orders(list);
    list.initialize(v, s);
    list
}

#[allow(unsafe_code)]
impl OrderListRef {
    /// `OrderList::Initialize`: recomputes everything. `v` is one of vehicle that is using
    /// this orderlist.
    pub(crate) fn initialize(self, v: VehicleRef, s: &Leaves) {
        self.set_first_shared(v.shell());

        self.set_num_manual_orders(VehicleOrderID(0));
        self.set_num_vehicles(1);
        write_field!(self.list, timetable_duration, 0);

        for index in 0..self.len() {
            let o = self.order(index).view();
            if !o.is_type(OrderType::OT_IMPLICIT) {
                self.set_num_manual_orders(VehicleOrderID(
                    self.get_num_manual_orders().0.wrapping_add(1),
                ));
            }
            self.update_total_duration(i32::from(o.wait_time()) + i32::from(o.travel_time()));
        }

        self.recalculate_timetable_duration();

        let mut u = previous_shared(record(self.first_shared_shell(), s), s);
        while let Some(shared) = u {
            self.set_num_vehicles(self.get_num_vehicles().wrapping_add(1));
            self.set_first_shared(shared.shell());
            u = previous_shared(shared, s);
        }

        let mut u = next_shared(v, s);
        while let Some(shared) = u {
            self.set_num_vehicles(self.get_num_vehicles().wrapping_add(1));
            u = next_shared(shared, s);
        }
    }

    /// `OrderList::RecalculateTimetableDuration`: recomputes timetable duration. Split out
    /// into a separate function so it can be used by afterload.
    pub(crate) fn recalculate_timetable_duration(self) {
        write_field!(self.list, timetable_duration, 0);
        for index in 0..self.len() {
            let o = self.order(index).view();
            self.update_timetable_duration(
                i32::from(o.timetabled_wait()) + i32::from(o.timetabled_travel()),
            );
        }
    }

    /// `OrderList::FreeChain`: free a complete order chain. If `keep_orderlist` is true only
    /// delete the orders, otherwise also delete the `OrderList`. Do not use on
    /// `current_order` vehicle orders!
    pub(crate) fn free_chain(self, keep_orderlist: bool, s: &Leaves) {
        /* We can visit oil rigs and buoys that are not our own. They will be shown in
         * the list of stations. So, we need to invalidate that window if needed. */
        for index in 0..self.len() {
            let order = self.order(index).view();
            if order.is_type(OrderType::OT_GOTO_STATION)
                || order.is_type(OrderType::OT_GOTO_WAYPOINT)
            {
                let bs = (s.base_station_get_if_valid)(order.destination().to_station_id().0);
                if !bs.is_null() && base_station_owner(bs, s) == Owner::NONE {
                    invalidate_window_classes_data(WindowClass::WC_STATION_LIST, 0, false);
                    break;
                }
            }
        }

        if keep_orderlist {
            // SAFETY: The live vector; no order handle is used across the clear.
            unsafe { (*self.vector).resize(0) };
            self.set_num_manual_orders(VehicleOrderID(0));
            write_field!(self.list, timetable_duration, 0);
        } else {
            // SAFETY: The live shell, whose owners no handle uses after this call.
            unsafe { (s.delete_list)(self.shell) };
        }
    }

    /// `OrderList::GetNextDecisionNode`: get the next order which will make the given vehicle
    /// stop at a station or refit at a depot or evaluate a non-trivial condition. `next` is
    /// the order to start looking at, `hops` the number of orders we have already looked at.
    /// Returns either of a station order, a refitting depot order, a non-trivial conditional
    /// order or `INVALID_VEH_ORDER_ID` if the vehicle won't stop anymore.
    pub(crate) fn get_next_decision_node(self, next: VehicleOrderID, hops: u32) -> VehicleOrderID {
        if hops > u32::from(self.get_num_orders().0) || next >= self.get_num_orders() {
            return VehicleOrderID::INVALID_VEH_ORDER_ID;
        }

        let order_next = self.order(usize::from(next.0)).view();
        if order_next.is_type(OrderType::OT_CONDITIONAL) {
            if order_next.condition_variable() != OrderConditionVariable::Unconditionally {
                return next;
            }

            /* We can evaluate trivial conditions right away. They're conceptually
             * the same as regular order progression. */
            return self.get_next_decision_node(
                order_next.condition_skip_to_order(),
                hops.wrapping_add(1),
            );
        }

        if order_next.is_type(OrderType::OT_GOTO_DEPOT) {
            if order_next
                .depot_action_flags()
                .test(OrderDepotActionFlag::Halt)
            {
                return VehicleOrderID::INVALID_VEH_ORDER_ID;
            }
            if order_next.is_refit() {
                return next;
            }
        }

        if !order_next.can_load_or_unload() {
            return self.get_next_decision_node(self.get_next(next), hops.wrapping_add(1));
        }

        next
    }

    /// `OrderList::GetNextStoppingStation`: recursively determine the next deterministic
    /// station to stop at, passing each to `next_station` in the order the original appends
    /// them. `first` is the order to start searching at or `INVALID_VEH_ORDER_ID` to start at
    /// `cur_implicit_order_index + 1`; `hops` the number of orders we have already looked at.
    /// The vehicle is currently loading and `v->last_station_visited` is meaningful.
    pub(crate) fn get_next_stopping_station(
        self,
        next_station: &mut impl FnMut(StationID),
        v: VehicleRef,
        first: VehicleOrderID,
        mut hops: u32,
    ) {
        let mut next = first;
        if first == VehicleOrderID::INVALID_VEH_ORDER_ID {
            next = consist(v).cur_implicit_order_index();
            if next >= self.get_num_orders() {
                next = self.get_first_order();
                if next == VehicleOrderID::INVALID_VEH_ORDER_ID {
                    return;
                }
            } else {
                /* GetNext never returns INVALID_VEH_ORDER_ID if there is a valid station in the list.
                 * As the given "next" is already valid and a station in the list, we
                 * don't have to check for INVALID_VEH_ORDER_ID here. */
                next = self.get_next(next);
                assert_with(|| next != VehicleOrderID::INVALID_VEH_ORDER_ID);
            }
        }

        let orders = |index: VehicleOrderID| self.order(usize::from(index.0)).view();
        loop {
            hops = hops.wrapping_add(1);
            next = self.get_next_decision_node(next, hops);

            /* Resolve possibly nested conditionals by estimation. */
            while next != VehicleOrderID::INVALID_VEH_ORDER_ID
                && orders(next).is_type(OrderType::OT_CONDITIONAL)
            {
                /* We return both options of conditional orders. */
                let skip_to =
                    self.get_next_decision_node(orders(next).condition_skip_to_order(), hops);
                let advance = self.get_next_decision_node(self.get_next(next), hops);
                if advance == VehicleOrderID::INVALID_VEH_ORDER_ID
                    || advance == first
                    || skip_to == advance
                {
                    next = if skip_to == first {
                        VehicleOrderID::INVALID_VEH_ORDER_ID
                    } else {
                        skip_to
                    };
                } else if skip_to == VehicleOrderID::INVALID_VEH_ORDER_ID || skip_to == first {
                    next = if advance == first {
                        VehicleOrderID::INVALID_VEH_ORDER_ID
                    } else {
                        advance
                    };
                } else {
                    self.get_next_stopping_station(next_station, v, skip_to, hops);
                    self.get_next_stopping_station(next_station, v, advance, hops);
                    return;
                }
                hops = hops.wrapping_add(1);
            }

            /* Don't return a next stop if the vehicle has to unload everything. */
            if next == VehicleOrderID::INVALID_VEH_ORDER_ID
                || ((orders(next).is_type(OrderType::OT_GOTO_STATION)
                    || orders(next).is_type(OrderType::OT_IMPLICIT))
                    && orders(next).destination().to_station_id() == v.last_station_visited()
                    && (orders(next).unload_type() == OrderUnloadType::Transfer
                        || orders(next).unload_type() == OrderUnloadType::Unload))
            {
                return;
            }
            if !(orders(next).is_type(OrderType::OT_GOTO_DEPOT)
                || orders(next).destination().to_station_id() == v.last_station_visited())
            {
                break;
            }
        }

        next_station(orders(next).destination().to_station_id());
    }

    /// `OrderList::InsertOrderAt`: insert a new order into the order chain. `index` is the
    /// position where the order is supposed to be inserted.
    pub(crate) fn insert_order_at(self, order: Order, index: VehicleOrderID, s: &Leaves) {
        let index = usize::from(index.0).min(self.len());
        // SAFETY: The live vector; no order handle is used across the insertion.
        unsafe { (*self.vector).insert(index, order) };
        let new_order = self.order(index).view();

        if !new_order.is_type(OrderType::OT_IMPLICIT) {
            self.set_num_manual_orders(VehicleOrderID(
                self.get_num_manual_orders().0.wrapping_add(1),
            ));
        }
        self.update_timetable_duration(
            i32::from(new_order.timetabled_wait()) + i32::from(new_order.timetabled_travel()),
        );
        self.update_total_duration(
            i32::from(new_order.wait_time()) + i32::from(new_order.travel_time()),
        );

        /* We can visit oil rigs and buoys that are not our own. They will be shown in
         * the list of stations. So, we need to invalidate that window if needed. */
        if new_order.is_type(OrderType::OT_GOTO_STATION)
            || new_order.is_type(OrderType::OT_GOTO_WAYPOINT)
        {
            let bs = (s.shared().base_station)(new_order.destination().to_station_id().0);
            if base_station_owner(bs, s) == Owner::NONE {
                invalidate_window_classes_data(WindowClass::WC_STATION_LIST, 0, false);
            }
        }
    }

    /// `OrderList::DeleteOrderAt`: remove an order from the order list and delete it. `index`
    /// is the position of the order which is to be deleted.
    pub(crate) fn delete_order_at(self, index: VehicleOrderID) {
        let index = usize::from(index.0);
        if index >= self.len() {
            return;
        }
        let to_remove = self.order(index).view();

        if !to_remove.is_type(OrderType::OT_IMPLICIT) {
            self.set_num_manual_orders(VehicleOrderID(
                self.get_num_manual_orders().0.wrapping_sub(1),
            ));
        }

        self.update_timetable_duration(
            -(i32::from(to_remove.timetabled_wait()) + i32::from(to_remove.timetabled_travel())),
        );
        self.update_total_duration(
            -(i32::from(to_remove.wait_time()) + i32::from(to_remove.travel_time())),
        );

        // SAFETY: The live vector; `to_remove` is not used after the erasure.
        unsafe { (*self.vector).erase(index) };
    }

    /// `OrderList::MoveOrder`: move an order to another position within the order list.
    /// `from` is the zero-based position of the order to move, `to` the zero-based position
    /// where the order is moved to.
    pub(crate) fn move_order(self, from: VehicleOrderID, to: VehicleOrderID) {
        if from == to {
            return;
        }
        if from >= self.get_num_orders() {
            return;
        }
        if to >= self.get_num_orders() {
            return;
        }

        // SAFETY: The live vector; both positions are below its length.
        unsafe { (*self.vector).rotate(usize::from(from.0), usize::from(to.0)) };
    }

    /// `OrderList::IsCompleteTimetable`: checks whether all orders of the list have a filled
    /// timetable.
    pub(crate) fn is_complete_timetable(self) -> bool {
        for index in 0..self.len() {
            let o = self.order(index).view();
            /* Implicit orders are, by definition, not timetabled. */
            if o.is_type(OrderType::OT_IMPLICIT) {
                continue;
            }
            if !o.is_completely_timetabled() {
                return false;
            }
        }
        true
    }
}

/// `Vehicle::orders`: the vehicle's order list, `None` where it has none.
pub(crate) fn order_list(v: VehicleRef, s: &Leaves) -> Option<OrderListRef> {
    let shell = crate::vehicle_base::order_list(v.0.as_ptr());
    (!shell.is_null()).then(|| OrderListRef::of(shell, s))
}

/// `GetNumOrders()` of a vehicle whose resolved list is `list`: 0 without one.
fn num_orders(list: Option<OrderListRef>) -> VehicleOrderID {
    list.map_or(VehicleOrderID(0), OrderListRef::get_num_orders)
}

/// `GetNumManualOrders()` of a vehicle whose resolved list is `list`: 0 without one.
fn num_manual_orders(list: Option<OrderListRef>) -> VehicleOrderID {
    list.map_or(VehicleOrderID(0), OrderListRef::get_num_manual_orders)
}

/// `GetOrder(index)` of a vehicle whose resolved list is `list`.
fn order_in(list: Option<OrderListRef>, index: VehicleOrderID) -> Option<OrderRef> {
    list?.get_order_at(index)
}

/// `GetFirstOrder()` of a vehicle whose resolved list is `list`.
fn first_order_in(list: Option<OrderListRef>) -> Option<OrderRef> {
    let list = list?;
    list.get_order_at(list.get_first_order())
}

/// `GetLastOrder()` of a vehicle whose resolved list is `list`.
fn last_order_in(list: Option<OrderListRef>) -> Option<OrderRef> {
    let list = list?;
    list.get_order_at(list.get_last_order())
}

/// `FirstShared()` of the vehicle `v` whose resolved list is `list`. With no list the
/// original returns the stored `first` link as it is, null included.
fn first_shared_in(list: Option<OrderListRef>, v: VehicleRef, s: &Leaves) -> Option<VehicleRef> {
    match list {
        None => v.first_link(),
        Some(list) => list.get_first_shared_vehicle(s),
    }
}

/// `v->orders` of the resolved list `list`, dereferenced unchecked where the original reads
/// it without a test.
fn deref_list(list: Option<OrderListRef>, v: VehicleRef, s: &Leaves) -> OrderListRef {
    list.unwrap_or_else(|| list_of(v, s))
}

/// `v->GetOrder(index)`, dereferenced unchecked where the original reads the order without a
/// test: the stored order `index` of the vehicle's list.
pub(crate) fn get_order_unchecked(v: VehicleRef, index: VehicleOrderID, s: &Leaves) -> OrderRef {
    list_of(v, s).order(usize::from(index.0))
}

/// `v->orders`, dereferenced unchecked where the original reads it without a test.
fn list_of(v: VehicleRef, s: &Leaves) -> OrderListRef {
    OrderListRef::of(crate::vehicle_base::order_list(v.0.as_ptr()), s)
}

/// `v->PreviousShared()`, dereferenced unchecked where the original reads it without a test.
#[allow(unsafe_code)]
fn previous_shared_of(v: VehicleRef, s: &Leaves) -> VehicleRef {
    record(read_field!(vehicle_orders(v), previous_shared), s)
}

/// `v->orders = list`.
#[allow(unsafe_code)]
fn set_order_list(v: VehicleRef, list: Option<OrderListRef>) {
    write_field!(
        vehicle_orders(v),
        orders,
        list.map_or(ptr::null_mut(), OrderListRef::shell)
    );
}

/// `Vehicle::NextShared`: get the next vehicle of the shared vehicle chain.
#[allow(unsafe_code)]
pub(crate) fn next_shared(v: VehicleRef, s: &Leaves) -> Option<VehicleRef> {
    record_if(read_field!(vehicle_orders(v), next_shared), s)
}

/// `Vehicle::PreviousShared`: get the previous vehicle of the shared vehicle chain.
#[allow(unsafe_code)]
pub(crate) fn previous_shared(v: VehicleRef, s: &Leaves) -> Option<VehicleRef> {
    record_if(read_field!(vehicle_orders(v), previous_shared), s)
}

/// `Vehicle::FirstShared`: get the first vehicle of this vehicle chain.
pub(crate) fn first_shared(v: VehicleRef, s: &Leaves) -> Option<VehicleRef> {
    first_shared_in(order_list(v, s), v, s)
}

/// `Vehicle::GetNumOrders`: get the number of orders this vehicle has.
pub(crate) fn get_num_orders(v: VehicleRef, s: &Leaves) -> VehicleOrderID {
    num_orders(order_list(v, s))
}

/// `Vehicle::GetOrder`: returns order `index` of a vehicle or `None` when it doesn't exists.
pub(crate) fn get_order(v: VehicleRef, index: VehicleOrderID, s: &Leaves) -> Option<OrderRef> {
    order_list(v, s)?.get_order_at(index)
}

/// `Vehicle::Orders`: the vehicle's orders, `None` for an empty span.
pub(crate) fn orders(v: VehicleRef, s: &Leaves) -> Option<OrderListRef> {
    order_list(v, s)
}

/// `std::ranges::any_of(v->Orders(), predicate)`.
fn any_order(v: VehicleRef, s: &Leaves, predicate: impl Fn(OrderView<'_>) -> bool) -> bool {
    any_order_in(order_list(v, s), predicate)
}

/// `std::ranges::any_of(v->Orders(), predicate)` over the vehicle's resolved list `list`.
fn any_order_in(list: Option<OrderListRef>, predicate: impl Fn(OrderView<'_>) -> bool) -> bool {
    list.is_some_and(|list| (0..list.len()).any(|index| predicate(list.order(index).view())))
}

/// `Vehicle::GetNextStoppingStation`: get the next station the vehicle will stop at, passing
/// each to `next_station`.
pub(crate) fn get_next_stopping_station(
    v: VehicleRef,
    next_station: &mut impl FnMut(StationID),
    s: &Leaves,
) {
    let Some(list) = order_list(v, s) else {
        return;
    };
    list.get_next_stopping_station(next_station, v, VehicleOrderID::INVALID_VEH_ORDER_ID, 0);
}

/// `Vehicle::IsGroundVehicle`: check if the vehicle is a ground vehicle.
fn is_ground_vehicle(v: VehicleRef) -> bool {
    crate::vehicle_base::is_ground_vehicle(v.vehicle_type())
}

/// `SetBit` or `ClrBit` of `GVF_SUPPRESS_IMPLICIT_ORDERS` in `v->GetGroundVehicleFlags()`.
fn assign_suppress_implicit_orders(v: VehicleRef, suppress: bool) {
    let mut gv_flags = v.ground_vehicle_flags();
    assign_bit16(&mut gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS, suppress);
    v.set_ground_vehicle_flags(gv_flags);
}

/// `Vehicle::SkipToNextRealOrderIndex`: advance `cur_real_order_index` to the next real
/// order. `cur_implicit_order_index` is not touched.
/// `list` is the vehicle's `orders`, resolved by the caller.
fn skip_to_next_real_order_index(v: VehicleRef, list: Option<OrderListRef>, s: &Leaves) {
    let c = consist(v);
    if num_manual_orders(list).0 > 0 {
        let list = deref_list(list, v, s);
        /* Advance to next real order */
        loop {
            c.set_cur_real_order_index(VehicleOrderID(c.cur_real_order_index().0.wrapping_add(1)));
            if c.cur_real_order_index() >= list.get_num_orders() {
                c.set_cur_real_order_index(VehicleOrderID(0));
            }
            if !list
                .order(usize::from(c.cur_real_order_index().0))
                .view()
                .is_type(OrderType::OT_IMPLICIT)
            {
                break;
            }
        }
    } else {
        c.set_cur_real_order_index(VehicleOrderID(0));
    }
}

/// `Vehicle::IncrementImplicitOrderIndex`: increments `cur_implicit_order_index`, keeps care
/// of the wrap-around and invalidates the GUI. `cur_real_order_index` is incremented as well,
/// if needed. Note: `current_order` is not invalidated.
pub(crate) fn increment_implicit_order_index(v: VehicleRef, s: &Leaves) {
    let c = consist(v);
    // `this->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);
    if c.cur_implicit_order_index() == c.cur_real_order_index() {
        /* Increment real order index as well */
        skip_to_next_real_order_index(v, list, s);
    }

    assert_with(|| c.cur_real_order_index().0 == 0 || c.cur_real_order_index() < num_orders(list));

    /* Advance to next implicit order */
    loop {
        c.set_cur_implicit_order_index(VehicleOrderID(
            c.cur_implicit_order_index().0.wrapping_add(1),
        ));
        if c.cur_implicit_order_index() >= num_orders(list) {
            c.set_cur_implicit_order_index(VehicleOrderID(0));
        }
        if c.cur_implicit_order_index() == c.cur_real_order_index()
            || deref_list(list, v, s)
                .order(usize::from(c.cur_implicit_order_index().0))
                .view()
                .is_type(OrderType::OT_IMPLICIT)
        {
            break;
        }
    }

    invalidate_vehicle_order(v, 0, s);
}

/// `Vehicle::IncrementRealOrderIndex`: advanced `cur_real_order_index` to the next real order,
/// keeps care of the wrap-around and invalidates the GUI. `cur_implicit_order_index` is
/// incremented as well, if it was equal to `cur_real_order_index`, i.e.
/// `cur_real_order_index` is skipped but not any implicit orders. Note: `current_order` is not
/// invalidated.
pub(crate) fn increment_real_order_index(v: VehicleRef, s: &Leaves) {
    let c = consist(v);
    if c.cur_implicit_order_index() == c.cur_real_order_index() {
        /* Increment both real and implicit order */
        increment_implicit_order_index(v, s);
    } else {
        /* Increment real order only */
        skip_to_next_real_order_index(v, order_list(v, s), s);
        invalidate_vehicle_order(v, 0, s);
    }
}

/// `Vehicle::UpdateRealOrderIndex`: skip implicit orders until `cur_real_order_index` is a
/// non-implicit order.
pub(crate) fn update_real_order_index(v: VehicleRef, s: &Leaves) {
    update_real_order_index_in(v, order_list(v, s));
}

/// `Vehicle::UpdateRealOrderIndex` over the vehicle's `orders` `list`, resolved by the caller:
/// nothing below replaces it.
fn update_real_order_index_in(v: VehicleRef, list: Option<OrderListRef>) {
    let c = consist(v);
    let num_orders = list.map_or(VehicleOrderID(0), OrderListRef::get_num_orders);
    /* Make sure the index is valid */
    if c.cur_real_order_index() >= num_orders {
        c.set_cur_real_order_index(VehicleOrderID(0));
    }

    if let Some(list) = list.filter(|list| list.get_num_manual_orders().0 > 0) {
        /* Advance to next real order */
        while list
            .order(usize::from(c.cur_real_order_index().0))
            .view()
            .is_type(OrderType::OT_IMPLICIT)
        {
            c.set_cur_real_order_index(VehicleOrderID(c.cur_real_order_index().0.wrapping_add(1)));
            if c.cur_real_order_index() >= num_orders {
                c.set_cur_real_order_index(VehicleOrderID(0));
            }
        }
    } else {
        c.set_cur_real_order_index(VehicleOrderID(0));
    }
}

/// `InvalidateVehicleOrder(v, data)`: updates the widgets of a vehicle which contains the
/// order-data.
#[allow(unsafe_code)]
fn invalidate_vehicle_order(v: VehicleRef, data: i32, s: &Leaves) {
    // SAFETY: The live vehicle shell; the C++ body reads only its index.
    unsafe { (s.invalidate_vehicle_order)(v.shell(), data) };
}

/// `Vehicle::HasDepotOrder`: checks if a vehicle has a depot in its order list. True iff at
/// least one order is a depot order.
pub(crate) fn has_depot_order(v: VehicleRef, s: &Leaves) -> bool {
    any_order(v, s, |order| order.is_type(OrderType::OT_GOTO_DEPOT))
}

/// `Vehicle::HasFullLoadOrder`: check if the current vehicle has a full load order. `list`
/// is the vehicle's `orders`, resolved by the caller.
fn has_full_load_order(list: Option<OrderListRef>) -> bool {
    any_order_in(list, |o| {
        o.is_type(OrderType::OT_GOTO_STATION) && o.is_full_load()
    })
}

/// `Vehicle::HasConditionalOrder`: check if the current vehicle has a conditional order.
/// `list` is the vehicle's `orders`, resolved by the caller.
fn has_conditional_order(list: Option<OrderListRef>) -> bool {
    any_order_in(list, |o| o.is_type(OrderType::OT_CONDITIONAL))
}

/// `Vehicle::HasUnbunchingOrder`: check if the current vehicle has an unbunching order.
/// `list` is the vehicle's `orders`, resolved by the caller.
fn has_unbunching_order(list: Option<OrderListRef>) -> bool {
    any_order_in(list, |o| {
        o.is_type(OrderType::OT_GOTO_DEPOT)
            && o.depot_action_flags().test(OrderDepotActionFlag::Unbunch)
    })
}

/// `PreviousOrderIsUnbunching`: check if the previous order is a depot unbunching order.
/// `list` is the vehicle's `orders`, resolved by the caller.
fn previous_order_is_unbunching(v: VehicleRef, list: Option<OrderListRef>) -> bool {
    let c = consist(v);
    /* If we are headed for the first order, we must wrap around back to the last order. */
    let is_first_order = order_in(list, c.cur_implicit_order_index()).map(|o| o.view().as_ptr())
        == first_order_in(list).map(|o| o.view().as_ptr());
    let previous_order = if is_first_order {
        last_order_in(list)
    } else {
        order_in(
            list,
            VehicleOrderID(c.cur_implicit_order_index().0.wrapping_sub(1)),
        )
    };

    let Some(previous_order) = previous_order.map(OrderRef::view) else {
        return false;
    };
    if !previous_order.is_type(OrderType::OT_GOTO_DEPOT) {
        return false;
    }
    previous_order
        .depot_action_flags()
        .test(OrderDepotActionFlag::Unbunch)
}

/// `Vehicle::LeaveUnbunchingDepot`: leave an unbunching depot and calculate the next departure
/// time for shared order vehicles.
#[allow(
    clippy::cast_sign_loss,
    reason = "the original's conversion to `TickCounter`"
)]
pub(crate) fn leave_unbunching_depot(v: VehicleRef, s: &Leaves) {
    // `this->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);
    /* Don't do anything if this is not our unbunching order. */
    if !previous_order_is_unbunching(v, list) {
        return;
    }

    let c = consist(v);
    let counter = (s.shared().tick_counter)();
    /* Set the start point for this round trip time. */
    c.set_depot_unbunching_last_departure(counter);

    /* Tell the timetable we are now "on time." */
    c.set_lateness_counter(0);
    set_window_dirty(
        WindowClass::WC_VEHICLE_TIMETABLE,
        WindowNumber::from(v.index(s.newgrf())),
    );

    /* Find the average travel time of vehicles that we share orders with. */
    let mut num_vehicles = 0i32;
    let mut total_travel_time = 0i32;

    let mut u = first_shared_in(list, v, s);
    while let Some(shared) = u {
        u = next_shared(shared, s);
        /* Ignore vehicles that are manually stopped or crashed. */
        if shared.vehstatus().any(
            VehStates::from_element(VehState::Stopped)
                .with(VehStates::from_element(VehState::Crashed)),
        ) {
            continue;
        }

        num_vehicles = num_vehicles.wrapping_add(1);
        total_travel_time = total_travel_time.wrapping_add(consist(shared).round_trip_time());
    }

    /* Make sure we cannot divide by 0. */
    num_vehicles = num_vehicles.max(1);

    /* Calculate the separation by finding the average travel time, then calculating equal separation (minimum 1 tick) between vehicles. */
    let separation = (total_travel_time / num_vehicles / num_vehicles).max(1);
    // `TickCounter + Ticks`: the `int32_t` separation converts to `uint64_t`.
    let next_departure = (s.shared().tick_counter)().wrapping_add(separation as u64);

    /* Set the departure time of all vehicles that we share orders with. */
    let mut u = first_shared_in(list, v, s);
    while let Some(shared) = u {
        u = next_shared(shared, s);
        /* Ignore vehicles that are manually stopped or crashed. */
        if shared.vehstatus().any(
            VehStates::from_element(VehState::Stopped)
                .with(VehStates::from_element(VehState::Crashed)),
        ) {
            continue;
        }

        consist(shared).set_depot_unbunching_next_departure(next_departure);
        set_window_dirty(
            WindowClass::WC_VEHICLE_VIEW,
            WindowNumber::from(shared.index(s.newgrf())),
        );
    }
}

/// `Vehicle::IsWaitingForUnbunching` after its `assert(this->IsInDepot())`, which
/// `vehicle::is_waiting_for_unbunching` makes: check whether a vehicle inside a depot is
/// waiting for unbunching. True if the vehicle must continue waiting, or false if it may try
/// to leave the depot.
pub(crate) fn is_waiting_for_unbunching(v: VehicleRef, s: &Leaves) -> bool {
    // `this->orders` is resolved once for the two tests; `PreviousOrderIsUnbunching` resolves
    // it in its own body.
    let list = order_list(v, s);
    /* Don't bother if there are no vehicles sharing orders. */
    if !list.is_some_and(OrderListRef::is_shared) {
        return false;
    }

    /* Don't do anything if there aren't enough orders. */
    if num_orders(list).0 <= 1 {
        return false;
    }

    /* Don't do anything if this is not our unbunching order. */
    if !previous_order_is_unbunching(v, list) {
        return false;
    }

    consist(v).depot_unbunching_next_departure() > (s.shared().tick_counter)()
}

/// The unbunching round-trip measurement of `VehicleEnterDepot`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the original's `Ticks` result"
)]
pub(crate) fn measure_unbunching(v: VehicleRef, s: &Leaves) {
    let c = consist(v);
    if v.current_order()
        .depot_action_flags()
        .test(OrderDepotActionFlag::Unbunch)
        && c.depot_unbunching_last_departure() > 0
    {
        // `TickCounter - TickCounter` narrowed to `Ticks`.
        let measured_round_trip =
            (s.shared().tick_counter)().wrapping_sub(c.depot_unbunching_last_departure()) as i32;
        if c.round_trip_time() == 0 {
            /* This might be our first round trip. */
            c.set_round_trip_time(measured_round_trip);
        } else {
            /* If we have a previous trip, smooth the effects of outlier trip calculations caused by jams or other interference. */
            // `round_trip_time * 2` is an `int` product before `ClampTo<Ticks>`.
            c.set_round_trip_time(crate::math::clamp(
                measured_round_trip,
                c.round_trip_time() / 2,
                c.round_trip_time().wrapping_mul(2),
                |condition| assert_with(|| condition),
            ));
        }
    }
}

/// `Vehicle::AddToShared`: adds this vehicle to a shared vehicle chain. `shared_chain` is a
/// vehicle of the chain with shared vehicles; `!v->IsOrderListShared()`.
#[allow(unsafe_code)]
pub(crate) fn add_to_shared(v: VehicleRef, shared_chain: VehicleRef, s: &Leaves) {
    let orders = vehicle_orders(v);
    let chain = vehicle_orders(shared_chain);
    assert_with(|| {
        read_field!(orders, previous_shared).is_null() && read_field!(orders, next_shared).is_null()
    });

    // `shared_chain->orders`, created here where the chain has none.
    let list = if let Some(list) = order_list(shared_chain, s) {
        list
    } else {
        assert_with(|| read_field!(chain, previous_shared).is_null());
        assert_with(|| read_field!(chain, next_shared).is_null());
        let list = new_order_list(shared_chain, |_| {}, s);
        set_order_list(v, Some(list));
        set_order_list(shared_chain, Some(list));
        list
    };

    write_field!(orders, next_shared, read_field!(chain, next_shared));
    write_field!(orders, previous_shared, shared_chain.shell());

    write_field!(chain, next_shared, v.shell());

    if let Some(next) = next_shared(v, s) {
        write_field!(vehicle_orders(next), previous_shared, v.shell());
    }

    list.add_vehicle();
}

/// `Vehicle::RemoveFromShared`: removes the vehicle from the shared order list.
#[allow(unsafe_code)]
#[allow(
    clippy::cast_possible_wrap,
    reason = "the original's conversion to `int`"
)]
pub(crate) fn remove_from_shared(v: VehicleRef, s: &Leaves) {
    let orders = vehicle_orders(v);
    // `this->orders` is resolved once: removing the vehicle keeps the list, whose first
    // shared vehicle each `FirstShared()` below rereads.
    let list = list_of(v, s);
    let first = || record(list.first_shared_shell(), s);
    /* Remember if we were first and the old window number before RemoveVehicle()
     * as this changes first if needed. */
    let first_before = first();
    let were_first = first_before == v;
    let vli = VehicleListIdentifier {
        list_type: VehicleListType::VL_SHARED_ORDERS,
        vtype: v.vehicle_type(),
        company: v.owner(),
        index: first_before.index(s.newgrf()).0,
    };

    list.remove_vehicle(v);

    if !were_first {
        /* We are not the first shared one, so only relink our previous one. */
        write_field!(
            vehicle_orders(previous_shared_of(v, s)),
            next_shared,
            read_field!(orders, next_shared)
        );
    }

    if let Some(next) = next_shared(v, s) {
        write_field!(
            vehicle_orders(next),
            previous_shared,
            read_field!(orders, previous_shared)
        );
    }

    if list.get_num_vehicles() == 1 {
        /* When there is only one vehicle, remove the shared order list window. */
        close_window_by_id(
            get_window_class_for_vehicle_type(v.vehicle_type()),
            vli.to_window_number(),
            true,
            0,
        );
        invalidate_vehicle_order(
            first(),
            VehicleInvalidateWindowData::VIWD_MODIFY_ORDERS.0,
            s,
        );
    } else if were_first {
        /* If we were the first one, update to the new first one.
         * Note: FirstShared() is already the new first */
        // `index.base() | (1U << 31)` passed as the `int` data.
        invalidate_window_data(
            get_window_class_for_vehicle_type(v.vehicle_type()),
            vli.to_window_number(),
            (first().index(s.newgrf()).0 | (1u32 << 31)) as i32,
            false,
        );
    }

    write_field!(orders, next_shared, ptr::null_mut());
    write_field!(orders, previous_shared, ptr::null_mut());
}

/// `Vehicle::DeleteUnreachedImplicitOrders`: delete all implicit orders which were not
/// reached.
pub(crate) fn delete_unreached_implicit_orders(v: VehicleRef, s: &Leaves) {
    let c = consist(v);
    if is_ground_vehicle(v) {
        let mut gv_flags = v.ground_vehicle_flags();
        if has_bit(gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS) {
            /* Do not delete orders, only skip them */
            crate::bitmath_func::clr_bit16(&mut gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS);
            v.set_ground_vehicle_flags(gv_flags);
            c.set_cur_implicit_order_index(c.cur_real_order_index());
            invalidate_vehicle_order(v, 0, s);
            return;
        }
    }

    // `this->Orders()`: the span holds the list's storage, which DeleteOrder's erasure keeps
    // in place, so its slots stay live, including the retained former last order.
    let list = order_list(v, s);
    let order = |cur: VehicleOrderID| deref_list(list, v, s).order(usize::from(cur.0)).view();
    let mut cur = c.cur_implicit_order_index();
    while cur != VehicleOrderID::INVALID_VEH_ORDER_ID {
        if c.cur_implicit_order_index() == c.cur_real_order_index() {
            break;
        }

        if order(cur).is_type(OrderType::OT_IMPLICIT) {
            delete_order(v, c.cur_implicit_order_index(), s);
            /* DeleteOrder does various magic with order_indices, so resync 'order' with 'cur_implicit_order_index' */
        } else {
            let list = deref_list(list, v, s);
            /* Skip non-implicit orders, e.g. service-orders */
            if cur < list.get_next(cur) {
                c.set_cur_implicit_order_index(VehicleOrderID(
                    c.cur_implicit_order_index().0.wrapping_add(1),
                ));
            } else {
                /* Wrapped around. */
                c.set_cur_implicit_order_index(VehicleOrderID(0));
            }
            cur = list.get_next(cur);
        }
    }
}

/// The order part of `Vehicle::BeginLoading`: prepare everything to begin the loading when
/// arriving at a station.
#[allow(clippy::too_many_lines, reason = "one function per original body")]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's conversions"
)]
pub(crate) fn begin_loading(v: VehicleRef, s: &Leaves) {
    let c = consist(v);
    if v.current_order().is_type(OrderType::OT_GOTO_STATION)
        && v.current_order().destination().to_station_id() == v.last_station_visited()
    {
        delete_unreached_implicit_orders(v, s);

        /* Now both order indices point to the destination station, and we can start loading */
        current_order(v).make_loading(true);
        update_vehicle_timetable(v, true, s);

        /* Furthermore add the Non Stop flag to mark that this station
         * is the actual destination of the vehicle, which is (for example)
         * necessary to be known for HandleTrainLoading to determine
         * whether the train is lost or not; not marking a train lost
         * that arrives at random stations is bad. */
        current_order(v).set_non_stop_type(
            OrderNonStopFlags::from_element(OrderNonStopFlag::NoIntermediate).with(
                OrderNonStopFlags::from_element(OrderNonStopFlag::NoDestination),
            ),
        );
    } else {
        /* We weren't scheduled to stop here. Insert an implicit order
         * to show that we are stopping here.
         * While only groundvehicles have implicit orders, e.g. aircraft might still enter
         * the 'wrong' terminal when skipping orders etc. */
        // `this->orders` is resolved once: nothing below replaces the vehicle's list until
        // `InsertOrder`, after its last read.
        let list = order_list(v, s);
        let in_list = order_in(list, c.cur_implicit_order_index()).map(OrderRef::view);
        if is_ground_vehicle(v)
            && (in_list.is_none_or(|in_list| {
                !in_list.is_type(OrderType::OT_IMPLICIT)
                    || in_list.destination().to_station_id() != v.last_station_visited()
            }))
        {
            let suppress_implicit_orders =
                has_bit(v.ground_vehicle_flags(), GVF_SUPPRESS_IMPLICIT_ORDERS);
            /* Do not create consecutive duplicates of implicit orders */
            let prev_order = if c.cur_implicit_order_index().0 > 0 {
                order_in(list, VehicleOrderID(c.cur_implicit_order_index().0 - 1))
            } else if num_orders(list).0 > 1 {
                last_order_in(list)
            } else {
                None
            }
            .map(OrderRef::view);
            if prev_order.is_none_or(|prev_order| {
                (!prev_order.is_type(OrderType::OT_IMPLICIT)
                    && !prev_order.is_type(OrderType::OT_GOTO_STATION))
                    || prev_order.destination().to_station_id() != v.last_station_visited()
            }) {
                /* Prefer deleting implicit orders instead of inserting new ones,
                 * so test whether the right order follows later. In case of only
                 * implicit orders treat the last order in the list like an
                 * explicit one, except if the overall number of orders surpasses
                 * IMPLICIT_ORDER_ONLY_CAP. */
                let mut target_index = i32::from(c.cur_implicit_order_index().0);
                let mut found = false;
                while target_index != i32::from(c.cur_real_order_index().0)
                    || num_manual_orders(list).0 == 0
                {
                    // `GetOrder(int)` narrows the index to `VehicleOrderID`.
                    let Some(order) =
                        order_in(list, VehicleOrderID(target_index as u8)).map(OrderRef::view)
                    else {
                        break; // No orders.
                    };
                    if order.is_type(OrderType::OT_IMPLICIT)
                        && order.destination().to_station_id() == v.last_station_visited()
                    {
                        found = true;
                        break;
                    }
                    target_index = target_index.wrapping_add(1);
                    if target_index >= i32::from(deref_list(list, v, s).get_num_orders().0) {
                        if num_manual_orders(list).0 == 0
                            && u32::from(num_orders(list).0) < IMPLICIT_ORDER_ONLY_CAP
                        {
                            break;
                        }
                        target_index = 0;
                    }
                    if target_index == i32::from(c.cur_implicit_order_index().0) {
                        break; // Avoid infinite loop.
                    }
                }

                if found {
                    if suppress_implicit_orders {
                        /* Skip to the found order */
                        // `int` stored in `VehicleOrderID`.
                        c.set_cur_implicit_order_index(VehicleOrderID(target_index as u8));
                        invalidate_vehicle_order(v, 0, s);
                    } else {
                        /* Delete all implicit orders up to the station we just reached */
                        let list = deref_list(list, v, s);
                        let mut order = list
                            .order(usize::from(c.cur_implicit_order_index().0))
                            .view();
                        while !order.is_type(OrderType::OT_IMPLICIT)
                            || order.destination().to_station_id() != v.last_station_visited()
                        {
                            if order.is_type(OrderType::OT_IMPLICIT) {
                                delete_order(v, c.cur_implicit_order_index(), s);
                            } else {
                                /* Skip non-implicit orders, e.g. service-orders */
                                c.set_cur_implicit_order_index(VehicleOrderID(
                                    c.cur_implicit_order_index().0.wrapping_add(1),
                                ));
                            }
                            let next = list.get_order_at(c.cur_implicit_order_index());

                            /* Wrapped around. */
                            order = if let Some(next) = next {
                                next.view()
                            } else {
                                c.set_cur_implicit_order_index(VehicleOrderID(0));
                                assert_with(|| {
                                    list.get_order_at(c.cur_implicit_order_index()).is_some()
                                });
                                list.order(usize::from(c.cur_implicit_order_index().0))
                                    .view()
                            };
                        }
                    }
                } else if !suppress_implicit_orders
                    && (match list {
                        None => (s.list_can_allocate_item)() != 0,
                        Some(list) => list.get_num_orders() < MAX_VEH_ORDER_ID,
                    })
                {
                    /* Insert new implicit order */
                    let mut implicit_order = Order::default();
                    implicit_order
                        .edit()
                        .make_implicit(v.last_station_visited());
                    insert_order(v, implicit_order, c.cur_implicit_order_index(), s);
                    if c.cur_implicit_order_index().0 > 0 {
                        c.set_cur_implicit_order_index(VehicleOrderID(
                            c.cur_implicit_order_index().0 - 1,
                        ));
                    }

                    /* InsertOrder disabled creation of implicit orders for all vehicles with the same implicit order.
                     * Reenable it for this vehicle */
                    assign_suppress_implicit_orders(v, false);
                }
            }
        }
        current_order(v).make_loading(false);
    }
}

/// `Station::Get(st)`, as the shared table's base-station lookup returns it.
fn station_get(st: StationID, s: &Leaves) -> *mut crate::station::BaseStation {
    (s.shared().base_station)(st.0)
}

/// `GetVehicleCannotUseStationReason(v, st)`: get reason string why this station can't be
/// used by the given vehicle. Returns the string explaining why the vehicle cannot use the
/// station.
#[allow(unsafe_code)]
fn get_vehicle_cannot_use_station_reason(
    v: VehicleRef,
    st: *const crate::station::BaseStation,
    s: &Leaves,
) -> StringID {
    // SAFETY: The live vehicle shell and the station the caller resolved for
    // `CanVehicleUseStation`, which the original passes on unchanged.
    StringID(unsafe { (s.get_vehicle_cannot_use_station_reason)(v.shell(), st) })
}

/// `OrderGoesToStation`: checks whether the order goes to a station or not, i.e. whether the
/// destination is a station.
fn order_goes_to_station(v: VehicleRef, o: OrderView<'_>) -> bool {
    o.is_type(OrderType::OT_GOTO_STATION)
        || (v.vehicle_type() == VehicleType::VEH_AIRCRAFT
            && o.is_type(OrderType::OT_GOTO_DEPOT)
            && o.destination().to_station_id() != StationID::INVALID)
}

/// `DeleteOrderWarnings`: delete all news items regarding defective orders about a vehicle.
/// This could kill still valid warnings (for example about void order when just another
/// order gets added), but assume the company will notice the problems, when they're changing
/// the orders.
fn delete_order_warnings(v: VehicleRef, s: &Leaves) {
    crate::news_func::delete_vehicle_news(v.index(s.newgrf()), AdviceType::Order);
}

/// `Depot::Get(id)->xy`: the strict pool lookup, which asserts natively on an invalid
/// index, and the member read from the depot it returns.
#[allow(unsafe_code)]
fn depot_get_xy(id: crate::depot_type::DepotID, s: &Leaves) -> TileIndex {
    let d = (s.depots().get)(id);
    // SAFETY: `Depot::Get` returns the live depot of a valid index.
    TileIndex(unsafe { (s.depots().xy)(d) })
}

/// `Order::GetLocation`: returns a tile somewhat representing the order destination (not
/// suitable for pathfinding). `airport` gets the airport tile and not the station location
/// for aircraft. Returns the destination of the order, or `INVALID_TILE` if none.
pub(crate) fn get_location(
    order: OrderView<'_>,
    v: VehicleRef,
    airport: bool,
    s: &Leaves,
) -> TileIndex {
    match order.kind() {
        OrderType::OT_GOTO_WAYPOINT | OrderType::OT_GOTO_STATION | OrderType::OT_IMPLICIT => {
            if airport && v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                return TileIndex((s.station_commands().airport_tile)(station_get(
                    order.destination().to_station_id(),
                    s,
                )));
            }
            TileIndex((s.shared().base_station_xy)(order.destination().0))
        }
        OrderType::OT_GOTO_DEPOT => {
            if order.destination() == DestinationID(crate::depot_type::DepotID::INVALID.0) {
                return TileIndex::INVALID;
            }
            if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                TileIndex((s.shared().station_xy)(order.destination().0))
            } else {
                depot_get_xy(order.destination().to_depot_id(), s)
            }
        }
        _ => TileIndex::INVALID,
    }
}

/// `GetOrderDistance`: get the distance between two orders of a vehicle. Conditional orders
/// are resolved and the bigger distance of the two order branches is returned. `prev` is the
/// origin order, `cur` the destination order and `conditional_depth` an internal parameter
/// for resolving conditional orders. `list` is the vehicle's `orders`, resolved once by the
/// caller: no call below replaces it.
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the original's conversions"
)]
pub(crate) fn get_order_distance(
    prev: VehicleOrderID,
    cur: VehicleOrderID,
    v: VehicleRef,
    list: Option<OrderListRef>,
    mut conditional_depth: i32,
    s: &Leaves,
) -> u32 {
    assert_with(|| list.is_some());
    let orderlist = deref_list(list, v, s);
    // `orderlist.GetOrders()` indexes the full stored span, not `GetOrderAt`'s narrowed count.
    let orders = |index: VehicleOrderID| orderlist.order(usize::from(index.0)).view();

    if orders(cur).is_type(OrderType::OT_CONDITIONAL) {
        if conditional_depth > i32::from(num_orders(list).0) {
            return 0;
        }

        conditional_depth = conditional_depth.wrapping_add(1);

        // The `uint` results are taken as `int`, compared, and returned as `uint`.
        {
            let dist1 = get_order_distance(
                prev,
                orders(cur).condition_skip_to_order(),
                v,
                list,
                conditional_depth,
                s,
            ) as i32;
            let dist2 =
                get_order_distance(prev, orderlist.get_next(cur), v, list, conditional_depth, s)
                    as i32;
            return dist1.max(dist2) as u32;
        }
    }

    let prev_tile = get_location(orders(prev), v, true, s);
    let cur_tile = get_location(orders(cur), v, true, s);
    if prev_tile == TileIndex::INVALID || cur_tile == TileIndex::INVALID {
        return 0;
    }
    let map = s.shared().map();
    if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
        map.distance_square(prev_tile, cur_tile)
    } else {
        map.distance_manhattan(prev_tile, cur_tile)
    }
}

/// `InsertOrder`: insert a new order but skip the validation. `new_o` is the new order and
/// `sel_ord` the position the order should be inserted at.
#[allow(unsafe_code)]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's conversions"
)]
pub(crate) fn insert_order(v: VehicleRef, new_o: Order, sel_ord: VehicleOrderID, s: &Leaves) {
    /* Create new order and link in list */
    // `v->orders` afterwards: it holds the order, and every shared vehicle holds the same list.
    let list = match order_list(v, s) {
        None => {
            let list = new_order_list(
                v,
                // SAFETY: The new list's live vector; no handle addresses its orders yet.
                |list| unsafe { (*list.vector).insert(0, new_o) },
                s,
            );
            set_order_list(v, Some(list));
            list
        }
        Some(list) => {
            list.insert_order_at(new_o, sel_ord, s);
            list
        }
    };

    let first = record(list.first_shared_shell(), s);
    let mut u = Some(first);
    delete_order_warnings(first, s);
    while let Some(shared) = u {
        assert_with(|| {
            crate::vehicle_base::order_list(v.0.as_ptr())
                == crate::vehicle_base::order_list(shared.0.as_ptr())
        });
        let c = consist(shared);

        /* If there is added an order before the current one, we need
         * to update the selected order. We do not change implicit/real order indices though.
         * If the new order is between the current implicit order and real order, the implicit order will
         * later skip the inserted order. */
        if sel_ord <= c.cur_real_order_index() {
            let cur = u32::from(c.cur_real_order_index().0) + 1;
            /* Check if we don't go out of bound */
            if cur < u32::from(list.get_num_orders().0) {
                // `uint` stored in `VehicleOrderID`; it is below the order count.
                c.set_cur_real_order_index(VehicleOrderID(cur as u8));
            }
        }
        if sel_ord == c.cur_implicit_order_index() && is_ground_vehicle(shared) {
            /* We are inserting an order just before the current implicit order.
             * We do not know whether we will reach current implicit or the newly inserted order first.
             * So, disable creation of implicit orders until we are on track again. */
            assign_suppress_implicit_orders(shared, true);
        }
        if sel_ord <= c.cur_implicit_order_index() {
            let cur = u32::from(c.cur_implicit_order_index().0) + 1;
            /* Check if we don't go out of bound */
            if cur < u32::from(list.get_num_orders().0) {
                // `uint` stored in `VehicleOrderID`; it is below the order count.
                c.set_cur_implicit_order_index(VehicleOrderID(cur as u8));
            }
        }
        /* Unbunching data is no longer valid. */
        c.reset_depot_unbunching();

        /* Update any possible open window of the vehicle */
        invalidate_vehicle_order(
            shared,
            i32::from(VehicleOrderID::INVALID_VEH_ORDER_ID.0) | (i32::from(sel_ord.0) << 8),
            s,
        );
        u = next_shared(shared, s);
    }

    /* As we insert an order, the order to skip to will be 'wrong'. */
    let mut cur_order_id = VehicleOrderID(0);
    {
        for index in 0..list.len() {
            let order = list.order(index);
            if order.view().is_type(OrderType::OT_CONDITIONAL) {
                let order_id = order.view().condition_skip_to_order();
                if order_id >= sel_ord {
                    order.set_condition_skip_to_order(VehicleOrderID(order_id.0.wrapping_add(1)));
                }
                if order_id == cur_order_id {
                    // `(order_id + 1) % GetNumOrders()` in `int`, narrowed.
                    order.set_condition_skip_to_order(VehicleOrderID(
                        ((i32::from(order_id.0) + 1) % i32::from(list.get_num_orders().0)) as u8,
                    ));
                }
            }
            cur_order_id = VehicleOrderID(cur_order_id.0.wrapping_add(1));
        }
    }

    /* Make sure to rebuild the whole list */
    invalidate_window_classes_data(
        get_window_class_for_vehicle_type(v.vehicle_type()),
        0,
        false,
    );
}

/// `DecloneOrder`: declone an order-list. `dst` is the vehicle whose orders are deleted.
fn declone_order(dst: VehicleRef, flags: DoCommandFlags, s: &Leaves) {
    if flags.test(DoCommandFlag::Execute) {
        delete_vehicle_orders(dst, false, true, s);
        invalidate_vehicle_order(
            dst,
            VehicleInvalidateWindowData::VIWD_REMOVE_ALL_ORDERS.0,
            s,
        );
        invalidate_window_classes_data(
            get_window_class_for_vehicle_type(dst.vehicle_type()),
            0,
            false,
        );
    }
}

/// `CancelLoadingDueToDeletedOrder`: cancel the current loading order of the vehicle as the
/// order was deleted.
fn cancel_loading_due_to_deleted_order(v: VehicleRef) {
    assert_with(|| v.current_order().is_type(OrderType::OT_LOADING));
    /* NON-stop flag is misused to see if a train is in a station that is
     * on its order list or not */
    current_order(v).set_non_stop_type(OrderNonStopFlags::EMPTY);
    /* When full loading, "cancel" that order so the vehicle doesn't
     * stay indefinitely at this station anymore. */
    if v.current_order().is_full_load() {
        current_order(v).set_load_type(OrderLoadType::LoadIfPossible);
    }
}

/// `DeleteOrder`: delete an order but skip the parameter validation. `sel_ord` is the id of
/// the order to be deleted.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's conversions"
)]
pub(crate) fn delete_order(v: VehicleRef, sel_ord: VehicleOrderID, s: &Leaves) {
    // `v->orders`, which every shared vehicle holds; deleting an order keeps the list.
    let list = list_of(v, s);
    list.delete_order_at(sel_ord);

    let first = record(list.first_shared_shell(), s);
    let mut u = Some(first);
    delete_order_warnings(first, s);
    while let Some(shared) = u {
        assert_with(|| {
            crate::vehicle_base::order_list(v.0.as_ptr())
                == crate::vehicle_base::order_list(shared.0.as_ptr())
        });
        let c = consist(shared);

        if sel_ord == c.cur_real_order_index()
            && shared.current_order().is_type(OrderType::OT_LOADING)
        {
            cancel_loading_due_to_deleted_order(shared);
        }

        if sel_ord < c.cur_real_order_index() {
            c.set_cur_real_order_index(VehicleOrderID(c.cur_real_order_index().0.wrapping_sub(1)));
        } else if sel_ord == c.cur_real_order_index() {
            update_real_order_index_in(shared, Some(list));
        }

        if sel_ord < c.cur_implicit_order_index() {
            c.set_cur_implicit_order_index(VehicleOrderID(
                c.cur_implicit_order_index().0.wrapping_sub(1),
            ));
        } else if sel_ord == c.cur_implicit_order_index() {
            /* Make sure the index is valid */
            if c.cur_implicit_order_index() >= list.get_num_orders() {
                c.set_cur_implicit_order_index(VehicleOrderID(0));
            }

            /* Skip non-implicit orders for the implicit-order-index (e.g. if the current implicit order was deleted */
            while c.cur_implicit_order_index() != c.cur_real_order_index()
                && !list
                    .order(usize::from(c.cur_implicit_order_index().0))
                    .view()
                    .is_type(OrderType::OT_IMPLICIT)
            {
                c.set_cur_implicit_order_index(VehicleOrderID(
                    c.cur_implicit_order_index().0.wrapping_add(1),
                ));
                if c.cur_implicit_order_index() >= list.get_num_orders() {
                    c.set_cur_implicit_order_index(VehicleOrderID(0));
                }
            }
        }
        /* Unbunching data is no longer valid. */
        c.reset_depot_unbunching();

        /* Update any possible open window of the vehicle */
        invalidate_vehicle_order(
            shared,
            i32::from(sel_ord.0) | (i32::from(VehicleOrderID::INVALID_VEH_ORDER_ID.0) << 8),
            s,
        );
        u = next_shared(shared, s);
    }

    /* As we delete an order, the order to skip to will be 'wrong'. */
    let mut cur_order_id = VehicleOrderID(0);
    {
        for index in 0..list.len() {
            let order = list.order(index);
            if order.view().is_type(OrderType::OT_CONDITIONAL) {
                let mut order_id = i32::from(order.view().condition_skip_to_order().0);
                if order_id >= i32::from(sel_ord.0) {
                    order_id = (order_id - 1).max(0);
                }
                if order_id == i32::from(cur_order_id.0) {
                    order_id = (order_id + 1) % i32::from(list.get_num_orders().0);
                }
                // The `int` result stored in `VehicleOrderID`.
                order.set_condition_skip_to_order(VehicleOrderID(order_id as u8));
            }
            cur_order_id = VehicleOrderID(cur_order_id.0.wrapping_add(1));
        }
    }

    invalidate_window_classes_data(
        get_window_class_for_vehicle_type(v.vehicle_type()),
        0,
        false,
    );
}

/// `DeleteVehicleOrders`: delete all orders from a vehicle. With `keep_orderlist` the order
/// list is not freed, only emptied. With `reset_order_indices`, `cur_implicit_order_index`
/// and `cur_real_order_index` are reset and the current full load order is cancelled (if the
/// vehicle is loading); without it, the caller makes sure the order indices are valid after
/// messing with them.
pub(crate) fn delete_vehicle_orders(
    v: VehicleRef,
    keep_orderlist: bool,
    reset_order_indices: bool,
    s: &Leaves,
) {
    delete_order_warnings(v, s);

    let list = order_list(v, s);
    if list.is_some_and(OrderListRef::is_shared) {
        /* Remove ourself from the shared order list. */
        remove_from_shared(v, s);
        set_order_list(v, None);
    } else if let Some(list) = list {
        /* Remove the orders */
        list.free_chain(keep_orderlist, s);
        if !keep_orderlist {
            set_order_list(v, None);
        }
    }

    let c = consist(v);
    /* Unbunching data is no longer valid. */
    c.reset_depot_unbunching();

    if reset_order_indices {
        c.set_cur_real_order_index(VehicleOrderID(0));
        c.set_cur_implicit_order_index(VehicleOrderID(0));
        if v.current_order().is_type(OrderType::OT_LOADING) {
            cancel_loading_due_to_deleted_order(v);
        }
    }
}

/// `CheckAircraftOrderDistance`: check if an aircraft has enough range for an order list.
/// `v_new` is the aircraft to check and `v_order` the vehicle currently holding the order
/// list. True if the aircraft has enough range for the orders.
#[allow(unsafe_code)]
fn check_aircraft_order_distance(v_new: VehicleRef, v_order: VehicleRef, s: &Leaves) -> bool {
    let aircraft = crate::aircraft::Vehicle::of(v_new, v_new.index(s.newgrf()));
    if aircraft.cached_max_range() == 0 {
        return true;
    }
    // `v_order->orders` is resolved once: nothing below replaces it.
    let list = order_list(v_order, s);
    if num_orders(list).0 == 0 {
        return true;
    }

    let orderlist = deref_list(list, v_order, s);

    /* Iterate over all orders to check the distance between all
     * 'goto' orders and their respective next order (of any type). */
    let mut cur = VehicleOrderID(0);
    while cur < orderlist.get_num_orders() {
        match orderlist.order(usize::from(cur.0)).view().kind() {
            /* If we don't have a next order, we've reached the end and must check the first order instead. */
            OrderType::OT_GOTO_STATION | OrderType::OT_GOTO_DEPOT | OrderType::OT_GOTO_WAYPOINT
                if get_order_distance(cur, orderlist.get_next(cur), v_order, list, 0, s)
                    > aircraft.cached_max_range_sqr() =>
            {
                return false;
            }
            _ => {}
        }
        cur = VehicleOrderID(cur.0.wrapping_add(1));
    }

    true
}

/// `CheckOrders`: check the orders of a vehicle, to see if there are invalid orders and
/// stuff.
#[allow(unsafe_code)]
pub(crate) fn check_orders(v: VehicleRef, s: &Leaves) {
    // SAFETY: The settings catalogue addresses the process-wide setting; one field-sized load.
    let order_review_system = || unsafe { s.shared().settings().order_review_system.read() };
    /* Does the user wants us to check things? */
    if order_review_system() == 0 {
        return;
    }

    /* Do nothing for crashed vehicles */
    if v.vehstatus().test(VehState::Crashed) {
        return;
    }

    /* Do nothing for stopped vehicles if setting is '1' */
    if order_review_system() == 1 && v.vehstatus().test(VehState::Stopped) {
        return;
    }

    // `v->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);

    /* do nothing we we're not the first vehicle in a share-chain */
    if first_shared_in(list, v, s) != Some(v) {
        return;
    }

    /* Only check every 20 days, so that we don't flood the message log */
    if v.owner() == CompanyID((s.shared().local_company)()) && v.day_counter().is_multiple_of(20) {
        let mut message = StringID::INVALID_STRING_ID;

        /* Check the order list */
        let mut n_st = 0i32;

        if let Some(list) = list {
            for index in 0..list.len() {
                let order = list.order(index).view();
                /* Dummy order? */
                if order.is_type(OrderType::OT_DUMMY) {
                    message = STR_NEWS_VEHICLE_HAS_VOID_ORDER;
                    break;
                }
                /* Does station have a load-bay for this vehicle? */
                if order.is_type(OrderType::OT_GOTO_STATION) {
                    let airports = crate::newgrf_airport::runtime();
                    let st = crate::newgrf_airport::station_by_id(
                        order.destination().to_station_id().0,
                        airports,
                    );

                    n_st = n_st.wrapping_add(1);
                    if !crate::vehicle_func::can_vehicle_use_station(v, st, airports) {
                        message = STR_NEWS_VEHICLE_HAS_INVALID_ENTRY;
                    } else if v.vehicle_type() == VehicleType::VEH_AIRCRAFT
                        && ((s.base.engine)(v).aircraft_subtype() & crate::engine_type::AIR_FAST)
                            != 0
                        && (airports.fta_flags)(crate::newgrf_airport::station_fta(st, airports))
                            & crate::airport::FTA_SHORT_STRIP
                            != 0
                        && !(s.shared().no_jetcrash)()
                        && message == StringID::INVALID_STRING_ID
                    {
                        message = STR_NEWS_PLANE_USES_TOO_SHORT_RUNWAY;
                    }
                }
            }
        }

        /* Check if the last and the first order are the same */
        if num_orders(list).0 > 1 {
            let list = deref_list(list, v, s);

            if list
                .order(0)
                .view()
                .equals(list.order(list.len() - 1).view())
            {
                message = STR_NEWS_VEHICLE_HAS_DUPLICATE_ENTRY;
            }
        }

        /* Do we only have 1 station in our order list? */
        if n_st < 2 && message == StringID::INVALID_STRING_ID {
            message = STR_NEWS_VEHICLE_HAS_TOO_FEW_ORDERS;
        }

        if let Some(list) = list {
            // SAFETY: The live list shell; the C++ body checks it under `WITH_ASSERT`.
            unsafe { (s.debug_check_sanity)(list.shell()) };
        }

        /* We don't have a problem */
        if message == StringID::INVALID_STRING_ID {
            return;
        }

        let index = v.index(s.newgrf());
        crate::news_func::add_vehicle_advice_news_item(
            AdviceType::Order,
            &crate::strings_func::get_encoded_string(
                message,
                &[crate::strings_type::StringParameterData::Integer(
                    u64::from(index.0),
                )],
            ),
            index,
        );
    }
}

/// `RemoveOrderFromAllVehicles`: removes an order from all vehicles. Triggers when, say, a
/// station is removed. `type` is the type of the order (`OT_GOTO_[STATION|DEPOT|WAYPOINT]`)
/// and `destination` a `StationID`, `DepotID` or `WaypointID`. `hangar` is only used for
/// airports in the destination: when false, remove airport and hangar orders; when true,
/// remove either airport or hangar order.
pub(crate) fn remove_order_from_all_vehicles(
    r#type: OrderType,
    destination: DestinationID,
    hangar: bool,
    s: &Leaves,
) {
    /* Aircraft have StationIDs for depot orders and never use DepotIDs
     * This fact is handled specially below
     */

    /* Go through all vehicles */
    let mut from = 0;
    while let Some(v) = (s.shared().vehicle_next)(from) {
        from = v.index(s.newgrf()).0.wrapping_add(1);
        let is_aircraft = v.vehicle_type() == VehicleType::VEH_AIRCRAFT;
        if (if is_aircraft && v.current_order().is_type(OrderType::OT_GOTO_DEPOT) && !hangar {
            OrderType::OT_GOTO_STATION
        } else {
            v.current_order().kind()
        }) == r#type
            && (!hangar || is_aircraft)
            && v.current_order().destination() == destination
        {
            current_order(v).make_dummy();
            set_window_dirty(
                WindowClass::WC_VEHICLE_VIEW,
                WindowNumber::from(v.index(s.newgrf())),
            );
        }

        let Some(list) = order_list(v, s) else {
            continue;
        };

        /* Clear the order from the order-list */
        // `v->orders` is resolved once: deleting an order keeps the list.
        let mut next_id = VehicleOrderID(0);
        while next_id < list.get_num_orders() {
            let id = next_id;
            next_id = VehicleOrderID(id.0.wrapping_add(1));
            let order = list.order(usize::from(id.0));
            let mut ot = order.view().kind();
            if ot == OrderType::OT_GOTO_DEPOT
                && order
                    .view()
                    .depot_action_flags()
                    .test(OrderDepotActionFlag::NearestDepot)
            {
                continue;
            }
            if ot == OrderType::OT_GOTO_DEPOT && hangar && !is_aircraft {
                continue; // Not an aircraft? Can't have a hangar order.
            }
            if ot == OrderType::OT_IMPLICIT
                || (is_aircraft && ot == OrderType::OT_GOTO_DEPOT && !hangar)
            {
                ot = OrderType::OT_GOTO_STATION;
            }
            if ot == r#type && order.view().destination() == destination {
                /* We want to clear implicit orders, but we don't want to make them
                 * dummy orders. They should just vanish. Also check the actual order
                 * type as ot is currently OT_GOTO_STATION. */
                if order.view().is_type(OrderType::OT_IMPLICIT) {
                    delete_order(v, id, s);
                    next_id = id;
                    continue;
                }

                /* Clear wait time */
                list.update_total_duration(-i32::from(order.view().wait_time()));
                if order.view().is_wait_timetabled() {
                    list.update_timetable_duration(-i32::from(order.view().timetabled_wait()));
                    order.set_wait_timetabled(false);
                }
                order.set_wait_time(0);

                /* Clear order, preserving travel time */
                let travel_timetabled = order.view().is_travel_timetabled();
                order.make_dummy();
                order.set_travel_timetabled(travel_timetabled);

                let mut w = list.get_first_shared_vehicle(s);
                while let Some(shared) = w {
                    /* In GUI, simulate by removing the order and adding it back */
                    invalidate_vehicle_order(
                        shared,
                        i32::from(id.0) | (i32::from(VehicleOrderID::INVALID_VEH_ORDER_ID.0) << 8),
                        s,
                    );
                    invalidate_vehicle_order(
                        shared,
                        (i32::from(VehicleOrderID::INVALID_VEH_ORDER_ID.0) << 8) | i32::from(id.0),
                        s,
                    );
                    w = next_shared(shared, s);
                }
            }
        }
    }

    order_backup_remove_order(r#type, destination, hangar, s);
}

/// `CheckForValidOrders`: check if a vehicle has any valid orders. False if there are no
/// valid orders; conditional orders are not considered valid destination orders.
#[allow(
    clippy::redundant_closure_for_method_calls,
    reason = "the view's lifetime is per call"
)]
fn check_for_valid_orders(list: Option<OrderListRef>) -> bool {
    any_order_in(list, |order| order.is_goto_order())
}

/// `OrderConditionCompare`: compare the variable and value based on the given comparator.
pub(crate) fn order_condition_compare(
    occ: OrderConditionComparator,
    variable: i32,
    value: i32,
) -> bool {
    match occ {
        OrderConditionComparator::Equal => variable == value,
        OrderConditionComparator::NotEqual => variable != value,
        OrderConditionComparator::LessThan => variable < value,
        OrderConditionComparator::LessThanOrEqual => variable <= value,
        OrderConditionComparator::MoreThan => variable > value,
        OrderConditionComparator::MoreThanOrEqual => variable >= value,
        OrderConditionComparator::IsTrue => variable != 0,
        OrderConditionComparator::IsFalse => variable == 0,
        _ => not_reached(),
    }
}

/// `ProcessConditionalOrder`: process a conditional order and determine the next order.
/// `order` is the order the vehicle currently has. Returns the index of next order to jump
/// to, or `INVALID_VEH_ORDER_ID` to use the next order.
pub(crate) fn process_conditional_order(
    order: OrderView<'_>,
    v: VehicleRef,
    s: &Leaves,
) -> VehicleOrderID {
    if order.kind() != OrderType::OT_CONDITIONAL {
        return VehicleOrderID::INVALID_VEH_ORDER_ID;
    }

    let occ = order.condition_comparator();
    let value = i32::from(order.condition_value());

    let skip_order = match order.condition_variable() {
        OrderConditionVariable::LoadPercentage => order_condition_compare(
            occ,
            i32::from(crate::vehicle::visits::percent_filled(v, None, s.visits())),
            value,
        ),
        OrderConditionVariable::Reliability => order_condition_compare(
            occ,
            crate::math::to_percent16(u32::from(v.reliability(s.newgrf()))).cast_signed(),
            value,
        ),
        OrderConditionVariable::MaxReliability => order_condition_compare(
            occ,
            crate::math::to_percent16(u32::from((s.base.engine)(v).reliability())).cast_signed(),
            value,
        ),
        OrderConditionVariable::MaxSpeed => {
            order_condition_compare(occ, v.get_display_max_speed().wrapping_mul(10) / 16, value)
        }
        OrderConditionVariable::Age => order_condition_compare(
            occ,
            crate::timer_game_common::date_to_year(v.age(s.newgrf()).0),
            value,
        ),
        OrderConditionVariable::RequiresService => order_condition_compare(
            occ,
            i32::from(crate::vehicle::maintenance::needs_servicing(v, s.base)),
            value,
        ),
        OrderConditionVariable::Unconditionally => true,
        OrderConditionVariable::RemainingLifetime => order_condition_compare(
            occ,
            crate::timer_game_common::date_to_year(
                v.max_age(s.newgrf())
                    .0
                    .wrapping_sub(v.age(s.newgrf()).0)
                    .wrapping_add(DAYS_IN_LEAP_YEAR)
                    .wrapping_sub(1),
            )
            .max(0),
            value,
        ),
        _ => not_reached(),
    };

    if skip_order {
        order.condition_skip_to_order()
    } else {
        VehicleOrderID::INVALID_VEH_ORDER_ID
    }
}

/// `UpdateOrderDest`: update the vehicle's destination tile from an order. `order` is the
/// order the vehicle currently has; `conditional_depth` the depth (amount of steps) to go
/// with conditional orders, which prevents infinite loops; `pbs_look_ahead` whether we are
/// forecasting orders for pbs reservations in advance, in which case the order indices must
/// not be modified. Returns true if the order was a new (non-implicit) order.
pub(crate) fn update_order_dest(
    v: VehicleRef,
    order: OrderView<'_>,
    conditional_depth: i32,
    pbs_look_ahead: bool,
    s: &Leaves,
) -> bool {
    update_order_dest_in(
        v,
        order_list(v, s),
        order,
        conditional_depth,
        pbs_look_ahead,
        s,
    )
}

/// [`update_order_dest`] with `list`, the vehicle's `orders`, resolved by the caller: nothing
/// in the body or its recursion replaces the vehicle's list.
#[allow(clippy::too_many_lines, reason = "one function per original body")]
#[allow(unsafe_code)]
#[allow(clippy::cast_sign_loss, reason = "the positive `DAY_TICKS` as `uint`")]
fn update_order_dest_in(
    v: VehicleRef,
    list: Option<OrderListRef>,
    order: OrderView<'_>,
    conditional_depth: i32,
    pbs_look_ahead: bool,
    s: &Leaves,
) -> bool {
    if conditional_depth > i32::from(num_orders(list).0) {
        current_order(v).free();
        crate::vehicle_base::set_dest_tile(v, TileIndex(0));
        return false;
    }

    match order.kind() {
        OrderType::OT_GOTO_STATION => {
            let tile = get_order_station_location(v, order.destination().to_station_id(), s);
            crate::vehicle_base::set_dest_tile(v, tile);
            return true;
        }

        OrderType::OT_GOTO_DEPOT => {
            if order.depot_type().test(OrderDepotTypeFlag::Service)
                && !crate::vehicle::maintenance::needs_servicing(v, s.base)
            {
                assert_with(|| !pbs_look_ahead);
                update_vehicle_timetable(v, true, s);
                increment_real_order_index(v, s);
            } else if v
                .current_order()
                .depot_action_flags()
                .test(OrderDepotActionFlag::NearestDepot)
            {
                /* If the vehicle can't find its destination, delay its next search.
                 * In case many vehicles are in this state, use the vehicle index to spread out pathfinder calls. */
                // `v->index % Ticks::DAY_TICKS` compares with the `uint16_t` fraction in `uint`.
                let spread = v.index(s.newgrf()).0 % (DAY_TICKS as u32);
                if !(v.dest_tile() == TileIndex(0)
                    && u32::from(crate::timer_game_common::economy_fraction()) != spread)
                {
                    /* We need to search for the nearest depot (hangar). */
                    let c = s.scheduler();
                    let closest_depot = crate::vehicle_base::find_closest_depot(v, c);

                    if closest_depot.found {
                        /* PBS reservations cannot reverse */
                        if pbs_look_ahead && closest_depot.reverse {
                            return false;
                        }

                        crate::vehicle_base::set_dest_tile(v, closest_depot.location);
                        current_order(v).set_destination(closest_depot.destination);

                        /* If there is no depot in front, reverse automatically (trains only) */
                        if v.vehicle_type() == VehicleType::VEH_TRAIN && closest_depot.reverse {
                            reverse_train_direction(v, s);
                        }

                        if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                            let a = crate::aircraft::Vehicle::of(v, v.index(s.newgrf()));
                            if a.state() == crate::airport::FLYING
                                && DestinationID(a.targetairport().0) != closest_depot.destination
                            {
                                /* The aircraft is now heading for a different hangar than the next in the orders */
                                crate::aircraft::aircraft_next_airport_pos_and_order(
                                    c.aircraft(),
                                    a,
                                );
                            }
                        }
                        return true;
                    }

                    /* If there is no depot, we cannot help PBS either. */
                    if pbs_look_ahead {
                        return false;
                    }

                    update_vehicle_timetable(v, true, s);
                    increment_real_order_index(v, s);
                }
            } else {
                if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                    let a = crate::aircraft::Vehicle::of(v, v.index(s.newgrf()));
                    let destination = v.current_order().destination();
                    if DestinationID(a.targetairport().0) != destination {
                        /* The aircraft is now heading for a different hangar than the next in the orders */
                        let tile = get_order_station_location(v, destination.to_station_id(), s);
                        crate::vehicle_base::set_dest_tile(v, tile);
                    }
                } else {
                    crate::vehicle_base::set_dest_tile(
                        v,
                        depot_get_xy(order.destination().to_depot_id(), s),
                    );
                }
                return true;
            }
        }

        OrderType::OT_GOTO_WAYPOINT => {
            crate::vehicle_base::set_dest_tile(
                v,
                TileIndex((s.shared().base_station_xy)(order.destination().0)),
            );
            return true;
        }

        OrderType::OT_CONDITIONAL => {
            assert_with(|| !pbs_look_ahead);
            let next_order = process_conditional_order(order, v, s);
            if next_order == VehicleOrderID::INVALID_VEH_ORDER_ID {
                update_vehicle_timetable(v, true, s);
                increment_real_order_index(v, s);
            } else {
                let con = consist(v);
                /* Jump to next_order. cur_implicit_order_index becomes exactly that order,
                 * cur_real_order_index might come after next_order. */
                update_vehicle_timetable(v, false, s);
                con.set_cur_real_order_index(next_order);
                con.set_cur_implicit_order_index(next_order);
                update_real_order_index_in(v, list);
                con.set_current_order_time(
                    con.current_order_time().wrapping_add(i32::from(
                        deref_list(list, v, s)
                            .order(usize::from(con.cur_real_order_index().0))
                            .view()
                            .timetabled_travel(),
                    )),
                );

                /* Disable creation of implicit orders.
                 * When inserting them we do not know that we would have to make the conditional orders point to them. */
                if is_ground_vehicle(v) {
                    assign_suppress_implicit_orders(v, true);
                }
            }
        }

        _ => {
            crate::vehicle_base::set_dest_tile(v, TileIndex(0));
            return false;
        }
    }

    let con = consist(v);
    assert_with(|| con.cur_implicit_order_index() < num_orders(list));
    assert_with(|| con.cur_real_order_index() < num_orders(list));

    /* Get the current order */
    let mut order = order_in(list, con.cur_real_order_index()).map(OrderRef::view);
    if order.is_some_and(|order| order.is_type(OrderType::OT_IMPLICIT)) {
        assert_with(|| num_manual_orders(list).0 == 0);
        order = None;
    }

    let Some(order) = order else {
        current_order(v).free();
        crate::vehicle_base::set_dest_tile(v, TileIndex(0));
        return false;
    };

    current_order(v).copy_assign(order.copied());
    update_order_dest_in(
        v,
        list,
        order,
        conditional_depth.wrapping_add(1),
        pbs_look_ahead,
        s,
    )
}

/// `v->GetOrderStationLocation(station)`, through the station command table's binding of the
/// virtual.
fn get_order_station_location(v: VehicleRef, station: StationID, s: &Leaves) -> TileIndex {
    TileIndex((s.station_commands().order_station_location)(v, station.0))
}

/// `Command<CMD_REVERSE_TRAIN_DIRECTION>::Do(DoCommandFlag::Execute, v->index, false)`.
#[allow(unsafe_code)]
fn reverse_train_direction(v: VehicleRef, s: &Leaves) {
    // SAFETY: The maintenance table names the process-lifetime transaction table.
    let build = (unsafe { &*s.maintenance().transaction }.build_services)();
    let c = s.scheduler();
    let veh_id = v.index(s.newgrf());
    command_do(
        s,
        DoCommandFlags::from_element(DoCommandFlag::Execute),
        |out, flags| {
            crate::train::cmd_reverse_train_direction(
                build,
                out.0,
                flags,
                crate::train::CmdReverseTrainDirection {
                    veh_id,
                    reverse_single_veh: false,
                },
                c.train(),
                c.shared(),
            );
        },
    );
}

/// `ProcessOrders`: handle the orders of a vehicle and determine the next place to go to if
/// needed. Returns true if the vehicle is eligible for reversing (basically only when leaving
/// a station).
pub(crate) fn process_orders(v: VehicleRef, s: &Leaves) -> bool {
    match v.current_order().kind() {
        OrderType::OT_GOTO_DEPOT => {
            /* Let a depot order in the orderlist interrupt. */
            if !v
                .current_order()
                .depot_type()
                .test(OrderDepotTypeFlag::PartOfOrders)
            {
                return false;
            }
        }

        OrderType::OT_LOADING => return false,

        OrderType::OT_LEAVESTATION if v.vehicle_type() != VehicleType::VEH_AIRCRAFT => {
            return false;
        }

        _ => {}
    }

    /*
     * Reversing because of order change is allowed only just after leaving a
     * station (and the difficulty setting to allowed, of course)
     * this can be detected because only after OT_LEAVESTATION, current_order
     * will be reset to nothing. (That also happens if no order, but in that case
     * it won't hit the point in code where may_reverse is checked)
     */
    let may_reverse = v.current_order().is_type(OrderType::OT_NOTHING);

    /* Check if we've reached a 'via' destination. */
    if ((v.current_order().is_type(OrderType::OT_GOTO_STATION)
        && v.current_order()
            .non_stop()
            .test(OrderNonStopFlag::NoDestination))
        || v.current_order().is_type(OrderType::OT_GOTO_WAYPOINT))
        && {
            // One map view for the two tile reads, built only where the original reads them.
            let map = crate::map_func::World::new(s.shared());
            map.is_tile_type(v.tile(), crate::tile_type::TileType::MP_STATION)
                && v.current_order().destination().to_station_id()
                    == map.get_station_index(v.tile())
        }
    {
        delete_unreached_implicit_orders(v, s);
        /* We set the last visited station here because we do not want
         * the train to stop at this 'via' station if the next order
         * is a no-non-stop order; in that case not setting the last
         * visited station will cause the vehicle to still stop. */
        v.set_last_station_visited(v.current_order().destination().to_station_id());
        update_vehicle_timetable(v, true, s);
        increment_implicit_order_index(v, s);
    }

    /* Get the current order */
    let c = consist(v);
    // `this->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);
    assert_with(|| {
        c.cur_implicit_order_index().0 == 0
            || c.cur_implicit_order_index()
                < list.map_or(VehicleOrderID(0), OrderListRef::get_num_orders)
    });
    update_real_order_index_in(v, list);

    let mut order = list
        .and_then(|list| list.get_order_at(c.cur_real_order_index()))
        .map(OrderRef::view);
    if order.is_some_and(|order| order.is_type(OrderType::OT_IMPLICIT)) {
        assert_with(|| list.map_or(0, |list| list.get_num_manual_orders().0) == 0);
        order = None;
    }

    /* If no order, do nothing. */
    let order = match order {
        Some(order)
            if v.vehicle_type() != VehicleType::VEH_AIRCRAFT || check_for_valid_orders(list) =>
        {
            order
        }
        _ => {
            if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                /* Aircraft do something vastly different here, so handle separately */
                crate::aircraft::handle_missing_aircraft_orders(
                    s.scheduler().aircraft(),
                    crate::aircraft::Vehicle::of(v, v.index(s.newgrf())),
                );
                return false;
            }

            current_order(v).free();
            crate::vehicle_base::set_dest_tile(v, TileIndex(0));
            return false;
        }
    };

    /* If it is unchanged, keep it. */
    if order.equals(v.current_order())
        && (v.vehicle_type() == VehicleType::VEH_AIRCRAFT || v.dest_tile() != TileIndex(0))
        && (v.vehicle_type() != VehicleType::VEH_SHIP
            || !order.is_type(OrderType::OT_GOTO_STATION)
            || ship_station_tile(order.destination().to_station_id(), s) != TileIndex::INVALID)
    {
        return false;
    }

    /* Otherwise set it, and determine the destination tile. */
    current_order(v).copy_assign(order.copied());

    invalidate_vehicle_order(v, VehicleInvalidateWindowData::VIWD_MODIFY_ORDERS.0, s);
    match v.vehicle_type() {
        VehicleType::VEH_ROAD | VehicleType::VEH_TRAIN => {}

        VehicleType::VEH_AIRCRAFT | VehicleType::VEH_SHIP => {
            set_window_classes_dirty(get_window_class_for_vehicle_type(v.vehicle_type()));
        }

        _ => not_reached(),
    }

    update_order_dest_in(v, list, order, 0, false, s) && may_reverse
}

/// `Station::Get(st)->ship_station.tile`, read in place through the station command table's
/// `ship_station` accessor.
#[allow(unsafe_code)]
fn ship_station_tile(st: StationID, s: &Leaves) -> TileIndex {
    let area = (s.station_commands().ship_station_area)(station_get(st, s));
    read_field!(area, tile)
}

/// `Command<cmd>::Do(flags, args)` around the Rust body `body` of a command with no tile
/// argument; the result is discarded, as each caller here discards it.
#[allow(unsafe_code)]
fn command_do(s: &Leaves, flags: DoCommandFlags, mut body: impl FnMut(Cost<'_>, DoCommandFlags)) {
    // SAFETY: `with_cost` constructs and destroys the native result around the call.
    unsafe {
        // SAFETY: The station command table names the process-lifetime cost table.
        crate::command::with_cost(&*s.station_commands().cost, |out| {
            crate::command::nested(s.commands(), s.shared(), out, 0, flags.0, |flags| {
                body(out, DoCommandFlags(flags));
            });
        });
    }
}

/// `CmdInsertOrder`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdInsertOrder<'a> {
    /// ID of the vehicle.
    pub(crate) veh: VehicleID,
    /// The selected order (if any). If the last order is given, the order will be inserted
    /// before that one; the maximum vehicle order id is 254.
    pub(crate) sel_ord: VehicleOrderID,
    /// Order to insert.
    pub(crate) new_order: OrderView<'a>,
}

/// `CmdInsertOrder`: add an order to the orderlist of a vehicle. The cost of this operation
/// or an error is written to `cost`.
#[allow(
    clippy::too_many_lines,
    reason = "the original body, with its comments"
)]
#[allow(unsafe_code)]
pub(crate) fn cmd_insert_order(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdInsertOrder<'_>,
) {
    let CmdInsertOrder {
        veh,
        sel_ord,
        new_order,
    } = args;
    let Some(v) = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }
    // `v->orders` is resolved once: nothing below replaces the vehicle's list until
    // `InsertOrder`, after its last read.
    let list = order_list(v, s);

    /* Validate properties we don't want to have different from default as they are set by other commands. */
    if new_order.refit_cargo() != CargoType(CARGO_NO_REFIT)
        || new_order.wait_time() != 0
        || new_order.travel_time() != 0
        || new_order.max_speed() != u16::MAX
    {
        return crate::command_func::cmd_error(cost);
    }

    /* Check if the inserted order is to the correct destination (owner, type),
     * and has the correct flags if any */
    match new_order.kind() {
        OrderType::OT_GOTO_STATION => {
            let st_id = new_order.destination().to_station_id();
            let st = (s.catchment().station_if_valid)(st_id);
            if st.is_null() {
                return crate::command_func::cmd_error(cost);
            }

            if base_station_owner(st, s) != Owner::NONE {
                cost.ownership(s.commands(), base_station_owner(st, s).0, 0);
                if !cost.ok() {
                    return;
                }
            }

            let airports = crate::newgrf_airport::runtime();
            if !crate::vehicle_func::can_vehicle_use_station(v, st, airports) {
                return cost.error_extra(
                    STR_ERROR_CAN_T_ADD_ORDER,
                    get_vehicle_cannot_use_station_reason(v, st, s),
                );
            }
            let mut u = first_shared_in(list, v, s);
            while let Some(shared) = u {
                if !crate::vehicle_func::can_vehicle_use_station(shared, st, airports) {
                    return cost.error_extra(
                        STR_ERROR_CAN_T_ADD_ORDER_SHARED,
                        get_vehicle_cannot_use_station_reason(shared, st, s),
                    );
                }
                u = next_shared(shared, s);
            }

            /* Non stop only allowed for ground vehicles. */
            if new_order.non_stop().any() && !is_ground_vehicle(v) {
                return crate::command_func::cmd_error(cost);
            }

            /* Filter invalid load/unload types. */
            match new_order.load_type() {
                OrderLoadType::LoadIfPossible | OrderLoadType::NoLoad => {}

                OrderLoadType::FullLoad | OrderLoadType::FullLoadAny => {
                    if has_unbunching_order(list) {
                        return cost.error(STR_ERROR_UNBUNCHING_NO_FULL_LOAD.0);
                    }
                }

                _ => return crate::command_func::cmd_error(cost),
            }
            match new_order.unload_type() {
                OrderUnloadType::UnloadIfPossible
                | OrderUnloadType::Unload
                | OrderUnloadType::Transfer
                | OrderUnloadType::NoUnload => {}

                _ => return crate::command_func::cmd_error(cost),
            }

            /* Filter invalid stop locations */
            match new_order.stop_location() {
                OrderStopLocation::NearEnd | OrderStopLocation::Middle => {
                    if v.vehicle_type() != VehicleType::VEH_TRAIN {
                        return crate::command_func::cmd_error(cost);
                    }
                }

                OrderStopLocation::FarEnd => {}

                _ => return crate::command_func::cmd_error(cost),
            }
        }

        OrderType::OT_GOTO_DEPOT => {
            if !new_order
                .depot_action_flags()
                .test(OrderDepotActionFlag::NearestDepot)
            {
                if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
                    let st_id = new_order.destination().to_station_id();
                    let st = (s.catchment().station_if_valid)(st_id);

                    if st.is_null() {
                        return crate::command_func::cmd_error(cost);
                    }

                    cost.ownership(s.commands(), base_station_owner(st, s).0, 0);
                    if !cost.ok() {
                        return;
                    }

                    let airports = crate::newgrf_airport::runtime();
                    if !crate::vehicle_func::can_vehicle_use_station(v, st, airports)
                        || !crate::station_base::airport_has_hangar(st, airports)
                    {
                        return crate::command_func::cmd_error(cost);
                    }
                } else {
                    let dp = (s.depots().depot)(new_order.destination().to_depot_id());

                    if dp.is_null() {
                        return crate::command_func::cmd_error(cost);
                    }

                    // SAFETY: The live depot `Depot::GetIfValid` returned.
                    let xy = TileIndex(unsafe { (s.depots().xy)(dp) });
                    let map = crate::map_func::World::new(s.shared());
                    cost.ownership(s.commands(), map.owner(xy).0, 0);
                    if !cost.ok() {
                        return;
                    }

                    match v.vehicle_type() {
                        VehicleType::VEH_TRAIN => {
                            if !map.is_rail_depot_tile(xy) {
                                return crate::command_func::cmd_error(cost);
                            }
                        }

                        VehicleType::VEH_ROAD => {
                            if !map.road_depot(xy) {
                                return crate::command_func::cmd_error(cost);
                            }
                        }

                        VehicleType::VEH_SHIP => {
                            if !map.is_ship_depot_tile(xy) {
                                return crate::command_func::cmd_error(cost);
                            }
                        }

                        _ => return crate::command_func::cmd_error(cost),
                    }
                }
            }

            if new_order.non_stop().any() && !is_ground_vehicle(v) {
                return crate::command_func::cmd_error(cost);
            }

            /* Check depot order type is valid. */
            let mut depot_order_type = new_order.depot_type();
            if depot_order_type.test(OrderDepotTypeFlag::PartOfOrders) {
                depot_order_type = depot_order_type.reset(OrderDepotTypeFlag::Service);
            }
            depot_order_type = depot_order_type.reset(OrderDepotTypeFlag::PartOfOrders);
            if depot_order_type.any() {
                return crate::command_func::cmd_error(cost);
            }

            /* Check depot action type is valid. */
            if new_order
                .depot_action_flags()
                .reset_all(
                    OrderDepotActionFlags::from_element(OrderDepotActionFlag::Halt)
                        .with(OrderDepotActionFlags::from_element(
                            OrderDepotActionFlag::NearestDepot,
                        ))
                        .with(OrderDepotActionFlags::from_element(
                            OrderDepotActionFlag::Unbunch,
                        )),
                )
                .any()
            {
                return crate::command_func::cmd_error(cost);
            }

            /* Vehicles cannot have a "service if needed" order that also has a depot action. */
            if new_order.depot_type().test(OrderDepotTypeFlag::Service)
                && new_order.depot_action_flags().any_of(
                    OrderDepotActionFlags::from_element(OrderDepotActionFlag::Halt).with(
                        OrderDepotActionFlags::from_element(OrderDepotActionFlag::Unbunch),
                    ),
                )
            {
                return crate::command_func::cmd_error(cost);
            }

            /* Check if we're allowed to have a new unbunching order. */
            if new_order
                .depot_action_flags()
                .test(OrderDepotActionFlag::Unbunch)
            {
                if has_full_load_order(list) {
                    return cost.error(STR_ERROR_UNBUNCHING_NO_UNBUNCHING_FULL_LOAD.0);
                }
                if has_unbunching_order(list) {
                    return cost.error(STR_ERROR_UNBUNCHING_ONLY_ONE_ALLOWED.0);
                }
                if has_conditional_order(list) {
                    return cost.error(STR_ERROR_UNBUNCHING_NO_UNBUNCHING_CONDITIONAL.0);
                }
            }
        }

        OrderType::OT_GOTO_WAYPOINT => {
            let wp = (s.waypoint_get_if_valid)(new_order.destination().0);
            if wp.is_null() {
                return crate::command_func::cmd_error(cost);
            }
            let facilities = s.station_commands().facilities(wp);

            match v.vehicle_type() {
                VehicleType::VEH_TRAIN => {
                    if !facilities.test(crate::station_type::StationFacility::Train) {
                        return cost
                            .error_extra(STR_ERROR_CAN_T_ADD_ORDER, STR_ERROR_NO_RAIL_WAYPOINT);
                    }

                    cost.ownership(s.commands(), base_station_owner(wp, s).0, 0);
                    if !cost.ok() {
                        return;
                    }
                }

                VehicleType::VEH_ROAD => {
                    if !facilities.test(crate::station_type::StationFacility::BusStop)
                        && !facilities.test(crate::station_type::StationFacility::TruckStop)
                    {
                        return cost
                            .error_extra(STR_ERROR_CAN_T_ADD_ORDER, STR_ERROR_NO_ROAD_WAYPOINT);
                    }

                    cost.ownership(s.commands(), base_station_owner(wp, s).0, 0);
                    if !cost.ok() {
                        return;
                    }
                }

                VehicleType::VEH_SHIP => {
                    if !facilities.test(crate::station_type::StationFacility::Dock) {
                        return cost.error_extra(STR_ERROR_CAN_T_ADD_ORDER, STR_ERROR_NO_BUOY);
                    }
                    if base_station_owner(wp, s) != Owner::NONE {
                        cost.ownership(s.commands(), base_station_owner(wp, s).0, 0);
                        if !cost.ok() {
                            return;
                        }
                    }
                }

                _ => return crate::command_func::cmd_error(cost),
            }

            /* Order flags can be any of the following for waypoints:
             * [non-stop]
             * non-stop orders (if any) are only valid for trains and road vehicles */
            if new_order.non_stop().any() && !is_ground_vehicle(v) {
                return crate::command_func::cmd_error(cost);
            }
        }

        OrderType::OT_CONDITIONAL => {
            let skip_to = new_order.condition_skip_to_order();
            if skip_to.0 != 0 && skip_to >= num_orders(list) {
                return crate::command_func::cmd_error(cost); // Always allow jumping to the first (even when there is no order).
            }
            if new_order.condition_variable() >= OrderConditionVariable::End {
                return crate::command_func::cmd_error(cost);
            }
            if has_unbunching_order(list) {
                return cost.error(STR_ERROR_UNBUNCHING_NO_CONDITIONAL.0);
            }

            let occ = new_order.condition_comparator();
            if occ >= OrderConditionComparator::End {
                return crate::command_func::cmd_error(cost);
            }
            match new_order.condition_variable() {
                OrderConditionVariable::RequiresService => {
                    if occ != OrderConditionComparator::IsTrue
                        && occ != OrderConditionComparator::IsFalse
                    {
                        return crate::command_func::cmd_error(cost);
                    }
                }

                OrderConditionVariable::Unconditionally => {
                    if occ != OrderConditionComparator::Equal {
                        return crate::command_func::cmd_error(cost);
                    }
                    if new_order.condition_value() != 0 {
                        return crate::command_func::cmd_error(cost);
                    }
                }

                variable => {
                    if matches!(
                        variable,
                        OrderConditionVariable::LoadPercentage
                            | OrderConditionVariable::Reliability
                            | OrderConditionVariable::MaxReliability
                    ) && new_order.condition_value() > 100
                    {
                        return crate::command_func::cmd_error(cost);
                    }
                    if occ == OrderConditionComparator::IsTrue
                        || occ == OrderConditionComparator::IsFalse
                    {
                        return crate::command_func::cmd_error(cost);
                    }
                }
            }
        }

        _ => return crate::command_func::cmd_error(cost),
    }

    if sel_ord > num_orders(list) {
        return crate::command_func::cmd_error(cost);
    }

    if num_orders(list) >= MAX_VEH_ORDER_ID {
        return cost.error(STR_ERROR_TOO_MANY_ORDERS.0);
    }
    if list.is_none() && (s.list_can_allocate_item)() == 0 {
        return cost.error(STR_ERROR_NO_MORE_SPACE_FOR_ORDERS.0);
    }

    if flags.test(DoCommandFlag::Execute) {
        insert_order(v, new_order.copied(), sel_ord, s);
    }
}

/// `CmdDeleteOrder`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdDeleteOrder {
    /// The ID of the vehicle.
    pub(crate) veh_id: VehicleID,
    /// The order to delete (max 255).
    pub(crate) sel_ord: VehicleOrderID,
}

/// `CmdDeleteOrder`: delete an order from the orderlist of a vehicle.
pub(crate) fn cmd_delete_order(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdDeleteOrder,
) {
    let CmdDeleteOrder { veh_id, sel_ord } = args;
    let Some(v) = vehicle_get_if_valid(veh_id, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    /* If we did not select an order, we maybe want to de-clone the orders */
    // `v->orders` is resolved once for the two tests.
    let list = order_list(v, s);
    if sel_ord >= num_orders(list) {
        return declone_order(v, flags, s);
    }

    if order_in(list, sel_ord).is_none() {
        return crate::command_func::cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        delete_order(v, sel_ord, s);
    }
}

/// `CmdSkipToOrder`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSkipToOrder {
    /// The ID of the vehicle which order is skipped.
    pub(crate) veh_id: VehicleID,
    /// The selected order to which we want to skip.
    pub(crate) sel_ord: VehicleOrderID,
}

/// `CmdSkipToOrder`: goto order of order-list.
pub(crate) fn cmd_skip_to_order(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSkipToOrder,
) {
    let CmdSkipToOrder { veh_id, sel_ord } = args;
    let Some(v) = vehicle_get_if_valid(veh_id, s).filter(|v| {
        v.is_primary_vehicle() && sel_ord != consist(*v).cur_implicit_order_index() && {
            let num_orders = get_num_orders(*v, s);
            sel_ord < num_orders && num_orders.0 >= 2
        }
    }) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        if v.current_order().is_type(OrderType::OT_LOADING) {
            crate::vehicle::visits::leave_station(v, s.visits());
        }

        let c = consist(v);
        c.set_cur_real_order_index(sel_ord);
        c.set_cur_implicit_order_index(sel_ord);
        update_real_order_index(v, s);

        /* Unbunching data is no longer valid. */
        c.reset_depot_unbunching();

        invalidate_vehicle_order(v, VehicleInvalidateWindowData::VIWD_MODIFY_ORDERS.0, s);

        /* We have an aircraft/ship, they have a mini-schedule, so update them all */
        if v.vehicle_type() == VehicleType::VEH_AIRCRAFT {
            set_window_classes_dirty(WindowClass::WC_AIRCRAFT_LIST);
        }
        if v.vehicle_type() == VehicleType::VEH_SHIP {
            set_window_classes_dirty(WindowClass::WC_SHIPS_LIST);
        }
    }
}

/// `CmdMoveOrder`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdMoveOrder {
    /// The ID of the vehicle.
    pub(crate) veh: VehicleID,
    /// The order to move.
    pub(crate) moving_order: VehicleOrderID,
    /// The target order.
    pub(crate) target_order: VehicleOrderID,
}

/// `CmdMoveOrder`: move an order inside the orderlist. The target order will move one place
/// down in the orderlist if you move the order upwards else it'll move it one place down.
pub(crate) fn cmd_move_order(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdMoveOrder,
) {
    let CmdMoveOrder {
        veh,
        moving_order,
        target_order,
    } = args;
    let Some(v) = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    // `v->orders` is resolved once: moving an order keeps the list.
    let list = order_list(v, s);

    /* Don't make senseless movements */
    if moving_order >= num_orders(list)
        || target_order >= num_orders(list)
        || moving_order == target_order
        || num_orders(list).0 <= 1
    {
        return crate::command_func::cmd_error(cost);
    }

    /* Don't move an empty order */
    if order_in(list, moving_order).is_none() {
        return crate::command_func::cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        let list = deref_list(list, v, s);
        list.move_order(moving_order, target_order);

        /* Update shared list */
        let first = record(list.first_shared_shell(), s);
        let mut u = Some(first);

        delete_order_warnings(first, s);

        let remap = |index: VehicleOrderID| {
            if index == moving_order {
                target_order
            } else if index > moving_order && index <= target_order {
                VehicleOrderID(index.0.wrapping_sub(1))
            } else if index < moving_order && index >= target_order {
                VehicleOrderID(index.0.wrapping_add(1))
            } else {
                index
            }
        };
        while let Some(shared) = u {
            let c = consist(shared);
            /* Update the current order.
             * There are multiple ways to move orders, which result in cur_implicit_order_index
             * and cur_real_order_index to not longer make any sense. E.g. moving another
             * real order between them.
             *
             * Basically one could choose to preserve either of them, but not both.
             * While both ways are suitable in this or that case from a human point of view, neither
             * of them makes really sense.
             * However, from an AI point of view, preserving cur_real_order_index is the most
             * predictable and transparent behaviour.
             *
             * With that decision it basically does not matter what we do to cur_implicit_order_index.
             * If we change orders between the implicit- and real-index, the implicit orders are mostly likely
             * completely out-dated anyway. So, keep it simple and just keep cur_implicit_order_index as well.
             * The worst which can happen is that a lot of implicit orders are removed when reaching current_order.
             */
            c.set_cur_real_order_index(remap(c.cur_real_order_index()));
            c.set_cur_implicit_order_index(remap(c.cur_implicit_order_index()));
            /* Unbunching data is no longer valid. */
            c.reset_depot_unbunching();

            assert_with(|| {
                crate::vehicle_base::order_list(v.0.as_ptr())
                    == crate::vehicle_base::order_list(shared.0.as_ptr())
            });
            /* Update any possible open window of the vehicle */
            invalidate_vehicle_order(
                shared,
                i32::from(moving_order.0) | (i32::from(target_order.0) << 8),
                s,
            );
            u = next_shared(shared, s);
        }

        /* As we move an order, the order to skip to will be 'wrong'. */
        {
            for index in 0..list.len() {
                let order = list.order(index);
                if order.view().is_type(OrderType::OT_CONDITIONAL) {
                    order
                        .set_condition_skip_to_order(remap(order.view().condition_skip_to_order()));
                }
            }
        }

        /* Make sure to rebuild the whole list */
        invalidate_window_classes_data(
            get_window_class_for_vehicle_type(v.vehicle_type()),
            0,
            false,
        );
    }
}

/// `CmdModifyOrder`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdModifyOrder {
    /// ID of the vehicle.
    pub(crate) veh: VehicleID,
    /// The selected order (if any). If the last order is given, the order will be inserted
    /// before that one; the maximum vehicle order id is 254.
    pub(crate) sel_ord: VehicleOrderID,
    /// What data to modify (`ModifyOrderFlags`).
    pub(crate) mof: ModifyOrderFlags,
    /// The data to modify.
    pub(crate) data: u16,
}

/// `CmdModifyOrder`: modify an order in the orderlist of a vehicle.
#[allow(
    clippy::too_many_lines,
    reason = "the original body, with its comments"
)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "the original's static_cast to `uint8_t`"
)]
pub(crate) fn cmd_modify_order(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdModifyOrder,
) {
    let CmdModifyOrder {
        veh,
        sel_ord,
        mof,
        data,
    } = args;
    if mof >= ModifyOrderFlags::MOF_END {
        return crate::command_func::cmd_error(cost);
    }

    let Some(v) = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    // `v->orders` is resolved once: modifying an order keeps the list.
    let list = order_list(v, s);

    /* Is it a valid order? */
    if sel_ord >= num_orders(list) {
        return crate::command_func::cmd_error(cost);
    }

    assert_with(|| order_in(list, sel_ord).is_some());
    let order = deref_list(list, v, s).order(usize::from(sel_ord.0));
    match order.view().kind() {
        OrderType::OT_GOTO_STATION => {
            if mof != ModifyOrderFlags::MOF_NON_STOP
                && mof != ModifyOrderFlags::MOF_STOP_LOCATION
                && mof != ModifyOrderFlags::MOF_UNLOAD
                && mof != ModifyOrderFlags::MOF_LOAD
            {
                return crate::command_func::cmd_error(cost);
            }
        }

        OrderType::OT_GOTO_DEPOT => {
            if mof != ModifyOrderFlags::MOF_NON_STOP && mof != ModifyOrderFlags::MOF_DEPOT_ACTION {
                return crate::command_func::cmd_error(cost);
            }
        }

        OrderType::OT_GOTO_WAYPOINT => {
            if mof != ModifyOrderFlags::MOF_NON_STOP {
                return crate::command_func::cmd_error(cost);
            }
        }

        OrderType::OT_CONDITIONAL => {
            if mof != ModifyOrderFlags::MOF_COND_VARIABLE
                && mof != ModifyOrderFlags::MOF_COND_COMPARATOR
                && mof != ModifyOrderFlags::MOF_COND_VALUE
                && mof != ModifyOrderFlags::MOF_COND_DESTINATION
            {
                return crate::command_func::cmd_error(cost);
            }
        }

        _ => return crate::command_func::cmd_error(cost),
    }

    // `static_cast<..>(data)` of the `uint16_t` data to each `uint8_t` enumeration.
    let byte = data as u8;
    match mof {
        ModifyOrderFlags::MOF_NON_STOP => {
            if !is_ground_vehicle(v) {
                return crate::command_func::cmd_error(cost);
            }

            let nonstop_flags = OrderNonStopFlags(byte);
            if nonstop_flags == order.view().non_stop() {
                return crate::command_func::cmd_error(cost);
            }

            /* Test for invalid flags. */
            let nonstop_flags = nonstop_flags.reset_all(
                OrderNonStopFlags::from_element(OrderNonStopFlag::NoIntermediate).with(
                    OrderNonStopFlags::from_element(OrderNonStopFlag::NoDestination),
                ),
            );
            if nonstop_flags.any() {
                return crate::command_func::cmd_error(cost);
            }
        }

        ModifyOrderFlags::MOF_STOP_LOCATION => {
            if v.vehicle_type() != VehicleType::VEH_TRAIN {
                return crate::command_func::cmd_error(cost);
            }
            if data >= u16::from(OrderStopLocation::End.0) {
                return crate::command_func::cmd_error(cost);
            }
        }

        ModifyOrderFlags::MOF_UNLOAD => {
            if order
                .view()
                .non_stop()
                .test(OrderNonStopFlag::NoDestination)
            {
                return crate::command_func::cmd_error(cost);
            }

            let unload_type = OrderUnloadType(byte);
            if unload_type == order.view().unload_type() {
                return crate::command_func::cmd_error(cost);
            }

            /* Test for invalid types. */
            match unload_type {
                OrderUnloadType::UnloadIfPossible
                | OrderUnloadType::Unload
                | OrderUnloadType::Transfer
                | OrderUnloadType::NoUnload => {}

                _ => return crate::command_func::cmd_error(cost),
            }
        }

        ModifyOrderFlags::MOF_LOAD => {
            if order
                .view()
                .non_stop()
                .test(OrderNonStopFlag::NoDestination)
            {
                return crate::command_func::cmd_error(cost);
            }

            let load_type = OrderLoadType(byte);
            if load_type == order.view().load_type() {
                return crate::command_func::cmd_error(cost);
            }

            /* Test for invalid types. */
            match load_type {
                OrderLoadType::LoadIfPossible | OrderLoadType::NoLoad => {}

                OrderLoadType::FullLoad | OrderLoadType::FullLoadAny => {
                    if has_unbunching_order(list) {
                        return cost.error(STR_ERROR_UNBUNCHING_NO_FULL_LOAD.0);
                    }
                }

                _ => return crate::command_func::cmd_error(cost),
            }
        }

        ModifyOrderFlags::MOF_DEPOT_ACTION => {
            let depot_action = OrderDepotAction(byte);
            if depot_action >= OrderDepotAction::End {
                return crate::command_func::cmd_error(cost);
            }
            /* Check if we are allowed to add unbunching. We are always allowed to remove it. */
            if depot_action == OrderDepotAction::Unbunch {
                /* Only one unbunching order is allowed in a vehicle's orders. If this order already has an unbunching action, no error is needed. */
                if has_unbunching_order(list)
                    && !order
                        .view()
                        .depot_action_flags()
                        .test(OrderDepotActionFlag::Unbunch)
                {
                    return cost.error(STR_ERROR_UNBUNCHING_ONLY_ONE_ALLOWED.0);
                }
                /* We don't allow unbunching if the vehicle has a conditional order. */
                if has_conditional_order(list) {
                    return cost.error(STR_ERROR_UNBUNCHING_NO_UNBUNCHING_CONDITIONAL.0);
                }
                /* We don't allow unbunching if the vehicle has a full load order. */
                if has_full_load_order(list) {
                    return cost.error(STR_ERROR_UNBUNCHING_NO_UNBUNCHING_FULL_LOAD.0);
                }
            }
        }

        ModifyOrderFlags::MOF_COND_VARIABLE => {
            let cond_variable = OrderConditionVariable(byte);
            if cond_variable >= OrderConditionVariable::End {
                return crate::command_func::cmd_error(cost);
            }
        }

        ModifyOrderFlags::MOF_COND_COMPARATOR => {
            let cond_comparator = OrderConditionComparator(byte);
            if cond_comparator >= OrderConditionComparator::End {
                return crate::command_func::cmd_error(cost);
            }
            match order.view().condition_variable() {
                OrderConditionVariable::Unconditionally => {
                    return crate::command_func::cmd_error(cost);
                }

                OrderConditionVariable::RequiresService => {
                    if cond_comparator != OrderConditionComparator::IsTrue
                        && cond_comparator != OrderConditionComparator::IsFalse
                    {
                        return crate::command_func::cmd_error(cost);
                    }
                }

                _ => {
                    if cond_comparator == OrderConditionComparator::IsTrue
                        || cond_comparator == OrderConditionComparator::IsFalse
                    {
                        return crate::command_func::cmd_error(cost);
                    }
                }
            }
        }

        ModifyOrderFlags::MOF_COND_VALUE => match order.view().condition_variable() {
            OrderConditionVariable::Unconditionally | OrderConditionVariable::RequiresService => {
                return crate::command_func::cmd_error(cost);
            }

            OrderConditionVariable::LoadPercentage
            | OrderConditionVariable::Reliability
            | OrderConditionVariable::MaxReliability => {
                if data > 100 {
                    return crate::command_func::cmd_error(cost);
                }
            }

            _ => {
                if data > 2047 {
                    return crate::command_func::cmd_error(cost);
                }
            }
        },

        ModifyOrderFlags::MOF_COND_DESTINATION => {
            if data >= u16::from(num_orders(list).0) {
                return crate::command_func::cmd_error(cost);
            }
        }

        _ => not_reached(),
    }

    if flags.test(DoCommandFlag::Execute) {
        match mof {
            ModifyOrderFlags::MOF_NON_STOP => {
                order.set_non_stop_type(OrderNonStopFlags(byte));
                if order
                    .view()
                    .non_stop()
                    .test(OrderNonStopFlag::NoDestination)
                {
                    order.set_refit(CargoType(CARGO_NO_REFIT));
                    order.set_load_type(OrderLoadType::LoadIfPossible);
                    order.set_unload_type(OrderUnloadType::UnloadIfPossible);
                }
            }

            ModifyOrderFlags::MOF_STOP_LOCATION => {
                order.set_stop_location(OrderStopLocation(byte));
            }

            ModifyOrderFlags::MOF_UNLOAD => {
                order.set_unload_type(OrderUnloadType(byte));
            }

            ModifyOrderFlags::MOF_LOAD => {
                order.set_load_type(OrderLoadType(byte));
                if order.view().load_type() == OrderLoadType::NoLoad {
                    order.set_refit(CargoType(CARGO_NO_REFIT));
                }
            }

            ModifyOrderFlags::MOF_DEPOT_ACTION => {
                let halt_and_unbunch =
                    OrderDepotActionFlags::from_element(OrderDepotActionFlag::Halt).with(
                        OrderDepotActionFlags::from_element(OrderDepotActionFlag::Unbunch),
                    );
                match OrderDepotAction(byte) {
                    OrderDepotAction::AlwaysGo => {
                        order.set_depot_order_type(
                            order.view().depot_type().reset(OrderDepotTypeFlag::Service),
                        );
                        order.set_depot_action_type(
                            order
                                .view()
                                .depot_action_flags()
                                .reset_all(halt_and_unbunch),
                        );
                    }

                    OrderDepotAction::Service => {
                        order.set_depot_order_type(
                            order.view().depot_type().set(OrderDepotTypeFlag::Service),
                        );
                        order.set_depot_action_type(
                            order
                                .view()
                                .depot_action_flags()
                                .reset_all(halt_and_unbunch),
                        );
                        order.set_refit(CargoType(CARGO_NO_REFIT));
                    }

                    OrderDepotAction::Stop => {
                        order.set_depot_order_type(
                            order.view().depot_type().reset(OrderDepotTypeFlag::Service),
                        );
                        order.set_depot_action_type(
                            order
                                .view()
                                .depot_action_flags()
                                .set(OrderDepotActionFlag::Halt)
                                .reset(OrderDepotActionFlag::Unbunch),
                        );
                        order.set_refit(CargoType(CARGO_NO_REFIT));
                    }

                    OrderDepotAction::Unbunch => {
                        order.set_depot_order_type(
                            order.view().depot_type().reset(OrderDepotTypeFlag::Service),
                        );
                        order.set_depot_action_type(
                            order
                                .view()
                                .depot_action_flags()
                                .reset(OrderDepotActionFlag::Halt)
                                .set(OrderDepotActionFlag::Unbunch),
                        );
                    }

                    _ => not_reached(),
                }
            }

            ModifyOrderFlags::MOF_COND_VARIABLE => {
                order.set_condition_variable(OrderConditionVariable(byte));

                let occ = order.view().condition_comparator();
                match order.view().condition_variable() {
                    OrderConditionVariable::Unconditionally => {
                        order.set_condition_comparator(OrderConditionComparator::Equal);
                        order.set_condition_value(0);
                    }

                    OrderConditionVariable::RequiresService => {
                        if occ != OrderConditionComparator::IsTrue
                            && occ != OrderConditionComparator::IsFalse
                        {
                            order.set_condition_comparator(OrderConditionComparator::IsTrue);
                        }
                        order.set_condition_value(0);
                    }

                    variable => {
                        if matches!(
                            variable,
                            OrderConditionVariable::LoadPercentage
                                | OrderConditionVariable::Reliability
                                | OrderConditionVariable::MaxReliability
                        ) && order.view().condition_value() > 100
                        {
                            order.set_condition_value(100);
                        }
                        if occ == OrderConditionComparator::IsTrue
                            || occ == OrderConditionComparator::IsFalse
                        {
                            order.set_condition_comparator(OrderConditionComparator::Equal);
                        }
                    }
                }
            }

            ModifyOrderFlags::MOF_COND_COMPARATOR => {
                order.set_condition_comparator(OrderConditionComparator(byte));
            }

            ModifyOrderFlags::MOF_COND_VALUE => {
                order.set_condition_value(data);
            }

            ModifyOrderFlags::MOF_COND_DESTINATION => {
                order.set_condition_skip_to_order(VehicleOrderID(byte));
            }

            _ => not_reached(),
        }

        /* Update the windows and full load flags, also for vehicles that share the same order list */
        let list = deref_list(list, v, s);
        let first = record(list.first_shared_shell(), s);
        let mut u = Some(first);
        delete_order_warnings(first, s);
        while let Some(shared) = u {
            /* Toggle u->current_order "Full load" flag if it changed.
             * However, as the same flag is used for depot orders, check
             * whether we are not going to a depot as there are three
             * cases where the full load flag can be active and only
             * one case where the flag is used for depot orders. In the
             * other cases for the OrderType the flags are not used,
             * so do not care and those orders should not be active
             * when this function is called.
             */
            let sc = consist(shared);
            if sel_ord == sc.cur_real_order_index()
                && (shared.current_order().is_type(OrderType::OT_GOTO_STATION)
                    || shared.current_order().is_type(OrderType::OT_LOADING))
                && shared.current_order().load_type() != order.view().load_type()
            {
                current_order(shared).set_load_type(order.view().load_type());
            }

            /* Unbunching data is no longer valid. */
            sc.reset_depot_unbunching();

            invalidate_vehicle_order(shared, VehicleInvalidateWindowData::VIWD_MODIFY_ORDERS.0, s);
            u = next_shared(shared, s);
        }
    }
}

/// `CmdCloneOrder`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdCloneOrder {
    /// Action to perform.
    pub(crate) action: CloneOptions,
    /// Destination vehicle to clone orders to.
    pub(crate) veh_dst: VehicleID,
    /// Source vehicle to clone orders from, if any (none for `CO_UNSHARE`).
    pub(crate) veh_src: VehicleID,
}

/// `CmdCloneOrder`: clone/share/copy an order-list of another vehicle.
#[allow(
    clippy::too_many_lines,
    reason = "the original body, with its comments"
)]
#[allow(unsafe_code)]
pub(crate) fn cmd_clone_order(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdCloneOrder,
) {
    let CmdCloneOrder {
        action,
        veh_dst,
        veh_src,
    } = args;
    let Some(dst) = vehicle_get_if_valid(veh_dst, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), dst.owner().0, 0);
    if !cost.ok() {
        return;
    }

    match action {
        CloneOptions::CO_SHARE => {
            let src = vehicle_get_if_valid(veh_src, s);

            /* Sanity checks */
            let Some(src) = src.filter(|src| {
                src.is_primary_vehicle() && dst.vehicle_type() == src.vehicle_type() && dst != *src
            }) else {
                return crate::command_func::cmd_error(cost);
            };

            cost.ownership(s.commands(), src.owner().0, 0);
            if !cost.ok() {
                return;
            }

            /* Trucks can't share orders with busses (and visa versa) */
            if src.vehicle_type() == VehicleType::VEH_ROAD
                && crate::road_stop::is_bus(s.shared(), src)
                    != crate::road_stop::is_bus(s.shared(), dst)
            {
                return crate::command_func::cmd_error(cost);
            }

            // Each vehicle's `orders` is resolved once: nothing before the deletion below
            // replaces either list.
            let src_list = order_list(src, s);
            let dst_list = order_list(dst, s);

            /* Is the vehicle already in the shared list? */
            if first_shared_in(src_list, src, s) == first_shared_in(dst_list, dst, s) {
                return crate::command_func::cmd_error(cost);
            }

            if let Some(list) = src_list {
                for index in 0..list.len() {
                    let order = list.order(index).view();
                    if !order_goes_to_station(dst, order) {
                        continue;
                    }

                    /* Allow copying unreachable destinations if they were already unreachable for the source.
                     * This is basically to allow cloning / autorenewing / autoreplacing vehicles, while the stations
                     * are temporarily invalid due to reconstruction. */
                    let st_id = order.destination().to_station_id();
                    let airports = crate::newgrf_airport::runtime();
                    let st = crate::newgrf_airport::station_by_id(st_id.0, airports);
                    if crate::vehicle_func::can_vehicle_use_station(src, st, airports)
                        && !crate::vehicle_func::can_vehicle_use_station(dst, st, airports)
                    {
                        return cost.error_extra(
                            STR_ERROR_CAN_T_COPY_SHARE_ORDER,
                            get_vehicle_cannot_use_station_reason(dst, st, s),
                        );
                    }
                }
            }

            /* Check for aircraft range limits. */
            if dst.vehicle_type() == VehicleType::VEH_AIRCRAFT
                && !check_aircraft_order_distance(dst, src, s)
            {
                return cost.error(STR_ERROR_AIRCRAFT_NOT_ENOUGH_RANGE.0);
            }

            if src_list.is_none() && (s.list_can_allocate_item)() == 0 {
                return cost.error(STR_ERROR_NO_MORE_SPACE_FOR_ORDERS.0);
            }

            if flags.test(DoCommandFlag::Execute) {
                /* If the destination vehicle had a OrderList, destroy it.
                 * We only reset the order indices, if the new orders are obviously different.
                 * (We mainly do this to keep the order indices valid and in range.) */
                delete_vehicle_orders(dst, false, num_orders(dst_list) != num_orders(src_list), s);

                set_order_list(dst, src_list);

                /* Link this vehicle in the shared-list */
                add_to_shared(dst, src, s);

                invalidate_vehicle_order(
                    dst,
                    VehicleInvalidateWindowData::VIWD_REMOVE_ALL_ORDERS.0,
                    s,
                );
                invalidate_vehicle_order(src, VehicleInvalidateWindowData::VIWD_MODIFY_ORDERS.0, s);

                invalidate_window_classes_data(
                    get_window_class_for_vehicle_type(dst.vehicle_type()),
                    0,
                    false,
                );
            }
        }

        CloneOptions::CO_COPY => {
            let src = vehicle_get_if_valid(veh_src, s);

            /* Sanity checks */
            let Some(src) = src.filter(|src| {
                src.is_primary_vehicle() && dst.vehicle_type() == src.vehicle_type() && dst != *src
            }) else {
                return crate::command_func::cmd_error(cost);
            };

            cost.ownership(s.commands(), src.owner().0, 0);
            if !cost.ok() {
                return;
            }

            // Each vehicle's `orders` is resolved once: nothing before the deletion below
            // replaces either list.
            let src_list = order_list(src, s);
            let dst_list = order_list(dst, s);

            /* Trucks can't copy all the orders from busses (and visa versa),
             * and neither can helicopters and aircraft. */
            if let Some(list) = src_list {
                for index in 0..list.len() {
                    let order = list.order(index).view();
                    if !order_goes_to_station(dst, order) {
                        continue;
                    }
                    let st_id = order.destination().to_station_id();
                    let airports = crate::newgrf_airport::runtime();
                    let st = crate::newgrf_airport::station_by_id(st_id.0, airports);
                    if !crate::vehicle_func::can_vehicle_use_station(dst, st, airports) {
                        return cost.error_extra(
                            STR_ERROR_CAN_T_COPY_SHARE_ORDER,
                            get_vehicle_cannot_use_station_reason(dst, st, s),
                        );
                    }
                }
            }

            /* Check for aircraft range limits. */
            if dst.vehicle_type() == VehicleType::VEH_AIRCRAFT
                && !check_aircraft_order_distance(dst, src, s)
            {
                return cost.error(STR_ERROR_AIRCRAFT_NOT_ENOUGH_RANGE.0);
            }

            /* make sure there are orders available */
            if (s.list_can_allocate_item)() == 0 {
                return cost.error(STR_ERROR_NO_MORE_SPACE_FOR_ORDERS.0);
            }

            if flags.test(DoCommandFlag::Execute) {
                /* If the destination vehicle had an order list, destroy the chain but keep the OrderList.
                 * We only reset the order indices, if the new orders are obviously different.
                 * (We mainly do this to keep the order indices valid and in range.) */
                delete_vehicle_orders(dst, true, num_orders(dst_list) != num_orders(src_list), s);

                let mut dst_orders: Vec<Order> = Vec::new();
                if let Some(list) = src_list {
                    for index in 0..list.len() {
                        dst_orders.push(list.order(index).view().copied());
                    }
                }

                if let Some(list) = order_list(dst, s) {
                    assert_with(|| list.get_num_orders().0 == 0);
                    assert_with(|| !list.is_shared());
                    // SAFETY: The emptied, unshared list's live shell, deleted once.
                    unsafe {
                        (s.delete_list)(list.shell());
                    }
                }

                assert_with(|| (s.list_can_allocate_item)() != 0);
                let list = new_order_list(
                    dst,
                    // SAFETY: The new list's live vector and the local copies outside it.
                    |list| unsafe { (*list.vector).assign(dst_orders.as_ptr(), dst_orders.len()) },
                    s,
                );
                set_order_list(dst, Some(list));

                invalidate_vehicle_order(
                    dst,
                    VehicleInvalidateWindowData::VIWD_REMOVE_ALL_ORDERS.0,
                    s,
                );

                invalidate_window_classes_data(
                    get_window_class_for_vehicle_type(dst.vehicle_type()),
                    0,
                    false,
                );
            }
        }

        CloneOptions::CO_UNSHARE => declone_order(dst, flags, s),
        _ => crate::command_func::cmd_error(cost),
    }
}

/// `CmdOrderRefit`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdOrderRefit {
    /// `VehicleIndex` of the vehicle having the order.
    pub(crate) veh: VehicleID,
    /// Number of order to modify.
    pub(crate) order_number: VehicleOrderID,
    /// `CargoType`.
    pub(crate) cargo: CargoType,
}

/// `CmdOrderRefit`: add/remove refit orders from an order.
pub(crate) fn cmd_order_refit(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdOrderRefit,
) {
    let CmdOrderRefit {
        veh,
        order_number,
        cargo,
    } = args;
    if cargo.0 >= NUM_CARGO && cargo.0 != CARGO_NO_REFIT && cargo.0 != CARGO_AUTO_REFIT {
        return crate::command_func::cmd_error(cost);
    }

    let Some(v) = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }
    // `v->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);

    let Some(order) = order_in(list, order_number) else {
        return crate::command_func::cmd_error(cost);
    };

    /* Automatic refit cargo is only supported for goto station orders. */
    if cargo.0 == CARGO_AUTO_REFIT && !order.view().is_type(OrderType::OT_GOTO_STATION) {
        return crate::command_func::cmd_error(cost);
    }

    if order.view().load_type() == OrderLoadType::NoLoad {
        return crate::command_func::cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        order.set_refit(cargo);

        /* Make the depot order an 'always go' order. */
        if cargo.0 != CARGO_NO_REFIT && order.view().is_type(OrderType::OT_GOTO_DEPOT) {
            order
                .set_depot_order_type(order.view().depot_type().reset(OrderDepotTypeFlag::Service));
            order.set_depot_action_type(
                order
                    .view()
                    .depot_action_flags()
                    .reset(OrderDepotActionFlag::Halt),
            );
        }

        let mut u = first_shared_in(list, v, s);
        while let Some(shared) = u {
            /* Update any possible open window of the vehicle */
            invalidate_vehicle_order(shared, VehicleInvalidateWindowData::VIWD_MODIFY_ORDERS.0, s);

            /* If the vehicle already got the current depot set as current order, then update current order as well */
            if consist(shared).cur_real_order_index() == order_number
                && shared
                    .current_order()
                    .depot_type()
                    .test(OrderDepotTypeFlag::PartOfOrders)
            {
                current_order(shared).set_refit(cargo);
            }
            u = next_shared(shared, s);
        }
    }
}

/// `GetStartTickFromDate`: get the `TimerGameTick::TickCounter` tick of a given date.
/// `start_date` is the date when the timetable starts; returns the first tick of this date.
#[allow(
    clippy::cast_sign_loss,
    reason = "the original's conversion to `TickCounter`"
)]
pub(crate) fn get_start_tick_from_date(start_date: EconomyDate, s: &Leaves) -> u64 {
    /* Calculate the offset in ticks from the current date. */
    let mut tick_offset = start_date
        .0
        .wrapping_sub(crate::timer_game_common::economy_date().0)
        .wrapping_mul(DAY_TICKS);

    /* Compensate for the current date_fract. */
    tick_offset = tick_offset.wrapping_sub(i32::from(crate::timer_game_common::economy_fraction()));

    /* Return the current tick plus the offset. */
    // `TickCounter + Ticks`: the `int32_t` offset converts to `uint64_t`.
    (s.shared().tick_counter)().wrapping_add(tick_offset as u64)
}

/// `GetDateFromStartTick`: get a date from a given start tick of timetable. `start_tick` is
/// the `TimerGameTick::TickCounter` when the timetable starts; returns the date when we reach
/// this tick.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the original's `Ticks` result"
)]
pub(crate) fn get_date_from_start_tick(start_tick: u64, s: &Leaves) -> EconomyDate {
    /* Calculate the offset in ticks from the current counter tick. */
    // `TickCounter - TickCounter` narrowed to `Ticks`.
    let mut tick_offset = start_tick.wrapping_sub((s.shared().tick_counter)()) as i32;

    /* Compensate for the current date_fract. */
    tick_offset = tick_offset.wrapping_add(i32::from(crate::timer_game_common::economy_fraction()));

    /* Return the current date plus the offset in days. */
    EconomyDate(
        crate::timer_game_common::economy_date()
            .0
            .wrapping_add(tick_offset / DAY_TICKS),
    )
}

/// `GetStartTickFromDate`, for the C++ callers.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orders_start_tick(start_date: i32, s: &'static Leaves) -> u64 {
    get_start_tick_from_date(EconomyDate(start_date), s)
}
/// `GetDateFromStartTick`, for the C++ callers.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_orders_start_date(start_tick: u64, s: &'static Leaves) -> i32 {
    get_date_from_start_tick(start_tick, s).0
}

/// `ChangeTimetable`: change/update a particular timetable entry. `order_number` is the index
/// of the timetable in the order list, `val` the new data of the timetable entry, `mtf` which
/// part of the timetable entry to change and `timetabled` whether the new value is explicitly
/// timetabled.
fn change_timetable(
    v: VehicleRef,
    order_number: VehicleOrderID,
    val: u16,
    mtf: ModifyTimetableFlags,
    timetabled: bool,
    s: &Leaves,
) {
    // `v->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);
    assert_with(|| order_in(list, order_number).is_some());
    let order = deref_list(list, v, s).order(usize::from(order_number.0));
    let mut total_delta = 0i32;
    let mut timetable_delta = 0i32;

    match mtf {
        ModifyTimetableFlags::MTF_WAIT_TIME => {
            total_delta = i32::from(val) - i32::from(order.view().wait_time());
            timetable_delta = i32::from(if timetabled { val } else { 0 })
                - i32::from(order.view().timetabled_wait());
            order.set_wait_time(val);
            order.set_wait_timetabled(timetabled);
        }

        ModifyTimetableFlags::MTF_TRAVEL_TIME => {
            total_delta = i32::from(val) - i32::from(order.view().travel_time());
            timetable_delta = i32::from(if timetabled { val } else { 0 })
                - i32::from(order.view().timetabled_travel());
            order.set_travel_time(val);
            order.set_travel_timetabled(timetabled);
        }

        ModifyTimetableFlags::MTF_TRAVEL_SPEED => {
            order.set_max_speed(val);
        }

        _ => not_reached(),
    }
    let list = deref_list(list, v, s);
    list.update_total_duration(total_delta);
    list.update_timetable_duration(timetable_delta);

    let mut u = list.get_first_shared_vehicle(s);
    while let Some(v) = u {
        if consist(v).cur_real_order_index() == order_number
            && v.current_order().equals(order.view())
        {
            let current = current_order(v);
            match mtf {
                ModifyTimetableFlags::MTF_WAIT_TIME => {
                    current.set_wait_time(val);
                    current.set_wait_timetabled(timetabled);
                }

                ModifyTimetableFlags::MTF_TRAVEL_TIME => {
                    current.set_travel_time(val);
                    current.set_travel_timetabled(timetabled);
                }

                ModifyTimetableFlags::MTF_TRAVEL_SPEED => {
                    current.set_max_speed(val);
                }

                _ => not_reached(),
            }
        }
        set_window_dirty(
            WindowClass::WC_VEHICLE_TIMETABLE,
            WindowNumber::from(v.index(s.newgrf())),
        );
        u = next_shared(v, s);
    }
}

/// `CmdChangeTimetable`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdChangeTimetable {
    /// Vehicle with the orders to change.
    pub(crate) veh: VehicleID,
    /// Order index to modify.
    pub(crate) order_number: VehicleOrderID,
    /// Timetable data to change (`ModifyTimetableFlags`).
    pub(crate) mtf: ModifyTimetableFlags,
    /// The data to modify as specified by `mtf`: 0 to clear times, `UINT16_MAX` to clear the
    /// speed limit.
    pub(crate) data: u16,
}

/// `CmdChangeTimetable`: change timetable data of an order.
#[allow(clippy::too_many_lines, reason = "one function per original body")]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's conversions"
)]
pub(crate) fn cmd_change_timetable(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdChangeTimetable,
) {
    let CmdChangeTimetable {
        veh,
        order_number,
        mtf,
        data,
    } = args;
    let Some(v) = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }
    // `v->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);

    let Some(order) =
        order_in(list, order_number).filter(|order| !order.view().is_type(OrderType::OT_IMPLICIT))
    else {
        return crate::command_func::cmd_error(cost);
    };

    if mtf >= ModifyTimetableFlags::MTF_END {
        return crate::command_func::cmd_error(cost);
    }

    let mut wait_time = i32::from(order.view().wait_time());
    let mut travel_time = i32::from(order.view().travel_time());
    let mut max_speed = i32::from(order.view().max_speed());
    match mtf {
        ModifyTimetableFlags::MTF_WAIT_TIME => wait_time = i32::from(data),

        ModifyTimetableFlags::MTF_TRAVEL_TIME => travel_time = i32::from(data),

        ModifyTimetableFlags::MTF_TRAVEL_SPEED => {
            max_speed = i32::from(data);
            if max_speed == 0 {
                max_speed = i32::from(u16::MAX); // Disable speed limit.
            }
        }

        _ => not_reached(),
    }

    if wait_time != i32::from(order.view().wait_time()) {
        match order.view().kind() {
            OrderType::OT_GOTO_STATION => {
                if order
                    .view()
                    .non_stop()
                    .test(OrderNonStopFlag::NoDestination)
                {
                    return cost.error(STR_ERROR_TIMETABLE_NOT_STOPPING_HERE.0);
                }
            }

            OrderType::OT_CONDITIONAL => {}

            _ => return cost.error(STR_ERROR_TIMETABLE_ONLY_WAIT_AT_STATIONS.0),
        }
    }

    if travel_time != i32::from(order.view().travel_time())
        && order.view().is_type(OrderType::OT_CONDITIONAL)
    {
        return crate::command_func::cmd_error(cost);
    }
    if max_speed != i32::from(order.view().max_speed())
        && (order.view().is_type(OrderType::OT_CONDITIONAL)
            || v.vehicle_type() == VehicleType::VEH_AIRCRAFT)
    {
        return crate::command_func::cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        // The `int` times are the `uint16_t` data or member values they were read from.
        match mtf {
            ModifyTimetableFlags::MTF_WAIT_TIME => {
                /* Set time if changing the value or confirming an estimated time as timetabled. */
                if wait_time != i32::from(order.view().wait_time())
                    || (wait_time > 0 && !order.view().is_wait_timetabled())
                {
                    change_timetable(
                        v,
                        order_number,
                        wait_time as u16,
                        ModifyTimetableFlags::MTF_WAIT_TIME,
                        wait_time > 0,
                        s,
                    );
                }
            }

            ModifyTimetableFlags::MTF_TRAVEL_TIME => {
                /* Set time if changing the value or confirming an estimated time as timetabled. */
                if travel_time != i32::from(order.view().travel_time())
                    || (travel_time > 0 && !order.view().is_travel_timetabled())
                {
                    change_timetable(
                        v,
                        order_number,
                        travel_time as u16,
                        ModifyTimetableFlags::MTF_TRAVEL_TIME,
                        travel_time > 0,
                        s,
                    );
                }
            }

            ModifyTimetableFlags::MTF_TRAVEL_SPEED
                if max_speed != i32::from(order.view().max_speed()) =>
            {
                change_timetable(
                    v,
                    order_number,
                    max_speed as u16,
                    ModifyTimetableFlags::MTF_TRAVEL_SPEED,
                    max_speed != i32::from(u16::MAX),
                    s,
                );
            }

            _ => {}
        }

        /* Unbunching data is no longer valid for any vehicle in this shared order group. */
        let mut u = first_shared_in(list, v, s);
        while let Some(shared) = u {
            consist(shared).reset_depot_unbunching();
            u = next_shared(shared, s);
        }
    }
}

/// `CmdBulkChangeTimetable`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdBulkChangeTimetable {
    /// Vehicle with the orders to change.
    pub(crate) veh: VehicleID,
    /// Timetable data to change (`ModifyTimetableFlags`).
    pub(crate) mtf: ModifyTimetableFlags,
    /// The data to modify as specified by `mtf`: 0 to clear times, `UINT16_MAX` to clear the
    /// speed limit.
    pub(crate) data: u16,
}

/// `CmdBulkChangeTimetable`: change timetable data of all orders of a vehicle.
pub(crate) fn cmd_bulk_change_timetable(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdBulkChangeTimetable,
) {
    let CmdBulkChangeTimetable { veh, mtf, data } = args;
    let Some(v) = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle()) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    if mtf >= ModifyTimetableFlags::MTF_END {
        return crate::command_func::cmd_error(cost);
    }
    // `v->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);

    if num_orders(list).0 == 0 {
        return crate::command_func::cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        let mut order_number = VehicleOrderID(0);
        while order_number < num_orders(list) {
            let order = order_in(list, order_number);
            if order.is_some_and(|order| !order.view().is_type(OrderType::OT_IMPLICIT)) {
                command_do(
                    s,
                    DoCommandFlags::from_element(DoCommandFlag::Execute),
                    |out, flags| {
                        cmd_change_timetable(
                            s,
                            out,
                            flags,
                            CmdChangeTimetable {
                                veh: v.index(s.newgrf()),
                                order_number,
                                mtf,
                                data,
                            },
                        );
                    },
                );
            }
            order_number = VehicleOrderID(order_number.0.wrapping_add(1));
        }
    }
}

/// `CmdSetVehicleOnTime`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetVehicleOnTime {
    /// Vehicle with the orders to change.
    pub(crate) veh: VehicleID,
    /// Set to reset the late counter for all vehicles sharing the orders.
    pub(crate) apply_to_group: bool,
}

/// `CmdSetVehicleOnTime`: clear the lateness counter to make the vehicle on time.
pub(crate) fn cmd_set_vehicle_on_time(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetVehicleOnTime,
) {
    let CmdSetVehicleOnTime {
        veh,
        apply_to_group,
    } = args;
    let v = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle());
    // `v->orders` is resolved once, in the lookup's own test.
    let (Some(v), Some(list)) = (v, v.and_then(|v| order_list(v, s))) else {
        return crate::command_func::cmd_error(cost);
    };
    let c = consist(v);

    /* A vehicle can't be late if its timetable hasn't started.
     * If we're setting all vehicles in the group, we handle that below. */
    if !apply_to_group && !c.vehicle_flags().test(VehicleFlag::TIMETABLE_STARTED) {
        return cost.error(STR_ERROR_TIMETABLE_NOT_STARTED.0);
    }

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        if apply_to_group {
            let mut most_late = 0i32;
            let mut u = list.get_first_shared_vehicle(s);
            while let Some(shared) = u {
                u = next_shared(shared, s);
                /* A vehicle can't be late if its timetable hasn't started. */
                if !c.vehicle_flags().test(VehicleFlag::TIMETABLE_STARTED) {
                    continue;
                }

                let sc = consist(shared);
                if sc.lateness_counter() > most_late {
                    most_late = sc.lateness_counter();
                }

                /* Unbunching data is no longer valid. */
                sc.reset_depot_unbunching();
            }
            if most_late > 0 {
                let mut u = list.get_first_shared_vehicle(s);
                while let Some(shared) = u {
                    u = next_shared(shared, s);
                    /* A vehicle can't be late if its timetable hasn't started. */
                    if !c.vehicle_flags().test(VehicleFlag::TIMETABLE_STARTED) {
                        continue;
                    }

                    let sc = consist(shared);
                    sc.set_lateness_counter(sc.lateness_counter().wrapping_sub(most_late));
                    set_window_dirty(
                        WindowClass::WC_VEHICLE_TIMETABLE,
                        WindowNumber::from(shared.index(s.newgrf())),
                    );
                }
            }
        } else {
            c.set_lateness_counter(0);
            /* Unbunching data is no longer valid. */
            c.reset_depot_unbunching();
            set_window_dirty(
                WindowClass::WC_VEHICLE_TIMETABLE,
                WindowNumber::from(v.index(s.newgrf())),
            );
        }
    }
}

/// `VehicleTimetableSorter`: order vehicles based on their timetable. The vehicles will be
/// sorted in order they would reach the first station. True where `a` sorts before `b`.
#[allow(clippy::many_single_char_names, reason = "the original's names")]
fn vehicle_timetable_sorter(a: VehicleRef, b: VehicleRef, s: &Leaves) -> bool {
    let (ac, bc) = (consist(a), consist(b));
    let mut a_order = ac.cur_real_order_index();
    let mut b_order = bc.cur_real_order_index();
    let j = i32::from(b_order.0) - i32::from(a_order.0);

    /* Are we currently at an ordered station (un)loading? */
    let a_load =
        a.current_order().is_type(OrderType::OT_LOADING) && a.current_order().non_stop().any();
    let b_load =
        b.current_order().is_type(OrderType::OT_LOADING) && b.current_order().non_stop().any();

    /* If the current order is not loading at the ordered station, decrease the order index by one since we have
     * not yet arrived at the station (and thus the timetable entry; still in the travelling of the previous one).
     * Since the ?_order variables are unsigned the -1 will flow under and place the vehicles going to order #0 at
     * the begin of the list with vehicles arriving at #0. */
    if !a_load {
        a_order = VehicleOrderID(a_order.0.wrapping_sub(1));
    }
    if !b_load {
        b_order = VehicleOrderID(b_order.0.wrapping_sub(1));
    }

    /* First check the order index that accounted for loading, then just the raw one. */
    let i = i32::from(b_order.0) - i32::from(a_order.0);
    if i != 0 {
        return i < 0;
    }
    if j != 0 {
        return j < 0;
    }

    /* Look at the time we spent in this order; the higher, the closer to its destination. */
    let i = bc
        .current_order_time()
        .wrapping_sub(ac.current_order_time());
    if i != 0 {
        return i < 0;
    }

    /* If all else is equal, use some unique index to sort it the same way. */
    b.unitnumber(s.shared()) < a.unitnumber(s.shared())
}

/// `CmdSetTimetableStart`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetTimetableStart {
    /// Vehicle ID.
    pub(crate) veh_id: VehicleID,
    /// Set to set timetable start for all vehicles sharing this order.
    pub(crate) timetable_all: bool,
    /// The `TimerGameTick::counter` tick when the timetable starts.
    pub(crate) start_tick: u64,
}

/// `CmdSetTimetableStart`: set the start date of the timetable.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the original's conversion to `TickCounter`"
)]
pub(crate) fn cmd_set_timetable_start(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetTimetableStart,
) {
    let CmdSetTimetableStart {
        veh_id,
        timetable_all,
        start_tick,
    } = args;
    let v = vehicle_get_if_valid(veh_id, s).filter(|v| v.is_primary_vehicle());
    // `v->orders` is resolved once, in the lookup's own test.
    let (Some(v), Some(list)) = (v, v.and_then(|v| order_list(v, s))) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    let total_duration = list.get_timetable_total_duration();

    let start_date = get_date_from_start_tick(start_tick, s).0;
    let date = crate::timer_game_common::economy_date().0;

    /* Don't let a timetable start at an invalid date. */
    if !(0..=crate::timer_game_common::MAX_DATE).contains(&start_date) {
        return crate::command_func::cmd_error(cost);
    }

    /* Don't let a timetable start more than 15 years into the future... */
    if start_date.wrapping_sub(date)
        > crate::timer_game_common::date_at_start_of_year(
            crate::timetable::MAX_TIMETABLE_START_YEARS,
        )
    {
        return crate::command_func::cmd_error(cost);
    }
    /* ...or 1 year in the past. */
    if date.wrapping_sub(start_date) > DAYS_IN_LEAP_YEAR {
        return crate::command_func::cmd_error(cost);
    }

    /* If trying to distribute start dates over a shared order group, we need to know the total duration. */
    if timetable_all && !list.is_complete_timetable() {
        return cost.error(STR_ERROR_TIMETABLE_INCOMPLETE.0);
    }

    /* Don't allow invalid start dates for other vehicles in the shared order group. */
    if timetable_all
        && start_date.wrapping_add(total_duration / DAY_TICKS) > crate::timer_game_common::MAX_DATE
    {
        return crate::command_func::cmd_error(cost);
    }

    if flags.test(DoCommandFlag::Execute) {
        let mut vehs: Vec<VehicleRef> = Vec::new();

        if timetable_all {
            let mut w = list.get_first_shared_vehicle(s);
            while let Some(shared) = w {
                vehs.push(shared);
                w = next_shared(shared, s);
            }
        } else {
            vehs.push(v);
        }

        // `(uint)vehs.size()` stored in `int`.
        let num_vehs = vehs.len() as i32;

        if num_vehs >= 2 {
            // The sorter is a strict total order over the shared vehicles, whose unit numbers
            // differ, so every correct sort gives `std::sort`'s result.
            vehs.sort_unstable_by(|a, b| {
                if vehicle_timetable_sorter(*a, *b, s) {
                    std::cmp::Ordering::Less
                } else if vehicle_timetable_sorter(*b, *a, s) {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            });
        }

        let mut idx = 0i32;

        for w in vehs {
            let c = consist(w);
            c.set_lateness_counter(0);
            c.set_vehicle_flags(c.vehicle_flags().reset(VehicleFlag::TIMETABLE_STARTED));
            /* Do multiplication, then division to reduce rounding errors. */
            // `TickCounter + int`: the `int` quotient converts to `uint64_t`.
            c.set_timetable_start(
                start_tick.wrapping_add((idx.wrapping_mul(total_duration) / num_vehs) as u64),
            );

            /* Unbunching data is no longer valid. */
            consist(v).reset_depot_unbunching();

            set_window_dirty(
                WindowClass::WC_VEHICLE_TIMETABLE,
                WindowNumber::from(w.index(s.newgrf())),
            );
            idx = idx.wrapping_add(1);
        }
    }
}

/// `CmdAutofillTimetable`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdAutofillTimetable {
    /// Vehicle index.
    pub(crate) veh: VehicleID,
    /// Enable or disable autofill.
    pub(crate) autofill: bool,
    /// Set to preserve waiting times in non-destructive mode.
    pub(crate) preserve_wait_time: bool,
}

/// `CmdAutofillTimetable`: start or stop filling the timetable automatically from the time
/// the vehicle actually takes to complete it. When starting to autofill the current times are
/// cleared and the timetable will start again from scratch.
pub(crate) fn cmd_autofill_timetable(
    s: &Leaves,
    cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdAutofillTimetable,
) {
    let CmdAutofillTimetable {
        veh,
        autofill,
        preserve_wait_time,
    } = args;
    let v = vehicle_get_if_valid(veh, s).filter(|v| v.is_primary_vehicle());
    // `v->orders` is resolved once, in the lookup's own test.
    let (Some(v), Some(list)) = (v, v.and_then(|v| order_list(v, s))) else {
        return crate::command_func::cmd_error(cost);
    };

    cost.ownership(s.commands(), v.owner().0, 0);
    if !cost.ok() {
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        let c = consist(v);
        if autofill {
            /* Start autofilling the timetable, which clears the
             * "timetable has started" bit. Times are not cleared anymore, but are
             * overwritten when the order is reached now. */
            c.set_vehicle_flags(c.vehicle_flags().set(VehicleFlag::AUTOFILL_TIMETABLE));
            c.set_vehicle_flags(c.vehicle_flags().reset(VehicleFlag::TIMETABLE_STARTED));

            /* Overwrite waiting times only if they got longer */
            if preserve_wait_time {
                c.set_vehicle_flags(
                    c.vehicle_flags()
                        .set(VehicleFlag::AUTOFILL_PRESERVE_WAIT_TIME),
                );
            }

            c.set_timetable_start(0);
            c.set_lateness_counter(0);
        } else {
            c.set_vehicle_flags(c.vehicle_flags().reset(VehicleFlag::AUTOFILL_TIMETABLE));
            c.set_vehicle_flags(
                c.vehicle_flags()
                    .reset(VehicleFlag::AUTOFILL_PRESERVE_WAIT_TIME),
            );
        }

        let mut v2 = list.get_first_shared_vehicle(s);
        while let Some(shared) = v2 {
            if shared != v {
                let c2 = consist(shared);
                /* Stop autofilling; only one vehicle at a time can perform autofill */
                c2.set_vehicle_flags(c2.vehicle_flags().reset(VehicleFlag::AUTOFILL_TIMETABLE));
                c2.set_vehicle_flags(
                    c2.vehicle_flags()
                        .reset(VehicleFlag::AUTOFILL_PRESERVE_WAIT_TIME),
                );
            }
            set_window_dirty(
                WindowClass::WC_VEHICLE_TIMETABLE,
                WindowNumber::from(shared.index(s.newgrf())),
            );
            v2 = next_shared(shared, s);
        }
    }
}

/// `UpdateVehicleTimetable`: update the timetable for the vehicle. `travelling` says whether
/// we just travelled or waited at a station.
#[allow(clippy::too_many_lines, reason = "one function per original body")]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the original's `Ticks` store"
)]
pub(crate) fn update_vehicle_timetable(v: VehicleRef, travelling: bool, s: &Leaves) {
    let c = consist(v);
    let time_taken = c.current_order_time();
    // `v->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);

    c.set_current_order_time(0);

    if v.current_order().is_type(OrderType::OT_IMPLICIT) {
        return; // no timetabling of auto orders
    }

    if c.cur_real_order_index() >= num_orders(list) {
        return;
    }
    assert_with(|| order_in(list, c.cur_real_order_index()).is_some());
    let real_current_order = deref_list(list, v, s).order(usize::from(c.cur_real_order_index().0));

    let mut first_manual_order = VehicleOrderID(0);
    if let Some(list) = list {
        for index in 0..list.len() {
            if !list.order(index).view().is_type(OrderType::OT_IMPLICIT) {
                break;
            }
            first_manual_order = VehicleOrderID(first_manual_order.0.wrapping_add(1));
        }
    }

    let mut just_started = false;

    /* This vehicle is arriving at the first destination in the timetable. */
    if c.cur_real_order_index() == first_manual_order && travelling {
        /* If the start date hasn't been set, or it was set automatically when
         * the vehicle last arrived at the first destination, update it to the
         * current time. Otherwise set the late counter appropriately to when
         * the vehicle should have arrived. */
        just_started = !c.vehicle_flags().test(VehicleFlag::TIMETABLE_STARTED);

        if c.timetable_start() != 0 {
            // `TickCounter - TickCounter` stored in `Ticks`.
            c.set_lateness_counter(
                (s.shared().tick_counter)().wrapping_sub(c.timetable_start()) as i32,
            );
            c.set_timetable_start(0);
        }

        c.set_vehicle_flags(c.vehicle_flags().set(VehicleFlag::TIMETABLE_STARTED));
        set_window_dirty(
            WindowClass::WC_VEHICLE_TIMETABLE,
            WindowNumber::from(v.index(s.newgrf())),
        );
    }

    if !c.vehicle_flags().test(VehicleFlag::TIMETABLE_STARTED) {
        return;
    }

    let autofilling = c.vehicle_flags().test(VehicleFlag::AUTOFILL_TIMETABLE);
    let remeasure_wait_time = !real_current_order.view().is_wait_timetabled()
        || (autofilling
            && !c
                .vehicle_flags()
                .test(VehicleFlag::AUTOFILL_PRESERVE_WAIT_TIME));

    if travelling && remeasure_wait_time {
        /* We just finished travelling and want to remeasure the loading time,
         * so do not apply any restrictions for the loading to finish. */
        current_order(v).set_wait_time(0);
    }

    if just_started {
        return;
    }

    /* Before modifying waiting times, check whether we want to preserve bigger ones. */
    if !real_current_order.view().is_type(OrderType::OT_CONDITIONAL)
        && (travelling
            || time_taken > i32::from(real_current_order.view().wait_time())
            || remeasure_wait_time)
    {
        /* Round up to the smallest unit of time commonly shown in the GUI (seconds) to avoid confusion.
         * Players timetabling in Ticks can adjust later.
         * For trains/aircraft multiple movement cycles are done in one
         * tick. This makes it possible to leave the station and process
         * e.g. a depot order in the same tick, causing it to not fill
         * the timetable entry like is done for road vehicles/ships.
         * Thus always make sure at least one tick is used between the
         * processing of different orders when filling the timetable. */
        // `CeilDiv(std::max(time_taken, 1), TICKS_PER_SECOND) * TICKS_PER_SECOND` as `uint`,
        // passed to `ChangeTimetable`'s `uint16_t`.
        let time_to_set = (crate::math::ceil_div(time_taken.max(1) as u32, TICKS_PER_SECOND as u32)
            .wrapping_mul(TICKS_PER_SECOND as u32)) as u16;

        if travelling && (autofilling || !real_current_order.view().is_travel_timetabled()) {
            change_timetable(
                v,
                c.cur_real_order_index(),
                time_to_set,
                ModifyTimetableFlags::MTF_TRAVEL_TIME,
                autofilling,
                s,
            );
        } else if !travelling && (autofilling || !real_current_order.view().is_wait_timetabled()) {
            change_timetable(
                v,
                c.cur_real_order_index(),
                time_to_set,
                ModifyTimetableFlags::MTF_WAIT_TIME,
                autofilling,
                s,
            );
        }
    }

    if c.cur_real_order_index() == first_manual_order && travelling {
        /* If we just started we would have returned earlier and have not reached
         * this code. So obviously, we have completed our round: So turn autofill
         * off again. */
        c.set_vehicle_flags(c.vehicle_flags().reset(VehicleFlag::AUTOFILL_TIMETABLE));
        c.set_vehicle_flags(
            c.vehicle_flags()
                .reset(VehicleFlag::AUTOFILL_PRESERVE_WAIT_TIME),
        );
    }

    if autofilling {
        return;
    }

    let timetabled = i32::from(if travelling {
        real_current_order.view().timetabled_travel()
    } else {
        real_current_order.view().timetabled_wait()
    });

    /* Vehicles will wait at stations if they arrive early even if they are not
     * timetabled to wait there, so make sure the lateness counter is updated
     * when this happens. */
    if timetabled == 0 && (travelling || c.lateness_counter() >= 0) {
        return;
    }

    c.set_lateness_counter(
        c.lateness_counter()
            .wrapping_sub(timetabled.wrapping_sub(time_taken)),
    );

    /* When we are more late than this timetabled bit takes we (somewhat expensively)
     * check how many ticks the (fully filled) timetable has. If a timetable cycle is
     * shorter than the amount of ticks we are late we reduce the lateness by the
     * length of a full cycle till lateness is less than the length of a timetable
     * cycle. When the timetable isn't fully filled the cycle will be Ticks::INVALID_TICKS. */
    if c.lateness_counter() > timetabled {
        let cycle = deref_list(list, v, s).get_timetable_total_duration();
        if cycle != INVALID_TICKS && c.lateness_counter() > cycle {
            c.set_lateness_counter(c.lateness_counter() % cycle);
        }
    }

    let mut u = first_shared_in(list, v, s);
    while let Some(shared) = u {
        set_window_dirty(
            WindowClass::WC_VEHICLE_TIMETABLE,
            WindowNumber::from(shared.index(s.newgrf())),
        );
        u = next_shared(shared, s);
    }
}

/// `OrderBackup::OrderBackup(v, user)`, the constructor of `ob`: create an order backup for
/// the given vehicle. `user` is the user that is requesting the backup.
#[allow(unsafe_code)]
fn order_backup_construct(ob: OrderBackupRef, v: VehicleRef, user: u32, s: &Leaves) {
    ob.set_user(user);
    ob.set_tile(v.tile());
    ob.set_group(v.group_id(s.maintenance_groups()));

    copy_consist_properties_to_backup(ob, v, s);
    // `v->orders` is resolved once: nothing below replaces the vehicle's list.
    let list = order_list(v, s);

    /* If we have shared orders, store the vehicle we share the order with. */
    if list.is_some_and(OrderListRef::is_shared) {
        let first = first_shared_in(list, v, s);
        let clone = if first == Some(v) {
            next_shared(v, s)
        } else {
            first
        };
        ob.set_clone(clone.map_or(ptr::null_mut(), VehicleRef::shell));
    } else {
        /* Else copy the orders */
        let span = list;
        // SAFETY: The backup's live vector and the vehicle's distinct live orders.
        unsafe {
            (*ob.vector).assign(
                span.map_or(ptr::null(), |list| list.data().cast_const()),
                span.map_or(0, OrderListRef::len),
            );
        }
    }
}

/// `ob->CopyConsistPropertiesFrom(v)`: copy properties of other `BaseConsist`.
#[allow(unsafe_code)]
fn copy_consist_properties_to_backup(ob: OrderBackupRef, v: VehicleRef, s: &Leaves) {
    // SAFETY: The live backup and vehicle shells.
    unsafe { (s.backup_set_name)(ob.shell, v.shell()) };
    ob.consist.copy_consist_properties_from(consist(v), || {
        // SAFETY: As above.
        unsafe { (s.backup_set_service_interval)(ob.shell, v.get_service_interval(s.newgrf())) };
    });
}

/// `v->CopyConsistPropertiesFrom(ob)`: copy properties of other `BaseConsist`.
#[allow(unsafe_code)]
fn copy_consist_properties_from_backup(v: VehicleRef, ob: OrderBackupRef, s: &Leaves) {
    // SAFETY: The live vehicle and backup shells.
    unsafe { (s.vehicle_set_name)(v.shell(), ob.shell) };
    consist(v).copy_consist_properties_from(ob.consist, || {
        // SAFETY: As above.
        unsafe {
            (s.vehicle_set_service_interval)(v.shell(), (s.backup_service_interval)(ob.shell));
        }
    });
}

/// `OrderBackup::DoRestore`: restore the data of this order to the given vehicle.
#[allow(unsafe_code)]
fn order_backup_do_restore(ob: OrderBackupRef, v: VehicleRef, s: &Leaves) {
    /* If we had shared orders, recover that */
    if !ob.clone_shell().is_null() {
        let clone = record(ob.clone_shell(), s);
        command_do(
            s,
            DoCommandFlags::from_element(DoCommandFlag::Execute),
            |out, flags| {
                cmd_clone_order(
                    s,
                    out,
                    flags,
                    CmdCloneOrder {
                        action: CloneOptions::CO_SHARE,
                        veh_dst: v.index(s.newgrf()),
                        veh_src: clone.index(s.newgrf()),
                    },
                );
            },
        );
    } else if ob.len() != 0 && (s.list_can_allocate_item)() != 0 {
        let list = new_order_list(
            v,
            // SAFETY: The new list's live vector and the backup's distinct live vector.
            |list| unsafe { (*list.vector).take(ob.vector) },
            s,
        );
        set_order_list(v, Some(list));
        /* Make sure buoys/oil rigs are updated in the station list. */
        invalidate_window_classes_data(WindowClass::WC_STATION_LIST, 0, false);
    }

    /* Remove backed up name if it's no longer unique. */
    // SAFETY: The live backup shell, whose name stays unchanged through the call, and the
    // process-lifetime transaction table.
    if !unsafe {
        crate::vehicle_transactions::unique_name(
            &*s.vehicle_transactions,
            (s.backup_name)(ob.shell),
        )
    } {
        // SAFETY: As above.
        unsafe { (s.backup_clear_name)(ob.shell) };
    }

    copy_consist_properties_from_backup(v, ob, s);

    /* Make sure orders are in range */
    update_real_order_index(v, s);
    let c = consist(v);
    if c.cur_implicit_order_index() >= get_num_orders(v, s) {
        c.set_cur_implicit_order_index(c.cur_real_order_index());
    }

    /* Restore vehicle group */
    let group = ob.group();
    let groups = s.maintenance().group;
    let id = v.index(s.newgrf());
    command_do(
        s,
        DoCommandFlags::from_element(DoCommandFlag::Execute),
        |out, flags| {
            let mut new_group = 0;
            // SAFETY: The process-lifetime group table; the command reads no list context.
            let error = unsafe {
                crate::fleet::add_vehicle_group(
                    &*groups,
                    u32::from(flags.0),
                    group.0,
                    id.0,
                    false,
                    ptr::null_mut(),
                    false,
                    &raw mut new_group,
                )
            };
            if error == u32::MAX {
                out.zero();
            } else {
                out.error(error);
            }
        },
    );
}

/// `OrderBackup::Backup`: create an order backup for the given vehicle. `user` is the user
/// that is requesting the backup. Will automatically remove any previous backups of this
/// user.
#[allow(unsafe_code)]
pub(crate) fn order_backup_backup(v: VehicleRef, user: u32, s: &Leaves) {
    /* Don't use reset as that broadcasts over the network to reset the variable,
     * which is what we are doing at the moment. */
    for_each_backup(s, |ob| {
        if ob.user() == user {
            ob.delete(s);
        }
    });
    if (s.backup_can_allocate_item)() != 0 {
        // SAFETY: The live vehicle shell; the constructor runs `order_backup_construct`.
        unsafe { (s.create_backup)(v.shell(), user) };
    }
}

/// `OrderBackup::Restore`: restore the data of this order to the given vehicle. `user` is the
/// user that built the vehicle, thus wants to restore. After restoration the backup will
/// automatically be removed.
pub(crate) fn order_backup_restore(v: VehicleRef, user: u32, s: &Leaves) {
    for_each_backup(s, |ob| {
        if v.tile() != ob.tile() || ob.user() != user {
            return;
        }

        order_backup_do_restore(ob, v, s);
        ob.delete(s);
    });
}

/// `OrderBackup::ResetOfUser`: reset an `OrderBackup` given a tile and user. Must not be
/// used from the GUI!
pub(crate) fn order_backup_reset_of_user(tile: TileIndex, user: u32, s: &Leaves) {
    for_each_backup(s, |ob| {
        if ob.user() == user && (ob.tile() == tile || tile == TileIndex::INVALID) {
            ob.delete(s);
        }
    });
}

/// `CmdClearOrderBackup`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdClearOrderBackup {
    /// Tile related to the to-be-cleared `OrderBackup`.
    pub(crate) tile: TileIndex,
    /// User that had the `OrderBackup`.
    pub(crate) user_id: ClientID,
}

/// `CmdClearOrderBackup`: clear an `OrderBackup`.
pub(crate) fn cmd_clear_order_backup(
    s: &Leaves,
    _cost: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdClearOrderBackup,
) {
    let CmdClearOrderBackup { tile, user_id } = args;
    /* No need to check anything. If the tile or user don't exist we just ignore it. */
    if flags.test(DoCommandFlag::Execute) {
        order_backup_reset_of_user(
            if tile == TileIndex(0) {
                TileIndex::INVALID
            } else {
                tile
            },
            user_id.0,
            s,
        );
    }
}

/// `OrderBackup::ResetUser`: reset an user's `OrderBackup` if needed. `_network_server`;
/// must not be used from a command.
#[allow(unsafe_code)]
pub(crate) fn order_backup_reset_user(user: u32, s: &'static Leaves) {
    let p = (s.shared().command_post)();
    // SAFETY: The command table returns the process-lifetime company table.
    let company = unsafe { &*((p.command)().company)() };
    assert_with(|| (company.network_server)());

    let mut cursor = 0;
    while let Some(ob) = next_backup(s, &mut cursor) {
        /* If it's not a backup of us, ignore it. */
        if ob.user() != user {
            continue;
        }

        crate::command_post::clear_order_backup(s, TileIndex(0), ClientID(user));
        return;
    }
}

/// `OrderBackup::Reset`: reset the `OrderBackup`s from GUI/game logic. `t` is the tile of the
/// order backup; `from_gui` says whether the call came from the GUI, i.e. whether it must be
/// synced over the network.
#[allow(unsafe_code)]
pub(crate) fn order_backup_reset(t: TileIndex, from_gui: bool, s: &Leaves) {
    let p = (s.shared().command_post)();
    // SAFETY: The command table returns the process-lifetime company table.
    let company = unsafe { &*((p.command)().company)() };
    /* The user has CLIENT_ID_SERVER as default when network play is not active,
     * but compiled it. A network client has its own variable for the unique
     * client/user identifier. Finally if networking isn't compiled in the
     * default is just plain and simple: 0. */
    let user = if (company.finance.networking)() && !(company.network_server)() {
        // SAFETY: The session table names the process-lifetime network frame table.
        let network = unsafe { &*(p.session)().network };
        crate::network_client::own_client_id(network.client).0
    } else {
        CLIENT_ID_SERVER
    };

    for_each_backup(s, |ob| {
        /* If this is a GUI action, and it's not a backup of us, ignore it. */
        if from_gui && ob.user() != user {
            return;
        }
        /* If it's not for our chosen tile either, ignore it. */
        if t != TileIndex::INVALID && t != ob.tile() {
            return;
        }

        if from_gui {
            /* We need to circumvent the "prevention" from this command being executed
             * while the game is paused, so use the internal method. Nor do we want
             * this command to get its cost estimated when shift is pressed. */
            (s.clear_order_backup_unsafe)(ob.tile().0, user);
        } else {
            /* The command came from the game logic, i.e. the clearing of a tile.
             * In that case we have no need to actually sync this, just do it. */
            ob.delete(s);
        }
    });
}

/// `OrderBackup::ClearGroup`: clear the group of all backups having this group ID.
pub(crate) fn order_backup_clear_group(group: GroupID, s: &Leaves) {
    for_each_backup(s, |ob| {
        if ob.group() == group {
            ob.set_group(GroupID::DEFAULT_GROUP);
        }
    });
}

/// `OrderBackup::ClearVehicle`: clear/update the (clone) vehicle from an order backup. If it
/// is not possible to set another vehicle as clone "example", then this backed up order will
/// be removed.
pub(crate) fn order_backup_clear_vehicle(v: VehicleRef, s: &Leaves) {
    for_each_backup(s, |ob| {
        if ob.clone_shell() == v.shell() {
            /* Get another item in the shared list. */
            let first = first_shared(v, s);
            let clone = if first == Some(v) {
                next_shared(v, s)
            } else {
                first
            };
            ob.set_clone(clone.map_or(ptr::null_mut(), VehicleRef::shell));
            /* But if that isn't there, remove it. */
            if ob.clone_shell().is_null() {
                ob.delete(s);
            }
        }
    });
}

/// `OrderBackup::RemoveOrder`: removes an order from all vehicles. Triggers when, say, a
/// station is removed. `type` is the type of the order (`OT_GOTO_[STATION|DEPOT|WAYPOINT]`)
/// and `destination` a `StationID`, `DepotID` or `WaypointID`. `hangar` is only used for
/// airports in the destination: when false, remove airport and hangar orders; when true,
/// remove either airport or hangar order.
fn order_backup_remove_order(
    r#type: OrderType,
    destination: DestinationID,
    hangar: bool,
    s: &Leaves,
) {
    let airports = crate::newgrf_airport::runtime();
    for_each_backup(s, |ob| {
        for index in 0..ob.len() {
            let order = ob.order(index);
            let mut ot = order.kind();
            if ot == OrderType::OT_GOTO_DEPOT
                && order
                    .depot_action_flags()
                    .test(OrderDepotActionFlag::NearestDepot)
            {
                continue;
            }
            if ot == OrderType::OT_GOTO_DEPOT
                && hangar
                && !crate::station_map::is_hangar_tile(ob.tile(), airports)
            {
                continue; // Not an aircraft? Can't have a hangar order.
            }
            if ot == OrderType::OT_IMPLICIT
                || (crate::station_map::is_hangar_tile(ob.tile(), airports)
                    && ot == OrderType::OT_GOTO_DEPOT
                    && !hangar)
            {
                ot = OrderType::OT_GOTO_STATION;
            }
            if ot == r#type && order.destination() == destination {
                /* Remove the order backup! If a station/depot gets removed, we can't/shouldn't restore those broken orders. */
                ob.delete(s);
                break;
            }
        }
    });
}

impl Leaves {
    /// The group table, for `Vehicle::group_id`.
    #[allow(unsafe_code)]
    fn maintenance_groups(&self) -> &'static crate::fleet::GroupServices {
        // SAFETY: The maintenance table names the process-lifetime group table.
        unsafe { &*self.maintenance().group }
    }
}

// The export shims of the C++ facades. Each converts the base integers C++ passes and calls
// the body; a vehicle crosses as its live record, an order as the address of a live order,
// a list or backup as its live shell, and a command's result as its live native
// `CommandCost`.

/// The order a facade passes by address.
#[allow(unsafe_code)]
fn order_arg<'a>(p: *const Order) -> OrderView<'a> {
    // SAFETY: Each facade passes the address of a live order.
    unsafe { OrderView::of(p) }
}

/// The order a facade writes by address.
#[allow(unsafe_code)]
fn order_mut(p: *mut Order) -> OrderRef {
    // SAFETY: Each facade passes the address of a live, writable order.
    unsafe { OrderRef::of(p) }
}

/// `Order::Free`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_free(p: *mut Order) {
    order_mut(p).free();
}
/// `Order::MakeGoToStation`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_station(p: *mut Order, destination: u16) {
    order_mut(p).make_go_to_station(StationID(destination));
}
/// `Order::MakeGoToDepot`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_depot(
    p: *mut Order,
    destination: u16,
    order: u8,
    non_stop_type: u8,
    action: u8,
    cargo: u8,
) {
    order_mut(p).make_go_to_depot(
        DestinationID(destination),
        OrderDepotTypeFlags(order),
        OrderNonStopFlags(non_stop_type),
        OrderDepotActionFlags(action),
        CargoType(cargo),
    );
}
/// `Order::MakeGoToWaypoint`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_waypoint(p: *mut Order, destination: u16) {
    order_mut(p).make_go_to_waypoint(StationID(destination));
}
/// `Order::MakeLoading`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_loading(p: *mut Order, ordered: u8) {
    order_mut(p).make_loading(ordered != 0);
}
/// `Order::MakeDummy`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_dummy(p: *mut Order) {
    order_mut(p).make_dummy();
}
/// `Order::MakeConditional`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_conditional(p: *mut Order, order: u8) {
    order_mut(p).make_conditional(VehicleOrderID(order));
}
/// `Order::MakeImplicit`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_implicit(p: *mut Order, destination: u16) {
    order_mut(p).make_implicit(StationID(destination));
}
/// `Order::SetRefit`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_refit(p: *mut Order, cargo: u8) {
    order_mut(p).set_refit(CargoType(cargo));
}
/// `Order::Equals`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_equals(p: *const Order, other: *const Order) -> u8 {
    u8::from(order_arg(p).equals(order_arg(other)))
}
/// `Order::AssignOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_assign(p: *mut Order, other: *const Order) {
    order_mut(p).assign_order(order_arg(other));
}
/// `Order::MapOldOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_convert(p: *const Order) -> u16 {
    order_arg(p).map_old_order()
}
/// `Order::GetLocation`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_location(
    p: *const Order,
    v: VehicleRef,
    airport: u8,
    s: &'static Leaves,
) -> u32 {
    get_location(order_arg(p), v, airport != 0, s).0
}
/// `GetOrderDistance`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_order_distance(
    prev: u8,
    cur: u8,
    v: VehicleRef,
    conditional_depth: i32,
    s: &'static Leaves,
) -> u32 {
    get_order_distance(
        VehicleOrderID(prev),
        VehicleOrderID(cur),
        v,
        order_list(v, s),
        conditional_depth,
        s,
    )
}

/// `OrderList::Initialize`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_list_init(list: *mut c_void, v: VehicleRef, s: &'static Leaves) {
    OrderListRef::of(list, s).initialize(v, s);
}
/// `OrderList::RecalculateTimetableDuration`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_list_recalculate(list: *mut c_void, s: &'static Leaves) {
    OrderListRef::of(list, s).recalculate_timetable_duration();
}
/// `OrderList::FreeChain`, which deletes the list unless `keep_orderlist` is set.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_list_free(
    list: *mut c_void,
    keep_orderlist: u8,
    s: &'static Leaves,
) {
    OrderListRef::of(list, s).free_chain(keep_orderlist != 0, s);
}
/// `OrderList::GetNextStoppingStation`, appending to the C++ vector `next_station`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_list_stopping(
    list: *mut c_void,
    next_station: *mut c_void,
    v: VehicleRef,
    first: u8,
    hops: u32,
    s: &'static Leaves,
) {
    OrderListRef::of(list, s).get_next_stopping_station(
        // SAFETY: The facade's live output vector.
        &mut |station| unsafe { (s.append_station)(next_station, station.0) },
        v,
        VehicleOrderID(first),
        hops,
    );
}
/// `OrderList::InsertOrderAt`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_list_insert(
    list: *mut c_void,
    order: *const Order,
    index: u8,
    s: &'static Leaves,
) {
    OrderListRef::of(list, s).insert_order_at(order_arg(order).copied(), VehicleOrderID(index), s);
}
/// `OrderList::IsCompleteTimetable`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_list_complete(list: *mut c_void, s: &'static Leaves) -> u8 {
    u8::from(OrderListRef::of(list, s).is_complete_timetable())
}

/// `Vehicle::AddToShared`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_add_shared(
    v: VehicleRef,
    shared_chain: VehicleRef,
    s: &'static Leaves,
) {
    add_to_shared(v, shared_chain, s);
}
/// `Vehicle::DeleteUnreachedImplicitOrders`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_delete_implicit(v: VehicleRef, s: &'static Leaves) {
    delete_unreached_implicit_orders(v, s);
}
/// `Vehicle::LeaveUnbunchingDepot`, for the orders console's direct call of the Rust body.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_leave_unbunching(v: *mut c_void, s: &'static Leaves) {
    leave_unbunching_depot(record(v, s), s);
}
/// `Vehicle::IsWaitingForUnbunching`, after the facade's `assert(this->IsInDepot())`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_wait_unbunching(v: VehicleRef, s: &'static Leaves) -> u8 {
    u8::from(is_waiting_for_unbunching(v, s))
}
/// `Vehicle::IncrementImplicitOrderIndex`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_increment_implicit(v: VehicleRef, s: &'static Leaves) {
    increment_implicit_order_index(v, s);
}
/// `Vehicle::IncrementRealOrderIndex`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_increment_real(v: VehicleRef, s: &'static Leaves) {
    increment_real_order_index(v, s);
}
/// `Vehicle::UpdateRealOrderIndex`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_update_real(v: VehicleRef, s: &'static Leaves) {
    update_real_order_index(v, s);
}
/// `InsertOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_insert_order(
    v: VehicleRef,
    new_o: *const Order,
    sel_ord: u8,
    s: &'static Leaves,
) {
    insert_order(v, order_arg(new_o).copied(), VehicleOrderID(sel_ord), s);
}
/// `DeleteVehicleOrders`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_delete_vehicle_orders(
    v: VehicleRef,
    keep_orderlist: u8,
    reset_order_indices: u8,
    s: &'static Leaves,
) {
    delete_vehicle_orders(v, keep_orderlist != 0, reset_order_indices != 0, s);
}
/// `ProcessOrders`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_process_orders(v: VehicleRef, s: &'static Leaves) -> u8 {
    u8::from(process_orders(v, s))
}
/// `RemoveOrderFromAllVehicles`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_remove_destination(
    r#type: u8,
    destination: u16,
    hangar: u8,
    s: &'static Leaves,
) {
    remove_order_from_all_vehicles(
        OrderType(r#type),
        DestinationID(destination),
        hangar != 0,
        s,
    );
}

/// `CmdInsertOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_insert(
    flags: u16,
    veh: u32,
    sel_ord: u8,
    new_order: *const Order,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_insert_order(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdInsertOrder {
            veh: VehicleID(veh),
            sel_ord: VehicleOrderID(sel_ord),
            new_order: order_arg(new_order),
        },
    );
}
/// `CmdDeleteOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_delete(
    flags: u16,
    veh_id: u32,
    sel_ord: u8,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_delete_order(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdDeleteOrder {
            veh_id: VehicleID(veh_id),
            sel_ord: VehicleOrderID(sel_ord),
        },
    );
}
/// `CmdSkipToOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_skip(
    flags: u16,
    veh_id: u32,
    sel_ord: u8,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_skip_to_order(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdSkipToOrder {
            veh_id: VehicleID(veh_id),
            sel_ord: VehicleOrderID(sel_ord),
        },
    );
}
/// `CmdMoveOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_move(
    flags: u16,
    veh: u32,
    moving_order: u8,
    target_order: u8,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_move_order(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdMoveOrder {
            veh: VehicleID(veh),
            moving_order: VehicleOrderID(moving_order),
            target_order: VehicleOrderID(target_order),
        },
    );
}
/// `CmdModifyOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_modify(
    flags: u16,
    veh: u32,
    sel_ord: u8,
    mof: u8,
    data: u16,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_modify_order(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdModifyOrder {
            veh: VehicleID(veh),
            sel_ord: VehicleOrderID(sel_ord),
            mof: ModifyOrderFlags(mof),
            data,
        },
    );
}
/// `CmdCloneOrder`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_clone(
    flags: u16,
    action: u8,
    veh_dst: u32,
    veh_src: u32,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_clone_order(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdCloneOrder {
            action: CloneOptions(action),
            veh_dst: VehicleID(veh_dst),
            veh_src: VehicleID(veh_src),
        },
    );
}
/// `CmdOrderRefit`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_refit(
    flags: u16,
    veh: u32,
    order_number: u8,
    cargo: u8,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_order_refit(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdOrderRefit {
            veh: VehicleID(veh),
            order_number: VehicleOrderID(order_number),
            cargo: CargoType(cargo),
        },
    );
}
/// `CmdClearOrderBackup`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_command_clear_backup(
    flags: u16,
    tile: u32,
    user_id: u32,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_clear_order_backup(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdClearOrderBackup {
            tile: TileIndex(tile),
            user_id: ClientID(user_id),
        },
    );
}
/// `CmdChangeTimetable`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_timetable_change(
    flags: u16,
    veh: u32,
    order_number: u8,
    mtf: u8,
    data: u16,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_change_timetable(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdChangeTimetable {
            veh: VehicleID(veh),
            order_number: VehicleOrderID(order_number),
            mtf: ModifyTimetableFlags(mtf),
            data,
        },
    );
}
/// `CmdBulkChangeTimetable`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_timetable_bulk(
    flags: u16,
    veh: u32,
    mtf: u8,
    data: u16,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_bulk_change_timetable(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdBulkChangeTimetable {
            veh: VehicleID(veh),
            mtf: ModifyTimetableFlags(mtf),
            data,
        },
    );
}
/// `CmdSetVehicleOnTime`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_timetable_on_time(
    flags: u16,
    veh: u32,
    apply_to_group: u8,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_set_vehicle_on_time(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdSetVehicleOnTime {
            veh: VehicleID(veh),
            apply_to_group: apply_to_group != 0,
        },
    );
}
/// `CmdSetTimetableStart`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_timetable_start(
    flags: u16,
    veh_id: u32,
    timetable_all: u8,
    start_tick: u64,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_set_timetable_start(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdSetTimetableStart {
            veh_id: VehicleID(veh_id),
            timetable_all: timetable_all != 0,
            start_tick,
        },
    );
}
/// `CmdAutofillTimetable`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_timetable_autofill(
    flags: u16,
    veh: u32,
    autofill: u8,
    preserve_wait_time: u8,
    out: *mut CommandCost,
    s: &'static Leaves,
) {
    cmd_autofill_timetable(
        s,
        s.cost(out),
        DoCommandFlags(flags),
        CmdAutofillTimetable {
            veh: VehicleID(veh),
            autofill: autofill != 0,
            preserve_wait_time: preserve_wait_time != 0,
        },
    );
}

/// The `OrderBackup(v, user)` constructor's body, run on the new backup `ob`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_capture_backup(
    ob: *mut c_void,
    v: VehicleRef,
    user: u32,
    s: &'static Leaves,
) {
    order_backup_construct(OrderBackupRef::of(ob, s), v, user, s);
}
/// `OrderBackup::Backup`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_backup_create(v: VehicleRef, user: u32, s: &'static Leaves) {
    order_backup_backup(v, user, s);
}
/// `OrderBackup::Restore`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_backup_restore(v: VehicleRef, user: u32, s: &'static Leaves) {
    order_backup_restore(v, user, s);
}
/// `OrderBackup::ResetOfUser`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_backup_clear_user(tile: u32, user: u32, s: &'static Leaves) {
    order_backup_reset_of_user(TileIndex(tile), user, s);
}
/// `OrderBackup::ResetUser`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_backup_reset(user: u32, s: &'static Leaves) {
    order_backup_reset_user(user, s);
}
/// `OrderBackup::Reset`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_backup_tile_clear(tile: u32, from_gui: u8, s: &'static Leaves) {
    order_backup_reset(TileIndex(tile), from_gui != 0, s);
}
/// `OrderBackup::ClearGroup`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_backup_group_clear(group: u16, s: &'static Leaves) {
    order_backup_clear_group(GroupID(group), s);
}
/// `OrderBackup::ClearVehicle`, after the facade's `assert(v != nullptr)`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_backup_vehicle_clear(v: VehicleRef, s: &'static Leaves) {
    order_backup_clear_vehicle(v, s);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDOrderFields" ["probe"] => Order {
        r#type => type,
        flags,
        dest,
        refit_cargo,
        wait_time,
        travel_time,
        max_speed,
    };
    "OpenTTDConsistState" ["probe"] => Consist {
        current_order_time,
        lateness_counter,
        timetable_start,
        depot_unbunching_last_departure,
        depot_unbunching_next_departure,
        round_trip_time,
        cur_real_order_index,
        cur_implicit_order_index,
        vehicle_flags,
    };
    "OpenTTDVehicleOrderState" ["probe"] => VehicleOrders {
        current_order,
        orders,
        next_shared,
        previous_shared,
    };
    "OpenTTDOrderListState" ["probe"] => List {
        num_manual_orders,
        num_vehicles,
        first_shared,
        timetable_duration,
        total_duration,
    };
    "OpenTTDOrderBackupState" ["probe"] => Backup {
        user,
        tile,
        group,
        clone,
    };
    "OpenTTDOrdersLeaves" ["probe"] => Leaves {
        allocate_list,
        backup,
        backup_can_allocate_item,
        backup_consist,
        backup_vector,
        create_backup,
        delete_backup,
        delete_list,
        list,
        list_can_allocate_item,
        next_backup,
        vector,
        backup_clear_name,
        backup_service_interval,
        backup_set_name,
        backup_set_service_interval,
        vehicle_set_name,
        vehicle_set_service_interval,
        base_station_get_if_valid,
        waypoint_get_if_valid,
        debug_check_sanity,
        get_vehicle_cannot_use_station_reason,
        invalidate_vehicle_order,
        append_station,
        clear_order_backup_unsafe,
        base,
        depots,
        scheduler,
        station_commands,
        vehicle_transactions,
    };
}

#[allow(unsafe_code)] // The tests build and read the fixture's native owners in place.
#[cfg(test)]
mod tests;
