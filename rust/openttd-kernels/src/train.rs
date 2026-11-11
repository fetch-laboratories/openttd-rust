/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete train control. Canonical private fields are accessed only for one scalar
//! operation; no Rust reference into an owner or world object crosses a callback.
#![warn(clippy::undocumented_unsafe_blocks)]
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::if_not_else,
    clippy::collapsible_if,
    clippy::verbose_bit_mask
)]
use crate::direction_type::DiagDirection;
use crate::direction_type::{DirDiff, Direction};
use crate::order_type::{
    OT_CONDITIONAL, OT_GOTO_DEPOT, OT_GOTO_STATION, OT_GOTO_WAYPOINT, OT_LEAVESTATION, OT_LOADING,
    OT_NOTHING, OrderStopLocation, OrderType,
};
use crate::raw_field::{read_field, write_field};
use crate::services::Services;
use crate::settings_type::SettingAddresses;
use crate::station_type::StationHadVehicleOfType;
const HVOT_TRAIN: StationHadVehicleOfType = StationHadVehicleOfType::HVOT_TRAIN;
use crate::station_type::StationID;
use crate::tile_map::is_tile_type;
use crate::tile_type::{TILE_SIZE, TileIndex};
use crate::track_type::TrackBits;
use crate::train_state::State;
use crate::vehicle::{VehicleRef, VehicleServices, maintenance};
use crate::vehicle_base::{VehState, VehStates};
use crate::vehicle_type::{AccelerationModel, VEHICLE_LENGTH};
/// Native identity and canonical scalar owner, resolved together once per entry or link.
/// Neither pointer permits a Rust borrow to survive a callback. Deletion invalidates both.
/// Each entry keeps `owner` live and serialized for the `raw_field` accesses below.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Handle {
    pub record: *mut crate::vehicle::OpenTTDVehicle,
    pub owner: *mut State,
}
impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        self.record == other.record
    }
}
impl Eq for Handle {}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Depot {
    pub tile: u32,
    pub length: u32,
}
impl Handle {
    /// Canonical record carried by this live train handle.
    pub(crate) fn vehicle(self) -> VehicleRef {
        // SAFETY: Entries and topology reads supply a live non-null train record.
        unsafe { VehicleRef(std::ptr::NonNull::new_unchecked(self.record)) }
    }
    pub(crate) fn from_record(record: *mut crate::vehicle::OpenTTDVehicle) -> Self {
        if record.is_null() {
            Self::NONE
        } else {
            Self {
                record,
                owner: read_field!(record, train),
            }
        }
    }
    pub(crate) fn next(self) -> Self {
        Self::from_record(read_field!(self.record, next))
    }
    pub(crate) fn previous(self) -> Self {
        Self::from_record(read_field!(self.record, previous))
    }
    pub(crate) fn first(self) -> Self {
        Self::from_record(self.vehicle().first().0.as_ptr())
    }
    pub(crate) fn last(self) -> Self {
        Self::from_record(self.vehicle().last().0.as_ptr())
    }
    pub(crate) fn next_vehicle(self) -> Self {
        self.vehicle()
            .next_vehicle()
            .map_or(Self::NONE, |v| Self::from_record(v.0.as_ptr()))
    }
    pub(crate) fn front(self) -> u8 {
        u8::from(crate::vehicle_base::is_front_engine(
            crate::vehicle_base::is_ground_vehicle(self.vehicle().vehicle_type()),
            self.vehicle().subtype(),
        ))
    }
    pub(crate) fn articulated(self) -> u8 {
        u8::from(crate::vehicle_base::is_articulated_part(
            crate::vehicle_base::is_ground_vehicle(self.vehicle().vehicle_type()),
            self.vehicle().subtype(),
        ))
    }
    pub(crate) fn multiheaded(self) -> u8 {
        u8::from(crate::ground_vehicle::is_multiheaded(
            self.vehicle().subtype(),
        ))
    }
    pub(crate) fn shell(self) -> *mut std::ffi::c_void {
        VehicleRef::shell_from_record(self.record).cast()
    }
    pub(crate) const NONE: Self = Self {
        record: std::ptr::null_mut(),
        owner: std::ptr::null_mut(),
    };
}
impl Default for Handle {
    fn default() -> Self {
        Self::NONE
    }
}
impl Handle {
    pub(crate) fn set_suppress_implicit(self, value: bool) {
        let mask = 1 << crate::ground_vehicle::GVF_SUPPRESS_IMPLICIT_ORDERS;
        let flags = read_field!(self.vehicle().ground(), flags);
        write_field!(
            self.vehicle().ground(),
            flags,
            if value { flags | mask } else { flags & !mask }
        );
    }
    pub(crate) fn clear_stuck(self) {
        self.set_flags(self.get_flags() & !(1 << 8));
    }
    pub(crate) fn stuck(self) -> bool {
        self.get_flags() & (1 << 8) != 0
    }

    pub(crate) fn track(self) -> u8 {
        self.get_track() as u8
    }
    pub(crate) fn wait_inc(self) -> u16 {
        let next = (self.get_wait_counter() as u16).wrapping_add(1);
        self.set_wait_counter(u64::from(next));
        next
    }

    fn flag(self, bit: u8) -> bool {
        self.get_flags() & (1 << bit) != 0
    }

