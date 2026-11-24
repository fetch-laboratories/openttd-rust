/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The vehicle scheduler of `vehicle.cpp`: `RunVehicleCalendarDayProc`,
//! `RunEconomyVehicleDayProc` and `CallVehicleTicks` up to its autoreplace loop, which is
//! [`fleet::pending::drain`], with the dispatch of the day virtuals `OnNewCalendarDay` and
//! `OnNewEconomyDay`. The session's two entries take the record [`Tables`], whose
//! [`SchedulerServices`] holds the vehicle pool's descriptor, the native `Tick` and the
//! controllers' tables.
//!
//! The pool and the vehicle records are read in place: the pool through its descriptor, each
//! member through the `VehicleRef` accessor named after it. No reference into a shell, a record
//! or the pool survives a service that can mutate or reenter: a `VehicleRef` is a pointer, each
//! member is loaded or stored at its use, and a walk rereads the pool's extent and slots at each
//! step. The tables are immutable for the process, and no service an entry calls resets the
//! pool, loads a game or yields the game-state mutex.

use super::{CALLBACK_FAILED, VehicleRef, VehicleServices};
use crate::bitmath_func::{gb, gb_u8, has_bit};
use crate::framerate_type::{PerformanceAccumulator, PerformanceElement, PerformanceMeasurer};
use crate::newgrf_callbacks::CBID_VEHICLE_32DAY_CALLBACK;
use crate::newgrf_sound::{VSE_RUNNING, VSE_RUNNING_16, VSE_STOPPED_16};
use crate::openttd::GameMode;
use crate::pool::{PoolDescriptor, PoolId};
use crate::services_assert::assert_with;
use crate::timer_game_tick::DAY_TICKS;
use crate::vehicle_base::{Pool, VehState, VehStates};
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::{
    aircraft, cargo_storage, fleet, ground_vehicle, newgrf_vehicle, road, services, session,
    ship_control, station_service, timer_game_common, train, vehicle_base,
};
use std::mem::MaybeUninit;

#[repr(C)]
pub struct SchedulerServices {
    /// The aircraft controller's table, immutable for the process.
    pub aircraft: *const aircraft::Leaves,
    /// `&_vehicle_pool.descriptor`: the vehicle pool's descriptor, Rust's own record.
    pub pool: *const *const PoolDescriptor,
    /// The road vehicle controller's table, immutable for the process.
    pub road: *const road::Leaves,
    /// The shared table, immutable for the process.
    pub shared: *const services::Services,
    /// The ship controller's table, immutable for the process.
    pub ship: *const ship_control::Leaves,
    /// Runs the virtual Tick; true when the vehicle survives.
    pub tick: extern "C" fn(VehicleRef) -> bool,
    /// The train controller's table, immutable for the process.
    pub train: *const train::Leaves,
    /// The base vehicle table, immutable for the process.
    pub vehicle: *const VehicleServices,
}
#[repr(C)]
pub struct Tables {
    pub cargo: *const cargo_storage::Services,
    pub group: *const fleet::GroupServices,
    pub loading: *const station_service::Loading,
    pub pending: *const fleet::pending::Services,
    pub refit_cost: *mut services::CommandCost,
    pub refit_temporary: *mut services::CommandCost,
    pub scheduler: *const SchedulerServices,
    pub station: *const station_service::World,
    pub transaction: *const fleet::transactions::Services,
    pub visits: *const super::visits::Services,
}

#[allow(unsafe_code)]
impl SchedulerServices {
    /// The vehicle pool, read in place.
    pub(crate) fn pool(&self) -> Pool {
        // SAFETY: The table's descriptor address and the descriptor live for the process.
        unsafe { Pool::from_descriptor(*self.pool) }
    }
    fn aircraft(&self) -> &aircraft::Leaves {
        // SAFETY: Each table link is a process-lifetime table.
        unsafe { &*self.aircraft }
    }
    fn road(&self) -> &road::Leaves {
        // SAFETY: As `aircraft`.
        unsafe { &*self.road }
    }
    fn shared(&self) -> &services::Services {
        // SAFETY: As `aircraft`.
        unsafe { &*self.shared }
    }
    fn ship(&self) -> &ship_control::Leaves {
        // SAFETY: As `aircraft`.
        unsafe { &*self.ship }
    }
    fn train(&self) -> &train::Leaves {
        // SAFETY: As `aircraft`.
        unsafe { &*self.train }
    }
    pub(crate) fn vehicle(&self) -> &VehicleServices {
        // SAFETY: As `aircraft`.
        unsafe { &*self.vehicle }
    }
}

// Select counted/plain implementations once at entry. In the plain monomorph,
// COUNTED=false erases every per-visit witness branch, load and update.
/// One C call of this file: a `vehicle.crossings` hit and its result's size.
macro_rules! svc {
    ($counted:ident, $function:expr $(, $arg:expr)*) => {
        crate::witness::counted!(
            VEHICLE_CROSSINGS, VEHICLE_OBSERVED_BYTES if $counted, ($function)($($arg),*)
        )
    };
}
/// A record member the body after `Tick` loads; its size counts in `vehicle.scalar_read_bytes`.
macro_rules! load {
    ($counted:ident, $read:expr) => {{
        let value = $read;
        if $counted {
            crate::witness::add(
                crate::witness::VEHICLE_SCALAR_READ_BYTES,
                ::std::mem::size_of_val(&value) as u64,
            );
        }
        value
    }};
}
/// A record member the body after `Tick` stores; its size counts in
/// `vehicle.scalar_write_bytes`.
macro_rules! store {
    ($counted:ident, $v:ident . $set:ident($value:expr)) => {{
        let value = $value;
        if $counted {
            crate::witness::add(
                crate::witness::VEHICLE_SCALAR_WRITE_BYTES,
                ::std::mem::size_of_val(&value) as u64,
            );
        }
        $v.$set(value);
    }};
}

