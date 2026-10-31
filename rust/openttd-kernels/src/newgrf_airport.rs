/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Airport and airport-tile invocation ownership; native frames supply stack lifetimes.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::if_not_else, // preserve source branch order
    clippy::too_many_arguments
)]
/// `TTDPatch` large airport classification.
pub const ATP_TTDP_LARGE: u8 = 1;

/// Native airport classification, returned without widening its enum ABI.
#[repr(transparent)]
pub struct TtdpAirportType(pub u8);

// Native specs and resolver frames are opaque, synchronous handles. Rust owns
// initialization, scope selection and runtime control. All world/spec reads end
// before common evaluator, lazy town materialization or storage reentry. No
// ordinary-play native exception is reachable; native leaves are noexcept and
// Rust panics abort. No ownership is transferred across this boundary.
use crate::airport::{AT_INVALID, INVALID_AIRPORTTILE, NEW_AIRPORT_OFFSET, NEW_AIRPORTTILE_OFFSET};
use crate::game_state::GameState;
use crate::newgrf_spritegroup::{VSG_SCOPE_PARENT, VSG_SCOPE_SELF};
use crate::tile_type::INVALID_TILE;
use crate::{
    animated_tile::AirportSpec as TileSpec, station::BaseStation, station_cmd::AirportSpec,
};
use std::{ffi::c_void, ptr};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct State {
    pub(crate) station: *mut BaseStation,
    spec: *const AirportSpec,
    tile_spec: *const TileSpec,
    tile: u32,
    layout: u8,
    town: *mut c_void,
}
#[repr(C)]
pub struct Services {
    shared: *const crate::services::Services,
    storage: *const crate::newgrf_storage::Services,
    pub(crate) vehicles: *const crate::vehicle::VehicleServices,
    pub(crate) by_tile: extern "C" fn(u32) -> *mut crate::station::Station,
    station_id: extern "C" fn(*const BaseStation) -> u16,
    facilities: extern "C" fn(*const BaseStation) -> u8,
    pub(crate) get_station: extern "C" fn(u16) -> *mut BaseStation,
    pub(crate) check: extern "C" fn(bool),
    pub(crate) engine_if_valid: extern "C" fn(u16) -> *const crate::vehicle::OpenTTDEngine,
    pub(crate) engine_id: extern "C" fn(*const crate::vehicle::OpenTTDEngine) -> u16,
    pub(crate) engine_kind:
        extern "C" fn(*const crate::vehicle::OpenTTDEngine) -> crate::vehicle::VehicleType,
    pub(crate) engine_subtype: extern "C" fn(*const crate::vehicle::OpenTTDEngine) -> u8,
    pub(crate) fta_flags: extern "C" fn(*const c_void) -> u8,
    pub(crate) has_hangar: extern "C" fn(*const BaseStation, *const AirportSpec) -> bool,
    pub(crate) hangar_tile: extern "C" fn(*const BaseStation, u32, *const AirportSpec) -> u32,
    pub(crate) depot_count: extern "C" fn(*const AirportSpec) -> usize,
    pub(crate) depot_offset:
        extern "C" fn(*const AirportSpec, usize) -> crate::map_type::TileIndexDiffC,
    pub(crate) rotated_offset: extern "C" fn(
        *const BaseStation,
        crate::map_type::TileIndexDiffC,
        *const AirportSpec,
    ) -> u32,
    pub(crate) fta_heading: extern "C" fn(*const c_void, u32) -> u8,
    pub(crate) fta_position: extern "C" fn(*const c_void, u32) -> u8,
    pub(crate) dummy_spec: *const AirportSpec,
    pub(crate) dummy_fta: *const c_void,
    pub(crate) fta: extern "C" fn(*const AirportSpec) -> *const c_void,
    /// `AirportSpec::catchment`.
    pub(crate) catchment: extern "C" fn(*const AirportSpec) -> u8,
    base_spec: extern "C" fn(u16) -> *const AirportSpec,
    enabled: extern "C" fn(*const AirportSpec) -> bool,
    min_year: extern "C" fn(*const AirportSpec) -> i32,
    max_year: extern "C" fn(*const AirportSpec) -> i32,
    override_id: extern "C" fn(*const AirportSpec) -> u16,
    never_expire: extern "C" fn() -> bool,
    station_layout: extern "C" fn(*const BaseStation) -> u8,
    random: extern "C" fn(*mut BaseStation) -> u16,
    build_date: extern "C" fn(*mut BaseStation) -> i32,
    root: extern "C" fn(*const AirportSpec, bool) -> *const c_void,
    tile_root: extern "C" fn(*const TileSpec, bool) -> *const c_void,
    badges: extern "C" fn(*const c_void, *const AirportSpec, u32) -> u32,
    tile_badges: extern "C" fn(*const c_void, *const TileSpec, u32) -> u32,
    tile_spec: extern "C" fn(u8) -> *const TileSpec,
    tile_override: extern "C" fn(*const TileSpec) -> u16,
    tile_local: extern "C" fn(*const TileSpec) -> u16,
    tile_grfid: extern "C" fn(*const TileSpec) -> u32,
    tile_has_groups: extern "C" fn(*const TileSpec) -> bool,
    tile_substitute: extern "C" fn(*const TileSpec) -> u16,
    belongs: extern "C" fn(*const BaseStation, u32) -> bool,
    tile_random: extern "C" fn(u32) -> u8,
    nearby: extern "C" fn(u8, u32) -> u32,
    nearby_info: extern "C" fn(u32, bool) -> u32,
    terrain: extern "C" fn(u32) -> u32,
    frame: extern "C" fn(u32) -> u8,
    version: extern "C" fn(*const c_void) -> u8,
    grfid: extern "C" fn(*const c_void) -> u32,
    base_scope: extern "C" fn(*mut c_void, u8, u8) -> *mut c_void,
    town_scope: extern "C" fn(*mut c_void, *const crate::services::TownHandle, bool) -> *mut c_void,
    evaluate_tile: extern "C" fn(
        u16,
        u32,
        u32,
        *const TileSpec,
        *mut BaseStation,
        u32,
        *mut i32,
        usize,
    ) -> u16,
    unhandled: extern "C" fn(u8),
}
static REGISTRY: GameState<*const Services> = GameState::new(ptr::null());
/// Install the immutable process-lifetime services before game/runtime queries.
/// SAFETY: Serialized initialization; tables outlive every synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_exchange(
    services: *const Services,
) -> *const Services {
    REGISTRY.exchange(services)
}
pub(crate) fn runtime() -> &'static Services {
    unsafe { REGISTRY.table() }
}
impl Services {
    pub(crate) fn shared(&self) -> &crate::services::Services {
        unsafe { &*self.shared }
    }
    pub(crate) fn commands(&self) -> &crate::station_cmd::CommandServicesTable {
        unsafe { &*(self.shared().station_commands)() }
    }
    fn map(&self) -> crate::map_func::Map {
        self.shared().map()
    }
}

