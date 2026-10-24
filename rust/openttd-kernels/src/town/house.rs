/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! House lifecycle and tile simulation: construction stages, builtin lifts,
//! cargo production and acceptance, clearing, rebuilding, placement, cache
//! rebuild and house-specific `NewGRF` trigger traversal. Packed map fields,
//! pools, specs, sprite resolvers and `AnimationBase` stay native. Every
//! service is a synchronous typed call that may reenter; owner fields are
//! accessed only immediately, never borrowed across a service.
use super::{AuthorityServices, Leaves, State, TownRef, change_population, engine, update_radius};
use crate::cargo_type::INVALID_CARGO;
use crate::command_type::DoCommandFlag;
use crate::company_type::{OWNER_NONE, OWNER_TOWN, OWNER_WATER};
use crate::economy_type::PR_BUILD_FOUNDATION;
use crate::history_type::THIS_MONTH;
use crate::newgrf_callbacks::CALLBACK_FAILED;
use crate::services::{
    CommandCost, CommandCostServices, CommandServices, Map, Services, TownHandle, chance16_i,
};
use crate::slope_func::{get_slope_max_z, is_steep_slope};
use crate::station::Station;
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{INVALID_TILE, MP_HOUSE};
use crate::witness;
use std::cell::{Cell, OnceCell};
use std::ffi::c_void;

