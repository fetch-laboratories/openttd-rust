/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Handling of trains (`train_cmd.cpp`) and the `Train` members of `train.h`.
//!
//! A train part is a [`Handle`]: its canonical vehicle record and the train owner its native
//! shell holds. Both are live for the synchronous entry that resolved them, and deletion
//! invalidates both. Fields are read and written field-sized at the original's observation
//! points; no Rust reference into a record or owner crosses a callback.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
use crate::company_type::Owner;
use crate::direction_func::{
    axis_to_diag_dir, change_dir, diag_dir_to_axis, diag_dir_to_dir, dir_difference,
    dir_to_diag_dir, is_valid_diag_direction, reverse_diag_dir, reverse_dir,
};
use crate::direction_type::{Axis, DiagDirection, DirDiff, Direction};
use crate::engine_type::{EngineID, EngineMiscFlag, EngineMiscFlags, VE_DISABLE_WAGON_POWER};
use crate::ground_vehicle::{AccelStatus, GroundVehicleFlags};
use crate::map_func::{Map, World};
use crate::movement::CachedTable;
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CBID_VEHICLE_LENGTH, VehicleCallbackMask, VehicleCallbackMasks,
};
use crate::newgrf_config::GRFBug;
use crate::newgrf_debug::invalidate_newgrf_inspect_window;
use crate::newgrf_properties::PropertyID;
use crate::news_func::{add_tile_news_item, add_vehicle_advice_news_item, add_vehicle_news_item};
use crate::news_type::{AdviceType, NewsType};
use crate::order_type::{OrderNonStopFlag, OrderStopLocation, OrderType};
use crate::orders::Closest;
use crate::pathfinder_type::FindDepotData;
use crate::pbs::Pbs;
use crate::rail_type::{RailType, RailTypes};
use crate::raw_field::{read_field, write_field};
use crate::script_event_types::{CrashReason, ScriptEvent};
use crate::services::Services;
use crate::signal_func::SigSegState;
use crate::signal_type::SignalType;
use crate::sound_type::{SoundFx, SoundID};
use crate::sprites::PAL_NONE;
use crate::station_type::{StationHadVehicleOfType, StationID, StationType};
use crate::strings_func::get_encoded_string;
use crate::strings_type::StringParameterData::Integer;
use crate::table::strings::{
    STR_ERROR_CAN_T_REVERSE_DIRECTION_RAIL_VEHICLE_MULTIPLE_UNITS,
    STR_ERROR_TRAINS_CAN_ONLY_BE_ALTERED_INSIDE_A_DEPOT, STR_NEWGRF_BROKEN,
    STR_NEWGRF_BROKEN_CAPACITY, STR_NEWS_FIRST_TRAIN_ARRIVAL, STR_NEWS_TRAIN_CRASH,
    STR_NEWS_TRAIN_IS_STUCK,
};
use crate::tile_cmd::{VehicleEnterTileState, VehicleEnterTileStates};
use crate::tile_type::{INVALID_TILE, TILE_SIZE, TileIndex, TileType};
use crate::track_type::{Track, TrackBits, TrackStatus, Trackdir, TrackdirBits};
use crate::train_reservation as reservation;
use crate::train_state::State;
use crate::transport_type::TransportType;
use crate::vehicle::{EngineRef, RailVehicleTypes, VehicleRef, VehicleServices, maintenance};
use crate::vehicle_base::{ClosestDepot, VehState};
use crate::vehicle_gui::VehicleInvalidateWindowData;
use crate::vehicle_type::{AccelerationModel, VEHICLE_LENGTH, VehicleID, VehicleType};
use crate::widgets::vehicle_widget::VehicleViewWidgets;
use crate::window_func::{
    invalidate_window_data, set_window_classes_dirty, set_window_dirty, set_window_widget_dirty,
};
use crate::window_type::{WindowClass, WindowNumber};

/// `VehicleRailFlag`: rail vehicle flags, as bit positions of `VehicleRailFlags`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VehicleRailFlag(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "the enumerator names of the original VehicleRailFlag (train.h)"
)]
impl VehicleRailFlag {
    /// Train is slowing down to reverse.
    pub const Reversing: Self = Self(0);
    /// Wagon is powered.
    pub const PoweredWagon: Self = Self(3);
    /// Reverse the visible direction of the vehicle.
    pub const Flipped: Self = Self(4);
    /// Electric train engine is allowed to run on normal rail.
    pub const AllowedOnNormalRail: Self = Self(6);
    /// Used for vehicle var 0xFE bit 8 (toggled each time the train is reversed, accurate for
    /// first vehicle only).
    pub const Reversed: Self = Self(7);
    /// Train can't get a path reservation.
    pub const Stuck: Self = Self(8);
    /// Train is just leaving a station.
    pub const LeavingStation: Self = Self(9);
}

/// `VehicleRailFlags`: the `VehicleRailFlag` bit set, stored as `uint16_t`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VehicleRailFlags(pub u16);

impl VehicleRailFlags {
    /// Set containing the named element.
    #[must_use]
    pub const fn from_element(element: VehicleRailFlag) -> Self {
        Self(1 << element.0)
    }
    /// Bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: VehicleRailFlag) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }
    /// `BaseBitSet::Any`: whether any element of `other` is set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// `BaseBitSet::Set`: this set with `element` added.
    #[must_use]
    pub const fn set(self, element: VehicleRailFlag) -> Self {
        Self(self.0 | Self::from_element(element).0)
    }
    /// `BaseBitSet::Reset`: this set with `element` removed.
    #[must_use]
    pub const fn reset(self, element: VehicleRailFlag) -> Self {
        Self(self.0 & !Self::from_element(element).0)
    }
    /// `BaseBitSet::Flip`: this set with `element` toggled.
    #[must_use]
    pub const fn flip(self, element: VehicleRailFlag) -> Self {
        Self(self.0 ^ Self::from_element(element).0)
    }
}

/// `TrainForceProceeding`: modes for ignoring signals.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainForceProceeding(pub u8);

impl TrainForceProceeding {
    /// Normal operation.
    pub const TFP_NONE: Self = Self(0);
    /// Proceed till next signal, but ignore being stuck till then. This includes force
    /// leaving depots.
    pub const TFP_STUCK: Self = Self(1);
    /// Ignore next signal, after the signal ignore being stuck.
    pub const TFP_SIGNAL: Self = Self(2);
}

/// `ConsistChangeFlag`: what may change when a consist changes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConsistChangeFlag(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "the enumerator names of the original ConsistChangeFlag (train.h)"
)]
impl ConsistChangeFlag {
    /// Allow vehicles to change length.
    pub const Length: Self = Self(0);
    /// Allow vehicles to change capacity.
    pub const Capacity: Self = Self(1);
}

/// `ConsistChangeFlags`: the `ConsistChangeFlag` bit set, stored as `uint8_t`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConsistChangeFlags(pub u8);

impl ConsistChangeFlags {
    /// Set containing the named element.
    #[must_use]
    pub const fn from_element(element: ConsistChangeFlag) -> Self {
        Self(1 << element.0)
    }
    /// Bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: ConsistChangeFlag) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }
}

/// Valid changes while vehicle is driving, and possibly changing tracks.
pub const CCF_TRACK: ConsistChangeFlags = ConsistChangeFlags(0);
/// Valid changes while vehicle is loading/unloading.
pub const CCF_LOADUNLOAD: ConsistChangeFlags = ConsistChangeFlags(0);
/// Valid changes for autorefitting in stations.
pub const CCF_AUTOREFIT: ConsistChangeFlags =
    ConsistChangeFlags::from_element(ConsistChangeFlag::Capacity);
/// Valid changes for refitting in a depot.
pub const CCF_REFIT: ConsistChangeFlags =
    ConsistChangeFlags::from_element(ConsistChangeFlag::Length).with(
        ConsistChangeFlags::from_element(ConsistChangeFlag::Capacity),
    );
/// Valid changes for arranging the consist in a depot.
pub const CCF_ARRANGE: ConsistChangeFlags =
    ConsistChangeFlags::from_element(ConsistChangeFlag::Length).with(
        ConsistChangeFlags::from_element(ConsistChangeFlag::Capacity),
    );

/// A `Train *`: the canonical record and the train owner of one live train part, or
/// [`Handle::NONE`] for `nullptr`. Each entry keeps the owner live and serialized for the
/// field accesses below.
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
impl Default for Handle {
    fn default() -> Self {
        Self::NONE
    }
}

impl Handle {
    /// `nullptr`.
    pub(crate) const NONE: Self = Self {
        record: std::ptr::null_mut(),
        owner: std::ptr::null_mut(),
    };
    /// The canonical record of this live train part.
    pub(crate) fn vehicle(self) -> VehicleRef {
        // SAFETY: Entries and topology reads supply a live non-null train record.
        unsafe { VehicleRef(std::ptr::NonNull::new_unchecked(self.record)) }
    }
    /// `Train::From(v)`: the handle of a vehicle that is a train, with `Train::From`'s type
    /// assertion.
    pub(crate) fn from_vehicle(v: VehicleRef) -> Self {
        crate::services_assert::assert_with(|| v.vehicle_type() == VehicleType::VEH_TRAIN);
        Self {
            record: v.0.as_ptr(),
            owner: read_field!(v.0.as_ptr(), train),
        }
    }
    /// The handle of a record the original already holds as a `Train *`, or
    /// [`Handle::NONE`] for null.
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
    /// `Train::Next`.
    pub(crate) fn next(self) -> Self {
        Self::from_record(read_field!(self.record, next))
    }
    /// `Train::Previous`.
    pub(crate) fn previous(self) -> Self {
        Self::from_record(read_field!(self.record, previous))
    }
    /// `Train::First`.
    pub(crate) fn first(self) -> Self {
        Self::from_record(self.vehicle().first().0.as_ptr())
    }
    /// `Train::Last`.
    pub(crate) fn last(self) -> Self {
        Self::from_record(self.vehicle().last().0.as_ptr())
    }
    /// `Train::GetNextVehicle`.
    pub(crate) fn get_next_vehicle(self) -> Self {
        self.vehicle()
            .next_vehicle()
            .map_or(Self::NONE, |v| Self::from_record(v.0.as_ptr()))
    }
    /// `Vehicle::IsFrontEngine`.
    pub(crate) fn is_front_engine(self) -> bool {
        self.vehicle().is_front_engine()
    }
    /// `Vehicle::IsArticulatedPart`.
    pub(crate) fn is_articulated_part(self) -> bool {
        self.vehicle().is_articulated_part()
    }
    /// `GroundVehicle::IsMultiheaded`.
    pub(crate) fn is_multiheaded(self) -> bool {
        crate::ground_vehicle::is_multiheaded(self.vehicle().subtype())
    }
    /// `GroundVehicle::IsFreeWagon`.
    pub(crate) fn is_free_wagon(self) -> bool {
        crate::ground_vehicle::is_free_wagon(self.vehicle().subtype())
    }
    /// `GroundVehicle::IsEngine`.
    pub(crate) fn is_engine(self) -> bool {
        crate::ground_vehicle::is_engine(self.vehicle().subtype())
    }
    /// The native shell of this train part.
    pub(crate) fn shell(self) -> *mut std::ffi::c_void {
        VehicleRef::shell_from_record(self.record).cast()
    }

    /// `Train::flags`.
    pub(crate) fn flags(self) -> VehicleRailFlags {
        read_field!(self.owner, flags)
    }
    pub(crate) fn set_flags(self, flags: VehicleRailFlags) {
        write_field!(self.owner, flags, flags);
    }
    /// `Train::crash_anim_pos`.
    pub(crate) fn crash_anim_pos(self) -> u16 {
        read_field!(self.owner, crash_anim_pos)
    }
    pub(crate) fn set_crash_anim_pos(self, crash_anim_pos: u16) {
        write_field!(self.owner, crash_anim_pos, crash_anim_pos);
    }
    /// `Train::wait_counter`.
    pub(crate) fn wait_counter(self) -> u16 {
        read_field!(self.owner, wait_counter)
    }
    pub(crate) fn set_wait_counter(self, wait_counter: u16) {
        write_field!(self.owner, wait_counter, wait_counter);
    }
    /// `Train::compatible_railtypes`.
    pub(crate) fn compatible_railtypes(self) -> RailTypes {
        read_field!(self.owner, compatible_railtypes)
    }
    pub(crate) fn set_compatible_railtypes(self, compatible_railtypes: RailTypes) {
        write_field!(self.owner, compatible_railtypes, compatible_railtypes);
    }
    /// `Train::railtypes`.
    pub(crate) fn railtypes(self) -> RailTypes {
        read_field!(self.owner, railtypes)
    }
    pub(crate) fn set_railtypes(self, railtypes: RailTypes) {
        write_field!(self.owner, railtypes, railtypes);
    }
    /// `Train::track`.
    pub(crate) fn track(self) -> TrackBits {
        read_field!(self.owner, track)
    }
    pub(crate) fn set_track(self, track: TrackBits) {
        write_field!(self.owner, track, track);
    }
    /// `Train::force_proceed`.
    pub(crate) fn force_proceed(self) -> TrainForceProceeding {
        read_field!(self.owner, force_proceed)
    }
    pub(crate) fn set_force_proceed(self, force_proceed: TrainForceProceeding) {
        write_field!(self.owner, force_proceed, force_proceed);
    }
    /// `TrainCache::cached_tilt`.
    pub(crate) fn cached_tilt(self) -> bool {
        read_field!(self.owner, cached_tilt)
    }
    pub(crate) fn set_cached_tilt(self, cached_tilt: bool) {
        write_field!(self.owner, cached_tilt, cached_tilt);
    }
    /// `TrainCache::user_def_data`.
    pub(crate) fn user_def_data(self) -> u8 {
        read_field!(self.owner, user_def_data)
    }
    pub(crate) fn set_user_def_data(self, user_def_data: u8) {
        write_field!(self.owner, user_def_data, user_def_data);
    }
    /// `TrainCache::cached_curve_speed_mod`.
    pub(crate) fn cached_curve_speed_mod(self) -> i16 {
        read_field!(self.owner, cached_curve_speed_mod)
    }
    pub(crate) fn set_cached_curve_speed_mod(self, cached_curve_speed_mod: i16) {
        write_field!(self.owner, cached_curve_speed_mod, cached_curve_speed_mod);
    }
    /// `TrainCache::cached_max_curve_speed`.
    pub(crate) fn cached_max_curve_speed(self) -> u16 {
        read_field!(self.owner, cached_max_curve_speed)
    }
    pub(crate) fn set_cached_max_curve_speed(self, cached_max_curve_speed: u16) {
        write_field!(self.owner, cached_max_curve_speed, cached_max_curve_speed);
    }
    /// `GroundVehicleCache::cached_veh_length`.
    fn cached_veh_length(self) -> u8 {
        read_field!(self.vehicle().ground_cache(), cached_veh_length)
    }
    /// `GroundVehicleCache::cached_total_length`.
    fn cached_total_length(self) -> u16 {
        read_field!(self.vehicle().ground_cache(), cached_total_length)
    }
    /// `Train::IsInDepot`.
    fn is_in_depot(self) -> bool {
        self.track() == TrackBits::TRACK_BIT_DEPOT
    }
    /// `Train::CalcNextVehicleOffset`: calculate the offset from this vehicle's center to the
    /// following center taking the vehicle lengths into account. Returns the offset from
    /// center to center.
    pub(crate) fn calc_next_vehicle_offset(self) -> i32 {
        /* For vehicles with odd lengths the part before the center will be one unit
         * longer than the part after the center. This means we have to round up the
         * length of the next vehicle but may not round the length of the current
         * vehicle. */
        i32::from(self.cached_veh_length()) / 2
            + (if self.next() == Self::NONE {
                0
            } else {
                i32::from(self.next().cached_veh_length()) + 1
            }) / 2
    }
}

/// The services a train entry calls: the shared tables are borrowed through pointers.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Leaves {
    /// Engine capability services, for `Engine::DetermineCapacity`.
    pub(crate) capabilities: &'static crate::engine_capabilities::Services,
    /// The ground vehicle physics table.
    pub physics: &'static crate::ground_vehicle::Services,
    /// The path reservation table of `train_reservation.rs`.
    pub reservation: *const reservation::Leaves,
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

/// `_breakdown_speeds`: maximum speeds for train that is broken down or approaching line end.
const BREAKDOWN_SPEEDS: [u16; 16] = [
    225, 210, 195, 180, 165, 150, 135, 120, 105, 90, 75, 60, 45, 30, 15, 15,
];

/// `_initial_tile_subcoord`: the sub-tile position and direction of a train entering a tile
/// on a track from a direction.
const INITIAL_TILE_SUBCOORD: [[[u8; 3]; 4]; 6] = [
    [[15, 8, 1], [0, 0, 0], [0, 8, 5], [0, 0, 0]],
    [[0, 0, 0], [8, 0, 3], [0, 0, 0], [8, 15, 7]],
    [[0, 0, 0], [7, 0, 2], [0, 7, 6], [0, 0, 0]],
    [[15, 8, 2], [0, 0, 0], [0, 0, 0], [8, 15, 6]],
    [[15, 7, 0], [8, 0, 4], [0, 0, 0], [0, 0, 0]],
    [[0, 0, 0], [0, 0, 0], [0, 8, 4], [7, 15, 0]],
];

/// `AccelerationSlowdownParams`: engine speed changes of an acceleration type.
struct AccelerationSlowdownParams {
    /// Speed change due to a small turn.
    small_turn: u8,
    /// Speed change due to a large turn.
    large_turn: u8,
    /// Fraction to remove when moving up.
    z_up: u8,
    /// Fraction to add when moving down.
    z_down: u8,
}

/// `_accel_slowdown`: speed update fractions for each acceleration type.
const ACCEL_SLOWDOWN: [AccelerationSlowdownParams; 3] = [
    /* normal accel */
    AccelerationSlowdownParams {
        small_turn: (256 / 4) as u8,
        large_turn: (256 / 2) as u8,
        z_up: (256 / 4) as u8,
        z_down: 2,
    }, // normal
    AccelerationSlowdownParams {
        small_turn: (256 / 4) as u8,
        large_turn: (256 / 2) as u8,
        z_up: (256 / 4) as u8,
        z_down: 2,
    }, // monorail
    AccelerationSlowdownParams {
        small_turn: 0,
        large_turn: (256 / 2) as u8,
        z_up: (256 / 4) as u8,
        z_down: 2,
    }, // maglev
];

/// The tables of one synchronous train entry. It holds no vehicle field.
struct Game<'a> {
    leaves: &'a Leaves,
    services: &'a Services,
    vs: &'a VehicleServices,
    movement: CachedTable<'a>,
    /// The canonical map descriptor, loaded once at the entry.
    world: World<'a>,
}

