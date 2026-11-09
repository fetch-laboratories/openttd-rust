/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Town commands, authority actions and ordered deletion. Post-destruction code never
//! resolves an ID.
use super::founding::{AcceptanceEffect, FoundingServices};
use super::{Engine, State, TownAction, TownActions, TownId, TownRef, engine};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, OWNER_NONE, OWNER_TOWN};
use crate::economy_type::PR_TOWN_ACTION;
use crate::objects::{Object, ObjectServices, QueryServices};
use crate::services::{ByteSpan, CommandCost, CommandCostServices, Map, Services, TownHandle};
use crate::slope_func::is_steep_slope;
use crate::station::BaseStation;
use crate::station::Station;
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{TileIndex, TileType};
use crate::town_type::{
    RATING_BRIBE_DOWN_TO, RATING_BRIBE_MAXIMUM, RATING_BRIBE_UP_STEP, RATING_MAXIMUM,
    RATING_MINIMUM,
};
use crate::witness as w;
use std::cell::OnceCell;
#[repr(C)]
pub struct Relation {
    pub next: u32,
    pub town: *const TownHandle,
}
#[repr(C)]
pub struct StationCursor {
    pub shell: *mut BaseStation,
    pub next: u32,
}
#[repr(C)]
pub struct CommandServices {
    pub founding: *const FoundingServices,
    pub rail: *const crate::services::RailMapServices,
    pub cargo: *const crate::cargo_storage::Services,
    pub objects: *const ObjectServices,
    pub subsidies: *const crate::subsidy::Services,
    pub fund_roads: extern "C" fn() -> bool,
    pub fund_buildings: extern "C" fn() -> bool,
    pub exclusive_rights: extern "C" fn() -> bool,
    pub bribe: extern "C" fn() -> bool,
    pub invalidate_view: extern "C" fn(u16),
    pub invalidate_authority: extern "C" fn(u16),
    pub road_news: extern "C" fn(u16),
    pub road_events: extern "C" fn(u16),
    pub rights_news: extern "C" fn(u16),
    pub rights_events: extern "C" fn(u16),
    pub station_views_dirty: extern "C" fn(),
    pub next_station: extern "C" fn(u32) -> StationCursor,
    pub station_town: extern "C" fn(*mut BaseStation) -> *const TownHandle,
    pub station_owner: extern "C" fn(*const BaseStation) -> u8,
    pub station_airport: extern "C" fn(*const Station) -> bool,
    pub airport_type: extern "C" fn(*const BaseStation) -> u8,
    pub airport_tile: extern "C" fn(*const BaseStation) -> u32,
    pub station_activity_owner:
        extern "C" fn(*const Station) -> *const crate::station_service::Service,
    pub local_company: extern "C" fn() -> bool,
    pub bribe_error: extern "C" fn(),
    pub next_waypoint: extern "C" fn(u32) -> Relation,
    pub next_depot: extern "C" fn(u32) -> Relation,
    pub next_industry: extern "C" fn(u32) -> Relation,
    pub set_object_town: extern "C" fn(*mut Object, *mut TownHandle),
    pub industry_town_by_tile: extern "C" fn(u32) -> *mut TownHandle,
    pub remove_town_sign: extern "C" fn(*mut TownHandle),
    pub delete_town: extern "C" fn(*mut TownHandle),
    pub cleaning_pool: extern "C" fn() -> bool,
    pub close_view: extern "C" fn(u16),
    pub close_graph: extern "C" fn(u16),
    pub storage_count: extern "C" fn(*mut TownHandle) -> usize,
    pub delete_storage: extern "C" fn(*mut TownHandle, usize),
    pub clear_storage: extern "C" fn(*mut TownHandle),
    pub delete_inspect: extern "C" fn(u16),
    pub assertions: bool,
    pub too_many_objects: u32,
    pub no_statue_place: u32,
}
struct Commands<'a> {
    s: &'a CommandServices,
    f: &'a FoundingServices,
    shared: &'a Services,
    cost: &'a CommandCostServices,
    map: OnceCell<Map>,
    world: OnceCell<crate::map_func::World<'a>>,
    size_y: OnceCell<u32>,
}
impl<'a> Commands<'a> {
    unsafe fn new(s: &'a CommandServices) -> Self {
        // SAFETY: Immutable tables live for the complete synchronous call.
        let f = unsafe { &*s.founding };
        Self {
            s,
            f,
            shared: unsafe { &*f.shared },
            cost: unsafe { &*f.cost },
            map: OnceCell::new(),
            world: OnceCell::new(),
            size_y: OnceCell::new(),
        }
    }
    fn objects(&self) -> &ObjectServices {
        // SAFETY: Canonical immutable object services outlive the synchronous call.
        unsafe { &*self.s.objects }
    }
    fn object_query(&self) -> &QueryServices {
        // SAFETY: The canonical query table has the same lifetime.
        unsafe { &*self.objects().query }
    }
    fn resolve(&self, id: TownId) -> TownRef {
        (self.f.resolve)(id)
    }
    fn current_company(&self) -> CompanyID {
        CompanyID((self.shared.current_company)())
    }
    fn tile_type(&self, tile: TileIndex) -> TileType {
        TileType((self.shared.tile_type)(tile.0))
    }
    fn map(&self) -> Map {
        *self.map.get_or_init(|| self.shared.map())
    }
    fn town_owned_road(&self, tile: TileIndex) -> bool {
        let world = self
            .world
            .get_or_init(|| crate::map_func::World::new(self.shared));
        crate::road_map::has_town_owned_road(world, tile.0)
    }
    fn size_y(&self) -> u32 {
        *self
            .size_y
            .get_or_init(|| (self.shared.map_max_y)().wrapping_add(1))
    }
    fn map_size(&self) -> u32 {
        self.map().size_x() * self.size_y()
    }
    fn growth(&self, town: TownRef) -> Engine<'a> {
        let mut e = engine(self.f.growth, self.f.shared);
        e.owner = town.state;
        let _ = e.house_shell.set(town.shell);
        e
    }
    fn error(&self, out: *mut CommandCost) {
        (self.cost.error)(out, INVALID_STRING_ID);
    }
    fn deity(&self, out: *mut CommandCost, id: TownId) -> Option<TownRef> {
        if self.current_company() != CompanyID::DEITY {
            self.error(out);
            return None;
        }
        let t = self.resolve(id);
        if t.shell.is_null() {
            self.error(out);
            None
        } else {
            Some(t)
        }
    }
    /// `Command<CMD_LANDSCAPE_CLEAR>::Do(flags, tile)`, returning whether it succeeded.
    fn clear(&self, out: *mut CommandCost, tile: TileIndex, flags: DoCommandFlags) -> bool {
        // SAFETY: Native command table and caller-owned result remain live.
        crate::landscape_cmd::clear_command(unsafe { &*self.f.command }, out, tile.0, flags.0);
        (self.cost.success)(out)
    }
    /// Clear the land as `OWNER_NONE`, restoring the current company afterwards; with empty
    /// flags this is `CheckClearTile`, checking whether the land can be cleared.
    fn check_clear_tile(
        &self,
        out: *mut CommandCost,
        tile: TileIndex,
        flags: DoCommandFlags,
    ) -> bool {
        // SAFETY: Immutable table; original Backup scope restores after the command.
        let command = unsafe { &*self.f.command };
        let old = (command.finance.current_company)();
        (command.finance.set_current_company)(OWNER_NONE);
        let okay = self.clear(out, tile, flags);
        (command.finance.set_current_company)(old);
        okay
    }
    /// Get a list of available town authority actions (`GetMaskOfTownActions`).
    /// Return the bitmasked value of enabled actions.
    fn get_mask_of_town_actions(&self, cid: CompanyID, t: TownRef) -> TownActions {
        let mut buttons = TownActions::EMPTY;
        // Spectators and unwanted have no options.
        if cid != CompanyID::SPECTATOR
            && !((self.s.bribe)()
                && unsafe { (*t.state).simulation.unwanted[usize::from(cid.0)] } != 0)
        {
            // Actions worth more than this are not able to be performed.
            // SAFETY: Canonical resolver returns a live owner or null as GetAvailableMoney permits.
            let c = unsafe { &*self.f.command };
            let avail = unsafe {
                crate::company::openttd_rust_company_available_money(
                    (c.finance.company_finances)(cid.0),
                    (c.infinite_money)(),
                )
            };
            // Check the action bits for validity and if they are valid add them.
            for cur in (0..TownAction::End.0).map(TownAction) {
                // Is the company prohibited from bribing?
                if cur == TownAction::Bribe {
                    // Company can't bribe if setting is disabled.
                    if !(self.s.bribe)() {
                        continue;
                    }
                    // Company can bribe if another company has exclusive transport rights,
                    // or its standing with the town is less than outstanding.
                    if i32::from(unsafe { (*t.state).simulation.ratings[usize::from(cid.0)] })
                        >= RATING_BRIBE_MAXIMUM
                    {
                        if unsafe { (*t.state).simulation.exclusivity }
                            == (self.shared.current_company)()
                        {
                            continue;
                        }
                        if unsafe { (*t.state).simulation.exclusive_counter } == 0 {
                            continue;
                        }
                    }
                }
                // Is the company not able to buy exclusive rights?
                if cur == TownAction::BuyRights
                    && (!(self.s.exclusive_rights)()
                        || unsafe { (*t.state).simulation.exclusive_counter } != 0)
                {
                    continue;
                }
                // Is the company not able to fund buildings?
                if cur == TownAction::FundBuildings && !(self.s.fund_buildings)() {
                    continue;
                }
                // Is the company not able to fund local road reconstruction?
                if cur == TownAction::RoadRebuild && !(self.s.fund_roads)() {
                    continue;
                }
                // Is the company not able to build a statue?
                if cur == TownAction::BuildStatue
                    && unsafe { (*t.state).simulation.statues } & (1 << cid.0) != 0
                {
                    continue;
                }
                let cost = i64::from(get_town_action_cost(cur))
                    .saturating_mul(crate::company::price(PR_TOWN_ACTION))
                    >> 8;
                if avail >= cost {
                    buttons.set(cur);
                }
            }
        }
        buttons
    }
    /// Perform a 9x9 tiles circular search from the center of the town in order to find a
    /// free tile to place a statue (`TownActionBuildStatue`). `flags` decide whether the
    /// statue must be built. Return false after writing the error.
    fn town_action_build_statue(
        &self,
        t: TownRef,
        out: *mut CommandCost,
        flags: DoCommandFlags,
    ) -> bool {
        // Number of tiles in the center of the city, where we try to protect houses.
        const STATUE_NUMBER_INNER_TILES: u32 = 25;
        if !unsafe { (self.objects().can_allocate)() } {
            (self.cost.error)(out, self.s.too_many_objects);
            return false;
        }
        let center = unsafe { super::position(t.state) };
        let mut spiral = crate::spiral::square(
            self.map().tile_x(center.0),
            self.map().tile_y(center.0),
            9,
            self.map().size_x(),
            self.size_y(),
        );
        // Best position found so far.
        let mut best_position = TileIndex::INVALID;
        // Number of tiles tried.
        let mut tile_count = 0;
        while !spiral.is_end() {
            let tile = TileIndex(self.map().tile_xy(spiral.x, spiral.y));
            spiral = spiral.advance(self.map().size_x(), self.size_y());
            tile_count += 1;
            // Statues can be build on slopes, just like houses. Only the steep slopes is a no go.
            if is_steep_slope(crate::map_func::World::new(self.shared).tile_slope(tile.0)) {
                continue;
            }
            // Don't build statues under bridges.
            if crate::map_func::World::new(self.shared).bridge_above(tile.0) {
                continue;
            }
            // A clear-able open space is always preferred.
            if matches!(
                self.tile_type(tile),
                TileType::MP_CLEAR | TileType::MP_TREES
            ) && self.check_clear_tile(out, tile, DoCommandFlags::EMPTY)
            {
                best_position = tile;
                break;
            }
            let house = self.tile_type(tile) == TileType::MP_HOUSE;
            // Searching inside the inner circle.
            if tile_count <= STATUE_NUMBER_INNER_TILES {
                // Save first house in inner circle.
                if house
                    && best_position == TileIndex::INVALID
                    && self.check_clear_tile(out, tile, DoCommandFlags::EMPTY)
                {
                    best_position = tile;
                }
                // If we have reached the end of the inner circle, and have a saved house,
                // terminate the search.
                if tile_count == STATUE_NUMBER_INNER_TILES && best_position != TileIndex::INVALID {
                    break;
                }
            }
            // Searching outside the circle, just pick the first possible spot.
            if !house || !self.check_clear_tile(out, tile, DoCommandFlags::EMPTY) {
                continue;
            }
            best_position = tile;
            break;
        }
        if best_position == TileIndex::INVALID {
            (self.cost.error)(out, self.s.no_statue_place);
            return false;
        }
        if flags.test(DoCommandFlag::Execute) {
            self.check_clear_tile(
                out,
                best_position,
                DoCommandFlags::from_element(DoCommandFlag::Execute),
            );
            let company = (self.shared.current_company)();
            // SAFETY: Live canonical tables and shell; no town-state borrow spans object callbacks.
            unsafe {
                crate::objects::build(
                    self.objects(),
                    crate::object_type::OBJECT_STATUE,
                    best_position.0,
                    company,
                    t.shell,
                    0,
                );
            }
            // Once found and built, "inform" the Town.
            let company = (self.shared.current_company)();
            unsafe {
                (*t.state).simulation.statues |= 1 << company;
            }
            (self.shared.mark_dirty)(best_position.0);
            w::hit(w::TOWN_COMMAND_STATUE);
        }
        true
    }
    /// Perform the "bribe" town action (`TownActionBribe`) when executing.
    fn town_action_bribe(&self, t: TownRef) {
        let company = (self.shared.current_company)();
        if crate::random_func::chance16_i(1, 14, self.shared.random()) {
            // Set as unwanted for 6 months.
            unsafe {
                (*t.state).simulation.unwanted[usize::from(company)] = 6;
            }
            // Set all close by station ratings to 0.
            let mut from = 0;
            loop {
                let st = (self.s.next_station)(from);
                if st.shell.is_null() {
                    break;
                }
                from = st.next;
                if (self.s.station_town)(st.shell) == t.shell
                    && (self.s.station_owner)(st.shell) == company
                {
                    let owner = (self.s.station_activity_owner)(st.shell.cast());
                    // SAFETY: Station stays live; its canonical Rust owner owns all 64 cargo slots.
                    unsafe {
                        crate::station_service::clear_ratings(owner.cast_mut());
                    }
                }
            }
            // Only show error message to the executing player. All errors are handled
            // command.c but this is special, because it can only 'fail' on a
            // DoCommandFlag::Execute.
            if (self.s.local_company)() {
                (self.s.bribe_error)();
            }
            // Decrease by a lot! ChangeTownRating is only for stuff in demolishing.
            // Bribe failure should be independent of any cheat settings.
            if i32::from(unsafe { (*t.state).simulation.ratings[usize::from(company)] })
                > RATING_BRIBE_DOWN_TO
            {
                unsafe {
                    (*t.state).simulation.ratings[usize::from(company)] =
                        RATING_BRIBE_DOWN_TO as i16;
                }
                (unsafe { &*self.f.growth }.dirty_authority)(u32::from(t.id.0));
            }
            w::hit(w::TOWN_COMMAND_BRIBE_FAILED);
        } else {
            // SAFETY: The town owner is live; the rating owner keeps its scratch and cheat semantics.
            unsafe {
                super::openttd_rust_town_change_rating(
                    self.f.growth,
                    self.f.shared,
                    t.state,
                    u32::from(t.id.0),
                    t.shell as usize,
                    company,
                    RATING_BRIBE_UP_STEP,
                    RATING_BRIBE_MAXIMUM,
                    false,
                    ((*(*self.f.growth).houses)
                        .authority
                        .as_ref()
                        .unwrap()
                        .bulldozer)(),
                );
            }
            if unsafe { (*t.state).simulation.exclusivity } != company
                && unsafe { (*t.state).simulation.exclusivity } != CompanyID::INVALID.0
            {
                unsafe {
                    (*t.state).simulation.exclusivity = CompanyID::INVALID.0;
                    (*t.state).simulation.exclusive_counter = 0;
                }
                w::hit(w::TOWN_COMMAND_RIGHTS_CANCEL);
            }
            w::hit(w::TOWN_COMMAND_BRIBE_SUCCESS);
        }
    }
    /// Check if a town 'owns' a bridge (`TestTownOwnsBridge`).
    /// Bridges do not directly have an owner, so we check the tiles adjacent to the bridge
    /// ends. If either adjacent tile belongs to the town then it will be assumed that the
    /// town built the bridge.
    fn test_town_owns_bridge(&self, tile: TileIndex, t: TownRef) -> bool {
        if !crate::tile_map::is_tile_owner(self.shared, tile.0, OWNER_TOWN) {
            return false;
        }
        let adjacent = TileIndex(tile.0.wrapping_add(self.map().offs_by_diag_dir(
            crate::direction_func::reverse_diag_dir(
                (self.shared.tunnel_bridge_direction)(tile.0),
                |ok| crate::movement::native_assert(self.shared, ok),
            ),
        )));
        let owns = |at: TileIndex| {
            self.tile_type(at) == TileType::MP_ROAD
                && crate::tile_map::is_tile_owner(self.shared, at.0, OWNER_TOWN)
                && TownId((self.shared.town_index)(at.0)) == t.id
        };
        if owns(adjacent) {
            return true;
        }
        // Or other adjacent road.
        let end = (unsafe { &*self.s.rail }.other_tunnel_bridge_end)(tile.0);
        // Historical source adds the other end's direction to the ORIGINAL tile.
        owns(TileIndex(tile.0.wrapping_add(self.map().offs_by_diag_dir(
            crate::direction_func::reverse_diag_dir(
                (self.shared.tunnel_bridge_direction)(end),
                |ok| crate::movement::native_assert(self.shared, ok),
            ),
        ))))
    }
    /// Delete a town (`CmdDeleteTown`, scenario editor or worldgen only).
    fn delete_town(&self, out: *mut CommandCost, flags: DoCommandFlags, t: TownRef) {
        // Stations refer to towns.
        let mut from = 0;
        loop {
            let st = (self.s.next_station)(from);
            if st.shell.is_null() {
                break;
            }
            from = st.next;
            if (self.s.station_town)(st.shell) == t.shell {
                // Non-oil rig stations are always a problem.
                if !(self.s.station_airport)(st.shell.cast())
                    || (self.s.airport_type)(st.shell) != crate::airport::AT_OILRIG
                {
                    self.error(out);
                    return;
                }
                w::hit(w::TOWN_COMMAND_DELETE_OILRIG);
                // We can only automatically delete oil rigs *if* there's no vehicle on them.
                if !self.clear(out, TileIndex((self.s.airport_tile)(st.shell)), flags) {
                    return;
                }
            }
        }
        // Waypoints, then depots, refer to towns.
        for next in [self.s.next_waypoint, self.s.next_depot] {
            from = 0;
            loop {
                let item = next(from);
                if item.next == u32::MAX {
                    break;
                }
                from = item.next;
                if item.town == t.shell {
                    self.error(out);
                    return;
                }
            }
        }
        // Check all tiles for town ownership. First check for bridge tiles, as these do not
        // directly have an owner so we need to check adjacent tiles. This won't work
        // correctly in the same loop if the adjacent tile was already deleted earlier in
        // the loop.
        for current_tile in crate::map_func::Map::iterate(self.map_size()).map(TileIndex) {
            if self.tile_type(current_tile) == TileType::MP_TUNNELBRIDGE
                && self.test_town_owns_bridge(current_tile, t)
            {
                if !self.clear(out, current_tile, flags) {
                    return;
                }
                w::hit(w::TOWN_COMMAND_DELETE_BRIDGE);
            }
        }
        // Check all remaining tiles for town ownership.
        for current_tile in crate::map_func::Map::iterate(self.map_size()).map(TileIndex) {
            let try_clear = match self.tile_type(current_tile) {
                TileType::MP_ROAD => {
                    self.town_owned_road(current_tile)
                        && TownId((self.shared.town_index)(current_tile.0)) == t.id
                }
                TileType::MP_HOUSE => TownId((self.shared.town_index)(current_tile.0)) == t.id,
                TileType::MP_INDUSTRY => (self.s.industry_town_by_tile)(current_tile.0) == t.shell,
                TileType::MP_OBJECT => {
                    if (self.shared.town_count)() == 1 {
                        // No towns will be left, remove it!
                        true
                    } else {
                        let object = unsafe { (self.object_query().by_tile)(current_tile.0) };
                        if unsafe { (self.object_query().town)(object) } == t.shell {
                            if unsafe { (self.object_query().kind)(object) }
                                == crate::object_type::OBJECT_STATUE
                            {
                                // Statue... always remove.
                                true
                            } else {
                                // Tell to find a new town.
                                if flags.test(DoCommandFlag::Execute) {
                                    (self.s.set_object_town)(object, std::ptr::null_mut());
                                    w::hit(w::TOWN_COMMAND_OBJECT_DETACH);
                                }
                                false
                            }
                        } else {
                            false
                        }
                    }
                }
                _ => false,
            };
            if try_clear && !self.clear(out, current_tile, flags) {
                return;
            }
        }
        // The town destructor will delete the other things related to the town.
        if flags.test(DoCommandFlag::Execute) {
            super::spatial::remove(self.shared, t.id);
            (self.s.remove_town_sign)(t.shell);
            (self.s.delete_town)(t.shell);
            w::hit(w::TOWN_COMMAND_DELETE);
        }
        (self.cost.zero)(out);
    }
}
/// Get cost factors for a `TownAction` (`GetTownActionCost`).
fn get_town_action_cost(action: TownAction) -> u8 {
    // Factor in the cost of each town action.
    const TOWN_ACTION_COSTS: [u8; TownAction::End.0 as usize] = [2, 4, 9, 35, 48, 53, 117, 175];
    TOWN_ACTION_COSTS[usize::from(action.0)]
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_action_cost(action: u8) -> u8 {
    get_town_action_cost(TownAction(action))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_action_mask(
    s: &CommandServices,
    cid: u8,
    t: TownRef,
) -> u8 {
    unsafe { Commands::new(s) }
        .get_mask_of_town_actions(CompanyID(cid), t)
        .0
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_cargo_goal(
    s: &CommandServices,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    effect: AcceptanceEffect,
    goal: u32,
) {
    let (flags, town_id) = (DoCommandFlags(flags), TownId(id));
    let c = unsafe { Commands::new(s) };
    if c.current_company() != CompanyID::DEITY
        || effect.0 >= crate::cargotype::TownAcceptanceEffect::NUM_TAE.0
    {
        c.error(out);
        return;
    }
    let t = c.resolve(town_id);
    // Validate if there is a cargo which is the requested TownEffect.
    if t.shell.is_null() || crate::cargo_registry::first_town_acceptance(effect.0).is_none() {
        c.error(out);
        return;
    }
    if flags.test(DoCommandFlag::Execute) {
        unsafe {
            (*t.state).simulation.goal[usize::from(effect.0)] = goal;
        }
        c.growth(t).update_town_growth(TownId(id));
        (s.invalidate_view)(id);
        w::hit(w::TOWN_COMMAND_GOAL);
    }
    (c.cost.zero)(out);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_text(state: *const State) -> ByteSpan {
    let text = unsafe { &(*state).text };
    ByteSpan {
        data: text.as_ptr(),
        size: text.len(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_set_text(state: *mut State, text: ByteSpan) {
    let target = unsafe { &mut (*state).text };
    target.clear();
    if text.size != 0 {
        target.extend_from_slice(unsafe { std::slice::from_raw_parts(text.data, text.size) });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_text_command(
    s: &CommandServices,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    text: ByteSpan,
) {
    let flags = DoCommandFlags(flags);
    let c = unsafe { Commands::new(s) };
    let Some(t) = c.deity(out, TownId(id)) else {
        return;
    };
    if flags.test(DoCommandFlag::Execute) {
        unsafe {
            openttd_rust_town_set_text(t.state, text);
        }
        (s.invalidate_view)(id);
        w::hit(w::TOWN_COMMAND_TEXT);
    }
    (c.cost.zero)(out);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_growth_command(
    s: &CommandServices,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    rate: u16,
) {
    let flags = DoCommandFlags(flags);
    let c = unsafe { Commands::new(s) };
    let Some(t) = c.deity(out, TownId(id)) else {
        return;
    };
    if flags.test(DoCommandFlag::Execute) {
        c.growth(t).town_growth_rate(TownId(id), rate);
        (s.invalidate_view)(id);
        w::hit(w::TOWN_COMMAND_RATE);
    }
    (c.cost.zero)(out);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_rating_command(
    s: &CommandServices,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    company: u8,
    rating: i16,
) {
    let flags = DoCommandFlags(flags);
    let c = unsafe { Commands::new(s) };
    let Some(t) = c.deity(out, TownId(id)) else {
        return;
    };
    if !(c.shared.company_valid)(company) {
        c.error(out);
        return;
    }
    let rating = i32::from(rating).clamp(RATING_MINIMUM, RATING_MAXIMUM) as i16;
    if flags.test(DoCommandFlag::Execute) {
        unsafe {
            (*t.state).simulation.ratings[usize::from(company)] = rating;
        }
        (s.invalidate_authority)(id);
        w::hit(w::TOWN_COMMAND_RATING);
    }
    (c.cost.zero)(out);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_expand_command(
    s: &CommandServices,
    road: *const crate::road_cmd::CommandContext,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    amount: u32,
    modes: u8,
) {
    let (flags, modes) = (
        DoCommandFlags(flags),
        crate::town_type::TownExpandModes(modes),
    );
    let c = unsafe { Commands::new(s) };
    if (!(unsafe { &*c.f.growth }.environment.editor)() && c.current_company() != CompanyID::DEITY)
        || modes.0 == 0
    {
        c.error(out);
        return;
    }
    let t = c.resolve(TownId(id));
    if t.shell.is_null() {
        c.error(out);
        return;
    }
    if flags.test(DoCommandFlag::Execute) {
        let mut e = c.growth(t);
        e.road = road;
        e.expand_town(TownId(id), amount, modes);
        w::hit(w::TOWN_COMMAND_EXPAND);
    }
    (c.cost.zero)(out);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_delete_command(
    s: &CommandServices,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
) {
    let c = unsafe { Commands::new(s) };
    let growth = unsafe { &*c.f.growth };
    if !(growth.environment.editor)() && !(growth.generating_world)() {
        c.error(out);
        return;
    }
    let t = c.resolve(TownId(id));
    if t.shell.is_null() {
        c.error(out);
        return;
    }
    c.delete_town(out, DoCommandFlags(flags), t);
}
/// Do a town action (`CmdDoTownAction`). This performs an action such as advertising,
/// building a statue, funding buildings, but also bribing the town-council.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_action(
    s: &CommandServices,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    action: u8,
) {
    let (flags, town_id, action) = (DoCommandFlags(flags), TownId(id), TownAction(action));
    let c = unsafe { Commands::new(s) };
    let t = c.resolve(town_id);
    if t.shell.is_null()
        || action.0 >= TownAction::End.0
        || !c
            .get_mask_of_town_actions(c.current_company(), t)
            .test(action)
    {
        c.error(out);
        return;
    }
    let price = crate::company::price(PR_TOWN_ACTION)
        .saturating_mul(i64::from(get_town_action_cost(action)))
        >> 8;
    if !c.town_action(t, out, flags, action) {
        return;
    }
    if flags.test(DoCommandFlag::Execute) {
        (unsafe { &*c.f.growth }.dirty_authority)(u32::from(id));
        w::hit(w::TOWN_COMMAND_ACTION);
    }
    (c.cost.expense_money)(out, crate::economy_type::EXPENSES_OTHER, price);
}
impl Commands<'_> {
    /// Original `_town_action_proc` dispatch; return false after writing an error.
    fn town_action(
        &self,
        t: TownRef,
        out: *mut CommandCost,
        flags: DoCommandFlags,
        action: TownAction,
    ) -> bool {
        let execute = flags.test(DoCommandFlag::Execute);
        match action {
            // Perform the "small/medium/large advertising campaign" town action
            // (`TownActionAdvertiseSmall`, `...Medium`, `...Large`).
            TownAction::AdvertiseSmall
            | TownAction::AdvertiseMedium
            | TownAction::AdvertiseLarge => {
                if execute {
                    crate::station::spatial::modify_rating_around(
                        self.shared,
                        unsafe { super::position(t.state) }.0,
                        (self.shared.current_company)(),
                        [0x40, 0x70, 0xA0][usize::from(action.0)],
                        [10, 15, 20][usize::from(action.0)],
                    );
                }
            }
            // Perform the "local road reconstruction" town action (`TownActionRoadRebuild`).
            TownAction::RoadRebuild => {
                // Check if the company is allowed to fund new roads.
                if !(self.s.fund_roads)() {
                    self.error(out);
                    return false;
                }
                if execute {
                    unsafe {
                        (*t.state).growth.road = 6;
                    }
                    (self.s.road_news)(t.id.0);
                    (self.s.road_events)(t.id.0);
                }
            }
            TownAction::BuildStatue => {
                if !self.town_action_build_statue(t, out, flags) {
                    return false;
                }
            }
            // Perform the "fund new buildings" town action (`TownActionFundBuildings`).
            TownAction::FundBuildings => {
                // Check if it's allowed to buy the rights.
                if !(self.s.fund_buildings)() {
                    self.error(out);
                    return false;
                }
                if execute {
                    self.growth(t).town_action_fund_buildings(t.id);
                    (unsafe { &*self.f.growth }.dirty_town)(u32::from(t.id.0));
                }
            }
            // Perform the "buy exclusive transport rights" town action (`TownActionBuyRights`).
            TownAction::BuyRights => {
                // Check if it's allowed to buy the rights.
                if !(self.s.exclusive_rights)()
                    || unsafe { (*t.state).simulation.exclusivity } != CompanyID::INVALID.0
                {
                    self.error(out);
                    return false;
                }
                if execute {
                    unsafe {
                        (*t.state).simulation.exclusive_counter = 12;
                        (*t.state).simulation.exclusivity = (self.shared.current_company)();
                    }
                    crate::station::spatial::modify_rating_around(
                        self.shared,
                        unsafe { super::position(t.state) }.0,
                        (self.shared.current_company)(),
                        130,
                        17,
                    );
                    (self.s.station_views_dirty)();
                    // Spawn news message.
                    (self.s.rights_news)(t.id.0);
                    (self.s.rights_events)(t.id.0);
                }
            }
            TownAction::Bribe => {
                if execute {
                    self.town_action_bribe(t);
                }
            }
            _ => unreachable!(),
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_destruct(s: &CommandServices, t: TownRef) {
    if (s.cleaning_pool)() {
        return;
    }
    let c = unsafe { Commands::new(s) };
    // Delete town authority window and remove from list of sorted towns.
    (s.close_view)(t.id.0);
    (s.close_graph)(t.id.0);
    if s.assertions {
        // Check no industry is related to us.
        let mut from = 0;
        loop {
            let i = (s.next_industry)(from);
            if i.next == u32::MAX {
                break;
            }
            from = i.next;
            assert!(i.town != t.shell);
        }
        // ... and no object is related to us.
        from = 0;
        loop {
            // SAFETY: Pool iteration returns a live object or null; no Rust borrow spans it.
            let o = unsafe { (c.objects().next)(from) };
            if o.object.is_null() {
                break;
            }
            from = o.id.wrapping_add(1);
            assert!(unsafe { (c.object_query().town)(o.object) } != t.shell);
        }
        // Check no tile is related to us.
        for tile in crate::map_func::Map::iterate(c.map_size()).map(TileIndex) {
            match c.tile_type(tile) {
                TileType::MP_HOUSE => assert_ne!(TownId((c.shared.town_index)(tile.0)), t.id),
                TileType::MP_ROAD => {
                    assert!(
                        !c.town_owned_road(tile) || TownId((c.shared.town_index)(tile.0)) != t.id
                    );
                }
                TileType::MP_TUNNELBRIDGE => assert!(!c.test_town_owns_bridge(tile, t)),
                _ => {}
            }
        }
    }
    // Clear the persistent storage list.
    for index in 0..(s.storage_count)(t.shell) {
        (s.delete_storage)(t.shell, index);
    }
    (s.clear_storage)(t.shell);
    // SAFETY: The canonical subsidy owner table outlives the synchronous call.
    unsafe { &*s.subsidies }.delete_with(crate::source_type::Source::town(t.id.0));
    (s.delete_inspect)(t.id.0);
    // SAFETY: Cargo's canonical process owner and native pool remain live; town ID is copied before shell removal.
    unsafe {
        crate::cargo_storage::openttd_rust_cargo_invalidate(
            s.cargo,
            t.id.0,
            crate::source_type::Source::TOWN,
        );
    }
    (unsafe { &*(*s.founding).shared }.screen_dirty)();
    w::hit(w::TOWN_COMMAND_DESTRUCT);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_post_destruct(s: &CommandServices) {
    // Invalidating the nearest town cache has to be done after removing the item from the pool.
    let c = unsafe { Commands::new(s) };
    (c.f.directory_rebuild)();
    // SAFETY: Road owner tables live for this call, and no town borrow exists after pool removal.
    unsafe {
        crate::road_cmd::openttd_rust_road_nearest_towns((c.shared.road_commands)(), false);
    }
    // Give objects a new home!
    let mut from = 0;
    loop {
        // SAFETY: Pool iteration returns a live object or null; no Rust borrow spans it.
        let o = unsafe { (c.objects().next)(from) };
        if o.object.is_null() {
            break;
        }
        from = o.id.wrapping_add(1);
        if unsafe { (c.object_query().town)(o.object) }.is_null() {
            let town = crate::town::spatial::calc_closest_town_from_tile(
                c.shared,
                TileIndex(unsafe { (c.object_query().north)(o.object) }),
                u32::MAX,
            );
            (s.set_object_town)(o.object, town.cast_mut());
            w::hit(w::TOWN_COMMAND_OBJECT_REHOME);
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTownStationCursor" ["probe"] => crate::town::commands::StationCursor {
        shell,
        next,
    };
    "OpenTTDTownPoolRelation" ["probe"] => crate::town::commands::Relation {
        next,
        town,
    };
    "OpenTTDTownCommandServices" ["probe"] => crate::town::commands::CommandServices {
        founding,
        rail,
        cargo,
        objects,
        subsidies,
        fund_roads,
        fund_buildings,
        exclusive_rights,
        bribe,
        invalidate_view,
        invalidate_authority,
        road_news,
        road_events,
        rights_news,
        rights_events,
        station_views_dirty,
        next_station,
        station_town,
        station_owner,
        station_airport,
        airport_type,
        airport_tile,
        station_activity_owner,
        local_company,
        bribe_error,
        next_waypoint,
        next_depot,
        next_industry,
        set_object_town,
        industry_town_by_tile,
        remove_town_sign,
        delete_town,
        cleaning_pool,
        close_view,
        close_graph,
        storage_count,
        delete_storage,
        clear_storage,
        delete_inspect,
        assertions,
        too_many_objects,
        no_statue_place,
    };
}
