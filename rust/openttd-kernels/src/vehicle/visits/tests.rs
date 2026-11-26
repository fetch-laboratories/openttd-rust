/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tests of the station and depot visits. Each fixture vehicle is a fresh record whose shell
//! is the record itself, with its own orders and consist owners; the tables are leaked for
//! the test's thread, and the leaves record what the bodies ask of them.
use super::*;
use crate::cargo_storage::VehicleCargoList;
use crate::orders::Order;
use crate::vehicle::OpenTTDVehicle;
use crate::window_func::testing::Effect as Window;
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicU32, Ordering};

/// The fixture vehicle's `index`.
const INDEX: u32 = 17;
/// `_local_company`.
const LOCAL: u8 = 3;

/// A call a leaf received, in the order the body made it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Call {
    SetCargoType(u8),
    SetCargoSubtype(u8),
    /// `GetBestFittingSubType(v, v, cargo)`, with the cargo type stored when it runs.
    BestFittingSubType(u8, u8),
    /// `Engine::DetermineCapacity` reading the vehicle's cargo type, with the cargo type and
    /// subtype stored when it reads it.
    Capacity(u8, u8),
    SetLastLoadingStation(u16),
    SetTripOccupancy(i8),
    HideFillingPercent,
    MarkDirty,
    DeletePayment,
    WriteProfit(i64),
    CostAnimation(i32, i32, i32, i64),
}

#[cfg(test)]

thread_local! {
    static CALLS: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
    static CARGO_LIST: Cell<*const VehicleCargoList> = const { Cell::new(std::ptr::null()) };
    static CARGO_TYPE: Cell<u8> = const { Cell::new(0) };
    static CARGO_SUBTYPE: Cell<u8> = const { Cell::new(0) };
    static ENGINE: Cell<*mut crate::engine_type::OpenTTDEngine> =
        const { Cell::new(std::ptr::null_mut()) };
    static RESOLVER: Cell<*const crate::newgrf_vehicle::Services> =
        const { Cell::new(std::ptr::null()) };
    static SUCCEEDED: Cell<bool> = const { Cell::new(false) };
    static MONEY: Cell<i64> = const { Cell::new(0) };
}

fn record(call: Call) {
    CALLS.with(|c| c.borrow_mut().push(call));
}
fn calls() -> Vec<Call> {
    CALLS.take()
}
fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

// The vehicle resolver's member readers.
unsafe extern "C" fn index(_: *mut OpenTTDVehicle) -> u32 {
    INDEX
}
unsafe extern "C" fn cargo_type(_: *mut OpenTTDVehicle) -> u8 {
    CARGO_TYPE.get()
}
unsafe extern "C" fn cargo_subtype(_: *mut OpenTTDVehicle) -> u8 {
    CARGO_SUBTYPE.get()
}
unsafe extern "C" fn cargo_owner(_: *mut OpenTTDVehicle) -> *const VehicleCargoList {
    CARGO_LIST.get()
}
extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
    RESOLVER.get()
}

// The vehicle table's member writers and the shell's virtuals.
extern "C" fn set_cargo_type(_: VehicleRef, cargo: u8) {
    CARGO_TYPE.set(cargo);
    record(Call::SetCargoType(cargo));
}
extern "C" fn set_cargo_subtype(_: VehicleRef, subtype: u8) {
    CARGO_SUBTYPE.set(subtype);
    record(Call::SetCargoSubtype(subtype));
}
extern "C" fn engine(_: VehicleRef) -> crate::vehicle::EngineRef {
    crate::vehicle::EngineRef(std::ptr::NonNull::new(ENGINE.get()).unwrap())
}
extern "C" fn profit(_: VehicleRef) -> i64 {
    1000
}
extern "C" fn write_profit(_: VehicleRef, value: i64) {
    record(Call::WriteProfit(value));
}
extern "C" fn mark_dirty(_: VehicleRef) {
    record(Call::MarkDirty);
}
unsafe extern "C" fn engine_by_id(_: u16) -> *const crate::engine_type::OpenTTDEngine {
    ENGINE.get()
}
unsafe extern "C" fn no_callback(
    _: *mut crate::newgrf_vehicle::State,
    _: u16,
    _: u16,
    _: u32,
    _: u32,
    _: *mut i32,
    _: usize,
) -> u16 {
    crate::newgrf_callbacks::CALLBACK_FAILED
}
// The engine capability table's leaves.
extern "C" fn capacity_engine_id(_: *const crate::engine_type::OpenTTDEngine) -> u16 {
    0
}
extern "C" fn capacity_cargo(_: *mut OpenTTDVehicle) -> u8 {
    record(Call::Capacity(CARGO_TYPE.get(), CARGO_SUBTYPE.get()));
    CARGO_TYPE.get()
}
unsafe extern "C" fn best_fitting_sub_type(_: *mut (), _: *mut (), cargo: u8) -> u8 {
    record(Call::BestFittingSubType(cargo, CARGO_TYPE.get()));
    9
}

