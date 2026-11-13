/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Tests of the aircraft bodies the scenarios cannot distinguish: the rotation table,
//! its unreachable rotation and the native assertions this port added.

use super::{AircraftState, Leaves, Vehicle, World};
use crate::airport::{AirportMovingData, AirportMovingDataFlags, TERM1};
use crate::direction_type::Direction;
use crate::services::Services;
use std::cell::RefCell;
use std::ffi::c_void;

#[cfg(test)]
thread_local! {
    static CHECKS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
}
/// The recording native assertion: each call's condition, in order.
extern "C" fn record_check(condition: bool) {
    CHECKS.with(|c| c.borrow_mut().push(condition));
}
fn take_checks() -> Vec<bool> {
    CHECKS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// A shared table whose native assertion records, over this thread's empty map.
fn shared() -> &'static Services {
    crate::map_func::fixture::set(
        crate::map_func::TileTypeAllocation {
            base: std::ptr::null(),
            extended: std::ptr::null(),
            count: 0,
            assertions: true,
        },
        6,
    );
    let mut shared = crate::test_tables::SHARED;
    shared.map = crate::map_func::fixture::statics();
    shared.movement = Box::leak(Box::new(crate::movement::MovementServices {
        native_assert: record_check,
        ..crate::test_tables::MOVEMENT
    }));
    Box::leak(Box::new(shared))
}

/// The unreachable leaf table over `shared` and `vehicle`.
fn leaves(vehicle: crate::vehicle::VehicleServices) -> Leaves {
    let mut leaves = crate::test_tables::AIRCRAFT_LEAVES;
    leaves.shared = shared();
    leaves.vehicle = Box::leak(Box::new(vehicle));
    leaves
}

/// A live aircraft record and state with the original constructor defaults.
fn aircraft() -> Vehicle {
    Vehicle {
        handle: crate::vehicle_base::openttd_rust_vehicle_record_new(
            std::ptr::null_mut(),
            crate::vehicle_type::VehicleType::VEH_AIRCRAFT,
        )
        .cast(),
        state: super::openttd_rust_aircraft_state_new(),
        id: 17,
    }
}

/// One asymmetric input, rotated by each `Direction` the original handles. The
/// expectations are `RotateAirportMovingData`'s formulas on a 2x3-tile airport.
#[test]
fn rotate_airport_moving_data_follows_the_original_rotations() {
    let orig = AirportMovingData {
        x: 3,
        y: 5,
        flags: AirportMovingDataFlags(0x0123),
        direction: Direction::DIR_NE,
    };
    let cases = [
        (Direction::DIR_N, 3, 5, Direction::DIR_NE),
        (Direction::DIR_E, 5, 48 - 3 - 1, Direction::DIR_SE),
        (Direction::DIR_S, 32 - 3 - 1, 48 - 5 - 1, Direction::DIR_SW),
        (Direction::DIR_W, 32 - 5 - 1, 3, Direction::DIR_NW),
    ];
    for (rotation, x, y, direction) in cases {
        let amd = crate::airport::rotate_airport_moving_data(orig, rotation, 2, 3, |ok| {
            assert!(ok, "ChangeDir precondition");
        });
        assert_eq!(
            amd,
            AirportMovingData {
                x,
                y,
                flags: orig.flags,
                direction
            },
            "rotation {rotation:?}"
        );
    }
}

/// `default: NOT_REACHED()` for an odd rotation: the native unreachable path aborts the
/// process, so the test runs it in a child and observes the abnormal exit.
#[test]
fn rotate_airport_moving_data_odd_rotation_is_not_reached() {
    const CHILD: &str = "OPENTTD_AIRCRAFT_ROTATION_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let orig = AirportMovingData {
            x: 0,
            y: 0,
            flags: AirportMovingDataFlags(0),
            direction: Direction::DIR_N,
        };
        let _ = crate::airport::rotate_airport_moving_data(orig, Direction::DIR_NE, 1, 1, |_| {});
        // Reaching this line would mean the unreachable path returned.
        std::process::exit(0);
    }
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "aircraft::tests::rotate_airport_moving_data_odd_rotation_is_not_reached",
        ])
        .env(CHILD, "1")
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "the odd rotation must take the native unreachable path"
    );
}

