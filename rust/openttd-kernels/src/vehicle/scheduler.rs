/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The vehicle scheduler of `vehicle.cpp`: `RunVehicleCalendarDayProc`,
//! `RunEconomyVehicleDayProc` and `CallVehicleTicks` up to its autoreplace loop, which is
//! [`fleet::pending::drain`], with the dispatch of the virtuals `Tick`, `OnNewCalendarDay` and
//! `OnNewEconomyDay`. The session's two entries take the record [`Tables`], whose
//! [`SchedulerServices`] holds the vehicle pool's descriptor and the controllers' tables.
//!
//! The pool and the vehicle records are read in place: the pool through its descriptor, each
//! member through the `VehicleRef` accessor named after it. No reference into a shell, a record
//! or the pool survives a service that can mutate or reenter: a `VehicleRef` is a pointer, each
//! member is loaded or stored at its use, and a walk rereads the pool's extent and slots at each
//! step. The tables are immutable for the process, and no service an entry calls resets the
//! pool, loads a game or yields the game-state mutex.

use super::{VehicleRef, VehicleServices};
use crate::bitmath_func::{gb, gb_u8, has_bit};
use crate::framerate_type::{PerformanceAccumulator, PerformanceElement, PerformanceMeasurer};
use crate::newgrf_callbacks::{CALLBACK_FAILED, CallbackID};
use crate::newgrf_sound::VehicleSoundEvent as SoundEvent;
use crate::openttd::GameMode;
use crate::pool::{PoolDescriptor, PoolId};
use crate::services_assert::assert_with;
use crate::timer_game_tick::DAY_TICKS;
use crate::vehicle_base::{Pool, VehState, VehStates};
use crate::vehicle_type::{VehicleID, VehicleType};
use crate::{
    aircraft, cargo_storage, disaster, effect, fleet, ground_vehicle, newgrf_vehicle, road,
    road_cmd, services, session, ship_control, station_service, timer_game_common, train,
    vehicle_base,
};
use std::mem::MaybeUninit;

