/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete town founding, placement and generation; owners are never borrowed
//! across native callbacks. Cancellation returns to its native exception frame.
use super::{
    Engine, Leaves, State, TOWN_GROWTH_DESERT, TOWN_GROWTH_WINTER, TownId, TownRef, names,
    town_ticks_to_game_ticks, update_radius,
};
use crate::cargotype::TownAcceptanceEffect;
use crate::clear_map::ClearGround;
use crate::company_type::{CompanyID, MAX_COMPANIES};
use crate::landscape_type::LandscapeType;
use crate::services::{ByteSpan, CommandCost, CommandCostServices, Services, TownHandle};
use crate::tile_type::{TileIndex, TileType};
use crate::timer_game_tick::TOWN_GROWTH_TICKS;
use crate::town_type::{RATING_INITIAL, TOWN_EXPAND_BUILDINGS, TOWN_EXPAND_ROADS, TownID};
use crate::tree_map::TreeGround;
use crate::witness as w;
use std::cell::OnceCell;

macro_rules! observed {
    ($call:expr) => {
        crate::witness::counted!(TOWN_SERVICES, TOWN_BYTES, $call)
    };
}
pub(super) use observed;

macro_rules! native_byte {
    ($($name:ident),+) => { $(
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct $name(pub u8);
    )+ };
}
native_byte!(CompanyId, TownFounding, AcceptanceEffect);
pub use crate::tile_type::TileIndex as Tile;
pub use crate::town_type::{TownLayout, TownSize};
#[repr(C)]
pub struct Frame {
    _private: [u8; 0],
}
#[repr(C)]
pub struct FoundingServices {
    pub shared: *const Services,
    pub growth: *const Leaves,
    pub names: *const names::NameServices,
    pub cost: *const CommandCostServices,
    pub allocate: extern "C" fn(Tile) -> TownRef,
    pub resolve: extern "C" fn(TownId) -> TownRef,
    pub can_allocate: extern "C" fn() -> bool,
    pub flat: extern "C" fn(Tile) -> bool,
    pub water_distance: extern "C" fn(Tile, bool) -> u32,
    pub scale_size: extern "C" fn(u32) -> u32,
    pub scale_land: extern "C" fn(u32) -> u32,
    pub found_setting: extern "C" fn() -> TownFounding,
    pub layout_setting: extern "C" fn() -> TownLayout,
    pub initial_city_size: extern "C" fn() -> u8,
    pub town_density: extern "C" fn() -> u8,
    pub custom_number: extern "C" fn() -> u16,
    pub larger_towns: extern "C" fn() -> u8,
    pub name_style: extern "C" fn() -> u8,
    pub prepare_rename: extern "C" fn(*mut TownHandle),
    pub name_changed: extern "C" fn(*mut TownHandle, ByteSpan),
    pub update_virt: extern "C" fn(*mut TownHandle),
    pub directory_rebuild: extern "C" fn(),
    pub directory_resort: extern "C" fn(),
    pub update_noise: extern "C" fn(),
    pub delete_town: extern "C" fn(TownId),
    pub assert_random_context: extern "C" fn(),
    pub assert_allocation: extern "C" fn(),
    pub initialize_cost: extern "C" fn(*mut CommandCost, i32),
    pub command: *const crate::services::CommandServices,
    pub money_deficit: extern "C" fn(*mut Frame, *const CommandCost),
    pub set_result: extern "C" fn(*mut Frame, TownId),
    pub exchange_generating: extern "C" fn(bool) -> bool,
    pub founded_news: extern "C" fn(TownId, Tile),
    pub founded_events: extern "C" fn(TownId),
    pub clear_station_names: extern "C" fn(),
    pub clear_industry_names: extern "C" fn(),
    pub update_station_coords: extern "C" fn(),
    pub cached_name: extern "C" fn(*const TownHandle) -> ByteSpan,
    pub set_progress: extern "C" fn(*mut Frame, u32) -> bool,
    pub increase_progress: extern "C" fn(*mut Frame) -> bool,
    pub no_town_error: extern "C" fn(),
    pub random: *mut names::Randomizer,
    pub edge_error: u32,
    pub close_error: u32,
    pub unsuitable_error: u32,
    pub unique_error: u32,
    pub too_many_error: u32,
    pub no_space_error: u32,
}
struct Founding<'a> {
    road: *const crate::road_cmd::CommandContext,
    s: &'a FoundingServices,
    shared: &'a Services,
    leaves: &'a Leaves,
    names: &'a names::NameServices,
    cost: &'a CommandCostServices,
    map: OnceCell<crate::services::Map>,
    size_y: OnceCell<u32>,
}
impl<'a> Founding<'a> {
    unsafe fn new(s: &'a FoundingServices) -> Self {
        // SAFETY: Native immutable tables remain live throughout this invocation.
        let (shared, leaves, names, cost) =
            unsafe { (&*s.shared, &*s.growth, &*s.names, &*s.cost) };
        Self {
            road: std::ptr::null(),
            s,
            shared,
            leaves,
            names,
            cost,
            map: OnceCell::new(),
            size_y: OnceCell::new(),
        }
    }
    fn map(&self) -> crate::services::Map {
        *self.map.get_or_init(|| self.shared.map())
    }
    fn size_y(&self) -> u32 {
        *self
            .size_y
            .get_or_init(|| observed!((self.shared.map_max_y)()).wrapping_add(1))
    }
    fn engine(&self, town: TownRef) -> Engine<'a> {
        let map = OnceCell::new();
        let _ = map.set(self.map());
        Engine {
            road: self.road,
            leaves: self.leaves,
            services: self.shared,
            map,
            owner: town.state,
            house_shell: std::cell::OnceCell::from(town.shell),
        }
    }
    fn random(&self) -> u32 {
        observed!(self.shared.random())
    }
    fn spiral(&self, tile: Tile, diameter: u32) -> crate::spiral::SpiralState {
        crate::spiral::square(
            tile.0 % self.map().size_x(),
            tile.0 / self.map().size_x(),
            diameter,
            self.map().size_x(),
            self.size_y(),
        )
    }
    /// Towns must all share a grid so their road networks mesh when they interpenetrate.
    /// Tell whether the tile is properly aligned (`town_cmd.cpp::IsTileAlignedToGrid`).
    fn is_tile_aligned_to_grid(&self, tile: TileIndex, layout: TownLayout) -> bool {
        let x = tile.0 % self.map().size_x();
        let y = tile.0 / self.map().size_x();
        match layout {
            TownLayout::TL_2X2_GRID => x.is_multiple_of(3) && y.is_multiple_of(3),
            TownLayout::TL_3X3_GRID => x.is_multiple_of(4) && y.is_multiple_of(4),
            _ => true,
        }
    }
    // Towns must share a grid so their road networks mesh when they interpenetrate.
    fn align_tile_to_grid(&self, tile: Tile, layout: TownLayout) -> Tile {
        let x = tile.0 % self.map().size_x();
        let y = tile.0 / self.map().size_x();
        match layout {
            TownLayout::TL_2X2_GRID => {
                Tile((x - x % 3).wrapping_add((y - y % 3).wrapping_mul(self.map().size_x())))
            }
            TownLayout::TL_3X3_GRID => {
                Tile((x & !3).wrapping_add((y & !3).wrapping_mul(self.map().size_x())))
            }
            _ => tile,
        }
    }
    /// Check whether a town can be placed on the tile (`town_cmd.cpp::TownCanBePlacedHere`).
    /// `check_surrounding` also requires surrounding tiles to be free.
    /// Return no error if allowed, otherwise the native placement error.
    fn town_can_be_placed_here(&self, tile: TileIndex, check_surrounding: bool) -> Option<u32> {
        w::hit(w::TOWN_PLACEMENT);
        // Check if too close to the edge of map.
        if self.map().distance_from_edge(tile.0, self.size_y()) < 12 {
            return Some(self.s.edge_error);
        }
        // Check distance to all other towns.
        if observed!(super::spatial::close(self.shared, tile.0, 20)) {
            return Some(self.s.close_error);
        }
        // Can only build on clear flat areas, possibly with trees.
        let tile_type = TileType(observed!((self.shared.tile_type)(tile.0)));
        if (tile_type != TileType::MP_CLEAR && tile_type != TileType::MP_TREES)
            || !observed!((self.s.flat)(tile))
        {
            return Some(self.s.unsuitable_error);
        }
        // We might want to make sure the town has enough room.
        if check_surrounding {
            const SEARCH_DIAMETER: u32 = 5; // Center tile of town + 2 tile radius.
            // Half of the tiles in the search must be valid for the town to build upon.
            const VALID_TILE_GOAL: u32 = (SEARCH_DIAMETER * SEARCH_DIAMETER) / 2;
            let mut counter = 0;
            let town_height = observed!((self.shared.tile_z)(tile.0));
            let mut spiral = self.spiral(tile, SEARCH_DIAMETER);
            while !spiral.is_end() {
                if counter == VALID_TILE_GOAL {
                    break;
                }
                let at = TileIndex(
                    spiral
                        .x
                        .wrapping_add(spiral.y.wrapping_mul(self.map().size_x())),
                );
                let valid = match TileType(observed!((self.shared.tile_type)(at.0))) {
                    // Don't allow rough tiles, as they are likely wetlands.
                    TileType::MP_CLEAR => {
                        observed!((self.shared.clear_ground)(at.0)) != ClearGround::CLEAR_ROUGH
                    }
                    // Don't allow rough trees, as they are likely wetlands.
                    TileType::MP_TREES => {
                        observed!((self.shared.tree_ground)(at.0)) != TreeGround::TREE_GROUND_ROUGH
                    }
                    _ => false,
                };
                if valid
                    && observed!((self.shared.tile_max_z)(at.0)) <= town_height.wrapping_add(1)
                    && observed!((self.shared.tile_z)(at.0)) >= town_height.wrapping_sub(1)
                {
                    counter += 1;
                }
                spiral = spiral.advance(self.map().size_x(), self.size_y());
            }
            if counter < VALID_TILE_GOAL {
                return Some(self.s.unsuitable_error);
            }
        }
        None
    }
    /// Find a good coastal town spot from a presumed water tile.
    /// Avoid building too close to the edge, which inhibits city growth, with a
    /// search within a search within a search. O(n*m^2), where n is how far to
    /// search for land and m is how far inland to look for a flat spot.
    /// `town_cmd.cpp::FindNearestGoodCoastalTownSpot`.
    fn find_nearest_good_coastal_town_spot(
        &self,
        tile: TileIndex,
        layout: TownLayout,
    ) -> TileIndex {
        w::hit(w::TOWN_COAST);
        let mut coast = self.spiral(tile, 40);
        while !coast.is_end() {
            let at = TileIndex(
                coast
                    .x
                    .wrapping_add(coast.y.wrapping_mul(self.map().size_x())),
            );
            // Find nearest land tile.
            if TileType(observed!((self.shared.tile_type)(at.0))) == TileType::MP_CLEAR {
                let mut furthest = TileIndex::INVALID;
                let mut max_dist = 0;
                let mut test = self.spiral(at, 10);
                while !test.is_end() {
                    let tile = TileIndex(
                        test.x
                            .wrapping_add(test.y.wrapping_mul(self.map().size_x())),
                    );
                    if TileType(observed!((self.shared.tile_type)(tile.0))) == TileType::MP_CLEAR
                        && observed!((self.s.flat)(tile))
                        && self.is_tile_aligned_to_grid(tile, layout)
                        && self.town_can_be_placed_here(tile, true).is_none()
                    {
                        let dist = observed!((self.s.water_distance)(tile, true));
                        if dist > max_dist {
                            furthest = tile;
                            max_dist = dist;
                        }
                    }
                    test = test.advance(self.map().size_x(), self.size_y());
                }
                return furthest;
            }
            coast = coast.advance(self.map().size_x(), self.size_y());
        }
        // If we get here just give up.
        TileIndex::INVALID
    }
    /// Actually create a town (`town_cmd.cpp::DoCreateTown`).
    ///
    /// `town` is the town, `tile` is where to put it, and `townnameparts` is its name.
    /// `size` is the preset size; `city` selects a city; `layout` is the road layout.
    /// `manual` records whether the town was placed manually.
    unsafe fn do_create_town(
        &self,
        town: TownRef,
        tile: TileIndex,
        townnameparts: u32,
        size: TownSize,
        city: bool,
        layout: TownLayout,
        manual: bool,
    ) {
        w::hit(w::TOWN_GENERATED);
        if manual {
            w::hit(w::TOWN_MANUAL);
        }
        if layout == TownLayout::TL_2X2_GRID || layout == TownLayout::TL_3X3_GRID {
            w::hit(w::TOWN_GRID);
        }
        let previous = super::openttd_rust_town_generating_exchange_abi(true) != 0;
        // SAFETY: Immediate canonical field writes; no borrow spans callbacks.
        unsafe {
            (*town.state).simulation.xy = tile.0;
            (*town.state).simulation.num_houses = 0;
            (*town.state).simulation.time_until_rebuild = 10;
            update_radius(town.state);
            (*town.state).growth.flags = 0;
            (*town.state).simulation.population = 0;
        }
        // SAFETY: Fresh canonical owner; no count view exists yet.
        unsafe {
            super::resolver::openttd_rust_house_counts_init(
                self.leaves,
                self.shared,
                town.state,
                (self.leaves.house_count)() as usize,
            );
        };
        // Spread growth across ticks so even if there are many
        // similar towns they're unlikely to grow all in one tick.
        unsafe {
            (*town.state).growth.counter = town.id.0 % TOWN_GROWTH_TICKS;
            (*town.state).growth.rate = town_ticks_to_game_ticks(250);
        }
        // Town's native default initializes show_zone=false before this creation entry.
        super::spatial::insert(self.shared, town.id);
        // Set the default cargo requirement for town growth.
        match LandscapeType(observed!((self.leaves.climate)())) {
            LandscapeType::Arctic => {
                if crate::cargo_registry::first_town_acceptance(TownAcceptanceEffect::TAE_FOOD.0)
                    .is_some()
                {
                    unsafe {
                        (*town.state).simulation.goal
                            [usize::from(TownAcceptanceEffect::TAE_FOOD.0)] = TOWN_GROWTH_WINTER;
                    }
                }
            }
            LandscapeType::Tropic => {
                if crate::cargo_registry::first_town_acceptance(TownAcceptanceEffect::TAE_FOOD.0)
                    .is_some()
                {
                    unsafe {
                        (*town.state).simulation.goal
                            [usize::from(TownAcceptanceEffect::TAE_FOOD.0)] = TOWN_GROWTH_DESERT;
                    }
                }
                if crate::cargo_registry::first_town_acceptance(TownAcceptanceEffect::TAE_WATER.0)
                    .is_some()
                {
                    unsafe {
                        (*town.state).simulation.goal
                            [usize::from(TownAcceptanceEffect::TAE_WATER.0)] = TOWN_GROWTH_DESERT;
                    }
                }
            }
            _ => {}
        }
        unsafe {
            (*town.state).growth.funding = 0;
            (*town.state).simulation.ratings = [RATING_INITIAL as i16; MAX_COMPANIES as usize];
            (*town.state).simulation.have_ratings = 0;
            (*town.state).simulation.exclusivity = CompanyID::INVALID.0;
            (*town.state).simulation.exclusive_counter = 0;
            (*town.state).simulation.statues = 0;
        }
        let parameters =
            unsafe { crate::townname_type::from_style(observed!((self.s.name_style)())) };
        unsafe {
            (*town.state).simulation.townnamegrfid = parameters.grfid;
            (*town.state).simulation.townnametype = parameters.kind;
            (*town.state).simulation.townnameparts = townnameparts;
            initialize_layout(town.state, layout, self.map().size_x());
            (*town.state).simulation.larger_town = city;
        }
        let mut x = i32::from(size.0).wrapping_mul(16).wrapping_add(3);
        if size == TownSize::TSZ_RANDOM {
            x = (self.random() & 15).wrapping_add(8) as i32;
        }
        // Don't create huge cities when founding town in-game.
        if city && (!manual || observed!((self.leaves.environment.editor)())) {
            x = x.wrapping_mul(i32::from(observed!((self.s.initial_city_size)())));
        }
        let engine = self.engine(town);
        engine.change_house_count(x as u32);
        unsafe {
            update_radius(town.state);
        }
        let mut i = x.wrapping_mul(4);
        loop {
            w::hit(w::TOWN_GROWTH);
            engine.grow(
                u32::from(town.id.0),
                u32::from(TOWN_EXPAND_BUILDINGS | TOWN_EXPAND_ROADS),
            );
            i = i.wrapping_sub(1);
            if i == 0 {
                break;
            }
        }
        observed!((self.s.update_virt)(town.shell));
        observed!((self.s.directory_rebuild)());
        engine.change_house_count((x as u32).wrapping_neg());
        unsafe {
            update_radius(town.state);
        }
        engine.update_rate(u32::from(town.id.0));
        super::policy::update_town_max_pass(self.leaves, town.state);
        observed!((self.s.update_noise)());
        super::openttd_rust_town_generating_exchange_abi(previous);
    }
    /// Create a random town somewhere in the world (`town_cmd.cpp::CreateRandomTown`).
    unsafe fn create_random_town(
        &self,
        mut attempts: u32,
        townnameparts: u32,
        size: TownSize,
        city: bool,
        layout: TownLayout,
    ) -> Option<TownRef> {
        // Editor or world generation is required by CMD_DELETE_TOWN.
        observed!((self.s.assert_random_context)());
        if !observed!((self.s.can_allocate)()) {
            return None;
        }
        loop {
            // Generate a tile index not too close to the edge.
            let mut tile: TileIndex = self.align_tile_to_grid(
                Tile(
                    self.random()
                        & self
                            .map()
                            .size_x()
                            .wrapping_mul(self.size_y())
                            .wrapping_sub(1),
                ),
                layout,
            );
            // If the random tile is water, find suitable land nearby; otherwise evaluate it.
            let valid =
                if TileType(observed!((self.shared.tile_type)(tile.0))) == TileType::MP_WATER {
                    tile = self.find_nearest_good_coastal_town_spot(tile, layout);
                    tile != TileIndex::INVALID
                } else {
                    self.town_can_be_placed_here(tile, true).is_none()
                };
            if valid {
                // Allocate a town struct.
                let town = observed!((self.s.allocate)(tile));
                unsafe {
                    self.do_create_town(town, tile, townnameparts, size, city, layout, false);
                }
                // A population still at zero means the placement was too bad to grow at all.
                if unsafe { super::population(town.state) } > 0 {
                    return Some(town);
                }
                w::hit(w::TOWN_ZERO_GROWTH);
                let cur_company =
                    observed!((self.leaves.exchange_company)(u32::from(CompanyID::TOWN.0)));
                observed!((self.s.delete_town)(town.id));
                observed!((self.leaves.exchange_company)(cur_company));
                // Creating and deleting resets allocation checks. We could allocate on entry,
                // but must check again when assertions are enabled.
                observed!((self.s.assert_allocation)());
            }
            w::hit(w::TOWN_RETRY);
            attempts = attempts.wrapping_sub(1);
            if attempts == 0 {
                break;
            }
        }
        None
    }
    /// Calculate the map-size-scaled town density, or return the custom newgame count.
    fn get_default_towns_for_map_size(&self) -> u32 {
        const NUM_INITIAL_TOWNS: [u32; 4] = [5, 11, 23, 46]; // Very low, low, normal, high.
        let number_towns = observed!((self.s.town_density)());
        if u32::from(number_towns) == super::CUSTOM_TOWN_NUMBER_DIFFICULTY {
            return u32::from(observed!((self.s.custom_number)()));
        }
        // Native setting's original array domain; no extra rejection is added.
        let value = unsafe { *NUM_INITIAL_TOWNS.get_unchecked(usize::from(number_towns)) };
        observed!((self.s.scale_size)(value))
    }
    unsafe fn generate_town_name(
        &self,
        name_frame: *mut names::NameFrame,
        scratch: *mut names::Names,
        result: *mut u32,
    ) -> bool {
        let parameters =
            unsafe { crate::townname_type::from_style(observed!((self.s.name_style)())) };
        unsafe {
            names::generate(
                self.names,
                name_frame,
                self.s.random,
                result,
                &raw const parameters,
                scratch,
            )
        }
    }
}
/// Assign the town layout (`town_cmd.cpp::Town::InitializeLayout`).
/// `layout` is the desired layout. If `TL_RANDOM`, pick one based on `TileHash`.
unsafe fn initialize_layout(state: *mut State, layout: TownLayout, size_x: u32) {
    let result = if layout == TownLayout::TL_RANDOM {
        let xy = unsafe { super::position(state) };
        let x = xy % size_x;
        let y = xy / size_x;
        let hash = ((x >> 4) ^ (x >> 6) ^ (y >> 4)).wrapping_sub(y >> 6);
        (hash % u32::from(TownLayout::NUM_TLS.0 - 1)) as u8
    } else {
        layout.0
    };
    unsafe {
        (*state).simulation.layout = result;
    }
}
/// `town_cmd.cpp::GetDefaultTownsForMapSize`.
/// # Safety
/// The immutable native tables and their pointed-to services remain live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_default_number(s: *const FoundingServices) -> u32 {
    unsafe { Founding::new(&*s) }.get_default_towns_for_map_size()
}
/// # Safety
/// Tables, native stack frames, command cost and input bytes remain live for this call. Owners are supplied by the pool and no borrow crosses a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_found(
    s: *const FoundingServices,
    road: *const crate::road_cmd::CommandContext,
    frame: *mut Frame,
    name_frame: *mut names::NameFrame,
    result: *mut CommandCost,
    tile: Tile,
    size: TownSize,
    city: bool,
    layout: TownLayout,
    random: bool,
    parts: u32,
    text: ByteSpan,
    execute: bool,
) {
    const MULT: [[i32; 4]; 2] = [[15, 25, 40, 25], [20, 35, 55, 35]];
    let mut f = unsafe { Founding::new(&*s) };
    f.road = road;
    w::hit(w::TOWN_ENTRIES);
    let parameters = unsafe { crate::townname_type::from_style(observed!((f.s.name_style)())) };
    observed!((f.cost.error)(result, 0xffff));
    if size.0 >= 4 || layout.0 >= 5 {
        w::hit(w::TOWN_INVALID);
        return;
    }
    let editor = observed!((f.leaves.environment.editor)());
    let company = CompanyId(observed!((f.shared.current_company)()));
    if editor {
        w::hit(w::TOWN_EDITOR);
    }
    if company.0 == 0x12 {
        w::hit(w::TOWN_DEITY);
    }
    if !editor && company.0 != 0x12 {
        let setting = observed!((f.s.found_setting)());
        if setting.0 == 0 || size.0 == 2 || random {
            return;
        }
        if setting.0 != 2 && layout != observed!((f.s.layout_setting)()) {
            return;
        }
    } else if company.0 == 0x12 && random {
        return;
    }
    if text.size == 0 {
        if !unsafe {
            names::verify(
                f.names,
                name_frame,
                parts,
                &raw const parameters,
                std::ptr::null_mut(),
            )
        } {
            w::hit(w::TOWN_DUPLICATE);
            observed!((f.cost.error)(result, f.s.unique_error));
            return;
        }
    } else {
        if observed!((f.names.utf8_length)(text.data, text.size)) >= 32 {
            return;
        }
        if !unsafe { names::unique_custom(f.names, text) } {
            w::hit(w::TOWN_DUPLICATE);
            observed!((f.cost.error)(result, f.s.unique_error));
            return;
        }
    }
    if !observed!((f.s.can_allocate)()) {
        observed!((f.cost.error)(result, f.s.too_many_error));
        return;
    }
    if !random {
        if let Some(error) = f.town_can_be_placed_here(tile, false) {
            observed!((f.cost.error)(result, error));
            return;
        }
    }
    observed!((f.s.initialize_cost)(
        result,
        MULT[usize::from(city)][usize::from(size.0)],
    ));
    if execute {
        if observed!((f.cost.money)(result))
            > crate::command::available_money(unsafe { &*f.s.command })
        {
            w::hit(w::TOWN_MONEY);
            observed!((f.s.money_deficit)(frame, result));
            observed!((f.cost.expense_money)(result, 12, 0));
            return;
        }
        let previous = observed!((f.s.exchange_generating)(true));
        unsafe {
            crate::road_cmd::openttd_rust_road_nearest_towns((*road).services, true);
        }
        let town = if random {
            unsafe { f.create_random_town(20, parts, size, city, layout) }
        } else {
            let town = observed!((f.s.allocate)(tile));
            unsafe {
                f.do_create_town(town, tile, parts, size, city, layout, true);
            }
            Some(town)
        };
        unsafe {
            crate::road_cmd::openttd_rust_road_nearest_towns((*road).services, false);
        }
        observed!((f.s.exchange_generating)(previous));
        let Some(town) = town else {
            observed!((f.cost.error)(result, f.s.no_space_error));
            return;
        };
        observed!((f.s.set_result)(frame, town.id));
        if text.size != 0 {
            unsafe {
                names::openttd_rust_town_set_name(town.state, text);
            }
            observed!((f.s.name_changed)(town.shell, unsafe {
                names::openttd_rust_town_name(town.state)
            }));
            observed!((f.s.update_virt)(town.shell));
        }
        if !observed!((f.leaves.environment.editor)()) {
            observed!((f.s.founded_news)(town.id, tile));
            observed!((f.s.founded_events)(town.id));
        }
    }
}
/// Generate towns for the Scenario Editor Random Towns button or world generation.
/// `has_number == false` uses the game settings (`town_cmd.cpp::GenerateTowns`).
/// # Safety
/// Tables and native stack frames remain live; result is writable. Only named progress services report cancellation; scratch drops before native rethrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_generate_abi(
    s: *const FoundingServices,
    road: *const crate::road_cmd::CommandContext,
    frame: *mut Frame,
    name_frame: *mut names::NameFrame,
    layout: TownLayout,
    has_number: bool,
    number: u32,
    result: *mut bool,
) -> u8 {
    let mut f = unsafe { Founding::new(&*s) };
    f.road = road;
    let mut current_number = 0_u32;
    let mut total = if has_number {
        number
    } else if u32::from(observed!((f.s.town_density)())) == super::CUSTOM_TOWN_NUMBER_DIFFICULTY {
        f.get_default_towns_for_map_size()
    } else {
        observed!((f.s.scale_land)(
            f.get_default_towns_for_map_size()
                .wrapping_add(f.random() & 7),
        ))
    }
    .clamp(1, u32::from(TownID::END.0));
    let mut townnameparts = 0;
    let mut town_names = names::Names::new();
    w::hit(w::TOWN_ENTRIES);
    if !observed!((f.s.set_progress)(frame, total)) {
        w::hit(w::TOWN_CANCELLED);
        return u8::from(false);
    }
    // Pre-populate the name list with the names of towns already on the map.
    let mut town = observed!((f.names.next)(u32::from(TownID::BEGIN.0)));
    while !town.shell.is_null() {
        let custom = unsafe { names::openttd_rust_town_name(town.state) };
        let name = if custom.size == 0 {
            observed!((f.s.cached_name)(town.shell))
        } else {
            custom
        };
        town_names.insert(unsafe { names::bytes(name) }.to_vec());
        town = observed!((f.names.next)(u32::from(town.id.0).wrapping_add(1)));
    }
    // Randomise the city-status offset: at 1 in 4, ten towns may yield two or three cities.
    let larger_towns = observed!((f.s.larger_towns)());
    let city_random_offset = if larger_towns == 0 {
        w::hit(w::TOWN_NO_CITIES);
        0
    } else {
        f.random() % u32::from(larger_towns)
    };
    // The requested number is a suggestion, so creation cannot lock up on a crowded map.
    loop {
        let larger_towns = observed!((f.s.larger_towns)());
        let city = larger_towns != 0
            && city_random_offset.wrapping_add(current_number) % u32::from(larger_towns) == 0;
        if !observed!((f.s.increase_progress)(frame)) {
            w::hit(w::TOWN_CANCELLED);
            return u8::from(false);
        }
        // Get a unique town name, then try 20 times to create a random-sized town.
        if unsafe { f.generate_town_name(name_frame, &raw mut town_names, &raw mut townnameparts) }
            && unsafe {
                f.create_random_town(20, townnameparts, TownSize::TSZ_RANDOM, city, layout)
            }
            .is_some()
        {
            // If creation succeeded, raise the flag.
            current_number = current_number.wrapping_add(1);
        }
        total = total.wrapping_sub(1);
        if total == 0 {
            break;
        }
    }
    town_names.clear();
    // Rebuild the town k-d tree so it is well balanced.
    observed!(crate::town::spatial::rebuild(f.shared));
    if current_number != 0 {
        unsafe {
            result.write(true);
        }
        return u8::from(true);
    }
    // No town was created. Give it a final, more aggressive try.
    w::hit(w::TOWN_FALLBACK);
    let success =
        unsafe { f.generate_town_name(name_frame, std::ptr::null_mut(), &raw mut townnameparts) }
            && unsafe {
                f.create_random_town(
                    10000,
                    townnameparts,
                    TownSize::TSZ_RANDOM,
                    observed!((f.s.larger_towns)()) != 0,
                    layout,
                )
            }
            .is_some();
    // If there are no towns at all while generating a new game, bail out.
    if !success
        && observed!((f.shared.town_count)()) == 0
        && !observed!((f.leaves.environment.editor)())
    {
        observed!((f.s.no_town_error)());
    }
    unsafe {
        result.write(success);
    }
    u8::from(true)
}
/// # Safety
/// Tables, command cost and input bytes remain live under the game mutex. Pool resolution supplies the sole owner; mutation ends before native callbacks.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_rename(
    s: *const FoundingServices,
    result: *mut CommandCost,
    id: TownId,
    text: ByteSpan,
    execute: bool,
) {
    let f = unsafe { Founding::new(&*s) };
    w::hit(w::TOWN_ENTRIES);
    w::hit(w::TOWN_RENAME);
    let town = observed!((f.s.resolve)(id));
    observed!((f.cost.error)(result, 0xffff));
    if town.shell.is_null() {
        return;
    }
    if text.size != 0 {
        if observed!((f.names.utf8_length)(text.data, text.size)) >= 32 {
            return;
        }
        if !unsafe { names::unique_custom(f.names, text) } {
            w::hit(w::TOWN_DUPLICATE);
            observed!((f.cost.error)(result, f.s.unique_error));
            return;
        }
    }
    if execute {
        observed!((f.s.prepare_rename)(town.shell));
        unsafe {
            names::openttd_rust_town_set_name(town.state, text);
        }
        observed!((f.s.name_changed)(town.shell, unsafe {
            names::openttd_rust_town_name(town.state)
        }));
        observed!((f.s.update_virt)(town.shell));
        observed!((f.s.directory_resort)());
        observed!((f.s.clear_station_names)());
        observed!((f.s.clear_industry_names)());
        observed!((f.s.update_station_coords)());
    }
    observed!((f.cost.zero)(result));
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTownFoundingServices" ["probe"] => FoundingServices {
        shared,
        growth,
        names,
        cost,
        allocate,
        resolve,
        can_allocate,
        flat,
        water_distance,
        scale_size,
        scale_land,
        found_setting,
        layout_setting,
        initial_city_size,
        town_density,
        custom_number,
        larger_towns,
        name_style,
        prepare_rename,
        name_changed,
        update_virt,
        directory_rebuild,
        directory_resort,
        update_noise,
        delete_town,
        assert_random_context,
        assert_allocation,
        initialize_cost,
        command,
        money_deficit,
        set_result,
        exchange_generating,
        founded_news,
        founded_events,
        clear_station_names,
        clear_industry_names,
        update_station_coords,
        cached_name,
        set_progress,
        increase_progress,
        no_town_error,
        random,
        edge_error,
        close_error,
        unsuitable_error,
        unique_error,
        too_many_error,
        no_space_error,
    };
}