impl<'a> Game<'a> {
    fn new(leaves: &'a Leaves, services: &'a Services) -> Self {
        Self {
            leaves,
            services,
            // SAFETY: The native table names the immutable process-wide base table.
            vs: unsafe { &*leaves.vehicle },
            movement: CachedTable::new(services),
            world: World::new(services),
        }
    }
    /// The canonical map descriptor of the entry.
    fn world(&self) -> &World<'a> {
        &self.world
    }
    /// The map dimensions.
    fn map(&self) -> Map {
        self.world().dimensions
    }
    /// The native assertion handler, as a header helper's precondition check.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        |ok| crate::movement::native_assert(self.services, ok)
    }
    /// The PBS routines.
    fn pbs(&self) -> Pbs<'a> {
        Pbs::acquire(self.services)
    }
    /// The path reservation context of train part `v`.
    fn reservation(&self, v: Handle) -> reservation::Game<'a> {
        // SAFETY: The native table names the immutable process-wide reservation table.
        reservation::Game::new(unsafe { &*self.leaves.reservation }, v.vehicle())
    }
    /// The signal update context.
    fn signals(&self) -> crate::signal::Context {
        crate::signal::context(self.services)
    }
    /// `v->GetEngine()`.
    fn engine(&self, v: Handle) -> EngineRef {
        (self.vs.engine)(v.vehicle())
    }
    /// `v->index`.
    fn index(&self, v: Handle) -> VehicleID {
        // SAFETY: A live record and the immutable binding.
        VehicleID(unsafe { (self.vs.newgrf().id)(v.vehicle().0.as_ptr()) })
    }
    /// `GetVehicleProperty(v, property, orig_value)`.
    fn get_vehicle_property(&self, v: Handle, property: PropertyID, orig_value: i32) -> i32 {
        crate::newgrf_vehicle::get_vehicle_property(
            self.vs.newgrf(),
            v.vehicle(),
            property,
            orig_value,
        )
    }
    /// `v->InvalidateNewGRFCache()`.
    fn invalidate_newgrf_cache(&self, v: Handle) {
        // SAFETY: A live serialized record; the invalidation retains no borrow.
        unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache(self.vs.newgrf(), v.vehicle()) };
    }
    /// `v->UpdatePosition()`.
    fn update_position(&self, v: Handle) {
        // SAFETY: The live shell survives the synchronous index update.
        unsafe { crate::vehicle::occupancy::update_position(self.vs, v.vehicle(), self.map()) };
    }
    /// `v->UpdateViewport(force_update, update_delta)` of a train.
    fn viewport(&self, v: Handle, force_update: bool, update_delta: bool) {
        (self.leaves.physics.viewport)(v.vehicle(), force_update, update_delta);
    }
    /// `v->UpdateInclination(new_tile, update_delta)`.
    fn update_inclination(&self, v: Handle, new_tile: bool, update_delta: bool) -> i32 {
        crate::ground_vehicle::update_inclination(
            self.leaves.physics,
            &self.movement,
            v.vehicle(),
            new_tile,
            update_delta,
        )
    }
    /// `v->SetLastSpeed()`.
    fn set_last_speed(&self, v: Handle) {
        crate::ground_vehicle::set_last_speed(self.leaves.physics, v.vehicle());
    }
    /// `v->CargoChanged()`.
    fn cargo_changed(&self, v: Handle) {
        crate::ground_vehicle::cargo_changed(self.leaves.physics, v.vehicle());
    }
    /// `v->IsChainInDepot()`.
    fn is_chain_in_depot(&self, v: Handle) -> bool {
        crate::ground_vehicle::is_chain_in_depot(self.leaves.physics, v.vehicle())
    }
    /// `v->GetAccelerationType()`.
    fn get_acceleration_type(&self, v: Handle) -> usize {
        usize::from(
            <crate::ground_vehicle::Train as crate::ground_vehicle::GroundVehicleType>::get_acceleration_type(
                self.leaves.physics,
                self.world(),
                v.vehicle(),
            )
            .0,
        )
    }
    /// `VehicleEnterTile(v, tile, x, y)`.
    fn vehicle_enter_tile(
        &self,
        v: Handle,
        tile: TileIndex,
        x: i32,
        y: i32,
    ) -> VehicleEnterTileStates {
        // SAFETY: The same live serialized shell as the original callback; no owner borrow
        // spans it.
        unsafe {
            self.movement
                .get()
                .vehicle_enter_tile(v.vehicle(), tile, x, y)
        }
    }
    /// `GetTileTrackStatus(tile, TRANSPORT_RAIL, 0, side)`.
    fn get_tile_track_status(&self, tile: TileIndex, side: DiagDirection) -> TrackStatus {
        self.movement
            .get()
            .get_tile_track_status(tile, TransportType::TRANSPORT_RAIL, 0, side)
    }
    /// `Rail90DegTurnDisallowed(rt1, rt2)` with the default `_settings_game.pf.forbid_90_deg`.
    fn rail_90_deg_turn_disallowed(&self, rt1: RailType, rt2: RailType) -> bool {
        (self.pbs().0.rail90)(rt1.0, rt2.0, (self.services.forbid_90_deg)())
    }
    /// `GetTileRailType(tile)`.
    fn get_tile_rail_type(&self, tile: TileIndex) -> RailType {
        crate::rail::get_tile_rail_type(self.services, tile)
    }
    /// `IsTileType(tile, type)`.
    fn is_tile_type(&self, tile: TileIndex, kind: TileType) -> bool {
        self.world().is_tile_type(tile, kind)
    }
    /// `IsRailDepotTile(tile)`.
    fn is_rail_depot_tile(&self, tile: TileIndex) -> bool {
        self.world().rail_depot(tile.0)
    }
    /// `GetRailDepotDirection(tile)`.
    fn get_rail_depot_direction(&self, tile: TileIndex) -> DiagDirection {
        DiagDirection(self.world().rail_depot_dir(tile.0))
    }
    /// `IsLevelCrossingTile(tile)`.
    fn is_level_crossing_tile(&self, tile: TileIndex) -> bool {
        self.world().level_crossing_tile(tile.0)
    }
    /// `IsCrossingBarred(tile)`.
    fn is_crossing_barred(&self, tile: TileIndex) -> bool {
        self.world().crossing_barred(tile.0)
    }
    /// `GetCrossingRoadAxis(tile)`.
    fn get_crossing_road_axis(&self, tile: TileIndex) -> Axis {
        Axis(self.world().crossing_road_axis(tile.0))
    }
    /// `IsRailStationTile(tile)`.
    fn is_rail_station_tile(&self, tile: TileIndex) -> bool {
        self.world().is_rail_station_tile(tile)
    }
    /// `GetStationIndex(tile)`.
    fn get_station_index(&self, tile: TileIndex) -> StationID {
        self.world().get_station_index(tile)
    }
    /// `HasSignalOnTrackdir(tile, trackdir)`.
    fn has_signal_on_trackdir(&self, tile: TileIndex, trackdir: Trackdir) -> bool {
        self.world().has_signal_on_trackdir(tile.0, trackdir.0)
    }
    /// `GetSignalType(tile, track)`.
    fn get_signal_type(&self, tile: TileIndex, track: Track) -> SignalType {
        SignalType(self.world().signal_type(tile.0, track.0))
    }
    /// `GetTunnelBridgeDirection(tile)`.
    fn get_tunnel_bridge_direction(&self, tile: TileIndex) -> DiagDirection {
        DiagDirection(self.world().tunnel_bridge_dir(tile.0))
    }
    /// `GetOtherTunnelBridgeEnd(tile)`.
    fn get_other_tunnel_bridge_end(&self, tile: TileIndex) -> TileIndex {
        TileIndex((self.services.other_tunnel_end)(tile.0))
    }
    /// `TileAddByDiagDir(tile, dir)`.
    fn tile_add_by_diag_dir(&self, tile: TileIndex, dir: DiagDirection) -> TileIndex {
        self.map().tile_add_by_diag_dir(tile, dir, self.check())
    }
    /// `TileOffsByDiagDir(dir)`.
    fn tile_offs_by_diag_dir(&self, dir: DiagDirection) -> crate::map_type::TileIndexDiff {
        self.map().tile_offs_by_diag_dir(dir, self.check())
    }
    /// `TileVirtXY(x, y)`.
    fn tile_virt_xy(&self, x: i32, y: i32) -> TileIndex {
        self.map().tile_virt_xy(x as u32, y as u32)
    }
    /// `MarkTileDirtyByTile(tile)`.
    fn mark_tile_dirty_by_tile(&self, tile: TileIndex) {
        (self.services.mark_dirty)(tile.0);
    }
    /// `SndPlayTileFx(sound, tile)`.
    fn snd_play_tile_fx(&self, sound: SoundFx, tile: TileIndex) {
        (self.services.play_sound)(tile.0, SoundID::from(sound).0);
    }
    /// `UpdateSignalsOnSegment(tile, side, owner)`.
    fn update_signals_on_segment(
        &self,
        tile: TileIndex,
        side: DiagDirection,
        owner: Owner,
    ) -> SigSegState {
        self.signals().update_signals_on_segment(tile, side, owner)
    }
    /// `TryReserveRailTrack(tile, t, trigger_stations)`.
    fn try_reserve_rail_track(&self, tile: TileIndex, t: Track, trigger_stations: bool) -> bool {
        self.pbs().try_reserve_rail_track(tile, t, trigger_stations)
    }
    /// `HasCrossingReservation(tile)`.
    fn has_crossing_reservation(&self, tile: TileIndex) -> bool {
        (self.services.crossing_reserved)(tile.0)
    }
    /// `_settings_client.sound.ambient`.
    fn ambient(&self) -> bool {
        // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
        unsafe { self.services.settings().ambient.read() }
    }
    /// `_settings_game.vehicle.train_acceleration_model`.
    fn train_acceleration_model(&self) -> AccelerationModel {
        // SAFETY: As above.
        AccelerationModel(unsafe { self.services.settings().train_acceleration_model.read() })
    }
    /// `_settings_client.gui.lost_vehicle_warn`.
    fn lost_vehicle_warn(&self) -> bool {
        // SAFETY: As above.
        unsafe { self.services.settings().lost_vehicle_warn.read() }
    }
    /// `_settings_game.pf.yapf.maximum_go_to_depot_penalty`.
    fn maximum_go_to_depot_penalty(&self) -> u32 {
        // SAFETY: As above.
        unsafe { self.services.settings().maximum_go_to_depot_penalty.read() }
    }
    /// `_settings_game.pf.reverse_at_signals`.
    fn reverse_at_signals(&self) -> bool {
        // SAFETY: As above.
        unsafe { self.services.settings().reverse_at_signals.read() }
    }
    /// `_settings_game.pf.wait_oneway_signal`.
    fn wait_oneway_signal(&self) -> u8 {
        // SAFETY: As above.
        unsafe { self.services.settings().wait_oneway_signal.read() }
    }
    /// `_settings_game.pf.wait_twoway_signal`.
    fn wait_twoway_signal(&self) -> u8 {
        // SAFETY: As above.
        unsafe { self.services.settings().wait_twoway_signal.read() }
    }
    /// `_settings_game.pf.wait_for_pbs_path`.
    fn wait_for_pbs_path(&self) -> u8 {
        // SAFETY: As above.
        unsafe { self.services.settings().wait_for_pbs_path.read() }
    }
    /// `_settings_game.pf.reserve_paths`.
    fn reserve_paths(&self) -> bool {
        // SAFETY: As above.
        unsafe { self.services.settings().reserve_paths.read() }
    }
    /// `_settings_game.pf.path_backoff_interval`.
    fn path_backoff_interval(&self) -> u8 {
        // SAFETY: As above.
        unsafe { self.services.settings().path_backoff_interval.read() }
    }
    /// `_settings_game.vehicle.wagon_speed_limits`.
    fn wagon_speed_limits(&self) -> bool {
        // SAFETY: As above.
        unsafe { self.services.settings().wagon_speed_limits.read() }
    }
    /// `SetWindowDirty(WC_VEHICLE_VIEW, v->index)`.
    fn set_vehicle_view_dirty(&self, v: Handle) {
        set_window_dirty(
            WindowClass::WC_VEHICLE_VIEW,
            WindowNumber::from(self.index(v)),
        );
    }
    /// `SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP)`.
    fn set_start_stop_dirty(&self, v: Handle) {
        set_window_widget_dirty(
            WindowClass::WC_VEHICLE_VIEW,
            WindowNumber::from(self.index(v)),
            VehicleViewWidgets::WID_VV_START_STOP.0,
        );
    }
    /// `SetWindowDirty(WC_VEHICLE_DETAILS, v->index)`.
    fn set_details_dirty(&self, v: Handle) {
        set_window_dirty(
            WindowClass::WC_VEHICLE_DETAILS,
            WindowNumber::from(self.index(v)),
        );
    }
    /// `InvalidateWindowData(WC_VEHICLE_DEPOT, tile)`.
    fn invalidate_depot_window(tile: TileIndex) {
        invalidate_window_data(
            WindowClass::WC_VEHICLE_DEPOT,
            WindowNumber::from(tile),
            0,
            false,
        );
    }
    /// The company defaults of `owner`'s company.
    fn servint_trains(&self, owner: Owner) -> u16 {
        // SAFETY: Base maintenance names the shared static command and company tables.
        let company = unsafe { &*((*(*(*self.vs.maintenance).transaction).commands).company)() };
        let defaults = ((company.settings)().company_vehicle_defaults)(owner);
        read_field!(defaults, servint_trains)
    }
    /// Run one `vehicle::maintenance` entry on `v`.
    fn maintain<R>(&self, v: Handle, entry: unsafe fn(VehicleRef, &VehicleServices) -> R) -> R {
        // SAFETY: A live train shell; no owner borrow spans the maintenance services.
        unsafe { self.vs.maintain(v.vehicle(), entry) }
    }
}