/// `AircraftGetEntryPoint`'s `assert(apc != nullptr)`, before the target station is read.
#[test]
fn aircraft_get_entry_point_asserts_a_null_airport() {
    extern "C" fn no_station(_: u16) -> *const crate::station::Station {
        std::ptr::null()
    }
    extern "C" fn entry(apc: *const c_void, dir: u8) -> u8 {
        assert!(apc.is_null());
        dir
    }
    let mut leaves = leaves(crate::test_tables::VEHICLE);
    leaves.station = no_station;
    leaves.airport_entry = entry;
    let v = aircraft();
    take_checks();
    // SAFETY: The leaked tables outlive the test.
    let w = unsafe { World::<false>::new(&raw const leaves) };
    // With target tile 0 and the aircraft at (0, 0), the direction is DIAGDIR_SE.
    let entry_point = w.aircraft_get_entry_point(v, std::ptr::null(), Direction::DIR_N);
    assert_eq!(entry_point, 1);
    assert_eq!(take_checks(), [false], "assert(apc != nullptr)");
}

/// `AirportMove` at a position past `nofelements`: the debug line, then the failed
/// assertion, then the original body over the out-of-range position.
#[test]
fn airport_move_past_the_last_position_reports_and_asserts() {
    extern "C" fn one_element(_: *const c_void) -> u8 {
        1
    }
    extern "C" fn node(apc: *const c_void, _: u8) -> *const c_void {
        apc
    }
    extern "C" fn heading(_: *const c_void) -> u8 {
        TERM1
    }
    extern "C" fn nothing(_: crate::vehicle::VehicleRef) -> u8 {
        crate::order_type::OT_NOTHING
    }
    let mut vehicle = crate::test_tables::VEHICLE;
    vehicle.order_type = nothing;
    let mut leaves = leaves(vehicle);
    leaves.airport_elements = one_element;
    leaves.node = node;
    leaves.fta_heading = heading;
    let v = aircraft();
    // SAFETY: The test owns the live state allocation.
    unsafe {
        let state: &mut AircraftState = &mut *v.state;
        state.pos = 5;
        state.previous_pos = 5;
        state.state = TERM1;
    }
    crate::debug::tests::take();
    take_checks();
    // SAFETY: The leaked tables outlive the test.
    let w = unsafe { World::<false>::new(&raw const leaves) };
    let apc = std::ptr::NonNull::<u8>::dangling()
        .as_ptr()
        .cast_const()
        .cast();
    /* At a terminal with no order, AircraftEventHandler_AtTerminal returns at once. */
    assert!(w.airport_move(v, apc));
    let lines = crate::debug::tests::take();
    assert_eq!(
        lines,
        [(
            "misc".to_owned(),
            0,
            b"[Ap] position 5 is not valid for current airport. Max position is 0".to_vec()
        )]
    );
    assert_eq!(take_checks(), [false], "assert(v->pos < apc->nofelements)");
}

/// `AircraftEventHandler_General`, the table's `TO_ALL` entry: `FatalError` with the
/// original's message, which does not return. The test runs it in a child and reads the
/// message the recording fatal-error service prints.
#[test]
fn aircraft_event_handler_general_is_a_fatal_error() {
    const CHILD: &str = "OPENTTD_AIRCRAFT_SCHEME_CHILD";
    extern "C" fn print_fatal(message: *const u8, size: usize) {
        // SAFETY: The service receives the message's bytes for this call.
        let message = unsafe { std::slice::from_raw_parts(message, size) };
        eprintln!("fatal: {}", String::from_utf8_lossy(message));
    }
    if std::env::var_os(CHILD).is_some() {
        let mut leaves = leaves(crate::test_tables::VEHICLE);
        let mut shared = *shared();
        shared.fatal_error = print_fatal;
        leaves.shared = Box::leak(Box::new(shared));
        let v = aircraft();
        // SAFETY: The test owns the live state allocation; state 0 is TO_ALL.
        unsafe { (*v.state).state = 0 };
        // SAFETY: The leaked tables outlive the test.
        let w = unsafe { World::<false>::new(&raw const leaves) };
        w.aircraft_state_handler(v, std::ptr::null());
        std::process::exit(0);
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "aircraft::tests::aircraft_event_handler_general_is_a_fatal_error",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(!output.status.success(), "FatalError does not return");
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("fatal: OK, you shouldn't be here, check your Airport Scheme!\n")
    );
}
