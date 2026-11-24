/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Unit tests of the train bodies against expectations read from `train_cmd.cpp`.

/// `Train::From`'s type assertion holds the handle constructor: a road vehicle fails it once,
/// a train passes it and its handle carries the record's train owner.
mod handle_tests {
    use crate::vehicle::VehicleRef;
    use crate::vehicle_type::VehicleType;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Failed native assertions seen by the isolated child.
    static FAILED: AtomicU32 = AtomicU32::new(0);
    extern "C" fn record_failure(condition: bool) {
        if !condition {
            FAILED.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn from_vehicle_asserts_the_vehicle_type_and_carries_the_train_owner() {
        if crate::services_assert::isolate_test_check(
            "train::tests::handle_tests::from_vehicle_asserts_the_vehicle_type_and_carries_the_train_owner",
            record_failure,
        ) {
            return;
        }
        let record = |kind| {
            VehicleRef::of(crate::vehicle_base::openttd_rust_vehicle_record_new(
                std::ptr::null_mut(),
                kind,
            ))
        };
        let owner = std::ptr::NonNull::<crate::train_state::TrainState>::dangling().as_ptr();
        let train = record(VehicleType::VEH_TRAIN);
        // SAFETY: The test's fresh record, which it leaks.
        unsafe { (*train.0.as_ptr()).train = owner };
        let handle = super::super::Handle::from_vehicle(train);
        assert_eq!((handle.record, handle.owner), (train.0.as_ptr(), owner));
        assert_eq!(FAILED.load(Ordering::Relaxed), 0);
        let _ = super::super::Handle::from_vehicle(record(VehicleType::VEH_ROAD));
        assert_eq!(FAILED.load(Ordering::Relaxed), 1);
    }
}

/// The bodies of `train_cmd.cpp` whose effects no scenario compares: the fatal paths, the
/// arrival news and events and the crash events, through recording tables. Each expectation
/// is read from the original body.
#[cfg(test)]
mod bodies {
    use super::super::*;
    use crate::news_func::testing::{Effect, Param, encoded, take};
    use crate::news_type::{NewsFlag, NewsFlags, NewsReference, NewsStyle};
    use crate::script_event::ScriptEventType;
    use crate::test_tables::{GROUND_VEHICLE, SHARED, TRAIN_LEAVES, VEHICLE};
    use std::cell::Cell;

    const VEHICLE_ID: u32 = 17;
    const LOCAL: u8 = 3;
    const TILE: TileIndex = TileIndex(100);
    const STATION: StationID = StationID(55);
    const REACHED: i32 = 43;
    const NOT_REACHED: i32 = 41;

    thread_local! {
        static HAD: Cell<u8> = const { Cell::new(0) };
        static ARRIVAL: Cell<*const (Vec<Effect>, Handle)> = const { Cell::new(std::ptr::null()) };
    }

    unsafe extern "C" fn id(_: *mut crate::vehicle::OpenTTDVehicle) -> u32 {
        VEHICLE_ID
    }
    extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
        thread_local! {
            static TABLE: &'static crate::newgrf_vehicle::Services = {
                let mut table = crate::newgrf_vehicle::TEST_TABLE;
                table.id = id;
                Box::leak(Box::new(table))
            };
        }
        TABLE.with(|t| std::ptr::from_ref(*t))
    }
    extern "C" fn local_company() -> u8 {
        LOCAL
    }
    extern "C" fn no_assertion(_: bool) {}
    extern "C" fn not_reached() {
        std::process::exit(NOT_REACHED);
    }
    extern "C" fn loading_station(id: u16) -> crate::station_service::LoadingStation {
        assert_eq!(id, STATION.0, "Station::Get(station)");
        crate::station_service::LoadingStation {
            native: std::ptr::dangling_mut(),
            base: std::ptr::dangling_mut(),
            state: std::ptr::null_mut(),
        }
    }
    extern "C" fn station_catchment() -> *const crate::station::CatchmentServices {
        thread_local! {
            static TABLE: &'static crate::station::CatchmentServices = Box::leak(Box::new(
                crate::station::CatchmentServices {
                    station: Box::leak(Box::new(crate::station_service::World {
                        station: loading_station,
                        ..crate::test_tables::STATION_WORLD
                    })),
                    ..crate::test_tables::CATCHMENT
                },
            ));
        }
        TABLE.with(|t| std::ptr::from_ref(*t))
    }

    /// Run the named test's body in a child with `case` in `CHILD`, requiring `code`.
    fn in_child(name: &str, case: &str, code: i32) -> Option<String> {
        const CHILD: &str = "OPENTTD_TRAIN_BODY_CHILD";
        if let Some(case) = std::env::var_os(CHILD) {
            return Some(case.into_string().unwrap());
        }
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name])
            .env(CHILD, case)
            .env(crate::services_assert::ISOLATED_CHILD, "1")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(code), "{name} case {case}");
        None
    }

    /// A train record of `owner` with `subtype`, its train and ground owners, a current
    /// order, and the tables of one entry over this thread's 16 by 16 clear map.
    struct Fixture {
        record: *mut crate::vehicle_base::VehicleRecord,
        /// The map's base tile layouts, which the fixture leaks.
        base: *mut crate::map_func::BaseTileLayout,
        leaves: &'static Leaves,
        shared: &'static Services,
    }
    impl Fixture {
        fn new(
            owner: u8,
            subtype: u8,
            shared: &Services,
            vehicle: &crate::vehicle::VehicleServices,
        ) -> Self {
            let base: Box<[crate::map_func::BaseTileLayout]> = (0..256)
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
            let extended: Box<[crate::map_func::ExtendedTileLayout]> = (0..256)
                .map(|_| crate::map_func::ExtendedTileLayout {
                    m6: 0,
                    m7: 0,
                    m8: 0,
                })
                .collect();
            let base: *mut crate::map_func::BaseTileLayout = Box::into_raw(base).cast();
            crate::map_func::fixture::set(
                crate::map_func::TileTypeAllocation {
                    base,
                    extended: Box::into_raw(extended).cast(),
                    count: 256,
                    assertions: false,
                },
                4,
            );
            let shared = Box::leak(Box::new(Services {
                map: crate::map_func::fixture::statics(),
                movement: Box::leak(Box::new(crate::movement::MovementServices {
                    native_assert: no_assertion,
                    ..crate::test_tables::MOVEMENT
                })),
                ..*shared
            }));
            let leaves = Box::leak(Box::new(Leaves {
                reservation: Box::leak(Box::new(crate::train_reservation::tests::leaves())),
                vehicle: Box::leak(Box::new(crate::vehicle::VehicleServices {
                    newgrf,
                    ..*vehicle
                })),
                ..TRAIN_LEAVES
            }));
            let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
                std::ptr::null_mut(),
                VehicleType::VEH_TRAIN,
            );
            let orders = Box::leak(Box::new(crate::orders::VehicleOrders {
                current: std::mem::MaybeUninit::new(crate::orders::Order {
                    kind: OrderType::OT_NOTHING.0,
                    flags: 0,
                    destination: 0,
                    refit: crate::cargo_type::CARGO_NO_REFIT,
                    wait: 0,
                    travel: 0,
                    speed: u16::MAX,
                }),
                orders: std::ptr::null_mut(),
                next: std::ptr::null_mut(),
                previous: std::ptr::null_mut(),
            }));
            // SAFETY: A fresh record; the test owns it and the owners it links.
            unsafe {
                (*record).train = crate::train_state::openttd_rust_train_state_new();
                (*record).ground =
                    Box::leak(Box::new(crate::ground_vehicle::GroundState::default()));
                (*record).first = record;
                (*record).owner = crate::company_type::CompanyID(owner);
                (*record).tile = TILE;
                (*record).subtype = subtype;
                (*record).orders = orders;
                (*record).direction = crate::direction_type::Direction::DIR_NE;
            }
            Self {
                record,
                base,
                leaves,
                shared,
            }
        }
        fn train(&self) -> Handle {
            Handle::from_record(self.record.cast())
        }
        fn game(&self) -> Game<'static> {
            Game::new(self.leaves, self.shared)
        }
    }
    /// `GVSF_FRONT` and `GVSF_ENGINE`.
    const FRONT_ENGINE: u8 = (1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_FRONT.0)
        | (1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_ENGINE.0);

    /// `Train::ConsistChanged` asserts `IsFrontEngine() || IsFreeWagon()` before it reads
    /// the consist: a wagon in a chain fails it, and the child ends at that assertion.
    #[test]
    fn consist_changed_of_a_wagon_in_a_chain_fails_its_first_assertion() {
        if crate::services_assert::isolate_test_exit(
            "train::tests::bodies::consist_changed_of_a_wagon_in_a_chain_fails_its_first_assertion",
        ) {
            return;
        }
        let f = Fixture::new(LOCAL, 0, &SHARED, &VEHICLE);
        f.game().consist_changed(f.train(), CCF_ARRANGE);
    }

    /// `Train::UpdateSpeed` switches on the acceleration model with `default: NOT_REACHED()`:
    /// a model other than `AM_ORIGINAL` and `AM_REALISTIC` takes the native unreachable path.
    #[test]
    fn update_speed_of_an_unknown_acceleration_model_takes_the_native_unreachable_path() {
        static MODEL: u8 = 2;
        if in_child(
            "train::tests::bodies::update_speed_of_an_unknown_acceleration_model_takes_the_native_unreachable_path",
            "model 2",
            NOT_REACHED,
        )
        .is_none()
        {
            return;
        }
        let settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
            train_acceleration_model: &raw const MODEL,
            ..crate::settings_type::SettingAddresses::NULL
        }));
        let f = Fixture::new(
            LOCAL,
            FRONT_ENGINE,
            &Services {
                settings,
                not_reached,
                ..SHARED
            },
            &VEHICLE,
        );
        // SAFETY: The fixture's shared table is leaked and its map statics belong to this
        // thread, which runs every assertion of the child.
        unsafe { crate::services_assert::bind_test_policy(no_assertion, f.shared) };
        f.game().update_speed(f.train());
    }

    /// `GetTrainStopLocation` switches on the stop location with `default: NOT_REACHED()`: an
    /// order to stop at this station whose stop location is the out-of-range `End` takes the
    /// native unreachable path, after the platform lengths.
    #[test]
    fn get_train_stop_location_of_an_unknown_stop_location_takes_the_native_unreachable_path() {
        if in_child(
            "train::tests::bodies::get_train_stop_location_of_an_unknown_stop_location_takes_the_native_unreachable_path",
            "stop location 3",
            NOT_REACHED,
        )
        .is_none()
        {
            return;
        }
        let f = Fixture::new(
            LOCAL,
            FRONT_ENGINE,
            &Services {
                station_catchment,
                not_reached,
                ..SHARED
            },
            &VEHICLE,
        );
        // SAFETY: The fixture's live record and order.
        unsafe {
            let order = (*(*f.record).orders).current.as_mut_ptr();
            (*order).kind = OrderType::OT_GOTO_STATION.0 | (OrderStopLocation::End.0 << 4);
            (*order).destination = STATION.0;
            crate::services_assert::bind_test_policy(no_assertion, f.shared);
        }
        f.game().get_train_stop_location(STATION, TILE, f.train());
    }

    /// `new ScriptEventStationFirstVehicle(st->index, v->index)`, field by field.
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
    extern "C" fn had_vehicle_of_type_field(st: *mut crate::station::Station) -> *mut u8 {
        assert_eq!(
            st,
            std::ptr::dangling_mut(),
            "the station Station::Get resolved"
        );
        assert_eq!(
            take(),
            Vec::new(),
            "the station bit is read and stored before the news"
        );
        HAD.with(Cell::as_ptr)
    }
    /// `v->BeginLoading()` follows the news, the events and the view window: its first
    /// service ends the child after checking them and the train's fields.
    extern "C" fn arrival_visits() -> *const crate::vehicle::visits::Services {
        // SAFETY: The test set the expectation before the call.
        let (expected, train) = unsafe { &*ARRIVAL.get() };
        let windows = crate::window_func::testing::take();
        let ok = take() == *expected
            && windows
                == [crate::window_func::testing::Effect::SetWindowDirty(
                    WindowClass::WC_VEHICLE_VIEW,
                    WindowNumber(VEHICLE_ID as i32),
                )]
            && train.force_proceed() == TrainForceProceeding::TFP_NONE
            && train.vehicle().last_station_visited() == STATION
            && HAD.get() & StationHadVehicleOfType::HVOT_TRAIN.0 != 0;
        std::process::exit(if ok { REACHED } else { 1 });
    }

    /// `TrainEnterStation`: the last station visited, then on a first train the station bit,
    /// the vehicle news `STR_NEWS_FIRST_TRAIN_ARRIVAL` with the station, `ArrivalCompany` for
    /// the local company and `ArrivalOther` otherwise, the owner's AI event and the game
    /// event; then `force_proceed`, the vehicle view window and `BeginLoading`. A station
    /// that had another vehicle type still reports its first train; one that had a train
    /// gets no news or event.
    #[test]
    fn train_enter_station_reports_a_first_arrival_before_loading() {
        let Some(case) = std::env::var_os("OPENTTD_TRAIN_BODY_CHILD") else {
            for case in ["3 0", "4 0", "3 4", "3 6"] {
                in_child_case(case);
            }
            return;
        };
        let case = case.into_string().unwrap();
        let (owner, had) = case.split_once(' ').unwrap();
        let (owner, had): (u8, u8) = (owner.parse().unwrap(), had.parse().unwrap());
        HAD.set(had);
        let f = Fixture::new(
            owner,
            FRONT_ENGINE,
            &Services {
                local_company,
                station_catchment,
                station_had_vehicle_of_type_field: had_vehicle_of_type_field,
                ..SHARED
            },
            &crate::vehicle::VehicleServices {
                visits: arrival_visits,
                ..VEHICLE
            },
        );
        f.train()
            .set_force_proceed(TrainForceProceeding::TFP_SIGNAL);
        let expected = if had & StationHadVehicleOfType::HVOT_TRAIN.0 == 0 {
            vec![
                Effect::News {
                    headline: encoded(
                        STR_NEWS_FIRST_TRAIN_ARRIVAL.0,
                        &[Param::N(u64::from(STATION.0))],
                    ),
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
            ]
        } else {
            Vec::new()
        };
        ARRIVAL.set(Box::leak(Box::new((expected, f.train()))));
        take();
        crate::window_func::testing::take();
        f.game().train_enter_station(f.train(), STATION);
    }
    /// Spawn one case of [`train_enter_station_reports_a_first_arrival_before_loading`].
    fn in_child_case(case: &str) {
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "train::tests::bodies::train_enter_station_reports_a_first_arrival_before_loading",
            ])
            .env("OPENTTD_TRAIN_BODY_CHILD", case)
            .env(crate::services_assert::ISOLATED_CHILD, "1")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(REACHED), "case {case}");
    }

    extern "C" fn base_crash(_: VehicleRef, flooded: bool) -> u32 {
        assert!(!flooded);
        4
    }

    thread_local! {
        static PBS: Cell<*const crate::pbs::PbsServices> = const { Cell::new(std::ptr::null()) };
        /// Each `TryReserveRailTrack` call's tile and track, with the effects recorded
        /// before it.
        static RESERVED: std::cell::RefCell<Vec<(u32, u8, Vec<Effect>)>> =
            const { std::cell::RefCell::new(Vec::new()) };
    }
    extern "C" fn pbs_services(_: *const Services) -> *const crate::pbs::PbsServices {
        PBS.with(Cell::get)
    }
    extern "C" fn try_track(tile: u32, track: u8) -> bool {
        RESERVED.with(|r| r.borrow_mut().push((tile, track, take())));
        true
    }
    extern "C" fn hidden() -> bool {
        false
    }
    /// The reservation table's `TrackBitsToTrack(v->track)`.
    extern "C" fn bits_track(_: *mut std::ffi::c_void, _: Handle, bits: u8) -> u64 {
        u64::from(crate::track_func::find_first_track(bits))
    }

    /// `TrainCrashed`: a train that is not crashed yet is crashed, then the owner's AI event
    /// and the game event report `CRASH_TRAIN` with the victims of `Crash()`; an already
    /// crashed train gets neither and counts no victims. Both then re-reserve the track
    /// under the consist: the part on a plain rail tile's `TRACK_X` reserves that track,
    /// after the events.
    #[test]
    fn train_crashed_reports_the_crash_once_and_re_reserves_the_track() {
        let physics = Box::leak(Box::new(crate::ground_vehicle::Services {
            base_crash,
            ..GROUND_VEHICLE
        }));
        for owner in [LOCAL, LOCAL + 1] {
            let mut f = Fixture::new(
                owner,
                0,
                &Services {
                    pbs_services,
                    ..SHARED
                },
                &VEHICLE,
            );
            // SAFETY: The fixture's leaked reservation table.
            let reservation = unsafe { *f.leaves.reservation };
            f.leaves = Box::leak(Box::new(Leaves {
                physics,
                reservation: Box::leak(Box::new(reservation::Leaves {
                    bits_track,
                    shared: f.shared,
                    ..reservation
                })),
                ..*f.leaves
            }));
            PBS.with(|p| {
                p.set(Box::leak(Box::new(crate::pbs::PbsServices {
                    shared: f.shared,
                    try_track,
                    show_reservation: hidden,
                    assertions: false,
                    ..crate::test_tables::PBS
                })));
            });
            // SAFETY: The fixture's map; no reference into it is live. `MP_RAILWAY` in the
            // type nibble, `RAIL_TILE_NORMAL` with `TRACK_BIT_X` in m5.
            unsafe {
                let tile = &mut *f.base.add(TILE.0 as usize);
                tile.kind = crate::tile_type::TileType::MP_RAILWAY.0 << 4;
                tile.m5 = TrackBits::TRACK_BIT_X.0;
            }
            f.train().set_track(TrackBits::TRACK_BIT_X);
            take();
            RESERVED.with(|r| r.borrow_mut().clear());
            assert_eq!(f.game().train_crashed(f.train()), 4);
            assert_eq!(f.train().crash_anim_pos(), 1);
            // `new ScriptEventVehicleCrashed(v->index, v->tile, CRASH_TRAIN, victims,
            // v->owner)`, field by field.
            let crashed = ScriptEvent {
                r#type: ScriptEventType::ET_VEHICLE_CRASHED,
                vehicle: VEHICLE_ID,
                tile: TILE.0,
                crash_reason: CrashReason(0),
                victims: 4,
                station: 0,
                subsidy: 0,
                industry: 0,
                town: 0,
                company: owner,
            };
            assert_eq!(
                RESERVED.with(|r| r.take()),
                vec![(
                    TILE.0,
                    Track::TRACK_X.0,
                    vec![
                        Effect::AiNewEvent(owner, crashed),
                        Effect::GameNewEvent(crashed)
                    ]
                )]
            );
            assert_eq!(take(), Vec::new());

            // Already crashed: no events, no victims, and the track is reserved again.
            f.train()
                .vehicle()
                .set_vehstatus(f.train().vehicle().vehstatus().set(VehState::Crashed));
            assert_eq!(f.game().train_crashed(f.train()), 0);
            assert_eq!(
                RESERVED.with(|r| r.take()),
                vec![(TILE.0, Track::TRACK_X.0, Vec::new())]
            );
            assert_eq!(take(), Vec::new());
        }
    }
}