impl Game<'_> {
    /// `Train::ConsistChanged`: recalculates the cached stuff of a train. Should be called
    /// each time a vehicle is added to/removed from the chain, and when the game is loaded.
    /// Note: this needs to be called too for 'wagon chains' (in the depot, without an engine).
    /// `allowed_changes` is the stuff that is allowed to change.
    fn consist_changed(&self, this: Handle, allowed_changes: ConsistChangeFlags) {
        let mut max_speed = u16::MAX;

        crate::services_assert::assert_with(|| this.is_front_engine() || this.is_free_wagon());

        let rvi_v = self.engine(this);
        let mut first_engine = if this.is_front_engine() {
            this.vehicle().engine_type()
        } else {
            EngineID::INVALID
        };
        write_field!(this.vehicle().ground_cache(), cached_total_length, 0);
        this.set_compatible_railtypes(RailTypes::EMPTY);

        let mut train_can_tilt = true;
        let mut min_curve_speed_mod = i16::MAX;

        let mut u = this;
        while u != Handle::NONE {
            let rvi_u = self.engine(u);

            /* Check the this->first cache. */
            crate::services_assert::assert_with(|| u.first() == this);

            /* update the 'first engine' */
            write_field!(
                u.vehicle().ground_cache(),
                first_engine,
                if this == u {
                    EngineID::INVALID
                } else {
                    first_engine
                }
                .0
            );
            u.set_railtypes(RailTypes(rvi_u.railtypes()));

            if u.is_engine() {
                first_engine = u.vehicle().engine_type();
            }

            /* Set user defined data to its default value */
            u.set_user_def_data(rvi_u.train_user_data());
            self.invalidate_newgrf_cache(this);
            self.invalidate_newgrf_cache(u);
            u = u.next();
        }

        let mut u = this;
        while u != Handle::NONE {
            /* Update user defined data (must be done before other properties) */
            u.set_user_def_data(self.get_vehicle_property(
                u,
                PropertyID::PROP_TRAIN_USER_DATA,
                i32::from(u.user_def_data()),
            ) as u8);
            self.invalidate_newgrf_cache(this);
            self.invalidate_newgrf_cache(u);
            u = u.next();
        }

        let mut u = this;
        while u != Handle::NONE {
            let e_u = self.engine(u);

            if !EngineMiscFlags(e_u.misc()).test(EngineMiscFlag::RailTilts) {
                train_can_tilt = false;
            }
            min_curve_speed_mod = min_curve_speed_mod.min(self.get_curve_speed_modifier(u));

            /* Cache wagon override sprite group. nullptr is returned if there is none */
            // SAFETY: A live serialized shell; evaluation retains no owner reference.
            unsafe { crate::newgrf_vehicle::cache_override(self.vs, u.vehicle()) };

            /* Reset colour map */
            (self.vs.set_colourmap)(u.vehicle(), PAL_NONE);

            /* Update powered-wagon-status and visual effect */
            crate::vehicle_effect::update_visual_effect(u.vehicle(), true);

            if rvi_v.wagon_power() != 0
                && e_u.rail_kind() == RailVehicleTypes::WAGON
                && crate::newgrf_vehicle::uses_wagon_override(self.vs.newgrf(), u.vehicle())
                && !crate::bitmath_func::has_bit(
                    u.vehicle().cached_vis_effect(),
                    VE_DISABLE_WAGON_POWER,
                )
            {
                /* wagon is powered */
                u.set_flags(u.flags().set(VehicleRailFlag::PoweredWagon)); // cache 'powered' status
            } else {
                u.set_flags(u.flags().reset(VehicleRailFlag::PoweredWagon));
            }

            if !u.is_articulated_part() {
                /* Do not count powered wagons for the compatible railtypes, as wagons always
                have railtype normal */
                if e_u.ground_power(VehicleType::VEH_TRAIN) > 0 {
                    this.set_compatible_railtypes(this.compatible_railtypes().with(RailTypes(
                        (self.services.all_powered_railtypes)(u.railtypes().0),
                    )));
                }

                /* Some electric engines can be allowed to run on normal rail. It happens to all
                 * existing electric engines when elrails are disabled and then re-enabled */
                if u.flags().test(VehicleRailFlag::AllowedOnNormalRail) {
                    let rail = RailTypes::from_element(RailType::RAILTYPE_RAIL);
                    u.set_railtypes(u.railtypes().with(rail));
                    u.set_compatible_railtypes(u.compatible_railtypes().with(rail));
                }

                /* max speed is the minimum of the speed limits of all vehicles in the consist */
                if (e_u.rail_kind() != RailVehicleTypes::WAGON || self.wagon_speed_limits())
                    && !crate::newgrf_vehicle::uses_wagon_override(self.vs.newgrf(), u.vehicle())
                {
                    let speed = self.get_vehicle_property(
                        u,
                        PropertyID::PROP_TRAIN_SPEED,
                        i32::from(e_u.train_speed()),
                    ) as u16;
                    if speed != 0 {
                        max_speed = speed.min(max_speed);
                    }
                }
            }

            // SAFETY: The current consist retains this shell during capacity callbacks.
            let new_cap = unsafe {
                crate::engine_capabilities::determine_capacity(
                    self.leaves.capabilities,
                    e_u.0.as_ptr(),
                    u.vehicle().0.as_ptr(),
                    std::ptr::null_mut(),
                )
            } as u16;
            if allowed_changes.test(ConsistChangeFlag::Capacity) {
                /* Update vehicle capacity. */
                if u.vehicle().cargo_cap() > new_cap {
                    self.truncate_cargo(u, new_cap);
                }
                u.vehicle()
                    .set_refit_cap(new_cap.min(u.vehicle().refit_cap()));
                u.vehicle().set_cargo_cap(new_cap);
            } else {
                /* Verify capacity hasn't changed. */
                if new_cap != u.vehicle().cargo_cap() {
                    (self.vs.show_newgrf_vehicle_error)(
                        u.vehicle().engine_type().0,
                        STR_NEWGRF_BROKEN.0,
                        STR_NEWGRF_BROKEN_CAPACITY.0,
                        GRFBug::VehCapacity,
                        true,
                    );
                }
            }
            u.vehicle()
                .set_cached_cargo_age_period(self.get_vehicle_property(
                    u,
                    PropertyID::PROP_TRAIN_CARGO_AGE_PERIOD,
                    i32::from(e_u.cargo_age()),
                ) as u16);

            /* check the vehicle length (callback) */
            let mut veh_len = CALLBACK_FAILED;
            if e_u.grf_version(self.services) >= 8 {
                /* Use callback 36 */
                veh_len = self.get_vehicle_property(
                    u,
                    PropertyID::PROP_TRAIN_SHORTEN_FACTOR,
                    i32::from(CALLBACK_FAILED),
                ) as u16;

                if veh_len != CALLBACK_FAILED && u32::from(veh_len) >= VEHICLE_LENGTH {
                    self.vs
                        .unknown_callback_result(e_u, CBID_VEHICLE_LENGTH, veh_len);
                }
            } else if VehicleCallbackMasks(e_u.callbacks()).test(VehicleCallbackMask::Length) {
                /* Use callback 11 */
                veh_len = crate::newgrf_vehicle::get_vehicle_callback(
                    self.vs.newgrf(),
                    CBID_VEHICLE_LENGTH,
                    0,
                    0,
                    u.vehicle().engine_type(),
                    Some(u.vehicle()),
                );
            }
            if veh_len == CALLBACK_FAILED {
                veh_len = u16::from(e_u.train_shorten());
            }
            let veh_len = (VEHICLE_LENGTH as i32
                - crate::math::clamp(
                    i32::from(veh_len),
                    0,
                    VEHICLE_LENGTH as i32 - 1,
                    self.check(),
                )) as u8;

            if allowed_changes.test(ConsistChangeFlag::Length) {
                /* Update vehicle length. */
                write_field!(u.vehicle().ground_cache(), cached_veh_length, veh_len);
            } else {
                /* Verify length hasn't changed. */
                if veh_len != u.cached_veh_length() {
                    (self.vs.length_changed)(u.vehicle());
                }
            }

            write_field!(
                this.vehicle().ground_cache(),
                cached_total_length,
                this.cached_total_length()
                    .wrapping_add(u16::from(u.cached_veh_length()))
            );
            self.invalidate_newgrf_cache(this);
            self.invalidate_newgrf_cache(u);
            u = u.next();
        }

        /* store consist weight/max speed in cache */
        this.vehicle().set_cached_max_speed(max_speed);
        this.set_cached_tilt(train_can_tilt);
        this.set_cached_curve_speed_mod(min_curve_speed_mod);
        this.set_cached_max_curve_speed(self.get_curve_speed_limit(this));

        /* recalculate cached weights and power too (we do this *after* the rest, so it is known which wagons are powered and need extra weight added) */
        self.cargo_changed(this);

        if this.is_front_engine() {
            update_acceleration(this);
            self.set_details_dirty(this);
            let index = WindowNumber::from(self.index(this));
            invalidate_window_data(
                WindowClass::WC_VEHICLE_REFIT,
                index,
                VehicleInvalidateWindowData::VIWD_CONSIST_CHANGED.0,
                false,
            );
            invalidate_window_data(
                WindowClass::WC_VEHICLE_ORDERS,
                index,
                VehicleInvalidateWindowData::VIWD_CONSIST_CHANGED.0,
                false,
            );
            invalidate_newgrf_inspect_window(
                crate::newgrf::GrfSpecFeature::GSF_TRAINS,
                self.index(this).0,
            );

            /* If the consist is changed while in a depot, the vehicle view window must be invalidated to update the availability of refitting. */
            invalidate_window_data(
                WindowClass::WC_VEHICLE_VIEW,
                index,
                VehicleInvalidateWindowData::VIWD_CONSIST_CHANGED.0,
                false,
            );
        }
    }

    /// `u->cargo.Truncate(max_move)`.
    fn truncate_cargo(&self, u: Handle, max_move: u16) {
        let refit = self.vs.visits().refit;
        let owner = (refit.scheduler.cargo_owner)(u.vehicle());
        // SAFETY: The live record's canonical cargo list; no borrow survives the truncation.
        unsafe { crate::cargo_storage::truncate_vehicle(owner, *refit.cargo, u32::from(max_move)) };
    }

    /// `Train::GetCurveSpeedModifier`: get the curve speed modifier of a single part of a
    /// train.
    #[inline]
    fn get_curve_speed_modifier(&self, v: Handle) -> i16 {
        crate::newgrf_vehicle::get_engine_property(
            self.vs.newgrf(),
            v.vehicle().engine_type(),
            PropertyID::PROP_TRAIN_CURVE_SPEED_MOD,
            i32::from(self.engine(v).train_curve_mod()),
            Some(v.vehicle()),
            true,
        ) as i16
    }

    /// `GetTrainStopLocation`: get the stop location of (the center) of the front vehicle of
    /// a train at a platform of a station. `station_id` is the ID of the station where we're
    /// stopping and `tile` the tile where the vehicle currently is. Returns the location,
    /// calculated from the begin of the station, to stop at; the amount of 1/16th tiles in
    /// front of the train; and the station length in 1/16th tiles.
    fn get_train_stop_location(
        &self,
        station_id: StationID,
        tile: TileIndex,
        v: Handle,
    ) -> (i32, i32, i32) {
        let world = self.world();
        let st = self.station_get(station_id);
        let station_ahead = (crate::station::get_platform_length_in_direction(
            world,
            tile,
            dir_to_diag_dir(v.vehicle().direction(), self.check()),
        ) as i32)
            .wrapping_mul(TILE_SIZE as i32);
        let station_length = (crate::station::get_platform_length(world, st, tile) as i32)
            .wrapping_mul(TILE_SIZE as i32);

        /* Default to the middle of the station for stations stops that are not in
         * the order list like intermediate stations when non-stop is disabled */
        let mut osl = OrderStopLocation::Middle;
        if i32::from(v.cached_total_length()) >= station_length {
            /* The train is longer than the station, make it stop at the far end of the platform */
            osl = OrderStopLocation::FarEnd;
        } else if v
            .vehicle()
            .current_order()
            .is_type(OrderType::OT_GOTO_STATION)
            && v.vehicle().current_order().destination().to_station_id() == station_id
        {
            osl = v.vehicle().current_order().stop_location();
        }

        /* The stop location of the FRONT! of the train */
        let stop = match osl {
            OrderStopLocation::NearEnd => i32::from(v.cached_total_length()),
            OrderStopLocation::Middle => station_length
                .wrapping_sub(station_length.wrapping_sub(i32::from(v.cached_total_length())) / 2),
            OrderStopLocation::FarEnd => station_length,
            _ => crate::services_assert::not_reached(),
        };

        /* Subtract half the front vehicle length of the train so we get the real
         * stop location of the train. */
        (
            stop.wrapping_sub((i32::from(v.cached_veh_length()) + 1) / 2),
            station_ahead,
            station_length,
        )
    }

    /// `Station::Get(station)`, through the station table's checked lookup.
    fn station_get(&self, station: StationID) -> *mut crate::station::BaseStation {
        self.station_lookup(station).base
    }
    /// `Station::Get(station)`, resolving both the station and its base.
    fn station_lookup(&self, station: StationID) -> crate::station_service::LoadingStation {
        // SAFETY: The process-lifetime tables resolve the original `Station::Get` once.
        let stations = unsafe { &*(*(self.services.station_catchment)()).station };
        (stations.station)(station.0)
    }

    /// `Train::GetCurveSpeedLimit`: computes train speed limit caused by curves. Returns the
    /// imposed speed limit.
    fn get_curve_speed_limit(&self, this: Handle) -> u16 {
        const ABSOLUTE_MAX_SPEED: i32 = u16::MAX as i32;

        crate::services_assert::assert_with(|| this.first() == this);

        let mut max_speed = ABSOLUTE_MAX_SPEED;

        if self.train_acceleration_model() == AccelerationModel::AM_ORIGINAL {
            return max_speed as u16;
        }

        let mut curvecount = [0_i32; 2];

        /* first find the curve speed limit */
        let mut numcurve = 0_i32;
        let mut sum = 0_i32;
        let mut pos = 0_i32;
        let mut lastpos = -1_i32;
        let mut u = this;
        let mut next = u.next();
        while next != Handle::NONE {
            'part: {
                let this_dir = u.vehicle().direction();
                let next_dir = next.vehicle().direction();

                let dirdiff = dir_difference(this_dir, next_dir, self.check());
                if dirdiff == DirDiff::DIRDIFF_SAME {
                    break 'part;
                }

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

                /* if we have a 90 degree turn, fix the speed limit to 60 */
                if dirdiff == DirDiff::DIRDIFF_90LEFT || dirdiff == DirDiff::DIRDIFF_90RIGHT {
                    max_speed = 61;
                }
            }
            u = next;
            pos = pos.wrapping_add(i32::from(u.cached_veh_length()));
            next = u.next();
        }

        if numcurve > 0 && max_speed > 88 {
            if curvecount[0] == 1 && curvecount[1] == 1 {
                max_speed = ABSOLUTE_MAX_SPEED;
            } else {
                sum = crate::math::ceil_div(sum as u32, VEHICLE_LENGTH) as i32;
                sum /= numcurve;
                let check = self.check();
                max_speed = 232
                    - (13 - crate::math::clamp(sum, 1, 12, check))
                        * (13 - crate::math::clamp(sum, 1, 12, check));
            }
        }

        if max_speed != ABSOLUTE_MAX_SPEED {
            /* Apply the current railtype's curve speed advantage */
            let curve_speed = (self.services.rail_type_curve_speed)(RailType(
                self.world().rail_type(this.vehicle().tile().0),
            ));
            max_speed =
                max_speed.wrapping_add((max_speed / 2).wrapping_mul(i32::from(curve_speed)));

            if this.cached_tilt() {
                /* Apply max_speed bonus of 20% for a tilting train */
                max_speed = max_speed.wrapping_add(max_speed / 5);
            }

            /* Apply max_speed modifier (cached value is fixed-point binary with 8 fractional bits)
             * and clamp the result to an acceptable range. */
            max_speed = max_speed.wrapping_add(
                max_speed.wrapping_mul(i32::from(this.cached_curve_speed_mod())) / 256,
            );
            max_speed = crate::math::clamp(max_speed, 2, ABSOLUTE_MAX_SPEED, self.check());
        }

        max_speed as u16
    }

    /// `Train::GetCurrentMaxSpeed`: calculates the maximum speed of the vehicle under its
    /// current conditions.
    fn get_current_max_speed(&self, this: Handle) -> i32 {
        let mut max_speed = if self.train_acceleration_model() == AccelerationModel::AM_ORIGINAL {
            i32::from(read_field!(
                this.vehicle().ground_cache(),
                cached_max_track_speed
            ))
        } else {
            i32::from(this.cached_max_curve_speed())
        };

        if self.train_acceleration_model() == AccelerationModel::AM_REALISTIC
            && self.is_rail_station_tile(this.vehicle().tile())
        {
            let sid = self.get_station_index(this.vehicle().tile());
            if crate::movement::should_stop_at_station(self.services, this.vehicle(), sid) {
                let (stop_at, station_ahead, station_length) =
                    self.get_train_stop_location(sid, this.vehicle().tile(), this);

                /* The distance to go is whatever is still ahead of the train minus the
                 * distance from the train's stop location to the end of the platform */
                let distance_to_go = station_ahead / TILE_SIZE as i32
                    - station_length.wrapping_sub(stop_at) / TILE_SIZE as i32;

                if distance_to_go > 0 {
                    let mut st_max_speed = 120;

                    let cur_speed = i32::from(this.vehicle().cur_speed());
                    let delta_v = cur_speed / (distance_to_go + 1);
                    if max_speed > cur_speed - delta_v {
                        st_max_speed = cur_speed - (delta_v / 10);
                    }

                    st_max_speed = st_max_speed.max(25_i32.wrapping_mul(distance_to_go));
                    max_speed = max_speed.min(st_max_speed);
                }
            }
        }

        let mut u = this;
        while u != Handle::NONE {
            if self.train_acceleration_model() == AccelerationModel::AM_REALISTIC
                && u.track() == TrackBits::TRACK_BIT_DEPOT
            {
                max_speed = max_speed.min(61);
                break;
            }

            /* Vehicle is on the middle part of a bridge. */
            if u.track() == TrackBits::TRACK_BIT_WORMHOLE
                && !u.vehicle().vehstatus().test(VehState::Hidden)
            {
                max_speed = max_speed.min(i32::from((self.services.bridge_speed)(
                    u.vehicle().tile().0,
                )));
            }
            u = u.next();
        }

        max_speed = max_speed.min(i32::from(this.vehicle().current_order().max_speed()));
        max_speed.min(i32::from(read_field!(
            this.vehicle().ground_cache(),
            cached_max_track_speed
        )))
    }

    /// `MarkTrainAsStuck`: mark a train as stuck and stop it if it isn't stopped right now.
    fn mark_train_as_stuck(&self, v: Handle) {
        if !v.flags().test(VehicleRailFlag::Stuck) {
            /* It is the first time the problem occurred, set the "train stuck" flag. */
            v.set_flags(v.flags().set(VehicleRailFlag::Stuck));

            v.set_wait_counter(0);

            /* Stop train */
            v.vehicle().set_cur_speed(0);
            v.vehicle().set_subspeed(0);
            self.set_last_speed(v);

            self.set_start_stop_dirty(v);
        }
    }

    /// `UpdateStatusAfterSwap`: updates some variables after swapping the vehicle.
    #[inline]
    fn update_status_after_swap(&self, v: Handle) {
        /* Reverse the direction. */
        if v.track() != TrackBits::TRACK_BIT_DEPOT {
            v.vehicle()
                .set_direction(reverse_dir(v.vehicle().direction(), self.check()));
        }

        /* Call the proper EnterTile function unless we are in a wormhole. */
        if v.track() == TrackBits::TRACK_BIT_WORMHOLE {
            crate::witness::hit(crate::witness::TRAIN_WORMHOLE_SWAP);
            /* VehicleEnter_TunnelBridge() sets TRACK_BIT_WORMHOLE when the vehicle
             * is on the last bit of the bridge head (frame == TILE_SIZE - 1).
             * If we were swapped with such a vehicle, we have set TRACK_BIT_WORMHOLE,
             * when we shouldn't have. Check if this is the case. */
            let vt = self.tile_virt_xy(v.vehicle().x_pos(), v.vehicle().y_pos());
            if self.is_tile_type(vt, TileType::MP_TUNNELBRIDGE) {
                self.vehicle_enter_tile(v, vt, v.vehicle().x_pos(), v.vehicle().y_pos());
                if v.track() != TrackBits::TRACK_BIT_WORMHOLE
                    && self.world().bridge_tile(v.vehicle().tile().0)
                {
                    /* We have just left the wormhole, possibly set the
                     * "goingdown" bit. UpdateInclination() can be used
                     * because we are at the border of the tile. */
                    self.update_position(v);
                    self.update_inclination(v, true, true);
                    return;
                }
            }
        } else {
            self.vehicle_enter_tile(
                v,
                v.vehicle().tile(),
                v.vehicle().x_pos(),
                v.vehicle().y_pos(),
            );
        }

        self.update_position(v);
        self.viewport(v, true, true);
    }

    /// `ReverseTrainSwapVeh`: swap vehicles `left` and `right` (the original's `l` and `r`)
    /// in consist `v`, and reverse their direction. Both are vehicle indices in the consist.
    fn reverse_train_swap_veh(&self, v: Handle, mut left: i32, mut right: i32) {
        /* locate vehicles to swap */
        let mut a = v;
        while left != 0 {
            a = a.next();
            left -= 1;
        }
        let mut b = v;
        while right != 0 {
            b = b.next();
            right -= 1;
        }

        if a == b {
            /* Swap GVF_GOINGUP_BIT/GVF_GOINGDOWN_BIT.
             * This is a little bit redundant way, a->gv_flags will
             * be (re)set twice, but it reduces code duplication */
            swap_train_flags(a.vehicle().ground(), a.vehicle().ground());
            self.update_status_after_swap(a);
        } else {
            /* swap the hidden bits */
            {
                let a_hidden = a.vehicle().vehstatus().test(VehState::Hidden);
                let b_hidden = b.vehicle().vehstatus().test(VehState::Hidden);
                b.vehicle().set_hidden(a_hidden);
                a.vehicle().set_hidden(b_hidden);
            }

            let track = a.track();
            a.set_track(b.track());
            b.set_track(track);
            let direction = a.vehicle().direction();
            a.vehicle().set_direction(b.vehicle().direction());
            b.vehicle().set_direction(direction);
            let x_pos = a.vehicle().x_pos();
            a.vehicle().set_x_pos(b.vehicle().x_pos());
            b.vehicle().set_x_pos(x_pos);
            let y_pos = a.vehicle().y_pos();
            a.vehicle().set_y_pos(b.vehicle().y_pos());
            b.vehicle().set_y_pos(y_pos);
            let tile = a.vehicle().tile();
            a.vehicle().set_tile(b.vehicle().tile());
            b.vehicle().set_tile(tile);
            let z_pos = a.vehicle().z_pos();
            a.vehicle().set_z_pos(b.vehicle().z_pos());
            b.vehicle().set_z_pos(z_pos);

            swap_train_flags(a.vehicle().ground(), b.vehicle().ground());

            self.update_status_after_swap(a);
            self.update_status_after_swap(b);
        }
    }

    /// `TrainOnCrossing`: check if a level crossing tile has a train on it.
    /// Precondition: `tile` is a level crossing.
    fn train_on_crossing(&self, tile: TileIndex) -> bool {
        crate::services_assert::assert_with(|| self.is_level_crossing_tile(tile));

        crate::vehicle_func::has_vehicle_on_tile(self.vs, self.map(), tile, is_train)
    }

    /// `TrainApproachingCrossingEnum`: checks if a train is approaching a rail-road crossing.
    /// `v` is the vehicle on tile and `tile` the tile with crossing we are testing.
    #[inline]
    fn train_approaching_crossing_enum(&self, v: VehicleRef, tile: TileIndex) -> bool {
        if v.vehicle_type() != VehicleType::VEH_TRAIN || v.vehstatus().test(VehState::Crashed) {
            return false;
        }

        let t = Handle::from_vehicle(v);
        if !t.is_front_engine() {
            return false;
        }

        self.train_approaching_crossing_tile(t) == tile
    }

    /// `TrainApproachingCrossing`: finds a vehicle approaching rail-road crossing.
    /// Precondition: `tile` is a rail-road crossing.
    #[inline]
    fn train_approaching_crossing(&self, tile: TileIndex) -> bool {
        crate::services_assert::assert_with(|| self.is_level_crossing_tile(tile));

        let mut dir = axis_to_diag_dir(
            crate::road_map::get_crossing_rail_axis(self.world(), tile),
            self.check(),
        );
        let mut tile_from = tile + self.tile_offs_by_diag_dir(dir);

        if crate::vehicle_func::has_vehicle_on_tile(self.vs, self.map(), tile_from, |v| {
            self.train_approaching_crossing_enum(v, tile)
        }) {
            return true;
        }

        dir = reverse_diag_dir(dir, self.check());
        tile_from = tile + self.tile_offs_by_diag_dir(dir);

        crate::vehicle_func::has_vehicle_on_tile(self.vs, self.map(), tile_from, |v| {
            self.train_approaching_crossing_enum(v, tile)
        })
    }

    /// `CheckLevelCrossing`: check if a level crossing should be barred.
    #[inline]
    fn check_level_crossing(&self, tile: TileIndex) -> bool {
        /* reserved || train on crossing || train approaching crossing */
        self.has_crossing_reservation(tile)
            || self.train_on_crossing(tile)
            || self.train_approaching_crossing(tile)
    }

    /// `UpdateLevelCrossingTile`: sets a level crossing tile to the correct state. `sound` is
    /// whether to play sound and `force_barred` whether to set the crossing to barred.
    /// Precondition: `tile` is a rail-road crossing.
    fn update_level_crossing_tile(&self, tile: TileIndex, sound: bool, force_barred: bool) {
        crate::services_assert::assert_with(|| self.is_level_crossing_tile(tile));

        /* We force the crossing to be barred when an adjacent crossing is barred, otherwise let it decide for itself. */
        let set_barred = force_barred || self.check_level_crossing(tile);

        /* The state has changed */
        if set_barred != self.is_crossing_barred(tile) {
            crate::witness::hit(if set_barred {
                crate::witness::TRAIN_CROSS_BAR
            } else {
                crate::witness::TRAIN_CROSS_UNBAR
            });
            if set_barred && sound && self.ambient() {
                self.snd_play_tile_fx(SoundFx::SND_0E_LEVEL_CROSSING, tile);
            }
            self.world().set_crossing_barred(tile, set_barred);
            self.mark_tile_dirty_by_tile(tile);
        }
    }

    /// `UpdateLevelCrossing`: update a level crossing to barred or open (crossing may include
    /// multiple adjacent tiles). `tile` causes the update; `sound` says whether to play
    /// sound; `force_bar` forces the crossing to be barred.
    fn update_level_crossing(&self, tile: TileIndex, sound: bool, force_bar: bool) {
        if !self.is_level_crossing_tile(tile) {
            return;
        }

        let mut forced_state = force_bar;

        let axis = self.get_crossing_road_axis(tile);
        let dir1 = axis_to_diag_dir(axis, self.check());
        let dir2 = reverse_diag_dir(dir1, self.check());

        /* Check if an adjacent crossing is barred. */
        for dir in [dir1, dir2] {
            let mut t = tile;
            while !forced_state
                && t.0 < self.services.map_size()
                && self.is_level_crossing_tile(t)
                && self.get_crossing_road_axis(t) == axis
            {
                forced_state |= self.check_level_crossing(t);
                t = self.tile_add_by_diag_dir(t, dir);
            }
        }

        /* Now that we know whether all tiles in this crossing should be barred or open,
         * we need to update those tiles. We start with the tile itself, then look along the road axis. */
        self.update_level_crossing_tile(tile, sound, forced_state);
        for dir in [dir1, dir2] {
            let mut t = self.tile_add_by_diag_dir(tile, dir);
            while t.0 < self.services.map_size()
                && self.is_level_crossing_tile(t)
                && self.get_crossing_road_axis(t) == axis
            {
                self.update_level_crossing_tile(t, sound, forced_state);
                t = self.tile_add_by_diag_dir(t, dir);
            }
        }
    }

    /// `MarkDirtyAdjacentLevelCrossingTiles`: find adjacent level crossing tiles in this
    /// multi-track crossing and mark them dirty. `tile` causes the update.
    fn mark_dirty_adjacent_level_crossing_tiles(&self, tile: TileIndex, road_axis: Axis) {
        let dir1 = axis_to_diag_dir(road_axis, self.check());
        let dir2 = reverse_diag_dir(dir1, self.check());
        for dir in [dir1, dir2] {
            let t = self.tile_add_by_diag_dir(tile, dir);
            if t.0 < self.services.map_size()
                && self.is_level_crossing_tile(t)
                && self.get_crossing_road_axis(t) == road_axis
            {
                self.mark_tile_dirty_by_tile(t);
            }
        }
    }

    /// `UpdateAdjacentLevelCrossingTilesOnLevelCrossingRemoval`: update adjacent level
    /// crossing tiles in this multi-track crossing, due to removal of a level crossing tile.
    /// `tile` is the crossing tile which has been or is about to be removed, and which caused
    /// the update.
    fn update_adjacent_level_crossing_tiles_on_level_crossing_removal(
        &self,
        tile: TileIndex,
        road_axis: Axis,
    ) {
        let dir1 = axis_to_diag_dir(road_axis, self.check());
        let dir2 = reverse_diag_dir(dir1, self.check());
        for dir in [dir1, dir2] {
            let diff = self.tile_offs_by_diag_dir(dir);
            let mut occupied = false;
            let mut t = tile + diff;
            while t.0 < self.services.map_size()
                && self.is_level_crossing_tile(t)
                && self.get_crossing_road_axis(t) == road_axis
            {
                occupied |= self.check_level_crossing(t);
                t += diff;
            }
            if occupied {
                /* Mark the immediately adjacent tile dirty */
                let t = tile + diff;
                if t.0 < self.services.map_size()
                    && self.is_level_crossing_tile(t)
                    && self.get_crossing_road_axis(t) == road_axis
                {
                    self.mark_tile_dirty_by_tile(t);
                }
            } else {
                /* Unbar the crossing tiles in this direction as necessary */
                let mut t = tile + diff;
                while t.0 < self.services.map_size()
                    && self.is_level_crossing_tile(t)
                    && self.get_crossing_road_axis(t) == road_axis
                {
                    if self.is_crossing_barred(t) {
                        /* The crossing tile is barred, unbar it and continue to check the next tile */
                        self.world().set_crossing_barred(t, false);
                        self.mark_tile_dirty_by_tile(t);
                    } else {
                        /* The crossing tile is already unbarred, mark the tile dirty and stop checking */
                        self.mark_tile_dirty_by_tile(t);
                        break;
                    }
                    t += diff;
                }
            }
        }
    }

    /// `MaybeBarCrossingWithSound`: bars crossing and plays ding-ding sound if not barred
    /// already. Precondition: `tile` is a rail-road crossing.
    #[inline]
    fn maybe_bar_crossing_with_sound(&self, tile: TileIndex) {
        if !self.is_crossing_barred(tile) {
            (self.services.set_crossing_reserved)(tile.0, true);
            self.update_level_crossing(tile, true, false);
        }
    }

    /// `AdvanceWagonsBeforeSwap`: advances wagons for train reversing, needed for variable
    /// length wagons. This one is called before the train is reversed. `v` is the first
    /// vehicle in chain.
    fn advance_wagons_before_swap(&self, v: Handle) {
        let mut base = v;
        let mut first = base; // first vehicle to move
        let mut last = v.last(); // last vehicle to move
        let mut length = crate::vehicle_func::count_vehicles_in_chain(v.vehicle());

        while length > 2 {
            last = last.previous();
            first = first.next();

            let differential = base.calc_next_vehicle_offset() - last.calc_next_vehicle_offset();

            /* do not update images now
             * negative differential will be handled in AdvanceWagonsAfterSwap() */
            for _ in 0..differential {
                crate::witness::hit(crate::witness::TRAIN_UNEQUAL_BEFORE);
                self.train_controller(first, last.next(), true);
            }

            base = first; // == base->Next()
            length -= 2;
        }
    }

    /// `AdvanceWagonsAfterSwap`: advances wagons for train reversing, needed for variable
    /// length wagons. This one is called after the train is reversed. `v` is the first
    /// vehicle in chain.
    fn advance_wagons_after_swap(&self, v: Handle) {
        /* first of all, fix the situation when the train was entering a depot */
        let mut dep = v; // last vehicle in front of just left depot
        while dep.next() != Handle::NONE
            && (dep.track() == TrackBits::TRACK_BIT_DEPOT
                || dep.next().track() != TrackBits::TRACK_BIT_DEPOT)
        {
            dep = dep.next(); // find first vehicle outside of a depot, with next vehicle inside a depot
        }

        let leave = dep.next(); // first vehicle in a depot we are leaving now

        if leave == Handle::NONE {
            dep = Handle::NONE; // no vehicle in a depot, so no vehicle leaving a depot
        } else {
            /* 'pull' next wagon out of the depot, so we won't miss it (it could stay in depot forever) */
            let d = crate::movement::ticks_to_leave_depot(self.services, dep);

            if d <= 0 {
                leave.vehicle().set_hidden(false); // move it out of the depot
                leave.set_track(TrackBits(crate::track_func::track_to_track_bits(
                    crate::rail_map::get_rail_depot_track(self.world(), leave.vehicle().tile()).0,
                    self.check(),
                )));
                let mut i = 0;
                while i >= d {
                    self.train_controller(leave, Handle::NONE, true); // maybe move it, and maybe let another wagon leave
                    i -= 1;
                }
            }
        }

        let mut base = v;
        let mut first = base; // first vehicle to move
        let mut last = v.last(); // last vehicle to move
        let mut length = crate::vehicle_func::count_vehicles_in_chain(v.vehicle());

        /* We have to make sure all wagons that leave a depot because of train reversing are moved correctly
         * they have already correct spacing, so we have to make sure they are moved how they should */
        let mut nomove = dep == Handle::NONE; // If there is no vehicle leaving a depot, limit the number of wagons moved immediately.

        while length > 2 {
            /* we reached vehicle (originally) in front of a depot, stop now
             * (we would move wagons that are already moved with new wagon length). */
            if base == dep {
                break;
            }

            /* the last wagon was that one leaving a depot, so do not move it anymore */
            if last == dep {
                nomove = true;
            }

            last = last.previous();
            first = first.next();

            let differential = last.calc_next_vehicle_offset() - base.calc_next_vehicle_offset();

            /* do not update images now */
            for _ in 0..differential {
                crate::witness::hit(crate::witness::TRAIN_UNEQUAL_AFTER);
                self.train_controller(first, if nomove { last.next() } else { Handle::NONE }, true);
            }

            base = first; // == base->Next()
            length -= 2;
        }
    }

    /// `IsWholeTrainInsideDepot`.
    fn is_whole_train_inside_depot(v: Handle) -> bool {
        let mut u = v;
        while u != Handle::NONE {
            if u.track() != TrackBits::TRACK_BIT_DEPOT || u.vehicle().tile() != v.vehicle().tile() {
                return false;
            }
            u = u.next();
        }
        true
    }

    /// `ReverseTrainDirection`: turn a train around.
    fn reverse_train_direction(&self, v: Handle) {
        crate::witness::hit(crate::witness::TRAIN_REVERSAL);
        if self.is_rail_depot_tile(v.vehicle().tile()) {
            if Self::is_whole_train_inside_depot(v) {
                return;
            }
            Self::invalidate_depot_window(v.vehicle().tile());
        }

        /* Clear path reservation in front if train is not stuck. */
        if !v.flags().test(VehicleRailFlag::Stuck) {
            reservation::free_train_track_reservation(&self.reservation(v), v);
        }

        /* Check if we were approaching a rail/road-crossing */
        let mut crossing = self.train_approaching_crossing_tile(v);

        /* count number of vehicles */
        let mut r = crate::vehicle_func::count_vehicles_in_chain(v.vehicle()) as i32 - 1; // number of vehicles - 1

        self.advance_wagons_before_swap(v);

        /* swap start<>end, start+1<>end-1, ... */
        let mut l = 0;
        loop {
            self.reverse_train_swap_veh(v, l, r);
            l += 1;
            r -= 1;
            if l > r {
                break;
            }
        }

        self.advance_wagons_after_swap(v);

        if self.is_rail_depot_tile(v.vehicle().tile()) {
            Self::invalidate_depot_window(v.vehicle().tile());
        }

        v.set_flags(v.flags().flip(VehicleRailFlag::Reversed));
        v.set_flags(v.flags().reset(VehicleRailFlag::Reversing));

        /* recalculate cached data */
        self.consist_changed(v, CCF_TRACK);

        /* update all images */
        let mut u = v;
        while u != Handle::NONE {
            self.viewport(u, false, false);
            u = u.next();
        }

        /* update crossing we were approaching */
        if crossing != TileIndex(INVALID_TILE) {
            self.update_level_crossing(crossing, true, false);
        }

        /* maybe we are approaching crossing now, after reversal */
        crossing = self.train_approaching_crossing_tile(v);
        if crossing != TileIndex(INVALID_TILE) {
            self.maybe_bar_crossing_with_sound(crossing);
        }

        /* If we are inside a depot after reversing, don't bother with path reserving. */
        if v.track() == TrackBits::TRACK_BIT_DEPOT {
            /* Can't be stuck here as inside a depot is always a safe tile. */
            if v.flags().test(VehicleRailFlag::Stuck) {
                self.set_start_stop_dirty(v);
            }
            v.set_flags(v.flags().reset(VehicleRailFlag::Stuck));
            return;
        }

        /* VehicleExitDir does not always produce the desired dir for depots and
         * tunnels/bridges that is needed for UpdateSignalsOnSegment. */
        let mut dir =
            crate::track_func::vehicle_exit_dir(v.vehicle().direction(), v.track(), self.check());
        if self.is_rail_depot_tile(v.vehicle().tile())
            || self.is_tile_type(v.vehicle().tile(), TileType::MP_TUNNELBRIDGE)
        {
            dir = DiagDirection::INVALID_DIAGDIR;
        }

        if self.update_signals_on_segment(v.vehicle().tile(), dir, v.vehicle().owner())
            == SigSegState::SIGSEG_PBS
            || self.reserve_paths()
        {
            /* If we are currently on a tile with conventional signals, we can't treat the
             * current tile as a safe tile or we would enter a PBS block without a reservation. */
            let tile = v.vehicle().tile();
            let mut first_tile_okay = !(self.is_tile_type(tile, TileType::MP_RAILWAY)
                && self.has_signal_on_trackdir(tile, self.get_vehicle_trackdir(v))
                && !crate::rail_map::is_pbs_signal(
                    self.get_signal_type(
                        tile,
                        Track(crate::track_func::find_first_track(v.track().0)),
                    )
                    .0,
                ));

            /* If we are on a depot tile facing outwards, do not treat the current tile as safe. */
            if self.is_rail_depot_tile(tile)
                && crate::track_func::trackdir_to_exitdir_asserted(
                    self.get_vehicle_trackdir(v),
                    self.check(),
                ) == self.get_rail_depot_direction(tile)
            {
                first_tile_okay = false;
            }

            if self.is_rail_station_tile(tile) {
                self.pbs().set_rail_station_platform_reservation(
                    tile,
                    crate::track_func::trackdir_to_exitdir_asserted(
                        self.get_vehicle_trackdir(v),
                        self.check(),
                    ),
                    true,
                );
            }
            if reservation::try_path_reserve(&self.reservation(v), v, false, first_tile_okay) {
                /* Do a look-ahead now in case our current tile was already a safe tile. */
                reservation::check_next_train_tile(&self.reservation(v), v);
            } else if v.vehicle().current_order().kind() != OrderType::OT_LOADING {
                /* Do not wait for a way out when we're still loading */
                self.mark_train_as_stuck(v);
            }
        } else if v.flags().test(VehicleRailFlag::Stuck) {
            /* A train not inside a PBS block can't be stuck. */
            v.set_flags(v.flags().reset(VehicleRailFlag::Stuck));
            v.set_wait_counter(0);
        }
    }
}

