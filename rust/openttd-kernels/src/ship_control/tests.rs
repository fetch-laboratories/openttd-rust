/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Unit tests of the ship bodies against expectations read from `ship_cmd.cpp`.

use super::*;
use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
use crate::vehicle_base::VehicleRecord;
use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};

/// `(type, m1, m4, m5)` of each tile of an 8 by 1 map: canal water, an aqueduct head, a
/// rail bridge head, a half-tile water rail tile, a barren rail tile, the south part of a
/// ship depot along the Y axis and two clear tiles.
fn install_map() -> (Vec<BaseTileLayout>, Vec<ExtendedTileLayout>) {
    let tiles = [
        (TileType::MP_WATER, WaterClass::Canal.0 << 5, 0, 0),
        (
            TileType::MP_TUNNELBRIDGE,
            0,
            0,
            TransportType::TRANSPORT_WATER.0 << 2,
        ),
        (
            TileType::MP_TUNNELBRIDGE,
            0,
            0,
            TransportType::TRANSPORT_RAIL.0 << 2,
        ),
        (
            TileType::MP_RAILWAY,
            0,
            crate::rail_map::RailGroundType::HalfTileWater.0,
            0,
        ),
        (
            TileType::MP_RAILWAY,
            0,
            crate::rail_map::RailGroundType::Barren.0,
            0,
        ),
        // `MakeShipDepot`'s `m5`: the depot type in bits 4 to 7, the axis and the part.
        (
            TileType::MP_WATER,
            0,
            0,
            crate::water_map::WaterTileType::Depot.0 << 4
                | crate::direction_type::Axis::AXIS_Y.0 << 1
                | crate::water_map::DepotPart::South.0,
        ),
        (TileType::MP_CLEAR, 0, 0, 0),
        (TileType::MP_CLEAR, 0, 0, 0),
    ];
    let base: Vec<BaseTileLayout> = tiles
        .iter()
        .map(|&(kind, m1, m4, m5)| BaseTileLayout {
            kind: kind.0 << 4,
            height: 0,
            m2: 0,
            m1,
            m3: 0,
            m4,
            m5,
        })
        .collect();
    let extended: Vec<ExtendedTileLayout> = (0..8)
        .map(|_| ExtendedTileLayout {
            m6: 0,
            m7: 0,
            m8: 0,
        })
        .collect();
    crate::map_func::fixture::set(
        TileTypeAllocation {
            base: base.as_ptr(),
            extended: extended.as_ptr(),
            count: 8,
            assertions: true,
        },
        3,
    );
    (base, extended)
}
extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
    unreachable!()
}
/// The shared table of the map [`install_map`] installs, with `native_assert` as its
/// movement table's assertion handler.
fn shared(native_assert: extern "C" fn(bool)) -> Services {
    let movement = Box::leak(Box::new(crate::movement::MovementServices {
        native_assert,
        ..crate::test_tables::MOVEMENT
    }));
    Services {
        movement: &raw const *movement,
        ..crate::services::fixture(std::ptr::null_mut(), random)
    }
}
/// A ship record with a fresh owner on its `ship` link, on `tile`.
fn ship(tile: u32) -> Ship {
    let record = VehicleRecord::new(std::ptr::null_mut(), VehicleType::VEH_SHIP);
    // SAFETY: The test's fresh record.
    unsafe {
        (*record).ship = openttd_rust_ship_state_new();
        (*record).tile = TileIndex(tile);
    }
    Ship::from_vehicle(VehicleRef::of(record))
}
/// Free a ship of [`ship`].
fn free(v: Ship) {
    // SAFETY: The test's record and owner, each freed once.
    unsafe {
        openttd_rust_ship_state_destroy(v.s.as_ptr());
        VehicleRecord::delete(v.v.0.as_ptr());
    }
}

#[test]
fn get_effective_water_class_takes_each_arm_and_records_each_assertion() {
    #[cfg(test)]
    thread_local! {
        static FAILED_ASSERTIONS: Cell<u32> = const { Cell::new(0) };
    }
    extern "C" fn record_assertion(okay: bool) {
        if !okay {
            FAILED_ASSERTIONS.set(FAILED_ASSERTIONS.get() + 1);
        }
    }
    let _map = install_map();
    let s = shared(record_assertion);
    let world = World::new(&s);
    let class = |tile| get_effective_water_class(&world, TileIndex(tile));
    assert!(
        class(0) == WaterClass::Canal && FAILED_ASSERTIONS.get() == 0,
        "the tile's own class"
    );
    assert!(
        class(1) == WaterClass::Canal && FAILED_ASSERTIONS.get() == 0,
        "an aqueduct"
    );
    assert!(
        class(2) == WaterClass::Canal && FAILED_ASSERTIONS.get() == 1,
        "a rail bridge breaks the transport assertion"
    );
    assert!(
        class(3) == WaterClass::Sea && FAILED_ASSERTIONS.get() == 1,
        "half-tile water rail"
    );
    assert!(
        class(4) == WaterClass::Sea && FAILED_ASSERTIONS.get() == 2,
        "barren rail breaks the ground assertion"
    );
}