    pub(crate) fn get_flags(self) -> u64 {
        u64::from(read_field!(self.owner, flags))
    }
    pub(crate) fn set_flags(self, value: u64) {
        write_field!(self.owner, flags, value as u16);
    }
    pub(crate) fn get_crash_anim_pos(self) -> u64 {
        u64::from(read_field!(self.owner, crash_anim_pos))
    }
    pub(crate) fn set_crash_anim_pos(self, value: u64) {
        write_field!(self.owner, crash_anim_pos, value as u16);
    }
    pub(crate) fn get_wait_counter(self) -> u64 {
        u64::from(read_field!(self.owner, wait_counter))
    }
    pub(crate) fn set_wait_counter(self, value: u64) {
        write_field!(self.owner, wait_counter, value as u16);
    }
    pub(crate) fn get_compatible_railtypes(self) -> u64 {
        read_field!(self.owner, compatible_railtypes)
    }
    pub(crate) fn set_compatible_railtypes(self, value: u64) {
        write_field!(self.owner, compatible_railtypes, value);
    }
    pub(crate) fn get_railtypes(self) -> u64 {
        read_field!(self.owner, railtypes)
    }
    pub(crate) fn set_railtypes(self, value: u64) {
        write_field!(self.owner, railtypes, value);
    }
    pub(crate) fn get_track(self) -> u64 {
        u64::from(read_field!(self.owner, track))
    }
    pub(crate) fn set_track(self, value: u64) {
        write_field!(self.owner, track, value as u8);
    }
    pub(crate) fn get_force_proceed(self) -> u64 {
        u64::from(read_field!(self.owner, force_proceed))
    }
    pub(crate) fn set_force_proceed(self, value: u64) {
        write_field!(self.owner, force_proceed, value as u8);
    }
    pub(crate) fn get_cached_tilt(self) -> u64 {
        u64::from(read_field!(self.owner, cached_tilt))
    }
    pub(crate) fn set_cached_tilt(self, value: u64) {
        write_field!(self.owner, cached_tilt, value != 0);
    }
    pub(crate) fn get_user_def_data(self) -> u64 {
        u64::from(read_field!(self.owner, user_def_data))
    }
    pub(crate) fn set_user_def_data(self, value: u64) {
        write_field!(self.owner, user_def_data, value as u8);
    }
    pub(crate) fn get_cached_curve_speed_mod(self) -> u64 {
        u64::from(read_field!(self.owner, cached_curve_speed_mod) as u16)
    }
    pub(crate) fn set_cached_curve_speed_mod(self, value: u64) {
        write_field!(self.owner, cached_curve_speed_mod, value as i16);
    }
    pub(crate) fn get_cached_max_curve_speed(self) -> u64 {
        u64::from(read_field!(self.owner, cached_max_curve_speed))
    }
    pub(crate) fn set_cached_max_curve_speed(self, value: u64) {
        write_field!(self.owner, cached_max_curve_speed, value as u16);
    }
    pub(crate) fn set_flag(self, bit: u8, value: bool) {
        let mask = 1 << bit;
        let old = self.get_flags();
        self.set_flags(if value { old | mask } else { old & !mask });
    }
}
/// Native `VehicleRailFlag::Flipped`.
pub(crate) const FLIPPED: u8 = 4;
/// `CCF_ARRANGE`: `ConsistChangeFlag::Length` and `ConsistChangeFlag::Capacity`.
pub(crate) const CCF_ARRANGE: u8 = 3;
/// `CCF_LOADUNLOAD`: no length or capacity property changes.
pub(crate) const CCF_LOADUNLOAD: u8 = 0;
/// `CCF_TRACK`: no length or capacity changes.
pub(crate) const CCF_TRACK: u8 = 0;
use crate::tile_type::INVALID_TILE;
use crate::track_type::{TRACK_BIT_DEPOT as DEPOT, TRACK_BIT_WORMHOLE as WORMHOLE};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    pub physics: &'static crate::ground_vehicle::Services,

    pub write_colourmap: extern "C" fn(Handle, u64),

    pub advance_distance: extern "C" fn(Handle) -> u64,
    pub all_powered: extern "C" fn(Handle, u64) -> u64,
    pub arrival_news: extern "C" fn(Handle, u16),

    pub axis_diag: extern "C" fn(Handle, u8) -> u64,
    pub backoff: extern "C" fn() -> u8,
    pub base_viewport: extern "C" fn(Handle, u8),
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    pub capacity_error: extern "C" fn(Handle),
    pub check_next: extern "C" fn(Handle),
    pub check_orders: extern "C" fn(Handle),
    pub check_reverse: extern "C" fn(Handle) -> u64,
    pub choose_track: extern "C" fn(Handle, u32, u8, u8) -> u64,
    pub clear_reservation: extern "C" fn(Handle, u32, u8),
    pub consist_windows: extern "C" fn(Handle),
    pub cost_divisor: extern "C" fn(Handle) -> u64,
    pub count_chain: extern "C" fn(Handle) -> u64,
    pub crash_event: extern "C" fn(Handle, u32),
    pub crash_news: extern "C" fn(Handle, u32),
    pub crash_sound: extern "C" fn(Handle),
    pub crossing_barred: extern "C" fn(Handle, u32) -> u64,
    pub crossing_rail_axis: extern "C" fn(Handle, u32) -> u64,
    pub crossing_road_axis: extern "C" fn(Handle, u32) -> u64,
    pub crossing_sound: extern "C" fn(Handle, u32),
    pub curve_advantage: extern "C" fn(Handle) -> u64,
    pub day_ticks: extern "C" fn(Handle) -> u64,
    pub delete_vehicle: extern "C" fn(Handle),
    pub depot_dir: extern "C" fn(Handle, u32) -> u64,
    pub depot_dirty: extern "C" fn(Handle),
    pub depot_track: extern "C" fn(Handle, u32) -> u64,
    pub depot_window: extern "C" fn(Handle, u32),
    pub diag_axis: extern "C" fn(Handle, u8) -> u64,
    pub diag_between: extern "C" fn(Handle, u32, u32) -> u64,
    pub diag_reaches_tracks: extern "C" fn(Handle, u8) -> u64,
    pub diag_trackdir: extern "C" fn(Handle, u8) -> u64,
    pub dir_diag: extern "C" fn(Handle, u8) -> u64,
    pub disconnect: extern "C" fn(Handle),
    pub find_depot: extern "C" fn(Handle, i32) -> Depot,
    pub first_track: extern "C" fn(Handle, u8) -> u64,
    pub has_depot_res: extern "C" fn(Handle, u32) -> u64,
    pub has_signal: extern "C" fn(Handle, u32, u8) -> u64,
    pub has_signals: extern "C" fn(Handle, u32) -> u64,
    pub has_signal_td: extern "C" fn(Handle, u32, u8) -> u64,
    pub hide_fill: extern "C" fn(Handle),
    pub invalidate_grf: extern "C" fn(Handle),
    pub is_bridge: extern "C" fn(Handle, u32) -> u64,
    pub is_crossing: extern "C" fn(Handle, u32) -> u64,
    pub is_depot: extern "C" fn(Handle, u32) -> u64,
    pub is_plain_rail: extern "C" fn(Handle, u32) -> u64,
    pub is_station: extern "C" fn(Handle, u32) -> u64,
    pub is_station_any: extern "C" fn(Handle, u32) -> u64,
    pub leave_sound: extern "C" fn(Handle),
    pub leave_unbunch: extern "C" fn(Handle),
    pub local_company: extern "C" fn(Handle) -> u64,
    pub no_90: extern "C" fn(Handle, u8, u8) -> u64,
    pub oneway_blocking: extern "C" fn(Handle, u32, u8) -> u64,
    pub order_depot_service: extern "C" fn(Handle, u16),
    pub order_dummy: extern "C" fn(Handle),
    pub order_free: extern "C" fn(Handle),
    pub order_max_speed: extern "C" fn(Handle) -> u64,
    pub other_end: extern "C" fn(Handle, u32) -> u64,
    pub pay_running: extern "C" fn(Handle, i64),
    pub pbs_signal_type: extern "C" fn(Handle) -> u64,
    pub rail_type: extern "C" fn(Handle, u32) -> u64,
    pub reserve_paths: extern "C" fn() -> bool,
    pub reserve_under: extern "C" fn(Handle),
    pub reset_unbunch: extern "C" fn(Handle),
    pub reverse_windows: extern "C" fn(Handle),
    pub running_windows: extern "C" fn(Handle),
    pub servint: extern "C" fn(Handle) -> u64,
    pub set_depot_res: extern "C" fn(Handle, u32, u8),
    pub set_signal_state: extern "C" fn(Handle, u32, u8, u8),
    pub set_tunnel_res: extern "C" fn(Handle, u32, u8),
    pub show_reservation: extern "C" fn(Handle) -> u64,
    pub signal_context: extern "C" fn() -> crate::signal::Context,
    pub signal_has_pbs: extern "C" fn(Handle, u32, u8) -> u64,
    pub signal_pbs: extern "C" fn(Handle, u8) -> u64,
    pub signal_type: extern "C" fn(Handle, u32, u8) -> u64,
    pub sigseg_full: extern "C" fn(Handle) -> u64,
    pub sigseg_pbs: extern "C" fn(Handle) -> u64,
    pub start_stop_dirty: extern "C" fn(Handle),
    pub station_axis: extern "C" fn(Handle, u32) -> u64,
    pub station_compatible: extern "C" fn(Handle, u32, u32) -> u64,
    pub station_dest: extern "C" fn(Handle) -> u64,
    pub stopped_in_depot: extern "C" fn(Handle) -> u64,
    pub stop_location: extern "C" fn(Handle) -> u64,
    pub stuck_news: extern "C" fn(Handle),
    pub suppress_implicit: extern "C" fn(Handle),
    pub tile_offset_axis: extern "C" fn(Handle, u8) -> u64,
    pub tile_offset_diag: extern "C" fn(Handle, u8) -> u64,
    pub tile_rail_type: extern "C" fn(Handle, u32) -> u64,
    pub tile_virt: extern "C" fn(i32, i32) -> u32,
    pub trackdir_exit: extern "C" fn(Handle, u8) -> u64,
    pub trackdir_reaches: extern "C" fn(Handle, u8) -> u64,
    pub track_bits: extern "C" fn(Handle, u32) -> u64,
    pub track_crosses: extern "C" fn(Handle, u8) -> u64,
    pub track_direction: extern "C" fn(Handle, u8, u8) -> u64,
    pub train_list: extern "C" fn(Handle),
    pub truncate_cargo: extern "C" fn(Handle, u32),
    pub veh_exit_dir: extern "C" fn(Handle, u8, u8) -> u64,
    pub viewport: extern "C" fn(Handle, u8, u8),
    pub wagon_override: extern "C" fn(Handle) -> u64,
    pub wagon_speed_limits: extern "C" fn(Handle) -> u64,
    pub wait_unbunch: extern "C" fn(Handle) -> u64,
    pub write_crossing_bar: extern "C" fn(Handle, u32, u8),

    /// Base vehicle table, immutable for the process.
    pub vehicle: *const VehicleServices,
}
/// The `Game` view of an entry's `leaves` and `services` tables.
macro_rules! entry_game {
    ($leaves:expr, $services:expr) => {
        // SAFETY: The entry's contract keeps both immutable tables live until it returns.
        Game::new(unsafe { &*$leaves }, unsafe { &*$services })
    };
}
/// `Train::UpdateAcceleration`: update acceleration of the train from the cached power and
/// weight.
pub(crate) fn update_acceleration(id: Handle) {
    let power: u32 = read_field!(id.vehicle().ground_cache(), cached_power);
    let weight: u32 = read_field!(id.vehicle().ground_cache(), cached_weight);
    // Clamp(int, int, int): the uint product converts to int first.
    let accel = ((power / weight).wrapping_mul(4) as i32).clamp(1, 255);
    write_field!((id).record, acceleration, accel as u8);
}
#[derive(Clone)]
struct Game<'a> {
    leaves: &'a Leaves,
    services: &'a Services,
    movement: crate::movement::CachedTable<'a>,
}
// Canonical settings, loaded in place from the shared catalog at each observation.
impl Game<'_> {
    fn setting<T: Copy + Into<u64>>(&self, field: fn(&SettingAddresses) -> *const T) -> u64 {
        // SAFETY: The catalog addresses live process-wide settings; one field-sized load.
        unsafe { field(self.services.settings()).read() }.into()
    }
    /// `_settings_game.vehicle.train_acceleration_model`.
    fn acc_model(&self) -> u64 {
        self.setting(|s| s.train_acceleration_model)
    }
    /// `_settings_client.gui.lost_vehicle_warn`.
    fn lost_warn(&self) -> u64 {
        self.setting(|s| s.lost_vehicle_warn)
    }
    /// `_settings_game.pf.yapf.maximum_go_to_depot_penalty`.
    fn max_depot_penalty(&self) -> u64 {
        self.setting(|s| s.maximum_go_to_depot_penalty)
    }
    /// `_settings_game.pf.reverse_at_signals`.
    fn reverse_at_signals(&self) -> u64 {
        self.setting(|s| s.reverse_at_signals)
    }
    /// `_settings_game.pf.wait_oneway_signal`.
    fn wait_oneway(&self) -> u64 {
        self.setting(|s| s.wait_oneway_signal)
    }
    /// `_settings_game.pf.wait_for_pbs_path`.
    fn wait_pbs(&self) -> u64 {
        self.setting(|s| s.wait_for_pbs_path)
    }
    /// `_settings_game.pf.wait_twoway_signal`.
    fn wait_twoway(&self) -> u64 {
        self.setting(|s| s.wait_twoway_signal)
    }
}
impl<'a> Game<'a> {
    fn new(leaves: &'a Leaves, services: &'a Services) -> Self {
        Self {
            leaves,
            services,
            movement: crate::movement::CachedTable::new(services),
        }
    }
    /// Raw map access over the shared table's canonical allocation.
    fn world(&self) -> crate::map_func::World<'a> {
        crate::map_func::World::new(self.services)
    }
    fn vs(&self) -> &VehicleServices {
        // SAFETY: The native table names the immutable process-wide base table.
        unsafe { &*self.leaves.vehicle }
    }
    fn engine(&self, id: Handle) -> crate::vehicle::EngineRef {
        (self.vs().engine)(id.vehicle())
    }
    /// Run one `vehicle::maintenance` entry on this entry's shell.
    /// # Safety
    /// The caller must satisfy `entry`'s preconditions for this shell and table.
    unsafe fn maintain<R>(
        &self,
        id: Handle,
        entry: unsafe fn(VehicleRef, &VehicleServices) -> R,
    ) -> R {
        // SAFETY: The caller meets the maintenance entry contract.
        unsafe { self.vs().maintain(id.vehicle(), entry) }
    }
    /// `TryPathReserve`.
    fn try_path_reserve(&self, id: Handle, mark_as_reserved: bool, first_tile_okay: bool) -> bool {
        // SAFETY: id.record identifies the live Train for this serialized entry; no owner borrow spans the call.
        unsafe {
            (self.services.try_path_reserve)(
                id.vehicle().shell().cast(),
                mark_as_reserved,
                first_tile_okay,
            )
        }
    }
    /// `FreeTrainTrackReservation`.
    fn free_train_reservation(&self, id: Handle) {
        // SAFETY: id.record identifies the live Train for this serialized entry; no owner borrow spans the call.
        unsafe { (self.services.free_train_reservation)(id.vehicle().shell().cast()) };
    }
    /// `UpdateVehicleTileHash` of this shell, which stays in place.
    fn update_occupancy(&self, id: Handle) {
        // SAFETY: The controller retains this live shell through the index update.
        unsafe { crate::vehicle::occupancy::update(self.vs(), id.vehicle(), false) };
    }
    fn handle_breakdown(&self, id: Handle) -> bool {
        // SAFETY: The live train and unborrowed owner satisfy `vehicle_handle_breakdown`.
        unsafe { self.maintain(id, maintenance::vehicle_handle_breakdown) }
    }
    fn needs_automatic_servicing(&self, id: Handle) -> bool {
        // SAFETY: The live train and unborrowed owner satisfy `needs_automatic_servicing`.
        unsafe { self.maintain(id, maintenance::needs_automatic_servicing) }
    }
    fn service_in_depot(&self, id: Handle) {
        // SAFETY: The live train and unborrowed owner satisfy `service_in_depot`.
        unsafe { self.maintain(id, maintenance::service_in_depot) };
    }
    /// `SetWindowDirty(WC_VEHICLE_VIEW, v->index)`.
    fn view_dirty(&self, id: Handle) {
        // SAFETY: The base table names its immutable maintenance table.
        (unsafe { &*self.vs().maintenance }.view_dirty)(id.vehicle());
    }
    fn visit_tile<F: FnMut(Handle) -> bool>(&self, tile: u32, mut visitor: F) -> bool {
        let vs = self.vs();
        vs.any_on_tile(tile, |v| {
            v.vehicle_type() == crate::vehicle::VehicleType::TRAIN.0
                && visitor((vs.train_handle)(v.0.as_ptr()))
        })
    }
    fn visit_near<F: FnMut(Handle) -> bool>(&self, x: i32, y: i32, mut visitor: F) -> bool {
        let vs = self.vs();
        vs.any_near(x, y, 7, |v| {
            v.vehicle_type() == crate::vehicle::VehicleType::TRAIN.0
                && visitor((vs.train_handle)(v.0.as_ptr()))
        })
    }

    fn signal_context(&self) -> crate::signal::Context {
        crate::signal::acquire_context(self.leaves.signal_context)
    }
    fn signal_segment(&self, id: Handle, tile: u32, side: u8) -> u64 {
        let owner = (id).vehicle().owner();
        crate::signal::count_context_owner_read();
        u64::from(self.signal_context().segment(tile, side, owner))
    }
}
impl Game<'_> {
    fn property(&self, id: Handle, prop: u32, default: u64) -> u64 {
        (self.vs().property)(
            (self.vs().newgrf)(),
            id.vehicle(),
            prop as u8,
            default as i32,
        ) as u64
    }
    fn consist_changed(&self, id: Handle, allowed: u8) {
        let mut max_speed = u16::MAX;
        let head_engine: u16 = id.vehicle().engine_id();
        let head_front: u8 = id.front();
        let mut first_engine = if head_front != 0 {
            head_engine
        } else {
            u16::MAX
        };
        write_field!((id).vehicle().ground_cache(), cached_total_length, 0_u16);
        id.set_compatible_railtypes(0);
        let mut tilt = true;
        let mut min_curve_mod = i16::MAX;
        let mut u = id;
        while u != Handle::NONE {
            let part_engine: u16 = u.vehicle().engine_id();
            let part_engine_part: u8 =
                u8::from(crate::ground_vehicle::is_engine(u.vehicle().subtype()));
            write_field!(
                (u).vehicle().ground_cache(),
                first_engine,
                (u64::from(if u == id { u16::MAX } else { first_engine })) as u16
            );
            u.set_railtypes(self.engine(u).railtypes());
            if part_engine_part != 0 {
                first_engine = part_engine;
            }
            u.set_user_def_data(u64::from(self.engine(u).train_user_data()));
            (self.leaves.invalidate_grf)(id);
            (self.leaves.invalidate_grf)(u);
            u = (u).next();
        }
        u = id;
        while u != Handle::NONE {
            u.set_user_def_data(self.property(
                u,
                u32::from(crate::newgrf_properties::PROP_TRAIN_USER_DATA),
                u.get_user_def_data(),
            ));
            (self.leaves.invalidate_grf)(id);
            (self.leaves.invalidate_grf)(u);
            u = (u).next();
        }
        u = id;
        while u != Handle::NONE {
            if self.engine(u).misc() & crate::engine_type::RAIL_TILTS == 0 {
                tilt = false;
            }
            // SAFETY: the current shell survives property evaluation; no owner borrow is held.
            min_curve_mod = min_curve_mod.min(unsafe {
                crate::newgrf_vehicle::signed_property(
                    self.vs(),
                    u.vehicle(),
                    crate::newgrf_properties::PROP_TRAIN_CURVE_SPEED_MOD,
                    i32::from(self.engine(u).train_curve_mod()),
                )
            } as i16);
            // SAFETY: live serialized shell; evaluation retains no owner reference.
            unsafe { crate::newgrf_vehicle::cache_override(self.vs(), u.vehicle()) };
            (self.leaves.write_colourmap)(u, 0);
            crate::vehicle_effect::openttd_rust_visual_update(u.vehicle(), true);
            let powered = self.engine(id).wagon_power() != 0
                && self.engine(u).rail_kind() == crate::vehicle::RailVehicleTypes::WAGON
                && (self.leaves.wagon_override)(u) != 0
                && (u).vehicle().visual_effect() & 0x80 == 0;
            u.set_flag(3, powered);
            if (u).articulated() == 0 {
                if u64::from(
                    self.engine(u)
                        .ground_power(crate::vehicle::VehicleType(u.vehicle().vehicle_type())),
                ) > 0
                {
                    id.set_compatible_railtypes(
                        id.get_compatible_railtypes()
                            | (self.leaves.all_powered)(u, u.get_railtypes()),
                    );
                }
                if u.flag(6) {
                    u.set_railtypes(u.get_railtypes() | 1);
                    u.set_compatible_railtypes(u.get_compatible_railtypes() | 1);
                }
                if (self.engine(u).rail_kind() != crate::vehicle::RailVehicleTypes::WAGON
                    || (self.leaves.wagon_speed_limits)(u) != 0)
                    && (self.leaves.wagon_override)(u) == 0
                {
                    let speed = self.property(
                        u,
                        u32::from(crate::newgrf_properties::PROP_TRAIN_SPEED),
                        u64::from(self.engine(u).train_speed()),
                    ) as u16;
                    if speed != 0 {
                        max_speed = max_speed.min(speed);
                    }
                }
            }
            // SAFETY: the current consist retains this shell during capacity callbacks.
            let cap = unsafe {
                crate::engine_capabilities::openttd_rust_engine_capacity(
                    self.leaves.capabilities,
                    (self.vs().engine)(u.vehicle()).0.as_ptr(),
                    u.vehicle().0.as_ptr(),
                    std::ptr::null_mut(),
                )
            } as u16;
            let part_cargo_cap: u16 = read_field!(u.record, cargo_cap);
            if allowed & 2 != 0 {
                if part_cargo_cap > cap {
                    (self.leaves.truncate_cargo)(u, u32::from(cap));
                }
                write_field!(
                    (u).record,
                    refit_cap,
                    (u64::from(cap.min(read_field!((u).record, refit_cap)))) as u16
                );
                (u).vehicle().write_cargo_cap((u64::from(cap)) as u16);
            } else if cap != part_cargo_cap {
                (self.leaves.capacity_error)(u);
            }
            write_field!(
                (u).record,
                cached_cargo_age_period,
                (self.property(
                    u,
                    u32::from(crate::newgrf_properties::PROP_TRAIN_CARGO_AGE_PERIOD),
                    u64::from(self.engine(u).cargo_age()),
                )) as u16
            );
            let mut length = u16::MAX;
            if self.engine(u).grf_version(self.services) >= 8 {
                length = self.property(
                    u,
                    u32::from(crate::newgrf_properties::PROP_TRAIN_SHORTEN_FACTOR),
                    u64::from(u16::MAX),
                ) as u16;
                if length != u16::MAX && length >= 8 {
                    let vs = self.vs();
                    vs.unknown_callback_result(
                        (vs.engine)(u.vehicle()),
                        crate::newgrf_callbacks::CBID_VEHICLE_LENGTH,
                        length,
                    );
                }
            } else if self.engine(u).callbacks() & crate::newgrf_callbacks::VEHICLE_LENGTH != 0 {
                length = (self.vs().callback)(
                    (self.vs().newgrf)(),
                    u.vehicle(),
                    crate::newgrf_callbacks::CBID_VEHICLE_LENGTH,
                );
            }
            if length == u16::MAX {
                length = u16::from(self.engine(u).train_shorten());
            }
            length = 8 - length.min(7);
            if allowed & 1 != 0 {
                write_field!(
                    (u).vehicle().ground_cache(),
                    cached_veh_length,
                    (u64::from(length)) as u8
                );
            } else if length
                != u16::from(read_field!((u).vehicle().ground_cache(), cached_veh_length))
            {
                (self.vs().length_changed)(u.vehicle());
            }
            write_field!(
                (id).vehicle().ground_cache(),
                cached_total_length,
                (u64::from(
                    read_field!((id).vehicle().ground_cache(), cached_total_length).wrapping_add(
                        u16::from(read_field!((u).vehicle().ground_cache(), cached_veh_length))
                    ),
                )) as u16
            );
            (self.leaves.invalidate_grf)(id);
            (self.leaves.invalidate_grf)(u);
            u = (u).next();
        }
        (id).vehicle()
            .write_max_speed((u64::from(max_speed)) as u16);
        id.set_cached_tilt(u64::from(tilt));
        id.set_cached_curve_speed_mod(u64::from(min_curve_mod as u16));
        id.set_cached_max_curve_speed(u64::from(self.get_curve_speed_limit(id)));
        crate::ground_vehicle::cargo_changed(self.leaves.physics, id.vehicle());
        if (id).front() != 0 {
            update_acceleration(id);
            (self.leaves.consist_windows)(id);
        }
    }
    /// Computes train speed limit caused by curves.
    /// Returns the imposed speed limit.
    fn get_curve_speed_limit(&self, id: Handle) -> u16 {
        let absolute_max_speed = i32::from(u16::MAX);
        let mut max_speed = absolute_max_speed;
        if self.acc_model() == u64::from(AccelerationModel::AM_ORIGINAL.0) {
            return max_speed as u16;
        }
        let mut curvecount = [0_i32; 2];
        // First find the curve speed limit.
        let (mut numcurve, mut sum, mut pos, mut lastpos) = (0_i32, 0_i32, 0_i32, -1_i32);
        let mut u = id;
        loop {
            let part_next: Handle = u.next();
            let this_dir = Direction(u.vehicle().direction());
            if part_next == Handle::NONE {
                break;
            }
            let next_dir = Direction((part_next).vehicle().direction());
            let dirdiff = DirDiff(this_dir.0.wrapping_sub(next_dir.0) & 7);
            if dirdiff != DirDiff::DIRDIFF_SAME {
                if dirdiff == DirDiff::DIRDIFF_45LEFT {
                    curvecount[0] = curvecount[0].wrapping_add(1);
                }
                if dirdiff == DirDiff::DIRDIFF_45RIGHT {
                    curvecount[1] = curvecount[1].wrapping_add(1);
                }
                if dirdiff == DirDiff::DIRDIFF_45LEFT || dirdiff == DirDiff::DIRDIFF_45RIGHT {
                    if lastpos != -1 {
                        numcurve = numcurve.wrapping_add(1);
                        sum = sum.wrapping_add(pos.wrapping_sub(lastpos));
                        if pos.wrapping_sub(lastpos) <= VEHICLE_LENGTH as i32 && max_speed > 88 {
                            max_speed = 88;
                        }
                    }
                    lastpos = pos;
                }
                // If we have a 90 degree turn, fix the speed limit to 60.
                if dirdiff == DirDiff::DIRDIFF_90LEFT || dirdiff == DirDiff::DIRDIFF_90RIGHT {
                    max_speed = 61;
                }
            }
            u = part_next;
            pos = pos.wrapping_add(i32::from(u16::from(read_field!(
                (u).vehicle().ground_cache(),
                cached_veh_length
            ))));
        }
        if numcurve > 0 && max_speed > 88 {
            if curvecount == [1, 1] {
                max_speed = absolute_max_speed;
            } else {
                sum = sum.wrapping_add(VEHICLE_LENGTH as i32 - 1) / VEHICLE_LENGTH as i32;
                sum /= numcurve;
                let d = 13 - sum.clamp(1, 12);
                max_speed = 232 - d * d;
            }
        }
        if max_speed != absolute_max_speed {
            // Apply the current railtype's curve speed advantage.
            max_speed = max_speed.wrapping_add(
                (max_speed / 2).wrapping_mul((self.leaves.curve_advantage)(id) as i32),
            );
            if id.get_cached_tilt() != 0 {
                // Apply max_speed bonus of 20% for a tilting train.
                max_speed = max_speed.wrapping_add(max_speed / 5);
            }
            // Apply max_speed modifier (cached value is fixed-point binary with 8 fractional bits)
            // and clamp the result to an acceptable range.
            max_speed = max_speed.wrapping_add(
                max_speed.wrapping_mul(i32::from(id.get_cached_curve_speed_mod() as i16)) / 256,
            );
            max_speed = max_speed.clamp(2, absolute_max_speed);
        }
        max_speed as u16
    }
    /// Get the stop location of (the center) of the front vehicle of a train at
    /// a platform of a station. Returns the location calculated from the begin of
    /// the station, the amount of 1/16th tiles ahead, and the station length.
    fn get_train_stop_location(
        &self,
        id: Handle,
        station_id: StationID,
        tile: TileIndex,
    ) -> (i32, i32, i32) {
        let world = crate::map_func::World::new(self.services);
        // SAFETY: The process-lifetime tables resolve the original Station::Get once.
        let stations = unsafe { &*(*((self.services.station_catchment)())).station };
        let st = (stations.station)(station_id.0);
        let dir = crate::direction_func::dir_to_diag_dir_raw((id).vehicle().direction(), |ok| {
            world.check(ok);
        });
        let station_ahead = (crate::follow_track::platform_length_station(&world, tile.0, dir)
            as i32)
            .wrapping_mul(TILE_SIZE as i32);
        let station_length = (crate::follow_track::platform_length_full(&world, st.base, tile.0)
            as i32)
            .wrapping_mul(TILE_SIZE as i32);
        let v_length: u16 = u16::from(read_field!(id.vehicle().ground_cache(), cached_veh_length));
        let v_total_length: u16 = read_field!(id.vehicle().ground_cache(), cached_total_length);
        let v_order_destination = StationID(id.vehicle().current_order().destination);
        let v_order = OrderType(id.vehicle().current_order().kind());
        // Default to the middle of the station for stations stops that are not in
        // the order list like intermediate stations when non-stop is disabled.
        let mut osl = u64::from(OrderStopLocation::Middle.0);
        if i32::from(v_total_length) >= station_length {
            // The train is longer than the station, make it stop at the far end of the platform.
            osl = u64::from(OrderStopLocation::FarEnd.0);
        } else if v_order == OrderType::OT_GOTO_STATION && v_order_destination == station_id {
            osl = (self.leaves.stop_location)(id);
        }
        // The stop location of the FRONT! of the train.
        let stop = match osl {
            value if value == u64::from(OrderStopLocation::NearEnd.0) => i32::from(v_total_length),
            value if value == u64::from(OrderStopLocation::Middle.0) => station_length
                .wrapping_sub(station_length.wrapping_sub(i32::from(v_total_length)) / 2),
            _ => station_length,
        };
        // Subtract half the front vehicle length of the train so we get the real
        // stop location of the train.
        (
            stop.wrapping_sub((i32::from(v_length) + 1) / 2),
            station_ahead,
            station_length,
        )
    }
    /// Calculates the maximum speed of the vehicle under its current conditions.
    fn get_current_max_speed(&self, id: Handle) -> i32 {
        let v_tile = TileIndex(id.vehicle().tile());
        let v_max_track_speed: u16 =
            read_field!(id.vehicle().ground_cache(), cached_max_track_speed);
        let v_speed: u16 = id.vehicle().speed();
        let model = self.acc_model();
        let mut max_speed = if model == u64::from(AccelerationModel::AM_ORIGINAL.0) {
            i32::from(v_max_track_speed)
        } else {
            id.get_cached_max_curve_speed() as i32
        };
        if model == u64::from(AccelerationModel::AM_REALISTIC.0)
            && (self.leaves.is_station)(id, v_tile.0) != 0
        {
            let sid = self.world().get_station_index(v_tile);
            if crate::movement::should_stop(self.services, id.vehicle(), sid.0) {
                let (stop_at, station_ahead, station_length) =
                    self.get_train_stop_location(id, sid, v_tile);
                // The distance to go is whatever is still ahead of the train minus the
                // distance from the train's stop location to the end of the platform.
                let distance_to_go = station_ahead / TILE_SIZE as i32
                    - station_length.wrapping_sub(stop_at) / TILE_SIZE as i32;
                if distance_to_go > 0 {
                    let mut st_max_speed = 120;
                    let delta_v = i32::from(v_speed) / distance_to_go.wrapping_add(1);
                    if max_speed > i32::from(v_speed) - delta_v {
                        st_max_speed = i32::from(v_speed) - delta_v / 10;
                    }
                    st_max_speed = st_max_speed.max(25_i32.wrapping_mul(distance_to_go));
                    max_speed = max_speed.min(st_max_speed);
                }
            }
        }
        let mut u = id;
        while u != Handle::NONE {
            if model == u64::from(AccelerationModel::AM_REALISTIC.0)
                && u.get_track() == u64::from(TrackBits::TRACK_BIT_DEPOT.0)
            {
                max_speed = max_speed.min(61);
                break;
            }
            // Vehicle is on the middle part of a bridge.
            if u.get_track() == u64::from(TrackBits::TRACK_BIT_WORMHOLE.0)
                && VehStates((u).vehicle().status()).0 & VehStates::from_element(VehState::Hidden).0
                    == 0
            {
                let tile = (u).vehicle().tile();
                max_speed = max_speed.min(i32::from((self.services.bridge_speed)(tile)));
            }
            u = (u).next();
        }
        max_speed = max_speed.min((self.leaves.order_max_speed)(id) as i32);
        max_speed.min(i32::from(read_field!(
            (id).vehicle().ground_cache(),
            cached_max_track_speed
        )))
    }
    fn update_speed(&self, id: Handle) -> i32 {
        let v_status: u8 = id.vehicle().status();
        let v_acceleration: u8 = id.vehicle().acceleration();
        let braking = v_status & 2 != 0 || id.flag(0) || id.flag(8);
        let max = self.get_current_max_speed(id);
        if self.acc_model() == 0 {
            crate::ground_vehicle::update_speed(
                id.vehicle(),
                (i64::from(i32::from(v_acceleration) * if braking { -4 } else { 2 }) as u64) as u32,
                0,
                (max as u64) as i32,
            )
            .distance as i32
        } else {
            crate::ground_vehicle::update_speed(
                id.vehicle(),
                crate::ground_vehicle::acceleration(self.leaves.physics, id.vehicle()) as u32,
                (u64::from(!braking) * 2) as i32,
                (max as u64) as i32,
            )
            .distance as i32
        }
    }
    fn mark_dirty(&self, id: Handle) {
        let mut u = id;
        loop {
            (self.leaves.write_colourmap)(u, 0);
            (self.leaves.viewport)(u, 1, 0);
            u = (u).next();
            if u == Handle::NONE {
                break;
            }
        }
        crate::ground_vehicle::cargo_changed(self.leaves.physics, id.vehicle());
        update_acceleration(id);
    }
    fn trackdir(&self, id: Handle) -> u8 {
        let v_tile: u32 = id.vehicle().tile();
        let v_direction: u8 = id.vehicle().direction();
        let v_status: u8 = id.vehicle().status();
        if v_status & 128 != 0 {
            return u8::MAX;
        }
        let track = id.get_track() as u8;
        if track == DEPOT {
            return (self.leaves.diag_trackdir)(id, (self.leaves.depot_dir)(id, v_tile) as u8)
                as u8;
        }
        if track == WORMHOLE {
            return (self.leaves.diag_trackdir)(id, (self.leaves.dir_diag)(id, v_direction) as u8)
                as u8;
        }
        (self.leaves.track_direction)(id, (self.leaves.first_track)(id, track) as u8, v_direction)
            as u8
    }
    fn running_cost(&self, id: Handle) -> i64 {
        let vs = self.vs();
        // SAFETY: The shared NewGRF services live for the simulation runtime.
        let cap = unsafe { (&*(vs.newgrf)()).capabilities };
        let mut cost = 0_i64;
        let mut u = id;
        loop {
            let e = (vs.engine)(u.vehicle()).0.as_ptr();
            if crate::vehicle::EngineRef::of(e).train_price() != crate::economy_type::INVALID_PRICE
            {
                let mut factor = self.property(
                    u,
                    u32::from(crate::newgrf_properties::PROP_TRAIN_RUNNING_COST_FACTOR),
                    u64::from(crate::vehicle::EngineRef::of(e).train_running()),
                ) as u32;
                if factor != 0 {
                    if (u).multiheaded() != 0 {
                        factor /= 2;
                    }
                    cost = cost.saturating_add(crate::engine_capabilities::price(
                        cap,
                        e,
                        u32::from(crate::vehicle::EngineRef::of(e).train_price()),
                        factor,
                        0,
                    ));
                }
            }
            u = (u).next_vehicle();
            if u == Handle::NONE {
                break;
            }
        }
        cost
    }
}
/// Train branch witness `index` (`crate::witness::NAMES` train order).
const BREAKDOWN_SPEEDS: [u16; 16] = [
    225, 210, 195, 180, 165, 150, 135, 120, 105, 90, 75, 60, 45, 30, 15, 15,
];
impl Game<'_> {
    fn write_status(id: Handle, mask: u8, value: bool) {
        let old = (id).vehicle().status();
        (id).vehicle()
            .write_status((u64::from(if value { old | mask } else { old & !mask })) as u8);
    }

    fn exitdir(&self, id: Handle) -> u8 {
        (self.leaves.veh_exit_dir)(id, (id).vehicle().direction(), id.track()) as u8
    }
    fn compatible(&self, id: Handle, tile: u32) -> bool {
        crate::tile_map::is_tile_owner(
            self.services,
            TileIndex(tile),
            crate::company_type::CompanyID((id).vehicle().owner()),
        ) && ((id).front() == 0
            || id.get_compatible_railtypes() & (1 << (self.leaves.rail_type)(Handle::NONE, tile))
                != 0)
    }
    fn can_leave(&self, id: Handle) -> bool {
        let v_tile: u32 = id.vehicle().tile();
        let v_direction: u8 = id.vehicle().direction();
        let track = id.track();
        if track == WORMHOLE || track == DEPOT {
            return false;
        }
        if is_tile_type(
            self.services,
            TileIndex(v_tile),
            crate::tile_type::TileType::MP_TUNNELBRIDGE,
        ) && self.world().tunnel_bridge_dir(v_tile) * 2 + 1 == v_direction
        {
            return false;
        }
        if (self.leaves.is_depot)(Handle::NONE, v_tile) != 0
            && ((self.leaves.depot_dir)(Handle::NONE, v_tile) as u8 ^ 2) * 2 + 1 == v_direction
        {
            return false;
        }
        true
    }
    fn approaching_crossing(&self, id: Handle) -> u32 {
        if !self.can_leave(id) {
            return INVALID_TILE;
        }
        let base = (id).vehicle().tile();
        let dir = self.exitdir(id);
        let tile = base.wrapping_add((self.leaves.tile_offset_diag)(id, dir) as u32);
        if (self.leaves.is_crossing)(Handle::NONE, tile) == 0
            || (self.leaves.diag_axis)(id, dir)
                == (self.leaves.crossing_road_axis)(Handle::NONE, tile)
            || !self.compatible(id, tile)
        {
            return INVALID_TILE;
        }
        tile
    }
    fn train_on_tile(&self, tile: u32) -> bool {
        self.visit_tile(tile, |_| true)
    }
    fn crossing_approach(&self, tile: u32) -> bool {
        let dir = (self.leaves.axis_diag)(
            Handle::NONE,
            (self.leaves.crossing_rail_axis)(Handle::NONE, tile) as u8,
        ) as u8;
        for d in [dir, dir ^ 2] {
            let from = tile.wrapping_add((self.leaves.tile_offset_diag)(Handle::NONE, d) as u32);
            if self.visit_tile(from, |id| {
                let v_status: u8 = id.vehicle().status();
                let v_front: u8 = id.front();
                if v_status & 128 == 0 && v_front != 0 && self.approaching_crossing(id) == tile {
                    return true;
                }

                false
            }) {
                return true;
            }
        }
        false
    }
    fn check_crossing(&self, tile: u32) -> bool {
        (self.services.crossing_reserved)(tile)
            || self.train_on_tile(tile)
            || self.crossing_approach(tile)
    }
    fn update_crossing_tile(&self, tile: u32, sound: bool, force: bool) {
        let barred = force || self.check_crossing(tile);
        if barred != ((self.leaves.crossing_barred)(Handle::NONE, tile) != 0) {
            crate::witness::hit(if barred {
                crate::witness::TRAIN_CROSS_BAR
            } else {
                crate::witness::TRAIN_CROSS_UNBAR
            });
            // SAFETY: The shared catalog and native setting live for this serialized entry.
            if barred && sound && unsafe { self.services.settings().ambient.read() } {
                (self.leaves.crossing_sound)(Handle::NONE, tile);
            }
            (self.leaves.write_crossing_bar)(Handle::NONE, tile, u8::from(barred));
            (self.services.mark_dirty)(tile);
        }
    }
    fn update_crossing(&self, tile: u32, sound: bool, force: bool) {
        if (self.leaves.is_crossing)(Handle::NONE, tile) == 0 {
            return;
        }
        let mut state = force;
        let axis = (self.leaves.crossing_road_axis)(Handle::NONE, tile);
        let dir = (self.leaves.axis_diag)(Handle::NONE, axis as u8) as u8;
        for d in [dir, dir ^ 2] {
            let mut t = tile;
            while !state
                && t < self.services.map_size()
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                state |= self.check_crossing(t);
                t = self
                    .services
                    .map()
                    .tile_add_by_diag_dir(
                        TileIndex(t),
                        DiagDirection(d),
                        crate::map_func::native_check,
                    )
                    .0;
            }
        }
        self.update_crossing_tile(tile, sound, state);
        for d in [dir, dir ^ 2] {
            let mut t = self
                .services
                .map()
                .tile_add_by_diag_dir(
                    TileIndex(tile),
                    DiagDirection(d),
                    crate::map_func::native_check,
                )
                .0;
            while t < self.services.map_size()
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                self.update_crossing_tile(t, sound, state);
                t = self
                    .services
                    .map()
                    .tile_add_by_diag_dir(
                        TileIndex(t),
                        DiagDirection(d),
                        crate::map_func::native_check,
                    )
                    .0;
            }
        }
    }
    fn adjacent_crossing_dirty(&self, tile: u32, axis: u64) {
        let dir = (self.leaves.axis_diag)(Handle::NONE, axis as u8) as u8;
        for d in [dir, dir ^ 2] {
            let t = self
                .services
                .map()
                .tile_add_by_diag_dir(
                    TileIndex(tile),
                    DiagDirection(d),
                    crate::map_func::native_check,
                )
                .0;
            if t < self.services.map_size()
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                (self.services.mark_dirty)(t);
            }
        }
    }
    fn crossing_removed(&self, tile: u32, axis: u64) {
        let dir = (self.leaves.axis_diag)(Handle::NONE, axis as u8) as u8;
        for d in [dir, dir ^ 2] {
            let diff = (self.leaves.tile_offset_diag)(Handle::NONE, d) as u32;
            let mut occupied = false;
            let mut t = tile.wrapping_add(diff);
            while t < self.services.map_size()
                && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
            {
                occupied |= self.check_crossing(t);
                t = t.wrapping_add(diff);
            }
            if occupied {
                t = tile.wrapping_add(diff);
                if t < self.services.map_size()
                    && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                    && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
                {
                    (self.services.mark_dirty)(t);
                }
            } else {
                t = tile.wrapping_add(diff);
                while t < self.services.map_size()
                    && (self.leaves.is_crossing)(Handle::NONE, t) != 0
                    && (self.leaves.crossing_road_axis)(Handle::NONE, t) == axis
                {
                    if (self.leaves.crossing_barred)(Handle::NONE, t) != 0 {
                        (self.leaves.write_crossing_bar)(Handle::NONE, t, 0);
                        (self.services.mark_dirty)(t);
                    } else {
                        (self.services.mark_dirty)(t);
                        break;
                    }
                    t = t.wrapping_add(diff);
                }
            }
        }
    }
    fn bar_crossing(&self, tile: u32) {
        if (self.leaves.crossing_barred)(Handle::NONE, tile) == 0 {
            (self.services.set_crossing_reserved)(tile, true);
            self.update_crossing(tile, true, false);
        }
    }
    fn mark_stuck(&self, id: Handle) {
        if !id.flag(8) {
            id.set_flag(8, true);
            id.set_wait_counter(0);
            (id).vehicle().write_speed(0_u16);
            (id).vehicle().write_subspeed(0_u8);
            crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
            (self.leaves.start_stop_dirty)(id);
        }
    }
    fn whole_in_depot(id: Handle) -> bool {
        let tile = (id).vehicle().tile();
        let mut u = id;
        while u != Handle::NONE {
            if u.track() != DEPOT || (u).vehicle().tile() != tile {
                return false;
            }
            u = (u).next();
        }
        true
    }
    fn next_offset(id: Handle) -> i32 {
        let v_next: Handle = id.next();
        let v_length: u16 = u16::from(read_field!(id.vehicle().ground_cache(), cached_veh_length));
        i32::from(v_length) / 2
            + if v_next == Handle::NONE {
                0
            } else {
                (i32::from(u16::from(read_field!(
                    (v_next).vehicle().ground_cache(),
                    cached_veh_length
                ))) + 1)
                    / 2
            }
    }
    fn after_swap(&self, id: Handle) {
        if id.track() == WORMHOLE {
            crate::witness::hit(crate::witness::TRAIN_WORMHOLE_SWAP);
        }
        if id.track() != DEPOT {
            (id).vehicle()
                .write_direction((u64::from((id).vehicle().direction() ^ 4)) as u8);
        }
        let v_tile: u32 = id.vehicle().tile();
        let v_x: i32 = id.vehicle().x();
        let v_y: i32 = id.vehicle().y();
        if id.track() != WORMHOLE {
            self.movement_enter(id, v_tile, (v_x as u64) as i32, (v_y as u64) as i32);
        } else {
            let vt = (self.leaves.tile_virt)((v_x as u64) as i32, (v_y as u64) as i32);
            if is_tile_type(
                self.services,
                TileIndex(vt),
                crate::tile_type::TileType::MP_TUNNELBRIDGE,
            ) {
                self.movement_enter(id, vt, (v_x as u64) as i32, (v_y as u64) as i32);
                if id.track() != WORMHOLE
                    && (self.leaves.is_bridge)(Handle::NONE, (id).vehicle().tile()) != 0
                {
                    self.update_occupancy(id);
                    crate::ground_vehicle::inclination(
                        self.leaves.physics,
                        &self.movement,
                        id.vehicle(),
                        true,
                        true,
                    );
                    return;
                }
            }
        }
        self.update_occupancy(id);
        (self.leaves.viewport)(id, 1, 1);
    }
    fn swap_ground_flags(a: Handle, b: Handle) {
        let fa = read_field!((a).vehicle().ground(), flags);
        let fb = read_field!((b).vehicle().ground(), flags);
        write_field!((a).vehicle().ground(), flags, (u64::from(fa & !3)) as u16);
        write_field!(
            (b).vehicle().ground(),
            flags,
            (u64::from(read_field!((b).vehicle().ground(), flags) & !3)) as u16
        );
        if fa & 1 != 0 {
            write_field!(
                (b).vehicle().ground(),
                flags,
                (u64::from(read_field!((b).vehicle().ground(), flags) | 2)) as u16
            );
        } else if fa & 2 != 0 {
            write_field!(
                (b).vehicle().ground(),
                flags,
                (u64::from(read_field!((b).vehicle().ground(), flags) | 1)) as u16
            );
        }
        if fb & 1 != 0 {
            write_field!(
                (a).vehicle().ground(),
                flags,
                (u64::from(read_field!((a).vehicle().ground(), flags) | 2)) as u16
            );
        } else if fb & 2 != 0 {
            write_field!(
                (a).vehicle().ground(),
                flags,
                (u64::from(read_field!((a).vehicle().ground(), flags) | 1)) as u16
            );
        }
    }
    fn reverse_swap(&self, id: Handle, l: i32, r: i32) {
        let (mut a, mut b) = (id, id);
        for _ in 0..l {
            a = (a).next();
        }
        for _ in 0..r {
            b = (b).next();
        }
        if a != b {
            let va_tile: u32 = a.vehicle().tile();
            let va_x: i32 = a.vehicle().x();
            let va_y: i32 = a.vehicle().y();
            let va_z: i32 = a.vehicle().z();
            let va_direction: u8 = a.vehicle().direction();
            let va_status: u8 = a.vehicle().status();
            let vb_tile: u32 = b.vehicle().tile();
            let vb_x: i32 = b.vehicle().x();
            let vb_y: i32 = b.vehicle().y();
            let vb_z: i32 = b.vehicle().z();
            let vb_direction: u8 = b.vehicle().direction();
            let vb_status: u8 = b.vehicle().status();
            Self::write_status(b, 1, va_status & 1 != 0);
            Self::write_status(a, 1, vb_status & 1 != 0);
            let ta = a.track();
            a.set_track(u64::from(b.track()));
            b.set_track(u64::from(ta));
            (a).vehicle()
                .write_direction((u64::from(vb_direction)) as u8);
            (b).vehicle()
                .write_direction((u64::from(va_direction)) as u8);
            (a).vehicle().write_x((vb_x as u64) as i32);
            (b).vehicle().write_x((va_x as u64) as i32);
            (a).vehicle().write_y((vb_y as u64) as i32);
            (b).vehicle().write_y((va_y as u64) as i32);
            (a).vehicle().write_tile((u64::from(vb_tile)) as u32);
            (b).vehicle().write_tile((u64::from(va_tile)) as u32);
            (a).vehicle().write_z((vb_z as u64) as i32);
            (b).vehicle().write_z((va_z as u64) as i32);
            Self::swap_ground_flags(a, b);
            self.after_swap(a);
            self.after_swap(b);
        } else {
            Self::swap_ground_flags(a, a);
            self.after_swap(a);
        }
    }
    fn advance_before_swap(&self, id: Handle) {
        let (mut base, mut first, mut last) = (id, id, (id).last());
        let mut length = (self.leaves.count_chain)(id) as u32;
        while length > 2 {
            last = (last).previous();
            first = (first).next();
            let diff = Self::next_offset(base) - Self::next_offset(last);
            for _ in 0..diff {
                crate::witness::hit(crate::witness::TRAIN_UNEQUAL_BEFORE);
                self.controller(first, (last).next(), true);
            }
            base = first;
            length -= 2;
        }
    }
    fn advance_after_swap(&self, id: Handle) {
        let mut dep = id;
        while (dep).next() != Handle::NONE
            && (dep.track() == DEPOT || (dep).next().track() != DEPOT)
        {
            dep = (dep).next();
        }
        let leave = (dep).next();
        if leave != Handle::NONE {
            let d = crate::movement::depot_ticks(self.services, dep);
            if d <= 0 {
                Self::write_status(leave, 1, false);
                leave.set_track(
                    1 << (self.leaves.depot_track)(Handle::NONE, (leave).vehicle().tile()),
                );
                for _ in d..=0 {
                    self.controller(leave, Handle::NONE, true);
                }
            }
        } else {
            dep = Handle::NONE;
        }
        let (mut base, mut first, mut last) = (id, id, (id).last());
        let mut length = (self.leaves.count_chain)(id) as u32;
        let mut nomove = dep == Handle::NONE;
        while length > 2 {
            if base == dep {
                break;
            }
            if last == dep {
                nomove = true;
            }
            last = (last).previous();
            first = (first).next();
            let diff = Self::next_offset(last) - Self::next_offset(base);
            for _ in 0..diff {
                crate::witness::hit(crate::witness::TRAIN_UNEQUAL_AFTER);
                self.controller(
                    first,
                    if nomove { (last).next() } else { Handle::NONE },
                    true,
                );
            }
            base = first;
            length -= 2;
        }
    }
    fn reverse(&self, id: Handle) {
        crate::witness::hit(crate::witness::TRAIN_REVERSAL);
        if (self.leaves.is_depot)(Handle::NONE, (id).vehicle().tile()) != 0 {
            if Self::whole_in_depot(id) {
                return;
            }
            (self.leaves.depot_dirty)(id);
        }
        if !id.flag(8) {
            self.free_train_reservation(id);
        }
        let crossing = self.approaching_crossing(id);
        let mut r = (self.leaves.count_chain)(id) as i32 - 1;
        self.advance_before_swap(id);
        let mut l = 0;
        loop {
            self.reverse_swap(id, l, r);
            l += 1;
            r -= 1;
            if l > r {
                break;
            }
        }
        self.advance_after_swap(id);
        if (self.leaves.is_depot)(Handle::NONE, (id).vehicle().tile()) != 0 {
            (self.leaves.depot_dirty)(id);
        }
        id.set_flag(7, !id.flag(7));
        id.set_flag(0, false);
        self.consist_changed(id, 0);
        let mut u = id;
        while u != Handle::NONE {
            (self.leaves.viewport)(u, 0, 0);
            u = (u).next();
        }
        if crossing != INVALID_TILE {
            self.update_crossing(crossing, true, false);
        }
        let crossing = self.approaching_crossing(id);
        if crossing != INVALID_TILE {
            self.bar_crossing(crossing);
        }
        if id.track() == DEPOT {
            if id.flag(8) {
                (self.leaves.start_stop_dirty)(id);
            }
            id.set_flag(8, false);
            return;
        }
        let tile = (id).vehicle().tile();
        let mut dir = self.exitdir(id);
        if (self.leaves.is_depot)(Handle::NONE, tile) != 0
            || is_tile_type(
                self.services,
                TileIndex(tile),
                crate::tile_type::TileType::MP_TUNNELBRIDGE,
            )
        {
            dir = u8::MAX;
        }
        if self.signal_segment(id, tile, dir) == (self.leaves.sigseg_pbs)(id)
            || (self.leaves.reserve_paths)()
        {
            let td = self.trackdir(id);
            let mut okay = !(is_tile_type(
                self.services,
                TileIndex(tile),
                crate::tile_type::TileType::MP_RAILWAY,
            ) && (self.leaves.has_signal_td)(id, tile, td) != 0
                && (self.leaves.signal_pbs)(
                    id,
                    (self.leaves.signal_type)(id, tile, id.track().trailing_zeros() as u8) as u8,
                ) == 0);
            if (self.leaves.is_depot)(Handle::NONE, tile) != 0
                && (self.leaves.trackdir_exit)(id, td)
                    == (self.leaves.depot_dir)(Handle::NONE, tile)
            {
                okay = false;
            }
            if (self.leaves.is_station)(Handle::NONE, tile) != 0 {
                crate::pbs::Pbs::acquire(self.services).platform(
                    tile,
                    (self.leaves.trackdir_exit)(id, td) as u8,
                    true,
                );
            }
            if self.try_path_reserve(id, false, okay) {
                (self.leaves.check_next)(id);
            } else if (id).vehicle().current_order().kind() != OT_LOADING {
                self.mark_stuck(id);
            }
        } else if id.flag(8) {
            id.set_flag(8, false);
            id.set_wait_counter(0);
        }
    }
    fn approaching_end(&self, id: Handle, signal: bool, reverse: bool) -> bool {
        let v_x: i32 = id.vehicle().x();
        let v_y: i32 = id.vehicle().y();
        let v_length: u16 = u16::from(read_field!(id.vehicle().ground_cache(), cached_veh_length));
        let v_speed: u16 = id.vehicle().speed();
        let v_direction: u8 = id.vehicle().direction();
        let mut x = v_x as u32 & 15;
        let y = v_y as u32 & 15;
        match v_direction {
            0 => x = (!x).wrapping_add(!y).wrapping_add(25),
            7 => x = (!y).wrapping_add(16),
            1 => x = (!x).wrapping_add(16),
            2 => x = (!x).wrapping_add(y).wrapping_add(9),
            3 => x = y,
            4 => x = x.wrapping_add(y).wrapping_sub(7),
            6 => x = (!y).wrapping_add(x).wrapping_add(9),
            _ => {}
        }
        if !signal
            && x.wrapping_add(
                u32::from(v_length + 1) / 2 * if v_direction & 1 != 0 { 1 } else { 2 },
            ) >= 16
        {
            (id).vehicle().write_speed(0_u16);
            if reverse {
                self.reverse(id);
            }
            return false;
        }
        Self::write_status(id, 16, true);
        let speed = BREAKDOWN_SPEEDS[(x & 15) as usize];
        if speed < v_speed {
            (id).vehicle().write_speed((u64::from(speed)) as u16);
        }
        true
    }
    fn line_ends(&self, id: Handle, reverse: bool) -> bool {
        let v_tile: u32 = id.vehicle().tile();
        let v_speed: u16 = id.vehicle().speed();
        let v_breakdown: u8 = read_field!(id.record, breakdown_ctr);
        let t = v_breakdown;
        if t > 1 {
            Self::write_status(id, 16, true);
            let speed = BREAKDOWN_SPEEDS[usize::from((!t >> 4) & 15)];
            if speed < v_speed {
                (id).vehicle().write_speed((u64::from(speed)) as u16);
            }
        } else {
            Self::write_status(id, 16, false);
        }
        if !self.can_leave(id) {
            return true;
        }
        let dir = self.exitdir(id);
        let tile = v_tile.wrapping_add((self.leaves.tile_offset_diag)(id, dir) as u32);
        let ts = self.movement_tracks(tile, dir ^ 2) as u32;
        let reaches = (self.leaves.trackdir_reaches)(id, dir) as u16;
        let tds = ts as u16 & reaches;
        let reds = (ts >> 16) as u16 & reaches;
        let mut bits = ((tds | tds >> 8) & 63) as u8;
        if (self.leaves.no_90)(
            id,
            (self.leaves.tile_rail_type)(Handle::NONE, v_tile) as u8,
            (self.leaves.tile_rail_type)(Handle::NONE, tile) as u8,
        ) != 0
        {
            bits &= !((self.leaves.track_crosses)(id, id.track().trailing_zeros() as u8) as u8);
        }
        if bits == 0 || !self.compatible(id, tile) {
            return self.approaching_end(id, false, reverse);
        }
        if tds & reds != 0 {
            return self.approaching_end(id, true, reverse);
        }
        if (self.leaves.is_crossing)(Handle::NONE, tile) != 0 {
            self.bar_crossing(tile);
        }
        true
    }
}
const INITIAL_SUBCOORD: [[[u8; 3]; 4]; 6] = [
    [[15, 8, 1], [0, 0, 0], [0, 8, 5], [0, 0, 0]],
    [[0, 0, 0], [8, 0, 3], [0, 0, 0], [8, 15, 7]],
    [[0, 0, 0], [7, 0, 2], [0, 7, 6], [0, 0, 0]],
    [[15, 8, 2], [0, 0, 0], [0, 0, 0], [8, 15, 6]],
    [[15, 7, 0], [8, 0, 4], [0, 0, 0], [0, 0, 0]],
    [[0, 0, 0], [0, 0, 0], [0, 8, 4], [7, 15, 0]],
];
impl Game<'_> {
    fn enter_station(&self, id: Handle, station: u16) {
        (id).vehicle()
            .write_last_station((u64::from(station)) as u16);
        if (self.services.station_had_vehicle_of_type)(station) & HVOT_TRAIN.0 == 0 {
            (self.services.station_add_vehicle_of_type)(station, HVOT_TRAIN.0);
            (self.leaves.arrival_news)(id, station);
        }
        id.set_force_proceed(0);
        self.view_dirty(id);
        crate::vehicle::visits::begin_loading(id.vehicle(), self.vs().visits());
        crate::animated_tile::rail_event(
            crate::animated_tile::station_base(station),
            (id.vehicle()).tile(),
            crate::station_type::SAT_VEHICLE_ARRIVES,
            crate::station_type::SRT_VEHICLE_ARRIVES,
        );
    }
    fn speed_z(&self, id: Handle, old: i32) {
        let v_z: i32 = id.vehicle().z();
        let v_max_track_speed: u16 =
            read_field!(id.vehicle().ground_cache(), cached_max_track_speed);
        let v_speed: u16 = id.vehicle().speed();
        if old == v_z || self.acc_model() != 0 {
            return;
        }
        if old < v_z {
            (id).vehicle()
                .write_speed((u64::from(v_speed - ((u32::from(v_speed) * 64) >> 8) as u16)) as u16);
        } else {
            let speed = v_speed.wrapping_add(2);
            if speed <= v_max_track_speed {
                (id).vehicle().write_speed((u64::from(speed)) as u16);
            }
        }
    }
    fn moved_signals(&self, id: Handle, tile: u32, dir: u8) -> bool {
        if is_tile_type(
            self.services,
            TileIndex(tile),
            crate::tile_type::TileType::MP_RAILWAY,
        ) && (self.leaves.has_signals)(Handle::NONE, tile) != 0
        {
            let tracks = (self.leaves.track_bits)(Handle::NONE, tile) as u16;
            let tds = (tracks | tracks << 8) & (self.leaves.trackdir_reaches)(id, dir) as u16;
            let td = tds.trailing_zeros() as u8;
            let exit = (self.leaves.trackdir_exit)(id, td);
            // Tile owner is used here, rather than the moving train's owner.
            if u64::from(self.signal_context().segment(
                tile,
                exit as u8,
                self.world().owner(TileIndex(tile)),
            )) == (self.leaves.sigseg_pbs)(id)
                && (self.leaves.has_signal_td)(id, tile, td) != 0
            {
                if (self.leaves.signal_pbs)(id, (self.leaves.signal_type)(id, tile, td & 7) as u8)
                    == 0
                {
                    return true;
                }
            }
        }
        false
    }
    // Result: 0 moved, 1 no move/early stop, 2 invalid rail, 3 requested reversal.
    fn move_vehicle(&self, id: Handle, prev: Handle, reverse: bool) -> (u8, bool) {
        let v_tile: u32 = id.vehicle().tile();
        let v_x: i32 = id.vehicle().x();
        let v_y: i32 = id.vehicle().y();
        let v_direction: u8 = id.vehicle().direction();
        let v_front: u8 = id.front();
        let v_articulated: u8 = id.articulated();
        if v_articulated != 0 {
            crate::witness::hit(crate::witness::TRAIN_ARTICULATED_MOVE);
        }
        let delta = [
            (-1, -1),
            (-1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
            (1, 0),
            (1, -1),
            (0, -1),
        ];
        let (dx, dy) = delta[usize::from(v_direction)];
        let (mut x, mut y) = (v_x.wrapping_add(dx), v_y.wrapping_add(dy));
        let mut old = v_tile;
        let new = (self.leaves.tile_virt)((x as u64) as i32, (y as u64) as i32);
        let mut entered = 0_u8;
        let mut signals = false;
        let mut changed = false;
        if id.track() != WORMHOLE {
            if old == new {
                if id.track() == DEPOT {
                    x = v_x;
                    y = v_y;
                } else {
                    if v_front != 0 && !self.line_ends(id, reverse) {
                        return (1, false);
                    }
                    let vets =
                        self.movement_enter(id, new, (x as u64) as i32, (y as u64) as i32) as u8;
                    if vets & 4 != 0 {
                        return (2, false);
                    }
                    if vets & 1 != 0 {
                        self.enter_station(id, self.world().get_station_index(TileIndex(new)).0);
                    }
                }
            } else {
                entered = (self.leaves.diag_between)(id, old, new) as u8;
                let ts = self.movement_tracks(new, entered ^ 2) as u32;
                let reaches = (self.leaves.trackdir_reaches)(id, entered) as u16;
                let tds = ts as u16 & reaches;
                let reds = (((ts >> 16) as u16 & reaches) | (((ts >> 16) as u16 & reaches) >> 8))
                    as u8
                    & 63;
                let mut bits = ((tds | tds >> 8) & 63) as u8;
                if (self.leaves.no_90)(
                    id,
                    (self.leaves.tile_rail_type)(Handle::NONE, old) as u8,
                    (self.leaves.tile_rail_type)(Handle::NONE, new) as u8,
                ) != 0
                    && prev == Handle::NONE
                {
                    bits &=
                        !((self.leaves.track_crosses)(id, id.track().trailing_zeros() as u8) as u8);
                }
                if bits == 0 || !self.compatible(id, new) {
                    return (2, false);
                }
                let mut chosen;
                if prev == Handle::NONE {
                    chosen = 1_u8 << (self.leaves.choose_track)(id, new, entered, bits);
                    if id.get_force_proceed() != 0
                        && (self.leaves.is_plain_rail)(Handle::NONE, new) != 0
                        && (self.leaves.has_signals)(Handle::NONE, new) != 0
                    {
                        let td = tds.trailing_zeros() as u8;
                        if (self.leaves.has_signal_td)(id, new, td) != 0
                            || ((self.leaves.has_signal_td)(id, new, td ^ 8) != 0
                                && (self.leaves.signal_type)(id, new, td & 7)
                                    != (self.leaves.pbs_signal_type)(id))
                        {
                            crate::witness::hit(crate::witness::TRAIN_FORCE_SIGNAL);
                            id.set_force_proceed(u64::from(id.get_force_proceed() == 2));
                            self.view_dirty(id);
                        }
                    }
                    if reds & chosen != 0 && id.get_force_proceed() == 0 {
                        let td = tds.trailing_zeros() as u8;
                        if id.flag(8) {
                            return (1, false);
                        }
                        if (self.leaves.has_signal_td)(id, new, td ^ 8) == 0 {
                            (id).vehicle().write_speed(0_u16);
                            (id).vehicle().write_subspeed(0_u8);
                            (id).vehicle().write_progress(255_u8);
                            crate::witness::hit(crate::witness::TRAIN_RED_ONEWAY);
                            if self.reverse_at_signals() == 0
                                || u64::from(id.wait_inc())
                                    < self.wait_oneway() * (self.leaves.day_ticks)(id) * 2
                            {
                                return (1, false);
                            }
                        } else if (self.leaves.has_signal_td)(id, new, td) != 0 {
                            (id).vehicle().write_speed(0_u16);
                            (id).vehicle().write_subspeed(0_u8);
                            (id).vehicle().write_progress(255_u8);
                            crate::witness::hit(crate::witness::TRAIN_RED_TWOWAY);
                            if self.reverse_at_signals() == 0
                                || u64::from(id.wait_inc())
                                    < self.wait_twoway() * (self.leaves.day_ticks)(id) * 2
                            {
                                let dir = (self.leaves.trackdir_exit)(id, td) as u8;
                                let tile = self
                                    .services
                                    .map()
                                    .tile_add_by_diag_dir(
                                        TileIndex(new),
                                        DiagDirection(dir),
                                        crate::map_func::native_check,
                                    )
                                    .0;
                                let mut waiting = false;
                                self.visit_tile(tile, |u| {
                                    let other_speed: u16 = u.vehicle().speed();
                                    let other_status: u8 = u.vehicle().status();
                                    let other_front: u8 = u.front();
                                    if other_status & 128 == 0
                                        && other_front != 0
                                        && u.track() & 63 != 0
                                        && other_speed <= 5
                                        && self.exitdir(u) == dir ^ 2
                                    {
                                        waiting = true;
                                        return true;
                                    }

                                    false
                                });
                                if !waiting {
                                    return (1, false);
                                }
                            }
                        }
                        if self.reverse_at_signals() == 0
                            && (self.leaves.oneway_blocking)(id, new, td) == 0
                            && self.signal_segment(id, (id).vehicle().tile(), entered)
                                == (self.leaves.sigseg_pbs)(id)
                        {
                            id.set_wait_counter(0);
                            return (1, false);
                        }
                        return (3, false);
                    }
                    crate::pbs::Pbs::acquire(self.services).try_reserve(
                        new,
                        chosen.trailing_zeros() as u8,
                        false,
                    );
                } else {
                    let before = (prev).vehicle().tile();
                    if before == new {
                        chosen = if prev.track() == WORMHOLE {
                            bits
                        } else {
                            prev.track()
                        };
                    } else {
                        let exit = (self.leaves.diag_between)(id, new, before) as usize;
                        let connecting =
                            [[1, 8, 0, 16], [4, 2, 16, 0], [0, 32, 1, 4], [32, 0, 8, 2]];
                        chosen = connecting[usize::from(entered)][exit];
                    }
                    chosen &= bits;
                }
                let sub = INITIAL_SUBCOORD[chosen.trailing_zeros() as usize][usize::from(entered)];
                x = (x & !15) | i32::from(sub[0]);
                y = (y & !15) | i32::from(sub[1]);
                let dir = sub[2];
                let vets = self.movement_enter(id, new, (x as u64) as i32, (y as u64) as i32) as u8;
                if vets & 4 != 0 {
                    return (2, false);
                }
                if vets & 2 == 0 {
                    if (id).front() != 0 {
                        crate::witness::hit(crate::witness::TRAIN_FIRST_TILE_RAIL);
                    }
                    let td =
                        (self.leaves.track_direction)(id, chosen.trailing_zeros() as u8, dir) as u8;
                    if (id).front() != 0 && (self.leaves.signal_has_pbs)(id, new, td) != 0 {
                        (self.leaves.set_signal_state)(id, new, td, 0);
                        (self.services.mark_dirty)(new);
                    }
                    if (id).next() == Handle::NONE {
                        (self.leaves.clear_reservation)(
                            id,
                            (id).vehicle().tile(),
                            self.trackdir(id),
                        );
                    }
                    (id).vehicle().write_tile((u64::from(new)) as u32);
                    if (self.leaves.tile_rail_type)(Handle::NONE, new)
                        != (self.leaves.tile_rail_type)(Handle::NONE, old)
                    {
                        self.consist_changed((id).first(), 0);
                    }
                    id.set_track(u64::from(chosen));
                }
                signals = true;
                if dir != (id).vehicle().direction() {
                    if prev == Handle::NONE && self.acc_model() == 0 {
                        let diff = (id).vehicle().direction().wrapping_sub(dir) & 7;
                        let small = if u64::from(crate::ground_vehicle::acceleration_type(
                            self.leaves.physics,
                            id.vehicle(),
                        )) == 2
                        {
                            0
                        } else {
                            64
                        };
                        let fraction = if diff == 1 || diff == 7 { small } else { 128 };
                        let speed = (id).vehicle().speed();
                        (id).vehicle().write_speed(
                            (u64::from(speed - ((u32::from(speed) * fraction) >> 8) as u16)) as u16,
                        );
                    }
                    changed = true;
                    (id).vehicle().write_direction((u64::from(dir)) as u8);
                }
                if (id).front() != 0 {
                    id.set_wait_counter(0);
                    let crossing = self.approaching_crossing(id);
                    if crossing != INVALID_TILE
                        && (self.services.crossing_reserved)(crossing)
                        // SAFETY: The shared catalog and native setting live for this entry.
                        && unsafe { self.services.settings().ambient.read() }
                    {
                        (self.leaves.crossing_sound)(Handle::NONE, crossing);
                    }
                    (self.leaves.check_next)(id);
                }
                if vets & 1 != 0 {
                    self.enter_station(id, self.world().get_station_index(TileIndex(new)).0);
                }
            }
        } else {
            if is_tile_type(
                self.services,
                TileIndex(new),
                crate::tile_type::TileType::MP_TUNNELBRIDGE,
            ) && self.movement_enter(id, new, (x as u64) as i32, (y as u64) as i32) & 2 != 0
            {
                if (id).front() != 0 {
                    crate::witness::hit(crate::witness::TRAIN_WORMHOLE_EXIT);
                    self.reserve_track(id, new, self.world().tunnel_bridge_dir(new) & 1);
                    (self.leaves.check_next)(id);
                }
                if old == new {
                    old = (self.leaves.other_end)(Handle::NONE, old) as u32;
                }
            } else {
                (id).vehicle().write_x((x as u64) as i32);
                (id).vehicle().write_y((y as u64) as i32);
                self.update_occupancy(id);
                if (id).vehicle().status() & 1 == 0 {
                    (self.leaves.base_viewport)(id, 1);
                }
                return (0, false);
            }
        }
        (self.vs().update_delta)(id.vehicle());
        (id).vehicle().write_x((x as u64) as i32);
        (id).vehicle().write_y((y as u64) as i32);
        self.update_occupancy(id);
        let old_z = crate::ground_vehicle::inclination(
            self.leaves.physics,
            &self.movement,
            id.vehicle(),
            new != old,
            false,
        );
        if prev == Handle::NONE {
            self.speed_z(id, old_z);
        }
        if signals {
            if (id).front() != 0 && self.moved_signals(id, new, entered) {
                if (!crate::pbs::Pbs::acquire(self.services).has_reserved_tracks(new, id.track())
                    && self.reserve_track(id, new, id.track().trailing_zeros() as u8) == 0)
                    || !self.try_path_reserve(id, false, false)
                {
                    self.mark_stuck(id);
                }
            }
            if (id).next() == Handle::NONE {
                self.moved_signals(id, old, entered ^ 2);
                if (self.leaves.is_crossing)(Handle::NONE, old) != 0 {
                    self.update_crossing(old, true, false);
                }
            }
        }
        if (id).front() != 0
            && u64::from((id).vehicle().tick()) % u64::from((self.leaves.backoff)()) == 0
        {
            (self.leaves.check_next)(id);
        }
        (0, changed)
    }
    fn controller(&self, mut id: Handle, nomove: Handle, reverse: bool) -> bool {
        let first = (id).first();
        let mut prev = (id).previous();
        let mut changed = false;
        while id != nomove {
            let (outcome, direction) = self.move_vehicle(id, prev, reverse);
            changed |= direction;
            if outcome == 1 {
                return false;
            }
            if outcome == 2 && prev != Handle::NONE {
                (self.leaves.disconnect)(id);
            }
            if outcome >= 2 {
                if reverse {
                    id.set_wait_counter(0);
                    (id).vehicle().write_speed(0_u16);
                    (id).vehicle().write_subspeed(0_u8);
                    self.reverse(id);
                }
                return false;
            }
            prev = id;
            id = (id).next();
        }
        if changed {
            first.set_cached_max_curve_speed(u64::from(self.get_curve_speed_limit(first)));
        }
        true
    }
}

