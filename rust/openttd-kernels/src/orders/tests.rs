//! Tests of `orders.rs`: the order views and writers against `order_base.h`'s layout, the
//! bodies whose effects the harness cannot see (windows, news and the native fatal paths),
//! and the command bodies' native `CommandCost` constructions.
//!
//! The fixture holds a vehicle pool descriptor whose shells are the integers `index + 1`,
//! each with a fresh record, `Consist` and `VehicleOrders`, and one `List` and `Vector`
//! behind stubbed `list` and `vector` slots, all on the test's own thread.

use super::{
    CmdBulkChangeTimetable, CmdInsertOrder, CmdSetVehicleOnTime, Consist, Leaves, List, Order,
    OrderView, Vector, VehicleOrders, change_timetable, check_orders, cmd_bulk_change_timetable,
    cmd_insert_order, cmd_set_vehicle_on_time, current_order, free, is_waiting_for_unbunching,
    make_dummy, openttd_rust_consist_new, openttd_rust_order_vector_insert,
    openttd_rust_order_vector_new, openttd_rust_orderlist_new, openttd_rust_vehicle_orders_new,
    order_condition_compare, process_conditional_order, remove_from_shared,
};
use crate::cargo_type::CargoType;
use crate::command_type::DoCommandFlags;
use crate::company_type::CompanyID;
use crate::industrytype::NativeVec;
use crate::order_type::{
    DestinationID, ModifyTimetableFlags, OrderConditionComparator, OrderConditionVariable,
    OrderDepotActionFlag, OrderDepotActionFlags, OrderDepotTypeFlags, OrderLoadType,
    OrderNonStopFlags, OrderType, OrderUnloadType, VehicleOrderID,
};
use crate::pool::{PoolDescriptor, PoolSlot};
use crate::services::{CommandCost, CommandCostServices, Cost};
use crate::station_type::StationID;
use crate::strings_type::StringID;
use crate::table::strings::{
    STR_ERROR_CAN_T_ADD_ORDER, STR_ERROR_NO_BUOY, STR_ERROR_NO_RAIL_WAYPOINT,
    STR_ERROR_NO_ROAD_WAYPOINT, STR_ERROR_TIMETABLE_NOT_STARTED,
    STR_NEWS_VEHICLE_HAS_DUPLICATE_ENTRY, STR_NEWS_VEHICLE_HAS_TOO_FEW_ORDERS,
    STR_NEWS_VEHICLE_HAS_VOID_ORDER,
};
use crate::vehicle::VehicleRef;
use crate::vehicle_base::GroundVehicleSubtypeFlags;
use crate::vehicle_base::VehicleRecord;
use crate::vehicle_gui::VehicleInvalidateWindowData;
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::vehiclelist::{VehicleListIdentifier, VehicleListType};
use crate::window_type::{WindowClass, WindowNumber};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::ptr::{self, null_mut};

/// The fixture pool's slot count: indices at or past it are outside the pool.
const SLOTS: usize = 8;

#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
thread_local! {
    static POOL: Cell<*mut PoolDescriptor> = const { Cell::new(null_mut()) };
    static POOL_VIEW: Cell<*const PoolDescriptor> = const { Cell::new(ptr::null()) };
    static LIST: Cell<*mut List> = const { Cell::new(null_mut()) };
    static VECTOR: Cell<*mut Vector> = const { Cell::new(null_mut()) };
    static INVALIDATED: RefCell<Vec<(u32, i32)>> = const { RefCell::new(Vec::new()) };
    static TICK: Cell<u64> = const { Cell::new(0) };
}

/// The fixture pool's slot `index`.
fn slot(index: usize) -> PoolSlot {
    assert!(index < SLOTS);
    // SAFETY: The thread's live fixture pool; one slot copied out.
    unsafe { *(*POOL.get()).slots.as_mut_ptr().add(index) }
}
extern "C" fn record_from_shell(shell: *mut c_void) -> *mut VehicleRecord {
    slot(shell.addr() - 1).record.cast()
}
unsafe extern "C" fn vehicle_id(v: *mut crate::vehicle::OpenTTDVehicle) -> u32 {
    u32::try_from(VehicleRef::of(v).shell().addr() - 1).unwrap()
}
extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
    const TABLE: crate::newgrf_vehicle::Services =
        crate::newgrf_vehicle::TEST_TABLE.with_id(vehicle_id);
    ptr::from_ref(&TABLE)
}
unsafe extern "C" fn list(_: *mut c_void) -> *mut List {
    LIST.get()
}
unsafe extern "C" fn vector(_: *mut c_void) -> *mut Vector {
    VECTOR.get()
}
unsafe extern "C" fn invalidate_vehicle_order(v: *mut c_void, data: i32) {
    let index = u32::try_from(v.addr() - 1).unwrap();
    INVALIDATED.with_borrow_mut(|i| i.push((index, data)));
}
unsafe extern "C" fn debug_check_sanity(_: *mut c_void) {}
extern "C" fn tick_counter() -> u64 {
    TICK.get()
}
extern "C" fn local_company() -> u8 {
    0
}
unsafe extern "C" fn construct(data: *mut c_void, count: usize) {
    for i in 0..count {
        // SAFETY: The vector's storage for `count` orders.
        unsafe { data.cast::<Order>().add(i).write(Order::default()) };
    }
}
unsafe extern "C" fn destroy(_: *mut c_void, _: usize) {}

fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

/// `OrderReviewSystem` 2: check every vehicle's orders.
static ORDER_REVIEW_SYSTEM: u8 = 2;

/// The thread's fixture: an empty pool of `SLOTS` slots, a fresh list and vector, and the
/// orders table over them. The fixture's owners are leaked.
fn leaves() -> &'static Leaves {
    let empty = PoolSlot {
        shell: null_mut(),
        record: null_mut(),
    };
    let pool = Box::into_raw(Box::new(PoolDescriptor {
        slots: NativeVec::from_vec(vec![empty; SLOTS]),
        used_bitmap: NativeVec::new(),
        first_free: 0,
        first_unused: SLOTS,
        items: 0,
        checked: 0,
        cleaning: false,
        alloc_cache: null_mut(),
    }));
    POOL.set(pool);
    POOL_VIEW.set(pool);
    let scheduler = leak(crate::vehicle::scheduler::SchedulerServices {
        pool: POOL_VIEW.with(|view| leak(view.get())),
        ..crate::test_tables::SCHEDULER
    });
    LIST.set(openttd_rust_orderlist_new());
    VECTOR.set(openttd_rust_order_vector_new(construct, destroy));
    INVALIDATED.with_borrow_mut(Vec::clear);
    let shared = leak(crate::services::Services {
        local_company,
        tick_counter,
        settings: leak(crate::settings_type::SettingAddresses {
            order_review_system: &raw const ORDER_REVIEW_SYSTEM,
            ..crate::settings_type::SettingAddresses::NULL
        }),
        ..crate::test_tables::SHARED
    });
    let maintenance = leak(crate::vehicle::maintenance::Services {
        shared,
        ..crate::test_tables::MAINTENANCE
    });
    let base = leak(crate::vehicle::VehicleServices {
        maintenance,
        newgrf,
        record_from_shell,
        ..crate::test_tables::VEHICLE
    });
    leak(Leaves {
        list,
        vector,
        debug_check_sanity,
        invalidate_vehicle_order,
        base,
        scheduler,
        ..crate::test_tables::ORDERS
    })
}

/// `new (VehicleID(index)) Vehicle(kind)` in the fixture pool, a front engine of company 0
/// with its consist and order owners.
fn vehicle(index: usize, kind: VehicleType) -> VehicleRef {
    let record = VehicleRecord::new(ptr::without_provenance_mut(index + 1), kind);
    // SAFETY: The fresh record and the thread's live fixture pool.
    unsafe {
        (*record).consist = openttd_rust_consist_new();
        (*record).orders = openttd_rust_vehicle_orders_new();
        (*record).first = record;
        *(*POOL.get()).slots.as_mut_ptr().add(index) = PoolSlot {
            shell: ptr::without_provenance_mut(index + 1),
            record: record.cast(),
        };
    }
    let v = VehicleRef::of(record.cast());
    v.set_subtype(1 << GroundVehicleSubtypeFlags::GVSF_FRONT.0);
    v.set_owner(CompanyID(0));
    v
}
fn consist(v: VehicleRef) -> *mut Consist {
    super::consist(v).0
}
fn orders(v: VehicleRef) -> *mut VehicleOrders {
    super::vehicle_orders(v)
}
/// Link `vehicles`, in order, as the shared chain of the fixture list.
fn share(vehicles: &[VehicleRef]) {
    let list = LIST.get();
    // SAFETY: The fixture's live owners, with no other access live.
    unsafe {
        (*list).num_vehicles = u32::try_from(vehicles.len()).unwrap();
        (*list).first_shared = vehicles[0].shell();
        for (i, v) in vehicles.iter().enumerate() {
            let o = orders(*v);
            (*o).orders = list.cast();
            (*o).previous_shared = if i == 0 {
                null_mut()
            } else {
                vehicles[i - 1].shell()
            };
            (*o).next_shared = vehicles.get(i + 1).map_or(null_mut(), |n| n.shell());
        }
    }
}
/// Append `order` to the fixture list's orders.
fn push(order: Order) {
    let vector = VECTOR.get();
    // SAFETY: The fixture's live vector; the order is copied in.
    unsafe {
        openttd_rust_order_vector_insert(
            vector,
            super::openttd_rust_order_vector_size(vector),
            &raw const order,
        );
    }
}
fn order_with(edit: impl FnOnce(super::OrderRef)) -> Order {
    let mut order = Order::default();
    edit(order.edit());
    order
}
fn view(o: &Order) -> OrderView<'_> {
    // SAFETY: A reference names a live, initialized order.
    unsafe { OrderView::of(o) }
}