/// Canonical `Airport::GetSpec` facade, shared with native-facing wrappers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_get_spec(
    tile: u32,
    kind: u8,
    s: *const Services,
) -> *const AirportSpec {
    crate::station_base::airport_get_spec(tile, kind, unsafe { &*s })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_fta(kind: u8, s: *const Services) -> *const c_void {
    crate::airport::get_airport(kind, unsafe { &*s })
}
/// Observe only the station airport association at its original query point.
pub(crate) fn station_by_id(id: u16, s: &Services) -> *mut BaseStation {
    (s.get_station)(id)
}
pub(crate) fn station_spec(st: *mut BaseStation, s: &Services) -> *const AirportSpec {
    let tile = (s.commands().airport_tile)(st);
    if tile == INVALID_TILE {
        s.dummy_spec
    } else {
        crate::station_base::airport_get_spec(tile, (s.commands().airport_type)(st), s)
    }
}
pub(crate) fn station_fta(st: *mut BaseStation, s: &Services) -> *const c_void {
    (s.fta)(station_spec(st, s))
}

/// Source `AirportSpec::Get`: disabled custom substitution precedes overrides.
pub(crate) fn get(kind: u8, s: &Services) -> *const AirportSpec {
    (s.check)(u16::from(kind) < crate::airport::NUM_AIRPORTS);
    let mut spec = (s.base_spec)(u16::from(kind));
    if kind >= NEW_AIRPORT_OFFSET && !(s.enabled)(spec) {
        if unsafe {
            crate::override_manager::openttd_rust_override_grfid(
                crate::override_manager::airport(),
                u16::from(kind),
            )
        } == 0
        {
            return spec;
        }
        let substitute = unsafe {
            crate::override_manager::openttd_rust_override_substitute(
                crate::override_manager::airport(),
                u16::from(kind),
            )
        } as u8;
        if u16::from(substitute) == AT_INVALID {
            return spec;
        }
        spec = (s.base_spec)(u16::from(substitute));
    }
    let over = (s.override_id)(spec);
    if over != AT_INVALID {
        (s.base_spec)(over)
    } else {
        spec
    }
}
pub(crate) fn available(spec: *const AirportSpec, s: &Services) -> bool {
    if !(s.enabled)(spec) {
        return false;
    }
    if crate::timer_game_common::calendar_year().0 < (s.min_year)(spec) {
        return false;
    }
    if (s.never_expire)() {
        return true;
    }
    crate::timer_game_common::calendar_year().0 <= (s.max_year)(spec)
}
pub(crate) fn in_bounds(spec: *const AirportSpec, layout: u8, tile: u32, s: &Services) -> bool {
    let c = s.commands();
    if usize::from(layout) >= (c.airport_layout_count)(spec) {
        return false;
    }
    let size = (c.airport_size)(spec);
    let (mut w, mut h) = (size.w, size.h);
    let rotation = (c.airport_layout)(spec, layout).rotation;
    if rotation == crate::direction_type::DIR_E || rotation == crate::direction_type::DIR_W {
        std::mem::swap(&mut w, &mut h);
    }
    s.map().tile_x(tile) + u32::from(w) < (s.shared().map_max_x)() + 1
        && s.map().tile_y(tile) + u32::from(h) < (s.shared().map_max_y)() + 1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_spec(
    kind: u8,
    s: *const Services,
) -> *const AirportSpec {
    get(kind, unsafe { &*s })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_available(
    spec: *const AirportSpec,
    s: *const Services,
) -> bool {
    available(spec, unsafe { &*s })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_in_bounds(
    spec: *const AirportSpec,
    layout: u8,
    tile: u32,
    s: *const Services,
) -> bool {
    in_bounds(spec, layout, tile, unsafe { &*s })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_init(
    state: *mut State,
    station: *mut BaseStation,
    spec: *const AirportSpec,
    tile_spec: *const TileSpec,
    tile: u32,
    layout: u8,
    _s: *const Services,
) {
    unsafe {
        state.write(State {
            station,
            spec,
            tile_spec,
            tile,
            layout,
            town: ptr::null_mut(),
        });
    }
}
/// Finish the tile invocation's parent selection in the Rust owner, before
/// returning to the native stack frame. No spec borrow crosses it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_tile_init(
    tile: *const State,
    parent: *mut State,
    services: *const Services,
) -> *const c_void {
    let tile = unsafe { *tile };
    let s = unsafe { &*services };
    let (spec, layout) = if tile.station.is_null() {
        (ptr::null(), 0)
    } else {
        let spec = get((s.commands().airport_type)(tile.station), s);
        (spec, (s.station_layout)(tile.station))
    };
    unsafe {
        (&raw mut (*parent).spec).write(spec);
        (&raw mut (*parent).layout).write(layout);
    }
    (s.tile_root)(tile.tile_spec, !tile.station.is_null())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_root(
    state: *const State,
    s: *const Services,
) -> *const c_void {
    let c = unsafe { *state };
    let s = unsafe { &*s };
    if c.tile_spec.is_null() {
        (s.root)(c.spec, !c.station.is_null())
    } else {
        (s.tile_root)(c.tile_spec, !c.station.is_null())
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_town(
    state: *mut State,
    resolver: *mut c_void,
    s: *const Services,
) -> *mut c_void {
    let c = unsafe { *state };
    let s = unsafe { &*s };
    if !c.town.is_null() {
        return c.town;
    }
    let town = if !c.station.is_null() {
        (s.commands().station_town)(c.station).cast_mut()
    } else if c.tile != INVALID_TILE {
        crate::town::spatial::closest(s.shared(), c.tile, u32::MAX).cast_mut()
    } else {
        ptr::null_mut()
    };
    if town.is_null() {
        return ptr::null_mut();
    }
    let scope = (s.town_scope)(resolver, town, c.station.is_null());
    unsafe {
        (&raw mut (*state).town).write(scope);
    }
    scope
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_scope(
    state: *mut State,
    resolver: *mut c_void,
    own: *mut c_void,
    parent: *mut c_void,
    scope: u8,
    relative: u8,
    s: *const Services,
) -> *mut c_void {
    if scope == VSG_SCOPE_SELF {
        return own;
    }
    if scope == VSG_SCOPE_PARENT {
        if !parent.is_null() {
            return parent;
        }
        let town = unsafe { openttd_rust_airport_town(state, resolver, s) };
        if !town.is_null() {
            return town;
        }
    }
    (unsafe { &*s }.base_scope)(resolver, scope, relative)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_variable(
    state: *const State,
    resolver: *const c_void,
    file: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
    s: *const Services,
) -> u32 {
    let c = unsafe { *state };
    let s = unsafe { &*s };
    match variable {
        0x40 => return u32::from(c.layout),
        0x7a => return (s.badges)(resolver, c.spec, parameter),
        _ => {}
    }
    if c.station.is_null() {
        unsafe {
            *available = false;
        }
        return u32::MAX;
    }
    match variable {
        0x7c => {
            let owner = (unsafe { &*s.storage }.airport)(c.station.cast());
            if owner.is_null() {
                0
            } else {
                (unsafe { crate::newgrf_storage::openttd_rust_storage256_get(owner, parameter) })
                    as u32
            }
        }
        0xf0 => u32::from((s.facilities)(c.station)),
        0xfa => (s.build_date)(c.station)
            .wrapping_sub(crate::timer_game_common::DAYS_TILL_ORIGINAL_BASE_YEAR)
            .clamp(0, 65535) as u32,
        _ => unsafe {
            crate::newgrf_station::openttd_rust_station_base_variable(
                c.station,
                file,
                variable,
                parameter as u8,
                available,
            )
        },
    }
}
fn id_at_offset(tile: u32, st: *mut BaseStation, grfid: u32, s: &Services) -> u32 {
    if !(s.belongs)(st, tile) {
        return 0xffff;
    }
    let gfx = crate::station_map::get_airport_gfx(tile, s.shared(), s);
    let spec = (s.tile_spec)(gfx);
    if gfx < NEW_AIRPORTTILE_OFFSET {
        let over = (s.tile_override)(spec);
        if over == INVALID_AIRPORTTILE {
            return 0xff00 | u32::from(gfx);
        }
        let other = (s.tile_spec)(over as u8);
        return if (s.tile_grfid)(other) == grfid {
            u32::from((s.tile_local)(other))
        } else {
            0xfffe
        };
    }
    if (s.tile_has_groups)(spec) {
        if (s.tile_grfid)(spec) == grfid {
            u32::from((s.tile_local)(spec))
        } else {
            0xfffe
        }
    } else {
        0xff00 | u32::from((s.tile_substitute)(spec))
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_tile_variable(
    state: *const State,
    resolver: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
    s: *const Services,
) -> u32 {
    let c = unsafe { *state };
    let s = unsafe { &*s };
    (s.check)(!c.station.is_null());
    match variable {
        0x41 => (s.terrain)(c.tile),
        0x42 => {
            let town = crate::town::spatial::closest(s.shared(), c.tile, u32::MAX);
            let owner = (s.shared().town_owner)(town);
            u32::from(unsafe { crate::town::get_radius_group(owner, c.tile, s.map()) })
        }
        0x43 => crate::industry::resolver::relative_position(
            c.tile,
            (s.commands().airport_tile)(c.station),
            s.map(),
        ),
        0x44 => u32::from((s.frame)(c.tile)),
        0x60 => {
            let tile = if parameter as u8 == 0 {
                c.tile
            } else {
                (s.nearby)(parameter as u8, c.tile)
            };
            let same = (s.shared().tile_type)(tile) == crate::tile_type::MP_STATION
                && crate::station_map::is_airport(tile, s.shared())
                && (s.shared().station_index)(tile) == (s.station_id)(c.station);
            (s.nearby_info)(tile, (s.version)(resolver) >= 8) | (u32::from(same) << 8)
        }
        0x61 => {
            let tile = (s.nearby)(parameter as u8, c.tile);
            if (s.belongs)(c.station, tile) {
                u32::from((s.frame)(tile))
            } else {
                u32::MAX
            }
        }
        0x62 => id_at_offset(
            (s.nearby)(parameter as u8, c.tile),
            c.station,
            (s.grfid)(resolver),
            s,
        ),
        0x7a => (s.tile_badges)(resolver, c.tile_spec, parameter),
        _ => {
            (s.unhandled)(variable);
            unsafe {
                *available = false;
            }
            u32::MAX
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_random(
    state: *const State,
    tile: bool,
    s: *const Services,
) -> u32 {
    let c = unsafe { *state };
    let s = unsafe { &*s };
    let station = if c.station.is_null() {
        0
    } else {
        u32::from((s.random)(c.station))
    };
    station
        | if tile && c.tile != INVALID_TILE {
            u32::from((s.tile_random)(c.tile)) << 16
        } else {
            0
        }
}
pub(crate) fn translate_tile(gfx: u8, s: &Services) -> u8 {
    let over = (s.tile_override)((s.tile_spec)(gfx));
    if over == INVALID_AIRPORTTILE {
        gfx
    } else {
        over as u8
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_translate_tile(gfx: u8, s: *const Services) -> u8 {
    translate_tile(gfx, unsafe { &*s })
}
/// Direct entry from the Rust animation owner; native stack frame construction only.
pub(crate) fn tile_spec_by_gfx(gfx: u8, s: &Services) -> *const TileSpec {
    (s.tile_spec)(translate_tile(gfx, s))
}
pub(crate) fn tile_spec_by_tile(tile: u32, s: &Services) -> *const TileSpec {
    (s.tile_spec)(crate::station_map::get_airport_gfx(tile, s.shared(), s))
}
pub(crate) fn tile_callback(
    callback: u16,
    p1: u32,
    p2: u32,
    spec: *const TileSpec,
    station: *mut crate::station::Station,
    tile: u32,
    s: &Services,
) -> u16 {
    (s.evaluate_tile)(
        callback,
        p1,
        p2,
        spec,
        station.cast(),
        tile,
        ptr::null_mut(),
        0,
    )
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAirportResolverState" ["probe"] => State { station, spec, tile_spec, tile, layout, town, };
    "OpenTTDAirportResolverServices" ["probe"] => Services { shared, storage, vehicles, by_tile, station_id, facilities, get_station, check, engine_if_valid, engine_id, engine_kind, engine_subtype, fta_flags, has_hangar, hangar_tile, depot_count, depot_offset, rotated_offset, fta_heading, fta_position, dummy_spec, dummy_fta, fta, catchment, base_spec, enabled, min_year, max_year, override_id, never_expire, station_layout, random, build_date, root, tile_root, badges, tile_badges, tile_spec, tile_override, tile_local, tile_grfid, tile_has_groups, tile_substitute, belongs, tile_random, nearby, nearby_info, terrain, frame, version, grfid, base_scope, town_scope, evaluate_tile, unhandled, };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_can_engine_use_station(
    engine: u16,
    station: *const BaseStation,
    s: *const Services,
) -> bool {
    crate::vehicle_func::can_engine_use_station(engine, station, unsafe { &*s })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_can_vehicle_use_station(
    vehicle: *const crate::vehicle::OpenTTDVehicle,
    station: *const BaseStation,
    s: *const Services,
) -> bool {
    unsafe { crate::vehicle_func::can_vehicle_use_station(vehicle, station, &*s) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_is_hangar(tile: u32, s: *const Services) -> bool {
    crate::station_map::is_hangar(tile, unsafe { &*s })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_pos_on_build(tile: u32, s: *const Services) -> u8 {
    crate::airport::get_vehicle_pos_on_build(tile, unsafe { &*s })
}
