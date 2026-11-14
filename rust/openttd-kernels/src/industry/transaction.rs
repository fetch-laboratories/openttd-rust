/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry construction, funding and prospecting transaction. The result is a native stack
//! `CommandCost`; specs and layouts are immutable.
use super::{
    Builder, ConstructionRequest, ConstructionServices, GenerationServices, Industry,
    IndustryCommandServices, IndustryCost, IndustryCostServices, LayoutHandle, LifecycleServices,
    Services, ShellHandle, SpecHandle, TileServices, TownHandle,
};
use crate::bitmath_func::{gb, has_bit};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, Owner};
use crate::direction_type::DiagDirection;
use crate::economy_type::ExpensesType;
use crate::industry_type::{IndustryGfx, IndustryID, IndustryType};
use crate::industrytype::{
    CheckProc, IndustryBehaviour, IndustryBehaviours, IndustryLifeTypes, IndustryTileSpec,
    get_industry_spec, get_industry_tile_spec,
};
use crate::landscape_type::LandscapeType;
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, IndustryCallbackMask, IndustryCallbackMasks, IndustryTileCallbackMask,
    IndustryTileCallbackMasks,
};
use crate::newgrf_industries::IndustryAvailabilityCallType;
use crate::slope_func::{complement_slope, is_steep_slope};
use crate::slope_type::Slope;
use crate::strings_type::StringID;
use crate::tile_type::{TileIndex, TileType, TropicZone};
use crate::tilearea::TileArea;
use std::{cell::OnceCell, ffi::c_void, ptr};

