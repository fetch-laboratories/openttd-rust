/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
//! Tests of `disaster.rs`. Their unsafe items reach the fixture tables the tests leak, which
//! live for the process, and the costs the command builds in its own storage.
use super::{DisasterVehicle, Services, Tables};
use crate::company_type::{CompanyID, Owner};
use crate::direction_type::{DiagDirection, Direction};
use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation, fixture};
use crate::services::{CommandCost, CommandCostStorage, Services as Shared};
use crate::tile_type::{TileIndex, TileType};
use crate::vehicle::VehicleRef;
use crate::vehicle_type::VehicleType;
use std::cell::{Cell, RefCell};
use std::ptr;

#[cfg(test)]
thread_local! {
    static FAILED_ASSERTIONS: Cell<u32> = const { Cell::new(0) };
    static RANDOM: Cell<u32> = const { Cell::new(0) };
    static DRAWS: Cell<u32> = const { Cell::new(0) };
    static AGE: Cell<i32> = const { Cell::new(0) };
}
extern "C" fn record_assertion(condition: bool) {
    if !condition {
        FAILED_ASSERTIONS.set(FAILED_ASSERTIONS.get() + 1);
    }
}
extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
    DRAWS.set(DRAWS.get() + 1);
    RANDOM.get()
}
extern "C" fn age(_: *mut crate::vehicle_type::OpenTTDVehicle) -> i32 {
    AGE.get()
}
extern "C" fn set_age(_: VehicleRef, value: i32) {
    AGE.set(value);
}
#[cfg(test)]
thread_local! {
    static RESOLVER: &'static crate::newgrf_vehicle::Services = {
        let mut resolver = crate::newgrf_vehicle::TEST_TABLE;
        resolver.age = age;
        resolver.id = vehicle_id;
        leak(resolver)
    };
    static POST: &'static crate::command_post::Services = leak(crate::command_post::Services {
        command: commands,
        ..crate::test_tables::COMMAND_POST
    });
}
extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
    RESOLVER.with(|r| *r)
}
extern "C" fn vehicle_id(_: *mut crate::vehicle_type::OpenTTDVehicle) -> u32 {
    0
}

fn leak<T>(table: T) -> &'static T {
    Box::leak(Box::new(table))
}

/// The vehicle table whose resolver reads `age` and `v->index` (0), and whose maintenance
/// table stores `age`.
fn vehicles() -> &'static crate::vehicle::VehicleServices {
    leak(crate::vehicle::VehicleServices {
        newgrf,
        maintenance: leak(crate::vehicle::maintenance::Services {
            set_age,
            ..crate::test_tables::MAINTENANCE
        }),
        ..crate::test_tables::VEHICLE
    })
}

/// A 64 by 64 map with every tile of `kind`, installed for this thread.
fn install_map(kind: TileType, m1: u8) -> (Vec<BaseTileLayout>, Vec<ExtendedTileLayout>) {
    let base: Vec<_> = (0..4096)
        .map(|_| BaseTileLayout {
            kind: kind.0 << 4,
            height: 0,
            m2: 0,
            m1,
            m3: 0,
            m4: 0,
            m5: 0,
        })
        .collect();
    let extended: Vec<_> = (0..4096)
        .map(|_| ExtendedTileLayout {
            m6: 0,
            m7: 0,
            m8: 0,
        })
        .collect();
    fixture::set(
        TileTypeAllocation {
            base: base.as_ptr(),
            extended: extended.as_ptr(),
            count: 4096,
            assertions: true,
        },
        6,
    );
    (base, extended)
}

/// A disaster vehicle record with its owner, which the test leaks.
#[allow(unsafe_code)]
fn disaster_vehicle() -> DisasterVehicle {
    let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
        ptr::null_mut(),
        VehicleType::VEH_DISASTER,
    );
    // SAFETY: The fresh record is the test's; nothing else holds it.
    unsafe { (*record).disaster = super::openttd_rust_disaster_state_create() };
    DisasterVehicle::of(VehicleRef::of(record))
}