/// `v->OnNewCalendarDay()`: the virtual's dispatch. Effect and disaster vehicles keep the base
/// class's empty body.
fn on_new_calendar_day(s: &SchedulerServices, v: VehicleRef, id: VehicleID) {
    match v.vehicle_type() {
        VehicleType::VEH_TRAIN => train::on_new_calendar_day(
            train::Handle::from_record(v.0.as_ptr()),
            s.train(),
            s.shared(),
        ),
        VehicleType::VEH_ROAD => road::on_new_calendar_day(v, s.road()),
        VehicleType::VEH_SHIP => ship_control::on_new_calendar_day(id.0, v, s.ship(), s.shared()),
        VehicleType::VEH_AIRCRAFT => {
            aircraft::on_new_calendar_day(s.aircraft(), aircraft::Vehicle::of(v, id));
        }
        _ => {}
    }
}
/// `v->OnNewEconomyDay()`: the virtual's dispatch. Effect and disaster vehicles keep the base
/// class's empty body.
fn on_new_economy_day(s: &SchedulerServices, v: VehicleRef, id: VehicleID) {
    match v.vehicle_type() {
        VehicleType::VEH_TRAIN => train::on_new_economy_day(
            train::Handle::from_record(v.0.as_ptr()),
            s.train(),
            s.shared(),
        ),
        VehicleType::VEH_ROAD => road::on_new_economy_day(v, s.road()),
        VehicleType::VEH_SHIP => ship_control::on_new_economy_day(id.0, v, s.ship(), s.shared()),
        VehicleType::VEH_AIRCRAFT => {
            aircraft::on_new_economy_day(s.aircraft(), aircraft::Vehicle::of(v, id));
        }
        _ => {}
    }
}

/// `RunVehicleCalendarDayProc`: age all vehicles, spreading out the action using the current
/// `TimerGameCalendar::date_fract`.
#[allow(unsafe_code)]
fn run_vehicle_calendar_day_proc<const COUNTED: bool>(s: &SchedulerServices) {
    if session::mode() != GameMode::GM_NORMAL {
        return;
    }

    let pool = s.pool();
    // Run the calendar day proc for every DAY_TICKS vehicle starting at
    // TimerGameCalendar::date_fract.
    let mut i = usize::from(timer_game_common::calendar_fraction());
    // SAFETY, for the pool reads: the live descriptor, read afresh at each step.
    while i < unsafe { pool.pool_size() } {
        let id = vehicle_base::pool_id(i);
        if let Some(v) = unsafe { vehicle_base::get(&pool, id) } {
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_DAYS);
            }
            on_new_calendar_day(s, v, id);
        }
        i += DAY_TICKS as usize;
    }
}

/// `RunEconomyVehicleDayProc`: increases the day counter for all vehicles and calls 1-day and
/// 32-day handlers. Each tick, it processes vehicles with
/// "index % DAY_TICKS == `TimerGameEconomy::date_fract`", so each day, all vehicles are
/// processes in `DAY_TICKS` steps.
#[allow(unsafe_code)]
fn run_economy_vehicle_day_proc<const COUNTED: bool>(s: &SchedulerServices) {
    if session::mode() != GameMode::GM_NORMAL {
        return;
    }

    let pool = s.pool();
    let vs = s.vehicle();
    // Run the economy day proc for every DAY_TICKS vehicle starting at
    // TimerGameEconomy::date_fract.
    let mut i = usize::from(timer_game_common::economy_fraction());
    // SAFETY, for the pool reads: the live descriptor, read afresh at each step.
    while i < unsafe { pool.pool_size() } {
        let id = vehicle_base::pool_id(i);
        if let Some(v) = unsafe { vehicle_base::get(&pool, id) } {
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_DAYS);
            }

            // Call the 32-day callback if needed
            if (v.day_counter() & 0x1F) == 0 && v.has_engine_type() {
                if COUNTED {
                    crate::witness::hit(crate::witness::VEHICLE_CALLBACK32);
                }
                let callback = newgrf_vehicle::get_vehicle_callback(
                    svc!(COUNTED, VehicleServices::newgrf, vs),
                    CBID_VEHICLE_32DAY_CALLBACK,
                    0,
                    0,
                    v.engine_type(),
                    Some(v),
                );
                if callback != CALLBACK_FAILED {
                    if has_bit(callback, 0) {
                        // SAFETY: The live vehicle and table.
                        unsafe {
                            newgrf_vehicle::trigger(vs, v, newgrf_vehicle::Trigger::Callback32);
                        } // Trigger vehicle trigger 10
                    }

                    // After a vehicle trigger, the graphics and properties of the vehicle could
                    // change.
                    // Note: MarkDirty also invalidates the palette, which is the meaning of bit 1.
                    // So, nothing special there.
                    if callback != 0 {
                        // SAFETY: The process-lifetime maintenance table.
                        let maintenance = unsafe { &*vs.maintenance };
                        svc!(COUNTED, maintenance.mark_dirty, v.first());
                    }

                    if callback & !3 != 0 {
                        let engine = svc!(COUNTED, vs.engine, v);
                        vs.unknown_callback_result(engine, CBID_VEHICLE_32DAY_CALLBACK, callback);
                    }
                }
            }

            // This is called once per day for each vehicle, but not in the first tick of the day
            on_new_economy_day(s, v, id);
        }
        i += DAY_TICKS as usize;
    }
}

/// `for (Station *st : Station::Iterate()) LoadUnloadStation(st);`. No world/owner borrow
/// spans any station loading service.
#[allow(unsafe_code)]
fn load_stations<const COUNTED: bool>(tables: &Tables) {
    // SAFETY: The caller's live scheduler table.
    let shared = unsafe { &*tables.scheduler }.shared();
    // The station iterator's crossing.
    if COUNTED {
        crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
    }
    shared.for_each_station(|_, service, _| {
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_STATIONS);
        }
        // SAFETY: The live station and the caller's live tables.
        unsafe {
            station_service::load_station(
                service,
                tables.station,
                tables.loading,
                tables.visits,
                tables.refit_cost,
                tables.refit_temporary,
            );
        };
    });
}