const NEW_HOUSE_OFFSET: u16 = 110;
const PRODUCE_END: u16 = 0x20FF;
// Building flags.
const NOT_SLOPED: u8 = 1 << 1;
const SIZE_2X1: u8 = 1 << 2;
const SIZE_1X2: u8 = 1 << 3;
const SIZE_2X2: u8 = 1 << 4;
const ANIMATED: u8 = 1 << 5;
const CHURCH: u8 = 1 << 6;
const STADIUM: u8 = 1 << 7;
const HAS_1_TILE: u8 = 1 | SIZE_2X1 | SIZE_1X2 | SIZE_2X2;
const HAS_2_TILES: u8 = SIZE_2X1 | SIZE_1X2 | SIZE_2X2;
const TILES_X: u8 = SIZE_2X1 | SIZE_2X2;
const TILES_Y: u8 = SIZE_1X2 | SIZE_2X2;
// House callback mask bits and callback IDs.
const CBM_ANIM_LOOP: u16 = 1 << 2;
const CBM_ANIM_STAGE: u16 = 1 << 3;
const CBM_ACCEPTANCE: u16 = 1 << 5;
const CBM_DESTRUCTION: u16 = 1 << 7;
const CBM_ACCEPT: u16 = 1 << 8;
const CBM_PRODUCE: u16 = 1 << 9;
const CBM_DENY: u16 = 1 << 10;
const CBM_AUTOSLOPE: u16 = 1 << 12;
const CB_ANIM_LOOP: u16 = 0x1B;
const CB_ANIM_STAGE: u16 = 0x1C;
const CB_ACCEPTANCE: u16 = 0x1F;
const CB_DESTRUCTION: u16 = 0x21;
const CB_ACCEPT: u16 = 0x2A;
const CB_PRODUCE: u16 = 0x2E;
const CB_DENY: u16 = 0x143;
const CB_WATCHED: u16 = 0x148;
const CB_AUTOSLOPE: u16 = 0x14F;

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
    pub cost: i64,
    pub rating: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseAcceptance {
    pub accepts: [u8; 16],
    pub amounts: [u8; 16],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HouseReseed {
    pub reseed: u32,
    pub used: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ProductionCargo {
    pub cargo: u8,
    pub multiplier: u16,
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
    pub town_shell: extern "C" fn(u32) -> *mut TownHandle,
    pub loop_spec: extern "C" fn(u16) -> HouseLoopSpec,
    pub animation_spec: extern "C" fn(u16) -> HouseAnimationSpec,
    pub new_spec: extern "C" fn(u16) -> HouseNewSpec,
    pub production: extern "C" fn(u16) -> HouseProduction,
    pub sprite_groups: extern "C" fn(u16) -> bool,
    pub removal: extern "C" fn(u16) -> HouseRemoval,
    pub acceptance: extern "C" fn(u16) -> HouseAcceptance,
    pub watched: extern "C" fn(u16) -> u64,
    pub substitute: extern "C" fn(u16) -> u16,
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
    pub callback: unsafe extern "C" fn(u16, u32, u32, u16, *mut TownHandle, u32, bool) -> u16,
    pub cargo_translation: extern "C" fn(u16, u8) -> u8,
    pub convert_boolean: extern "C" fn(u16, u16, u16) -> bool,
    pub convert_8bit: extern "C" fn(u16, u16, u16) -> bool,
    pub resolve_reseed: unsafe extern "C" fn(u16, u32, *mut TownHandle, u8) -> HouseReseed,

    pub next_near_station: unsafe extern "C" fn(*mut TownHandle, *mut Station) -> *mut Station,
    pub erase_near_station: unsafe extern "C" fn(*mut TownHandle, *mut Station),
    pub station_catchment: unsafe extern "C" fn(*const Station, u32) -> bool,
    pub station_covers_town: unsafe extern "C" fn(*const Station, u16) -> bool,
    /// Native `StationFinder` for a 1x1 house tile area, valid until the next call.
    pub station_finder: extern "C" fn(u32) -> *mut c_void,
    pub move_goods: unsafe extern "C" fn(*mut c_void, u8, u32, u16) -> u32,
    pub production_cargo: extern "C" fn(u8, u32) -> ProductionCargo,
    pub add_cargo: unsafe extern "C" fn(*mut c_void, u8, u32),
    pub food_cargo: extern "C" fn() -> u8,
    pub scale_cargo: extern "C" fn(u32) -> u32,
    pub cargogen_mode: extern "C" fn() -> u8,
    pub place_houses: extern "C" fn() -> u8,
    pub company_human: extern "C" fn(u8) -> bool,
    pub init_counts: extern "C" fn(),
    pub errors: HouseErrors,
}

/// `TileLoop_Town`'s `StationFinder`. The original computes its station list on
/// the first `GetStations`; the native finder is likewise requested only when
/// cargo moves, then reused for the rest of the tile.
struct StationFinder {
    tile: u32,
    native: Cell<*mut c_void>,
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
impl House<'_> {
    fn map(&self) -> Map {
        *self.map.get_or_init(|| Map {
            log_x: (self.l.map_log_x)() as u8,
        })
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
    fn dirty(&self, tile: u32) {
        (self.s.mark_dirty)(tile);
    }
    /// `GetHouseNorthPart`: house IDs 0..2 are single-tile by definition.
    fn north_part(&self, house: &mut u16, map: Map) -> u32 {
        if *house >= 3 {
            let previous_flags = (self.l.house_flags)(*house - 1);
            if previous_flags & SIZE_2X1 != 0 {
                *house -= 1;
                return map.diff(-1, 0);
            } else if previous_flags & TILES_Y != 0 {
                *house -= 1;
                return map.diff(0, -1);
            } else if (self.l.house_flags)(*house - 2) & SIZE_2X2 != 0 {
                *house -= 2;
                return map.diff(-1, 0);
            } else if (self.l.house_flags)(*house - 3) & SIZE_2X2 != 0 {
                *house -= 3;
                return map.diff(-1, -1);
            }
        }
        0
    }
    /// `TriggerHouseAnimation_ConstructionStageChanged`.
    fn stage_changed(&self, town: TownRef, tile: u32, house: u16, first: bool) {
        if (self.l.house_callbacks)(house) & CBM_ANIM_STAGE != 0 {
            let random = self.s.random();
            // SAFETY: The entry resolved this live town shell; no owner borrow
            // spans the synchronous animation callback.
            unsafe {
                crate::animated_tile::house_change(
                    CB_ANIM_STAGE,
                    house,
                    town.shell,
                    tile,
                    random,
                    u32::from(first),
                    0,
                );
            }
        }
    }
    /// `CanDeleteHouse`.
    fn can_delete(&self, town: TownRef, tile: u32, house: u16) -> bool {
        let company = (self.s.current_company)();
        if (self.h.company_human)(company)
            || company == OWNER_WATER
            || company == OWNER_NONE
            || (self.l.environment.editor)()
            || (self.l.generating_world)()
        {
            return true;
        }
        if (self.l.house_callbacks)(house) & CBM_DENY != 0 {
            // SAFETY: The entry resolved the live town; no owner borrow spans resolution.
            let res = unsafe { (self.h.callback)(CB_DENY, 0, 0, house, town.shell, tile, false) };
            res == CALLBACK_FAILED || !(self.h.convert_boolean)(house, CB_DENY, res)
        } else {
            !(self.h.protected)(tile)
        }
    }
    /// `RemoveNearbyStations`: ordered set traversal; erasure keeps the successor.
    fn remove_nearby_stations(&self, town: TownRef, tile: u32, flags: u8, map: Map) {
        // SAFETY: The entry resolved the live town. The services return live
        // stations; erasure removes only set membership, leaving each shell
        // valid for upper_bound. No owner borrow spans any service.
        unsafe {
            let mut station = (self.h.next_near_station)(town.shell, std::ptr::null_mut());
            while !station.is_null() {
                let mut covers = (self.h.station_catchment)(station, tile);
                if flags & TILES_Y != 0 {
                    covers |=
                        (self.h.station_catchment)(station, tile.wrapping_add(map.diff(0, 1)));
                }
                if flags & TILES_X != 0 {
                    covers |=
                        (self.h.station_catchment)(station, tile.wrapping_add(map.diff(1, 0)));
                }
                if flags & SIZE_2X2 != 0 {
                    covers |=
                        (self.h.station_catchment)(station, tile.wrapping_add(map.diff(1, 1)));
                }
                if covers && !(self.h.station_covers_town)(station, town.id.0) {
                    witness::hit(W_STATION_REMOVED);
                    (self.h.erase_near_station)(town.shell, station);
                }
                station = (self.h.next_near_station)(town.shell, station);
            }
        }
    }
    fn clear_part(&self, town: TownRef, tile: u32, house: u16) {
        let class = (self.l.house_class)(house);
        // SAFETY: Live owner; count access ends before the map services below.
        unsafe { super::decrease_building(town.state, house, class) };
        crate::ground::clear_square(tile, self.s);
        (self.h.delete_inspect)(tile);
    }
    /// `ClearTownHouse`: population, counts and flags precede the footprint.
    fn clear_town_house(&self, town: TownRef, tile: u32, mut house: u16) {
        let map = self.map();
        let tile = tile.wrapping_add(self.north_part(&mut house, map));
        let flags = (self.l.house_flags)(house);
        if (self.h.completed)(tile) {
            // SAFETY: Live owner; presentation callbacks run after the write.
            unsafe {
                change_population(
                    self.l,
                    town.state,
                    u32::from(town.id.0),
                    -i32::from((self.l.house_population)(house)),
                );
            }
        }
        // SAFETY: Immediate owner mutations; no service runs while accessed.
        unsafe {
            let fields = &raw mut (*town.state).simulation;
            (*fields).num_houses = (*fields).num_houses.wrapping_sub(1);
            if flags & CHURCH != 0 {
                (*town.state).growth.flags &= !2;
            } else if flags & STADIUM != 0 {
                (*town.state).growth.flags &= !4;
            }
        }
        self.clear_part(town, tile, house);
        if flags & TILES_Y != 0 {
            house = house.wrapping_add(1);
            self.clear_part(town, tile.wrapping_add(map.diff(0, 1)), house);
        }
        if flags & TILES_X != 0 {
            house = house.wrapping_add(1);
            self.clear_part(town, tile.wrapping_add(map.diff(1, 0)), house);
        }
        if flags & SIZE_2X2 != 0 {
            house = house.wrapping_add(1);
            self.clear_part(town, tile.wrapping_add(map.diff(1, 1)), house);
        }
        self.remove_nearby_stations(town, tile, flags, map);
        // SAFETY: Pure immediate owner mutation, without native callbacks.
        unsafe { update_radius(town.state) };
    }
    /// `TriggerHouseAnimation_TileLoop`.
    fn animation_loop(&self, town: TownRef, tile: u32, sync: bool, random_bits: u32) {
        let house = (self.h.house_type)(tile);
        let spec = (self.h.animation_spec)(house);
        if spec.callbacks & CBM_ANIM_LOOP != 0 && (spec.extra_flags & 4 != 0) == sync {
            let param = if sync {
                (self.s.random() & 0xFFFF) | (random_bits << 16)
            } else {
                self.s.random()
            };
            // SAFETY: The entry resolved this live town shell; no owner borrow
            // spans the synchronous animation callback.
            unsafe {
                crate::animated_tile::house_change(
                    CB_ANIM_LOOP,
                    house,
                    town.shell,
                    tile,
                    param,
                    0,
                    0,
                );
            }
        }
    }
    /// `DoTriggerHouseRandomisation`, including TTDP's scope-ignoring reseed.
    fn randomise(&self, town: TownRef, tile: u32, trigger: u8, base: u8, first: bool) {
        let house = (self.h.house_type)(tile);
        if !(self.h.sprite_groups)(house) {
            return;
        }
        let rec = (self.h.random)(tile);
        let waiting = rec.triggers | (1 << trigger);
        (self.h.set_random_triggers)(tile, waiting);
        // SAFETY: The entry resolved the live town; no owner borrow spans resolution.
        let reseed = unsafe { (self.h.resolve_reseed)(house, tile, town.shell, waiting) };
        (self.h.set_random_triggers)(tile, waiting & !reseed.used);
        let new_random = self.s.random() as u8;
        let mut bits = (u32::from(rec.bits) & !reseed.reseed) as u8;
        bits |= (u32::from(if first { new_random } else { base }) & reseed.reseed) as u8;
        (self.h.set_random_bits)(tile, bits);
        if trigger == 1 {
            if !first {
                self.dirty(tile);
                return;
            }
            let map = self.map();
            let flags = (self.l.house_flags)(house);
            if flags & TILES_Y != 0 {
                self.randomise(
                    town,
                    tile.wrapping_add(map.diff(0, 1)),
                    trigger,
                    bits,
                    false,
                );
            }
            if flags & TILES_X != 0 {
                self.randomise(
                    town,
                    tile.wrapping_add(map.diff(1, 0)),
                    trigger,
                    bits,
                    false,
                );
            }
            if flags & SIZE_2X2 != 0 {
                self.randomise(
                    town,
                    tile.wrapping_add(map.diff(1, 1)),
                    trigger,
                    bits,
                    false,
                );
            }
        }
    }
    /// `NewHouseTileLoop`; false when callback 21 destroyed the house.
    fn new_tile_loop(&self, town: TownRef, tile: u32, house: u16) -> bool {
        if (self.h.processing)(tile) > 0 {
            (self.h.dec_processing)(tile);
            return true;
        }
        let spec = (self.h.new_spec)(house);
        let flags = spec.flags;
        self.randomise(town, tile, 0, 0, true);
        if flags & HAS_1_TILE != 0 {
            self.randomise(town, tile, 1, 0, true);
        }
        self.animation_loop(town, tile, false, 0);
        if flags & HAS_1_TILE != 0 {
            let map = self.map();
            let random = self.s.random() & 0xFFFF;
            self.animation_loop(town, tile, true, random);
            if flags & TILES_Y != 0 {
                self.animation_loop(town, tile.wrapping_add(map.diff(0, 1)), true, random);
            }
            if flags & TILES_X != 0 {
                self.animation_loop(town, tile.wrapping_add(map.diff(1, 0)), true, random);
            }
            if flags & SIZE_2X2 != 0 {
                self.animation_loop(town, tile.wrapping_add(map.diff(1, 1)), true, random);
            }
        }
        if spec.callbacks & CBM_DESTRUCTION != 0 {
            let house = (self.h.house_type)(tile);
            // SAFETY: Live town; the destruction command follows resolution.
            let res =
                unsafe { (self.h.callback)(CB_DESTRUCTION, 0, 0, house, town.shell, tile, false) };
            if res != CALLBACK_FAILED && (self.h.convert_8bit)(house, CB_DESTRUCTION, res) {
                self.clear_town_house(town, tile, house);
                return false;
            }
        }
        (self.h.set_processing)(tile, spec.processing_time);
        self.dirty(tile);
        true
    }
    fn advance_single(&self, town: TownRef, tile: u32) {
        (self.h.inc_construction)(tile);
        let rec = (self.h.construction)(tile);
        if rec.tick != 0 {
            return;
        }
        witness::hit(W_STAGE);
        self.stage_changed(town, tile, rec.house, false);
        // Animation callbacks change frames only; completion is unchanged.
        if rec.completed {
            witness::hit(W_COMPLETE);
            let population = (self.l.house_population)(rec.house);
            // SAFETY: Live owner; presentation callbacks follow the write.
            unsafe {
                change_population(
                    self.l,
                    town.state,
                    u32::from(town.id.0),
                    i32::from(population),
                );
            }
            (self.h.reset_age)(tile);
        }
        self.dirty(tile);
    }
    fn advance_construction(&self, town: TownRef, tile: u32, house: u16) {
        let flags = (self.l.house_flags)(house);
        let map = self.map();
        if flags & HAS_1_TILE != 0 {
            self.advance_single(town, tile);
        }
        if flags & TILES_Y != 0 {
            self.advance_single(town, tile.wrapping_add(map.diff(0, 1)));
        }
        if flags & TILES_X != 0 {
            self.advance_single(town, tile.wrapping_add(map.diff(1, 0)));
        }
        if flags & SIZE_2X2 != 0 {
            self.advance_single(town, tile.wrapping_add(map.diff(1, 1)));
        }
    }
    /// `TownGenerateCargo`: production precedes the delivery; transported follows.
    fn generate(
        &self,
        town: TownRef,
        cargo: u8,
        mut amount: u32,
        stations: &StationFinder,
        recession: bool,
    ) {
        if amount == 0 {
            return;
        }
        if recession && (self.s.recession)() {
            witness::hit(W_RECESSION);
            amount = (amount + 1) >> 1;
        }
        amount = (self.h.scale_cargo)(amount);
        if amount == 0 {
            return;
        }
        // SAFETY: Live owner. Like the source reference, the row pointer spans
        // only the station delivery, which never touches town supplied rows.
        let row = unsafe { super::supplied_entry(town.state, cargo) }.entry;
        unsafe {
            (*row).history[usize::from(THIS_MONTH)].production = (*row).history
                [usize::from(THIS_MONTH)]
            .production
            .wrapping_add(amount);
        }
        let mut finder = stations.native.get();
        if finder.is_null() {
            finder = (self.h.station_finder)(stations.tile);
            stations.native.set(finder);
        }
        // SAFETY: The native finder stays live until the next tile requests one.
        let moved = unsafe { (self.h.move_goods)(finder, cargo, amount, town.id.0) };
        // SAFETY: As above; the row is still the live canonical entry.
        unsafe {
            (*row).history[usize::from(THIS_MONTH)].transported = (*row).history
                [usize::from(THIS_MONTH)]
            .transported
            .wrapping_add(moved);
        }
    }
    fn generate_original(&self, town: TownRef, effect: u8, rate: u8, stations: &StationFinder) {
        let mut at = 0;
        loop {
            let cargo = (self.h.production_cargo)(effect, at);
            if cargo.cargo == INVALID_CARGO {
                break;
            }
            at += 1;
            let r = self.s.random();
            if r & 0xFF < u32::from(rate) {
                witness::hit(W_ORIGINAL);
                let amount = (r & 0xFF) * u32::from(cargo.multiplier) / 256 / 8 + 1;
                self.generate(town, cargo.cargo, amount, stations, true);
            }
        }
    }
    fn generate_binomial(&self, town: TownRef, effect: u8, rate: u8, stations: &StationFinder) {
        let mut at = 0;
        loop {
            let cargo = (self.h.production_cargo)(effect, at);
            if cargo.cargo == INVALID_CARGO {
                break;
            }
            at += 1;
            let r = self.s.random();
            let genmax = u32::from(rate).div_ceil(8);
            let genmask = if genmax >= 32 {
                u32::MAX
            } else {
                (1 << genmax) - 1
            };
            witness::hit(W_BINOMIAL);
            let amount = (r & genmask).count_ones() * u32::from(cargo.multiplier) / 256;
            self.generate(town, cargo.cargo, amount, stations, true);
        }
    }
    /// `TileLoop_Town`.
    fn tile_loop(&self, tile: u32) {
        let entry = (self.h.entry)(tile);
        let town = entry.town;
        let house = entry.house;
        if house >= NEW_HOUSE_OFFSET && !self.new_tile_loop(town, tile, house) {
            return;
        }
        // NewGRF callbacks finish before the source observes completion/age.
        let rec = (self.h.loop_tile)(tile);
        if !rec.completed {
            self.advance_construction(town, tile, house);
            return;
        }
        let spec = (self.h.loop_spec)(house);
        let flags = spec.flags;
        if flags & ANIMATED != 0
            && house < NEW_HOUSE_OFFSET
            && !rec.lift_has_destination
            && chance16_i(1, 2, self.s.random())
        {
            crate::animated_tile::openttd_rust_animation_add(tile, true);
        }
        let r = self.s.random();
        let stations = StationFinder {
            tile,
            native: Cell::new(std::ptr::null_mut()),
        };
        if spec.callbacks & CBM_PRODUCE != 0 {
            for i in 0..256 {
                // SAFETY: Live town; no owner borrow spans the callback.
                let cb =
                    unsafe { (self.h.callback)(CB_PRODUCE, i, r, house, town.shell, tile, false) };
                if cb == CALLBACK_FAILED || cb == PRODUCE_END {
                    break;
                }
                let cargo = (self.h.cargo_translation)(house, ((cb >> 8) & 0x7F) as u8);
                if cargo == INVALID_CARGO {
                    continue;
                }
                let amount = u32::from(cb & 0xFF);
                if amount == 0 {
                    continue;
                }
                self.generate(town, cargo, amount, &stations, false);
            }
        } else {
            match (self.h.cargogen_mode)() {
                0 => {
                    self.generate_original(town, 1, spec.population, &stations);
                    self.generate_original(town, 2, spec.mail, &stations);
                }
                1 => {
                    if ((self.s.tick_counter)() >> 8) & 3 == u64::from(tile & 3) {
                        self.generate_binomial(town, 1, spec.population, &stations);
                        self.generate_binomial(town, 2, spec.mail, &stations);
                    }
                }
                _ => unreachable!("NOT_REACHED"),
            }
        }
        let company = (self.l.exchange_company)(u32::from(OWNER_TOWN));
        // SAFETY: Immediate owner reads/writes in source short-circuit order;
        // each access ends before the next service.
        let growing = unsafe { (*town.state).growth.flags } & 1 != 0;
        if flags & HAS_1_TILE != 0
            && growing
            && self.can_delete(town, tile, house)
            && rec.age >= spec.minimum_life
            && unsafe {
                let rebuild = &raw mut (*town.state).simulation.time_until_rebuild;
                *rebuild = (*rebuild).wrapping_sub(1);
                *rebuild == 0
            }
        {
            witness::hit(W_REBUILD);
            // SAFETY: Immediate owner write before clearing services.
            unsafe {
                (*town.state).simulation.time_until_rebuild = ((r >> 16) & 0xFF) as u16 + 192;
            };
            self.clear_town_house(town, tile, house);
            if r >> 24 >= 12 {
                witness::hit(W_REPLACE);
                let mut tile = tile;
                if flags & HAS_2_TILES != 0 {
                    let map = self.map();
                    // SAFETY: Immediate copied owner read.
                    let center = unsafe { super::position(town.state) };
                    let x = i32::from(
                        (map.tile_x(center).wrapping_sub(map.tile_x(tile)) as i16).clamp(0, 1),
                    );
                    let y = i32::from(
                        (map.tile_y(center).wrapping_sub(map.tile_y(tile)) as i16).clamp(0, 1),
                    );
                    if flags & SIZE_2X2 != 0 {
                        tile = tile.wrapping_add(map.diff(x, y));
                    } else if flags & SIZE_1X2 != 0 {
                        tile = tile.wrapping_add(map.diff(0, y));
                    } else if flags & SIZE_2X1 != 0 {
                        tile = tile.wrapping_add(map.diff(x, 0));
                    }
                }
                let modes = 1 | if (self.l.allow_roads)() { 2 } else { 0 };
                let id = u32::from(town.id.0);
                self.engine(town).try_house(id, tile, modes);
            }
        }
        (self.l.exchange_company)(company);
    }
    /// `AnimateTile_Town`: builtin lifts; `NewGRF` houses use `AnimationBase`.
    fn animate(&self, tile: u32) {
        let house = (self.h.house_type)(tile);
        if house >= NEW_HOUSE_OFFSET {
            crate::animated_tile::openttd_rust_animate_house(tile);
            return;
        }
        if (self.s.tick_counter)() & 3 != 0 {
            return;
        }
        if (self.l.house_flags)(house) & ANIMATED == 0 {
            crate::animated_tile::openttd_rust_animation_delete(tile, false);
            return;
        }
        let rec = (self.h.lift)(tile);
        let mut destination = rec.destination;
        if !rec.has_destination {
            witness::hit(W_LIFT_START);
            let mut i;
            loop {
                i = self.s.random_range(7);
                if !(i == 1 || i * 6 == u32::from(rec.position)) {
                    break;
                }
            }
            (self.h.set_lift_destination)(tile, i as u8);
            destination = i as u8 & 7;
        }
        let mut pos = i32::from(rec.position);
        let dest = i32::from(destination) * 6;
        pos += if pos < dest { 1 } else { -1 };
        (self.h.set_lift_position)(tile, pos as u8);
        if pos == dest {
            witness::hit(W_LIFT_STOP);
            (self.h.halt_lift)(tile);
            crate::animated_tile::openttd_rust_animation_delete(tile, false);
        }
        self.dirty(tile);
    }
    /// `ClearTile_Town`.
    fn clear(&self, result: *mut CommandCost, tile: u32, flags: u16) {
        let c = self.costs();
        if flags & DoCommandFlag::AUTO != 0 {
            witness::hit(W_CLEAR_AUTO);
            (c.error)(result, self.h.errors.demolish);
            return;
        }
        let entry = (self.h.entry)(tile);
        let house = entry.house;
        let town = entry.town;
        if !self.can_delete(town, tile, house) {
            witness::hit(W_PROTECTED);
            (c.error)(result, self.h.errors.protected);
            return;
        }
        let removal = (self.h.removal)(house);
        (c.expense_money)(result, 0, removal.cost);
        let rating = i32::from(removal.rating);
        let company = (self.s.current_company)();
        let authority = self.authority();
        if (self.s.company_valid)(company)
            && !(authority.bulldozer)()
            && flags & DoCommandFlag::NO_TEST_TOWN_RATING == 0
        {
            if rating > 1000 {
                (c.error)(result, self.h.errors.protected);
                return;
            }
            // SAFETY: Valid company; immediate canonical (not scratch) rating read.
            let current =
                i32::from(unsafe { (*town.state).simulation.ratings[usize::from(company)] });
            if rating > current && (authority.council_tolerance)() != 3 {
                witness::hit(W_CLEAR_REFUSED);
                (c.error_param)(result, self.h.errors.refuses, u64::from(town.id.0));
                return;
            }
        }
        // SAFETY: Static tables and live owner; the shell address keys scratch ratings.
        unsafe {
            super::openttd_rust_town_change_rating(
                self.l,
                self.s,
                town.state,
                u32::from(town.id.0),
                town.shell as usize,
                company,
                -rating,
                -1000,
                flags & DoCommandFlag::NO_MODIFY_TOWN_RATING != 0,
                (authority.bulldozer)(),
            );
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            witness::hit(W_CLEAR_EXECUTE);
            self.clear_town_house(town, tile, house);
        }
    }
    /// `TerraformTile_Town`.
    fn terraform(&self, result: *mut CommandCost, tile: u32, flags: u16, z_new: i32, slope: u8) {
        if (self.s.autoslope)() {
            let house = (self.h.house_type)(tile);
            let mut north = house;
            self.north_part(&mut north, self.map());
            if (self.l.house_flags)(north) & NOT_SLOPED == 0
                && !is_steep_slope(slope)
                && (self.s.tile_max_z)(tile) == z_new + get_slope_max_z(slope)
            {
                let mut allow = true;
                if (self.l.house_callbacks)(house) & CBM_AUTOSLOPE != 0 {
                    let town = (self.h.town)(tile);
                    // SAFETY: Live town; no owner borrow spans the callback.
                    let res = unsafe {
                        (self.h.callback)(CB_AUTOSLOPE, 0, 0, house, town.shell, tile, false)
                    };
                    if res != CALLBACK_FAILED && (self.h.convert_boolean)(house, CB_AUTOSLOPE, res)
                    {
                        allow = false;
                    }
                }
                if allow {
                    witness::hit(W_AUTOSLOPE);
                    (self.costs().expense_money)(
                        result,
                        0,
                        crate::company::price(PR_BUILD_FOUNDATION),
                    );
                    return;
                }
            }
        }
        // SAFETY: Static command table referenced by the house table.
        crate::landscape_cmd::clear_command(unsafe { &*self.h.commands }, result, tile, flags);
    }
    /// `CmdPlaceHouse`; test clears write into the result, replaced on success.
    fn place(
        &self,
        result: *mut CommandCost,
        flags: u16,
        tile: u32,
        house: u16,
        protected: bool,
        replace: bool,
    ) {
        let c = self.costs();
        if !(self.l.environment.editor)() && (self.h.place_houses)() == 0 {
            (c.error)(result, INVALID_STRING_ID);
            return;
        }
        if (self.s.town_count)() == 0 {
            (c.error)(result, self.h.errors.no_town);
            return;
        }
        if u32::from(house) >= (self.l.house_count)() || !(self.l.house_availability)(house).enabled
        {
            (c.error)(result, INVALID_STRING_ID);
            return;
        }
        let maxz = (self.s.tile_max_z)(tile);
        let size = (self.l.house_flags)(house);
        let map = self.map();
        // OrthogonalTileArea::Add over the footprint, then its iteration order.
        let (mut sx, mut sy) = (map.tile_x(tile), map.tile_y(tile));
        let (mut ex, mut ey) = (sx, sy);
        for (enabled, x, y) in [(SIZE_2X2, 1, 1), (SIZE_2X1, 1, 0), (SIZE_1X2, 0, 1)] {
            if size & enabled != 0 {
                let add = tile.wrapping_add(map.diff(x, y));
                let (ax, ay) = (map.tile_x(add), map.tile_y(add));
                (sx, sy, ex, ey) = (ax.min(sx), ay.min(sy), ax.max(ex), ay.max(ey));
            }
        }
        let (w, h) = (
            ex.wrapping_sub(sx).wrapping_add(1) as u16,
            ey.wrapping_sub(sy).wrapping_add(1) as u16,
        );
        let base = (sy << map.log_x).wrapping_add(sx);
        let area = (0..u32::from(h)).flat_map(|y| (0..u32::from(w)).map(move |x| (x, y)));
        let commands = unsafe { &*self.h.commands };
        for (x, y) in area.clone() {
            let subtile = base.wrapping_add(map.diff(x as i32, y as i32));
            let slope = (self.s.tile_slope)(subtile);
            if is_steep_slope(slope) {
                (c.error)(result, self.h.errors.sloped);
                return;
            }
            if (self.s.bridge_above)(subtile) {
                (c.error)(result, self.h.errors.bridge);
                return;
            }
            if size & NOT_SLOPED != 0 && slope != 0 {
                (c.error)(result, self.h.errors.flat);
                return;
            }
            if (self.s.tile_max_z)(subtile) != maxz {
                (c.error)(result, self.h.errors.sloped);
                return;
            }
            if !(replace && (self.s.tile_type)(subtile) == MP_HOUSE) {
                crate::landscape_cmd::clear_command(
                    commands,
                    result,
                    subtile,
                    DoCommandFlag::AUTO | DoCommandFlag::NO_WATER,
                );
                if !(c.success)(result) {
                    witness::hit(W_PLACE_ERROR);
                    return;
                }
            }
        }
        (c.zero)(result);
        if flags & DoCommandFlag::EXECUTE != 0 {
            witness::hit(W_PLACE);
            if replace {
                for (x, y) in area {
                    let subtile = base.wrapping_add(map.diff(x as i32, y as i32));
                    if (self.s.tile_type)(subtile) == MP_HOUSE {
                        self.clear_town_house(
                            (self.h.town)(subtile),
                            subtile,
                            (self.h.house_type)(subtile),
                        );
                    }
                }
            }
            let town = (self.s.closest_town)(tile, u32::MAX);
            let id = u32::from((self.authority().town_index)(town));
            let completed = (self.h.place_houses)() == 2;
            let random = self.s.random() & 0xFF;
            let town = TownRef {
                shell: town.cast_mut(),
                state: (self.s.town_owner)(town),
                id: super::TownId(id as u16),
            };
            self.engine(town)
                .build_house(id, tile, u32::from(house), random, completed, protected);
        }
    }
    fn add_acceptance(&self, ctx: *mut c_void, always: *mut u64, cargo: u8, amount: u32) {
        if cargo == INVALID_CARGO || amount == 0 {
            return;
        }
        // SAFETY: Native stack-owned CargoArray outlives this synchronous entry.
        unsafe { (self.h.add_cargo)(ctx, cargo, amount) };
        // SAFETY: Native stack-owned mask outlives this synchronous entry.
        unsafe { *always |= 1 << cargo };
    }
    /// `AddAcceptedCargoOfHouse`; `INVALID_TILE` with a null town is a prototype.
    fn accepted(
        &self,
        tile: u32,
        house: u16,
        town: *mut TownHandle,
        ctx: *mut c_void,
        always: *mut u64,
    ) {
        let spec = (self.h.acceptance)(house);
        let callbacks = (self.l.house_callbacks)(house);
        let mut accepts = spec.accepts;
        let prototype = tile == INVALID_TILE;
        if callbacks & CBM_ACCEPT != 0 {
            // SAFETY: The caller supplies a live town, or null for a prototype.
            let cb = unsafe { (self.h.callback)(CB_ACCEPT, 0, 0, house, town, tile, prototype) };
            if cb != CALLBACK_FAILED {
                accepts[0] = (self.h.cargo_translation)(house, (cb & 31) as u8);
                accepts[1] = (self.h.cargo_translation)(house, ((cb >> 5) & 31) as u8);
                accepts[2] = (self.h.cargo_translation)(house, ((cb >> 10) & 31) as u8);
            }
        }
        if callbacks & CBM_ACCEPTANCE != 0 {
            // SAFETY: The caller supplies a live town, or null for a prototype.
            let cb =
                unsafe { (self.h.callback)(CB_ACCEPTANCE, 0, 0, house, town, tile, prototype) };
            if cb != CALLBACK_FAILED {
                self.add_acceptance(ctx, always, accepts[0], u32::from(cb & 15));
                self.add_acceptance(ctx, always, accepts[1], u32::from((cb >> 4) & 15));
                if (self.l.climate)() != 0 && cb & (1 << 12) != 0 {
                    self.add_acceptance(
                        ctx,
                        always,
                        (self.h.food_cargo)(),
                        u32::from((cb >> 8) & 15),
                    );
                } else {
                    self.add_acceptance(ctx, always, accepts[2], u32::from((cb >> 8) & 15));
                }
                return;
            }
        }
        for (cargo, amount) in accepts.into_iter().zip(spec.amounts) {
            self.add_acceptance(ctx, always, cargo, u32::from(amount));
        }
    }
    /// `AddProducedCargo_Town`.
    fn produced(&self, tile: u32, ctx: *mut c_void) {
        let entry = (self.h.entry)(tile);
        let house = entry.house;
        let spec = (self.h.production)(house);
        let town = entry.town;
        if spec.callbacks & CBM_PRODUCE != 0 {
            for i in 0..256 {
                // SAFETY: Live town; no owner borrow spans the callback.
                let cb =
                    unsafe { (self.h.callback)(CB_PRODUCE, i, 0, house, town.shell, tile, false) };
                if cb == CALLBACK_FAILED || cb == PRODUCE_END {
                    break;
                }
                let cargo = (self.h.cargo_translation)(house, ((cb >> 8) & 0x7F) as u8);
                if cargo == INVALID_CARGO {
                    continue;
                }
                // SAFETY: Native stack-owned CargoArray outlives this entry.
                unsafe { (self.h.add_cargo)(ctx, cargo, 1) };
            }
        } else {
            for (effect, rate) in [(1, spec.population), (2, spec.mail)] {
                if rate == 0 {
                    continue;
                }
                let mut at = 0;
                loop {
                    let cargo = (self.h.production_cargo)(effect, at);
                    if cargo.cargo == INVALID_CARGO {
                        break;
                    }
                    // SAFETY: Native stack-owned CargoArray outlives this entry.
                    unsafe { (self.h.add_cargo)(ctx, cargo.cargo, 1) };
                    at += 1;
                }
            }
        }
    }
    /// `TriggerHouseAnimation_WatchedCargoAccepted`.
    fn watched(&self, tile: u32, cargoes: u64) {
        let mut house = (self.h.house_type)(tile);
        let cargoes = cargoes & (self.h.watched)(house);
        if cargoes == 0 {
            return;
        }
        let random = self.s.random() & 0xFFFF;
        let map = self.map();
        let north = tile.wrapping_add(self.north_part(&mut house, map));
        let flags = (self.l.house_flags)(house);
        let town = (self.h.town)(tile);
        let trigger = |part: u32| {
            let x = map.tile_x(tile).wrapping_sub(map.tile_x(part)) as u8;
            let y = map.tile_y(tile).wrapping_sub(map.tile_y(part)) as u8;
            let info = random << 16 | u32::from(y) << 8 | u32::from(x);
            // SAFETY: The entry resolved this live town shell; no owner borrow
            // spans the synchronous animation callback.
            unsafe {
                crate::animated_tile::house_change(
                    CB_WATCHED,
                    (self.h.house_type)(part),
                    town.shell,
                    part,
                    0,
                    info,
                    cargoes,
                );
            }
        };
        trigger(north);
        if flags & TILES_Y != 0 {
            trigger(north.wrapping_add(map.diff(0, 1)));
        }
        if flags & TILES_X != 0 {
            trigger(north.wrapping_add(map.diff(1, 0)));
        }
        if flags & SIZE_2X2 != 0 {
            trigger(north.wrapping_add(map.diff(1, 1)));
        }
    }
    fn map_size(&self) -> u32 {
        self.map()
            .size_x()
            .wrapping_mul((self.s.map_max_y)().wrapping_add(1))
    }
    fn towns(&self, mut visit: impl FnMut(*mut State)) {
        let mut town = (self.l.next_town)(0);
        while town != u32::MAX {
            visit((self.l.resolve_owner)(town));
            town = (self.l.next_town)(town.wrapping_add(1));
        }
    }
    /// `RebuildTownCaches`: canonical counts, population and houses in map order.
    fn rebuild_caches(&self) {
        (self.h.init_counts)();
        (self.s.rebuild_town_tree)();
        // SAFETY: Immediate owner writes; no service runs while accessed.
        self.towns(|state| unsafe {
            (*state).simulation.population = 0;
            (*state).simulation.num_houses = 0;
        });
        let map = self.map();
        for tile in 0..self.map_size() {
            if (self.s.tile_type)(tile) != MP_HOUSE {
                continue;
            }
            let rec = (self.h.cache)(tile);
            let state = (self.l.resolve_owner)(u32::from(rec.town));
            let class = (self.l.house_class)(rec.house);
            // SAFETY: Live owner; each access ends before the next spec query.
            unsafe { super::increase_building(state, rec.house, class) };
            if rec.completed {
                let population = (self.l.house_population)(rec.house);
                // SAFETY: As above.
                unsafe {
                    let fields = &raw mut (*state).simulation;
                    (*fields).population = (*fields).population.wrapping_add(u32::from(population));
                }
            }
            let mut house = rec.house;
            if self.north_part(&mut house, map) == 0 {
                // SAFETY: As above.
                unsafe {
                    let fields = &raw mut (*state).simulation;
                    (*fields).num_houses = (*fields).num_houses.wrapping_add(1);
                }
            }
        }
        // SAFETY: Pure owner mutation per live town.
        self.towns(|state| unsafe { update_radius(state) });
    }
    /// Yearly `IncrementHouseAge` in `Map::Iterate` order, saturating at 255.
    fn age_all(&self) {
        for tile in 0..self.map_size() {
            if (self.s.tile_type)(tile) != MP_HOUSE {
                continue;
            }
            crate::town_map::increment_house_age(self.h, tile);
        }
    }
    fn part_valid(&self, tile: u32, house: u32) -> bool {
        (self.s.tile_type)(tile) == MP_HOUSE && u32::from((self.h.clean_type)(tile)) == house
    }
    /// `UpdateHousesAndTowns`: substitute missing specs, drop broken footprints.
    fn update_all(&self) {
        let size = self.map_size();
        for tile in 0..size {
            if (self.s.tile_type)(tile) != MP_HOUSE {
                continue;
            }
            let house = (self.h.clean_type)(tile);
            if !(self.l.house_availability)(house).enabled && house >= NEW_HOUSE_OFFSET {
                (self.h.set_type)(tile, (self.h.substitute)(house));
            }
        }
        let map = self.map();
        for tile in 0..size {
            if (self.s.tile_type)(tile) != MP_HOUSE {
                continue;
            }
            let mut house = (self.h.clean_type)(tile);
            let north = tile.wrapping_add(self.north_part(&mut house, map));
            let id = u32::from(house);
            if tile == north {
                let flags = (self.l.house_flags)(house);
                let mut valid = true;
                if flags & SIZE_2X1 != 0 {
                    valid &= self.part_valid(tile.wrapping_add(map.diff(1, 0)), id + 1);
                } else if flags & SIZE_1X2 != 0 {
                    valid &= self.part_valid(tile.wrapping_add(map.diff(0, 1)), id + 1);
                } else if flags & SIZE_2X2 != 0 {
                    valid &= self.part_valid(tile.wrapping_add(map.diff(0, 1)), id + 1);
                    valid &= self.part_valid(tile.wrapping_add(map.diff(1, 0)), id + 2);
                    valid &= self.part_valid(tile.wrapping_add(map.diff(1, 1)), id + 3);
                }
                if !valid {
                    crate::ground::clear_square(tile, self.s);
                }
            } else if !self.part_valid(north, id) {
                crate::ground::clear_square(tile, self.s);
            }
        }
        self.rebuild_caches();
    }
}

/// Construction-stage trigger used by town growth's house construction.
pub(super) fn stage_changed(leaves: &Leaves, services: &Services, town: TownRef, tile: u32) {
    let engine = house(leaves, services);
    // The original reads GetHouseType after construction for each part: the
    // stored ID can have a NewGRF override, including on CmdPlaceHouse.
    let house_id = (engine.h.house_type)(tile);
    engine.stage_changed(town, tile, house_id, true);
}

// Native entries: immutable static tables outlive each synchronous call; native
// result and cargo-array storage are stack-owned by the native frame. The tile
// scheduler calls the tile loop directly, like every Rust-owned tile kind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_tile_loop(
    leaves: *const Leaves,
    services: *const Services,
    tile: u32,
) {
    house(leaves, services).tile_loop(tile);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_animate(
    leaves: *const Leaves,
    services: *const Services,
    tile: u32,
) {
    house(leaves, services).animate(tile);
}
pub(crate) unsafe fn openttd_rust_house_clear(
    leaves: *const Leaves,
    services: *const Services,
    result: *mut CommandCost,
    tile: u32,
    flags: u16,
) {
    house(leaves, services).clear(result, tile, flags);
}
pub(crate) unsafe fn openttd_rust_house_terraform(
    leaves: *const Leaves,
    services: *const Services,
    result: *mut CommandCost,
    tile: u32,
    flags: u16,
    z_new: i32,
    slope: u8,
) {
    house(leaves, services).terraform(result, tile, flags, z_new, slope);
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
    house(leaves, services).place(result, flags, tile, id, protected, replace);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_accepted(
    leaves: *const Leaves,
    services: *const Services,
    tile: u32,
    id: u16,
    town: *mut TownHandle,
    acceptance: *mut c_void,
    always: *mut u64,
) {
    house(leaves, services).accepted(tile, id, town, acceptance, always);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_produced(
    leaves: *const Leaves,
    services: *const Services,
    tile: u32,
    produced: *mut c_void,
) {
    house(leaves, services).produced(tile, produced);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_watched(
    leaves: *const Leaves,
    services: *const Services,
    tile: u32,
    cargoes: u64,
) {
    house(leaves, services).watched(tile, cargoes);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_rebuild_caches(
    leaves: *const Leaves,
    services: *const Services,
) {
    house(leaves, services).rebuild_caches();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_update_all(
    leaves: *const Leaves,
    services: *const Services,
) {
    house(leaves, services).update_all();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_age_all(
    leaves: *const Leaves,
    services: *const Services,
) {
    house(leaves, services).age_all();
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
        cost,
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
    "OpenTTDHouseCargo" ["probe"] => ProductionCargo {
        cargo,
        multiplier,
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
        town_shell,
        loop_spec,
        animation_spec,
        new_spec,
        production,
        sprite_groups,
        removal,
        acceptance,
        watched,
        substitute,
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
        callback,
        cargo_translation,
        convert_boolean,
        convert_8bit,
        resolve_reseed,

        next_near_station,
        erase_near_station,
        station_catchment,
        station_covers_town,
        station_finder,
        move_goods,
        production_cargo,
        add_cargo,
        food_cargo,
        scale_cargo,
        cargogen_mode,
        place_houses,
        company_human,
        init_counts,
        errors,
    };
}
