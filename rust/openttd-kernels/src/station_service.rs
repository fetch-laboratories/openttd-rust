/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical station cargo-service state. Native scalar aliases and packet
//! services never overlap Rust references into this owner. Addresses are stable
//! for the shell lifetime, including indexed loading and retired-station reuse.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
use crate::cargo_payment::NativePayment;
use crate::linkgraph::NativeGraph;
use crate::orders::NativeOrderList;
use crate::station::{BaseStation, Station};
use crate::vehicle::{EngineRef, VS_CRASHED, VS_STOPPED, VehicleRef};
use std::ffi::c_void;
use std::ptr::NonNull;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CargoFields {
    pub max_waiting_cargo: u32,
    pub status: u8,
    pub time_since_pickup: u8,
    pub rating: u8,
    pub last_speed: u8,
    pub last_age: u8,
    pub amount_fract: u8,
}
impl Default for CargoFields {
    fn default() -> Self {
        Self {
            max_waiting_cargo: 0,
            status: 0,
            time_since_pickup: 255,
            rating: 175,
            last_speed: 0,
            last_age: 255,
            amount_fract: 0,
        }
    }
}
#[repr(C)]
pub struct Fields {
    pub always_accepted: u64,
    pub delete_ctr: u8,
    pub time_since_load: u8,
    pub time_since_unload: u8,
    pub last_vehicle_type: u8,
}
pub struct Service {
    fields: Fields,
    cargo: [CargoFields; 64],
    // Each entry is raw-owned: native iterators retain raw pointers, so even a
    // temporary Box/&mut reborrow on a surviving entry would invalidate them.
    queue: Vec<*mut QueueEntry>,
}
/// Activity fields consumed together after a station-radius filter.
#[repr(C)]
pub struct StationActivity {
    /// Canonical number of days since loading.
    pub time_since_load: u8,
    /// Canonical number of days since unloading.
    pub time_since_unload: u8,
}