/// `CallVehicleTicks`'s loop body after `v->Tick()` returned true: its `switch (v->type)`.
/// `newgrf` is the vehicle resolver, whose slot reads a vehicle's cargo list.
#[allow(unsafe_code)]
fn after_tick<const COUNTED: bool>(
    tables: &Tables,
    vs: &VehicleServices,
    newgrf: &newgrf_vehicle::Services,
    v: VehicleRef,
) {
    if COUNTED {
        crate::witness::hit(crate::witness::VEHICLE_POST_TICK);
    }
    let kind = load!(COUNTED, v.vehicle_type());
    if COUNTED {
        crate::witness::hit(
            crate::witness::VEHICLE_VISITS
                + match kind.0 {
                    0..=6 => usize::from(kind.0),
                    _ => 7,
                },
        );
    }
    match kind {
        VehicleType::VEH_TRAIN
        | VehicleType::VEH_ROAD
        | VehicleType::VEH_AIRCRAFT
        | VehicleType::VEH_SHIP => {}
        _ => return,
    }
    let front = load!(COUNTED, v.first());

    if load!(COUNTED, v.cached_cargo_age_period()) != 0 {
        let counter =
            load!(COUNTED, v.cargo_age_counter()).min(load!(COUNTED, v.cached_cargo_age_period()));
        store!(COUNTED, v.set_cargo_age_counter(counter));
        let counter = counter.wrapping_sub(1);
        store!(COUNTED, v.set_cargo_age_counter(counter));
        if counter == 0 {
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_CARGO_OWNER);
            }
            let cargo = svc!(COUNTED, VehicleRef::cargo, v, newgrf);
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_CARGO_AGES);
            }
            // SAFETY: The surviving vehicle's live list and the process-lifetime cargo table.
            unsafe { cargo_storage::age::<COUNTED>(cargo, tables.cargo) };
            store!(
                COUNTED,
                v.set_cargo_age_counter(load!(COUNTED, v.cached_cargo_age_period()))
            );
        }
    }

    // Do not play any sound when crashed
    if load!(COUNTED, front.vehstatus()).test(VehState::Crashed) {
        return;
    }

    // Do not play any sound when in depot or tunnel
    if load!(COUNTED, v.vehstatus()).test(VehState::Hidden) {
        return;
    }

    // Do not play any sound when stopped
    if load!(COUNTED, front.vehstatus()).test(VehState::Stopped)
        && (load!(COUNTED, front.vehicle_type()) != VehicleType::VEH_TRAIN
            || load!(COUNTED, front.cur_speed()) == 0)
    {
        return;
    }

    if COUNTED {
        crate::witness::hit(crate::witness::VEHICLE_MOTION);
    }
    // Update motion counter for animation purposes.
    store!(
        COUNTED,
        v.set_motion_counter(
            load!(COUNTED, v.motion_counter())
                .wrapping_add(u32::from(load!(COUNTED, front.cur_speed())))
        )
    );

    // Check vehicle type specifics
    match load!(COUNTED, v.vehicle_type()) {
        VehicleType::VEH_TRAIN => {
            let train = train::Handle::from_vehicle(v);
            if !ground_vehicle::is_engine(load!(COUNTED, train.vehicle().subtype())) {
                return;
            }
        }
        VehicleType::VEH_ROAD => {
            // `RoadVehicle::From(v)`.
            assert_with(|| v.vehicle_type() == VehicleType::VEH_ROAD);
            if !load!(COUNTED, v.is_front_engine()) {
                return;
            }
        }
        VehicleType::VEH_AIRCRAFT => {
            // `Aircraft::From(v)`.
            assert_with(|| v.vehicle_type() == VehicleType::VEH_AIRCRAFT);
            if !aircraft::is_normal_aircraft(load!(COUNTED, v.subtype())) {
                return;
            }
        }
        _ => {}
    }

    // Play a running sound if the motion counter passes 256 (Do we not skip sounds?)
    if gb(load!(COUNTED, v.motion_counter()), 0, 8) < u32::from(load!(COUNTED, front.cur_speed())) {
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_RUNNING_SOUND);
        }
        svc!(COUNTED, vs.play_vehicle_sound, v, VSE_RUNNING);
    }

    // Play an alternating running sound every 16 ticks
    if gb_u8(load!(COUNTED, v.tick_counter()), 0, 4) == 0 {
        // Play running sound when speed > 0 and not braking
        let running = (load!(COUNTED, front.cur_speed()) > 0)
            && !load!(COUNTED, front.vehstatus()).any(
                VehStates::from_element(VehState::Stopped)
                    .with(VehStates::from_element(VehState::TrainSlowing)),
            );
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_PERIODIC_SOUND);
        }
        svc!(
            COUNTED,
            vs.play_vehicle_sound,
            v,
            if running {
                VSE_RUNNING_16
            } else {
                VSE_STOPPED_16
            }
        );
    }
}

/// `CallVehicleTicks` up to its autoreplace loop, which `fleet::pending::drain` ports; `cost`
/// is the native frame's command cost storage.
#[allow(unsafe_code)]
fn call_vehicle_ticks<const COUNTED: bool>(tables: &Tables, cost: *mut ()) {
    // SAFETY: The caller's live scheduler table.
    let s = unsafe { &*tables.scheduler };
    // SAFETY: Serial game thread; no pending entry is borrowed.
    unsafe { fleet::pending::clear() };

    run_economy_vehicle_day_proc::<COUNTED>(s);

    {
        let mut storage = MaybeUninit::uninit();
        // The measurer's begin and end are two counted crossings.
        // SAFETY: `CallVehicleTicks` runs in the game loop, serialized by the game-state
        // mutex; the guard drops at the end of this block, as do the resets below.
        let _framerate =
            unsafe { PerformanceMeasurer::new(&mut storage, PerformanceElement::PFE_GL_ECONOMY) };
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
            crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
        }
        load_stations::<COUNTED>(tables);
    }
    // SAFETY: As the measurer's.
    unsafe {
        svc!(
            COUNTED,
            PerformanceAccumulator::reset,
            PerformanceElement::PFE_GL_TRAINS
        );
        svc!(
            COUNTED,
            PerformanceAccumulator::reset,
            PerformanceElement::PFE_GL_ROADVEHS
        );
        svc!(
            COUNTED,
            PerformanceAccumulator::reset,
            PerformanceElement::PFE_GL_SHIPS
        );
        svc!(
            COUNTED,
            PerformanceAccumulator::reset,
            PerformanceElement::PFE_GL_AIRCRAFT
        );
    }

    let vs = s.vehicle();
    let newgrf = svc!(COUNTED, VehicleServices::newgrf, vs);
    let pool = s.pool();
    // `Vehicle::Iterate()`: each step rereads the pool's extent and slot, so a vehicle that
    // `Tick` allocates ahead of the walk, past the old extent included, is visited this tick.
    let mut index = 0;
    // SAFETY, for the pool reads: the live descriptor, read afresh at each step.
    while let Some(id) = unsafe { vehicle_base::next_vehicle(&pool, index) } {
        index = id.index() + 1;
        let v = VehicleRef::of(unsafe { pool.record(id) });
        // `v->index`, which the pool stores each vehicle at, so the walk's `id`; only the two
        // assertions read it. The shell is read before `Tick`, which can delete the record.
        let vehicle_index = id;
        let shell = v.shell();
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_OUTER_VISITS);
        }

        // Vehicle could be deleted in this tick
        if !svc!(COUNTED, s.tick, v) {
            // `Vehicle::Get` also asserts `vehicle_index < first_unused`, where
            // `TypedPool::shell` returns null instead. That assertion holds: the walk yielded
            // the index below `first_unused`, `FreeItem` never lowers it, and `CleanPool` does
            // not run during this entry.
            assert_with(|| unsafe { pool.shell(vehicle_index) }.is_null());
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_DELETED);
            }
            continue;
        }

        assert_with(|| unsafe { pool.shell(vehicle_index) }.cast::<std::ffi::c_void>() == shell);

        after_tick::<COUNTED>(tables, vs, newgrf, v);
    }

    // SAFETY: No pool or record borrow survives into the fleet commands.
    unsafe {
        fleet::pending::drain(tables.pending, tables.transaction, tables.group, cost);
    };
}

