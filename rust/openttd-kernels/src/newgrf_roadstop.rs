/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Road-stop invocation and spec-association policy over Rust-owned lists; element
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
use crate::station_record::record_of;
use crate::station_storage::road;
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
    pub base_scope: unsafe extern "C" fn(*mut c_void, u8, u8) -> *mut c_void,
    pub by_tile: unsafe extern "C" fn(u32) -> *mut BaseStation,
    pub company: unsafe extern "C" fn(u8, *const c_void) -> u32,
    pub custom: unsafe extern "C" fn(u32) -> bool,
    pub group: unsafe extern "C" fn(*const RoadSpec, u8) -> *const c_void,
    pub group_cargo: unsafe extern "C" fn(*const RoadSpec, usize) -> u8,
    pub group_count: unsafe extern "C" fn(*const RoadSpec) -> usize,
    pub index: unsafe extern "C" fn(u32) -> u32,
    pub make_town: unsafe extern "C" fn(*mut c_void, *mut TownHandle, bool) -> *mut c_void,
    pub resolve_callback: unsafe extern "C" fn(*const State, u16, u32, u32, *mut i32, usize) -> u16,
    pub resolve_reseed: unsafe extern "C" fn(*const State, u8) -> Reseed,
    pub shared: *const Shared,
    pub spec_cargo: unsafe extern "C" fn(*const RoadSpec) -> u64,
    pub spec_triggers: unsafe extern "C" fn(*const RoadSpec) -> SpecTriggers,
    pub station_id: unsafe extern "C" fn(*mut BaseStation) -> u16,
    pub town_tile: unsafe extern "C" fn(*const TownHandle) -> u32,
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
    } else if crate::station_record::is_station(st.cast()) {
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
        crate::town::spatial::closest(s.shared(), c.tile, u32::MAX).cast_mut()
    } else {
        crate::station_record::read(c.station.cast(), |r| r.town.cast())
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
                (u32::from(crate::map_func::World::new(shared).tile_slope(c.tile)) << 8)
                    | (shared.terrain_type)(c.tile)
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
                u32::from(crate::type_registry::openttd_rust_road_reverse(
                    rt,
                    (s.shared().specs().road_stop().file)(c.spec),
                ))
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
                (u32::from(unsafe { crate::town::radius_from_shell(s.shared(), t, c.tile) }) << 16)
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
                        crate::station_record::owner_company(c.station)
                    },
                    ptr::null(),
                )
            };
        }
        0x49 => {
            return if c.tile == INVALID_TILE {
                0
            } else {
                u32::from(crate::station_storage::roadstop_value(
                    c.station, c.tile, true,
                ))
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
                (shared.nearby_tile)(parameter as u8, c.tile)
            };
            return if crate::map_func::World::new(shared).any_road_stop_tile(tile)
                && (shared.station_index)(tile) == unsafe { (s.station_id)(c.station) }
            {
                u32::from(crate::station_storage::roadstop_value(
                    c.station, tile, true,
                ))
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
                (shared.nearby_tile)(parameter as u8, c.tile)
            };
            return (shared.nearby_tile_information)(
                tile,
                (s.shared().resolver_grf_version)(ro) >= 8,
            );
        }
        0x68 | 0x6a | 0x6b => {
            if c.tile == INVALID_TILE {
                return u32::MAX;
            }
            let tile = (shared.nearby_tile)(parameter as u8, c.tile);
            if !crate::map_func::World::new(shared).any_road_stop_tile(tile) {
                return u32::MAX;
            }
            if variable == 0x6a {
                return if unsafe { (s.custom)(tile) } {
                    unsafe { road::get((s.by_tile)(tile), (s.index)(tile) as usize).grfid }
                } else {
                    0
                };
            }
            if variable == 0x6b && !unsafe { (s.custom)(tile) } {
                return 0xfffe;
            }
            let grfid = unsafe { road::get(c.station, (s.index)(c.tile) as usize).grfid };
            if variable == 0x6b {
                let slot = unsafe { road::get((s.by_tile)(tile), (s.index)(tile) as usize) };
                return if slot.grfid == grfid {
                    u32::from(slot.localidx)
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
                let slot = unsafe { road::get((s.by_tile)(tile), (s.index)(tile) as usize) };
                res |= (1 << if slot.grfid == grfid { 8 } else { 9 })
                    | u32::from(slot.localidx.min(255));
            }
            return res;
        }
        0x7a => {
            return (s.shared().specs().road_stop().badges)(c.spec).resolve(
                s.shared(),
                ro,
                parameter,
            );
        }
        0xf0 => {
            return if c.station.is_null() {
                0
            } else {
                u32::from(crate::station_record::facilities(c.station.cast()))
            };
        }
        0xfa => {
            let date = if c.station.is_null() {
                crate::timer_game_common::calendar_date().0
            } else {
                crate::station_record::read(c.station.cast(), |r| r.build_date)
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
pub unsafe extern "C" fn openttd_rust_roadstop_random(c: *const State, triggers: bool) -> u32 {
    let c = unsafe { *c };
    if c.station.is_null() {
        return 0;
    }
    if triggers {
        return u32::from(crate::station_record::read(c.station.cast(), |r| {
            r.waiting_random_triggers
        }));
    }
    let mut bits = u32::from(crate::station_record::read(c.station.cast(), |r| {
        r.random_bits
    }));
    if c.tile != INVALID_TILE && crate::station_record::is_station(c.station.cast()) {
        bits |= u32::from(crate::station_storage::roadstop_value(
            c.station, c.tile, false,
        )) << 16;
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
    if index < road::count(st) {
        entry.spec = road::get(st, index).spec;
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
) -> i16 {
    if spec.is_null() {
        return 0;
    }
    if st.is_null() {
        return 1;
    }
    let count = road::count(st);
    let limit = crate::newgrf_roadstop::NUM_ROADSTOPSPECS_PER_STATION;
    for i in 1..count.min(limit) {
        if road::get(st, i).spec == spec {
            return i as i16;
        }
    }
    let mut i = 1;
    while i < count && i < limit {
        let slot = road::get(st, i);
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
    crate::station_record::set_cached_roadstop_triggers(st, 0, 0);
    for i in 0..road::count(st) {
        let spec = road::get(st, i).spec;
        if spec.is_null() {
            continue;
        }
        let triggers = unsafe { (s.spec_triggers)(spec) };
        animation |= triggers.animation;
        cargo |= triggers.cargo;
        crate::station_record::set_cached_roadstop_triggers(st, animation, cargo);
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
    if usize::from(index) >= road::count(st) {
        unsafe {
            road::resize(record_of(st), usize::from(index) + 1);
        }
    }
    unsafe {
        road::set(
            st,
            index,
            spec,
            (services.shared().specs().road_stop().grfid)(spec),
            (services.shared().specs().road_stop().local_id)(spec),
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
    for i in 0..crate::station_storage::roadstop_count(st) {
        let tile = crate::station_storage::roadstop_tile(st, i);
        if unsafe { (services.index)(tile) } == u32::from(index) {
            return;
        }
    }
    road::set(st, index, ptr::null(), 0, 0);
    let count = road::count(st);
    if usize::from(index) == count - 1 {
        let mut num = count - 1;
        while num > 0 {
            if road::get(st, num).grfid != 0 {
                break;
            }
            num -= 1;
        }
        if num > 0 {
            unsafe {
                road::resize(record_of(st), num + 1);
            }
        } else {
            unsafe {
                road::resize(record_of(st), 0);
                crate::station_record::set_cached_roadstop_triggers(st, 0, 0);
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
    let cached = crate::station_record::read(st.cast(), |r| r.cached_roadstop_cargo_triggers);
    if cached == 0 || (cargo < NUM_CARGO && cached & (1u64 << cargo) == 0) {
        return;
    }
    let waiting =
        crate::station_record::read(st.cast(), |r| r.waiting_random_triggers) | (1 << trigger);
    crate::station_record::set_waiting_triggers(st, waiting);
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
        let reseed = unsafe {
            (s.resolve_reseed)(
                &raw const c,
                crate::station_record::read(st.cast(), |r| r.waiting_random_triggers),
            )
        };
        used |= reseed.used;
        if reseed.reseed != 0 {
            whole_reseed |= reseed.reseed;
            let mask = reseed.reseed >> 16;
            let bits = u32::from(crate::station_storage::roadstop_value(st, tile, false));
            let bits = (bits & !mask) | (s.random() & mask);
            crate::station_storage::roadstop_set(st, tile, bits as u8, false);
            (s.shared().mark_dirty)(tile);
        }
    };
    if trigger == SRT_NEW_CARGO || trigger == SRT_CARGO_TAKEN {
        for i in 0..crate::station_storage::roadstop_count(st) {
            process(crate::station_storage::roadstop_tile(st, i));
        }
    } else {
        process(tile);
    }
    crate::station_record::set_waiting_triggers(
        st,
        crate::station_record::read(st.cast(), |r| r.waiting_random_triggers) & !used,
    );
    if whole_reseed & 0xffff != 0 {
        let bits =
            u32::from(crate::station_record::read(st.cast(), |r| r.random_bits)) & !whole_reseed;
        crate::station_record::set_random_bits(st, bits as u16);
        crate::station_record::set_random_bits(st, (bits | (s.random() & whole_reseed)) as u16);
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRoadStopResolverState" ["station_service"] => State { station, spec, root, town_scope, tile, roadtype, kind => type, view, cargo, };
    "OpenTTDRoadStopSpecTriggers" ["station_service"] => SpecTriggers { animation, cargo, };
    "OpenTTDRoadStopReseed" ["station_service"] => Reseed { reseed, used, };
    "OpenTTDRoadStopResolverServices" ["station_service"] => Services { base_scope, by_tile, company, custom, group, group_cargo, group_count, index, make_town, resolve_callback, resolve_reseed, shared, spec_cargo, spec_triggers, station_id, town_tile, };
}

/// Class index limit from `newgrf_roadstop.h`.
pub(crate) const ROADSTOP_CLASS_MAX: u16 = u16::MAX;
/// Per-file definition limit from `newgrf_roadstop.h`.
pub(crate) const NUM_ROADSTOPS_PER_GRF: u32 = u16::MAX as u32 - 1;
