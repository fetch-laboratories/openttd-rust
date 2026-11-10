/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! House lifecycle and tile simulation: construction stages, builtin lifts, cargo, clearing,
//! rebuilding, placement, cache rebuild and house `NewGRF` trigger traversal. Packed map
//! fields, pools, specs, sprite resolvers and `AnimationBase` stay native.
use super::{
    AuthorityServices, Leaves, State, TownCouncilAttitudes, TownFlag, TownFlags, TownRef,
    change_population, engine, update_town_radius,
};
use crate::bitmath_func::{gb, has_bit, set_bit64};
use crate::cargo_type::{CargoArray, CargoLabel, CargoType, CargoTypes, is_valid_cargo_type};
use crate::cargotype::{TOWN_PRODUCTION_DIVISOR, TownProductionEffect};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::CompanyID;
use crate::direction_type::DiagDirection;
use crate::economy_type::{EXPENSES_CONSTRUCTION, PR_BUILD_FOUNDATION};
use crate::history_type::THIS_MONTH;
use crate::house::{
    BuildingFlag, BuildingFlags, HOUSE_NUM_ACCEPTS, HouseExtraFlag, HouseExtraFlags,
    NEW_HOUSE_OFFSET,
};
use crate::house_type::{HouseID, HouseRandomTrigger, HouseRandomTriggers};
use crate::landscape_type::LandscapeType;
use crate::map_type::TileIndexDiff;
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CALLBACK_HOUSEPRODCARGO_END, CallbackID, HouseCallbackMask, HouseCallbackMasks,
};
use crate::random_func::chance16_i;
use crate::services::{
    CommandCost, CommandCostServices, CommandServices, Map, Services, TownHandle,
};
use crate::settings_type::PlaceHouses;
use crate::slope_func::{get_slope_max_z, is_steep_slope};
use crate::slope_type::Slope;
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_map::is_tile_type;
use crate::tile_type::{TileIndex, TileType};
use crate::town_type::{
    RATING_HOUSE_MINIMUM, RATING_MAXIMUM, TownCargoGenMode, TownExpandMode, TownExpandModes, TownID,
};
use crate::witness;
use std::cell::{OnceCell, RefCell};

// Branch witnesses, in `witness::NAMES` order from `witness::HOUSE`.
const W_STAGE: usize = witness::HOUSE;
const W_COMPLETE: usize = witness::HOUSE_COMPLETE;
const W_LIFT_START: usize = witness::HOUSE_LIFT_START;
const W_LIFT_STOP: usize = witness::HOUSE_LIFT_STOP;
const W_ORIGINAL: usize = witness::HOUSE_ORIGINAL;
const W_BINOMIAL: usize = witness::HOUSE_BINOMIAL;
const W_RECESSION: usize = witness::HOUSE_RECESSION;
const W_REBUILD: usize = witness::HOUSE_REBUILD;
const W_REPLACE: usize = witness::HOUSE_REPLACE;
const W_CLEAR_AUTO: usize = witness::HOUSE_CLEAR_AUTO;
const W_CLEAR_REFUSED: usize = witness::HOUSE_CLEAR_REFUSED;
const W_CLEAR_EXECUTE: usize = witness::HOUSE_CLEAR_EXECUTE;
const W_AUTOSLOPE: usize = witness::HOUSE_AUTOSLOPE;
const W_PLACE: usize = witness::HOUSE_PLACE;
const W_PLACE_ERROR: usize = witness::HOUSE_PLACE_ERROR;
const W_STATION_REMOVED: usize = witness::HOUSE_STATION_REMOVED;
const W_PROTECTED: usize = witness::HOUSE_PROTECTED;