#[repr(C)]
pub struct ConflictKinds {
    pub kinds: [u8; 3],
}
#[repr(C)]
pub struct Allocation {
    pub shell: *mut ShellHandle,
    pub owner: *mut Industry,
    pub id: u16,
}
#[repr(C)]
pub struct ConstructionErrors {
    pub site: u32,
    pub forest: u32,
    pub positioned: u32,
    pub desert: u32,
    pub rainforest: u32,
    pub lowland: u32,
    pub one_per_town: u32,
    pub town: u32,
    pub population: u32,
    pub near_town: u32,
    pub conflict: u32,
    pub too_many: u32,
}
#[repr(C)]
pub struct LocationResult {
    pub callback: u16,
    pub registers: [i32; 16],
}
#[repr(C)]
pub struct TransactionServices {
    pub town: unsafe extern "C" fn(*mut ShellHandle) -> *const TownHandle,
    pub population: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub town_position: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub industry: unsafe extern "C" fn(u16) -> *mut ShellHandle,
    pub edge_distance: unsafe extern "C" fn(u32, u8) -> u32,
    pub tile_height: unsafe extern "C" fn(u32) -> u32,
    pub set_company: unsafe extern "C" fn(u8),
    pub temporary_shell: unsafe extern "C" fn(
        *mut Industry,
        *mut c_void,
        unsafe extern "C" fn(*mut c_void, *mut ShellHandle),
    ),
    pub resolve_location:
        unsafe extern "C" fn(*mut ShellHandle, u32, u8, u32, u8) -> LocationResult,
    pub resolve_shape: unsafe extern "C" fn(*mut ShellHandle, u32, u16, u32) -> LocationResult,
    pub location_error:
        unsafe extern "C" fn(*mut IndustryCost, *const LocationResult, *const SpecHandle),
    pub shape_error:
        unsafe extern "C" fn(*mut IndustryCost, *const LocationResult, *const super::TileSpec),
    pub ignore_restrictions: unsafe extern "C" fn() -> bool,
    pub can_allocate: unsafe extern "C" fn() -> bool,
    pub allocate: unsafe extern "C" fn(u32) -> Allocation,
    pub local_company: unsafe extern "C" fn() -> bool,
    pub prospect_no_place: unsafe extern "C" fn(),
    pub prospect_unlucky: unsafe extern "C" fn(),
}
#[repr(C)]
pub struct TransactionContext {
    pub map: super::Map,
    pub builder: *mut Builder,
    pub production: *const Services,
    pub generation: *const GenerationServices,
    pub construction: *const ConstructionServices,
    pub lifecycle: *const LifecycleServices,
    pub tiles: *const TileServices,
    pub commands: *const IndustryCommandServices,
    pub command_checks: *const crate::services::CommandServices,
    pub costs: *const IndustryCostServices,
    pub transaction: *const TransactionServices,
    pub errors: *const ConstructionErrors,
}
/// The industry to be built, as `CreateNewIndustryHelper` and `CheckIfIndustryTileSlopes`
/// pass it to the `NewGRF` checks: its north tile, type, layout index, the random bits it is
/// going to have after construction, founder and the circumstances of its creation.
#[derive(Clone, Copy)]
struct IndustryCandidate {
    tile: TileIndex,
    kind: IndustryType,
    layout_index: usize,
    initial_random_bits: u16,
    founder: Owner,
    creation_type: IndustryAvailabilityCallType,
}
struct Engine<'a> {
    context: &'a TransactionContext,
    coordinates: OnceCell<crate::map_func::Map>,
    s: &'a Services,
    g: &'a GenerationServices,
    c: &'a ConstructionServices,
    t: &'a TileServices,
    m: &'a IndustryCommandServices,
    cost: &'a IndustryCostServices,
    x: &'a TransactionServices,
    e: &'a ConstructionErrors,
    out: *mut IndustryCost,
}
impl Engine<'_> {
    fn fail(&self, message: StringID) -> bool {
        unsafe { (self.cost.error)(self.out, message.0) };
        false
    }
    fn success(&self) -> bool {
        unsafe { (self.cost.zero)(self.out) };
        true
    }
    fn succeeded(&self) -> bool {
        unsafe { (self.cost.success)(self.out) }
    }
    fn company(&self) -> Owner {
        CompanyID((self.s.shared().current_company)())
    }
    fn swap_company(&self, company: Owner) -> Owner {
        let original = self.company();
        unsafe { (self.x.set_company)(company.0) };
        original
    }
    fn map(&self) -> super::Map {
        self.context.map
    }
    fn tile_add_wrap(&self, tile: TileIndex, x: i32, y: i32) -> TileIndex {
        let map = self.map();
        map.coordinates().tile_add_wrap(
            tile,
            x,
            y,
            map.size_y,
            (self.g.environment.freeform_edges)(),
        )
    }
    fn distance_max(&self, a: TileIndex, b: TileIndex) -> u32 {
        self.map().coordinates().distance_max(a, b)
    }
    fn tile_add_xy(&self, tile: TileIndex, x: i32, y: i32) -> TileIndex {
        self.coordinates
            .get_or_init(|| {
                let mut map = self.s.shared().map();
                map.log_x = self.context.map.size_x.trailing_zeros();
                map
            })
            .tile_add_xy(tile, x, y)
    }
    fn tile_type(&self, tile: TileIndex) -> TileType {
        TileType((self.s.shared().tile_type)(tile.0))
    }
    fn tile_height(&self, tile: TileIndex) -> u32 {
        unsafe { (self.x.tile_height)(tile.0) }
    }
    fn tile_slope(&self, tile: TileIndex) -> Slope {
        Slope(
            crate::map_func::World::new(self.s.shared())
                .tile_slope(tile)
                .0,
        )
    }
    fn tropic_zone(&self, tile: TileIndex) -> TropicZone {
        TropicZone((self.s.shared().tropic_zone)(tile.0))
    }
    fn editor(&self) -> bool {
        (self.g.environment.editor)()
    }
    fn arctic(&self) -> bool {
        LandscapeType(self.map().landscape) == LandscapeType::Arctic
    }
    /// `CheckScaledDistanceFromEdge`: check if a tile is within a distance from map edges,
    /// scaled by map dimensions independently. Each dimension is checked independently, and
    /// dimensions smaller than 256 are not scaled. `maxdist` is the normal distance on a
    /// 256x256 map.
    fn check_scaled_distance_from_edge(&self, tile: TileIndex, maxdist: u32) -> bool {
        let map = self.map();
        let mut maxdist_x = maxdist;
        let mut maxdist_y = maxdist;
        if map.size_x > 256 {
            maxdist_x = maxdist_x.wrapping_mul(map.size_x / 256);
        }
        if map.size_y > 256 {
            maxdist_y = maxdist_y.wrapping_mul(map.size_y / 256);
        }
        let distance = |dir: DiagDirection| unsafe { (self.x.edge_distance)(tile.0, dir.0) };
        distance(DiagDirection::DIAGDIR_NE) < maxdist_x
            || distance(DiagDirection::DIAGDIR_NW) < maxdist_y
            || distance(DiagDirection::DIAGDIR_SW) < maxdist_x
            || distance(DiagDirection::DIAGDIR_SE) < maxdist_y
    }
    fn oil_refinery_limit(&self) -> u32 {
        u32::from(self.s.oil_refinery_limit())
    }
    /// `_check_new_industry_procs[indspec->check_proc](tile)`.
    fn check_new_industry_proc(&self, tile: TileIndex, indspec: *const SpecHandle) -> bool {
        let shared = self.s.shared();
        match CheckProc(unsafe { (*indspec).check_proc }) {
            // `CheckNewIndustry_Forest`: industry should be built above the snow line in
            // the arctic climate.
            CheckProc::CHECK_FOREST => {
                if self.arctic()
                    && (shared.tile_z)(tile.0) < i32::from(crate::snow_line::highest(shared)) + 2
                {
                    return self.fail(StringID(self.e.forest));
                }
            }
            // `CheckNewIndustry_OilRefinery`: industry should be positioned near the edge
            // of the map.
            CheckProc::CHECK_REFINERY => {
                if !self.editor()
                    && !self.check_scaled_distance_from_edge(
                        self.tile_add_xy(tile, 1, 1),
                        self.oil_refinery_limit(),
                    )
                {
                    return self.fail(StringID(self.e.positioned));
                }
            }
            // `CheckNewIndustry_Farm`: industry should be below the snow line in the arctic.
            CheckProc::CHECK_FARM => {
                if self.arctic()
                    && (shared.tile_z)(tile.0).wrapping_add(2)
                        >= i32::from(crate::snow_line::highest(shared))
                {
                    return self.fail(StringID(self.e.site));
                }
            }
            // `CheckNewIndustry_Plantation`: industry should NOT be in the desert.
            CheckProc::CHECK_PLANTATION
                if self.tropic_zone(tile) == TropicZone::TROPICZONE_DESERT =>
            {
                return self.fail(StringID(self.e.site));
            }
            // `CheckNewIndustry_Water`: industry should be in the desert.
            CheckProc::CHECK_WATER if self.tropic_zone(tile) != TropicZone::TROPICZONE_DESERT => {
                return self.fail(StringID(self.e.desert));
            }
            // `CheckNewIndustry_Lumbermill`: industry should be in the rainforest.
            CheckProc::CHECK_LUMBERMILL
                if self.tropic_zone(tile) != TropicZone::TROPICZONE_RAINFOREST =>
            {
                return self.fail(StringID(self.e.rainforest));
            }
            // `CheckNewIndustry_BubbleGen`: industry should be in low land.
            CheckProc::CHECK_BUBBLEGEN if (shared.tile_z)(tile.0) > 4 => {
                return self.fail(StringID(self.e.lowland));
            }
            // `CheckNewIndustry_OilRig`: industries at sea should be positioned near the
            // edge of the map.
            CheckProc::CHECK_OIL_RIG => {
                if self.editor() && unsafe { (self.x.ignore_restrictions)() } {
                    return self.success();
                }
                if self.tile_height(tile) != 0
                    || !self.check_scaled_distance_from_edge(
                        self.tile_add_xy(tile, 1, 1),
                        self.oil_refinery_limit(),
                    )
                {
                    return self.fail(StringID(self.e.positioned));
                }
            }
            // `CheckNewIndustry_NULL` (`CHECK_NOTHING`) always succeeds.
            _ => {}
        }
        self.success()
    }
    fn registry(&self, kind: IndustryType) -> *const Vec<IndustryID> {
        unsafe {
            ptr::addr_of!((*self.context.builder).registry)
                .cast::<Vec<IndustryID>>()
                .add(usize::from(kind.0))
        }
    }
    fn registry_count(&self, kind: IndustryType) -> usize {
        unsafe { (&*self.registry(kind)).len() }
    }
    fn registry_id(&self, kind: IndustryType, n: usize) -> IndustryID {
        unsafe { ptr::read((&*self.registry(kind)).as_ptr().add(n)) }
    }
    /// `CheckIfFarEnoughFromConflictingIndustry`: check that the new industry is far enough
    /// from conflicting industries.
    fn check_if_far_enough_from_conflicting_industry(
        &self,
        tile: TileIndex,
        kind: IndustryType,
    ) -> bool {
        let indspec = get_industry_spec(kind);
        let conflicting = unsafe { (*indspec).conflicting };
        for conflicting_type in conflicting.map(IndustryType) {
            if conflicting_type == IndustryType::IT_INVALID {
                continue;
            }
            for n in 0..self.registry_count(conflicting_type) {
                let shell = unsafe { (self.x.industry)(self.registry_id(conflicting_type, n).0) };
                let owner = unsafe { (self.m.owner)(shell) };
                let position = TileIndex(unsafe { (*owner).metadata.tile });
                // Within 14 tiles from another industry is considered close.
                if self.distance_max(tile, position) <= 14 {
                    return self.fail(StringID(self.e.conflict));
                }
            }
        }
        self.success()
    }
    /// `FindTownForIndustry`: find a town for the industry, while checking for multiple
    /// industries in the same town. `None` means no good town can be found.
    fn find_town_for_industry(
        &self,
        tile: TileIndex,
        kind: IndustryType,
    ) -> Option<*const TownHandle> {
        let town = crate::town::spatial::closest_town_from_tile(
            unsafe { &*self.s.shared },
            tile,
            u32::MAX,
        );
        if !self.s.multiple_industry_per_town() {
            for n in 0..self.registry_count(kind) {
                let shell = unsafe { (self.x.industry)(self.registry_id(kind, n).0) };
                if unsafe { (self.x.town)(shell) } == town {
                    self.fail(StringID(self.e.one_per_town));
                    return None;
                }
            }
        }
        self.success();
        Some(town)
    }
    /// `CheckIfIndustryIsAllowed`: is the industry allowed to be built at this place for
    /// the town authority `t` it belongs to?
    fn check_if_industry_is_allowed(
        &self,
        tile: TileIndex,
        kind: IndustryType,
        t: *const TownHandle,
    ) -> bool {
        if IndustryBehaviours(unsafe { (*get_industry_spec(kind)).behaviour })
            .test(IndustryBehaviour::Town1200More)
            && unsafe { (self.x.population)(t) } < 1200
        {
            return self.fail(StringID(self.e.population));
        }
        if IndustryBehaviours(unsafe { (*get_industry_spec(kind)).behaviour })
            .test(IndustryBehaviour::OnlyNearTown)
            && self.distance_max(TileIndex(unsafe { (self.x.town_position)(t) }), tile) > 9
        {
            return self.fail(StringID(self.e.near_town));
        }
        self.success()
    }
    /// `CheckIfIndustryTilesAreFree`: are the tiles of the industry free?
    fn check_if_industry_tiles_are_free(
        &self,
        tile: TileIndex,
        layout: *const LayoutHandle,
        kind: IndustryType,
    ) -> bool {
        let ind_behav = IndustryBehaviours(unsafe { (*get_industry_spec(kind)).behaviour });
        let shared = self.s.shared();
        for n in 0..unsafe { (*layout).len() } {
            let it = super::layout_tile(layout, n);
            let gfx = IndustryGfx(unsafe { (self.c.translated_gfx)(it.gfx) });
            let cur_tile = self.tile_add_wrap(tile, i32::from(it.x), i32::from(it.y));
            let world = crate::map_func::World::new(shared);
            if !world.is_valid_tile(cur_tile) {
                return self.fail(StringID(self.e.site));
            }
            if gfx == IndustryGfx::GFX_WATERTILE_SPECIALCHECK {
                if !(shared.is_water_tile)(cur_tile.0) || !world.tile_flat(cur_tile) {
                    return self.fail(StringID(self.e.site));
                }
            } else {
                unsafe { (*self.context.command_checks).no_vehicle(self.out.cast(), cur_tile.0) };
                if !self.succeeded() {
                    return false;
                }
                if crate::map_func::World::new(shared).bridge_above(cur_tile.0) {
                    return self.fail(StringID(self.e.site));
                }
                let its = get_industry_tile_spec(gfx);
                // Perform land/water check if not disabled.
                if !has_bit(unsafe { (*its).slopes_refused }, 5)
                    && ((shared.has_water_class)(cur_tile.0)
                        && unsafe { (self.t.on_water)(cur_tile.0) })
                        != ind_behav.test(IndustryBehaviour::BuiltOnWater)
                {
                    return self.fail(StringID(self.e.site));
                }
                let in_town = IndustryBehaviours::from_element(IndustryBehaviour::OnlyInTown).with(
                    IndustryBehaviours::from_element(IndustryBehaviour::Town1200More),
                );
                let near_town = ind_behav.test(IndustryBehaviour::OnlyNearTown);
                if ind_behav.any(in_town) // Tile must be a house
                    || (near_town && self.tile_type(cur_tile) == TileType::MP_HOUSE)
                // Tile is allowed to be a house (and it is a house)
                {
                    if self.tile_type(cur_tile) != TileType::MP_HOUSE {
                        return self.fail(StringID(self.e.town));
                    }
                    // Clear the tiles as OWNER_TOWN to not affect town rating, and to not
                    // clear protected buildings.
                    let cur_company = self.swap_company(Owner::TOWN);
                    unsafe {
                        crate::landscape_cmd::clear_command(
                            &*self.context.command_checks,
                            self.out.cast(),
                            cur_tile.0,
                            DoCommandFlags::EMPTY.0,
                        );
                        (self.x.set_company)(cur_company.0);
                    };
                    if !self.succeeded() {
                        return false;
                    }
                } else {
                    // Clear the tiles, but do not affect town ratings.
                    unsafe {
                        crate::landscape_cmd::clear_command(
                            &*self.context.command_checks,
                            self.out.cast(),
                            cur_tile.0,
                            DoCommandFlag::AUTO
                                | DoCommandFlag::NO_TEST_TOWN_RATING
                                | DoCommandFlag::NO_MODIFY_TOWN_RATING,
                        );
                    };
                    if !self.succeeded() {
                        return false;
                    }
                }
            }
        }
        self.success()
    }
    /// `CheckIfIndustryTileSlopes`: check slope requirements for industry tiles, and report
    /// whether a custom check for the site was performed.
    fn check_if_industry_tile_slopes(
        &self,
        candidate: &IndustryCandidate,
        layout: *const LayoutHandle,
        custom_shape_check: &mut bool,
    ) -> bool {
        let mut refused_slope = false;
        let mut custom_shape = false;
        for n in 0..unsafe { (*layout).len() } {
            let it = super::layout_tile(layout, n);
            let gfx = IndustryGfx(unsafe { (self.c.translated_gfx)(it.gfx) });
            let cur_tile = self.tile_add_wrap(candidate.tile, i32::from(it.x), i32::from(it.y));
            let world = crate::map_func::World::new(self.s.shared());
            world.assert_with(|| world.is_valid_tile(cur_tile)); // checked before in CheckIfIndustryTilesAreFree

            if gfx != IndustryGfx::GFX_WATERTILE_SPECIALCHECK {
                let its = get_industry_tile_spec(gfx);
                if unsafe {
                    IndustryTileCallbackMasks((*its).callback_mask)
                        .test(IndustryTileCallbackMask::ShapeCheck)
                } {
                    custom_shape = true;
                    self.perform_industry_tile_slope_check(candidate, cur_tile, its, gfx);
                    if !self.succeeded() {
                        return false;
                    }
                } else {
                    let tileh = self.tile_slope(cur_tile);
                    refused_slope |=
                        is_slope_refused(tileh, Slope(unsafe { (*its).slopes_refused }));
                }
            }
        }
        *custom_shape_check = custom_shape;
        // It is almost impossible to have a fully flat land in TG, so what we
        //  do is that we check if we can make the land flat later on. See
        //  CheckIfCanLevelIndustryPlatform().
        if !refused_slope
            || (self.s.terragenesis()
                && (self.s.shared().clear_square_services.generating_world)()
                && !custom_shape
                && !unsafe { (self.x.ignore_restrictions)() })
        {
            return self.success();
        }
        self.fail(StringID(self.e.site))
    }
    /// `CheckCanTerraformSurroundingTiles`.
    fn check_can_terraform_surrounding_tiles(
        &self,
        tile: TileIndex,
        height: u32,
        internal: i32,
    ) -> bool {
        let coordinates = self.map().coordinates();
        // Check if we don't leave the map.
        if coordinates.tile_x(tile) == 0
            || coordinates.tile_y(tile) == 0
            || self.tile_type(tile) == TileType::MP_VOID
        {
            return false;
        }
        let ta = TileArea {
            tile: tile - coordinates.diff(1, 1),
            w: 2,
            h: 2,
        };
        for tile_walk in ta.tiles(coordinates) {
            let curh = self.tile_height(tile_walk);
            // Is the tile clear?
            if self.tile_type(tile_walk) != TileType::MP_CLEAR
                && self.tile_type(tile_walk) != TileType::MP_TREES
            {
                return false;
            }
            // Don't allow too big of a change if this is the sub-tile check.
            if internal != 0 && curh.abs_diff(height) > 1 {
                return false;
            }
            // Different height, so the surrounding tiles of this tile
            //  has to be correct too (in level, or almost in level)
            //  else you get a chain-reaction of terraforming.
            if internal == 0
                && curh != height
                && (coordinates.tile_x(tile_walk) == 0
                    || coordinates.tile_y(tile_walk) == 0
                    || !self.check_can_terraform_surrounding_tiles(
                        tile_walk + coordinates.diff(-1, -1),
                        height,
                        internal.wrapping_add(1),
                    ))
            {
                return false;
            }
        }
        true
    }
    /// `CheckIfCanLevelIndustryPlatform`: this function tries to flatten out the land below
    /// an industry, without damaging the surroundings too much.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "CheckIfCanLevelIndustryPlatform keeps the original's implicit C++ integer conversions"
    )]
    fn check_if_can_level_industry_platform(
        &self,
        tile: TileIndex,
        flags: DoCommandFlags,
        layout: *const LayoutHandle,
    ) -> bool {
        let (mut max_x, mut max_y) = (0_i32, 0_i32);
        // Finds dimensions of largest variant of this industry.
        for n in 0..unsafe { (*layout).len() } {
            let it = super::layout_tile(layout, n);
            // Watercheck tiles don't count for footprint size.
            if IndustryGfx(it.gfx) == IndustryGfx::GFX_WATERTILE_SPECIALCHECK {
                continue;
            }
            max_x = max_x.max(i32::from(it.x));
            max_y = max_y.max(i32::from(it.y));
        }
        // Remember level height.
        let h = self.tile_height(tile);
        let coordinates = self.map().coordinates();
        let industry_platform = u32::from(self.s.industry_platform());
        if coordinates.tile_x(tile) <= industry_platform.wrapping_add(1)
            || coordinates.tile_y(tile) <= industry_platform.wrapping_add(1)
        {
            return false;
        }
        // Check that all tiles in area and surrounding are clear; this determines that there
        // are no obstructing items. TileArea::Expand is not used here as we need to abort
        // instead of clamping if the bounds cannot expanded.
        let ta = TileArea {
            tile: tile + coordinates.diff(-(industry_platform as i32), -(industry_platform as i32)),
            w: (max_x
                .wrapping_add(2)
                .wrapping_add((2 * industry_platform) as i32)) as u16,
            h: (max_y
                .wrapping_add(2)
                .wrapping_add((2 * industry_platform) as i32)) as u16,
        };
        if coordinates.tile_x(ta.tile).wrapping_add(u32::from(ta.w)) >= coordinates.max_x()
            || coordinates.tile_y(ta.tile).wrapping_add(u32::from(ta.h))
                >= crate::map_func::Map::max_y(self.map().size_y)
        {
            return false;
        }
        // _current_company is OWNER_NONE for randomly generated industries and in editor, or
        // the company who funded or prospected the industry. Perform terraforming as
        // OWNER_TOWN to disable autoslope and town ratings.
        let cur_company = self.swap_company(Owner::TOWN);
        for tile_walk in ta.tiles(coordinates) {
            let curh = self.tile_height(tile_walk);
            if curh != h {
                // This tile needs terraforming. Check if we can do that without damaging the
                // surroundings too much.
                if !self.check_can_terraform_surrounding_tiles(tile_walk, h, 0) {
                    unsafe { (self.x.set_company)(cur_company.0) };
                    return false;
                }
                // This is not 100% correct check, but the best we can do without modifying
                // the map. What is missing, is if the difference in height is more than 1.
                unsafe {
                    crate::landscape_cmd::terraform_command(
                        &*self.context.command_checks,
                        self.out.cast(),
                        tile_walk.0,
                        Slope::SLOPE_N.0,
                        curh <= h,
                        flags.reset(DoCommandFlag::Execute).0,
                    )
                };
                if !self.succeeded() {
                    unsafe { (self.x.set_company)(cur_company.0) };
                    return false;
                }
            }
        }
        if flags.test(DoCommandFlag::Execute) {
            // Terraform the land under the industry.
            for tile_walk in ta.tiles(coordinates) {
                let mut curh = self.tile_height(tile_walk);
                while curh != h {
                    // We give the terraforming for free here, because we can't calculate
                    // exact cost in the test-round, and as we all know, that will cause a
                    // nice assert if they don't match ;)
                    unsafe {
                        crate::landscape_cmd::terraform_command(
                            &*self.context.command_checks,
                            self.out.cast(),
                            tile_walk.0,
                            Slope::SLOPE_N.0,
                            curh <= h,
                            flags.0,
                        )
                    };
                    curh = if curh > h {
                        curh.wrapping_sub(1)
                    } else {
                        curh.wrapping_add(1)
                    };
                }
            }
        }
        unsafe { (self.x.set_company)(cur_company.0) };
        true
    }
    /// `CheckIfCallBackAllowsCreation` (`newgrf_industries.cpp`): check that the industry
    /// callback allows creation of the industry, with `seed` for the random generator.
    fn check_if_call_back_allows_creation(&self, candidate: &IndustryCandidate, seed: u32) {
        let indspec = get_industry_spec(candidate.kind);
        let mut owner = Industry::default();
        let mut call = LocationCall {
            engine: self,
            indspec,
            candidate: *candidate,
            seed,
        };
        // The owner is a stack value. Only raw pointers cross the synchronous
        // native ctor/visit/resolver/dtor scope; none survives that scope.
        unsafe { (self.x.temporary_shell)(&raw mut owner, (&raw mut call).cast(), location_visit) };
    }
    /// `PerformIndustryTileSlopeCheck` (`newgrf_industrytiles.cpp`): check the slope of
    /// `ind_tile`, a tile of the new industry with specification `its` and gfx `gfx`.
    fn perform_industry_tile_slope_check(
        &self,
        candidate: &IndustryCandidate,
        ind_tile: TileIndex,
        its: *const IndustryTileSpec,
        gfx: IndustryGfx,
    ) {
        let mut owner = Industry::default();
        let mut call = ShapeCall {
            engine: self,
            candidate: *candidate,
            ind_tile,
            its,
            gfx,
        };
        unsafe { (self.x.temporary_shell)(&raw mut owner, (&raw mut call).cast(), shape_visit) };
    }
    /// `CreateNewIndustryHelper`: helper function for building or funding an industry.
    /// `random_var8f` is the random seed (possibly) used by industries. `ip` receives the
    /// newly created industry if all checks are successful and `flags` request actual
    /// creation, else it contains null afterwards.
    fn create_new_industry_helper(
        &self,
        candidate: &IndustryCandidate,
        flags: DoCommandFlags,
        indspec: *const SpecHandle,
        random_var8f: u32,
        ip: *mut *mut ShellHandle,
    ) -> bool {
        let (tile, kind) = (candidate.tile, candidate.kind);
        self.s
            .assert_with(|| candidate.layout_index < unsafe { (*indspec).layouts.len() });
        let layout = unsafe { &raw const (*indspec).layouts.as_slice()[candidate.layout_index] };
        unsafe { *ip = ptr::null_mut() };
        // 1. Cheap: Built-in checks on industry level.
        if !self.check_if_far_enough_from_conflicting_industry(tile, kind) {
            return false;
        }
        let Some(t) = self.find_town_for_industry(tile, kind) else {
            return false;
        };
        self.s.assert_with(|| !t.is_null());

        if !self.check_if_industry_is_allowed(tile, kind, t) {
            return false;
        }
        // 2. Built-in checks on industry tiles.
        let object_areas = crate::objects::snapshot_areas();
        let free = self.check_if_industry_tiles_are_free(tile, layout, kind);
        crate::objects::restore_areas(object_areas);
        if !free {
            return false;
        }
        // 3. NewGRF-defined checks on industry level.
        if IndustryCallbackMasks(unsafe { (*get_industry_spec(kind)).callback_mask })
            .test(IndustryCallbackMask::Location)
        {
            self.check_if_call_back_allows_creation(candidate, random_var8f);
        } else {
            self.check_new_industry_proc(tile, indspec);
        }
        if !self.succeeded() {
            return false;
        }
        // 4. Expensive: NewGRF-defined checks on industry tiles.
        let mut custom_shape_check = false;
        if !self.check_if_industry_tile_slopes(candidate, layout, &mut custom_shape_check) {
            return false;
        }
        if !custom_shape_check
            && self.s.terragenesis()
            && (self.s.shared().clear_square_services.generating_world)()
            && !unsafe { (self.x.ignore_restrictions)() }
            && !self.check_if_can_level_industry_platform(
                tile,
                DoCommandFlags::from_element(DoCommandFlag::NoWater),
                layout,
            )
        {
            return self.fail(StringID(self.e.site));
        }
        if !unsafe { (self.x.can_allocate)() } {
            return self.fail(StringID(self.e.too_many));
        }
        if flags.test(DoCommandFlag::Execute) {
            let allocation = unsafe { (self.x.allocate)(tile.0) };
            unsafe { *ip = allocation.shell };
            if !custom_shape_check {
                self.check_if_can_level_industry_platform(
                    tile,
                    DoCommandFlags::from_element(DoCommandFlag::NoWater)
                        .with(DoCommandFlags::from_element(DoCommandFlag::Execute)),
                    layout,
                );
            }
            let request = ConstructionRequest {
                layout,
                town: t,
                layout_index: candidate.layout_index,
                tile: tile.0,
                size_x: self.map().size_x,
                size_y: self.map().size_y,
                id: allocation.id,
                random_bits: candidate.initial_random_bits,
                kind: kind.0,
                founder: candidate.founder.0,
            };
            unsafe {
                super::do_create_new_industry(
                    allocation.shell.cast(),
                    allocation.owner,
                    &request,
                    self.context,
                );
            };
        }
        self.success()
    }
}
struct LocationCall<'a, 'b> {
    engine: &'a Engine<'b>,
    indspec: *const SpecHandle,
    candidate: IndustryCandidate,
    seed: u32,
}
#[allow(
    clippy::cast_possible_truncation,
    reason = "CheckIfCallBackAllowsCreation's location callback keeps the original's implicit C++ integer conversions"
)]
unsafe extern "C" fn location_visit(context: *mut c_void, shell: *mut ShellHandle) {
    let call = unsafe { &*context.cast::<LocationCall<'_, '_>>() };
    let (e, candidate) = (call.engine, &call.candidate);
    let owner = unsafe { (e.m.owner)(shell) };
    unsafe {
        (*owner).metadata.tile = candidate.tile.0;
        // Zero width is important to mark the industry invalid.
        (*owner).metadata.width = 0;
        (*owner).metadata.kind = candidate.kind.0;
        (*owner).metadata.selected_layout = candidate.layout_index as u8;
    }
    let town = crate::town::spatial::closest_town_from_tile(
        unsafe { &*e.s.shared },
        candidate.tile,
        u32::MAX,
    );
    unsafe {
        (e.c.set_town)(shell.cast(), town);
        (*owner).metadata.random = candidate.initial_random_bits;
        (*owner).metadata.founder = candidate.founder.0;
    }
    let result = unsafe {
        (e.x.resolve_location)(
            shell,
            candidate.tile.0,
            candidate.kind.0,
            call.seed,
            candidate.creation_type.0,
        )
    };
    // Unlike the "normal" cases, not having a valid result means we allow the building of
    // the industry, as that's how it's done in TTDP.
    if result.callback == CALLBACK_FAILED {
        e.success();
    } else {
        unsafe { (e.x.location_error)(e.out, &raw const result, call.indspec) };
    }
}
struct ShapeCall<'a, 'b> {
    engine: &'a Engine<'b>,
    candidate: IndustryCandidate,
    ind_tile: TileIndex,
    its: *const IndustryTileSpec,
    gfx: IndustryGfx,
}
#[allow(
    clippy::cast_possible_truncation,
    reason = "CheckIfIndustryTileSlopes's shape callback keeps the original's implicit C++ integer conversions"
)]
unsafe extern "C" fn shape_visit(context: *mut c_void, shell: *mut ShellHandle) {
    let call = unsafe { &*context.cast::<ShapeCall<'_, '_>>() };
    let (e, candidate) = (call.engine, &call.candidate);
    let owner = unsafe { (e.m.owner)(shell) };
    unsafe {
        (*owner).metadata.tile = candidate.tile.0;
        (*owner).metadata.width = 0;
        (*owner).metadata.kind = candidate.kind.0;
        (*owner).metadata.random = candidate.initial_random_bits;
        (*owner).metadata.founder = candidate.founder.0;
    }
    let parameter = (u32::from(candidate.creation_type.0) << 8) | candidate.layout_index as u32;
    let callback_res =
        unsafe { (e.x.resolve_shape)(shell, call.ind_tile.0, call.gfx.0, parameter) };
    if callback_res.callback == CALLBACK_FAILED {
        if is_slope_refused(
            e.tile_slope(call.ind_tile),
            Slope(unsafe { (*call.its).slopes_refused }),
        ) {
            e.fail(StringID(e.e.site));
        } else {
            e.success();
        }
    } else if (e.s.shared().grf_file_version)(unsafe { (*call.its).grf_prop.grffile }) < 7 {
        if callback_res.callback != 0 {
            e.success();
        } else {
            e.fail(StringID(e.e.site));
        }
    } else {
        unsafe { (e.x.shape_error)(e.out, &raw const callback_res, call.its) };
    }
}
/// `IsSlopeRefused`.
fn is_slope_refused(current: Slope, refused: Slope) -> bool {
    if is_steep_slope(current) {
        return true;
    }
    if current != Slope::SLOPE_FLAT {
        if is_steep_slope(refused) {
            return true;
        }
        let t = complement_slope(current, |ok| crate::services_assert::check(ok)).0;
        if refused.0 & Slope::SLOPE_W.0 != 0 && t & Slope::SLOPE_NW.0 != 0 {
            return true;
        }
        if refused.0 & Slope::SLOPE_S.0 != 0 && t & Slope::SLOPE_NE.0 != 0 {
            return true;
        }
        if refused.0 & Slope::SLOPE_E.0 != 0 && t & Slope::SLOPE_SW.0 != 0 {
            return true;
        }
        if refused.0 & Slope::SLOPE_N.0 != 0 && t & Slope::SLOPE_SE.0 != 0 {
            return true;
        }
    }
    false
}
unsafe fn engine<'a>(context: *const TransactionContext, out: *mut IndustryCost) -> Engine<'a> {
    let context = unsafe { &*context };
    unsafe {
        Engine {
            context,
            coordinates: OnceCell::new(),
            s: &*context.production,
            g: &*context.generation,
            c: &*context.construction,
            t: &*context.tiles,
            m: &*context.commands,
            cost: &*context.costs,
            x: &*context.transaction,
            e: &*context.errors,
            out,
        }
    }
}
/// `CmdBuildIndustry`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdBuildIndustry {
    /// The tile to build the industry at.
    pub(crate) tile: TileIndex,
    /// The industry type to build.
    pub(crate) it: IndustryType,
    /// The first layout to try.
    pub(crate) first_layout: u32,
    /// False = prospect, True = fund (only valid if current company is DEITY).
    pub(crate) fund: bool,
    /// Seed to use for desyncfree randomisations.
    pub(crate) seed: u32,
}
/// `CmdBuildIndustry`: build/fund an industry. The result in `out` holds the cost of this
/// operation or an error.
///
/// # Safety
/// Live native stack cost/output and synchronous services. Borrowed specs/layouts
/// remain immutable. Commands, allocation and resolvers receive no owner borrow.
#[allow(
    clippy::cast_possible_truncation,
    reason = "CmdBuildIndustry keeps the original's implicit C++ integer conversions"
)]
#[allow(
    clippy::too_many_lines,
    reason = "CmdBuildIndustry is ported as the original's single body; its length is tracked in the Clippy debt ledger"
)]
pub(crate) unsafe fn cmd_build_industry(
    out: *mut IndustryCost,
    flags: DoCommandFlags,
    args: CmdBuildIndustry,
    context: &TransactionContext,
) {
    let CmdBuildIndustry {
        tile,
        it,
        first_layout,
        fund,
        seed,
    } = args;
    if it >= IndustryType::NUM_INDUSTRYTYPES {
        unsafe { ((*context.costs).error)(out, StringID::INVALID_STRING_ID.0) };
        return;
    }
    let e = unsafe { engine(context, out) };
    let indspec = get_industry_spec(it);
    // Check if the to-be built/founded industry is available for this climate.
    if !unsafe { (*indspec).enabled } || unsafe { (*indspec).layouts.is_empty() } {
        e.fail(StringID::INVALID_STRING_ID);
        return;
    }
    // If the setting for raw-material industries is not on, you cannot build raw-material
    // industries. Raw material industries are industries that do not accept cargo (at
    // least for now).
    if !e.editor()
        && e.company() != Owner::DEITY
        && e.s.raw_industry_construction() == 0
        && unsafe { super::is_raw_industry(IndustryLifeTypes((*indspec).life_type)) }
    {
        e.fail(StringID::INVALID_STRING_ID);
        return;
    }
    if !e.editor()
        && unsafe {
            super::resolver::get_industry_probability_callback(
                it,
                if e.company() == Owner::DEITY {
                    IndustryAvailabilityCallType::IACT_RANDOMCREATION
                } else {
                    IndustryAvailabilityCallType::IACT_USERCREATION
                },
                1,
                &*e.s.resolver,
            )
        } == 0
    {
        e.fail(StringID::INVALID_STRING_ID);
        return;
    }
    let mut randomizer = crate::random_func::Randomizer::default();
    randomizer.set_seed(seed);
    let random_initial_bits = gb(seed, 0, 16) as u16;
    let random_var8f = randomizer.next();
    let num_layouts = unsafe { (*indspec).layouts.len() };
    e.fail(StringID(e.e.site));
    let deity_prospect = e.company() == Owner::DEITY && !fund;
    let mut ind = ptr::null_mut();
    if deity_prospect
        || (!e.editor()
            && e.company() != Owner::DEITY
            && e.s.raw_industry_construction() == 2
            && unsafe { super::is_raw_industry(IndustryLifeTypes((*indspec).life_type)) })
    {
        if flags.test(DoCommandFlag::Execute) {
            // Prospecting has a chance to fail, however we cannot guarantee that something
            // can be built on the map, so the chance gets lower when the map is fuller, but
            // there is nothing we can really do about that.
            let prospect_success =
                deity_prospect || e.s.shared().random() <= unsafe { (*indspec).prospecting_chance };
            if prospect_success {
                // Prospected industries are build as OWNER_TOWN to not e.g. be build on owned
                // land of the founder.
                let calltype = if e.company() == Owner::DEITY {
                    IndustryAvailabilityCallType::IACT_RANDOMCREATION
                } else {
                    IndustryAvailabilityCallType::IACT_PROSPECTCREATION
                };
                let cur_company = e.swap_company(Owner::TOWN);
                for _ in 0..5000 {
                    // We should not have more than one Random() in a function call because
                    // parameter evaluation order is not guaranteed in the c++ standard.
                    let world = crate::map_func::World::new(e.s.shared());
                    let tile = world
                        .dimensions
                        .random_tile_seed(world.size_y(), e.s.shared().random());
                    // Start with a random layout.
                    let mut layout = e.s.shared().random_range(num_layouts as u32) as usize;
                    // Check now each layout, starting with the random one.
                    for _ in 0..num_layouts {
                        layout = layout.wrapping_add(1) % num_layouts;
                        let candidate = IndustryCandidate {
                            tile,
                            kind: it,
                            layout_index: layout,
                            initial_random_bits: random_initial_bits,
                            founder: cur_company,
                            creation_type: calltype,
                        };
                        if e.create_new_industry_helper(
                            &candidate,
                            flags,
                            indspec,
                            random_var8f,
                            &raw mut ind,
                        ) {
                            break;
                        }
                    }
                    if e.succeeded() {
                        break;
                    }
                }
                unsafe { (e.x.set_company)(cur_company.0) };
            }
            if !e.succeeded() && unsafe { (e.x.local_company)() } {
                if prospect_success {
                    unsafe { (e.x.prospect_no_place)() };
                } else {
                    unsafe { (e.x.prospect_unlucky)() };
                }
            }
        }
    } else {
        let mut layout = first_layout as usize;
        if layout >= num_layouts {
            e.fail(StringID::INVALID_STRING_ID);
            return;
        }
        // Check subsequently each layout, starting with the given layout in p1.
        for _ in 0..num_layouts {
            layout = layout.wrapping_add(1) % num_layouts;
            let candidate = IndustryCandidate {
                tile,
                kind: it,
                layout_index: layout,
                initial_random_bits: random_initial_bits,
                founder: e.company(),
                creation_type: if e.company() == Owner::DEITY {
                    IndustryAvailabilityCallType::IACT_RANDOMCREATION
                } else {
                    IndustryAvailabilityCallType::IACT_USERCREATION
                },
            };
            if e.create_new_industry_helper(&candidate, flags, indspec, random_var8f, &raw mut ind)
            {
                break;
            }
        }
        // If it still failed, there's no suitable layout to build here, return the error.
        if !e.succeeded() {
            return;
        }
    }
    if flags.test(DoCommandFlag::Execute) && !ind.is_null() && !e.editor() {
        super::advertise(ind.cast(), e.s);
    }
    let construction = unsafe {
        super::get_construction_cost(
            (*indspec).cost_multiplier,
            IndustryLifeTypes((*indspec).life_type),
            e.s.raw_industry_construction(),
        )
    };
    unsafe { (e.cost.expense_money)(out, ExpensesType::EXPENSES_OTHER.0, construction) };
}
/// # Safety
/// As `cmd_build_industry`; the transaction tables are live.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_build(
    out: *mut IndustryCost,
    tile: u32,
    kind: u8,
    flags: u16,
    first_layout: u32,
    fund: bool,
    seed: u32,
    context: *const TransactionContext,
) {
    unsafe {
        cmd_build_industry(
            out,
            DoCommandFlags(flags),
            CmdBuildIndustry {
                tile: TileIndex(tile),
                it: IndustryType(kind),
                first_layout,
                fund,
                seed,
            },
            &*context,
        );
    }
}
/// # Safety
/// Live stack result/output; source preconditions require a nonempty layout list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_create(
    out: *mut IndustryCost,
    industry: *mut *mut ShellHandle,
    tile: u32,
    kind: u8,
    creation_type: u8,
    context: *const TransactionContext,
) {
    let e = unsafe { engine(context, out) };
    unsafe {
        create_new_industry(
            &e,
            industry,
            TileIndex(tile),
            IndustryType(kind),
            IndustryAvailabilityCallType(creation_type),
        );
    }
}
/// `CreateNewIndustry`: create a new industry of random layout at `tile`. `industry`
/// receives the created industry, or null if it failed.
///
/// # Safety
/// Live output; source preconditions require a nonempty layout list.
#[allow(
    clippy::cast_possible_truncation,
    reason = "CreateNewIndustry keeps the original's implicit C++ integer conversions"
)]
unsafe fn create_new_industry(
    e: &Engine<'_>,
    industry: *mut *mut ShellHandle,
    tile: TileIndex,
    kind: IndustryType,
    creation_type: IndustryAvailabilityCallType,
) {
    let indspec = get_industry_spec(kind);
    let seed = e.s.shared().random();
    let seed2 = e.s.shared().random();
    let layout_index =
        e.s.shared()
            .random_range(unsafe { (*indspec).layouts.len() } as u32) as usize;
    let candidate = IndustryCandidate {
        tile,
        kind,
        layout_index,
        initial_random_bits: gb(seed2, 0, 16) as u16,
        founder: Owner::NONE,
        creation_type,
    };
    let succeeded = e.create_new_industry_helper(
        &candidate,
        DoCommandFlags::from_element(DoCommandFlag::Execute),
        indspec,
        seed,
        industry,
    );
    e.s.assert_with(|| unsafe { !(*industry).is_null() } || !succeeded);
}