/// The session's calendar-day entry: `RunVehicleCalendarDayProc`.
/// # Safety
/// `tables` and the tables it links are live for this call.
#[allow(unsafe_code)]
pub(crate) unsafe fn calendar_day(tables: *const Tables) {
    // SAFETY: The caller's live tables.
    let s = unsafe { &*(*tables).scheduler };
    if crate::witness::enabled() {
        run_vehicle_calendar_day_proc::<true>(s);
    } else {
        run_vehicle_calendar_day_proc::<false>(s);
    }
}
/// The session's tick entry: `CallVehicleTicks`.
/// # Safety
/// As [`calendar_day`]; `cost` is native stack storage valid through the fleet drain.
#[allow(unsafe_code)]
pub(crate) unsafe fn ticks(tables: *const Tables, cost: *mut ()) {
    // SAFETY: The caller's live tables.
    let tables = unsafe { &*tables };
    if crate::witness::enabled() {
        call_vehicle_ticks::<true>(tables, cost);
    } else {
        call_vehicle_ticks::<false>(tables, cost);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleSchedulerServices" ["scheduler"] => SchedulerServices {
        aircraft,
        pool,
        road,
        shared,
        ship,
        tick,
        train,
        vehicle,
    };
    "OpenTTDVehicleSchedulerTables" ["scheduler"] => Tables {
        cargo,
        group,
        loading,
        pending,
        refit_cost,
        refit_temporary,
        scheduler,
        station,
        transaction,
        visits,
    };
}