/// Narrow observations at their original map read points. No owner or map
/// reference survives a service; these contain only fields used by that phase.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseLoopTile {
    pub age: u8,
    pub completed: bool,
    pub lift_has_destination: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseLift {
    pub position: u8,
    pub destination: u8,
    pub has_destination: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseRandom {
    pub bits: u8,
    pub triggers: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseConstruction {
    pub house: u16,
    pub tick: u8,
    pub completed: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseCache {
    pub house: u16,
    pub town: u16,
    pub completed: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseAge {
    pub age: u8,
    pub completed: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseEntry {
    pub town: TownRef,
    pub house: u16,
}
/// Native projections compose the canonical spec readers, so observing a phase
/// uses one crossing without defining another accessor for any `HouseSpec` field.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseLoopSpec {
    pub callbacks: u16,
    pub flags: u8,
    pub population: u8,
    pub mail: u8,
    pub minimum_life: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseAnimationSpec {
    pub callbacks: u16,
    pub extra_flags: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseNewSpec {
    pub callbacks: u16,
    pub flags: u8,
    pub processing_time: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseProduction {
    pub callbacks: u16,
    pub population: u8,
    pub mail: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseRemoval {
    pub factor: u8,
    pub rating: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseAcceptance {
    pub accepts: [u8; HOUSE_NUM_ACCEPTS as usize],
    pub amounts: [u8; HOUSE_NUM_ACCEPTS as usize],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseReseed {
    pub reseed: u32,
    pub used: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseErrors {
    pub demolish: u32,
    pub protected: u32,
    pub refuses: u32,
    pub sloped: u32,
    pub bridge: u32,
    pub flat: u32,
    pub no_town: u32,
}
/// Typed native house services; shared tables are referenced, not copied.
#[repr(C)]
pub struct HouseServices {
    pub cost: *const CommandCostServices,
    pub commands: *const CommandServices,
    pub authority: *const AuthorityServices,
    pub entry: extern "C" fn(u32) -> HouseEntry,
    pub loop_tile: extern "C" fn(u32) -> HouseLoopTile,
    pub house_type: extern "C" fn(u32) -> u16,
    pub completed: extern "C" fn(u32) -> bool,
    pub protected: extern "C" fn(u32) -> bool,
    pub lift: extern "C" fn(u32) -> HouseLift,
    pub random: extern "C" fn(u32) -> HouseRandom,
    pub construction: extern "C" fn(u32) -> HouseConstruction,
    pub cache: extern "C" fn(u32) -> HouseCache,
    pub age: extern "C" fn(u32) -> HouseAge,
    pub processing: extern "C" fn(u32) -> u8,
    pub clean_type: extern "C" fn(u32) -> u16,
    pub town: extern "C" fn(u32) -> TownRef,
    pub loop_spec: extern "C" fn(u16) -> HouseLoopSpec,
    pub animation_spec: extern "C" fn(u16) -> HouseAnimationSpec,
    pub new_spec: extern "C" fn(u16) -> HouseNewSpec,
    pub production: extern "C" fn(u16) -> HouseProduction,
    pub sprite_groups: extern "C" fn(u16) -> bool,
    pub removal: extern "C" fn(u16) -> HouseRemoval,
    pub acceptance: extern "C" fn(u16) -> HouseAcceptance,
    pub watched: extern "C" fn(u16) -> u64,
    pub inc_construction: extern "C" fn(u32),
    pub reset_age: extern "C" fn(u32),
    pub set_age: extern "C" fn(u32, u8),
    pub set_lift_destination: extern "C" fn(u32, u8),
    pub set_lift_position: extern "C" fn(u32, u8),
    pub halt_lift: extern "C" fn(u32),
    pub dec_processing: extern "C" fn(u32),
    pub set_processing: extern "C" fn(u32, u8),
    pub set_random_bits: extern "C" fn(u32, u8),
    pub set_random_triggers: extern "C" fn(u32, u8),
    pub set_type: extern "C" fn(u32, u16),

    pub delete_inspect: extern "C" fn(u32),
    pub cargo_file: extern "C" fn(u16) -> *const std::ffi::c_void,
    pub convert_8bit: extern "C" fn(u16, u16, u16) -> bool,

    pub scale_cargo: extern "C" fn(u32) -> u32,
    pub cargogen_mode: extern "C" fn() -> u8,
    pub place_houses: extern "C" fn() -> u8,
    pub errors: HouseErrors,
}

/// `TileLoop_Town`'s call-local `StationFinder`, computed on the first
/// `GetStations` and reused for the rest of the tile; the catchment tables are
/// captured with it.
struct StationFinder<'a> {
    finder: RefCell<crate::station::StationFinder>,
    catchment: OnceCell<crate::station::Ctx<'a>>,
}

impl StationFinder<'_> {
    /// `StationFinder stations(TileArea(tile, 1, 1))`.
    fn new(tile: TileIndex) -> Self {
        let area = crate::tilearea::TileArea::new(tile.0, 1, 1);
        Self {
            finder: RefCell::new(crate::station::StationFinder::new(area)),
            catchment: OnceCell::new(),
        }
    }
}

/// `produced[cargo]++`.
fn add_one(cargoes: &mut CargoArray, cargo: CargoType) {
    let slot = &mut cargoes[usize::from(cargo.0)];
    *slot = slot.wrapping_add(1);
}

/// Gameplay arguments of `town_cmd.cpp::CmdPlaceHouse`, in command order.
#[derive(Clone, Copy)]
struct CmdPlaceHouse {
    tile: TileIndex,
    house: HouseID,
    is_protected: bool,
    replace: bool,
}

/// Execution flags and result storage for the house command.
#[derive(Clone, Copy)]
struct HouseCommandCall {
    result: *mut CommandCost,
    flags: DoCommandFlags,
}

/// Immutable native tables for one synchronous entry.
struct House<'a> {
    l: &'a Leaves,
    s: &'a Services,
    h: &'a HouseServices,
    map: OnceCell<Map>,
}
fn house<'a>(leaves: *const Leaves, services: *const Services) -> House<'a> {
    // SAFETY: Native immutable static tables outlive every synchronous entry.
    let (l, s) = unsafe { (&*leaves, &*services) };
    // SAFETY: The leaves table references the static house table.
    let h = unsafe { &*l.houses };
    House {
        l,
        s,
        h,
        map: OnceCell::new(),
    }
}
/// `HouseSpec::Get(house)->building_flags`.
pub(super) fn building_flags(leaves: &Leaves, house: HouseID) -> BuildingFlags {
    BuildingFlags((leaves.house_flags)(house))
}
/// Determines if a given `HouseID` is part of a multitile house (`GetHouseNorthPart`).
/// The given ID is set to the ID of the north tile and the `TileIndexDiff` to the north
/// tile is returned. Shared by lifecycle and resolver queries.
pub(super) fn get_house_north_part(
    leaves: &Leaves,
    house: &mut HouseID,
    map: Map,
) -> TileIndexDiff {
    // House id 0,1,2 MUST be single tile houses, or this code breaks.
    if *house >= 3 {
        let previous_flags = building_flags(leaves, *house - 1);
        if previous_flags.test(BuildingFlag::Size2x1) {
            *house -= 1;
            return map.diff(-1, 0).cast_signed();
        } else if previous_flags.any(BuildingFlags::BUILDING_2_TILES_Y) {
            *house -= 1;
            return map.diff(0, -1).cast_signed();
        } else if building_flags(leaves, *house - 2).any(BuildingFlags::BUILDING_HAS_4_TILES) {
            *house -= 2;
            return map.diff(-1, 0).cast_signed();
        } else if building_flags(leaves, *house - 3).any(BuildingFlags::BUILDING_HAS_4_TILES) {
            *house -= 3;
            return map.diff(-1, -1).cast_signed();
        }
    }
    map.diff(0, 0).cast_signed()
}
impl<'a> House<'a> {
    fn map(&self) -> Map {
        *self
            .map
            .get_or_init(|| Map::from_log_x((self.l.map_log_x)()))
    }
    fn engine(&self, town: TownRef) -> super::Engine<'_> {
        let mut engine = engine(self.l, self.s);
        engine.owner = town.state;
        let _ = engine.house_shell.set(town.shell);
        engine.map = self.map.clone();
        engine
    }
    fn costs(&self) -> &CommandCostServices {
        // SAFETY: Static table referenced by the static house table.
        unsafe { &*self.h.cost }
    }
    fn authority(&self) -> &AuthorityServices {
        // SAFETY: Static table referenced by the static house table.
        unsafe { &*self.h.authority }
    }
    fn building_flags(&self, house: HouseID) -> BuildingFlags {
        building_flags(self.l, house)
    }
    /// `HouseSpec::Get(house)->callback_mask`.
    fn callback_mask(&self, house: HouseID) -> HouseCallbackMasks {
        HouseCallbackMasks((self.l.house_callbacks)(house))
    }
    /// `GetHouseType`.
    fn get_house_type(&self, tile: TileIndex) -> HouseID {
        (self.h.house_type)(tile.0)
    }
    /// `IsTileType(tile, MP_HOUSE)`.
    fn is_house_tile(&self, tile: TileIndex) -> bool {
        is_tile_type(self.s, tile.0, TileType::MP_HOUSE.0)
    }
    /// `tile + TileDiffXY(x, y)`.
    fn tile_diff_xy(&self, tile: TileIndex, x: i32, y: i32) -> TileIndex {
        TileIndex(tile.0.wrapping_add(self.map().diff(x, y)))
    }
    /// `TileAddXY`.
    fn tile_add_xy(&self, tile: TileIndex, x: i32, y: i32) -> TileIndex {
        TileIndex(self.map().tile_add_xy(tile.0, x, y))
    }
    /// `MarkTileDirtyByTile`.
    fn mark_tile_dirty_by_tile(&self, tile: TileIndex) {
        (self.s.mark_dirty)(tile.0);
    }
    fn get_house_north_part(&self, house: &mut HouseID) -> TileIndexDiff {
        get_house_north_part(self.l, house, self.map())
    }
    /// `TriggerHouseAnimation_ConstructionStageChanged`.
    fn trigger_house_animation_construction_stage_changed(
        &self,
        town: TownRef,
        tile: TileIndex,
        house: HouseID,
        first_call: bool,
    ) {
        if self
            .callback_mask(house)
            .test(HouseCallbackMask::AnimationTriggerConstructionStageChanged)
        {
            let random = self.s.random();
            // SAFETY: The entry resolved this live town shell; no owner borrow
            // spans the synchronous animation callback.
            unsafe {
                crate::animated_tile::house_change(
                    CallbackID::CBID_HOUSE_ANIMATION_TRIGGER_CONSTRUCTION_STAGE_CHANGED.0,
                    house,
                    town.shell,
                    tile.0,
                    random,
                    u32::from(first_call),
                    0,
                );
            }
        }
    }
    /// `CanDeleteHouse`.
    fn can_delete_house(&self, town: TownRef, tile: TileIndex, house: HouseID) -> bool {
        let company = CompanyID((self.s.current_company)());
        // Humans are always allowed to remove buildings, as is water and disasters and
        // anyone using the scenario editor.
        if (self.s.company_human)(company.0)
            || company == CompanyID::WATER
            || company == CompanyID::NONE
            || (self.l.environment.editor)()
            || (self.l.generating_world)()
        {
            return true;
        }
        if self
            .callback_mask(house)
            .test(HouseCallbackMask::DenyDestruction)
        {
            let callback = CallbackID::CBID_HOUSE_DENY_DESTRUCTION;
            // SAFETY: The entry resolved the live town; no owner borrow spans resolution.
            let callback_res = unsafe {
                super::resolver::get_house_callback(callback, 0, 0, house, town.shell, tile, false)
            };
            callback_res == CALLBACK_FAILED
                || !(self.s.grf_boolean_callback)(
                    (self.h.cargo_file)(house),
                    callback.0,
                    callback_res,
                )
        } else {
            !(self.h.protected)(tile.0)
        }
    }
    /// `RemoveNearbyStations`: remove stations from the nearby station list if the town
    /// is no longer in the catchment area of each. To improve performance only checks
    /// stations that cover the provided house area (which need not contain a house).
    /// Ordered set traversal; erasure keeps the successor.
    fn remove_nearby_stations(&self, town: TownRef, tile: TileIndex, flags: BuildingFlags) {
        let c = crate::station::Ctx::from_shared(self.s);
        let list = super::stations_near(town.state);
        let below = self.tile_diff_xy(tile, 0, 1);
        let right = self.tile_diff_xy(tile, 1, 0);
        let diagonal = self.tile_diff_xy(tile, 1, 1);
        // SAFETY: The entry resolved the live town; erasure removes only set
        // membership and traversal resumes after the visited station.
        unsafe {
            crate::station::for_each_station(list, |near| {
                let st = (c.c.owner)(near.station);
                let mut covers_area = crate::station::tile_is_in_catchment(st, tile, &c);
                if flags.any(BuildingFlags::BUILDING_2_TILES_Y) {
                    covers_area |= crate::station::tile_is_in_catchment(st, below, &c);
                }
                if flags.any(BuildingFlags::BUILDING_2_TILES_X) {
                    covers_area |= crate::station::tile_is_in_catchment(st, right, &c);
                }
                if flags.any(BuildingFlags::BUILDING_HAS_4_TILES) {
                    covers_area |= crate::station::tile_is_in_catchment(st, diagonal, &c);
                }
                if covers_area && !crate::station::catchment_covers_town(st, town.id, &c) {
                    witness::hit(W_STATION_REMOVED);
                    (*list).erase(near.id);
                }
            });
        }
    }
    /// `DoClearTownHouseHelper`: update data structures when a house is removed.
    fn do_clear_town_house_helper(&self, tile: TileIndex, town: TownRef, house: HouseID) {
        let class = (self.l.house_class)(house);
        // SAFETY: Live owner; count access ends before the map services below.
        unsafe { super::decrease_building(town.state, house, class) };
        crate::ground::clear_square(tile.0, self.s);
        (self.h.delete_inspect)(tile.0);
    }
    /// `ClearTownHouse`: population, counts and flags precede the footprint.
    fn clear_town_house(&self, town: TownRef, tile: TileIndex, mut house: HouseID) {
        // The northernmost tile of the house is the main house.
        let tile = TileIndex(
            tile.0
                .wrapping_add_signed(self.get_house_north_part(&mut house)),
        );
        let flags = self.building_flags(house);
        // Remove population from the town if the house is finished.
        if (self.h.completed)(tile.0) {
            // SAFETY: Live owner; presentation callbacks run after the write.
            unsafe {
                change_population(
                    self.l,
                    town.state,
                    town.id,
                    -i32::from((self.l.house_population)(house)),
                );
            }
        }
        // SAFETY: Immediate owner mutations; no service runs while accessed.
        unsafe {
            let fields = &raw mut (*town.state).simulation;
            (*fields).num_houses = (*fields).num_houses.wrapping_sub(1);
            // Clear flags for houses that only may exist once/town.
            let mut town_flags = TownFlags((*town.state).growth.flags);
            if flags.test(BuildingFlag::IsChurch) {
                town_flags.reset(TownFlag::HasChurch);
                (*town.state).growth.flags = town_flags.0;
            } else if flags.test(BuildingFlag::IsStadium) {
                town_flags.reset(TownFlag::HasStadium);
                (*town.state).growth.flags = town_flags.0;
            }
        }
        // Do the actual clearing of tiles.
        self.do_clear_town_house_helper(tile, town, house);
        if flags.any(BuildingFlags::BUILDING_2_TILES_Y) {
            house = house.wrapping_add(1);
            self.do_clear_town_house_helper(self.tile_diff_xy(tile, 0, 1), town, house);
        }
        if flags.any(BuildingFlags::BUILDING_2_TILES_X) {
            house = house.wrapping_add(1);
            self.do_clear_town_house_helper(self.tile_diff_xy(tile, 1, 0), town, house);
        }
        if flags.any(BuildingFlags::BUILDING_HAS_4_TILES) {
            house = house.wrapping_add(1);
            self.do_clear_town_house_helper(self.tile_diff_xy(tile, 1, 1), town, house);
        }
        self.remove_nearby_stations(town, tile, flags);
        // SAFETY: Pure immediate owner mutation, without native callbacks.
        unsafe { update_town_radius(town.state) };
    }
    /// `TriggerHouseAnimation_TileLoop`: call the tile loop animation trigger for
    /// houses, if enabled; `random_bits` are the shared bits of the synchronized trigger.
    fn trigger_house_animation_tile_loop(
        &self,
        town: TownRef,
        tile: TileIndex,
        sync: bool,
        random_bits: u16,
    ) {
        let house = self.get_house_type(tile);
        let spec = (self.h.animation_spec)(house);
        // Check whether the matching trigger is enabled.
        if HouseCallbackMasks(spec.callbacks).test(HouseCallbackMask::AnimationTriggerTileLoop)
            && HouseExtraFlags(spec.extra_flags).test(HouseExtraFlag::SynchronisedCallback1B)
                == sync
        {
            let param = if sync {
                gb(self.s.random(), 0, 16) | u32::from(random_bits) << 16
            } else {
                self.s.random()
            };
            // SAFETY: The entry resolved this live town shell; no owner borrow
            // spans the synchronous animation callback.
            unsafe {
                crate::animated_tile::house_change(
                    CallbackID::CBID_HOUSE_ANIMATION_TRIGGER_TILE_LOOP.0,
                    house,
                    town.shell,
                    tile.0,
                    param,
                    0,
                    0,
                );
            }
        }
    }
    /// `DoTriggerHouseRandomisation`, including TTDP's scope-ignoring reseed.
    fn do_trigger_house_randomisation(
        &self,
        town: TownRef,
        tile: TileIndex,
        trigger: HouseRandomTrigger,
        base_random: u8,
        first: bool,
    ) {
        // We can't trigger a non-existent building...
        let hid = self.get_house_type(tile);
        if !(self.h.sprite_groups)(hid) {
            return;
        }
        let rec = (self.h.random)(tile.0);
        let mut waiting_random_triggers = HouseRandomTriggers(rec.triggers);
        waiting_random_triggers.set(trigger);
        // Store now for var 5F.
        (self.h.set_random_triggers)(tile.0, waiting_random_triggers.0);
        // SAFETY: The entry resolved the live town; no owner borrow spans resolution.
        let reseed = unsafe {
            super::resolver::reseed_house_tile(hid, tile, town.shell, waiting_random_triggers)
        };
        // Store remaining triggers.
        waiting_random_triggers.reset(HouseRandomTriggers(reseed.used));
        (self.h.set_random_triggers)(tile.0, waiting_random_triggers.0);
        // Rerandomise bits. Scopes other than SELF are invalid for houses. For
        // bug-to-bug-compatibility with TTDP we ignore the scope.
        let new_random_bits = self.s.random() as u8;
        let mut random_bits = (u32::from(rec.bits) & !reseed.reseed) as u8;
        random_bits |=
            (u32::from(if first { new_random_bits } else { base_random }) & reseed.reseed) as u8;
        (self.h.set_random_bits)(tile.0, random_bits);
        // `HouseRandomTrigger::TileLoop`: random value already set.
        if trigger == HouseRandomTrigger::TileLoopNorth {
            if !first {
                // The top tile is marked dirty by the usual TileLoop.
                self.mark_tile_dirty_by_tile(tile);
                return;
            }
            // Random value of first tile already set.
            let flags = self.building_flags(hid);
            if flags.any(BuildingFlags::BUILDING_2_TILES_Y) {
                let part = self.tile_add_xy(tile, 0, 1);
                self.do_trigger_house_randomisation(town, part, trigger, random_bits, false);
            }
            if flags.any(BuildingFlags::BUILDING_2_TILES_X) {
                let part = self.tile_add_xy(tile, 1, 0);
                self.do_trigger_house_randomisation(town, part, trigger, random_bits, false);
            }
            if flags.any(BuildingFlags::BUILDING_HAS_4_TILES) {
                let part = self.tile_add_xy(tile, 1, 1);
                self.do_trigger_house_randomisation(town, part, trigger, random_bits, false);
            }
        }
    }
    /// `TriggerHouseRandomisation`.
    fn trigger_house_randomisation(
        &self,
        town: TownRef,
        tile: TileIndex,
        trigger: HouseRandomTrigger,
    ) {
        self.do_trigger_house_randomisation(town, tile, trigger, 0, true);
    }
    /// `NewHouseTileLoop`; false when callback 21 destroyed the house.
    fn new_house_tile_loop(&self, town: TownRef, tile: TileIndex, house: HouseID) -> bool {
        if (self.h.processing)(tile.0) > 0 {
            (self.h.dec_processing)(tile.0);
            return true;
        }
        let hs = (self.h.new_spec)(house);
        let flags = BuildingFlags(hs.flags);
        self.trigger_house_randomisation(town, tile, HouseRandomTrigger::TileLoop);
        if flags.any(BuildingFlags::BUILDING_HAS_1_TILE) {
            self.trigger_house_randomisation(town, tile, HouseRandomTrigger::TileLoopNorth);
        }
        // Call the unsynchronized tile loop trigger.
        self.trigger_house_animation_tile_loop(town, tile, false, 0);
        // Call the synchronized tile loop trigger, if this is the north tile.
        if flags.any(BuildingFlags::BUILDING_HAS_1_TILE) {
            let random = gb(self.s.random(), 0, 16) as u16;
            self.trigger_house_animation_tile_loop(town, tile, true, random);
            if flags.any(BuildingFlags::BUILDING_2_TILES_Y) {
                let part = self.tile_add_xy(tile, 0, 1);
                self.trigger_house_animation_tile_loop(town, part, true, random);
            }
            if flags.any(BuildingFlags::BUILDING_2_TILES_X) {
                let part = self.tile_add_xy(tile, 1, 0);
                self.trigger_house_animation_tile_loop(town, part, true, random);
            }
            if flags.any(BuildingFlags::BUILDING_HAS_4_TILES) {
                let part = self.tile_add_xy(tile, 1, 1);
                self.trigger_house_animation_tile_loop(town, part, true, random);
            }
        }
        // Check callback 21, which determines if a house should be destroyed.
        if HouseCallbackMasks(hs.callbacks).test(HouseCallbackMask::Destruction) {
            let callback = CallbackID::CBID_HOUSE_DESTRUCTION;
            let house = self.get_house_type(tile);
            // SAFETY: Live town; the destruction command follows resolution.
            let callback_res = unsafe {
                super::resolver::get_house_callback(callback, 0, 0, house, town.shell, tile, false)
            };
            if callback_res != CALLBACK_FAILED
                && (self.h.convert_8bit)(house, callback.0, callback_res)
            {
                self.clear_town_house(town, tile, house);
                return false;
            }
        }
        (self.h.set_processing)(tile.0, hs.processing_time);
        self.mark_tile_dirty_by_tile(tile);
        true
    }
    /// `AdvanceSingleHouseConstruction`: house construction stage progression.
    fn advance_single_house_construction(&self, town: TownRef, tile: TileIndex) {
        // Progress in construction stages.
        (self.h.inc_construction)(tile.0);
        let rec = (self.h.construction)(tile.0);
        if rec.tick != 0 {
            return;
        }
        witness::hit(W_STAGE);
        self.trigger_house_animation_construction_stage_changed(town, tile, rec.house, false);
        // Animation callbacks change frames only; completion is unchanged.
        if rec.completed {
            witness::hit(W_COMPLETE);
            // Now that construction is complete, we can add the population of the
            // building to the town.
            let population = (self.l.house_population)(rec.house);
            // SAFETY: Live owner; presentation callbacks follow the write.
            unsafe {
                change_population(self.l, town.state, town.id, i32::from(population));
            }
            (self.h.reset_age)(tile.0);
        }
        self.mark_tile_dirty_by_tile(tile);
    }
    /// `AdvanceHouseConstruction`: increase the construction stage of a house.
    fn advance_house_construction(&self, town: TownRef, tile: TileIndex, house: HouseID) {
        let flags = self.building_flags(house);
        if flags.any(BuildingFlags::BUILDING_HAS_1_TILE) {
            self.advance_single_house_construction(town, self.tile_add_xy(tile, 0, 0));
        }
        if flags.any(BuildingFlags::BUILDING_2_TILES_Y) {
            self.advance_single_house_construction(town, self.tile_add_xy(tile, 0, 1));
        }
        if flags.any(BuildingFlags::BUILDING_2_TILES_X) {
            self.advance_single_house_construction(town, self.tile_add_xy(tile, 1, 0));
        }
        if flags.any(BuildingFlags::BUILDING_HAS_4_TILES) {
            self.advance_single_house_construction(town, self.tile_add_xy(tile, 1, 1));
        }
    }
    /// `TownGenerateCargo`: generate cargo for a house, scaled by the current economy
    /// scale. Production precedes the delivery; transported follows.
    fn town_generate_cargo(
        &self,
        town: TownRef,
        cargo: CargoType,
        mut amount: u32,
        stations: &StationFinder<'a>,
        affected_by_recession: bool,
    ) {
        if amount == 0 {
            return;
        }
        // All production is halved during a recession (except for NewGRF-supplied town cargo).
        if affected_by_recession && (self.s.recession)() {
            witness::hit(W_RECESSION);
            amount = (amount + 1) >> 1;
        }
        // Scale by cargo scale setting.
        amount = (self.h.scale_cargo)(amount);
        if amount == 0 {
            return;
        }
        // Actually generate cargo and update town statistics.
        // SAFETY: Live owner. Like the source reference, the row pointer spans
        // only the station delivery, which never touches town supplied rows.
        let supplied = unsafe { super::supplied_entry(town.state, cargo) }.entry;
        unsafe {
            (*supplied).history[usize::from(THIS_MONTH)].production = (*supplied).history
                [usize::from(THIS_MONTH)]
            .production
            .wrapping_add(amount);
        }
        let c = stations
            .catchment
            .get_or_init(|| crate::station::Ctx::from_shared(self.s));
        let mut finder = stations.finder.borrow_mut();
        let list = std::ptr::from_ref(finder.get_stations(c));
        let source = crate::cargo_storage::Source {
            id: town.id.0,
            source_type: crate::source_type::Source::TOWN,
        };
        // SAFETY: The call-local finder's set outlives the delivery and no
        // service can reach it.
        let moved = unsafe {
            crate::station::move_goods_to_station(
                cargo,
                amount,
                source,
                list,
                CompanyID::INVALID,
                c,
            )
        };
        // SAFETY: As above; the row is still the live canonical entry.
        unsafe {
            (*supplied).history[usize::from(THIS_MONTH)].transported = (*supplied).history
                [usize::from(THIS_MONTH)]
            .transported
            .wrapping_add(moved);
        }
    }
    /// `TownGenerateCargoOriginal`: generate cargo for a house using the original algorithm.
    fn town_generate_cargo_original(
        &self,
        town: TownRef,
        tpe: TownProductionEffect,
        rate: u8,
        stations: &StationFinder<'a>,
    ) {
        let mut at = 0;
        loop {
            let cs = crate::cargo_registry::production(tpe.0, at);
            if cs.cargo == CargoType::INVALID_CARGO.0 {
                break;
            }
            at += 1;
            let r = self.s.random();
            if gb(r, 0, 8) < u32::from(rate) {
                witness::hit(W_ORIGINAL);
                let cargo_type = CargoType(cs.cargo);
                let amt =
                    (gb(r, 0, 8) * u32::from(cs.multiplier) / TOWN_PRODUCTION_DIVISOR) / 8 + 1;
                self.town_generate_cargo(town, cargo_type, amt, stations, true);
            }
        }
    }
    /// `TownGenerateCargoBinomial`: generate cargo for a house using the binomial algorithm.
    fn town_generate_cargo_binomial(
        &self,
        town: TownRef,
        tpe: TownProductionEffect,
        rate: u8,
        stations: &StationFinder<'a>,
    ) {
        let mut at = 0;
        loop {
            let cs = crate::cargo_registry::production(tpe.0, at);
            if cs.cargo == CargoType::INVALID_CARGO.0 {
                break;
            }
            at += 1;
            let cargo_type = CargoType(cs.cargo);
            let r = self.s.random();
            // Make a bitmask with up to 32 bits set, one for each potential pax.
            let genmax = u32::from(rate).div_ceil(8);
            let genmask = if genmax >= 32 {
                u32::MAX
            } else {
                (1 << genmax) - 1
            };
            witness::hit(W_BINOMIAL);
            // Mask random value by potential pax and count number of actual pax.
            let amt =
                (r & genmask).count_ones() * u32::from(cs.multiplier) / TOWN_PRODUCTION_DIVISOR;
            self.town_generate_cargo(town, cargo_type, amt, stations, true);
        }
    }
    /// `TileLoop_Town`: periodic tick handler for the tiles of a town.
    fn tile_loop_town(&self, tile: TileIndex) {
        let entry = (self.h.entry)(tile.0);
        let t = entry.town;
        let house_id = entry.house;
        // NewHouseTileLoop returns false if Callback 21 succeeded, i.e. the house
        // doesn't exist any more, so don't continue here.
        if house_id >= NEW_HOUSE_OFFSET && !self.new_house_tile_loop(t, tile, house_id) {
            return;
        }
        // NewGRF callbacks finish before the source observes completion/age.
        let rec = (self.h.loop_tile)(tile.0);
        if !rec.completed {
            // Construction is not completed, so we advance a construction stage.
            self.advance_house_construction(t, tile, house_id);
            return;
        }
        let hs = (self.h.loop_spec)(house_id);
        let flags = BuildingFlags(hs.flags);
        // If the lift has a destination, it is already an animated tile.
        if flags.test(BuildingFlag::IsAnimated)
            && house_id < NEW_HOUSE_OFFSET
            && !rec.lift_has_destination
            && chance16_i(1, 2, self.s.random())
        {
            crate::animated_tile::openttd_rust_animation_add(tile.0, true);
        }
        let r = self.s.random();
        let stations = StationFinder::new(tile);
        if HouseCallbackMasks(hs.callbacks).test(HouseCallbackMask::ProduceCargo) {
            for i in 0..256 {
                // SAFETY: Live town; no owner borrow spans the callback.
                let callback = unsafe {
                    let id = CallbackID::CBID_HOUSE_PRODUCE_CARGO;
                    super::resolver::get_house_callback(id, i, r, house_id, t.shell, tile, false)
                };
                if callback == CALLBACK_FAILED || callback == CALLBACK_HOUSEPRODCARGO_END {
                    break;
                }
                let file = (self.h.cargo_file)(house_id);
                let cargo = crate::cargo_registry::get_cargo_translation(
                    gb(u32::from(callback), 8, 7) as u8,
                    file,
                    false,
                );
                if !is_valid_cargo_type(cargo.0) {
                    continue;
                }
                let amt = gb(u32::from(callback), 0, 8);
                if amt == 0 {
                    continue;
                }
                // NewGRF-supplied town cargos are not affected by recessions.
                self.town_generate_cargo(t, cargo, amt, &stations, false);
            }
        } else {
            let pax = TownProductionEffect::TPE_PASSENGERS;
            let mail = TownProductionEffect::TPE_MAIL;
            match TownCargoGenMode((self.h.cargogen_mode)()) {
                TownCargoGenMode::TCGM_ORIGINAL => {
                    // Original (quadratic) cargo generation algorithm.
                    self.town_generate_cargo_original(t, pax, hs.population, &stations);
                    self.town_generate_cargo_original(t, mail, hs.mail, &stations);
                }
                TownCargoGenMode::TCGM_BITCOUNT => {
                    // Binomial distribution per tick, by a series of coin flips.
                    // Reduce generation rate to a 1/4, using tile bits to spread out distribution.
                    // As tick counter is incremented by 256 between each call, we ignore the lower 8 bits.
                    if gb((self.s.tick_counter)() as u32, 8, 2) == gb(tile.0, 0, 2) {
                        self.town_generate_cargo_binomial(t, pax, hs.population, &stations);
                        self.town_generate_cargo_binomial(t, mail, hs.mail, &stations);
                    }
                }
                _ => unreachable!("NOT_REACHED"),
            }
        }
        let cur_company = (self.l.exchange_company)(u32::from(CompanyID::TOWN.0));
        // SAFETY: Immediate owner reads/writes in source short-circuit order;
        // each access ends before the next service.
        let growing = TownFlags(unsafe { (*t.state).growth.flags }).test(TownFlag::IsGrowing);
        if flags.any(BuildingFlags::BUILDING_HAS_1_TILE)
            && growing
            && self.can_delete_house(t, tile, house_id)
            && rec.age >= hs.minimum_life
            && unsafe {
                let rebuild = &raw mut (*t.state).simulation.time_until_rebuild;
                *rebuild = (*rebuild).wrapping_sub(1);
                *rebuild == 0
            }
        {
            witness::hit(W_REBUILD);
            // SAFETY: Immediate owner write before clearing services.
            unsafe { (*t.state).simulation.time_until_rebuild = gb(r, 16, 8) as u16 + 192 };
            self.clear_town_house(t, tile, house_id);
            // Rebuild with another house?
            if gb(r, 24, 8) >= 12 {
                witness::hit(W_REPLACE);
                let mut tile = tile;
                // If we are multi-tile houses, make sure to replace the house
                // closest to city center. If we do not do this, houses tend to
                // wander away from roads and other houses.
                if flags.any(BuildingFlags::BUILDING_HAS_2_TILES) {
                    // House tiles are always the most north tile. Move the new
                    // house to the south if we are north of the city center.
                    let map = self.map();
                    // SAFETY: Immediate copied owner read.
                    let xy = unsafe { super::position(t.state) };
                    // `TileIndexToTileIndexDiffC(t->xy, tile)`, clamped to 0..=1.
                    let grid_pos = map.tile_index_to_tile_index_diff_c(xy, tile);
                    let x = i32::from(grid_pos.x.clamp(0, 1));
                    let y = i32::from(grid_pos.y.clamp(0, 1));
                    if flags.test(BuildingFlag::Size2x2) {
                        tile = self.tile_add_xy(tile, x, y);
                    } else if flags.test(BuildingFlag::Size1x2) {
                        tile = self.tile_add_xy(tile, 0, y);
                    } else if flags.test(BuildingFlag::Size2x1) {
                        tile = self.tile_add_xy(tile, x, 0);
                    }
                }
                let mut modes = TownExpandModes::from_element(TownExpandMode::Buildings);
                if unsafe { self.s.settings().allow_town_roads.read() } {
                    modes = modes.with(TownExpandModes::from_element(TownExpandMode::Roads));
                }
                self.engine(t).try_build_town_house(t.id, tile, modes);
            }
        }
        (self.l.exchange_company)(cur_company);
    }
    /// `AnimateTile_Town`: animate a tile for a town. Only certain houses can be
    /// animated; the `NewGRF` house animation (`AnimationBase`) supersedes builtin lifts.
    fn animate_tile_town(&self, tile: TileIndex) {
        let house = self.get_house_type(tile);
        if house >= NEW_HOUSE_OFFSET {
            crate::animated_tile::openttd_rust_animate_house(tile.0);
            return;
        }
        if (self.s.tick_counter)() & 3 != 0 {
            return;
        }
        // If the house is not one with a lift anymore, then stop this animating.
        // Not exactly sure when this happens, but probably when a house changes.
        // Before this was just a return...so it'd leak animated tiles..
        // That bug seems to have been here since day 1??
        if !self.building_flags(house).test(BuildingFlag::IsAnimated) {
            crate::animated_tile::openttd_rust_animation_delete(tile.0, false);
            return;
        }
        let rec = (self.h.lift)(tile.0);
        if !rec.has_destination {
            witness::hit(W_LIFT_START);
            let mut i;
            // Building has 6 floors, number 0 .. 6, where 1 is illegal.
            // This is due to the fact that the first floor is, in the graphics,
            //  the height of 2 'normal' floors.
            // Furthermore, there are 6 lift positions from floor N (incl) to floor N + 1 (excl)
            loop {
                i = self.s.random_range(7);
                if !(i == 1 || i * 6 == u32::from(rec.position)) {
                    break;
                }
            }
            (self.h.set_lift_destination)(tile.0, i as u8);
        }
        let lift = (self.h.lift)(tile.0);
        let mut pos = i32::from(lift.position);
        let dest = i32::from(lift.destination) * 6;
        pos += if pos < dest { 1 } else { -1 };
        (self.h.set_lift_position)(tile.0, pos as u8);
        if pos == dest {
            witness::hit(W_LIFT_STOP);
            (self.h.halt_lift)(tile.0);
            crate::animated_tile::openttd_rust_animation_delete(tile.0, false);
        }
        self.mark_tile_dirty_by_tile(tile);
    }
    /// `ClearTile_Town`: callback function to clear a house tile.
    fn clear_tile_town(&self, result: *mut CommandCost, tile: TileIndex, flags: DoCommandFlags) {
        let c = self.costs();
        if flags.test(DoCommandFlag::Auto) {
            witness::hit(W_CLEAR_AUTO);
            (c.error)(result, self.h.errors.demolish);
            return;
        }
        let entry = (self.h.entry)(tile.0);
        let house = entry.house;
        let t = entry.town;
        if !self.can_delete_house(t, tile, house) {
            witness::hit(W_PROTECTED);
            (c.error)(result, self.h.errors.protected);
            return;
        }
        let hs = (self.h.removal)(house);
        (c.expense_money)(
            result,
            EXPENSES_CONSTRUCTION,
            super::policy::get_removal_cost(hs.factor),
        );
        let rating = i32::from(hs.rating);
        let company = CompanyID((self.s.current_company)());
        let authority = self.authority();
        if (self.s.company_valid)(company.0)
            && !(authority.bulldozer)()
            && !flags.test(DoCommandFlag::NoTestTownRating)
        {
            // NewGRFs can add indestructible houses.
            if rating > RATING_MAXIMUM {
                (c.error)(result, self.h.errors.protected);
                return;
            }
            // If town authority controls removal, check the company's rating.
            // SAFETY: Valid company; immediate canonical (not scratch) rating read.
            let current =
                i32::from(unsafe { (*t.state).simulation.ratings[usize::from(company.0)] });
            if rating > current
                && TownCouncilAttitudes((authority.council_tolerance)())
                    != TownCouncilAttitudes::TOWN_COUNCIL_PERMISSIVE
            {
                witness::hit(W_CLEAR_REFUSED);
                (c.error_param)(result, self.h.errors.refuses, u64::from(t.id.0));
                return;
            }
        }
        // SAFETY: Static tables and live owner; the shell address keys scratch ratings.
        unsafe {
            super::change_town_rating(
                self.l,
                self.s,
                super::RatedTown {
                    state: t.state,
                    id: t.id,
                    shell: t.shell,
                },
                super::RatingChange {
                    company,
                    add: -rating,
                    maximum: RATING_HOUSE_MINIMUM,
                    flags,
                    bulldozer: (authority.bulldozer)(),
                },
            );
        }
        if flags.test(DoCommandFlag::Execute) {
            witness::hit(W_CLEAR_EXECUTE);
            self.clear_town_house(t, tile, house);
        }
    }
    /// `TerraformTile_Town`.
    fn terraform_tile_town(
        &self,
        result: *mut CommandCost,
        tile: TileIndex,
        flags: DoCommandFlags,
        z_new: i32,
        tileh_new: Slope,
    ) {
        if (self.s.autoslope)() {
            let mut house = self.get_house_type(tile);
            // Modifies house to the ID of the north tile.
            self.get_house_north_part(&mut house);
            // Here we differ from TTDP by checking BuildingFlag::NotSloped.
            if !self.building_flags(house).test(BuildingFlag::NotSloped)
                && !is_steep_slope(tileh_new.0)
                && (self.s.tile_max_z)(tile.0) == z_new + get_slope_max_z(tileh_new.0)
            {
                let mut allow_terraform = true;
                // Call the autosloping callback per tile, not for the whole building at once.
                let house = self.get_house_type(tile);
                if self.callback_mask(house).test(HouseCallbackMask::Autoslope) {
                    let callback = CallbackID::CBID_HOUSE_AUTOSLOPE;
                    let town = (self.h.town)(tile.0);
                    // If the callback fails, allow autoslope.
                    // SAFETY: Live town; no owner borrow spans the callback.
                    let res = unsafe {
                        super::resolver::get_house_callback(
                            callback, 0, 0, house, town.shell, tile, false,
                        )
                    };
                    if res != CALLBACK_FAILED
                        && (self.s.grf_boolean_callback)(
                            (self.h.cargo_file)(house),
                            callback.0,
                            res,
                        )
                    {
                        allow_terraform = false;
                    }
                }
                if allow_terraform {
                    witness::hit(W_AUTOSLOPE);
                    (self.costs().expense_money)(
                        result,
                        EXPENSES_CONSTRUCTION,
                        crate::company::price(PR_BUILD_FOUNDATION),
                    );
                    return;
                }
            }
        }
        // SAFETY: Static command table referenced by the house table.
        crate::landscape_cmd::clear_command(unsafe { &*self.h.commands }, result, tile.0, flags.0);
    }
    /// `CmdPlaceHouse`; test clears write into the result, replaced on success.
    fn cmd_place_house(&self, call: HouseCommandCall, args: CmdPlaceHouse) {
        let tile = args.tile;
        let house = args.house;
        let replace = args.replace;
        let c = self.costs();
        if !(self.l.environment.editor)()
            && PlaceHouses((self.h.place_houses)()) == PlaceHouses::PH_FORBIDDEN
        {
            // CMD_ERROR.
            (c.error)(call.result, INVALID_STRING_ID);
            return;
        }
        if (self.s.town_count)() == 0 {
            (c.error)(call.result, self.h.errors.no_town);
            return;
        }
        if u32::from(house) >= (self.l.house_count)() || !(self.l.house_availability)(house).enabled
        {
            // CMD_ERROR.
            (c.error)(call.result, INVALID_STRING_ID);
            return;
        }
        let maxz = (self.s.tile_max_z)(tile.0);
        let building_flags = self.building_flags(house);
        let map = self.map();
        // Check each tile of a multi-tile house.
        let mut ta = crate::tilearea::TileArea::new(tile.0, 1, 1);
        if building_flags.test(BuildingFlag::Size2x2) {
            ta.add(self.tile_add_xy(tile, 1, 1).0, map);
        }
        if building_flags.test(BuildingFlag::Size2x1) {
            ta.add(map.add_diag(tile.0, DiagDirection::DIAGDIR_SW.0), map);
        }
        if building_flags.test(BuildingFlag::Size1x2) {
            ta.add(map.add_diag(tile.0, DiagDirection::DIAGDIR_SE.0), map);
        }
        // SAFETY: Static command table referenced by the house table.
        let commands = unsafe { &*self.h.commands };
        for subtile in ta.tiles(map).map(TileIndex) {
            // Houses cannot be built on steep slopes.
            let slope = Slope(crate::map_func::World::new(self.s).tile_slope(subtile.0));
            if is_steep_slope(slope.0) {
                (c.error)(call.result, self.h.errors.sloped);
                return;
            }
            // Houses cannot be built under bridges.
            if crate::map_func::World::new(self.s).bridge_above(subtile.0) {
                (c.error)(call.result, self.h.errors.bridge);
                return;
            }
            // Make sure there is no slope?
            let noslope = building_flags.test(BuildingFlag::NotSloped);
            if noslope && slope != Slope::SLOPE_FLAT {
                (c.error)(call.result, self.h.errors.flat);
                return;
            }
            // All tiles of a multi-tile house must have the same z-level.
            if (self.s.tile_max_z)(subtile.0) != maxz {
                (c.error)(call.result, self.h.errors.sloped);
                return;
            }
            // We might be replacing an existing house, otherwise check if we can clear land.
            if !(replace && TileType((self.s.tile_type)(subtile.0)) == TileType::MP_HOUSE) {
                crate::landscape_cmd::clear_command(
                    commands,
                    call.result,
                    subtile.0,
                    DoCommandFlags::from_element(DoCommandFlag::Auto)
                        .with(DoCommandFlags::from_element(DoCommandFlag::NoWater))
                        .0,
                );
                if !(c.success)(call.result) {
                    witness::hit(W_PLACE_ERROR);
                    return;
                }
            }
        }
        (c.zero)(call.result);
        if call.flags.test(DoCommandFlag::Execute) {
            witness::hit(W_PLACE);
            // If replacing, clear any existing houses first.
            if replace {
                for subtile in ta.tiles(map).map(TileIndex) {
                    if TileType((self.s.tile_type)(subtile.0)) == TileType::MP_HOUSE {
                        self.clear_town_house(
                            (self.h.town)(subtile.0),
                            subtile,
                            self.get_house_type(subtile),
                        );
                    }
                }
            }
            let town = crate::town::spatial::closest_town_from_tile(self.s, tile, u32::MAX);
            let id = TownID((self.authority().town_index)(town));
            let house_completed =
                PlaceHouses((self.h.place_houses)()) == PlaceHouses::PH_ALLOWED_CONSTRUCTED;
            // `BuildTownHouse` takes `uint8_t random_bits`.
            let random_bits = gb(self.s.random(), 0, 8) as u8;
            let t = TownRef {
                shell: town.cast_mut(),
                state: (self.s.town_owner)(town),
                id,
            };
            self.engine(t).build_town_house(
                id,
                tile,
                house,
                random_bits,
                house_completed,
                args.is_protected,
            );
        }
    }
    /// `AddAcceptedCargoSetMask`: fill the cargo acceptance array and always-accepted
    /// mask, if the cargo type is valid.
    fn add_accepted_cargo_set_mask(
        cargo: CargoType,
        amount: u32,
        acceptance: &mut CargoArray,
        always_accepted: &mut CargoTypes,
    ) {
        if !is_valid_cargo_type(cargo.0) || amount == 0 {
            return;
        }
        let slot = &mut acceptance[usize::from(cargo.0)];
        *slot = slot.wrapping_add(amount);
        set_bit64(&mut always_accepted.0, cargo.0);
    }
    /// `AddAcceptedCargoOfHouse`: determine accepted cargo for a house.
    /// `TileIndex::INVALID` with a null town is a prototype not yet built.
    fn add_accepted_cargo_of_house(
        &self,
        tile: TileIndex,
        house: HouseID,
        t: *mut TownHandle,
        acceptance: &mut CargoArray,
        always_accepted: &mut CargoTypes,
    ) {
        let hs = (self.h.acceptance)(house);
        let callback_mask = self.callback_mask(house);
        let not_yet_constructed = tile == TileIndex::INVALID;
        // Set the initial accepted cargo types.
        let mut accepts = hs.accepts.map(CargoType);
        // Check for custom accepted cargo types.
        if callback_mask.test(HouseCallbackMask::AcceptCargo) {
            let id = CallbackID::CBID_HOUSE_ACCEPT_CARGO;
            // SAFETY: The caller supplies a live town, or null for a prototype.
            let callback = unsafe {
                super::resolver::get_house_callback(id, 0, 0, house, t, tile, not_yet_constructed)
            };
            if callback != CALLBACK_FAILED {
                // Replace accepted cargo types with translated values from callback.
                let callback = u32::from(callback);
                accepts[0] = crate::cargo_registry::get_cargo_translation(
                    gb(callback, 0, 5) as u8,
                    (self.h.cargo_file)(house),
                    false,
                );
                accepts[1] = crate::cargo_registry::get_cargo_translation(
                    gb(callback, 5, 5) as u8,
                    (self.h.cargo_file)(house),
                    false,
                );
                accepts[2] = crate::cargo_registry::get_cargo_translation(
                    gb(callback, 10, 5) as u8,
                    (self.h.cargo_file)(house),
                    false,
                );
            }
        }
        // Check for custom cargo acceptance.
        if callback_mask.test(HouseCallbackMask::CargoAcceptance) {
            let id = CallbackID::CBID_HOUSE_CARGO_ACCEPTANCE;
            // SAFETY: The caller supplies a live town, or null for a prototype.
            let callback = unsafe {
                super::resolver::get_house_callback(id, 0, 0, house, t, tile, not_yet_constructed)
            };
            if callback != CALLBACK_FAILED {
                let callback = u32::from(callback);
                let (a, always) = (acceptance, always_accepted);
                Self::add_accepted_cargo_set_mask(
                    accepts[0],
                    gb(callback, 0, 4),
                    &mut *a,
                    &mut *always,
                );
                Self::add_accepted_cargo_set_mask(
                    accepts[1],
                    gb(callback, 4, 4),
                    &mut *a,
                    &mut *always,
                );
                if LandscapeType((self.l.climate)()) != LandscapeType::Temperate
                    && has_bit(callback, 12)
                {
                    // The 'S' bit indicates food instead of goods.
                    let food = crate::cargo_registry::get_cargo_type_by_label(CargoLabel::CT_FOOD);
                    Self::add_accepted_cargo_set_mask(food, gb(callback, 8, 4), a, always);
                } else {
                    Self::add_accepted_cargo_set_mask(accepts[2], gb(callback, 8, 4), a, always);
                }
                return;
            }
        }
        // No custom acceptance, so fill in with the default values.
        for (cargo, amount) in accepts.into_iter().zip(hs.amounts) {
            Self::add_accepted_cargo_set_mask(
                cargo,
                u32::from(amount),
                &mut *acceptance,
                &mut *always_accepted,
            );
        }
    }
    /// `AddAcceptedCargo_Town`.
    fn add_accepted_cargo_town(
        &self,
        tile: TileIndex,
        acceptance: &mut CargoArray,
        always: &mut CargoTypes,
    ) {
        let entry = (self.h.entry)(tile.0);
        self.add_accepted_cargo_of_house(tile, entry.house, entry.town.shell, acceptance, always);
    }
    /// `AddProducedCargo_Town`.
    fn add_produced_cargo_town(&self, tile: TileIndex, produced: &mut CargoArray) {
        let entry = (self.h.entry)(tile.0);
        let house_id = entry.house;
        let hs = (self.h.production)(house_id);
        let t = entry.town;
        if HouseCallbackMasks(hs.callbacks).test(HouseCallbackMask::ProduceCargo) {
            for i in 0..256 {
                // SAFETY: Live town; no owner borrow spans the callback.
                let callback = unsafe {
                    let id = CallbackID::CBID_HOUSE_PRODUCE_CARGO;
                    super::resolver::get_house_callback(id, i, 0, house_id, t.shell, tile, false)
                };
                if callback == CALLBACK_FAILED || callback == CALLBACK_HOUSEPRODCARGO_END {
                    break;
                }
                let file = (self.h.cargo_file)(house_id);
                let cargo = crate::cargo_registry::get_cargo_translation(
                    gb(u32::from(callback), 8, 7) as u8,
                    file,
                    false,
                );
                if !is_valid_cargo_type(cargo.0) {
                    continue;
                }
                add_one(produced, cargo);
            }
        } else {
            for (tpe, rate) in [
                (TownProductionEffect::TPE_PASSENGERS, hs.population),
                (TownProductionEffect::TPE_MAIL, hs.mail),
            ] {
                if rate == 0 {
                    continue;
                }
                let mut at = 0;
                loop {
                    let cs = crate::cargo_registry::production(tpe.0, at);
                    if cs.cargo == CargoType::INVALID_CARGO.0 {
                        break;
                    }
                    add_one(produced, CargoType(cs.cargo));
                    at += 1;
                }
            }
        }
    }
    /// `TriggerHouseAnimation_WatchedCargoAccepted`: run the watched cargo accepted
    /// callback for every tile of a house.
    fn trigger_house_animation_watched_cargo_accepted(
        &self,
        tile: TileIndex,
        trigger_cargoes: CargoTypes,
    ) {
        let mut id = self.get_house_type(tile);
        let trigger_cargoes = CargoTypes(trigger_cargoes.0 & (self.h.watched)(id));
        // None of the trigger cargoes is watched?
        if trigger_cargoes.0 == 0 {
            return;
        }
        // Same random value for all tiles of a multi-tile house.
        let r = self.s.random() as u16;
        let map = self.map();
        // Do the callback, start at northern tile.
        let north = TileIndex(
            tile.0
                .wrapping_add_signed(self.get_house_north_part(&mut id)),
        );
        let flags = self.building_flags(id);
        let town = (self.h.town)(tile.0);
        // `DoTriggerHouseAnimation_WatchedCargoAccepted`: run the callback for a single
        // house tile, with `TileIndexToTileIndexDiffC(origin, tile)`.
        let trigger = |part: TileIndex| {
            let diff = map.tile_index_to_tile_index_diff_c(tile, part);
            let (x, y) = (diff.x as u8, diff.y as u8);
            let cb_info = u32::from(r) << 16 | u32::from(y) << 8 | u32::from(x);
            // SAFETY: The entry resolved this live town shell; no owner borrow
            // spans the synchronous animation callback.
            unsafe {
                crate::animated_tile::house_change(
                    CallbackID::CBID_HOUSE_ANIMATION_TRIGGER_WATCHED_CARGO_ACCEPTED.0,
                    self.get_house_type(part),
                    town.shell,
                    part.0,
                    0,
                    cb_info,
                    trigger_cargoes.0,
                );
            }
        };
        trigger(north);
        if flags.any(BuildingFlags::BUILDING_2_TILES_Y) {
            trigger(self.tile_add_xy(north, 0, 1));
        }
        if flags.any(BuildingFlags::BUILDING_2_TILES_X) {
            trigger(self.tile_add_xy(north, 1, 0));
        }
        if flags.any(BuildingFlags::BUILDING_HAS_4_TILES) {
            trigger(self.tile_add_xy(north, 1, 1));
        }
    }
    /// `Map::Iterate`, over `Map::Size()`.
    fn map_iterate(&self) -> impl Iterator<Item = TileIndex> {
        Map::iterate(self.s.map_size()).map(TileIndex)
    }
    /// `Town::Iterate`.
    fn towns(&self, mut visit: impl FnMut(*mut State)) {
        for town in super::iterate(|from| (self.l.next_town)(from)) {
            visit((self.l.resolve_owner)(u32::from(town.0)));
        }
    }
    /// `RebuildTownCaches`: rebuild all the cached variables of towns; canonical
    /// counts, population and houses in map order.
    fn rebuild_town_caches(&self) {
        // SAFETY: Rebuild entry is serialized; no native count view survives reset.
        unsafe {
            super::resolver::initialize_building_counts(
                self.l,
                self.s,
                std::ptr::null_mut(),
                (self.l.house_count)() as usize,
            );
        };
        crate::town::spatial::rebuild_town_kdtree(self.s);
        // Reset town population and num_houses.
        // SAFETY: Immediate owner writes; no service runs while accessed.
        self.towns(|town| unsafe {
            (*town).simulation.population = 0;
            (*town).simulation.num_houses = 0;
        });
        for t in self.map_iterate() {
            if !self.is_house_tile(t) {
                continue;
            }
            let rec = (self.h.cache)(t.0);
            let house_id = rec.house;
            let town = (self.l.resolve_owner)(u32::from(rec.town));
            let class = (self.l.house_class)(house_id);
            // SAFETY: Live owner; each access ends before the next spec query.
            unsafe { super::increase_building(town, house_id, class) };
            if rec.completed {
                let population = (self.l.house_population)(house_id);
                // SAFETY: As above.
                unsafe {
                    let fields = &raw mut (*town).simulation;
                    (*fields).population = (*fields).population.wrapping_add(u32::from(population));
                }
            }
            // Increase the number of houses for every house, but only once.
            let mut house_id = house_id;
            if self.get_house_north_part(&mut house_id) == self.map().diff(0, 0).cast_signed() {
                // SAFETY: As above.
                unsafe {
                    let fields = &raw mut (*town).simulation;
                    (*fields).num_houses = (*fields).num_houses.wrapping_add(1);
                }
            }
        }
        // Update the population and num_house dependent values.
        // SAFETY: Pure owner mutation per live town.
        self.towns(|town| unsafe { update_town_radius(town) });
    }
    /// The yearly economy timer: increment house ages in `Map::Iterate` order.
    fn increment_house_ages(&self) {
        for t in self.map_iterate() {
            if !self.is_house_tile(t) {
                continue;
            }
            crate::town_map::increment_house_age(self.s, self.h, t);
        }
    }
    /// `IsTileType(tile, MP_HOUSE) && GetCleanHouseType(tile) == house_type`.
    fn is_house_part(&self, tile: TileIndex, house_type: u32) -> bool {
        self.is_house_tile(tile) && u32::from((self.h.clean_type)(tile.0)) == house_type
    }
    /// `UpdateHousesAndTowns`: check the `HouseID`s and update the town population, the
    /// number of houses per town, the town radius and the max passengers of the town.
    fn update_houses_and_towns(&self) {
        for t in self.map_iterate() {
            if !self.is_house_tile(t) {
                continue;
            }
            let house_id = (self.h.clean_type)(t.0);
            if !(self.l.house_availability)(house_id).enabled && house_id >= NEW_HOUSE_OFFSET {
                // The specs for this type of house are not available any more, so
                // replace it with the substitute original house type.
                (self.h.set_type)(t.0, unsafe {
                    crate::override_manager::openttd_rust_override_substitute(
                        crate::override_manager::house(),
                        house_id,
                    )
                });
            }
        }
        // Check for cases when a NewGRF has set a wrong house substitute type.
        for t in self.map_iterate() {
            if !self.is_house_tile(t) {
                continue;
            }
            let mut house_type = (self.h.clean_type)(t.0);
            // Modifies 'house_type'!
            let north_tile =
                TileIndex(t.0.wrapping_add_signed(self.get_house_north_part(&mut house_type)));
            let id = u32::from(house_type);
            if t == north_tile {
                let flags = self.building_flags(house_type);
                let mut valid_house = true;
                if flags.test(BuildingFlag::Size2x1) {
                    valid_house &= self.is_house_part(self.tile_diff_xy(t, 1, 0), id + 1);
                } else if flags.test(BuildingFlag::Size1x2) {
                    valid_house &= self.is_house_part(self.tile_diff_xy(t, 0, 1), id + 1);
                } else if flags.test(BuildingFlag::Size2x2) {
                    valid_house &= self.is_house_part(self.tile_diff_xy(t, 0, 1), id + 1);
                    valid_house &= self.is_house_part(self.tile_diff_xy(t, 1, 0), id + 2);
                    valid_house &= self.is_house_part(self.tile_diff_xy(t, 1, 1), id + 3);
                }
                // If not all tiles of this house are present remove the house.
                // The other tiles will get removed later in this loop because
                // their north tile is not the correct type anymore.
                if !valid_house {
                    crate::ground::clear_square(t.0, self.s);
                }
            } else if !self.is_house_part(north_tile, id) {
                // This tile should be part of a multi-tile building but the
                // north tile of this house isn't on the map.
                crate::ground::clear_square(t.0, self.s);
            }
        }
        self.rebuild_town_caches();
    }
}

/// `TriggerHouseAnimation_ConstructionStageChanged` with `first_call`, used by town
/// growth's house construction.
pub(super) fn trigger_house_animation_construction_stage_changed(
    leaves: &Leaves,
    services: &Services,
    town: TownRef,
    tile: TileIndex,
) {
    let engine = house(leaves, services);
    // The original reads GetHouseType after construction for each part: the
    // stored ID can have a NewGRF override, including on CmdPlaceHouse.
    let house_id = engine.get_house_type(tile);
    engine.trigger_house_animation_construction_stage_changed(town, tile, house_id, true);
}

// Native entries: immutable static tables outlive each synchronous call; native
// result and cargo-array storage are stack-owned by the native frame.
/// `TileLoop_Town`; the tile scheduler calls it directly, like every Rust-owned tile kind.
pub(crate) unsafe fn tile_loop(leaves: *const Leaves, services: *const Services, tile: TileIndex) {
    house(leaves, services).tile_loop_town(tile);
}
/// `AnimateTile_Town`, for the animated-tile dispatch.
pub(crate) unsafe fn animate(leaves: *const Leaves, services: *const Services, tile: TileIndex) {
    house(leaves, services).animate_tile_town(tile);
}
/// `ClearTile_Town` for the landscape clear dispatch, which passes base integers
/// until its own pass.
pub(crate) unsafe fn openttd_rust_house_clear(
    leaves: *const Leaves,
    services: *const Services,
    result: *mut CommandCost,
    tile: u32,
    flags: u16,
) {
    house(leaves, services).clear_tile_town(result, TileIndex(tile), DoCommandFlags(flags));
}
/// `TerraformTile_Town` for the landscape terraform dispatch, which passes base
/// integers until its own pass.
pub(crate) unsafe fn openttd_rust_house_terraform(
    leaves: *const Leaves,
    services: *const Services,
    result: *mut CommandCost,
    tile: u32,
    flags: u16,
    z_new: i32,
    slope: u8,
) {
    house(leaves, services).terraform_tile_town(
        result,
        TileIndex(tile),
        DoCommandFlags(flags),
        z_new,
        Slope(slope),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_place(
    leaves: *const Leaves,
    services: *const Services,
    result: *mut CommandCost,
    flags: u16,
    tile: u32,
    id: u16,
    protected: bool,
    replace: bool,
) {
    house(leaves, services).cmd_place_house(
        HouseCommandCall {
            result,
            flags: DoCommandFlags(flags),
        },
        CmdPlaceHouse {
            tile: TileIndex(tile),
            house: id,
            is_protected: protected,
            replace,
        },
    );
}
/// `AddAcceptedCargoOfHouse`: determine accepted cargo for a house.
/// # Safety
/// Live immutable tables; `town` is a live town or null for a prototype.
pub(crate) unsafe fn add_accepted_cargo_of_house(
    leaves: *const Leaves,
    services: *const Services,
    tile: TileIndex,
    id: HouseID,
    town: *mut TownHandle,
    acceptance: &mut CargoArray,
    always: &mut CargoTypes,
) {
    house(leaves, services).add_accepted_cargo_of_house(tile, id, town, acceptance, always);
}
/// `AddProducedCargo_Town`.
/// # Safety
/// Live immutable tables; `tile` is a house tile.
pub(crate) unsafe fn add_produced_cargo_town(
    leaves: *const Leaves,
    services: *const Services,
    tile: TileIndex,
    produced: &mut CargoArray,
) {
    house(leaves, services).add_produced_cargo_town(tile, produced);
}
/// `AddAcceptedCargo_Town`, for the shared tile dispatch.
/// # Safety
/// Live tables; `acceptance` names 64 values and `always` is writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_accepted_tile(
    leaves: *const Leaves,
    services: *const Services,
    tile: u32,
    acceptance: *mut u32,
    always: *mut u64,
) {
    house(leaves, services).add_accepted_cargo_town(
        TileIndex(tile),
        unsafe { &mut *acceptance.cast::<CargoArray>() },
        unsafe { &mut *always.cast::<CargoTypes>() },
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_produced(
    leaves: *const Leaves,
    services: *const Services,
    tile: u32,
    produced: *mut u32,
) {
    house(leaves, services).add_produced_cargo_town(TileIndex(tile), unsafe {
        &mut *produced.cast::<CargoArray>()
    });
}
/// `TriggerHouseAnimation_WatchedCargoAccepted`.
/// # Safety
/// Live immutable tables; `tile` is a house tile.
pub(crate) unsafe fn trigger_house_animation_watched_cargo_accepted(
    leaves: *const Leaves,
    services: *const Services,
    tile: TileIndex,
    trigger_cargoes: CargoTypes,
) {
    house(leaves, services).trigger_house_animation_watched_cargo_accepted(tile, trigger_cargoes);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_rebuild_caches(
    leaves: *const Leaves,
    services: *const Services,
) {
    house(leaves, services).rebuild_town_caches();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_update_all(
    leaves: *const Leaves,
    services: *const Services,
) {
    house(leaves, services).update_houses_and_towns();
}
/// Yearly house ageing for the economy timer.
pub(crate) unsafe fn openttd_rust_house_age_all(leaves: *const Leaves, services: *const Services) {
    house(leaves, services).increment_house_ages();
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDHouseLoopTile" ["probe"] => HouseLoopTile {
        age,
        completed,
        lift_has_destination,
    };
    "OpenTTDHouseLift" ["probe"] => HouseLift {
        position,
        destination,
        has_destination,
    };
    "OpenTTDHouseRandom" ["probe"] => HouseRandom {
        bits,
        triggers,
    };
    "OpenTTDHouseConstruction" ["probe"] => HouseConstruction {
        house,
        tick,
        completed,
    };
    "OpenTTDHouseCache" ["probe"] => HouseCache {
        house,
        town,
        completed,
    };
    "OpenTTDHouseAge" ["probe"] => HouseAge {
        age,
        completed,
    };
    "OpenTTDHouseLoopSpec" ["probe"] => HouseLoopSpec {
        callbacks,
        flags,
        population,
        mail,
        minimum_life,
    };
    "OpenTTDHouseAnimationSpec" ["probe"] => HouseAnimationSpec {
        callbacks,
        extra_flags,
    };
    "OpenTTDHouseNewSpec" ["probe"] => HouseNewSpec {
        callbacks,
        flags,
        processing_time,
    };
    "OpenTTDHouseProduction" ["probe"] => HouseProduction {
        callbacks,
        population,
        mail,
    };
    "OpenTTDHouseEntry" ["probe"] => HouseEntry {
        town,
        house,
    };
    "OpenTTDHouseRemoval" ["probe"] => HouseRemoval {
        factor,
        rating,
    };
    "OpenTTDHouseAcceptance" ["probe"] => HouseAcceptance {
        accepts,
        amounts,
    };
    "OpenTTDHouseReseed" ["probe"] => HouseReseed {
        reseed,
        used,
    };
    "OpenTTDHouseErrors" ["probe"] => HouseErrors {
        demolish,
        protected => is_protected,
        refuses,
        sloped,
        bridge,
        flat,
        no_town,
    };
    "OpenTTDHouseServices" ["probe"] => HouseServices {
        cost,
        commands,
        authority,
        entry,
        loop_tile,
        house_type,
        completed,
        protected => is_protected,
        lift,
        random,
        construction,
        cache,
        age,
        processing,
        clean_type,
        town,
        loop_spec,
        animation_spec,
        new_spec,
        production,
        sprite_groups,
        removal,
        acceptance,
        watched,
        inc_construction,
        reset_age,
        set_age,
        set_lift_destination,
        set_lift_position,
        halt_lift,
        dec_processing,
        set_processing,
        set_random_bits,
        set_random_triggers,
        set_type,

        delete_inspect,
        cargo_file,
        convert_8bit,

        scale_cargo,
        cargogen_mode,
        place_houses,
        errors,
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_accepted_prototype(
    leaves: *const Leaves,
    services: *const Services,
    id: u16,
    acceptance: *mut u32,
) {
    unsafe {
        std::ptr::write_bytes(acceptance, 0, usize::from(crate::cargo_type::NUM_CARGO));
    }
    let mut always = CargoTypes(0);
    house(leaves, services).add_accepted_cargo_of_house(
        TileIndex::INVALID,
        id,
        std::ptr::null_mut(),
        unsafe { &mut *acceptance.cast::<CargoArray>() },
        &mut always,
    );
}