/// # Safety
/// Native stack cost and immutable tables. Temporary shell construction/destruction
/// is synchronous and no temporary owner or resolver pointer may escape the visit.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_location_check(
    out: *mut IndustryCost,
    tile: u32,
    kind: u8,
    layout: usize,
    random: u32,
    bits: u16,
    founder: u8,
    creation: u8,
    context: *const TransactionContext,
) {
    let candidate = IndustryCandidate {
        tile: TileIndex(tile),
        kind: IndustryType(kind),
        layout_index: layout,
        initial_random_bits: bits,
        founder: CompanyID(founder),
        creation_type: IndustryAvailabilityCallType(creation),
    };
    unsafe { engine(context, out) }.check_if_call_back_allows_creation(&candidate, random);
}
/// # Safety
/// Same temporary shell/stack cost contract as the native location callback.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_shape_check(
    out: *mut IndustryCost,
    tile: u32,
    current: u32,
    spec: *const super::TileSpec,
    kind: u8,
    gfx: u16,
    layout: usize,
    bits: u16,
    founder: u8,
    creation: u8,
    context: *const TransactionContext,
) {
    let candidate = IndustryCandidate {
        tile: TileIndex(tile),
        kind: IndustryType(kind),
        layout_index: layout,
        initial_random_bits: bits,
        founder: CompanyID(founder),
        creation_type: IndustryAvailabilityCallType(creation),
    };
    unsafe { engine(context, out) }.perform_industry_tile_slope_check(
        &candidate,
        TileIndex(current),
        spec,
        IndustryGfx(gfx),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_industry_type_returns_error_without_map_metadata() {
        unsafe extern "C" fn error(out: *mut IndustryCost, message: u32) {
            unsafe { out.cast::<u32>().write(message) };
        }
        let costs = IndustryCostServices {
            zero: crate::test_tables::unreachable1,
            error,
            expense_money: crate::test_tables::unreachable3,
            success: crate::test_tables::unreachable1,
        };
        let context = TransactionContext {
            map: super::super::Map {
                size_x: 64,
                size_y: 64,
                landscape: 0,
            },
            builder: ptr::null_mut(),
            production: ptr::null(),
            generation: ptr::null(),
            construction: ptr::null(),
            lifecycle: ptr::null(),
            tiles: ptr::null(),
            commands: ptr::null(),
            command_checks: ptr::null(),
            costs: &raw const costs,
            transaction: ptr::null(),
            errors: ptr::null(),
        };
        let mut message = 0;
        // Original CmdBuildIndustry rejects the type before any world observation.
        unsafe {
            cmd_build_industry(
                ptr::from_mut(&mut message).cast(),
                DoCommandFlags(0),
                CmdBuildIndustry {
                    tile: TileIndex(0),
                    it: IndustryType::NUM_INDUSTRYTYPES,
                    first_layout: 0,
                    fund: false,
                    seed: 0,
                },
                &context,
            );
        }
        assert_eq!(message, StringID::INVALID_STRING_ID.0);
    }
}
