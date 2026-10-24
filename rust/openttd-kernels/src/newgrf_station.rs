/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station/waypoint resolver policy. State lives in the native resolver's stack
//! carrier for its whole lifetime, including repeated common Action2 resolutions.
//! Calls run under the game-state mutex. Native aliases and evaluator reentry
//! require field-sized reads/writes; no state borrow crosses a service call.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
use crate::direction_type::{AXIS_X, AXIS_Y, INVALID_AXIS, INVALID_DIAGDIR};
use crate::{
    map_func::Map,
    services::{CommandCost, Services as Shared},
    station::BaseStation,
    station_cmd::RailSpec,
};
use std::{cell::UnsafeCell, ffi::c_void, ptr};
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
    slope_error: extern "C" fn(*mut CommandCost, *const SlopeResult, *const RailSpec),
}
struct Registry(UnsafeCell<*const Services>);
// SAFETY: installation and all resolver use are serialized by the game-state mutex.
unsafe impl Sync for Registry {}
static REGISTRY: Registry = Registry(UnsafeCell::new(ptr::null()));
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_resolver_exchange(
    s: *const Services,
) -> *const Services {
    unsafe {
        let old = *REGISTRY.0.get();
        *REGISTRY.0.get() = s;
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
    fn empty_mask(&self, st: *mut BaseStation) -> u64 {
        (0..64).fold(0, |mask, c| {
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
    let s = services();
    unsafe {
        state.write(State {
            station: st,
            spec,
            town: ptr::null(),
            tile,
            cargo: 66,
            axis: INVALID_AXIS,
            valid: 0,
            cache: [0; 6],
        });
    }
    let mut cargo = 66;
    if st.is_null() {
        cargo = 65;
    } else if (s.world().is_station)(st) {
        for i in 0..(s.group_count)(spec) {
            let c = (s.group_cargo)(spec, i);
            if c < 64 && s.count(st, c) > 0 {
                cargo = c;
                break;
            }
        }
    }
    let mut root = (s.group)(spec, cargo);
    if root.is_null() {
        cargo = 64;
        root = (s.group)(spec, cargo);
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
fn find_end(s: &Services, mut tile: u32, delta: i32, check_type: bool, check_axis: bool) -> u32 {
    let sid = (s.shared().station_index)(tile);
    let kind = if check_type {
        (s.commands().custom_rail_index)(tile)
    } else {
        0
    };
    let axis = if check_axis { (s.axis)(tile).0 } else { AXIS_X };
    loop {
        let next = (s.tile_add)(tile, delta);
        if (s.shared().tile_type)(next) != crate::tile_type::MP_STATION
            || (s.shared().station_index)(next) != sid
        {
            break;
        }
        if !(s.rail().has_station_rail)(next) {
            break;
        }
        if check_type && (s.commands().custom_rail_index)(next) != kind {
            break;
        }
        if check_axis && (s.axis)(next).0 != axis {
            break;
        }
        tile = next;
    }
    tile
}
fn platform(s: &Services, tile: u32, check_type: bool, check_axis: bool, centred: bool) -> u32 {
    let m = s.map();
    let sx = m.tile_x(find_end(
        s,
        tile,
        m.diff(-1, 0) as i32,
        check_type,
        check_axis,
    ));
    let sy = m.tile_y(find_end(
        s,
        tile,
        m.diff(0, -1) as i32,
        check_type,
        check_axis,
    ));
    let ex = m
        .tile_x(find_end(
            s,
            tile,
            m.diff(1, 0) as i32,
            check_type,
            check_axis,
        ))
        .wrapping_add(1);
    let ey = m
        .tile_y(find_end(
            s,
            tile,
            m.diff(0, 1) as i32,
            check_type,
            check_axis,
        ))
        .wrapping_add(1);
    openttd_rust_platform_info(
        (s.axis)(tile).0,
        (s.shared().station_gfx)(tile),
        ex.wrapping_sub(sx) as i32,
        ey.wrapping_sub(sy) as i32,
        m.tile_x(tile).wrapping_sub(sx) as i32,
        m.tile_y(tile).wrapping_sub(sy) as i32,
        centred,
    )
}
fn continuation(s: &Services, tile: u32) -> u32 {
    let (dirs, exits) = if (s.axis)(tile).0 == AXIS_X {
        ([5, 1, 3, 7, 4, 2, 6, 0], [2, 0, 1, 3, 2, 0, 2, 0])
    } else {
        ([3, 7, 5, 1, 4, 6, 2, 0], [1, 3, 2, 0, 1, 3, 1, 3])
    };
    let m = s.map();
    let mut result = 0;
    for i in 0..8 {
        let neighbour = m.add_dir(tile, dirs[i]);
        let bits = crate::track_func::track_status_to_track_bits(crate::movement::track_status(
            s.shared(),
            neighbour,
            crate::transport_type::TRANSPORT_RAIL,
            0,
            INVALID_DIAGDIR,
        ));
        if bits != 0 {
            result |= 1 << (i + 8);
            if (s.shared().tile_type)(neighbour) == crate::tile_type::MP_TUNNELBRIDGE
                && (s.shared().tunnel_bridge_direction)(neighbour) != exits[i]
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_cached_variable(
    state: *mut State,
    var: u8,
    out: *mut u32,
) -> bool {
    let (slot, typed, axis, centred) = match var {
        0x40 => (0, false, false, false),
        0x41 => (1, true, false, false),
        0x45 => (2, false, false, false),
        0x46 => (3, false, false, true),
        0x47 => (4, true, false, true),
        0x49 => (5, false, true, false),
        _ => return false,
    };
    let s = services();
    if unsafe { (*state).valid } & (1 << slot) == 0 {
        let tile = unsafe { (*state).tile };
        let value = if var == 0x45 {
            continuation(s, tile)
        } else {
            platform(s, tile, typed, axis, centred)
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_resolve_real(
    state: *const State,
    group: *const c_void,
) -> *const c_void {
    let s = services();
    let st = unsafe { (*state).station };
    let loading = (s.loading_count)(group);
    if st.is_null() || !(s.world().is_station)(st) {
        return if loading > 0 {
            (s.loading)(group, 0)
        } else {
            ptr::null()
        };
    }
    let c = unsafe { (*state).cargo };
    let mut cargo = match c {
        255 | 65 | 66 => 0,
        64 => (0..64).fold(0_u32, |n, c| n.wrapping_add(s.count(st, c))),
        _ => s.count(st, c),
    };
    let spec = (s.spec)(unsafe { (*state).spec });
    if spec.divide {
        let a = unsafe { (s.commands().train_area)(st) };
        cargo /= u32::from(a.w) + u32::from(a.h);
    }
    cargo = cargo.min(0xfff);
    let threshold = u32::from(spec.threshold);
    if cargo > threshold {
        if loading > 0 {
            return (s.loading)(
                group,
                (cargo.wrapping_sub(threshold).wrapping_mul(loading as u32)
                    / (4096_u32.wrapping_sub(threshold))) as usize,
            );
        }
    } else {
        let loaded = (s.loaded_count)(group);
        if loaded > 0 {
            return (s.loaded)(
                group,
                (cargo.wrapping_mul(loaded as u32) / threshold.wrapping_add(1)) as usize,
            );
        }
    }
    if loading > 0 {
        (s.loading)(group, 0)
    } else {
        ptr::null()
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_random_bits(state: *const State) -> u32 {
    let s = services();
    let st = unsafe { (*state).station };
    let tile = unsafe { (*state).tile };
    (if st.is_null() {
        0
    } else {
        u32::from((s.station_random)(st))
    }) | if tile == u32::MAX {
        0
    } else {
        u32::from((s.tile_random)(tile)) << 16
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
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_randomisation(
    st: *mut BaseStation,
    tile: u32,
    trigger: u8,
    cargo: u8,
) {
    let s = services();
    let mask = (s.cached_cargo)(st);
    if mask == 0 || cargo < 64 && mask & (1_u64 << cargo) == 0 {
        return;
    }
    let empty = if trigger == 1 { s.empty_mask(st) } else { 0 };
    (s.set_waiting)(st, (s.waiting)(st) | (1 << trigger));
    let a = crate::animated_tile::services();
    let area = (a.rail_area)(st, tile, if trigger < 2 { 2 } else { 1 });
    let m = s.map();
    let mut whole = 0;
    let mut used = 0;
    for y in 0..area.height {
        for x in 0..area.width {
            let t = area.tile.wrapping_add(m.diff(i32::from(x), i32::from(y)));
            if !(a.rail_belongs)(st, t) {
                continue;
            }
            let spec = (a.rail_spec)(t);
            if spec.is_null() {
                continue;
            }
            let triggers = (s.spec_cargo)(spec);
            if trigger == 1 && triggers & !empty != 0 {
                continue;
            }
            if cargo >= 64 || triggers & (1_u64 << cargo) != 0 {
                let r = (s.resolve_random)(spec, st, t, (s.waiting)(st));
                used |= r.used;
                if r.reseed != 0 {
                    whole |= r.reseed;
                    let bits = r.reseed >> 16;
                    let random = u32::from((s.tile_random)(t)) & !bits | s.shared().random() & bits;
                    (s.commands().set_random_bits)(t, random as u8);
                    (s.shared().mark_dirty)(t);
                }
            }
        }
    }
    (s.set_waiting)(st, (s.waiting)(st) & !(used as u8));
    if whole & 0xffff != 0 {
        let random = u32::from((s.station_random)(st)) & !whole | s.shared().random() & whole;
        (s.set_random)(st, random as u16);
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
    let s = services();
    unsafe {
        ((*s.commands().cost).zero)(out);
    }
    let m = s.map();
    let diff = tile.wrapping_sub(north);
    let slope = u32::from((s.shared().tile_slope)(tile));
    let swap = axis == AXIS_Y && (slope & 1 != 0) != (slope & 4 != 0);
    let p1 = (slope << 4) | (slope ^ if swap { 5 } else { 0 });
    let p2 = u32::from(tracks) << 24
        | u32::from(length) << 16
        | if axis == AXIS_Y {
            m.tile_x(diff) << 8 | m.tile_y(diff)
        } else {
            m.tile_y(diff) << 8 | m.tile_x(diff)
        };
    let mut r = SlopeResult {
        callback: 0,
        registers: [0; 16],
    };
    r.callback = (s.resolve_callback)(
        0x149,
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
        return;
    }
    let a = crate::animated_tile::services();
    let b = unsafe { &*a.station };
    if !(b.version8)(spec) {
        r.callback ^= 1 << 10;
    }
    (s.slope_error)(out, &raw const r, spec);
}
crate::abi::layouts! {
 fn abi_layout;
 "OpenTTDStationResolverState" ["station_service"] => State { station, spec, town, tile, cargo, axis, valid, cache };
 "OpenTTDStationReseed" ["station_service"] => Reseed { reseed, used };
 "OpenTTDStationResolverSpec" ["station_service"] => Spec { threshold, divide };
 "OpenTTDStationSlopeResult" ["station_service"] => SlopeResult { callback, registers };
 "OpenTTDStationResolverServices" ["station_service"] => Services {
 shared, cargo, cargo_owner, group_count, group_cargo, group, spec, loading_count, loaded_count,
 loading, loaded, axis, tile_add, tile_random, station_random, waiting, set_waiting, set_random,
 cached_cargo, spec_cargo, resolve_random, resolve_callback, slope_error
 };
}
