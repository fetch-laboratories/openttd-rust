/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical NOSAVE tile index. Native shells own only these stable intrusive
//! links, never a second index. The game-state mutex serializes all access.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use super::{OpenTTDVehicle, VehicleRef, VehicleServices};
use crate::{map_func::Map, tile_type::TILE_SIZE};
use std::{cell::UnsafeCell, ptr};

const HASH_BITS: u32 = 7;
const HASH_SIZE: u32 = 1 << HASH_BITS;
const HASH_MASK: u32 = HASH_SIZE - 1;
const TOTAL: usize = (HASH_SIZE * HASH_SIZE) as usize;
type Shell = *mut OpenTTDVehicle;
/// Field layout is shared, not the Vehicle layout. Native construction zeros it;
/// the Rust owner alone changes links. Shell addresses never move while indexed.
#[repr(C)]
pub struct Links {
    pub next: Shell,
    pub prev: *mut Shell,
    pub current: *mut Shell,
}
struct Buckets(UnsafeCell<[Shell; TOTAL]>);
// SAFETY: The game-state mutex excludes concurrent index access, including GUI
// commands. No reference into this static or a shell spans a callback or reset.
unsafe impl Sync for Buckets {}
static BUCKETS: Buckets = Buckets(UnsafeCell::new([ptr::null_mut(); TOTAL]));
fn hash_1d(p: u32) -> u32 {
    p & HASH_MASK
}
fn increment(h: u32) -> u32 {
    h.wrapping_add(1) & HASH_MASK
}
fn compose(x: u32, y: u32) -> usize {
    (x | (y << HASH_BITS)) as usize
}
fn bucket(x: u32, y: u32) -> *mut Shell {
    BUCKETS
        .0
        .get()
        .cast::<Shell>()
        .wrapping_add(compose(hash_1d(x), hash_1d(y)))
}
fn tile_bucket(s: &VehicleServices, tile: u32) -> *mut Shell {
    let map = Map {
        log_x: (s.map_log_x)() as u8,
    };
    bucket(map.tile_x(tile), map.tile_y(tile))
}
unsafe fn handle(shell: Shell) -> VehicleRef {
    VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(shell) })
}
/// The current shell must survive advancement (as in the native iterator).
/// Callbacks may change links: read LIVE next only AFTER the callback returns.
unsafe fn next(s: &VehicleServices, shell: Shell) -> Shell {
    unsafe { (*((s.tile_links)(handle(shell)))).next }
}
/// Complete `UpdateVehicleTileHash`, including stale next/prev after removal.
pub(crate) unsafe fn update(s: &VehicleServices, v: VehicleRef, remove: bool) {
    let links = (s.tile_links)(v);
    let old = unsafe { (*links).current };
    let new = if remove {
        ptr::null_mut()
    } else {
        tile_bucket(s, (s.tile)(v))
    };
    if old == new {
        return;
    }
    unsafe {
        if !old.is_null() {
            let n = (*links).next;
            if !n.is_null() {
                (*((s.tile_links)(handle(n)))).prev = (*links).prev;
            }
            *(*links).prev = (*links).next;
        }
        if !new.is_null() {
            (*links).next = *new;
            if !(*links).next.is_null() {
                (*((s.tile_links)(handle((*links).next)))).prev = ptr::addr_of_mut!((*links).next);
            }
            (*links).prev = new;
            *new = v.0.as_ptr();
        }
        (*links).current = new;
    }
}
/// No pool allocation/deletion/yield occurs during reset. Retain stale links.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_reset(s: &super::scheduler::SchedulerServices) {
    let pool = (s.pool)();
    let vs = unsafe { &*s.vehicle };
    for i in 0..unsafe { *pool.first_unused } {
        if let Some(v) = unsafe { *pool.slots.add(i) } {
            unsafe {
                (*((vs.tile_links)(v))).current = ptr::null_mut();
            }
        }
    }
    unsafe {
        ptr::write_bytes(BUCKETS.0.get().cast::<Shell>(), 0, TOTAL);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_update(
    s: &VehicleServices,
    v: VehicleRef,
    remove: bool,
) {
    unsafe {
        update(s, v, remove);
    }
}
/// Complete tile cursor; no reference into index storage is retained.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OnTile {
    pub tile: u32,
    pub current: Shell,
}
impl OnTile {
    pub(crate) unsafe fn new(s: &VehicleServices, tile: u32) -> Self {
        let mut it = Self {
            tile,
            current: unsafe { *tile_bucket(s, tile) },
        };
        unsafe {
            it.filter(s);
        }
        it
    }
    unsafe fn filter(&mut self, s: &VehicleServices) {
        while !self.current.is_null() && (s.tile)(unsafe { handle(self.current) }) != self.tile {
            self.current = unsafe { next(s, self.current) };
        }
    }
    pub(crate) unsafe fn advance(&mut self, s: &VehicleServices) {
        self.current = unsafe { next(s, self.current) };
        unsafe {
            self.filter(s);
        }
    }
    pub(crate) fn get(&self) -> Option<VehicleRef> {
        std::ptr::NonNull::new(self.current).map(VehicleRef)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_on_tile(s: &VehicleServices, tile: u32) -> OnTile {
    unsafe { OnTile::new(s, tile) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_on_advance(s: &VehicleServices, it: &mut OnTile) {
    unsafe {
        it.advance(s);
    }
}
/// Complete near cursor. Integer fields preserve native unsigned arithmetic and
/// `Rect::Contains` wrap, including extreme coordinates in the default probe.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Near {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub hxmin: u32,
    pub hxmax: u32,
    pub hymin: u32,
    pub hymax: u32,
    pub hx: u32,
    pub hy: u32,
    pub current: Shell,
}
impl Near {
    pub(crate) unsafe fn new(s: &VehicleServices, x: i32, y: i32, distance: u32) -> Self {
        let left = (x as u32).wrapping_sub(distance) as i32;
        let right = (x as u32).wrapping_add(distance) as i32;
        let top = (y as u32).wrapping_sub(distance) as i32;
        let bottom = (y as u32).wrapping_add(distance) as i32;
        let mut it = Self {
            left: left.max(0),
            top: top.max(0),
            right: right.max(0),
            bottom: bottom.max(0),
            hxmin: 0,
            hxmax: HASH_MASK,
            hymin: 0,
            hymax: HASH_MASK,
            hx: 0,
            hy: 0,
            current: ptr::null_mut(),
        };
        if distance.wrapping_mul(2) < HASH_MASK * TILE_SIZE {
            it.hxmin = hash_1d(it.left as u32 / TILE_SIZE);
            it.hxmax = hash_1d(it.right as u32 / TILE_SIZE);
            it.hymin = hash_1d(it.top as u32 / TILE_SIZE);
            it.hymax = hash_1d(it.bottom as u32 / TILE_SIZE);
        }
        it.hx = it.hxmin;
        it.hy = it.hymin;
        it.current = unsafe { *bucket(it.hx, it.hy) };
        unsafe {
            it.skip_empty();
            it.filter(s);
        }
        it
    }
    unsafe fn skip_empty(&mut self) {
        while self.current.is_null() {
            if self.hx != self.hxmax {
                self.hx = increment(self.hx);
            } else if self.hy != self.hymax {
                self.hx = self.hxmin;
                self.hy = increment(self.hy);
            } else {
                return;
            }
            self.current = unsafe { *bucket(self.hx, self.hy) };
        }
    }
    unsafe fn increment(&mut self, s: &VehicleServices) {
        if s.assertions {
            assert!(!self.current.is_null());
        }
        self.current = unsafe { next(s, self.current) };
        unsafe {
            self.skip_empty();
        }
    }
    unsafe fn filter(&mut self, s: &VehicleServices) {
        while let Some(v) = self.get() {
            let xy = (s.xy)(v);
            if xy.x.wrapping_sub(self.left) as u32 <= self.right.wrapping_sub(self.left) as u32
                && xy.y.wrapping_sub(self.top) as u32 <= self.bottom.wrapping_sub(self.top) as u32
            {
                break;
            }
            unsafe {
                self.increment(s);
            }
        }
    }
    pub(crate) unsafe fn advance(&mut self, s: &VehicleServices) {
        unsafe {
            self.increment(s);
            self.filter(s);
        }
    }
    pub(crate) fn get(&self) -> Option<VehicleRef> {
        std::ptr::NonNull::new(self.current).map(VehicleRef)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_near(
    s: &VehicleServices,
    x: i32,
    y: i32,
    distance: u32,
) -> Near {
    unsafe { Near::new(s, x, y, distance) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_near_advance(s: &VehicleServices, it: &mut Near) {
    unsafe {
        it.advance(s);
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDOccupancyCommands" ["probe"] => Commands { vehicles, shared, costs, vehicle_error, };
    "OpenTTDVehicleTileLinks" ["probe"] => Links { next, prev, current, };
    "OpenTTDVehicleOnTile" ["probe"] => OnTile { tile, current, };
    "OpenTTDVehicleNear" ["probe"] => Near { left, top, right, bottom, hxmin, hxmax, hymin, hymax, hx, hy, current, };
}

/// Native stack costs retain their complete representation. These services are
/// immutable; all traversal and command refusal policy belongs to this owner.
#[repr(C)]
pub struct Commands {
    pub vehicles: &'static VehicleServices,
    pub shared: &'static crate::services::Services,
    pub costs: &'static crate::services::CommandCostServices,
    pub vehicle_error: u32,
}
impl Commands {
    fn result(&self, out: *mut crate::services::CommandCost, vehicle: Option<u8>) {
        if let Some(kind) = vehicle {
            (self.costs.error)(out, self.vehicle_error.wrapping_add(u32::from(kind)));
        } else {
            (self.costs.zero)(out);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_ground(
    c: &Commands,
    out: *mut crate::services::CommandCost,
    tile: u32,
) {
    let s = c.vehicles;
    let z = (c.shared.tile_max_z)(tile);
    let mut it = unsafe { OnTile::new(s, tile) };
    while let Some(v) = it.get() {
        let kind = (s.vehicle_type)(v);
        if kind != super::VehicleType::DISASTER.0
            && !(kind == super::VehicleType::AIRCRAFT.0
                && (s.subtype)(v) == crate::aircraft::AIR_SHADOW)
            && (s.z)(v) <= z
        {
            c.result(out, Some(kind));
            return;
        }
        unsafe {
            it.advance(s);
        }
    }
    c.result(out, None);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_wormhole(
    c: &Commands,
    out: *mut crate::services::CommandCost,
    tile: u32,
    end: u32,
    ignore: *const OpenTTDVehicle,
) {
    let s = c.vehicles;
    for tile in [tile, end] {
        let mut it = unsafe { OnTile::new(s, tile) };
        while let Some(v) = it.get() {
            let kind = (s.vehicle_type)(v);
            if [
                super::VehicleType::TRAIN.0,
                super::VehicleType::ROAD.0,
                super::VehicleType::SHIP.0,
            ]
            .contains(&kind)
                && !ptr::eq(v.0.as_ptr(), ignore)
            {
                c.result(out, Some(kind));
                return;
            }
            unsafe {
                it.advance(s);
            }
        }
    }
    c.result(out, None);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_train(
    c: &Commands,
    out: *mut crate::services::CommandCost,
    tile: u32,
    bits: u8,
) {
    let s = c.vehicles;
    let mut it = unsafe { OnTile::new(s, tile) };
    while let Some(v) = it.get() {
        if (s.vehicle_type)(v) == super::VehicleType::TRAIN.0 {
            let track = (s.train_handle)(v.0.as_ptr()).track();
            if track == bits || crate::track_func::tracks_overlap(track | bits) {
                c.result(out, Some(super::VehicleType::TRAIN.0));
                return;
            }
        }
        unsafe {
            it.advance(s);
        }
    }
    c.result(out, None);
}

impl crate::services::CommandServices {
    pub(crate) unsafe fn no_vehicle(&self, out: *mut crate::services::CommandCost, tile: u32) {
        unsafe {
            openttd_rust_occupancy_ground(&*(self.occupancy)(), out, tile);
        }
    }
    pub(crate) unsafe fn no_train(
        &self,
        out: *mut crate::services::CommandCost,
        tile: u32,
        bits: u8,
    ) {
        unsafe {
            openttd_rust_occupancy_train(&*(self.occupancy)(), out, tile, bits);
        }
    }
    pub(crate) unsafe fn wormhole_free(
        &self,
        out: *mut crate::services::CommandCost,
        tile: u32,
        end: u32,
        ignore: *const crate::services::Train,
    ) {
        unsafe {
            openttd_rust_occupancy_wormhole(&*(self.occupancy)(), out, tile, end, ignore.cast());
        }
    }
}