/// A submarine on row 0 of a 64x64 sea heading north-west. Pinned `disaster_vehicle.cpp:685`
/// adds `TileOffsByDiagDir` by hand, so at the native `_DEBUG` projection the step leaves the
/// map without an assertion, `IsValidTile` rejects it and the submarine turns where it is.
/// A step that wrapped to row 63, or that `TileAdd` let through, would reach sea, and the
/// random source fails `Chance16(1, 90)`, so a submarine that took it would move.
#[test]
fn submarine_leaves_row_zero_without_assertion_under_native_debug() {
    if crate::services_assert::isolate_test_check(
        "disaster::tests::submarine_leaves_row_zero_without_assertion_under_native_debug",
        record_assertion,
    ) {
        return;
    }
    let _map = install_map(TileType::MP_WATER, Owner::WATER.0);
    let mut shared = crate::services::fixture(ptr::null_mut(), random);
    shared.map = fixture::statics();
    let services = Services {
        shared: leak(shared),
        vehicles: vehicles(),
        ..crate::test_tables::DISASTER
    };
    let t = Tables::new(&services);
    let v = disaster_vehicle();
    v.v.set_subtype(crate::disaster_vehicle::DisasterSubType::ST_SMALL_SUBMARINE.0);
    v.v.set_tile(TileIndex(10)); // (10, 0)
    v.v.set_direction(Direction::DIR_NW);
    v.v.set_tick_counter(0); // Incremented to an odd tick, which moves.
    RANDOM.set(0xFFFF);
    FAILED_ASSERTIONS.set(0);
    let alive = crate::map_func::with_native_debug(true, || super::tick(&t, v));
    assert!(alive);
    assert_eq!(FAILED_ASSERTIONS.get(), 0);
    assert_eq!(v.v.tile(), TileIndex(10), "the submarine stays where it is");
    // One draw: the turn. Random bit 0 is set: DIRDIFF_90RIGHT turns north-west to north-east.
    assert_eq!(DRAWS.get(), 1);
    assert_eq!(v.v.direction(), Direction::DIR_NE);
    assert_eq!(AGE.get(), 1);
    // The same step through TileAdd fails the y and wrap assertions.
    crate::map_func::with_native_debug(true, || {
        let _ = t.world().dimensions.tile_add_by_diag_dir(
            TileIndex(10),
            DiagDirection::DIAGDIR_NW,
            crate::map_func::native_check,
        );
    });
    assert_eq!(FAILED_ASSERTIONS.get(), 2);
}

/// `_disaster_delay` is one process-wide counter: a serialized worker thread sees and
/// writes the same storage as the thread that started it.
#[test]
#[allow(unsafe_code)]
fn disaster_delay_is_process_wide_across_serialized_threads() {
    let address = super::openttd_rust_disaster_delay() as usize;
    // SAFETY: The worker runs after this access and is joined before the next.
    unsafe {
        (address as *mut u16).write(41);
    }
    std::thread::spawn(move || {
        let worker = super::openttd_rust_disaster_delay();
        assert_eq!(worker as usize, address);
        // SAFETY: As above.
        unsafe {
            assert_eq!(worker.read(), 41);
            worker.write(73);
        }
    })
    .join()
    .unwrap();
    // SAFETY: As above.
    unsafe {
        assert_eq!((address as *const u16).read(), 73);
    }
}

/// One effect `DisasterClearSquare` and the command it runs have, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Event {
    /// `_current_company = company`.
    SetCompany(CompanyID),
    /// The tile's clear procedure, in the test (`false`) or execute (`true`) phase, with the
    /// current company of that call.
    Clear {
        tile: TileIndex,
        execute: bool,
        company: CompanyID,
    },
    /// The signal update of a buffered block for `owner`, through its `Company::IsValidID`.
    SignalUpdate(CompanyID),
}

#[cfg(test)]
thread_local! {
    static EVENTS: RefCell<Vec<Event>> = const { RefCell::new(Vec::new()) };
    static CURRENT: Cell<u8> = const { Cell::new(0) };
    static SHARED_TABLE: Cell<*const Shared> = const { Cell::new(ptr::null()) };
    static AIRPLANE: Cell<Option<DisasterVehicle>> = const { Cell::new(None) };
}
fn event(e: Event) {
    EVENTS.with_borrow_mut(|events| events.push(e));
}
fn take_events() -> Vec<Event> {
    EVENTS.with_borrow_mut(std::mem::take)
}
extern "C" fn current_company() -> u8 {
    CURRENT.get()
}
extern "C" fn set_current_company(company: u8) {
    CURRENT.set(company);
    event(Event::SetCompany(CompanyID(company)));
}
extern "C" fn company_valid(owner: u8) -> bool {
    event(Event::SignalUpdate(CompanyID(owner)));
    true
}
extern "C" fn no_finances(_: u8) -> *mut crate::company::Finances {
    ptr::null_mut()
}
extern "C" fn map_max_y() -> u32 {
    63
}
extern "C" fn water_services() -> *const crate::water::WaterServices {
    static WATER: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *WATER.get_or_init(|| ptr::from_ref(leak(crate::water::fixture())) as usize) as *const _
}
/// The landscape command's own view of every tile: a kind it sends to the recording clear
/// procedure, which stands for the tile's clear handler. `DisasterClearSquare` classifies
/// the tile from the real map.
extern "C" fn landscape_tile_type(_: u32) -> u8 {
    TileType::MP_TUNNELBRIDGE.0
}
/// The tile's clear procedure. In the execute phase it releases the disasters targeting
/// industry 7, as an industry deletion nested in a clear would.
#[allow(unsafe_code)]
extern "C" fn clear(cost: *mut CommandCost, tile: u32, flags: u16) {
    let execute = flags & crate::command_type::DoCommandFlag::EXECUTE != 0;
    event(Event::Clear {
        tile: TileIndex(tile),
        execute,
        company: CompanyID(CURRENT.get()),
    });
    set_cost_flag(cost, true);
    if execute {
        // SAFETY: The test's shared table outlives the command.
        let shared = unsafe { &*SHARED_TABLE.get() };
        super::release_disasters_targeting_industry(shared, crate::industry_type::IndustryID(7));
    }
}
extern "C" fn vehicle_next(from: u32) -> Option<VehicleRef> {
    if from == 0 {
        AIRPLANE.get().map(|v| v.v)
    } else {
        None
    }
}

