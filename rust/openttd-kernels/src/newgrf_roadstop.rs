/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Road-stop invocation and spec-association policy. Shells and vectors stay native; element
//! observations end before resize or Action2 evaluation. Resolver invocations cannot reset
//! the map or yield the game-state mutex.
// Casts preserve the native uint8/uint16 assignments and accessor narrowing.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use crate::cargo_type::NUM_CARGO;
use crate::station_type::{
    ROAD_WAYPOINT, SRT_CARGO_TAKEN, SRT_NEW_CARGO, STATION_BUS, STATION_TRUCK,
};
use crate::tile_type::INVALID_TILE;
use crate::{
    services::{Services as Shared, TownHandle},
    station::BaseStation,
    station_cmd::RoadSpec,
};
use std::{ffi::c_void, ptr};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct State {
    pub station: *mut BaseStation,
    pub spec: *const RoadSpec,
    pub root: *const c_void,
    pub town_scope: *mut c_void,
    pub tile: u32,
    pub roadtype: u8,
    pub kind: u8,
    pub view: u8,
    pub cargo: u8,
}
#[repr(C)]
pub struct SpecKey {
    pub grfid: u32,
    pub local_id: u16,
}
#[repr(C)]
pub struct SpecVacancy {
    pub spec: *const RoadSpec,
    pub grfid: u32,
}
#[repr(C)]
pub struct SpecTriggers {
    pub animation: u16,
    pub cargo: u64,
}
#[repr(C)]
pub struct Reseed {
    pub reseed: u32,
    pub used: u8,
}
#[repr(C)]
pub struct Services {
    pub rebind: unsafe extern "C" fn(*mut BaseStation, usize, *const RoadSpec),
    pub badges: unsafe extern "C" fn(*const c_void, *const RoadSpec, u32) -> u32,
    pub base_scope: unsafe extern "C" fn(*mut c_void, u8, u8) -> *mut c_void,
    pub build_date: unsafe extern "C" fn(*const BaseStation) -> i32,
    pub by_tile: unsafe extern "C" fn(u32) -> *mut BaseStation,
    pub cached_cargo: unsafe extern "C" fn(*const BaseStation) -> u64,
    pub clear: unsafe extern "C" fn(*mut BaseStation) -> (),
    pub company: unsafe extern "C" fn(u8, *const c_void) -> u32,
    pub custom: unsafe extern "C" fn(u32) -> bool,
    pub facilities: unsafe extern "C" fn(*const BaseStation) -> u8,
    pub frame: unsafe extern "C" fn(*mut BaseStation, u32) -> u8,
    pub group: unsafe extern "C" fn(*const RoadSpec, u8) -> *const c_void,
    pub group_cargo: unsafe extern "C" fn(*const RoadSpec, usize) -> u8,
    pub group_count: unsafe extern "C" fn(*const RoadSpec) -> usize,
    pub index: unsafe extern "C" fn(u32) -> u32,
    pub is_station: unsafe extern "C" fn(*mut BaseStation) -> bool,
    pub make_town: unsafe extern "C" fn(*mut c_void, *mut TownHandle, bool) -> *mut c_void,
    pub nearby: unsafe extern "C" fn(u8, u32) -> u32,
    pub nearby_info: unsafe extern "C" fn(u32, bool) -> u32,
    pub owner: unsafe extern "C" fn(*mut BaseStation) -> u8,
    pub random_bits: unsafe extern "C" fn(*const BaseStation) -> u16,
    pub resize: unsafe extern "C" fn(*mut BaseStation, usize) -> (),
    pub resolve_callback: unsafe extern "C" fn(*const State, u16, u32, u32, *mut i32, usize) -> u16,
    pub resolve_reseed: unsafe extern "C" fn(*const State, u8) -> Reseed,
    pub set_cached: unsafe extern "C" fn(*mut BaseStation, u16, u64) -> (),
    pub set_random_bits: unsafe extern "C" fn(*mut BaseStation, u16) -> (),
    pub set_slot: unsafe extern "C" fn(*mut BaseStation, u8, *const RoadSpec, u32, u16) -> (),
    pub set_tile_random: unsafe extern "C" fn(*mut BaseStation, u32, u8) -> (),
    pub set_waiting: unsafe extern "C" fn(*mut BaseStation, u8) -> (),
    pub shared: *const Shared,
    pub slot_grfid: unsafe extern "C" fn(*const BaseStation, usize) -> u32,
    pub slot_key: unsafe extern "C" fn(*const BaseStation, usize) -> SpecKey,
    pub slot_spec: unsafe extern "C" fn(*const BaseStation, usize) -> *const RoadSpec,
    pub slot_vacancy: unsafe extern "C" fn(*const BaseStation, usize) -> SpecVacancy,
    pub spec_cargo: unsafe extern "C" fn(*const RoadSpec) -> u64,
    pub spec_count: unsafe extern "C" fn(*const BaseStation) -> usize,
    pub spec_file: unsafe extern "C" fn(*const RoadSpec) -> *const c_void,
    pub spec_grfid: unsafe extern "C" fn(*const RoadSpec) -> u32,
    pub spec_local_id: unsafe extern "C" fn(*const RoadSpec) -> u16,
    pub spec_triggers: unsafe extern "C" fn(*const RoadSpec) -> SpecTriggers,
    pub station_id: unsafe extern "C" fn(*mut BaseStation) -> u16,
    pub terrain: unsafe extern "C" fn(u32) -> u32,
    pub tile: unsafe extern "C" fn(*mut BaseStation, usize) -> u32,
    pub tile_count: unsafe extern "C" fn(*mut BaseStation) -> usize,
    pub tile_random: unsafe extern "C" fn(*const BaseStation, u32) -> u8,
    pub town: unsafe extern "C" fn(*const BaseStation) -> *mut TownHandle,
    pub town_tile: unsafe extern "C" fn(*const TownHandle) -> u32,
    pub town_zone: unsafe extern "C" fn(*const TownHandle, u32) -> u8,
    pub version: unsafe extern "C" fn(*const c_void) -> u8,
    pub waiting: unsafe extern "C" fn(*const BaseStation) -> u8,
}
impl Services {
    fn shared(&self) -> &Shared {
        unsafe { &*self.shared }
    }
    fn map(&self) -> crate::map_func::Map {
        self.shared().map()
    }
    fn random(&self) -> u32 {
        (self.shared().random)(self.shared().context)
    }
}
unsafe fn init(c: *mut State, s: &Services) {
    let st = unsafe { (*c).station };
    let spec = unsafe { (*c).spec };
    let mut cargo = crate::newgrf_commons::SG_DEFAULT_NA;
    if st.is_null() {
        cargo = crate::newgrf_commons::SG_PURCHASE;
    } else if unsafe { (s.is_station)(st) } {
        for i in 0..unsafe { (s.group_count)(spec) } {
            let candidate = unsafe { (s.group_cargo)(spec, i) };
            if candidate < NUM_CARGO && crate::newgrf_station::cargo_count(st, candidate) > 0 {
                cargo = candidate;
                break;
            }
        }
    }
    let mut root = unsafe { (s.group)(spec, cargo) };
    if root.is_null() {
        cargo = crate::newgrf_commons::SG_DEFAULT;
        root = unsafe { (s.group)(spec, cargo) };
    }
    unsafe {
        (*c).cargo = cargo;
        (*c).root = root;
    }
}
fn state(
    spec: *const RoadSpec,
    station: *mut BaseStation,
    tile: u32,
    roadtype: u8,
    kind: u8,
    view: u8,
    s: &Services,
) -> State {
    let mut c = State {
        station,
        spec,
        root: ptr::null(),
        town_scope: ptr::null_mut(),
        tile,
        roadtype,
        kind,
        view,
        cargo: 0,
    };
    unsafe {
        init(&raw mut c, s);
    }
    c
}
unsafe fn town(c: State, s: &Services) -> *mut TownHandle {
    if c.station.is_null() {
        (s.shared().closest_town)(c.tile, u32::MAX).cast_mut()
    } else {
        unsafe { (s.town)(c.station) }
    }
}
unsafe fn town_scope(c: *mut State, ro: *mut c_void, s: &Services) -> *mut c_void {
    let current = unsafe { *c };
    if current.town_scope.is_null() {
        let t = unsafe { town(current, s) };
        if t.is_null() {
            return ptr::null_mut();
        }
        let scope = unsafe { (s.make_town)(ro, t, current.station.is_null()) };
        unsafe {
            (*c).town_scope = scope;
        }
    }
    unsafe { (*c).town_scope }
}
unsafe fn variable(
    c: State,
    ro: *mut c_void,
    file: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
    s: &Services,
) -> u32 {
    let shared = s.shared();
    match variable {
        0x40 => return u32::from(c.view),
        0x41 => {
            return if c.kind == STATION_BUS {
                0
            } else if c.kind == STATION_TRUCK {
                1
            } else {
                2
            };
        }
        0x42 => {
            return if c.tile == INVALID_TILE {
                0
            } else {
                (u32::from((shared.tile_slope)(c.tile)) << 8) | unsafe { (s.terrain)(c.tile) }
            };
        }
        0x43 | 0x44 => {
            let tram = variable == 0x44;
            let rt = if c.tile == INVALID_TILE {
                if crate::road::get_road_tram_type(c.roadtype).0 == tram {
                    c.roadtype
                } else {
                    crate::road_type::INVALID_ROADTYPE
                }
            } else {
                if tram {
                    (shared.tram_type)(c.tile).0
                } else {
                    (shared.road_type)(c.tile)
                }
            };
            return if rt == crate::road_type::INVALID_ROADTYPE {
                u32::MAX
            } else {
                u32::from(unsafe {
                    crate::type_registry::openttd_rust_road_reverse(rt, (s.spec_file)(c.spec))
                })
            };
        }
        0x45 | 0x46 => {
            let edge = if variable == 0x45 {
                u32::from(crate::house::HOUSE_ZONE_TOWN_EDGE) << 16
            } else {
                0
            };
            if c.tile == INVALID_TILE {
                return edge;
            }
            let t = unsafe { town(c, s) };
            if t.is_null() {
                return edge;
            }
            let xy = unsafe { (s.town_tile)(t) };
            return if variable == 0x45 {
                (u32::from(unsafe { (s.town_zone)(t, c.tile) }) << 16)
                    | s.map().distance_manhattan(c.tile, xy).min(65535)
            } else {
                s.map().distance_square(c.tile, xy)
            };
        }
        0x47 => {
            return unsafe {
                (s.company)(
                    if c.station.is_null() {
                        (shared.current_company)()
                    } else {
                        (s.owner)(c.station)
                    },
                    ptr::null(),
                )
            };
        }
        0x49 => {
            return if c.tile == INVALID_TILE {
                0
            } else {
                u32::from(unsafe { (s.frame)(c.station, c.tile) })
            };
        }
        0x50 => return if c.tile == INVALID_TILE { 1 << 4 } else { 0 },
        0x66 => {
            if c.tile == INVALID_TILE {
                return u32::MAX;
            }
            let tile = if parameter == 0 {
                c.tile
            } else {
                unsafe { (s.nearby)(parameter as u8, c.tile) }
            };
            return if (shared.is_any_road_stop)(tile)
                && (shared.station_index)(tile) == unsafe { (s.station_id)(c.station) }
            {
                u32::from(unsafe { (s.frame)(c.station, tile) })
            } else {
                u32::MAX
            };
        }
        0x67 => {
            if c.tile == INVALID_TILE {
                return 0;
            }
            let tile = if parameter == 0 {
                c.tile
            } else {
                unsafe { (s.nearby)(parameter as u8, c.tile) }
            };
            return unsafe { (s.nearby_info)(tile, (s.version)(ro) >= 8) };
        }
        0x68 | 0x6a | 0x6b => {
            if c.tile == INVALID_TILE {
                return u32::MAX;
            }
            let tile = unsafe { (s.nearby)(parameter as u8, c.tile) };
            if !(shared.is_any_road_stop)(tile) {
                return u32::MAX;
            }
            if variable == 0x6a {
                return if unsafe { (s.custom)(tile) } {
                    unsafe { (s.slot_grfid)((s.by_tile)(tile), (s.index)(tile) as usize) }
                } else {
                    0
                };
            }
            if variable == 0x6b && !unsafe { (s.custom)(tile) } {
                return 0xfffe;
            }
            let grfid = unsafe { (s.slot_grfid)(c.station, (s.index)(c.tile) as usize) };
            if variable == 0x6b {
                let slot = unsafe { (s.slot_key)((s.by_tile)(tile), (s.index)(tile) as usize) };
                return if slot.grfid == grfid {
                    u32::from(slot.local_id)
                } else {
                    0xfffe
                };
            }
            let mut res = (u32::from((shared.station_gfx)(tile)) << 12)
                | (u32::from((shared.station_gfx)(c.tile) != (shared.station_gfx)(tile)) << 11)
                | (u32::from((shared.station_index)(tile) == unsafe { (s.station_id)(c.station) })
                    << 10);
            let kind = (shared.station_type)(tile).0;
            if kind == STATION_TRUCK {
                res |= 1 << 16;
            }
            if kind == ROAD_WAYPOINT {
                res |= 2 << 16;
            }
            if kind == c.kind {
                res |= 1 << 20;
            }
            if unsafe { (s.custom)(tile) } {
                let slot = unsafe { (s.slot_key)((s.by_tile)(tile), (s.index)(tile) as usize) };
                res |= (1 << if slot.grfid == grfid { 8 } else { 9 })
                    | u32::from(slot.local_id.min(255));
            }
            return res;
        }
        0x7a => return unsafe { (s.badges)(ro, c.spec, parameter) },
        0xf0 => {
            return if c.station.is_null() {
                0
            } else {
                u32::from(unsafe { (s.facilities)(c.station) })
            };
        }
        0xfa => {
            let date = if c.station.is_null() {
                (shared.calendar_date)()
            } else {
                unsafe { (s.build_date)(c.station) }
            };
            return date
                .wrapping_sub(crate::timer_game_common::DAYS_TILL_ORIGINAL_BASE_YEAR)
                .clamp(0, 65535) as u32;
        }
        _ => {}
    }
    if !c.station.is_null() {
        return unsafe {
            crate::newgrf_station::openttd_rust_station_base_variable(
                c.station,
                file,
                variable,
                parameter as u8,
                available,
            )
        };
    }
    unsafe {
        *available = false;
    }
    u32::MAX
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_init(c: *mut State, s: *const Services) {
    unsafe {
        init(c, &*s);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_variable(
    c: *const State,
    ro: *mut c_void,
    file: *const c_void,
    var: u8,
    param: u32,
    available: *mut bool,
    s: *const Services,
) -> u32 {
    unsafe { variable(*c, ro, file, var, param, available, &*s) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_random(
    c: *const State,
    triggers: bool,
    s: *const Services,
) -> u32 {
    let c = unsafe { *c };
    let s = unsafe { &*s };
    if c.station.is_null() {
        return 0;
    }
    if triggers {
        return u32::from(unsafe { (s.waiting)(c.station) });
    }
    let mut bits = u32::from(unsafe { (s.random_bits)(c.station) });
    if c.tile != INVALID_TILE && unsafe { (s.is_station)(c.station) } {
        bits |= u32::from(unsafe { (s.tile_random)(c.station, c.tile) }) << 16;
    }
    bits
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_town(
    c: *mut State,
    ro: *mut c_void,
    s: *const Services,
) -> *mut c_void {
    unsafe { town_scope(c, ro, &*s) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_scope(
    c: *mut State,
    ro: *mut c_void,
    own: *mut c_void,
    scope: u8,
    relative: u8,
    s: *const Services,
) -> *mut c_void {
    if scope == crate::newgrf_spritegroup::VSG_SCOPE_SELF {
        return own;
    }
    let s = unsafe { &*s };
    if scope == crate::newgrf_spritegroup::VSG_SCOPE_PARENT {
        let town = unsafe { town_scope(c, ro, s) };
        if !town.is_null() {
            return town;
        }
    }
    unsafe { (s.base_scope)(ro, scope, relative) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_callback(
    cb: u16,
    p1: u32,
    p2: u32,
    spec: *const RoadSpec,
    st: *mut BaseStation,
    tile: u32,
    roadtype: u8,
    kind: u8,
    view: u8,
    regs: *mut i32,
    size: usize,
    s: *const Services,
) -> u16 {
    let s = unsafe { &*s };
    let c = state(spec, st, tile, roadtype, kind, view, s);
    unsafe { (s.resolve_callback)(&raw const c, cb, p1, p2, regs, size) }
}
unsafe fn spec_entry(
    tile: u32,
    s: &Services,
) -> crate::newgrf_animation_base::Entry<RoadSpec, BaseStation> {
    let mut entry = crate::newgrf_animation_base::Entry {
        spec: ptr::null(),
        object: ptr::null_mut(),
    };
    if !unsafe { (s.custom)(tile) } {
        return entry;
    }
    let st = unsafe { (s.by_tile)(tile) };
    let index = unsafe { (s.index)(tile) } as usize;
    if index < unsafe { (s.spec_count)(st) } {
        entry.spec = unsafe { (s.slot_spec)(st, index) };
        if !entry.spec.is_null() {
            entry.object = st;
        }
    }
    entry
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_spec(
    tile: u32,
    s: *const Services,
) -> *const RoadSpec {
    unsafe { spec_entry(tile, &*s).spec }
}
/// Direct animation binding: the registry is installed before synchronous play;
/// native spec/pool handles remain live until this entry returns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_animation_entry(
    tile: u32,
    out: *mut crate::newgrf_animation_base::Entry<RoadSpec, BaseStation>,
) {
    // SAFETY: the synchronous native adapter supplies aligned stack output that
    // remains valid until this call returns; spec_entry initializes both handles.
    unsafe {
        out.write(spec_entry(
            tile,
            &*crate::animated_tile::roadstop_resolver_services(),
        ));
    }
}
/// Direct callback route; the animation owner retains ordering and frame policy.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_animation_callback(
    cb: u16,
    p1: u32,
    p2: u32,
    spec: *const RoadSpec,
    st: *mut BaseStation,
    tile: u32,
    _extra: i32,
) -> u16 {
    let s = crate::animated_tile::roadstop_resolver_services();
    let shared = unsafe { (&*s).shared() };
    unsafe {
        openttd_rust_roadstop_callback(
            cb,
            p1,
            p2,
            spec,
            st,
            tile,
            crate::road_type::INVALID_ROADTYPE,
            (shared.station_type)(tile).0,
            (shared.station_gfx)(tile),
            ptr::null_mut(),
            0,
            s,
        )
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_allocate(
    spec: *const RoadSpec,
    st: *mut BaseStation,
    s: *const Services,
) -> i16 {
    if spec.is_null() {
        return 0;
    }
    if st.is_null() {
        return 1;
    }
    let s = unsafe { &*s };
    let count = unsafe { (s.spec_count)(st) };
    let limit = crate::newgrf_roadstop::NUM_ROADSTOPSPECS_PER_STATION;
    for i in 1..count.min(limit) {
        if unsafe { (s.slot_spec)(st, i) } == spec {
            return i as i16;
        }
    }
    let mut i = 1;
    while i < count && i < limit {
        let slot = unsafe { (s.slot_vacancy)(st, i) };
        if slot.spec.is_null() && slot.grfid == 0 {
            break;
        }
        i += 1;
    }
    if i == limit { -1 } else { i as i16 }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_cache(st: *mut BaseStation, s: *const Services) {
    let s = unsafe { &*s };
    let mut animation = 0;
    let mut cargo = 0;
    unsafe {
        (s.set_cached)(st, 0, 0);
    }
    for i in 0..unsafe { (s.spec_count)(st) } {
        let spec = unsafe { (s.slot_spec)(st, i) };
        if spec.is_null() {
            continue;
        }
        let triggers = unsafe { (s.spec_triggers)(spec) };
        animation |= triggers.animation;
        cargo |= triggers.cargo;
        unsafe {
            (s.set_cached)(st, animation, cargo);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_assign(
    spec: *const RoadSpec,
    st: *mut BaseStation,
    index: u8,
    s: *const Services,
) {
    if index == 0 {
        return;
    }
    let services = unsafe { &*s };
    if usize::from(index) >= unsafe { (services.spec_count)(st) } {
        unsafe {
            (services.resize)(st, usize::from(index) + 1);
        }
    }
    unsafe {
        (services.set_slot)(
            st,
            index,
            spec,
            (services.spec_grfid)(spec),
            (services.spec_local_id)(spec),
        );
        openttd_rust_roadstop_cache(st, s);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_deallocate(
    st: *mut BaseStation,
    index: u8,
    s: *const Services,
) {
    if index == 0 {
        return;
    }
    let services = unsafe { &*s };
    for i in 0..unsafe { (services.tile_count)(st) } {
        let tile = unsafe { (services.tile)(st, i) };
        if unsafe { (services.index)(tile) } == u32::from(index) {
            return;
        }
    }
    unsafe {
        (services.set_slot)(st, index, ptr::null(), 0, 0);
    }
    let count = unsafe { (services.spec_count)(st) };
    if usize::from(index) == count - 1 {
        let mut num = count - 1;
        while num > 0 {
            if unsafe { (services.slot_grfid)(st, num) } != 0 {
                break;
            }
            num -= 1;
        }
        if num > 0 {
            unsafe {
                (services.resize)(st, num + 1);
            }
        } else {
            unsafe {
                (services.clear)(st);
                (services.set_cached)(st, 0, 0);
            }
            return;
        }
    }
    unsafe {
        openttd_rust_roadstop_cache(st, s);
    }
}
pub const NUM_ROADSTOPSPECS_PER_STATION: usize = 63;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_randomise(
    st: *mut BaseStation,
    tile: u32,
    trigger: u8,
    cargo: u8,
    s: *const Services,
) {
    let s = unsafe { &*s };
    let cached = unsafe { (s.cached_cargo)(st) };
    if cached == 0 || (cargo < NUM_CARGO && cached & (1u64 << cargo) == 0) {
        return;
    }
    let waiting = unsafe { (s.waiting)(st) } | (1 << trigger);
    unsafe {
        (s.set_waiting)(st, waiting);
    }
    let empty_mask = if trigger == SRT_CARGO_TAKEN {
        crate::newgrf_station::openttd_rust_station_empty_mask(st)
    } else {
        0
    };
    let mut whole_reseed = 0;
    let mut used = 0;
    let mut process = |tile| {
        let spec = unsafe { openttd_rust_roadstop_spec(tile, s) };
        if spec.is_null() {
            return;
        }
        let triggers = unsafe { (s.spec_cargo)(spec) };
        if trigger == SRT_CARGO_TAKEN && triggers & !empty_mask != 0 {
            return;
        }
        if cargo < NUM_CARGO && triggers & (1u64 << cargo) == 0 {
            return;
        }
        let c = state(
            spec,
            st,
            tile,
            crate::road_type::INVALID_ROADTYPE,
            (s.shared().station_type)(tile).0,
            (s.shared().station_gfx)(tile),
            s,
        );
        let reseed = unsafe { (s.resolve_reseed)(&raw const c, (s.waiting)(st)) };
        used |= reseed.used;
        if reseed.reseed != 0 {
            whole_reseed |= reseed.reseed;
            let mask = reseed.reseed >> 16;
            let bits = u32::from(unsafe { (s.tile_random)(st, tile) });
            let bits = (bits & !mask) | (s.random() & mask);
            unsafe {
                (s.set_tile_random)(st, tile, bits as u8);
            }
            (s.shared().mark_dirty)(tile);
        }
    };
    if trigger == SRT_NEW_CARGO || trigger == SRT_CARGO_TAKEN {
        for i in 0..unsafe { (s.tile_count)(st) } {
            process(unsafe { (s.tile)(st, i) });
        }
    } else {
        process(tile);
    }
    unsafe {
        (s.set_waiting)(st, (s.waiting)(st) & !used);
    }
    if whole_reseed & 0xffff != 0 {
        let bits = u32::from(unsafe { (s.random_bits)(st) }) & !whole_reseed;
        unsafe {
            (s.set_random_bits)(st, bits as u16);
        }
        unsafe {
            (s.set_random_bits)(st, (bits | (s.random() & whole_reseed)) as u16);
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadStopResolverState" ["station_service"] => State { station, spec, root, town_scope, tile, roadtype, kind => type, view, cargo, };
    "OpenTTDRoadStopSpecKey" ["station_service"] => SpecKey { grfid, local_id, };
    "OpenTTDRoadStopSpecVacancy" ["station_service"] => SpecVacancy { spec, grfid, };
    "OpenTTDRoadStopSpecTriggers" ["station_service"] => SpecTriggers { animation, cargo, };
    "OpenTTDRoadStopReseed" ["station_service"] => Reseed { reseed, used, };
    "OpenTTDRoadStopResolverServices" ["station_service"] => Services { rebind, badges, base_scope, build_date, by_tile, cached_cargo, clear, company, custom, facilities, frame, group, group_cargo, group_count, index, is_station, make_town, nearby, nearby_info, owner, random_bits, resize, resolve_callback, resolve_reseed, set_cached, set_random_bits, set_slot, set_tile_random, set_waiting, shared, slot_grfid, slot_key, slot_spec, slot_vacancy, spec_cargo, spec_count, spec_file, spec_grfid, spec_local_id, spec_triggers, station_id, terrain, tile, tile_count, tile_random, town, town_tile, town_zone, version, waiting, };
}

/// Class index limit from `newgrf_roadstop.h`.
pub(crate) const ROADSTOP_CLASS_MAX: u16 = u16::MAX;
/// Per-file definition limit from `newgrf_roadstop.h`.
pub(crate) const NUM_ROADSTOPS_PER_GRF: u32 = u16::MAX as u32 - 1;