impl Game<'_> {
    /// `TryReserveRailTrack(tile, track)` with its default station triggers.
    fn reserve_track(&self, _id: Handle, tile: u32, track: u8) -> u64 {
        u64::from(crate::pbs::Pbs::acquire(self.services).try_reserve(tile, track, true))
    }
    fn crash(&self, id: Handle, flooded: bool) -> u32 {
        let mut victims = 0_u32;
        if (id).front() != 0 {
            victims += 2;
            if !id.flag(8) {
                self.free_train_reservation(id);
            }
            let mut u = id;
            while u != Handle::NONE {
                let tile = (u).vehicle().tile();
                (self.leaves.clear_reservation)(u, tile, self.trackdir(u));
                if is_tile_type(
                    self.services,
                    TileIndex(tile),
                    crate::tile_type::TileType::MP_TUNNELBRIDGE,
                ) {
                    (self.leaves.set_tunnel_res)(
                        u,
                        (self.leaves.other_end)(Handle::NONE, tile) as u32,
                        0,
                    );
                }
                u = (u).next();
            }
            let crossing = self.approaching_crossing(id);
            if crossing != INVALID_TILE {
                self.update_crossing(crossing, true, false);
            }
            (self.leaves.hide_fill)(id);
        }
        victims = victims.wrapping_add(crate::ground_vehicle::crash(
            self.leaves.physics,
            id.vehicle(),
            flooded,
        ));
        id.set_crash_anim_pos(if flooded { 4000 } else { 1 });
        victims
    }
    fn crashed(&self, id: Handle) -> u32 {
        let mut victims = 0;
        if (id).vehicle().status() & 128 == 0 {
            victims = self.crash(id, false);
            (self.leaves.crash_event)(id, victims);
        }
        (self.leaves.reserve_under)(id);
        victims
    }
    fn collision_one(&self, other: Handle, id: Handle) -> u32 {
        if other.track() == DEPOT {
            return 0;
        }
        let v_first: Handle = other.first();
        let v_x: i32 = other.vehicle().x();
        let v_y: i32 = other.vehicle().y();
        let v_z: i32 = other.vehicle().z();
        let v_length: u16 = u16::from(read_field!(
            other.vehicle().ground_cache(),
            cached_veh_length
        ));
        let v_owner: u8 = other.vehicle().owner();
        let t_x: i32 = id.vehicle().x();
        let t_y: i32 = id.vehicle().y();
        let t_z: i32 = id.vehicle().z();
        let t_length: u16 = u16::from(read_field!(id.vehicle().ground_cache(), cached_veh_length));
        let t_owner: u8 = id.vehicle().owner();
        if v_owner != t_owner || v_first == id {
            return 0;
        }
        let x = v_x.wrapping_sub(t_x);
        let y = v_y.wrapping_sub(t_y);
        if ((y.wrapping_add(7) | x.wrapping_add(7)) as u32) & !15 != 0 {
            return 0;
        }
        let min = (i32::from(v_length) + 1) / 2 + (i32::from(t_length) + 1) / 2 - 1;
        if x.wrapping_mul(x).wrapping_add(y.wrapping_mul(y)) > min.wrapping_mul(min)
            || v_z.wrapping_sub(t_z).wrapping_abs() > 5
        {
            return 0;
        }
        let victims = self.crashed(id);
        victims.wrapping_add(self.crashed(v_first))
    }
    fn collision(&self, id: Handle) -> bool {
        if id.track() == DEPOT {
            return false;
        }
        let v_tile: u32 = id.vehicle().tile();
        let v_x: i32 = id.vehicle().x();
        let v_y: i32 = id.vehicle().y();
        let mut victims = 0_u32;
        if id.track() == WORMHOLE {
            self.visit_tile(v_tile, |u| {
                victims = victims.wrapping_add(self.collision_one(u, id));

                false
            });
            self.visit_tile((self.leaves.other_end)(Handle::NONE, v_tile) as u32, |u| {
                victims = victims.wrapping_add(self.collision_one(u, id));

                false
            });
        } else {
            self.visit_near(v_x, v_y, |u| {
                victims = victims.wrapping_add(self.collision_one(u, id));

                false
            });
        }
        if victims == 0 {
            return false;
        }
        crate::witness::hit(crate::witness::TRAIN_COLLISION);
        (self.leaves.crash_news)(id, victims);
        crate::station::spatial::modify_rating_around(
            self.services,
            (id.vehicle()).tile(),
            (id.vehicle()).owner(),
            -160,
            30,
        );
        if (self.services.disaster_sound)() {
            (self.leaves.crash_sound)(id);
        }
        true
    }
    fn platform_occupied(&self, tile: u32) -> bool {
        let delta = (self.leaves.tile_offset_axis)(
            Handle::NONE,
            (self.leaves.station_axis)(Handle::NONE, tile) as u8,
        ) as u32;
        let mut t = tile;
        while (self.leaves.station_compatible)(Handle::NONE, t, tile) != 0 {
            if self.train_on_tile(t) {
                return true;
            }
            t = t.wrapping_sub(delta);
        }
        t = tile.wrapping_add(delta);
        while (self.leaves.station_compatible)(Handle::NONE, t, tile) != 0 {
            if self.train_on_tile(t) {
                return true;
            }
            t = t.wrapping_add(delta);
        }
        false
    }
    fn delete_last(&self, mut id: Handle) {
        crate::witness::hit(crate::witness::TRAIN_CRASH_DELETE);
        let first = (id).first();
        let mut last = id;
        while (id).next() != Handle::NONE {
            last = id;
            id = (id).next();
        }
        // SAFETY: The crash chain remains live until the following delete.
        unsafe { self.vs().set_next_record(last.record, std::ptr::null_mut()) };
        if first != id {
            self.consist_changed(first, 3);
            if first.track() == DEPOT {
                (self.leaves.depot_window)(first, (first).vehicle().tile());
            }
            (id).vehicle()
                .write_last_station((u64::from((first).vehicle().last_station())) as u16);
        }
        let mut tracks = id.track();
        let v_tile: u32 = id.vehicle().tile();
        let v_owner: u8 = id.vehicle().owner();
        let tile = v_tile;
        let owner = v_owner;
        (self.leaves.delete_vehicle)(id);
        if tracks == WORMHOLE {
            tracks = 1 << (u64::from(self.world().tunnel_bridge_dir(tile)) & 1);
        }
        let track = tracks.trailing_zeros() as u8;
        if crate::pbs::Pbs::acquire(self.services).has_reserved_tracks(tile, tracks) {
            crate::pbs::Pbs::acquire(self.services).unreserve(tile, track);
            let mut remaining = 0_u8;
            self.visit_tile(tile, |u| {
                if (u).vehicle().status() & 128 == 0 {
                    return false;
                }
                let t = u.track();
                if t == WORMHOLE {
                    remaining |=
                        1 << (u64::from(self.world().tunnel_bridge_dir((u).vehicle().tile())) & 1);
                } else if t != DEPOT {
                    remaining |= t;
                }

                false
            });
            for t in 0..6 {
                if remaining & (1 << t) != 0 {
                    self.reserve_track(Handle::NONE, tile, t);
                }
            }
        }
        if (self.leaves.is_crossing)(Handle::NONE, tile) != 0 {
            self.update_crossing(tile, true, false);
        }
        if (self.leaves.is_station)(Handle::NONE, tile) != 0 {
            let occupied = self.platform_occupied(tile);
            let dir = (self.leaves.axis_diag)(
                Handle::NONE,
                (self.leaves.station_axis)(Handle::NONE, tile) as u8,
            ) as u8;
            crate::pbs::Pbs::acquire(self.services).platform(tile, dir, occupied);
            crate::pbs::Pbs::acquire(self.services).platform(tile, dir ^ 2, occupied);
        }
        if is_tile_type(
            self.services,
            TileIndex(tile),
            crate::tile_type::TileType::MP_TUNNELBRIDGE,
        ) || (self.leaves.is_depot)(Handle::NONE, tile) != 0
        {
            self.signal_context().segment(tile, u8::MAX, owner);
        } else {
            self.signal_context().both(tile, track, owner);
        }
    }
    fn change_dir_randomly(&self, mut id: Handle) {
        loop {
            if (id).vehicle().status() & 1 == 0 {
                let delta = [7, 0, 0, 1][(self.services.random() & 3) as usize];
                (id).vehicle().write_direction(
                    (u64::from((id).vehicle().direction().wrapping_add(delta) & 7)) as u8,
                );
                if id.track() != WORMHOLE {
                    self.update_occupancy(id);
                    crate::ground_vehicle::inclination(
                        self.leaves.physics,
                        &self.movement,
                        id.vehicle(),
                        false,
                        true,
                    );
                } else {
                    (self.leaves.viewport)(id, 0, 1);
                }
            }
            id = (id).next();
            if id == Handle::NONE {
                break;
            }
        }
    }
    fn handle_crashed(&self, id: Handle) -> bool {
        let state = (id.get_crash_anim_pos() as u16).wrapping_add(1);
        id.set_crash_anim_pos(u64::from(state));
        if state == 4 && (id).vehicle().status() & 1 == 0 {
            crate::effect::openttd_rust_effect_create_rel(
                id.vehicle().0.as_ptr(),
                4,
                4,
                8,
                crate::effectvehicle_func::EV_EXPLOSION_LARGE,
            );
        }
        if state <= 200 {
            let mut r = self.services.random();
            if crate::random_func::chance16_i(1, 7, r) {
                let mut index = (r.wrapping_mul(10) >> 16) as i32;
                let mut u = id;
                loop {
                    index -= 1;
                    if index < 0 {
                        r = self.services.random();
                        crate::effect::openttd_rust_effect_create_rel(
                            u.vehicle().0.as_ptr(),
                            (((r >> 8) & 7) + 2) as i32,
                            (((r >> 16) & 7) + 2) as i32,
                            ((r & 7) + 5) as i32,
                            crate::effectvehicle_func::EV_EXPLOSION_SMALL,
                        );
                        break;
                    }
                    u = (u).next();
                    if u == Handle::NONE {
                        break;
                    }
                }
            }
        }
        let tick = (id).vehicle().tick();
        if state <= 240 && tick & 3 == 0 {
            self.change_dir_randomly(id);
        }
        if state >= 4440 && tick & 31 == 0 {
            let remains = (id).next() != Handle::NONE;
            self.delete_last(id);
            return remains;
        }
        true
    }
    fn stay_depot(&self, id: Handle) -> bool {
        if !Self::whole_in_depot(id) {
            return false;
        }
        let v_tile: u32 = id.vehicle().tile();
        let v_power: u32 = read_field!(id.vehicle().ground_cache(), cached_power);
        if v_power == 0 {
            Self::write_status(id, 2, true);
            (self.leaves.depot_window)(id, v_tile);
            return true;
        }
        if (self.leaves.wait_unbunch)(id) != 0 {
            return true;
        }
        if id.get_force_proceed() == 0 {
            if id.wait_inc() < 37 {
                (self.leaves.train_list)(id);
                return true;
            }
            id.set_wait_counter(0);
        }
        let segment = if (self.leaves.reserve_paths)() {
            (self.leaves.sigseg_pbs)(id)
        } else {
            self.signal_segment(id, v_tile, u8::MAX)
        };
        if id.get_force_proceed() == 0
            && (segment == (self.leaves.sigseg_full)(id)
                || (self.leaves.has_depot_res)(Handle::NONE, v_tile) != 0)
        {
            (self.leaves.train_list)(id);
            return true;
        }
        if (id).vehicle().current_order().kind() == OT_GOTO_DEPOT
            && (id).vehicle().tile() == (id).vehicle().dest_tile()
        {
            if (self.leaves.has_depot_res)(Handle::NONE, (id).vehicle().tile()) == 0 {
                crate::witness::hit(crate::witness::TRAIN_DEPOT_REENTRY);
                self.vs().enter_depot(id.vehicle());
            }
            return true;
        }
        if segment == (self.leaves.sigseg_pbs)(id)
            && !self.try_path_reserve(id, false, false)
            && id.get_force_proceed() == 0
        {
            (self.leaves.train_list)(id);
            self.mark_stuck(id);
            return true;
        }
        (self.leaves.set_depot_res)(id, (id).vehicle().tile(), 1);
        if (self.leaves.show_reservation)(id) != 0 {
            (self.services.mark_dirty)((id).vehicle().tile());
        }
        crate::witness::hit(crate::witness::TRAIN_DEPOT_START);
        self.service_in_depot(id);
        (self.leaves.leave_unbunch)(id);
        (self.leaves.leave_sound)(id);
        (self.leaves.train_list)(id);
        id.set_track(if (id).vehicle().direction() & 2 != 0 {
            2
        } else {
            1
        });
        Self::write_status(id, 1, false);
        (id).vehicle().write_speed(0_u16);
        (self.leaves.viewport)(id, 1, 1);
        self.update_occupancy(id);
        self.signal_segment(id, (id).vehicle().tile(), u8::MAX);
        update_acceleration(id);
        (self.leaves.depot_dirty)(id);
        false
    }
    fn loco(&self, id: Handle, mode: bool) -> bool {
        crate::witness::hit(if mode {
            crate::witness::TRAIN_TICK_SECOND
        } else {
            crate::witness::TRAIN_TICK_FIRST
        });
        if (id).vehicle().status() & 128 != 0 {
            return if mode { true } else { self.handle_crashed(id) };
        }
        if id.get_force_proceed() != 0 {
            id.set_flag(8, false);
            (self.leaves.start_stop_dirty)(id);
        }
        if self.handle_breakdown(id) {
            return true;
        }
        if id.flag(0) && (id).vehicle().speed() == 0 {
            self.reverse(id);
        }
        let v_speed: u16 = id.vehicle().speed();
        let v_status: u8 = id.vehicle().status();
        let v_order: u8 = id.vehicle().current_order().kind();
        if v_status & 2 != 0 && v_speed == 0 {
            return true;
        }
        let valid_order = v_order != OT_NOTHING && v_order != OT_CONDITIONAL;
        let orders = self.vs().orders();
        // SAFETY: The original live train shell survives orders; no owner borrow spans reentry.
        if unsafe {
            crate::orders::openttd_rust_process_orders(id.vehicle().shell().cast(), orders)
        } != 0
            && (self.leaves.check_reverse)(id) != 0
        {
            id.set_wait_counter(0);
            (id).vehicle().write_speed(0_u16);
            (id).vehicle().write_subspeed(0_u8);
            id.set_flag(9, false);
            self.reverse(id);
            return true;
        } else if id.flag(9) {
            let tile = (id).vehicle().tile();
            let mut dir = self.exitdir(id);
            if (self.leaves.is_depot)(Handle::NONE, tile) != 0
                || is_tile_type(
                    self.services,
                    TileIndex(tile),
                    crate::tile_type::TileType::MP_TUNNELBRIDGE,
                )
            {
                dir = u8::MAX;
            }
            if self.signal_segment(id, tile, dir) == (self.leaves.sigseg_pbs)(id)
                || (self.leaves.reserve_paths)()
            {
                self.try_path_reserve(id, true, true);
            }
            id.set_flag(9, false);
        }
        self.vs().handle_loading(id.vehicle(), mode, orders);
        if (id).vehicle().current_order().kind() == OT_LOADING || self.stay_depot(id) {
            return true;
        }
        if !mode {
            if crate::vehicle_effect::openttd_rust_visual_eligible_abi(id.vehicle()) != 0 {
                crate::vehicle_effect::show_visual_effect(
                    id.vehicle(),
                    self.get_current_max_speed(id) as u32,
                );
            }
        }
        if !valid_order && (id).vehicle().current_order().kind() != OT_NOTHING {
            (self.leaves.check_next)(id);
        }
        if !mode && id.flag(8) {
            crate::witness::hit(crate::witness::TRAIN_STUCK_RETRY);
            let wait = u64::from(id.wait_inc());
            let turn = wait % (self.wait_pbs() * (self.leaves.day_ticks)(id)) == 0
                && self.reverse_at_signals() != 0;
            if !turn
                && wait % u64::from((self.leaves.backoff)()) != 0
                && id.get_force_proceed() == 0
            {
                return true;
            }
            if !self.try_path_reserve(id, false, false) {
                if turn {
                    crate::witness::hit(crate::witness::TRAIN_STUCK_REVERSE);
                    self.reverse(id);
                }
                if id.flag(8)
                    && id.get_wait_counter() > 2 * self.wait_pbs() * (self.leaves.day_ticks)(id)
                {
                    if self.lost_warn() != 0
                        && u64::from((id).vehicle().owner()) == (self.leaves.local_company)(id)
                    {
                        (self.leaves.stuck_news)(id);
                    }
                    id.set_wait_counter(0);
                }
                if id.get_force_proceed() == 0 {
                    return true;
                }
                id.set_flag(8, false);
                id.set_wait_counter(0);
                (self.leaves.start_stop_dirty)(id);
            }
        }
        if (id).vehicle().current_order().kind() == OT_LEAVESTATION {
            (self.leaves.order_free)(id);
            (self.leaves.start_stop_dirty)(id);
            return true;
        }
        let mut distance = self.update_speed(id);
        if (id).vehicle().speed() == 0 && (id).vehicle().status() & 2 != 0 {
            id.set_force_proceed(0);
            self.view_dirty(id);
        }
        let mut advance = (self.leaves.advance_distance)(id) as i32;
        if distance < advance {
            if (id).vehicle().speed() == 0 {
                crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
            }
        } else {
            self.line_ends(id, true);
            loop {
                distance = distance.wrapping_sub(advance);
                self.controller(id, Handle::NONE, true);
                if self.collision(id) {
                    break;
                }
                advance = (self.leaves.advance_distance)(id) as i32;
                if distance < advance || (id).vehicle().speed() == 0 {
                    break;
                }
                let v_tile: u32 = id.vehicle().tile();
                let v_order_destination: u16 = id.vehicle().current_order().destination;
                let v_order: u8 = id.vehicle().current_order().kind();
                let v_nonstop: u8 = id.vehicle().current_order().non_stop();
                if (v_order == OT_GOTO_WAYPOINT || v_order == OT_GOTO_STATION)
                    && v_nonstop & 2 != 0
                    && (self.leaves.is_station_any)(Handle::NONE, v_tile) != 0
                    && v_order_destination == self.world().get_station_index(TileIndex(v_tile)).0
                {
                    // SAFETY: The orders owner is resolved at this source call; no owner borrow remains.
                    unsafe {
                        crate::orders::openttd_rust_process_orders(
                            id.vehicle().shell().cast(),
                            self.vs().orders(),
                        )
                    };
                }
            }
            crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
        }
        let mut u = id;
        while u != Handle::NONE {
            if (u).vehicle().status() & 1 == 0 {
                (self.leaves.viewport)(u, 0, 0);
            }
            u = (u).next();
        }
        if (id).vehicle().progress() == 0 {
            (id).vehicle().write_progress((distance as u64) as u8);
        }
        true
    }
    fn tick(&self, id: Handle) -> bool {
        (id).vehicle()
            .write_tick((u64::from((id).vehicle().tick().wrapping_add(1))) as u8);
        let v_speed: u16 = id.vehicle().speed();
        let v_status: u8 = id.vehicle().status();
        let v_running: u8 = id.vehicle().running();
        let v_front: u8 = id.front();
        let v_free_wagon: u8 =
            u8::from(crate::ground_vehicle::is_free_wagon(id.vehicle().subtype()));
        if v_front != 0 {
            if v_status & 2 == 0 || v_speed > 0 {
                (id).vehicle()
                    .write_running((u64::from(v_running.wrapping_add(1))) as u8);
            }
            (id).vehicle()
                .write_order_time(((id).vehicle().order_time().wrapping_add(1) as u64) as i32);
            if !self.loco(id, false) {
                return false;
            }
            return self.loco(id, true);
        } else if v_free_wagon != 0 && v_status & 128 != 0 {
            let state = (id.get_crash_anim_pos() as u16).wrapping_add(1);
            id.set_crash_anim_pos(u64::from(state));
            if state >= 4400 {
                crate::witness::hit(crate::witness::TRAIN_FREE_WAGON_DELETE);
                (self.leaves.delete_vehicle)(id);
                return false;
            }
        }
        true
    }
    fn needs_service(&self, id: Handle) {
        if (self.leaves.servint)(id) == 0 || !self.needs_automatic_servicing(id) {
            return;
        }
        if u64::from(crate::ground_vehicle::chain_in_depot(
            self.leaves.physics,
            id.vehicle(),
        )) != 0
        {
            crate::witness::hit(crate::witness::TRAIN_DEPOT_SERVICE);
            self.service_in_depot(id);
            return;
        }
        let max = self.max_depot_penalty() as u32;
        let depot = (self.leaves.find_depot)(id, max as i32);
        let length = depot.length;
        if length == u32::MAX || length > max {
            if (id).vehicle().current_order().kind() == OT_GOTO_DEPOT {
                (self.leaves.order_dummy)(id);
                (self.leaves.start_stop_dirty)(id);
            }
            return;
        }
        let tile = depot.tile;
        let depot_id = (self.services.depot_index)(tile);
        let v_order_destination: u16 = id.vehicle().current_order().destination;
        let v_order: u8 = id.vehicle().current_order().kind();
        if v_order == OT_GOTO_DEPOT
            && v_order_destination != depot_id
            && !crate::random_func::chance16_i(3, 16, self.services.random())
        {
            return;
        }
        (self.leaves.suppress_implicit)(id);
        (self.leaves.order_depot_service)(id, depot_id);
        (id).vehicle().write_dest((u64::from(tile)) as u32);
        (self.leaves.start_stop_dirty)(id);
    }
    fn economy_day(&self, id: Handle) {
        // SAFETY: The live train and unborrowed owner satisfy `economy_age_vehicle`.
        unsafe { self.maintain(id, maintenance::economy_age_vehicle) };
        let day = (id).vehicle().day().wrapping_add(1);
        (id).vehicle().write_day((u64::from(day)) as u8);
        if day & 7 == 0 {
            // SAFETY: The live train and unborrowed owner satisfy `decrease_vehicle_value`.
            unsafe { self.maintain(id, maintenance::decrease_vehicle_value) };
        }
        if (id).front() != 0 {
            // SAFETY: The live train and unborrowed owner satisfy `check_vehicle_breakdown`.
            unsafe { self.maintain(id, maintenance::check_vehicle_breakdown) };
            self.needs_service(id);
            (self.leaves.check_orders)(id);
            if (id).vehicle().current_order().kind() == OT_GOTO_STATION {
                let tile = (self.leaves.station_dest)(id) as u32;
                if tile != INVALID_TILE {
                    (id).vehicle().write_dest((u64::from(tile)) as u32);
                }
            }
            let running = (id).vehicle().running();
            if running != 0 {
                let cost = self.running_cost(id).saturating_mul(i64::from(running))
                    / ((self.leaves.cost_divisor)(id) as i64);
                // `running_ticks = 0` precedes the company payment; nothing between observes it.
                (id).vehicle().write_running(0);
                (self.leaves.pay_running)(id, (cost as u64) as i64);
                (self.leaves.running_windows)(id);
            }
        }
    }
}
impl Game<'_> {
    fn next_force(&self, id: Handle) -> u8 {
        let v_tile: u32 = id.vehicle().tile();
        let v_status: u8 = id.vehicle().status();
        if v_status & 128 != 0 || id.get_force_proceed() == 2 {
            return 0;
        }
        if !id.flag(8) {
            return if u64::from(crate::ground_vehicle::chain_in_depot(
                self.leaves.physics,
                id.vehicle(),
            )) != 0
            {
                1
            } else {
                2
            };
        }
        let dir = (self.leaves.trackdir_exit)(id, self.trackdir(id)) as u8;
        let tile = self
            .services
            .map()
            .tile_add_by_diag_dir(
                TileIndex(v_tile),
                DiagDirection(dir),
                crate::map_func::native_check,
            )
            .0;
        if tile == INVALID_TILE
            || !is_tile_type(
                self.services,
                TileIndex(tile),
                crate::tile_type::TileType::MP_RAILWAY,
            )
            || (self.leaves.has_signals)(Handle::NONE, tile) == 0
        {
            return 1;
        }
        let tracks = (self.leaves.diag_reaches_tracks)(id, dir) as u8
            & (self.leaves.track_bits)(Handle::NONE, tile) as u8;
        if tracks != 0 && (self.leaves.has_signal)(id, tile, tracks.trailing_zeros() as u8) != 0 {
            2
        } else {
            1
        }
    }
    fn reverse_command(&self, id: Handle, execute: bool, single: bool) -> u8 {
        if single {
            if (id).multiheaded() != 0
                || self.engine(id).callbacks() & crate::newgrf_callbacks::VEHICLE_ARTIC_ENGINE != 0
            {
                return 1;
            }
            let first = (id).first();
            if (self.leaves.stopped_in_depot)(first) == 0 {
                return 2;
            }
            if execute {
                id.set_flag(4, !id.flag(4));
                self.consist_changed(first, 3);
                (self.leaves.reverse_windows)(first);
            }
        } else {
            let v_status: u8 = id.vehicle().status();
            let v_breakdown: u8 = read_field!(id.record, breakdown_ctr);
            let v_front: u8 = id.front();
            if v_front == 0 || v_status & 128 != 0 || v_breakdown != 0 {
                return 3;
            }
            if execute {
                if (id).vehicle().current_order().kind() == OT_LOADING {
                    let last = ((id).last()).vehicle().tile();
                    if (self.leaves.is_station_any)(Handle::NONE, last) == 0
                        || self.world().get_station_index(TileIndex(last))
                            != self
                                .world()
                                .get_station_index(TileIndex((id).vehicle().tile()))
                    {
                        crate::vehicle::visits::leave_station(id.vehicle(), self.vs().visits());
                    }
                }
                id.set_force_proceed(0);
                self.view_dirty(id);
                if self.acc_model() != 0 && (id).vehicle().speed() != 0 {
                    id.set_flag(0, !id.flag(0));
                } else {
                    (id).vehicle().write_speed(0_u16);
                    crate::ground_vehicle::last_speed(self.leaves.physics, id.vehicle());
                    (self.leaves.hide_fill)(id);
                    self.reverse(id);
                }
                (self.leaves.reset_unbunch)(id);
            }
        }
        0
    }
    fn force_command(&self, id: Handle, execute: bool) -> bool {
        if (id).front() == 0 {
            return false;
        }
        if execute {
            id.set_force_proceed(u64::from(self.next_force(id)));
            self.view_dirty(id);
            (self.leaves.reset_unbunch)(id);
        }
        true
    }
}

