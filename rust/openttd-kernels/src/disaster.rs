/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `disaster_vehicle.cpp`: all disaster/easter egg vehicles, with the `DisasterVehicle`
//! members of `disaster_vehicle.h` whose state Rust owns: `image_override`,
//! `big_ufo_destroyer_target`, `flags` and `state`.
//!
//! The general flow of control for the disaster vehicles is as follows:
//! 1. Initialize the disaster in a disaster specific way (eg start position, possible
//!    target, etc.) `Disaster_XXX_Init()` function
//! 2. Add a subtype to a disaster, which is an index into the function array that handles
//!    the vehicle's ticks.
//! 3. Run the disaster vehicles each tick until their target has been reached, this happens
//!    in the `DisasterTick_XXX()` functions. In here, a vehicle's state is kept by
//!    `v->state` variable. Each achieved sub-target will increase this value, and the last
//!    one will remove the disaster itself
//!
//! Each native `DisasterVehicle` shell owns one [`State`], allocated with the shell and freed
//! by its destructor, and its record's `disaster` link names it. An entry's record and
//! `State` are live and serialized by the game-state mutex for the call. Every field is
//! read and written in place at the original's observation point, no reference into a
//! record or an owner lives across a service, and a deleted vehicle's handle is not used
//! again: each `delete v; return false;` of the original is
//! [`VehicleServices::destroy`] at its statement, followed by the return.
#![allow(
    clippy::cast_possible_truncation,
    reason = "the original's integer stores into narrower members: _disaster_delay = GB(Random(), 0, 9) + 730"
)]
#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "the original's uint coordinates as int: TileX(tile) * TILE_SIZE, GB(r, 0, 6) + x and (uint)x > Map::MaxX() * TILE_SIZE - 1"
)]

use crate::aircraft::{self, ROTOR_Z_OFFSET};
use crate::airport::{AT_LARGE, AT_SMALL, AirportBlock, AirportBlocks};
use crate::bitmath_func::{gb, has_bit};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_base::is_human_id;
use crate::company_type::{CompanyID, Owner};
use crate::direction_func::{change_dir, dir_to_diag_dir};
use crate::direction_type::{DiagDirection, DirDiff, Direction};
use crate::disaster_vehicle::DisasterSubType;
use crate::effect::{create_effect_vehicle_above, create_effect_vehicle_rel};
use crate::effectvehicle_func::EffectVehicleType;
use crate::gfx_type::SpriteID;
use crate::industry_type::IndustryID;
use crate::industrytype::{IndustryBehaviour, IndustryBehaviours, get_industry_spec};
use crate::map_func::{Map, World};
use crate::math::{clamp, delta};
use crate::news_func::{add_industry_news_item, add_tile_news_item};
use crate::news_type::NewsType;
use crate::script_event_types::{CrashReason, ScriptEvent};
use crate::services::Services as Shared;
use crate::sound_type::{SoundFx, SoundID};
use crate::sprites::{
    SPR_AH_64A, SPR_AH_64A_FIRING, SPR_BLIMP, SPR_BLIMP_CRASHED, SPR_BLIMP_CRASHING, SPR_F_15,
    SPR_F_15_FIRING, SPR_ROTOR_MOVING_1, SPR_ROTOR_MOVING_3, SPR_SUB_LARGE_NE, SPR_SUB_LARGE_NW,
    SPR_SUB_LARGE_SE, SPR_SUB_LARGE_SW, SPR_SUB_SMALL_NE, SPR_SUB_SMALL_NW, SPR_SUB_SMALL_SE,
    SPR_SUB_SMALL_SW, SPR_UFO_HARVESTER, SPR_UFO_SMALL_SCOUT, SPR_UFO_SMALL_SCOUT_DARKER,
    SPR_XCOM_SKYRANGER,
};
use crate::station_type::StationID;
use crate::strings_func::get_encoded_string;
use crate::strings_type::StringID;
use crate::strings_type::StringParameterData::Integer;
use crate::table::strings::{
    STR_NEWS_DISASTER_AIRPLANE_OIL_REFINERY, STR_NEWS_DISASTER_BIG_UFO,
    STR_NEWS_DISASTER_COAL_MINE_SUBSIDENCE, STR_NEWS_DISASTER_HELICOPTER_FACTORY,
    STR_NEWS_DISASTER_SMALL_UFO, STR_NEWS_DISASTER_ZEPPELIN,
};
use crate::tile_type::{TILE_SIZE, TileIndex, TileType};
use crate::timer_game_calendar::CalendarYear;
use crate::timer_game_common::MIN_DATE;
use crate::track_func::track_status_to_track_bits;
use crate::track_type::TrackBits;
use crate::transport_type::TransportType;
use crate::vehicle::scheduler::SchedulerServices;
use crate::vehicle::{VehicleRef, VehicleServices, get_direction_towards, get_new_vehicle_pos};
use crate::vehicle_base::{VehState, VehStates, is_ground_vehicle};
use crate::vehicle_type::{VehicleID, VehicleType};
use std::ptr::NonNull;

/// Delay counter for considering the next disaster.
// Process-wide, initialized without runtime construction. The game-state mutex serializes
// access; save/load dereferences its address only when no Rust call is active, and no Rust
// reference to it is formed.
#[allow(non_upper_case_globals, reason = "the original's name")]
static mut _disaster_delay: u16 = 0;

/// The members `DisasterVehicle` adds to `Vehicle` (`disaster_vehicle.h`), which the C++
/// shell's references and save/load read in place.
#[repr(C)]
#[allow(
    clippy::struct_field_names,
    reason = "the member DisasterVehicle::state"
)]
pub struct State {
    /// Override for the default disaster vehicle sprite.
    pub image_override: SpriteID,
    /// The big UFO that this destroyer is supposed to bomb.
    pub big_ufo_destroyer_target: u32,
    /// Action stage of the disaster vehicle.
    pub state: u16,
    /// Flags about the state of the vehicle, @see `AirVehicleFlags`
    pub flags: u8,
}

/// `DisasterVehicle *`: the shared record of one disaster vehicle and the owner its shell
/// holds.
#[derive(Clone, Copy)]
pub(crate) struct DisasterVehicle {
    pub(crate) v: VehicleRef,
    s: NonNull<State>,
}

#[allow(unsafe_code)]
impl DisasterVehicle {
    /// The disaster vehicle whose record is `v`, a live disaster vehicle's, as a
    /// `DisasterVehicle` member's `this` is.
    fn of(v: VehicleRef) -> Self {
        // SAFETY: Every disaster shell's constructor links its non-null owner.
        Self {
            v,
            s: unsafe { NonNull::new_unchecked(v.disaster()) },
        }
    }
    /// `Next()`: the next part of the disaster vehicle, a plain cast as
    /// `SpecializedVehicle::Next` has it.
    fn next(self) -> Option<Self> {
        self.v.next().map(Self::of)
    }
    /// `image_override`.
    fn image_override(self) -> SpriteID {
        crate::raw_field::read_field!(self.s.as_ptr(), image_override)
    }
    /// Store `image_override`.
    fn set_image_override(self, image_override: SpriteID) {
        crate::raw_field::write_field!(self.s.as_ptr(), image_override, image_override);
    }
    /// `big_ufo_destroyer_target`.
    fn big_ufo_destroyer_target(self) -> VehicleID {
        VehicleID(crate::raw_field::read_field!(
            self.s.as_ptr(),
            big_ufo_destroyer_target
        ))
    }
    /// Store `big_ufo_destroyer_target`.
    fn set_big_ufo_destroyer_target(self, target: VehicleID) {
        crate::raw_field::write_field!(self.s.as_ptr(), big_ufo_destroyer_target, target.0);
    }
    /// `state`.
    fn state(self) -> u16 {
        crate::raw_field::read_field!(self.s.as_ptr(), state)
    }
    /// Store `state`.
    fn set_state(self, state: u16) {
        crate::raw_field::write_field!(self.s.as_ptr(), state, state);
    }
    /// `&flags`, which `GetAircraftFlightLevel` reads and writes.
    fn flags(self) -> *mut u8 {
        // SAFETY: Computing a raw field address creates no reference.
        unsafe { &raw mut (*self.s.as_ptr()).flags }
    }
}

/// The services a disaster entry calls that C++ still owns, and the tables it reaches.
#[repr(C)]
pub struct Services {
    /// The aircraft controller's table, immutable for the process.
    pub aircraft: *const aircraft::Leaves,
    /// `new DisasterVehicle()`: allocate a disaster vehicle shell with its member defaults.
    pub allocate: extern "C" fn() -> VehicleRef,
    /// The environment table, immutable for the process.
    pub environment: *const crate::services::EnvironmentServices,
    /// The company identity table, immutable for the process.
    pub identity: *const crate::company_identity::Services,
    /// The industry production table, immutable for the process.
    pub industry: *const crate::industry::Services,
    /// The road vehicle controller's table, immutable for the process.
    pub road: *const crate::road::Leaves,
    /// `c->group_all[VEH_ROAD].num_vehicle` of a live company.
    pub road_count: extern "C" fn(u8) -> u16,
    /// The vehicle scheduler's table, whose descriptor names the vehicle pool.
    pub scheduler: *const SchedulerServices,
    /// The shared table, immutable for the process.
    pub shared: *const Shared,
    /// The base vehicle table, immutable for the process.
    pub vehicles: *const VehicleServices,
}

