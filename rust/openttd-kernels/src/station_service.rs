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
    clippy::cast_sign_loss
)]
use crate::bitmath_func::{clr_bit64, gb, has_bit, has_bit64, set_bit64};
use crate::cargo_payment::NativePayment;
use crate::cargo_type::{CargoArray, CargoType, CargoTypes, NUM_CARGO};
use crate::cargopacket::MoveToAction;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, OWNER_END, Owner};
use crate::linkgraph::live;
use crate::math::{ceil_div, clamp_to_u8, clamp_to_u16};
use crate::order_type::{OrderLoadType, OrderType, OrderUnloadType};
use crate::orders::NativeOrderList;
use crate::station::{BaseStation, Station};
use crate::station_base::goods_entry::{
    self, State as GoodsState, States as GoodsStates, convert_state,
};
use crate::station_base::{
    INITIAL_STATION_RATING, MAX_STATION_RATING, has_vehicle_ever_tried_loading,
};
use crate::station_type::{StationFacilities, StationFacility, StationID};
use crate::strings_type::StringID;
use crate::tile_type::TILE_SIZE;
use crate::timer_game_common::INVALID_DATE;
use crate::timer_game_tick::{
    STATION_ACCEPTANCE_TICKS, STATION_LINKGRAPH_TICKS, STATION_RATING_TICKS,
};
use crate::vehicle::VehicleRef;
use crate::vehicle::visits::{Services as Visits, refresh_links};
use crate::vehicle_base::{OpenTTDVehicleShell, VS_CRASHED, VS_STOPPED};
use crate::vehicle_type::VehicleType;
use std::ffi::c_void;
use std::ptr::NonNull;
/// `GoodsEntry::ConvertState`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_goods_convert_state(status: u8) -> u8 {
    convert_state(GoodsStates(status))
}