/// `VehicleRailFlag::LeavingStation`.
pub(crate) const VRF_LEAVING_STATION: u64 = 1 << 9;
/// `VehicleRailFlag::Reversed`.
pub const VRF_REVERSED: u64 = 1 << 7;

/// The train part of `VehicleEnterDepot`: clear the depot reservation,
/// update the segment's signals and arrange the consist.
pub(crate) fn enter_depot(id: Handle, leaves: &Leaves, services: &Services) {
    let g = Game::new(leaves, services);
    let tile = id.vehicle().tile();
    (leaves.set_depot_res)(id, tile, 0);
    if (leaves.show_reservation)(id) != 0 {
        (services.mark_dirty)(tile);
    }
    g.signal_segment(id, tile, crate::direction_type::INVALID_DIAGDIR);
    id.set_wait_counter(0);
    id.set_force_proceed(0);
    id.set_flags(id.get_flags() & !VRF_REVERSED);
    g.consist_changed(id, CCF_ARRANGE);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_consist_changed(
    id: Handle,
    allowed: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.consist_changed(id, allowed);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_curve_limit(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u16 {
    let g = entry_game!(leaves, services);
    g.get_curve_speed_limit(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_current_max_speed(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i32 {
    let g = entry_game!(leaves, services);
    g.get_current_max_speed(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_update_speed(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i32 {
    let g = entry_game!(leaves, services);
    g.update_speed(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_mark_dirty(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.mark_dirty(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_tick(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = entry_game!(leaves, services);
    u8::from(g.tick(id))
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_calendar_day(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    // SAFETY: The live train and unborrowed owner satisfy `age_vehicle`.
    unsafe { g.maintain(id, maintenance::age_vehicle) };
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_economy_day(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.economy_day(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_running_cost(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i64 {
    let g = entry_game!(leaves, services);
    g.running_cost(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_trackdir(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = entry_game!(leaves, services);
    g.trackdir(id)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_mark_stuck(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.mark_stuck(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse_swap(
    id: Handle,
    left: i32,
    right: i32,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.reverse_swap(id, left, right);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_train_on_tile(
    tile: u32,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = entry_game!(leaves, services);
    u8::from(g.train_on_tile(tile))
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_update_crossing(
    tile: u32,
    sound: u8,
    force_bar: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.update_crossing(tile, sound != 0, force_bar != 0);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_adjacent_crossing_dirty(
    tile: u32,
    axis: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.adjacent_crossing_dirty(tile, u64::from(axis));
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_crossing_removed(
    tile: u32,
    axis: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.crossing_removed(tile, u64::from(axis));
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.reverse(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_controller(
    id: Handle,
    nomove: Handle,
    reverse: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = entry_game!(leaves, services);
    u8::from(g.controller(id, nomove, reverse != 0))
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_crash(
    id: Handle,
    flooded: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u32 {
    let g = entry_game!(leaves, services);
    g.crash(id, flooded != 0)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_needs_service(
    id: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    let g = entry_game!(leaves, services);
    g.needs_service(id);
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse_command(
    id: Handle,
    execute: u8,
    single: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = entry_game!(leaves, services);
    g.reverse_command(id, execute != 0, single != 0)
}
/// Invoke synchronous train policy with live serialized services.
/// # Safety
/// The descriptors and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_force_command(
    id: Handle,
    execute: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    let g = entry_game!(leaves, services);
    u8::from(g.force_command(id, execute != 0))
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTrainHandle" ["probe"] => Handle {
        record,
        owner,
    };
    "OpenTTDTrainServices" ["probe"] => Leaves {
        write_colourmap,
        advance_distance,
        all_powered,
        arrival_news,

        axis_diag,
        backoff,
        base_viewport,
        capabilities,
        capacity_error,
        check_next,
        check_orders,
        check_reverse,
        choose_track,
        clear_reservation,
        consist_windows,
        cost_divisor,
        count_chain,
        crash_event,
        crash_news,
        crash_sound,
        crossing_barred,
        crossing_rail_axis,
        crossing_road_axis,
        crossing_sound,
        curve_advantage,
        day_ticks,
        delete_vehicle,
        depot_dir,
        depot_dirty,
        depot_track,
        depot_window,
        diag_axis,
        diag_between,
        diag_reaches_tracks,
        diag_trackdir,
        dir_diag,
        disconnect,
        find_depot,
        first_track,
        has_depot_res,
        has_signal,
        has_signals,
        has_signal_td,
        hide_fill,
        invalidate_grf,
        is_bridge,
        is_crossing,
        is_depot,
        is_plain_rail,
        is_station,
        is_station_any,
        leave_sound,
        leave_unbunch,
        local_company,
        no_90,
        oneway_blocking,
        order_depot_service,
        order_dummy,
        order_free,
        order_max_speed,
        other_end,
        pay_running,
        pbs_signal_type,
        rail_type,
        reserve_paths,
        reserve_under,
        reset_unbunch,
        reverse_windows,
        running_windows,
        servint,
        set_depot_res,
        set_signal_state,
        set_tunnel_res,
        show_reservation,
        signal_context,
        signal_has_pbs,
        signal_pbs,
        signal_type,
        sigseg_full,
        sigseg_pbs,
        start_stop_dirty,
        station_axis,
        station_compatible,
        station_dest,
        stopped_in_depot,
        stop_location,
        stuck_news,
        suppress_implicit,
        tile_offset_axis,
        tile_offset_diag,
        tile_rail_type,
        tile_virt,
        trackdir_exit,
        trackdir_reaches,
        track_bits,
        track_crosses,
        track_direction,
        train_list,
        truncate_cargo,
        veh_exit_dir,
        viewport,
        wagon_override,
        wagon_speed_limits,
        wait_unbunch,
        write_crossing_bar,
        vehicle,
        physics,
    };
    "OpenTTDTrainDepot" ["probe"] => Depot {
        tile,
        length,
    };
}
/// `VehicleRailFlag::PoweredWagon`, from `train.h`.
pub(crate) const FLAG_POWERED_WAGON: u64 = 1 << 3;
/// `VehicleRailFlag::Reversing`, from `train.h`.
pub(crate) const FLAG_REVERSING: u64 = 1 << 0;
/// `VehicleRailFlag::Stuck`, from `train.h`.
pub(crate) const FLAG_STUCK: u64 = 1 << 8;

/// Same owned stop-location callee for tile admission and train speed control.
pub(crate) fn stop_location(
    leaves: &Leaves,
    services: &Services,
    id: Handle,
    station: u16,
    tile: u32,
) -> (i32, i32, i32) {
    Game::new(leaves, services).get_train_stop_location(id, StationID(station), TileIndex(tile))
}
/// `Train::CalcNextVehicleOffset`, shared with depot movement policy.
pub(crate) fn next_offset(id: Handle) -> i32 {
    Game::next_offset(id)
}

impl Game<'_> {
    fn movement_enter(&self, id: Handle, tile: u32, x: i32, y: i32) -> u64 {
        // SAFETY: Same live train as the original synchronous native callback.
        u64::from(unsafe {
            crate::movement::openttd_rust_movement_enter(
                self.movement.get(),
                id.vehicle(),
                tile,
                x,
                y,
            )
        })
    }
    fn movement_tracks(&self, tile: u32, side: u8) -> u64 {
        u64::from(self.movement.get().track_status(tile, 0, 0, side))
    }
}

pub const VRF_POWERED_WAGON: u64 = 1 << 3;
pub const VRF_FLIPPED: u64 = 1 << 4;