/// The tables one disaster entry runs in. The shared and vehicle tables are taken at the
/// entry; the others are reached when a body first calls into them.
pub(crate) struct Tables<'a> {
    s: &'a Services,
    shared: &'a Shared,
    vehicles: &'a VehicleServices,
}

#[allow(unsafe_code)]
impl<'a> Tables<'a> {
    /// The tables `s` names.
    pub(crate) fn new(s: &'a Services) -> Self {
        // SAFETY: Each table link is a process-lifetime table, or a test's that outlives the
        // entry; every entry has the shared and vehicle tables.
        unsafe {
            Self {
                s,
                shared: &*s.shared,
                vehicles: &*s.vehicles,
            }
        }
    }
    /// The aircraft controller's table.
    fn aircraft(&self) -> &'a aircraft::Leaves {
        // SAFETY: As `new`, for the bodies that reach this table.
        unsafe { &*self.s.aircraft }
    }
    /// The environment table.
    fn environment(&self) -> &'a crate::services::EnvironmentServices {
        // SAFETY: As `aircraft`.
        unsafe { &*self.s.environment }
    }
    /// The company identity table.
    fn identity(&self) -> &'a crate::company_identity::Services {
        // SAFETY: As `aircraft`.
        unsafe { &*self.s.identity }
    }
    /// The industry production table.
    fn industry(&self) -> &'a crate::industry::Services {
        // SAFETY: As `aircraft`.
        unsafe { &*self.s.industry }
    }
    /// The road vehicle controller's table.
    fn road(&self) -> &'a crate::road::Leaves {
        // SAFETY: As `aircraft`.
        unsafe { &*self.s.road }
    }
    /// The vehicle scheduler's table.
    fn scheduler(&self) -> &'a SchedulerServices {
        // SAFETY: As `aircraft`.
        unsafe { &*self.s.scheduler }
    }
    /// The map, read in place. Disaster callbacks cannot load or reset the map or yield the
    /// game-state mutex, so the descriptor holds for the entry.
    fn world(&self) -> World<'a> {
        World::new(self.shared)
    }
    /// The native assertion.
    fn check(&self) -> impl Fn(bool) + Copy + '_ {
        |ok| crate::movement::native_assert(self.shared, ok)
    }
    /// The vehicle resolver table.
    fn newgrf(&self) -> &'static crate::newgrf_vehicle::Services {
        self.vehicles.newgrf()
    }
    /// The vehicle maintenance table.
    fn maintenance(&self) -> &crate::vehicle::maintenance::Services {
        // SAFETY: The base table names the process-lifetime maintenance table.
        unsafe { &*self.vehicles.maintenance }
    }
    /// `RandomTile()`: `RandomTileSeed(Random())`.
    fn random_tile(&self, world: &World<'_>) -> TileIndex {
        world
            .dimensions
            .random_tile_seed(world.size_y(), self.shared.random())
    }
    /// `GetSlopePixelZ(x, y)`.
    fn get_slope_pixel_z(&self, x: i32, y: i32) -> i32 {
        crate::movement::get_slope_pixel_z(self.shared, x, y, false)
    }
    /// `GetAircraftFlightLevel(v)`.
    fn get_aircraft_flight_level(&self, v: DisasterVehicle) -> i32 {
        aircraft::get_aircraft_flight_level(self.aircraft(), v.v, v.flags(), false)
    }
    /// `GetAircraftFlightLevelBounds(v, &v->z_pos, nullptr)`: the bounds function reads no
    /// `z_pos`, so the minimum is stored after it returns.
    fn set_z_pos_to_flight_level_minimum(&self, v: VehicleRef) {
        let mut z_pos = 0;
        aircraft::get_aircraft_flight_level_bounds(self.aircraft(), v, Some(&mut z_pos), None);
        v.set_z_pos(z_pos);
    }
    /// `v->UpdatePositionAndViewport()`.
    fn update_position_and_viewport(&self, v: VehicleRef, world: &World<'_>) {
        self.vehicles
            .update_position_and_viewport(v, || world.dimensions);
    }
    /// `++v->age`.
    fn increment_age(&self, v: VehicleRef) -> i32 {
        let age = v.age(self.newgrf()).0.wrapping_add(1);
        v.set_age(
            self.maintenance(),
            crate::timer_game_calendar::CalendarDate(age),
        );
        age
    }
    /// `v->age = CalendarTime::MIN_DATE`.
    fn reset_age(&self, v: VehicleRef) {
        v.set_age(self.maintenance(), MIN_DATE);
    }
    /// `if (_settings_client.sound.disaster) SndPlayVehicleFx(SND_12_EXPLOSION, v)`.
    fn play_explosion(&self, v: VehicleRef) {
        if (self.shared.disaster_sound)() {
            (self.vehicles.play_sound)(v, SoundID::from(SoundFx::SND_12_EXPLOSION).0);
        }
    }
    /// `delete v`.
    fn delete(&self, v: VehicleRef) {
        (self.vehicles.destroy)(v);
    }
    /// `T::Iterate()` for the vehicle type `kind`, or `Vehicle::Iterate()` for `None`: each
    /// live vehicle in pool order. Each step reads the next index before the loop body
    /// runs, as the pool iterator does.
    fn iterate(&self, kind: Option<VehicleType>) -> impl Iterator<Item = VehicleRef> + use<'_, 'a> {
        vehicles_of(self.shared, self.newgrf(), kind)
    }
    /// `Vehicle::Get(index)`: `Pool::Get`'s `assert(index < this->first_unused)`, then the
    /// slot's record, which a free slot leaves null.
    fn vehicle_get(&self, index: u32) -> Option<VehicleRef> {
        let pool = self.scheduler().pool();
        // SAFETY: The scheduler's live descriptor; nothing mutates the pool during the read.
        crate::services_assert::assert_with(|| (index as usize) < unsafe { pool.pool_size() });
        // SAFETY: As above.
        unsafe { crate::vehicle_base::get(&pool, VehicleID(index)) }
    }
    /// `Industry::Get(index)`: the industry's shell, through the pool's asserting lookup.
    fn industry_get(&self, index: u32) -> *mut crate::industry::tile::ShellHandle {
        // SAFETY: The industry table names the process-lifetime transaction table.
        unsafe { ((*self.industry().transaction).industry)(index) }
    }
    /// The Rust owner of a live industry shell.
    fn industry_owner(
        &self,
        shell: *mut crate::industry::tile::ShellHandle,
    ) -> *mut crate::industry::Industry {
        (crate::station::Ctx::from_shared(self.shared)
            .c
            .industry_owner)(shell)
    }
}

/// `T::Iterate()` for the vehicle type `kind`, or `Vehicle::Iterate()` for `None`, over the
/// shared vehicle walk and the resolver's vehicle index.
fn vehicles_of<'a>(
    shared: &'a Shared,
    resolver: &'a crate::newgrf_vehicle::Services,
    kind: Option<VehicleType>,
) -> impl Iterator<Item = VehicleRef> + 'a {
    let mut from = 0_u32;
    std::iter::from_fn(move || {
        loop {
            let v = (shared.vehicle_next)(from)?;
            from = v.index(resolver).0.wrapping_add(1);
            if kind.is_none_or(|kind| v.vehicle_type() == kind) {
                return Some(v);
            }
        }
    })
}

/// `Delta(a, b) + Delta(c, d)`, the original's Manhattan distance of two positions.
fn delta_sum(ax: i32, bx: i32, ay: i32, by: i32) -> i32 {
    delta(ax, bx).wrapping_add(delta(ay, by))
}

/// `DisasterClearSquare`.
fn disaster_clear_square(t: &Tables<'_>, tile: TileIndex) {
    let world = t.world();
    if crate::vehicle::occupancy::ensure_no_vehicle_on_ground(&world, tile).is_some() {
        return;
    }

    match world.kind(tile) {
        TileType::MP_RAILWAY => {
            if is_human_id(t.identity(), world.owner(tile)) && !world.is_rail_depot(tile) {
                landscape_clear_as(t, tile, Owner::WATER);

                /* update signals in buffer */
                crate::signal::context(t.shared).update_signals_in_buffer();
            }
        }

        TileType::MP_HOUSE => {
            landscape_clear_as(t, tile, Owner::NONE);
        }

        TileType::MP_TREES | TileType::MP_CLEAR => {
            crate::ground::do_clear_square(tile, t.shared);
        }

        _ => {}
    }
}

/// `Backup<CompanyID> cur_company(_current_company, company)`,
/// `Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile)` and
/// `cur_company.Restore()`.
#[allow(unsafe_code)]
fn landscape_clear_as(t: &Tables<'_>, tile: TileIndex, company: CompanyID) {
    let commands = ((t.shared.command_post)().command)();
    let cur_company = (t.shared.current_company)();
    (commands.finance.set_current_company)(company.0);
    // SAFETY: The cost lives for the nested command, which keeps no pointer to it.
    unsafe {
        crate::command::with_cost(commands.cost, |cost| {
            crate::landscape_cmd::clear_command(
                commands,
                cost.0,
                tile,
                DoCommandFlags::from_element(DoCommandFlag::Execute),
            );
        });
    }
    (commands.finance.set_current_company)(cur_company);
}