/// `SwapTrainFlags`: swap the two up/down flags in two ways:
/// - swap values of `swap_flag1` and `swap_flag2`, and
/// - if going up previously (`GVF_GOINGUP_BIT` set), the `GVF_GOINGDOWN_BIT` is set, and vice
///   versa.
///
/// The two ground states may be the same.
#[inline]
fn swap_train_flags(
    swap_flag1: *mut crate::ground_vehicle::GroundState,
    swap_flag2: *mut crate::ground_vehicle::GroundState,
) {
    use crate::bitmath_func::{clr_bit16, has_bit, set_bit16};
    const UP: u8 = GroundVehicleFlags::GVF_GOINGUP_BIT.0;
    const DOWN: u8 = GroundVehicleFlags::GVF_GOINGDOWN_BIT.0;
    let flag1: u16 = read_field!(swap_flag1, flags);
    let flag2: u16 = read_field!(swap_flag2, flags);
    let update = |p: *mut crate::ground_vehicle::GroundState, f: &dyn Fn(&mut u16)| {
        let mut bits: u16 = read_field!(p, flags);
        f(&mut bits);
        write_field!(p, flags, bits);
    };

    /* Clear the flags */
    update(swap_flag1, &|f| {
        clr_bit16(f, UP);
    });
    update(swap_flag1, &|f| {
        clr_bit16(f, DOWN);
    });
    update(swap_flag2, &|f| {
        clr_bit16(f, UP);
    });
    update(swap_flag2, &|f| {
        clr_bit16(f, DOWN);
    });

    /* Reverse the rail-flags (if needed) */
    if has_bit(flag1, UP) {
        update(swap_flag2, &|f| {
            set_bit16(f, DOWN);
        });
    } else if has_bit(flag1, DOWN) {
        update(swap_flag2, &|f| {
            set_bit16(f, UP);
        });
    }
    if has_bit(flag2, UP) {
        update(swap_flag1, &|f| {
            set_bit16(f, DOWN);
        });
    } else if has_bit(flag2, DOWN) {
        update(swap_flag1, &|f| {
            set_bit16(f, UP);
        });
    }
}

/// `IsTrain`: check if the vehicle is a train.
#[inline]
fn is_train(v: VehicleRef) -> bool {
    v.vehicle_type() == VehicleType::VEH_TRAIN
}

/// `Train::UpdateAcceleration`: update acceleration of the train from the cached power and
/// weight.
pub(crate) fn update_acceleration(this: Handle) {
    crate::services_assert::assert_with(|| this.is_front_engine() || this.is_free_wagon());

    let power: u32 = read_field!(this.vehicle().ground_cache(), cached_power);
    let weight: u32 = read_field!(this.vehicle().ground_cache(), cached_weight);
    crate::services_assert::assert_with(|| weight != 0);
    // Clamp(int, int, int): the uint product converts to int first.
    let acceleration = crate::math::clamp(
        (power / weight).wrapping_mul(4) as i32,
        1,
        255,
        crate::map_func::native_check,
    );
    write_field!(this.record, acceleration, acceleration as u8);
}

/// `CmdReverseTrainDirection`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdReverseTrainDirection {
    /// Train to reverse.
    pub(crate) veh_id: VehicleID,
    /// If true, reverse a unit in a train (needs to be in a depot).
    pub(crate) reverse_single_veh: bool,
}

/// `CmdForceTrainProceed`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdForceTrainProceed {
    /// Train to ignore the red signal.
    pub(crate) veh_id: VehicleID,
}

/// The call context of a train command: the vehicle build services' rail and common
/// tables (`Train::GetIfValid`, the record lookup, `CheckOwnership` and the cost results),
/// the cost output and the execution flags.
#[derive(Clone, Copy)]
pub(crate) struct CommandContext<'a> {
    pub(crate) rail: &'a crate::vehicle_transactions::rail::Services,
    pub(crate) common: &'a crate::vehicle_transactions::CommonServices,
    pub(crate) out: *mut crate::services::CommandCost,
    pub(crate) flags: crate::command_type::DoCommandFlags,
}

impl CommandContext<'_> {
    /// `Train::GetIfValid(veh_id)`.
    fn train_get_if_valid(&self, veh_id: VehicleID) -> Handle {
        let shell = (self.rail.train_if_valid)(veh_id.0);
        if shell.is_null() {
            return Handle::NONE;
        }
        Handle::from_record((self.common.vehicles.record_from_shell)(shell.cast()))
    }
    /// `return CMD_ERROR`.
    fn cmd_error(&self) {
        crate::command_func::cmd_error(crate::services::Cost(self.out, self.common.base.cost));
    }
    /// `return CommandCost(msg)`.
    fn error(&self, msg: crate::strings_type::StringID) {
        (self.common.base.cost.error)(self.out, msg.0);
    }
    /// `CommandCost ret = CheckOwnership(owner); if (ret.Failed()) return ret;`: whether
    /// the check succeeded, with its result in the output.
    fn check_ownership(&self, owner: Owner) -> bool {
        let base = self.common.base;
        // SAFETY: The command's live native result.
        unsafe { base.commands.ownership(self.out, owner.0, 0) };
        (base.cost.success)(self.out)
    }
    /// `flags.Test(DoCommandFlag::Execute)`.
    fn execute(&self) -> bool {
        self.flags.test(crate::command_type::DoCommandFlag::Execute)
    }
}