/// The fixture's command cost: one success flag, read from a live fixture cost.
#[allow(unsafe_code)]
fn cost_flag(cost: *const CommandCost) -> bool {
    // SAFETY: A live fixture cost, or storage `construct` has written.
    unsafe { *cost.cast::<bool>() }
}
/// Store the fixture cost's success flag. The storage is the command's own aligned stack
/// capacity or a live fixture cost.
#[allow(unsafe_code)]
fn set_cost_flag(cost: *mut CommandCost, value: bool) {
    // SAFETY: As above; a `bool` store needs no initialized old value.
    unsafe { cost.cast::<bool>().write(value) };
}
extern "C" fn construct(storage: *mut CommandCostStorage) -> *mut CommandCost {
    let cost = storage.cast::<CommandCost>();
    set_cost_flag(cost, true);
    cost
}
extern "C" fn destroy(_: *mut CommandCost) {}
extern "C" fn expense_money(cost: *mut CommandCost, _: u8, _: i64) {
    set_cost_flag(cost, true);
}
extern "C" fn add(cost: *mut CommandCost, other: *mut CommandCost) {
    set_cost_flag(cost, cost_flag(cost) & cost_flag(other));
}
extern "C" fn money(_: *const CommandCost) -> i64 {
    0
}
extern "C" fn success(cost: *const CommandCost) -> bool {
    cost_flag(cost)
}

/// The service tables `disaster_clear_square_clears_house_and_human_rail_as_its_owner` runs
/// under: the real landscape command over the recording clear procedure, the fixture's
/// costs and companies, and the signal buffer.
fn clear_fixture() -> (&'static Services, &'static Shared) {
    let costs = leak(crate::services::CommandCostServices {
        construct,
        destroy,
        expense_money,
        add,
        success,
        money,
        ..crate::test_tables::COMMAND_COSTS
    });
    let finance = leak(crate::company::FinanceServices {
        current_company,
        set_current_company,
        company_finances: no_finances,
        ..crate::test_tables::FINANCE
    });
    let signals = leak(crate::signal::SignalServices {
        assertions: true,
        assert_condition: record_assertion,
    });
    let mut shared = crate::services::fixture(ptr::null_mut(), random);
    shared.map = fixture::statics();
    shared.map_max_y = map_max_y;
    shared.current_company = current_company;
    shared.company_valid = company_valid;
    shared.water_services = water_services;
    shared.vehicle_next = vehicle_next;
    let mut movement_world = crate::test_tables::SHARED;
    movement_world.map = fixture::statics();
    shared.movement = leak(crate::movement::MovementServices {
        shared: leak(movement_world),
        native_assert: record_assertion,
        ..crate::test_tables::MOVEMENT
    });
    let shared: &'static mut Shared = Box::leak(Box::new(shared));
    let mut landscape_shared = Shared { ..*shared };
    landscape_shared.tile_type = landscape_tile_type;
    let landscape_shared = leak(landscape_shared);
    let commands: &'static crate::services::CommandServices =
        leak(crate::services::CommandServices {
            finance,
            shared: landscape_shared,
            cost: costs,
            signals,
            landscape_commands,
            ..crate::test_tables::COMMANDS
        });
    COMMANDS.set(commands);
    LANDSCAPE_TABLE.set(leak(crate::landscape_cmd::LandscapeServices {
        shared: landscape_shared,
        costs,
        commands,
        tile_procs: leak(crate::landscape_cmd::TileServices {
            clear,
            owners: leak(crate::test_tables::TILE_OWNERS),
            ..crate::test_tables::LANDSCAPE_TILES
        }),
        ..crate::test_tables::LANDSCAPE
    }));
    shared.command_post = command_post;
    let vehicles = vehicles();
    STATION_COMMANDS.set(leak(crate::station_cmd::CommandServicesTable {
        vehicle: vehicles,
        ..crate::test_tables::STATION_COMMANDS
    }));
    shared.station_commands = station_commands;
    SHARED_TABLE.set(shared);
    let identity = leak(crate::company_identity::Services {
        company: company_of,
        is_ai,
        ..crate::test_tables::COMPANY_IDENTITY
    });
    let services = leak(Services {
        identity,
        shared,
        vehicles,
        ..crate::test_tables::DISASTER
    });
    (services, shared)
}