#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_1: [SpriteID; 8] = [SPR_BLIMP; 8];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_2: [SpriteID; 8] = [SPR_UFO_SMALL_SCOUT; 8];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_3: [SpriteID; 8] = [SPR_F_15; 8];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_4: [SpriteID; 8] = [
    SPR_SUB_SMALL_NE,
    SPR_SUB_SMALL_NE,
    SPR_SUB_SMALL_SE,
    SPR_SUB_SMALL_SE,
    SPR_SUB_SMALL_SW,
    SPR_SUB_SMALL_SW,
    SPR_SUB_SMALL_NW,
    SPR_SUB_SMALL_NW,
];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_5: [SpriteID; 8] = [
    SPR_SUB_LARGE_NE,
    SPR_SUB_LARGE_NE,
    SPR_SUB_LARGE_SE,
    SPR_SUB_LARGE_SE,
    SPR_SUB_LARGE_SW,
    SPR_SUB_LARGE_SW,
    SPR_SUB_LARGE_NW,
    SPR_SUB_LARGE_NW,
];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_6: [SpriteID; 8] = [SPR_UFO_HARVESTER; 8];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_7: [SpriteID; 8] = [SPR_XCOM_SKYRANGER; 8];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_8: [SpriteID; 8] = [SPR_AH_64A; 8];
#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images_9: [SpriteID; 8] = [SPR_ROTOR_MOVING_1; 8];

#[allow(non_upper_case_globals, reason = "the original's name")]
static _disaster_images: [&[SpriteID; 8]; 15] = [
    &_disaster_images_1,
    &_disaster_images_1, // zeppeliner and zeppeliner shadow
    &_disaster_images_2,
    &_disaster_images_2, // small ufo and small ufo shadow
    &_disaster_images_3,
    &_disaster_images_3, // combat aircraft and shadow
    &_disaster_images_8,
    &_disaster_images_8,
    &_disaster_images_9, // combat helicopter, shadow and rotor
    &_disaster_images_6,
    &_disaster_images_6, // big ufo and shadow
    &_disaster_images_7,
    &_disaster_images_7, // skyranger and shadow
    &_disaster_images_4,
    &_disaster_images_5, // small and big submarine sprites
];

/// `DisasterVehicle::UpdateImage`.
fn update_image(t: &Tables<'_>, v: DisasterVehicle) {
    let mut img = v.image_override();
    if img == 0 {
        img = _disaster_images[usize::from(v.v.subtype())][usize::from(v.v.direction().0)];
    }
    (t.vehicles.write_sprite_seq)(v.v, img);
}

/// Construct the disaster vehicle (`DisasterVehicle::DisasterVehicle`): `x` and `y` are the
/// coordinates, `direction` the direction the vehicle is facing, `subtype` the sub type of
/// vehicle and `big_ufo_destroyer_target` the target for the UFO destroyer.
fn disaster_vehicle_new(
    t: &Tables<'_>,
    x: i32,
    y: i32,
    direction: Direction,
    subtype: DisasterSubType,
    big_ufo_destroyer_target: VehicleID,
) -> DisasterVehicle {
    let this = DisasterVehicle::of((t.s.allocate)());
    this.set_big_ufo_destroyer_target(big_ufo_destroyer_target);
    let v = this.v;
    let world = t.world();

    v.set_vehstatus(VehStates::from_element(VehState::Unclickable));

    v.set_x_pos(x);
    v.set_y_pos(y);
    match subtype {
        DisasterSubType::ST_ZEPPELINER
        | DisasterSubType::ST_SMALL_UFO
        | DisasterSubType::ST_AIRPLANE
        | DisasterSubType::ST_HELICOPTER
        | DisasterSubType::ST_BIG_UFO
        | DisasterSubType::ST_BIG_UFO_DESTROYER => {
            t.set_z_pos_to_flight_level_minimum(v);
        }

        DisasterSubType::ST_HELICOPTER_ROTORS => {
            t.set_z_pos_to_flight_level_minimum(v);
            v.set_z_pos(v.z_pos().wrapping_add(ROTOR_Z_OFFSET));
        }

        DisasterSubType::ST_SMALL_SUBMARINE | DisasterSubType::ST_BIG_SUBMARINE => {
            v.set_z_pos(0);
        }

        DisasterSubType::ST_ZEPPELINER_SHADOW
        | DisasterSubType::ST_SMALL_UFO_SHADOW
        | DisasterSubType::ST_AIRPLANE_SHADOW
        | DisasterSubType::ST_HELICOPTER_SHADOW
        | DisasterSubType::ST_BIG_UFO_SHADOW
        | DisasterSubType::ST_BIG_UFO_DESTROYER_SHADOW => {
            v.set_z_pos(0);
            v.set_vehstatus(v.vehstatus().set(VehState::Shadow));
        }

        _ => {}
    }

    v.set_direction(direction);
    v.set_tile(world.dimensions.tile_virt_xy(x as u32, y as u32));
    v.set_subtype(subtype.0);
    (t.vehicles.update_delta)(v);
    v.set_owner(Owner::NONE);
    this.set_image_override(0);
    this.set_state(0);

    update_image(t, this);
    t.update_position_and_viewport(v, &world);
    this
}

/// `DisasterVehicle::UpdatePosition`: update the position of the vehicle to the new `x`,
/// `y` and `z` coordinates.
#[allow(
    clippy::many_single_char_names,
    reason = "DisasterVehicle::UpdatePosition's x, y, z and u"
)]
fn update_position(t: &Tables<'_>, this: DisasterVehicle, x: i32, y: i32, z: i32) {
    let world = t.world();
    let check = t.check();
    let v = this.v;
    v.set_x_pos(x);
    v.set_y_pos(y);
    v.set_z_pos(z);
    v.set_tile(world.dimensions.tile_virt_xy(x as u32, y as u32));

    update_image(t, this);
    t.update_position_and_viewport(v, &world);

    if let Some(u) = this.next() {
        let max_x = (world.dimensions.max_x() * TILE_SIZE) as i32;
        let max_y = (Map::max_y(world.size_y()) * TILE_SIZE) as i32;
        let safe_x = clamp(x, 0, max_x, check);
        let mut safe_y = clamp(y.wrapping_sub(1), 0, max_y, check);

        u.v.set_x_pos(x);
        u.v.set_y_pos(
            y.wrapping_sub(1)
                .wrapping_sub(z.wrapping_sub(t.get_slope_pixel_z(safe_x, safe_y)).max(0) >> 3),
        );
        safe_y = clamp(u.v.y_pos(), 0, max_y, check);
        u.v.set_z_pos(t.get_slope_pixel_z(safe_x, safe_y));
        u.v.set_direction(v.direction());

        update_image(t, u);
        t.update_position_and_viewport(u.v, &world);

        if let Some(u) = u.next() {
            u.v.set_x_pos(x);
            u.v.set_y_pos(y);
            u.v.set_z_pos(z.wrapping_add(ROTOR_Z_OFFSET));
            t.update_position_and_viewport(u.v, &world);
        }
    }
}

/// `GetNewVehiclePos(v)` and `v->UpdatePosition(gp.x, gp.y, GetAircraftFlightLevel(v))`.
fn fly(t: &Tables<'_>, v: DisasterVehicle) -> crate::vehicle_func::GetNewVehiclePosResult {
    let gp = get_new_vehicle_pos(v.v, t.world().dimensions);
    let z = t.get_aircraft_flight_level(v);
    update_position(t, v, gp.x, gp.y, z);
    gp
}

/// `v->tick_counter++`, returning the new count.
fn increment_tick_counter(v: VehicleRef) -> u8 {
    let tick_counter = v.tick_counter().wrapping_add(1);
    v.set_tick_counter(tick_counter);
    tick_counter
}