impl Game<'_> {
    /// `CmdReverseTrainDirection`: reverse train.
    fn cmd_reverse_train_direction(&self, c: &CommandContext, cmd: CmdReverseTrainDirection) {
        let CmdReverseTrainDirection {
            veh_id,
            reverse_single_veh,
        } = cmd;
        let v = c.train_get_if_valid(veh_id);
        if v == Handle::NONE {
            return c.cmd_error();
        }

        if !c.check_ownership(v.vehicle().owner()) {
            return;
        }

        if reverse_single_veh {
            /* turn a single unit around */

            if v.is_multiheaded()
                || VehicleCallbackMasks(self.engine(v).callbacks())
                    .test(VehicleCallbackMask::ArticEngine)
            {
                return c.error(STR_ERROR_CAN_T_REVERSE_DIRECTION_RAIL_VEHICLE_MULTIPLE_UNITS);
            }

            let front = v.first();
            /* make sure the vehicle is stopped in the depot */
            if !crate::vehicle_base::is_stopped_in_depot(front.vehicle(), self.leaves.physics) {
                return c.error(STR_ERROR_TRAINS_CAN_ONLY_BE_ALTERED_INSIDE_A_DEPOT);
            }

            if c.execute() {
                v.set_flags(v.flags().flip(VehicleRailFlag::Flipped));

                self.consist_changed(front, CCF_ARRANGE);
                set_window_dirty(
                    WindowClass::WC_VEHICLE_DEPOT,
                    WindowNumber::from(front.vehicle().tile()),
                );
                set_window_dirty(
                    WindowClass::WC_VEHICLE_DETAILS,
                    WindowNumber::from(self.index(front)),
                );
                set_window_dirty(
                    WindowClass::WC_VEHICLE_VIEW,
                    WindowNumber::from(self.index(front)),
                );
                set_window_classes_dirty(WindowClass::WC_TRAINS_LIST);
            }
        } else {
            /* turn the whole train around */
            if !v.is_front_engine() {
                return c.cmd_error();
            }
            if v.vehicle().vehstatus().test(VehState::Crashed) || v.vehicle().breakdown_ctr() != 0 {
                return c.cmd_error();
            }

            if c.execute() {
                /* Properly leave the station if we are loading and won't be loading anymore */
                if v.vehicle().current_order().is_type(OrderType::OT_LOADING) {
                    let last = v.vehicle().last();

                    /* not a station || different station --> leave the station */
                    if !self.is_tile_type(last.tile(), TileType::MP_STATION)
                        || self.get_station_index(last.tile())
                            != self.get_station_index(v.vehicle().tile())
                    {
                        crate::vehicle::visits::leave_station(v.vehicle(), self.vs.visits());
                    }
                }

                /* We cancel any 'skip signal at dangers' here */
                v.set_force_proceed(TrainForceProceeding::TFP_NONE);
                self.set_vehicle_view_dirty(v);

                if self.train_acceleration_model() != AccelerationModel::AM_ORIGINAL
                    && v.vehicle().cur_speed() != 0
                {
                    v.set_flags(v.flags().flip(VehicleRailFlag::Reversing));
                } else {
                    v.vehicle().set_cur_speed(0);
                    self.set_last_speed(v);
                    (self.vs.visits().hide_filling)(v.vehicle());
                    self.reverse_train_direction(v);
                }

                /* Unbunching data is no longer valid. */
                crate::orders::reset_unbunching(v.vehicle());
            }
        }
    }

    /// `DetermineNextTrainForceProceeding`: determine to what `force_proceed` should be
    /// changed. If we are forced to proceed, cancel that order. If we are marked stuck we
    /// would want to force the train to proceed to the next signal unless we are stuck just
    /// before the next signal. In the other cases we would like to pass the signal at danger
    /// and run till the next signal we encounter.
    #[inline]
    fn determine_next_train_force_proceeding(&self, t: Handle) -> TrainForceProceeding {
        if t.vehicle().vehstatus().test(VehState::Crashed)
            || t.force_proceed() == TrainForceProceeding::TFP_SIGNAL
        {
            return TrainForceProceeding::TFP_NONE;
        }
        if !t.flags().test(VehicleRailFlag::Stuck) {
            return if self.is_chain_in_depot(t) {
                TrainForceProceeding::TFP_STUCK
            } else {
                TrainForceProceeding::TFP_SIGNAL
            };
        }

        let next_tile = self.tile_add_by_diag_dir(
            t.vehicle().tile(),
            crate::track_func::trackdir_to_exitdir_asserted(
                self.get_vehicle_trackdir(t),
                self.check(),
            ),
        );
        if next_tile == TileIndex(INVALID_TILE)
            || !self.is_tile_type(next_tile, TileType::MP_RAILWAY)
            || !self.world().has_signals(next_tile.0)
        {
            return TrainForceProceeding::TFP_STUCK;
        }
        let new_tracks = TrackBits(
            crate::track_func::diagdir_reaches_tracks(
                crate::track_func::trackdir_to_exitdir_asserted(
                    self.get_vehicle_trackdir(t),
                    self.check(),
                )
                .0,
            ) & self.world().tracks(next_tile.0),
        );
        if new_tracks != TrackBits::TRACK_BIT_NONE
            && self.world().has_signal_on_track(
                next_tile.0,
                crate::track_func::find_first_track(new_tracks.0),
            )
        {
            TrainForceProceeding::TFP_SIGNAL
        } else {
            TrainForceProceeding::TFP_STUCK
        }
    }

    /// `CmdForceTrainProceed`: force a train through a red signal.
    fn cmd_force_train_proceed(&self, c: &CommandContext, cmd: CmdForceTrainProceed) {
        let CmdForceTrainProceed { veh_id } = cmd;
        let t = c.train_get_if_valid(veh_id);
        if t == Handle::NONE {
            return c.cmd_error();
        }

        if !t.is_front_engine() {
            return c.cmd_error();
        }

        if !c.check_ownership(t.vehicle().owner()) {
            return;
        }

        if c.execute() {
            t.set_force_proceed(self.determine_next_train_force_proceeding(t));
            self.set_vehicle_view_dirty(t);

            /* Unbunching data is no longer valid. */
            crate::orders::reset_unbunching(t.vehicle());
        }
    }

    /// `FindClosestTrainDepot`: try to find a depot nearby, within `max_distance`.
    /// Precondition: the given vehicle must not be crashed!
    #[inline]
    fn find_closest_train_depot(&self, v: Handle, max_distance: i32) -> FindDepotData {
        crate::services_assert::assert_with(|| !v.vehicle().vehstatus().test(VehState::Crashed));

        crate::rail_yapf::yapf_train_find_nearest_depot(self.services, v, max_distance)
    }

    /// `Train::FindClosestDepot`.
    fn find_closest_depot(&self, this: Handle) -> ClosestDepot {
        let tfdd = self.find_closest_train_depot(this, 0);
        if tfdd.best_length == u32::MAX {
            return ClosestDepot::default();
        }

        ClosestDepot::new(
            tfdd.tile,
            crate::order_type::DestinationID((self.services.depot_index)(tfdd.tile.0)),
            tfdd.reverse,
        )
    }

    /// `CheckTrainStayInDepot`: will the train stay in the depot the next tick?
    fn check_train_stay_in_depot(&self, v: Handle) -> bool {
        /* bail out if not all wagons are in the same depot or not in a depot at all */
        if !Self::is_whole_train_inside_depot(v) {
            return false;
        }

        /* if the train got no power, then keep it in the depot */
        if read_field!(v.vehicle().ground_cache(), cached_power) == 0 {
            v.vehicle()
                .set_vehstatus(v.vehicle().vehstatus().set(VehState::Stopped));
            set_window_dirty(
                WindowClass::WC_VEHICLE_DEPOT,
                WindowNumber::from(v.vehicle().tile()),
            );
            return true;
        }

        /* Check if we should wait here for unbunching. */
        if crate::vehicle::is_waiting_for_unbunching(v.vehicle(), self.vs.orders(), || {
            v.is_in_depot()
        }) {
            return true;
        }

        let seg_state;

        if v.force_proceed() == TrainForceProceeding::TFP_NONE {
            /* force proceed was not pressed */
            v.set_wait_counter(v.wait_counter().wrapping_add(1));
            if v.wait_counter() < 37 {
                set_window_classes_dirty(WindowClass::WC_TRAINS_LIST);
                return true;
            }

            v.set_wait_counter(0);

            seg_state = if self.reserve_paths() {
                SigSegState::SIGSEG_PBS
            } else {
                self.update_signals_on_segment(
                    v.vehicle().tile(),
                    DiagDirection::INVALID_DIAGDIR,
                    v.vehicle().owner(),
                )
            };
            if seg_state == SigSegState::SIGSEG_FULL
                || self.has_depot_reservation(v.vehicle().tile())
            {
                /* Full and no PBS signal in block or depot reserved, can't exit. */
                set_window_classes_dirty(WindowClass::WC_TRAINS_LIST);
                return true;
            }
        } else {
            seg_state = if self.reserve_paths() {
                SigSegState::SIGSEG_PBS
            } else {
                self.update_signals_on_segment(
                    v.vehicle().tile(),
                    DiagDirection::INVALID_DIAGDIR,
                    v.vehicle().owner(),
                )
            };
        }

        /* We are leaving a depot, but have to go to the exact same one; re-enter. */
        if v.vehicle()
            .current_order()
            .is_type(OrderType::OT_GOTO_DEPOT)
            && v.vehicle().tile() == v.vehicle().dest_tile()
        {
            /* Service when depot has no reservation. */
            if !self.has_depot_reservation(v.vehicle().tile()) {
                crate::witness::hit(crate::witness::TRAIN_DEPOT_REENTRY);
                self.vs.enter_depot(v.vehicle());
            }
            return true;
        }

        /* Only leave when we can reserve a path to our destination. */
        if seg_state == SigSegState::SIGSEG_PBS
            && !reservation::try_path_reserve(&self.reservation(v), v, false, false)
            && v.force_proceed() == TrainForceProceeding::TFP_NONE
        {
            /* No path and no force proceed. */
            set_window_classes_dirty(WindowClass::WC_TRAINS_LIST);
            self.mark_train_as_stuck(v);
            return true;
        }

        (self.pbs().0.set_depot)(v.vehicle().tile().0, true);
        if (self.pbs().0.show_reservation)() {
            self.mark_tile_dirty_by_tile(v.vehicle().tile());
        }

        crate::witness::hit(crate::witness::TRAIN_DEPOT_START);
        self.maintain(v, maintenance::service_in_depot);
        crate::orders::VisitOrders::new(self.vs.orders()).leave_unbunching_depot(v.vehicle());
        (self.vs.visits().leave_sound)(v.vehicle());
        set_window_classes_dirty(WindowClass::WC_TRAINS_LIST);

        v.set_track(TrackBits::TRACK_BIT_X);
        if v.vehicle().direction().0 & 2 != 0 {
            v.set_track(TrackBits::TRACK_BIT_Y);
        }

        v.vehicle().set_hidden(false);
        v.vehicle().set_cur_speed(0);

        self.viewport(v, true, true);
        self.update_position(v);
        self.update_signals_on_segment(
            v.vehicle().tile(),
            DiagDirection::INVALID_DIAGDIR,
            v.vehicle().owner(),
        );
        update_acceleration(v);
        Self::invalidate_depot_window(v.vehicle().tile());

        false
    }

    /// `HasDepotReservation(tile)`.
    fn has_depot_reservation(&self, tile: TileIndex) -> bool {
        // SAFETY: The PBS table names the immutable rail map table.
        (unsafe { &*self.pbs().0.rail }.depot_reserved)(tile.0)
    }

    /// `Train::MarkDirty`: goods at the consist have changed, update the graphics, cargo, and
    /// acceleration.
    fn mark_dirty(&self, this: Handle) {
        let mut v = this;
        loop {
            (self.vs.set_colourmap)(v.vehicle(), PAL_NONE);
            self.viewport(v, true, false);
            v = v.next();
            if v == Handle::NONE {
                break;
            }
        }

        /* need to update acceleration and cached values since the goods on the train changed. */
        self.cargo_changed(this);
        update_acceleration(this);
    }

    /// `Train::UpdateSpeed`: this function looks at the vehicle and updates its speed
    /// (`cur_speed` and `subspeed`) variables. Furthermore, it returns the distance that the
    /// train can drive this tick. `Vehicle::GetAdvanceDistance()` determines the distance to
    /// drive before moving a step on the map.
    fn update_speed(&self, this: Handle) -> i32 {
        let o = crate::ground_vehicle::Owner::of(this.vehicle());
        let braking = crate::ground_vehicle::get_acceleration_status_owned(this.vehicle(), o)
            == AccelStatus::AS_BRAKE;
        match self.train_acceleration_model() {
            AccelerationModel::AM_ORIGINAL => crate::ground_vehicle::do_update_speed(
                this.vehicle(),
                (i32::from(this.vehicle().acceleration()) * if braking { -4 } else { 2 }) as u32,
                0,
                self.get_current_max_speed(this),
            ) as i32,
            AccelerationModel::AM_REALISTIC => crate::ground_vehicle::do_update_speed(
                this.vehicle(),
                crate::ground_vehicle::get_acceleration_owned(
                    self.leaves.physics,
                    this.vehicle(),
                    o,
                ) as u32,
                if braking { 0 } else { 2 },
                self.get_current_max_speed(this),
            ) as i32,
            _ => crate::services_assert::not_reached(),
        }
    }

    /// `TrainEnterStation`: trains enters a station, send out a news item if it is the first
    /// train, and start loading.
    fn train_enter_station(&self, v: Handle, station: StationID) {
        v.vehicle().set_last_station_visited(station);

        /* check if a train ever visited this station before */
        let st = self.station_lookup(station);
        // SAFETY: A live station resolved in this serialized entry; one byte-sized load.
        let had = unsafe { *(self.services.station_had_vehicle_of_type_field)(st.native) };
        if had & StationHadVehicleOfType::HVOT_TRAIN.0 == 0 {
            // SAFETY: As above; one byte-sized read-modify-write.
            unsafe {
                *(self.services.station_had_vehicle_of_type_field)(st.native) |=
                    StationHadVehicleOfType::HVOT_TRAIN.0;
            }
            add_vehicle_news_item(
                &get_encoded_string(
                    STR_NEWS_FIRST_TRAIN_ARRIVAL,
                    &[Integer(u64::from(station.0))],
                ),
                if v.vehicle().owner().0 == (self.services.local_company)() {
                    NewsType::ArrivalCompany
                } else {
                    NewsType::ArrivalOther
                },
                self.index(v),
                station,
            );
            crate::ai::new_event(
                v.vehicle().owner(),
                &ScriptEvent::station_first_vehicle(station, self.index(v)),
            );
            crate::game::new_event(&ScriptEvent::station_first_vehicle(station, self.index(v)));
        }

        v.set_force_proceed(TrainForceProceeding::TFP_NONE);
        self.set_vehicle_view_dirty(v);

        crate::vehicle::visits::begin_loading(v.vehicle(), self.vs.visits());

        crate::newgrf_station::trigger_station_randomisation(
            st.base,
            v.vehicle().tile(),
            crate::station_type::StationRandomTrigger::VehicleArrives,
            crate::cargo_type::CargoType::INVALID_CARGO,
        );
        crate::animated_tile::trigger_station_animation(
            st.base,
            v.vehicle().tile(),
            crate::station_type::StationAnimationTrigger::VehicleArrives,
            crate::cargo_type::CargoType::INVALID_CARGO,
        );
    }

    /// `CheckCompatibleRail`: check if the vehicle is compatible with the specified tile.
    #[inline]
    fn check_compatible_rail(&self, v: Handle, tile: TileIndex) -> bool {
        crate::tile_map::is_tile_owner(self.services, tile, v.vehicle().owner())
            && (!v.is_front_engine()
                || v.compatible_railtypes()
                    .test(RailType(self.world().rail_type(tile.0))))
    }

    /// `AffectSpeedByZChange`: modify the speed of the vehicle due to a change in altitude.
    #[inline]
    fn affect_speed_by_z_change(&self, v: Handle, old_z: i32) {
        if old_z == v.vehicle().z_pos()
            || self.train_acceleration_model() != AccelerationModel::AM_ORIGINAL
        {
            return;
        }

        let asp = &ACCEL_SLOWDOWN[self.get_acceleration_type(v)];

        let cur_speed = v.vehicle().cur_speed();
        if old_z < v.vehicle().z_pos() {
            v.vehicle().set_cur_speed(
                cur_speed.wrapping_sub(((u32::from(cur_speed) * u32::from(asp.z_up)) >> 8) as u16),
            );
        } else {
            let spd = cur_speed.wrapping_add(u16::from(asp.z_down));
            if spd <= read_field!(v.vehicle().ground_cache(), cached_max_track_speed) {
                v.vehicle().set_cur_speed(spd);
            }
        }
    }

    /// `TrainMovedChangeSignals`.
    fn train_moved_change_signals(&self, tile: TileIndex, dir: DiagDirection) -> bool {
        if self.is_tile_type(tile, TileType::MP_RAILWAY)
            && self.world().rail_tile_type(tile.0) == crate::rail_map::RailTileType::Signals.0
        {
            let tracks =
                crate::track_func::track_bits_to_trackdir_bits(self.world().tracks(tile.0))
                    & crate::track_func::diagdir_reaches_trackdirs(dir.0);
            let trackdir = Trackdir(crate::track_func::find_first_trackdir(tracks));
            if self.update_signals_on_segment(
                tile,
                crate::track_func::trackdir_to_exitdir_asserted(trackdir, self.check()),
                self.world().owner(tile),
            ) == SigSegState::SIGSEG_PBS
                && self.has_signal_on_trackdir(tile, trackdir)
            {
                /* A PBS block with a non-PBS signal facing us? */
                if !crate::rail_map::is_pbs_signal(
                    self.get_signal_type(
                        tile,
                        Track(crate::track_func::trackdir_to_track(trackdir.0)),
                    )
                    .0,
                ) {
                    return true;
                }
            }
        }
        false
    }

    /// `Train::Crash`: the train vehicle crashed! Update its status and other parts around
    /// it. `flooded` says the crash was caused by flooding. Returns the number of people
    /// killed.
    fn crash(&self, this: Handle, flooded: bool) -> u32 {
        let mut victims = 0_u32;
        if this.is_front_engine() {
            victims += 2; // driver

            /* Remove the reserved path in front of the train if it is not stuck.
             * Also clear all reserved tracks the train is currently on. */
            if !this.flags().test(VehicleRailFlag::Stuck) {
                reservation::free_train_track_reservation(&self.reservation(this), this);
            }
            let mut v = this;
            while v != Handle::NONE {
                reservation::clear_path_reservation(
                    &self.reservation(v),
                    v,
                    v.vehicle().tile(),
                    self.get_vehicle_trackdir(v),
                );
                if self.is_tile_type(v.vehicle().tile(), TileType::MP_TUNNELBRIDGE) {
                    /* ClearPathReservation will not free the wormhole exit
                     * if the train has just entered the wormhole. */
                    (self.pbs().0.set_wormhole)(
                        self.get_other_tunnel_bridge_end(v.vehicle().tile()).0,
                        false,
                    );
                }
                v = v.next();
            }

            /* we may need to update crossing we were approaching,
             * but must be updated after the train has been marked crashed */
            let crossing = self.train_approaching_crossing_tile(this);
            if crossing != TileIndex(INVALID_TILE) {
                self.update_level_crossing(crossing, true, false);
            }

            /* Remove the loading indicators (if any) */
            (self.vs.visits().hide_filling)(this.vehicle());
        }

        victims = victims.wrapping_add(crate::ground_vehicle::crash(
            self.leaves.physics,
            this.vehicle(),
            flooded,
        ));

        this.set_crash_anim_pos(if flooded { 4000 } else { 1 }); // max 4440, disappear pretty fast when flooded
        victims
    }

    /// `TrainCrashed`: marks train as crashed and creates an AI event. Doesn't do anything if
    /// the train is crashed already. `v` is the first vehicle of chain. Returns the number of
    /// victims (including 2 drivers; zero if train was already crashed).
    #[inline]
    fn train_crashed(&self, v: Handle) -> u32 {
        let mut victims = 0;

        /* do not crash train twice */
        if !v.vehicle().vehstatus().test(VehState::Crashed) {
            victims = self.crash(v, false);
            let crashed = || {
                ScriptEvent::vehicle_crashed(
                    self.index(v),
                    v.vehicle().tile(),
                    CrashReason::CRASH_TRAIN,
                    victims,
                    v.vehicle().owner(),
                )
            };
            crate::ai::new_event(v.vehicle().owner(), &crashed());
            crate::game::new_event(&crashed());
        }

        /* Try to re-reserve track under already crashed train too.
         * Crash() clears the reservation! */
        reservation::reserve_track_under_consist(&self.reservation(v), v);

        victims
    }

    /// `CheckTrainCollision(Vehicle *v, Train *t)`, the per-vehicle overload: collision test
    /// of vehicle `v` with train `t`. Returns the number of victims.
    fn check_train_collision_with(&self, v: VehicleRef, t: Handle) -> u32 {
        /* not a train or in depot */
        if v.vehicle_type() != VehicleType::VEH_TRAIN {
            return 0;
        }
        let u = Handle::from_vehicle(v);
        if u.track() == TrackBits::TRACK_BIT_DEPOT {
            return 0;
        }

        /* do not crash into trains of another company. */
        if v.owner() != t.vehicle().owner() {
            return 0;
        }

        /* get first vehicle now to make most usual checks faster */
        let coll = u.first();

        /* can't collide with own wagons */
        if coll == t {
            return 0;
        }

        let x_diff = v.x_pos().wrapping_sub(t.vehicle().x_pos());
        let y_diff = v.y_pos().wrapping_sub(t.vehicle().y_pos());

        /* Do fast calculation to check whether trains are not in close vicinity
         * and quickly reject trains distant enough for any collision.
         * Differences are shifted by 7, mapping range [-7 .. 8] into [0 .. 15]
         * Differences are then ORed and then we check for any higher bits */
        let hash = (y_diff.wrapping_add(7) | x_diff.wrapping_add(7)) as u32;
        if hash & !15 != 0 {
            return 0;
        }

        /* Slower check using multiplication */
        let min_diff = (i32::from(u.cached_veh_length()) + 1) / 2
            + (i32::from(t.cached_veh_length()) + 1) / 2
            - 1;
        if x_diff
            .wrapping_mul(x_diff)
            .wrapping_add(y_diff.wrapping_mul(y_diff))
            > min_diff * min_diff
        {
            return 0;
        }

        /* Happens when there is a train under bridge next to bridge head */
        if v.z_pos().wrapping_sub(t.vehicle().z_pos()).wrapping_abs() > 5 {
            return 0;
        }

        /* Crash both trains. Two statements required to guarantee execution
         * order because RandomRange() is involved. */
        let num_victims = self.train_crashed(t);
        num_victims.wrapping_add(self.train_crashed(coll))
    }

    /// `CheckTrainCollision(Train *v)`: checks whether the specified train has a collision
    /// with another vehicle. If so, destroys this vehicle, and the other vehicle if its
    /// subtype has `TS_Front`. Reports the incident in a flashy news item, modifies station
    /// ratings and plays a sound.
    fn check_train_collision(&self, v: Handle) -> bool {
        /* can't collide in depot */
        if v.track() == TrackBits::TRACK_BIT_DEPOT {
            return false;
        }

        crate::services_assert::assert_with(|| {
            v.track() == TrackBits::TRACK_BIT_WORMHOLE
                || self.tile_virt_xy(v.vehicle().x_pos(), v.vehicle().y_pos()) == v.vehicle().tile()
        });

        let mut num_victims = 0_u32;

        /* find colliding vehicles */
        if v.track() == TrackBits::TRACK_BIT_WORMHOLE {
            crate::vehicle_func::vehicles_on_tile(self.vs, self.map(), v.vehicle().tile(), |u| {
                num_victims = num_victims.wrapping_add(self.check_train_collision_with(u, v));
            });
            crate::vehicle_func::vehicles_on_tile(
                self.vs,
                self.map(),
                self.get_other_tunnel_bridge_end(v.vehicle().tile()),
                |u| {
                    num_victims = num_victims.wrapping_add(self.check_train_collision_with(u, v));
                },
            );
        } else {
            crate::vehicle_func::vehicles_near_tile_xy(
                self.vs,
                v.vehicle().x_pos(),
                v.vehicle().y_pos(),
                7,
                |u| {
                    num_victims = num_victims.wrapping_add(self.check_train_collision_with(u, v));
                },
            );
        }

        /* any dead -> no crash */
        if num_victims == 0 {
            return false;
        }

        crate::witness::hit(crate::witness::TRAIN_COLLISION);
        add_tile_news_item(
            &get_encoded_string(STR_NEWS_TRAIN_CRASH, &[Integer(u64::from(num_victims))]),
            NewsType::Accident,
            v.vehicle().tile(),
            StationID::INVALID,
        );

        crate::station::spatial::modify_station_rating_around(
            self.services,
            v.vehicle().tile(),
            v.vehicle().owner(),
            -160,
            30,
        );
        if (self.services.disaster_sound)() {
            (self.vs.play_sound)(
                v.vehicle(),
                SoundID::from(SoundFx::SND_13_TRAIN_COLLISION).0,
            );
        }
        true
    }

    /// `TrainController`: move a vehicle chain one movement stop forwards. `v` is the first
    /// vehicle to move; `nomove` stops moving this and all following vehicles; `reverse` set
    /// to false does not execute the vehicle reversing, which does not change any other
    /// logic. Returns true if the vehicle could be moved forward, false otherwise.
    fn train_controller(&self, mut v: Handle, nomove: Handle, reverse: bool) -> bool {
        let first = v.first();
        let mut prev = v.previous();
        let mut direction_changed = false; // has direction of any part changed?

        /* The original's `goto invalid_rail` breaks with true and its
         * `goto reverse_train_direction` with false. */
        let invalid_rail = 'controller: {
            /* For every vehicle after and including the given vehicle */
            while v != nomove {
                'part: {
                    if v.is_articulated_part() {
                        crate::witness::hit(crate::witness::TRAIN_ARTICULATED_MOVE);
                    }
                    let mut enterdir = DiagDirection::DIAGDIR_BEGIN;
                    let mut update_signals_crossing = false; // will we update signals or crossing state?

                    let mut gp = crate::vehicle::get_new_vehicle_pos(v.vehicle(), self.map());
                    if v.track() != TrackBits::TRACK_BIT_WORMHOLE {
                        /* Not inside tunnel */
                        if gp.old_tile == gp.new_tile {
                            /* Staying in the old tile */
                            if v.track() == TrackBits::TRACK_BIT_DEPOT {
                                /* Inside depot */
                                gp.x = v.vehicle().x_pos();
                                gp.y = v.vehicle().y_pos();
                            } else {
                                /* Not inside depot */

                                /* Reverse when we are at the end of the track already, do not move to the new position */
                                if v.is_front_engine() && !self.train_check_if_line_ends(v, reverse)
                                {
                                    return false;
                                }

                                let vets = self.vehicle_enter_tile(v, gp.new_tile, gp.x, gp.y);
                                if vets.test(VehicleEnterTileState::CannotEnter) {
                                    break 'controller true;
                                }
                                if vets.test(VehicleEnterTileState::EnteredStation) {
                                    /* The new position is the end of the platform */
                                    self.train_enter_station(
                                        v,
                                        self.get_station_index(gp.new_tile),
                                    );
                                }
                            }
                        } else {
                            /* A new tile is about to be entered. */

                            /* Determine what direction we're entering the new tile from */
                            enterdir = self.map().diagdir_between_tiles(gp.old_tile, gp.new_tile);
                            crate::services_assert::assert_with(|| {
                                is_valid_diag_direction(enterdir)
                            });

                            /* Get the status of the tracks in the new tile and mask
                             * away the bits that aren't reachable. */
                            let ts = self.get_tile_track_status(
                                gp.new_tile,
                                reverse_diag_dir(enterdir, self.check()),
                            );
                            let reachable_trackdirs = TrackdirBits(
                                crate::track_func::diagdir_reaches_trackdirs(enterdir.0),
                            );

                            let trackdirbits = TrackdirBits(
                                crate::track_func::track_status_to_trackdir_bits(ts)
                                    & reachable_trackdirs.0,
                            );
                            let red_signals =
                                TrackBits(crate::track_func::trackdir_bits_to_track_bits(
                                    crate::track_func::track_status_to_red_signals(ts).0
                                        & reachable_trackdirs.0,
                                ));

                            let mut bits = TrackBits(
                                crate::track_func::trackdir_bits_to_track_bits(trackdirbits.0),
                            );
                            if self.rail_90_deg_turn_disallowed(
                                self.get_tile_rail_type(gp.old_tile),
                                self.get_tile_rail_type(gp.new_tile),
                            ) && prev == Handle::NONE
                            {
                                /* We allow wagons to make 90 deg turns, because forbid_90_deg
                                 * can be switched on halfway a turn */
                                bits.0 &= !crate::track_func::track_crosses_tracks(
                                    Track(crate::track_func::find_first_track(v.track().0)),
                                    self.check(),
                                )
                                .0;
                            }

                            if bits == TrackBits::TRACK_BIT_NONE {
                                break 'controller true;
                            }

                            /* Check if the new tile constrains tracks that are compatible
                             * with the current train, if not, bail out. */
                            if !self.check_compatible_rail(v, gp.new_tile) {
                                break 'controller true;
                            }

                            let mut chosen_track;
                            if prev == Handle::NONE {
                                /* Currently the locomotive is active. Determine which one of the
                                 * available tracks to choose */
                                let (track, _) = reservation::choose_train_track(
                                    &self.reservation(v),
                                    v,
                                    gp.new_tile,
                                    enterdir,
                                    bits,
                                    false,
                                    true,
                                );
                                chosen_track = TrackBits(crate::track_func::track_to_track_bits(
                                    track.0,
                                    self.check(),
                                ));
                                crate::services_assert::assert_with(|| {
                                    chosen_track.0
                                        & (bits.0
                                            | self.pbs().get_reserved_trackbits(gp.new_tile).0)
                                        != 0
                                });

                                if v.force_proceed() != TrainForceProceeding::TFP_NONE
                                    && self.world().plain_rail(gp.new_tile.0)
                                    && self.world().has_signals(gp.new_tile.0)
                                {
                                    /* For each signal we find decrease the counter by one.
                                     * We start at two, so the first signal we pass decreases
                                     * this to one, then if we reach the next signal it is
                                     * decreased to zero and we won't pass that new signal. */
                                    let dir = Trackdir(crate::track_func::find_first_trackdir(
                                        trackdirbits.0,
                                    ));
                                    if self.has_signal_on_trackdir(gp.new_tile, dir)
                                        || (self.has_signal_on_trackdir(
                                            gp.new_tile,
                                            Trackdir(crate::track_func::reverse_trackdir(dir.0)),
                                        ) && self.get_signal_type(
                                            gp.new_tile,
                                            Track(crate::track_func::trackdir_to_track(dir.0)),
                                        ) != SignalType::SIGTYPE_PBS)
                                    {
                                        /* However, we do not want to be stopped by PBS signals
                                         * entered via the back. */
                                        crate::witness::hit(crate::witness::TRAIN_FORCE_SIGNAL);
                                        v.set_force_proceed(
                                            if v.force_proceed() == TrainForceProceeding::TFP_SIGNAL
                                            {
                                                TrainForceProceeding::TFP_STUCK
                                            } else {
                                                TrainForceProceeding::TFP_NONE
                                            },
                                        );
                                        self.set_vehicle_view_dirty(v);
                                    }
                                }

                                /* Check if it's a red signal and that force proceed is not clicked. */
                                if red_signals.0 & chosen_track.0 != 0
                                    && v.force_proceed() == TrainForceProceeding::TFP_NONE
                                {
                                    /* In front of a red signal */
                                    let i = Trackdir(crate::track_func::find_first_trackdir(
                                        trackdirbits.0,
                                    ));

                                    /* Don't handle stuck trains here. */
                                    if v.flags().test(VehicleRailFlag::Stuck) {
                                        return false;
                                    }

                                    if !self.has_signal_on_trackdir(
                                        gp.new_tile,
                                        Trackdir(crate::track_func::reverse_trackdir(i.0)),
                                    ) {
                                        v.vehicle().set_cur_speed(0);
                                        v.vehicle().set_subspeed(0);
                                        v.vehicle().set_progress(255); // make sure that every bit of acceleration will hit the signal again, so speed stays 0.
                                        crate::witness::hit(crate::witness::TRAIN_RED_ONEWAY);
                                        if !self.reverse_at_signals() || {
                                            v.set_wait_counter(v.wait_counter().wrapping_add(1));
                                            i32::from(v.wait_counter())
                                                < i32::from(self.wait_oneway_signal())
                                                    * crate::timer_game_tick::DAY_TICKS
                                                    * 2
                                        } {
                                            return false;
                                        }
                                    } else if self.has_signal_on_trackdir(gp.new_tile, i) {
                                        v.vehicle().set_cur_speed(0);
                                        v.vehicle().set_subspeed(0);
                                        v.vehicle().set_progress(255); // make sure that every bit of acceleration will hit the signal again, so speed stays 0.
                                        crate::witness::hit(crate::witness::TRAIN_RED_TWOWAY);
                                        if !self.reverse_at_signals() || {
                                            v.set_wait_counter(v.wait_counter().wrapping_add(1));
                                            i32::from(v.wait_counter())
                                                < i32::from(self.wait_twoway_signal())
                                                    * crate::timer_game_tick::DAY_TICKS
                                                    * 2
                                        } {
                                            let mut exitdir =
                                                crate::track_func::trackdir_to_exitdir_asserted(
                                                    i,
                                                    self.check(),
                                                );
                                            let o_tile =
                                                self.tile_add_by_diag_dir(gp.new_tile, exitdir);

                                            exitdir = reverse_diag_dir(exitdir, self.check());

                                            /* check if a train is waiting on the other side */
                                            if !crate::vehicle_func::has_vehicle_on_tile(
                                                self.vs,
                                                self.map(),
                                                o_tile,
                                                |u| {
                                                    if u.vehicle_type() != VehicleType::VEH_TRAIN
                                                        || u.vehstatus().test(VehState::Crashed)
                                                    {
                                                        return false;
                                                    }
                                                    let t = Handle::from_vehicle(u);

                                                    /* not front engine of a train, inside wormhole or depot, crashed */
                                                    if !t.is_front_engine()
                                                        || t.track().0 & TrackBits::TRACK_BIT_MASK.0
                                                            == 0
                                                    {
                                                        return false;
                                                    }

                                                    if t.vehicle().cur_speed() > 5
                                                        || crate::track_func::vehicle_exit_dir(
                                                            t.vehicle().direction(),
                                                            t.track(),
                                                            self.check(),
                                                        ) != exitdir
                                                    {
                                                        return false;
                                                    }

                                                    true
                                                },
                                            ) {
                                                return false;
                                            }
                                        }
                                    }

                                    /* If we would reverse but are currently in a PBS block and
                                     * reversing of stuck trains is disabled, don't reverse.
                                     * This does not apply if the reason for reversing is a one-way
                                     * signal blocking us, because a train would then be stuck forever. */
                                    if !self.reverse_at_signals()
                                        && !(self.pbs().0.blocking)(gp.new_tile.0, i.0)
                                        && self.update_signals_on_segment(
                                            v.vehicle().tile(),
                                            enterdir,
                                            v.vehicle().owner(),
                                        ) == SigSegState::SIGSEG_PBS
                                    {
                                        v.set_wait_counter(0);
                                        return false;
                                    }
                                    break 'controller false;
                                }
                                self.try_reserve_rail_track(
                                    gp.new_tile,
                                    Track(crate::track_func::track_bits_to_track(
                                        chosen_track.0,
                                        self.check(),
                                    )),
                                    false,
                                );
                            } else {
                                /* The wagon is active, simply follow the prev vehicle. */
                                if prev.vehicle().tile() == gp.new_tile {
                                    /* Choose the same track as prev */
                                    if prev.track() == TrackBits::TRACK_BIT_WORMHOLE {
                                        /* Vehicles entering tunnels enter the wormhole earlier than for bridges.
                                         * However, just choose the track into the wormhole. */
                                        crate::services_assert::assert_with(|| {
                                            self.world().tunnel(prev.vehicle().tile().0)
                                        });
                                        chosen_track = bits;
                                    } else {
                                        chosen_track = prev.track();
                                    }
                                } else {
                                    /* Choose the track that leads to the tile where prev is.
                                     * This case is active if 'prev' is already on the second next tile, when 'v' just enters the next tile.
                                     * I.e. when the tile between them has only space for a single vehicle like
                                     *  1) horizontal/vertical track tiles and
                                     *  2) some orientations of tunnel entries, where the vehicle is already inside the wormhole at 8/16 from the tile edge.
                                     *     Is also the train just reversing, the wagon inside the tunnel is 'on' the tile of the opposite tunnel entry.
                                     */
                                    const CONNECTING_TRACK: [[TrackBits;
                                        DiagDirection::DIAGDIR_END.0 as usize];
                                        DiagDirection::DIAGDIR_END.0 as usize] = [
                                        [
                                            TrackBits::TRACK_BIT_X,
                                            TrackBits::TRACK_BIT_LOWER,
                                            TrackBits::TRACK_BIT_NONE,
                                            TrackBits::TRACK_BIT_LEFT,
                                        ],
                                        [
                                            TrackBits::TRACK_BIT_UPPER,
                                            TrackBits::TRACK_BIT_Y,
                                            TrackBits::TRACK_BIT_LEFT,
                                            TrackBits::TRACK_BIT_NONE,
                                        ],
                                        [
                                            TrackBits::TRACK_BIT_NONE,
                                            TrackBits::TRACK_BIT_RIGHT,
                                            TrackBits::TRACK_BIT_X,
                                            TrackBits::TRACK_BIT_UPPER,
                                        ],
                                        [
                                            TrackBits::TRACK_BIT_RIGHT,
                                            TrackBits::TRACK_BIT_NONE,
                                            TrackBits::TRACK_BIT_LOWER,
                                            TrackBits::TRACK_BIT_Y,
                                        ],
                                    ];
                                    let exitdir = self
                                        .map()
                                        .diagdir_between_tiles(gp.new_tile, prev.vehicle().tile());
                                    crate::services_assert::assert_with(|| {
                                        is_valid_diag_direction(exitdir)
                                    });
                                    chosen_track = CONNECTING_TRACK[usize::from(enterdir.0)]
                                        [usize::from(exitdir.0)];
                                }
                                chosen_track.0 &= bits.0;
                            }

                            /* Make sure chosen track is a valid track */
                            crate::services_assert::assert_with(|| {
                                matches!(
                                    chosen_track,
                                    TrackBits::TRACK_BIT_X
                                        | TrackBits::TRACK_BIT_Y
                                        | TrackBits::TRACK_BIT_UPPER
                                        | TrackBits::TRACK_BIT_LOWER
                                        | TrackBits::TRACK_BIT_LEFT
                                        | TrackBits::TRACK_BIT_RIGHT
                                )
                            });

                            /* Update XY to reflect the entrance to the new tile, and select the direction to use */
                            let b = INITIAL_TILE_SUBCOORD[usize::from(
                                crate::bitmath_func::find_first_bit(u64::from(chosen_track.0)),
                            )][usize::from(enterdir.0)];
                            gp.x = (gp.x & !0xF) | i32::from(b[0]);
                            gp.y = (gp.y & !0xF) | i32::from(b[1]);
                            let chosen_dir = Direction(b[2]);

                            /* Call the landscape function and tell it that the vehicle entered the tile */
                            let vets = self.vehicle_enter_tile(v, gp.new_tile, gp.x, gp.y);
                            if vets.test(VehicleEnterTileState::CannotEnter) {
                                break 'controller true;
                            }

                            if !vets.test(VehicleEnterTileState::EnteredWormhole) {
                                if v.is_front_engine() {
                                    crate::witness::hit(crate::witness::TRAIN_FIRST_TILE_RAIL);
                                }
                                let track =
                                    Track(crate::track_func::find_first_track(chosen_track.0));
                                let tdir = crate::track_func::track_direction_to_trackdir(
                                    track,
                                    chosen_dir,
                                    self.check(),
                                );
                                if v.is_front_engine()
                                    && (self.pbs().0.has_pbs)(gp.new_tile.0, tdir.0)
                                {
                                    // SAFETY: The PBS table names the immutable rail map table.
                                    (unsafe { &*self.pbs().0.rail }.set_signal_state)(
                                        gp.new_tile.0,
                                        tdir.0,
                                        crate::signal_type::SignalState::SIGNAL_STATE_RED.0,
                                    );
                                    self.mark_tile_dirty_by_tile(gp.new_tile);
                                }

                                /* Clear any track reservation when the last vehicle leaves the tile */
                                if v.next() == Handle::NONE {
                                    reservation::clear_path_reservation(
                                        &self.reservation(v),
                                        v,
                                        v.vehicle().tile(),
                                        self.get_vehicle_trackdir(v),
                                    );
                                }

                                v.vehicle().set_tile(gp.new_tile);

                                if self.get_tile_rail_type(gp.new_tile)
                                    != self.get_tile_rail_type(gp.old_tile)
                                {
                                    self.consist_changed(v.first(), CCF_TRACK);
                                }

                                v.set_track(chosen_track);
                                crate::services_assert::assert_with(|| {
                                    v.track() != TrackBits::TRACK_BIT_NONE
                                });
                            }

                            /* We need to update signal status, but after the vehicle position hash
                             * has been updated by UpdateInclination() */
                            update_signals_crossing = true;

                            if chosen_dir != v.vehicle().direction() {
                                if prev == Handle::NONE
                                    && self.train_acceleration_model()
                                        == AccelerationModel::AM_ORIGINAL
                                {
                                    let asp = &ACCEL_SLOWDOWN[self.get_acceleration_type(v)];
                                    let diff = dir_difference(
                                        v.vehicle().direction(),
                                        chosen_dir,
                                        self.check(),
                                    );
                                    let fraction = if diff == DirDiff::DIRDIFF_45RIGHT
                                        || diff == DirDiff::DIRDIFF_45LEFT
                                    {
                                        asp.small_turn
                                    } else {
                                        asp.large_turn
                                    };
                                    let cur_speed = v.vehicle().cur_speed();
                                    v.vehicle().set_cur_speed(cur_speed.wrapping_sub(
                                        ((u32::from(fraction) * u32::from(cur_speed)) >> 8) as u16,
                                    ));
                                }
                                direction_changed = true;
                                v.vehicle().set_direction(chosen_dir);
                            }

                            if v.is_front_engine() {
                                v.set_wait_counter(0);

                                /* If we are approaching a crossing that is reserved, play the sound now. */
                                let crossing = self.train_approaching_crossing_tile(v);
                                if crossing != TileIndex(INVALID_TILE)
                                    && self.has_crossing_reservation(crossing)
                                    && self.ambient()
                                {
                                    self.snd_play_tile_fx(SoundFx::SND_0E_LEVEL_CROSSING, crossing);
                                }

                                /* Always try to extend the reservation when entering a tile. */
                                reservation::check_next_train_tile(&self.reservation(v), v);
                            }

                            if vets.test(VehicleEnterTileState::EnteredStation) {
                                /* The new position is the location where we want to stop */
                                self.train_enter_station(v, self.get_station_index(gp.new_tile));
                            }
                        }
                    } else if self.is_tile_type(gp.new_tile, TileType::MP_TUNNELBRIDGE)
                        && self
                            .vehicle_enter_tile(v, gp.new_tile, gp.x, gp.y)
                            .test(VehicleEnterTileState::EnteredWormhole)
                    {
                        /* Perform look-ahead on tunnel exit. */
                        if v.is_front_engine() {
                            crate::witness::hit(crate::witness::TRAIN_WORMHOLE_EXIT);
                            self.try_reserve_rail_track(
                                gp.new_tile,
                                Track(crate::track_func::diag_dir_to_diag_track(
                                    self.get_tunnel_bridge_direction(gp.new_tile).0,
                                    self.check(),
                                )),
                                true,
                            );
                            reservation::check_next_train_tile(&self.reservation(v), v);
                        }
                        /* Prevent v->UpdateInclination() being called with wrong parameters.
                         * This could happen if the train was reversed inside the tunnel/bridge. */
                        if gp.old_tile == gp.new_tile {
                            gp.old_tile = self.get_other_tunnel_bridge_end(gp.old_tile);
                        }
                    } else {
                        v.vehicle().set_x_pos(gp.x);
                        v.vehicle().set_y_pos(gp.y);
                        self.update_position(v);
                        if !v.vehicle().vehstatus().test(VehState::Hidden) {
                            (self.vs.base_viewport)(v.vehicle());
                        }
                        break 'part;
                    }

                    /* update image of train, as well as delta XY */
                    (self.vs.update_delta)(v.vehicle());

                    v.vehicle().set_x_pos(gp.x);
                    v.vehicle().set_y_pos(gp.y);
                    self.update_position(v);

                    /* update the Z position of the vehicle */
                    let old_z = self.update_inclination(v, gp.new_tile != gp.old_tile, false);

                    if prev == Handle::NONE {
                        /* This is the first vehicle in the train */
                        self.affect_speed_by_z_change(v, old_z);
                    }

                    if update_signals_crossing {
                        if v.is_front_engine()
                            && self.train_moved_change_signals(gp.new_tile, enterdir)
                        {
                            /* We are entering a block with PBS signals right now, but
                             * not through a PBS signal. This means we don't have a
                             * reservation right now. As a conventional signal will only
                             * ever be green if no other train is in the block, getting
                             * a path should always be possible. If the player built
                             * such a strange network that it is not possible, the train
                             * will be marked as stuck and the player has to deal with
                             * the problem. */
                            if (!self.pbs().has_reserved_tracks(gp.new_tile, v.track())
                                && !self.try_reserve_rail_track(
                                    gp.new_tile,
                                    Track(crate::track_func::find_first_track(v.track().0)),
                                    true,
                                ))
                                || !reservation::try_path_reserve(
                                    &self.reservation(v),
                                    v,
                                    false,
                                    false,
                                )
                            {
                                self.mark_train_as_stuck(v);
                            }
                        }

                        /* Signals can only change when the first
                         * (above) or the last vehicle moves. */
                        if v.next() == Handle::NONE {
                            self.train_moved_change_signals(
                                gp.old_tile,
                                reverse_diag_dir(enterdir, self.check()),
                            );
                            if self.is_level_crossing_tile(gp.old_tile) {
                                self.update_level_crossing(gp.old_tile, true, false);
                            }
                        }
                    }

                    /* Do not check on every tick to save some computing time. */
                    if v.is_front_engine()
                        && v.vehicle()
                            .tick_counter()
                            .is_multiple_of(self.path_backoff_interval())
                    {
                        reservation::check_next_train_tile(&self.reservation(v), v);
                    }
                }
                prev = v;
                v = v.next();
            }

            if direction_changed {
                first.set_cached_max_curve_speed(self.get_curve_speed_limit(first));
            }

            return true;
        };

        if invalid_rail {
            /* We've reached end of line?? */
            if prev != Handle::NONE {
                crate::error_func::fatal_error(self.services, "Disconnecting train");
            }
        }

        /* reverse_train_direction: */
        if reverse {
            v.set_wait_counter(0);
            v.vehicle().set_cur_speed(0);
            v.vehicle().set_subspeed(0);
            self.reverse_train_direction(v);
        }

        false
    }
}

