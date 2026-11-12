/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Frozen observations from the unchanged original, replacing `MovementGap` in
//! `tools/simulation/landscape-loop-reference.cpp`. The existing `landscape.py`
//! extractor supplied `slope_func.h`, track helpers/tables, `GetRailFoundation`,
//! `GetRoadFoundation`, `GetPartialPixelZ`, `GetTileSlopeGivenHeight`,
//! `GetTilePixelSlopeOutsideMap`, `GetSlopePixelZ_Void` and `GetSlopePixelZOutsideMap`.
//! The original-only runner used `MovementGap`'s identical inputs and height shim;
//! no candidate function or output contributed an expected value. Native ABI
//! round trips remain in `ground_protocol.hpp` and `landscape_protocol.hpp`.

use super::{CallbackFoundation, MovementServices, MovementVehicle, ObjectFoundation, VehicleRef};
use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation, fixture};
use crate::road_type::RoadBits;
use crate::services::{RoadTramType, RoadType, Services};
use crate::slope_type::Slope;
use crate::track_type::TrackBits;
use std::cell::Cell;
use std::ffi::c_void;
use std::ptr;

// Raw slope input bytes used by the retiring native corpus, including the
// original's nonnumeric order of the four steep corners.
const SLOPES: [u8; 19] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 12, 14, 11, 27, 23, 30, 29,
];
const X_COORDS: [i32; 13] = [
    i32::MIN,
    -65537,
    -16,
    -1,
    0,
    1,
    15,
    16,
    1023,
    1024,
    65535,
    65536,
    i32::MAX,
];
const Y_COORDS: [i32; 10] = [i32::MIN, -16, -1, 0, 15, 1023, 1024, 65535, 65536, i32::MAX];

fn rows(text: &str) -> impl Iterator<Item = Vec<i32>> + '_ {
    text.lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            line.split_whitespace()
                .map(|value| value.parse().unwrap())
                .collect()
        })
}

#[test]
fn foundations_match_original() {
    let mut oracle = rows(include_str!("foundation-oracle.txt"));
    let mut cases = 0;
    for slope in SLOPES {
        let expected = oracle.next().unwrap();
        assert_eq!(expected.len(), 513);
        assert_eq!(expected[0], i32::from(slope));
        for bits in u8::MIN..=u8::MAX {
            let index = 1 + usize::from(bits) * 2;
            assert_eq!(
                i32::from(
                    super::get_rail_foundation(Slope(slope), TrackBits(bits), |ok| assert!(ok)).0
                ),
                expected[index],
                "rail slope {slope}, mask {bits}"
            );
            assert_eq!(
                i32::from(
                    super::get_road_foundation(Slope(slope), RoadBits(bits), |ok| assert!(ok)).0
                ),
                expected[index + 1],
                "road slope {slope}, mask {bits}"
            );
            cases += 2;
        }
    }
    assert!(oracle.next().is_none());
    assert_eq!(cases, 9728);
}

thread_local! {
    static DIMENSIONS: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
}
extern "C" fn max_x() -> u32 {
    DIMENSIONS.get().0 - 1
}
extern "C" fn max_y() -> u32 {
    DIMENSIONS.get().1 - 1
}
extern "C" fn random(_: *mut c_void) -> u32 {
    unreachable!()
}
extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
    unreachable!()
}
extern "C" fn compatible(_: VehicleRef) -> u64 {
    unreachable!()
}
extern "C" fn speed(_: RoadType) -> u16 {
    unreachable!()
}
extern "C" fn kind(_: VehicleRef) -> RoadTramType {
    unreachable!()
}
extern "C" fn resolve(_: VehicleRef) -> MovementVehicle {
    unreachable!()
}
extern "C" fn invalidate(_: VehicleRef) {
    unreachable!()
}
extern "C" fn check(ok: bool) {
    assert!(ok);
}
extern "C" fn object(_: u32) -> ObjectFoundation {
    unreachable!()
}
extern "C" fn callback(_: u32) -> CallbackFoundation {
    unreachable!()
}
extern "C" fn industry_callback(_: u32, _: u16) -> u16 {
    unreachable!()
}

fn services(shared: &Services) -> MovementServices {
    MovementServices {
        shared,
        orders: ptr::null(),
        train: ptr::null(),
        vehicle: ptr::null(),
        tunnel_bridge: ptr::null(),
        road_compatible: compatible,
        road_type_speed: speed,
        road_kind: kind,
        road_vehicle: ptr::null(),
        resolve,
        depot_invalidate: invalidate,
        native_assert: check,
        object_foundation: object,
        house_foundation: callback,
        industry_foundation: callback,
        industry_callback,
    }
}

fn allocation(log_x: u32, log_y: u32) -> (Vec<BaseTileLayout>, Vec<ExtendedTileLayout>) {
    let mut tiles: Vec<_> = (0..1_u32 << (log_x + log_y))
        .map(|_| BaseTileLayout {
            kind: 0,
            height: 3,
            m2: 0,
            m1: 0,
            m3: 0,
            m4: 0,
            m5: 0,
        })
        .collect();
    let extended: Vec<_> = (0..tiles.len())
        .map(|_| ExtendedTileLayout {
            m6: 0,
            m7: 0,
            m8: 0,
        })
        .collect();
    DIMENSIONS.set((1 << log_x, 1 << log_y));
    fixture::set(
        TileTypeAllocation {
            base: tiles.as_mut_ptr(),
            extended: extended.as_ptr(),
            count: 1 << (log_x + log_y),
            assertions: true,
        },
        log_x,
    );
    (tiles, extended)
}

#[test]
fn extreme_coordinates_match_original() {
    let mut shared = crate::services::fixture(ptr::null_mut(), random, industry);
    shared.map_max_x = max_x;
    shared.map_max_y = max_y;
    let movement = services(&shared);
    let mut oracle = rows(include_str!("coordinate-oracle.txt"));
    let mut cases = 0;
    for log_x in [6, 12] {
        for log_y in [6, 12] {
            let (mut tiles, _extended) = allocation(log_x, log_y);
            for slope in SLOPES {
                let expected = oracle.next().unwrap();
                assert_eq!(expected.len(), 133);
                assert_eq!(
                    expected[..3],
                    [
                        i32::try_from(log_x).unwrap(),
                        i32::try_from(log_y).unwrap(),
                        i32::from(slope)
                    ]
                );
                let corner =
                    |raised: Slope| 3 + u8::from(Slope(slope) & raised != Slope::SLOPE_FLAT);
                tiles[0].height = corner(Slope::SLOPE_N);
                tiles[1].height = corner(Slope::SLOPE_W);
                tiles[1 << log_x].height = corner(Slope::SLOPE_E);
                tiles[(1 << log_x) + 1].height = corner(Slope::SLOPE_S);
                for (index, (x, y)) in X_COORDS
                    .into_iter()
                    .flat_map(|x| Y_COORDS.map(|y| (x, y)))
                    .enumerate()
                {
                    // Initialized tables and allocation remain live for this
                    // entry; callbacks neither reset nor yield the map.
                    let actual = movement.get_slope_pixel_z_outside_map(x, y);
                    assert_eq!(
                        actual,
                        expected[3 + index],
                        "map {log_x}/{log_y}, slope {slope}, ({x}, {y})"
                    );
                    cases += 1;
                }
            }
        }
    }
    fixture::set(
        TileTypeAllocation {
            base: ptr::null(),
            extended: ptr::null(),
            count: 0,
            assertions: true,
        },
        0,
    );
    assert!(oracle.next().is_none());
    assert_eq!(cases, 9880);
}