/// `DisasterTick_Zeppeliner`: zeppeliner handling, `v->state` states:
/// 0. Zeppeliner initialization has found an airport, go there and crash
/// 1. Create crash and animate falling down for extra dramatic effect
/// 2. Create more smoke and leave debris on ground
/// 3. Clear the runway after some time and remove crashed zeppeliner
///
/// If not airport was found, only state 0 is reached until zeppeliner leaves map
#[allow(
    clippy::many_single_char_names,
    reason = "DisasterTick_Zeppeliner's v, x, y, z and r"
)]
fn disaster_tick_zeppeliner(t: &Tables<'_>, v: DisasterVehicle) -> bool {
    increment_tick_counter(v.v);
    let world = t.world();

    if v.state() < 2 {
        if has_bit(v.v.tick_counter(), 0) {
            return true;
        }

        fly(t, v);

        if v.state() == 1 {
            if t.increment_age(v.v) == 38 {
                v.set_state(2);
                t.reset_age(v.v);
            }

            if gb(u32::from(v.v.tick_counter()), 0, 3) == 0 {
                let _ =
                    create_effect_vehicle_rel(v.v, 0, -17, 2, EffectVehicleType::EV_CRASH_SMOKE);
            }
        } else if v.state() == 0
            && world.is_valid_tile(v.v.tile())
            && world.is_airport_tile(v.v.tile())
        {
            v.set_state(1);
            t.reset_age(v.v);

            add_tile_news_item(
                &get_encoded_string(
                    STR_NEWS_DISASTER_ZEPPELIN,
                    &[Integer(u64::from(world.get_station_index(v.v.tile()).0))],
                ),
                NewsType::Accident,
                v.v.tile(),
                StationID::INVALID,
            );
            crate::ai::new_event(
                world.owner(v.v.tile()),
                &ScriptEvent::disaster_zeppeliner_crashed(world.get_station_index(v.v.tile())),
            );
        }

        if v.v.y_pos() >= ((world.size_y() + 9) * TILE_SIZE - 1) as i32 {
            t.delete(v.v);
            return false;
        }

        return true;
    }

    if world.is_valid_tile(v.v.tile()) && world.is_airport_tile(v.v.tile()) {
        airport_blocks_update(t, v.v.tile(), |blocks| {
            blocks.set(zeppeliner_runway());
        });
    }

    if v.state() > 2 {
        if t.increment_age(v.v) <= 13320 {
            return true;
        }

        if world.is_valid_tile(v.v.tile()) && world.is_airport_tile(v.v.tile()) {
            let st = station_get_by_tile(t, v.v.tile());
            airport_blocks_update_station(t, st, |blocks| {
                blocks.reset(zeppeliner_runway());
            });
            crate::ai::new_event(
                world.owner(v.v.tile()),
                &ScriptEvent::disaster_zeppeliner_cleared(station_index(t, st)),
            );
        }

        let z = t.get_aircraft_flight_level(v);
        update_position(t, v, v.v.x_pos(), v.v.y_pos(), z);
        t.delete(v.v);
        return false;
    }

    let x = v.v.x_pos();
    let y = v.v.y_pos();
    let mut z = t.get_slope_pixel_z(x, y);
    if z < v.v.z_pos() {
        z = v.v.z_pos().wrapping_sub(1);
    }
    update_position(t, v, x, y, z);

    let age = t.increment_age(v.v);
    if age == 1 {
        let _ = create_effect_vehicle_rel(v.v, 0, 7, 8, EffectVehicleType::EV_EXPLOSION_LARGE);
        t.play_explosion(v.v);
        v.set_image_override(SPR_BLIMP_CRASHING);
    } else if age == 70 {
        v.set_image_override(SPR_BLIMP_CRASHED);
    } else if age <= 300 {
        if gb(u32::from(v.v.tick_counter()), 0, 3) == 0 {
            let r = t.shared.random();

            let _ = create_effect_vehicle_rel(
                v.v,
                gb(r, 0, 4).wrapping_sub(7) as i32,
                gb(r, 4, 4).wrapping_sub(7) as i32,
                gb(r, 8, 3).wrapping_add(5) as i32,
                EffectVehicleType::EV_EXPLOSION_SMALL,
            );
        }
    } else if age == 350 {
        v.set_state(3);
        t.reset_age(v.v);
    }

    true
}

/// `{AirportBlock::Zeppeliner, AirportBlock::RunwayIn}`.
fn zeppeliner_runway() -> AirportBlocks {
    *AirportBlocks::from_element(AirportBlock::ZEPPELINER)
        .set(AirportBlocks::from_element(AirportBlock::RUNWAY_IN))
}

/// `Station::GetByTile(tile)`.
fn station_get_by_tile(t: &Tables<'_>, tile: TileIndex) -> *mut crate::station::Station {
    crate::station_map::station_get_by_tile(tile, airport_services(t))
}

/// The airport resolver table, which reads `st->index`.
#[allow(unsafe_code)]
fn airport_services<'a>(t: &Tables<'a>) -> &'a crate::newgrf_airport::Services {
    // SAFETY: The catchment table names the process-lifetime airport table.
    unsafe { &*crate::station::Ctx::from_shared(t.shared).c.airport }
}

/// `st->index`.
fn station_index(t: &Tables<'_>, st: *mut crate::station::Station) -> StationID {
    airport_services(t).station_index(st.cast_const().cast())
}

/// `Station::GetByTile(tile)->airport.blocks`, updated in place.
fn airport_blocks_update(t: &Tables<'_>, tile: TileIndex, update: impl FnOnce(&mut AirportBlocks)) {
    airport_blocks_update_station(t, station_get_by_tile(t, tile), update);
}

/// `st->airport.blocks`, updated in place.
#[allow(unsafe_code)]
fn airport_blocks_update_station(
    t: &Tables<'_>,
    st: *mut crate::station::Station,
    update: impl FnOnce(&mut AirportBlocks),
) {
    let blocks = t.aircraft().airport_blocks(st);
    // SAFETY: One field-sized load and store of the live mask, with no callback between.
    unsafe {
        let mut value = blocks.read();
        update(&mut value);
        blocks.write(value);
    }
}

/// `DisasterTick_Ufo`: (Small) Ufo handling, `v->state` states:
/// 0. Fly around to the middle of the map, then randomly, after a while target a road
///    vehicle
/// 1. Home in on a road vehicle and crash it >:)
///
/// If not road vehicle was found, only state 0 is used and Ufo disappears after a while.
/// Returns whether the tick was successful (i.e. false if the Ufo has been destroyed).
#[allow(
    clippy::many_single_char_names,
    reason = "DisasterTick_Ufo's x, y, n, u and z"
)]
#[allow(
    clippy::too_many_lines,
    reason = "DisasterTick_Ufo is one body in the original"
)]
#[allow(unsafe_code)]
fn disaster_tick_ufo(t: &Tables<'_>, ufo: DisasterVehicle) -> bool {
    ufo.set_image_override(if has_bit(increment_tick_counter(ufo.v), 3) {
        SPR_UFO_SMALL_SCOUT_DARKER
    } else {
        SPR_UFO_SMALL_SCOUT
    });
    let world = t.world();
    let map = world.dimensions;

    if ufo.state() == 0 {
        /* Fly around randomly */
        let x = (map.tile_x(ufo.v.dest_tile()) * TILE_SIZE) as i32;
        let y = (map.tile_y(ufo.v.dest_tile()) * TILE_SIZE) as i32;
        if delta_sum(x, ufo.v.x_pos(), y, ufo.v.y_pos()) >= TILE_SIZE as i32 {
            ufo.v
                .set_direction(get_direction_towards(ufo.v, x, y, t.check()));
            fly(t, ufo);
            return true;
        }
        if t.increment_age(ufo.v) < 6 {
            ufo.v.set_dest_tile(t.random_tile(&world));
            return true;
        }
        ufo.set_state(1);

        let mut n: u32 = 0; // Total number of targetable road vehicles.
        let mut from = 0;
        loop {
            let c = (t.identity().next_company)(from);
            if c.is_null() {
                break;
            }
            let index = (t.identity().index)(c);
            from = u32::from(index.0) + 1;
            n = n.wrapping_add(u32::from((t.s.road_count)(index.0)));
        }

        if n == 0 {
            /* If there are no targetable road vehicles, destroy the UFO. */
            t.delete(ufo.v);
            return false;
        }

        n = t.shared.random_range(n); // Choose one of them.
        for u in t.iterate(Some(VehicleType::VEH_ROAD)) {
            /* Find (n+1)-th road vehicle. */
            if u.is_front_engine() && {
                let old = n;
                n = n.wrapping_sub(1);
                old == 0
            } {
                // SAFETY: A live road vehicle's owner; one scalar access each.
                if unsafe { crate::road::crashed_ctr(u.road()) } != 0
                    || unsafe { crate::road::disaster_vehicle(u.road()) } != VehicleID::INVALID
                {
                    /* Targetted vehicle is crashed or already a target, destroy the UFO. */
                    t.delete(ufo.v);
                    return false;
                }
                /* Target it. */
                ufo.v.set_dest_tile(TileIndex(u.index(t.newgrf()).0));
                t.reset_age(ufo.v);
                // SAFETY: As above.
                unsafe { crate::road::set_disaster_vehicle(u.road(), ufo.v.index(t.newgrf())) };
                break;
            }
        }

        return true;
    }

    /* Target a vehicle */
    let target = t.vehicle_get(ufo.v.dest_tile().0);
    crate::services_assert::assert_with(|| {
        target.is_some_and(|target| {
            target.vehicle_type() == VehicleType::VEH_ROAD && target.is_front_engine()
        })
    });
    let target = target.expect("RoadVehicle::Get of the targeted road vehicle");

    let dist = delta_sum(ufo.v.x_pos(), target.x_pos(), ufo.v.y_pos(), target.y_pos()) as u32;

    if dist < TILE_SIZE && !target.vehstatus().test(VehState::Hidden) && target.breakdown_ctr() == 0
    {
        target.set_breakdown_ctr(3);
        target.set_breakdown_delay(t.maintenance(), 140);
    }

    ufo.v.set_direction(get_direction_towards(
        ufo.v,
        target.x_pos(),
        target.y_pos(),
        t.check(),
    ));
    let gp = get_new_vehicle_pos(ufo.v, map);

    let mut z = ufo.v.z_pos();
    if dist <= TILE_SIZE && z > target.z_pos() {
        z = z.wrapping_sub(1);
    }
    update_position(t, ufo, gp.x, gp.y, z);

    /* If the vehicle is hidden in a depot or similar treat it as having "escaped" being crashed to avoid the Ufo looping forever,
     * but we'll still explode the surrounding area ;) */
    if z <= target.z_pos() {
        t.increment_age(ufo.v);
        // SAFETY: The target is a live road vehicle; one scalar access.
        if !target.vehstatus().test(VehState::Hidden)
            && unsafe { crate::road::crashed_ctr(target.road()) } == 0
        {
            let victims = crate::road::crash(t.road(), target, false);
            // SAFETY: As above.
            unsafe { crate::road::set_disaster_vehicle(target.road(), VehicleID::INVALID) };

            add_tile_news_item(
                &get_encoded_string(STR_NEWS_DISASTER_SMALL_UFO, &[]),
                NewsType::Accident,
                target.tile(),
                StationID::INVALID,
            );

            crate::ai::new_event(
                target.owner(),
                &ScriptEvent::vehicle_crashed(
                    target.index(t.newgrf()),
                    target.tile(),
                    CrashReason::CRASH_RV_UFO,
                    victims,
                    target.owner(),
                ),
            );
            crate::game::new_event(&ScriptEvent::vehicle_crashed(
                target.index(t.newgrf()),
                target.tile(),
                CrashReason::CRASH_RV_UFO,
                victims,
                target.owner(),
            ));
        }
    }

    /* Destroy? */
    if ufo.v.age(t.newgrf()).0 > 50 {
        let _ = create_effect_vehicle_rel(ufo.v, 0, 7, 8, EffectVehicleType::EV_EXPLOSION_LARGE);
        t.play_explosion(ufo.v);
        t.delete(ufo.v);
        return false;
    }

    true
}

