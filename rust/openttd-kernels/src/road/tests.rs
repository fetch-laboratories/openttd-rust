/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Replaces the native movement-table comparison. The oracle is transcribed from
//! `OpenTTD` 15.3 (14ec60f248547d4d062a1160f0fc26d742319888), `table/roadveh_movement.h`.
//! Each line is tram, state, then every original (x, y) byte pair in frame order.
//! It covers the tram/articulated table gap without claiming controller coverage.

#[test]
fn original_movement_and_stop_tables() {
    for line in include_str!("tests/movement-oracle.txt").lines() {
        let mut columns = line.split_whitespace();
        let tram = columns.next().unwrap() == "1";
        let state = columns.next().unwrap().parse::<usize>().unwrap();
        let row = columns.next().unwrap();
        for frame in 0..row.len() / 4 {
            let start = frame * 4;
            let x = u8::from_str_radix(&row[start..start + 2], 16).unwrap();
            let y = u8::from_str_radix(&row[start + 2..start + 4], 16).unwrap();
            assert_eq!(
                crate::road_data::entry(tram, state, frame),
                (x, y),
                "tram={tram}, state={state}, frame={frame}"
            );
        }
    }
    let stop_frames = [
        20, 20, 16, 16, 20, 20, 16, 16, 19, 19, 15, 15, 19, 19, 15, 15, 16, 16, 12, 12, 16, 16, 12,
        12, 15, 15, 11, 11, 15, 15, 11, 11,
    ];
    for (index, expected) in (0_u32..).zip(stop_frames) {
        assert_eq!(super::openttd_rust_road_stop_frame(index), expected);
    }
}

/// The two reversal switches of `IndividualRoadVehicleController`, every valid input
/// against the original's case table (`roadveh_cmd.cpp:1237-1242` and `:1380-1385`),
/// written out as the original's enum values.
#[test]
fn tram_reversal_switches() {
    use super::{tram_long_corner_trackdir, tram_turn_around_needed};
    use crate::road_type::RoadBits;
    use crate::track_type::Trackdir;
    /* TRACKDIR_RVREV_NE = 6, _SE = 7, _SW = 14, _NW = 15; ROAD_NW = 1, _SW = 2,
     * _SE = 4, _NE = 8. */
    for (dir, needed) in [(6, 2), (7, 1), (14, 8), (15, 4)] {
        assert_eq!(
            tram_turn_around_needed(Trackdir(dir)),
            RoadBits(needed),
            "trackdir {dir}"
        );
    }
    /* DIAGDIR_NE = 0, _SE = 1, _SW = 2, _NW = 3; the upper drive data bits are ignored. */
    for (x, dir) in [(3, 7), (0, 14), (1, 15), (2, 6)] {
        for high in [0, 0xFC] {
            assert_eq!(
                tram_long_corner_trackdir(x | high),
                Trackdir(dir),
                "x {x:#x}"
            );
        }
    }
}

/// The controller bodies against `src/roadveh_cmd.cpp`, through recording tables.
#[cfg(test)]
mod controller {
    use super::super::*;
    use crate::news_func::testing::{Effect, Param, encoded, take};
    use crate::news_type::{AdviceType, NewsFlag, NewsFlags, NewsReference, NewsStyle};
    use crate::script_event::ScriptEventType;
    use crate::table::strings::{
        STR_NEWS_FIRST_BUS_ARRIVAL, STR_NEWS_FIRST_CARGO_TRAM_ARRIVAL,
        STR_NEWS_FIRST_PASSENGER_TRAM_ARRIVAL, STR_NEWS_FIRST_TRUCK_ARRIVAL,
        STR_NEWS_ROAD_VEHICLE_CRASH, STR_NEWS_ROAD_VEHICLE_CRASH_DRIVER,
    };
    use crate::test_tables::{GROUND_VEHICLE, ROAD_LEAVES, SHARED, VEHICLE};
    use std::cell::Cell;

    thread_local! {
        static BUS: Cell<bool> = const { Cell::new(true) };
        static ROADTYPE: Cell<u8> = const { Cell::new(0) };
        static HAD: Cell<u8> = const { Cell::new(0) };
        /// The `had_vehicle_of_type` field accesses since the last reset.
        static ACCESSES: Cell<u32> = const { Cell::new(0) };
        /// The `st->index` reads since the last reset.
        static INDEX_READS: Cell<u32> = const { Cell::new(0) };
        static VICTIMS: Cell<u32> = const { Cell::new(0) };
        static EXPECTED: Cell<*const Vec<Effect>> = const { Cell::new(std::ptr::null()) };
    }
    const VEHICLE_ID: u32 = 17;
    const STATION: StationID = StationID(55);
    /// The arrival's `Station *`, never dereferenced by the controller.
    const STATION_HANDLE: StationHandle = StationHandle {
        st: std::ptr::dangling_mut(),
    };
    /// `st->index` of the arrival's station, counted at each read.
    extern "C" fn station_id(st: *const crate::station::BaseStation) -> u16 {
        assert_eq!(st, STATION_HANDLE.base().cast_const());
        INDEX_READS.set(INDEX_READS.get() + 1);
        STATION.0
    }
    /// The airport services table holding [`station_id`], installed until dropped.
    struct StationIndexReader(*const crate::newgrf_airport::Services);
    impl StationIndexReader {
        fn install() -> Self {
            // SAFETY: Serialized by the game-state test lock the caller holds; restored on drop.
            let table = Box::leak(Box::new(crate::newgrf_airport::Services::with_station_id(
                station_id,
            )));
            Self(unsafe { crate::newgrf_airport::openttd_rust_airport_exchange(table) })
        }
    }
    impl Drop for StationIndexReader {
        fn drop(&mut self) {
            // SAFETY: As `install`; the lock is still held.
            unsafe { crate::newgrf_airport::openttd_rust_airport_exchange(self.0) };
        }
    }
    const LOCAL: u8 = 3;
    const TILE: TileIndex = TileIndex(100);
    /// `new ScriptEventStationFirstVehicle(st->index, v->index)`, field by field,
    /// independent of its constructor.
    const FIRST_VEHICLE: ScriptEvent = ScriptEvent {
        r#type: ScriptEventType::ET_STATION_FIRST_VEHICLE,
        vehicle: VEHICLE_ID,
        tile: 0,
        crash_reason: CrashReason(0),
        victims: 0,
        station: 55,
        subsidy: 0,
        industry: 0,
        town: 0,
        company: 0,
    };
    /// The default road type table's classification, `ROADTYPE_ROAD` a road type and
    /// `ROADTYPE_TRAM` a tram type, held under the game-state test lock until dropped,
    /// which restores the previous masks.
    struct DefaultRoadTypes {
        saved: (u64, u64),
        _guard: std::sync::MutexGuard<'static, ()>,
    }
    impl DefaultRoadTypes {
        fn new() -> Self {
            let guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
            // SAFETY: Process-wide masks, serialized by the lock and restored on drop.
            let saved = unsafe {
                (
                    crate::type_registry::openttd_rust_road_mask()
                        .replace(1 << RoadType::ROADTYPE_ROAD.0),
                    crate::type_registry::openttd_rust_tram_mask()
                        .replace(1 << RoadType::ROADTYPE_TRAM.0),
                )
            };
            Self {
                saved,
                _guard: guard,
            }
        }
    }
    impl Drop for DefaultRoadTypes {
        fn drop(&mut self) {
            // SAFETY: The lock is still held.
            unsafe {
                *crate::type_registry::openttd_rust_road_mask() = self.saved.0;
                *crate::type_registry::openttd_rust_tram_mask() = self.saved.1;
            }
        }
    }