/// The body after `Tick` over two live records, a part and its front, with recording sound,
/// cargo-list and packet leaves. Each test asserts the stores and calls the original makes.
#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use crate::newgrf_sound::VehicleSoundEvent;
    use crate::vehicle_base::{VehStates, VehicleRecord};
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    const TRAIN: VehicleType = VehicleType::VEH_TRAIN;
    const ROAD: VehicleType = VehicleType::VEH_ROAD;
    const SHIP: VehicleType = VehicleType::VEH_SHIP;
    const AIRCRAFT: VehicleType = VehicleType::VEH_AIRCRAFT;
    /// `GVSF_FRONT` and `GVSF_ENGINE` as subtype bits.
    const FRONT: u8 = 1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_FRONT.0;
    const ENGINE: u8 = 1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_ENGINE.0;
    const CRASHED: VehStates = VehStates::from_element(VehState::Crashed);
    const HIDDEN: VehStates = VehStates::from_element(VehState::Hidden);
    const STOPPED: VehStates = VehStates::from_element(VehState::Stopped);

    /// What the recording leaves saw, and how the running sound changes the records.
    struct Observed {
        events: Vec<&'static str>,
        running: Option<bool>,
        sound_changes: u8,
        packet_reloads: bool,
        cargo: *mut cargo_storage::List,
        packet: cargo_storage::Packet,
        part: *mut VehicleRecord,
        front: *mut VehicleRecord,
    }
    thread_local! {
        static OBSERVED: RefCell<Observed> = const {
            RefCell::new(Observed {
                events: Vec::new(),
                running: None,
                sound_changes: 0,
                packet_reloads: false,
                cargo: null_mut(),
                packet: cargo_storage::Packet {
                    feeder_share: 0,
                    source_xy: 0,
                    count: 1,
                    periods_in_transit: 0,
                    first_station: 0,
                    next_hop: 0,
                    source_id: 0,
                    travelled_x: 0,
                    travelled_y: 0,
                    source_type: 0,
                    in_vehicle: 1,
                },
                part: null_mut(),
                front: null_mut(),
            })
        };
    }
    fn observed<T>(f: impl FnOnce(&mut Observed) -> T) -> T {
        OBSERVED.with(|o| f(&mut o.borrow_mut()))
    }

    /// A part of type `kind` and its front of type `front_kind`, with the original
    /// constructor defaults; the part's own members start as the old fixture's cells did.
    struct Pair {
        part: *mut VehicleRecord,
        front: *mut VehicleRecord,
    }
    impl Pair {
        fn new(kind: VehicleType, front_kind: VehicleType) -> Self {
            let part = VehicleRecord::new(null_mut(), kind);
            let front = VehicleRecord::new(null_mut(), front_kind);
            // SAFETY: Both fresh records; the part's chain starts at the front.
            unsafe {
                (*part).first = front;
                (*part).cargo_age_counter = 9;
                (*part).tick_counter = 1;
                (*part).subtype = FRONT;
            }
            observed(|o| {
                o.events.clear();
                o.running = None;
                o.sound_changes = 0;
                o.packet_reloads = false;
                o.cargo = null_mut();
                o.packet.periods_in_transit = 0;
                o.part = part;
                o.front = front;
            });
            Self { part, front }
        }
        fn part(&self) -> &mut VehicleRecord {
            // SAFETY: The pair's live record; no other reference to it is live.
            unsafe { &mut *self.part }
        }
        fn front(&self) -> &mut VehicleRecord {
            // SAFETY: As `part`.
            unsafe { &mut *self.front }
        }
        /// Run the body after `Tick` over the part.
        fn run(&self) {
            let cargo_services = cargo_services();
            let tables = Tables {
                cargo: &raw const cargo_services,
                ..null_tables()
            };
            let vs = VehicleServices {
                play_vehicle_sound: sound,
                ..crate::test_tables::VEHICLE
            };
            let newgrf = newgrf_vehicle::test_table_with(newgrf_vehicle::TestLeaves {
                cargo_owner,
                ..newgrf_vehicle::TestLeaves::UNREACHABLE
            });
            after_tick::<false>(&tables, &vs, &newgrf, VehicleRef::of(self.part));
        }
    }
    impl Drop for Pair {
        fn drop(&mut self) {
            // SAFETY: The pair's records, released once.
            unsafe {
                VehicleRecord::delete(self.part);
                VehicleRecord::delete(self.front);
            }
        }
    }
    pub(super) fn null_tables() -> Tables {
        Tables {
            cargo: null(),
            group: null(),
            loading: null(),
            pending: null(),
            refit_cost: null_mut(),
            refit_temporary: null_mut(),
            scheduler: null(),
            station: null(),
            transaction: null(),
            visits: null(),
        }
    }

    unsafe extern "C" fn cargo_owner(v: *mut VehicleRecord) -> *const cargo_storage::List {
        observed(|o| {
            assert_eq!(v, o.part);
            o.events.push("age-owner");
            o.cargo
        })
    }
    /// `PlayVehicleSound`. The running sound applies the test's change to the records.
    extern "C" fn sound(v: VehicleRef, event: VehicleSoundEvent) -> bool {
        observed(|o| {
            assert_eq!(v.0.as_ptr(), o.part);
            if event != VSE_RUNNING {
                assert!(event == VSE_RUNNING_16 || event == VSE_STOPPED_16);
                o.events.push("periodic-sound");
                o.running = Some(event == VSE_RUNNING_16);
                return;
            }
            o.events.push("running-sound");
            // SAFETY: The pair's live records; no reference to them is live.
            let (part, front) = unsafe { (&mut *o.part, &mut *o.front) };
            match o.sound_changes {
                1 => {
                    front.cur_speed = 0;
                    part.tick_counter = 0;
                    front.vehstatus = STOPPED;
                }
                2 => {
                    front.cur_speed = 3;
                    part.tick_counter = 0;
                    front.vehstatus = STOPPED;
                }
                3 => part.tick_counter = 1,
                _ => {}
            }
        });
        false
    }
    /// The cargo list's packet lookup; a reloading packet changes the part's period.
    extern "C" fn packet(_: *mut c_void) -> *mut cargo_storage::Packet {
        observed(|o| {
            if o.packet_reloads {
                // SAFETY: The pair's live part; no reference to it is live.
                unsafe { (*o.part).cached_cargo_age_period = 17 };
                o.events.push("age-packet");
            }
            &raw mut o.packet
        })
    }
    extern "C" fn destroy(_: *mut c_void) {}
    extern "C" fn can_allocate() -> u8 {
        1
    }
    extern "C" fn create(_: *const cargo_storage::Packet) -> *mut c_void {
        null_mut()
    }
    extern "C" fn pay(_: *mut c_void, _: u8, _: u8, _: *const c_void, _: u32, _: u32) -> i64 {
        0
    }
    extern "C" fn origin(_: *mut c_void, _: u16, _: u32, _: u8) {}
    extern "C" fn flow_map(_: *const c_void) -> *mut crate::cargo_flow::FlowMap {
        null_mut()
    }
    extern "C" fn packet_next(_: *mut c_void) -> *mut c_void {
        null_mut()
    }
    fn cargo_services() -> cargo_storage::Services {
        cargo_storage::Services {
            can_allocate,
            create,
            packet,
            destroy,
            pay,
            origin,
            flow_map,
            shared: null(),
            packet_next,
        }
    }
    fn events() -> Vec<&'static str> {
        observed(|o| o.events.clone())
    }

    #[test]
    fn cargo_short_circuit_wrap_clamp_and_period_reload() {
        let pair = Pair::new(ROAD, ROAD);
        pair.front().vehstatus = CRASHED;
        pair.run();
        assert_eq!(pair.part().cargo_age_counter, 9);
        assert_eq!(events(), [] as [&str; 0]);
        for (counter, expected) in [(0, u16::MAX), (9, 3)] {
            let pair = Pair::new(ROAD, ROAD);
            pair.part().cached_cargo_age_period = 4;
            pair.part().cargo_age_counter = counter;
            pair.front().vehstatus = CRASHED;
            pair.run();
            assert_eq!(pair.part().cargo_age_counter, expected);
            assert_eq!(events(), [] as [&str; 0]);
        }
        let pair = Pair::new(ROAD, ROAD);
        pair.part().cached_cargo_age_period = 4;
        pair.part().cargo_age_counter = 1;
        pair.front().vehstatus = CRASHED;
        let cargo_services = cargo_services();
        let list = cargo_storage::openttd_rust_cargo_list_new(1);
        // SAFETY: The fresh list; its one packet's shell is never dereferenced.
        unsafe {
            cargo_storage::openttd_rust_cargo_list_append(
                list,
                &raw const cargo_services,
                pair.part.cast(),
                2, // Cargo keep action, as in the existing cargo-storage fixture.
            );
        };
        observed(|o| {
            o.cargo = list;
            o.packet_reloads = true;
        });
        pair.run();
        assert_eq!(pair.part().cargo_age_counter, 17); // The packet changes the later read.
        assert_eq!(observed(|o| o.packet.periods_in_transit), 1);
        assert_eq!(events(), ["age-owner", "age-packet"]);
        // SAFETY: The test's list, destroyed once.
        unsafe { cargo_storage::openttd_rust_cargo_list_destroy(list, &raw const cargo_services) };
    }

    #[test]
    fn non_transport_type_bytes_skip_all_transport_state() {
        for byte in 4..=u8::MAX {
            let pair = Pair::new(VehicleType(byte), ROAD);
            pair.part().cached_cargo_age_period = 7;
            pair.part().cargo_age_counter = 3;
            pair.part().motion_counter = u32::MAX;
            pair.front().cur_speed = 10;
            pair.run();
            assert_eq!(pair.part().cargo_age_counter, 3);
            assert_eq!(pair.part().motion_counter, u32::MAX);
            assert_eq!(events(), [] as [&str; 0]);
        }
    }

    #[test]
    fn hidden_stopped_and_type_gates() {
        let pair = Pair::new(ROAD, ROAD);
        pair.part().vehstatus = HIDDEN;
        pair.run();
        assert_eq!(pair.part().motion_counter, 0);
        assert_eq!(events(), [] as [&str; 0]);
        for (kind, speed) in [(ROAD, 23), (TRAIN, 0)] {
            let pair = Pair::new(kind, kind);
            pair.front().cur_speed = speed;
            pair.front().vehstatus = STOPPED;
            pair.run();
            assert_eq!(pair.part().motion_counter, 0);
            assert_eq!(events(), [] as [&str; 0]);
        }
        for (kind, subtype) in [(ROAD, 0), (TRAIN, 0), (AIRCRAFT, 3)] {
            let pair = Pair::new(kind, kind);
            pair.front().cur_speed = 5;
            pair.part().motion_counter = u32::MAX;
            pair.part().subtype = subtype;
            pair.run();
            assert_eq!(pair.part().motion_counter, 4); // Original unsigned motion wraps.
            assert_eq!(events(), [] as [&str; 0]);
        }
    }

    #[test]
    fn fresh_sound_observations_and_ship_subtype_short_circuit() {
        let pair = Pair::new(ROAD, ROAD);
        pair.front().cur_speed = 8;
        pair.part().motion_counter = 252;
        observed(|o| o.sound_changes = 1);
        pair.run();
        assert_eq!(pair.part().motion_counter, 260);
        assert_eq!(observed(|o| o.running), Some(false));
        assert_eq!(events(), ["running-sound", "periodic-sound"]);
        for change in [2, 3] {
            let pair = Pair::new(ROAD, ROAD);
            pair.front().cur_speed = 8;
            pair.part().motion_counter = 252;
            pair.part().tick_counter = 0;
            observed(|o| o.sound_changes = change);
            pair.run();
            // The changed tick counter of change 3 skips the periodic sound.
            let expected = if change == 2 { Some(false) } else { None };
            assert_eq!(observed(|o| o.running), expected);
        }
        let pair = Pair::new(SHIP, SHIP);
        pair.front().cur_speed = 5;
        pair.part().motion_counter = 17;
        pair.part().tick_counter = 0;
        pair.part().subtype = 0; // A ship has no subtype gate.
        pair.run();
        assert_eq!(observed(|o| o.running), Some(true));
        let pair = Pair::new(TRAIN, TRAIN);
        pair.front().vehstatus = STOPPED;
        pair.front().cur_speed = 5;
        pair.part().subtype = ENGINE;
        pair.part().tick_counter = 0;
        pair.part().motion_counter = 17;
        pair.run(); // A coasting stopped train still increments its motion.
        assert_eq!(pair.part().motion_counter, 22);
        assert_eq!(observed(|o| o.running), Some(false));
    }

}