// The visit table's member slots.
extern "C" fn set_last_loading_station(_: VehicleRef, station: u16) {
    record(Call::SetLastLoadingStation(station));
}
extern "C" fn set_trip_occupancy(_: VehicleRef, occupancy: i8) {
    record(Call::SetTripOccupancy(occupancy));
}
extern "C" fn hide_filling_percent(_: VehicleRef) {
    record(Call::HideFillingPercent);
}

// The shared table.
extern "C" fn local_company() -> u8 {
    LOCAL
}
extern "C" fn cost_animation(x: i32, y: i32, z: i32, cost: i64) {
    record(Call::CostAnimation(x, y, z, cost));
}

// The command result.
extern "C" fn success(_: *const crate::services::CommandCost) -> bool {
    SUCCEEDED.get()
}
extern "C" fn money(_: *const crate::services::CommandCost) -> i64 {
    MONEY.get()
}

/// The visit table over the recording leaves, with `edit` applied to the shared, station
/// and loading tables first.
fn visits(
    edit: impl FnOnce(
        &mut crate::services::Services,
        &mut crate::station_service::World,
        &mut crate::station_service::Loading,
    ),
) -> &'static Services {
    let mut shared = crate::services::Services {
        cost_animation,
        local_company,
        map: crate::map_func::fixture::statics(),
        ..crate::test_tables::SHARED
    };
    let mut station = crate::test_tables::STATION_WORLD;
    let mut loading = crate::test_tables::STATION_LOADING;
    edit(&mut shared, &mut station, &mut loading);
    let shared = leak(shared);
    let resolver = leak(crate::newgrf_vehicle::test_table_answering(
        &crate::newgrf_vehicle::test_table_with_members(
            index,
            cargo_type,
            cargo_subtype,
            cargo_owner,
        ),
        engine_by_id,
        no_callback,
    ));
    RESOLVER.set(resolver);
    let vehicles = leak(crate::vehicle::VehicleServices {
        engine,
        maintenance: leak(crate::vehicle::maintenance::Services {
            mark_dirty,
            ..crate::test_tables::MAINTENANCE
        }),
        newgrf,
        profit,
        set_cargo_subtype,
        set_cargo_type,
        write_profit,
        ..crate::test_tables::VEHICLE
    });
    let orders = leak(crate::orders::Leaves {
        base: vehicles,
        ..crate::test_tables::ORDERS
    });
    let base = leak(crate::vehicle_transactions::TransactionServices {
        shared,
        cost: leak(crate::services::CommandCostServices {
            success,
            money,
            ..crate::test_tables::COMMAND_COSTS
        }),
        ..crate::test_tables::VEHICLE_TRANSACTION
    });
    let transactions = leak(crate::fleet::transactions::Services {
        subtype: best_fitting_sub_type,
        ..crate::test_tables::FLEET_TRANSACTIONS
    });
    let common = leak(crate::vehicle_transactions::CommonServices {
        base,
        transactions,
        vehicles,
        orders,
        ..crate::test_tables::VEHICLE_COMMON
    });
    let mut capabilities = crate::engine_capabilities::test_table_with_shared(shared);
    capabilities.vehicles = vehicles;
    capabilities.id = capacity_engine_id;
    capabilities.vehicle_cargo = capacity_cargo;
    let refit = leak(refit::Services {
        common,
        capabilities: leak(capabilities),
        ..crate::test_tables::REFIT
    });
    leak(Services {
        hide_filling_percent,
        links: leak(crate::station_service::Links {
            vehicle_resolver: resolver,
            ..crate::test_tables::STATION_LINKS
        }),
        loading: leak(loading),
        refit,
        set_last_loading_station,
        set_trip_occupancy,
        shared,
        station: leak(station),
        ..crate::test_tables::VISITS
    })
}