    unsafe extern "C" fn id(_: *mut crate::vehicle::OpenTTDVehicle) -> u32 {
        VEHICLE_ID
    }
    unsafe extern "C" fn roadtype(_: *mut crate::vehicle::OpenTTDVehicle) -> RoadType {
        RoadType(ROADTYPE.get())
    }
    extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
        thread_local! {
            static TABLE: &'static crate::newgrf_vehicle::Services = {
                // The other fields are private, so the two live entries are set in place.
                let mut table = crate::newgrf_vehicle::TEST_TABLE;
                table.id = id;
                table.roadtype = roadtype;
                Box::leak(Box::new(table))
            };
        }
        TABLE.with(|t| std::ptr::from_ref(*t))
    }
    extern "C" fn is_bus(_: VehicleRef) -> bool {
        BUS.get()
    }
    extern "C" fn road_stops() -> *const crate::road_stop::RoadStopServices {
        thread_local! {
            static STOPS: crate::road_stop::RoadStopServices = crate::road_stop::RoadStopServices {
                vehicle: std::ptr::null(),
                primary: crate::test_tables::unreachable2,
                head: crate::test_tables::unreachable2,
                is_bus,
                next: crate::test_tables::unreachable1,
                tile: crate::test_tables::unreachable1,
                state: crate::test_tables::unreachable1,
                pool_next: crate::test_tables::unreachable1,
            };
        }
        STOPS.with(std::ptr::from_ref)
    }
    extern "C" fn local_company() -> u8 {
        LOCAL
    }
    /// `&st->had_vehicle_of_type` of the arrival's station, at each access: the read,
    /// then on a first arrival the store, both before any news or event.
    extern "C" fn had_vehicle_of_type_field(st: *mut crate::station::Station) -> *mut u8 {
        assert_eq!(st, STATION_HANDLE.st);
        assert_eq!(
            take(),
            Vec::new(),
            "the station bit is read and stored first"
        );
        ACCESSES.set(ACCESSES.get() + 1);
        HAD.with(Cell::as_ptr)
    }

    /// A road vehicle record of `owner`, its road owner and the tables of one entry.
    struct Fixture {
        record: *mut crate::vehicle_base::VehicleRecord,
        state: Box<State>,
        leaves: &'static Leaves,
    }
    impl Fixture {
        fn new(owner: u8, shared: &crate::services::Services) -> Self {
            let vehicle = Box::leak(Box::new(crate::vehicle::VehicleServices {
                newgrf,
                ..VEHICLE
            }));
            let ground = Box::leak(Box::new(GroundServices {
                physics: &GROUND_VEHICLE,
            }));
            let shared = Box::leak(Box::new(crate::services::Services {
                road_stops,
                local_company,
                station_had_vehicle_of_type_field: had_vehicle_of_type_field,
                ..*shared
            }));
            let leaves = Box::leak(Box::new(Leaves {
                ground,
                shared,
                vehicle,
                ..ROAD_LEAVES
            }));
            let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
                std::ptr::null_mut(),
                crate::vehicle_type::VehicleType::VEH_ROAD,
            );
            let mut state = Box::<State>::default();
            // SAFETY: A fresh record; the fixture owns it and its road owner.
            unsafe {
                (*record).road = &raw mut state.fields;
                (*record).ground =
                    Box::leak(Box::new(crate::ground_vehicle::GroundState::default()));
                (*record).first = record.cast();
                (*record).owner = crate::company_type::CompanyID(owner);
                (*record).tile = TILE;
                (*record).subtype =
                    1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_FRONT.0;
            }
            Self {
                record,
                state,
                leaves,
            }
        }
        fn part(&mut self) -> Part {
            Part::new(VehicleRef::of(self.record.cast()), &raw mut *self.state)
        }
        fn game(&self) -> Game<'static> {
            // SAFETY: The leaked tables outlive the test.
            unsafe { Game::new(self.leaves) }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // SAFETY: The record came from `openttd_rust_vehicle_record_new`.
            unsafe { crate::vehicle_base::openttd_rust_vehicle_record_delete(self.record) };
        }
    }

    /// `RoadVehArrivesAt`: for a bus or a truck on road or tram, owned by the local or
    /// another company, a first arrival stores the station bit, then adds the vehicle
    /// news with the headline of `RoadTypeIsRoad(v->roadtype)` and the station as its
    /// parameter, then the AI event of the owner and the game event. A station that
    /// already had a vehicle of the type gets nothing.
    #[test]
    fn road_veh_arrives_at_marks_the_station_then_reports_the_first_arrival() {
        let cases = [
            (
                true,
                0,
                StationHadVehicleOfType::HVOT_BUS,
                STR_NEWS_FIRST_BUS_ARRIVAL,
            ),
            (
                true,
                1,
                StationHadVehicleOfType::HVOT_BUS,
                STR_NEWS_FIRST_PASSENGER_TRAM_ARRIVAL,
            ),
            (
                false,
                0,
                StationHadVehicleOfType::HVOT_TRUCK,
                STR_NEWS_FIRST_TRUCK_ARRIVAL,
            ),
            (
                false,
                1,
                StationHadVehicleOfType::HVOT_TRUCK,
                STR_NEWS_FIRST_CARGO_TRAM_ARRIVAL,
            ),
        ];
        let _types = DefaultRoadTypes::new();
        let _index = StationIndexReader::install();
        take();
        for (bus, road_type, bit, headline) in cases {
            ROADTYPE.set(road_type);
            BUS.set(bus);
            for owner in [LOCAL, LOCAL + 1] {
                let mut f = Fixture::new(owner, &SHARED);
                HAD.set(0);
                ACCESSES.set(0);
                INDEX_READS.set(0);
                f.game().road_veh_arrives_at(f.part(), STATION_HANDLE);
                assert_eq!(HAD.get(), bit.0, "bus {bus}, road type {road_type}");
                assert_eq!(ACCESSES.get(), 2, "a read and a store on the first arrival");
                assert_eq!(
                    INDEX_READS.get(),
                    4,
                    "st->index at the headline, the news reference and both events"
                );
                let expected = vec![
                    Effect::News {
                        headline: encoded(headline.0, &[Param::N(u64::from(STATION.0))]),
                        r#type: if owner == LOCAL {
                            NewsType::ArrivalCompany
                        } else {
                            NewsType::ArrivalOther
                        },
                        style: NewsStyle::Thin,
                        flags: NewsFlags::new(&[NewsFlag::NoTransparency, NewsFlag::Shaded]),
                        ref1: NewsReference::Vehicle(VehicleID(VEHICLE_ID)),
                        ref2: NewsReference::Station(STATION),
                        data: None,
                        advice_type: AdviceType::Invalid,
                    },
                    Effect::AiNewEvent(owner, FIRST_VEHICLE),
                    Effect::GameNewEvent(FIRST_VEHICLE),
                ];
                assert_eq!(
                    take(),
                    expected,
                    "bus {bus}, road type {road_type}, owner {owner}"
                );

                // Visited before: no store, news or event.
                ACCESSES.set(0);
                INDEX_READS.set(0);
                f.game().road_veh_arrives_at(f.part(), STATION_HANDLE);
                assert_eq!(HAD.get(), bit.0);
                assert_eq!(ACCESSES.get(), 1, "only the read on a repeat visit");
                assert_eq!(INDEX_READS.get(), 0, "no st->index read on a repeat visit");
                assert_eq!(take(), Vec::new());
            }
        }
    }

    extern "C" fn base_crash(_: VehicleRef, flooded: bool) -> u32 {
        assert!(!flooded);
        VICTIMS.get()
    }
    const RATING_REACHED: i32 = 43;
    /// `ModifyStationRatingAround` follows the news: its first service ends the child,
    /// after checking the effects recorded so far.
    extern "C" fn station_catchment() -> *const crate::station::CatchmentServices {
        // SAFETY: The test set the expected effects before the call.
        let expected = unsafe { &*EXPECTED.get() };
        if take() == *expected {
            std::process::exit(RATING_REACHED);
        }
        std::process::exit(1);
    }

    /// `RoadVehCrash`: the crash counts the driver, then the AI event of the owner and
    /// the game event (`CRASH_RV_LEVEL_CROSSING`, the victims and the owner), then the
    /// tile news: `STR_NEWS_ROAD_VEHICLE_CRASH_DRIVER` for one victim, otherwise
    /// `STR_NEWS_ROAD_VEHICLE_CRASH` with the victim count, `Accident` for the local
    /// company and `AccidentOther` otherwise; then the station ratings around. Each case
    /// runs in a child, which the ratings' first service ends.
    #[test]
    fn road_veh_crash_reports_events_then_news_before_the_ratings() {
        const NAME: &str =
            "road::tests::controller::road_veh_crash_reports_events_then_news_before_the_ratings";
        const CHILD: &str = "OPENTTD_ROAD_CRASH_CHILD";
        let Some(case) = std::env::var_os(CHILD) else {
            for case in ["0 3", "0 4", "4 3", "4 4"] {
                let status = std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", NAME])
                    .env(CHILD, case)
                    .status()
                    .unwrap();
                assert_eq!(status.code(), Some(RATING_REACHED), "case {case}");
            }
            return;
        };
        let case = case.into_string().unwrap();
        let (base_victims, owner) = case.split_once(' ').unwrap();
        let (base_victims, owner) = (base_victims.parse().unwrap(), owner.parse().unwrap());
        VICTIMS.set(base_victims);
        let physics = Box::leak(Box::new(crate::ground_vehicle::Services {
            base_crash,
            ..GROUND_VEHICLE
        }));
        let mut f = Fixture::new(
            owner,
            &crate::services::Services {
                station_catchment,
                ..SHARED
            },
        );
        let ground = Box::leak(Box::new(GroundServices { physics }));
        let leaves = Leaves {
            ground,
            ..*f.leaves
        };
        f.leaves = Box::leak(Box::new(leaves));
        let victims = base_victims + 1; // the driver
        // `new ScriptEventVehicleCrashed(v->index, v->tile, CRASH_RV_LEVEL_CROSSING,
        // victims, v->owner)`, field by field, independent of its constructor.
        let crashed = ScriptEvent {
            r#type: ScriptEventType::ET_VEHICLE_CRASHED,
            vehicle: VEHICLE_ID,
            tile: TILE.0,
            crash_reason: CrashReason(1),
            victims,
            station: 0,
            subsidy: 0,
            industry: 0,
            town: 0,
            company: owner,
        };
        let headline = if victims == 1 {
            encoded(STR_NEWS_ROAD_VEHICLE_CRASH_DRIVER.0, &[])
        } else {
            encoded(
                STR_NEWS_ROAD_VEHICLE_CRASH.0,
                &[Param::N(u64::from(victims))],
            )
        };
        let expected = Box::leak(Box::new(vec![
            Effect::AiNewEvent(owner, crashed),
            Effect::GameNewEvent(crashed),
            Effect::News {
                headline,
                r#type: if owner == LOCAL {
                    NewsType::Accident
                } else {
                    NewsType::AccidentOther
                },
                style: NewsStyle::Thin,
                flags: NewsFlags::new(&[NewsFlag::NoTransparency, NewsFlag::Shaded]),
                ref1: NewsReference::Tile(TILE),
                ref2: NewsReference::None,
                data: None,
                advice_type: AdviceType::Invalid,
            },
        ]));
        EXPECTED.set(expected);
        take();
        let part = f.part();
        f.game().road_veh_crash(part);
    }

    extern "C" fn not_reached() {
        std::process::exit(NOT_REACHED_EXIT);
    }
    extern "C" fn no_assertion(_: bool) {}
    const NOT_REACHED_EXIT: i32 = 41;

    /// `RoadVehicle::UpdateSpeed`: an acceleration model other than `AM_ORIGINAL` and
    /// `AM_REALISTIC` takes the switch's `default: NOT_REACHED()`. The native unreachable
    /// path ends the process, so the call runs in a child whose `NOT_REACHED` service
    /// exits with a code the parent checks.
    #[test]
    fn update_speed_of_an_unknown_acceleration_model_takes_the_native_unreachable_path() {
        const NAME: &str = "road::tests::controller::update_speed_of_an_unknown_acceleration_model_takes_the_native_unreachable_path";
        const CHILD: &str = "OPENTTD_ROAD_UPDATE_SPEED_CHILD";
        static MODEL: u8 = 2;
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", NAME])
                .env(CHILD, "1")
                .env(crate::services_assert::ISOLATED_CHILD, "1")
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(NOT_REACHED_EXIT));
            return;
        }
        let settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
            roadveh_acceleration_model: &raw const MODEL,
            ..crate::settings_type::SettingAddresses::NULL
        }));
        let shared = crate::services::Services {
            settings,
            not_reached,
            map: crate::map_func::fixture::statics(),
            ..SHARED
        };
        let mut f = Fixture::new(LOCAL, &shared);
        // SAFETY: `Fixture::new` leaks the shared table, so it lives for the process, and
        // its map statics belong to this thread, which runs every assertion in the child.
        unsafe { crate::services_assert::bind_test_policy(no_assertion, &*f.leaves.shared) };
        let part = f.part();
        f.game().update_speed(part);
    }

    extern "C" fn native_assert(condition: bool) {
        assert!(!condition);
        std::process::exit(ASSERTION_EXIT);
    }
    const ASSERTION_EXIT: i32 = 42;

    /// `RoadVehUpdateCache` asserts `v->type == VEH_ROAD`, then `v->IsFrontEngine()`,
    /// where the native build asserts. A non-front part fails the second: the native
    /// assertion handler, which ends the child here, gets the failed condition before
    /// any `NewGRF` cache or length is touched.
    #[test]
    fn road_veh_update_cache_of_a_part_asserts_that_it_is_the_front_engine() {
        const NAME: &str = "road::tests::controller::road_veh_update_cache_of_a_part_asserts_that_it_is_the_front_engine";
        const CHILD: &str = "OPENTTD_ROAD_UPDATE_CACHE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", NAME])
                .env(CHILD, "1")
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(ASSERTION_EXIT));
            return;
        }
        let movement = Box::leak(Box::new(crate::movement::MovementServices {
            native_assert,
            ..crate::test_tables::MOVEMENT
        }));
        let mut f = Fixture::new(LOCAL, &crate::services::Services { movement, ..SHARED });
        let vehicle = Box::leak(Box::new(crate::vehicle::VehicleServices {
            assertions: true,
            newgrf,
            ..VEHICLE
        }));
        f.leaves = Box::leak(Box::new(Leaves {
            vehicle,
            ..*f.leaves
        }));
        // SAFETY: The fixture's record; a part has no front-engine bit.
        unsafe { (*f.record).subtype = 0 };
        let part = f.part();
        f.game().road_veh_update_cache(part, false);
    }

    /// `RoadVehUpdateCache` asserts `u->First() == v` for each part. `First()` returns the
    /// stored link, so a null first link fails the comparison: the native assertion
    /// handler, which ends the child here, gets the failed condition instead of a Rust
    /// panic on the null link.
    #[test]
    fn road_veh_update_cache_asserts_a_null_first_link_through_the_native_handler() {
        const NAME: &str = "road::tests::controller::road_veh_update_cache_asserts_a_null_first_link_through_the_native_handler";
        const CHILD: &str = "OPENTTD_ROAD_UPDATE_CACHE_FIRST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", NAME])
                .env(CHILD, "1")
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(ASSERTION_EXIT));
            return;
        }
        unsafe extern "C" fn cache(
            _: *mut crate::vehicle::OpenTTDVehicle,
        ) -> *mut crate::newgrf_vehicle::Cache {
            Box::leak(Box::default())
        }
        extern "C" fn newgrf_with_cache() -> *const crate::newgrf_vehicle::Services {
            let mut table = crate::newgrf_vehicle::TEST_TABLE;
            table.id = id;
            table.roadtype = roadtype;
            table.cache = cache;
            Box::leak(Box::new(table))
        }
        let movement = Box::leak(Box::new(crate::movement::MovementServices {
            native_assert,
            ..crate::test_tables::MOVEMENT
        }));
        let mut f = Fixture::new(LOCAL, &crate::services::Services { movement, ..SHARED });
        let vehicle = Box::leak(Box::new(crate::vehicle::VehicleServices {
            assertions: true,
            newgrf: newgrf_with_cache,
            ..VEHICLE
        }));
        f.leaves = Box::leak(Box::new(Leaves {
            vehicle,
            ..*f.leaves
        }));
        // SAFETY: The fixture's front record; its first link is broken.
        unsafe { (*f.record).first = std::ptr::null_mut() };
        let part = f.part();
        f.game().road_veh_update_cache(part, false);
    }

    /// `IsStraightRoadTrackdir` is `(dir & 0x06) == 0` over the sixteen trackdirs, and
    /// asserts `IsValidTrackdirForRoadVehicle` first.
    #[test]
    fn is_straight_road_trackdir_matches_the_original_over_every_trackdir() {
        for dir in 0..16_u8 {
            let failed = Cell::new(false);
            let straight = crate::track_func::is_straight_road_trackdir(Trackdir(dir), |ok| {
                failed.set(failed.get() || !ok);
            });
            assert!(!failed.get());
            assert_eq!(straight, matches!(dir, 0 | 1 | 8 | 9), "trackdir {dir}");
        }
        let failed = Cell::new(false);
        crate::track_func::is_straight_road_trackdir(Trackdir::INVALID_TRACKDIR, |ok| {
            failed.set(failed.get() || !ok);
        });
        assert!(failed.get());
    }

    /// The owners a scheduled bay arrival reaches: the bay's road stop, the station's
    /// loading queue, and the vehicle's orders and consist.
    struct Bay {
        stop: *mut crate::road_stop::State,
        station: *mut crate::station_service::Service,
        orders: *mut crate::orders::VehicleOrders,
        consist: *mut crate::orders::Consist,
    }
    thread_local! {
        static BAY: Cell<*const Bay> = const { Cell::new(std::ptr::null()) };
        static RECORD: Cell<*mut crate::vehicle_base::VehicleRecord> =
            const { Cell::new(std::ptr::null_mut()) };
        /// The bay's status when `BeginLoading` marks the arrival dirty.
        static LOADING_STATUS: Cell<Option<u8>> = const { Cell::new(None) };
        static ROAD_SIDE: Cell<u8> = const { Cell::new(0) };
    }
    /// The opaque native shells of the vehicle and the bay's `RoadStop`.
    static SHELL: u8 = 0;
    static STOP_SHELL: u8 = 0;
    fn bay() -> &'static Bay {
        // SAFETY: Set for the test's duration, before any entry.
        unsafe { &*BAY.get() }
    }
    fn shell() -> *mut std::ffi::c_void {
        (&raw const SHELL).cast_mut().cast()
    }
    extern "C" fn record_from_shell(
        shell_ptr: *mut std::ffi::c_void,
    ) -> *mut crate::vehicle::OpenTTDVehicle {
        assert_eq!(shell_ptr, shell());
        RECORD.get().cast()
    }
    unsafe extern "C" fn order_vehicle(
        shell_ptr: *mut std::ffi::c_void,
    ) -> *mut crate::orders::VehicleOrders {
        assert_eq!(shell_ptr, shell());
        bay().orders
    }
    unsafe extern "C" fn order_consist(
        shell_ptr: *mut std::ffi::c_void,
    ) -> *mut crate::orders::Consist {
        assert_eq!(shell_ptr, shell());
        bay().consist
    }
    unsafe extern "C" fn road_vehicle_type(_: *mut std::ffi::c_void) -> u64 {
        u64::from(crate::vehicle_type::VehicleType::VEH_ROAD.0)
    }
    extern "C" fn bay_primary(tile: u32, kind: RoadStopType) -> *mut crate::road_stop::Stop {
        assert_eq!((tile, kind), (TILE.0, RoadStopType::Bus));
        (&raw const STOP_SHELL).cast_mut().cast()
    }
    extern "C" fn bay_tile(_: *const crate::road_stop::Stop) -> u32 {
        TILE.0
    }
    extern "C" fn bay_state(rs: *const crate::road_stop::Stop) -> *mut crate::road_stop::State {
        assert_eq!(rs, (&raw const STOP_SHELL).cast());
        bay().stop
    }
    extern "C" fn bay_stops() -> *const crate::road_stop::RoadStopServices {
        thread_local! {
            static STOPS: crate::road_stop::RoadStopServices = crate::road_stop::RoadStopServices {
                vehicle: std::ptr::null(),
                primary: bay_primary,
                head: crate::test_tables::unreachable2,
                is_bus,
                next: crate::test_tables::unreachable1,
                tile: bay_tile,
                state: bay_state,
                pool_next: crate::test_tables::unreachable1,
            };
        }
        STOPS.with(std::ptr::from_ref)
    }
    extern "C" fn by_tile(tile: u32) -> *mut crate::station::Station {
        assert_eq!(tile, TILE.0);
        STATION_HANDLE.st
    }
    extern "C" fn tick_counter() -> u64 {
        0
    }
    extern "C" fn loading_station(id: u16) -> crate::station_service::LoadingStation {
        assert_eq!(id, STATION.0);
        crate::station_service::LoadingStation {
            native: STATION_HANDLE.st,
            base: STATION_HANDLE.base(),
            state: bay().station,
        }
    }
    extern "C" fn set_loading_finished(v: VehicleRef, finished: bool) {
        assert_eq!((v.0.as_ptr(), finished), (RECORD.get().cast(), false));
    }
    extern "C" fn set_load_ticks(v: VehicleRef, ticks: u16) {
        assert_eq!((v.0.as_ptr(), ticks), (RECORD.get().cast(), 1));
    }
    extern "C" fn create_payment(_: VehicleRef) -> *mut crate::cargo_payment::NativePayment {
        std::ptr::dangling_mut()
    }
    extern "C" fn with_next_stations(
        _: VehicleRef,
        _: *mut std::ffi::c_void,
        _: crate::station_service::Scope,
    ) {
    }
    /// No last loading station (`INVALID_STATION`): no link statistics.
    extern "C" fn prediction(_: VehicleRef) -> crate::vehicle::visits::Prediction {
        crate::vehicle::visits::Prediction {
            loading_tick: 0,
            last_loading: u16::MAX,
            last_station: STATION.0,
            ticks: 0,
        }
    }
    /// `BeginLoading`'s arrival invalidation, after its orders and unload preparation.
    extern "C" fn arrival_dirty(v: VehicleRef) {
        assert_eq!(v.0.as_ptr(), RECORD.get().cast());
        // SAFETY: The bay's live road stop owner; one byte read.
        LOADING_STATUS.set(Some(unsafe { (*bay().stop).status }));
    }
    extern "C" fn mark_dirty(_: VehicleRef) {}
    /// Failed native assertions in the isolated child.
    static FAILED_CHECKS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    extern "C" fn record_check(condition: bool) {
        if !condition {
            FAILED_CHECKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    unsafe extern "C" fn no_cargo(_: *const crate::station::BaseStation) -> u64 {
        0
    }
    extern "C" fn no_road_triggers(_: *mut crate::station::BaseStation, _: u8) -> bool {
        false
    }

    /// `IndividualRoadVehicleController` at the stopping frame of a bus bay
    /// (`roadveh_cmd.cpp:1520-1540`) on a scheduled arrival at the bay's station: it frees
    /// the bay's entrance, sets the entered-stop bit and the last visited station, and
    /// `BeginLoading` runs with the entrance free. The controller returns before the bay
    /// tile marks the entrance busy again.
    #[test]
    fn scheduled_bay_arrival_frees_the_entrance_before_loading_begins() {
        /* The ground vehicle flag accessors check the vehicle type through the
         * process-wide native policy, bound in a child process. */
        if crate::services_assert::isolate_test_check(
            "road::tests::controller::scheduled_bay_arrival_frees_the_entrance_before_loading_begins",
            record_check,
        ) {
            return;
        }
        let _types = DefaultRoadTypes::new();
        let _map = {
            let mut base: Vec<_> = (0..4096)
                .map(|_| crate::map_func::BaseTileLayout {
                    kind: 0,
                    height: 0,
                    m2: 0,
                    m1: 0,
                    m3: 0,
                    m4: 0,
                    m5: 0,
                })
                .collect();
            let mut extended: Vec<_> = (0..4096)
                .map(|_| crate::map_func::ExtendedTileLayout {
                    m6: 0,
                    m7: 0,
                    m8: 0,
                })
                .collect();
            /* The bus bay of station 55: GFX 0 is a bay, not a drive-through stop. */
            let bay_tile = &mut base[TILE.0 as usize];
            bay_tile.kind = crate::tile_type::TileType::MP_STATION.0 << 4;
            bay_tile.m2 = STATION.0;
            extended[TILE.0 as usize].m6 = crate::station_type::StationType::Bus.0 << 3;
            crate::map_func::fixture::set(
                crate::map_func::TileTypeAllocation {
                    base: base.as_ptr(),
                    extended: extended.as_ptr(),
                    count: 4096,
                    assertions: false,
                },
                6,
            );
            (base, extended)
        };
        let mut stations = crate::newgrf_airport::Services::with_station_id(station_id);
        stations.by_tile = by_tile;
        let stations = Box::leak(Box::new(stations));
        // SAFETY: Serialized by the game-state test lock; restored below.
        let previous_stations =
            unsafe { crate::newgrf_airport::openttd_rust_airport_exchange(stations) };
        let resolver = Box::leak(Box::new(crate::newgrf_roadstop::Services {
            cached_cargo: no_cargo,
            ..crate::test_tables::ROAD_STOPS
        }));
        let animation = Box::leak(Box::new(crate::animated_tile::Services {
            roadstop_resolver: resolver,
            road_triggers: no_road_triggers,
            ..crate::test_tables::ANIMATION
        }));
        // SAFETY: As above.
        let previous_animation =
            unsafe { crate::animated_tile::openttd_rust_animation_exchange(animation) };

        let stop = crate::road_stop::openttd_rust_road_stop_new();
        // SAFETY: The fresh owner.
        unsafe { (*stop).status = 0xFF };
        let bay_owners = Bay {
            stop,
            station: crate::station_service::openttd_rust_station_service_new(),
            orders: crate::orders::openttd_rust_vehicle_orders_new(),
            consist: crate::orders::openttd_rust_consist_new(),
        };
        BAY.set(&raw const bay_owners);
        // SAFETY: The fresh order owner; its current order starts here.
        unsafe {
            (*bay_owners.orders).current.write(crate::orders::Order {
                kind: OrderType::OT_GOTO_STATION.0,
                flags: 0,
                destination: STATION.0,
                refit: 254,
                wait: 0,
                travel: 0,
                speed: u16::MAX,
            });
        }

        let settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
            road_side: ROAD_SIDE.with(Cell::as_ptr).cast_const(),
            ..crate::settings_type::SettingAddresses::NULL
        }));
        let shared = Box::leak(Box::new(crate::services::Services {
            map: crate::map_func::fixture::statics(),
            settings,
            tick_counter,
            ..SHARED
        }));
        let mut f = Fixture::new(LOCAL, shared);
        let vehicles: &'static crate::vehicle::VehicleServices =
            Box::leak(Box::new(crate::vehicle::VehicleServices {
                record_from_shell,
                maintenance: Box::leak(Box::new(crate::vehicle::maintenance::Services {
                    mark_dirty,
                    ..crate::test_tables::MAINTENANCE
                })),
                ..*f.game().vs
            }));
        let orders = Box::leak(Box::new(crate::orders::Leaves {
            vehicle: order_vehicle,
            consist: order_consist,
            vehicle_type: road_vehicle_type,
            base: vehicles,
            ..crate::test_tables::ORDERS
        }));
        let common = Box::leak(Box::new(crate::vehicle_transactions::CommonServices {
            vehicles,
            orders,
            ..crate::test_tables::VEHICLE_COMMON
        }));
        let refit = Box::leak(Box::new(crate::vehicle_transactions::refit::Services {
            common,
            ..crate::test_tables::REFIT
        }));
        let station = Box::leak(Box::new(crate::station_service::World {
            station: loading_station,
            ..crate::test_tables::STATION_WORLD
        }));
        let loading = Box::leak(Box::new(crate::station_service::Loading {
            set_loading_finished,
            set_load_ticks,
            with_next_stations,
            create_payment,
            ..crate::test_tables::STATION_LOADING
        }));
        thread_local! {
            static VISITS: Cell<*const crate::vehicle::visits::Services> =
                const { Cell::new(std::ptr::null()) };
        }
        VISITS.set(Box::leak(Box::new(crate::vehicle::visits::Services {
            shared: f.game().ss,
            station,
            loading,
            refit,
            prediction,
            arrival_dirty,
            ..crate::test_tables::VISITS
        })));
        extern "C" fn visits() -> *const crate::vehicle::visits::Services {
            VISITS.get()
        }
        f.leaves = Box::leak(Box::new(Leaves {
            vehicle: Box::leak(Box::new(crate::vehicle::VehicleServices {
                visits,
                ..*vehicles
            })),
            shared: Box::leak(Box::new(crate::services::Services {
                road_stops: bay_stops,
                ..*f.game().ss
            })),
            ..*f.leaves
        }));

        /* A bus at the bay's stopping frame (state 32, frame 20), heading NE onto the
         * pinned drive data of frame 21, (5, 7). The station had a bus before: no news. */
        RECORD.set(f.record);
        // SAFETY: The fixture's live record.
        unsafe {
            let r = &mut *f.record;
            r.shell = (&raw const SHELL).cast_mut().cast();
            r.orders = bay_owners.orders;
            r.x_pos = 6;
            r.y_pos = 7;
            r.direction = crate::direction_type::Direction::DIR_NE;
        }
        f.state.fields.state = RVSB_IN_ROAD_STOP;
        f.state.fields.frame = 20;
        BUS.set(true);
        ROADTYPE.set(RoadType::ROADTYPE_ROAD.0);
        HAD.set(StationHadVehicleOfType::HVOT_BUS.0);
        LOADING_STATUS.set(None);
        take();

        let part = f.part();
        let moved = f.game().individual_road_vehicle_controller(part, None);
        assert_eq!(FAILED_CHECKS.load(std::sync::atomic::Ordering::Relaxed), 0);

        assert!(!moved, "the arrival returns false");
        assert_eq!(
            LOADING_STATUS.get(),
            Some(0x7F),
            "BeginLoading runs with the entrance freed"
        );
        // SAFETY: The live owners.
        unsafe {
            assert_eq!(
                (*stop).status,
                0x7F,
                "the bay tile does not mark it busy again"
            );
            assert_eq!(
                f.state.fields.state,
                RVSB_IN_ROAD_STOP | (1 << RVS_ENTERED_STOP)
            );
            assert_eq!((*f.record).last_station_visited, STATION);
            assert_eq!(
                (*bay_owners.orders).current.assume_init_ref().kind & 0x0F,
                OrderType::OT_LOADING.0
            );
            assert_eq!(
                crate::station_service::openttd_rust_station_queue_size(bay_owners.station),
                1,
                "PrepareUnload queued the vehicle at the station"
            );
            assert_eq!(
                crate::station_service::openttd_rust_station_queue_value(
                    crate::station_service::openttd_rust_station_queue_first(bay_owners.station)
                ),
                shell()
            );
        }
        assert_eq!(take(), Vec::new());

        // SAFETY: The owners created above, each released once; the tables restored.
        unsafe {
            crate::station_service::openttd_rust_station_queue_clear(bay_owners.station);
            crate::station_service::openttd_rust_station_service_destroy(bay_owners.station);
            crate::road_stop::openttd_rust_road_stop_destroy(stop);
            crate::orders::openttd_rust_vehicle_orders_delete(bay_owners.orders);
            crate::orders::openttd_rust_consist_delete(bay_owners.consist);
            crate::animated_tile::openttd_rust_animation_exchange(previous_animation);
            crate::newgrf_airport::openttd_rust_airport_exchange(previous_stations);
        }
        BAY.set(std::ptr::null());
    }

    thread_local! {
        /// The service search's fixture map, for the tile type service.
        static SERVICE_MAP: Cell<*const crate::map_func::BaseTileLayout> =
            const { Cell::new(std::ptr::null()) };
        /// The depot tile the search is expected to report.
        static SERVICE_DEPOT: Cell<u32> = const { Cell::new(0) };
        /// The road speed reads of the search's followed tiles since the last reset.
        static FOLLOWED: Cell<u32> = const { Cell::new(0) };
        /// The orders marked dirty by the service handler since the last reset.
        static DEPOT_ORDERS: Cell<u32> = const { Cell::new(0) };
        static SERVICE_DEFAULTS: Cell<crate::settings_table::VehicleDefaults> =
            const {
                Cell::new(crate::settings_table::VehicleDefaults {
                    servint_ispercent: false,
                    servint_trains: 0,
                    servint_roadveh: 1,
                    servint_aircraft: 0,
                    servint_ships: 0,
                })
            };
        static SERVICE_PENALTY: Cell<u32> = const { Cell::new(0) };
        static SERVICE_ORDERS: Cell<*mut crate::orders::VehicleOrders> =
            const { Cell::new(std::ptr::null_mut()) };
    }
    /// The vehicle's order owner, for the search's current order speed limit.
    unsafe extern "C" fn service_order_vehicle(
        shell_ptr: *mut std::ffi::c_void,
    ) -> *mut crate::orders::VehicleOrders {
        assert_eq!(shell_ptr, shell());
        SERVICE_ORDERS.get()
    }
    extern "C" fn service_tile_type(tile: u32) -> u8 {
        // SAFETY: The test's live 4096-tile map; `tile` is on it.
        unsafe { (*SERVICE_MAP.get().add(tile as usize)).kind >> 4 }
    }
    extern "C" fn service_may_have_road(_: u32) -> bool {
        true
    }
    extern "C" fn service_depot_index(tile: u32) -> u16 {
        assert_eq!(tile, SERVICE_DEPOT.get());
        55
    }
    extern "C" fn service_road_kind(_: VehicleRef) -> crate::road_type::RoadTramType {
        crate::road_type::RoadTramType::RTT_ROAD
    }
    extern "C" fn service_road_compatible(_: VehicleRef) -> u64 {
        1
    }
    /// `GetRoadTypeInfo(rt)->max_speed` of a followed tile: counted, no limit.
    extern "C" fn service_road_type_speed(_: crate::services::RoadType) -> u16 {
        FOLLOWED.set(FOLLOWED.get() + 1);
        0
    }
    extern "C" fn service_vehicle_defaults(
        _: crate::company_type::CompanyID,
    ) -> *mut crate::settings_table::VehicleDefaults {
        SERVICE_DEFAULTS.with(Cell::as_ptr)
    }
    extern "C" fn service_settings() -> &'static crate::settings_table::Services {
        thread_local! {
            static SETTINGS: &'static crate::settings_table::Services =
                Box::leak(Box::new(crate::settings_table::Services {
                    company_vehicle_defaults: service_vehicle_defaults,
                    ..crate::test_tables::SETTINGS
                }));
        }
        SETTINGS.with(|s| *s)
    }
    extern "C" fn service_company() -> *const crate::company::Services {
        thread_local! {
            static COMPANY: crate::company::Services = crate::company::Services {
                settings: service_settings,
                ..crate::test_tables::COMPANY
            };
        }
        COMPANY.with(std::ptr::from_ref)
    }
    /// No depot order, and not loading.
    extern "C" fn service_automatic(_: VehicleRef) -> crate::vehicle::maintenance::Automatic {
        crate::vehicle::maintenance::Automatic {
            has_depot_order: false,
            order_type: OrderType::OT_NOTHING,
        }
    }
    /// Serviced on day 0 with no interval, now day 1, servicing allowed and breakdowns
    /// off (`breakdowns: 0`): due, since the zero interval has passed.
    extern "C" fn service_interval(_: VehicleRef) -> crate::vehicle::maintenance::Interval {
        crate::vehicle::maintenance::Interval {
            company: std::ptr::null(),
            last_service: crate::timer_game_economy::EconomyDate(0),
            now: crate::timer_game_economy::EconomyDate(1),
            interval: 0,
            reliability: 0,
            percent: false,
            wallclock: false,
            no_service: false,
            breakdowns: 0,
        }
    }
    /// Each order the handler sends to the depot: a go-to-depot order for depot 55, with
    /// implicit orders suppressed.
    extern "C" fn service_start_stop_dirty(v: VehicleRef) {
        let order = v.current_order();
        assert_eq!(order.kind(), OrderType::OT_GOTO_DEPOT);
        assert_eq!(order.destination().0, 55);
        // SAFETY: The test's live ground owner of `v`.
        let flags = unsafe { (*(*v.0.as_ptr()).ground).flags };
        assert_ne!(
            flags
                & (1 << crate::ground_vehicle::GroundVehicleFlags::GVF_SUPPRESS_IMPLICIT_ORDERS.0),
            0
        );
        DEPOT_ORDERS.set(DEPOT_ORDERS.get() + 1);
    }

    /// `CheckIfRoadVehNeedsService` (`roadveh_cmd.cpp`) of a stopped bus due for service
    /// on a straight road, searching the real road YAPF over a fixture map for a depot
    /// within `maximum_go_to_depot_penalty`. The vehicle at tile 2080 faces north-east
    /// (`TRACKDIR_X_NE`); the depot's south-west entrance faces it. With the depot one
    /// tile on, at cost 100, a penalty of 99 orders nothing and 100 orders the depot.
    /// With the depot three tiles on, a penalty of 150 ends the search on the second
    /// road tile (cost 200) before following it, where an unlimited search follows on
    /// to the depot: the road speed of each followed tile counts the follows.
    #[test]
    fn service_search_orders_the_depot_only_within_the_configured_penalty() {
        /* The ground vehicle accessors check the vehicle type through the process-wide
         * native policy, and the search takes the process-wide YAPF scratch: run in a
         * child process. */
        if crate::services_assert::isolate_test_check(
            "road::tests::controller::service_search_orders_the_depot_only_within_the_configured_penalty",
            record_check,
        ) {
            return;
        }
        let _types = DefaultRoadTypes::new();
        const VEHICLE_TILE: u32 = 32 + 32 * 64;
        let road = |kind: &mut crate::map_func::BaseTileLayout, m5: u8| {
            kind.kind = crate::tile_type::TileType::MP_ROAD.0 << 4;
            kind.m5 = m5;
        };
        let depot = (crate::road_map::ROAD_TILE_DEPOT << 6)
            | crate::direction_type::DiagDirection::DIAGDIR_SW.0;
        let mut base: Vec<_> = (0..4096)
            .map(|_| crate::map_func::BaseTileLayout {
                kind: 0,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            })
            .collect();
        let extended: Vec<_> = (0..4096)
            .map(|_| crate::map_func::ExtendedTileLayout {
                m6: 0,
                m7: 0,
                m8: 0,
            })
            .collect();
        road(&mut base[VEHICLE_TILE as usize], crate::road_type::ROAD_X);
        road(&mut base[VEHICLE_TILE as usize - 1], depot);
        let install = |base: &[crate::map_func::BaseTileLayout]| {
            SERVICE_MAP.set(base.as_ptr());
            crate::map_func::fixture::set(
                crate::map_func::TileTypeAllocation {
                    base: base.as_ptr(),
                    extended: extended.as_ptr(),
                    count: 4096,
                    assertions: true,
                },
                6,
            );
        };
        install(&base);

        /* The pf.yapf road settings the search reads, at their defaults. */
        let yapf = [10_000_u32, 200, 300, 800, 800, 1500, 100];
        let settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
            maximum_go_to_depot_penalty: SERVICE_PENALTY.with(Cell::as_ptr).cast_const(),
            max_search_nodes: &raw const yapf[0],
            road_slope_penalty: &raw const yapf[1],
            road_crossing_penalty: &raw const yapf[2],
            road_stop_penalty: &raw const yapf[3],
            road_stop_occupied_penalty: &raw const yapf[4],
            road_stop_bay_occupied_penalty: &raw const yapf[5],
            road_curve_penalty: &raw const yapf[6],
            ..crate::settings_type::SettingAddresses::NULL
        }));
        let movement = Box::leak(Box::new(crate::movement::MovementServices {
            native_assert: record_check,
            road_kind: service_road_kind,
            road_compatible: service_road_compatible,
            road_type_speed: service_road_type_speed,
            ..crate::test_tables::MOVEMENT
        }));
        let shared: &'static crate::services::Services =
            Box::leak(Box::new(crate::services::Services {
                map: crate::map_func::fixture::statics(),
                settings,
                movement,
                tile_type: service_tile_type,
                may_have_road: service_may_have_road,
                depot_index: service_depot_index,
                road_stops,
                ..SHARED
            }));
        movement.shared = shared;
        let mut f = Fixture::new(0, shared);
        let commands = Box::leak(Box::new(crate::services::CommandServices {
            company: service_company,
            ..crate::test_tables::COMMANDS
        }));
        let transactions = Box::leak(Box::new(crate::fleet::transactions::Services {
            commands,
            ..crate::test_tables::FLEET_TRANSACTIONS
        }));
        let maintenance = Box::leak(Box::new(crate::vehicle::maintenance::Services {
            transaction: transactions,
            automatic: service_automatic,
            interval: service_interval,
            ..crate::test_tables::MAINTENANCE
        }));
        movement.vehicle = f.leaves.vehicle;
        /* IsChainInDepot reads the map through the ground services' shared table. */
        let physics = Box::leak(Box::new(crate::ground_vehicle::Services {
            capabilities: Box::leak(Box::new(
                crate::engine_capabilities::test_table_with_shared(shared),
            )),
            ..GROUND_VEHICLE
        }));
        f.leaves = Box::leak(Box::new(Leaves {
            ground: Box::leak(Box::new(GroundServices { physics })),
            vehicle: Box::leak(Box::new(crate::vehicle::VehicleServices {
                maintenance,
                start_stop_dirty: service_start_stop_dirty,
                ..*f.game().vs
            })),
            ..*f.leaves
        }));
        let orders = crate::orders::openttd_rust_vehicle_orders_new();
        SERVICE_ORDERS.set(orders);
        movement.orders = Box::leak(Box::new(crate::orders::Leaves {
            vehicle: service_order_vehicle,
            ..crate::test_tables::ORDERS
        }));
        // SAFETY: The fixture's live record and its fresh order owner.
        unsafe {
            (*f.record).orders = orders;
            (*f.record).shell = shell().cast();
        }
        BUS.set(true);
        ROADTYPE.set(RoadType::ROADTYPE_ROAD.0);

        /* A stopped vehicle with no order heading north-east, due for service. */
        let day = |f: &mut Fixture, penalty: u32| {
            SERVICE_PENALTY.set(penalty);
            FOLLOWED.set(0);
            DEPOT_ORDERS.set(0);
            // SAFETY: The fixture's live record and owners.
            unsafe {
                let r = &mut *f.record;
                r.tile = TileIndex(VEHICLE_TILE);
                r.dest_tile = TileIndex(42);
                r.cur_speed = 0;
                (*r.ground).flags = 0;
                (*orders).current.write(crate::orders::Order {
                    kind: OrderType::OT_NOTHING.0,
                    flags: 0,
                    destination: 0,
                    refit: 0,
                    wait: 0,
                    travel: 0,
                    speed: 0,
                });
            }
            f.state.fields.state = Trackdir::TRACKDIR_X_NE.0;
            let part = f.part();
            f.game().check_if_road_veh_needs_service(part);
            // SAFETY: The fixture's live record.
            unsafe { (*f.record).dest_tile.0 }
        };

        /* The depot one tile on costs 100. */
        SERVICE_DEPOT.set(VEHICLE_TILE - 1);
        assert_eq!(day(&mut f, 99), 42, "a penalty below the depot's cost");
        assert_eq!((DEPOT_ORDERS.get(), FOLLOWED.get()), (0, 0));
        assert_eq!(day(&mut f, 100), VEHICLE_TILE - 1, "a penalty covering it");
        assert_eq!((DEPOT_ORDERS.get(), FOLLOWED.get()), (1, 0));

        /* The depot three tiles on costs 300. */
        road(
            &mut base[VEHICLE_TILE as usize - 1],
            crate::road_type::ROAD_X,
        );
        road(
            &mut base[VEHICLE_TILE as usize - 2],
            crate::road_type::ROAD_X,
        );
        road(&mut base[VEHICLE_TILE as usize - 3], depot);
        install(&base);
        SERVICE_DEPOT.set(VEHICLE_TILE - 3);
        assert_eq!(day(&mut f, 150), 42, "the search stops at its limit");
        assert_eq!(
            (DEPOT_ORDERS.get(), FOLLOWED.get()),
            (0, 1),
            "one tile is followed before the second exceeds the penalty"
        );
        FOLLOWED.set(0);
        let part = f.part();
        let unlimited = f.game().find_closest_road_depot(part, 0);
        assert_eq!(
            (unlimited.tile.0, unlimited.best_length),
            (VEHICLE_TILE - 3, 300),
            "an unlimited search reaches the depot"
        );
        assert!(
            FOLLOWED.get() > 1,
            "an unlimited search follows past the limited one's last tile"
        );
        assert_eq!(FAILED_CHECKS.load(std::sync::atomic::Ordering::Relaxed), 0);

        // SAFETY: The owner created above, released once; the record forgets it first.
        unsafe {
            (*f.record).orders = std::ptr::null_mut();
            crate::orders::openttd_rust_vehicle_orders_delete(orders);
        }
    }
}