/// Traversal on the real Rust pool owner: deletion, reuse and growth order; the live walk;
/// and Tick growth with a fresh type. Traces are checked against `scheduler/pool-oracle.txt`,
/// captured from the unchanged original `PoolIterator`.
#[cfg(test)]
#[allow(unsafe_code)]
mod pool_tests {
    use super::*;
    use crate::framerate_type::testing::{self as framerate, Effect};
    use crate::oracle;
    use crate::pool::{NativeOps, PoolConfig, PoolCore};
    use crate::test_tables::{FLEET_GROUPS, FLEET_PENDING, FLEET_TRANSACTIONS, SCHEDULER, VEHICLE};
    use crate::vehicle_base::VehicleRecord;
    use std::cell::{Cell, RefCell};
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    const ORACLE: &str = include_str!("scheduler/pool-oracle.txt");

    /// A test vehicle shell: its pool index and its chain successor.
    #[repr(C)]
    struct Shell {
        index: usize,
        next: *mut Shell,
    }
    const SIZE: usize = size_of::<Shell>();

    thread_local! {
        static POOL: Cell<*mut PoolCore> = const { Cell::new(null_mut()) };
        static VISITS: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
        static GROWN_VISITS: Cell<u32> = const { Cell::new(0) };
        static SURVIVORS: Cell<u32> = const { Cell::new(0) };
        static MUTATE: Cell<bool> = const { Cell::new(false) };
        static GROWTH: Cell<usize> = const { Cell::new(usize::MAX) };
        static STEPS: RefCell<Vec<Step>> = const { RefCell::new(Vec::new()) };
    }
    /// `CallVehicleTicks`'s observable steps before its vehicle loop ends.
    #[derive(Debug, PartialEq, Eq)]
    enum Step {
        Framerate(Effect),
        Stations,
        Tick(usize),
    }
    fn record_framerate(effect: Effect) {
        STEPS.with(|s| s.borrow_mut().push(Step::Framerate(effect)));
    }
    fn pool() -> *mut PoolCore {
        POOL.get()
    }
    fn view() -> *mut crate::pool::PoolDescriptor {
        // SAFETY: The test's live pool owner.
        unsafe { PoolCore::descriptor(pool()) }
    }
    fn slot(index: usize) -> crate::pool::PoolSlot {
        // SAFETY: A slot index below the live slot length.
        unsafe { (*view()).slots.as_slice()[index] }
    }
    fn valid(index: usize) -> bool {
        // SAFETY: The test's live pool owner.
        let len = unsafe { (*view()).slots.len() };
        index < len && !slot(index).shell.is_null()
    }

    unsafe extern "C" fn allocate(_: *mut c_void, size: usize) -> *mut c_void {
        assert_eq!(size, SIZE);
        Box::into_raw(Box::new(Shell {
            index: usize::MAX,
            next: null_mut(),
        }))
        .cast()
    }
    unsafe extern "C" fn deallocate(_: *mut c_void, shell: *mut c_void, size: usize) {
        assert_eq!(size, SIZE);
        // SAFETY: Allocated by `allocate`.
        drop(unsafe { Box::from_raw(shell.cast::<Shell>()) });
    }
    unsafe extern "C" fn set_index(_: *mut c_void, shell: *mut c_void, index: usize) {
        // SAFETY: The shell being allocated.
        unsafe { (&raw mut (*shell.cast::<Shell>()).index).write(index) };
    }
    /// `delete Vehicle *`: `~Vehicle` deletes the chain successor, then the record owner
    /// retires and the sized delete frees the slot.
    unsafe extern "C" fn destroy(_: *mut c_void, shell: *mut c_void) {
        let shell = shell.cast::<Shell>();
        // SAFETY: A live shell of this pool; its record is the slot's.
        unsafe {
            let next = std::mem::replace(&mut (*shell).next, null_mut());
            if !next.is_null() {
                destroy(null_mut(), next.cast());
            }
            let index = (*shell).index;
            let record = slot(index)
                .record
                .cast::<crate::vehicle_base::VehicleRecord>();
            PoolCore::retire_record(pool(), index);
            VehicleRecord::delete(record);
            PoolCore::free_item(pool(), SIZE, index);
        }
    }
    unsafe extern "C" fn post_destructor(_: *mut c_void, _: usize) {}
    unsafe extern "C" fn assertion(_: *mut c_void, condition: bool) {
        assert!(condition);
    }

    /// The test's pool owner, kept at the address `POOL` records.
    struct TestPool {
        _core: Box<PoolCore>,
    }
    /// The vehicle pool's parameters: `Pool<Vehicle, VehicleID, 512>`.
    fn new_pool() -> TestPool {
        let mut core = PoolCore::new(
            PoolConfig {
                max_size: 0xFF000,
                growth_step: 512,
                cache: false,
                item_size: SIZE,
                assertions: true,
            },
            NativeOps {
                context: null_mut(),
                allocate,
                deallocate,
                set_index,
                destroy,
                post_destructor,
                assertion,
            },
        );
        POOL.set(&raw mut *core);
        TestPool { _core: core }
    }
    /// `new (VehicleID(index)) Vehicle(VEH_EFFECT)`: shell, then its fresh canonical record.
    fn new_vehicle(index: usize) -> *mut Shell {
        new_vehicle_of(index, VehicleType::VEH_EFFECT)
    }
    /// `new (VehicleID(index)) Vehicle(kind)`.
    fn new_vehicle_of(index: usize, kind: VehicleType) -> *mut Shell {
        // SAFETY: The test's live pool owner; the index is free.
        unsafe {
            let shell = PoolCore::allocate_at(pool(), SIZE, index).unwrap();
            let record = VehicleRecord::new(shell.cast(), kind);
            PoolCore::bind_record(pool(), index, record.cast());
            shell.cast()
        }
    }
    fn delete_vehicle(index: usize) {
        // SAFETY: A live shell of this pool.
        unsafe { destroy(null_mut(), slot(index).shell) };
    }
    fn record(index: usize) -> *mut crate::vehicle_base::VehicleRecord {
        slot(index).record.cast()
    }
    fn clean() {
        // SAFETY: The test's live pool owner; nothing traverses it during cleanup.
        unsafe { PoolCore::clean(pool()) };
    }
    fn slots_len() -> usize {
        // SAFETY: The test's live pool owner.
        unsafe { (*view()).slots.len() }
    }