#[repr(C)]
pub struct SchedulerServices {
    /// The aircraft controller's table, immutable for the process.
    pub aircraft: *const aircraft::Leaves,
    /// The disaster vehicle table, immutable for the process.
    pub disaster: *const disaster::Services,
    /// `&_vehicle_pool.descriptor`: the vehicle pool's descriptor, Rust's own record.
    pub pool: *const *const PoolDescriptor,
    /// The road vehicle controller's table, immutable for the process.
    pub road: *const road::Leaves,
    /// The shared table, immutable for the process.
    pub shared: *const services::Services,
    /// The ship controller's table, immutable for the process.
    pub ship: *const ship_control::Leaves,
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
    /// The session frame's road command slot for the vehicle tick; null on the calendar-day
    /// entry.
    pub road_slot: *mut road_cmd::CommandSlot,
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
    pub(crate) fn aircraft(&self) -> &aircraft::Leaves {
        // SAFETY: Each table link is a process-lifetime table.
        unsafe { &*self.aircraft }
    }
    fn disaster(&self) -> &disaster::Services {
        // SAFETY: As `aircraft`.
        unsafe { &*self.disaster }
    }
    pub(crate) fn road(&self) -> &road::Leaves {
        // SAFETY: As `aircraft`.
        unsafe { &*self.road }
    }
    pub(crate) fn shared(&self) -> &services::Services {
        // SAFETY: As `aircraft`.
        unsafe { &*self.shared }
    }
    pub(crate) fn ship(&self) -> &ship_control::Leaves {
        // SAFETY: As `aircraft`.
        unsafe { &*self.ship }
    }
    pub(crate) fn train(&self) -> &train::Leaves {
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

/// `VehicleServices::newgrf`, the resolver getter, as one counted crossing of this file.
fn resolver<const COUNTED: bool>(vs: &VehicleServices) -> &'static newgrf_vehicle::Services {
    let newgrf = vs.newgrf();
    if COUNTED {
        crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
        crate::witness::add(
            crate::witness::VEHICLE_OBSERVED_BYTES,
            size_of::<*const newgrf_vehicle::Services>() as u64,
        );
    }
    newgrf
}

/// `v->Tick()`: the virtual's dispatch; true when the vehicle survives. Other types keep the
/// base class's `return true;`.
fn tick(tables: &Tables, s: &SchedulerServices, v: VehicleRef, id: VehicleID) -> bool {
    match v.vehicle_type() {
        VehicleType::VEH_TRAIN => train::tick(
            train::Handle::from_record(v.0.as_ptr()),
            s.train(),
            s.shared(),
        ),
        VehicleType::VEH_ROAD => road::tick(v, s.road(), tables.road_slot),
        VehicleType::VEH_SHIP => ship_control::scheduler_tick(v, s.ship(), s.shared()),
        VehicleType::VEH_AIRCRAFT => aircraft::tick(s.aircraft(), aircraft::Vehicle::of(v, id)),
        VehicleType::VEH_EFFECT => effect::tick(v, s.vehicle(), s.shared()),
        VehicleType::VEH_DISASTER => disaster::scheduler_tick(s.disaster(), v),
        _ => true,
    }
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
        VehicleType::VEH_SHIP => ship_control::on_new_calendar_day(v, s.ship()),
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
        VehicleType::VEH_SHIP => ship_control::on_new_economy_day(v, s.ship(), s.shared()),
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
/// "`index % DAY_TICKS == TimerGameEconomy::date_fract`", so each day, all vehicles are
/// processes in `DAY_TICKS` steps.
#[allow(unsafe_code)]
#[allow(
    clippy::verbose_bit_mask,
    reason = "RunEconomyVehicleDayProc's (v->day_counter & 0x1F) == 0"
)]
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
                    resolver::<COUNTED>(vs),
                    CallbackID::CBID_VEHICLE_32DAY_CALLBACK,
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
                        let maintenance = crate::vehicle::maintenance::Services::of(vs);
                        svc!(COUNTED, maintenance.mark_dirty, v.first());
                    }

                    if callback & !3 != 0 {
                        let engine = svc!(COUNTED, vs.engine, v);
                        vs.unknown_callback_result(
                            engine,
                            CallbackID::CBID_VEHICLE_32DAY_CALLBACK,
                            callback,
                        );
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

/// The cargo ageing of a transport vehicle that survived its `Tick`. `newgrf` is the vehicle
/// resolver, whose slot reads the vehicle's cargo list.
#[allow(unsafe_code)]
fn age_cargo<const COUNTED: bool>(
    tables: &Tables,
    newgrf: &newgrf_vehicle::Services,
    v: VehicleRef,
) {
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
}

/// The loop body of [`call_vehicle_ticks`] after `v->Tick()` returned true: its
/// `switch (v->type)`.
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
        crate::witness::hit(match kind {
            VehicleType::VEH_TRAIN => crate::witness::VEHICLE_VISITS,
            VehicleType::VEH_ROAD => crate::witness::VEHICLE_VISITS_ROAD,
            VehicleType::VEH_SHIP => crate::witness::VEHICLE_VISITS_SHIP,
            VehicleType::VEH_AIRCRAFT => crate::witness::VEHICLE_VISITS_AIRCRAFT,
            VehicleType::VEH_EFFECT => crate::witness::VEHICLE_VISITS_EFFECT,
            VehicleType::VEH_DISASTER => crate::witness::VEHICLE_VISITS_DISASTER,
            VehicleType::VEH_END => crate::witness::VEHICLE_VISITS_END,
            _ => crate::witness::VEHICLE_VISITS_INVALID,
        });
    }
    match kind {
        VehicleType::VEH_TRAIN
        | VehicleType::VEH_ROAD
        | VehicleType::VEH_AIRCRAFT
        | VehicleType::VEH_SHIP => {}
        _ => return,
    }
    let front = load!(COUNTED, v.first());

    age_cargo::<COUNTED>(tables, newgrf, v);

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
    // The switch's read of `v->type` serves the `From` casts' type assertions and
    // `IsFrontEngine`'s ground test: nothing between them writes, calls or draws.
    let kind = load!(COUNTED, v.vehicle_type());
    match kind {
        VehicleType::VEH_TRAIN => {
            // `Train::From(v)`.
            train::Handle::assert_from(|| kind);
            if !ground_vehicle::is_engine(load!(COUNTED, v.subtype())) {
                return;
            }
        }
        VehicleType::VEH_ROAD => {
            // `RoadVehicle::From(v)`.
            assert_with(|| kind == VehicleType::VEH_ROAD);
            if !vehicle_base::is_front_engine(
                vehicle_base::is_ground_vehicle(kind),
                load!(COUNTED, v.subtype()),
            ) {
                return;
            }
        }
        VehicleType::VEH_AIRCRAFT => {
            // `Aircraft::From(v)`.
            assert_with(|| kind == VehicleType::VEH_AIRCRAFT);
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
        svc!(COUNTED, vs.play_vehicle_sound, v, SoundEvent::VSE_RUNNING);
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
                SoundEvent::VSE_RUNNING_16
            } else {
                SoundEvent::VSE_STOPPED_16
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
    let newgrf = resolver::<COUNTED>(vs);
    let pool = s.pool();
    // `Vehicle::Iterate()`: each step rereads the pool's extent and slot, so a vehicle that
    // `Tick` allocates ahead of the walk, past the old extent included, is visited this tick.
    let mut index = 0;
    // SAFETY, for the pool reads: the live descriptor, read afresh at each step.
    // One slot load per step serves the iterator's validity test and its `*it`: nothing
    // between them writes, calls or draws.
    while let Some((id, shell, v)) = unsafe { vehicle_base::next_vehicle(&pool, index) } {
        index = id.index() + 1;
        // `v->index`, which the pool stores each vehicle at, so the walk's `id`; only the two
        // assertions read it. `v` is the slot's shell, the iterator's `Vehicle::Get(index)`.
        let vehicle_index = id;
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_OUTER_VISITS);
        }

        // Vehicle could be deleted in this tick
        if !tick(tables, s, v, id) {
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

        // `Tick` can delete or allocate vehicles, so the slot is read again.
        assert_with(|| unsafe { pool.shell(vehicle_index) } == shell);

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
        disaster,
        pool,
        road,
        shared,
        ship,
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
        road_slot,
        scheduler,
        station,
        transaction,
        visits,
    };
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    /// The body after `Tick` over two live records, a part and its front, with recording sound,
    /// cargo-list and packet leaves. Each test asserts the stores and calls the original makes.
    mod post_tick {
        use super::super::*;
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
            fn part(&mut self) -> &mut VehicleRecord {
                // SAFETY: The pair's live record; no other reference to it is live.
                unsafe { &mut *self.part }
            }
            fn front(&mut self) -> &mut VehicleRecord {
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
                let newgrf = newgrf_vehicle::TEST_TABLE.with_cargo_owner(cargo_owner);
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
                road_slot: null_mut(),
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
                if event != SoundEvent::VSE_RUNNING {
                    assert!(
                        event == SoundEvent::VSE_RUNNING_16 || event == SoundEvent::VSE_STOPPED_16
                    );
                    o.events.push("periodic-sound");
                    o.running = Some(event == SoundEvent::VSE_RUNNING_16);
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
        pub(super) fn cargo_services() -> cargo_storage::Services {
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
            let mut pair = Pair::new(ROAD, ROAD);
            pair.front().vehstatus = CRASHED;
            pair.run();
            assert_eq!(pair.part().cargo_age_counter, 9);
            assert_eq!(events(), [] as [&str; 0]);
            for (counter, expected) in [(0, u16::MAX), (9, 3)] {
                let mut pair = Pair::new(ROAD, ROAD);
                pair.part().cached_cargo_age_period = 4;
                pair.part().cargo_age_counter = counter;
                pair.front().vehstatus = CRASHED;
                pair.run();
                assert_eq!(pair.part().cargo_age_counter, expected);
                assert_eq!(events(), [] as [&str; 0]);
            }
            let mut pair = Pair::new(ROAD, ROAD);
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
                    u16::from(crate::cargopacket::MoveToAction::MTA_KEEP.0),
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
            unsafe {
                cargo_storage::openttd_rust_cargo_list_destroy(list, &raw const cargo_services);
            }
        }

        #[test]
        fn non_transport_type_bytes_skip_all_transport_state() {
            for byte in 4..=u8::MAX {
                let mut pair = Pair::new(VehicleType(byte), ROAD);
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
            let mut pair = Pair::new(ROAD, ROAD);
            pair.part().vehstatus = HIDDEN;
            pair.run();
            assert_eq!(pair.part().motion_counter, 0);
            assert_eq!(events(), [] as [&str; 0]);
            for (kind, speed) in [(ROAD, 23), (TRAIN, 0)] {
                let mut pair = Pair::new(kind, kind);
                pair.front().cur_speed = speed;
                pair.front().vehstatus = STOPPED;
                pair.run();
                assert_eq!(pair.part().motion_counter, 0);
                assert_eq!(events(), [] as [&str; 0]);
            }
            for (kind, subtype) in [(ROAD, 0), (TRAIN, 0), (AIRCRAFT, 3)] {
                let mut pair = Pair::new(kind, kind);
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
            let mut pair = Pair::new(ROAD, ROAD);
            pair.front().cur_speed = 8;
            pair.part().motion_counter = 252;
            observed(|o| o.sound_changes = 1);
            pair.run();
            assert_eq!(pair.part().motion_counter, 260);
            assert_eq!(observed(|o| o.running), Some(false));
            assert_eq!(events(), ["running-sound", "periodic-sound"]);
            for change in [2, 3] {
                let mut pair = Pair::new(ROAD, ROAD);
                pair.front().cur_speed = 8;
                pair.part().motion_counter = 252;
                pair.part().tick_counter = 0;
                observed(|o| o.sound_changes = change);
                pair.run();
                // The changed tick counter of change 3 skips the periodic sound.
                let expected = if change == 2 { Some(false) } else { None };
                assert_eq!(observed(|o| o.running), expected);
            }
            let mut pair = Pair::new(SHIP, SHIP);
            pair.front().cur_speed = 5;
            pair.part().motion_counter = 17;
            pair.part().tick_counter = 0;
            pair.part().subtype = 0; // A ship has no subtype gate.
            pair.run();
            assert_eq!(observed(|o| o.running), Some(true));
            let mut pair = Pair::new(TRAIN, TRAIN);
            pair.front().vehstatus = STOPPED;
            pair.front().cur_speed = 5;
            pair.part().subtype = ENGINE;
            pair.part().tick_counter = 0;
            pair.part().motion_counter = 17;
            pair.run(); // A coasting stopped train still increments its motion.
            assert_eq!(pair.part().motion_counter, 22);
            assert_eq!(observed(|o| o.running), Some(false));
        }

        /// `Train::From`, `RoadVehicle::From` and `Aircraft::From` assert the vehicle's type; inside
        /// their `case` it holds, so the native policy records no failure on each type's own path.
        #[test]
        fn subtype_casts_assert_their_own_type() {
            use std::sync::atomic::{AtomicU32, Ordering};
            static FAILURES: AtomicU32 = AtomicU32::new(0);
            extern "C" fn record(condition: bool) {
                if !condition {
                    FAILURES.fetch_add(1, Ordering::Relaxed);
                }
            }
            if crate::services_assert::isolate_test_check(
                "vehicle::scheduler::tests::post_tick::subtype_casts_assert_their_own_type",
                record,
            ) {
                return;
            }
            for (kind, subtype) in [
                (TRAIN, ENGINE),
                (ROAD, FRONT),
                (AIRCRAFT, crate::aircraft::AircraftSubType::AIR_AIRCRAFT.0),
            ] {
                let mut pair = Pair::new(kind, kind);
                pair.front().cur_speed = 5;
                pair.part().motion_counter = 254; // 259 passes 256: GB(259, 0, 8) < 5.
                pair.part().subtype = subtype;
                pair.run();
                assert_eq!(events(), ["running-sound"], "{kind:?} reaches its sounds");
            }
            assert_eq!(FAILURES.load(Ordering::Relaxed), 0);
        }
    }

    /// Traversal on the real Rust pool owner: deletion, reuse and growth order; the live walk;
    /// and Tick growth with a fresh type. Traces are checked against `scheduler/pool-oracle.txt`,
    /// captured from the unchanged original `PoolIterator`.
    mod pool {
        use super::super::*;
        use crate::framerate_type::testing::{self as framerate, Effect};
        use crate::oracle;
        use crate::pool::{NativeOps, PoolConfig, PoolCore};
        use crate::test_tables::{
            FLEET_GROUPS, FLEET_PENDING, FLEET_TRANSACTIONS, SCHEDULER, VEHICLE,
        };
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
            /// What an effect's sprite read does: the visit's script.
            static SPRITE: Cell<fn(VehicleRef)> = const { Cell::new(lifetime_visit) };
            /// What an effect's viewport update, its procedure's last service call, does.
            static VIEWPORT: Cell<fn(VehicleRef)> = const { Cell::new(|_| {}) };
            /// Whether `delete v` of an expired effect deletes it.
            static DELETES: Cell<bool> = const { Cell::new(true) };
            /// Whether deleting a vehicle keeps its record and owner allocated.
            static KEEPS_RECORDS: Cell<bool> = const { Cell::new(false) };
        }
        /// `CallVehicleTicks`'s observable steps before its vehicle loop ends.
        #[derive(Debug, PartialEq, Eq)]
        enum Step {
            Framerate(Effect),
            Stations,
            /// An effect's procedure read its sprite.
            Tick(usize),
            /// `delete v` of an expired effect, with its owner's `animation_state` and its
            /// `progress` then.
            Destroy {
                id: usize,
                animation_state: u16,
                progress: u8,
            },
            /// The post-`Tick` body's `v->cargo.AgeCargo()`.
            AgeCargo(usize),
            /// The `tick_counter` of the vehicle at ID 1 when a timing scope ended.
            EndedAtTickCounter(u8),
        }
        fn record_framerate(effect: Effect) {
            STEPS.with(|s| s.borrow_mut().push(Step::Framerate(effect)));
        }
        /// `record_framerate`, and at a scope's end the tick count of the vehicle at ID 1.
        fn record_framerate_and_count(effect: Effect) {
            record_framerate(effect);
            if let Effect::AccumulatorEnd(_) = effect {
                // SAFETY: The test's live record at ID 1.
                let counter = unsafe { (*record(1)).tick_counter };
                STEPS.with(|s| s.borrow_mut().push(Step::EndedAtTickCounter(counter)));
            }
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
                if !KEEPS_RECORDS.get() {
                    let owner = (*record).effect;
                    if !owner.is_null() {
                        crate::effect::openttd_rust_effect_destroy(owner);
                    }
                    VehicleRecord::delete(record);
                }
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
        /// `new (VehicleID(index)) EffectVehicle()` of type `EV_BREAKDOWN_SMOKE`: shell, then its
        /// fresh canonical record and its owner, with a `progress` whose next tick reads the
        /// sprite. Its `animation_state` 0 survives `BreakdownSmokeTick`.
        fn new_vehicle(index: usize) -> *mut Shell {
            let shell = new_vehicle_of(index, VehicleType::VEH_EFFECT);
            let fresh = record(index);
            // SAFETY: The fresh record.
            unsafe {
                (*fresh).effect = crate::effect::openttd_rust_effect_new();
                (*fresh).subtype =
                    crate::effectvehicle_func::EffectVehicleType::EV_BREAKDOWN_SMOKE.0;
                (*fresh).progress = 7;
            }
            shell
        }
        /// Give the effect at `index` the `animation_state` 1 with which its next tick expires it.
        fn expire(index: usize) {
            // SAFETY: The live effect's owner.
            unsafe { crate::effect::openttd_rust_effect_set_animation((*record(index)).effect, 1) };
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
        /// `BreakdownSmokeTick`'s sprite read: the test's script for the visit. It returns a
        /// sprite other than the animation's last.
        extern "C" fn first_sprite(vehicle: VehicleRef) -> u32 {
            SPRITE.get()(vehicle);
            0
        }
        extern "C" fn write_first_sprite(_: VehicleRef, _: crate::gfx_type::SpriteID) {}
        extern "C" fn viewport(vehicle: VehicleRef) {
            VIEWPORT.get()(vehicle);
        }
        /// `delete v` of an expired effect.
        extern "C" fn destroy_effect(vehicle: VehicleRef) {
            let id = index_of(vehicle);
            // SAFETY: The expired effect's live owner and record.
            let (animation_state, progress) = unsafe {
                (
                    crate::effect::openttd_rust_effect_get_animation((*vehicle.0.as_ptr()).effect),
                    (*vehicle.0.as_ptr()).progress,
                )
            };
            STEPS.with(|s| {
                s.borrow_mut().push(Step::Destroy {
                    id,
                    animation_state,
                    progress,
                });
            });
            if DELETES.get() {
                delete_vehicle(id);
            }
        }
        /// `VEHICLE` with the resolver table the tick entry resolves before its loop and the
        /// services `BreakdownSmokeTick` calls.
        const VEHICLES: VehicleServices = VehicleServices {
            base_viewport: viewport,
            destroy: destroy_effect,
            first_sprite,
            newgrf: resolver,
            write_first_sprite,
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
        /// The fixture's visit script inside an effect's tick: record the visit, keep the sprite
        /// read for the next tick and, on the mutating tick, run ID 2's script. ID 0 and the
        /// grown ID expire on the mutating tick: deleting ID 0 also deletes the unvisited chain
        /// member at ID 1, and the newly discovered vehicle dies before the next slot is read.
        fn lifetime_visit(vehicle: VehicleRef) {
            let id = index_of(vehicle);
            VISITS.with(|v| v.borrow_mut().push(id));
            // SAFETY: The visiting vehicle's live record.
            unsafe { (*vehicle.0.as_ptr()).progress = 7 };
            if MUTATE.get() && id == 2 {
                assert!(!valid(1), "deleting ID 0 deleted its chain member");
                mutate_at_two(id);
                expire(GROWTH.get());
            }
        }
        /// `Station::Iterate()` over no stations, marking the iteration.
        extern "C" fn mark_stations(_: *mut c_void, _: crate::services::StationVisitor) {
            STEPS.with(|s| s.borrow_mut().push(Step::Stations));
        }
        extern "C" fn keep_company(_: u8) {}

        /// Run `CallVehicleTicks` (outside normal play: no day work) over the effects' tables. It
        /// clears and drains the process-wide pending map, so it holds that map's test lock.
        fn run_ticks() {
            run_ticks_with(&VEHICLES, null(), null());
        }
        /// `run_ticks` with the base vehicle table `vehicle`, the road controller's table `road`
        /// and the cargo table `cargo`.
        fn run_ticks_with(
            vehicle: &VehicleServices,
            road: *const road::Leaves,
            cargo: *const cargo_storage::Services,
        ) {
            static FINANCE: crate::company::FinanceServices = crate::company::FinanceServices {
                set_current_company: keep_company,
                ..crate::test_tables::FINANCE
            };
            let _guard = crate::fleet::pending::TEST_LOCK.lock().unwrap();
            let mut shared = crate::services::fixture(null_mut(), draw);
            shared.for_each_station = mark_stations;
            let descriptor = view().cast_const();
            let mut scheduler = SCHEDULER;
            scheduler.pool = &raw const descriptor;
            scheduler.road = road;
            scheduler.vehicle = vehicle;
            scheduler.shared = &raw const shared;
            let pending = FLEET_PENDING;
            let mut commands = crate::test_tables::COMMANDS;
            commands.finance = &FINANCE;
            let mut transaction = FLEET_TRANSACTIONS;
            transaction.commands = &raw const commands;
            let mut group = FLEET_GROUPS;
            group.shared = &raw const shared;
            let tables = Tables {
                cargo,
                group: &raw const group,
                pending: &raw const pending,
                scheduler: &raw const scheduler,
                transaction: &raw const transaction,
                ..super::post_tick::null_tables()
            };
            // SAFETY: The tables and the pool outlive the synchronous pass.
            unsafe { ticks(&raw const tables, null_mut()) };
        }
        extern "C" fn draw(_: *mut c_void) -> u32 {
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
            expire(0);
            for mutate in [true, false] {
                MUTATE.set(mutate);
                run_ticks();
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
            while let Some((next, _, vehicle)) = unsafe { vehicle_base::next_vehicle(&walk, from) }
            {
                from = next.index() + 1;
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

        fn ordered_visit(vehicle: VehicleRef) {
            STEPS.with(|s| s.borrow_mut().push(Step::Tick(index_of(vehicle))));
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
            SPRITE.set(ordered_visit);
            framerate::take();
            framerate::observe(Some(record_framerate));
            run_ticks();
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

        /// "Base vehicle scheduler - Tick growth, timing return and fresh type": ID 0's tick
        /// grows the pool past its old extent with an expiring effect; the newly discovered
        /// vehicle expires on its visit.
        fn growing_visit(vehicle: VehicleRef) {
            if index_of(vehicle) == 0 {
                GROWTH.set(slots_len());
                new_vehicle(slots_len());
                expire(GROWTH.get());
                return;
            }
            if index_of(vehicle) == GROWTH.get() {
                GROWN_VISITS.set(GROWN_VISITS.get() + 1);
            }
        }
        /// ID 0's last service call in its tick makes it a crashed road vehicle whose cargo
        /// ages every 4 ticks.
        fn retyping_viewport(vehicle: VehicleRef) {
            if index_of(vehicle) != 0 {
                return;
            }
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
        }
        #[test]
        fn tick_growth_rereads_the_pool_and_the_fresh_type() {
            let _pool = new_pool();
            new_vehicle(0);
            GROWTH.set(usize::MAX);
            GROWN_VISITS.set(0);
            SURVIVORS.set(0);
            SPRITE.set(growing_visit);
            VIEWPORT.set(retyping_viewport);
            run_ticks();
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

        /// The cargo list of a road part whose post-`Tick` body ages its cargo.
        unsafe extern "C" fn ageing_cargo(v: *mut VehicleRecord) -> *const cargo_storage::List {
            thread_local! {
                static LIST: *mut cargo_storage::List = cargo_storage::openttd_rust_cargo_list_new(1);
            }
            let id = index_of(VehicleRef::of(v));
            STEPS.with(|s| s.borrow_mut().push(Step::AgeCargo(id)));
            LIST.with(|l| *l)
        }
        extern "C" fn ageing_resolver() -> *const newgrf_vehicle::Services {
            thread_local! {
                static RESOLVER: &'static newgrf_vehicle::Services =
                    Box::leak(Box::new(newgrf_vehicle::TEST_TABLE.with_cargo_owner(ageing_cargo)));
            }
            RESOLVER.with(|r| *r)
        }

        /// `CallVehicleTicks` dispatches `Tick` on the type and runs the post-`Tick` body only for
        /// a survivor, after the body's timing scope ended: an expiring effect's procedure
        /// stores its `progress` and `animation_state`, then deletes it (`effectvehicle.cpp`,
        /// `BreakdownSmokeTick`), and the walk skips its post-`Tick` body; a road part's
        /// `RoadVehicle::Tick` begins and ends `PFE_GL_ROADVEHS` (`roadveh_cmd.cpp`) before the
        /// post-`Tick` body ages its cargo (`vehicle.cpp`).
        #[test]
        fn tick_dispatches_each_body_and_its_timing_scope_ends_before_the_post_tick_body() {
            use PerformanceElement as E;
            // The walk reads the process-wide game mode, which other tests write.
            if crate::services_assert::isolate_test_check(
                "vehicle::scheduler::tests::pool::tick_dispatches_each_body_and_its_timing_scope_ends_before_the_post_tick_body",
                record_failure,
            ) {
                return;
            }
            let _pool = new_pool();
            new_vehicle(0);
            expire(0);
            let part = road_part(1, EngineID(0), 0);
            // SAFETY: The fresh record.
            unsafe {
                (*part).cached_cargo_age_period = 1;
                (*part).vehstatus = VehStates::from_element(VehState::Crashed);
            }
            let vehicles = VehicleServices {
                newgrf: ageing_resolver,
                ..VEHICLES
            };
            let cargo = super::post_tick::cargo_services();
            STEPS.with(|s| s.borrow_mut().clear());
            SPRITE.set(ordered_visit);
            framerate::take();
            FAILURES.store(0, Ordering::Relaxed);
            framerate::observe(Some(record_framerate_and_count));
            run_ticks_with(&vehicles, day_scheduler(VEHICLES).road, &raw const cargo);
            framerate::observe(None);
            assert_eq!(
                FAILURES.load(Ordering::Relaxed),
                0,
                "no failed native assertion"
            );
            let steps = STEPS.with(|s| std::mem::take(&mut *s.borrow_mut()));
            assert_eq!(
                steps[7..],
                [
                    Step::Tick(0),
                    Step::Destroy {
                        id: 0,
                        animation_state: 0,
                        progress: 8
                    },
                    Step::Framerate(Effect::AccumulatorBegin(E::PFE_GL_ROADVEHS)),
                    Step::Framerate(Effect::AccumulatorEnd(E::PFE_GL_ROADVEHS)),
                    // The part's tick count lies inside the scope.
                    Step::EndedAtTickCounter(1),
                    Step::AgeCargo(1),
                ]
            );
            // SAFETY: The surviving part's record.
            assert_eq!(unsafe { (*part).tick_counter }, 1);
            clean();
        }

        use crate::engine_type::EngineID;
        use std::sync::atomic::{AtomicU32, Ordering};

        /// The failed native assertions an isolated child records.
        static FAILURES: AtomicU32 = AtomicU32::new(0);
        extern "C" fn record_failure(condition: bool) {
            if !condition {
                FAILURES.fetch_add(1, Ordering::Relaxed);
            }
        }

        /// An effect's tick that deletes its own vehicle from the pool, keeping its record and
        /// owner allocated for the rest of its procedure, which then reports survival.
        fn deleting_survivor_visit(vehicle: VehicleRef) {
            KEEPS_RECORDS.set(true);
            delete_vehicle(index_of(vehicle));
        }
        /// `assert(Vehicle::Get(vehicle_index) == v)` after a `Tick` that deleted its vehicle and
        /// reported survival. The body after `Tick` would read the freed record, so the native
        /// policy must end the process at the failed assertion rather than return into it.
        #[test]
        fn a_deleting_tick_that_reports_survival_fails_the_native_assertion() {
            if crate::services_assert::isolate_test_exit(
                "vehicle::scheduler::tests::pool::a_deleting_tick_that_reports_survival_fails_the_native_assertion",
            ) {
                return;
            }
            let _pool = new_pool();
            new_vehicle(0);
            SPRITE.set(deleting_survivor_visit);
            run_ticks();
        }

        /// `assert(Vehicle::Get(vehicle_index) == nullptr)` after a false `Tick` and `== v` after a
        /// true one run through the native policy: the two honest results record no failure, and
        /// a `Tick` that reports a deletion it did not make records one.
        #[test]
        fn survival_assertions_run_through_the_native_policy() {
            if crate::services_assert::isolate_test_check(
                "vehicle::scheduler::tests::pool::survival_assertions_run_through_the_native_policy",
                record_failure,
            ) {
                return;
            }
            // Whether the effect expires, whether its `delete v` deletes it, and the failures.
            let ticks = [(false, true, 0), (true, true, 0), (true, false, 1)];
            SPRITE.set(|_| {});
            for (expires, deletes, failures) in ticks {
                let _pool = new_pool();
                new_vehicle(0);
                if expires {
                    expire(0);
                }
                DELETES.set(deletes);
                FAILURES.store(0, Ordering::Relaxed);
                run_ticks();
                assert_eq!(FAILURES.load(Ordering::Relaxed), failures);
                clean();
            }
        }

        /// What the day procedures' recording leaves saw, in order.
        #[derive(Debug, PartialEq, Eq)]
        enum Day {
            /// The 32-day callback, asked for that engine.
            Callback(crate::engine_type::EngineID),
            /// `TriggerVehicleRandomisation`'s store of the waiting triggers.
            Trigger(u8),
            Dirty(*mut VehicleRecord),
            /// `v->GetGRFID()`'s engine lookup.
            Engine,
            /// `ErrorUnknownCallbackResult(grfid, callback, result)`.
            Unknown(u16, u16),
        }
        thread_local! {
            static DAY: RefCell<Vec<Day>> = const { RefCell::new(Vec::new()) };
            static ANSWER: Cell<u16> = const { Cell::new(CALLBACK_FAILED) };
            static ENGINE: Cell<*mut crate::vehicle::OpenTTDEngine> = const { Cell::new(null_mut()) };
        }
        fn day(event: Day) {
            DAY.with(|d| d.borrow_mut().push(event));
        }
        fn take_days() -> Vec<Day> {
            DAY.with(|d| std::mem::take(&mut *d.borrow_mut()))
        }
        unsafe extern "C" fn resolve_callback(
            _: *mut newgrf_vehicle::State,
            id: u16,
            callback: u16,
            param1: u32,
            param2: u32,
            _: *mut i32,
            _: usize,
        ) -> u16 {
            assert_eq!(
                (callback, param1, param2),
                (CallbackID::CBID_VEHICLE_32DAY_CALLBACK.0, 0, 0)
            );
            day(Day::Callback(EngineID(id)));
            ANSWER.get()
        }
        unsafe extern "C" fn engine_by_id(_: u16) -> *const crate::vehicle::OpenTTDEngine {
            ENGINE.get()
        }
        unsafe extern "C" fn cargo_type(_: *mut VehicleRecord) -> u8 {
            0
        }
        unsafe extern "C" fn cache(_: *mut VehicleRecord) -> *mut newgrf_vehicle::Cache {
            Box::leak(Box::default())
        }
        unsafe extern "C" fn write_waiting(_: *mut VehicleRecord, waiting: u8) {
            if waiting != 0 {
                day(Day::Trigger(waiting));
            }
        }
        extern "C" fn unknown_refit(grfid: u32, callback: u16, result: u16) {
            assert_eq!(grfid, 0); // The test's engine has no GRF file.
            day(Day::Unknown(callback, result));
        }
        extern "C" fn engine_loading() -> *const crate::engine_loading::Services {
            const LOADING: crate::engine_loading::Services =
                crate::engine_loading::test_table_with_unknown_refit(unknown_refit);
            &LOADING
        }
        extern "C" fn mark_dirty(v: VehicleRef) {
            day(Day::Dirty(v.0.as_ptr()));
        }
        extern "C" fn vehicle_engine(_: VehicleRef) -> crate::vehicle::EngineRef {
            day(Day::Engine);
            crate::vehicle::EngineRef(std::ptr::NonNull::new(ENGINE.get()).unwrap())
        }
        extern "C" fn day_resolver() -> *const newgrf_vehicle::Services {
            const CAPABILITIES: crate::engine_capabilities::Services =
                crate::engine_capabilities::test_table_with_loading(engine_loading);
            thread_local! {
                static RESOLVER: &'static newgrf_vehicle::Services =
                    Box::leak(Box::new(
                        newgrf_vehicle::TEST_TABLE
                            .with_cache(cache)
                            .with_capabilities(&CAPABILITIES)
                            .with_cargo_type(cargo_type)
                            .with_engine_by_id(engine_by_id)
                            .with_resolve_callback(resolve_callback)
                            .with_shared(day_shared())
                            .with_write_waiting(write_waiting)
                            .with_inert_randomisation(),
                    ));
            }
            RESOLVER.with(|r| *r)
        }
        /// A shared table over this thread's empty fixture map.
        fn day_shared() -> &'static crate::services::Services {
            thread_local! {
                static SHARED: &'static crate::services::Services = {
                    crate::map_func::fixture::set(
                        crate::map_func::TileTypeAllocation {
                            base: null(),
                            extended: null(),
                            count: 0,
                            assertions: true,
                        },
                        6,
                    );
                    let mut shared = crate::services::fixture(null_mut(), draw);
                    shared.map = crate::map_func::fixture::statics();
                    Box::leak(Box::new(shared))
                };
            }
            SHARED.with(|s| *s)
        }
        /// The day procedures' table over the test pool: the base vehicle table `vehicle`, and
        /// controller tables over it and the shared table whose own leaves trap.
        fn day_scheduler(vehicle: VehicleServices) -> SchedulerServices {
            let shared = day_shared();
            let vehicle: &'static VehicleServices = Box::leak(Box::new(vehicle));
            SchedulerServices {
                aircraft: Box::leak(Box::new(aircraft::Leaves {
                    shared,
                    vehicle,
                    ..crate::test_tables::AIRCRAFT_LEAVES
                })),
                disaster: null(),
                pool: Box::leak(Box::new(view().cast_const())),
                road: Box::leak(Box::new(road::Leaves {
                    ground: &crate::test_tables::ROAD_GROUND,
                    shared,
                    vehicle,
                    ..crate::test_tables::ROAD_LEAVES
                })),
                shared,
                ship: Box::leak(Box::new(ship_control::Leaves {
                    vehicle,
                    ..crate::test_tables::SHIP_LEAVES
                })),
                train: Box::leak(Box::new(train::Leaves {
                    vehicle,
                    ..crate::test_tables::TRAIN_LEAVES
                })),
                vehicle,
            }
        }
        /// The base vehicle table of the 32-day callback tests.
        fn callback_vehicles() -> VehicleServices {
            const MAINTENANCE: crate::vehicle::maintenance::Services =
                crate::vehicle::maintenance::Services {
                    mark_dirty,
                    ..crate::test_tables::MAINTENANCE
                };
            VehicleServices {
                engine: vehicle_engine,
                maintenance: &MAINTENANCE,
                newgrf: day_resolver,
                ..VEHICLE
            }
        }
        /// A road vehicle part at `index` that is not a front engine, with engine number `engine`,
        /// its road and ground owners and the day counter `day_counter`.
        fn road_part(
            index: usize,
            engine: crate::engine_type::EngineID,
            day_counter: u8,
        ) -> *mut VehicleRecord {
            new_vehicle_of(index, VehicleType::VEH_ROAD);
            let part = record(index);
            // SAFETY: The fresh record; the owners live for the isolated child.
            unsafe {
                (*part).road = road::openttd_rust_road_new().cast();
                (*part).ground = ground_vehicle::openttd_rust_ground_new();
                (*part).engine_type = engine;
                (*part).day_counter = day_counter;
            }
            part
        }
        /// Run the day procedures in normal play, starting at day fraction `fraction`.
        fn in_normal_play(fraction: u16) {
            session::set_mode(GameMode::GM_NORMAL);
            timer_game_common::write_economy_fraction(fraction);
            timer_game_common::write_calendar_fraction(fraction);
            ENGINE.set(crate::engine::openttd_rust_engine_new().cast());
        }

        /// `RunEconomyVehicleDayProc` over a pool with a hole: from the day fraction it visits every
        /// `DAY_TICKS`-th slot and skips the free one, and it asks the 32-day callback of a vehicle
        /// that has an engine type when its day counter is a multiple of 32. The calendar
        /// procedure asks no callback, and neither runs outside normal play.
        #[test]
        fn economy_day_visits_every_day_ticks_vehicle_from_the_fraction() {
            if crate::services_assert::isolate_test_check(
                "vehicle::scheduler::tests::pool::economy_day_visits_every_day_ticks_vehicle_from_the_fraction",
                record_failure,
            ) {
                return;
            }
            let _pool = new_pool();
            in_normal_play(3);
            let stride = DAY_TICKS as usize;
            road_part(3, EngineID(30), 0);
            road_part(4, EngineID(40), 0); // Not a slot of this fraction.
            road_part(3 + stride, EngineID(77), 0);
            delete_vehicle(3 + stride); // The hole.
            road_part(3 + 2 * stride, EngineID(151), 64);
            road_part(3 + 3 * stride, EngineID(225), 33); // Its day counter is no multiple of 32.
            let scheduler = day_scheduler(callback_vehicles());
            ANSWER.set(0);
            run_economy_vehicle_day_proc::<false>(&scheduler);
            assert_eq!(
                take_days(),
                [Day::Callback(EngineID(30)), Day::Callback(EngineID(151))]
            );
            run_vehicle_calendar_day_proc::<false>(&scheduler);
            assert_eq!(take_days(), Vec::<Day>::new());
            session::set_mode(GameMode::GM_EDITOR);
            run_economy_vehicle_day_proc::<false>(&scheduler);
            run_vehicle_calendar_day_proc::<false>(&scheduler);
            assert_eq!(take_days(), Vec::<Day>::new());
            assert_eq!(FAILURES.load(Ordering::Relaxed), 0);
        }

        /// The 32-day callback's result: bit 0 triggers the randomisation (trigger 10, waiting
        /// bit `Callback32`), any nonzero result marks the front dirty, bits above 1 report an
        /// unknown result for the engine's GRF, and a zero or failed result does none of them.
        #[test]
        fn economy_day_callback_results() {
            if crate::services_assert::isolate_test_check(
                "vehicle::scheduler::tests::pool::economy_day_callback_results",
                record_failure,
            ) {
                return;
            }
            let _pool = new_pool();
            in_normal_play(0);
            let part = road_part(0, EngineID(7), 32);
            // `v->First()` is a distinct live front, which the dirty marking must reach.
            let front = VehicleRecord::new(null_mut(), VehicleType::VEH_ROAD);
            // SAFETY: The fresh records; the front lives for the isolated child.
            unsafe { (*part).first = front };
            let scheduler = day_scheduler(callback_vehicles());
            let trigger = 1 << newgrf_vehicle::Trigger::Callback32.0;
            for (answer, expected) in [
                (
                    1,
                    vec![
                        Day::Callback(EngineID(7)),
                        Day::Trigger(trigger),
                        Day::Dirty(front),
                    ],
                ),
                (2, vec![Day::Callback(EngineID(7)), Day::Dirty(front)]),
                (
                    4,
                    vec![
                        Day::Callback(EngineID(7)),
                        Day::Dirty(front),
                        Day::Engine,
                        Day::Unknown(CallbackID::CBID_VEHICLE_32DAY_CALLBACK.0, 4),
                    ],
                ),
                (0, vec![Day::Callback(EngineID(7))]),
                (CALLBACK_FAILED, vec![Day::Callback(EngineID(7))]),
            ] {
                ANSWER.set(answer);
                run_economy_vehicle_day_proc::<false>(&scheduler);
                assert_eq!(take_days(), expected, "callback result {answer}");
            }
            assert_eq!(FAILURES.load(Ordering::Relaxed), 0);
        }

        /// The day virtuals' dispatch: effect and disaster vehicles keep the base class's empty
        /// handlers, and a road vehicle that is not a front engine and an aircraft that is not a
        /// normal aircraft (a shadow) return at their handlers' first tests, before any leaf.
        /// Every controller leaf traps.
        #[test]
        fn day_dispatch_skips_vehicles_without_a_handler_or_past_its_guard() {
            if crate::services_assert::isolate_test_check(
                "vehicle::scheduler::tests::pool::day_dispatch_skips_vehicles_without_a_handler_or_past_its_guard",
                record_failure,
            ) {
                return;
            }
            let _pool = new_pool();
            in_normal_play(0);
            let stride = DAY_TICKS as usize;
            new_vehicle_of(0, VehicleType::VEH_EFFECT);
            new_vehicle_of(stride, VehicleType::VEH_DISASTER);
            road_part(2 * stride, EngineID(7), 1);
            new_vehicle_of(3 * stride, VehicleType::VEH_AIRCRAFT);
            // SAFETY: The fresh aircraft record.
            unsafe {
                (*record(3 * stride)).subtype = crate::aircraft::AircraftSubType::AIR_SHADOW.0;
            }
            let scheduler = day_scheduler(VEHICLE);
            run_vehicle_calendar_day_proc::<false>(&scheduler);
            run_economy_vehicle_day_proc::<false>(&scheduler);
            assert_eq!(FAILURES.load(Ordering::Relaxed), 0);
        }

        /// One day arm for a vehicle no scenario shows: the production day procedure over a live
        /// vehicle of `kind` with `subtype` at index 0 reaches its handler's first leaf, which
        /// `vehicle` names. That leaf ends the child through the native assertion policy, so the
        /// handler goes no further than it.
        fn day_arm(kind: VehicleType, subtype: u8, calendar: bool, vehicle: VehicleServices) {
            let _pool = new_pool();
            in_normal_play(0);
            new_vehicle_of(0, kind);
            // SAFETY: The fresh record; its ground owner, which the 32-day callback's resolver
            // reads, lives for the isolated child. No handler reaches a controller owner before
            // the leaf that ends the child.
            unsafe {
                let v = record(0);
                (*v).subtype = subtype;
                (*v).ground = ground_vehicle::openttd_rust_ground_new();
            }
            ARM_RECORD.set(record(0));
            let scheduler = day_scheduler(vehicle);
            if calendar {
                run_vehicle_calendar_day_proc::<false>(&scheduler);
            } else {
                run_economy_vehicle_day_proc::<false>(&scheduler);
            }
        }
        thread_local! {
            static ARM_RECORD: Cell<*mut VehicleRecord> = const { Cell::new(null_mut()) };
        }
        /// The first leaf an arm reaches, with the vehicle it was given.
        fn reached(v: *mut VehicleRecord) -> ! {
            if v == ARM_RECORD.get() {
                crate::services_assert::check(false);
            }
            std::process::exit(1)
        }
        /// `AgeVehicle`'s first read, `v->age`, through the calendar handlers' resolver.
        extern "C" fn age_resolver() -> *const newgrf_vehicle::Services {
            unsafe extern "C" fn age(v: *mut VehicleRecord) -> i32 {
                reached(v)
            }
            const RESOLVER: newgrf_vehicle::Services = newgrf_vehicle::TEST_TABLE.with_age(age);
            &RESOLVER
        }
        /// `EconomyAgeVehicle`'s first read, `v->economy_age`.
        fn economy_age_vehicles() -> VehicleServices {
            extern "C" fn economy_age(v: VehicleRef) -> i32 {
                reached(v.0.as_ptr())
            }
            const MAINTENANCE: crate::vehicle::maintenance::Services =
                crate::vehicle::maintenance::Services {
                    economy_age,
                    ..crate::test_tables::MAINTENANCE
                };
            VehicleServices {
                maintenance: &MAINTENANCE,
                ..VEHICLE
            }
        }
        const ENGINE_SUBTYPE: u8 =
            1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_ENGINE.0;
        const NORMAL_AIRCRAFT: u8 = crate::aircraft::AircraftSubType::AIR_AIRCRAFT.0;

        #[test]
        fn calendar_day_dispatches_a_train() {
            if crate::services_assert::isolate_test_exit(
                "vehicle::scheduler::tests::pool::calendar_day_dispatches_a_train",
            ) {
                return;
            }
            let vehicle = VehicleServices {
                maintenance: &crate::test_tables::MAINTENANCE,
                newgrf: age_resolver,
                ..VEHICLE
            };
            day_arm(VehicleType::VEH_TRAIN, ENGINE_SUBTYPE, true, vehicle);
        }
        #[test]
        fn economy_day_dispatches_a_train() {
            if crate::services_assert::isolate_test_exit(
                "vehicle::scheduler::tests::pool::economy_day_dispatches_a_train",
            ) {
                return;
            }
            // Day counter 0 and an engine type: the 32-day callback runs first and fails.
            let vehicle = VehicleServices {
                newgrf: failing_callback_resolver,
                ..economy_age_vehicles()
            };
            day_arm(VehicleType::VEH_TRAIN, ENGINE_SUBTYPE, false, vehicle);
        }
        #[test]
        fn calendar_day_dispatches_an_aircraft() {
            if crate::services_assert::isolate_test_exit(
                "vehicle::scheduler::tests::pool::calendar_day_dispatches_an_aircraft",
            ) {
                return;
            }
            let vehicle = VehicleServices {
                maintenance: &crate::test_tables::MAINTENANCE,
                newgrf: age_resolver,
                ..VEHICLE
            };
            day_arm(VehicleType::VEH_AIRCRAFT, NORMAL_AIRCRAFT, true, vehicle);
        }
        #[test]
        fn economy_day_dispatches_an_aircraft() {
            if crate::services_assert::isolate_test_exit(
                "vehicle::scheduler::tests::pool::economy_day_dispatches_an_aircraft",
            ) {
                return;
            }
            let vehicle = VehicleServices {
                newgrf: failing_callback_resolver,
                ..economy_age_vehicles()
            };
            day_arm(VehicleType::VEH_AIRCRAFT, NORMAL_AIRCRAFT, false, vehicle);
        }
        /// `Ship::OnNewCalendarDay` is `AgeVehicle(this)`: its first leaf is `v->age`.
        #[test]
        fn calendar_day_dispatches_a_ship() {
            if crate::services_assert::isolate_test_exit(
                "vehicle::scheduler::tests::pool::calendar_day_dispatches_a_ship",
            ) {
                return;
            }
            let vehicle = VehicleServices {
                maintenance: &crate::test_tables::MAINTENANCE,
                newgrf: age_resolver,
                ..VEHICLE
            };
            day_arm(VehicleType::VEH_SHIP, 0, true, vehicle);
        }
        /// The economy arms' resolver: the 32-day callback, which a fresh train or aircraft asks
        /// first, fails.
        extern "C" fn failing_callback_resolver() -> *const newgrf_vehicle::Services {
            ANSWER.set(CALLBACK_FAILED);
            day_resolver()
        }
    }
}