/// Exit code of [`ends_unreachable`]'s child at the bound native `NOT_REACHED`.
const NOT_REACHED_EXIT: i32 = 87;
/// What the bound native `NOT_REACHED` writes before it ends the child.
const NOT_REACHED_MARKER: &str = "openttd-rust test: orders reached the native NOT_REACHED";

/// Run `body` in a child of this test binary whose native `NOT_REACHED` callback is bound
/// to write a marker and exit, and require the child to end through that callback.
fn ends_unreachable(name: &str, body: impl FnOnce()) {
    extern "C" fn record_not_reached() {
        eprintln!("{NOT_REACHED_MARKER}");
        std::process::exit(NOT_REACHED_EXIT);
    }
    extern "C" fn abort_on_failure(condition: bool) {
        if !condition {
            std::process::abort();
        }
    }
    if crate::services_assert::isolate_test_ends(name, NOT_REACHED_EXIT, NOT_REACHED_MARKER) {
        return;
    }
    // `assert_with` reads only `assertions` of the bound map statics.
    let map = leak(crate::map_func::MapStatics {
        base: ptr::null(),
        extended: ptr::null(),
        size: ptr::null(),
        log_x: ptr::null(),
        log_y: ptr::null(),
        assertions: true,
    });
    let shared = leak(crate::services::Services {
        not_reached: record_not_reached,
        map,
        ..crate::test_tables::SHARED
    });
    // SAFETY: The leaked map statics live through the child's last assertion.
    unsafe { crate::services_assert::bind_test_policy(abort_on_failure, shared) };
    body();
    // Reaching this line would mean the unreachable path returned.
    std::process::exit(0);
}

/// `Vehicle::IsWaitingForUnbunching` past its depot assertion: a vehicle sharing a
/// two-order list whose previous order is its unbunching depot order waits while its next
/// departure is still ahead, and does not wait once that tick comes or without a shared list.
#[test]
fn is_waiting_for_unbunching_follows_the_shared_list_and_departure_time() {
    let s = leaves();
    let v = vehicle(0, VehicleType::VEH_ROAD);
    let other = vehicle(1, VehicleType::VEH_ROAD);
    share(&[v, other]);
    push(order_with(|o| {
        o.make_go_to_depot(
            DestinationID(0),
            OrderDepotTypeFlags(0),
            OrderNonStopFlags(0),
            OrderDepotActionFlags::from_element(OrderDepotActionFlag::Unbunch),
            CargoType::CARGO_NO_REFIT,
        );
    }));
    push(order_with(|o| o.make_go_to_station(StationID(1))));
    TICK.set(100);
    let c = consist(v);
    // SAFETY: Each write is to the fixture's leaked owners, with no other access live.
    unsafe {
        // The second order is current, so the previous one is the unbunching depot order.
        (*c).cur_implicit_order_index = 1;
        (*c).depot_unbunching_next_departure = 101;
    }
    assert!(is_waiting_for_unbunching(v, s));
    // SAFETY: As above.
    unsafe { (*c).depot_unbunching_next_departure = 100 };
    assert!(!is_waiting_for_unbunching(v, s));
    // SAFETY: As above.
    unsafe {
        (*c).depot_unbunching_next_departure = 101;
        (*LIST.get()).num_vehicles = 1;
    };
    assert!(!is_waiting_for_unbunching(v, s));
    // SAFETY: As above.
    unsafe { (*LIST.get()).num_vehicles = 2 };
    assert!(is_waiting_for_unbunching(v, s));
}

