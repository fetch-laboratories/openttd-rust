/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Live base-vehicle scheduling prototype. Owners stay opaque and call-local;
//! no Rust world/owner reference survives a service that can mutate or reenter.
#![allow(unsafe_code, clippy::verbose_bit_mask)]
use super::{CALLBACK_FAILED, VehicleRef, VehicleServices};
use crate::aircraft::AIR_AIRCRAFT;
use crate::newgrf_callbacks::CBID_VEHICLE_32DAY_CALLBACK;
use crate::newgrf_sound::{VSE_RUNNING, VSE_RUNNING_16, VSE_STOPPED_16, VehicleSoundEvent};
use crate::timer_game_tick::DAY_TICKS;
use crate::vehicle_base::{
    GVSF_ENGINE, GVSF_FRONT, VS_CRASHED, VS_HIDDEN, VS_STOPPED, VS_TRAIN_SLOWING,
};
use crate::{cargo_storage, fleet, services, station_service};

use super::VehicleType;
const TRAIN: VehicleType = VehicleType::TRAIN;
const ROAD: VehicleType = VehicleType::ROAD;
const AIRCRAFT: VehicleType = VehicleType::AIRCRAFT;
const FRONT: u8 = 1 << GVSF_FRONT;
const ENGINE: u8 = 1 << GVSF_ENGINE;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PoolCursor {
    pub slots: *const crate::pool::PoolSlot,
    pub slots_len: usize,
    pub first_unused: *const usize,
}
/// Field addresses in the surviving vehicle's and its front's canonical records.
pub struct PostTick {
    pub vehicle_type: VehicleType,
    pub period: *const u16,
    pub counter: *mut u16,
    pub motion: *mut u32,
    pub speed: *const u16,
    pub front_type: *const VehicleType,
    pub subtype: *const u8,
    pub tick: *const u8,
    pub front_status: *const u8,
    pub status: *const u8,
}
#[repr(C)]
pub struct SchedulerServices {
    pub pool: extern "C" fn() -> PoolCursor,
    pub assertions: bool,
    /// Runs the virtual Tick; true when the vehicle survives.
    pub tick: extern "C" fn(VehicleRef) -> bool,
    pub calendar_day: extern "C" fn(VehicleRef),
    pub economy_day: extern "C" fn(VehicleRef),
    pub cargo_owner: extern "C" fn(VehicleRef) -> *mut cargo_storage::List,
    /// Base vehicle and shared tables, immutable for the process.
    pub vehicle: *const VehicleServices,
    pub shared: *const services::Services,
    /// Tables remain live through the synchronous loading callback.
    pub with_loading_timer:
        unsafe extern "C" fn(*const Tables, unsafe extern "C" fn(*const Tables)),
    pub reset_accumulators: extern "C" fn(),
}
#[repr(C)]
pub struct Tables {
    pub scheduler: *const SchedulerServices,
    pub cargo: *const cargo_storage::Services,
    pub station: *const station_service::World,
    pub loading: *const station_service::Loading,
    pub pending: *const fleet::pending::Services,
    pub transaction: *const fleet::transactions::Services,
    pub group: *const fleet::GroupServices,
    pub visits: *const super::visits::Services,
    pub refit_cost: *mut crate::services::CommandCost,
    pub refit_temporary: *mut crate::services::CommandCost,
}
// Select counted/plain implementations once at entry. In the plain monomorph,
// COUNTED=false erases every new per-visit witness branch/load/update.
macro_rules! svc {
    ($counted:ident, $function:expr $(, $arg:expr)*) => {
        crate::witness::counted!(
            VEHICLE_CROSSINGS, VEHICLE_OBSERVED_BYTES if $counted, ($function)($($arg),*)
        )
    };
}