/// `DestructIndustry`.
#[allow(unsafe_code)]
fn destruct_industry(t: &Tables<'_>, i: *mut crate::industry::tile::ShellHandle) {
    let world = t.world();
    let index = crate::industry::industry_index(i.cast(), t.industry());
    // SAFETY: The industry table names the process-lifetime resolver table.
    let tiles = unsafe { (*t.industry().resolver).tile() };
    for tile in Map::iterate(world.types.count()) {
        if crate::industry::tile_belongs_to_industry(&world, tile, index) {
            tiles.reset_industry_construction_stage(tile);
            (t.shared.mark_dirty)(tile.0);
        }
    }
}

/// `DisasterTick_Aircraft`: aircraft handling, `v->state` states:
/// 0. Fly towards the targeted industry
/// 1. If within 15 tiles, fire away rockets and destroy industry
/// 2. Industry explosions
/// 3. Fly out of the map
///
/// If the industry was removed in the meantime just fly to the end of the map.
/// `image_override` is the image at the time the aircraft is firing, `leave_at_top` true iff
/// the vehicle leaves the map at the north side, `news_message` the string that's used as
/// news message and `behaviour` the behaviour an industry needs to be attacked.
#[allow(
    clippy::many_single_char_names,
    reason = "DisasterTick_Aircraft's v, i, x, y and r"
)]
fn disaster_tick_aircraft(
    t: &Tables<'_>,
    v: DisasterVehicle,
    image_override: SpriteID,
    leave_at_top: bool,
    news_message: StringID,
    behaviour: IndustryBehaviour,
) -> bool {
    increment_tick_counter(v.v);
    v.set_image_override(if v.state() == 1 && has_bit(v.v.tick_counter(), 2) {
        image_override
    } else {
        0
    });

    let gp = fly(t, v);
    let world = t.world();
    let map = world.dimensions;

    if (leave_at_top && gp.x < -10 * TILE_SIZE as i32)
        || (!leave_at_top && gp.x > (map.size_x() * TILE_SIZE + 9 * TILE_SIZE) as i32 - 1)
    {
        t.delete(v.v);
        return false;
    }

    if v.state() == 2 {
        if gb(u32::from(v.v.tick_counter()), 0, 2) == 0 {
            let i = t.industry_get(v.v.dest_tile().0); // Industry destructor calls ReleaseDisastersTargetingIndustry, so this is valid
            let location = crate::industry::location(t.industry_owner(i)).tile;
            let x = map.tile_x(location) * TILE_SIZE;
            let y = map.tile_y(location) * TILE_SIZE;
            let r = t.shared.random();

            let _ = create_effect_vehicle_above(
                gb(r, 0, 6).wrapping_add(x) as i32,
                gb(r, 6, 6).wrapping_add(y) as i32,
                gb(r, 12, 4) as i32,
                EffectVehicleType::EV_EXPLOSION_SMALL,
            );

            if t.increment_age(v.v) >= 55 {
                v.set_state(3);
            }
        }
    } else if v.state() == 1 {
        if t.increment_age(v.v) == 112 {
            v.set_state(2);
            t.reset_age(v.v);

            let i = t.industry_get(v.v.dest_tile().0); // Industry destructor calls ReleaseDisastersTargetingIndustry, so this is valid
            destruct_industry(t, i);

            add_industry_news_item(
                &get_encoded_string(
                    news_message,
                    &[Integer(u64::from(
                        crate::industry::town_index(i.cast(), t.industry()).0,
                    ))],
                ),
                NewsType::Accident,
                crate::industry::industry_index(i.cast(), t.industry()),
            );
            if (t.shared.disaster_sound)() {
                (t.shared.play_sound)(
                    crate::industry::location(t.industry_owner(i)).tile.0,
                    SoundID::from(SoundFx::SND_12_EXPLOSION).0,
                );
            }
        }
    } else if v.state() == 0 {
        let x =
            v.v.x_pos()
                .wrapping_add((if leave_at_top { -15 } else { 15 }) * TILE_SIZE as i32);
        let y = v.v.y_pos();

        if x as u32 > map.max_x() * TILE_SIZE - 1 {
            return true;
        }

        let tile = map.tile_virt_xy(x as u32, y as u32);
        if world.kind(tile) != TileType::MP_INDUSTRY {
            return true;
        }

        let ind: IndustryID = world.asserted(|m| crate::industry_map::get_industry_index(m, tile));
        v.v.set_dest_tile(TileIndex(u32::from(ind.0)));

        if industry_behaviour(t.industry_owner(t.industry_get(u32::from(ind.0)))).test(behaviour) {
            v.set_state(1);
            t.reset_age(v.v);
        }
    }

    true
}

/// `GetIndustrySpec(i->type)->behaviour` of a live industry owner.
#[allow(unsafe_code)]
fn industry_behaviour(owner: *mut crate::industry::Industry) -> IndustryBehaviours {
    // SAFETY: `GetIndustrySpec` returns a live spec record for the industry's type.
    IndustryBehaviours(unsafe {
        (*get_industry_spec(crate::industry::industry_type(owner))).behaviour
    })
}

/// `DisasterTick_Airplane`: airplane handling.
fn disaster_tick_airplane(t: &Tables<'_>, v: DisasterVehicle) -> bool {
    disaster_tick_aircraft(
        t,
        v,
        SPR_F_15_FIRING,
        true,
        STR_NEWS_DISASTER_AIRPLANE_OIL_REFINERY,
        IndustryBehaviour::AirplaneAttacks,
    )
}

/// `DisasterTick_Helicopter`: helicopter handling.
fn disaster_tick_helicopter(t: &Tables<'_>, v: DisasterVehicle) -> bool {
    disaster_tick_aircraft(
        t,
        v,
        SPR_AH_64A_FIRING,
        false,
        STR_NEWS_DISASTER_HELICOPTER_FACTORY,
        IndustryBehaviour::ChopperAttacks,
    )
}

/// `DisasterTick_Helicopter_Rotors`: helicopter rotor blades; keep these spinning
fn disaster_tick_helicopter_rotors(t: &Tables<'_>, v: DisasterVehicle) -> bool {
    if has_bit(increment_tick_counter(v.v), 0) {
        return true;
    }

    // `++cur_image` and the wrap store into `seq[0].sprite` with no call between, as one
    // store.
    let mut cur_image = (t.vehicles.first_sprite)(v.v).wrapping_add(1);
    if cur_image > SPR_ROTOR_MOVING_3 {
        cur_image = SPR_ROTOR_MOVING_1;
    }
    (t.vehicles.write_first_sprite)(v.v, cur_image);

    t.update_position_and_viewport(v.v, &t.world());

    true
}

