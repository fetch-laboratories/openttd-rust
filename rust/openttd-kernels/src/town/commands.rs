/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Town commands, authority actions and ordered deletion. Post-destruction code never
//! resolves an ID.
use super::founding::{AcceptanceEffect, FoundingServices};
use super::{Engine, State, TownId, TownRef, engine};
use crate::command_type::DoCommandFlag;
use crate::company_type::{INVALID_OWNER, OWNER_DEITY, OWNER_NONE, OWNER_TOWN};
use crate::economy_type::PR_TOWN_ACTION;
use crate::objects::{Object, ObjectServices, QueryServices};
use crate::services::{ByteSpan, CommandCost, CommandCostServices, Map, Services, TownHandle};
use crate::slope_func::is_steep_slope;
use crate::station::BaseStation;
use crate::station::Station;
use crate::strings_type::INVALID_STRING_ID;
use crate::tile_type::{
    INVALID_TILE, MP_CLEAR, MP_HOUSE, MP_INDUSTRY, MP_OBJECT, MP_ROAD, MP_TREES, MP_TUNNELBRIDGE,
};
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
    fn resolve(&self, id: u16) -> TownRef {
        (self.f.resolve)(TownId(id))
    }
    fn map(&self) -> Map {
        *self.map.get_or_init(|| self.shared.map())
    }
    fn town_owned_road(&self, tile: u32) -> bool {
        let world = self
            .world
            .get_or_init(|| crate::map_func::World::new(self.shared));
        crate::road_map::has_town_owned_road(world, tile)
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
    fn deity(&self, out: *mut CommandCost, id: u16) -> Option<TownRef> {
        if (self.shared.current_company)() != OWNER_DEITY {
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
    fn clear(&self, out: *mut CommandCost, tile: u32, flags: u16) -> bool {
        // SAFETY: Native command table and caller-owned result remain live.
        crate::landscape_cmd::clear_command(unsafe { &*self.f.command }, out, tile, flags);
        (self.cost.success)(out)
    }
    fn clear_unowned(&self, out: *mut CommandCost, tile: u32, flags: u16) -> bool {
        // SAFETY: Immutable table; original Backup scope restores after the command.
        let command = unsafe { &*self.f.command };
        let old = (command.finance.current_company)();
        (command.finance.set_current_company)(OWNER_NONE);
        let okay = self.clear(out, tile, flags);
        (command.finance.set_current_company)(old);
        okay
    }
    fn mask(&self, cid: u8, t: TownRef) -> u8 {
        let mut buttons = 0;
        if cid != crate::company_type::COMPANY_SPECTATOR
            && !((self.s.bribe)()
                && unsafe { (*t.state).simulation.unwanted[usize::from(cid)] } != 0)
        {
            // SAFETY: Canonical resolver returns a live owner or null as GetAvailableMoney permits.
            let c = unsafe { &*self.f.command };
            let available = unsafe {
                crate::company::openttd_rust_company_available_money(
                    (c.finance.company_finances)(cid),
                    (c.infinite_money)(),
                )
            };
            for action in 0..8 {
                if action == 7 {
                    if !(self.s.bribe)() {
                        continue;
                    }
                    if i32::from(unsafe { (*t.state).simulation.ratings[usize::from(cid)] })
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
                if action == 6
                    && (!(self.s.exclusive_rights)()
                        || unsafe { (*t.state).simulation.exclusive_counter } != 0)
                {
                    continue;
                }
                if action == 5 && !(self.s.fund_buildings)() {
                    continue;
                }
                if action == 3 && !(self.s.fund_roads)() {
                    continue;
                }
                if action == 4 && unsafe { (*t.state).simulation.statues } & (1 << cid) != 0 {
                    continue;
                }
                let cost = i64::from(action_cost(action))
                    .saturating_mul(crate::company::price(PR_TOWN_ACTION))
                    >> 8;
                if available >= cost {
                    buttons |= 1 << action;
                }
            }
        }
        buttons
    }
    fn statue(&self, t: TownRef, out: *mut CommandCost, flags: u16) -> bool {
        if !unsafe { (self.objects().can_allocate)() } {
            (self.cost.error)(out, self.s.too_many_objects);
            return false;
        }
        let center = unsafe { super::position(t.state) };
        let mut spiral = crate::spiral::square(
            self.map().tile_x(center),
            self.map().tile_y(center),
            9,
            self.map().size_x(),
            self.size_y(),
        );
        let mut best = INVALID_TILE;
        let mut count = 0;
        while !spiral.is_end() {
            let tile = spiral
                .x
                .wrapping_add(spiral.y.wrapping_mul(self.map().size_x()));
            spiral = spiral.advance(self.map().size_x(), self.size_y());
            count += 1;
            if is_steep_slope((self.shared.tile_slope)(tile)) {
                continue;
            }
            if (self.shared.bridge_above)(tile) {
                continue;
            }
            if matches!((self.shared.tile_type)(tile), MP_CLEAR | MP_TREES)
                && self.clear_unowned(out, tile, 0)
            {
                best = tile;
                break;
            }
            let house = (self.shared.tile_type)(tile) == MP_HOUSE;
            if count <= 25 {
                if house && best == INVALID_TILE && self.clear_unowned(out, tile, 0) {
                    best = tile;
                }
                if count == 25 && best != INVALID_TILE {
                    break;
                }
            }
            // Preserve the original's fall-through into the outer-circle choice,
            // including a second clear test for an inner house.
            if !house || !self.clear_unowned(out, tile, 0) {
                continue;
            }
            best = tile;
            break;
        }
        if best == INVALID_TILE {
            (self.cost.error)(out, self.s.no_statue_place);
            return false;
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            self.clear_unowned(out, best, DoCommandFlag::EXECUTE);
            let company = (self.shared.current_company)();
            // SAFETY: Live canonical tables and shell; no town-state borrow spans object callbacks.
            unsafe {
                crate::objects::build(
                    self.objects(),
                    crate::object_type::OBJECT_STATUE,
                    best,
                    company,
                    t.shell,
                    0,
                );
            }
            let company = (self.shared.current_company)();
            unsafe {
                (*t.state).simulation.statues |= 1 << company;
            }
            (self.shared.mark_dirty)(best);
            w::hit(w::TOWN_COMMAND_STATUE);
        }
        true
    }
    fn bribe(&self, t: TownRef) {
        let company = (self.shared.current_company)();
        if crate::random_func::chance16_i(1, 14, self.shared.random()) {
            unsafe {
                (*t.state).simulation.unwanted[usize::from(company)] = 6;
            }
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
            if (self.s.local_company)() {
                (self.s.bribe_error)();
            }
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
                && unsafe { (*t.state).simulation.exclusivity } != INVALID_OWNER
            {
                unsafe {
                    (*t.state).simulation.exclusivity = INVALID_OWNER;
                    (*t.state).simulation.exclusive_counter = 0;
                }
                w::hit(w::TOWN_COMMAND_RIGHTS_CANCEL);
            }
            w::hit(w::TOWN_COMMAND_BRIBE_SUCCESS);
        }
    }
    fn bridge_owned(&self, tile: u32, t: TownRef) -> bool {
        if !crate::tile_map::is_tile_owner(self.shared, tile, OWNER_TOWN) {
            return false;
        }
        let adjacent = tile.wrapping_add(self.map().offs_by_diag_dir(
            crate::direction_func::reverse_diag_dir(
                (self.shared.tunnel_bridge_direction)(tile),
                |ok| crate::movement::native_assert(self.shared, ok),
            ),
        ));
        let owns = |at| {
            (self.shared.tile_type)(at) == MP_ROAD
                && crate::tile_map::is_tile_owner(self.shared, at, OWNER_TOWN)
                && (self.shared.town_index)(at) == t.id.0
        };
        if owns(adjacent) {
            return true;
        }
        let end = (unsafe { &*self.s.rail }.other_tunnel_bridge_end)(tile);
        // Historical source adds the other end's direction to the ORIGINAL tile.
        owns(
            tile.wrapping_add(self.map().offs_by_diag_dir(
                crate::direction_func::reverse_diag_dir(
                    (self.shared.tunnel_bridge_direction)(end),
                    |ok| crate::movement::native_assert(self.shared, ok),
                ),
            )),
        )
    }
    fn delete(&self, out: *mut CommandCost, flags: u16, t: TownRef) {
        let mut from = 0;
        loop {
            let st = (self.s.next_station)(from);
            if st.shell.is_null() {
                break;
            }
            from = st.next;
            if (self.s.station_town)(st.shell) == t.shell {
                if !(self.s.station_airport)(st.shell.cast())
                    || (self.s.airport_type)(st.shell) != crate::airport::AT_OILRIG
                {
                    self.error(out);
                    return;
                }
                w::hit(w::TOWN_COMMAND_DELETE_OILRIG);
                if !self.clear(out, (self.s.airport_tile)(st.shell), flags) {
                    return;
                }
            }
        }
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
        for tile in 0..self.map_size() {
            if (self.shared.tile_type)(tile) == MP_TUNNELBRIDGE && self.bridge_owned(tile, t) {
                if !self.clear(out, tile, flags) {
                    return;
                }
                w::hit(w::TOWN_COMMAND_DELETE_BRIDGE);
            }
        }
        for tile in 0..self.map_size() {
            let clear = match (self.shared.tile_type)(tile) {
                MP_ROAD => self.town_owned_road(tile) && (self.shared.town_index)(tile) == t.id.0,
                MP_HOUSE => (self.shared.town_index)(tile) == t.id.0,
                MP_INDUSTRY => (self.s.industry_town_by_tile)(tile) == t.shell,
                MP_OBJECT => {
                    if (self.shared.town_count)() == 1 {
                        true
                    } else {
                        let object = unsafe { (self.object_query().by_tile)(tile) };
                        if unsafe { (self.object_query().town)(object) } == t.shell {
                            if unsafe { (self.object_query().kind)(object) }
                                == crate::object_type::OBJECT_STATUE
                            {
                                true
                            } else {
                                if flags & DoCommandFlag::EXECUTE != 0 {
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
            if clear && !self.clear(out, tile, flags) {
                return;
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            super::spatial::remove(self.shared, t.id);
            (self.s.remove_town_sign)(t.shell);
            (self.s.delete_town)(t.shell);
            w::hit(w::TOWN_COMMAND_DELETE);
        }
        (self.cost.zero)(out);
    }
}
const COSTS: [u8; 8] = [2, 4, 9, 35, 48, 53, 117, 175];
fn action_cost(action: u8) -> u8 {
    COSTS[usize::from(action)]
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_town_action_cost(action: u8) -> u8 {
    action_cost(action)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_action_mask(
    s: &CommandServices,
    cid: u8,
    t: TownRef,
) -> u8 {
    unsafe { Commands::new(s) }.mask(cid, t)
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
    let c = unsafe { Commands::new(s) };
    if (c.shared.current_company)() != OWNER_DEITY || effect.0 >= 6 {
        c.error(out);
        return;
    }
    let t = c.resolve(id);
    if t.shell.is_null() || crate::cargo_registry::first_town_acceptance(effect.0).is_none() {
        c.error(out);
        return;
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        unsafe {
            (*t.state).simulation.goal[usize::from(effect.0)] = goal;
        }
        c.growth(t).update_growth(u32::from(id));
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
    let c = unsafe { Commands::new(s) };
    let Some(t) = c.deity(out, id) else {
        return;
    };
    if flags & DoCommandFlag::EXECUTE != 0 {
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
    let c = unsafe { Commands::new(s) };
    let Some(t) = c.deity(out, id) else {
        return;
    };
    if flags & DoCommandFlag::EXECUTE != 0 {
        c.growth(t).custom_rate(u32::from(id), u32::from(rate));
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
    let c = unsafe { Commands::new(s) };
    let Some(t) = c.deity(out, id) else {
        return;
    };
    if !(c.shared.company_valid)(company) {
        c.error(out);
        return;
    }
    let rating = i32::from(rating).clamp(RATING_MINIMUM, RATING_MAXIMUM) as i16;
    if flags & DoCommandFlag::EXECUTE != 0 {
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
    let c = unsafe { Commands::new(s) };
    if (!(unsafe { &*c.f.growth }.environment.editor)()
        && (c.shared.current_company)() != OWNER_DEITY)
        || modes == 0
    {
        c.error(out);
        return;
    }
    let t = c.resolve(id);
    if t.shell.is_null() {
        c.error(out);
        return;
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        let mut e = c.growth(t);
        e.road = road;
        e.expand(u32::from(id), amount, u32::from(modes));
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
    let t = c.resolve(id);
    if t.shell.is_null() {
        c.error(out);
        return;
    }
    c.delete(out, flags, t);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_action(
    s: &CommandServices,
    out: *mut CommandCost,
    flags: u16,
    id: u16,
    action: u8,
) {
    let c = unsafe { Commands::new(s) };
    let t = c.resolve(id);
    if t.shell.is_null()
        || action >= 8
        || c.mask((c.shared.current_company)(), t) & (1 << action) == 0
    {
        c.error(out);
        return;
    }
    let price =
        crate::company::price(PR_TOWN_ACTION).saturating_mul(i64::from(action_cost(action))) >> 8;
    let execute = flags & DoCommandFlag::EXECUTE != 0;
    match action {
        0..=2 => {
            if execute {
                crate::station::spatial::modify_rating_around(
                    c.shared,
                    unsafe { super::position(t.state) },
                    (c.shared.current_company)(),
                    [0x40, 0x70, 0xA0][usize::from(action)],
                    [10, 15, 20][usize::from(action)],
                );
            }
        }
        3 => {
            if !(s.fund_roads)() {
                c.error(out);
                return;
            }
            if execute {
                unsafe {
                    (*t.state).growth.road = 6;
                }
                (s.road_news)(id);
                (s.road_events)(id);
            }
        }
        4 => {
            if !c.statue(t, out, flags) {
                return;
            }
        }
        5 => {
            if !(s.fund_buildings)() {
                c.error(out);
                return;
            }
            if execute {
                c.growth(t).fund(u32::from(id));
                (unsafe { &*c.f.growth }.dirty_town)(u32::from(id));
            }
        }
        6 => {
            if !(s.exclusive_rights)()
                || unsafe { (*t.state).simulation.exclusivity } != INVALID_OWNER
            {
                c.error(out);
                return;
            }
            if execute {
                unsafe {
                    (*t.state).simulation.exclusive_counter = 12;
                    (*t.state).simulation.exclusivity = (c.shared.current_company)();
                }
                crate::station::spatial::modify_rating_around(
                    c.shared,
                    unsafe { super::position(t.state) },
                    (c.shared.current_company)(),
                    130,
                    17,
                );
                (s.station_views_dirty)();
                (s.rights_news)(id);
                (s.rights_events)(id);
            }
        }
        7 => {
            if execute {
                c.bribe(t);
            }
        }
        _ => unreachable!(),
    }
    if execute {
        (unsafe { &*c.f.growth }.dirty_authority)(u32::from(id));
        w::hit(w::TOWN_COMMAND_ACTION);
    }
    (c.cost.expense_money)(out, crate::economy_type::EXPENSES_OTHER, price);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_destruct(s: &CommandServices, t: TownRef) {
    if (s.cleaning_pool)() {
        return;
    }
    let c = unsafe { Commands::new(s) };
    (s.close_view)(t.id.0);
    (s.close_graph)(t.id.0);
    if s.assertions {
        let mut from = 0;
        loop {
            let i = (s.next_industry)(from);
            if i.next == u32::MAX {
                break;
            }
            from = i.next;
            assert!(i.town != t.shell);
        }
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
        for tile in 0..c.map_size() {
            match (c.shared.tile_type)(tile) {
                MP_HOUSE => assert_ne!((c.shared.town_index)(tile), t.id.0),
                MP_ROAD => {
                    assert!(!c.town_owned_road(tile) || (c.shared.town_index)(tile) != t.id.0);
                }
                MP_TUNNELBRIDGE => assert!(!c.bridge_owned(tile, t)),
                _ => {}
            }
        }
    }
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
    let c = unsafe { Commands::new(s) };
    (c.f.directory_rebuild)();
    // SAFETY: Road owner tables live for this call, and no town borrow exists after pool removal.
    unsafe {
        crate::road_cmd::openttd_rust_road_nearest_towns((c.shared.road_commands)(), false);
    }
    let mut from = 0;
    loop {
        // SAFETY: Pool iteration returns a live object or null; no Rust borrow spans it.
        let o = unsafe { (c.objects().next)(from) };
        if o.object.is_null() {
            break;
        }
        from = o.id.wrapping_add(1);
        if unsafe { (c.object_query().town)(o.object) }.is_null() {
            let town = crate::town::spatial::calc_closest(
                c.shared,
                unsafe { (c.object_query().north)(o.object) },
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
