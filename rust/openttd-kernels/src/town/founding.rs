/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete town founding, placement and generation; owners are never borrowed
//! across native callbacks. Cancellation returns to its native exception frame.
use super::{
    Engine, Leaves, State, TownId, TownRef, names, town_ticks_to_game_ticks, update_radius,
};
use crate::services::{ByteSpan, CommandCost, CommandCostServices, Services, TownHandle};
use crate::witness as w;
use std::cell::OnceCell;

macro_rules! observed {
    ($call:expr) => {{
        let result = $call;
        crate::witness::hit(crate::witness::TOWN_SERVICES);
        crate::witness::add(
            crate::witness::TOWN_BYTES,
            std::mem::size_of_val(&result) as u64,
        );
        result
    }};
}
pub(super) use observed;

macro_rules! native_byte {
    ($($name:ident),+) => { $(
        #[repr(transparent)]
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub struct $name(pub u8);
    )+ };
}
native_byte!(
    TownSize,
    TownLayout,
    CompanyId,
    TownFounding,
    ClearGround,
    TreeGround,
    AcceptanceEffect
);
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Tile(pub u32);
#[repr(C)]
pub struct Frame {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Metadata {
    pub grfid: u32,
    pub kind: u16,
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
    pub close_to_town: extern "C" fn(Tile, u32) -> bool,
    pub flat: extern "C" fn(Tile) -> bool,
    pub clear_ground: extern "C" fn(Tile) -> ClearGround,
    pub tree_ground: extern "C" fn(Tile) -> TreeGround,
    pub water_distance: extern "C" fn(Tile, bool) -> u32,
    pub scale_size: extern "C" fn(u32) -> u32,
    pub scale_land: extern "C" fn(u32) -> u32,
    pub found_setting: extern "C" fn() -> TownFounding,
    pub layout_setting: extern "C" fn() -> TownLayout,
    pub initial_city_size: extern "C" fn() -> u8,
    pub town_density: extern "C" fn() -> u8,
    pub custom_number: extern "C" fn() -> u16,
    pub larger_towns: extern "C" fn() -> u8,
    pub parameters: extern "C" fn(*mut Frame) -> *const names::NameParams,
    pub metadata: extern "C" fn(*const names::NameParams) -> Metadata,
    pub initialize_counts: extern "C" fn(*mut TownHandle),
    pub insert_town: extern "C" fn(*mut TownHandle),
    pub cargo_effect_exists: extern "C" fn(AcceptanceEffect) -> bool,
    pub prepare_rename: extern "C" fn(*mut TownHandle),
    pub name_changed: extern "C" fn(*mut TownHandle, ByteSpan),
    pub update_virt: extern "C" fn(*mut TownHandle),
    pub directory_rebuild: extern "C" fn(),
    pub directory_resort: extern "C" fn(),
    pub update_noise: extern "C" fn(),
    pub rebuild_tree: extern "C" fn(),
    pub delete_town: extern "C" fn(TownId),
    pub assert_random_context: extern "C" fn(),
    pub assert_allocation: extern "C" fn(),
    pub initialize_cost: extern "C" fn(*mut CommandCost, i32),
    pub sufficient_money: extern "C" fn(*const CommandCost) -> bool,
    pub money_deficit: extern "C" fn(*mut Frame, *const CommandCost),
    pub set_result: extern "C" fn(*mut Frame, TownId),
    pub exchange_generating: extern "C" fn(bool) -> bool,
    pub nearest_town_roads: extern "C" fn(bool),
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
        *self.map.get_or_init(|| crate::services::Map {
            log_x: observed!((self.shared.map_log_x)()) as u8,
        })
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
            leaves: self.leaves,
            services: self.shared,
            map,
            owner: town.state,
            house_town: Some(town),
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
    fn aligned(&self, tile: Tile, layout: TownLayout) -> bool {
        let x = tile.0 % self.map().size_x();
        let y = tile.0 / self.map().size_x();
        match layout.0 {
            2 => x.is_multiple_of(3) && y.is_multiple_of(3),
            3 => x.is_multiple_of(4) && y.is_multiple_of(4),
            _ => true,
        }
    }
    fn align(&self, tile: Tile, layout: TownLayout) -> Tile {
        let x = tile.0 % self.map().size_x();
        let y = tile.0 / self.map().size_x();
        match layout.0 {
            2 => Tile((x - x % 3).wrapping_add((y - y % 3).wrapping_mul(self.map().size_x()))),
            3 => Tile((x & !3).wrapping_add((y & !3).wrapping_mul(self.map().size_x()))),
            _ => tile,
        }
    }
    fn placement_error(&self, tile: Tile, surrounding: bool) -> Option<u32> {
        w::hit(w::TOWN_PLACEMENT);
        if observed!((self.shared.distance_from_edge)(tile.0)) < 12 {
            return Some(self.s.edge_error);
        }
        if observed!((self.s.close_to_town)(tile, 20)) {
            return Some(self.s.close_error);
        }
        let tile_type = observed!((self.shared.tile_type)(tile.0));
        if (tile_type != 0 && tile_type != 4) || !observed!((self.s.flat)(tile)) {
            return Some(self.s.unsuitable_error);
        }
        if surrounding {
            let mut count = 0;
            let height = observed!((self.shared.tile_z)(tile.0));
            let mut spiral = self.spiral(tile, 5);
            while !spiral.is_end() {
                if count == 12 {
                    break;
                }
                let at = Tile(
                    spiral
                        .x
                        .wrapping_add(spiral.y.wrapping_mul(self.map().size_x())),
                );
                let valid = match observed!((self.shared.tile_type)(at.0)) {
                    0 => observed!((self.s.clear_ground)(at)).0 != 1,
                    4 => observed!((self.s.tree_ground)(at)).0 != 1,
                    _ => false,
                };
                if valid
                    && observed!((self.shared.tile_max_z)(at.0)) <= height.wrapping_add(1)
                    && observed!((self.shared.tile_z)(at.0)) >= height.wrapping_sub(1)
                {
                    count += 1;
                }
                spiral = spiral.advance(self.map().size_x(), self.size_y());
            }
            if count < 12 {
                return Some(self.s.unsuitable_error);
            }
        }
        None
    }
    fn coastal(&self, tile: Tile, layout: TownLayout) -> Tile {
        w::hit(w::TOWN_COAST);
        let mut coast = self.spiral(tile, 40);
        while !coast.is_end() {
            let at = Tile(
                coast
                    .x
                    .wrapping_add(coast.y.wrapping_mul(self.map().size_x())),
            );
            if observed!((self.shared.tile_type)(at.0)) == 0 {
                let mut furthest = Tile(u32::MAX);
                let mut max_distance = 0;
                let mut test = self.spiral(at, 10);
                while !test.is_end() {
                    let tile = Tile(
                        test.x
                            .wrapping_add(test.y.wrapping_mul(self.map().size_x())),
                    );
                    if observed!((self.shared.tile_type)(tile.0)) == 0
                        && observed!((self.s.flat)(tile))
                        && self.aligned(tile, layout)
                        && self.placement_error(tile, true).is_none()
                    {
                        let distance = observed!((self.s.water_distance)(tile, true));
                        if distance > max_distance {
                            furthest = tile;
                            max_distance = distance;
                        }
                    }
                    test = test.advance(self.map().size_x(), self.size_y());
                }
                return furthest;
            }
            coast = coast.advance(self.map().size_x(), self.size_y());
        }
        Tile(u32::MAX)
    }
    unsafe fn create(
        &self,
        frame: *mut Frame,
        town: TownRef,
        tile: Tile,
        parts: u32,
        size: TownSize,
        city: bool,
        layout: TownLayout,
        manual: bool,
    ) {
        w::hit(w::TOWN_GENERATED);
        if manual {
            w::hit(w::TOWN_MANUAL);
        }
        if layout.0 == 2 || layout.0 == 3 {
            w::hit(w::TOWN_GRID);
        }
        let previous = super::openttd_rust_town_generating_exchange(true);
        // SAFETY: Immediate canonical field writes; no borrow spans callbacks.
        unsafe {
            (*town.state).simulation.xy = tile.0;
            (*town.state).simulation.num_houses = 0;
            (*town.state).simulation.time_until_rebuild = 10;
            update_radius(town.state);
            (*town.state).growth.flags = 0;
            (*town.state).simulation.population = 0;
        }
        observed!((self.s.initialize_counts)(town.shell));
        unsafe {
            (*town.state).growth.counter = town.id.0 % 70;
            (*town.state).growth.rate = town_ticks_to_game_ticks(250);
        }
        observed!((self.s.insert_town)(town.shell));
        match observed!((self.leaves.climate)()) {
            1 => {
                if observed!((self.s.cargo_effect_exists)(AcceptanceEffect(5))) {
                    unsafe {
                        (*town.state).simulation.goal[5] = 0xffff_fffe;
                    }
                }
            }
            2 => {
                if observed!((self.s.cargo_effect_exists)(AcceptanceEffect(5))) {
                    unsafe {
                        (*town.state).simulation.goal[5] = u32::MAX;
                    }
                }
                if observed!((self.s.cargo_effect_exists)(AcceptanceEffect(4))) {
                    unsafe {
                        (*town.state).simulation.goal[4] = u32::MAX;
                    }
                }
            }
            _ => {}
        }
        unsafe {
            (*town.state).growth.funding = 0;
            (*town.state).simulation.ratings = [500; 15];
            (*town.state).simulation.have_ratings = 0;
            (*town.state).simulation.exclusivity = u8::MAX;
            (*town.state).simulation.exclusive_counter = 0;
            (*town.state).simulation.statues = 0;
        }
        let parameters = observed!((self.s.parameters)(frame));
        let metadata = observed!((self.s.metadata)(parameters));
        unsafe {
            (*town.state).simulation.townnamegrfid = metadata.grfid;
            (*town.state).simulation.townnametype = metadata.kind;
            (*town.state).simulation.townnameparts = parts;
            initialize_layout(town.state, layout, self.map().size_x());
            (*town.state).simulation.larger_town = city;
        }
        let mut x = i32::from(size.0).wrapping_mul(16).wrapping_add(3);
        if size.0 == 3 {
            x = (self.random() & 15).wrapping_add(8) as i32;
        }
        if city && (!manual || observed!((self.leaves.editor)())) {
            x = x.wrapping_mul(i32::from(observed!((self.s.initial_city_size)())));
        }
        let engine = self.engine(town);
        engine.change_house_count(x as u32);
        unsafe {
            update_radius(town.state);
        }
        let mut remaining = x.wrapping_mul(4);
        loop {
            w::hit(w::TOWN_GROWTH);
            engine.grow(u32::from(town.id.0), 3);
            remaining = remaining.wrapping_sub(1);
            if remaining == 0 {
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
        observed!((self.leaves.update_max_pass)(u32::from(town.id.0)));
        observed!((self.s.update_noise)());
        super::openttd_rust_town_generating_exchange(previous);
    }
    unsafe fn random_town(
        &self,
        frame: *mut Frame,
        mut attempts: u32,
        parts: u32,
        size: TownSize,
        city: bool,
        layout: TownLayout,
    ) -> Option<TownRef> {
        observed!((self.s.assert_random_context)());
        if !observed!((self.s.can_allocate)()) {
            return None;
        }
        loop {
            let mut tile = self.align(
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
            let valid = if observed!((self.shared.tile_type)(tile.0)) == 6 {
                tile = self.coastal(tile, layout);
                tile.0 != u32::MAX
            } else {
                self.placement_error(tile, true).is_none()
            };
            if valid {
                let town = observed!((self.s.allocate)(tile));
                unsafe {
                    self.create(frame, town, tile, parts, size, city, layout, false);
                }
                if unsafe { super::population(town.state) } > 0 {
                    return Some(town);
                }
                w::hit(w::TOWN_ZERO_GROWTH);
                let company = observed!((self.leaves.exchange_company)(15));
                observed!((self.s.delete_town)(town.id));
                observed!((self.leaves.exchange_company)(company));
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
    fn default_number(&self) -> u32 {
        const INITIAL: [u32; 4] = [5, 11, 23, 46];
        let density = observed!((self.s.town_density)());
        if density == 4 {
            return u32::from(observed!((self.s.custom_number)()));
        }
        // Native setting's original array domain; no extra rejection is added.
        let value = unsafe { *INITIAL.get_unchecked(usize::from(density)) };
        observed!((self.s.scale_size)(value))
    }
    unsafe fn generate_name(
        &self,
        frame: *mut Frame,
        name_frame: *mut names::NameFrame,
        scratch: *mut names::Names,
        result: *mut u32,
    ) -> bool {
        let parameters = observed!((self.s.parameters)(frame));
        unsafe {
            names::generate(
                self.names,
                name_frame,
                self.s.random,
                result,
                parameters,
                scratch,
            )
        }
    }
}
unsafe fn initialize_layout(state: *mut State, layout: TownLayout, size_x: u32) {
    let result = if layout.0 == 4 {
        let xy = unsafe { super::position(state) };
        let x = xy % size_x;
        let y = xy / size_x;
        let hash = ((x >> 4) ^ (x >> 6) ^ (y >> 4)).wrapping_sub(y >> 6);
        (hash % 4) as u8
    } else {
        layout.0
    };
    unsafe {
        (*state).simulation.layout = result;
    }
}
/// # Safety
/// The immutable native tables and their pointed-to services remain live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_default_number(s: *const FoundingServices) -> u32 {
    unsafe { Founding::new(&*s) }.default_number()
}
/// # Safety
/// Tables, native stack frames, command cost and input bytes remain live for this call. Owners are supplied by the pool and no borrow crosses a callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_found(
    s: *const FoundingServices,
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
    let f = unsafe { Founding::new(&*s) };
    w::hit(w::TOWN_ENTRIES);
    let parameters = observed!((f.s.parameters)(frame));
    observed!((f.cost.error)(result, 0xffff));
    if size.0 >= 4 || layout.0 >= 5 {
        w::hit(w::TOWN_INVALID);
        return;
    }
    let editor = observed!((f.leaves.editor)());
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
        if !unsafe { names::verify(f.names, name_frame, parts, parameters, std::ptr::null_mut()) } {
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
        if let Some(error) = f.placement_error(tile, false) {
            observed!((f.cost.error)(result, error));
            return;
        }
    }
    observed!((f.s.initialize_cost)(
        result,
        MULT[usize::from(city)][usize::from(size.0)],
    ));
    if execute {
        if !observed!((f.s.sufficient_money)(result)) {
            w::hit(w::TOWN_MONEY);
            observed!((f.s.money_deficit)(frame, result));
            observed!((f.cost.expense_money)(result, 12, 0));
            return;
        }
        let previous = observed!((f.s.exchange_generating)(true));
        observed!((f.s.nearest_town_roads)(true));
        let town = if random {
            unsafe { f.random_town(frame, 20, parts, size, city, layout) }
        } else {
            let town = observed!((f.s.allocate)(tile));
            unsafe {
                f.create(frame, town, tile, parts, size, city, layout, true);
            }
            Some(town)
        };
        observed!((f.s.nearest_town_roads)(false));
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
        if !observed!((f.leaves.editor)()) {
            observed!((f.s.founded_news)(town.id, tile));
            observed!((f.s.founded_events)(town.id));
        }
    }
}
/// # Safety
/// Tables and native stack frames remain live; result is writable. Only named progress services report cancellation; scratch drops before native rethrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_town_generate(
    s: *const FoundingServices,
    frame: *mut Frame,
    name_frame: *mut names::NameFrame,
    layout: TownLayout,
    has_number: bool,
    number: u32,
    result: *mut bool,
) -> bool {
    let f = unsafe { Founding::new(&*s) };
    let mut current = 0_u32;
    let mut total = if has_number {
        number
    } else if observed!((f.s.town_density)()) == 4 {
        f.default_number()
    } else {
        observed!((f.s.scale_land)(
            f.default_number().wrapping_add(f.random() & 7),
        ))
    }
    .clamp(1, 64000);
    let mut parts = 0;
    let mut scratch = names::Names::new();
    w::hit(w::TOWN_ENTRIES);
    if !observed!((f.s.set_progress)(frame, total)) {
        w::hit(w::TOWN_CANCELLED);
        return false;
    }
    let mut town = observed!((f.names.next)(0));
    while !town.shell.is_null() {
        let custom = unsafe { names::openttd_rust_town_name(town.state) };
        let name = if custom.size == 0 {
            observed!((f.s.cached_name)(town.shell))
        } else {
            custom
        };
        scratch.insert(unsafe { names::bytes(name) }.to_vec());
        town = observed!((f.names.next)(u32::from(town.id.0).wrapping_add(1)));
    }
    let larger_towns = observed!((f.s.larger_towns)());
    let offset = if larger_towns == 0 {
        w::hit(w::TOWN_NO_CITIES);
        0
    } else {
        f.random() % u32::from(larger_towns)
    };
    loop {
        let larger_towns = observed!((f.s.larger_towns)());
        let city = larger_towns != 0 && offset.wrapping_add(current) % u32::from(larger_towns) == 0;
        if !observed!((f.s.increase_progress)(frame)) {
            w::hit(w::TOWN_CANCELLED);
            return false;
        }
        if unsafe { f.generate_name(frame, name_frame, &raw mut scratch, &raw mut parts) }
            && unsafe { f.random_town(frame, 20, parts, TownSize(3), city, layout) }.is_some()
        {
            current = current.wrapping_add(1);
        }
        total = total.wrapping_sub(1);
        if total == 0 {
            break;
        }
    }
    scratch.clear();
    observed!((f.s.rebuild_tree)());
    if current != 0 {
        unsafe {
            result.write(true);
        }
        return true;
    }
    w::hit(w::TOWN_FALLBACK);
    let success =
        unsafe { f.generate_name(frame, name_frame, std::ptr::null_mut(), &raw mut parts) }
            && unsafe {
                f.random_town(
                    frame,
                    10000,
                    parts,
                    TownSize(3),
                    observed!((f.s.larger_towns)()) != 0,
                    layout,
                )
            }
            .is_some();
    if !success && observed!((f.shared.town_count)()) == 0 && !observed!((f.leaves.editor)()) {
        observed!((f.s.no_town_error)());
    }
    unsafe {
        result.write(success);
    }
    true
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