/// `DisasterTick_Big_Ufo`: (Big) Ufo handling, `v->state` states:
/// 0. Fly around to the middle of the map, then randomly for a while and home in on a piece
///    of rail
/// 1. Land there and breakdown all trains in a radius of 12 tiles; and now we wait...
///    because as soon as the Ufo lands, a fighter jet, a Skyranger, is called to clear up
///    the mess
#[allow(
    clippy::many_single_char_names,
    reason = "DisasterTick_Big_Ufo's v, x, y, z, t, u, w and n"
)]
#[allow(
    clippy::too_many_lines,
    reason = "DisasterTick_Big_Ufo is one body in the original"
)]
fn disaster_tick_big_ufo(t: &Tables<'_>, v: DisasterVehicle) -> bool {
    increment_tick_counter(v.v);
    let world = t.world();
    let map = world.dimensions;

    if v.state() == 1 {
        let x = (map.tile_x(v.v.dest_tile()) * TILE_SIZE + TILE_SIZE / 2) as i32;
        let y = (map.tile_y(v.v.dest_tile()) * TILE_SIZE + TILE_SIZE / 2) as i32;
        if delta_sum(v.v.x_pos(), x, v.v.y_pos(), y) >= 8 {
            v.v.set_direction(get_direction_towards(v.v, x, y, t.check()));

            fly(t, v);
            return true;
        }

        if !world.is_valid_tile(v.v.dest_tile()) {
            /* Make sure we don't land outside the map. */
            t.delete(v.v);
            return false;
        }

        let z = t.get_slope_pixel_z(v.v.x_pos(), v.v.y_pos());
        if z < v.v.z_pos() {
            update_position(t, v, v.v.x_pos(), v.v.y_pos(), v.v.z_pos().wrapping_sub(1));
            return true;
        }

        v.set_state(2);

        for target in t.iterate(None) {
            if is_ground_vehicle(target.vehicle_type())
                && delta_sum(target.x_pos(), v.v.x_pos(), target.y_pos(), v.v.y_pos())
                    <= 12 * TILE_SIZE as i32
            {
                target.set_breakdown_ctr(5);
                target.set_breakdown_delay(t.maintenance(), 0xF0);
            }
        }

        let town =
            crate::town::spatial::closest_town_from_tile(t.shared, v.v.dest_tile(), u32::MAX);
        add_tile_news_item(
            &get_encoded_string(
                STR_NEWS_DISASTER_BIG_UFO,
                &[Integer(u64::from(town_index(t, town).0))],
            ),
            NewsType::Accident,
            v.v.tile(),
            StationID::INVALID,
        );

        if !(t.vehicles.can_allocate)(2) {
            t.delete(v.v);
            return false;
        }
        let u = disaster_vehicle_new(
            t,
            -6 * TILE_SIZE as i32,
            v.v.y_pos(),
            Direction::DIR_SW,
            DisasterSubType::ST_BIG_UFO_DESTROYER,
            v.v.index(t.newgrf()),
        );
        let w = disaster_vehicle_new(
            t,
            -6 * TILE_SIZE as i32,
            v.v.y_pos(),
            Direction::DIR_SW,
            DisasterSubType::ST_BIG_UFO_DESTROYER_SHADOW,
            VehicleID::INVALID,
        );
        set_next(u, w);
    } else if v.state() == 0 {
        let x = (map.tile_x(v.v.dest_tile()) * TILE_SIZE) as i32;
        let y = (map.tile_y(v.v.dest_tile()) * TILE_SIZE) as i32;
        if delta_sum(x, v.v.x_pos(), y, v.v.y_pos()) >= TILE_SIZE as i32 {
            v.v.set_direction(get_direction_towards(v.v, x, y, t.check()));
            fly(t, v);
            return true;
        }

        if t.increment_age(v.v) < 6 {
            v.v.set_dest_tile(t.random_tile(&world));
            return true;
        }
        v.set_state(1);

        let is_valid_target = |u: VehicleRef| {
            u.is_front_engine() // Only the engines
                && is_human_id(t.identity(), u.owner()) // Don't break AIs
                && world.is_plain_rail_tile(u.tile()) // No tunnels
                && !u.vehstatus().test(VehState::Crashed) // Not crashed
        };

        let mut n: u32 = 0; // Total number of targetable trains.
        for u in t.iterate(Some(VehicleType::VEH_TRAIN)) {
            if is_valid_target(u) {
                n = n.wrapping_add(1);
            }
        }

        if n == 0 {
            /* If there are no targetable trains, destroy the UFO. */
            t.delete(v.v);
            return false;
        }

        n = t.shared.random_range(n); // Choose one of them.
        for u in t.iterate(Some(VehicleType::VEH_TRAIN)) {
            /* Find (n+1)-th train. */
            if is_valid_target(u) && {
                let old = n;
                n = n.wrapping_sub(1);
                old == 0
            } {
                /* Target it. */
                v.v.set_dest_tile(u.tile());
                t.reset_age(v.v);
                break;
            }
        }
    }

    true
}

/// `t->index` of a live town.
#[allow(unsafe_code)]
fn town_index(
    t: &Tables<'_>,
    town: *const crate::services::TownHandle,
) -> crate::town_type::TownID {
    // SAFETY: The industry table names the process-lifetime resolver table.
    crate::industry::resolver::town_index(town, unsafe { &*t.industry().resolver })
}

/// `u->SetNext(w)` of two disaster vehicles.
#[allow(unsafe_code)]
fn set_next(u: DisasterVehicle, w: DisasterVehicle) {
    // SAFETY: Both records are live, and so is every record their links reach.
    unsafe { crate::vehicle_base::set_next(u.v.0.as_ptr(), w.v.0.as_ptr()) };
}

/// `DisasterTick_Big_Ufo_Destroyer`: skyranger destroying (Big) Ufo handling, `v->state` states:
/// 0. Home in on landed Ufo and shoot it down
fn disaster_tick_big_ufo_destroyer(t: &Tables<'_>, v: DisasterVehicle) -> bool {
    increment_tick_counter(v.v);

    let gp = fly(t, v);
    let world = t.world();
    let map = world.dimensions;

    if gp.x > (map.size_x() * TILE_SIZE + 9 * TILE_SIZE) as i32 - 1 {
        t.delete(v.v);
        return false;
    }

    if v.state() == 0 {
        let u = t
            .vehicle_get(v.big_ufo_destroyer_target().0)
            .expect("Vehicle::Get of the destroyer's target");
        if delta(v.v.x_pos(), u.x_pos()) > TILE_SIZE as i32 {
            return true;
        }
        v.set_state(1);

        let _ = create_effect_vehicle_rel(u, 0, 7, 8, EffectVehicleType::EV_EXPLOSION_LARGE);
        t.play_explosion(u);

        t.delete(u);

        for _ in 0..80 {
            let r = t.shared.random();
            let _ = create_effect_vehicle_above(
                gb(r, 0, 6)
                    .wrapping_add(v.v.x_pos() as u32)
                    .wrapping_sub(32) as i32,
                gb(r, 5, 6)
                    .wrapping_add(v.v.y_pos() as u32)
                    .wrapping_sub(32) as i32,
                0,
                EffectVehicleType::EV_EXPLOSION_SMALL,
            );
        }

        for dy in -3..3 {
            for dx in -3..3 {
                let freeform_edges = (t.environment().freeform_edges)();
                let tile = map.tile_add_wrap(v.v.tile(), dx, dy, world.size_y(), freeform_edges);
                if tile != TileIndex::INVALID {
                    disaster_clear_square(t, tile);
                }
            }
        }
    }

    true
}

/// `DisasterTick_Submarine`: submarine, `v->state` states:
/// Unused, just float around aimlessly and pop up at different places, turning around
fn disaster_tick_submarine(t: &Tables<'_>, v: DisasterVehicle) -> bool {
    increment_tick_counter(v.v);

    if t.increment_age(v.v) > 8880 {
        t.delete(v.v);
        return false;
    }

    if !has_bit(v.v.tick_counter(), 0) {
        return true;
    }

    let world = t.world();
    let check = t.check();
    let tile = v.v.tile()
        + world
            .dimensions
            .tile_offs_by_diag_dir(dir_to_diag_dir(v.v.direction(), check), check);
    if world.is_valid_tile(tile) {
        let trackbits =
            track_status_to_track_bits(crate::movement::table(t.shared).get_tile_track_status(
                tile,
                TransportType::TRANSPORT_WATER,
                0,
                DiagDirection::INVALID_DIAGDIR,
            ));
        if trackbits == TrackBits::TRACK_BIT_ALL && !crate::random_func::chance16(t.shared, 1, 90) {
            let gp = get_new_vehicle_pos(v.v, world.dimensions);
            update_position(t, v, gp.x, gp.y, v.v.z_pos());
            return true;
        }
    }

    v.v.set_direction(change_dir(
        v.v.direction(),
        if gb(t.shared.random(), 0, 1) != 0 {
            DirDiff::DIRDIFF_90RIGHT
        } else {
            DirDiff::DIRDIFF_90LEFT
        },
        check,
    ));

    true
}

/// `DisasterTick_NULL`.
fn disaster_tick_null(_: &Tables<'_>, _: DisasterVehicle) -> bool {
    true
}