/// A front vehicle of `kind`, owned by `owner`, at `current`, with no orders list.
fn vehicle(kind: VehicleType, owner: crate::company_type::CompanyID, current: Order) -> VehicleRef {
    let record = crate::vehicle_base::openttd_rust_vehicle_record_new(std::ptr::null_mut(), kind);
    // SAFETY: The fresh record and owners; the test owns them.
    unsafe {
        (*record).shell = record.cast();
        (*record).orders = crate::orders::openttd_rust_vehicle_orders_new();
        (*record).consist = crate::orders::openttd_rust_consist_new();
        (*record).owner = owner;
        (*record).tile = crate::tile_type::TileIndex(0x0123);
        (*record).x_pos = 21;
        (*record).y_pos = 22;
        (*record).z_pos = 23;
    }
    let v = VehicleRef::of(record);
    // SAFETY: The fresh owner's current order starts here.
    unsafe { v.current_order_pointer().write(current) };
    v
}
const fn order(kind: OrderType, flags: u8, refit: u8) -> Order {
    Order {
        r#type: kind.0,
        flags,
        dest: 0,
        refit_cargo: refit,
        wait_time: 0,
        travel_time: 0,
        max_speed: u16::MAX,
    }
}

static FAILED_CHECKS: AtomicU32 = AtomicU32::new(0);
extern "C" fn count_failed_check(condition: bool) {
    if !condition {
        FAILED_CHECKS.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn autorefit_restore_reuses_storage_and_restores_payload() {
    // These identities are only compared, never dereferenced by restore.
    let vehicle = VehicleRef(std::ptr::NonNull::dangling());
    let mut frame = Refresher {
        vehicle,
        capacities: [37; CargoType::NUM_CARGO.0 as usize],
        refit_capacities: vec![
            RefitDesc {
                cargo: CargoType(2),
                capacity: 113,
                remaining: 71,
            },
            RefitDesc {
                cargo: CargoType(9),
                capacity: 211,
                remaining: 0,
            },
        ],
        cargo: CargoType(9),
        allow_merge: true,
        is_full_loading: true,
    };
    // The original makes one backup allocation, outside the cargo loop.
    let backup = frame.clone();
    let storage = frame.refit_capacities.as_ptr();
    let capacity = frame.refit_capacities.capacity();
    for _ in 0..CargoType::NUM_CARGO.0 {
        let mut other_identity = 0_u8;
        frame.vehicle = VehicleRef(std::ptr::NonNull::from(&mut other_identity).cast());
        frame.capacities.fill(0);
        frame.refit_capacities.fill(RefitDesc {
            cargo: CargoType(0),
            capacity: 0,
            remaining: 0,
        });
        frame.cargo = CargoType(0);
        frame.allow_merge = false;
        frame.is_full_loading = false;
        frame.restore(&backup);
        assert_eq!(frame.refit_capacities.as_ptr(), storage);
        assert_eq!(frame.refit_capacities.capacity(), capacity);
        assert_eq!(frame.refit_capacities.len(), backup.refit_capacities.len());
        assert_eq!(frame.vehicle.0, vehicle.0);
        assert_eq!(frame.capacities, backup.capacities);
        for (actual, expected) in frame.refit_capacities.iter().zip(&backup.refit_capacities) {
            assert_eq!(
                (actual.cargo, actual.capacity, actual.remaining),
                (expected.cargo, expected.capacity, expected.remaining)
            );
        }
        assert_eq!(frame.cargo, backup.cargo);
        assert_eq!(frame.allow_merge, backup.allow_merge);
        assert_eq!(frame.is_full_loading, backup.is_full_loading);
    }
}

/// `LinkRefresher::HandleRefit` (`refresh.cpp:81-92`) stores the refit cargo type, asks
/// `GetBestFittingSubType` with that type stored, stores its subtype, and then restores the
/// type and the subtype, each with its own store. `DetermineCapacity` runs between the subtype
/// store and the restores, seeing the refit cargo and subtype. A ship engine of capacity 40
/// whose property callback fails has that capacity, and the refit drops the part's
/// remaining capacity of its old cargo.
#[test]
fn handle_refit_stores_and_restores_type_and_subtype_in_the_original_order() {
    let s = visits(|_, _, _| {});
    let engine = crate::engine::openttd_rust_engine_new();
    // A ship engine carrying cargo 1, with capacity 40 and no callbacks.
    // SAFETY: The fresh engine; the test owns it.
    unsafe {
        (*engine).info.refit_mask = 1 << 5;
        (*engine).info.cargo_type = 1;
        (*engine).r#type = VehicleType::VEH_SHIP;
        (*engine).vehicle_info_type = VehicleType::VEH_SHIP;
        (*crate::engine::Lifecycle::ship(engine)).capacity = 40;
    }
    ENGINE.set(engine);
    CARGO_TYPE.set(2);
    CARGO_SUBTYPE.set(7);
    let v = vehicle(
        VehicleType::VEH_SHIP,
        crate::company_type::CompanyID(LOCAL),
        order(OrderType::OT_NOTHING, 0, 0xFE),
    );
    let mut refresher = Refresher {
        vehicle: v,
        capacities: [0; CargoType::NUM_CARGO.0 as usize],
        refit_capacities: vec![RefitDesc {
            cargo: CargoType(2),
            capacity: 10,
            remaining: 10,
        }],
        cargo: CargoType::INVALID_CARGO,
        allow_merge: true,
        is_full_loading: false,
    };
    refresher.capacities[2] = 10;
    calls();

    assert!(refresher.handle_refit(CargoType(5), s));
    assert_eq!(
        calls(),
        [
            Call::SetCargoType(5),
            Call::BestFittingSubType(5, 5),
            Call::SetCargoSubtype(9),
            Call::Capacity(5, 9),
            Call::SetCargoType(2),
            Call::SetCargoSubtype(7),
        ]
    );
    let desc = refresher.refit_capacities[0];
    assert_eq!(
        (desc.cargo, desc.capacity, desc.remaining),
        (CargoType(5), 40, 0)
    );
    assert_eq!(refresher.capacities[2], 0);
    assert_eq!(refresher.cargo, CargoType(5));

    // A cargo outside the refit mask leaves the part as it was.
    assert!(!refresher.handle_refit(CargoType(6), s));
    assert_eq!(calls(), []);
}

/// `CalcPercentVehicleFilled` asserts that a wanted colour has a valid station and a loading
/// order (`vehicle.cpp:1473`), now that its C++ facade no longer does; without a colour the
/// station may be absent.
#[test]
fn percent_filled_asserts_that_a_colour_needs_a_valid_station() {
    extern "C" fn no_station(_: u16) -> *mut BaseStation {
        std::ptr::null_mut()
    }
    if crate::services_assert::isolate_test_check(
        "vehicle::visits::tests::percent_filled_asserts_that_a_colour_needs_a_valid_station",
        count_failed_check,
    ) {
        return;
    }
    let s = visits(|_, station, _| station.get = no_station);
    CARGO_LIST.set(crate::cargo_storage::VehicleCargoList::of(
        crate::cargo_storage::openttd_rust_cargo_list_new(1),
    ));
    // A part without capacity: the colour loop reads no station.
    let v = vehicle(
        VehicleType::VEH_SHIP,
        crate::company_type::CompanyID(LOCAL),
        order(OrderType::OT_LOADING, 0, 0xFE),
    );

    let mut colour = StringID(0);
    assert_eq!(percent_filled(v, Some(&mut colour), s), 100);
    assert_eq!(colour, STR_PERCENT_NONE);
    assert_eq!(FAILED_CHECKS.load(Ordering::Relaxed), 1);
    assert_eq!(percent_filled(v, None, s), 100);
    assert_eq!(FAILED_CHECKS.load(Ordering::Relaxed), 1);
}

/// `Vehicle::LeaveStation` stores `CalcPercentVehicleFilled`'s `uint8_t` in the `int8_t`
/// `trip_occupancy`: 200 cargo in a capacity of 100 is 200%, stored as -56.
#[test]
fn leave_station_stores_the_fill_as_the_signed_trip_occupancy() {
    extern "C" fn no_payment(_: VehicleRef) -> *mut crate::cargo_payment::NativePayment {
        std::ptr::null_mut()
    }
    extern "C" fn delete_payment(payment: *mut crate::cargo_payment::NativePayment) {
        assert!(payment.is_null());
        record(Call::DeletePayment);
    }
    extern "C" fn no_station(_: u16) -> *mut BaseStation {
        std::ptr::null_mut()
    }
    #[cfg(test)]
    thread_local! {
        static STATE: Cell<*mut crate::station_service::Service> =
            const { Cell::new(std::ptr::null_mut()) };
    }
    extern "C" fn station(id: u16) -> crate::station_service::LoadingStation {
        assert_eq!(id, 5);
        crate::station_service::LoadingStation {
            native: std::ptr::NonNull::dangling().as_ptr(),
            base: std::ptr::NonNull::dangling().as_ptr(),
            state: STATE.get(),
        }
    }
    STATE.set(crate::station_service::openttd_rust_station_service_new());
    let s = visits(|_, world, loading| {
        world.get = no_station;
        world.station = station;
        loading.get_payment = no_payment;
        loading.delete_payment = delete_payment;
    });
    let list = crate::cargo_storage::openttd_rust_cargo_list_new(1);
    // SAFETY: The fresh list; the test owns it.
    unsafe { (*list).fields.count = 200 };
    CARGO_LIST.set(crate::cargo_storage::VehicleCargoList::of(list));
    // No loading and no unloading: no link refresh, no last loading station.
    let v = vehicle(
        VehicleType::VEH_SHIP,
        crate::company_type::CompanyID(LOCAL),
        order(OrderType::OT_LOADING, 0x44, 0xFE),
    );
    v.set_last_station_visited(StationID(5));
    v.set_cargo_cap(100);
    calls();

    leave_station(v, s);
    assert_eq!(
        calls(),
        [
            Call::DeletePayment,
            Call::HideFillingPercent,
            Call::SetTripOccupancy(-56),
            Call::MarkDirty,
        ]
    );
    assert!(v.current_order().is_type(OrderType::OT_LEAVESTATION));
}

/// The halt of `VehicleEnterDepot` (`vehicle.cpp:1623-1639`): it forgets the last loading
/// station and announces the wait to the local company as `AdviceType::VehicleWaiting`
/// advice with `STR_NEWS_TRAIN_IS_WAITING + v->type`, and to the owner's AI as
/// `ScriptEventVehicleWaitingInDepot` whoever the owner is.
#[test]
fn depot_halt_announces_the_wait_to_the_local_company_and_the_owners_ai() {
    use crate::news_func::testing::{Effect, Param, encoded, take};
    use crate::news_type::{NewsFlag, NewsFlags, NewsReference, NewsStyle, NewsType};
    // The halt adds the vehicle to the process-wide autoreplace map.
    if crate::services_assert::isolate_test_check(
        "vehicle::visits::tests::depot_halt_announces_the_wait_to_the_local_company_and_the_owners_ai",
        count_failed_check,
    ) {
        return;
    }
    let s = visits(|_, _, _| {});
    take();
    for owner in [LOCAL, LOCAL + 1] {
        let v = vehicle(
            VehicleType::VEH_ROAD,
            crate::company_type::CompanyID(owner),
            order(OrderType::OT_GOTO_DEPOT, 0, 0xFE),
        );
        calls();
        depot_halt(v, VehicleID(INDEX), s);
        assert_eq!(calls(), [Call::SetLastLoadingStation(u16::MAX)]);
        let mut expected = Vec::new();
        if owner == LOCAL {
            expected.push(Effect::News {
                headline: encoded(
                    STR_NEWS_TRAIN_IS_WAITING.0 + u32::from(VehicleType::VEH_ROAD.0),
                    &[Param::N(u64::from(INDEX))],
                ),
                r#type: NewsType::Advice,
                style: NewsStyle::Small,
                flags: NewsFlags::new(&[NewsFlag::InColour, NewsFlag::VehicleParam0]),
                ref1: NewsReference::Vehicle(VehicleID(INDEX)),
                ref2: NewsReference::None,
                data: None,
                advice_type: AdviceType::VehicleWaiting,
            });
        }
        // `ET_VEHICLE_WAITING_IN_DEPOT` is 14 in `script_event.hpp`'s enumeration; the
        // event carries only the vehicle.
        expected.push(Effect::AiNewEvent(
            owner,
            ScriptEvent {
                r#type: crate::script_event::ScriptEventType(14),
                ..ScriptEvent::aircraft_dest_too_far(VehicleID(INDEX))
            },
        ));
        assert_eq!(take(), expected, "owner {owner}");
    }
    assert_eq!(FAILED_CHECKS.load(Ordering::Relaxed), 0);
}

/// The outcome of the depot refit (`vehicle.cpp:1603-1614`): a failure tells the local
/// company with `AdviceType::RefitFailed` advice and `STR_NEWS_ORDER_REFIT_FAILED`; a success
/// with a cost takes the cost from this year's profit (`<< 8`) and shows the local company the
/// cost at the vehicle's position; a success without cost does neither.
#[test]
fn depot_refit_result_tells_the_failure_or_shows_the_cost() {
    use crate::news_func::testing::{Effect, Param, encoded, take};
    use crate::news_type::{NewsFlag, NewsFlags, NewsReference, NewsStyle, NewsType};
    // A failure adds the vehicle to the process-wide autoreplace map.
    if crate::services_assert::isolate_test_check(
        "vehicle::visits::tests::depot_refit_result_tells_the_failure_or_shows_the_cost",
        count_failed_check,
    ) {
        return;
    }
    let s = visits(|_, _, _| {});
    let cost = crate::services::Cost(
        std::ptr::NonNull::dangling().as_ptr(),
        s.refit.common.base.cost,
    );
    take();
    for owner in [LOCAL, LOCAL + 1] {
        let v = vehicle(
            VehicleType::VEH_ROAD,
            crate::company_type::CompanyID(owner),
            order(OrderType::OT_GOTO_DEPOT, 0, 5),
        );
        calls();

        SUCCEEDED.set(false);
        depot_refit_result(v, VehicleID(INDEX), cost, s);
        assert_eq!(calls(), []);
        let news = Effect::News {
            headline: encoded(STR_NEWS_ORDER_REFIT_FAILED.0, &[Param::N(u64::from(INDEX))]),
            r#type: NewsType::Advice,
            style: NewsStyle::Small,
            flags: NewsFlags::new(&[NewsFlag::InColour, NewsFlag::VehicleParam0]),
            ref1: NewsReference::Vehicle(VehicleID(INDEX)),
            ref2: NewsReference::None,
            data: None,
            advice_type: AdviceType::RefitFailed,
        };
        assert_eq!(
            take(),
            if owner == LOCAL { vec![news] } else { vec![] },
            "owner {owner}"
        );

        SUCCEEDED.set(true);
        MONEY.set(5);
        depot_refit_result(v, VehicleID(INDEX), cost, s);
        let mut expected = vec![Call::WriteProfit(1000 - (5 << 8))];
        if owner == LOCAL {
            expected.push(Call::CostAnimation(21, 22, 23, 5));
        }
        assert_eq!(calls(), expected, "owner {owner}");

        MONEY.set(0);
        depot_refit_result(v, VehicleID(INDEX), cost, s);
        assert_eq!(calls(), []);
        assert_eq!(take(), []);
    }
    assert_eq!(FAILED_CHECKS.load(Ordering::Relaxed), 0);
}

/// What the ending children write before they exit with `ENDED`.
const ENDED_MARKER: &str = "openttd-rust test: visits observation point reached";
const ENDED: i32 = 87;

/// `VehicleEnterDepot` for a road vehicle (`vehicle.cpp:1541-1568`): the road vehicle list,
/// then the vehicle's view, the depot's data (the vehicle is not a train) and the depot's
/// window, before the vehicle is hidden, stopped and serviced. The child ends at servicing's
/// first window call, the vehicle's details.
#[test]
fn enter_depot_of_a_road_vehicle_invalidates_its_windows_in_order() {
    if crate::services_assert::isolate_test_ends(
        "vehicle::visits::tests::enter_depot_of_a_road_vehicle_invalidates_its_windows_in_order",
        ENDED,
        ENDED_MARKER,
    ) {
        return;
    }
    #[cfg(test)]
    thread_local! {
        static VEHICLE: Cell<Option<VehicleRef>> = const { Cell::new(None) };
    }
    fn at_service(effect: &Window) {
        if *effect != Window::SetWindowDirty(WindowClass::WC_VEHICLE_DETAILS, WindowNumber(17)) {
            return;
        }
        let tile = WindowNumber(0x0123);
        let expected = [
            Window::SetWindowClassesDirty(WindowClass::WC_ROADVEH_LIST),
            Window::SetWindowDirty(WindowClass::WC_VEHICLE_VIEW, WindowNumber(17)),
            Window::InvalidateWindowData(WindowClass::WC_VEHICLE_DEPOT, tile, 0, false),
            Window::SetWindowDirty(WindowClass::WC_VEHICLE_DEPOT, tile),
        ];
        let windows = crate::window_func::testing::take();
        let v = VEHICLE.get().unwrap();
        let state = (v.vehstatus().test(VehState::Hidden), v.cur_speed());
        if windows == expected && state == (true, 0) {
            eprintln!("{ENDED_MARKER}");
        } else {
            eprintln!("windows {windows:?}, hidden and speed {state:?}");
        }
        std::process::exit(if windows == expected && state == (true, 0) {
            ENDED
        } else {
            1
        });
    }
    let s = visits(|_, _, _| {});
    let v = vehicle(
        VehicleType::VEH_ROAD,
        crate::company_type::CompanyID(LOCAL),
        order(OrderType::OT_NOTHING, 0, 0xFE),
    );
    v.set_cur_speed(30);
    VEHICLE.set(Some(v));
    crate::window_func::testing::take();
    crate::window_func::testing::observe(Some(at_service));
    enter_depot(v, s);
    unreachable!("vehicle servicing marks the details window dirty");
}

/// `VehicleEnterDepot` for a vehicle of no depot-entering type takes the native
/// `NOT_REACHED()` of its type `switch`, before any window call.
#[test]
fn enter_depot_of_an_effect_vehicle_is_not_reached_before_any_window_call() {
    extern "C" fn not_reached() {
        let windows = crate::window_func::testing::take();
        if windows.is_empty() {
            eprintln!("{ENDED_MARKER}");
            std::process::exit(ENDED);
        }
        eprintln!("windows before NOT_REACHED: {windows:?}");
        std::process::exit(1);
    }
    if crate::services_assert::isolate_test_ends(
        "vehicle::visits::tests::enter_depot_of_an_effect_vehicle_is_not_reached_before_any_window_call",
        ENDED,
        ENDED_MARKER,
    ) {
        return;
    }
    let s = visits(|shared, _, _| shared.not_reached = not_reached);
    // SAFETY: The leaked shared table and this thread's map statics live through the child.
    unsafe { crate::services_assert::bind_test_policy(count_failed_check, s.shared) };
    let v = vehicle(
        VehicleType::VEH_EFFECT,
        crate::company_type::CompanyID(LOCAL),
        order(OrderType::OT_NOTHING, 0, 0xFE),
    );
    crate::window_func::testing::take();
    enter_depot(v, s);
}

/// `VehicleEnterDepot` asserts that `v == v->First()`, with `First()` the stored link: a null
/// link fails the native assertion rather than ending the process another way.
#[test]
fn enter_depot_asserts_on_a_null_first_link() {
    if crate::services_assert::isolate_test_exit(
        "vehicle::visits::tests::enter_depot_asserts_on_a_null_first_link",
    ) {
        return;
    }
    let s = visits(|_, _, _| {});
    let v = vehicle(
        VehicleType::VEH_ROAD,
        crate::company_type::CompanyID(LOCAL),
        order(OrderType::OT_NOTHING, 0, 0xFE),
    );
    // SAFETY: The test's fresh record.
    unsafe { (*v.0.as_ptr()).first = std::ptr::null_mut() };
    enter_depot(v, s);
    unreachable!("the failed native assertion ends the child");
}
