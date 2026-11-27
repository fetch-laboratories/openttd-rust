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
use crate::debug::debug;
use crate::game_state::GameState;
use crate::newgrf_class::{ClassSpec, Registry};
use crate::newgrf_specs::{SpecServices, Specs};
use crate::newgrf_spritegroup::{VSG_SCOPE_PARENT, VSG_SCOPE_SELF};
use crate::newgrf_storage::{PersistentStorageArray, PersistentStorageRef};
use crate::table::strings::{
    STR_AIRPORT_CLASS_HELIPORTS, STR_AIRPORT_CLASS_HUB, STR_AIRPORT_CLASS_LARGE,
    STR_AIRPORT_CLASS_SMALL,
};
use crate::tile_type::{INVALID_TILE, TileIndex};
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
    /// `st->airport.psa`.
    airport_psa: extern "C" fn(*const BaseStation) -> *mut PersistentStorageArray<256>,
    /// `st->airport.psa = psa`.
    set_airport_psa:
        extern "C" fn(*mut BaseStation, *mut crate::newgrf_storage::BasePersistentStorageArray),
    pub(crate) vehicles: *const crate::vehicle::VehicleServices,
    pub(crate) by_tile: extern "C" fn(u32) -> *mut crate::station::Station,
    station_id: extern "C" fn(*const BaseStation) -> u16,
    facilities: extern "C" fn(*const BaseStation) -> u8,
    pub(crate) get_station: extern "C" fn(u16) -> *mut BaseStation,
    pub(crate) check: extern "C" fn(bool),
    pub(crate) engine_if_valid: extern "C" fn(u16) -> *const crate::vehicle::OpenTTDEngine,
    pub(crate) engine_id: extern "C" fn(*const crate::vehicle::OpenTTDEngine) -> u16,
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
    min_year: extern "C" fn(*const AirportSpec) -> i32,
    max_year: extern "C" fn(*const AirportSpec) -> i32,
    station_layout: extern "C" fn(*const BaseStation) -> u8,
    random: extern "C" fn(*mut BaseStation) -> u16,
    build_date: extern "C" fn(*mut BaseStation) -> i32,
    tile_spec: extern "C" fn(u8) -> *const TileSpec,
    belongs: extern "C" fn(*const BaseStation, u32) -> bool,
    tile_random: extern "C" fn(u32) -> u8,
    frame: extern "C" fn(u32) -> u8,
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
}
static REGISTRY: GameState<*const Services> = GameState::new(ptr::null());
#[cfg(test)]
impl Services {
    /// A table whose only reachable entry is `st->index`.
    pub(crate) const fn with_station_id(
        station_id: extern "C" fn(*const BaseStation) -> u16,
    ) -> Self {
        use crate::test_tables::{unreachable1, unreachable2, unreachable3};
        extern "C" fn evaluate_tile(
            _: u16,
            _: u32,
            _: u32,
            _: *const TileSpec,
            _: *mut BaseStation,
            _: u32,
            _: *mut i32,
            _: usize,
        ) -> u16 {
            unreachable!("unexpected airport tile evaluation")
        }
        Self {
            shared: ptr::null(),
            storage: ptr::null(),
            airport_psa: unreachable1,
            set_airport_psa: unreachable2,
            vehicles: ptr::null(),
            by_tile: unreachable1,
            station_id,
            facilities: unreachable1,
            get_station: unreachable1,
            check: unreachable1,
            engine_if_valid: unreachable1,
            engine_id: unreachable1,
            fta_flags: unreachable1,
            has_hangar: unreachable2,
            hangar_tile: unreachable3,
            depot_count: unreachable1,
            depot_offset: unreachable2,
            rotated_offset: unreachable3,
            fta_heading: unreachable2,
            fta_position: unreachable2,
            dummy_spec: ptr::null(),
            dummy_fta: ptr::null(),
            fta: unreachable1,
            catchment: unreachable1,
            base_spec: unreachable1,
            min_year: unreachable1,
            max_year: unreachable1,
            station_layout: unreachable1,
            random: unreachable1,
            build_date: unreachable1,
            tile_spec: unreachable1,
            belongs: unreachable2,
            tile_random: unreachable1,
            frame: unreachable1,
            base_scope: unreachable3,
            town_scope: unreachable3,
            evaluate_tile,
        }
    }
}
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
    /// `st->index`.
    pub(crate) fn station_index(&self, st: *const BaseStation) -> crate::station_type::StationID {
        crate::station_type::StationID((self.station_id)(st))
    }
    pub(crate) fn shared(&self) -> &crate::services::Services {
        unsafe { &*self.shared }
    }
    /// `st->airport.psa`.
    ///
    /// # Safety
    /// `st` is a live station, whose storage stays live while the handle is used.
    unsafe fn airport_psa(&self, st: *const BaseStation) -> Option<PersistentStorageRef> {
        unsafe { PersistentStorageRef::of((self.airport_psa)(st)) }
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
    crate::station_base::airport_get_spec_raw(tile, kind, unsafe { &*s })
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
        crate::station_base::airport_get_spec_raw(tile, (s.commands().airport_type)(st), s)
    }
}
pub(crate) fn station_fta(st: *mut BaseStation, s: &Services) -> *const c_void {
    (s.fta)(station_spec(st, s))
}