/// `Order::AssignOrder` copies each of the seven members of the source order.
#[test]
fn assign_order_copies_every_member() {
    let source = Order {
        r#type: 0x15,
        flags: 0x26,
        dest: 0x3456,
        refit_cargo: 7,
        wait_time: 0x1234,
        travel_time: 0x2345,
        max_speed: 0x3456,
    };
    let mut target = Order::default();
    target.edit().assign_order(view(&source));
    let fields = |o: &Order| {
        (
            o.r#type,
            o.flags,
            o.dest,
            o.refit_cargo,
            o.wait_time,
            o.travel_time,
            o.max_speed,
        )
    };
    assert_eq!(fields(&target), fields(&source));
}

/// `OrderList::MoveOrder` rotates the moved order to its new place in both directions, as
/// `std::rotate` over the original's vector does, and leaves the list unchanged for an equal
/// pair or an index at or past the order count.
#[test]
fn move_order_rotates_the_order_into_place_both_ways() {
    let s = leaves();
    for i in 0..6 {
        push(order_with(|o| o.make_go_to_station(StationID(i))));
    }
    let list = super::OrderListRef::of(ptr::without_provenance_mut(1), s);
    let destinations = || -> Vec<u16> {
        (0..list.len())
            .map(|i| list.order(i).view().destination().0)
            .collect()
    };
    list.move_order(VehicleOrderID(1), VehicleOrderID(4));
    assert_eq!(destinations(), [0, 2, 3, 4, 1, 5]);
    list.move_order(VehicleOrderID(4), VehicleOrderID(1));
    assert_eq!(destinations(), [0, 1, 2, 3, 4, 5]);
    list.move_order(VehicleOrderID(5), VehicleOrderID(0));
    assert_eq!(destinations(), [5, 0, 1, 2, 3, 4]);
    list.move_order(VehicleOrderID(2), VehicleOrderID(2));
    list.move_order(VehicleOrderID(6), VehicleOrderID(0));
    list.move_order(VehicleOrderID(0), VehicleOrderID(6));
    assert_eq!(destinations(), [5, 0, 1, 2, 3, 4]);
}

/// The `OrderView` getters, on orders whose fields are all distinct, against `order_base.h`'s
/// bit layouts: the unload type is bits 0-2 of `flags`, the load type bits 4-6, the
/// wait-timetabled flag bit 3 (`wait_time > 0` for a conditional order), and the skip-to
/// order of a conditional order is the whole `flags` byte.
mod order_view_getter_tests {
    use super::{CargoType, Order, OrderLoadType, OrderUnloadType, VehicleOrderID, view};