    /// The vehicle pool over the test's live owner, read in place.
    fn typed_pool() -> Pool {
        // SAFETY: The test's live pool owner outlives each walk.
        unsafe { Pool::from_descriptor(view()) }
    }
    /// The test's resolver table, which no vehicle of these tests reaches.
    extern "C" fn resolver() -> *const newgrf_vehicle::Services {
        &newgrf_vehicle::TEST_TABLE
    }
    /// `VEHICLE` with the resolver table the tick entry resolves before its loop.
    const VEHICLES: VehicleServices = VehicleServices {
        newgrf: resolver,
        ..VEHICLE
    };
    fn index_of(vehicle: VehicleRef) -> usize {
        // SAFETY: The live record's shell is this test's.
        unsafe { (*(*vehicle.0.as_ptr()).shell.cast::<Shell>()).index }
    }
    fn trace(visits: &[usize]) -> String {
        let growth = GROWTH.get();
        visits
            .iter()
            .map(|&id| match id {
                id if growth != usize::MAX && id == growth => "g".to_owned(),
                id if growth != usize::MAX && id == growth + 1 => "g+1".to_owned(),
                id => id.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// The fixture's `SchedulerLifetimeVehicle::Tick` script on the second visit to ID 2:
    /// reuse behind and ahead, a fresh record for a reused slot, a hole ahead, and growth that
    /// keeps the visiting vehicle's record and scalar addresses.
    fn mutate_at_two(id: usize) {
        new_vehicle(0); // Behind: waits one tick.
        // SAFETY: Record 4 is live until deleted below.
        unsafe {
            (*record(4)).cargo_age_counter = u16::MAX;
            (*record(4)).motion_counter = u32::MAX;
        }
        delete_vehicle(4);
        new_vehicle(4); // Ahead: visited this tick.
        // SAFETY: The reused slot's fresh record.
        let fresh = unsafe { ((*record(4)).cargo_age_counter, (*record(4)).motion_counter) };
        assert_eq!(fresh, (1, 0), "a reused slot gets a fresh record");
        new_vehicle(3); // Inside initialized storage.
        delete_vehicle(5); // Hole ahead must be skipped.
        let growth = slots_len();
        GROWTH.set(growth);
        let own = record(id);
        // SAFETY: The visiting vehicle's live record.
        let (counter, motion) = unsafe {
            (*own).cargo_age_counter = u16::MAX;
            (*own).motion_counter = u32::MAX;
            (
                &raw mut (*own).cargo_age_counter,
                &raw mut (*own).motion_counter,
            )
        };
        new_vehicle(growth);
        new_vehicle(growth + 1);
        assert!(slots_len() > growth);
        assert_eq!(record(id), own, "growth keeps the record");
        // SAFETY: The unchanged live record.
        unsafe {
            assert_eq!(
                (
                    &raw mut (*own).cargo_age_counter,
                    &raw mut (*own).motion_counter
                ),
                (counter, motion)
            );
            assert_eq!(
                ((*own).cargo_age_counter, (*own).motion_counter),
                (u16::MAX, u32::MAX)
            );
        }
    }
    extern "C" fn lifetime_tick(vehicle: VehicleRef) -> bool {
        let id = index_of(vehicle);
        VISITS.with(|v| v.borrow_mut().push(id));
        if !MUTATE.get() {
            return true;
        }
        if id == 0 {
            delete_vehicle(0); // Also deletes the unvisited chain member at ID 1.
            assert!(!valid(1));
            return false;
        }
        if id == 2 {
            mutate_at_two(id);
        }
        if id == GROWTH.get() {
            delete_vehicle(id); // Newly discovered vehicle dies before the next slot is read.
            return false;
        }
        true
    }
    /// `Station::Iterate()` over no stations, marking the iteration.
    extern "C" fn mark_stations(_: *mut c_void, _: crate::services::StationVisitor) {
        STEPS.with(|s| s.borrow_mut().push(Step::Stations));
    }
    extern "C" fn keep_company(_: u8) {}

    /// Run `CallVehicleTicks` (outside normal play: no day work) with `tick`. It clears and
    /// drains the process-wide pending map, so it holds that map's test lock.
    fn run_ticks(tick: extern "C" fn(VehicleRef) -> bool) {
        static FINANCE: crate::company::FinanceServices = crate::company::FinanceServices {
            set_current_company: keep_company,
            ..crate::test_tables::FINANCE
        };
        let _guard = crate::fleet::pending::TEST_LOCK.lock().unwrap();
        let mut shared = crate::services::fixture(null_mut(), draw, no_industry);
        shared.for_each_station = mark_stations;
        let descriptor = view().cast_const();
        let mut scheduler = SCHEDULER;
        scheduler.pool = &raw const descriptor;
        scheduler.tick = tick;
        scheduler.vehicle = &VEHICLES;
        scheduler.shared = &raw const shared;
        let pending = FLEET_PENDING;
        let mut commands = crate::test_tables::COMMANDS;
        commands.finance = &FINANCE;
        let mut transaction = FLEET_TRANSACTIONS;
        transaction.commands = &raw const commands;
        let mut group = FLEET_GROUPS;
        group.shared = &raw const shared;
        let tables = Tables {
            group: &raw const group,
            pending: &raw const pending,
            scheduler: &raw const scheduler,
            transaction: &raw const transaction,
            ..super::tests::null_tables()
        };
        // SAFETY: The tables and the pool outlive the synchronous pass.
        unsafe { ticks(&raw const tables, null_mut()) };
    }
    extern "C" fn draw(_: *mut c_void) -> u32 {
        0
    }
    extern "C" fn no_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }

    /// "Base vehicle scheduler - production deletion, reuse and growth order".
    fn lifetime() -> (String, String) {
        let _pool = new_pool();
        VISITS.with(|v| v.borrow_mut().clear());
        GROWTH.set(usize::MAX);
        let head = new_vehicle(0);
        // SAFETY: Both live shells; the chain models `head->SetNext(...)`.
        unsafe { (*head).next = new_vehicle(1) };
        for id in [2, 4, 5] {
            new_vehicle(id);
        }
        for mutate in [true, false] {
            MUTATE.set(mutate);
            run_ticks(lifetime_tick);
        }
        // SAFETY: The live pool owner.
        let items = unsafe { (*view()).items };
        let visits = VISITS.with(|v| trace(&v.borrow()));
        clean();
        ("lifetime".to_owned(), format!("{items} | {visits}"))
    }

    /// "Base vehicle scheduler - live cursor equals unchanged PoolIterator": mutate the pool
    /// after each visit of the walk.
    fn live_cursor(indexing_growth: bool) -> (String, String) {
        let _pool = new_pool();
        GROWTH.set(usize::MAX);
        for id in [0, 2, 4] {
            new_vehicle(id);
        }
        let walk = typed_pool();
        let mut visits = Vec::new();
        let mut first_unused = 0;
        let mut from = 0;
        // SAFETY: The test's live pool, reread at each step.
        while let Some(next) = unsafe { vehicle_base::next_vehicle(&walk, from) } {
            from = next.index() + 1;
            // SAFETY: As the walk's.
            let vehicle = VehicleRef::of(unsafe { walk.record(next) });
            let id = index_of(vehicle);
            visits.push(id);
            if id == 0 {
                delete_vehicle(0); // Deleted-current and reuse behind the advancing cursor.
                new_vehicle(0);
                delete_vehicle(2);
                new_vehicle(2); // Reuse ahead must still be visited.
                new_vehicle(3);
                new_vehicle(6); // Raise extent without indexing growth.
                // SAFETY: The test's live pool owner.
                first_unused = unsafe { (*view()).first_unused };
            } else if id == 2 {
                delete_vehicle(4); // Hole ahead.
                if indexing_growth {
                    let growth = slots_len();
                    GROWTH.set(growth);
                    new_vehicle(growth); // Growth reallocates the slots.
                    new_vehicle(growth + 1);
                }
            }
        }
        clean();
        // SAFETY: The cleaned live pool; an empty pool ends the walk.
        let empty = unsafe { vehicle_base::next_vehicle(&walk, 0) }.is_none();
        let inputs = format!("indexing growth {indexing_growth}");
        (
            inputs,
            format!("{first_unused} | {} | {}", trace(&visits), u8::from(empty)),
        )
    }

    extern "C" fn ordered_tick(vehicle: VehicleRef) -> bool {
        STEPS.with(|s| s.borrow_mut().push(Step::Tick(index_of(vehicle))));
        true
    }

    /// `CallVehicleTicks` (`vehicle.cpp:961-969`): the economy measurer spans the station
    /// iteration, then the four vehicle accumulators reset, all before the first `Tick`.
    #[test]
    fn station_loading_is_timed_and_accumulators_reset_before_ticks() {
        use PerformanceElement as E;
        let _pool = new_pool();
        GROWTH.set(usize::MAX);
        MUTATE.set(false);
        new_vehicle(0);
        STEPS.with(|s| s.borrow_mut().clear());
        framerate::take();
        framerate::observe(Some(record_framerate));
        run_ticks(ordered_tick);
        framerate::observe(None);
        clean();
        let steps = STEPS.with(|s| std::mem::take(&mut *s.borrow_mut()));
        assert_eq!(
            steps,
            [
                Step::Framerate(Effect::MeasurerBegin(E::PFE_GL_ECONOMY)),
                Step::Stations,
                Step::Framerate(Effect::MeasurerEnd(E::PFE_GL_ECONOMY)),
                Step::Framerate(Effect::AccumulatorReset(E::PFE_GL_TRAINS)),
                Step::Framerate(Effect::AccumulatorReset(E::PFE_GL_ROADVEHS)),
                Step::Framerate(Effect::AccumulatorReset(E::PFE_GL_SHIPS)),
                Step::Framerate(Effect::AccumulatorReset(E::PFE_GL_AIRCRAFT)),
                Step::Tick(0),
            ]
        );
    }

    #[test]
    fn traversal_matches_original_pool_iterator() {
        let sections = oracle::parse(ORACLE);
        assert_eq!(oracle::check(&sections, "lifetime", [lifetime()]), 1);
        assert_eq!(
            oracle::check(&sections, "cursor", [live_cursor(true), live_cursor(false)]),
            2
        );
    }

    /// "Base vehicle scheduler - Tick growth, timing return and fresh type": ID 0 grows the
    /// pool past its old extent and becomes a crashed road vehicle whose cargo ages every 4
    /// ticks during Tick; the newly discovered vehicle deletes itself.
    extern "C" fn growing_tick(vehicle: VehicleRef) -> bool {
        if index_of(vehicle) == 0 {
            GROWTH.set(slots_len());
            new_vehicle(slots_len());
            // SAFETY: The visiting vehicle's live record.
            unsafe {
                let record = vehicle.0.as_ptr();
                (*record).r#type = VehicleType::VEH_ROAD;
                (*record).vehstatus = crate::vehicle_base::VehStates::from_element(
                    crate::vehicle_base::VehState::Crashed,
                );
                (*record).cached_cargo_age_period = 4;
                (*record).cargo_age_counter = 9;
            }
            SURVIVORS.set(SURVIVORS.get() + 1);
            return true;
        }
        if index_of(vehicle) == GROWTH.get() {
            GROWN_VISITS.set(GROWN_VISITS.get() + 1);
        }
        delete_vehicle(index_of(vehicle));
        false
    }
    #[test]
    fn tick_growth_rereads_the_pool_and_the_fresh_type() {
        let _pool = new_pool();
        new_vehicle(0);
        GROWTH.set(usize::MAX);
        GROWN_VISITS.set(0);
        SURVIVORS.set(0);
        run_ticks(growing_tick);
        assert_eq!(
            GROWN_VISITS.get(),
            1,
            "the walk visits the vehicle above the old extent"
        );
        assert_eq!(
            SURVIVORS.get(),
            1,
            "only the survivor reaches its post-Tick reads"
        );
        // The body after Tick read the fresh road type: it clamped the counter to the period,
        // then decremented it, and the crashed front ended the body before any sound.
        // SAFETY: The surviving record.
        assert_eq!(unsafe { (*record(0)).cargo_age_counter }, 3);
        // SAFETY: The live pool owner.
        assert_eq!(unsafe { (*view()).items }, 1);
        clean();
    }

}