type DisasterVehicleTickProc = fn(&Tables<'_>, DisasterVehicle) -> bool;

#[allow(non_upper_case_globals, reason = "the original's name")]
static _disastervehicle_tick_procs: [DisasterVehicleTickProc; 15] = [
    disaster_tick_zeppeliner,
    disaster_tick_null,
    disaster_tick_ufo,
    disaster_tick_null,
    disaster_tick_airplane,
    disaster_tick_null,
    disaster_tick_helicopter,
    disaster_tick_null,
    disaster_tick_helicopter_rotors,
    disaster_tick_big_ufo,
    disaster_tick_null,
    disaster_tick_big_ufo_destroyer,
    disaster_tick_null,
    disaster_tick_submarine,
    disaster_tick_submarine,
];

/// `DisasterVehicle::Tick`.
pub(crate) fn tick(t: &Tables<'_>, this: DisasterVehicle) -> bool {
    _disastervehicle_tick_procs[usize::from(this.v.subtype())](t, this)
}

type DisasterInitProc = fn(&Tables<'_>);

/// `Disaster_Zeppeliner_Init`: zeppeliner which crashes on an airport if one found, otherwise crashes on a random tile
fn disaster_zeppeliner_init(t: &Tables<'_>) {
    if !(t.vehicles.can_allocate)(2) {
        return;
    }
    let world = t.world();
    let map = world.dimensions;

    /* Pick a random place, unless we find an airport */
    let mut x = (map.tile_x(t.random_tile(&world)) * TILE_SIZE + TILE_SIZE / 2) as i32;

    let catchment = crate::station::Ctx::from_shared(t.shared);
    let mut st = (catchment.c.next_station)(u32::from(StationID::BEGIN.0));
    while !st.is_null() {
        let station = st.cast_const().cast::<crate::station::Station>();
        if t.aircraft().airport_tile(station) != TileIndex::INVALID
            && (t.aircraft().airport_type(station) == AT_SMALL
                || t.aircraft().airport_type(station) == AT_LARGE)
        {
            x = ((map.tile_x(t.aircraft().airport_tile(station)) + 2) * TILE_SIZE) as i32;
            break;
        }
        st = (catchment.c.next_station)(u32::from((catchment.c.id)(st).0) + 1);
    }

    let v = disaster_vehicle_new(
        t,
        x,
        0,
        Direction::DIR_SE,
        DisasterSubType::ST_ZEPPELINER,
        VehicleID::INVALID,
    );
    /* Allocate shadow */
    let u = disaster_vehicle_new(
        t,
        x,
        0,
        Direction::DIR_SE,
        DisasterSubType::ST_ZEPPELINER_SHADOW,
        VehicleID::INVALID,
    );
    set_next(v, u);
}

/// `Disaster_Small_Ufo_Init`: ufo which flies around aimlessly from the middle of the map a bit until it locates a road
/// vehicle which it targets and then destroys
fn disaster_small_ufo_init(t: &Tables<'_>) {
    if !(t.vehicles.can_allocate)(2) {
        return;
    }
    let world = t.world();
    let map = world.dimensions;

    let x = (map.tile_x(t.random_tile(&world)) * TILE_SIZE + TILE_SIZE / 2) as i32;
    let v = disaster_vehicle_new(
        t,
        x,
        0,
        Direction::DIR_SE,
        DisasterSubType::ST_SMALL_UFO,
        VehicleID::INVALID,
    );
    v.v.set_dest_tile(map.tile_xy(map.size_x() / 2, world.size_y() / 2));

    /* Allocate shadow */
    let u = disaster_vehicle_new(
        t,
        x,
        0,
        Direction::DIR_SE,
        DisasterSubType::ST_SMALL_UFO_SHADOW,
        VehicleID::INVALID,
    );
    set_next(v, u);
}

/// `for (Industry *i : Industry::Iterate())`: each industry's shell and owner, in pool
/// order.
#[allow(unsafe_code)]
fn industries<'a>(t: &Tables<'a>) -> impl Iterator<Item = crate::industry::Entry> + use<'a> {
    let services = t.industry();
    let mut from = u32::from(IndustryID::BEGIN.0);
    std::iter::from_fn(move || {
        // SAFETY: The process-lifetime industry table's walk.
        let e = unsafe { (services.next_tick)(from) };
        if e.industry.is_null() {
            return None;
        }
        from = u32::from(e.id) + 1;
        Some(e)
    })
}

/// `Disaster_Airplane_Init`: combat airplane which destroys an oil refinery
#[allow(
    clippy::many_single_char_names,
    reason = "Disaster_Airplane_Init's i, x, y, v and u"
)]
fn disaster_airplane_init(t: &Tables<'_>) {
    if !(t.vehicles.can_allocate)(2) {
        return;
    }

    let mut found: Option<crate::industry::Entry> = None;

    for i in industries(t) {
        if industry_behaviour(i.owner).test(IndustryBehaviour::AirplaneAttacks)
            && (found.is_none() || crate::random_func::chance16(t.shared, 1, 2))
        {
            found = Some(i);
        }
    }

    let Some(found) = found else {
        return;
    };
    let map = t.world().dimensions;

    /* Start from the bottom (south side) of the map */
    let x = ((map.size_x() + 9) * TILE_SIZE - 1) as i32;
    let y = (map.tile_y(crate::industry::location(found.owner).tile) * TILE_SIZE + 37) as i32;

    let v = disaster_vehicle_new(
        t,
        x,
        y,
        Direction::DIR_NE,
        DisasterSubType::ST_AIRPLANE,
        VehicleID::INVALID,
    );
    let u = disaster_vehicle_new(
        t,
        x,
        y,
        Direction::DIR_NE,
        DisasterSubType::ST_AIRPLANE_SHADOW,
        VehicleID::INVALID,
    );
    set_next(v, u);
}

/// `Disaster_Helicopter_Init`: combat helicopter that destroys a factory
#[allow(
    clippy::many_single_char_names,
    reason = "Disaster_Helicopter_Init's i, x, y, v, u and w"
)]
fn disaster_helicopter_init(t: &Tables<'_>) {
    if !(t.vehicles.can_allocate)(3) {
        return;
    }

    let mut found: Option<crate::industry::Entry> = None;

    for i in industries(t) {
        if industry_behaviour(i.owner).test(IndustryBehaviour::ChopperAttacks)
            && (found.is_none() || crate::random_func::chance16(t.shared, 1, 2))
        {
            found = Some(i);
        }
    }

    let Some(found) = found else {
        return;
    };
    let map = t.world().dimensions;

    let x = -16 * TILE_SIZE as i32;
    let y = (map.tile_y(crate::industry::location(found.owner).tile) * TILE_SIZE + 37) as i32;

    let v = disaster_vehicle_new(
        t,
        x,
        y,
        Direction::DIR_SW,
        DisasterSubType::ST_HELICOPTER,
        VehicleID::INVALID,
    );
    let u = disaster_vehicle_new(
        t,
        x,
        y,
        Direction::DIR_SW,
        DisasterSubType::ST_HELICOPTER_SHADOW,
        VehicleID::INVALID,
    );
    set_next(v, u);

    let w = disaster_vehicle_new(
        t,
        x,
        y,
        Direction::DIR_SW,
        DisasterSubType::ST_HELICOPTER_ROTORS,
        VehicleID::INVALID,
    );
    set_next(u, w);
}

/// `Disaster_Big_Ufo_Init`: big Ufo which lands on a piece of rail and will consequently be shot down by a combat
/// airplane, destroying the surroundings
#[allow(
    clippy::many_single_char_names,
    reason = "Disaster_Big_Ufo_Init's x, y, v and u"
)]
fn disaster_big_ufo_init(t: &Tables<'_>) {
    if !(t.vehicles.can_allocate)(2) {
        return;
    }
    let world = t.world();
    let map = world.dimensions;

    let x = (map.tile_x(t.random_tile(&world)) * TILE_SIZE + TILE_SIZE / 2) as i32;
    let y = (map.max_x() * TILE_SIZE - 1) as i32;

    let v = disaster_vehicle_new(
        t,
        x,
        y,
        Direction::DIR_NW,
        DisasterSubType::ST_BIG_UFO,
        VehicleID::INVALID,
    );
    v.v.set_dest_tile(map.tile_xy(map.size_x() / 2, world.size_y() / 2));

    /* Allocate shadow */
    let u = disaster_vehicle_new(
        t,
        x,
        y,
        Direction::DIR_NW,
        DisasterSubType::ST_BIG_UFO_SHADOW,
        VehicleID::INVALID,
    );
    set_next(v, u);
}

/// `Disaster_Submarine_Init`.
fn disaster_submarine_init(t: &Tables<'_>, subtype: DisasterSubType) {
    if !(t.vehicles.can_allocate)(1) {
        return;
    }
    let world = t.world();
    let map = world.dimensions;

    let r = t.shared.random();
    let x =
        (map.tile_x(map.random_tile_seed(world.size_y(), r)) * TILE_SIZE + TILE_SIZE / 2) as i32;

    let (y, dir) = if has_bit(r, 31) {
        (
            (Map::max_y(world.size_y()) * TILE_SIZE - TILE_SIZE / 2 - 1) as i32,
            Direction::DIR_NW,
        )
    } else {
        let mut y = (TILE_SIZE / 2) as i32;
        if (t.environment().freeform_edges)() {
            y += TILE_SIZE as i32;
        }
        (y, Direction::DIR_SE)
    };
    if !world.is_water_tile(map.tile_virt_xy(x as u32, y as u32)) {
        return;
    }

    disaster_vehicle_new(t, x, y, dir, subtype, VehicleID::INVALID);
}