    fn order(r#type: u8, flags: u8, refit_cargo: u8, wait_time: u16) -> Order {
        Order {
            r#type,
            flags,
            dest: 0x1234,
            refit_cargo,
            wait_time,
            travel_time: 888,
            max_speed: 999,
        }
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

/// `Order::Free` and `Order::MakeDummy` on a vehicle's current order write only the fields
/// `order_cmd.cpp` names: `Free` the type, flags and destination; `MakeDummy` the type and
/// flags. Every field starts with a distinct value, so a whole-order store fails.
#[test]
fn free_and_make_dummy_write_only_the_original_fields() {
    fn fields(o: Order) -> [u32; 7] {
        [
            o.r#type.into(),
            o.flags.into(),
            o.dest.into(),
            o.refit_cargo.into(),
            o.wait_time.into(),
            o.travel_time.into(),
            o.max_speed.into(),
        ]
    }
    leaves();
    let v = vehicle(0, VehicleType::VEH_TRAIN);
    let full = Order {
        r#type: 0x12,
        flags: 0x34,
        dest: 0x5678,
        refit_cargo: 0x9a,
        wait_time: 0xbcde,
        travel_time: 0xf012,
        max_speed: 0x3456,
    };
    current_order(v).copy_assign(full);
    free(v);
    assert_eq!(
        fields(current_order(v).view().copied()),
        [
            OrderType::OT_NOTHING.0.into(),
            0,
            0,
            0x9a,
            0xbcde,
            0xf012,
            0x3456
        ]
    );
    current_order(v).copy_assign(full);
    make_dummy(v);
    assert_eq!(
        fields(current_order(v).view().copied()),
        [
            OrderType::OT_DUMMY.0.into(),
            0,
            0x5678,
            0x9a,
            0xbcde,
            0xf012,
            0x3456
        ]
    );
}

/// `order_base.h` packs `type` as type (bits 0-3), stop location (4-5) and non-stop flags
/// (6-7), and `flags` as depot order type or unload type (0-2), depot action type (3-6) and
/// load type (4-6). The expectations are those fields of distinct stored values, so a getter
/// reading the wrong field or bits fails.
#[test]
fn view_getters_decode_the_original_bit_layout() {
    let order = Order {
        r#type: 0b1011_0110,
        flags: 0b1101_1010,
        dest: 0x1234,
        refit_cargo: 7,
        wait_time: 0x2345,
        travel_time: 0x3456,
        max_speed: 0x4567,
    };
    let view = view(&order);
    assert_eq!(view.kind(), OrderType(6));
    assert_eq!(view.non_stop(), OrderNonStopFlags(2));
    assert_eq!(view.depot_type(), OrderDepotTypeFlags(2));
    assert_eq!(view.depot_action_flags(), OrderDepotActionFlags(11));
    assert_eq!(view.destination(), DestinationID(0x1234));
    assert_eq!(view.destination_raw(), 0x1234);
    assert_eq!(view.max_speed(), 0x4567);
    // Load type 5 is neither `FullLoad` (2) nor `FullLoadAny` (3).
    assert!(!view.is_full_load());
    let copy = view.copied();
    assert_eq!(
        (
            copy.r#type,
            copy.flags,
            copy.dest,
            copy.refit_cargo,
            copy.wait_time,
            copy.travel_time,
            copy.max_speed
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
            flags,
            ..Order::default()
        };
        assert_eq!(view(&order).is_full_load(), full, "flags {flags:#x}");
    }
}

/// One native `CommandCost` error constructor call.
#[derive(Debug, PartialEq, Eq)]
enum Constructed {
    /// `CommandCost(msg)`.
    Error(u32),
    /// `CommandCost(msg, extra_msg)`.
    ErrorExtra(u32, u32),
}
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
thread_local! {
    static CONSTRUCTED: RefCell<Vec<Constructed>> = const { RefCell::new(Vec::new()) };
}
extern "C" fn cost_error(_: *mut CommandCost, msg: u32) {
    CONSTRUCTED.with(|c| c.borrow_mut().push(Constructed::Error(msg)));
}
extern "C" fn cost_error_extra(_: *mut CommandCost, msg: u32, extra_msg: u32) {
    CONSTRUCTED.with(|c| c.borrow_mut().push(Constructed::ErrorExtra(msg, extra_msg)));
}
extern "C" fn cost_zero(_: *mut CommandCost) {}
extern "C" fn cost_success(_: *const CommandCost) -> bool {
    true
}
static COSTS: CommandCostServices = CommandCostServices {
    zero: cost_zero,
    error: cost_error,
    error_extra: cost_error_extra,
    success: cost_success,
    ..crate::test_tables::COMMAND_COSTS
};
extern "C" fn current_company() -> u8 {
    0
}
extern "C" fn company() -> *const crate::company::Services {
    const SHARED: crate::services::Services = crate::services::Services {
        current_company,
        ..crate::test_tables::SHARED
    };
    const COMPANY: crate::company::Services = crate::company::Services {
        shared: &SHARED,
        ..crate::test_tables::COMPANY
    };
    &COMPANY
}
extern "C" fn waypoint(_: u16) -> *mut crate::station::BaseStation {
    ptr::NonNull::dangling().as_ptr()
}
extern "C" fn no_facilities(_: *mut crate::station::BaseStation) -> u8 {
    0
}

/// Order entries write the original's `CommandCost` at their returns: `CMD_ERROR` as
/// `CommandCost(INVALID_STRING_ID)` (`CmdBulkChangeTimetable` of a vehicle outside the
/// pool), a single string (`CmdSetVehicleOnTime` before the timetable starts) and
/// `CmdInsertOrder`'s three waypoint returns, whose message and extra message take the
/// original's positions.
#[test]
fn order_errors_construct_the_original_command_costs() {
    let base = leaves();
    let station_commands = leak(crate::station_cmd::CommandServicesTable {
        cost: &raw const COSTS,
        command: leak(crate::services::CommandServices {
            company,
            ..crate::test_tables::COMMANDS
        }),
        station: leak(crate::station_service::World {
            facilities: no_facilities,
            ..crate::test_tables::STATION_WORLD
        }),
        ..crate::test_tables::STATION_COMMANDS
    });
    let s = leak(Leaves {
        waypoint_get_if_valid: waypoint,
        station_commands,
        ..*base
    });
    let train = vehicle(0, VehicleType::VEH_TRAIN);
    vehicle(1, VehicleType::VEH_ROAD);
    vehicle(2, VehicleType::VEH_SHIP);
    share(&[train]);
    let cost = Cost(ptr::NonNull::dangling().as_ptr(), &COSTS);
    let flags = DoCommandFlags::EMPTY;
    CONSTRUCTED.with(|c| c.borrow_mut().clear());
    cmd_bulk_change_timetable(
        s,
        cost,
        flags,
        CmdBulkChangeTimetable {
            veh: VehicleID(u32::try_from(SLOTS).unwrap()),
            mtf: ModifyTimetableFlags::MTF_WAIT_TIME,
            data: 0,
        },
    );
    cmd_set_vehicle_on_time(
        s,
        cost,
        flags,
        CmdSetVehicleOnTime {
            veh: VehicleID(0),
            apply_to_group: false,
        },
    );
    let order = order_with(|o| o.make_go_to_waypoint(StationID(0)));
    for veh in 0..3 {
        cmd_insert_order(
            s,
            cost,
            flags,
            CmdInsertOrder {
                veh: VehicleID(veh),
                sel_ord: VehicleOrderID(0),
                new_order: view(&order),
            },
        );
    }
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

/// `OrderConditionCompare`: each of the eight comparators against the original's switch.
#[test]
fn order_condition_compare_follows_each_comparator() {
    use OrderConditionComparator as C;
    for (variable, value) in [(3, 3), (2, 3), (4, 3), (0, 0), (0, 5), (-1, 0)] {
        let expected = [
            (C::Equal, variable == value),
            (C::NotEqual, variable != value),
            (C::LessThan, variable < value),
            (C::LessThanOrEqual, variable <= value),
            (C::MoreThan, variable > value),
            (C::MoreThanOrEqual, variable >= value),
            (C::IsTrue, variable != 0),
            (C::IsFalse, variable == 0),
        ];
        for (occ, result) in expected {
            assert_eq!(
                order_condition_compare(occ, variable, value),
                result,
                "{occ:?} {variable} {value}"
            );
        }
    }
}

/// `default: NOT_REACHED()` for a comparator past `OCC_IS_FALSE`.
#[test]
fn order_condition_compare_past_is_false_is_not_reached() {
    ends_unreachable(
        "orders::tests::order_condition_compare_past_is_false_is_not_reached",
        || {
            order_condition_compare(OrderConditionComparator::End, 0, 0);
        },
    );
}

fn conditional(variable: OrderConditionVariable, skip_to: u8) -> Order {
    order_with(|o| {
        o.make_conditional(VehicleOrderID(skip_to));
        o.set_condition_variable(variable);
        o.set_condition_comparator(OrderConditionComparator::Equal);
    })
}

/// `ProcessConditionalOrder`: `OCV_UNCONDITIONALLY` returns the skip-to order, and an order
/// that is not conditional returns `INVALID_VEH_ORDER_ID` before any read of the vehicle,
/// whose record here holds no consist or order owners.
#[test]
fn process_conditional_order_returns_the_skip_to_order_or_none() {
    let s = leaves();
    let record = VehicleRecord::new(ptr::without_provenance_mut(1), VehicleType::VEH_TRAIN);
    let v = VehicleRef::of(record.cast());
    let unconditional = conditional(OrderConditionVariable::Unconditionally, 3);
    assert_eq!(
        process_conditional_order(view(&unconditional), v, s),
        VehicleOrderID(3)
    );
    for order in [
        order_with(|o| o.make_go_to_station(StationID(4))),
        order_with(super::OrderRef::make_dummy),
        Order::default(),
    ] {
        assert_eq!(
            process_conditional_order(view(&order), v, s),
            VehicleOrderID::INVALID_VEH_ORDER_ID
        );
    }
}

/// `default: NOT_REACHED()` for `OCV_END`, the first variable with no arm.
#[test]
fn process_conditional_order_past_the_last_variable_is_not_reached() {
    ends_unreachable(
        "orders::tests::process_conditional_order_past_the_last_variable_is_not_reached",
        || {
            let s = leaves();
            let v = vehicle(0, VehicleType::VEH_TRAIN);
            let order = conditional(OrderConditionVariable::End, 1);
            process_conditional_order(view(&order), v, s);
        },
    );
}

/// `ChangeTimetable` with `MTF_WAIT_TIME` writes the list's order and its durations, then,
/// for each shared vehicle, its `current_order` only where `cur_real_order_index` is the
/// order and `current_order` equals it, and dirties each vehicle's timetable window.
#[test]
fn change_timetable_updates_only_matching_current_orders() {
    let s = leaves();
    let matching = vehicle(0, VehicleType::VEH_TRAIN);
    let other_order = vehicle(1, VehicleType::VEH_TRAIN);
    let other_index = vehicle(2, VehicleType::VEH_TRAIN);
    share(&[matching, other_order, other_index]);
    let target = order_with(|o| {
        o.make_go_to_station(StationID(5));
        o.set_wait_time(20);
        o.set_wait_timetabled(true);
    });
    push(order_with(|o| o.make_go_to_station(StationID(4))));
    push(target);
    for (v, index, current) in [
        (matching, 1, target),
        (
            other_order,
            1,
            order_with(|o| o.make_go_to_station(StationID(6))),
        ),
        (other_index, 0, target),
    ] {
        // SAFETY: The fixture's owners, with no other access live.
        unsafe { (*consist(v)).cur_real_order_index = index };
        current_order(v).copy_assign(current);
    }
    crate::window_func::testing::take();
    change_timetable(
        matching,
        VehicleOrderID(1),
        50,
        ModifyTimetableFlags::MTF_WAIT_TIME,
        true,
        s,
    );
    let wait = |v| {
        let o = current_order(v).view();
        (o.wait_time(), o.is_wait_timetabled())
    };
    assert_eq!(wait(matching), (50, true));
    assert_eq!(wait(other_order), (0, false));
    assert_eq!(wait(other_index), (20, true));
    let list = super::order_list(matching, s).unwrap();
    assert_eq!(list.order(1).view().wait_time(), 50);
    // SAFETY: The fixture's list.
    let (total, timetable) = unsafe {
        (
            (*LIST.get()).total_duration,
            (*LIST.get()).timetable_duration,
        )
    };
    assert_eq!((total, timetable), (30, 30));
    assert_eq!(
        crate::window_func::testing::take(),
        (0..3)
            .map(|i| crate::window_func::testing::Effect::SetWindowDirty(
                WindowClass::WC_VEHICLE_TIMETABLE,
                WindowNumber(i)
            ))
            .collect::<Vec<_>>()
    );
}

/// `default: NOT_REACHED()` for a flag past `MTF_TRAVEL_SPEED`.
#[test]
fn change_timetable_past_the_last_flag_is_not_reached() {
    ends_unreachable(
        "orders::tests::change_timetable_past_the_last_flag_is_not_reached",
        || {
            let s = leaves();
            let v = vehicle(0, VehicleType::VEH_TRAIN);
            share(&[v]);
            push(order_with(|o| o.make_go_to_station(StationID(4))));
            change_timetable(
                v,
                VehicleOrderID(0),
                1,
                ModifyTimetableFlags::MTF_END,
                true,
                s,
            );
        },
    );
}

/// The shared-orders window number of a list whose first vehicle is `first`.
fn shared_window(first: u32) -> WindowNumber {
    VehicleListIdentifier {
        list_type: VehicleListType::VL_SHARED_ORDERS,
        vtype: VehicleType::VEH_TRAIN,
        company: CompanyID(0),
        index: first,
    }
    .to_window_number()
}

/// `Vehicle::RemoveFromShared`: with one vehicle left, the shared-orders window of the
/// vehicle type's list class and the old first's number is closed and the remaining vehicle's
/// orders invalidated; when the first vehicle leaves a longer list, that window is
/// invalidated with the new first's index and bit 31.
#[test]
#[allow(clippy::cast_possible_wrap, reason = "the original's `int` data")]
fn remove_from_shared_closes_or_renumbers_the_shared_orders_window() {
    use crate::window_func::testing::{Effect, take};
    let s = leaves();
    let first = vehicle(0, VehicleType::VEH_TRAIN);
    let second = vehicle(1, VehicleType::VEH_TRAIN);
    share(&[first, second]);
    take();
    remove_from_shared(second, s);
    assert_eq!(
        take(),
        [Effect::CloseWindowById(
            WindowClass::WC_TRAINS_LIST,
            shared_window(0),
            true,
            0
        )]
    );
    assert_eq!(
        INVALIDATED.with_borrow_mut(std::mem::take),
        [(0, VehicleInvalidateWindowData::VIWD_MODIFY_ORDERS.0)]
    );
    // SAFETY: The fixture's owners.
    unsafe {
        assert!((*orders(second)).next_shared.is_null());
        assert!((*orders(second)).previous_shared.is_null());
        assert!((*orders(first)).next_shared.is_null());
        assert_eq!((*LIST.get()).num_vehicles, 1);
    }

    let s = leaves();
    let first = vehicle(2, VehicleType::VEH_TRAIN);
    let second = vehicle(3, VehicleType::VEH_TRAIN);
    let third = vehicle(4, VehicleType::VEH_TRAIN);
    share(&[first, second, third]);
    take();
    remove_from_shared(first, s);
    let data = (3u32 | (1 << 31)) as i32;
    assert_eq!(
        take(),
        [Effect::InvalidateWindowData(
            WindowClass::WC_TRAINS_LIST,
            shared_window(2),
            data,
            false
        )]
    );
    assert!(INVALIDATED.with_borrow(Vec::is_empty));
    // SAFETY: The fixture's owners.
    unsafe {
        assert_eq!((*LIST.get()).first_shared, second.shell());
        assert!((*orders(second)).previous_shared.is_null());
        assert_eq!((*LIST.get()).num_vehicles, 2);
    }
}

/// `VehicleListIdentifier::ToWindowNumber`: `c << 28 | type << 23 | vtype << 26 | index`,
/// with `OWNER_NONE` packed as `0xF`.
#[test]
#[allow(
    clippy::cast_possible_wrap,
    reason = "the original's `int` window number"
)]
fn to_window_number_packs_the_original_fields() {
    let number = |list_type, vtype, company, index| {
        VehicleListIdentifier {
            list_type,
            vtype,
            company,
            index,
        }
        .to_window_number()
        .0
    };
    let expected =
        |c: u32, t: u32, vt: u32, index: u32| (c << 28 | t << 23 | vt << 26 | index) as i32;
    assert_eq!(
        number(
            VehicleListType::VL_SHARED_ORDERS,
            VehicleType::VEH_ROAD,
            CompanyID(3),
            77
        ),
        expected(3, 1, 1, 77)
    );
    assert_eq!(
        number(
            VehicleListType::VL_STATION_LIST,
            VehicleType::VEH_TRAIN,
            crate::company_type::Owner::NONE,
            5
        ),
        expected(0xF, u32::from(VehicleListType::VL_STATION_LIST.0), 0, 5)
    );
    let last = VehicleListType(VehicleListType::VLT_END.0 - 1);
    assert_eq!(
        number(
            last,
            VehicleType::VEH_AIRCRAFT,
            CompanyID(14),
            (1 << 20) - 1
        ),
        expected(14, u32::from(last.0), 3, (1 << 20) - 1)
    );
}

/// `ToWindowNumber`'s `assert(this->index < (1 << 20))`.
#[test]
fn to_window_number_asserts_an_index_past_twenty_bits() {
    if crate::services_assert::isolate_test_exit(
        "orders::tests::to_window_number_asserts_an_index_past_twenty_bits",
    ) {
        return;
    }
    let _ = VehicleListIdentifier {
        list_type: VehicleListType::VL_SHARED_ORDERS,
        vtype: VehicleType::VEH_TRAIN,
        company: CompanyID(0),
        index: 1 << 20,
    }
    .to_window_number();
}

/// `CheckOrders` news for the first shared vehicle of company 0 on a twentieth day: a dummy
/// order, a duplicate first and last order, or a single order; none for a vehicle that is not
/// the first shared one.
#[test]
fn check_orders_posts_the_original_advice() {
    use crate::news_func::testing::{Effect, Param, encoded, take};
    use crate::news_type::{AdviceType, NewsFlag, NewsFlags, NewsReference, NewsStyle, NewsType};
    let news = |message: StringID, index: u32| Effect::News {
        headline: encoded(message.0, &[Param::N(u64::from(index))]),
        r#type: NewsType::Advice,
        style: NewsStyle::Small,
        flags: NewsFlags::new(&[NewsFlag::InColour, NewsFlag::VehicleParam0]),
        ref1: NewsReference::Vehicle(VehicleID(index)),
        ref2: NewsReference::None,
        data: None,
        advice_type: AdviceType::Order,
    };
    let waypoint = |id| order_with(|o| o.make_go_to_waypoint(StationID(id)));
    for (index, orders, message) in [
        (
            0,
            vec![waypoint(1), order_with(super::OrderRef::make_dummy)],
            STR_NEWS_VEHICLE_HAS_VOID_ORDER,
        ),
        (
            1,
            vec![waypoint(1), waypoint(2), waypoint(1)],
            STR_NEWS_VEHICLE_HAS_DUPLICATE_ENTRY,
        ),
        (2, vec![waypoint(1)], STR_NEWS_VEHICLE_HAS_TOO_FEW_ORDERS),
    ] {
        let s = leaves();
        let first = vehicle(index, VehicleType::VEH_TRAIN);
        let second = vehicle(index + 1, VehicleType::VEH_TRAIN);
        share(&[first, second]);
        for order in orders {
            push(order);
        }
        take();
        check_orders(second, s);
        assert_eq!(take(), []);
        check_orders(first, s);
        assert_eq!(take(), [news(message, u32::try_from(index).unwrap())]);
    }
}