/// Refresh before ANY cached slot read after callbacks. `ResizeFor` grows only
/// when allocating at/above old size; `AllocateItem` raises canonical `first_unused`
/// above that size before return. `FreeItem` never lowers it. `CleanPool` is excluded
/// during this entry; the descriptor ends before load/reset or fleet drain.
unsafe fn extent<const COUNTED: bool>(s: &SchedulerServices, pool: &mut PoolCursor) -> usize {
    let live = unsafe { scalar_read::<_, COUNTED>(pool.first_unused) };
    if live > pool.slots_len {
        *pool = svc!(COUNTED, s.pool);
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_POOL_GROWTH);
        }
    }
    live
}
/// # Safety
/// Call-scoped descriptor from `pool()`, serialized pool; no `CleanPool`/load/reset.
unsafe fn next<const COUNTED: bool>(
    s: &SchedulerServices,
    pool: &mut PoolCursor,
    from: &mut usize,
) -> Option<VehicleRef> {
    while *from < unsafe { extent::<COUNTED>(s, pool) } {
        let vehicle = unsafe {
            scalar_read::<_, COUNTED>(std::ptr::addr_of!((*pool.slots.add(*from)).record))
        };
        *from += 1;
        if !vehicle.is_null() {
            // SAFETY: The slot carries its live owner record directly.
            return Some(VehicleRef::of(vehicle.cast()));
        }
    }
    None
}
unsafe fn day<const COUNTED: bool>(
    tables: &Tables,
    s: &SchedulerServices,
    calendar: bool,
    mut id: usize,
) {
    let mut pool = svc!(COUNTED, s.pool);
    while id < unsafe { extent::<COUNTED>(s, &mut pool) } {
        let vehicle =
            unsafe { scalar_read::<_, COUNTED>(std::ptr::addr_of!((*pool.slots.add(id)).record)) };
        id += DAY_TICKS as usize;
        if vehicle.is_null() {
            continue;
        }
        // SAFETY: This published record remains live for its day callback.
        let vehicle = VehicleRef::of(vehicle.cast());
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_DAYS);
        }
        if !calendar && vehicle.day() & 0x1f == 0 && vehicle.has_engine_type() {
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_CALLBACK32);
            }
            let vs = unsafe { &*s.vehicle };
            let callback = svc!(
                COUNTED,
                vs.callback,
                (vs.newgrf)(),
                vehicle,
                CBID_VEHICLE_32DAY_CALLBACK
            );
            if callback != CALLBACK_FAILED {
                if callback & 1 != 0 {
                    unsafe {
                        crate::newgrf_vehicle::trigger(
                            vs,
                            vehicle,
                            crate::newgrf_vehicle::Trigger::Callback32,
                        );
                    };
                }
                if callback != 0 {
                    // SAFETY: The immutable maintenance table lives for this entry.
                    let maintenance = unsafe { &*vs.maintenance };
                    svc!(COUNTED, maintenance.mark_dirty, vehicle.first());
                }
                if callback & !3 != 0 {
                    let engine = svc!(COUNTED, vs.engine, vehicle);
                    vs.unknown_callback_result(engine, CBID_VEHICLE_32DAY_CALLBACK, callback);
                }
            }
        }
        if calendar {
            svc!(COUNTED, s.calendar_day, vehicle);
        } else {
            svc!(COUNTED, s.economy_day, vehicle);
        }
    }
    let _ = tables;
}
impl PostTick {
    /// `v->type` and `v->First()` after the survival assertion; motion types
    /// address the record fields `CallVehicleTicks` reads and writes next.
    fn after_tick(v: VehicleRef) -> Self {
        let vehicle_type = v.vehicle_type();
        let record = v.0.as_ptr();
        if VehicleType(vehicle_type).0 > AIRCRAFT.0 {
            return Self {
                vehicle_type: VehicleType(vehicle_type),
                period: std::ptr::null(),
                counter: std::ptr::null_mut(),
                motion: std::ptr::null_mut(),
                speed: std::ptr::null(),
                front_type: std::ptr::null(),
                subtype: std::ptr::null(),
                tick: std::ptr::null(),
                front_status: std::ptr::null(),
                status: std::ptr::null(),
            };
        }
        let front = v.first().0.as_ptr();
        // SAFETY: Both canonical records live while their surviving shells do; only
        // addresses are formed here, and post_tick loads each field at its use.
        unsafe {
            Self {
                vehicle_type: VehicleType(vehicle_type),
                period: &raw const (*record).cached_cargo_age_period,
                counter: &raw mut (*record).cargo_age_counter,
                motion: &raw mut (*record).motion_counter,
                speed: &raw const (*front).cur_speed,
                front_type: &raw const (*front).r#type,
                subtype: &raw const (*record).subtype,
                tick: &raw const (*record).tick_counter,
                front_status: &raw const (*front).vehstatus,
                status: &raw const (*record).vehstatus,
            }
        }
    }
}
struct PostTickServices {
    cargo_owner: extern "C" fn(VehicleRef) -> *mut cargo_storage::List,
    sound: extern "C" fn(VehicleRef, VehicleSoundEvent) -> bool,
    cargo: *const cargo_storage::Services,
}
/// A field-sized raw access ends before the next service. These functions never
/// form references into the native shell. COUNTED=false removes accounting.
unsafe fn scalar_read<T: Copy, const COUNTED: bool>(address: *const T) -> T {
    if COUNTED {
        crate::witness::add(
            crate::witness::VEHICLE_SCALAR_READ_BYTES,
            std::mem::size_of::<T>() as u64,
        );
    }
    unsafe { address.read() }
}
unsafe fn scalar_write<T, const COUNTED: bool>(address: *mut T, value: T) {
    if COUNTED {
        crate::witness::add(
            crate::witness::VEHICLE_SCALAR_WRITE_BYTES,
            std::mem::size_of::<T>() as u64,
        );
    }
    unsafe { address.write(value) };
}
macro_rules! post_svc {
    ($counted:ident, $slot:expr, $function:expr $(, $arg:expr)*) => {{
        if $counted { crate::witness::hit($slot); }
        svc!($counted, $function $(, $arg)*)
    }};
}