/// `Disaster_Small_Submarine_Init`: curious submarine #1, just floats around
fn disaster_small_submarine_init(t: &Tables<'_>) {
    disaster_submarine_init(t, DisasterSubType::ST_SMALL_SUBMARINE);
}

/// `Disaster_Big_Submarine_Init`: curious submarine #2, just floats around
fn disaster_big_submarine_init(t: &Tables<'_>) {
    disaster_submarine_init(t, DisasterSubType::ST_BIG_SUBMARINE);
}

/// `Disaster_CoalMine_Init`: coal mine catastrophe, destroys a stretch of 30 tiles of land in a certain direction
fn disaster_coal_mine_init(t: &Tables<'_>) {
    let mut index = gb(t.shared.random(), 0, 4) as i32;

    for _m in 0..15 {
        for i in industries(t) {
            if industry_behaviour(i.owner).test(IndustryBehaviour::CanSubsidence) && {
                index -= 1;
                index < 0
            } {
                let world = t.world();
                let map = world.dimensions;
                add_tile_news_item(
                    &get_encoded_string(
                        STR_NEWS_DISASTER_COAL_MINE_SUBSIDENCE,
                        &[Integer(u64::from(
                            crate::industry::town_index(i.industry, t.industry()).0,
                        ))],
                    ),
                    NewsType::Accident,
                    crate::industry::location(i.owner).tile + map.diff(1, 1),
                    StationID::INVALID,
                ); // keep the news, even when the mine closes

                {
                    let mut tile = crate::industry::location(i.owner).tile;
                    let step = map.tile_offs_by_diag_dir(
                        DiagDirection(gb(t.shared.random(), 0, 2) as u8),
                        t.check(),
                    );

                    for _n in 0..30 {
                        disaster_clear_square(t, tile);
                        tile += step;
                        if !world.is_valid_tile(tile) {
                            break;
                        }
                    }
                }
                return;
            }
        }
    }
}

struct Disaster {
    /// The init function for this disaster.
    init_proc: DisasterInitProc,
    /// The first year this disaster will occur.
    min_year: CalendarYear,
    /// The last year this disaster will occur.
    max_year: CalendarYear,
}

#[allow(non_upper_case_globals, reason = "the original's name")]
static _disasters: [Disaster; 8] = [
    Disaster {
        init_proc: disaster_zeppeliner_init,
        min_year: CalendarYear(1930),
        max_year: CalendarYear(1955),
    }, // zeppeliner
    Disaster {
        init_proc: disaster_small_ufo_init,
        min_year: CalendarYear(1940),
        max_year: CalendarYear(1970),
    }, // ufo {small}
    Disaster {
        init_proc: disaster_airplane_init,
        min_year: CalendarYear(1960),
        max_year: CalendarYear(1990),
    }, // airplane
    Disaster {
        init_proc: disaster_helicopter_init,
        min_year: CalendarYear(1970),
        max_year: CalendarYear(2000),
    }, // helicopter
    Disaster {
        init_proc: disaster_big_ufo_init,
        min_year: CalendarYear(2000),
        max_year: CalendarYear(2100),
    }, // ufo {big}
    Disaster {
        init_proc: disaster_small_submarine_init,
        min_year: CalendarYear(1940),
        max_year: CalendarYear(1965),
    }, // submarine {small}
    Disaster {
        init_proc: disaster_big_submarine_init,
        min_year: CalendarYear(1975),
        max_year: CalendarYear(2010),
    }, // submarine {big}
    Disaster {
        init_proc: disaster_coal_mine_init,
        min_year: CalendarYear(1950),
        max_year: CalendarYear(1985),
    }, // coalmine
];

/// `DoDisaster`.
fn do_disaster(t: &Tables<'_>) {
    let mut available_disasters: Vec<DisasterInitProc> = Vec::new();

    let year = crate::timer_game_common::calendar_year();
    for disaster in &_disasters {
        if year >= disaster.min_year && year < disaster.max_year {
            available_disasters.push(disaster.init_proc);
        }
    }

    if available_disasters.is_empty() {
        return;
    }

    available_disasters[t.shared.random_range(available_disasters.len() as u32) as usize](t);
}

/// `ResetDisasterDelay`.
#[allow(unsafe_code)]
fn reset_disaster_delay(t: &Tables<'_>) {
    let delay = (gb(t.shared.random(), 0, 9) + 730) as u16;
    // SAFETY: One store to the process-wide counter (its declaration's contract).
    unsafe { _disaster_delay = delay };
}

/// The `_economy_disaster_daily` timer's body.
///
/// # Safety
/// `s` is the live process-lifetime table, and the caller holds the game-state mutex.
#[allow(unsafe_code)]
pub(crate) unsafe fn economy_disaster_daily(s: *const Services) {
    // SAFETY: As this function's contract.
    let t = Tables::new(unsafe { &*s });
    // SAFETY: One load and store of the process-wide counter (its declaration's contract).
    let delay = unsafe {
        _disaster_delay = _disaster_delay.wrapping_sub(1);
        _disaster_delay
    };
    if delay != 0 {
        return;
    }

    reset_disaster_delay(&t);

    if t.shared.settings().disasters() {
        do_disaster(&t);
    }
}

/// `StartupDisasters`.
///
/// # Safety
/// As [`economy_disaster_daily`].
#[allow(unsafe_code)]
pub(crate) unsafe fn startup_disasters(s: *const Services) {
    // SAFETY: As this function's contract.
    reset_disaster_delay(&Tables::new(unsafe { &*s }));
}

/// `ReleaseDisastersTargetingIndustry`: marks all disasters targeting the deleted industry
/// `i` in such a way they won't call `Industry::Get(v->dest_tile)` on invalid industry
/// anymore.
#[allow(unsafe_code)]
pub(crate) fn release_disasters_targeting_industry(shared: &Shared, i: IndustryID) {
    // SAFETY: The shared table names the process-lifetime command tables.
    let vehicles = unsafe { &*(*(shared.station_commands)()).vehicle };
    for v in vehicles_of(shared, vehicles.newgrf(), Some(VehicleType::VEH_DISASTER)) {
        let v = DisasterVehicle::of(v);
        /* primary disaster vehicles that have chosen target */
        let subtype = DisasterSubType(v.v.subtype());
        if subtype == DisasterSubType::ST_AIRPLANE || subtype == DisasterSubType::ST_HELICOPTER {
            /* if it has chosen target, and it is this industry (yes, dest_tile is IndustryID here), set order to "leaving map peacefully" */
            if v.state() > 0 && v.v.dest_tile().0 == u32::from(i.0) {
                v.set_state(3);
            }
        }
    }
}

/// `ReleaseDisasterVehicle`'s body after its lookup: notify the disaster `v` that its
/// target is about to be deleted, so make it head elsewhere.
pub(crate) fn release_disaster_vehicle(t: &Tables<'_>, v: DisasterVehicle) {
    /* primary disaster vehicles that have chosen target */
    crate::services_assert::assert_with(|| {
        DisasterSubType(v.v.subtype()) == DisasterSubType::ST_SMALL_UFO
    });
    crate::services_assert::assert_with(|| v.state() != 0);

    /* Revert to target-searching */
    v.set_state(0);
    v.v.set_dest_tile(t.random_tile(&t.world()));
    t.set_z_pos_to_flight_level_minimum(v.v);
    t.reset_age(v.v);
}

/// Allocate the owner of one native shell, with the member defaults.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_disaster_state_create() -> *mut State {
    Box::into_raw(Box::new(State {
        image_override: 0,
        big_ufo_destroyer_target: VehicleID::INVALID.0,
        state: 0,
        flags: 0,
    }))
}
/// # Safety
/// `state` is its live allocation with no remaining native alias use or Rust borrow.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_state_destroy(state: *mut State) {
    // SAFETY: As this function's contract.
    unsafe {
        drop(Box::from_raw(state));
    }
}
/// The address of `_disaster_delay`, which the modern and legacy save descriptors share.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_disaster_delay() -> *mut u16 {
    &raw mut _disaster_delay
}

// The C++ shell's entries. SAFETY for each: the caller holds the game-state mutex and passes
// the immutable process-lifetime table and a live disaster vehicle record.

/// `DisasterVehicle::Tick`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_tick_abi(s: *const Services, v: VehicleRef) -> u8 {
    // SAFETY: As the entries' contract.
    let t = Tables::new(unsafe { &*s });
    u8::from(tick(&t, DisasterVehicle::of(v)))
}
/// `ReleaseDisasterVehicle`, after its `DisasterVehicle::GetIfValid` found `v`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_disaster_release_vehicle(s: *const Services, v: VehicleRef) {
    // SAFETY: As the entries' contract.
    let t = Tables::new(unsafe { &*s });
    release_disaster_vehicle(&t, DisasterVehicle::of(v));
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDDisasterState" ["probe"] => State {
        image_override,
        big_ufo_destroyer_target,
        state,
        flags,
    };
    "OpenTTDDisasterServices" ["probe"] => Services {
        aircraft,
        allocate,
        environment,
        identity,
        industry,
        road,
        road_count,
        scheduler,
        shared,
        vehicles,
    };
}

#[cfg(test)]
mod tests;