/// `DisasterClearSquare` through the real `Command<CMD_LANDSCAPE_CLEAR>::Do`, its nesting and
/// its company handling. A house is cleared as `OWNER_NONE`, plain rail of a human company
/// as `OWNER_WATER` with the signal buffer updated after the company is restored, and a rail
/// depot or an AI's rail is left alone. A top-level command runs its procedure in a test
/// phase and then an execute phase, so each expectation names its phase.
#[test]
fn disaster_clear_square_clears_house_and_human_rail_as_its_owner() {
    if crate::services_assert::isolate_test_check(
        "disaster::tests::disaster_clear_square_clears_house_and_human_rail_as_its_owner",
        record_assertion,
    ) {
        return;
    }
    let house = TileIndex(100);
    let rail = TileIndex(200);
    let depot = TileIndex(300);
    let ai_rail = TileIndex(400);
    let block = TileIndex(1000);
    let (mut base, _extended) = install_map(TileType::MP_CLEAR, 0);
    base[house.0 as usize].kind = TileType::MP_HOUSE.0 << 4;
    for (tile, owner, m5) in [(rail, 1, 0x01), (depot, 1, 0xC0), (ai_rail, 2, 0x01)] {
        let tile = tile.0 as usize;
        base[tile].kind = TileType::MP_RAILWAY.0 << 4;
        base[tile].m1 = owner;
        base[tile].m5 = m5;
    }
    let (services, shared) = clear_fixture();
    let t = Tables::new(services);

    // An airplane in state 1 whose target, in `dest_tile`, is industry 7.
    let airplane = disaster_vehicle();
    airplane
        .v
        .set_subtype(crate::disaster_vehicle::DisasterSubType::ST_AIRPLANE.0);
    airplane.v.set_dest_tile(TileIndex(7));
    AIRPLANE.set(Some(airplane));

    // A block queued for company 5 on clear ground, before each clear.
    let queue_block = || {
        crate::signal::context(shared).add_track_to_signal_buffer(
            block,
            crate::track_type::Track::TRACK_X,
            CompanyID(5),
        );
    };
    let saved = CompanyID(3);

    for (tile, company) in [(house, Owner::NONE), (rail, Owner::WATER)] {
        CURRENT.set(saved.0);
        airplane.set_state(1);
        queue_block();
        super::disaster_clear_square(&t, tile);
        let mut expected = vec![
            Event::SetCompany(company),
            Event::Clear {
                tile,
                execute: false,
                company,
            },
            Event::Clear {
                tile,
                execute: true,
                company,
            },
            Event::SetCompany(saved),
        ];
        if tile == rail {
            // The house's block is still queued; the rail's clear updates it.
            expected.push(Event::SignalUpdate(CompanyID(5)));
        }
        assert_eq!(take_events(), expected, "tile {}", tile.0);
        assert_eq!(airplane.state(), 3, "the nested release, tile {}", tile.0);
        assert_eq!(CURRENT.get(), saved.0);
    }

    for tile in [depot, ai_rail] {
        super::disaster_clear_square(&t, tile);
        assert_eq!(take_events(), [], "tile {}", tile.0);
    }
    assert_eq!(FAILED_ASSERTIONS.get(), 0);
}

#[cfg(test)]
thread_local! {
    static COMMANDS: Cell<&'static crate::services::CommandServices> =
        const { Cell::new(&crate::test_tables::COMMANDS) };
    static LANDSCAPE_TABLE: Cell<*const crate::landscape_cmd::LandscapeServices> =
        const { Cell::new(ptr::null()) };
    static STATION_COMMANDS: Cell<*const crate::station_cmd::CommandServicesTable> =
        const { Cell::new(ptr::null()) };
}
extern "C" fn landscape_commands() -> *const crate::landscape_cmd::LandscapeServices {
    LANDSCAPE_TABLE.get()
}
extern "C" fn commands() -> &'static crate::services::CommandServices {
    COMMANDS.get()
}
extern "C" fn command_post() -> &'static crate::command_post::Services {
    POST.with(|p| *p)
}
extern "C" fn station_commands() -> *const crate::station_cmd::CommandServicesTable {
    STATION_COMMANDS.get()
}
/// Company `id` as its own address-free handle.
extern "C" fn company_of(id: CompanyID) -> *mut crate::company_type::OpenTTDCompany {
    ptr::without_provenance_mut(usize::from(id.0) + 1)
}
/// Company 1 is human, every other company an AI.
extern "C" fn is_ai(c: *mut crate::company_type::OpenTTDCompany) -> bool {
    c.addr() != 2
}