impl Game<'_> {
    /// `IsRailStationPlatformOccupied`.
    fn is_rail_station_platform_occupied(&self, tile: TileIndex) -> bool {
        let world = self.world();
        let delta = self
            .map()
            .offs_by_axis(world.get_rail_station_axis(tile), self.check());

        let mut t = tile;
        while world.is_compatible_train_station_tile(t, tile) {
            if crate::vehicle_func::has_vehicle_on_tile(self.vs, self.map(), t, is_train) {
                return true;
            }
            t = t - delta;
        }
        let mut t = tile + delta;
        while world.is_compatible_train_station_tile(t, tile) {
            if crate::vehicle_func::has_vehicle_on_tile(self.vs, self.map(), t, is_train) {
                return true;
            }
            t += delta;
        }

        false
    }

    /// `DeleteLastWagon`: deletes/clears the last wagon of a crashed train. It takes the
    /// engine of the train, then goes to the last wagon and deletes that. Each call to this
    /// function will remove the last wagon of a crashed train. If this wagon was on a
    /// crossing, or inside a tunnel/bridge, recalculate the signals as they might need
    /// updating.
    fn delete_last_wagon(&self, mut v: Handle) {
        crate::witness::hit(crate::witness::TRAIN_CRASH_DELETE);
        let first = v.first();

        /* Go to the last wagon and delete the link pointing there
         * new_last is then the one-before-last wagon, and v the last
         * one which will physically be removed */
        let mut new_last = v;
        while v.next() != Handle::NONE {
            new_last = v;
            v = v.next();
        }
        // SAFETY: The crashed chain stays live through the unlinking.
        unsafe { crate::vehicle_base::set_next(new_last.record, std::ptr::null_mut()) };

        if first != v {
            /* Recalculate cached train properties */
            self.consist_changed(first, CCF_ARRANGE);
            /* Update the depot window if the first vehicle is in depot -
             * if v == first, then it is updated in PreDestructor() */
            if first.track() == TrackBits::TRACK_BIT_DEPOT {
                set_window_dirty(
                    WindowClass::WC_VEHICLE_DEPOT,
                    WindowNumber::from(first.vehicle().tile()),
                );
            }
            v.vehicle()
                .set_last_station_visited(first.vehicle().last_station_visited()); // for PreDestructor
        }

        /* 'v' shouldn't be accessed after it has been deleted */
        let mut trackbits = v.track();
        let tile = v.vehicle().tile();
        let owner = v.vehicle().owner();

        (self.vs.destroy)(v.vehicle());

        if trackbits == TrackBits::TRACK_BIT_WORMHOLE {
            /* Vehicle is inside a wormhole, v->track contains no useful value then. */
            trackbits = TrackBits(crate::track_func::diag_dir_to_diag_track_bits(
                self.get_tunnel_bridge_direction(tile).0,
                self.check(),
            ));
        }

        let track = Track(crate::track_func::track_bits_to_track(
            trackbits.0,
            self.check(),
        ));
        if self.pbs().has_reserved_tracks(tile, trackbits) {
            self.pbs().unreserve_rail_track(tile, track);

            /* If there are still crashed vehicles on the tile, give the track reservation to them */
            let mut remaining_trackbits = TrackBits::TRACK_BIT_NONE;
            crate::vehicle_func::vehicles_on_tile(self.vs, self.map(), tile, |u| {
                if u.vehicle_type() != VehicleType::VEH_TRAIN
                    || !u.vehstatus().test(VehState::Crashed)
                {
                    return;
                }
                let train_tbits = Handle::from_vehicle(u).track();
                if train_tbits == TrackBits::TRACK_BIT_WORMHOLE {
                    /* Vehicle is inside a wormhole, u->track contains no useful value then. */
                    remaining_trackbits.0 |= crate::track_func::diag_dir_to_diag_track_bits(
                        self.get_tunnel_bridge_direction(u.tile()).0,
                        self.check(),
                    );
                } else if train_tbits != TrackBits::TRACK_BIT_DEPOT {
                    remaining_trackbits.0 |= train_tbits.0;
                }
            });

            /* It is important that these two are the first in the loop, as reservation cannot deal with every trackbit combination */
            crate::services_assert::assert_with(|| {
                Track::TRACK_BEGIN == Track::TRACK_X && Track::TRACK_Y.0 == Track::TRACK_BEGIN.0 + 1
            });
            for t in crate::track_func::set_track_bit_iterator(remaining_trackbits) {
                self.try_reserve_rail_track(tile, t, true);
            }
        }

        /* check if the wagon was on a road/rail-crossing */
        if self.is_level_crossing_tile(tile) {
            self.update_level_crossing(tile, true, false);
        }

        if self.is_rail_station_tile(tile) {
            let occupied = self.is_rail_station_platform_occupied(tile);
            let dir = axis_to_diag_dir(self.world().get_rail_station_axis(tile), self.check());
            self.pbs()
                .set_rail_station_platform_reservation(tile, dir, occupied);
            self.pbs().set_rail_station_platform_reservation(
                tile,
                reverse_diag_dir(dir, self.check()),
                occupied,
            );
        }

        /* Update signals */
        if self.is_tile_type(tile, TileType::MP_TUNNELBRIDGE) || self.is_rail_depot_tile(tile) {
            self.update_signals_on_segment(tile, DiagDirection::INVALID_DIAGDIR, owner);
        } else {
            self.signals().set_signals_on_both_dir(tile, track, owner);
        }
    }

    /// `ChangeTrainDirRandomly`: rotate all vehicles of a (crashed) train chain randomly to
    /// animate the crash. `v` is the first crashed vehicle.
    fn change_train_dir_randomly(&self, mut v: Handle) {
        const DELTA: [DirDiff; 4] = [
            DirDiff::DIRDIFF_45LEFT,
            DirDiff::DIRDIFF_SAME,
            DirDiff::DIRDIFF_SAME,
            DirDiff::DIRDIFF_45RIGHT,
        ];

        loop {
            /* We don't need to twist around vehicles if they're not visible */
            if !v.vehicle().vehstatus().test(VehState::Hidden) {
                v.vehicle().set_direction(change_dir(
                    v.vehicle().direction(),
                    DELTA[(self.services.random() & 3) as usize],
                    self.check(),
                ));
                /* Refrain from updating the z position of the vehicle when on
                 * a bridge, because UpdateInclination() will put the vehicle under
                 * the bridge in that case */
                if v.track() == TrackBits::TRACK_BIT_WORMHOLE {
                    self.viewport(v, false, true);
                } else {
                    self.update_position(v);
                    self.update_inclination(v, false, true);
                }
            }
            v = v.next();
            if v == Handle::NONE {
                break;
            }
        }
    }

    /// `HandleCrashedTrain`: handle a crashed train. `v` is the first train vehicle. Returns
    /// whether the vehicle chain still exists.
    fn handle_crashed_train(&self, v: Handle) -> bool {
        v.set_crash_anim_pos(v.crash_anim_pos().wrapping_add(1));
        let state = i32::from(v.crash_anim_pos());

        if state == 4 && !v.vehicle().vehstatus().test(VehState::Hidden) {
            crate::effect::create_effect_vehicle_rel(
                v.vehicle(),
                4,
                4,
                8,
                crate::effectvehicle_func::EffectVehicleType::EV_EXPLOSION_LARGE,
            );
        }

        if state <= 200 {
            let mut r = 0;
            if crate::random_func::chance16r(self.services, 1, 7, &mut r) {
                let mut index = (r.wrapping_mul(10) >> 16) as i32;

                let mut u = v.vehicle();
                loop {
                    index -= 1;
                    if index < 0 {
                        r = self.services.random();

                        crate::effect::create_effect_vehicle_rel(
                            u,
                            crate::bitmath_func::gb(r, 8, 3) as i32 + 2,
                            crate::bitmath_func::gb(r, 16, 3) as i32 + 2,
                            crate::bitmath_func::gb(r, 0, 3) as i32 + 5,
                            crate::effectvehicle_func::EffectVehicleType::EV_EXPLOSION_SMALL,
                        );
                        break;
                    }
                    match u.next() {
                        Some(next) => u = next,
                        None => break,
                    }
                }
            }
        }

        if state <= 240 && v.vehicle().tick_counter().trailing_zeros() >= 2 {
            self.change_train_dir_randomly(v);
        }

        if state >= 4440 && v.vehicle().tick_counter().trailing_zeros() >= 5 {
            let ret = v.next() != Handle::NONE;
            self.delete_last_wagon(v);
            return ret;
        }

        true
    }

    /// `TrainApproachingLineEnd`: train is approaching line end, slow down and possibly
    /// reverse. `v` is the front train engine; `signal` means not line end, just a red
    /// signal; `reverse` set to false does not execute the vehicle reversing, which does not
    /// change any other logic. Returns true iff we did NOT have to reverse.
    fn train_approaching_line_end(&self, v: Handle, signal: bool, reverse: bool) -> bool {
        /* Calc position within the current tile */
        let mut x = v.vehicle().x_pos() as u32 & 0xF;
        let y = v.vehicle().y_pos() as u32 & 0xF;

        /* for diagonal directions, 'x' will be 0..15 -
         * for other directions, it will be 1, 3, 5, ..., 15 */
        match v.vehicle().direction() {
            Direction::DIR_N => x = (!x).wrapping_add(!y).wrapping_add(25),
            Direction::DIR_NW => {
                x = y;
                x = (!x).wrapping_add(16);
            }
            Direction::DIR_NE => x = (!x).wrapping_add(16),
            Direction::DIR_E => x = (!x).wrapping_add(y).wrapping_add(9),
            Direction::DIR_SE => x = y,
            Direction::DIR_S => x = x.wrapping_add(y).wrapping_sub(7),
            Direction::DIR_W => x = (!y).wrapping_add(x).wrapping_add(9),
            _ => {}
        }

        /* Do not reverse when approaching red signal. Make sure the vehicle's front
         * does not cross the tile boundary when we do reverse, but as the vehicle's
         * location is based on their center, use half a vehicle's length as offset.
         * Multiply the half-length by two for straight directions to compensate that
         * we only get odd x offsets there. */
        if !signal
            && x.wrapping_add(
                u32::from(v.cached_veh_length()).div_ceil(2)
                    * if crate::direction_func::is_diagonal_direction(
                        v.vehicle().direction(),
                        self.check(),
                    ) {
                        1
                    } else {
                        2
                    },
            ) >= TILE_SIZE
        {
            /* we are too near the tile end, reverse now */
            v.vehicle().set_cur_speed(0);
            if reverse {
                self.reverse_train_direction(v);
            }
            return false;
        }

        /* slow down */
        v.vehicle()
            .set_vehstatus(v.vehicle().vehstatus().set(VehState::TrainSlowing));
        let break_speed = BREAKDOWN_SPEEDS[(x & 0xF) as usize];
        if break_speed < v.vehicle().cur_speed() {
            v.vehicle().set_cur_speed(break_speed);
        }

        true
    }

    /// `TrainCanLeaveTile`: determines whether train would like to leave the tile. Returns
    /// true iff vehicle is NOT entering or inside a depot or tunnel/bridge.
    fn train_can_leave_tile(&self, v: Handle) -> bool {
        /* Exit if inside a tunnel/bridge or a depot */
        if v.track() == TrackBits::TRACK_BIT_WORMHOLE || v.track() == TrackBits::TRACK_BIT_DEPOT {
            return false;
        }

        let tile = v.vehicle().tile();

        /* entering a tunnel/bridge? */
        if self.is_tile_type(tile, TileType::MP_TUNNELBRIDGE) {
            let dir = self.get_tunnel_bridge_direction(tile);
            if diag_dir_to_dir(dir, self.check()) == v.vehicle().direction() {
                return false;
            }
        }

        /* entering a depot? */
        if self.is_rail_depot_tile(tile) {
            let dir = reverse_diag_dir(self.get_rail_depot_direction(tile), self.check());
            if diag_dir_to_dir(dir, self.check()) == v.vehicle().direction() {
                return false;
            }
        }

        true
    }

    /// `TrainApproachingCrossingTile`: determines whether train is approaching a rail-road
    /// crossing (thus making it barred). `v` is the front engine of train. Returns the
    /// `TileIndex` of crossing the train is approaching, else `INVALID_TILE`.
    /// Precondition: `v` is a non-crashed front engine.
    fn train_approaching_crossing_tile(&self, v: Handle) -> TileIndex {
        crate::services_assert::assert_with(|| v.is_front_engine());
        crate::services_assert::assert_with(|| !v.vehicle().vehstatus().test(VehState::Crashed));

        if !self.train_can_leave_tile(v) {
            return TileIndex(INVALID_TILE);
        }

        let dir =
            crate::track_func::vehicle_exit_dir(v.vehicle().direction(), v.track(), self.check());
        let tile = v.vehicle().tile() + self.tile_offs_by_diag_dir(dir);

        /* not a crossing || wrong axis || unusable rail (wrong type or owner) */
        if !self.is_level_crossing_tile(tile)
            || diag_dir_to_axis(dir, self.check()) == self.get_crossing_road_axis(tile)
            || !self.check_compatible_rail(v, tile)
        {
            return TileIndex(INVALID_TILE);
        }

        tile
    }

    /// `TrainCheckIfLineEnds`: checks for line end. Also, bars crossing at next tile if
    /// needed. `reverse` set to false does not execute the vehicle reversing, which does not
    /// change any other logic. Returns true iff we did NOT have to reverse.
    fn train_check_if_line_ends(&self, v: Handle, reverse: bool) -> bool {
        /* First, handle broken down train */

        let t = i32::from(v.vehicle().breakdown_ctr());
        if t > 1 {
            v.vehicle()
                .set_vehstatus(v.vehicle().vehstatus().set(VehState::TrainSlowing));

            let break_speed = BREAKDOWN_SPEEDS[crate::bitmath_func::gb(!t as u32, 4, 4) as usize];
            if break_speed < v.vehicle().cur_speed() {
                v.vehicle().set_cur_speed(break_speed);
            }
        } else {
            v.vehicle()
                .set_vehstatus(v.vehicle().vehstatus().reset(VehState::TrainSlowing));
        }

        if !self.train_can_leave_tile(v) {
            return true;
        }

        /* Determine the non-diagonal direction in which we will exit this tile */
        let dir =
            crate::track_func::vehicle_exit_dir(v.vehicle().direction(), v.track(), self.check());
        /* Calculate next tile */
        let tile = v.vehicle().tile() + self.tile_offs_by_diag_dir(dir);

        /* Determine the track status on the next tile */
        let ts = self.get_tile_track_status(tile, reverse_diag_dir(dir, self.check()));
        let reachable_trackdirs = TrackdirBits(crate::track_func::diagdir_reaches_trackdirs(dir.0));

        let trackdirbits = TrackdirBits(
            crate::track_func::track_status_to_trackdir_bits(ts) & reachable_trackdirs.0,
        );
        let red_signals = TrackdirBits(
            crate::track_func::track_status_to_red_signals(ts).0 & reachable_trackdirs.0,
        );

        /* We are sure the train is not entering a depot, it is detected above */

        /* mask unreachable track bits if we are forbidden to do 90deg turns */
        let mut bits = TrackBits(crate::track_func::trackdir_bits_to_track_bits(
            trackdirbits.0,
        ));
        if self.rail_90_deg_turn_disallowed(
            self.get_tile_rail_type(v.vehicle().tile()),
            self.get_tile_rail_type(tile),
        ) {
            bits.0 &= !crate::track_func::track_crosses_tracks(
                Track(crate::track_func::find_first_track(v.track().0)),
                self.check(),
            )
            .0;
        }

        /* no suitable trackbits at all || unusable rail (wrong type or owner) */
        if bits == TrackBits::TRACK_BIT_NONE || !self.check_compatible_rail(v, tile) {
            return self.train_approaching_line_end(v, false, reverse);
        }

        /* approaching red signal */
        if trackdirbits.0 & red_signals.0 != 0 {
            return self.train_approaching_line_end(v, true, reverse);
        }

        /* approaching a rail/road crossing? then make it red */
        if self.is_level_crossing_tile(tile) {
            self.maybe_bar_crossing_with_sound(tile);
        }

        true
    }

    /// `TrainLocoHandler`.
    fn train_loco_handler(&self, v: Handle, mode: bool) -> bool {
        crate::witness::hit(if mode {
            crate::witness::TRAIN_TICK_SECOND
        } else {
            crate::witness::TRAIN_TICK_FIRST
        });
        /* train has crashed? */
        if v.vehicle().vehstatus().test(VehState::Crashed) {
            return if mode {
                true
            } else {
                self.handle_crashed_train(v)
            }; // 'this' can be deleted here
        }

        if v.force_proceed() != TrainForceProceeding::TFP_NONE {
            v.set_flags(v.flags().reset(VehicleRailFlag::Stuck));
            self.set_start_stop_dirty(v);
        }

        /* train is broken down? */
        if self.maintain(v, maintenance::vehicle_handle_breakdown) {
            return true;
        }

        if v.flags().test(VehicleRailFlag::Reversing) && v.vehicle().cur_speed() == 0 {
            self.reverse_train_direction(v);
        }

        /* exit if train is stopped */
        if v.vehicle().vehstatus().test(VehState::Stopped) && v.vehicle().cur_speed() == 0 {
            return true;
        }

        let valid_order = !v.vehicle().current_order().is_type(OrderType::OT_NOTHING)
            && v.vehicle().current_order().kind() != OrderType::OT_CONDITIONAL;
        if crate::orders::process_orders(v.vehicle(), self.vs.orders())
            && reservation::check_reverse_train(&self.reservation(v), v)
        {
            v.set_wait_counter(0);
            v.vehicle().set_cur_speed(0);
            v.vehicle().set_subspeed(0);
            v.set_flags(v.flags().reset(VehicleRailFlag::LeavingStation));
            self.reverse_train_direction(v);
            return true;
        } else if v.flags().test(VehicleRailFlag::LeavingStation) {
            /* Try to reserve a path when leaving the station as we
             * might not be marked as wanting a reservation, e.g.
             * when an overlength train gets turned around in a station. */
            let mut dir = crate::track_func::vehicle_exit_dir(
                v.vehicle().direction(),
                v.track(),
                self.check(),
            );
            if self.is_rail_depot_tile(v.vehicle().tile())
                || self.is_tile_type(v.vehicle().tile(), TileType::MP_TUNNELBRIDGE)
            {
                dir = DiagDirection::INVALID_DIAGDIR;
            }

            if self.update_signals_on_segment(v.vehicle().tile(), dir, v.vehicle().owner())
                == SigSegState::SIGSEG_PBS
                || self.reserve_paths()
            {
                reservation::try_path_reserve(&self.reservation(v), v, true, true);
            }
            v.set_flags(v.flags().reset(VehicleRailFlag::LeavingStation));
        }

        self.vs.handle_loading(v.vehicle(), mode);

        if v.vehicle().current_order().is_type(OrderType::OT_LOADING) {
            return true;
        }

        if self.check_train_stay_in_depot(v) {
            return true;
        }

        if !mode && crate::vehicle_effect::show_visual_effect_allowed(v.vehicle()) {
            crate::vehicle_effect::show_visual_effect(
                v.vehicle(),
                self.get_current_max_speed(v) as u32,
            );
        }

        /* We had no order but have an order now, do look ahead. */
        if !valid_order && !v.vehicle().current_order().is_type(OrderType::OT_NOTHING) {
            reservation::check_next_train_tile(&self.reservation(v), v);
        }

        /* Handle stuck trains. */
        if !mode && v.flags().test(VehicleRailFlag::Stuck) {
            crate::witness::hit(crate::witness::TRAIN_STUCK_RETRY);
            v.set_wait_counter(v.wait_counter().wrapping_add(1));

            /* Should we try reversing this tick if still stuck? */
            let turn_around = i32::from(v.wait_counter())
                % (i32::from(self.wait_for_pbs_path()) * crate::timer_game_tick::DAY_TICKS)
                == 0
                && self.reverse_at_signals();

            if !turn_around
                && i32::from(v.wait_counter()) % i32::from(self.path_backoff_interval()) != 0
                && v.force_proceed() == TrainForceProceeding::TFP_NONE
            {
                return true;
            }
            if !reservation::try_path_reserve(&self.reservation(v), v, false, false) {
                /* Still stuck. */
                if turn_around {
                    crate::witness::hit(crate::witness::TRAIN_STUCK_REVERSE);
                    self.reverse_train_direction(v);
                }

                if v.flags().test(VehicleRailFlag::Stuck)
                    && i32::from(v.wait_counter())
                        > 2 * i32::from(self.wait_for_pbs_path())
                            * crate::timer_game_tick::DAY_TICKS
                {
                    /* Show message to player. */
                    if self.lost_vehicle_warn()
                        && v.vehicle().owner().0 == (self.services.local_company)()
                    {
                        add_vehicle_advice_news_item(
                            AdviceType::TrainStuck,
                            &get_encoded_string(
                                STR_NEWS_TRAIN_IS_STUCK,
                                &[Integer(u64::from(self.index(v).0))],
                            ),
                            self.index(v),
                        );
                    }
                    v.set_wait_counter(0);
                }
                /* Exit if force proceed not pressed, else reset stuck flag anyway. */
                if v.force_proceed() == TrainForceProceeding::TFP_NONE {
                    return true;
                }
                v.set_flags(v.flags().reset(VehicleRailFlag::Stuck));
                v.set_wait_counter(0);
                self.set_start_stop_dirty(v);
            }
        }

        if v.vehicle()
            .current_order()
            .is_type(OrderType::OT_LEAVESTATION)
        {
            crate::orders::free(v.vehicle());
            self.set_start_stop_dirty(v);
            return true;
        }

        let mut j = self.update_speed(v);

        /* we need to invalidate the widget if we are stopping from 'Stopping 0 km/h' to 'Stopped' */
        if v.vehicle().cur_speed() == 0 && v.vehicle().vehstatus().test(VehState::Stopped) {
            /* If we manually stopped, we're not force-proceeding anymore. */
            v.set_force_proceed(TrainForceProceeding::TFP_NONE);
            self.set_vehicle_view_dirty(v);
        }

        let mut adv_spd = v.vehicle().get_advance_distance() as i32;
        if j < adv_spd {
            /* if the vehicle has speed 0, update the last_speed field. */
            if v.vehicle().cur_speed() == 0 {
                self.set_last_speed(v);
            }
        } else {
            self.train_check_if_line_ends(v, true);
            /* Loop until the train has finished moving. */
            loop {
                j = j.wrapping_sub(adv_spd);
                self.train_controller(v, Handle::NONE, true);
                /* Don't continue to move if the train crashed. */
                if self.check_train_collision(v) {
                    break;
                }
                /* Determine distance to next map position */
                adv_spd = v.vehicle().get_advance_distance() as i32;

                /* No more moving this tick */
                if j < adv_spd || v.vehicle().cur_speed() == 0 {
                    break;
                }

                let order_type = v.vehicle().current_order().kind();
                /* Do not skip waypoints (incl. 'via' stations) when passing through at full speed. */
                if (order_type == OrderType::OT_GOTO_WAYPOINT
                    || order_type == OrderType::OT_GOTO_STATION)
                    && v.vehicle()
                        .current_order()
                        .non_stop()
                        .test(OrderNonStopFlag::NoDestination)
                    && self.is_tile_type(v.vehicle().tile(), TileType::MP_STATION)
                    && v.vehicle().current_order().destination().to_station_id()
                        == self.get_station_index(v.vehicle().tile())
                {
                    crate::orders::process_orders(v.vehicle(), self.vs.orders());
                }
            }
            self.set_last_speed(v);
        }

        let mut u = v;
        while u != Handle::NONE {
            if !u.vehicle().vehstatus().test(VehState::Hidden) {
                self.viewport(u, false, false);
            }
            u = u.next();
        }

        if v.vehicle().progress() == 0 {
            v.vehicle().set_progress(j as u8); // Save unused spd for next time, if TrainController didn't set progress
        }

        true
    }

    /// `Train::GetRunningCost`: get running cost for the train consist. Returns yearly
    /// running costs.
    fn get_running_cost(&self, this: Handle) -> i64 {
        let mut cost = 0_i64;
        let mut v = this;

        loop {
            'part: {
                let e = self.engine(v);
                if e.train_price() == crate::economy_type::INVALID_PRICE {
                    break 'part;
                }

                let mut cost_factor = self.get_vehicle_property(
                    v,
                    PropertyID::PROP_TRAIN_RUNNING_COST_FACTOR,
                    i32::from(e.train_running()),
                ) as u32;
                if cost_factor == 0 {
                    break 'part;
                }

                /* Halve running cost for multiheaded parts */
                if v.is_multiheaded() {
                    cost_factor /= 2;
                }

                cost = cost.saturating_add(crate::engine_capabilities::price(
                    self.vs.newgrf().capabilities,
                    e.0.as_ptr(),
                    u32::from(e.train_price()),
                    cost_factor,
                    0,
                ));
            }
            v = v.get_next_vehicle();
            if v == Handle::NONE {
                break;
            }
        }

        cost
    }

    /// `Train::Tick`: update train vehicle data for a tick. Returns true if the vehicle
    /// still exists, false if it has ceased to exist (front of consists only).
    fn tick(&self, this: Handle) -> bool {
        this.vehicle()
            .set_tick_counter(this.vehicle().tick_counter().wrapping_add(1));

        if this.is_front_engine() {
            if !this.vehicle().vehstatus().test(VehState::Stopped) || this.vehicle().cur_speed() > 0
            {
                this.vehicle()
                    .set_running_ticks(this.vehicle().running_ticks().wrapping_add(1));
            }

            this.vehicle()
                .set_current_order_time(this.vehicle().current_order_time().wrapping_add(1));

            if !self.train_loco_handler(this, false) {
                return false;
            }

            return self.train_loco_handler(this, true);
        } else if this.is_free_wagon() && this.vehicle().vehstatus().test(VehState::Crashed) {
            /* Delete flooded standalone wagon chain */
            this.set_crash_anim_pos(this.crash_anim_pos().wrapping_add(1));
            if this.crash_anim_pos() >= 4400 {
                crate::witness::hit(crate::witness::TRAIN_FREE_WAGON_DELETE);
                (self.vs.destroy)(this.vehicle());
                return false;
            }
        }

        true
    }

    /// `CheckIfTrainNeedsService`: check whether a train needs service, and if so, find a
    /// depot or service it.
    fn check_if_train_needs_service(&self, v: Handle) {
        if self.servint_trains(v.vehicle().owner()) == 0
            || !self.maintain(v, maintenance::needs_automatic_servicing)
        {
            return;
        }
        if self.is_chain_in_depot(v) {
            crate::witness::hit(crate::witness::TRAIN_DEPOT_SERVICE);
            self.maintain(v, maintenance::service_in_depot);
            return;
        }

        let max_penalty = self.maximum_go_to_depot_penalty();

        let tfdd = self.find_closest_train_depot(v, max_penalty as i32);
        /* Only go to the depot if it is not too far out of our way. */
        if tfdd.best_length == u32::MAX || tfdd.best_length > max_penalty {
            if v.vehicle()
                .current_order()
                .is_type(OrderType::OT_GOTO_DEPOT)
            {
                /* If we were already heading for a depot but it has
                 * suddenly moved farther away, we continue our normal
                 * schedule? */
                // SAFETY: The current order belongs to this live serialized record.
                unsafe { crate::orders::make_dummy(v.vehicle().current_order_pointer()) };
                self.set_start_stop_dirty(v);
            }
            return;
        }

        let depot = crate::order_type::DestinationID((self.services.depot_index)(tfdd.tile.0));

        if v.vehicle()
            .current_order()
            .is_type(OrderType::OT_GOTO_DEPOT)
            && v.vehicle().current_order().destination() != depot
            && !crate::random_func::chance16(self.services, 3, 16)
        {
            return;
        }

        let mut gv_flags = read_field!(v.vehicle().ground(), flags);
        crate::bitmath_func::set_bit16(
            &mut gv_flags,
            GroundVehicleFlags::GVF_SUPPRESS_IMPLICIT_ORDERS.0,
        );
        write_field!(v.vehicle().ground(), flags, gv_flags);
        // SAFETY: The current order belongs to this live serialized record.
        unsafe {
            crate::orders::make_go_to_depot(
                v.vehicle().current_order_pointer(),
                depot,
                crate::order_type::OrderDepotTypeFlags::from_element(
                    crate::order_type::OrderDepotTypeFlag::Service,
                ),
                crate::order_type::OrderNonStopFlags::from_element(
                    OrderNonStopFlag::NoIntermediate,
                ),
                crate::order_type::OrderDepotActionFlags::from_element(
                    crate::order_type::OrderDepotActionFlag::NearestDepot,
                ),
                crate::cargo_type::CargoType::CARGO_NO_REFIT,
            );
        }
        v.vehicle().set_dest_tile(tfdd.tile);
        self.set_start_stop_dirty(v);
    }

    /// `Train::OnNewCalendarDay`: calendar day handler.
    fn on_new_calendar_day(&self, this: Handle) {
        self.maintain(this, maintenance::age_vehicle);
    }

    /// `Train::OnNewEconomyDay`: economy day handler.
    fn on_new_economy_day(&self, this: Handle) {
        self.maintain(this, maintenance::economy_age_vehicle);

        this.vehicle()
            .set_day_counter(this.vehicle().day_counter().wrapping_add(1));
        if this.vehicle().day_counter().trailing_zeros() >= 3 {
            self.maintain(this, maintenance::decrease_vehicle_value);
        }

        if this.is_front_engine() {
            self.maintain(this, maintenance::check_vehicle_breakdown);

            self.check_if_train_needs_service(this);

            crate::orders::check_orders(this.vehicle(), self.vs.orders());

            /* update destination */
            if this
                .vehicle()
                .current_order()
                .is_type(OrderType::OT_GOTO_STATION)
            {
                let st =
                    self.station_get(this.vehicle().current_order().destination().to_station_id());
                // SAFETY: The shared table names the immutable station command table.
                let commands = unsafe { &*(self.services.station_commands)() };
                let tile = (commands.tile_area)(st, StationType::Rail.0).tile;
                if tile != TileIndex(INVALID_TILE) {
                    this.vehicle().set_dest_tile(tile);
                }
            }

            if this.vehicle().running_ticks() != 0 {
                /* running costs */
                let cost = self
                    .get_running_cost(this)
                    .saturating_mul(i64::from(this.vehicle().running_ticks()))
                    / i64::from(
                        crate::timer_game_common::DAYS_IN_YEAR * crate::timer_game_tick::DAY_TICKS,
                    );

                (self.vs.write_profit)(
                    this.vehicle(),
                    (self.vs.profit)(this.vehicle()).saturating_sub(cost),
                );
                this.vehicle().set_running_ticks(0);

                // SAFETY: Base maintenance names the shared static command and company tables.
                let company =
                    unsafe { &*((*(*(*self.vs.maintenance).transaction).commands).company)() };
                crate::company::subtract_money_from_company_fract(
                    company.finance,
                    this.vehicle().owner(),
                    cost,
                    crate::economy_type::ExpensesType::EXPENSES_TRAIN_RUN,
                );

                self.set_details_dirty(this);
                set_window_classes_dirty(WindowClass::WC_TRAINS_LIST);
            }
        }
    }

    /// `Train::GetVehicleTrackdir`: get the tracks of the train vehicle. Returns the current
    /// tracks of the vehicle.
    fn get_vehicle_trackdir(&self, this: Handle) -> Trackdir {
        if this.vehicle().vehstatus().test(VehState::Crashed) {
            return Trackdir::INVALID_TRACKDIR;
        }

        if this.track() == TrackBits::TRACK_BIT_DEPOT {
            /* We'll assume the train is facing outwards */
            return Trackdir(crate::track_func::diag_dir_to_diag_trackdir(
                self.get_rail_depot_direction(this.vehicle().tile()).0,
                self.check(),
            )); // Train in depot
        }

        if this.track() == TrackBits::TRACK_BIT_WORMHOLE {
            /* train in tunnel or on bridge, so just use its direction and assume a diagonal track */
            return Trackdir(crate::track_func::diag_dir_to_diag_trackdir(
                dir_to_diag_dir(this.vehicle().direction(), self.check()).0,
                self.check(),
            ));
        }

        crate::track_func::track_direction_to_trackdir(
            Track(crate::track_func::find_first_track(this.track().0)),
            this.vehicle().direction(),
            self.check(),
        )
    }
}

