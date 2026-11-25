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
        other_company: 0,
        value: 0,
        text: std::ptr::null(),
        text_length: 0,
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
            f.leaves = Box::leak(Box::new(Leaves {
                physics,
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
                other_company: 0,
                value: 0,
                text: std::ptr::null(),
                text_length: 0,
            };
            assert_eq!(
                RESERVED.with(std::cell::RefCell::take),
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
                RESERVED.with(std::cell::RefCell::take),
                vec![(TILE.0, Track::TRACK_X.0, Vec::new())]
            );
            assert_eq!(take(), Vec::new());
        }
    }

    /// `CheckTrainCollision(Train *v)` against the original body: both walks, the per-
    /// vehicle overload's crash order, the victim-count headline and the rating before the
    /// sound. Each case runs in an isolated child because the tile index is process-wide.
    mod collision {
        use super::*;
        use crate::vehicle::occupancy::Links;
        use std::cell::RefCell;

        /// What the child observes, in call order.
        #[derive(Debug, PartialEq)]
        enum Call {
            /// `Vehicle::Crash` of the record, through the ground table's `base_crash`.
            Crash(*mut crate::vehicle_base::VehicleRecord),
            /// `ModifyStationRatingAround`'s station table read.
            Rating,
            /// `SndPlayVehicleFx(SND_13_TRAIN_COLLISION, v)`.
            Sound(*mut crate::vehicle_base::VehicleRecord, u16),
            /// `SetTunnelBridgeReservation(tile, true)`.
            Wormhole(u32),
        }
        thread_local! {
            static CALLS: RefCell<Vec<Call>> = const { RefCell::new(Vec::new()) };
            static LINKS: RefCell<Vec<(*mut crate::vehicle_base::VehicleRecord, *mut Links)>> =
                const { RefCell::new(Vec::new()) };
            static FAILED: Cell<u32> = const { Cell::new(0) };
        }
        extern "C" fn record_failure(ok: bool) {
            if !ok {
                FAILED.with(|f| f.set(f.get() + 1));
            }
        }
        fn call(c: Call) {
            CALLS.with(|calls| calls.borrow_mut().push(c));
        }
        /// `Vehicle::Crash` sets `VehState::Crashed` on each part; the fixture's parts are
        /// single.
        extern "C" fn base_crash(v: VehicleRef, flooded: bool) -> u32 {
            assert!(!flooded);
            call(Call::Crash(v.0.as_ptr().cast()));
            v.set_vehstatus(v.vehstatus().set(VehState::Crashed));
            4
        }
        extern "C" fn tile_links(v: VehicleRef) -> *mut Links {
            let record = v.0.as_ptr().cast();
            LINKS.with(|links| {
                let mut links = links.borrow_mut();
                if let Some(&(_, l)) = links.iter().find(|(r, _)| *r == record) {
                    return l;
                }
                let l = Box::leak(Box::new(Links {
                    next: std::ptr::null_mut(),
                    prev: std::ptr::null_mut(),
                    current: std::ptr::null_mut(),
                }));
                links.push((record, l));
                l
            })
        }
        extern "C" fn yes() -> bool {
            true
        }
        extern "C" fn map_max_y() -> u32 {
            15
        }
        extern "C" fn station_commands() -> *const crate::station_cmd::CommandServicesTable {
            call(Call::Rating);
            thread_local! {
                static TABLE: &'static crate::station_cmd::CommandServicesTable =
                    Box::leak(Box::new(crate::station_cmd::CommandServicesTable {
                        station: &crate::test_tables::STATION_WORLD,
                        ..crate::test_tables::STATION_COMMANDS
                    }));
            }
            TABLE.with(|t| std::ptr::from_ref(*t))
        }
        extern "C" fn play_sound(v: VehicleRef, sound: u16) {
            call(Call::Sound(v.0.as_ptr().cast(), sound));
        }
        extern "C" fn other_end(tile: u32) -> u32 {
            assert_eq!(tile, TUNNEL.0);
            OTHER_END.0
        }
        extern "C" fn wormhole_bits(_: u32) -> TrackBits {
            TrackBits::TRACK_BIT_NONE
        }
        extern "C" fn set_wormhole(tile: u32, reserved: bool) {
            assert!(reserved);
            call(Call::Wormhole(tile));
        }

        /// Tile (5, 5): plain rail with `TRACK_BIT_X`.
        const RAIL: TileIndex = TileIndex(5 + 5 * 16);
        /// Tile (8, 2), a rail tunnel entrance facing south-east, and its other end (8, 10).
        const TUNNEL: TileIndex = TileIndex(8 + 2 * 16);
        const OTHER_END: TileIndex = TileIndex(8 + 10 * 16);

        /// The fixture's train and the tables of the collision bodies.
        fn fixture() -> Fixture {
            let mut f = Fixture::new(
                LOCAL,
                0,
                &Services {
                    disaster_sound: yes,
                    map_max_y,
                    other_tunnel_end: other_end,
                    pbs_services,
                    station_commands,
                    station_catchment,
                    ..SHARED
                },
                &crate::vehicle::VehicleServices {
                    play_sound,
                    tile_links,
                    ..VEHICLE
                },
            );
            f.leaves = Box::leak(Box::new(Leaves {
                physics: Box::leak(Box::new(crate::ground_vehicle::Services {
                    base_crash,
                    ..GROUND_VEHICLE
                })),
                ..*f.leaves
            }));
            PBS.with(|p| {
                p.set(Box::leak(Box::new(crate::pbs::PbsServices {
                    shared: f.shared,
                    try_track,
                    wormhole_bits,
                    set_wormhole,
                    show_reservation: hidden,
                    assertions: false,
                    ..crate::test_tables::PBS
                })));
            });
            // SAFETY: The fixture's map; no reference into it is live. `RAIL` is
            // `MP_RAILWAY`, `RAIL_TILE_NORMAL` with `TRACK_BIT_X`; both tunnel ends are
            // `MP_TUNNELBRIDGE` rail tunnels (m5 bit 7 clear, transport 0), facing
            // `DIAGDIR_SE` and `DIAGDIR_NW`.
            unsafe {
                let tile = |t: TileIndex| &mut *f.base.add(t.0 as usize);
                tile(RAIL).kind = crate::tile_type::TileType::MP_RAILWAY.0 << 4;
                tile(RAIL).m5 = TrackBits::TRACK_BIT_X.0;
                for (t, dir) in [(TUNNEL, 1), (OTHER_END, 3)] {
                    tile(t).kind = crate::tile_type::TileType::MP_TUNNELBRIDGE.0 << 4;
                    tile(t).m5 = dir;
                }
            }
            f
        }

        /// A train part of the fixture's owner on `tile` at (`x`, `y`), eight units long,
        /// on `track`, inserted into the tile index.
        fn part(
            f: &Fixture,
            record: *mut crate::vehicle_base::VehicleRecord,
            tile: TileIndex,
            x: i32,
            y: i32,
            track: TrackBits,
        ) -> Handle {
            // SAFETY: The test's live record and its owners.
            unsafe {
                if (*record).train.is_null() {
                    (*record).train = crate::train_state::openttd_rust_train_state_new();
                    (*record).ground =
                        Box::leak(Box::new(crate::ground_vehicle::GroundState::default()));
                    (*record).first = record;
                    (*record).owner = crate::company_type::CompanyID(LOCAL);
                    (*record).direction = crate::direction_type::Direction::DIR_NE;
                }
                (*record).tile = tile;
                (*record).x_pos = x;
                (*record).y_pos = y;
                (*(*record).ground).cache.cached_veh_length = 8;
            }
            let h = Handle::from_record(record.cast());
            h.set_track(track);
            // SAFETY: The test's live record; the fixture's map.
            unsafe {
                crate::vehicle::occupancy::update_position(
                    f.game().vs,
                    h.vehicle(),
                    f.shared.map(),
                );
            }
            h
        }
        fn new_part(f: &Fixture, tile: TileIndex, x: i32, y: i32, track: TrackBits) -> Handle {
            part(
                f,
                crate::vehicle_base::openttd_rust_vehicle_record_new(
                    std::ptr::null_mut(),
                    VehicleType::VEH_TRAIN,
                ),
                tile,
                x,
                y,
                track,
            )
        }
        /// `STR_NEWS_TRAIN_CRASH` with `victims`, `NewsType::Accident` and the tile.
        fn crash_news(victims: u64, tile: TileIndex) -> Effect {
            Effect::News {
                headline: encoded(STR_NEWS_TRAIN_CRASH.0, &[Param::N(victims)]),
                r#type: NewsType::Accident,
                style: NewsStyle::Thin,
                flags: NewsFlags::new(&[NewsFlag::NoTransparency, NewsFlag::Shaded]),
                ref1: NewsReference::Tile(tile),
                ref2: NewsReference::None,
                data: None,
                advice_type: AdviceType::Invalid,
            }
        }

        /// Outside a wormhole the body walks the vehicles near the train to the end. Each
        /// colliding train crashes `v` first (`TrainCrashed(t)`, then `TrainCrashed(coll)`),
        /// and the second collision finds `v` already crashed; a train beyond the walk's
        /// distance is not visited. 4 + 4 + 0 + 4 victims give the headline, then the
        /// rating change and then the sound.
        #[test]
        fn near_walk_crashes_each_colliding_train_and_reports_the_victims() {
            if crate::services_assert::isolate_test_check(
                "train::tests::bodies::collision::near_walk_crashes_each_colliding_train_and_reports_the_victims",
                record_failure,
            ) {
                return;
            }
            let f = fixture();
            let v = part(&f, f.record, RAIL, 88, 88, TrackBits::TRACK_BIT_X);
            let first = new_part(&f, RAIL, 90, 88, TrackBits::TRACK_BIT_X);
            let second = new_part(&f, RAIL, 88, 86, TrackBits::TRACK_BIT_X);
            let _far = new_part(
                &f,
                TileIndex(12 + 12 * 16),
                200,
                200,
                TrackBits::TRACK_BIT_X,
            );
            take();
            CALLS.with(|c| c.borrow_mut().clear());
            assert!(f.game().check_train_collision(v));
            let calls = CALLS.with(RefCell::take);
            let crashes: Vec<_> = calls
                .iter()
                .filter_map(|c| match c {
                    Call::Crash(r) => Some(*r),
                    _ => None,
                })
                .collect();
            assert_eq!(crashes.len(), 3, "{calls:?}");
            assert_eq!(crashes[0], v.record.cast());
            let mut victims_hit = crashes[1..].to_vec();
            victims_hit.sort();
            let mut expected = vec![first.record.cast(), second.record.cast()];
            expected.sort();
            assert_eq!(victims_hit, expected);
            assert_eq!(
                &calls[3..],
                [
                    Call::Rating,
                    Call::Sound(
                        v.record.cast(),
                        SoundID::from(SoundFx::SND_13_TRAIN_COLLISION).0
                    )
                ]
            );
            let news: Vec<_> = take()
                .into_iter()
                .filter(|e| matches!(e, Effect::News { .. }))
                .collect();
            assert_eq!(news, vec![crash_news(12, RAIL)]);
            assert_eq!(FAILED.with(Cell::get), 0);
        }

        /// In a wormhole the body walks the vehicles on the train's tile and then those on
        /// the other end, each to the end: the train itself is skipped as its own wagon,
        /// the part on the entrance crashes before the one on the other end. Each
        /// `TrainCrashed` re-reserves its part's wormhole, including the second one of
        /// `v`, which finds it already crashed.
        #[test]
        fn wormhole_walk_visits_the_tile_then_the_other_end() {
            if crate::services_assert::isolate_test_check(
                "train::tests::bodies::collision::wormhole_walk_visits_the_tile_then_the_other_end",
                record_failure,
            ) {
                return;
            }
            let f = fixture();
            let v = part(&f, f.record, TUNNEL, 136, 40, TrackBits::TRACK_BIT_WORMHOLE);
            let entrance = new_part(&f, TUNNEL, 138, 40, TrackBits::TRACK_BIT_WORMHOLE);
            let other = new_part(&f, OTHER_END, 136, 42, TrackBits::TRACK_BIT_WORMHOLE);
            take();
            CALLS.with(|c| c.borrow_mut().clear());
            assert!(f.game().check_train_collision(v));
            let calls = CALLS.with(RefCell::take);
            assert_eq!(
                calls,
                [
                    Call::Crash(v.record.cast()),
                    Call::Wormhole(TUNNEL.0),
                    Call::Crash(entrance.record.cast()),
                    Call::Wormhole(TUNNEL.0),
                    // The second collision: `TrainCrashed(v)` crashes nothing but
                    // re-reserves `v`'s wormhole.
                    Call::Wormhole(TUNNEL.0),
                    Call::Crash(other.record.cast()),
                    Call::Wormhole(OTHER_END.0),
                    Call::Rating,
                    Call::Sound(
                        v.record.cast(),
                        SoundID::from(SoundFx::SND_13_TRAIN_COLLISION).0
                    ),
                ]
            );
            let news: Vec<_> = take()
                .into_iter()
                .filter(|e| matches!(e, Effect::News { .. }))
                .collect();
            assert_eq!(news, vec![crash_news(12, TUNNEL)]);
            assert_eq!(FAILED.with(Cell::get), 0);
        }
    }

    /// `CmdReverseTrainDirection` and `CmdForceTrainProceed` through their argument structs,
    /// with recording cost, ownership and window tables. Each expectation is read from the
    /// original command.
    mod commands {
        use super::*;
        use crate::table::strings::STR_ERROR_OWNED_BY;
        use crate::window_func::testing::Effect as Window;
        use std::cell::RefCell;

        /// One cost operation on the command's result, in call order.
        #[derive(Clone, Copy, Debug, PartialEq)]
        enum Cost {
            /// `CommandCost(msg)`, or `CMD_ERROR` with `INVALID_STRING_ID`.
            Error(u32),
            /// A successful zero cost.
            Zero,
            /// `CheckOwnership`'s owner and tile parameters for its error message.
            OwnedBy(u8, u32),
            /// `CheckOwnership`'s error owner.
            ErrorOwner(u8),
        }
        const CMD_ERROR: Cost = Cost::Error(crate::strings_type::INVALID_STRING_ID);
        thread_local! {
            static COSTS: RefCell<Vec<Cost>> = const { RefCell::new(Vec::new()) };
            static FAILED: Cell<bool> = const { Cell::new(false) };
            static TRAIN: Cell<*mut crate::vehicle_base::VehicleRecord> =
                const { Cell::new(std::ptr::null_mut()) };
            static ENGINE: Cell<*mut crate::engine::Lifecycle> =
                const { Cell::new(std::ptr::null_mut()) };
        }
        fn cost(c: Cost) {
            COSTS.with(|costs| costs.borrow_mut().push(c));
        }
        extern "C" fn error(_: *mut crate::services::CommandCost, msg: u32) {
            FAILED.set(true);
            cost(Cost::Error(msg));
        }
        extern "C" fn zero(_: *mut crate::services::CommandCost) {
            FAILED.set(false);
            cost(Cost::Zero);
        }
        extern "C" fn success(_: *const crate::services::CommandCost) -> bool {
            !FAILED.get()
        }
        extern "C" fn encode_owned_by(
            _: *mut crate::services::CommandCost,
            owner: Owner,
            tile: TileIndex,
        ) {
            cost(Cost::OwnedBy(owner.0, tile.0));
        }
        extern "C" fn error_owner(_: *mut crate::services::CommandCost, owner: Owner) {
            cost(Cost::ErrorOwner(owner.0));
        }
        extern "C" fn current_company() -> u8 {
            LOCAL
        }
        /// `Train::GetIfValid`: the fixture's train for its index, otherwise null.
        extern "C" fn train_if_valid(index: u32) -> *mut () {
            if index == VEHICLE_ID {
                TRAIN.get().cast()
            } else {
                std::ptr::null_mut()
            }
        }
        /// The shell is the record itself.
        extern "C" fn record_from_shell(
            shell: *mut std::ffi::c_void,
        ) -> *mut crate::vehicle::OpenTTDVehicle {
            shell.cast()
        }
        extern "C" fn engine(_: VehicleRef) -> crate::vehicle::EngineRef {
            crate::vehicle::EngineRef::of(ENGINE.get())
        }

        /// The fixture's train, `AM_REALISTIC`, and the command tables over it.
        struct Command {
            f: Fixture,
            rail: &'static crate::vehicle_transactions::rail::Services,
            common: &'static crate::vehicle_transactions::CommonServices,
        }
        impl Command {
            fn new(owner: u8, subtype: u8) -> Self {
                static MODEL: u8 = crate::vehicle_type::AccelerationModel::AM_REALISTIC.0;
                let settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
                    train_acceleration_model: &raw const MODEL,
                    ..crate::settings_type::SettingAddresses::NULL
                }));
                let mut f = Fixture::new(
                    owner,
                    subtype,
                    &Services {
                        local_company,
                        settings,
                        ..SHARED
                    },
                    &crate::vehicle::VehicleServices { engine, ..VEHICLE },
                );
                // `GroundVehicle::IsChainInDepot` reads the map through the ground table.
                let mut capabilities = crate::engine_capabilities::TEST_TABLE;
                capabilities.shared = f.shared;
                f.leaves = Box::leak(Box::new(Leaves {
                    physics: Box::leak(Box::new(crate::ground_vehicle::Services {
                        capabilities: Box::leak(Box::new(capabilities)),
                        ..GROUND_VEHICLE
                    })),
                    ..*f.leaves
                }));
                ENGINE.set(Box::leak(Box::default()));
                TRAIN.set(f.record);
                // SAFETY: The fixture's record; `ResetDepotUnbunching` writes its consist.
                unsafe {
                    (*f.record).consist = Box::leak(Box::new(crate::orders::Consist {
                        time: 0,
                        lateness: 0,
                        start: 0,
                        last_departure: 1,
                        next_departure: 2,
                        round_trip: 3,
                        real: 0,
                        implicit: 0,
                        flags: 0,
                    }));
                }
                let costs = Box::leak(Box::new(crate::services::CommandCostServices {
                    zero,
                    error,
                    success,
                    encode_owned_by,
                    error_owner,
                    ..crate::test_tables::COMMAND_COSTS
                }));
                let company: &'static crate::company::Services =
                    Box::leak(Box::new(crate::company::Services {
                        finance: Box::leak(Box::new(crate::company::FinanceServices {
                            current_company,
                            ..crate::test_tables::FINANCE
                        })),
                        shared: Box::leak(Box::new(crate::services::Services {
                            current_company,
                            ..*f.shared
                        })),
                        cost: costs,
                        ..crate::test_tables::COMPANY
                    }));
                COMPANY.with(|c| c.set(company));
                let common = Box::leak(Box::new(crate::vehicle_transactions::CommonServices {
                    base: Box::leak(Box::new(crate::vehicle_transactions::TransactionServices {
                        commands: Box::leak(Box::new(crate::services::CommandServices {
                            company: company_table,
                            cost: costs,
                            ..crate::test_tables::COMMANDS
                        })),
                        cost: costs,
                        ..crate::test_tables::VEHICLE_TRANSACTION
                    })),
                    vehicles: Box::leak(Box::new(crate::vehicle::VehicleServices {
                        record_from_shell,
                        ..VEHICLE
                    })),
                    ..crate::test_tables::VEHICLE_COMMON
                }));
                let rail = Box::leak(Box::new(crate::vehicle_transactions::rail::Services {
                    cargo_capacity: Box::leak(Box::new(crate::cargo_storage::CapacityServices {
                        read: crate::test_tables::unreachable2,
                        cargo: std::ptr::null(),
                    })),
                    train_if_valid,
                    new_wagon: crate::test_tables::unreachable2,
                    new_engine: crate::test_tables::unreachable2,
                    engine_dates: crate::test_tables::unreachable2,
                    new_rear: crate::test_tables::unreachable1,
                    invalidate_grf: crate::test_tables::unreachable1,
                    unit: crate::test_tables::unreachable1,
                    use_unit: crate::test_tables::unreachable2,
                    release_unit: crate::test_tables::unreachable1,
                    company_dirty: crate::test_tables::unreachable0,
                    depot_changed: crate::test_tables::unreachable1,
                }));
                COSTS.with(|c| c.borrow_mut().clear());
                FAILED.set(false);
                crate::window_func::testing::take();
                Self { f, rail, common }
            }
            fn context(&self, execute: bool) -> CommandContext<'static> {
                CommandContext {
                    rail: self.rail,
                    common: self.common,
                    out: std::ptr::dangling_mut(),
                    flags: if execute {
                        crate::command_type::DoCommandFlags::from_element(
                            crate::command_type::DoCommandFlag::Execute,
                        )
                    } else {
                        crate::command_type::DoCommandFlags(0)
                    },
                }
            }
            fn reverse(&self, veh_id: u32, reverse_single_veh: bool, execute: bool) -> Vec<Cost> {
                self.f.game().cmd_reverse_train_direction(
                    &self.context(execute),
                    CmdReverseTrainDirection {
                        veh_id: VehicleID(veh_id),
                        reverse_single_veh,
                    },
                );
                COSTS.with(RefCell::take)
            }
            fn force(&self, veh_id: u32, execute: bool) -> Vec<Cost> {
                self.f.game().cmd_force_train_proceed(
                    &self.context(execute),
                    CmdForceTrainProceed {
                        veh_id: VehicleID(veh_id),
                    },
                );
                COSTS.with(RefCell::take)
            }
            fn train(&self) -> Handle {
                self.f.train()
            }
            /// Make the fixture's tile a rail depot and its train stand in it.
            fn in_depot(&self) {
                // SAFETY: The fixture's map; no reference into it is live. `MP_RAILWAY` with
                // `RAIL_TILE_DEPOT` in m5 bits 6..7.
                unsafe {
                    let tile = &mut *self.f.base.add(TILE.0 as usize);
                    tile.kind = crate::tile_type::TileType::MP_RAILWAY.0 << 4;
                    tile.m5 = 0xC0;
                }
                self.train().set_track(TrackBits::TRACK_BIT_DEPOT);
                self.train().vehicle().set_cur_speed(0);
            }
            fn consist(&self) -> (u64, u64, i32) {
                // SAFETY: The consist this fixture set.
                unsafe {
                    let c = &*(*self.f.record).consist;
                    (c.last_departure, c.next_departure, c.round_trip)
                }
            }
        }
        thread_local! {
            static COMPANY: Cell<*const crate::company::Services> = const { Cell::new(std::ptr::null()) };
        }
        extern "C" fn company_table() -> *const crate::company::Services {
            COMPANY.with(Cell::get)
        }
        const MULTIHEADED: u8 =
            1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_MULTIHEADED.0;
        const FREE_WAGON: u8 =
            1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_FREE_WAGON.0;
        const WAGON: u8 = 1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_WAGON.0;

        /// `if (v == nullptr) return CMD_ERROR;` then `CheckOwnership(v->owner)`: another
        /// company's train fails with `STR_ERROR_OWNED_BY`, whose owner and tile parameters
        /// the local company's command adds. Both commands share these results; force
        /// proceed tests `IsPrimaryVehicle` first, so another company's wagon gets
        /// `CMD_ERROR` and no ownership check.
        #[test]
        fn lookup_and_ownership_fail_in_the_original_order() {
            let c = Command::new(LOCAL, FRONT_ENGINE);
            assert_eq!(c.reverse(VEHICLE_ID + 1, false, true), [CMD_ERROR]);
            assert_eq!(c.force(VEHICLE_ID + 1, true), [CMD_ERROR]);

            let other = LOCAL + 1;
            let owned_by = [
                Cost::Error(STR_ERROR_OWNED_BY.0),
                Cost::OwnedBy(other, 0),
                Cost::ErrorOwner(other),
            ];
            let c = Command::new(other, FRONT_ENGINE);
            assert_eq!(c.reverse(VEHICLE_ID, false, true), owned_by);
            assert_eq!(c.reverse(VEHICLE_ID, true, true), owned_by);
            assert_eq!(c.force(VEHICLE_ID, true), owned_by);
            let c = Command::new(other, WAGON);
            assert_eq!(c.force(VEHICLE_ID, true), [CMD_ERROR]);
            assert_eq!(crate::window_func::testing::take(), []);
        }

        /// The whole-train branch alone tests `IsFrontEngine`, crashed and broken down for
        /// `CMD_ERROR`. A query succeeds without a change; the executed command cancels
        /// `force_proceed`, dirties the vehicle view, and under `AM_REALISTIC` with the train
        /// moving flips `Reversing`, then resets the unbunching data.
        #[test]
        fn reverse_whole_train_checks_the_front_engine_and_changes_state_only_on_execute() {
            let c = Command::new(LOCAL, WAGON);
            assert_eq!(c.reverse(VEHICLE_ID, false, true), [Cost::Zero, CMD_ERROR]);
            for broken in [false, true] {
                let c = Command::new(LOCAL, FRONT_ENGINE);
                if broken {
                    c.train().vehicle().set_breakdown_ctr(1);
                } else {
                    c.train()
                        .vehicle()
                        .set_vehstatus(c.train().vehicle().vehstatus().set(VehState::Crashed));
                }
                assert_eq!(c.reverse(VEHICLE_ID, false, true), [Cost::Zero, CMD_ERROR]);
            }

            let c = Command::new(LOCAL, FRONT_ENGINE);
            c.train()
                .set_force_proceed(TrainForceProceeding::TFP_SIGNAL);
            c.train().vehicle().set_cur_speed(10);
            assert_eq!(c.reverse(VEHICLE_ID, false, false), [Cost::Zero]);
            assert_eq!(c.train().force_proceed(), TrainForceProceeding::TFP_SIGNAL);
            assert!(!c.train().flags().test(VehicleRailFlag::Reversing));
            assert_eq!(c.consist(), (1, 2, 3));
            assert_eq!(crate::window_func::testing::take(), []);

            assert_eq!(c.reverse(VEHICLE_ID, false, true), [Cost::Zero]);
            assert_eq!(c.train().force_proceed(), TrainForceProceeding::TFP_NONE);
            assert!(c.train().flags().test(VehicleRailFlag::Reversing));
            assert_eq!(c.consist(), (0, 0, 0));
            assert_eq!(
                crate::window_func::testing::take(),
                [Window::SetWindowDirty(
                    WindowClass::WC_VEHICLE_VIEW,
                    WindowNumber(VEHICLE_ID as i32)
                )]
            );
        }

        /// The single-unit branch: a multiheaded unit gets
        /// `STR_ERROR_CAN_T_REVERSE_DIRECTION_RAIL_VEHICLE_MULTIPLE_UNITS`; a front engine
        /// that is not stopped in its depot gets
        /// `STR_ERROR_TRAINS_CAN_ONLY_BE_ALTERED_INSIDE_A_DEPOT`, and so does a stopped
        /// one outside a depot. A free wagon has no stopped state, so in its depot it
        /// passes `IsStoppedInDepot`; the whole-train checks do not apply to it, and the
        /// query succeeds without a change.
        #[test]
        fn reverse_single_unit_returns_the_two_strings_and_passes_a_free_wagon_in_its_depot() {
            let c = Command::new(LOCAL, FRONT_ENGINE | MULTIHEADED);
            assert_eq!(
                c.reverse(VEHICLE_ID, true, true),
                [
                    Cost::Zero,
                    Cost::Error(STR_ERROR_CAN_T_REVERSE_DIRECTION_RAIL_VEHICLE_MULTIPLE_UNITS.0)
                ]
            );
            let depot = [
                Cost::Zero,
                Cost::Error(STR_ERROR_TRAINS_CAN_ONLY_BE_ALTERED_INSIDE_A_DEPOT.0),
            ];
            let c = Command::new(LOCAL, FRONT_ENGINE);
            c.in_depot();
            assert_eq!(c.reverse(VEHICLE_ID, true, true), depot);
            let c = Command::new(LOCAL, FRONT_ENGINE);
            c.train()
                .vehicle()
                .set_vehstatus(c.train().vehicle().vehstatus().set(VehState::Stopped));
            assert_eq!(c.reverse(VEHICLE_ID, true, true), depot);
            // Stopped in its depot: the query succeeds without a change.
            let c = Command::new(LOCAL, FRONT_ENGINE);
            c.in_depot();
            c.train()
                .vehicle()
                .set_vehstatus(c.train().vehicle().vehstatus().set(VehState::Stopped));
            assert_eq!(c.reverse(VEHICLE_ID, true, false), [Cost::Zero]);
            assert!(!c.train().flags().test(VehicleRailFlag::Flipped));

            let c = Command::new(LOCAL, FREE_WAGON);
            c.in_depot();
            c.train()
                .vehicle()
                .set_vehstatus(c.train().vehicle().vehstatus().set(VehState::Crashed));
            assert_eq!(c.reverse(VEHICLE_ID, true, false), [Cost::Zero]);
            assert!(!c.train().flags().test(VehicleRailFlag::Flipped));
            assert_eq!(crate::window_func::testing::take(), []);
        }

        /// `DetermineNextTrainForceProceeding`: a running train is forced to the signal, a
        /// train in its depot is marked stuck, a stuck train before a tile without signals
        /// stays stuck, and a train already passing a signal is cancelled. Only the
        /// executed command stores it, dirties the view and resets the unbunching data.
        #[test]
        fn force_proceed_stores_the_next_force_proceeding_only_on_execute() {
            let c = Command::new(LOCAL, FRONT_ENGINE);
            c.train().set_track(TrackBits::TRACK_BIT_X);
            assert_eq!(c.force(VEHICLE_ID, false), [Cost::Zero]);
            assert_eq!(c.train().force_proceed(), TrainForceProceeding::TFP_NONE);
            assert_eq!(c.consist(), (1, 2, 3));
            assert_eq!(crate::window_func::testing::take(), []);
            assert_eq!(c.force(VEHICLE_ID, true), [Cost::Zero]);
            assert_eq!(c.train().force_proceed(), TrainForceProceeding::TFP_SIGNAL);
            assert_eq!(c.consist(), (0, 0, 0));
            assert_eq!(
                crate::window_func::testing::take(),
                [Window::SetWindowDirty(
                    WindowClass::WC_VEHICLE_VIEW,
                    WindowNumber(VEHICLE_ID as i32)
                )]
            );
            // Passing a signal already: the order is cancelled.
            assert_eq!(c.force(VEHICLE_ID, true), [Cost::Zero]);
            assert_eq!(c.train().force_proceed(), TrainForceProceeding::TFP_NONE);

            let c = Command::new(LOCAL, FRONT_ENGINE);
            c.in_depot();
            assert_eq!(c.force(VEHICLE_ID, true), [Cost::Zero]);
            assert_eq!(c.train().force_proceed(), TrainForceProceeding::TFP_STUCK);

            let c = Command::new(LOCAL, FRONT_ENGINE);
            c.train().set_track(TrackBits::TRACK_BIT_X);
            c.train()
                .set_flags(c.train().flags().set(VehicleRailFlag::Stuck));
            assert_eq!(c.force(VEHICLE_ID, true), [Cost::Zero]);
            assert_eq!(c.train().force_proceed(), TrainForceProceeding::TFP_STUCK);
        }
        thread_local! {
            static HEAD_FAILED: Cell<u32> = const { Cell::new(0) };
        }
        extern "C" fn record_head_failure(ok: bool) {
            if !ok {
                HEAD_FAILED.with(|f| f.set(f.get() + 1));
            }
        }
        /// `Vehicle::IsStoppedInDepot` asserts `this == this->First()`: the second part of
        /// a chain fails it once, and the head passes it.
        #[test]
        fn is_stopped_in_depot_asserts_the_head_of_the_chain() {
            if crate::services_assert::isolate_test_check(
                "train::tests::bodies::commands::is_stopped_in_depot_asserts_the_head_of_the_chain",
                record_head_failure,
            ) {
                return;
            }
            let c = Command::new(LOCAL, FRONT_ENGINE);
            let second = crate::vehicle_base::openttd_rust_vehicle_record_new(
                std::ptr::null_mut(),
                VehicleType::VEH_TRAIN,
            );
            // SAFETY: The fresh record and the fixture's head, linked as `Next()`.
            unsafe {
                (*second).train = crate::train_state::openttd_rust_train_state_new();
                (*second).ground =
                    Box::leak(Box::new(crate::ground_vehicle::GroundState::default()));
                (*second).first = c.f.record;
                (*second).tile = TILE;
                (*c.f.record).next = second;
            }
            let head = c.train().vehicle();
            assert!(!crate::vehicle_base::is_stopped_in_depot(
                head,
                c.f.leaves.physics
            ));
            assert_eq!(HEAD_FAILED.with(Cell::get), 0);
            let _ = crate::vehicle_base::is_stopped_in_depot(
                VehicleRef::of(second),
                c.f.leaves.physics,
            );
            assert_eq!(HEAD_FAILED.with(Cell::get), 1);
        }
    }
}
