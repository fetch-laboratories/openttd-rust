/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station/waypoint resolver policy. State lives in the native resolver stack
//! carrier for its whole lifetime, including repeated common Action2 resolutions.
//! Calls run under the game-state mutex. Native aliases and evaluator reentry
//! require field-sized reads/writes; no state borrow crosses a service call.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    clippy::if_not_else, // retain source branch order
    clippy::too_many_arguments
)]
use crate::cargo_type::{INVALID_CARGO, NUM_CARGO, is_valid_cargo_type};
use crate::direction_type::{AXIS_X, AXIS_Y, INVALID_AXIS, INVALID_DIAGDIR};
use crate::newgrf_commons::{SG_DEFAULT, SG_DEFAULT_NA, SG_PURCHASE};
use crate::slope_func::{SLOPE_E, SLOPE_EW, SLOPE_W};
use crate::station_type::SRT_CARGO_TAKEN;
use crate::{
    map_func::Map,
    services::{CommandCost, Services as Shared},
    station::BaseStation,
    station_cmd::RailSpec,
};
use std::{cell::UnsafeCell, ffi::c_void, ptr};
const TA_PLATFORM: u8 = 1;
const TA_WHOLE: u8 = 2;
#[repr(C)]
pub struct State {
    station: *mut BaseStation,
    spec: *const RailSpec,
    town: *const crate::services::TownHandle,
    tile: u32,
    cargo: u8,
    axis: u8,
    valid: u8,
    cache: [u32; 6],
}
#[repr(C)]
pub struct SpecIdentity {
    grfid: u32,
    local: u16,
}
#[repr(C)]
pub struct Reseed {
    reseed: u32,
    used: u32,
}
#[repr(C)]
pub struct Spec {
    threshold: u16,
    divide: bool,
}
#[repr(C)]
pub struct SlopeResult {
    callback: u16,
    registers: [i32; 16],
}
#[repr(C)]
pub struct Services {
    shared: *const Shared,
    cargo: *const crate::cargo_storage::Services,
    cargo_owner: extern "C" fn(*mut BaseStation, u8) -> *const crate::cargo_storage::List,
    group_count: extern "C" fn(*const RailSpec) -> usize,
    group_cargo: extern "C" fn(*const RailSpec, usize) -> u8,
    group: extern "C" fn(*const RailSpec, u8) -> *const c_void,
    spec: extern "C" fn(*const RailSpec) -> Spec,
    loading_count: extern "C" fn(*const c_void) -> usize,
    loaded_count: extern "C" fn(*const c_void) -> usize,
    loading: extern "C" fn(*const c_void, usize) -> *const c_void,
    loaded: extern "C" fn(*const c_void, usize) -> *const c_void,
    axis: extern "C" fn(u32) -> crate::services::Axis,
    tile_add: extern "C" fn(u32, i32) -> u32,
    tile_random: extern "C" fn(u32) -> u8,
    station_random: extern "C" fn(*mut BaseStation) -> u16,
    waiting: extern "C" fn(*mut BaseStation) -> u8,
    set_waiting: extern "C" fn(*mut BaseStation, u8),
    set_random: extern "C" fn(*mut BaseStation, u16),
    cached_cargo: extern "C" fn(*mut BaseStation) -> u64,
    spec_cargo: extern "C" fn(*const RailSpec) -> u64,
    resolve_random: extern "C" fn(*const RailSpec, *mut BaseStation, u32, u8) -> Reseed,
    resolve_callback: extern "C" fn(
        u16,
        u32,
        u32,
        *const RailSpec,
        *mut BaseStation,
        u32,
        u8,
        *mut i32,
        usize,
    ) -> u16,
    file_version: extern "C" fn(*const c_void) -> u8,
    company: extern "C" fn(u8, *const c_void) -> u32,
    reverse_rail: extern "C" fn(crate::services::RailType, *const c_void) -> u8,
    nearby: extern "C" fn(u8, u32) -> u32,
    nearby_station: extern "C" fn(u8, u32, crate::services::Axis) -> u32,
    nearby_info: extern "C" fn(u32, bool) -> u32,
    terrain: extern "C" fn(u32) -> u32,
    badges: extern "C" fn(*const c_void, *const RailSpec, u32) -> u32,
    translate: extern "C" fn(u8, *const c_void) -> u8,
    build_date: extern "C" fn(*mut BaseStation) -> i32,
    airport_spec: extern "C" fn(*mut BaseStation) -> *const crate::station_cmd::AirportSpec,
    airport_ttd_type: extern "C" fn(
        *const crate::station_cmd::AirportSpec,
    ) -> crate::newgrf_airport::TtdpAirportType,
    identity: extern "C" fn(*mut BaseStation, u8) -> SpecIdentity,
    unhandled: extern "C" fn(u8),
    slope_error: extern "C" fn(*mut CommandCost, *const SlopeResult, *const RailSpec),
}
struct Registry(UnsafeCell<*const Services>);
// SAFETY: installation and all resolver use are serialized by the game-state mutex.
unsafe impl Sync for Registry {}
static REGISTRY: Registry = Registry(UnsafeCell::new(ptr::null()));
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_resolver_exchange(
    native: *const Services,
) -> *const Services {
    unsafe {
        let old = *REGISTRY.0.get();
        *REGISTRY.0.get() = native;
        old
    }
}
fn services<'a>() -> &'a Services {
    unsafe { &**REGISTRY.0.get() }
}
impl Services {
    fn shared(&self) -> &Shared {
        unsafe { &*self.shared }
    }
    fn commands(&self) -> &crate::station_cmd::CommandServicesTable {
        unsafe { &*(self.shared().station_commands)() }
    }
    fn world(&self) -> &crate::station_service::World {
        unsafe { &*self.commands().station }
    }
    fn rail(&self) -> &crate::services::RailMapServices {
        unsafe { &*(*self.commands().rail).rail }
    }
    fn map(&self) -> Map {
        Map {
            log_x: (self.shared().map_log_x)() as u8,
        }
    }
    fn count(&self, st: *mut BaseStation, cargo: u8) -> u32 {
        let list = (self.cargo_owner)(st, cargo);
        if list.is_null() {
            0
        } else {
            unsafe { crate::cargo_storage::openttd_rust_cargo_list_station_total(list) }
        }
    }
    fn get_empty_mask(&self, st: *mut BaseStation) -> u64 {
        (0..NUM_CARGO).fold(0, |mask, c| {
            if self.count(st, c) == 0 {
                mask | 1_u64 << c
            } else {
                mask
            }
        })
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_resolver_init(
    state: *mut State,
    spec: *const RailSpec,
    st: *mut BaseStation,
    tile: u32,
) -> *const c_void {
    let native = services();
    unsafe {
        state.write(State {
            station: st,
            spec,
            town: ptr::null(),
            tile,
            cargo: SG_DEFAULT_NA,
            axis: INVALID_AXIS,
            valid: 0,
            cache: [0; 6],
        });
    }
    let mut cargo = SG_DEFAULT_NA;
    if st.is_null() {
        cargo = SG_PURCHASE;
    } else if (native.world().is_station)(st) {
        for i in 0..(native.group_count)(spec) {
            let c = (native.group_cargo)(spec, i);
            if c < NUM_CARGO && native.count(st, c) > 0 {
                cargo = c;
                break;
            }
        }
    }
    let mut root = (native.group)(spec, cargo);
    if root.is_null() {
        cargo = SG_DEFAULT;
        root = (native.group)(spec, cargo);
    }
    unsafe {
        (&raw mut (*state).cargo).write(cargo);
    }
    root
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_platform_info(
    axis: u8,
    tile: u8,
    mut platforms: i32,
    mut length: i32,
    mut x: i32,
    mut y: i32,
    centred: bool,
) -> u32 {
    if axis == AXIS_X {
        std::mem::swap(&mut platforms, &mut length);
        std::mem::swap(&mut x, &mut y);
    }
    let mut result = 0;
    if centred {
        x = x.wrapping_sub(platforms / 2).clamp(-8, 7);
        y = y.wrapping_sub(length / 2).clamp(-8, 7);
        result |= (y as u32 & 15) | ((x as u32 & 15) << 4);
    } else {
        result |= (y.min(15) as u32 & 15)
            | ((length.wrapping_sub(y).wrapping_sub(1).min(15) as u32 & 15) << 4)
            | ((x.min(15) as u32 & 15) << 8)
            | ((platforms.wrapping_sub(x).wrapping_sub(1).min(15) as u32 & 15) << 12);
    }
    result
        | ((length.min(15) as u32 & 15) << 16)
        | ((platforms.min(15) as u32 & 15) << 20)
        | (u32::from(tile) << 24)
}
fn find_end(
    native: &Services,
    mut tile: u32,
    delta: i32,
    check_type: bool,
    check_axis: bool,
) -> u32 {
    let sid = (native.shared().station_index)(tile);
    let kind = if check_type {
        (native.commands().custom_rail_index)(tile)
    } else {
        0
    };
    let axis = if check_axis {
        (native.axis)(tile).0
    } else {
        AXIS_X
    };
    loop {
        let next = (native.tile_add)(tile, delta);
        if (native.shared().tile_type)(next) != crate::tile_type::MP_STATION
            || (native.shared().station_index)(next) != sid
        {
            break;
        }
        if !(native.rail().has_station_rail)(next) {
            break;
        }
        if check_type && (native.commands().custom_rail_index)(next) != kind {
            break;
        }
        if check_axis && (native.axis)(next).0 != axis {
            break;
        }
        tile = next;
    }
    tile
}
fn platform(
    native: &Services,
    tile: u32,
    check_type: bool,
    check_axis: bool,
    centred: bool,
) -> u32 {
    let map = native.map();
    let sx = map.tile_x(find_end(
        native,
        tile,
        map.diff(-1, 0) as i32,
        check_type,
        check_axis,
    ));
    let sy = map.tile_y(find_end(
        native,
        tile,
        map.diff(0, -1) as i32,
        check_type,
        check_axis,
    ));
    let ex = map
        .tile_x(find_end(
            native,
            tile,
            map.diff(1, 0) as i32,
            check_type,
            check_axis,
        ))
        .wrapping_add(1);
    let ey = map
        .tile_y(find_end(
            native,
            tile,
            map.diff(0, 1) as i32,
            check_type,
            check_axis,
        ))
        .wrapping_add(1);
    openttd_rust_platform_info(
        (native.axis)(tile).0,
        (native.shared().station_gfx)(tile),
        ex.wrapping_sub(sx) as i32,
        ey.wrapping_sub(sy) as i32,
        map.tile_x(tile).wrapping_sub(sx) as i32,
        map.tile_y(tile).wrapping_sub(sy) as i32,
        centred,
    )
}
fn continuation(native: &Services, tile: u32) -> u32 {
    let (dirs, exits) = if (native.axis)(tile).0 == AXIS_X {
        ([5, 1, 3, 7, 4, 2, 6, 0], [2, 0, 1, 3, 2, 0, 2, 0])
    } else {
        ([3, 7, 5, 1, 4, 6, 2, 0], [1, 3, 2, 0, 1, 3, 1, 3])
    };
    let map = native.map();
    let mut result = 0;
    for i in 0..8 {
        let neighbour = map.add_dir(tile, dirs[i]);
        let bits = crate::track_func::track_status_to_track_bits(crate::movement::track_status(
            native.shared(),
            neighbour,
            crate::transport_type::TRANSPORT_RAIL,
            0,
            INVALID_DIAGDIR,
        ));
        if bits != 0 {
            result |= 1 << (i + 8);
            if (native.shared().tile_type)(neighbour) == crate::tile_type::MP_TUNNELBRIDGE
                && (native.shared().tunnel_bridge_direction)(neighbour) != exits[i]
            {
                continue;
            }
            if bits & crate::track_func::diagdir_reaches_tracks(exits[i]) != 0 {
                result |= 1 << i;
            }
        }
    }
    result
}
unsafe fn cached_variable(state: *mut State, var: u8, out: *mut u32) -> bool {
    let (slot, typed, axis, centred) = match var {
        0x40 => (0, false, false, false),
        0x41 => (1, true, false, false),
        0x45 => (2, false, false, false),
        0x46 => (3, false, false, true),
        0x47 => (4, true, false, true),
        0x49 => (5, false, true, false),
        _ => return false,
    };
    let native = services();
    if unsafe { (*state).valid } & (1 << slot) == 0 {
        let tile = unsafe { (*state).tile };
        let value = if var == 0x45 {
            continuation(native, tile)
        } else {
            platform(native, tile, typed, axis, centred)
        };
        unsafe {
            (&raw mut (*state).cache)
                .cast::<u32>()
                .add(slot)
                .write(value);
            (&raw mut (*state).valid).write((*state).valid | (1 << slot));
        }
    }
    unsafe {
        out.write((&raw const (*state).cache).cast::<u32>().add(slot).read());
    }
    true
}
/// Canonical lazy parent selection. A null result is retried on the next call,
/// just as the native optional scope is absent until successful construction.
/// Scope storage remains a native common-Action2 shell; Rust owns its selection.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_resolver_town(
    state: *mut State,
) -> *const crate::services::TownHandle {
    let town = unsafe { (*state).town };
    if !town.is_null() {
        return town;
    }
    let native = services();
    let st = unsafe { (*state).station };
    let tile = unsafe { (*state).tile };
    let town = if !st.is_null() {
        (native.commands().station_town)(st)
    } else if tile != u32::MAX {
        (native.shared().closest_town)(tile, u32::MAX)
    } else {
        ptr::null()
    };
    unsafe {
        (&raw mut (*state).town).write(town);
    }
    town
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_scope_is_parent(
    state: *mut State,
    scope: u8,
) -> bool {
    // VSG_SCOPE_PARENT: parent selection ends all state borrows before the native
    // TownScopeResolver is entered, including persistent-storage callbacks.
    scope == 1 && !unsafe { openttd_rust_station_resolver_town(state) }.is_null()
}
fn unhandled(native: &Services, variable: u8, available: *mut bool) -> u32 {
    (native.unhandled)(variable);
    unsafe {
        available.write(false);
    }
    u32::MAX
}
fn cargo_variable(
    native: &Services,
    st: *mut BaseStation,
    owner: *mut crate::station_service::Service,
    c: u8,
    var: u8,
    legacy: bool,
) -> u32 {
    match var {
        0x60 => {
            let count = native.count(st, c);
            if legacy { count } else { count.min(4095) }
        }
        0x63 | 0x68 => {
            let list = (native.cargo_owner)(st, c);
            if list.is_null() {
                return if var == 0x68 {
                    u32::from(crate::station_type::INVALID_STATION)
                } else {
                    0
                };
            }
            unsafe {
                if var == 0x68 {
                    u32::from(crate::cargo_storage::openttd_rust_cargo_list_first(
                        list,
                        native.cargo,
                    ))
                } else {
                    crate::cargo_storage::openttd_rust_cargo_list_periods(list)
                }
            }
        }
        _ => unsafe { crate::station_service::newgrf_cargo_variable(owner, c, var, legacy) },
    }
}
/// Complete station/waypoint virtual variable policy, also used by other native
/// resolver shells. Cargo/state/list owners are acquired at the original points.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_base_variable(
    st: *mut BaseStation,
    file: *const c_void,
    variable: u8,
    parameter: u8,
    available: *mut bool,
) -> u32 {
    let native = services();
    if !(native.world().is_station)(st) {
        return match variable {
            0x48 | 0xf1..=0xf3 | 0xf6 | 0xf7 | 0x60..=0x65 => 0,
            0x8a => u32::from(crate::station_type::HVOT_WAYPOINT),
            0x8c..=0xec => match (variable - 0x8c) & 7 {
                3 => u32::from(crate::station_base::INITIAL_STATION_RATING),
                4 => u32::from(crate::station_type::INVALID_STATION),
                _ => 0,
            },
            _ => unhandled(native, variable, available),
        };
    }
    match variable {
        0x48 => {
            return unsafe {
                crate::station_service::acceptance_mask((native.world().owner)(st)) as u32
            };
        }
        0x8a => {
            return u32::from(unsafe { ((*native.commands().aircraft).had_vehicle)(st.cast()) });
        }
        0xf1 => {
            return if (native.commands().airport_tile)(st) != u32::MAX {
                u32::from((native.airport_ttd_type)((native.airport_spec)(st)).0)
            } else {
                u32::from(crate::newgrf_airport::ATP_TTDP_LARGE)
            };
        }
        0xf2 | 0xf3 => {
            let stops = unsafe { &*(native.shared().road_stops)() };
            let stop = (stops.head)(st, variable == 0xf3);
            return if stop.is_null() {
                0
            } else {
                u32::from(unsafe { (*((stops.state)(stop))).status })
            };
        }
        0xf6 | 0xf7 => {
            let bits = unsafe { ((*native.commands().aircraft).airport_blocks)(st.cast()).read() };
            return if variable == 0xf6 {
                bits as u32
            } else {
                (bits >> 8) as u32 & 255
            };
        }
        _ => {}
    }
    if (0x60..=0x65).contains(&variable) || variable == 0x69 {
        let c = (native.translate)(parameter, file);
        if !is_valid_cargo_type(c) {
            return match variable {
                0x62 => u32::MAX,
                0x64 => 0xff00,
                _ => 0,
            };
        }
        let owner = (native.world().owner)(st);
        return cargo_variable(native, st, owner, c, variable, false);
    }
    if (0x8c..=0xec).contains(&variable) {
        let c = ((variable - 0x8c) >> 3) & 15;
        let owner = (native.world().owner)(st);
        return match (variable - 0x8c) & 7 {
            0 => cargo_variable(native, st, owner, c, 0x60, true),
            1 => {
                native.count(st, c).min(4095) & 15
                    | (unsafe {
                        crate::station_service::newgrf_cargo_variable(owner, c, 0x65, true)
                    } << 4)
            }
            2 => cargo_variable(native, st, owner, c, 0x61, true),
            3 => cargo_variable(native, st, owner, c, 0x62, true),
            4 => cargo_variable(native, st, owner, c, 0x68, true),
            5 => cargo_variable(native, st, owner, c, 0x63, true),
            6 => cargo_variable(native, st, owner, c, 0x66, true),
            7 => cargo_variable(native, st, owner, c, 0x67, true),
            _ => unreachable!(),
        };
    }
    unhandled(native, variable, available)
}
fn land_info(native: &Services, file: *const c_void, tile: u32, axis: u8) -> u32 {
    let slope = (native.shared().tile_slope)(tile);
    let swap = axis == AXIS_Y && (slope & SLOPE_W != 0) != (slope & SLOPE_E != 0);
    (native.nearby_info)(tile, (native.file_version)(file) >= 8)
        ^ if swap { u32::from(SLOPE_EW) } else { 0 }
}
fn build_date(date: i32) -> u32 {
    date.wrapping_sub(crate::timer_game_common::DAYS_TILL_ORIGINAL_BASE_YEAR)
        .clamp(0, i32::from(u16::MAX)) as u32
}
/// Full scope policy. Field-sized observations end before common evaluator or
/// parent storage reentry; caches survive every `DoResolve` of this native shell.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_variable(
    state: *mut State,
    resolver: *const c_void,
    file: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
) -> u32 {
    let native = services();
    let st = unsafe { (*state).station };
    let tile = unsafe { (*state).tile };
    let spec = unsafe { (*state).spec };
    if st.is_null() {
        return match variable {
            0x40 | 0x41 | 0x46 | 0x47 | 0x49 => 0x0211_0000,
            0x42 => 0,
            0x43 => (native.company)((native.shared().current_company)(), ptr::null()),
            0x44 => 2,
            0x67 if unsafe { (*state).axis } != INVALID_AXIS && tile != u32::MAX => {
                let axis = unsafe { (*state).axis };
                let nearby = if parameter != 0 {
                    (native.nearby_station)(parameter as u8, tile, crate::services::Axis(axis))
                } else {
                    tile
                };
                land_info(native, file, nearby, axis)
            }
            0x7a => (native.badges)(resolver, spec, parameter),
            0xfa => build_date((native.shared().calendar_date)()),
            _ => {
                unsafe {
                    available.write(false);
                }
                u32::MAX
            }
        };
    }
    let mut cached = 0;
    if unsafe { cached_variable(state, variable, &raw mut cached) } {
        return cached;
    }
    match variable {
        0x42 => {
            (native.terrain)(tile)
                | (u32::from((native.reverse_rail)((native.rail().rail_type)(tile), file)) << 8)
        }
        0x43 => (native.company)((native.world().station_owner)(st), ptr::null()),
        0x44 => {
            if (crate::pbs::Pbs::acquire(native.shared()).0.station_reserved)(tile) {
                7
            } else {
                4
            }
        }
        0x4a => {
            u32::from(unsafe { ((*crate::animated_tile::services().industry_tiles).frame)(tile) })
        }
        0x66 => {
            let nearby = if parameter != 0 {
                (native.nearby)(parameter as u8, tile)
            } else {
                tile
            };
            if (crate::animated_tile::services().rail_belongs)(st, nearby) {
                u32::from(unsafe {
                    ((*crate::animated_tile::services().industry_tiles).frame)(nearby)
                })
            } else {
                u32::MAX
            }
        }
        0x67 => {
            let axis = (native.axis)(tile).0;
            let nearby = if parameter != 0 {
                (native.nearby)(parameter as u8, tile)
            } else {
                tile
            };
            land_info(native, file, nearby, axis)
        }
        0x68 | 0x6a | 0x6b => {
            let nearby = (native.nearby)(parameter as u8, tile);
            if !(native.rail().has_station_rail)(nearby) {
                return u32::MAX;
            }
            let index = (native.commands().custom_rail_index)(nearby);
            if variable == 0x6a && index == 0 {
                return 0;
            }
            if variable == 0x6b && index == 0 {
                return 0xfffe;
            }
            let origin_id = if variable != 0x6a {
                (native.identity)(st, (native.commands().custom_rail_index)(tile) as u8).grfid
            } else {
                0
            };
            let mut value = if variable == 0x68 {
                (u32::from((native.shared().station_gfx)(nearby) >> 1 & 3) << 12)
                    | (u32::from((native.axis)(tile).0 != (native.axis)(nearby).0) << 11)
                    | (u32::from((crate::animated_tile::services().rail_belongs)(st, nearby)) << 10)
            } else {
                0
            };
            if index != 0 {
                let base = (crate::pbs::Pbs::acquire(native.shared()).0.station)(nearby);
                let id = (native.identity)(base, index as u8);
                if variable == 0x6a {
                    return id.grfid;
                }
                if variable == 0x6b {
                    return if id.grfid == origin_id {
                        u32::from(id.local)
                    } else {
                        0xfffe
                    };
                }
                value |=
                    (1 << if id.grfid != origin_id { 9 } else { 8 }) | u32::from(id.local.min(255));
            }
            value
        }
        0x7a => (native.badges)(resolver, spec, parameter),
        0x82 => 50,
        0x84 => (native.commands().string_id)(st),
        0x86 => 0,
        0xf0 => u32::from((native.world().facilities)(st)),
        0xfa => build_date((native.build_date)(st)),
        _ => unsafe {
            openttd_rust_station_base_variable(st, file, variable, parameter as u8, available)
        },
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_resolve_real(
    state: *const State,
    group: *const c_void,
) -> *const c_void {
    let native = services();
    let st = unsafe { (*state).station };
    let loading = (native.loading_count)(group);
    if st.is_null() || !(native.world().is_station)(st) {
        return if loading > 0 {
            (native.loading)(group, 0)
        } else {
            ptr::null()
        };
    }
    let c = unsafe { (*state).cargo };
    let mut cargo = match c {
        INVALID_CARGO | SG_PURCHASE | SG_DEFAULT_NA => 0,
        SG_DEFAULT => (0..NUM_CARGO).fold(0_u32, |n, c| n.wrapping_add(native.count(st, c))),
        _ => native.count(st, c),
    };
    let spec = (native.spec)(unsafe { (*state).spec });
    if spec.divide {
        let a = unsafe { (native.commands().train_area)(st) };
        cargo /= u32::from(a.w) + u32::from(a.h);
    }
    cargo = cargo.min(0xfff);
    let threshold = u32::from(spec.threshold);
    if cargo > threshold {
        if loading > 0 {
            return (native.loading)(
                group,
                (cargo.wrapping_sub(threshold).wrapping_mul(loading as u32)
                    / (4096_u32.wrapping_sub(threshold))) as usize,
            );
        }
    } else {
        let loaded = (native.loaded_count)(group);
        if loaded > 0 {
            return (native.loaded)(
                group,
                (cargo.wrapping_mul(loaded as u32) / threshold.wrapping_add(1)) as usize,
            );
        }
    }
    if loading > 0 {
        (native.loading)(group, 0)
    } else {
        ptr::null()
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_random_bits(state: *const State) -> u32 {
    let native = services();
    let st = unsafe { (*state).station };
    let tile = unsafe { (*state).tile };
    (if st.is_null() {
        0
    } else {
        u32::from((native.station_random)(st))
    }) | if tile == u32::MAX {
        0
    } else {
        u32::from((native.tile_random)(tile)) << 16
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_random_triggers(state: *const State) -> u32 {
    let st = unsafe { (*state).station };
    if st.is_null() {
        0
    } else {
        u32::from((services().waiting)(st))
    }
}
/// Shared `GetEmptyMask` reducer for remaining native callers.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_empty_mask(st: *mut BaseStation) -> u64 {
    services().get_empty_mask(st)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_randomisation(
    st: *mut BaseStation,
    tile: u32,
    trigger: u8,
    cargo: u8,
) {
    let native = services();
    let mask = (native.cached_cargo)(st);
    if mask == 0 || is_valid_cargo_type(cargo) && mask & 1_u64.wrapping_shl(u32::from(cargo)) == 0 {
        return;
    }
    let empty = if trigger == SRT_CARGO_TAKEN {
        native.get_empty_mask(st)
    } else {
        0
    };
    (native.set_waiting)(
        st,
        (native.waiting)(st) | (1_u8.wrapping_shl(u32::from(trigger))),
    );
    let a = crate::animated_tile::services();
    let area = (a.rail_area)(
        st,
        tile,
        if trigger <= SRT_CARGO_TAKEN {
            TA_WHOLE
        } else {
            TA_PLATFORM
        },
    );
    let map = native.map();
    let mut whole = 0;
    let mut used = 0;
    for y in 0..area.height {
        for x in 0..area.width {
            let t = area.tile.wrapping_add(map.diff(i32::from(x), i32::from(y)));
            if !(a.rail_belongs)(st, t) {
                continue;
            }
            let spec = (a.rail_spec)(t);
            if spec.is_null() {
                continue;
            }
            let triggers = (native.spec_cargo)(spec);
            if trigger == SRT_CARGO_TAKEN && triggers & !empty != 0 {
                continue;
            }
            if !is_valid_cargo_type(cargo) || triggers & 1_u64.wrapping_shl(u32::from(cargo)) != 0 {
                let r = (native.resolve_random)(spec, st, t, (native.waiting)(st));
                used |= r.used;
                if r.reseed != 0 {
                    whole |= r.reseed;
                    let bits = r.reseed >> 16;
                    let random = u32::from((native.tile_random)(t)) & !bits
                        | native.shared().random() & bits;
                    (native.commands().set_random_bits)(t, random as u8);
                    (native.shared().mark_dirty)(t);
                }
            }
        }
    }
    (native.set_waiting)(st, (native.waiting)(st) & !(used as u8));
    if whole & 0xffff != 0 {
        let random =
            u32::from((native.station_random)(st)) & !whole | native.shared().random() & whole;
        (native.set_random)(st, random as u16);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_callback(
    cb: u16,
    p1: u32,
    p2: u32,
    spec: *const RailSpec,
    st: *mut BaseStation,
    tile: u32,
    regs: *mut i32,
    count: usize,
) -> u16 {
    (services().resolve_callback)(cb, p1, p2, spec, st, tile, INVALID_AXIS, regs, count)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_slope_check(
    out: *mut CommandCost,
    north: u32,
    tile: u32,
    spec: *const RailSpec,
    axis: u8,
    length: u8,
    tracks: u8,
) {
    let native = services();
    let map = native.map();
    let diff = tile.wrapping_sub(north);
    let slope = u32::from((native.shared().tile_slope)(tile));
    let swap =
        axis == AXIS_Y && (slope & u32::from(SLOPE_W) != 0) != (slope & u32::from(SLOPE_E) != 0);
    let p1 = (slope << 4) | (slope ^ if swap { u32::from(SLOPE_EW) } else { 0 });
    let p2 = u32::from(tracks) << 24
        | u32::from(length) << 16
        | if axis == AXIS_Y {
            map.tile_x(diff) << 8 | map.tile_y(diff)
        } else {
            map.tile_y(diff) << 8 | map.tile_x(diff)
        };
    let mut r = SlopeResult {
        callback: 0,
        registers: [0; 16],
    };
    r.callback = (native.resolve_callback)(
        crate::newgrf_callbacks::CBID_STATION_LAND_SLOPE_CHECK.0,
        p1,
        p2,
        spec,
        ptr::null_mut(),
        tile,
        axis,
        r.registers.as_mut_ptr(),
        16,
    );
    if r.callback == crate::newgrf_callbacks::CALLBACK_FAILED {
        unsafe {
            ((*native.commands().cost).zero)(out);
        }
        return;
    }
    let a = crate::animated_tile::services();
    let b = unsafe { &*a.station };
    if !(b.version8)(spec) {
        r.callback ^= 1 << 10;
    }
    (native.slope_error)(out, &raw const r, spec);
}
crate::abi::layouts! {
    fn abi_layout;
 "OpenTTDStationResolverState" ["station_service"] => State { station, spec, town, tile, cargo, axis, valid, cache };
 "OpenTTDStationSpecIdentity" ["station_service"] => SpecIdentity { grfid, local };
 "OpenTTDStationReseed" ["station_service"] => Reseed { reseed, used };
 "OpenTTDStationResolverSpec" ["station_service"] => Spec { threshold, divide };
 "OpenTTDStationSlopeResult" ["station_service"] => SlopeResult { callback, registers };
 "OpenTTDStationResolverServices" ["station_service"] => Services {
 shared, cargo, cargo_owner, group_count, group_cargo, group, spec, loading_count, loaded_count,
 loading, loaded, axis, tile_add, tile_random, station_random, waiting, set_waiting, set_random,
 cached_cargo, spec_cargo, resolve_random, resolve_callback, file_version, company, reverse_rail, nearby, nearby_station, nearby_info, terrain, badges, translate, build_date, airport_spec, airport_ttd_type, identity, unhandled, slope_error
 };
}

/// Direct caller with no result-register span.
pub(crate) fn callback(
    cb: u16,
    p1: u32,
    p2: u32,
    spec: *const RailSpec,
    st: *mut BaseStation,
    tile: u32,
) -> u16 {
    openttd_rust_station_callback(cb, p1, p2, spec, st, tile, ptr::null_mut(), 0)
}
/// Command callers use the original fixed X axis and platform ordering.
pub(crate) fn platform_info(gfx: u8, length: u8, tracks: u8, x: u8, y: u8) -> u32 {
    openttd_rust_platform_info(
        AXIS_X,
        gfx,
        i32::from(length),
        i32::from(tracks),
        i32::from(x),
        i32::from(y),
        false,
    )
}