/// `Train::ConsistChanged`, for Rust callers outside a train entry.
pub(crate) fn consist_changed(
    leaves: &Leaves,
    services: &Services,
    v: Handle,
    allowed_changes: ConsistChangeFlags,
) {
    Game::new(leaves, services).consist_changed(v, allowed_changes);
}
/// `Train::MarkDirty`, for Rust callers outside a train entry.
pub(crate) fn mark_dirty(leaves: &Leaves, services: &Services, v: Handle) {
    Game::new(leaves, services).mark_dirty(v);
}
/// `Train::GetCurrentMaxSpeed`, for Rust callers outside a train entry.
pub(crate) fn get_current_max_speed(leaves: &Leaves, services: &Services, v: Handle) -> i32 {
    Game::new(leaves, services).get_current_max_speed(v)
}
/// `Train::GetCurveSpeedLimit`, for Rust callers outside a train entry.
pub(crate) fn get_curve_speed_limit(leaves: &Leaves, services: &Services, v: Handle) -> u16 {
    Game::new(leaves, services).get_curve_speed_limit(v)
}
/// `GetTrainStopLocation`, for Rust callers outside a train entry: the stop location, the
/// station ahead and the station length.
pub(crate) fn get_train_stop_location(
    leaves: &Leaves,
    services: &Services,
    station_id: StationID,
    tile: TileIndex,
    v: Handle,
) -> (i32, i32, i32) {
    Game::new(leaves, services).get_train_stop_location(station_id, tile, v)
}
/// `TrainOnCrossing`, for Rust callers outside a train entry.
pub(crate) fn train_on_crossing(leaves: &Leaves, services: &Services, tile: TileIndex) -> bool {
    Game::new(leaves, services).train_on_crossing(tile)
}
/// `UpdateLevelCrossing`, for Rust callers outside a train entry.
pub(crate) fn update_level_crossing(
    leaves: &Leaves,
    services: &Services,
    tile: TileIndex,
    sound: bool,
    force_bar: bool,
) {
    Game::new(leaves, services).update_level_crossing(tile, sound, force_bar);
}
/// `Train::GetVehicleTrackdir`, for Rust callers outside a train entry.
pub(crate) fn get_vehicle_trackdir(leaves: &Leaves, services: &Services, v: Handle) -> Trackdir {
    Game::new(leaves, services).get_vehicle_trackdir(v)
}