/// The Rust-owned scalar fields of a `GoodsEntry`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CargoFields {
    /// Max cargo from this station waiting at any station.
    pub max_waiting_cargo: u32,
    /// Status of this cargo, see `GoodsEntry::State`.
    pub status: u8,
    /// Number of rating-intervals (up to 255) since the last vehicle tried to
    /// load this cargo. The unit used is `STATION_RATING_TICKS`. This does not
    /// imply there was any cargo to load.
    pub time_since_pickup: u8,
    /// Station rating for this cargo.
    pub rating: u8,
    /// Maximum speed (up to 255) of the last vehicle that tried to load this
    /// cargo. This does not imply there was any cargo to load. The unit used is
    /// a special vehicle-specific speed unit for station ratings.
    /// - Trains: km-ish/h
    /// - RV: km-ish/h
    /// - Ships: 0.5 * km-ish/h
    /// - Aircraft: 8 * mph
    pub last_speed: u8,
    /// Age in years (up to 255) of the last vehicle that tried to load this
    /// cargo. This does not imply there was any cargo to load.
    pub last_age: u8,
    /// Fractional part of the amount in the cargo list.
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
    /// Bitmask of always accepted cargo types (by houses, HQs, industry tiles when industry doesn't accept cargo)
    pub always_accepted: u64,
    pub delete_ctr: u8,
    pub time_since_load: u8,
    pub time_since_unload: u8,
    pub last_vehicle_type: u8,
}
pub struct Service {
    fields: Fields,
    cargo: [CargoFields; NUM_CARGO as usize],
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
/// `HasBit(st->always_accepted, cargo)`.
/// SAFETY: Live serialized station owner; no callback occurs during these accesses.
pub(crate) unsafe fn always_accepts(state: *const Service, cargo: CargoType) -> bool {
    has_bit64(unsafe { (*state).fields.always_accepted }, cargo.0)
}
/// `DeliverGoods`'s `status.Set({EverAccepted, CurrentMonth, AcceptedBigtick})`.
/// SAFETY: Live serialized station owner and valid cargo slot.
pub(crate) unsafe fn record_delivery(state: *mut Service, cargo: CargoType) {
    let ge = unsafe { cp(state, cargo) };
    let mut status = GoodsStates(unsafe { (*ge).status });
    status.set(GoodsState::EverAccepted);
    status.set(GoodsState::CurrentMonth);
    status.set(GoodsState::AcceptedBigtick);
    unsafe { (&raw mut (*ge).status).write(status.0) };
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
impl Service {
    /// The Rust-owned part of `Station::Station`, with the members' defaults.
    fn new() -> Self {
        Self {
            fields: Fields {
                always_accepted: 0,
                delete_ctr: 0,
                time_since_load: 255,
                time_since_unload: 255,
                last_vehicle_type: VehicleType::VEH_INVALID.0,
            },
            cargo: [CargoFields::default(); NUM_CARGO as usize],
            queue: Vec::new(),
            catchment: crate::station::Catchment::default(),
        }
    }
}
/// Allocate the Rust-owned part of a new station.
fn create() -> *mut Service {
    Box::into_raw(Box::new(Service::new()))
}
/// Release the Rust-owned part of a destroyed station.
/// # Safety
/// `state` came from `create` and is not used afterwards.
unsafe fn destroy(state: *mut Service) {
    unsafe {
        drop(Box::from_raw(state));
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_service_new() -> *mut Service {
    create()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_service_destroy(state: *mut Service) {
    unsafe { destroy(state) };
}
/// The station's Rust-owned scalar fields, for immediate field-sized access only.
/// # Safety
/// `state` is a live owner.
pub(crate) unsafe fn fields(state: *mut Service) -> *mut Fields {
    unsafe { &raw mut (*state).fields }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_service_fields(state: *mut Service) -> *mut Fields {
    unsafe { fields(state) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_service_cargo(
    state: *mut Service,
    cargo: u8,
) -> *mut CargoFields {
    unsafe { cp(state, CargoType(cargo)) }
}
/// `st->goods[cargo]`'s Rust-owned fields, for immediate field-sized access only.
/// # Safety
/// `state` is a live owner and `cargo` is below `NUM_CARGO`.
pub(crate) unsafe fn goods(state: *mut Service, cargo: CargoType) -> *mut CargoFields {
    unsafe { cp(state, cargo) }
}
pub struct QueueEntry {
    vehicle: *mut OpenTTDVehicleShell,
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
/// `st->loading_vehicles.size()`.
/// SAFETY: `state` is a live owner.
unsafe fn loading_vehicles_size(state: *const Service) -> usize {
    unsafe { (*state).queue.len() }
}
/// `st->loading_vehicles.push_back(vehicle)`.
/// SAFETY: `state` is a live owner and no queue node is borrowed.
unsafe fn loading_vehicles_push_back(state: *mut Service, vehicle: *mut OpenTTDVehicleShell) {
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
/// `st->loading_vehicles.remove(vehicle)`.
/// SAFETY: `state` is a live owner; no removed node is used afterwards.
pub(crate) unsafe fn loading_vehicles_remove(
    state: *mut Service,
    vehicle: *mut OpenTTDVehicleShell,
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
/// `st->loading_vehicles.clear()`.
/// SAFETY: `state` is a live owner; no node is used afterwards.
unsafe fn loading_vehicles_clear(state: *mut Service) {
    for entry in unsafe { (*state).queue.drain(..) } {
        unsafe {
            drop(Box::from_raw(entry));
        }
    }
}
/// `st->loading_vehicles.begin()`; null at the end.
/// SAFETY: `state` is a live owner.
pub(crate) unsafe fn loading_vehicles_begin(state: *const Service) -> *mut QueueEntry {
    unsafe {
        (*state)
            .queue
            .first()
            .copied()
            .unwrap_or(std::ptr::null_mut())
    }
}
/// `++it` over `st->loading_vehicles`; null at the end.
/// SAFETY: `entry` is a live node.
pub(crate) unsafe fn loading_vehicles_next(entry: *const QueueEntry) -> *mut QueueEntry {
    unsafe { (&raw const (*entry).next).read() }
}
/// `*it` over `st->loading_vehicles`: the vehicle shell.
/// SAFETY: `entry` is a live node.
pub(crate) unsafe fn loading_vehicles_value(entry: *const QueueEntry) -> *mut OpenTTDVehicleShell {
    unsafe { (&raw const (*entry).vehicle).read() }
}
/// `st->loading_vehicles.erase(it)`, returning the successor.
/// SAFETY: `entry` is a live node of this owner.
unsafe fn loading_vehicles_erase(state: *mut Service, entry: *mut QueueEntry) -> *mut QueueEntry {
    let successor = unsafe { (&raw const (*entry).next).read() };
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_size(state: *const Service) -> usize {
    unsafe { loading_vehicles_size(state) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_push(
    state: *mut Service,
    vehicle: *mut c_void,
) {
    unsafe { loading_vehicles_push_back(state, vehicle.cast()) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_remove(
    state: *mut Service,
    vehicle: *mut c_void,
) {
    unsafe { loading_vehicles_remove(state, vehicle.cast()) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_clear(state: *mut Service) {
    unsafe { loading_vehicles_clear(state) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_first(
    state: *const Service,
) -> *mut QueueEntry {
    unsafe { loading_vehicles_begin(state) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_next(
    _state: *const Service,
    entry: *const QueueEntry,
) -> *mut QueueEntry {
    unsafe { loading_vehicles_next(entry) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_value(entry: *const QueueEntry) -> *mut c_void {
    unsafe { loading_vehicles_value(entry) }.cast()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_queue_erase_node(
    state: *mut Service,
    entry: *mut QueueEntry,
) -> *mut QueueEntry {
    unsafe { loading_vehicles_erase(state, entry) }
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
    /// `GetDepotActionType().Test(OrderDepotActionFlag::NearestDepot)`.
    pub(crate) nearest_depot: bool,
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
    pub(crate) is_station: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) town_statue: extern "C" fn(*mut BaseStation) -> bool,
    pub(crate) rating_cheat: extern "C" fn() -> bool,
    pub(crate) available: extern "C" fn(*mut BaseStation, u8) -> u32,
    pub(crate) cargo_valid: extern "C" fn(u8) -> bool,
    pub(crate) rating_callback_enabled: extern "C" fn(u8) -> bool,
    pub(crate) passenger_cargo: extern "C" fn(u8) -> bool,
    pub(crate) packet_available: extern "C" fn() -> bool,
    pub(crate) rating_redraw: extern "C" fn(*mut BaseStation, bool),
    pub(crate) acceptance_news: extern "C" fn(*mut BaseStation, u64, bool),
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
    /// `BaseStation::IsInUse`: any facility other than a waypoint.
    pub(crate) fn in_use(&self, st: *mut BaseStation) -> bool {
        crate::base_station_base::is_in_use(StationFacilities((self.facilities)(st)))
    }
    /// `st->owner == _local_company`.
    fn local_owner(&self, st: *mut BaseStation) -> bool {
        // SAFETY: The shared, station command, command and company tables are immutable
        // for the process lifetime.
        let local = unsafe {
            let commands = &*(*((*self.shared).station_commands)()).command;
            ((*(commands.company)()).local_company)()
        };
        (self.station_owner)(st) == local
    }
    /// `_settings_game.order.selectgoods`, from the settings catalog.
    fn select_goods(&self) -> bool {
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        unsafe { (*self.shared).settings().select_goods.read() }
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
/// `&st->goods[cargo]`.
unsafe fn cp(state: *mut Service, cargo: CargoType) -> *mut CargoFields {
    unsafe {
        (&raw mut (*state).cargo)
            .cast::<CargoFields>()
            .add(usize::from(cargo.0))
    }
}
/// `ge->status`.
unsafe fn goods_status(ge: *const CargoFields) -> GoodsStates {
    GoodsStates(unsafe { (&raw const (*ge).status).read() })
}
/// `ge->status = status`.
unsafe fn set_goods_status(ge: *mut CargoFields, status: GoodsStates) {
    unsafe { (&raw mut (*ge).status).write(status.0) };
}
/// `GoodsEntry::HasRating()` of a live station owner.
pub(crate) fn has_rating(state: *mut Service, cargo: CargoType) -> bool {
    // SAFETY: scalar read of the live serialized owner; no borrow survives.
    goods_entry::has_rating(unsafe { goods_status(cp(state, cargo)) })
}
/// `Station::GetNewGRFVariable`'s cargo variables 0x61, 0x62 and 0x64 to 0x69 for one goods
/// entry. `legacy` selects the deprecated 0x8C to 0xEC forms, which read the fields without
/// the `HasVehicleEverTriedLoading` and `HasRating` tests. Container counts, source and periods
/// are queried directly by the resolver's cargo owner. No station borrow survives a service call.
pub(crate) unsafe fn newgrf_cargo_variable(
    state: *mut Service,
    cargo: CargoType,
    variable: u8,
    legacy: bool,
) -> u32 {
    let ge = unsafe { cp(state, cargo) };
    unsafe {
        match variable {
            0x61 => {
                if legacy || has_vehicle_ever_tried_loading((*ge).last_speed) {
                    u32::from((*ge).time_since_pickup)
                } else {
                    0
                }
            }
            0x62 => {
                if legacy || has_rating(state, cargo) {
                    u32::from((*ge).rating)
                } else {
                    0xFFFF_FFFF
                }
            }
            0x64 => {
                if has_vehicle_ever_tried_loading((*ge).last_speed) {
                    u32::from((*ge).last_speed) | (u32::from((*ge).last_age) << 8)
                } else {
                    0xFF00
                }
            }
            0x65 => {
                if goods_status(ge).test(GoodsState::Acceptance) {
                    1 << 3
                } else {
                    0
                }
            }
            0x69 => u32::from(convert_state(goods_status(ge))),
            0x66 => u32::from((*ge).last_speed),
            0x67 => u32::from((*ge).last_age),
            _ => unreachable!(),
        }
    }
}
/// `GetAcceptanceMask`: get a mask of the cargo types that the station accepts.
pub(crate) unsafe fn acceptance_mask(state: *mut Service) -> CargoTypes {
    let mut mask = 0;
    for cargo in 0..NUM_CARGO {
        if unsafe { goods_status(cp(state, CargoType(cargo))) }.test(GoodsState::Acceptance) {
            set_bit64(&mut mask, cargo);
        }
    }
    CargoTypes(mask)
}
/// Native facade for the canonical `GetAcceptanceMask` reducer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_acceptance_mask(state: *mut Service) -> u64 {
    unsafe { acceptance_mask(state) }.0
}
/// `TruncateCargo`: truncate the cargo by a specific amount.
unsafe fn truncate_cargo(st: *mut BaseStation, cargo: CargoType, amount: u32, w: &World) {
    /* If truncating also punish the source stations' ratings to
     * decrease the flow of incoming cargo. */
    let mut body = |map| {
        loop {
            let observed = (w.truncate_next)(map);
            if !observed.present {
                break;
            }
            let source_station = (w.get)(observed.source);
            if source_station.is_null() {
                continue;
            }
            let source_ge = unsafe { cp((w.owner)(source_station), cargo) };
            unsafe {
                (&raw mut (*source_ge).max_waiting_cargo)
                    .write((*source_ge).max_waiting_cargo.max(observed.amount));
            }
        }
    };
    let (context, callback) = scope_context(&mut body);
    (w.with_truncation)(st, cargo.0, amount, context, callback);
}
/// `UpdateStationAcceptance`: update the acceptance for a station.
pub(crate) unsafe fn update_station_acceptance(st: *mut BaseStation, show_msg: bool, w: &World) {
    let state = (w.owner)(st);
    /* old accepted goods types */
    let old_acc = unsafe { acceptance_mask(state) };
    /* And retrieve the acceptance. */
    let mut acceptance = [0_u32; NUM_CARGO as usize];
    // SAFETY: Live owner; the tile procs cannot reset this station's catchment.
    if !unsafe { crate::station::rect_is_empty(state) } {
        let c = crate::station::Ctx::from_shared(unsafe { &*w.shared });
        let always: *mut CargoTypes = unsafe { &raw mut (*state).fields.always_accepted }.cast();
        acceptance = unsafe { crate::station::get_acceptance_around_station(state, always, &c) };
    }
    /* Adjust in case our station only accepts fewer kinds of goods */
    let facilities = StationFacilities((w.facilities)(st));
    for cargo in (0..NUM_CARGO).map(CargoType) {
        let mut amt = acceptance[usize::from(cargo.0)];
        /* Make sure the station can accept the goods type. */
        let is_passengers = (w.passenger_cargo)(cargo.0);
        if (!is_passengers
            && !facilities.any(
                StationFacilities::from_element(StationFacility::Train)
                    .with(StationFacilities::from_element(StationFacility::TruckStop))
                    .with(StationFacilities::from_element(StationFacility::Airport))
                    .with(StationFacilities::from_element(StationFacility::Dock)),
            ))
            || (is_passengers
                && !facilities.any(
                    StationFacilities::from_element(StationFacility::Train)
                        .with(StationFacilities::from_element(StationFacility::BusStop))
                        .with(StationFacilities::from_element(StationFacility::Airport))
                        .with(StationFacilities::from_element(StationFacility::Dock)),
                ))
        {
            amt = 0;
        }
        let ge = unsafe { cp(state, cargo) };
        let mut status = unsafe { goods_status(ge) };
        status.set_to(GoodsState::Acceptance, amt >= 8);
        unsafe { set_goods_status(ge, status) };
        unsafe {
            live::station_demand(st, cargo.0, amt / 8, w.graphs);
        }
    }
    /* Only show a message in case the acceptance was actually changed. */
    let new_acc = unsafe { acceptance_mask(state) };
    if old_acc == new_acc {
        return;
    }
    /* show a message to report that the acceptance was changed? */
    if show_msg && w.local_owner(st) && w.in_use(st) {
        /* Combine old and new masks to get changes */
        let accepts = new_acc.0 & !old_acc.0;
        let rejects = !new_acc.0 & old_acc.0;
        /* Show news message if there are any changes */
        if accepts != 0 {
            (w.acceptance_news)(st, accepts, false);
        }
        if rejects != 0 {
            (w.acceptance_news)(st, rejects, true);
        }
    }
    /* redraw the station view since acceptance changed */
    // `SetWindowWidgetDirty` of the ratings list; the same redraw as unchanged waiting.
    (w.rating_redraw)(st, false);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_acceptance(
    st: *mut BaseStation,
    world: *const World,
    show: u8,
) {
    unsafe { update_station_acceptance(st, show != 0, &*world) };
}
/// `TriggerWatchedCargoCallbacks`: run the watched cargo callback for all houses in the
/// catchment area.
unsafe fn trigger_watched_cargo_callbacks(st: *mut BaseStation, w: &World) {
    let state = (w.owner)(st);
    /* Collect cargoes accepted since the last big tick. */
    let mut cargoes = 0_u64;
    for cargo_type in 0..NUM_CARGO {
        if unsafe { goods_status(cp(state, CargoType(cargo_type))) }
            .test(GoodsState::AcceptedBigtick)
        {
            set_bit64(&mut cargoes, cargo_type);
        }
    }
    /* Anything to do? */
    if cargoes == 0 {
        return;
    }
    // The immutable shared table outlives this entry.
    let shared = unsafe { &*w.shared };
    let c = crate::station::Ctx::from_shared(shared);
    /* Loop over all houses in the catchment. */
    // SAFETY: Live owner; house animation triggers cannot reset its catchment.
    unsafe {
        crate::station::for_each_catchment_tile(state, &c, |tile| {
            if (shared.tile_type)(tile.0) == crate::tile_type::MP_HOUSE {
                crate::town::house::trigger_house_animation_watched_cargo_accepted(
                    c.c.town,
                    shared,
                    tile,
                    CargoTypes(cargoes),
                );
            }
        });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_watched(st: *mut BaseStation, world: *const World) {
    unsafe { trigger_watched_cargo_callbacks(st, &*world) };
}
/// `byte_inc_sat`: increment a byte, saturating at its maximum.
fn byte_inc_sat(p: *mut u8) {
    // SAFETY: Callers pass a live field of a serialized station owner.
    unsafe {
        let b = p.read().wrapping_add(1);
        if b != 0 {
            p.write(b);
        }
    }
}
/// `UpdateStationRating`: periodic update of a station's rating.
#[allow(clippy::too_many_lines)]
unsafe fn update_station_rating(st: *mut BaseStation, w: &World) {
    const WAITING_CARGO_THRESHOLD: u32 = 1 << 12;
    const WAITING_CARGO_CUT_FACTOR: u32 = 1 << 6;
    const MAX_WAITING_CARGO: u32 = 1 << 15;
    let state = (w.owner)(st);
    let mut waiting_changed = false;
    byte_inc_sat(unsafe { &raw mut (*state).fields.time_since_load });
    byte_inc_sat(unsafe { &raw mut (*state).fields.time_since_unload });
    for cargo in (0..NUM_CARGO).map(CargoType) {
        if !(w.cargo_valid)(cargo.0) {
            continue;
        }
        let ge = unsafe { cp(state, cargo) };
        /* The station might not currently be moving this cargo. */
        if !goods_entry::has_rating(unsafe { goods_status(ge) }) {
            /* Slowly increase the rating back to its original level in the case we
             *  didn't deliver cargo yet to this station. This happens when a bribe
             *  failed while you didn't moved that cargo yet to a station. */
            unsafe {
                if (*ge).rating < INITIAL_STATION_RATING {
                    (&raw mut (*ge).rating).write((*ge).rating + 1);
                }
            }
            /* Nothing else to do with this cargo. */
            continue;
        }
        byte_inc_sat(unsafe { &raw mut (*ge).time_since_pickup });
        /* If this cargo hasn't been picked up in a long time, get rid of it. */
        if unsafe { (*ge).time_since_pickup } == 255 && w.select_goods() {
            unsafe {
                let mut status = goods_status(ge);
                status.reset(GoodsState::Rating);
                set_goods_status(ge, status);
                (&raw mut (*ge).last_speed).write(0);
                truncate_cargo(st, cargo, u32::MAX, w);
            }
            waiting_changed = true;
            continue;
        }
        let mut skip = false;
        let mut rating = 0_i32;
        let counts = (w.rating_counts)(st, cargo.0);
        let mut waiting = counts.available;
        /* num_dests is at least 1 if there is any cargo as
         * StationID::Invalid() is also a destination.
         */
        let num_dests = counts.destinations;
        /* Average amount of cargo per next hop, but prefer solitary stations
         * with only one or two next hops. They are allowed to have more
         * cargo waiting per next hop.
         * With manual cargo distribution waiting_avg = waiting / 2 as then
         * StationID::Invalid() is the only destination.
         */
        let waiting_avg = waiting / num_dests.wrapping_add(1);
        if (w.rating_cheat)() {
            unsafe {
                (&raw mut (*ge).rating).write(MAX_STATION_RATING);
            }
            rating = i32::from(MAX_STATION_RATING);
            skip = true;
        } else if (w.rating_callback_enabled)(cargo.0) {
            /* Perform custom station rating. If it succeeds the speed, days in transit and
             * waiting cargo ratings must not be executed. */
            /* NewGRFs expect last speed to be 0xFF when no vehicle has arrived yet. */
            let last_speed = if has_vehicle_ever_tried_loading(unsafe { (*ge).last_speed }) {
                u32::from(unsafe { (*ge).last_speed })
            } else {
                0xFF
            };
            let var18 = unsafe {
                u32::from(clamp_to_u8(i64::from((*ge).time_since_pickup)))
                    | (u32::from(clamp_to_u16(i64::from((*ge).max_waiting_cargo))) << 8)
                    | (u32::from(clamp_to_u8(i64::from(last_speed))) << 24)
            };
            /* Convert to the 'old' vehicle types */
            let last_vehicle_type = VehicleType(unsafe { (*state).fields.last_vehicle_type });
            let var10 = if last_vehicle_type == VehicleType::VEH_INVALID {
                0x0
            } else {
                u32::from(last_vehicle_type.0) + 0x10
            };
            let callback = unsafe {
                crate::cargo_registry::get_cargo_callback(
                    &*w.cargo_registry,
                    cargo.0,
                    crate::newgrf_callbacks::CBID_CARGO_STATION_RATING_CALC.0,
                    var10,
                    var18,
                )
            };
            if callback != crate::newgrf_callbacks::CALLBACK_FAILED {
                skip = true;
                rating = gb(u32::from(callback), 0, 14) as i32;
                /* Simulate a 15 bit signed value */
                if has_bit(callback, 14) {
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
            if VehicleType(unsafe { (*state).fields.last_vehicle_type }) == VehicleType::SHIP {
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
            let max_waiting_cargo = unsafe { (*ge).max_waiting_cargo };
            if max_waiting_cargo <= 1500 {
                rating += 55;
            }
            if max_waiting_cargo <= 1000 {
                rating += 35;
            }
            if max_waiting_cargo <= 600 {
                rating += 10;
            }
            if max_waiting_cargo <= 300 {
                rating += 20;
            }
            if max_waiting_cargo <= 100 {
                rating += 10;
            }
        }
        if (w.town_statue)(st) {
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
        {
            // old rating
            let or_ = i32::from(unsafe { (*ge).rating });
            /* only modify rating in steps of -2, -1, 0, 1 or 2 */
            let new_rating = clamp_to_u8(i64::from(or_ + (rating - or_).clamp(-2, 2)));
            unsafe {
                (&raw mut (*ge).rating).write(new_rating);
            }
            rating = i32::from(new_rating);
            /* if rating is <= 64 and more than 100 items waiting on average per destination,
             * remove some random amount of goods from the station */
            if rating <= 64 && waiting_avg >= 100 {
                let mut dec = w.random() & 0x1F;
                if waiting_avg < 200 {
                    dec &= 7;
                }
                waiting = waiting.wrapping_sub((dec + 1).wrapping_mul(num_dests));
                waiting_changed = true;
            }
            /* if rating is <= 127 and there are any items waiting, maybe remove some goods. */
            if rating <= 127 && waiting != 0 {
                let r = w.random();
                if rating <= gb(r, 0, 7) as i32 {
                    /* Need to have int, otherwise it will just overflow etc. */
                    waiting = (waiting as i32)
                        .wrapping_sub((gb(r, 8, 2) + 1).wrapping_mul(num_dests) as i32)
                        .max(0) as u32;
                    waiting_changed = true;
                }
            }
            /* At some point we really must cap the cargo. Previously this
             * was a strict 4095, but now we'll have a less strict, but
             * increasingly aggressive truncation of the amount of cargo. */
            if waiting > WAITING_CARGO_THRESHOLD {
                let difference = waiting - WAITING_CARGO_THRESHOLD;
                waiting -= difference / WAITING_CARGO_CUT_FACTOR;
                waiting = waiting.min(MAX_WAITING_CARGO);
                waiting_changed = true;
            }
            /* We can't truncate cargo that's already reserved for loading.
             * Thus StoredCount() here. */
            let available = (w.available)(st, cargo.0);
            if waiting_changed && waiting < available {
                /* Feed back the exact own waiting cargo at this station for the
                 * next rating calculation. */
                unsafe {
                    (&raw mut (*ge).max_waiting_cargo).write(0);
                    truncate_cargo(st, cargo, available - waiting, w);
                }
            } else {
                /* If the average number per next hop is low, be more forgiving. */
                unsafe {
                    (&raw mut (*ge).max_waiting_cargo).write(waiting_avg);
                }
            }
        }
    }
    // `SetWindowDirty` of the whole window when waiting changed; otherwise only the ratings list.
    (w.rating_redraw)(st, waiting_changed);
}
/// The economy monthly loop for stations (`_economy_stations_monthly`).
pub(crate) unsafe fn station_monthly_loop(w: &World) {
    for id in 0..(w.pool_size)() {
        let st = (w.get)(id as u16);
        if st.is_null() || !(w.is_station)(st) {
            continue;
        }
        let state = (w.owner)(st);
        for cargo in (0..NUM_CARGO).map(CargoType) {
            let ge = unsafe { cp(state, cargo) };
            let mut status = unsafe { goods_status(ge) };
            status.set_to(GoodsState::LastMonth, status.test(GoodsState::CurrentMonth));
            status.reset(GoodsState::CurrentMonth);
            unsafe { set_goods_status(ge, status) };
        }
    }
}

/// `UpdateStationWaiting`.
unsafe fn update_station_waiting(
    st: *mut BaseStation,
    cargo: CargoType,
    mut amount: u32,
    source: crate::cargo_storage::Source,
    w: &World,
    visits: &Visits,
) -> u32 {
    /* We can't allocate a CargoPacket? Then don't do anything
     * at all; i.e. just discard the incoming cargo. */
    if !(w.packet_available)() {
        return 0;
    }
    let state = (w.owner)(st);
    let ge = unsafe { cp(state, cargo) };
    amount = amount.wrapping_add(u32::from(unsafe { (*ge).amount_fract }));
    unsafe {
        (&raw mut (*ge).amount_fract).write(gb(amount, 0, 8) as u8);
    }
    amount >>= 8;
    /* No new "real" cargo item yet. */
    if amount == 0 {
        return 0;
    }
    (w.append_packet)(st, cargo.0, amount, source);
    unsafe {
        live::station_supply(st, cargo.0, amount, w.graphs, visits.schedule);
    }
    if !goods_entry::has_rating(unsafe { goods_status(ge) }) {
        (w.station_list_invalidation)(st, 0);
        let mut status = unsafe { goods_status(ge) };
        status.set(GoodsState::Rating);
        unsafe { set_goods_status(ge, status) };
    }
    crate::animated_tile::cargo_event(
        st,
        cargo.0,
        crate::station_type::SAT_NEW_CARGO,
        crate::station_type::AAT_NEW_CARGO,
        crate::station_type::SRT_NEW_CARGO,
    );
    (w.waiting_redraw)(st);
    amount
}
/// `CanMoveGoodsToStation`, after the caller's observation of the station's owner and
/// service state.
fn can_move_goods_to_station(
    st: *mut BaseStation,
    state: *mut Service,
    owner: Owner,
    cargo: CargoType,
    w: &World,
) -> bool {
    /* Is the station reserved exclusively for somebody else? */
    if owner != Owner::NONE {
        let observed = (w.station_exclusivity)(st);
        if observed.counter > 0 && CompanyID(observed.exclusive) != owner {
            return false;
        }
    }
    // SAFETY: scalar reads of the live serialized owner; no borrow survives.
    let ge = unsafe { cp(state, cargo) };
    /* Lowest possible rating, better not to give cargo anymore. */
    if unsafe { (*ge).rating } == 0 {
        return false;
    }
    /* Selectively servicing stations, and not this one. */
    if w.select_goods() && !has_vehicle_ever_tried_loading(unsafe { (*ge).last_speed }) {
        return false;
    }
    let facilities = StationFacilities((w.facilities)(st));
    if (w.passenger_cargo)(cargo.0) {
        /* Passengers are never served by just a truck stop. */
        if facilities == StationFacilities::from_element(StationFacility::TruckStop) {
            return false;
        }
    } else {
        /* Non-passengers are never served by just a bus stop. */
        if facilities == StationFacilities::from_element(StationFacility::BusStop) {
            return false;
        }
    }
    true
}
/// `MoveGoodsToStation` over a live station set.
/// # Safety
/// `stations`, `world` and `visits` are live for the call; the set may be
/// re-read between station observations.
pub(crate) unsafe fn move_goods_to_station(
    cargo: CargoType,
    mut amount: u32,
    source: crate::cargo_storage::Source,
    all_stations: *const crate::station::StationList,
    exclusivity: Owner,
    world: *const World,
    visits: &Visits,
) -> u32 {
    /* Return if nothing to do. Also the rounding below fails for 0. */
    if amount == 0 {
        return 0;
    }
    let w = unsafe { &*world };
    // `st->goods[cargo].rating`, through the owner resolved once per station.
    let rating = |state| u32::from(unsafe { (*cp(state, cargo)).rating });
    let mut first_station = None;
    // The station, its service owner, its company and its share of the cargo.
    let mut used_stations: Vec<(*mut BaseStation, *mut Service, Owner, u32)> = Vec::new();
    // SAFETY: Caller contract; observations below do not change station sets.
    unsafe {
        crate::station::for_each_station(all_stations, |entry| {
            let st = entry.station;
            let owner = CompanyID((w.station_owner)(st));
            if exclusivity != Owner::INVALID && exclusivity != owner {
                return;
            }
            let state = (w.owner)(st);
            if !can_move_goods_to_station(st, state, owner, cargo, w) {
                return;
            }
            /* Avoid allocating a vector if there is only one station to significantly
             * improve performance in this common case. */
            let Some(first) = first_station else {
                first_station = Some((st, state, owner, 0));
                return;
            };
            if used_stations.is_empty() {
                used_stations.reserve(2);
                used_stations.push(first);
            }
            used_stations.push((st, state, owner, 0));
        });
    };
    /* no stations around at all? */
    let Some((first_station, first_state, _, _)) = first_station else {
        return 0;
    };
    if used_stations.is_empty() {
        /* only one station around */
        amount = amount.wrapping_mul(rating(first_state) + 1);
        return unsafe { update_station_waiting(first_station, cargo, amount, source, w, visits) };
    }
    let mut company_best = [0_u32; OWNER_END as usize]; // best rating for each company, including OWNER_NONE
    let mut company_sum = [0_u32; OWNER_END as usize]; // sum of ratings for each company
    let mut best_rating = 0;
    let mut best_sum = 0_u32; // sum of best ratings for each company
    for &(_, state, owner, _) in &used_stations {
        let owner = usize::from(owner.0);
        let rating = rating(state);
        if rating > company_best[owner] {
            best_sum = best_sum.wrapping_add(rating - company_best[owner]); // it's usually faster than iterating companies later
            company_best[owner] = rating;
            if rating > best_rating {
                best_rating = rating;
            }
        }
        company_sum[owner] = company_sum[owner].wrapping_add(rating);
    }
    /* From now we'll calculate with fractional cargo amounts.
     * First determine how much cargo we really have. */
    amount = amount.wrapping_mul(best_rating + 1);
    let mut moving = 0_u32;
    for (_, state, owner, share) in &mut used_stations {
        let owner = usize::from(owner.0);
        /* Multiply the amount by (company best / sum of best for each company) to get cargo allocated to a company
         * and by (station rating / sum of ratings in a company) to get the result for a single station. */
        *share = amount
            .wrapping_mul(company_best[owner])
            .wrapping_mul(rating(*state))
            / best_sum
            / company_sum[owner];
        moving = moving.wrapping_add(*share);
    }
    /* If there is some cargo left due to rounding issues distribute it among the best rated stations. */
    if amount > moving {
        used_stations.sort_by_key(|&(_, state, _, _)| std::cmp::Reverse(rating(state)));

        let map = crate::map_func::World::new(unsafe { &*w.shared });
        if map.types.assertions() {
            map.check((amount - moving) as usize <= used_stations.len());
        }
        for i in 0..amount - moving {
            used_stations[i as usize].3 = used_stations[i as usize].3.wrapping_add(1);
        }
    }
    let mut moved = 0_u32;
    for (st, _, _, share) in used_stations {
        moved = moved
            .wrapping_add(unsafe { update_station_waiting(st, cargo, share, source, w, visits) });
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
        cargo: CargoType,
    ) -> *const crate::cargo_storage::List {
        unsafe { ((*self.station_resolver).cargo_owner)(st, cargo.0) }
    }
    pub(crate) fn vehicle_cargo(&self, v: VehicleRef) -> *mut crate::cargo_storage::List {
        unsafe { ((*self.vehicle_resolver).cargo_owner)(v.0.as_ptr()) }.cast_mut()
    }
}
/// `RerouteCargo`: reroute cargo of type `cargo` at station `st` or in any vehicles unloading
/// there. Make sure the cargo's new next hop is neither `avoid` nor `avoid2`.
pub(crate) unsafe fn reroute_cargo(
    st: *mut BaseStation,
    cargo: CargoType,
    avoid: StationID,
    avoid2: StationID,
    w: &World,
    l: &Links,
) {
    let storage = l.storage();
    let ge = (l.goods_entry)(st, cargo.0);
    /* Reroute cargo in station. */
    let list = l.station_cargo(st, cargo);
    if !list.is_null() {
        unsafe {
            crate::cargo_storage::reroute_station(list.cast_mut(), storage, ge, avoid.0, avoid2.0);
        }
    }
    /* Reroute cargo staged to be transferred. */
    let state = (w.owner)(st);
    let mut node = unsafe { loading_vehicles_begin(state) };
    while !node.is_null() {
        let mut u = NonNull::new(unsafe { loading_vehicles_value(node) }).map(|shell| unsafe {
            VehicleRef::from_shell(shell.as_ptr().cast(), (*l.vehicle_resolver).vehicle)
        });
        while let Some(vehicle) = u {
            if (l.cargo_type)(vehicle) == cargo.0 {
                unsafe {
                    crate::cargo_storage::reroute_vehicle(
                        l.vehicle_cargo(vehicle),
                        storage,
                        ge,
                        avoid.0,
                        avoid2.0,
                    );
                }
            }
            u = vehicle.next();
        }
        node = unsafe { loading_vehicles_next(node) };
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
    unsafe {
        reroute_cargo(
            st,
            CargoType(cargo),
            StationID(avoid),
            StationID(avoid2),
            &*world,
            &*links,
        );
    }
}
/// `DeleteStaleLinks`: check all next hops of cargo packets in this station for existence of
/// a valid link they may use to travel on. Reroute any cargo not having a valid link and
/// remove timed out links found like this from the linkgraph. We're not all links here as
/// that is expensive and useless. A link no one is using doesn't hurt either.
#[allow(clippy::too_many_lines)]
unsafe fn delete_stale_links(from: *mut BaseStation, w: &World, l: &Links, visits: &Visits) {
    let map = crate::map_func::World::new(unsafe { &*w.shared });
    for cargo in (0..NUM_CARGO).map(CargoType) {
        let auto_distributed = (l.automatic_distribution)(cargo.0);
        let lg = live::station_graph(from, cargo.0, l.graphs);
        if lg.is_null() {
            continue;
        }
        let node = (l.graphs.node)(from, cargo.0);
        let mut to_remove = Vec::new();
        let count = unsafe { &*lg }.nodes[usize::from(node)].edges.len();
        for index in 0..count {
            let (destination, dest_node, last_update) = {
                let g = unsafe { &*lg };
                let e = &g.nodes[usize::from(node)].edges[index];
                (
                    g.nodes[usize::from(e.dest_node)].station,
                    e.dest_node,
                    e.last_update(),
                )
            };
            let edge = (l.edge)(from, destination);
            let to = edge.destination;
            let date = crate::timer_game_common::economy_date().0;
            if map.types.assertions() {
                map.check((l.graphs.node)(to, cargo.0) == dest_node);
                map.check(date >= last_update);
            }
            let timeout = live::MIN_TIMEOUT_DISTANCE.wrapping_add(edge.distance >> 3) as i32;
            if date.wrapping_sub(last_update) > timeout {
                let mut updated = false;
                if auto_distributed {
                    /* Have all vehicles refresh their next hops before deciding to
                     * remove the node. */
                    let mut vehicles = Vec::new();
                    let from_id = StationID((l.graphs.station_id)(from));
                    let to_id = StationID((l.graphs.station_id)(to));
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
                            if observed.kind != OrderType::OT_GOTO_STATION
                                && observed.kind != OrderType::OT_IMPLICIT
                            {
                                continue;
                            }
                            let destination = StationID(observed.destination);
                            if destination == from_id {
                                found_from = true;
                                if found_to {
                                    break;
                                }
                            } else if destination == to_id {
                                found_to = true;
                                if found_from {
                                    break;
                                }
                            }
                        }
                        if !found_to || !found_from {
                            continue;
                        }
                        vehicles.push(unsafe { (l.order_vehicle)(list).unwrap_unchecked() });
                    }
                    let mut iter = 0;
                    while iter < vehicles.len() {
                        let v = vehicles[iter];
                        /* Do not refresh links of vehicles that have been stopped in depot for a long time. */
                        if !(l.stopped_in_depot)(v)
                            || date.wrapping_sub((l.last_service)(v))
                                <= live::STALE_LINK_DEPOT_TIMEOUT
                        {
                            refresh_links(v, false, false, visits); // Don't allow merging. Otherwise lg might get deleted.
                        }
                        if unsafe { &*lg }.nodes[usize::from(node)].edges[index].last_update()
                            == date
                        {
                            crate::witness::hit(crate::witness::STATION_STALE_REFRESH);
                            updated = true;
                            break;
                        }
                        let next_shared = (l.next_shared)(v);
                        if let Some(next_shared) = next_shared {
                            vehicles[iter] = next_shared;
                            iter += 1;
                        } else {
                            vehicles.remove(iter);
                        }
                        if iter == vehicles.len() {
                            iter = 0;
                        }
                    }
                }
                if !updated {
                    /* If it's still considered dead remove it. */
                    crate::witness::hit(crate::witness::STATION_STALE_REMOVE);
                    to_remove.push((l.graphs.node)(to, cargo.0));
                    let to_index = StationID((l.graphs.station_id)(to));
                    (l.delete_flows)(from, cargo.0, to_index.0);
                    unsafe {
                        reroute_cargo(
                            from,
                            cargo,
                            to_index,
                            StationID((l.graphs.station_id)(from)),
                            w,
                            l,
                        );
                    }
                }
            } else {
                let unrestricted =
                    unsafe { &*lg }.nodes[usize::from(node)].edges[index].last_unrestricted_update;
                if unrestricted != INVALID_DATE && date.wrapping_sub(unrestricted) > timeout {
                    crate::witness::hit(crate::witness::STATION_STALE_RESTRICT);
                    unsafe { &mut *lg }.nodes[usize::from(node)].edges[index].restrict();
                    let to_index = StationID((l.graphs.station_id)(to));
                    (l.restrict_flows)(from, cargo.0, to_index.0);
                    unsafe {
                        reroute_cargo(
                            from,
                            cargo,
                            to_index,
                            StationID((l.graphs.station_id)(from)),
                            w,
                            l,
                        );
                    }
                } else {
                    let restricted = unsafe { &*lg }.nodes[usize::from(node)].edges[index]
                        .last_restricted_update;
                    if restricted != INVALID_DATE && date.wrapping_sub(restricted) > timeout {
                        crate::witness::hit(crate::witness::STATION_STALE_RELEASE);
                        unsafe { &mut *lg }.nodes[usize::from(node)].edges[index].release();
                    }
                }
            }
        }
        /* Remove dead edges. */
        for r in to_remove {
            unsafe { &mut *lg }.nodes[usize::from(node)].remove_edge(r);
        }
        let date = crate::timer_game_common::economy_date().0;
        if map.types.assertions() {
            map.check(date >= unsafe { &*lg }.last_compression);
        }
        if date.wrapping_sub(unsafe { &*lg }.last_compression) > live::COMPRESSION_INTERVAL {
            unsafe { &mut *lg }.compress(date);
        }
    }
}
/// `StationHandleSmallTick`: called for every station each tick.
unsafe fn station_handle_small_tick(st: *mut BaseStation, w: &World) {
    if StationFacilities((w.facilities)(st)).test(StationFacility::Waypoint) || !w.in_use(st) {
        return;
    }
    let state = (w.owner)(st);
    let mut b = unsafe { (*state).fields.delete_ctr }.wrapping_add(1);
    if i32::from(b) >= STATION_RATING_TICKS {
        b = 0;
    }
    unsafe {
        (&raw mut (*state).fields.delete_ctr).write(b);
    }
    if b == 0 {
        unsafe { update_station_rating(st, w) };
    }
}
/// `StationHandleBigTick`: called for each station once every 250 ticks. Not all stations
/// will get the tick at the same time. Returns true if the station is still valid (wasn't
/// deleted).
unsafe fn station_handle_big_tick(st: *mut BaseStation, w: &World) -> bool {
    let state = (w.owner)(st);
    if !w.in_use(st) {
        let delete_ctr = unsafe { (*state).fields.delete_ctr }.wrapping_add(1);
        unsafe {
            (&raw mut (*state).fields.delete_ctr).write(delete_ctr);
        }
        if delete_ctr >= 8 {
            (w.delete_station)(st);
        }
        return false;
    }
    if (w.is_station)(st) {
        unsafe { trigger_watched_cargo_callbacks(st, w) };
        for cargo in (0..NUM_CARGO).map(CargoType) {
            let ge = unsafe { cp(state, cargo) };
            let mut status = unsafe { goods_status(ge) };
            status.reset(GoodsState::AcceptedBigtick);
            unsafe { set_goods_status(ge, status) };
        }
    }
    if !StationFacilities((w.facilities)(st)).test(StationFacility::Waypoint) {
        unsafe { update_station_acceptance(st, true, w) };
    }
    true
}
/// `OnTick_Station`.
pub(crate) unsafe fn on_tick_station(w: &World, l: &Links, visits: &Visits) {
    if (w.environment.editor)() {
        return;
    }
    let mut id = 0_usize;
    while id < (w.pool_size)() {
        let st = (w.get)(id as u16);
        if !st.is_null() {
            unsafe { station_handle_small_tick(st, w) };
            // SAFETY: the immutable shared table outlives this synchronous entry.
            let counter = (unsafe { &*w.shared }.tick_counter)().wrapping_add(id as u64);
            /* Clean up the link graph about once a week. */
            if (w.is_station)(st) && counter % STATION_LINKGRAPH_TICKS as u64 == 0 {
                unsafe { delete_stale_links(st, w, l, visits) };
            }
            /* Spread out big-tick over STATION_ACCEPTANCE_TICKS ticks. */
            /* Stop processing this station if it was deleted */
            if counter % STATION_ACCEPTANCE_TICKS as u64 == 0
                && !unsafe { station_handle_big_tick(st, w) }
            {
                id += 1;
                continue;
            }
            /* Spread out station animation over STATION_ACCEPTANCE_TICKS ticks. */
            if counter % STATION_ACCEPTANCE_TICKS as u64 == 0 {
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

/// `v->cargo`: the vehicle's Rust-owned cargo list, resolved once per vehicle entry. Its
/// counts and reassignments run here on the owner the native cargo observation names, not
/// through native wrappers. The owner is stable for the vehicle's lifetime.
#[derive(Clone, Copy)]
struct VehicleCargo(*mut crate::cargo_storage::List);
impl Loading {
    /// `v->cargo`.
    fn cargo_list(&self, v: VehicleRef) -> VehicleCargo {
        VehicleCargo((self.cargo_capacity)(v).owner.cast_mut())
    }
    /// `v->cargo.StoredCount()`.
    pub(crate) fn stored_count(&self, v: VehicleRef) -> u32 {
        self.cargo_list(v).stored_count()
    }
}
impl VehicleCargo {
    /// `StoredCount()`.
    fn stored_count(self) -> u32 {
        // SAFETY: The live vehicle's list; the count read ends before any callback.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_stored(self.0) }
    }
    /// `UnloadCount()`.
    fn unload_count(self) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_unload(self.0) }
    }
    /// `TotalCount()`.
    fn total_count(self) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_count(self.0) }
    }
    /// `RemainingCount()`.
    fn remaining_count(self) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_remaining(self.0) }
    }
    /// `ReservedCount()`.
    fn reserved_count(self) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_reserved(self.0) }
    }
    /// `ActionCount(action)`.
    fn action_count(self, action: MoveToAction) -> u32 {
        // SAFETY: As `stored_count`.
        unsafe { crate::cargo_storage::openttd_rust_cargo_list_action_count(self.0, action.0) }
    }
    /// `Reassign<from, to>(amount)`.
    fn reassign(self, l: &Loading, from: MoveToAction, to: MoveToAction, amount: u32) {
        // SAFETY: The live vehicle's list and the immutable cargo services; no Rust
        // borrow of the list spans the reassignment.
        unsafe { crate::cargo_storage::reassign(self.0, *l.cargo, from, to, amount) };
    }
}
/// `v->orders != nullptr`: the vehicle's order list is assigned.
fn has_orders(v: VehicleRef) -> bool {
    let orders = crate::raw_field::read_field!(v.0.as_ptr(), orders);
    !crate::raw_field::read_field!(orders, orders).is_null()
}

/// `IterateVehicleParts`: iterate the articulated parts of a vehicle, also considering the
/// special cases of "normal" aircraft and double headed trains. Apply an action to each
/// vehicle and immediately return false if that action does so. Otherwise return true.
fn iterate_vehicle_parts(
    v: VehicleRef,
    shared: &crate::services::Services,
    mut action: impl FnMut(VehicleRef) -> bool,
) -> bool {
    let mut w = Some(v);
    while let Some(part) = w {
        if !action(part) {
            return false;
        }
        if VehicleType(part.vehicle_type()) == VehicleType::TRAIN
            && crate::ground_vehicle::is_multiheaded(part.subtype())
            && !action(part.other_multiheaded_part().unwrap())
        {
            return false;
        }
        w = if part.has_articulated_part() {
            part.get_next_articulated_part(shared)
        } else {
            None
        };
    }
    if VehicleType(v.vehicle_type()) == VehicleType::AIRCRAFT
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
/// `MayLoadUnderExclusiveRights`: test whether a vehicle can load cargo at a station even if
/// exclusive transport rights are present.
fn may_load_under_exclusive_rights(st: LoadingStation, v: VehicleRef, w: &World) -> bool {
    if CompanyID((w.station_owner)(st.base)) != Owner::NONE {
        return true;
    }
    let observed = (w.station_exclusivity)(st.base);
    observed.counter == 0 || observed.exclusive == v.owner()
}
/// `ReserveConsist`: reserves cargo if the full load order and `improved_load` is set or if
/// the current order allows autorefit.
fn reserve_consist(
    st: LoadingStation,
    u: VehicleRef,
    control: &LoadingControl,
    mut consist_capleft: Option<&mut CargoArray>,
    next_station: *mut c_void,
    w: &World,
    l: &Loading,
) {
    /* If there is a cargo payment not all vehicles of the consist have tried to do the refit.
     * In that case, only reserve if it's a fixed refit and the equivalent of "articulated chain"
     * a vehicle belongs to already has the right cargo. */
    let must_reserve = !control.refit || (l.get_payment)(u).is_null();
    let map = crate::map_func::World::new(unsafe { &*w.shared });
    let mut current = Some(u);
    while let Some(v) = current {
        if map.types.assertions() {
            map.check(u32::from(v.cargo_cap()) >= l.cargo_list(v).remaining_count());
        }

        /* Exclude various ways in which the vehicle might not be the head of an equivalent of
         * "articulated chain". Also don't do the reservation if the vehicle is going to refit
         * to a different cargo and hasn't tried to do so, yet. */
        let reserve = if v.is_articulated_part() {
            false
        } else {
            let kind = VehicleType(v.vehicle_type());
            (kind != VehicleType::TRAIN || !crate::ground_vehicle::is_rear_dualheaded(v.subtype()))
                && (kind != VehicleType::AIRCRAFT
                    || crate::aircraft::is_normal_aircraft(v.subtype()))
                && (must_reserve || control.refit_cargo == (l.cargo_type)(v))
        };
        if reserve {
            // `ReserveCargoAction`.
            iterate_vehicle_parts(v, unsafe { &*w.shared }, |part| {
                let cap = u32::from(part.cargo_cap());
                let remaining = l.cargo_list(part).remaining_count();
                if cap > remaining && may_load_under_exclusive_rights(st, part, w) {
                    (l.reserve_cargo)(st.native, part, cap - remaining, next_station);
                }
                true
            });
        }
        if let Some(ref mut consist_capleft) = consist_capleft
            && v.cargo_cap() != 0
        {
            let observed = (l.cargo_capacity)(v);
            let remaining = VehicleCargo(observed.owner.cast_mut()).remaining_count();
            let cargo = usize::from(observed.cargo);
            consist_capleft[cargo] = consist_capleft[cargo]
                .wrapping_add(u32::from(v.cargo_cap()).wrapping_sub(remaining));
        }
        current = v.next();
    }
}
/// `GetLoadAmount`: gets the amount of cargo the given vehicle can load in the current tick.
/// This is only about loading speed. The free capacity is ignored.
fn get_load_amount(
    v: VehicleRef,
    gradual_loading: bool,
    l: &Loading,
    shared: &crate::services::Services,
) -> u32 {
    // SAFETY: the static base services outlive this synchronous call.
    let vehicle = unsafe { &*l.vehicle };
    let e = (vehicle.engine)(v);
    let mut load_amount = u32::from(e.load_amount());
    /* The default loadamount for mail is 1/4 of the load amount for passengers */
    let air_mail = VehicleType(v.vehicle_type()) == VehicleType::AIRCRAFT
        && !crate::aircraft::is_normal_aircraft(v.subtype());
    if air_mail {
        load_amount = ceil_div(load_amount, 4);
    }
    if gradual_loading {
        let mut cb_load_amount = crate::newgrf_callbacks::CALLBACK_FAILED;
        if e.grf_version(shared) >= 8 {
            /* Use callback 36 */
            cb_load_amount = (vehicle.property)(
                (vehicle.newgrf)(),
                v,
                crate::newgrf_properties::PROP_VEHICLE_LOAD_AMOUNT,
                i32::from(crate::newgrf_callbacks::CALLBACK_FAILED),
            ) as u16;
        } else if e.callbacks() & crate::newgrf_callbacks::VEHICLE_LOAD_AMOUNT != 0 {
            /* Use callback 12 */
            cb_load_amount = (vehicle.callback)(
                (vehicle.newgrf)(),
                v,
                crate::newgrf_callbacks::CBID_VEHICLE_LOAD_AMOUNT,
            );
        }
        if cb_load_amount != crate::newgrf_callbacks::CALLBACK_FAILED {
            if e.grf_version(shared) < 8 {
                cb_load_amount = gb(u32::from(cb_load_amount), 0, 8) as u16;
            }
            if cb_load_amount >= 0x100 {
                vehicle.unknown_callback_result(
                    e,
                    crate::newgrf_callbacks::CBID_VEHICLE_LOAD_AMOUNT,
                    cb_load_amount,
                );
            } else if cb_load_amount != 0 {
                load_amount = u32::from(cb_load_amount);
            }
        }
    }
    /* Scale load amount the same as capacity */
    if e.misc() & crate::engine_type::NO_DEFAULT_CARGO_MULTIPLIER != 0 && !air_mail {
        load_amount = ceil_div(
            load_amount.wrapping_mul(u32::from((l.cargo_multiplier)(v))),
            0x100,
        );
    }
    /* Zero load amount breaks a lot of things. */
    load_amount.max(1)
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
    /// `Command<CMD_REFIT_VEHICLE>::Do(flags, v->index, cargo_type, 0xFF, true, false, 1)`:
    /// auto-refit and only this vehicle including artic parts.
    fn run(
        &self,
        v: VehicleRef,
        cargo: CargoType,
        flags: DoCommandFlags,
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
            flags.0,
            |flags| unsafe {
                crate::vehicle_transactions::refit::cmd_refit_vehicle(
                    services,
                    self.out,
                    self.temporary,
                    DoCommandFlags(flags),
                    crate::vehicle_transactions::refit::CmdRefitVehicle {
                        veh_id: crate::vehicle_type::VehicleID(id),
                        new_cargo_type: cargo,
                        new_subtype: 0xFF,
                        auto_refit: true,
                        only_this: false,
                        num_vehicles: 1,
                    },
                )
            },
        )
    }
}
/// The tables and native cost slots of one `LoadUnloadStation` entry.
struct LoadingContext<'a> {
    w: &'a World,
    l: &'a Loading,
    refit: &'a RefitContext,
    visits: &'a Visits,
}
/// `HandleStationRefit`: refit a vehicle in a station.
fn handle_station_refit(
    v: VehicleRef,
    consist_capleft: &mut CargoArray,
    st: LoadingStation,
    next_station: *mut c_void,
    mut new_cargo_type: CargoType,
    c: &LoadingContext<'_>,
) {
    let (w, l) = (c.w, c.l);
    let v_start = v.first_engine_part();
    // `IsEmptyAction`: the vehicle is either empty or has only reserved cargo.
    let shared = unsafe { &*w.shared };
    if !iterate_vehicle_parts(v_start, shared, |part| {
        l.cargo_list(part).stored_count() == 0
    }) {
        return;
    }
    let cur_company = (l.current_company)();
    (l.set_current_company)(v.owner());
    let mut refit_mask = (unsafe { &*l.vehicle }.engine)(v).refit_mask();
    /* Remove old capacity from consist capacity and collect refit mask. */
    // `PrepareRefitAction`.
    iterate_vehicle_parts(v_start, shared, |part| {
        let observed = (l.cargo_capacity)(part);
        let reserved = VehicleCargo(observed.owner.cast_mut()).reserved_count();
        let cargo = usize::from(observed.cargo);
        consist_capleft[cargo] =
            consist_capleft[cargo].wrapping_sub(u32::from(part.cargo_cap()).wrapping_sub(reserved));
        refit_mask |= (unsafe { &*l.vehicle }.engine)(part).refit_mask();
        true
    });
    let is_auto_refit = new_cargo_type == CargoType::CARGO_AUTO_REFIT;
    if is_auto_refit {
        /* Get a refittable cargo type with waiting cargo for next_station or StationID::Invalid(). */
        new_cargo_type = CargoType((l.cargo_type)(v_start));
        for cargo_type in (0..NUM_CARGO)
            .filter(|&bit| has_bit64(refit_mask, bit))
            .map(CargoType)
        {
            if !(l.has_cargo_for)(st.native, cargo_type.0, next_station) {
                continue;
            }
            /* Try to find out if auto-refitting would succeed. In case the refit is allowed,
             * the returned refit capacity will be greater than zero. */
            let refit_capacity = c
                .refit
                .run(
                    v_start,
                    cargo_type,
                    DoCommandFlags::from_element(DoCommandFlag::QueryCost),
                )
                .total;
            /* Try to balance different loadable cargoes between parts of the consist, so that
             * all of them can be loaded. Avoid a situation where all vehicles suddenly switch
             * to the first loadable cargo for which there is only one packet. If the capacities
             * are equal refit to the cargo of which most is available. This is important for
             * consists of only a single vehicle as those will generally have a consist_capleft
             * of 0 for all cargoes. */
            let (candidate, current) = (usize::from(cargo_type.0), usize::from(new_cargo_type.0));
            if refit_capacity > 0
                && (consist_capleft[candidate] < consist_capleft[current]
                    || (consist_capleft[candidate] == consist_capleft[current]
                        && (w.available)(st.base, cargo_type.0)
                            > (w.available)(st.base, new_cargo_type.0)))
            {
                new_cargo_type = cargo_type;
            }
        }
    }
    /* Refit if given a valid cargo. */
    if new_cargo_type.0 < NUM_CARGO && new_cargo_type.0 != (l.cargo_type)(v_start) {
        /* StationID::Invalid() because in the DT_MANUAL case that's correct and in the DT_(A)SYMMETRIC
         * cases the next hop of the vehicle doesn't really tell us anything if the cargo had been
         * "via any station" before reserving. We rather produce some more "any station" cargo than
         * misrouting it. */
        // `ReturnCargoAction`.
        iterate_vehicle_parts(v_start, shared, |part| {
            (l.return_cargo)(st.native, part, u32::MAX);
            true
        });
        c.refit.run(
            v_start,
            new_cargo_type,
            DoCommandFlags::from_element(DoCommandFlag::Execute),
        );
        let cost = c.refit.cost().money();
        if c.refit.cost().ok() {
            (l.subtract_profit)(v.first(), cost.wrapping_shl(8));
        }
    }
    /* Add new capacity to consist capacity and reserve cargo */
    // `FinalizeRefitAction`.
    let do_reserve = is_auto_refit || v.first().current_order().is_full_load();
    iterate_vehicle_parts(v_start, shared, |part| {
        let part_cargo = l.cargo_list(part);
        if do_reserve {
            let remaining = part_cargo.remaining_count();
            (l.reserve_cargo)(
                st.native,
                part,
                u32::from(part.cargo_cap()).wrapping_sub(remaining),
                next_station,
            );
        }
        let remaining = part_cargo.remaining_count();
        let cargo = usize::from((l.cargo_type)(part));
        consist_capleft[cargo] = consist_capleft[cargo]
            .wrapping_add(u32::from(part.cargo_cap()).wrapping_sub(remaining));
        true
    });
    (l.set_current_company)(cur_company);
}
/// `PrepareUnload`: prepare the vehicle to be unloaded.
pub(crate) fn prepare_unload(front_v: VehicleRef, w: &World, l: &Loading) {
    let curr_station = (w.station)(front_v.last_station());
    unsafe {
        loading_vehicles_push_back(curr_station.state, front_v.shell().cast());
    }
    /* At this moment loading cannot be finished */
    (l.set_loading_finished)(front_v, false);
    /* Start unloading at the first possible moment */
    (l.set_load_ticks)(front_v, 1);
    /* One CargoPayment per vehicle and the vehicle limit equals the
     * limit in number of CargoPayments. Can't go wrong. */
    let payment = (l.create_payment)(front_v);
    let mut body = |next_station| {
        if !has_orders(front_v)
            || OrderUnloadType((l.unload_type)(front_v)) != OrderUnloadType::NoUnload
        {
            let mut current = Some(front_v);
            while let Some(v) = current {
                let cargo = l.cargo_list(v);
                if u32::from(v.cargo_cap()) > 0 && cargo.total_count() > 0 {
                    (l.stage)(
                        curr_station.native,
                        v,
                        (l.unload_type)(front_v),
                        next_station,
                        payment,
                    );
                    if cargo.unload_count() > 0 {
                        (l.set_cargo_unloading)(v, true);
                    }
                }
                current = v.next();
            }
        }
    };
    let (context, callback) = scope_context(&mut body);
    (l.with_next_stations)(front_v, context, callback);
}
/// `UpdateLoadUnloadTicks`: update the vehicle's `load_unload_ticks`, the time it will wait
/// until it tries to load or unload again. Adjust for overhang of trains and set it at least
/// to 1.
fn update_load_unload_ticks(
    front: VehicleRef,
    st: LoadingStation,
    mut ticks: i32,
    w: &World,
    l: &Loading,
) {
    if VehicleType(front.vehicle_type()) == VehicleType::TRAIN && (l.station_length_penalty)() {
        /* Each platform tile is worth 2 rail vehicles. */
        let cached_length = u32::from(crate::raw_field::read_field!(
            front.ground_cache(),
            cached_total_length
        ));
        let tile = front.tile();
        // SAFETY: The loading entry keeps the immutable shared services live.
        let world = crate::map_func::World::new(unsafe { &*w.shared });
        let length = crate::follow_track::platform_length_full(&world, st.base, tile);
        let overhang = cached_length.wrapping_sub(length.wrapping_mul(TILE_SIZE)) as i32;
        if overhang > 0 {
            ticks = ticks.wrapping_shl(1);
            ticks = ticks.wrapping_add(overhang.wrapping_mul(ticks) / 8);
        }
    }
    /* Always wait at least 1, otherwise we'll wait 'infinitively' long. */
    (l.set_load_ticks)(front, ticks.max(1) as u16);
}
/// `LoadUnloadVehicle`: loads/unload the vehicle if possible.
// Preserve complete loading control at the source observation points.
#[allow(clippy::too_many_lines)]
unsafe fn load_unload_vehicle(front: VehicleRef, c: &LoadingContext<'_>) {
    let (w, l) = (c.w, c.l);
    let map = crate::map_func::World::new(unsafe { &*w.shared });
    if map.types.assertions() {
        map.check(front.current_order().kind() == crate::order_type::OrderType::OT_LOADING.0);
    }

    // SAFETY: The immutable shared vehicle table outlives this entry.
    let vehicle = unsafe { &*l.vehicle };
    let st = (w.station)(front.last_station());
    let mut body = |next_station| {
        let control = (l.control)(front);
        let use_autorefit =
            control.refit && CargoType(control.refit_cargo) == CargoType::CARGO_AUTO_REFIT;
        let mut consist_capleft: CargoArray = [0; NUM_CARGO as usize];
        if if control.improved && use_autorefit {
            (l.get_payment)(front).is_null()
        } else {
            control.full_load
        } {
            if control.full_load {
                crate::witness::hit(crate::witness::STATION_FULL_LOAD_RESERVE);
            }
            reserve_consist(
                st,
                front,
                &control,
                if use_autorefit && control.ticks != 0 {
                    Some(&mut consist_capleft)
                } else {
                    None
                },
                next_station,
                w,
                l,
            );
        }
        /* We have not waited enough time till the next round of loading/unloading */
        if control.ticks != 0 {
            return;
        }
        if VehicleType(front.vehicle_type()) == VehicleType::TRAIN && {
            let tile = front.tile();
            let shared = unsafe { &*w.shared };
            (shared.tile_type)(tile) != crate::vehicle::MP_STATION
                || (shared.station_index)(tile) != (w.graphs.station_id)(st.base)
        } {
            /* The train reversed in the station. Take the "easy" way
             * out and let the train just leave as it always did. */
            (l.set_loading_finished)(front, true);
            (l.set_load_ticks)(front, 1);
            return;
        }
        let state = st.state;
        let mut new_load_unload_ticks = 0_i32;
        let mut dirty_vehicle = false;
        let mut dirty_station = false;
        let mut completely_emptied = true;
        let mut anything_unloaded = false;
        let mut anything_loaded = false;
        let mut full_load_amount = 0_u64;
        let mut cargo_not_full = 0_u64;
        let mut cargo_full = 0_u64;
        let mut reservation_left = 0_u64;
        front.write_speed(0);
        let payment = (l.get_payment)(front);
        let mut artic_part = 0_u32; // Articulated part we are currently trying to load. (not counting parts without capacity)
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
            let v_cargo = l.cargo_list(v);
            let mut cargo = CargoType(part.cargo);
            let mut ge = unsafe { cp(state, cargo) };
            if part.unloading && OrderUnloadType(control.unload_type) != OrderUnloadType::NoUnload {
                let cargo_count = v_cargo.unload_count();
                let mut amount_unloaded = if control.gradual {
                    if cargo_count > 0 {
                        crate::witness::hit(crate::witness::STATION_GRADUAL_UNLOAD);
                    }
                    cargo_count.min(get_load_amount(v, control.gradual, l, unsafe {
                        &*w.shared
                    }))
                } else {
                    cargo_count
                };
                if !unsafe { goods_status(ge) }.test(GoodsState::Acceptance)
                    && v_cargo.action_count(MoveToAction::MTA_DELIVER) > 0
                {
                    crate::witness::hit(crate::witness::STATION_REJECTED_DELIVERY);
                    /* The station does not accept our goods anymore. */
                    let unload_type = OrderUnloadType(control.unload_type);
                    if unload_type == OrderUnloadType::Transfer
                        || unload_type == OrderUnloadType::Unload
                    {
                        /* Transfer instead of delivering. */
                        v_cargo.reassign(
                            l,
                            MoveToAction::MTA_DELIVER,
                            MoveToAction::MTA_TRANSFER,
                            v_cargo.action_count(MoveToAction::MTA_DELIVER),
                        );
                    } else {
                        let new_remaining = v_cargo
                            .remaining_count()
                            .wrapping_add(v_cargo.action_count(MoveToAction::MTA_DELIVER));
                        let cargo_cap = u32::from(v.cargo_cap());
                        if cargo_cap < new_remaining {
                            /* Return some of the reserved cargo to not overload the vehicle. */
                            (l.return_cargo)(st.native, v, new_remaining - cargo_cap);
                        }
                        /* Keep instead of delivering. This may lead to no cargo being unloaded, so ...*/
                        v_cargo.reassign(
                            l,
                            MoveToAction::MTA_DELIVER,
                            MoveToAction::MTA_KEEP,
                            v_cargo.action_count(MoveToAction::MTA_DELIVER),
                        );
                        /* ... say we unloaded something, otherwise we'll think we didn't unload
                         * something and we didn't load something, so we must be finished
                         * at this station. Setting the unloaded means that we will get a
                         * retry for loading in the next cycle. */
                        anything_unloaded = true;
                    }
                }
                if v_cargo.action_count(MoveToAction::MTA_TRANSFER) > 0 {
                    /* Mark the station dirty if we transfer, but not if we only deliver. */
                    dirty_station = true;
                    if !goods_entry::has_rating(unsafe { goods_status(ge) }) {
                        /* Upon transferring cargo, make sure the station has a rating. Fake a pickup for the
                         * first unload to prevent the cargo from quickly decaying after the initial drop. */
                        unsafe {
                            (&raw mut (*ge).time_since_pickup).write(0);
                            let mut status = goods_status(ge);
                            status.set(GoodsState::Rating);
                            set_goods_status(ge, status);
                        }
                    }
                }
                crate::witness::hit(crate::witness::STATION_UNLOAD_CARGO);
                if map.types.assertions() {
                    map.check(!payment.is_null());
                }
                amount_unloaded = (l.unload_cargo)(st.native, v, amount_unloaded, payment);
                let remaining = v_cargo.unload_count() > 0;
                if amount_unloaded > 0 {
                    dirty_vehicle = true;
                    anything_unloaded = true;
                    new_load_unload_ticks =
                        new_load_unload_ticks.wrapping_add(amount_unloaded as i32);
                    /* Deliver goods to the station */
                    unsafe {
                        (&raw mut (*state).fields.time_since_unload).write(0);
                    }
                }
                if control.gradual && remaining {
                    completely_emptied = false;
                } else {
                    /* We have finished unloading (cargo count == 0) */
                    (l.set_cargo_unloading)(v, false);
                }
                current = v.next();
                continue;
            }
            /* Do not pick up goods when we have no-load set or loading is stopped. */
            if OrderLoadType(control.load_type) == OrderLoadType::NoLoad || control.stop {
                current = v.next();
                continue;
            }
            /* This order has a refit, if this is the first vehicle part carrying cargo and the whole vehicle is empty, try refitting. */
            if control.refit && artic_part == 1 {
                handle_station_refit(
                    v,
                    &mut consist_capleft,
                    st,
                    next_station,
                    CargoType(control.refit_cargo),
                    c,
                );
                cargo = CargoType((l.cargo_type)(v));
                ge = unsafe { cp(state, cargo) };
            }
            /* As we're loading here the following link can carry the full capacity of the vehicle. */
            v.write_refit_cap(v.cargo_cap());
            /* update stats */
            let t = match VehicleType(front.vehicle_type()) {
                VehicleType::TRAIN | VehicleType::SHIP => i32::from(front.max_speed()),
                VehicleType::ROAD => i32::from(front.max_speed()) / 2,
                VehicleType::AIRCRAFT => {
                    // Convert to old units.
                    crate::aircraft::speed_old_units(u32::from(front.max_speed())) as i32
                }
                _ => unreachable!("original NOT_REACHED vehicle type"),
            };
            /* if last speed is 0, we treat that as if no vehicle has ever visited the station. */
            unsafe {
                (&raw mut (*ge).last_speed).write(clamp_to_u8(i64::from(t)));
            }
            let resolver = unsafe { &*(vehicle.newgrf)() };
            let age = crate::timer_game_common::calendar_year()
                .0
                .wrapping_sub(unsafe { (resolver.build_year)(front.0.as_ptr()) });
            unsafe {
                (&raw mut (*ge).last_age).write(clamp_to_u8(i64::from(age)));
            }
            if map.types.assertions() {
                map.check(u32::from(v.cargo_cap()) >= v_cargo.stored_count());
            }
            /* Capacity available for loading more cargo. */
            let mut cap_left = u32::from(v.cargo_cap()).wrapping_sub(v_cargo.stored_count());
            if cap_left > 0 {
                /* If vehicle can load cargo, reset time_since_pickup. */
                unsafe {
                    (&raw mut (*ge).time_since_pickup).write(0);
                }
                /* If there's goods waiting at the station, and the vehicle
                 * has capacity for it, load it on the vehicle. */
                if (v_cargo.action_count(MoveToAction::MTA_LOAD) > 0
                    || (w.available)(st.base, cargo.0) > 0)
                    && may_load_under_exclusive_rights(st, v, w)
                {
                    if v_cargo.stored_count() == 0 {
                        unsafe {
                            crate::newgrf_vehicle::trigger(
                                vehicle,
                                v,
                                crate::newgrf_vehicle::Trigger::NewCargo,
                            );
                        };
                    }
                    if control.gradual {
                        cap_left = cap_left.min(get_load_amount(v, control.gradual, l, unsafe {
                            &*w.shared
                        }));
                    }
                    let loaded = (l.load_cargo)(st.native, v, cap_left, next_station);
                    if v_cargo.action_count(MoveToAction::MTA_LOAD) > 0 {
                        /* Remember if there are reservations left so that we don't stop
                         * loading before they're loaded. */
                        set_bit64(&mut reservation_left, cargo.0);
                    }
                    /* Store whether the maximum possible load amount was loaded or not.*/
                    if loaded == cap_left {
                        set_bit64(&mut full_load_amount, cargo.0);
                    } else {
                        clr_bit64(&mut full_load_amount, cargo.0);
                    }
                    /* TODO: Regarding this, when we do gradual loading, we
                     * should first unload all vehicles and then start
                     * loading them. Since this will cause
                     * VEHICLE_TRIGGER_EMPTY to be called at the time when
                     * the whole vehicle chain is really totally empty, the
                     * completely_emptied assignment can then be safely
                     * removed; that's how TTDPatch behaves too. --pasky */
                    if loaded > 0 {
                        completely_emptied = false;
                        anything_loaded = true;
                        unsafe {
                            (&raw mut (*state).fields.time_since_load).write(0);
                            (&raw mut (*state).fields.last_vehicle_type).write(v.vehicle_type());
                        }
                        if (l.station_cargo_total)(st.native, cargo.0) == 0 {
                            crate::animated_tile::cargo_event(
                                st.base,
                                cargo.0,
                                crate::station_type::SAT_CARGO_TAKEN,
                                crate::station_type::AAT_CARGO_TAKEN,
                                crate::station_type::SRT_CARGO_TAKEN,
                            );
                        }
                        new_load_unload_ticks = new_load_unload_ticks.wrapping_add(loaded as i32);
                        dirty_vehicle = true;
                        dirty_station = true;
                    }
                }
            }
            if v_cargo.stored_count() >= u32::from(v.cargo_cap()) {
                set_bit64(&mut cargo_full, cargo.0);
            } else {
                set_bit64(&mut cargo_not_full, cargo.0);
            }
            current = v.next();
        }
        if anything_loaded || anything_unloaded {
            if VehicleType(front.vehicle_type()) == VehicleType::TRAIN {
                crate::animated_tile::rail_event(
                    st.base,
                    front.tile(),
                    crate::station_type::SAT_VEHICLE_LOADS,
                    crate::station_type::SRT_VEHICLE_LOADS,
                );
            } else if VehicleType(front.vehicle_type()) == VehicleType::ROAD {
                crate::animated_tile::road_event(
                    st.base,
                    front.tile(),
                    crate::station_type::SAT_VEHICLE_LOADS,
                    crate::station_type::SRT_VEHICLE_LOADS,
                );
            }
        }
        /* Only set completely_emptied, if we just unloaded all remaining cargo */
        completely_emptied &= anything_unloaded;
        if !anything_unloaded {
            (l.delete_payment)(payment);
        }
        (l.set_stop)(front, false);
        if anything_loaded || anything_unloaded {
            if control.gradual {
                /* The time it takes to load one 'slice' of cargo or passengers depends
                 * on the vehicle type - the values here are those found in TTDPatch */
                const GRADUAL_LOADING_WAIT_TIME: [i32; 4] = [40, 20, 10, 20];
                new_load_unload_ticks =
                    GRADUAL_LOADING_WAIT_TIME[usize::from(front.vehicle_type())];
            }
            /* We loaded less cargo than possible for all cargo types and it's not full
             * load and we're not supposed to wait any longer: stop loading. */
            if !anything_unloaded
                && full_load_amount == 0
                && reservation_left == 0
                && !control.full_load
                && {
                    let observed = (l.wait_observation)(front);
                    front.order_time()
                        >= i32::from(observed.wait)
                            .wrapping_sub(observed.lateness)
                            .max(0)
                }
            {
                (l.set_stop)(front, true);
            }
            update_load_unload_ticks(front, st, new_load_unload_ticks, w, l);
        } else {
            update_load_unload_ticks(front, st, 20, w, l); // We need the ticks for link refreshing.
            let mut finished_loading = true;
            if control.full_load {
                if OrderLoadType(control.load_type) == OrderLoadType::FullLoadAny {
                    /* if the aircraft carries passengers and is NOT full, then
                     * continue loading, no matter how much mail is in */
                    if (VehicleType(front.vehicle_type()) == VehicleType::AIRCRAFT
                        && (l.passenger_cargo)(front)
                        && u32::from(front.cargo_cap()) > l.cargo_list(front).stored_count())
                        || (cargo_not_full != 0 && (cargo_full & !cargo_not_full) == 0)
                    // There are still non-full cargoes
                    {
                        finished_loading = false;
                    }
                } else if cargo_not_full != 0 {
                    finished_loading = false;
                }
                /* Refresh next hop stats if we're full loading to make the links
                 * known to the distribution algorithm and allow cargo to be sent
                 * along them. Otherwise the vehicle could wait for cargo
                 * indefinitely if it hasn't visited the other links yet, or if the
                 * links die while it's loading. */
                if !finished_loading {
                    refresh_links(front, true, true, c.visits);
                }
            }
            (l.set_loading_finished)(front, finished_loading);
        }
        /* Calculate the loading indicator fill percent and display
         * In the Game Menu do not display indicators
         * If _settings_client.gui.loading_indicators == 2, show indicators (bool can be promoted to int as 0 or 1 - results in 2 > 0,1 )
         * if _settings_client.gui.loading_indicators == 1, _local_company must be the owner or must be a spectator to show ind., so 1 > 0
         * if _settings_client.gui.loading_indicators == 0, do not display indicators ... 0 is never greater than anything
         */
        if (l.indicator)(front) {
            let mut percent_up_down = StringID::STR_NULL;
            let percent = crate::vehicle::visits::percent_filled(
                front,
                Some(&mut percent_up_down.0),
                c.visits,
            );
            (l.show_indicator)(front, percent, percent_up_down.0);
        }
        if completely_emptied {
            /* Make sure the vehicle is marked dirty, since we need to update the NewGRF
             * properties such as weight, power and TE whenever the trigger runs. */
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
/// `LoadUnloadStation`: load/unload the vehicles in this station according to the order
/// they entered.
// The empty-queue check stays inlined at the scheduler, before loading-table setup.
#[inline]
pub(crate) unsafe fn load_unload_station(
    state: *mut Service,
    world: *const World,
    loading: *const Loading,
    visits: *const Visits,
    out: *mut crate::services::CommandCost,
    temporary: *mut crate::services::CommandCost,
) {
    /* No vehicle is here... */
    // C++ supplies the queue owner directly: no callback precedes the empty
    // queue check, and the queue station is not the vehicles' loading station.
    let node = unsafe { loading_vehicles_begin(state) };
    if node.is_null() {
        return;
    }
    let refit = RefitContext {
        services: unsafe { &*visits }.refit,
        out,
        temporary,
    };
    let c = LoadingContext {
        w: unsafe { &*world },
        l: unsafe { &*loading },
        refit: &refit,
        visits: unsafe { &*visits },
    };
    // SAFETY: The first queue node and immutable tables are live for the same
    // synchronous load. The helper retains the original live-next traversal.
    unsafe { load_unload_station_queue(state, node, &c) };
}
/// Temporary adapter for `vehicle/scheduler.rs`, held by #853; delegates to
/// `load_unload_station`.
pub(crate) unsafe fn load_station(
    state: *mut Service,
    world: *const World,
    loading: *const Loading,
    visits: *const Visits,
    out: *mut crate::services::CommandCost,
    temporary: *mut crate::services::CommandCost,
) {
    unsafe { load_unload_station(state, world, loading, visits, out, temporary) };
}

/// `LoadUnloadStation` after its empty-queue check.
// Out of line so the empty-queue check above stays inlined at the scheduler.
#[inline(never)]
unsafe fn load_unload_station_queue(
    state: *mut Service,
    mut node: *mut QueueEntry,
    c: &LoadingContext<'_>,
) {
    let l = c.l;
    let map = crate::map_func::World::new(unsafe { &*c.w.shared });
    let mut last_loading = None;
    /* Check if anything will be loaded at all. Otherwise we don't need to reserve either. */
    while !node.is_null() {
        let v = unsafe { VehicleRef::from_shell(loading_vehicles_value(node).cast(), &*l.vehicle) };
        if v.status() & (VS_STOPPED | VS_CRASHED) == 0 {
            if map.types.assertions() {
                map.check((l.load_unload_ticks)(v) != 0);
            }
            let ticks = (l.load_unload_ticks)(v).wrapping_sub(1);
            (l.set_load_ticks)(v, ticks);
            if ticks == 0 {
                last_loading = Some(v);
            }
        }
        node = unsafe { loading_vehicles_next(node) };
    }
    /* We only need to reserve and load/unload up to the last loading vehicle.
     * Anything else will be forgotten anyway after returning from this function.
     *
     * Especially this means we do _not_ need to reserve cargo for a single
     * consist in a station which is not allowed to load yet because its
     * load_unload_ticks is still not 0.
     */
    if last_loading.is_none() {
        return;
    }
    node = unsafe { loading_vehicles_begin(state) };
    while !node.is_null() {
        let v = unsafe { VehicleRef::from_shell(loading_vehicles_value(node).cast(), &*l.vehicle) };
        if v.status() & (VS_STOPPED | VS_CRASHED) == 0 {
            unsafe {
                load_unload_vehicle(v, c);
            }
        }
        if Some(v) == last_loading {
            break;
        }
        node = unsafe { loading_vehicles_next(node) };
    }
    /* Call the production machinery of industries */
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
    unsafe { load_unload_station(state, world, loading, visits, out, temporary) };
}

/// `ModifyStationRatingAround`'s per-station update: change the rating of every goods entry
/// with any status by `amount`.
pub(crate) unsafe fn modify_rating(state: *mut Service, amount: i32) {
    for cargo in (0..NUM_CARGO).map(CargoType) {
        let ge = unsafe { cp(state, cargo) };
        if unsafe { goods_status(ge) }.any() {
            unsafe {
                (&raw mut (*ge).rating)
                    .write(clamp_to_u8(i64::from((*ge).rating) + i64::from(amount)));
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
        nearest_depot,
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
        is_station,
        town_statue,
        rating_cheat,
        available,
        cargo_valid,
        rating_callback_enabled,
        passenger_cargo,
        packet_available,
        rating_redraw,
        acceptance_news,
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
