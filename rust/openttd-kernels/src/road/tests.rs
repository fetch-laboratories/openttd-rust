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
    use crate::test_tables::{GROUND_VEHICLE, ROAD_LEAVES, SHARED, VEHICLE, unreachable1};
    use std::cell::Cell;

    thread_local! {
        static BUS: Cell<bool> = const { Cell::new(true) };
        static ROADTYPE: Cell<u8> = const { Cell::new(0) };
        static HAD: Cell<u8> = const { Cell::new(0) };
        /// The `had_vehicle_of_type` stores since the last reset.
        static STORES: Cell<u32> = const { Cell::new(0) };
        static VICTIMS: Cell<u32> = const { Cell::new(0) };
        static EXPECTED: Cell<*const Vec<Effect>> = const { Cell::new(std::ptr::null()) };
    }
    const VEHICLE_ID: u32 = 17;
    const STATION: StationID = StationID(55);
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
            static TABLE: *const crate::newgrf_vehicle::Services = {
                let table = Box::leak(Box::new(std::mem::MaybeUninit::<
                    crate::newgrf_vehicle::Services,
                >::uninit()));
                // SAFETY: Only `id` and `roadtype` are read from this table.
                unsafe {
                    (&raw mut (*table.as_mut_ptr()).id).write(id);
                    (&raw mut (*table.as_mut_ptr()).roadtype).write(roadtype);
                }
                table.as_ptr()
            };
        }
        TABLE.with(|t| *t)
    }
    extern "C" fn is_bus(_: VehicleRef) -> bool {
        BUS.get()
    }
    extern "C" fn road_stops() -> *const crate::road_stop::RoadStopServices {
        thread_local! {
            static STOPS: crate::road_stop::RoadStopServices = crate::road_stop::RoadStopServices {
                vehicle: std::ptr::null(),
                ground: std::ptr::null(),
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
    extern "C" fn had_vehicle_of_type(station: u16) -> u8 {
        assert_eq!(station, STATION.0);
        HAD.get()
    }
    /// `st->had_vehicle_of_type |= bit`, before any news or event.
    extern "C" fn add_vehicle_of_type(station: u16, bit: u8) {
        assert_eq!(station, STATION.0);
        assert_eq!(take(), Vec::new(), "the station bit is stored first");
        HAD.set(HAD.get() | bit);
        STORES.set(STORES.get() + 1);
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
                has_articulated: unreachable1,
            }));
            let shared = Box::leak(Box::new(crate::services::Services {
                road_stops,
                local_company,
                station_had_vehicle_of_type: had_vehicle_of_type,
                station_add_vehicle_of_type: add_vehicle_of_type,
                ..*shared
            }));
            let leaves = Box::leak(Box::new(Leaves {
                vehicle,
                ground,
                shared,
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
        take();
        for (bus, road_type, bit, headline) in cases {
            ROADTYPE.set(road_type);
            BUS.set(bus);
            for owner in [LOCAL, LOCAL + 1] {
                let mut f = Fixture::new(owner, &SHARED);
                HAD.set(0);
                STORES.set(0);
                f.game().road_veh_arrives_at(f.part(), STATION);
                assert_eq!(HAD.get(), bit.0, "bus {bus}, road type {road_type}");
                assert_eq!(STORES.get(), 1, "one store on the first arrival");
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
                STORES.set(0);
                f.game().road_veh_arrives_at(f.part(), STATION);
                assert_eq!(HAD.get(), bit.0);
                assert_eq!(STORES.get(), 0, "no store on a repeat visit");
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
        let ground = Box::leak(Box::new(GroundServices {
            physics,
            has_articulated: unreachable1,
        }));
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
            ..SHARED
        };
        let mut f = Fixture::new(LOCAL, &shared);
        // SAFETY: The leaked table outlives the child's only call.
        unsafe { crate::services_assert::openttd_rust_assert_bind(no_assertion, f.leaves.shared) };
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

    /// `IsStraightRoadTrackdir` is `(dir & 0x06) == 0` over the sixteen trackdirs, and
    /// asserts `IsValidTrackdirForRoadVehicle` first.
    #[test]
    fn is_straight_road_trackdir_matches_the_original_over_every_trackdir() {
        for dir in 0..16_u8 {
            let failed = Cell::new(false);
            let straight = crate::track_func::is_straight_road_trackdir(dir, |ok| {
                failed.set(failed.get() || !ok);
            });
            assert!(!failed.get());
            assert_eq!(straight, matches!(dir, 0 | 1 | 8 | 9), "trackdir {dir}");
        }
        let failed = Cell::new(false);
        crate::track_func::is_straight_road_trackdir(Trackdir::INVALID_TRACKDIR.0, |ok| {
            failed.set(failed.get() || !ok);
        });
        assert!(failed.get());
    }
}