/// Source `AirportSpec::Get`: disabled custom substitution precedes overrides.
pub(crate) fn get(kind: u8, s: &Services) -> *const AirportSpec {
    (s.check)(u16::from(kind) < crate::airport::NUM_AIRPORTS);
    let mut spec = (s.base_spec)(u16::from(kind));
    if kind >= NEW_AIRPORT_OFFSET && !(s.shared().specs().airport().substitute().enabled)(spec) {
        let airport_mngr = crate::override_manager::airport_mngr();
        // SAFETY: `_airport_mngr` is the live registered manager.
        if unsafe { (*airport_mngr).get_grfid(u16::from(kind)) } == 0 {
            return spec;
        }
        // SAFETY: As above.
        let substitute = unsafe { (*airport_mngr).get_substitute_id(u16::from(kind)) } as u8;
        if u16::from(substitute) == AT_INVALID {
            return spec;
        }
        spec = (s.base_spec)(u16::from(substitute));
    }
    let over = (s.shared().specs().airport().substitute().override_id)(spec);
    if over != AT_INVALID {
        (s.base_spec)(over)
    } else {
        spec
    }
}
pub(crate) fn available(spec: *const AirportSpec, s: &Services) -> bool {
    if !(s.shared().specs().airport().substitute().enabled)(spec) {
        return false;
    }
    if crate::timer_game_common::calendar_year().0 < (s.min_year)(spec) {
        return false;
    }
    if unsafe { s.shared().settings().never_expire_airports.read() } {
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
    s.map().tile_x(TileIndex(tile)) + u32::from(w) < (s.shared().map_max_x)() + 1
        && s.map().tile_y(TileIndex(tile)) + u32::from(h) < (s.shared().map_max_y)() + 1
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_spec(
    kind: u8,
    s: *const Services,
) -> *const AirportSpec {
    get(kind, unsafe { &*s })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_available_abi(
    spec: *const AirportSpec,
    s: *const Services,
) -> u8 {
    u8::from(available(spec, unsafe { &*s }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_in_bounds_abi(
    spec: *const AirportSpec,
    layout: u8,
    tile: u32,
    s: *const Services,
) -> u8 {
    u8::from(in_bounds(spec, layout, tile, unsafe { &*s }))
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
    (s.shared()
        .specs()
        .airport_tile()
        .standard()
        .get_sprite_group)(tile.tile_spec, !tile.station.is_null())
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_root(
    state: *const State,
    s: *const Services,
) -> *const c_void {
    let c = unsafe { *state };
    let s = unsafe { &*s };
    if c.tile_spec.is_null() {
        (s.shared().specs().airport().standard().get_sprite_group)(c.spec, !c.station.is_null())
    } else {
        (s.shared()
            .specs()
            .airport_tile()
            .standard()
            .get_sprite_group)(c.tile_spec, !c.station.is_null())
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
/// `AirportScopeResolver::StorePSA`: store a value into the object's persistent storage.
/// @param pos Position in the persistent storage to use.
/// @param value Value to store.
///
/// # Safety
/// A non-null `st` is a live station, and a non-null `grffile` the resolver's live file.
pub(crate) unsafe fn store_psa(
    s: &Services,
    st: *mut BaseStation,
    grffile: *const crate::newgrf::GRFFile,
    pos: u32,
    value: i32,
) {
    if st.is_null() {
        return;
    }

    let psa = if let Some(psa) = unsafe { s.airport_psa(st) } {
        psa
    } else {
        /* There is no need to create a storage if the value is zero. */
        if value == 0 {
            return;
        }

        /* Create storage on first modification. */
        let grfid = unsafe { crate::newgrf::GRFFileRef::new(s.shared(), grffile) }
            .map_or(0, crate::newgrf::GRFFileRef::grfid);
        let storage = unsafe { &*s.storage };
        crate::services_assert::assert_with(|| PersistentStorageRef::can_allocate_item(storage));
        let psa = PersistentStorageRef::new(
            storage,
            grfid,
            crate::newgrf::GrfSpecFeature::GSF_AIRPORTS,
            TileIndex((s.commands().airport_tile)(st)),
        );
        (s.set_airport_psa)(st, psa.shell());
        psa
    };
    psa.store_value(pos, value);
}
/// `AirportScopeResolver::StorePSA`, for the C++ scope resolver.
///
/// # Safety
/// As [`store_psa`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_storage_airport(
    s: &Services,
    st: *mut BaseStation,
    grffile: *const crate::newgrf::GRFFile,
    pos: u32,
    value: i32,
) {
    unsafe { store_psa(s, st, grffile, pos, value) }
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
        0x7a => {
            return (s.shared().specs().airport().badges)(c.spec).get_badge_variable_result(
                s.shared(),
                resolver,
                parameter,
            );
        }
        _ => {}
    }
    if c.station.is_null() {
        unsafe {
            *available = false;
        }
        return u32::MAX;
    }
    match variable {
        0x7c => match unsafe { s.airport_psa(c.station) } {
            Some(psa) => psa.get_value(parameter) as u32,
            None => 0,
        },
        0xf0 => u32::from((s.facilities)(c.station)),
        0xfa => (s.build_date)(c.station)
            .wrapping_sub(crate::timer_game_common::DAYS_TILL_ORIGINAL_BASE_YEAR)
            .clamp(0, 65535) as u32,
        _ => unsafe {
            crate::newgrf_station::station_base_variable(
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
    let gfx = crate::station_map::get_airport_gfx_raw(tile, s.shared(), s);
    let spec = (s.tile_spec)(gfx);
    if gfx < NEW_AIRPORTTILE_OFFSET {
        let over = (s.shared().specs().airport_tile().substitute().override_id)(spec);
        if over == INVALID_AIRPORTTILE {
            return 0xff00 | u32::from(gfx);
        }
        let other = (s.tile_spec)(over as u8);
        return if (s.shared().specs().airport_tile().grfid)(other) == grfid {
            u32::from((s.shared().specs().airport_tile().local_id)(other))
        } else {
            0xfffe
        };
    }
    if (s
        .shared()
        .specs()
        .airport_tile()
        .standard()
        .has_sprite_groups)(spec)
    {
        if (s.shared().specs().airport_tile().grfid)(spec) == grfid {
            u32::from((s.shared().specs().airport_tile().local_id)(spec))
        } else {
            0xfffe
        }
    } else {
        0xff00
            | u32::from((s.shared().specs().airport_tile().substitute().subst_id)(
                spec,
            ))
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
        0x41 => (s.shared().terrain_type)(c.tile),
        0x42 => {
            let town = crate::town::spatial::closest(s.shared(), c.tile, u32::MAX);
            let owner = (s.shared().town_owner)(town);
            u32::from(
                unsafe { crate::town::get_town_radius_group(owner, TileIndex(c.tile), s.map()) }.0,
            )
        }
        0x43 => crate::industry::resolver::get_relative_position(
            TileIndex(c.tile),
            TileIndex((s.commands().airport_tile)(c.station)),
            s.map(),
        ),
        0x44 => u32::from((s.frame)(c.tile)),
        0x60 => {
            let tile = if parameter as u8 == 0 {
                c.tile
            } else {
                (s.shared().nearby_tile)(parameter as u8, c.tile)
            };
            let same = (s.shared().tile_type)(tile) == crate::tile_type::MP_STATION
                && crate::station_map::is_airport_raw(tile, s.shared())
                && (s.shared().station_index)(tile) == (s.station_id)(c.station);
            (s.shared().nearby_tile_information)(
                tile,
                (s.shared().resolver_grf_version)(resolver) >= 8,
            ) | (u32::from(same) << 8)
        }
        0x61 => {
            let tile = (s.shared().nearby_tile)(parameter as u8, c.tile);
            if (s.belongs)(c.station, tile) {
                u32::from((s.frame)(tile))
            } else {
                u32::MAX
            }
        }
        0x62 => id_at_offset(
            (s.shared().nearby_tile)(parameter as u8, c.tile),
            c.station,
            (s.shared().resolver_grf_file_id)(resolver),
            s,
        ),
        0x7a => (s.shared().specs().airport_tile().badges)(c.tile_spec).get_badge_variable_result(
            s.shared(),
            resolver,
            parameter,
        ),
        _ => {
            debug!(grf, 1, "Unhandled airport tile variable 0x{:X}", variable);
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
    let over = (s.shared().specs().airport_tile().substitute().override_id)((s.tile_spec)(gfx));
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
    (s.tile_spec)(crate::station_map::get_airport_gfx_raw(tile, s.shared(), s))
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
    "OpenTTDAirportResolverServices" ["probe"] => Services { shared, storage, airport_psa, set_airport_psa, vehicles, by_tile, station_id, facilities, get_station, check, engine_if_valid, engine_id,   fta_flags, has_hangar, hangar_tile, depot_count, depot_offset, rotated_offset, fta_heading, fta_position, dummy_spec, dummy_fta, fta, catchment, base_spec, min_year, max_year, station_layout, random, build_date, tile_spec, belongs, tile_random, frame, base_scope, town_scope, evaluate_tile, };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_can_engine_use_station_abi(
    engine: u16,
    station: *const BaseStation,
    s: *const Services,
) -> u8 {
    u8::from(crate::vehicle_func::can_engine_use_station(
        crate::engine_type::EngineID(engine),
        station,
        unsafe { &*s },
    ))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_can_vehicle_use_station_abi(
    vehicle: *const crate::vehicle::OpenTTDVehicle,
    station: *const BaseStation,
    s: *const Services,
) -> u8 {
    u8::from(crate::vehicle_func::can_vehicle_use_station(
        crate::vehicle::VehicleRef::of(vehicle.cast_mut()),
        station,
        unsafe { &*s },
    ))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_is_hangar_abi(tile: u32, s: *const Services) -> u8 {
    u8::from(crate::station_map::is_hangar_raw(tile, unsafe { &*s }))
}

/// `AirportClassID`: list of default airport classes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AirportClassID(pub u8);
impl AirportClassID {
    /// Maximum number of airport classes.
    pub const APC_MAX: Self = Self(16);
}

/// `AirportClass`: the class registry of the airport specs.
pub(crate) type AirportClass = crate::newgrf_class::NewGRFClass<crate::station_cmd::AirportSpec>;

static AIRPORT_REGISTRY: GameState<Registry<crate::station_cmd::AirportSpec>> =
    GameState::new(Registry::new());
static AIRPORT_SERVICES: GameState<
    *const crate::newgrf_class::Services<crate::station_cmd::AirportSpec>,
> = GameState::new(std::ptr::null());
impl ClassSpec for crate::station_cmd::AirportSpec {
    type Index = crate::newgrf_airport::AirportClassID;
    type Triggers = u8;
    const SPEC_SERVICES: fn(&Specs) -> &SpecServices<Self, Self::Triggers> = Specs::airport;
    fn registry() -> *mut Registry<Self> {
        AIRPORT_REGISTRY.get()
    }
    fn services_slot() -> &'static GameState<*const crate::newgrf_class::Services<Self>> {
        &AIRPORT_SERVICES
    }
    /// `AirportClass::IsUIAvailable`.
    fn is_ui_available(_class: crate::newgrf_class::ClassRef<Self>, _index: u32) -> bool {
        true
    }
    /// `AirportClass::InsertDefaults`: reset the airport classes to their default state, the
    /// four default classes with their names, for the standard airports.
    fn insert_defaults() {
        let max = AirportClassID::APC_MAX;
        AirportClass::get(AirportClass::allocate(u32::from_be_bytes(*b"SMAL"), max))
            .set_name(STR_AIRPORT_CLASS_SMALL);
        AirportClass::get(AirportClass::allocate(u32::from_be_bytes(*b"LARG"), max))
            .set_name(STR_AIRPORT_CLASS_LARGE);
        AirportClass::get(AirportClass::allocate(u32::from_be_bytes(*b"HUB_"), max))
            .set_name(STR_AIRPORT_CLASS_HUB);
        AirportClass::get(AirportClass::allocate(u32::from_be_bytes(*b"HELI"), max))
            .set_name(STR_AIRPORT_CLASS_HELIPORTS);
    }
}

#[cfg(test)]
mod tests {
    use super::{BaseStation, Services, store_psa};
    use crate::newgrf::GrfSpecFeature;
    use crate::newgrf_storage::evidence_tests::fixture;
    use crate::newgrf_storage::{PersistentStorageArray, openttd_rust_storage256_get};
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::ptr;
    use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

    thread_local! {
        static PSA: Cell<*mut PersistentStorageArray<256>> = const { Cell::new(ptr::null_mut()) };
        static PSA_READS: Cell<usize> = const { Cell::new(0) };
    }
    extern "C" fn airport_psa(_: *const BaseStation) -> *mut PersistentStorageArray<256> {
        PSA_READS.set(PSA_READS.get() + 1);
        PSA.get()
    }
    extern "C" fn set_airport_psa(
        _: *mut BaseStation,
        shell: *mut crate::newgrf_storage::BasePersistentStorageArray,
    ) {
        PSA.set(fixture::array_of(shell));
    }
    extern "C" fn airport_tile(_: *const BaseStation) -> u32 {
        0x0506
    }
    static COMMANDS: AtomicPtr<crate::station_cmd::CommandServicesTable> =
        AtomicPtr::new(ptr::null_mut());
    extern "C" fn station_commands() -> *const crate::station_cmd::CommandServicesTable {
        COMMANDS.load(Ordering::SeqCst)
    }
    #[allow(unsafe_code, reason = "a load of the test's live file")]
    extern "C" fn grf_file_id(grffile: *const c_void) -> u32 {
        // SAFETY: each file is the test's live `u32` GRFID.
        unsafe { grffile.cast::<u32>().read() }
    }
    static FAILED_ASSERTIONS: AtomicUsize = AtomicUsize::new(0);
    extern "C" fn record_assertion(condition: bool) {
        if !condition {
            FAILED_ASSERTIONS.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// `AirportScopeResolver::StorePSA`: no station stores nothing and reads no storage;
    /// a zero value creates none; the first other value creates the storage with the
    /// file's GRFID, or 0 without a file, `GSF_AIRPORTS` and the airport's tile and
    /// publishes it in `airport.psa`, even at a position past the array, which stores
    /// nothing; a later value goes to that storage.
    #[test]
    #[allow(unsafe_code, reason = "stand-in station and file in an isolated child")]
    fn store_psa_creates_the_storage_on_the_first_nonzero_value() {
        const NAME: &str =
            "newgrf_airport::tests::store_psa_creates_the_storage_on_the_first_nonzero_value";
        if crate::services_assert::isolate_test_check(NAME, record_assertion) {
            return;
        }
        COMMANDS.store(
            Box::leak(Box::new(crate::station_cmd::CommandServicesTable {
                airport_tile,
                ..crate::test_tables::STATION_COMMANDS
            })),
            Ordering::SeqCst,
        );
        let shared = Box::leak(Box::new(crate::services::Services {
            station_commands,
            grf_file_id,
            ..crate::test_tables::SHARED
        }));
        let s = Services {
            shared,
            storage: &raw const fixture::STORAGE,
            airport_psa,
            set_airport_psa,
            ..Services::with_station_id(crate::test_tables::unreachable1)
        };
        let own = 0x3931_2345_u32;
        let file: *const crate::newgrf::GRFFile = (&raw const own).cast();
        let st: *mut BaseStation = ptr::dangling_mut();
        // SAFETY (each call): the station stand-in reaches only the leaves above, and the
        // file is live.
        unsafe { store_psa(&s, ptr::null_mut(), file, 0, 7) };
        assert_eq!(PSA_READS.get(), 0);
        unsafe { store_psa(&s, st, file, 0, 0) };
        assert_eq!((PSA_READS.get(), PSA.get()), (1, ptr::null_mut()));
        assert_eq!(fixture::created(), []);
        unsafe { store_psa(&s, st, ptr::null(), 256, 7) };
        assert_eq!(
            fixture::created(),
            [(0, GrfSpecFeature::GSF_AIRPORTS.0, 0x0506)]
        );
        assert!(!PSA.get().is_null());
        PSA.set(ptr::null_mut());
        unsafe { store_psa(&s, st, file, 255, 393) };
        assert_eq!(
            fixture::created(),
            [(own, GrfSpecFeature::GSF_AIRPORTS.0, 0x0506)]
        );
        unsafe { store_psa(&s, st, file, 254, -1) };
        assert_eq!(fixture::created(), []);
        // SAFETY: the storage just created.
        let get = |pos| unsafe { openttd_rust_storage256_get(PSA.get(), pos) };
        assert_eq!((get(255), get(254)), (393, -1));
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::SeqCst), 0);
    }
}