/// # Safety
/// Tick survived. Shells are separately allocated, so pool growth cannot move their members;
/// cargo packet lookup and `NewGRF` sound resolution may reenter but cannot destroy these
/// shells, reset the pool or yield the mutex. Never retain a descriptor across Tick,
/// traversal, loading, day or fleet work. `cargo` addresses the surviving records; only
/// addresses span callbacks, and field loads and stores end first so subsequent values are fresh.
unsafe fn post_tick<const COUNTED: bool>(
    schedule: &PostTickServices,
    vehicle: VehicleRef,
    cargo: &PostTick,
) {
    if COUNTED {
        crate::witness::hit(crate::witness::VEHICLE_POST_TICK);
    }
    let vehicle_type = cargo.vehicle_type;
    if COUNTED {
        crate::witness::hit(
            crate::witness::VEHICLE_VISITS
                + match vehicle_type.0 {
                    0..=6 => usize::from(vehicle_type.0),
                    _ => 7,
                },
        );
    }
    if vehicle_type.0 > AIRCRAFT.0 {
        return;
    }
    if COUNTED {
        crate::witness::add(
            crate::witness::VEHICLE_DESCRIPTOR_BYTES,
            std::mem::size_of::<PostTick>() as u64,
        );
    }
    let period = unsafe { scalar_read::<_, COUNTED>(cargo.period) };
    if period != 0 {
        let counter = unsafe { scalar_read::<_, COUNTED>(cargo.counter) }.min(period);
        // Preserve the original clamp store, then unsigned predecrement store.
        unsafe { scalar_write::<_, COUNTED>(cargo.counter, counter) };
        let counter = counter.wrapping_sub(1);
        unsafe { scalar_write::<_, COUNTED>(cargo.counter, counter) };
        if counter == 0 {
            let owner = post_svc!(
                COUNTED,
                crate::witness::VEHICLE_CARGO_OWNER,
                schedule.cargo_owner,
                vehicle
            );
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_CARGO_AGES);
            }
            unsafe { cargo_storage::age::<COUNTED>(owner, schedule.cargo) };
            let period = unsafe { scalar_read::<_, COUNTED>(cargo.period) };
            unsafe { scalar_write::<_, COUNTED>(cargo.counter, period) };
        }
    }
    let front_status = unsafe { scalar_read::<_, COUNTED>(cargo.front_status) };
    if front_status & VS_CRASHED != 0 {
        return;
    }
    if unsafe { scalar_read::<_, COUNTED>(cargo.status) } & VS_HIDDEN != 0 {
        return;
    }
    let motion = cargo;
    if unsafe { scalar_read::<_, COUNTED>(cargo.front_status) } & VS_STOPPED != 0
        && (unsafe { scalar_read::<_, COUNTED>(motion.front_type) } != TRAIN
            || unsafe { scalar_read::<_, COUNTED>(motion.speed) } == 0)
    {
        return;
    }
    if COUNTED {
        crate::witness::hit(crate::witness::VEHICLE_MOTION);
    }
    let speed = unsafe { scalar_read::<_, COUNTED>(motion.speed) };
    let counter =
        unsafe { scalar_read::<_, COUNTED>(motion.motion) }.wrapping_add(u32::from(speed));
    unsafe { scalar_write::<_, COUNTED>(motion.motion, counter) };
    // Ship has no subtype gate; do not read its subtype.
    match vehicle_type {
        TRAIN if unsafe { scalar_read::<_, COUNTED>(motion.subtype) } & ENGINE == 0 => return,
        ROAD if unsafe { scalar_read::<_, COUNTED>(motion.subtype) } & FRONT == 0 => return,
        AIRCRAFT if unsafe { scalar_read::<_, COUNTED>(motion.subtype) } > AIR_AIRCRAFT => {
            return;
        }
        _ => {}
    }
    // Only the motion counter was stored; no callback has changed this speed.
    if unsafe { scalar_read::<_, COUNTED>(motion.motion) } & 0xff < u32::from(speed) {
        post_svc!(
            COUNTED,
            crate::witness::VEHICLE_RUNNING_SOUND,
            schedule.sound,
            vehicle,
            VSE_RUNNING
        );
    }
    // Running sound can change the following observations; load after it returns.
    if unsafe { scalar_read::<_, COUNTED>(motion.tick) } & 15 == 0 {
        let running = unsafe { scalar_read::<_, COUNTED>(motion.speed) } > 0
            && unsafe { scalar_read::<_, COUNTED>(cargo.front_status) }
                & (VS_STOPPED | VS_TRAIN_SLOWING)
                == 0;
        post_svc!(
            COUNTED,
            crate::witness::VEHICLE_PERIODIC_SOUND,
            schedule.sound,
            vehicle,
            if running {
                VSE_RUNNING_16
            } else {
                VSE_STOPPED_16
            }
        );
    }
}