/// `Train::ConsistChanged`.
/// # Safety
/// The tables and shell must remain live until the call returns; callbacks may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_consist_changed(
    v: Handle,
    allowed_changes: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).consist_changed(v, ConsistChangeFlags(allowed_changes));
}
/// `Train::GetCurrentMaxSpeed`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_current_max_speed(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i32 {
    entry_game!(leaves, services).get_current_max_speed(v)
}
/// `Train::UpdateSpeed`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_update_speed(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i32 {
    entry_game!(leaves, services).update_speed(v)
}
/// `Train::MarkDirty`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_mark_dirty(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).mark_dirty(v);
}
/// `Train::Tick`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_tick(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    u8::from(entry_game!(leaves, services).tick(v))
}
/// `Train::OnNewCalendarDay`: calendar day handler.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_calendar_day(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).on_new_calendar_day(v);
}
/// `Train::OnNewEconomyDay`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_economy_day(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).on_new_economy_day(v);
}
/// `Train::GetRunningCost`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_running_cost(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> i64 {
    entry_game!(leaves, services).get_running_cost(v)
}
/// `Train::GetVehicleTrackdir`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_trackdir(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    entry_game!(leaves, services).get_vehicle_trackdir(v).0
}
/// `MarkTrainAsStuck`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_mark_stuck(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).mark_train_as_stuck(v);
}
/// `ReverseTrainSwapVeh`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse_swap(
    v: Handle,
    l: i32,
    r: i32,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).reverse_train_swap_veh(v, l, r);
}
/// `UpdateLevelCrossing`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_update_crossing(
    tile: u32,
    sound: u8,
    force_bar: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).update_level_crossing(
        TileIndex(tile),
        sound != 0,
        force_bar != 0,
    );
}
/// `MarkDirtyAdjacentLevelCrossingTiles`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_adjacent_crossing_dirty(
    tile: u32,
    road_axis: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services)
        .mark_dirty_adjacent_level_crossing_tiles(TileIndex(tile), Axis(road_axis));
}
/// `UpdateAdjacentLevelCrossingTilesOnLevelCrossingRemoval`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_crossing_removed(
    tile: u32,
    road_axis: u8,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).update_adjacent_level_crossing_tiles_on_level_crossing_removal(
        TileIndex(tile),
        Axis(road_axis),
    );
}
/// `TrainController`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_controller(
    v: Handle,
    nomove: Handle,
    reverse: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u8 {
    u8::from(entry_game!(leaves, services).train_controller(v, nomove, reverse != 0))
}
/// `Train::Crash`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_crash(
    v: Handle,
    flooded: u8,
    leaves: *const Leaves,
    services: *const Services,
) -> u32 {
    entry_game!(leaves, services).crash(v, flooded != 0)
}
/// `CheckIfTrainNeedsService`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_needs_service(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).check_if_train_needs_service(v);
}
/// `Train::FindClosestDepot`.
/// # Safety
/// As [`openttd_rust_train_consist_changed`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_find_closest_depot(
    v: Handle,
    leaves: *const Leaves,
    services: *const Services,
    closest: *mut Closest,
) {
    let result = entry_game!(leaves, services).find_closest_depot(v);
    // SAFETY: As this function's contract; `closest` is a writable record.
    unsafe {
        closest.write(Closest {
            tile: result.location.0,
            destination: result.destination.0,
            reverse: u8::from(result.reverse),
            found: u8::from(result.found),
        });
    };
}
/// `CmdReverseTrainDirection`, over the positional base integers of the command.
/// # Safety
/// As [`openttd_rust_train_consist_changed`]; `out` is the command's live native result.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_reverse_direction(
    build: &crate::vehicle_transactions::build::Services,
    out: *mut crate::services::CommandCost,
    flags: u16,
    veh_id: u32,
    reverse_single_veh: bool,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).cmd_reverse_train_direction(
        &CommandContext {
            rail: build.rail,
            common: build.refit.common,
            out,
            flags: crate::command_type::DoCommandFlags(flags),
        },
        CmdReverseTrainDirection {
            veh_id: VehicleID(veh_id),
            reverse_single_veh,
        },
    );
}
/// `CmdForceTrainProceed`, over the positional base integers of the command.
/// # Safety
/// As [`openttd_rust_train_reverse_direction`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_force_proceed(
    build: &crate::vehicle_transactions::build::Services,
    out: *mut crate::services::CommandCost,
    flags: u16,
    veh_id: u32,
    leaves: *const Leaves,
    services: *const Services,
) {
    entry_game!(leaves, services).cmd_force_train_proceed(
        &CommandContext {
            rail: build.rail,
            common: build.refit.common,
            out,
            flags: crate::command_type::DoCommandFlags(flags),
        },
        CmdForceTrainProceed {
            veh_id: VehicleID(veh_id),
        },
    );
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTrainHandle" ["probe"] => Handle {
        record,
        owner,
    };
    "OpenTTDTrainServices" ["probe"] => Leaves {
        capabilities,
        physics,
        reservation,
        vehicle,
    };
}

#[cfg(test)]
mod evidence_tests;
#[cfg(test)]
mod tests;