/// `Ship::GetVehicleTrackdir`'s four arms, which the searches read at entry and no witness
/// distinguishes for a crashed ship or one on an aqueduct.
#[test]
fn get_vehicle_trackdir_takes_each_arm() {
    extern "C" fn no_failure(okay: bool) {
        assert!(okay, "no arm fails an assertion");
    }
    let _map = install_map();
    let s = shared(no_failure);
    let world = World::new(&s);
    let v = ship(5);

    v.set_state(TrackBits::TRACK_BIT_DEPOT);
    v.v.set_direction(Direction::DIR_N);
    v.v.set_vehstatus(VehStates::from_element(VehState::Crashed));
    assert!(
        get_vehicle_trackdir(&world, v) == Trackdir::INVALID_TRACKDIR,
        "a crashed ship has no trackdir, in a depot too"
    );

    v.v.set_vehstatus(VehStates(0));
    assert!(
        get_vehicle_trackdir(&world, v) == Trackdir::TRACKDIR_Y_SE,
        "in the south part of a Y depot the ship faces out, south-east"
    );

    v.set_state(TrackBits::TRACK_BIT_WORMHOLE);
    v.v.set_direction(Direction::DIR_SW);
    assert!(
        get_vehicle_trackdir(&world, v) == Trackdir::TRACKDIR_X_SW,
        "on an aqueduct the diagonal trackdir of the ship's direction"
    );

    v.set_state(TrackBits::TRACK_BIT_LOWER);
    v.v.set_direction(Direction::DIR_E);
    assert!(
        get_vehicle_trackdir(&world, v) == Trackdir::TRACKDIR_LOWER_E,
        "otherwise the trackdir of its track and direction"
    );
    free(v);
}

/// `ReverseShipIntoTrackdir` from a moving ship with a cached path: the direction and the
/// track of the trackdir, the speed and path cleared and both rotation coordinates narrowed
/// from the position, all before the position update and the one viewport call. The
/// scenarios cannot show it ran: a blocked search that finds no trackdir takes `ReverseShip`.
#[test]
fn reverse_ship_into_trackdir_turns_onto_the_trackdir_and_stops() {
    const NAME: &str =
        "ship_control::tests::reverse_ship_into_trackdir_turns_onto_the_trackdir_and_stops";
    static FAILED: AtomicU32 = AtomicU32::new(0);
    extern "C" fn record_failure(okay: bool) {
        if !okay {
            FAILED.fetch_add(1, Ordering::Relaxed);
        }
    }
    if crate::services_assert::isolate_test_check(NAME, record_failure) {
        return;
    }
    #[cfg(test)]
    thread_local! {
        static VIEWPORTS: Cell<u32> = const { Cell::new(0) };
        static SHIP: Cell<Option<Ship>> = const { Cell::new(None) };
    }
    extern "C" fn update_viewport(v: VehicleRef, force_update: bool, update_delta: bool) {
        let ship = SHIP.get().unwrap();
        assert!(
            v == ship.v && force_update && update_delta,
            "one forced update"
        );
        // SAFETY: The test's live owner; field-sized reads.
        let state = ship.s.as_ptr();
        assert!(
            v.direction() == Direction::DIR_E
                && ship.state() == TrackBits::TRACK_BIT_LOWER
                && v.cur_speed() == 0
                && ship.path_empty()
                // The `int16_t` narrowings of 70,000 and -40,000.
                && read_field!(state, rotation_x_pos) == 4_464
                && read_field!(state, rotation_y_pos) == 25_536,
            "every write precedes the viewport call"
        );
        assert!(
            !read_field!(v.0.as_ptr(), hash_tile_current).is_null(),
            "the position update precedes the viewport call"
        );
        VIEWPORTS.set(VIEWPORTS.get() + 1);
    }
    let _map = install_map();
    let s = shared(record_failure);
    let leaves = Leaves {
        update_viewport,
        ..crate::test_tables::SHIP_LEAVES
    };
    let v = ship(6);
    SHIP.set(Some(v));
    v.v.set_direction(Direction::DIR_NE);
    v.v.set_cur_speed(33);
    v.v.set_x_pos(70_000);
    v.v.set_y_pos(-40_000);
    v.set_state(TrackBits::TRACK_BIT_X);
    // SAFETY: The test's live owner.
    unsafe {
        crate::ship_yapf::openttd_rust_ship_path_push(
            openttd_rust_ship_state_path(v.s.as_ptr()),
            Trackdir::TRACKDIR_X_NE.0,
        );
    }

    let t = Tables::new(&leaves, &s);
    reverse_ship_into_trackdir(&t, v, Trackdir::TRACKDIR_LOWER_E);
    assert_eq!(VIEWPORTS.get(), 1, "one viewport call");
    assert_eq!(FAILED.load(Ordering::Relaxed), 0, "no failed assertion");

    // SAFETY: The test's live record; the removal reads no map.
    unsafe {
        crate::vehicle::occupancy::openttd_rust_occupancy_update(std::ptr::null(), v.v, true);
    }
    free(v);
}

/// `FindClosestShipDepot`'s two function statics are one process-wide slot, which the
/// game thread and the GUI thread share under the game-state mutex.
#[test]
fn depot_search_statics_are_reused_across_serialized_game_threads() {
    // Spawn/join provides the same ordering as the native game-state mutex.
    let capacity = std::thread::spawn(|| {
        let mut search = DepotSearch::take();
        search.patches_to_search.reserve(64);
        let capacity = search.patches_to_search.capacity();
        search.put_back();
        capacity
    })
    .join()
    .unwrap();
    let search = DepotSearch::take();
    assert_eq!(search.patches_to_search.capacity(), capacity);
    // SAFETY: This test's slot access; no other test uses it.
    unsafe { *DEPOT_SEARCH.get() = None };
}
