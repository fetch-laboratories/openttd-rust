/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Station cargo-service state. Addresses are stable for the shell lifetime, including
//! indexed loading and retired-station reuse.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
use crate::cargo_payment::NativePayment;
use crate::cargo_type::CARGO_AUTO_REFIT;
use crate::cargopacket::MoveToAction;
use crate::command_type::DoCommandFlag;
use crate::company_type::{INVALID_OWNER, OWNER_END, OWNER_NONE};
use crate::linkgraph::live;
use crate::order_type::{OrderLoadType, OrderUnloadType};
use crate::orders::NativeOrderList;
use crate::station::{BaseStation, Station};
use crate::station_base::goods_entry::{State as GoodsState, States as GoodsStates};
use crate::station_type::{StationFacilities, StationFacility};
use crate::timer_game_common::INVALID_DATE;
use crate::vehicle::VehicleRef;
use crate::vehicle::visits::{Services as Visits, refresh_links};
use crate::vehicle_base::{VS_CRASHED, VS_STOPPED};
use crate::vehicle_type::VehicleType;
use std::ffi::c_void;
use std::ptr::NonNull;
/// `GoodsEntry::ConvertState`: the four history bits keep their native order.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_goods_convert_state(status: u8) -> u8 {
    ((status >> GoodsState::EverAccepted.0) & 1)
        | (((status >> GoodsState::LastMonth.0) & 1) << 1)
        | (((status >> GoodsState::CurrentMonth.0) & 1) << 2)
        | (((status >> GoodsState::AcceptedBigtick.0) & 1) << 3)
}

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
            rating: crate::station_base::INITIAL_STATION_RATING,
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
    catchment: crate::station::Catchment,
}
/// The station's catchment state, for immediate borrows only.
/// # Safety
/// `state` is a live owner.
pub(crate) unsafe fn catchment(state: *mut Service) -> *mut crate::station::Catchment {
    unsafe { &raw mut (*state).catchment }
}
/// SAFETY: Live serialized station owner; no callback occurs during these accesses.
pub(crate) unsafe fn always_accepts(state: *const Service, cargo: u8) -> bool {
    unsafe { (*state).fields.always_accepted & (1_u64 << cargo) != 0 }
}
/// Record the original `GoodsEntry` delivery history bits.
/// SAFETY: Live serialized station owner and valid cargo slot.
pub(crate) unsafe fn record_delivery(state: *mut Service, cargo: u8) {
    unsafe {
        (*cp(state, cargo)).status |= (1 << GoodsState::EverAccepted.0)
            | (1 << GoodsState::CurrentMonth.0)
            | (1 << GoodsState::AcceptedBigtick.0);
    };
}
/// Clear every goods rating at the failed-bribe point, without crossing back
/// into the native station shell.
/// # Safety
/// The caller supplies a live serialized station owner.
pub(crate) unsafe fn clear_ratings(state: *mut Service) {
    // SAFETY: No callback or other access while the immediate slots are written.
    for cargo in unsafe { &mut (*state).cargo } {
        cargo.rating = 0;
    }
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
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_service_new() -> *mut Service {
    Box::into_raw(Box::new(Service {
        fields: Fields {
            always_accepted: 0,
            delete_ctr: 0,
            time_since_load: 255,
            time_since_unload: 255,
            last_vehicle_type: VehicleType::VEH_INVALID.0,
        },
        cargo: [CargoFields::default(); 64],
        queue: Vec::new(),
        catchment: crate::station::Catchment::default(),
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
pub struct LoadingControl {
    pub(crate) ticks: u16,
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
    pub(crate) cargo: u8,
    pub(crate) unloading: bool,
}
#[repr(C)]
pub struct LoadingWait {
    pub(crate) lateness: i32,
    pub(crate) wait: u16,
}
#[repr(C)]
pub struct OrderDestination {
    pub(crate) destination: u16,
    pub(crate) kind: crate::vehicle::OrderType,
}
#[repr(C)]
pub struct TruncatedSource {
    pub(crate) amount: u32,
    pub(crate) source: u16,
    pub(crate) present: bool,
}
#[repr(C)]
pub struct World {
    pub(crate) graphs: &'static live::Services,
    pub(crate) station: extern "C" fn(u16) -> LoadingStation,
    pub(crate) station_owner: extern "C" fn(*mut BaseStation) -> u8,
    pub(crate) station_exclusivity: extern "C" fn(*mut BaseStation) -> StationExclusivity,
    pub(crate) rating_counts: extern "C" fn(*mut BaseStation, u8) -> RatingCounts,
    pub(crate) get: extern "C" fn(u16) -> *mut BaseStation,
    pub(crate) owner: extern "C" fn(*mut BaseStation) -> *mut Service,
    pub(crate) pool_size: extern "C" fn() -> usize,
    pub(crate) environment: crate::services::EnvironmentServices,
    pub(crate) facilities: extern "C" fn(*mut BaseStation) -> u8,
    pub(crate) in_use: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) is_station: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) local_owner: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) town_statue: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) select_goods: extern "C" fn() -> bool,
    pub(crate) rating_cheat: extern "C" fn() -> bool,
    pub(crate) available: extern "C" fn(*mut BaseStation, u8) -> u32,
    pub(crate) cargo_valid: extern "C" fn(u8) -> bool,
    pub(crate) rating_callback_enabled: extern "C" fn(u8) -> bool,
    pub(crate) passenger_cargo: extern "C" fn(u8) -> bool,
    pub(crate) packet_available: extern "C" fn() -> bool,
    pub(crate) rating_redraw: extern "C" fn(*mut BaseStation, bool),
    pub(crate) acceptance_news: extern "C" fn(*mut BaseStation, u64, bool),
    pub(crate) acceptance_redraw: extern "C" fn(*mut BaseStation),
    pub(crate) delete_station: extern "C" fn(*mut BaseStation),

    pub(crate) append_packet:
        extern "C" fn(*mut BaseStation, u8, u32, crate::cargo_storage::Source),
    pub(crate) station_list_invalidation: extern "C" fn(*mut BaseStation, i32),

    pub(crate) waiting_redraw: extern "C" fn(*mut BaseStation),
    pub(crate) cargo_registry: *const crate::cargo_registry::Services,
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
pub(crate) type Scope = extern "C" fn(*mut c_void, *mut c_void);
extern "C" fn scoped<F: FnMut(*mut c_void)>(context: *mut c_void, frame: *mut c_void) {
    // SAFETY: with_* invokes this callback exactly once, before the stack context
    // ends; only the callback accesses its private closure during the invocation.
    unsafe {
        (&mut *context.cast::<F>())(frame);
    }
}
pub(crate) fn scope_context<F: FnMut(*mut c_void)>(body: &mut F) -> (*mut c_void, Scope) {
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
/// `GoodsEntry::HasRating()` of a live station owner.
pub(crate) fn has_rating(state: *mut Service, cargo: u8) -> bool {
    // SAFETY: scalar read of the live serialized owner; no borrow survives.
    unsafe { crate::station_base::has_rating((*cp(state, cargo)).status) }
}
/// Read only the fields consumed by a station `NewGRF` cargo variable. Container
/// counts, source and periods are queried directly by the resolver's cargo owner.
/// No station borrow survives a service call.
pub(crate) unsafe fn newgrf_cargo_variable(
    state: *mut Service,
    cargo: u8,
    variable: u8,
    legacy: bool,
) -> u32 {
    let ge = unsafe { cp(state, cargo) };
    unsafe {
        match variable {
            0x61 => {
                if legacy || crate::station_base::has_vehicle_ever_tried_loading((*ge).last_speed) {
                    u32::from((*ge).time_since_pickup)
                } else {
                    0
                }
            }
            0x62 => {
                if legacy || has_rating(state, cargo) {
                    u32::from((*ge).rating)
                } else {
                    u32::MAX
                }
            }
            0x64 => {
                let speed = (*ge).last_speed;
                if crate::station_base::has_vehicle_ever_tried_loading(speed) {
                    u32::from(speed) | (u32::from((*ge).last_age) << 8)
                } else {
                    0xff00
                }
            }
            0x65 => {
                if GoodsStates((*ge).status).test(GoodsState::Acceptance) {
                    8
                } else {
                    0
                }
            }
            0x69 => u32::from(openttd_rust_goods_convert_state((*ge).status)),
            0x66 => u32::from((*ge).last_speed),
            0x67 => u32::from((*ge).last_age),
            _ => unreachable!(),
        }
    }
}
unsafe fn status(state: *mut Service, cargo: u8, bit: GoodsState, value: bool) {
    let p = unsafe { &raw mut (*cp(state, cargo)).status };
    let old = unsafe { p.read() };
    unsafe {
        p.write(if value {
            old | (1 << bit.0)
        } else {
            old & !(1 << bit.0)
        });
    }
}
pub(crate) unsafe fn acceptance_mask(state: *mut Service) -> u64 {
    let mut mask = 0;
    for cargo in 0..64 {
        if GoodsStates(unsafe { (*cp(state, cargo)).status }).test(GoodsState::Acceptance) {
            mask |= 1 << cargo;
        }
    }
    mask
}
/// Native facade for the canonical `GetAcceptanceMask` reducer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_acceptance_mask(state: *mut Service) -> u64 {
    unsafe { acceptance_mask(state) }
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
    // SAFETY: Live owner; the tile procs cannot reset this station's catchment.
    if !unsafe { crate::station::rect_is_empty(state) } {
        let c = crate::station::Ctx::from_shared(unsafe { &*w.shared });
        let always = unsafe { &raw mut (*state).fields.always_accepted };
        acceptance =
            unsafe { crate::station::get_acceptance_around_station_raw(state, always, &c) };
    }
    let facilities = u32::from((w.facilities)(st));
    for cargo in 0..64 {
        let mut amt = acceptance[usize::from(cargo)];
        let passenger = u32::from((w.passenger_cargo)(cargo)) != 0;
        // Make sure the station can accept this cargo type.
        if facilities
            & u32::from(
                (StationFacilities::from_element(StationFacility::Train)
                    | StationFacilities::from_element(if passenger {
                        StationFacility::BusStop
                    } else {
                        StationFacility::TruckStop
                    })
                    | StationFacilities::from_element(StationFacility::Airport)
                    | StationFacilities::from_element(StationFacility::Dock))
                .0,
            )
            == 0
        {
            amt = 0;
        }
        unsafe {
            status(state, cargo, GoodsState::Acceptance, amt >= 8);
        }
        unsafe {
            live::station_demand(st, cargo, amt / 8, w.graphs);
        }
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
        if GoodsStates(unsafe { (*cp(state, cargo)).status }).test(GoodsState::AcceptedBigtick) {
            cargoes |= 1 << cargo;
        }
    }
    if cargoes == 0 {
        return;
    }
    // The immutable shared table outlives this entry.
    let shared = unsafe { &*w.shared };
    let c = crate::station::Ctx::from_shared(shared);
    // SAFETY: Live owner; house animation triggers cannot reset its catchment.
    unsafe {
        crate::station::for_each_catchment_tile_raw(state, &c, |tile| {
            if (shared.tile_type)(tile) == crate::tile_type::MP_HOUSE {
                crate::town::house::trigger_house_animation_watched_cargo_accepted(
                    c.c.town,
                    shared,
                    crate::tile_type::TileIndex(tile),
                    crate::cargo_type::CargoTypes(cargoes),
                );
            }
        });
    }
}
// Preserve the complete source rating loop and its immediate mutation order.
#[allow(clippy::too_many_lines)]
pub unsafe extern "C" fn openttd_rust_station_rating(st: *mut BaseStation, world: *const World) {
    const WAITING_CARGO_THRESHOLD: u32 = 1 << 12;
    const WAITING_CARGO_CUT_FACTOR: u32 = 1 << 6;
    const MAX_WAITING_CARGO: u32 = 1 << 15;
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
        // The station might not currently be moving this cargo.
        if !GoodsStates(unsafe { (*ge).status }).test(GoodsState::Rating) {
            // Slowly restore the initial rating after, for example, a failed
            // bribe before this cargo ever reached the station.
            unsafe {
                if (*ge).rating < crate::station_base::INITIAL_STATION_RATING {
                    (&raw mut (*ge).rating).write((*ge).rating + 1);
                }
            }
            continue;
        }
        unsafe {
            (&raw mut (*ge).time_since_pickup).write((*ge).time_since_pickup.saturating_add(1));
        }
        // Get rid of cargo that has not been picked up in a long time.
        if unsafe { (*ge).time_since_pickup } == 255 && u32::from((w.select_goods)()) != 0 {
            unsafe {
                status(state, cargo, GoodsState::Rating, false);
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
        // INVALID_STATION also counts as a destination, so any cargo has
        // at least one destination.
        let num_dests = counts.destinations;
        // Average per next hop, with more allowance for solitary stations
        // with one or two next hops. Manual distribution has only the invalid
        // destination and therefore divides waiting by two.
        let waiting_avg = waiting / num_dests.wrapping_add(1);
        if u32::from((w.rating_cheat)()) != 0 {
            unsafe {
                (&raw mut (*ge).rating).write(255);
            }
            rating = 255;
            skip = true;
        } else if u32::from((w.rating_callback_enabled)(cargo)) != 0 {
            // A successful custom rating skips speed, pickup time and waiting
            // cargo ratings. NewGRFs expect 0xFF if no vehicle has arrived yet.
            let speed = unsafe { (*ge).last_speed };
            let var18 = unsafe {
                u32::from((*ge).time_since_pickup)
                    | ((*ge).max_waiting_cargo.min(65535) << 8)
                    | (u32::from(if speed == 0 { 255 } else { speed }) << 24)
            };
            let kind = unsafe { (*state).fields.last_vehicle_type };
            // Convert to the old vehicle types.
            let var10 = if kind == VehicleType::VEH_INVALID.0 {
                0
            } else {
                u32::from(kind) + 0x10
            };
            let cb = unsafe {
                crate::cargo_registry::get_cargo_callback(
                    &*w.cargo_registry,
                    cargo,
                    crate::newgrf_callbacks::CBID_CARGO_STATION_RATING_CALC.0,
                    var10,
                    var18,
                )
            };
            if cb != crate::newgrf_callbacks::CALLBACK_FAILED {
                skip = true;
                rating = i32::from(cb & 0x3fff);
                // Simulate a 15-bit signed value.
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
            if unsafe { (*state).fields.last_vehicle_type } == VehicleType::SHIP.0 {
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
        // Only modify the rating in steps of -2, -1, 0, 1 or 2.
        rating = (old_rating + (rating - old_rating).clamp(-2, 2)).clamp(0, 255);
        unsafe {
            (&raw mut (*ge).rating).write(rating as u8);
        }
        // With rating <= 64 and at least 100 waiting per destination, remove
        // a random amount of goods.
        if rating <= 64 && waiting_avg >= 100 {
            let mut dec = w.random() & 31;
            if waiting_avg < 200 {
                dec &= 7;
            }
            waiting = waiting.wrapping_sub((dec + 1).wrapping_mul(num_dests));
            waiting_changed = true;
        }
        // With rating <= 127, maybe remove some of the remaining goods.
        if rating <= 127 && waiting != 0 {
            let random = w.random();
            if rating <= (random & 127) as i32 {
                // Signed arithmetic prevents unsigned overflow here.
                waiting = (waiting as i32)
                    .wrapping_sub((((random >> 8) & 3) + 1).wrapping_mul(num_dests) as i32)
                    .max(0) as u32;
                waiting_changed = true;
            }
        }
        // Replace the old strict 4095 cap with increasingly aggressive
        // truncation as the amount of waiting cargo grows.
        if waiting > WAITING_CARGO_THRESHOLD {
            waiting -= (waiting - WAITING_CARGO_THRESHOLD) / WAITING_CARGO_CUT_FACTOR;
            waiting = waiting.min(MAX_WAITING_CARGO);
            waiting_changed = true;
        }
        let available = (w.available)(st, cargo);
        // Reserved cargo cannot be truncated. Feed back the exact own waiting
        // cargo for the next rating calculation.
        if waiting_changed && waiting < available {
            unsafe {
                (&raw mut (*ge).max_waiting_cargo).write(0);
                truncate(st, cargo, available - waiting, w);
            }
        } else {
            // Be more forgiving when the average per next hop is low.
            unsafe {
                (&raw mut (*ge).max_waiting_cargo).write(waiting_avg);
            }
        }
    }
    (w.rating_redraw)(st, u32::from(waiting_changed) != 0);
}
pub(crate) unsafe fn openttd_rust_station_monthly(world: *const World) {
    let w = unsafe { &*world };
    for id in 0..(w.pool_size)() {
        let st = (w.get)(id as u16);
        if st.is_null() || u32::from((w.is_station)(st)) == 0 {
            continue;
        }
        let state = (w.owner)(st);
        for cargo in 0..64 {
            let current =
                GoodsStates(unsafe { (*cp(state, cargo)).status }).test(GoodsState::CurrentMonth);
            unsafe {
                status(state, cargo, GoodsState::LastMonth, current);
                status(state, cargo, GoodsState::CurrentMonth, false);
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
    visits: &Visits,
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
    unsafe {
        live::station_supply(st, cargo, amount, w.graphs, visits.schedule);
    }
    if !GoodsStates(unsafe { (*ge).status }).test(GoodsState::Rating) {
        (w.station_list_invalidation)(st, 0);
        unsafe {
            status(state, cargo, GoodsState::Rating, true);
        }
    }
    crate::animated_tile::cargo_event(
        st,
        cargo,
        crate::station_type::SAT_NEW_CARGO,
        crate::station_type::AAT_NEW_CARGO,
        crate::station_type::SRT_NEW_CARGO,
    );
    (w.waiting_redraw)(st);
    amount
}
/// `MoveGoodsToStation` over a live station set.
/// # Safety
/// `stations`, `world` and `visits` are live for the call; the set may be
/// re-read between station observations.
pub(crate) unsafe fn distribute(
    cargo: u8,
    mut amount: u32,
    source: crate::cargo_storage::Source,
    stations: *const crate::station::StationList,
    exclusivity: u8,
    world: *const World,
    visits: &Visits,
) -> u32 {
    if amount == 0 {
        return 0;
    }
    let w = unsafe { &*world };
    let mut used = Vec::new();
    let mut first = None;
    // SAFETY: Caller contract; observations below do not change station sets.
    unsafe {
        crate::station::for_each_station(stations, |entry| {
            let st = entry.station;
            let owner = (w.station_owner)(st);
            if exclusivity != INVALID_OWNER && exclusivity != owner {
                return;
            }
            if owner != OWNER_NONE {
                let observed = (w.station_exclusivity)(st);
                if observed.counter > 0 && observed.exclusive != owner {
                    return;
                }
            }
            let ge = cp((w.owner)(st), cargo);
            let rating = (*ge).rating;
            if rating == 0 {
                return;
            }
            if u32::from((w.select_goods)()) != 0 && (*ge).last_speed == 0 {
                return;
            }
            let passenger = u32::from((w.passenger_cargo)(cargo)) != 0;
            // Passengers cannot use just a truck stop, nor freight just a bus stop.
            if u32::from((w.facilities)(st))
                == u32::from(
                    StationFacilities::from_element(if passenger {
                        StationFacility::TruckStop
                    } else {
                        StationFacility::BusStop
                    })
                    .0,
                )
            {
                return;
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
        });
    };
    let Some((st, _, rating, _)) = first else {
        return 0;
    };
    if used.is_empty() {
        return unsafe {
            admit(
                st,
                cargo,
                amount.wrapping_mul(rating + 1),
                source,
                w,
                visits,
            )
        };
    }
    let mut company_best = [0_u32; OWNER_END as usize];
    let mut company_sum = [0_u32; OWNER_END as usize];
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
        moved = moved.wrapping_add(unsafe { admit(st, cargo, share, source, w, visits) });
    }
    moved
}

#[repr(C)]
#[derive(Default)]
pub struct Edge {
    pub(crate) destination: *mut BaseStation,
    pub(crate) distance: u32,
}
#[repr(C)]
pub struct Links {
    pub(crate) graphs: &'static live::Services,
    pub(crate) automatic_distribution: extern "C" fn(u8) -> bool,
    pub(crate) edge: extern "C" fn(*mut BaseStation, u16) -> Edge,
    pub(crate) delete_flows: extern "C" fn(*mut BaseStation, u8, u16),
    pub(crate) restrict_flows: extern "C" fn(*mut BaseStation, u8, u16),
    pub(crate) order_list: extern "C" fn(u16) -> *mut NativeOrderList,
    pub(crate) order_pool_size: extern "C" fn() -> usize,
    pub(crate) order_count: extern "C" fn(*mut NativeOrderList) -> usize,
    pub(crate) order_destination: extern "C" fn(*mut NativeOrderList, usize) -> OrderDestination,
    pub(crate) order_vehicle: extern "C" fn(*mut NativeOrderList) -> Option<VehicleRef>,
    pub(crate) next_shared: extern "C" fn(VehicleRef) -> Option<VehicleRef>,
    pub(crate) stopped_in_depot: extern "C" fn(VehicleRef) -> bool,
    pub(crate) last_service: extern "C" fn(VehicleRef) -> i32,
    pub(crate) cargo_type: extern "C" fn(VehicleRef) -> u8,
    pub(crate) goods_entry:
        extern "C" fn(*mut BaseStation, u8) -> *const crate::station_base::GoodsEntry,
    pub(crate) station_resolver: *const crate::newgrf_station::Services,
    pub(crate) vehicle_resolver: *const crate::newgrf_vehicle::Services,
}
impl Links {
    pub(crate) fn storage(&self) -> &crate::cargo_storage::Services {
        unsafe { &*(*self.station_resolver).cargo }
    }
    pub(crate) fn station_cargo(
        &self,
        st: *mut BaseStation,
        cargo: u8,
    ) -> *const crate::cargo_storage::List {
        unsafe { ((*self.station_resolver).cargo_owner)(st, cargo) }
    }
    pub(crate) fn vehicle_cargo(&self, v: VehicleRef) -> *mut crate::cargo_storage::List {
        unsafe { ((*self.vehicle_resolver).cargo_owner)(v.0.as_ptr()) }.cast_mut()
    }
}
unsafe fn reroute(st: *mut BaseStation, cargo: u8, avoid: u16, avoid2: u16, w: &World, l: &Links) {
    let storage = l.storage();
    let goods = (l.goods_entry)(st, cargo);
    let list = l.station_cargo(st, cargo);
    if !list.is_null() {
        unsafe {
            crate::cargo_storage::reroute_station(list.cast_mut(), storage, goods, avoid, avoid2);
        }
    }
    let state = (w.owner)(st);
    let mut node = unsafe { openttd_rust_station_queue_first(state) };
    while !node.is_null() {
        let mut current =
            NonNull::new(unsafe { openttd_rust_station_queue_value(node) }).map(|shell| unsafe {
                VehicleRef::from_shell(shell.as_ptr(), (*l.vehicle_resolver).vehicle)
            });
        while let Some(vehicle) = current {
            if u32::from((l.cargo_type)(vehicle)) == u32::from(cargo) {
                unsafe {
                    crate::cargo_storage::reroute_vehicle(
                        l.vehicle_cargo(vehicle),
                        storage,
                        goods,
                        avoid,
                        avoid2,
                    );
                }
            }
            current = vehicle.next();
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
unsafe fn stale(from: *mut BaseStation, w: &World, l: &Links, visits: &Visits) {
    for cargo in 0..64 {
        let auto = (l.automatic_distribution)(cargo);
        let graph = live::station_graph(from, cargo, l.graphs);
        if graph.is_null() {
            continue;
        }
        let node = u32::from((l.graphs.node)(from, cargo)) as u16;
        let mut remove = Vec::new();
        let owner = graph;
        let count = unsafe { &*owner }.nodes[usize::from(node)].edges.len();
        for index in 0..count {
            let (destination, last_update) = {
                let g = unsafe { &*owner };
                let e = &g.nodes[usize::from(node)].edges[index];
                (g.nodes[usize::from(e.dest_node)].station, e.last_update())
            };
            let edge = (l.edge)(from, destination);
            let date = crate::timer_game_common::economy_date().0;
            let timeout = live::MIN_TIMEOUT_DISTANCE.wrapping_add(edge.distance >> 3) as i32;
            if date.wrapping_sub(last_update) > timeout {
                let mut updated = false;
                if auto {
                    let mut vehicles = Vec::new();
                    let from_id = (l.graphs.station_id)(from);
                    let to_id = (l.graphs.station_id)(edge.destination);
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
                            if !matches!(observed.kind.0, 1 | 8) {
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
                            || date.wrapping_sub((l.last_service)(vehicle))
                                <= live::STALE_LINK_DEPOT_TIMEOUT
                        {
                            refresh_links(vehicle, false, false, visits);
                        }
                        if unsafe { &*owner }.nodes[usize::from(node)].edges[index].last_update()
                            == date
                        {
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
                    remove.push((l.graphs.node)(edge.destination, cargo));
                    let to_id = (l.graphs.station_id)(edge.destination);
                    (l.delete_flows)(from, cargo, to_id);
                    unsafe {
                        reroute(from, cargo, to_id, (l.graphs.station_id)(from), w, l);
                    }
                }
            } else {
                let unrestricted = unsafe { &*owner }.nodes[usize::from(node)].edges[index]
                    .last_unrestricted_update;
                if unrestricted != INVALID_DATE && date.wrapping_sub(unrestricted) > timeout {
                    crate::witness::hit(crate::witness::STATION_STALE_RESTRICT);
                    unsafe { &mut *owner }.nodes[usize::from(node)].edges[index].restrict();
                    let to_id = (l.graphs.station_id)(edge.destination);
                    (l.restrict_flows)(from, cargo, to_id);
                    unsafe {
                        reroute(from, cargo, to_id, (l.graphs.station_id)(from), w, l);
                    }
                } else {
                    let restricted = unsafe { &*owner }.nodes[usize::from(node)].edges[index]
                        .last_restricted_update;
                    if restricted != INVALID_DATE && date.wrapping_sub(restricted) > timeout {
                        crate::witness::hit(crate::witness::STATION_STALE_RELEASE);
                        unsafe { &mut *owner }.nodes[usize::from(node)].edges[index].release();
                    }
                }
            }
        }
        for dest in remove {
            unsafe { &mut *owner }.nodes[usize::from(node)].remove_edge(dest);
        }
        let date = crate::timer_game_common::economy_date().0;
        if date.wrapping_sub(unsafe { &*owner }.last_compression) > live::COMPRESSION_INTERVAL {
            unsafe { &mut *owner }.compress(date);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_stale(
    st: *mut BaseStation,
    world: *const World,
    links: *const Links,
    visits: *const Visits,
) {
    let w = unsafe { &*world };
    let l = unsafe { &*links };
    let visits = unsafe { &*visits };
    unsafe {
        stale(st, w, l, visits);
    }
}
pub(crate) unsafe fn tick(world: *const World, links: *const Links, visits: *const Visits) {
    let w = unsafe { &*world };
    let l = unsafe { &*links };
    let visits = unsafe { &*visits };
    if u32::from((w.environment.editor)()) != 0 {
        return;
    }
    let mut id = 0_usize;
    while id < (w.pool_size)() {
        let st = (w.get)(id as u16);
        if !st.is_null() {
            let state = (w.owner)(st);
            if !StationFacilities((w.facilities)(st)).test(StationFacility::Waypoint)
                && u32::from((w.in_use)(st)) != 0
            {
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
                    stale(st, w, l, visits);
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
                            status(state, cargo, GoodsState::AcceptedBigtick, false);
                        }
                    }
                }
                if !StationFacilities((w.facilities)(st)).test(StationFacility::Waypoint) {
                    unsafe {
                        openttd_rust_station_acceptance(st, world, 1);
                    }
                }
            }
            if phase % 250 == 0 {
                crate::animated_tile::acceptance(st);
            }
        }
        id += 1;
    }
}

#[repr(C)]
pub struct Loading {
    pub(crate) control: extern "C" fn(VehicleRef) -> LoadingControl,
    pub(crate) part: extern "C" fn(VehicleRef) -> LoadingPart,
    pub(crate) wait_observation: extern "C" fn(VehicleRef) -> LoadingWait,
    pub(crate) cargo_capacity: extern "C" fn(VehicleRef) -> CargoCapacity,
    pub(crate) cargo_type: extern "C" fn(VehicleRef) -> u8,
    pub(crate) unload_type: extern "C" fn(VehicleRef) -> u8,
    pub(crate) load_unload_ticks: extern "C" fn(VehicleRef) -> u16,
    pub(crate) cargo_multiplier: extern "C" fn(VehicleRef) -> u16,
    pub(crate) passenger_cargo: extern "C" fn(VehicleRef) -> bool,
    pub(crate) station_length_penalty: extern "C" fn() -> bool,
    pub(crate) current_company: extern "C" fn() -> u8,
    pub(crate) vehicle: *const crate::vehicle::VehicleServices,
    /// The canonical cargo-list services for in-place vehicle list operations.
    pub(crate) cargo: *const crate::cargo_storage::Services,
    pub(crate) set_cargo_unloading: extern "C" fn(VehicleRef, bool),
    pub(crate) set_loading_finished: extern "C" fn(VehicleRef, bool),
    pub(crate) set_stop: extern "C" fn(VehicleRef, bool),
    pub(crate) set_load_ticks: extern "C" fn(VehicleRef, u16),
    pub(crate) subtract_profit: extern "C" fn(VehicleRef, i64),
    pub(crate) vehicle_dirty: extern "C" fn(VehicleRef),
    pub(crate) indicator: extern "C" fn(VehicleRef) -> bool,
    pub(crate) show_indicator: extern "C" fn(VehicleRef, u8, u32),
    pub(crate) set_current_company: extern "C" fn(u8),
    pub(crate) with_next_stations: extern "C" fn(VehicleRef, *mut c_void, Scope),
    pub(crate) stage: extern "C" fn(*mut Station, VehicleRef, u8, *mut c_void, *mut NativePayment),
    pub(crate) return_cargo: extern "C" fn(*mut Station, VehicleRef, u32),
    pub(crate) unload_cargo:
        extern "C" fn(*mut Station, VehicleRef, u32, *mut NativePayment) -> u32,
    pub(crate) load_cargo: extern "C" fn(*mut Station, VehicleRef, u32, *mut c_void) -> u32,
    pub(crate) reserve_cargo: extern "C" fn(*mut Station, VehicleRef, u32, *mut c_void),
    pub(crate) has_cargo_for: extern "C" fn(*mut Station, u8, *mut c_void) -> bool,
    pub(crate) station_cargo_total: extern "C" fn(*mut Station, u8) -> u32,
    pub(crate) get_payment: extern "C" fn(VehicleRef) -> *mut NativePayment,
    pub(crate) create_payment: extern "C" fn(VehicleRef) -> *mut NativePayment,
    pub(crate) delete_payment: extern "C" fn(*mut NativePayment),
    pub(crate) flush_delivery: extern "C" fn(),
    pub(crate) station_dirty: extern "C" fn(*mut Station),
}

// The vehicle's cargo list is Rust-owned: its counts and reassignments run here
// on the owner the native cargo observation names, not through native wrappers.
impl Loading {
    fn list(&self, v: VehicleRef) -> *mut crate::cargo_storage::List {
        (self.cargo_capacity)(v).owner.cast_mut()
    }
    /// `v->cargo.StoredCount()`.
    pub(crate) fn stored_count(&self, v: VehicleRef) -> u32 {
        // SAFETY: The live vehicle's list; the count read ends before any callback.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_stored(self.list(v)) }
    }
    /// `v->cargo.UnloadCount()`.
    fn unload_count(&self, v: VehicleRef) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_unload(self.list(v)) }
    }
    /// `v->cargo.TotalCount()`.
    fn total_count(&self, v: VehicleRef) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_count(self.list(v)) }
    }
    /// `v->cargo.ActionCount(action)`.
    fn action_count(&self, v: VehicleRef, action: MoveToAction) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe {
            crate::cargo_storage::openttd_rust_cargo_list_action_count(self.list(v), action.0)
        }
    }
    fn deliver_count(&self, v: VehicleRef) -> u32 {
        self.action_count(v, MoveToAction::MTA_DELIVER)
    }
    fn transfer_count(&self, v: VehicleRef) -> u32 {
        self.action_count(v, MoveToAction::MTA_TRANSFER)
    }
    fn load_count(&self, v: VehicleRef) -> u32 {
        self.action_count(v, MoveToAction::MTA_LOAD)
    }
    /// `v->cargo.Reassign<from, to>(amount)`.
    fn reassign(&self, v: VehicleRef, from: MoveToAction, to: MoveToAction, amount: u32) {
        // SAFETY: The live vehicle's list and the immutable cargo services; no Rust
        // borrow of the list spans the reassignment.
        unsafe { crate::cargo_storage::reassign(self.list(v), *self.cargo, from, to, amount) };
    }
    fn deliver_to_transfer(&self, v: VehicleRef, amount: u32) {
        self.reassign(
            v,
            MoveToAction::MTA_DELIVER,
            MoveToAction::MTA_TRANSFER,
            amount,
        );
    }
    fn deliver_to_keep(&self, v: VehicleRef, amount: u32) {
        self.reassign(v, MoveToAction::MTA_DELIVER, MoveToAction::MTA_KEEP, amount);
    }
}
/// `v->orders != nullptr`: the vehicle's order list is assigned.
fn has_orders(v: VehicleRef) -> bool {
    let orders = crate::raw_field::read_field!(v.0.as_ptr(), orders);
    !crate::raw_field::read_field!(orders, orders).is_null()
}

fn parts(v: VehicleRef, mut action: impl FnMut(VehicleRef) -> bool) -> bool {
    let mut current = Some(v);
    while let Some(part) = current {
        if !action(part) {
            return false;
        }
        if part.vehicle_type() == VehicleType::TRAIN.0
            && crate::ground_vehicle::is_multiheaded(part.subtype())
            && !action(part.other_multiheaded_part().unwrap())
        {
            return false;
        }
        current = if part.has_articulated_part() {
            part.next()
        } else {
            None
        };
    }
    if v.vehicle_type() == VehicleType::AIRCRAFT.0
        && crate::aircraft::is_normal_aircraft(v.subtype())
    {
        return action(unsafe { v.next().unwrap_unchecked() });
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
fn may_load(st: LoadingStation, v: VehicleRef, w: &World) -> bool {
    if (w.station_owner)(st.base) != OWNER_NONE {
        return true;
    }
    let observed = (w.station_exclusivity)(st.base);
    observed.counter == 0 || observed.exclusive == v.owner()
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
        let reserve_part = if v.is_articulated_part() {
            false
        } else {
            let kind = v.vehicle_type();
            (kind != 0 || !crate::ground_vehicle::is_rear_dualheaded(v.subtype()))
                && (kind != 3 || crate::aircraft::is_normal_aircraft(v.subtype()))
                && (must_reserve || control.refit_cargo == (l.cargo_type)(v))
        };
        if reserve_part {
            parts(v, |part| {
                let observed = (l.cargo_capacity)(part);
                let cap = u32::from(part.cargo_cap());
                let remaining = observed.remaining();
                if cap > remaining && may_load(st, part, w) {
                    (l.reserve_cargo)(st.native, part, cap - remaining, next);
                }
                true
            });
        }
        if let Some(ref mut capleft) = capleft
            && v.cargo_cap() != 0
        {
            let observed = (l.cargo_capacity)(v);
            let cargo = usize::from(observed.cargo);
            capleft[cargo] = capleft[cargo]
                .wrapping_add(u32::from(v.cargo_cap()).wrapping_sub(observed.remaining()));
        }
        current = v.next();
    }
}
fn load_amount(
    v: VehicleRef,
    gradual: bool,
    l: &Loading,
    shared: &crate::services::Services,
) -> u32 {
    // SAFETY: the static base services outlive this synchronous call.
    let vehicle = unsafe { &*l.vehicle };
    let engine = (vehicle.engine)(v);
    let mut amount = u32::from(engine.load_amount());
    let mail = v.vehicle_type() == VehicleType::AIRCRAFT.0
        && !crate::aircraft::is_normal_aircraft(v.subtype());
    if mail {
        amount = amount.wrapping_add(3) / 4;
    }
    if gradual {
        let version = engine.grf_version(shared);
        let mut cb = crate::newgrf_callbacks::CALLBACK_FAILED;
        if version >= 8 {
            cb = (vehicle.property)(
                (vehicle.newgrf)(),
                v,
                crate::newgrf_properties::PROP_VEHICLE_LOAD_AMOUNT,
                i32::from(crate::newgrf_callbacks::CALLBACK_FAILED),
            ) as u16;
        } else if engine.callbacks() & crate::newgrf_callbacks::VEHICLE_LOAD_AMOUNT != 0 {
            cb = (vehicle.callback)(
                (vehicle.newgrf)(),
                v,
                crate::newgrf_callbacks::CBID_VEHICLE_LOAD_AMOUNT,
            );
        }
        if cb != crate::newgrf_callbacks::CALLBACK_FAILED {
            if engine.grf_version(shared) < 8 {
                cb &= 255;
            }
            if cb >= 256 {
                vehicle.unknown_callback_result(
                    engine,
                    crate::newgrf_callbacks::CBID_VEHICLE_LOAD_AMOUNT,
                    cb,
                );
            } else if cb != 0 {
                amount = u32::from(cb);
            }
        }
    }
    if engine.misc() & crate::engine_type::NO_DEFAULT_CARGO_MULTIPLIER != 0 && !mail {
        amount = amount
            .wrapping_mul(u32::from((l.cargo_multiplier)(v)))
            .wrapping_add(255)
            / 256;
    }
    amount.max(1)
}
// Call-local native costs live in LoadUnloadStation's stack frame. They never
// alias another loading/reentrant command frame; no owner reference spans calls.
struct RefitContext {
    services: *const crate::vehicle_transactions::refit::Services,
    out: *mut crate::services::CommandCost,
    temporary: *mut crate::services::CommandCost,
}
impl RefitContext {
    fn cost(&self) -> crate::services::Cost<'_> {
        // SAFETY: Native loading entry supplies immutable services and stack slots.
        crate::services::Cost(self.out, unsafe { &*self.services }.common.base.cost)
    }
    fn run(
        &self,
        v: VehicleRef,
        cargo: u8,
        flags: u16,
    ) -> crate::vehicle_transactions::refit::Capacity {
        // SAFETY: Loading entry owns native stack costs for this synchronous call;
        // immutable service-table borrows contain no world/vehicle owner borrow.
        let services = unsafe { &*self.services };
        let base = services.common.base;
        let id = unsafe { (base.groups.vehicle_id)(v.shell().cast()) };
        crate::command::nested(
            base.commands,
            base.shared,
            self.cost(),
            0,
            flags,
            |flags| unsafe {
                crate::vehicle_transactions::refit::refit(
                    services,
                    self.out,
                    self.temporary,
                    flags,
                    id,
                    cargo,
                    0xff,
                    true,
                    false,
                    1,
                )
            },
        )
    }
}
fn refit_vehicle(
    v: VehicleRef,
    capleft: &mut [u32; 64],
    st: LoadingStation,
    next: *mut c_void,
    mut new_cargo: u8,
    w: &World,
    l: &Loading,
    refit: &RefitContext,
) {
    let start = v.first_engine_part();
    if !parts(start, |part| l.stored_count(part) == 0) {
        return;
    }
    let old_company = (l.current_company)();
    (l.set_current_company)(v.owner());
    let mut mask = (unsafe { &*l.vehicle }.engine)(v).refit_mask();
    parts(start, |part| {
        let observed = (l.cargo_capacity)(part);
        let cargo = usize::from(observed.cargo);
        let reserved =
            unsafe { crate::cargo_storage::openttd_rust_cargo_list_reserved(observed.owner) };
        capleft[cargo] =
            capleft[cargo].wrapping_sub(u32::from(part.cargo_cap()).wrapping_sub(reserved));
        mask |= (unsafe { &*l.vehicle }.engine)(part).refit_mask();
        true
    });
    let auto = new_cargo == 253;
    if auto {
        new_cargo = (l.cargo_type)(start);
        for cargo in 0..64_u8 {
            if mask & (1_u64 << cargo) == 0 || !(l.has_cargo_for)(st.native, cargo, next) {
                continue;
            }
            let capacity = refit.run(start, cargo, DoCommandFlag::QUERY_COST).total;
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
        parts(start, |part| {
            (l.return_cargo)(st.native, part, u32::MAX);
            true
        });
        refit.run(start, new_cargo, DoCommandFlag::EXECUTE);
        let cost = refit.cost().money();
        let succeeded = refit.cost().ok();
        if succeeded {
            (l.subtract_profit)(v.first(), cost.wrapping_shl(8));
        }
    }
    let do_reserve = auto || v.first().current_order().is_full_load();
    parts(start, |part| {
        let observed = (l.cargo_capacity)(part);
        let remaining = u32::from(part.cargo_cap()).wrapping_sub(observed.remaining());
        if do_reserve {
            (l.reserve_cargo)(st.native, part, remaining, next);
        }
        // Reserve changes RemainingCount; observe again at the original point.
        let observed = (l.cargo_capacity)(part);
        let cargo = usize::from(observed.cargo);
        capleft[cargo] = capleft[cargo]
            .wrapping_add(u32::from(part.cargo_cap()).wrapping_sub(observed.remaining()));
        true
    });
    (l.set_current_company)(old_company);
}
/// `PrepareUnload`.
pub(crate) fn prepare_unload(front: VehicleRef, w: &World, l: &Loading) {
    let st = (w.station)(front.last_station());
    unsafe {
        openttd_rust_station_queue_push(st.state, front.shell().cast());
    }
    (l.set_loading_finished)(front, false);
    (l.set_load_ticks)(front, 1);
    let payment = (l.create_payment)(front);
    let mut body = |next| {
        if !has_orders(front) || (l.unload_type)(front) != OrderUnloadType::NoUnload.0 {
            let mut current = Some(front);
            while let Some(v) = current {
                if u32::from(v.cargo_cap()) > 0 && l.total_count(v) > 0 {
                    (l.stage)(st.native, v, (l.unload_type)(front), next, payment);
                    if l.unload_count(v) > 0 {
                        (l.set_cargo_unloading)(v, true);
                    }
                }
                current = v.next();
            }
        }
    };
    let (context, callback) = scope_context(&mut body);
    (l.with_next_stations)(front, context, callback);
}
fn update_ticks(
    front: VehicleRef,
    st: LoadingStation,
    kind: u8,
    mut ticks: i32,
    w: &World,
    l: &Loading,
) {
    if kind == 0 && (l.station_length_penalty)() {
        let cached_length = u32::from(crate::raw_field::read_field!(
            front.ground_cache(),
            cached_total_length
        ));
        let tile = front.tile();
        // SAFETY: The loading entry keeps the immutable shared services live.
        let world = crate::map_func::World::new(unsafe { &*w.shared });
        let length = crate::follow_track::platform_length_full(&world, st.base, tile);
        let overhang = cached_length.wrapping_sub(length.wrapping_mul(16)) as i32;
        if overhang > 0 {
            ticks = ticks.wrapping_shl(1);
            ticks = ticks.wrapping_add(overhang.wrapping_mul(ticks) / 8);
        }
    }
    // Always wait at least one tick; zero would wait indefinitely.
    (l.set_load_ticks)(front, ticks.max(1) as u16);
}
// Preserve complete loading control at the source observation points.
#[allow(clippy::too_many_lines)]
unsafe fn load_vehicle(
    front: VehicleRef,
    w: &World,
    l: &Loading,
    refit: &RefitContext,
    visits: &Visits,
) {
    // SAFETY: The immutable shared vehicle table outlives this entry.
    let vehicle = unsafe { &*l.vehicle };
    let st = (w.station)(front.last_station());
    let mut body = |next| {
        let control = (l.control)(front);
        let auto = control.refit && control.refit_cargo == CARGO_AUTO_REFIT;
        let mut capleft = [0_u32; 64];
        // Keep the original conditional-operator precedence: payment controls
        // reservation only when improved loading and autorefit are both active.
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
        // The next round of loading/unloading has not yet arrived.
        if control.ticks != 0 {
            return;
        }
        if front.vehicle_type() == VehicleType::TRAIN.0 && {
            let tile = front.tile();
            let shared = unsafe { &*w.shared };
            (shared.tile_type)(tile) != crate::vehicle::MP_STATION
                || (shared.station_index)(tile) != (w.graphs.station_id)(st.base)
        } {
            // The train reversed in the station. Take the easy way out and
            // let it leave as it always did.
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
        front.write_speed(0);
        let payment = (l.get_payment)(front);
        // Articulated part being loaded, excluding parts without capacity.
        let mut artic_part = 0_u32;
        let mut current = Some(front);
        while let Some(v) = current {
            if v == front || !v.previous().unwrap().has_articulated_part() {
                artic_part = 0;
            }
            if v.cargo_cap() == 0 {
                current = v.next();
                continue;
            }
            let part = (l.part)(v);
            artic_part = artic_part.wrapping_add(1);
            let mut cargo = part.cargo;
            let mut ge = unsafe { cp(state, cargo) };
            if part.unloading && control.unload_type != OrderUnloadType::NoUnload.0 {
                let count = l.unload_count(v);
                let mut amount = if control.gradual {
                    if count > 0 {
                        crate::witness::hit(crate::witness::STATION_GRADUAL_UNLOAD);
                    }
                    count.min(load_amount(v, control.gradual, l, unsafe { &*w.shared }))
                } else {
                    count
                };
                if !GoodsStates(unsafe { (*ge).status }).test(GoodsState::Acceptance)
                    && l.deliver_count(v) > 0
                {
                    crate::witness::hit(crate::witness::STATION_REJECTED_DELIVERY);
                    // The station no longer accepts our cargo.
                    let unload_type = control.unload_type;
                    if unload_type == OrderUnloadType::Transfer.0
                        || unload_type == OrderUnloadType::Unload.0
                    {
                        // Transfer instead of delivering.
                        l.deliver_to_transfer(v, l.deliver_count(v));
                    } else {
                        let observed = (l.cargo_capacity)(v);
                        let remaining = unsafe {
                            crate::cargo_storage::openttd_rust_cargo_list_remaining(observed.owner)
                        }
                        .wrapping_add(l.deliver_count(v));
                        let capacity = u32::from(v.cargo_cap());
                        if capacity < remaining {
                            // Return reserved cargo so the vehicle is not overloaded.
                            (l.return_cargo)(st.native, v, remaining - capacity);
                        }
                        // Keep instead of delivering. Pretend something was unloaded
                        // so loading gets another cycle instead of finishing now.
                        l.deliver_to_keep(v, l.deliver_count(v));
                        unloaded = true;
                    }
                }
                if l.transfer_count(v) > 0 {
                    // Transfers dirty the station; final deliveries alone do not.
                    dirty_station = true;
                    // Give transferred cargo a rating and fake the first pickup
                    // so it does not quickly decay after its initial drop.
                    if !GoodsStates(unsafe { (*ge).status }).test(GoodsState::Rating) {
                        unsafe {
                            (&raw mut (*ge).time_since_pickup).write(0);
                            status(state, cargo, GoodsState::Rating, true);
                        }
                    }
                }
                crate::witness::hit(crate::witness::STATION_UNLOAD_CARGO);
                amount = (l.unload_cargo)(st.native, v, amount, payment);
                let remaining = l.unload_count(v) > 0;
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
                current = v.next();
                continue;
            }
            // Do not pick up goods with no-load set or loading stopped.
            if control.load_type == OrderLoadType::NoLoad.0 || control.stop {
                current = v.next();
                continue;
            }
            // Try refitting at the first cargo-carrying part when the consist
            // is empty. Reacquire the goods entry after the refit.
            let refitted = control.refit && artic_part == 1;
            if refitted {
                refit_vehicle(v, &mut capleft, st, next, control.refit_cargo, w, l, refit);
            }
            if refitted {
                cargo = (l.cargo_type)(v);
                ge = unsafe { cp(state, cargo) };
            }
            // The following link can carry the full capacity while loading here.
            v.write_refit_cap(v.cargo_cap());
            let speed = match VehicleType(front.vehicle_type()) {
                VehicleType::TRAIN | VehicleType::SHIP => i32::from(front.max_speed()),
                VehicleType::ROAD => i32::from(front.max_speed()) / 2,
                VehicleType::AIRCRAFT => {
                    crate::aircraft::speed_old_units(u32::from(front.max_speed())) as i32
                }
                _ => unreachable!("original NOT_REACHED vehicle type"),
            };
            unsafe {
                // Zero means no vehicle has ever visited the station.
                (&raw mut (*ge).last_speed).write(speed.clamp(0, 255) as u8);
            }
            let resolver = unsafe { &*(vehicle.newgrf)() };
            let age = crate::timer_game_common::calendar_year()
                .0
                .wrapping_sub(unsafe { (resolver.build_year)(front.0.as_ptr()) });
            unsafe {
                (&raw mut (*ge).last_age).write(age.clamp(0, 255) as u8);
            }
            // Capacity available to load more cargo.
            let mut cap = u32::from(v.cargo_cap()).wrapping_sub(l.stored_count(v));
            if cap > 0 {
                unsafe {
                    (&raw mut (*ge).time_since_pickup).write(0);
                }
                if (l.load_count(v) > 0 || (w.available)(st.base, cargo) > 0) && may_load(st, v, w)
                {
                    if l.stored_count(v) == 0 {
                        unsafe {
                            crate::newgrf_vehicle::trigger(
                                vehicle,
                                v,
                                crate::newgrf_vehicle::Trigger::NewCargo,
                            );
                        };
                    }
                    if control.gradual {
                        cap = cap.min(load_amount(v, control.gradual, l, unsafe { &*w.shared }));
                    }
                    let loaded = (l.load_cargo)(st.native, v, cap, next);
                    if l.load_count(v) > 0 {
                        // Keep loading while reservations remain.
                        reservation_left |= 1 << cargo;
                    }
                    // Record whether the maximum possible amount was loaded.
                    if loaded == cap {
                        full_load |= 1 << cargo;
                    } else {
                        full_load &= !(1 << cargo);
                    }
                    // TODO: With gradual loading, unload the complete consist
                    // before loading any part, as TTDPatch does. That would let
                    // the Empty trigger run when the entire chain is empty and
                    // make this emptied assignment unnecessary.
                    if loaded > 0 {
                        emptied = false;
                        loaded_any = true;
                        unsafe {
                            (&raw mut (*state).fields.time_since_load).write(0);
                            (&raw mut (*state).fields.last_vehicle_type).write(v.vehicle_type());
                        }
                        if (l.station_cargo_total)(st.native, cargo) == 0 {
                            crate::animated_tile::cargo_event(
                                st.base,
                                cargo,
                                crate::station_type::SAT_CARGO_TAKEN,
                                crate::station_type::AAT_CARGO_TAKEN,
                                crate::station_type::SRT_CARGO_TAKEN,
                            );
                        }
                        ticks = ticks.wrapping_add(loaded as i32);
                        dirty_vehicle = true;
                        dirty_station = true;
                    }
                }
            }
            if l.stored_count(v) >= u32::from(v.cargo_cap()) {
                cargo_full |= 1 << cargo;
            } else {
                cargo_not_full |= 1 << cargo;
            }
            current = v.next();
        }
        if loaded_any || unloaded {
            if front.vehicle_type() == VehicleType::TRAIN.0 {
                crate::animated_tile::rail_event(
                    st.base,
                    (front).tile(),
                    crate::station_type::SAT_VEHICLE_LOADS,
                    crate::station_type::SRT_VEHICLE_LOADS,
                );
            } else if front.vehicle_type() == VehicleType::ROAD.0 {
                crate::animated_tile::road_event(
                    st.base,
                    (front).tile(),
                    crate::station_type::SAT_VEHICLE_LOADS,
                    crate::station_type::SRT_VEHICLE_LOADS,
                );
            }
        }
        // Only count as completely emptied if this cycle unloaded something.
        emptied &= unloaded;
        if !unloaded {
            (l.delete_payment)(payment);
        }
        (l.set_stop)(front, false);
        if loaded_any || unloaded {
            if control.gradual {
                // Per-vehicle slice times are the values found in TTDPatch.
                ticks = [40, 20, 10, 20][front.vehicle_type() as usize];
            }
            if !unloaded && full_load == 0 && reservation_left == 0 && !control.full_load && {
                let observed = (l.wait_observation)(front);
                let wait = i32::from(observed.wait)
                    .wrapping_sub(observed.lateness)
                    .max(0);
                front.order_time() >= wait
            } {
                (l.set_stop)(front, true);
            }
            update_ticks(front, st, front.vehicle_type(), ticks, w, l);
        } else {
            update_ticks(front, st, front.vehicle_type(), 20, w, l);
            let mut finished = true;
            if control.full_load {
                if control.load_type == OrderLoadType::FullLoadAny.0 {
                    // A passenger aircraft continues loading until its passenger
                    // capacity is full, irrespective of how much mail is loaded.
                    if (front.vehicle_type() == VehicleType::AIRCRAFT.0
                        && (l.passenger_cargo)(front)
                        && u32::from(front.cargo_cap()) > l.stored_count(front))
                        || (cargo_not_full != 0 && (cargo_full & !cargo_not_full) == 0)
                    {
                        finished = false;
                    }
                } else if cargo_not_full != 0 {
                    finished = false;
                }
                // Refresh full-load next hops so distribution can send cargo
                // along links not yet visited or whose statistics expired.
                if !finished {
                    refresh_links(front, true, true, visits);
                }
            }
            (l.set_loading_finished)(front, finished);
        }
        if (l.indicator)(front) {
            let mut colour = 0;
            let percent = crate::vehicle::visits::percent_filled(front, Some(&mut colour), visits);
            (l.show_indicator)(front, percent, colour);
        }
        if emptied {
            // The empty trigger can change weight, power and tractive effort.
            dirty_vehicle = true;
            unsafe {
                crate::newgrf_vehicle::trigger(
                    vehicle,
                    front,
                    crate::newgrf_vehicle::Trigger::Empty,
                );
            };
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
// Keep the queue-empty check visible to Rust callers before loading-table setup.
// The native entry below uses this same named owner body.
#[inline]
pub(crate) unsafe fn load_station(
    state: *mut Service,
    world: *const World,
    loading: *const Loading,
    visits: *const Visits,
    out: *mut crate::services::CommandCost,
    temporary: *mut crate::services::CommandCost,
) {
    // C++ supplies the queue owner directly: no callback precedes the empty
    // queue check, and the queue station is not the vehicles' loading station.
    let node = unsafe { openttd_rust_station_queue_first(state) };
    if node.is_null() {
        return;
    }
    let w = unsafe { &*world };
    let l = unsafe { &*loading };
    let visits = unsafe { &*visits };
    let refit = RefitContext {
        services: visits.refit,
        out,
        temporary,
    };
    // SAFETY: The first queue node and immutable tables are live for the same
    // synchronous load. The helper retains the original live-next traversal.
    unsafe { load_station_queue(state, node, w, l, visits, &refit) };
}

// Out of line so the empty-queue check above stays inlined at the scheduler.
#[inline(never)]
unsafe fn load_station_queue(
    state: *mut Service,
    mut node: *mut QueueEntry,
    w: &World,
    l: &Loading,
    visits: &Visits,
    refit: &RefitContext,
) {
    let mut last = None;
    while !node.is_null() {
        let v =
            unsafe { VehicleRef::from_shell(openttd_rust_station_queue_value(node), &*l.vehicle) };
        if v.status() & (VS_STOPPED | VS_CRASHED) == 0 {
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
        let v =
            unsafe { VehicleRef::from_shell(openttd_rust_station_queue_value(node), &*l.vehicle) };
        if v.status() & (VS_STOPPED | VS_CRASHED) == 0 {
            unsafe {
                load_vehicle(v, w, l, refit, visits);
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
pub unsafe extern "C" fn openttd_rust_station_load(
    state: *mut Service,
    world: *const World,
    loading: *const Loading,
    visits: *const Visits,
    out: *mut crate::services::CommandCost,
    temporary: *mut crate::services::CommandCost,
) {
    // SAFETY: Forward the native entry's live queue/table/cost contract unchanged.
    unsafe { load_station(state, world, loading, visits, out, temporary) };
}

pub(crate) unsafe fn modify_rating(state: *mut Service, amount: i32) {
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

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDLoadingStation" ["probe"] => LoadingStation {
        native,
        base,
        state,
    };
    "OpenTTDRatingCounts" ["probe"] => RatingCounts {
        available,
        destinations,
    };
    "OpenTTDStationExclusivity" ["probe"] => StationExclusivity {
        counter,
        exclusive,
    };
    "OpenTTDCargoCapacity" ["probe"] => CargoCapacity {
        owner,
        cargo,
    };
    "OpenTTDLoadingWait" ["probe"] => LoadingWait {
        lateness,
        wait,
    };
    "OpenTTDOrderDestination" ["probe"] => OrderDestination {
        destination,
        kind,
    };
    "OpenTTDTruncatedSource" ["probe"] => TruncatedSource {
        amount,
        source,
        present,
    };
    "OpenTTDLoadingControl" ["probe"] => LoadingControl {
        ticks,
        unload_type,
        load_type,
        refit_cargo,
        refit,
        full_load,
        stop,
        gradual,
        improved,
    };
    "OpenTTDLoadingPart" ["probe"] => LoadingPart {
        cargo,
        unloading,
    };
    "OpenTTDStationCargo" ["station_service"] => CargoFields {
        max_waiting_cargo,
        status,
        time_since_pickup,
        rating,
        last_speed,
        last_age,
        amount_fract,
    };
    "OpenTTDStationFields" ["station_service"] => Fields {
        always_accepted,
        delete_ctr,
        time_since_load,
        time_since_unload,
        last_vehicle_type,
    };
    "OpenTTDStationServices" ["probe"] => World {
        graphs,
        station,
        station_owner,
        station_exclusivity,
        rating_counts,
        get,
        owner,
        pool_size,
        environment,
        facilities,
        in_use,
        is_station,
        local_owner,
        town_statue,
        select_goods,
        rating_cheat,
        available,
        cargo_valid,
        rating_callback_enabled,
        passenger_cargo,
        packet_available,
        rating_redraw,
        acceptance_news,
        acceptance_redraw,
        delete_station,

        append_packet,
        station_list_invalidation,

        waiting_redraw,
        cargo_registry,
        with_truncation,
        truncate_next,
        shared,
    };
    "OpenTTDStationEdge" ["station_service"] => Edge {
        destination,
        distance,
    };
    "OpenTTDStationLinks" ["probe"] => Links {
        graphs,
        automatic_distribution,
        edge,
        delete_flows,
        restrict_flows,
        order_list,
        order_pool_size,
        order_count,
        order_destination,
        order_vehicle,
        next_shared,
        stopped_in_depot,
        last_service,
        cargo_type,
        goods_entry, station_resolver, vehicle_resolver,
    };
    "OpenTTDStationLoading" ["probe"] => Loading {
        control,
        part,
        wait_observation,
        cargo_capacity,
        cargo_type,
        unload_type,
        load_unload_ticks,
        cargo_multiplier,
        passenger_cargo,
        station_length_penalty,
        current_company,
        vehicle,
        set_cargo_unloading,
        set_loading_finished,
        set_stop,
        set_load_ticks,
        subtract_profit,
        vehicle_dirty,
        indicator,
        show_indicator,
        set_current_company,
        with_next_stations,
        stage,
        return_cargo,
        unload_cargo,
        load_cargo,
        reserve_cargo,
        has_cargo_for,
        station_cargo_total,
        get_payment,
        create_payment,
        delete_payment,
        flush_delivery,
        station_dirty,
    };
}