/// # Safety
/// Immutable native tables and serialized pools remain valid until return.
/// No selected callback resets pools, loads a game or yields the game-state mutex.
pub(crate) unsafe fn calendar_day(tables: *const Tables, normal: bool, date_fract: usize) {
    if !normal {
        return;
    }
    let tables = unsafe { &*tables };
    if crate::witness::enabled() {
        unsafe { day::<true>(tables, &*tables.scheduler, true, date_fract) };
    } else {
        unsafe { day::<false>(tables, &*tables.scheduler, true, date_fract) };
    }
}
/// # Safety
/// As calendar entry; cost is native stack storage valid through fleet drain.
pub(crate) unsafe fn ticks(tables: *const Tables, normal: bool, date_fract: usize, cost: *mut ()) {
    if crate::witness::enabled() {
        unsafe { run_ticks::<true>(&*tables, normal, date_fract, cost) };
    } else {
        unsafe { run_ticks::<false>(&*tables, normal, date_fract, cost) };
    }
}
// The native RAII scope invokes this synchronous callback without retaining it.
// No world/owner borrow spans any station loading service.
// SAFETY: the caller supplies immutable live tables for this call.
unsafe extern "C" fn load_stations<const COUNTED: bool>(tables: *const Tables) {
    let tables = unsafe { &*tables };
    let s = unsafe { &*tables.scheduler };
    let shared = unsafe { &*s.shared };
    if COUNTED {
        crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
    }
    shared.for_each_station(|_, service, _| {
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_STATIONS);
        }
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
unsafe fn run_ticks<const COUNTED: bool>(
    tables: &Tables,
    normal: bool,
    date_fract: usize,
    cost: *mut (),
) {
    let s = unsafe { &*tables.scheduler };
    unsafe { fleet::pending::openttd_rust_fleet_pending_clear() };
    if normal {
        unsafe { day::<COUNTED>(tables, s, false, date_fract) };
    }
    // SAFETY: tables and their services outlive the synchronous callback.
    unsafe {
        svc!(
            COUNTED,
            s.with_loading_timer,
            std::ptr::from_ref(tables),
            load_stations::<COUNTED>
        );
    }
    svc!(COUNTED, s.reset_accumulators);
    let post = PostTickServices {
        cargo_owner: s.cargo_owner,
        sound: unsafe { &*s.vehicle }.play_vehicle_sound,
        cargo: tables.cargo,
    };
    {
        let mut pool = svc!(COUNTED, s.pool);
        let mut id = 0;
        while let Some(vehicle) = unsafe { next::<COUNTED>(s, &mut pool, &mut id) } {
            let current_id = id - 1;
            let shell = vehicle.shell();
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_OUTER_VISITS);
            }
            // Native controller timing completes before the source survival assertion.
            let survives = svc!(COUNTED, s.tick, vehicle);
            if s.assertions {
                let _ = unsafe { extent::<COUNTED>(s, &mut pool) };
                let actual = unsafe {
                    scalar_read::<_, COUNTED>(std::ptr::addr_of!(
                        (*pool.slots.add(current_id)).shell
                    ))
                };
                assert!(
                    actual
                        == if survives {
                            shell.cast()
                        } else {
                            std::ptr::null_mut()
                        }
                );
            }
            if !survives {
                if COUNTED {
                    crate::witness::hit(crate::witness::VEHICLE_DELETED);
                }
                continue;
            }
            if COUNTED {
                crate::witness::add(
                    crate::witness::VEHICLE_OBSERVED_BYTES,
                    std::mem::size_of::<PostTick>() as u64,
                );
            }
            // The survivor's type and front are read after the source assertion.
            let bundle = PostTick::after_tick(vehicle);
            unsafe { post_tick::<COUNTED>(&post, vehicle, &bundle) };
        }
    } // Every pool/scalar descriptor is discarded before commands in fleet drain.
    unsafe {
        fleet::pending::drain(tables.pending, tables.transaction, tables.group, cost);
    };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehiclePoolCursor" ["scheduler"] => PoolCursor {
        slots,
        slots_len,
        first_unused,
    };
    "OpenTTDVehicleSchedulerServices" ["scheduler"] => SchedulerServices {
        pool,
        assertions,
        tick,
        calendar_day,
        economy_day,
        cargo_owner,
        vehicle,
        shared,
        with_loading_timer,
        reset_accumulators,
    };
    "OpenTTDVehicleSchedulerTables" ["scheduler"] => Tables {
        scheduler,
        cargo,
        station,
        loading,
        pending,
        transaction,
        group,
        visits,
        refit_cost,
        refit_temporary,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;

    /// The survivor's own fields and its front's speed, type and status are the
    /// canonical record members; types above aircraft address none of them.
    #[test]
    fn post_tick_addresses_survivor_and_front_records() {
        let record = |kind| {
            VehicleRef::of(crate::vehicle_base::openttd_rust_vehicle_record_new(
                null_mut(),
                kind,
            ))
        };
        let (front, part) = (record(ROAD), record(ROAD));
        // SAFETY: Both private records are live; `first` links the part to its front.
        unsafe { (*part.0.as_ptr()).first = front.0.as_ptr() };
        let post = PostTick::after_tick(part);
        let (f, p) = (front.0.as_ptr(), part.0.as_ptr());
        assert_eq!(post.vehicle_type.0, ROAD.0);
        // SAFETY: Only addresses of the live private records are formed, then one store.
        unsafe {
            assert_eq!(post.period, &raw const (*p).cached_cargo_age_period);
            assert_eq!(post.counter, &raw mut (*p).cargo_age_counter);
            assert_eq!(post.motion, &raw mut (*p).motion_counter);
            assert_eq!(post.speed, &raw const (*f).cur_speed);
            assert_eq!(post.front_type, &raw const (*f).r#type);
            assert_eq!(post.subtype, &raw const (*p).subtype);
            assert_eq!(post.tick, &raw const (*p).tick_counter);
            assert_eq!(post.front_status, &raw const (*f).vehstatus);
            assert_eq!(post.status, &raw const (*p).vehstatus);
            (*p).r#type = VehicleType::EFFECT;
        }
        let post = PostTick::after_tick(part);
        assert!(
            post.vehicle_type == VehicleType::EFFECT
                && post.period.is_null()
                && post.status.is_null()
        );
        for v in [front, part] {
            // SAFETY: No traversal follows these private records after cleanup.
            unsafe { crate::vehicle_base::openttd_rust_vehicle_record_delete(v.0.as_ptr()) };
        }
    }
    const SHIP: VehicleType = VehicleType::SHIP;
    use std::ptr::{NonNull, null, null_mut};

    // Test-owned scalar cells, not the native Vehicle layout.
    #[allow(clippy::struct_excessive_bools)] // Independent inaccessible-field probes.
    struct Cells {
        period: u16,
        counter: u16,
        motion: u32,
        speed: u16,
        kind: VehicleType,
        subtype: u8,
        tick: u8,
        front_status: u8,
        status: u8,
        post_type: VehicleType,
        hide_counter: bool,
        hide_motion: bool,
        hide_speed: bool,
        hide_subtype: bool,
        hide_tick: bool,
        hide_kind: bool,
        sound_changes: u8,
        packet_reloads: bool,
        events: Vec<&'static str>,
        running: Option<bool>,
        cargo: *mut cargo_storage::List,
        packet: cargo_storage::Packet,
    }
    impl Cells {
        fn new() -> Self {
            Self {
                period: 0,
                counter: 9,
                motion: 0,
                speed: 0,
                kind: ROAD,
                subtype: FRONT,
                tick: 1,
                front_status: 0,
                status: 0,
                post_type: ROAD,
                hide_counter: false,
                hide_motion: false,
                hide_speed: false,
                hide_subtype: false,
                hide_tick: false,
                hide_kind: false,
                sound_changes: 0,
                packet_reloads: false,
                events: Vec::new(),
                running: None,
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
            }
        }
        fn handle(&mut self) -> VehicleRef {
            VehicleRef(NonNull::from(self).cast())
        }
        fn run(&mut self, kind: VehicleType) {
            let handle = self.handle();
            self.post_type = kind;
            let cargo_services = cargo_services();
            let services = PostTickServices {
                cargo_owner,
                sound,
                cargo: &raw const cargo_services,
            };
            // Stands in for the native survival fill inside the Tick facade.
            let bundle = addresses(handle);
            unsafe { post_tick::<false>(&services, handle, &bundle) };
        }
    }
    unsafe fn cells(v: VehicleRef) -> *mut Cells {
        v.0.as_ptr().cast()
    }
    fn addresses(v: VehicleRef) -> PostTick {
        let c = unsafe { &mut *cells(v) };
        c.events.push("post");
        PostTick {
            vehicle_type: c.post_type,
            period: &raw const c.period,
            counter: if c.hide_counter {
                null_mut()
            } else {
                &raw mut c.counter
            },
            motion: if c.hide_motion {
                null_mut()
            } else {
                &raw mut c.motion
            },
            speed: if c.hide_speed {
                null()
            } else {
                &raw const c.speed
            },
            front_type: if c.hide_kind {
                null()
            } else {
                &raw const c.kind
            },
            subtype: if c.hide_subtype {
                null()
            } else {
                &raw const c.subtype
            },
            tick: if c.hide_tick {
                null()
            } else {
                &raw const c.tick
            },
            front_status: &raw const c.front_status,
            status: &raw const c.status,
        }
    }
    extern "C" fn cargo_owner(v: VehicleRef) -> *mut cargo_storage::List {
        let c = unsafe { &mut *cells(v) };
        c.events.push("age-owner");
        c.cargo
    }
    extern "C" fn sound(v: VehicleRef, event: VehicleSoundEvent) -> bool {
        if event != VSE_RUNNING {
            let c = unsafe { &mut *cells(v) };
            c.events.push("periodic-sound");
            c.running = Some(event == VSE_RUNNING_16);
            assert!(event == VSE_RUNNING_16 || event == VSE_STOPPED_16);
            return false;
        }
        let c = unsafe { &mut *cells(v) };
        c.events.push("running-sound");
        match c.sound_changes {
            1 => {
                c.speed = 0;
                c.tick = 0;
                c.front_status = VS_STOPPED;
            }
            2 => {
                c.speed = 3;
                c.tick = 0;
                c.front_status = VS_STOPPED;
            }
            3 => {
                c.tick = 1;
            }
            _ => {}
        }
        false
    }
    extern "C" fn packet(shell: *mut c_void) -> *mut cargo_storage::Packet {
        let c = unsafe { &mut *shell.cast::<Cells>() };
        if c.packet_reloads {
            c.period = 17;
            c.events.push("age-packet");
        }
        &raw mut c.packet
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

    #[test]
    fn cargo_short_circuit_wrap_clamp_and_period_reload() {
        let cargo_services = cargo_services();
        let mut c = Cells::new();
        c.front_status = VS_CRASHED;
        c.hide_counter = true; // Source never accesses the counter when period=0.
        c.run(ROAD);
        assert_eq!(c.counter, 9);
        assert_eq!(c.events, ["post"]);
        for (counter, expected) in [(0, u16::MAX), (9, 3)] {
            let mut c = Cells::new();
            c.period = 4;
            c.counter = counter;
            c.front_status = VS_CRASHED;
            c.run(ROAD);
            assert_eq!(c.counter, expected);
            assert_eq!(c.events, ["post"]);
        }
        let mut c = Cells::new();
        c.period = 4;
        c.counter = 1;
        c.front_status = VS_CRASHED;
        c.cargo = cargo_storage::openttd_rust_cargo_list_new(1);
        let shell = c.handle().0.as_ptr().cast();
        unsafe {
            cargo_storage::openttd_rust_cargo_list_append(
                c.cargo,
                &raw const cargo_services,
                shell,
                2, // Cargo keep action, as in the existing cargo-storage fixture.
            );
        };
        c.packet_reloads = true;
        c.run(ROAD);
        assert_eq!(c.counter, 17); // Packet callback changes the later period read.
        assert_eq!(c.packet.periods_in_transit, 1);
        assert_eq!(c.events, ["post", "age-owner", "age-packet"]);
        unsafe {
            cargo_storage::openttd_rust_cargo_list_destroy(c.cargo, &raw const cargo_services);
        };
    }

    #[test]
    fn non_transport_type_bytes_skip_all_transport_state() {
        for byte in 4..=u8::MAX {
            let mut c = Cells::new();
            c.period = 7;
            c.counter = 3;
            c.speed = 10;
            c.motion = u32::MAX;
            c.hide_counter = true;
            c.hide_motion = true;
            c.hide_speed = true;
            c.hide_kind = true;
            c.hide_subtype = true;
            c.hide_tick = true;
            c.run(VehicleType(byte));
            assert_eq!(c.counter, 3);
            assert_eq!(c.motion, u32::MAX);
            assert_eq!(c.events, ["post"]);
        }
    }

    #[test]
    fn hidden_stopped_and_type_gates_skip_inaccessible_members() {
        let mut c = Cells::new();
        c.status = VS_HIDDEN;
        c.hide_motion = true;
        c.run(ROAD);
        assert_eq!(c.events, ["post"]);
        for (kind, speed) in [(ROAD, 23), (TRAIN, 0)] {
            let mut c = Cells::new();
            c.kind = kind;
            c.speed = speed;
            c.front_status = VS_STOPPED;
            c.hide_motion = true;
            c.hide_subtype = true;
            c.hide_tick = true;
            c.hide_speed = kind == ROAD; // Native || skips speed on a stopped road.
            c.run(kind);
            assert_eq!(c.events, ["post"]);
        }
        for (kind, subtype) in [(ROAD, 0), (TRAIN, 0), (AIRCRAFT, 3)] {
            let mut c = Cells::new();
            c.speed = 5;
            c.motion = u32::MAX;
            c.subtype = subtype;
            c.hide_kind = true;
            c.hide_tick = true;
            c.run(kind);
            assert_eq!(c.motion, 4); // Original unsigned motion wraps.
            assert_eq!(c.events, ["post"]);
        }
    }

    #[test]
    fn fresh_sound_observations_and_ship_subtype_short_circuit() {
        let mut c = Cells::new();
        c.speed = 8;
        c.motion = 252;
        c.sound_changes = 1;
        c.run(ROAD);
        assert_eq!(c.motion, 260);
        assert_eq!(c.running, Some(false));
        assert_eq!(c.events, ["post", "running-sound", "periodic-sound"]);
        for change in [2, 3] {
            let mut c = Cells::new();
            c.speed = 8;
            c.motion = 252;
            c.tick = 0;
            c.sound_changes = change;
            c.run(ROAD);
            if change == 2 {
                assert_eq!(c.running, Some(false));
            } else {
                assert_eq!(c.running, None); // Changed tick skips periodic observations.
            }
        }
        let mut c = Cells::new();
        c.speed = 5;
        c.motion = 17;
        c.tick = 0;
        c.hide_subtype = true;
        c.hide_kind = true;
        c.run(SHIP);
        assert_eq!(c.running, Some(true));
        let mut c = Cells::new();
        c.front_status = VS_STOPPED;
        c.kind = TRAIN;
        c.speed = 5;
        c.subtype = ENGINE;
        c.tick = 0;
        c.motion = 17;
        c.run(TRAIN); // A coasting stopped train still increments its motion.
        assert_eq!(c.motion, 22);
        assert_eq!(c.running, Some(false));
    }
}

/// Traversal on the real Rust pool owner: deletion, reuse and growth order; the live cursor;
/// and Tick growth with a fresh type. Traces are checked against `scheduler/pool-oracle.txt`,
/// captured from the unchanged original `PoolIterator`.
#[cfg(test)]
mod pool_tests {
    use super::*;
    use crate::oracle;
    use crate::pool::{NativeOps, PoolConfig, PoolCore};
    use crate::test_tables::{FLEET_GROUPS, FLEET_PENDING, FLEET_TRANSACTIONS, SCHEDULER, VEHICLE};
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
        static POOL_READS: Cell<u32> = const { Cell::new(0) };
        static SURVIVORS: Cell<u32> = const { Cell::new(0) };
        static MUTATE: Cell<bool> = const { Cell::new(false) };
        static GROWTH: Cell<usize> = const { Cell::new(usize::MAX) };
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
            crate::vehicle_base::openttd_rust_vehicle_record_delete(record);
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
        // SAFETY: The test's live pool owner; the index is free.
        unsafe {
            let shell = PoolCore::allocate_at(pool(), SIZE, index).unwrap();
            let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
                shell.cast(),
                VehicleType::EFFECT,
            );
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

    extern "C" fn cursor() -> PoolCursor {
        POOL_READS.set(POOL_READS.get() + 1);
        let view = view();
        // SAFETY: The test's live pool owner; the cursor is refreshed on growth.
        unsafe {
            PoolCursor {
                slots: (*view).slots.as_mut_ptr(),
                slots_len: (*view).slots.len(),
                first_unused: &raw const (*view).first_unused,
            }
        }
    }
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
    extern "C" fn no_station_loading(
        tables: *const Tables,
        load: unsafe extern "C" fn(*const Tables),
    ) {
        // SAFETY: The caller's live tables.
        unsafe { load(tables) };
    }
    extern "C" fn no_accumulators() {}
    extern "C" fn keep_company(_: u8) {}

    /// Run `CallVehicleTicks` (outside normal play: no day work) with `tick`. It clears and
    /// drains the process-wide pending map, so it holds that map's test lock.
    fn run_ticks(tick: extern "C" fn(VehicleRef) -> bool) {
        let _guard = crate::fleet::pending::TEST_LOCK.lock().unwrap();
        let shared = crate::services::fixture(null_mut(), draw, no_industry);
        let mut scheduler = SCHEDULER;
        scheduler.pool = cursor;
        scheduler.assertions = true;
        scheduler.tick = tick;
        scheduler.vehicle = &VEHICLE;
        scheduler.shared = &raw const shared;
        scheduler.with_loading_timer = no_station_loading;
        scheduler.reset_accumulators = no_accumulators;
        let mut pending = FLEET_PENDING;
        pending.set_current = keep_company;
        let mut group = FLEET_GROUPS;
        group.shared = &raw const shared;
        let tables = Tables {
            scheduler: &raw const scheduler,
            cargo: null(),
            station: null(),
            loading: null(),
            pending: &raw const pending,
            transaction: &FLEET_TRANSACTIONS,
            group: &raw const group,
            visits: null(),
            refit_cost: null_mut(),
            refit_temporary: null_mut(),
        };
        // SAFETY: The tables and the pool outlive the synchronous pass.
        unsafe { ticks(&raw const tables, false, 0, null_mut()) };
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
    /// after each visit of `next`.
    fn live_cursor(indexing_growth: bool) -> (String, String) {
        let _pool = new_pool();
        GROWTH.set(usize::MAX);
        for id in [0, 2, 4] {
            new_vehicle(id);
        }
        let mut services = SCHEDULER;
        services.pool = cursor;
        let mut pool_cursor = cursor();
        let mut visits = Vec::new();
        let mut first_unused = 0;
        let mut from = 0;
        // SAFETY: The test's live pool; the cursor is refreshed by `next`.
        while let Some(vehicle) = unsafe { next::<false>(&services, &mut pool_cursor, &mut from) } {
            let id = index_of(vehicle);
            visits.push(id);
            if id == 0 {
                delete_vehicle(0); // Deleted-current and reuse behind the advancing cursor.
                new_vehicle(0);
                delete_vehicle(2);
                new_vehicle(2); // Reuse ahead must still be visited.
                new_vehicle(3);
                new_vehicle(6); // Raise extent without indexing growth.
                // SAFETY: The cursor's canonical extent.
                first_unused = unsafe { *pool_cursor.first_unused };
            } else if id == 2 {
                delete_vehicle(4); // Hole ahead.
                if indexing_growth {
                    let growth = slots_len();
                    GROWTH.set(growth);
                    new_vehicle(growth); // Growth invalidates captured slots.
                    new_vehicle(growth + 1);
                }
                // SAFETY: The test's live pool owner.
                assert_eq!(pool_cursor.first_unused, unsafe {
                    &raw const (*view()).first_unused
                });
            }
        }
        // SAFETY: The test's live pool owner.
        unsafe {
            assert_eq!(pool_cursor.slots, (*view()).slots.as_mut_ptr().cast_const());
            assert_eq!(pool_cursor.slots_len, (*view()).slots.len());
        }
        clean();
        pool_cursor = cursor(); // Fresh acquisition after reset; an empty pool ends.
        from = 0;
        // SAFETY: The cleaned live pool.
        let empty = unsafe { next::<false>(&services, &mut pool_cursor, &mut from) }.is_none();
        let inputs = format!("indexing growth {indexing_growth}");
        (
            inputs,
            format!("{first_unused} | {} | {}", trace(&visits), u8::from(empty)),
        )
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
    /// pool and changes its own type during Tick; the newly discovered vehicle deletes itself.
    extern "C" fn growing_tick(vehicle: VehicleRef) -> bool {
        if index_of(vehicle) == 0 {
            new_vehicle(slots_len());
            // SAFETY: The visiting vehicle's live record.
            unsafe { (*vehicle.0.as_ptr()).r#type = VehicleType::DISASTER };
            SURVIVORS.set(SURVIVORS.get() + 1);
            return true;
        }
        delete_vehicle(index_of(vehicle));
        false
    }
    #[test]
    fn tick_growth_rereads_the_pool_and_the_fresh_type() {
        let _pool = new_pool();
        new_vehicle(0);
        POOL_READS.set(0);
        SURVIVORS.set(0);
        run_ticks(growing_tick);
        assert_eq!(POOL_READS.get(), 2, "pool reads: start and growth");
        assert_eq!(
            SURVIVORS.get(),
            1,
            "only the survivor reaches its post-Tick reads"
        );
        // SAFETY: The surviving record.
        assert_eq!(unsafe { (*record(0)).r#type }.0, VehicleType::DISASTER.0);
        // SAFETY: The live pool owner.
        assert_eq!(unsafe { (*view()).items }, 1);
        clean();
    }
}
