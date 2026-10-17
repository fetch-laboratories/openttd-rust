/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Live base-vehicle scheduling prototype. Owners stay opaque and call-local;
//! no Rust world/owner reference survives a service that can mutate or reenter.
#![allow(unsafe_code, clippy::too_many_lines, clippy::verbose_bit_mask)]
use super::{CALLBACK_FAILED, VS_CRASHED, VS_HIDDEN, VS_STOPPED, VehicleRef};
use crate::{cargo_storage, fleet, station_service};

use super::VehicleType;
const TRAIN: VehicleType = VehicleType::TRAIN;
const ROAD: VehicleType = VehicleType::ROAD;
const AIRCRAFT: VehicleType = VehicleType::AIRCRAFT;
const FRONT: u8 = 1;
const ENGINE: u8 = 1 << 3;
const NORMAL_AIRCRAFT: u8 = 2;
const TRAIN_SLOWING: u8 = 1 << 4;
const DAY_TICKS: usize = 74;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PoolCursor {
    pub slots: *const Option<VehicleRef>,
    pub slots_len: usize,
    pub first_unused: *const usize,
}
#[repr(C)]
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
pub struct DayCallback {
    pub day: u8,
    pub has_engine: bool,
}
#[repr(C)]
pub struct StationLookup {
    pub station: *mut station_service::Service,
    pub id: u32,
}
#[repr(C)]
pub struct SchedulerServices {
    pub pool: extern "C" fn() -> PoolCursor,
    pub assertions: bool,
    /// Runs the virtual Tick; fills `out` only when the vehicle survives.
    /// Caller supplies writable, aligned stack storage for one `PostTick`.
    pub tick: unsafe extern "C" fn(VehicleRef, *mut PostTick) -> bool,
    pub calendar_day: extern "C" fn(VehicleRef),
    pub economy_day: extern "C" fn(VehicleRef),
    pub day_callback_read: extern "C" fn(VehicleRef) -> DayCallback,
    pub day_callback: extern "C" fn(VehicleRef) -> u16,
    pub randomise_callback: extern "C" fn(VehicleRef),
    pub first_dirty: extern "C" fn(VehicleRef),
    pub unknown_day_callback: extern "C" fn(VehicleRef, u16),
    pub cargo_owner: extern "C" fn(VehicleRef) -> *mut cargo_storage::List,
    pub running_sound: extern "C" fn(VehicleRef),
    pub periodic_sound: extern "C" fn(VehicleRef, bool),
    pub next_station: extern "C" fn(u32) -> StationLookup,
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
}
// Select counted/plain implementations once at entry. In the plain monomorph,
// COUNTED=false erases every new per-visit witness branch/load/update.
macro_rules! svc {
    ($counted:ident, $function:expr $(, $arg:expr)*) => {{
        let result = ($function)($($arg),*);
        if $counted {
            crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
            crate::witness::add(crate::witness::VEHICLE_OBSERVED_BYTES, std::mem::size_of_val(&result) as u64);
        }
        result
    }};
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
        let vehicle = unsafe { scalar_read::<_, COUNTED>(pool.slots.add(*from)) };
        *from += 1;
        if vehicle.is_some() {
            return vehicle;
        }
    }
    None
}
/// Native transient probe uses the same cursor as scheduler, with actual pool storage.
/// # Safety
/// As next; native test owns writable descriptor and numeric cursor for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_cursor_next(
    s: *const SchedulerServices,
    pool: *mut PoolCursor,
    from: *mut usize,
) -> Option<VehicleRef> {
    unsafe { next::<false>(&*s, &mut *pool, &mut *from) }
}
unsafe fn day<const COUNTED: bool>(
    tables: &Tables,
    s: &SchedulerServices,
    calendar: bool,
    mut id: usize,
) {
    let mut pool = svc!(COUNTED, s.pool);
    while id < unsafe { extent::<COUNTED>(s, &mut pool) } {
        let vehicle = unsafe { scalar_read::<_, COUNTED>(pool.slots.add(id)) };
        id += DAY_TICKS;
        let Some(vehicle) = vehicle else {
            continue;
        };
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_DAYS);
        }
        if !calendar {
            let callback_state = svc!(COUNTED, s.day_callback_read, vehicle);
            if callback_state.day & 0x1f == 0 && callback_state.has_engine {
                if COUNTED {
                    crate::witness::hit(crate::witness::VEHICLE_CALLBACK32);
                }
                let callback = svc!(COUNTED, s.day_callback, vehicle);
                if callback != CALLBACK_FAILED {
                    if callback & 1 != 0 {
                        svc!(COUNTED, s.randomise_callback, vehicle);
                    }
                    if callback != 0 {
                        svc!(COUNTED, s.first_dirty, vehicle);
                    }
                    if callback & !3 != 0 {
                        svc!(COUNTED, s.unknown_day_callback, vehicle, callback);
                    }
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
struct PostTickServices {
    cargo_owner: extern "C" fn(VehicleRef) -> *mut cargo_storage::List,
    running_sound: extern "C" fn(VehicleRef),
    periodic_sound: extern "C" fn(VehicleRef, bool),
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
/// Tick survived. Native shells are separately allocated: pool array growth
/// cannot move their scalar members. Cargo packet lookup and `NewGRF` sound
/// resolution may reenter, but cannot destroy these shells, reset the pool or
/// yield the game-state mutex. Only addresses survive those audited callbacks;
/// field loads/stores end before each callback and subsequent values are fresh.
/// Never retain a descriptor across Tick, traversal, loading, day or fleet work.
/// `cargo` is the bundle the surviving Tick filled; it is read only here.
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
        AIRCRAFT if unsafe { scalar_read::<_, COUNTED>(motion.subtype) } > NORMAL_AIRCRAFT => {
            return;
        }
        _ => {}
    }
    // Only the motion counter was stored; no callback has changed this speed.
    if unsafe { scalar_read::<_, COUNTED>(motion.motion) } & 0xff < u32::from(speed) {
        post_svc!(
            COUNTED,
            crate::witness::VEHICLE_RUNNING_SOUND,
            schedule.running_sound,
            vehicle
        );
    }
    // Running sound can change the following observations; load after it returns.
    if unsafe { scalar_read::<_, COUNTED>(motion.tick) } & 15 == 0 {
        let running = unsafe { scalar_read::<_, COUNTED>(motion.speed) } > 0
            && unsafe { scalar_read::<_, COUNTED>(cargo.front_status) }
                & (VS_STOPPED | TRAIN_SLOWING)
                == 0;
        post_svc!(
            COUNTED,
            crate::witness::VEHICLE_PERIODIC_SOUND,
            schedule.periodic_sound,
            vehicle,
            running
        );
    }
}

/// # Safety
/// Immutable native tables and serialized pools remain valid until return.
/// No selected callback resets pools, loads a game or yields the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_calendar_day(
    tables: *const Tables,
    normal: bool,
    date_fract: usize,
) {
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_ticks(
    tables: *const Tables,
    normal: bool,
    date_fract: usize,
    cost: *mut (),
) {
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
    let mut station_from = 0;
    loop {
        let station = svc!(COUNTED, s.next_station, station_from);
        if station.station.is_null() {
            break;
        }
        station_from = station.id + 1;
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_STATIONS);
        }
        unsafe {
            station_service::openttd_rust_station_load(
                station.station,
                tables.station,
                tables.loading,
            );
        };
    }
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
        running_sound: s.running_sound,
        periodic_sound: s.periodic_sound,
        cargo: tables.cargo,
    };
    {
        let mut pool = svc!(COUNTED, s.pool);
        let mut id = 0;
        while let Some(vehicle) = unsafe { next::<COUNTED>(s, &mut pool, &mut id) } {
            let current_id = id - 1;
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_OUTER_VISITS);
            }
            // Native controller timing completes before the source survival assertion.
            // The bundle is written only on survival and read only after the assertion.
            let mut bundle = std::mem::MaybeUninit::<PostTick>::uninit();
            // SAFETY: bundle is aligned writable storage, unused until success.
            let survives = unsafe { svc!(COUNTED, s.tick, vehicle, bundle.as_mut_ptr()) };
            if s.assertions {
                let _ = unsafe { extent::<COUNTED>(s, &mut pool) };
                let actual = unsafe { scalar_read::<_, COUNTED>(pool.slots.add(current_id)) };
                assert!(actual == if survives { Some(vehicle) } else { None });
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
            // SAFETY: a surviving Tick initialized every bundle field.
            unsafe { post_tick::<COUNTED>(&post, vehicle, bundle.assume_init_ref()) };
        }
    } // Every pool/scalar descriptor is discarded before commands in fleet drain.
    unsafe {
        fleet::pending::openttd_rust_fleet_pending_drain(
            tables.pending,
            tables.transaction,
            tables.group,
            cost,
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;
    const SHIP: VehicleType = VehicleType::SHIP;
    use std::ptr::{NonNull, null, null_mut};

    // Only test-owned scalar cells, never a replica of the native Vehicle layout.
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
            let services = PostTickServices {
                cargo_owner,
                running_sound,
                periodic_sound,
                cargo: &raw const CARGO_SERVICES,
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
    extern "C" fn running_sound(v: VehicleRef) {
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
    }
    extern "C" fn periodic_sound(v: VehicleRef, running: bool) {
        let c = unsafe { &mut *cells(v) };
        c.events.push("periodic-sound");
        c.running = Some(running);
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
    extern "C" fn random(_: u32) -> u32 {
        0
    }
    extern "C" fn coordinate(_: u32, _: u8) -> u32 {
        0
    }
    extern "C" fn flow(
        _: *const c_void,
        _: u8,
        _: u16,
        _: u16,
        _: u16,
        _: *const u16,
        _: usize,
        _: *mut u8,
    ) -> u16 {
        0
    }
    extern "C" fn pay(_: *mut c_void, _: u8, _: u8, _: *const c_void, _: u32, _: u32) -> i64 {
        0
    }
    extern "C" fn origin(_: *mut c_void, _: u16, _: u32, _: u8) {}
    extern "C" fn flow_owner(_: *const c_void, _: u16) -> *mut crate::cargo_flow::Flow {
        null_mut()
    }
    extern "C" fn random_draw() -> u32 {
        0
    }
    extern "C" fn packet_next(_: *mut c_void) -> *mut c_void {
        null_mut()
    }
    static CARGO_SERVICES: cargo_storage::Services = cargo_storage::Services {
        can_allocate,
        create,
        packet,
        destroy,
        random,
        coordinate,
        flow,
        pay,
        origin,
        flow_owner,
        random_draw,
        packet_next,
    };

    #[test]
    fn cargo_short_circuit_wrap_clamp_and_period_reload() {
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
                &raw const CARGO_SERVICES,
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
            cargo_storage::openttd_rust_cargo_list_destroy(c.cargo, &raw const CARGO_SERVICES);
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