/// Read the canonical fields without retaining a borrow across native callbacks.
///
/// # Safety
/// `state` is a live station owner whose fields are not being accessed concurrently.
pub(crate) unsafe fn activity(state: *const Service) -> StationActivity {
    // SAFETY: Immediate field-sized reads from the caller's live serialized owner.
    unsafe {
        StationActivity {
            time_since_load: (&raw const (*state).fields.time_since_load).read(),
            time_since_unload: (&raw const (*state).fields.time_since_unload).read(),
        }
    }
}
/// Shared native town observation of the canonical station owner.
///
/// # Safety
/// `state` is live and game-state-mutex serialized; no borrow survives this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_activity(state: *const Service) -> StationActivity {
    // SAFETY: Native shared helper supplies the station's live canonical owner.
    unsafe { activity(state) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_service_new() -> *mut Service {
    Box::into_raw(Box::new(Service {
        fields: Fields {
            always_accepted: 0,
            delete_ctr: 0,
            time_since_load: 255,
            time_since_unload: 255,
            last_vehicle_type: 255,
        },
        cargo: [CargoFields::default(); 64],
        queue: Vec::new(),
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_service_destroy(state: *mut Service) {
    unsafe {
        drop(Box::from_raw(state));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_service_fields(state: *mut Service) -> *mut Fields {
    unsafe { &raw mut (*state).fields }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_service_cargo(
    state: *mut Service,
    cargo: u8,
) -> *mut CargoFields {
    unsafe {
        (&raw mut (*state).cargo)
            .cast::<CargoFields>()
            .add(usize::from(cargo))
    }
}
pub struct QueueEntry {
    vehicle: *mut c_void,
    next: *mut QueueEntry,
}
impl Drop for Service {
    fn drop(&mut self) {
        for entry in self.queue.drain(..) {
            // SAFETY: The owner uniquely owns each live, individually allocated
            // entry. Native aliases have ended before shell/owner destruction.
            unsafe {
                drop(Box::from_raw(entry));
            }
        }
    }
}
unsafe fn queue_relink(state: *mut Service) {
    let mut next = std::ptr::null_mut();
    // Borrow only the private vector of handles, never a live exported node.
    for &entry in unsafe { &(*state).queue }.iter().rev() {
        unsafe {
            (&raw mut (*entry).next).write(next);
        }
        next = entry;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_size(state: *const Service) -> usize {
    unsafe { (*state).queue.len() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_push(
    state: *mut Service,
    vehicle: *mut c_void,
) {
    let entry = Box::into_raw(Box::new(QueueEntry {
        vehicle,
        next: std::ptr::null_mut(),
    }));
    unsafe {
        if let Some(&last) = (*state).queue.last() {
            (&raw mut (*last).next).write(entry);
        }
        (*state).queue.push(entry);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_remove(
    state: *mut Service,
    vehicle: *mut c_void,
) {
    unsafe {
        (*state).queue.retain(|&entry| {
            if (&raw const (*entry).vehicle).read() == vehicle {
                drop(Box::from_raw(entry));
                false
            } else {
                true
            }
        });
        queue_relink(state);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_clear(state: *mut Service) {
    for entry in unsafe { (*state).queue.drain(..) } {
        unsafe {
            drop(Box::from_raw(entry));
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_first(
    state: *const Service,
) -> *mut QueueEntry {
    unsafe {
        (*state)
            .queue
            .first()
            .copied()
            .unwrap_or(std::ptr::null_mut())
            .cast()
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_next(
    _state: *const Service,
    entry: *const QueueEntry,
) -> *mut QueueEntry {
    unsafe { (&raw const (*entry).next).read().cast() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_value(entry: *const QueueEntry) -> *mut c_void {
    unsafe { (&raw const (*entry).vehicle).read() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_erase_node(
    state: *mut Service,
    entry: *mut QueueEntry,
) -> *mut QueueEntry {
    let successor = unsafe { (&raw const (*entry).next).read().cast() };
    // Original erase requires a live iterator from this owner; no new rejection
    // or fatal path is introduced for an input outside that precondition.
    let index = unsafe {
        (*state)
            .queue
            .iter()
            .position(|&node| node == entry)
            .unwrap_unchecked()
    };
    unsafe {
        let entry = (*state).queue.remove(index);
        drop(Box::from_raw(entry));
        queue_relink(state);
    }
    successor
}

#[repr(C)]
pub struct RatingCounts {
    pub(crate) available: u32,
    pub(crate) destinations: u32,
}
#[repr(C)]
pub struct StationExclusivity {
    pub(crate) counter: u8,
    pub(crate) exclusive: u8,
}
#[repr(C)]
pub struct CargoCapacity {
    pub(crate) owner: *const crate::cargo_storage::List,
    pub(crate) capacity: u16,
    pub(crate) cargo: u8,
}
impl CargoCapacity {
    fn remaining(&self) -> u32 {
        // SAFETY: the synchronous native observation supplies the stable cargo
        // owner of a live vehicle. This scalar read ends before any callback;
        // callers obtain a new observation after refit/reservation mutations.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_remaining(self.owner) }
    }
}
#[repr(C)]
pub struct CargoReservation {
    pub(crate) mask: u64,
    pub(crate) reserved: u32,
    pub(crate) capacity: u16,
    pub(crate) cargo: u8,
}
#[repr(C)]
pub struct CargoStored {
    pub(crate) stored: u32,
    pub(crate) capacity: u16,
}
#[repr(C)]
pub struct DeliveryRemaining {
    pub(crate) remaining: u32,
    pub(crate) deliver: u32,
    pub(crate) capacity: u16,
}
#[repr(C)]
pub struct LoadEngine {
    pub(crate) engine: EngineRef,
    pub(crate) amount: u8,
}
#[repr(C)]
pub struct LoadingControl {
    pub(crate) ticks: u16,
    pub(crate) vehicle_type: u8,
    pub(crate) unload_type: u8,
    pub(crate) load_type: u8,
    pub(crate) refit_cargo: u8,
    pub(crate) refit: bool,
    pub(crate) full_load: bool,
    pub(crate) stop: bool,
    pub(crate) gradual: bool,
    pub(crate) improved: bool,
}
#[repr(C)]
pub struct LoadingPart {
    pub(crate) capacity: u16,
    pub(crate) cargo: u8,
    pub(crate) vehicle_type: u8,
    pub(crate) unloading: bool,
}
#[repr(C)]
pub struct LoadingStats {
    pub(crate) stored: u32,
    pub(crate) load_count: u32,
    pub(crate) speed: i32,
    pub(crate) age: i32,
    pub(crate) capacity: u16,
    pub(crate) cargo: u8,
}
#[repr(C)]
pub struct LoadingWait {
    pub(crate) lateness: i32,
    pub(crate) wait: u16,
}
#[repr(C)]
pub struct OrderDestination {
    pub(crate) destination: u16,
    pub(crate) is_station: bool,
}
#[repr(C)]
pub struct TruncatedSource {
    pub(crate) amount: u32,
    pub(crate) source: u16,
    pub(crate) present: bool,
}
#[repr(C)]
pub struct World {
    pub(crate) station: extern "C" fn(u16) -> LoadingStation,
    pub(crate) station_owner: extern "C" fn(*mut BaseStation) -> u8,
    pub(crate) station_exclusivity: extern "C" fn(*mut BaseStation) -> StationExclusivity,
    pub(crate) station_next: extern "C" fn(*mut c_void) -> *mut BaseStation,
    pub(crate) rating_counts: extern "C" fn(*mut BaseStation, u8) -> RatingCounts,
    pub(crate) get: extern "C" fn(u16) -> *mut BaseStation,
    pub(crate) owner: extern "C" fn(*mut BaseStation) -> *mut Service,
    pub(crate) pool_size: extern "C" fn() -> usize,
    pub(crate) editor: extern "C" fn() -> bool,
    pub(crate) facilities: extern "C" fn(*mut BaseStation) -> u8,
    pub(crate) in_use: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) is_station: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) local_owner: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) rectangle_empty: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) town_statue: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) select_goods: extern "C" fn() -> bool,
    pub(crate) rating_cheat: extern "C" fn() -> bool,
    pub(crate) available: extern "C" fn(*mut BaseStation, u8) -> u32,
    pub(crate) cargo_valid: extern "C" fn(u8) -> bool,
    pub(crate) rating_callback_enabled: extern "C" fn(u8) -> bool,
    pub(crate) passenger_cargo: extern "C" fn(u8) -> bool,
    pub(crate) packet_available: extern "C" fn() -> bool,
    pub(crate) rating_redraw: extern "C" fn(*mut BaseStation, bool),
    pub(crate) set_demand: extern "C" fn(*mut BaseStation, u8, u32),
    pub(crate) acceptance_news: extern "C" fn(*mut BaseStation, u64, bool),
    pub(crate) acceptance_redraw: extern "C" fn(*mut BaseStation),
    pub(crate) delete_station: extern "C" fn(*mut BaseStation),
    pub(crate) acceptance_animation: extern "C" fn(*mut BaseStation),
    pub(crate) watched_animation: extern "C" fn(u32, u64),
    pub(crate) append_packet:
        extern "C" fn(*mut BaseStation, u8, u32, crate::cargo_storage::Source),
    pub(crate) update_supply: extern "C" fn(*mut BaseStation, u8, u32),
    pub(crate) station_list_invalidation: extern "C" fn(*mut BaseStation),
    pub(crate) new_cargo_animation: extern "C" fn(*mut BaseStation, u8),
    pub(crate) waiting_redraw: extern "C" fn(*mut BaseStation),
    pub(crate) rating_callback: extern "C" fn(*mut BaseStation, u8, u32, u32) -> u16,
    pub(crate) with_tiles: extern "C" fn(*mut BaseStation, *mut c_void, Scope),
    pub(crate) tile_next: extern "C" fn(*mut c_void) -> u32,
    pub(crate) accept_tile: extern "C" fn(*mut c_void, u32, *mut u64),
    pub(crate) acceptance_amounts: extern "C" fn(*mut c_void) -> *const u32,
    pub(crate) with_truncation: extern "C" fn(*mut BaseStation, u8, u32, *mut c_void, Scope),
    pub(crate) truncate_next: extern "C" fn(*mut c_void) -> TruncatedSource,
    pub(crate) shared: *const crate::services::Services,
}
impl World {
    fn random(&self) -> u32 {
        // SAFETY: the immutable shared table outlives this synchronous entry.
        let shared = unsafe { &*self.shared };
        (shared.random)(shared.context)
    }
}
// Native iterator/container frames live on the C++ caller's stack. This private
// callback context is synchronous and does not retain aliases into a game owner.
type Scope = extern "C" fn(*mut c_void, *mut c_void);
extern "C" fn scoped<F: FnMut(*mut c_void)>(context: *mut c_void, frame: *mut c_void) {
    // SAFETY: with_* invokes this callback exactly once, before the stack context
    // ends; only the callback accesses its private closure during the invocation.
    unsafe {
        (&mut *context.cast::<F>())(frame);
    }
}
fn scope_context<F: FnMut(*mut c_void)>(body: &mut F) -> (*mut c_void, Scope) {
    (std::ptr::from_mut(body).cast(), scoped::<F>)
}
// Field-sized operations avoid owner borrows while C++ aliases are live. A copied
// observation is used only up to the next original write/observation point.
unsafe fn cp(state: *mut Service, cargo: u8) -> *mut CargoFields {
    unsafe {
        (&raw mut (*state).cargo)
            .cast::<CargoFields>()
            .add(usize::from(cargo))
    }
}
unsafe fn status(state: *mut Service, cargo: u8, bit: u8, value: bool) {
    let p = unsafe { &raw mut (*cp(state, cargo)).status };
    let old = unsafe { p.read() };
    unsafe {
        p.write(if value {
            old | (1 << bit)
        } else {
            old & !(1 << bit)
        });
    }
}
unsafe fn acceptance_mask(state: *mut Service) -> u64 {
    let mut mask = 0;
    for cargo in 0..64 {
        if unsafe { (*cp(state, cargo)).status } & 1 != 0 {
            mask |= 1 << cargo;
        }
    }
    mask
}
unsafe fn truncate(st: *mut BaseStation, cargo: u8, amount: u32, w: &World) {
    let mut body = |map| {
        loop {
            let observed = (w.truncate_next)(map);
            if !observed.present {
                break;
            }
            let source_st = (w.get)(observed.source);
            if source_st.is_null() {
                continue;
            }
            let ge = unsafe { cp((w.owner)(source_st), cargo) };
            unsafe {
                (&raw mut (*ge).max_waiting_cargo)
                    .write((*ge).max_waiting_cargo.max(observed.amount));
            }
        }
    };
    let (context, callback) = scope_context(&mut body);
    (w.with_truncation)(st, cargo, amount, context, callback);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_acceptance(
    st: *mut BaseStation,
    world: *const World,
    show: u8,
) {
    let w = unsafe { &*world };
    let state = (w.owner)(st);
    let old = unsafe { acceptance_mask(state) };
    let mut acceptance = [0_u32; 64];
    if u32::from((w.rectangle_empty)(st)) == 0 {
        let always = unsafe { &raw mut (*state).fields.always_accepted };
        unsafe {
            always.write(0);
        }
        let mut body = |tiles| {
            loop {
                let tile = (w.tile_next)(tiles);
                if tile == u32::MAX {
                    break;
                }
                (w.accept_tile)(tiles, tile, always);
            }
            let amounts = (w.acceptance_amounts)(tiles);
            // SAFETY: native CargoArray belongs to this synchronous stack frame;
            // the scan is complete and this copy ends before another service.
            unsafe {
                std::ptr::copy_nonoverlapping(amounts, acceptance.as_mut_ptr(), 64);
            }
        };
        let (context, callback) = scope_context(&mut body);
        (w.with_tiles)(st, context, callback);
    }
    let facilities = u32::from((w.facilities)(st));
    for cargo in 0..64 {
        let mut amt = acceptance[usize::from(cargo)];
        let passenger = u32::from((w.passenger_cargo)(cargo)) != 0;
        // Train=0,truck=1,bus=2,airport=3,dock=4 in StationFacility.
        if facilities & if passenger { 0b11101 } else { 0b11011 } == 0 {
            amt = 0;
        }
        unsafe {
            status(state, cargo, 0, amt >= 8);
        }
        (w.set_demand)(st, cargo, amt / 8);
    }
    let new = unsafe { acceptance_mask(state) };
    if old == new {
        return;
    }
    if show != 0 && u32::from((w.local_owner)(st)) != 0 && u32::from((w.in_use)(st)) != 0 {
        let accepts = new & !old;
        let rejects = !new & old;
        if accepts != 0 {
            (w.acceptance_news)(st, accepts, false);
        }
        if rejects != 0 {
            (w.acceptance_news)(st, rejects, true);
        }
    }
    (w.acceptance_redraw)(st);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_watched(st: *mut BaseStation, world: *const World) {
    let w = unsafe { &*world };
    let state = (w.owner)(st);
    let mut cargoes = 0_u64;
    for cargo in 0..64 {
        if unsafe { (*cp(state, cargo)).status } & 32 != 0 {
            cargoes |= 1 << cargo;
        }
    }
    if cargoes == 0 {
        return;
    }
    // The immutable shared table outlives this entry and every native tile frame.
    let shared = unsafe { &*w.shared };
    let mut body = |tiles| {
        loop {
            let tile = (w.tile_next)(tiles);
            if tile == u32::MAX {
                break;
            }
            if (shared.tile_type)(tile) == crate::services::MP_HOUSE {
                (w.watched_animation)(tile, cargoes);
            }
        }
    };
    let (context, callback) = scope_context(&mut body);
    (w.with_tiles)(st, context, callback);
}
// Preserve the complete source rating loop and its immediate mutation order.
#[allow(clippy::too_many_lines)]
pub unsafe extern "C" fn openttd_rust_station_rating(st: *mut BaseStation, world: *const World) {
    let w = unsafe { &*world };
    let state = (w.owner)(st);
    let mut waiting_changed = false;
    let activity = unsafe { activity(state) };
    unsafe {
        let p = &raw mut (*state).fields.time_since_load;
        p.write(activity.time_since_load.saturating_add(1));
        let p = &raw mut (*state).fields.time_since_unload;
        p.write(activity.time_since_unload.saturating_add(1));
    }
    for cargo in 0..64_u8 {
        if u32::from((w.cargo_valid)(cargo)) == 0 {
            continue;
        }
        let ge = unsafe { cp(state, cargo) };
        if unsafe { (*ge).status } & 2 == 0 {
            unsafe {
                if (*ge).rating < 175 {
                    (&raw mut (*ge).rating).write((*ge).rating + 1);
                }
            }
            continue;
        }
        unsafe {
            (&raw mut (*ge).time_since_pickup).write((*ge).time_since_pickup.saturating_add(1));
        }
        if unsafe { (*ge).time_since_pickup } == 255 && u32::from((w.select_goods)()) != 0 {
            unsafe {
                status(state, cargo, 1, false);
                (&raw mut (*ge).last_speed).write(0);
                truncate(st, cargo, u32::MAX, w);
            }
            waiting_changed = true;
            continue;
        }
        let mut skip = false;
        let mut rating = 0_i32;
        let counts = (w.rating_counts)(st, cargo);
        let mut waiting = counts.available;
        let num_dests = counts.destinations;
        let waiting_avg = waiting / num_dests.wrapping_add(1);
        if u32::from((w.rating_cheat)()) != 0 {
            unsafe {
                (&raw mut (*ge).rating).write(255);
            }
            rating = 255;
            skip = true;
        } else if u32::from((w.rating_callback_enabled)(cargo)) != 0 {
            let speed = unsafe { (*ge).last_speed };
            let var18 = unsafe {
                u32::from((*ge).time_since_pickup)
                    | ((*ge).max_waiting_cargo.min(65535) << 8)
                    | (u32::from(if speed == 0 { 255 } else { speed }) << 24)
            };
            let kind = unsafe { (*state).fields.last_vehicle_type };
            let var10 = if kind == 255 {
                0
            } else {
                u32::from(kind) + 0x10
            };
            let cb = (w.rating_callback)(st, cargo, var10, var18);
            if cb != 0xffff {
                skip = true;
                rating = i32::from(cb & 0x3fff);
                if cb & 0x4000 != 0 {
                    rating -= 0x4000;
                }
            }
        }
        if !skip {
            let b = i32::from(unsafe { (*ge).last_speed }) - 85;
            if b >= 0 {
                rating += b >> 2;
            }
            let mut waittime = unsafe { (*ge).time_since_pickup };
            if unsafe { (*state).fields.last_vehicle_type } == 2 {
                waittime >>= 2;
            }
            if waittime <= 21 {
                rating += 25;
            }
            if waittime <= 12 {
                rating += 25;
            }
            if waittime <= 6 {
                rating += 45;
            }
            if waittime <= 3 {
                rating += 35;
            }
            rating -= 90;
            let max = unsafe { (*ge).max_waiting_cargo };
            if max <= 1500 {
                rating += 55;
            }
            if max <= 1000 {
                rating += 35;
            }
            if max <= 600 {
                rating += 10;
            }
            if max <= 300 {
                rating += 20;
            }
            if max <= 100 {
                rating += 10;
            }
        }
        if u32::from((w.town_statue)(st)) != 0 {
            rating += 26;
        }
        let age = unsafe { (*ge).last_age };
        if age < 3 {
            rating += 10;
        }
        if age < 2 {
            rating += 10;
        }
        if age < 1 {
            rating += 13;
        }
        let old_rating = i32::from(unsafe { (*ge).rating });
        rating = (old_rating + (rating - old_rating).clamp(-2, 2)).clamp(0, 255);
        unsafe {
            (&raw mut (*ge).rating).write(rating as u8);
        }
        if rating <= 64 && waiting_avg >= 100 {
            let mut dec = w.random() & 31;
            if waiting_avg < 200 {
                dec &= 7;
            }
            waiting = waiting.wrapping_sub((dec + 1).wrapping_mul(num_dests));
            waiting_changed = true;
        }
        if rating <= 127 && waiting != 0 {
            let random = w.random();
            if rating <= (random & 127) as i32 {
                waiting = (waiting as i32)
                    .wrapping_sub((((random >> 8) & 3) + 1).wrapping_mul(num_dests) as i32)
                    .max(0) as u32;
                waiting_changed = true;
            }
        }
        if waiting > 4096 {
            waiting -= (waiting - 4096) / 64;
            waiting = waiting.min(32768);
            waiting_changed = true;
        }
        let available = (w.available)(st, cargo);
        if waiting_changed && waiting < available {
            unsafe {
                (&raw mut (*ge).max_waiting_cargo).write(0);
                truncate(st, cargo, available - waiting, w);
            }
        } else {
            unsafe {
                (&raw mut (*ge).max_waiting_cargo).write(waiting_avg);
            }
        }
    }
    (w.rating_redraw)(st, u32::from(waiting_changed) != 0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_monthly(world: *const World) {
    let w = unsafe { &*world };
    for id in 0..(w.pool_size)() {
        let st = (w.get)(id as u16);
        if st.is_null() || u32::from((w.is_station)(st)) == 0 {
            continue;
        }
        let state = (w.owner)(st);
        for cargo in 0..64 {
            let current = unsafe { (*cp(state, cargo)).status } & 16 != 0;
            unsafe {
                status(state, cargo, 3, current);
                status(state, cargo, 4, false);
            }
        }
    }
}

unsafe fn admit(
    st: *mut BaseStation,
    cargo: u8,
    mut amount: u32,
    source: crate::cargo_storage::Source,
    w: &World,
) -> u32 {
    if u32::from((w.packet_available)()) == 0 {
        return 0;
    }
    let state = (w.owner)(st);
    let ge = unsafe { cp(state, cargo) };
    amount = amount.wrapping_add(u32::from(unsafe { (*ge).amount_fract }));
    unsafe {
        (&raw mut (*ge).amount_fract).write(amount as u8);
    }
    amount >>= 8;
    if amount == 0 {
        return 0;
    }
    (w.append_packet)(st, cargo, amount, source);
    (w.update_supply)(st, cargo, amount);
    if unsafe { (*ge).status } & 2 == 0 {
        (w.station_list_invalidation)(st);
        unsafe {
            status(state, cargo, 1, true);
        }
    }
    (w.new_cargo_animation)(st, cargo);
    (w.waiting_redraw)(st);
    amount
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_distribute(
    cargo: u8,
    mut amount: u32,
    source: crate::cargo_storage::Source,
    stations: *mut c_void,
    exclusivity: u8,
    world: *const World,
) -> u32 {
    if amount == 0 {
        return 0;
    }
    let w = unsafe { &*world };
    let mut used = Vec::new();
    let mut first = None;
    loop {
        let st = (w.station_next)(stations);
        if st.is_null() {
            break;
        }
        let owner = (w.station_owner)(st);
        if exclusivity != 255 && exclusivity != owner {
            continue;
        }
        if owner != 16 {
            let observed = (w.station_exclusivity)(st);
            if observed.counter > 0 && observed.exclusive != owner {
                continue;
            }
        }
        let ge = unsafe { cp((w.owner)(st), cargo) };
        let rating = unsafe { (*ge).rating };
        if rating == 0 {
            continue;
        }
        if u32::from((w.select_goods)()) != 0 && unsafe { (*ge).last_speed } == 0 {
            continue;
        }
        let passenger = u32::from((w.passenger_cargo)(cargo)) != 0;
        if u32::from((w.facilities)(st)) == if passenger { 2 } else { 4 } {
            continue;
        }
        let observed = (st, owner as usize, u32::from(rating), 0_u32);
        if let Some(first) = first {
            if used.is_empty() {
                used.reserve(2);
                used.push(first);
            }
            used.push(observed);
        } else {
            first = Some(observed);
        }
    }
    let Some((st, _, rating, _)) = first else {
        return 0;
    };
    if used.is_empty() {
        return unsafe { admit(st, cargo, amount.wrapping_mul(rating + 1), source, w) };
    }
    let mut company_best = [0_u32; 19];
    let mut company_sum = [0_u32; 19];
    let mut best_rating = 0;
    let mut best_sum = 0_u32;
    for &(_, owner, rating, _) in &used {
        if rating > company_best[owner] {
            best_sum = best_sum.wrapping_add(rating - company_best[owner]);
            company_best[owner] = rating;
            best_rating = best_rating.max(rating);
        }
        company_sum[owner] = company_sum[owner].wrapping_add(rating);
    }
    amount = amount.wrapping_mul(best_rating + 1);
    let mut moving = 0_u32;
    for &mut (_, owner, rating, ref mut share) in &mut used {
        *share = amount
            .wrapping_mul(company_best[owner])
            .wrapping_mul(rating)
            / best_sum
            / company_sum[owner];
        moving = moving.wrapping_add(*share);
    }
    if amount > moving {
        used.sort_by_key(|entry| std::cmp::Reverse(entry.2));
        for index in 0..amount - moving {
            used[index as usize].3 = used[index as usize].3.wrapping_add(1);
        }
    }
    let mut moved = 0_u32;
    for (st, _, _, share) in used {
        moved = moved.wrapping_add(unsafe { admit(st, cargo, share, source, w) });
    }
    moved
}

#[repr(C)]
#[derive(Default)]
pub struct Edge {
    pub(crate) destination: *mut BaseStation,
    pub(crate) last_update: i32,
    pub(crate) distance: u32,
}
#[repr(C)]
pub struct Links {
    pub(crate) graph: extern "C" fn(*mut BaseStation, u8) -> *mut NativeGraph,
    pub(crate) automatic_distribution: extern "C" fn(u8) -> bool,
    pub(crate) node: extern "C" fn(*mut BaseStation, u8) -> u16,
    pub(crate) edge_count: extern "C" fn(*mut NativeGraph, u16) -> usize,
    pub(crate) date: extern "C" fn() -> i32,
    pub(crate) station_id: extern "C" fn(*mut BaseStation) -> u16,
    pub(crate) edge_last_update: extern "C" fn(*mut NativeGraph, u16, usize) -> i32,
    pub(crate) last_compression: extern "C" fn(*mut NativeGraph) -> i32,
    pub(crate) edge: extern "C" fn(*mut NativeGraph, u16, usize, *mut BaseStation) -> Edge,
    pub(crate) edge_unrestricted: extern "C" fn(*mut NativeGraph, u16, usize) -> i32,
    pub(crate) edge_restricted: extern "C" fn(*mut NativeGraph, u16, usize) -> i32,
    pub(crate) delete_flows: extern "C" fn(*mut BaseStation, u8, u16),
    pub(crate) restrict_edge: extern "C" fn(*mut NativeGraph, u16, usize),
    pub(crate) restrict_flows: extern "C" fn(*mut BaseStation, u8, u16),
    pub(crate) release_edge: extern "C" fn(*mut NativeGraph, u16, usize),
    pub(crate) remove_edge: extern "C" fn(*mut NativeGraph, u16, u16),
    pub(crate) compress: extern "C" fn(*mut NativeGraph),
    pub(crate) order_list: extern "C" fn(u16) -> *mut NativeOrderList,
    pub(crate) order_pool_size: extern "C" fn() -> usize,
    pub(crate) order_count: extern "C" fn(*mut NativeOrderList) -> usize,
    pub(crate) order_destination: extern "C" fn(*mut NativeOrderList, usize) -> OrderDestination,
    pub(crate) order_vehicle: extern "C" fn(*mut NativeOrderList) -> Option<VehicleRef>,
    pub(crate) next_vehicle: extern "C" fn(VehicleRef) -> Option<VehicleRef>,
    pub(crate) next_shared: extern "C" fn(VehicleRef) -> Option<VehicleRef>,
    pub(crate) stopped_in_depot: extern "C" fn(VehicleRef) -> bool,
    pub(crate) last_service: extern "C" fn(VehicleRef) -> i32,
    pub(crate) cargo_type: extern "C" fn(VehicleRef) -> u8,
    pub(crate) refresh: extern "C" fn(VehicleRef),
    pub(crate) reroute: extern "C" fn(*mut BaseStation, Option<VehicleRef>, u8, u16, u16),
}
unsafe fn reroute(st: *mut BaseStation, cargo: u8, avoid: u16, avoid2: u16, w: &World, l: &Links) {
    (l.reroute)(st, None, cargo, avoid, avoid2);
    let state = (w.owner)(st);
    let mut node = unsafe { openttd_rust_station_queue_first(state) };
    while !node.is_null() {
        let mut current =
            NonNull::new(unsafe { openttd_rust_station_queue_value(node) }.cast()).map(VehicleRef);
        while let Some(vehicle) = current {
            if u32::from((l.cargo_type)(vehicle)) == u32::from(cargo) {
                (l.reroute)(st, Some(vehicle), cargo, avoid, avoid2);
            }
            current = (l.next_vehicle)(vehicle);
        }
        node = unsafe { openttd_rust_station_queue_next(state, node) };
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_reroute(
    st: *mut BaseStation,
    cargo: u8,
    avoid: u16,
    avoid2: u16,
    world: *const World,
    links: *const Links,
) {
    let w = unsafe { &*world };
    let l = unsafe { &*links };
    unsafe {
        reroute(st, cargo, avoid, avoid2, w, l);
    }
}
// Keep the source edge/vehicle iteration and its immediate mutation order together.
#[allow(clippy::too_many_lines)]
unsafe fn stale(from: *mut BaseStation, w: &World, l: &Links) {
    for cargo in 0..64 {
        let auto = (l.automatic_distribution)(cargo);
        let graph = (l.graph)(from, cargo);
        if graph.is_null() {
            continue;
        }
        let node = u32::from((l.node)(from, cargo)) as u16;
        let mut remove = Vec::new();
        let count = (l.edge_count)(graph, node);
        for index in 0..count {
            let edge = (l.edge)(graph, node, index, from);
            let date = (l.date)();
            let timeout = 32_u32.wrapping_add(edge.distance >> 3) as i32;
            if date.wrapping_sub(edge.last_update) > timeout {
                let mut updated = false;
                if auto {
                    let mut vehicles = Vec::new();
                    let from_id = (l.station_id)(from);
                    let to_id = (l.station_id)(edge.destination);
                    let order_count = (l.order_pool_size)();
                    for id in 0..order_count {
                        // OrderList pool IDs are bounded by their u16 representation.
                        let list = (l.order_list)(id as u16);
                        if list.is_null() {
                            continue;
                        }
                        let mut found_from = false;
                        let mut found_to = false;
                        for order in 0..(l.order_count)(list) {
                            let observed = (l.order_destination)(list, order);
                            if !observed.is_station {
                                continue;
                            }
                            let dest = observed.destination;
                            if dest == from_id {
                                found_from = true;
                                if found_to {
                                    break;
                                }
                            } else if dest == to_id {
                                found_to = true;
                                if found_from {
                                    break;
                                }
                            }
                        }
                        if found_from && found_to {
                            vehicles.push(unsafe { (l.order_vehicle)(list).unwrap_unchecked() });
                        }
                    }
                    let mut iterator = 0;
                    while iterator < vehicles.len() {
                        let vehicle = vehicles[iterator];
                        if !(l.stopped_in_depot)(vehicle)
                            || date.wrapping_sub((l.last_service)(vehicle)) <= 1024
                        {
                            (l.refresh)(vehicle);
                        }
                        if (l.edge_last_update)(graph, node, index) == date {
                            crate::witness::hit(crate::witness::STATION_STALE_REFRESH);
                            updated = true;
                            break;
                        }
                        let next_shared = (l.next_shared)(vehicle);
                        if let Some(next_shared) = next_shared {
                            vehicles[iterator] = next_shared;
                            iterator += 1;
                        } else {
                            vehicles.remove(iterator);
                        }
                        if iterator == vehicles.len() {
                            iterator = 0;
                        }
                    }
                }
                if !updated {
                    crate::witness::hit(crate::witness::STATION_STALE_REMOVE);
                    remove.push((l.node)(edge.destination, cargo));
                    let to_id = (l.station_id)(edge.destination);
                    (l.delete_flows)(from, cargo, to_id);
                    unsafe {
                        reroute(from, cargo, to_id, (l.station_id)(from), w, l);
                    }
                }
            } else {
                let unrestricted = (l.edge_unrestricted)(graph, node, index);
                if unrestricted != -1 && date.wrapping_sub(unrestricted) > timeout {
                    crate::witness::hit(crate::witness::STATION_STALE_RESTRICT);
                    (l.restrict_edge)(graph, node, index);
                    let to_id = (l.station_id)(edge.destination);
                    (l.restrict_flows)(from, cargo, to_id);
                    unsafe {
                        reroute(from, cargo, to_id, (l.station_id)(from), w, l);
                    }
                } else {
                    let restricted = (l.edge_restricted)(graph, node, index);
                    if restricted != -1 && date.wrapping_sub(restricted) > timeout {
                        crate::witness::hit(crate::witness::STATION_STALE_RELEASE);
                        (l.release_edge)(graph, node, index);
                    }
                }
            }
        }
        for dest in remove {
            (l.remove_edge)(graph, node, dest);
        }
        let date = (l.date)();
        if date.wrapping_sub((l.last_compression)(graph)) > 256 {
            (l.compress)(graph);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_stale(
    st: *mut BaseStation,
    world: *const World,
    links: *const Links,
) {
    let w = unsafe { &*world };
    let l = unsafe { &*links };
    unsafe {
        stale(st, w, l);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_tick(world: *const World, links: *const Links) {
    let w = unsafe { &*world };
    let l = unsafe { &*links };
    if u32::from((w.editor)()) != 0 {
        return;
    }
    let mut id = 0_usize;
    while id < (w.pool_size)() {
        let st = (w.get)(id as u16);
        if !st.is_null() {
            let state = (w.owner)(st);
            if u32::from((w.facilities)(st)) & 128 == 0 && u32::from((w.in_use)(st)) != 0 {
                let mut b = unsafe { (*state).fields.delete_ctr }.wrapping_add(1);
                if b >= 185 {
                    b = 0;
                }
                unsafe {
                    (&raw mut (*state).fields.delete_ctr).write(b);
                }
                if b == 0 {
                    unsafe {
                        openttd_rust_station_rating(st, world);
                    }
                }
            }
            // SAFETY: the immutable shared table outlives this synchronous entry.
            let tick = (unsafe { &*w.shared }.tick_counter)();
            let phase = tick.wrapping_add(id as u64);
            if u32::from((w.is_station)(st)) != 0 && phase % 504 == 0 {
                unsafe {
                    stale(st, w, l);
                }
            }
            if phase % 250 == 0 {
                if u32::from((w.in_use)(st)) == 0 {
                    let b = unsafe { (*state).fields.delete_ctr }.wrapping_add(1);
                    unsafe {
                        (&raw mut (*state).fields.delete_ctr).write(b);
                    }
                    if b >= 8 {
                        (w.delete_station)(st);
                    }
                    id += 1;
                    continue;
                }
                if u32::from((w.is_station)(st)) != 0 {
                    unsafe {
                        openttd_rust_station_watched(st, world);
                    }
                    for cargo in 0..64 {
                        unsafe {
                            status(state, cargo, 5, false);
                        }
                    }
                }
                if u32::from((w.facilities)(st)) & 128 == 0 {
                    unsafe {
                        openttd_rust_station_acceptance(st, world, 1);
                    }
                }
            }
            if phase % 250 == 0 {
                (w.acceptance_animation)(st);
            }
        }
        id += 1;
    }
}

#[repr(C)]
pub struct Loading {
    pub(crate) control: extern "C" fn(VehicleRef) -> LoadingControl,
    pub(crate) part: extern "C" fn(VehicleRef) -> LoadingPart,
    pub(crate) set_refit_capacity_and_stats:
        extern "C" fn(VehicleRef, VehicleRef, u8, u16, bool) -> LoadingStats,
    pub(crate) wait_observation: extern "C" fn(VehicleRef) -> LoadingWait,
    pub(crate) load_engine: extern "C" fn(VehicleRef) -> LoadEngine,
    pub(crate) grf_version: extern "C" fn(EngineRef) -> u8,
    pub(crate) engine_load_callback: extern "C" fn(EngineRef) -> bool,
    pub(crate) engine_no_multiplier: extern "C" fn(EngineRef) -> bool,
    pub(crate) delivery_remaining: extern "C" fn(VehicleRef) -> DeliveryRemaining,
    pub(crate) cargo_stored: extern "C" fn(VehicleRef) -> CargoStored,
    pub(crate) cargo_reservation: extern "C" fn(VehicleRef) -> CargoReservation,
    pub(crate) cargo_capacity: extern "C" fn(VehicleRef) -> CargoCapacity,
    pub(crate) vehicle_type: extern "C" fn(VehicleRef) -> u8,
    pub(crate) cargo_type: extern "C" fn(VehicleRef) -> u8,
    pub(crate) cargo_cap: extern "C" fn(VehicleRef) -> u16,
    pub(crate) stored_count: extern "C" fn(VehicleRef) -> u32,
    pub(crate) unload_count: extern "C" fn(VehicleRef) -> u32,
    pub(crate) deliver_count: extern "C" fn(VehicleRef) -> u32,
    pub(crate) transfer_count: extern "C" fn(VehicleRef) -> u32,
    pub(crate) load_count: extern "C" fn(VehicleRef) -> u32,
    pub(crate) unload_type: extern "C" fn(VehicleRef) -> u8,
    pub(crate) has_orders: extern "C" fn(VehicleRef) -> bool,
    pub(crate) load_unload_ticks: extern "C" fn(VehicleRef) -> u16,
    pub(crate) status: extern "C" fn(VehicleRef) -> u8,
    pub(crate) has_articulated_part: extern "C" fn(VehicleRef) -> bool,
    pub(crate) is_articulated_part: extern "C" fn(VehicleRef) -> bool,
    pub(crate) is_rear_dualheaded: extern "C" fn(VehicleRef) -> bool,
    pub(crate) is_normal_aircraft: extern "C" fn(VehicleRef) -> bool,
    pub(crate) is_multiheaded: extern "C" fn(VehicleRef) -> bool,
    pub(crate) cargo_multiplier: extern "C" fn(VehicleRef) -> u16,
    pub(crate) cached_total_length: extern "C" fn(VehicleRef) -> u16,
    pub(crate) full_load_order: extern "C" fn(VehicleRef) -> bool,
    pub(crate) vehicle_owner: extern "C" fn(VehicleRef) -> u8,
    pub(crate) passenger_cargo: extern "C" fn(VehicleRef) -> bool,
    pub(crate) station_length_penalty: extern "C" fn() -> bool,
    pub(crate) current_company: extern "C" fn() -> u8,
    pub(crate) vehicle: *const crate::vehicle::VehicleServices,
    pub(crate) total_count: extern "C" fn(VehicleRef) -> u32,
    pub(crate) order_time: extern "C" fn(VehicleRef) -> i32,
    pub(crate) refit_mask: extern "C" fn(VehicleRef) -> u64,
    pub(crate) set_cargo_unloading: extern "C" fn(VehicleRef, bool),
    pub(crate) set_loading_finished: extern "C" fn(VehicleRef, bool),
    pub(crate) set_stop: extern "C" fn(VehicleRef, bool),
    pub(crate) set_load_ticks: extern "C" fn(VehicleRef, u16),
    pub(crate) set_speed: extern "C" fn(VehicleRef, u16),
    pub(crate) subtract_profit: extern "C" fn(VehicleRef, i64),
    pub(crate) unknown_load_result: extern "C" fn(VehicleRef, u16),
    pub(crate) refresh_capacity: extern "C" fn(VehicleRef),
    pub(crate) vehicle_dirty: extern "C" fn(VehicleRef),
    pub(crate) update_indicator: extern "C" fn(VehicleRef),
    pub(crate) set_current_company: extern "C" fn(u8),
    pub(crate) next_vehicle: extern "C" fn(VehicleRef) -> Option<VehicleRef>,
    pub(crate) next_articulated: extern "C" fn(VehicleRef) -> Option<VehicleRef>,
    pub(crate) other_multiheaded: extern "C" fn(VehicleRef) -> VehicleRef,
    pub(crate) first_engine_part: extern "C" fn(VehicleRef) -> VehicleRef,
    pub(crate) first_vehicle: extern "C" fn(VehicleRef) -> VehicleRef,
    pub(crate) previous_vehicle: extern "C" fn(VehicleRef) -> VehicleRef,
    pub(crate) with_next_stations: extern "C" fn(VehicleRef, *mut c_void, Scope),
    pub(crate) stage: extern "C" fn(*mut Station, VehicleRef, u8, *mut c_void, *mut NativePayment),
    pub(crate) return_cargo: extern "C" fn(*mut Station, VehicleRef, u32),
    pub(crate) unload_cargo:
        extern "C" fn(*mut Station, VehicleRef, u32, *mut NativePayment) -> u32,
    pub(crate) load_cargo: extern "C" fn(*mut Station, VehicleRef, u32, *mut c_void) -> u32,
    pub(crate) reserve_cargo: extern "C" fn(*mut Station, VehicleRef, u32, *mut c_void),
    pub(crate) deliver_to_transfer: extern "C" fn(VehicleRef, u32),
    pub(crate) deliver_to_keep: extern "C" fn(VehicleRef, u32),
    pub(crate) has_cargo_for: extern "C" fn(*mut Station, u8, *mut c_void) -> bool,
    pub(crate) platform_length: extern "C" fn(*mut Station, VehicleRef) -> u32,
    pub(crate) vehicle_tile: extern "C" fn(VehicleRef) -> u32,
    pub(crate) station_index: extern "C" fn(*mut Station) -> u16,
    pub(crate) station_cargo_total: extern "C" fn(*mut Station, u8) -> u32,
    pub(crate) load_property: extern "C" fn(VehicleRef) -> u16,
    pub(crate) load_callback: extern "C" fn(VehicleRef) -> u16,
    pub(crate) get_payment: extern "C" fn(VehicleRef) -> *mut NativePayment,
    pub(crate) create_payment: extern "C" fn(VehicleRef) -> *mut NativePayment,
    pub(crate) delete_payment: extern "C" fn(*mut NativePayment),
    pub(crate) flush_delivery: extern "C" fn(),
    pub(crate) vehicle_new_cargo: extern "C" fn(VehicleRef),
    pub(crate) cargo_taken: extern "C" fn(*mut Station, u8),
    pub(crate) train_loads: extern "C" fn(*mut Station, VehicleRef),
    pub(crate) road_loads: extern "C" fn(*mut Station, VehicleRef),
    pub(crate) vehicle_empty: extern "C" fn(VehicleRef),
    pub(crate) station_dirty: extern "C" fn(*mut Station),
    pub(crate) test_refit: extern "C" fn(VehicleRef, u8) -> u32,
    pub(crate) execute_refit: extern "C" fn(VehicleRef, u8, *mut i64) -> bool,
}

fn parts(l: &Loading, v: VehicleRef, mut action: impl FnMut(VehicleRef) -> bool) -> bool {
    let mut current = Some(v);
    while let Some(part) = current {
        if !action(part) {
            return false;
        }
        if (l.vehicle_type)(part) == 0
            && (l.is_multiheaded)(part)
            && !action((l.other_multiheaded)(part))
        {
            return false;
        }
        current = if (l.has_articulated_part)(part) {
            (l.next_articulated)(part)
        } else {
            None
        };
    }
    if (l.vehicle_type)(v) == 3 && (l.is_normal_aircraft)(v) {
        return action(unsafe { (l.next_vehicle)(v).unwrap_unchecked() });
    }
    true
}
/// Native identities retained for one loading entry; no field borrow crosses a callback.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LoadingStation {
    pub(crate) native: *mut Station,
    pub(crate) base: *mut BaseStation,
    pub(crate) state: *mut Service,
}
fn may_load(st: LoadingStation, v: VehicleRef, w: &World, l: &Loading) -> bool {
    if (w.station_owner)(st.base) != 16 {
        return true;
    }
    let observed = (w.station_exclusivity)(st.base);
    observed.counter == 0 || observed.exclusive == (l.vehicle_owner)(v)
}
fn reserve_consist(
    st: LoadingStation,
    front: VehicleRef,
    control: &LoadingControl,
    mut capleft: Option<&mut [u32; 64]>,
    next: *mut c_void,
    w: &World,
    l: &Loading,
) {
    let must_reserve = !control.refit || (l.get_payment)(front).is_null();
    let mut current = Some(front);
    while let Some(v) = current {
        let reserve_part = if (l.is_articulated_part)(v) {
            false
        } else {
            let kind = (l.vehicle_type)(v);
            (kind != 0 || !(l.is_rear_dualheaded)(v))
                && (kind != 3 || (l.is_normal_aircraft)(v))
                && (must_reserve || control.refit_cargo == (l.cargo_type)(v))
        };
        if reserve_part {
            parts(l, v, |part| {
                let observed = (l.cargo_capacity)(part);
                let cap = u32::from(observed.capacity);
                let remaining = observed.remaining();
                if cap > remaining && may_load(st, part, w, l) {
                    (l.reserve_cargo)(st.native, part, cap - remaining, next);
                }
                true
            });
        }
        if let Some(ref mut capleft) = capleft
            && (l.cargo_cap)(v) != 0
        {
            let observed = (l.cargo_capacity)(v);
            let cargo = usize::from(observed.cargo);
            capleft[cargo] = capleft[cargo]
                .wrapping_add(u32::from(observed.capacity).wrapping_sub(observed.remaining()));
        }
        current = (l.next_vehicle)(v);
    }
}
fn load_amount(v: VehicleRef, gradual: bool, l: &Loading) -> u32 {
    let observed = (l.load_engine)(v);
    let engine = observed.engine;
    let mut amount = u32::from(observed.amount);
    let mail = (l.vehicle_type)(v) == 3 && !(l.is_normal_aircraft)(v);
    if mail {
        amount = amount.wrapping_add(3) / 4;
    }
    if gradual {
        let version = (l.grf_version)(engine);
        let mut cb = 0xffff;
        if version >= 8 {
            cb = (l.load_property)(v);
        } else if (l.engine_load_callback)(engine) {
            cb = (l.load_callback)(v);
        }
        if cb != 0xffff {
            if (l.grf_version)(engine) < 8 {
                cb &= 255;
            }
            if cb >= 256 {
                (l.unknown_load_result)(v, cb);
            } else if cb != 0 {
                amount = u32::from(cb);
            }
        }
    }
    if (l.engine_no_multiplier)(engine) && !mail {
        amount = amount
            .wrapping_mul(u32::from((l.cargo_multiplier)(v)))
            .wrapping_add(255)
            / 256;
    }
    amount.max(1)
}
fn refit_vehicle(
    v: VehicleRef,
    capleft: &mut [u32; 64],
    st: LoadingStation,
    next: *mut c_void,
    mut new_cargo: u8,
    w: &World,
    l: &Loading,
) {
    let start = (l.first_engine_part)(v);
    if !parts(l, start, |part| (l.stored_count)(part) == 0) {
        return;
    }
    let old_company = (l.current_company)();
    (l.set_current_company)((l.vehicle_owner)(v));
    let mut mask = (l.refit_mask)(v);
    parts(l, start, |part| {
        let observed = (l.cargo_reservation)(part);
        let cargo = usize::from(observed.cargo);
        capleft[cargo] = capleft[cargo]
            .wrapping_sub(u32::from(observed.capacity).wrapping_sub(observed.reserved));
        mask |= observed.mask;
        true
    });
    let auto = new_cargo == 253;
    if auto {
        new_cargo = (l.cargo_type)(start);
        for cargo in 0..64_u8 {
            if mask & (1_u64 << cargo) == 0 || !(l.has_cargo_for)(st.native, cargo, next) {
                continue;
            }
            let capacity = (l.test_refit)(start, cargo);
            if capacity > 0
                && (capleft[usize::from(cargo)] < capleft[usize::from(new_cargo)]
                    || (capleft[usize::from(cargo)] == capleft[usize::from(new_cargo)]
                        && (w.available)(st.base, cargo) > (w.available)(st.base, new_cargo)))
            {
                new_cargo = cargo;
            }
        }
    }
    if new_cargo < 64 && u32::from(new_cargo) != u32::from((l.cargo_type)(start)) {
        parts(l, start, |part| {
            (l.return_cargo)(st.native, part, u32::MAX);
            true
        });
        let mut cost = 0;
        let succeeded = (l.execute_refit)(start, new_cargo, &raw mut cost);
        if succeeded {
            (l.subtract_profit)((l.first_vehicle)(v), cost.wrapping_shl(8));
        }
    }
    let do_reserve = auto || (l.full_load_order)((l.first_vehicle)(v));
    parts(l, start, |part| {
        let observed = (l.cargo_capacity)(part);
        let remaining = u32::from(observed.capacity).wrapping_sub(observed.remaining());
        if do_reserve {
            (l.reserve_cargo)(st.native, part, remaining, next);
        }
        // Reserve changes RemainingCount; observe again at the original point.
        let observed = (l.cargo_capacity)(part);
        let cargo = usize::from(observed.cargo);
        capleft[cargo] = capleft[cargo]
            .wrapping_add(u32::from(observed.capacity).wrapping_sub(observed.remaining()));
        true
    });
    (l.set_current_company)(old_company);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_prepare(
    front: VehicleRef,
    world: *const World,
    loading: *const Loading,
) {
    let w = unsafe { &*world };
    let l = unsafe { &*loading };
    // SAFETY: the shared immutable vehicle table outlives this entry. Station
    // identities and owner are resolved before next-station discovery (#228).
    let vehicle = unsafe { &*l.vehicle };
    let st = (w.station)((vehicle.last_station)(front));
    unsafe {
        openttd_rust_station_queue_push(st.state, front.0.as_ptr().cast());
    }
    (l.set_loading_finished)(front, false);
    (l.set_load_ticks)(front, 1);
    let payment = (l.create_payment)(front);
    let mut body = |next| {
        if !(l.has_orders)(front) || (l.unload_type)(front) != 4 {
            let mut current = Some(front);
            while let Some(v) = current {
                if u32::from((l.cargo_cap)(v)) > 0 && (l.total_count)(v) > 0 {
                    (l.stage)(st.native, v, (l.unload_type)(front), next, payment);
                    if (l.unload_count)(v) > 0 {
                        (l.set_cargo_unloading)(v, true);
                    }
                }
                current = (l.next_vehicle)(v);
            }
        }
    };
    let (context, callback) = scope_context(&mut body);
    (l.with_next_stations)(front, context, callback);
}
fn update_ticks(front: VehicleRef, st: LoadingStation, kind: u8, mut ticks: i32, l: &Loading) {
    if kind == 0 && (l.station_length_penalty)() {
        // Platform length is a world-geometry query on the native station shell.
        let overhang = u32::from((l.cached_total_length)(front))
            .wrapping_sub((l.platform_length)(st.native, front).wrapping_mul(16))
            as i32;
        if overhang > 0 {
            ticks = ticks.wrapping_shl(1);
            ticks = ticks.wrapping_add(overhang.wrapping_mul(ticks) / 8);
        }
    }
    (l.set_load_ticks)(front, ticks.max(1) as u16);
}
// Preserve complete loading control at the source observation points.
#[allow(clippy::too_many_lines)]
unsafe fn load_vehicle(front: VehicleRef, w: &World, l: &Loading) {
    // SAFETY: the shared immutable vehicle table outlives this entry. Station
    // identities and owner are resolved before next-station discovery (#228).
    let vehicle = unsafe { &*l.vehicle };
    let st = (w.station)((vehicle.last_station)(front));
    let mut body = |next| {
        let control = (l.control)(front);
        let auto = control.refit && control.refit_cargo == 253;
        let mut capleft = [0_u32; 64];
        let reserve = if control.improved && auto {
            (l.get_payment)(front).is_null()
        } else {
            control.full_load
        };
        if reserve {
            if control.full_load {
                crate::witness::hit(crate::witness::STATION_FULL_LOAD_RESERVE);
            }
            reserve_consist(
                st,
                front,
                &control,
                if auto && control.ticks != 0 {
                    Some(&mut capleft)
                } else {
                    None
                },
                next,
                w,
                l,
            );
        }
        if control.ticks != 0 {
            return;
        }
        if control.vehicle_type == 0 && {
            let tile = (l.vehicle_tile)(front);
            let shared = unsafe { &*w.shared };
            (shared.tile_type)(tile) != crate::vehicle::MP_STATION
                || (shared.station_index)(tile) != (l.station_index)(st.native)
        } {
            (l.set_loading_finished)(front, true);
            (l.set_load_ticks)(front, 1);
            return;
        }
        let state = st.state;
        let mut ticks = 0_i32;
        let mut dirty_vehicle = false;
        let mut dirty_station = false;
        let mut emptied = true;
        let mut unloaded = false;
        let mut loaded_any = false;
        let mut full_load = 0_u64;
        let mut cargo_not_full = 0_u64;
        let mut cargo_full = 0_u64;
        let mut reservation_left = 0_u64;
        (l.set_speed)(front, 0);
        let payment = (l.get_payment)(front);
        let mut artic_part = 0_u32;
        let mut current = Some(front);
        while let Some(v) = current {
            if v == front || !(l.has_articulated_part)((l.previous_vehicle)(v)) {
                artic_part = 0;
            }
            let part = (l.part)(v);
            if part.capacity == 0 {
                current = (l.next_vehicle)(v);
                continue;
            }
            artic_part = artic_part.wrapping_add(1);
            let mut cargo = part.cargo;
            let mut ge = unsafe { cp(state, cargo) };
            if part.unloading && control.unload_type != 4 {
                let count = (l.unload_count)(v);
                let mut amount = if control.gradual {
                    if count > 0 {
                        crate::witness::hit(crate::witness::STATION_GRADUAL_UNLOAD);
                    }
                    count.min(load_amount(v, control.gradual, l))
                } else {
                    count
                };
                if unsafe { (*ge).status } & 1 == 0 && (l.deliver_count)(v) > 0 {
                    crate::witness::hit(crate::witness::STATION_REJECTED_DELIVERY);
                    let unload_type = control.unload_type;
                    if unload_type == 2 || unload_type == 1 {
                        (l.deliver_to_transfer)(v, (l.deliver_count)(v));
                    } else {
                        let observed = (l.delivery_remaining)(v);
                        let remaining = observed.remaining.wrapping_add(observed.deliver);
                        if u32::from(observed.capacity) < remaining {
                            (l.return_cargo)(
                                st.native,
                                v,
                                remaining - u32::from(observed.capacity),
                            );
                        }
                        (l.deliver_to_keep)(v, (l.deliver_count)(v));
                        unloaded = true;
                    }
                }
                if (l.transfer_count)(v) > 0 {
                    dirty_station = true;
                    if unsafe { (*ge).status } & 2 == 0 {
                        unsafe {
                            (&raw mut (*ge).time_since_pickup).write(0);
                            status(state, cargo, 1, true);
                        }
                    }
                }
                crate::witness::hit(crate::witness::STATION_UNLOAD_CARGO);
                amount = (l.unload_cargo)(st.native, v, amount, payment);
                let remaining = (l.unload_count)(v) > 0;
                if amount > 0 {
                    dirty_vehicle = true;
                    unloaded = true;
                    ticks = ticks.wrapping_add(amount as i32);
                    unsafe {
                        (&raw mut (*state).fields.time_since_unload).write(0);
                    }
                }
                if control.gradual && remaining {
                    emptied = false;
                } else {
                    (l.set_cargo_unloading)(v, false);
                }
                current = (l.next_vehicle)(v);
                continue;
            }
            if control.load_type == 4 || control.stop {
                current = (l.next_vehicle)(v);
                continue;
            }
            let refitted = control.refit && artic_part == 1;
            if refitted {
                refit_vehicle(v, &mut capleft, st, next, control.refit_cargo, w, l);
            }
            let observed = (l.set_refit_capacity_and_stats)(
                v,
                front,
                control.vehicle_type,
                part.capacity,
                refitted,
            );
            if refitted {
                cargo = observed.cargo;
                ge = unsafe { cp(state, cargo) };
            }
            unsafe {
                (&raw mut (*ge).last_speed).write(observed.speed.clamp(0, 255) as u8);
                (&raw mut (*ge).last_age).write(observed.age.clamp(0, 255) as u8);
            }
            let mut cap = u32::from(observed.capacity).wrapping_sub(observed.stored);
            if cap > 0 {
                unsafe {
                    (&raw mut (*ge).time_since_pickup).write(0);
                }
                if (observed.load_count > 0 || (w.available)(st.base, cargo) > 0)
                    && may_load(st, v, w, l)
                {
                    if observed.stored == 0 {
                        (l.vehicle_new_cargo)(v);
                    }
                    if control.gradual {
                        cap = cap.min(load_amount(v, control.gradual, l));
                    }
                    let loaded = (l.load_cargo)(st.native, v, cap, next);
                    if (l.load_count)(v) > 0 {
                        reservation_left |= 1 << cargo;
                    }
                    if loaded == cap {
                        full_load |= 1 << cargo;
                    } else {
                        full_load &= !(1 << cargo);
                    }
                    if loaded > 0 {
                        emptied = false;
                        loaded_any = true;
                        unsafe {
                            (&raw mut (*state).fields.time_since_load).write(0);
                            (&raw mut (*state).fields.last_vehicle_type).write(part.vehicle_type);
                        }
                        if (l.station_cargo_total)(st.native, cargo) == 0 {
                            (l.cargo_taken)(st.native, cargo);
                        }
                        ticks = ticks.wrapping_add(loaded as i32);
                        dirty_vehicle = true;
                        dirty_station = true;
                    }
                }
            }
            let stored = (l.cargo_stored)(v);
            if stored.stored >= u32::from(stored.capacity) {
                cargo_full |= 1 << cargo;
            } else {
                cargo_not_full |= 1 << cargo;
            }
            current = (l.next_vehicle)(v);
        }
        if loaded_any || unloaded {
            if control.vehicle_type == 0 {
                (l.train_loads)(st.native, front);
            } else if control.vehicle_type == 1 {
                (l.road_loads)(st.native, front);
            }
        }
        emptied &= unloaded;
        if !unloaded {
            (l.delete_payment)(payment);
        }
        (l.set_stop)(front, false);
        if loaded_any || unloaded {
            if control.gradual {
                ticks = [40, 20, 10, 20][control.vehicle_type as usize];
            }
            if !unloaded && full_load == 0 && reservation_left == 0 && !control.full_load && {
                let observed = (l.wait_observation)(front);
                let wait = i32::from(observed.wait)
                    .wrapping_sub(observed.lateness)
                    .max(0);
                (l.order_time)(front) >= wait
            } {
                (l.set_stop)(front, true);
            }
            update_ticks(front, st, control.vehicle_type, ticks, l);
        } else {
            update_ticks(front, st, control.vehicle_type, 20, l);
            let mut finished = true;
            if control.full_load {
                if control.load_type == 3 {
                    if (control.vehicle_type == 3
                        && (l.passenger_cargo)(front)
                        && u32::from((l.cargo_cap)(front)) > (l.stored_count)(front))
                        || (cargo_not_full != 0 && (cargo_full & !cargo_not_full) == 0)
                    {
                        finished = false;
                    }
                } else if cargo_not_full != 0 {
                    finished = false;
                }
                if !finished {
                    (l.refresh_capacity)(front);
                }
            }
            (l.set_loading_finished)(front, finished);
        }
        (l.update_indicator)(front);
        if emptied {
            dirty_vehicle = true;
            (l.vehicle_empty)(front);
        }
        if dirty_vehicle {
            (l.vehicle_dirty)(front);
        }
        if dirty_station {
            (l.station_dirty)(st.native);
        }
    };
    let (context, callback) = scope_context(&mut body);
    (l.with_next_stations)(front, context, callback);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_load(
    state: *mut Service,
    world: *const World,
    loading: *const Loading,
) {
    // C++ supplies the queue owner directly: no callback precedes the empty
    // queue check, and the queue station is not the vehicles' loading station.
    let mut node = unsafe { openttd_rust_station_queue_first(state) };
    if node.is_null() {
        return;
    }
    let w = unsafe { &*world };
    let l = unsafe { &*loading };
    let mut last = None;
    while !node.is_null() {
        let v = VehicleRef(unsafe {
            NonNull::new_unchecked(openttd_rust_station_queue_value(node).cast())
        });
        if (l.status)(v) & (VS_STOPPED | VS_CRASHED) == 0 {
            let ticks = (l.load_unload_ticks)(v).wrapping_sub(1);
            (l.set_load_ticks)(v, ticks);
            if ticks == 0 {
                last = Some(v);
            }
        }
        node = unsafe { openttd_rust_station_queue_next(state, node) };
    }
    if last.is_none() {
        return;
    }
    node = unsafe { openttd_rust_station_queue_first(state) };
    while !node.is_null() {
        let v = VehicleRef(unsafe {
            NonNull::new_unchecked(openttd_rust_station_queue_value(node).cast())
        });
        if (l.status)(v) & (VS_STOPPED | VS_CRASHED) == 0 {
            unsafe {
                load_vehicle(v, w, l);
            }
        }
        if Some(v) == last {
            break;
        }
        node = unsafe { openttd_rust_station_queue_next(state, node) };
    }
    (l.flush_delivery)();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_modify_rating(state: *mut Service, amount: i32) {
    for cargo in 0..64 {
        let ge = unsafe { cp(state, cargo) };
        if unsafe { (*ge).status } != 0 {
            unsafe {
                (&raw mut (*ge).rating)
                    .write(i32::from((*ge).rating).wrapping_add(amount).clamp(0, 255) as u8);
            }
        }
    }
}
