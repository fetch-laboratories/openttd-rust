/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! The vehicle tile hash of `vehicle.cpp`: `_vehicle_tile_hash` with `UpdateVehicleTileHash`
//! and the tile part of `ResetVehicleHash`, the iterators `VehiclesOnTile` and
//! `VehiclesNearTileXY`, and `EnsureNoVehicleOnGround`, `TunnelBridgeIsFree` and
//! `EnsureNoTrainOnTrackBits`.
//!
//! The hash links are the record members `hash_tile_next`, `hash_tile_prev` and
//! `hash_tile_current`. A record's address is stable while it is in the hash: the record is
//! boxed, `Vehicle::~Vehicle` removes the vehicle from the hash, and the record is released
//! after the whole base destructor. Every record pointer in the hash is live and serialized
//! under the game-state mutex for each access here.
//!
//! An iterator's visitor may change links but not delete the current vehicle, so an iterator
//! reads the current vehicle's `hash_tile_next` after the visitor returns and never carries it
//! across a visitor.
use crate::aircraft::AircraftSubType;
use crate::game_state::GameState;
use crate::map_func::{Map, World};
use crate::services::{CommandCost, CommandServices, Cost};
use crate::table::strings::STR_ERROR_TRAIN_IN_THE_WAY;
use crate::tile_type::{TILE_SIZE, TileIndex};
use crate::track_type::TrackBits;
use crate::vehicle::VehicleRef;
use crate::vehicle_base::VehicleRecord;
use crate::vehicle_type::VehicleType;
use std::ptr;

// Size of the hash, 6 = 64 x 64, 7 = 128 x 128. Larger sizes will (in theory) reduce hash
// lookup times at the expense of memory usage.
const TILE_HASH_BITS: u8 = 7;
// Resolution of the hash, 0 = 1*1 tile, 1 = 2*2 tiles, 2 = 4*4 tiles, etc.
// Profiling results show that 0 is fastest.
const TILE_HASH_RES: u8 = 0;
const TILE_HASH_SIZE: u32 = 1 << TILE_HASH_BITS;
const TILE_HASH_MASK: u32 = TILE_HASH_SIZE - 1;
const TOTAL_TILE_HASH_SIZE: usize = 1 << (TILE_HASH_BITS * 2);

static VEHICLE_TILE_HASH: GameState<[*mut VehicleRecord; TOTAL_TILE_HASH_SIZE]> =
    GameState::new([ptr::null_mut(); TOTAL_TILE_HASH_SIZE]);

/// Compute hash for 1D tile coordinate.
fn get_tile_hash_1d(p: u32) -> u32 {
    crate::bitmath_func::gb(p, TILE_HASH_RES, TILE_HASH_BITS)
}
/// Increment 1D hash to next bucket.
fn inc_tile_hash_1d(h: u32) -> u32 {
    h.wrapping_add(1) & TILE_HASH_MASK
}
/// Compose two 1D hashes into 2D hash.
fn compose_tile_hash(hx: u32, hy: u32) -> usize {
    (hx | (hy << TILE_HASH_BITS)) as usize
}
/// Compute hash for tile coordinate.
fn get_tile_hash(x: u32, y: u32) -> usize {
    compose_tile_hash(get_tile_hash_1d(x), get_tile_hash_1d(y))
}
/// `&_vehicle_tile_hash[hash]`.
fn tile_hash_bucket(hash: usize) -> *mut *mut VehicleRecord {
    VEHICLE_TILE_HASH
        .get()
        .cast::<*mut VehicleRecord>()
        .wrapping_add(hash)
}
/// `_vehicle_tile_hash[hash]`.
#[allow(unsafe_code)]
fn tile_hash_head(hash: usize) -> *mut VehicleRecord {
    // SAFETY: `hash` is below `TOTAL_TILE_HASH_SIZE`; the module's serialized access.
    unsafe { tile_hash_bucket(hash).read() }
}

/// `Vehicle::UpdatePosition`: update the position of the vehicle. This will update the hash
/// that tells which vehicles are on a tile. `map` gives the map dimensions the tile is hashed
/// with.
#[inline]
pub(crate) fn update_position(v: VehicleRef, map: Map) {
    let tile = v.tile();
    update_vehicle_tile_hash(
        v,
        tile_hash_bucket(get_tile_hash(map.tile_x(tile), map.tile_y(tile))),
    );
}
/// `UpdateVehicleTileHash` from `old_hash == new_hash` on, with the `new_hash` its `remove`
/// selects: null to remove the vehicle, or its tile's bucket.
#[allow(unsafe_code)]
#[inline]
fn update_vehicle_tile_hash(v: VehicleRef, new_hash: *mut *mut VehicleRecord) {
    let old_hash = v.hash_tile_current();
    if old_hash == new_hash {
        crate::witness::hit(crate::witness::OCCUPANCY_SAME);
        return;
    }

    // Remove from the old position in the hash table
    if !old_hash.is_null() {
        crate::witness::hit(crate::witness::OCCUPANCY_REMOVE);
        if let Some(next) = ptr::NonNull::new(v.hash_tile_next()) {
            VehicleRef(next).set_hash_tile_prev(v.hash_tile_prev());
        }
        // SAFETY: An indexed vehicle's `hash_tile_prev` addresses its bucket or its
        // predecessor's `hash_tile_next`.
        unsafe { v.hash_tile_prev().write(v.hash_tile_next()) };
    }

    // Insert vehicle at beginning of the new position in the hash table
    if !new_hash.is_null() {
        crate::witness::hit(crate::witness::OCCUPANCY_INSERT);
        // SAFETY: `new_hash` is a bucket of the hash.
        v.set_hash_tile_next(unsafe { new_hash.read() });
        if let Some(next) = ptr::NonNull::new(v.hash_tile_next()) {
            VehicleRef(next).set_hash_tile_prev(v.hash_tile_next_address());
        }
        v.set_hash_tile_prev(new_hash);
        // SAFETY: As above.
        unsafe { new_hash.write(v.0.as_ptr()) };
    }

    // Remember current hash position
    v.set_hash_tile_current(new_hash);
}
/// `UpdateVehicleTileHash(v, remove)` for `Vehicle::~Vehicle` and `Vehicle::UpdatePosition`.
/// Only an insertion reads `shared`, for the map dimensions.
///
/// # Safety
/// `v` is a live record; an insertion's `shared` is the process-lifetime shared table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_update(
    shared: *const crate::services::Services,
    v: VehicleRef,
    remove: bool,
) {
    if remove {
        update_vehicle_tile_hash(v, ptr::null_mut());
    } else {
        // SAFETY: The caller's process-lifetime shared table.
        update_position(v, unsafe { (*shared).map() });
    }
}

/// The tile part of `ResetVehicleHash`: clear every vehicle's `hash_tile_current` and empty
/// the hash, keeping each vehicle's stale `hash_tile_next` and `hash_tile_prev`.
///
/// # Safety
/// `pool`'s descriptor is live, and nothing allocates, deletes or yields during the reset.
#[allow(unsafe_code)]
pub(crate) unsafe fn reset_vehicle_tile_hash(pool: &crate::vehicle_base::Pool) {
    crate::witness::hit(crate::witness::OCCUPANCY_RESET);
    // `for (Vehicle *v : Vehicle::Iterate())`.
    let mut index = 0;
    // SAFETY: The caller's live descriptor.
    while let Some((id, _, v)) = unsafe { crate::vehicle_base::next_vehicle(pool, index) } {
        index = crate::pool::PoolId::index(id) + 1;
        v.set_hash_tile_current(ptr::null_mut());
    }
    // SAFETY: The hash's whole extent; the module's serialized access.
    unsafe { ptr::write_bytes(tile_hash_bucket(0), 0, TOTAL_TILE_HASH_SIZE) };
}
/// [`reset_vehicle_tile_hash`] for `ResetVehicleHash`.
///
/// # Safety
/// As [`reset_vehicle_tile_hash`], for the scheduler table's pool.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_reset(s: &super::scheduler::SchedulerServices) {
    // SAFETY: The caller's contract.
    unsafe { reset_vehicle_tile_hash(&s.pool()) };
}

/// `VehiclesOnTile::Iterator`: iterate over all vehicles on a tile.
/// The order is non-deterministic. You have to make sure, that your processing is not order
/// dependant.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OnTile {
    tile: TileIndex,
    current: *mut VehicleRecord,
}
impl OnTile {
    /// Iterator constructor. Find first vehicle on tile.
    pub(crate) fn new(map: Map, tile: TileIndex) -> Self {
        let mut it = Self {
            tile,
            current: tile_hash_head(get_tile_hash(map.tile_x(tile), map.tile_y(tile))),
        };
        it.skip_false_matches();
        it
    }
    /// Advance the internal state to the next potential vehicle.
    /// The vehicle may not be on the correct tile though.
    fn increment(&mut self) {
        self.current = VehicleRef::of(self.current).hash_tile_next();
    }
    /// Advance the internal state until it reaches a vehicle on the correct tile or the end.
    fn skip_false_matches(&mut self) {
        while !self.current.is_null() && VehicleRef::of(self.current).tile() != self.tile {
            crate::witness::hit(crate::witness::OCCUPANCY_ALIAS);
            self.increment();
        }
    }
    /// `operator++`: increment, then skip false matches, for this module's checks, whose
    /// bodies delete no vehicle.
    fn step(&mut self) {
        self.increment();
        self.skip_false_matches();
    }
    /// `operator++`: increment, then skip false matches.
    ///
    /// # Safety
    /// The current vehicle is live: the visitor deleted no vehicle since the cursor reached it.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn advance(&mut self) {
        self.step();
    }
    /// `operator*`: the current vehicle, or `None` at the end.
    pub(crate) fn get(&self) -> Option<VehicleRef> {
        ptr::NonNull::new(self.current).map(VehicleRef)
    }
}
/// `VehiclesOnTile::Iterator(tile)` for the native iterator class.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_occupancy_on_tile(
    shared: &crate::services::Services,
    tile: u32,
) -> OnTile {
    OnTile::new(shared.map(), TileIndex(tile))
}
/// `VehiclesOnTile::Iterator::operator++` for the native iterator class.
///
/// # Safety
/// As [`OnTile::advance`].
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_on_advance(it: &mut OnTile) {
    // SAFETY: The caller's contract.
    unsafe { it.advance() };
}

/// `VehiclesNearTileXY::Iterator`: iterate over all vehicles near a given world coordinate.
/// This only works for vehicles with proper `Vehicle::Tile`, so only ground vehicles outside
/// wormholes. The order is non-deterministic. You have to make sure, that your processing is
/// not order dependant. `left` to `bottom` are its `pos_rect`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Near {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    hxmin: u32,
    hxmax: u32,
    hymin: u32,
    hymax: u32,
    hx: u32,
    hy: u32,
    current_veh: *mut VehicleRecord,
}
impl Near {
    /// Iterator constructor. Find first vehicle near (x, y).
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "`x - max_dist` and `y + max_dist` are unsigned and std::max<int> converts them to int; `pos_rect.left / TILE_SIZE` divides as unsigned"
    )]
    #[inline]
    pub(crate) fn new(x: i32, y: i32, max_dist: u32) -> Self {
        crate::witness::hit(crate::witness::OCCUPANCY_NEAR);
        // There are no negative tile coordinates
        let left = ((x as u32).wrapping_sub(max_dist) as i32).max(0);
        let right = ((x as u32).wrapping_add(max_dist) as i32).max(0);
        let top = ((y as u32).wrapping_sub(max_dist) as i32).max(0);
        let bottom = ((y as u32).wrapping_add(max_dist) as i32).max(0);

        let mut it = Self {
            left,
            top,
            right,
            bottom,
            hxmin: 0,
            hxmax: 0,
            hymin: 0,
            hymax: 0,
            hx: 0,
            hy: 0,
            current_veh: ptr::null_mut(),
        };
        if max_dist.wrapping_mul(2) < TILE_HASH_MASK * TILE_SIZE {
            // Hash area to scan
            it.hxmin = get_tile_hash_1d(it.left as u32 / TILE_SIZE);
            it.hx = it.hxmin;
            it.hxmax = get_tile_hash_1d(it.right as u32 / TILE_SIZE);
            it.hymin = get_tile_hash_1d(it.top as u32 / TILE_SIZE);
            it.hy = it.hymin;
            it.hymax = get_tile_hash_1d(it.bottom as u32 / TILE_SIZE);
        } else {
            // Scan all
            it.hxmin = 0;
            it.hx = 0;
            it.hxmax = TILE_HASH_MASK;
            it.hymin = 0;
            it.hy = 0;
            it.hymax = TILE_HASH_MASK;
        }

        it.current_veh = tile_hash_head(compose_tile_hash(it.hx, it.hy));
        it.skip_empty_buckets();
        it.skip_false_matches();
        it
    }
    /// Advance the internal state to the next potential vehicle.
    #[inline]
    fn increment(&mut self) {
        crate::services_assert::assert_with(|| !self.current_veh.is_null());
        self.current_veh = VehicleRef::of(self.current_veh).hash_tile_next();
        self.skip_empty_buckets();
    }
    /// Advance the internal state until we reach a non-empty bucket, or the end.
    #[inline]
    fn skip_empty_buckets(&mut self) {
        while self.current_veh.is_null() {
            if self.hx != self.hxmax {
                self.hx = inc_tile_hash_1d(self.hx);
            } else if self.hy != self.hymax {
                self.hx = self.hxmin;
                self.hy = inc_tile_hash_1d(self.hy);
            } else {
                return;
            }
            self.current_veh = tile_hash_head(compose_tile_hash(self.hx, self.hy));
        }
    }
    /// Advance the internal state until it reaches a vehicle within the search area.
    #[inline]
    fn skip_false_matches(&mut self) {
        while let Some(v) = self.get()
            && !crate::geometry_type::rect_contains(
                self.left,
                self.top,
                self.right,
                self.bottom,
                v.x_pos(),
                v.y_pos(),
            )
        {
            self.increment();
        }
    }
    /// `operator++`: increment, then skip false matches.
    ///
    /// # Safety
    /// The current vehicle is live: the visitor deleted no vehicle since the cursor reached it.
    #[allow(unsafe_code)]
    #[inline]
    pub(crate) unsafe fn advance(&mut self) {
        self.increment();
        self.skip_false_matches();
    }
    /// `operator*`: the current vehicle, or `None` at the end.
    #[inline]
    pub(crate) fn get(&self) -> Option<VehicleRef> {
        ptr::NonNull::new(self.current_veh).map(VehicleRef)
    }
}
/// `VehiclesNearTileXY::Iterator(x, y, max_dist)` for the native iterator class.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_occupancy_near(x: i32, y: i32, max_dist: u32) -> Near {
    Near::new(x, y, max_dist)
}
/// `VehiclesNearTileXY::Iterator::operator++` for the native iterator class.
///
/// # Safety
/// As [`Near::advance`].
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_near_advance(it: &mut Near) {
    // SAFETY: The caller's contract.
    unsafe { it.advance() };
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleOnTile" ["probe"] => OnTile { tile, current, };
    "OpenTTDVehicleNear" ["probe"] => Near { left, top, right, bottom, hxmin, hxmax, hymin, hymax, hx, hy, current_veh, };
}

/// `EnsureNoVehicleOnGround`: ensure there is no vehicle at the ground at the given position.
/// `None` is the original's succeeded command (ground is free), and `Some(type)` its failed
/// command `STR_ERROR_TRAIN_IN_THE_WAY + type` (a vehicle is found).
pub(crate) fn ensure_no_vehicle_on_ground(world: &World, tile: TileIndex) -> Option<VehicleType> {
    let z = world.tile_max_pixel_z(tile);

    // Value v is not safe in MP games, however, it is used to generate a local
    // error message only (which may be different for different machines).
    // Such a message does not affect MP synchronisation.
    let mut it = OnTile::new(world.dimensions, tile);
    while let Some(v) = it.get() {
        if v.vehicle_type() == VehicleType::VEH_DISASTER
            || (v.vehicle_type() == VehicleType::VEH_AIRCRAFT
                && AircraftSubType(v.subtype()) == AircraftSubType::AIR_SHADOW)
        {
            it.step();
            continue;
        }
        if v.z_pos() > z {
            it.step();
            continue;
        }

        return Some(v.vehicle_type());
    }
    None
}
/// `TunnelBridgeIsFree`: finds vehicle in tunnel / bridge. `ignore` is the vehicle to ignore
/// when searching. `None` is the original's succeeded command (the tunnel/bridge is free), and
/// `Some(type)` its failed command `STR_ERROR_TRAIN_IN_THE_WAY + type` (a vehicle is using
/// the tunnel/bridge).
pub(crate) fn tunnel_bridge_is_free(
    map: Map,
    tile: TileIndex,
    endtile: TileIndex,
    ignore: Option<VehicleRef>,
) -> Option<VehicleType> {
    for t in [tile, endtile] {
        // Value v is not safe in MP games, however, it is used to generate a local
        // error message only (which may be different for different machines).
        // Such a message does not affect MP synchronisation.
        let mut it = OnTile::new(map, t);
        while let Some(v) = it.get() {
            if v.vehicle_type() != VehicleType::VEH_TRAIN
                && v.vehicle_type() != VehicleType::VEH_ROAD
                && v.vehicle_type() != VehicleType::VEH_SHIP
            {
                it.step();
                continue;
            }
            if Some(v) == ignore {
                it.step();
                continue;
            }
            return Some(v.vehicle_type());
        }
    }
    None
}
/// `EnsureNoTrainOnTrackBits`: tests if a vehicle interacts with the specified track bits.
/// All track bits interact except parallel `TRACK_BIT_HORZ` or `TRACK_BIT_VERT`.
/// `None` is the original's succeeded command (no train that interacts is found), and
/// `Some(type)` its failed command `STR_ERROR_TRAIN_IN_THE_WAY + type` (a train is found).
pub(crate) fn ensure_no_train_on_track_bits(
    map: Map,
    tile: TileIndex,
    track_bits: TrackBits,
) -> Option<VehicleType> {
    // Value v is not safe in MP games, however, it is used to generate a local
    // error message only (which may be different for different machines).
    // Such a message does not affect MP synchronisation.
    let mut it = OnTile::new(map, tile);
    while let Some(v) = it.get() {
        if v.vehicle_type() != VehicleType::VEH_TRAIN {
            it.step();
            continue;
        }

        let t = crate::train::Handle::from_vehicle(v);
        if t.track() != track_bits && !crate::track_func::tracks_overlap(t.track() | track_bits) {
            it.step();
            continue;
        }

        return Some(v.vehicle_type());
    }
    None
}
/// The command result of a check: `CommandCost()` for `None`, and
/// `CommandCost(STR_ERROR_TRAIN_IN_THE_WAY + type)` for `Some(type)`.
fn command_cost(cost: Cost<'_>, blocker: Option<VehicleType>) {
    match blocker {
        Some(vehicle_type) => cost.error(STR_ERROR_TRAIN_IN_THE_WAY.0 + u32::from(vehicle_type.0)),
        None => cost.zero(),
    }
}
/// `EnsureNoVehicleOnGround(tile)` for the native function, into `out`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_occupancy_ground(
    command: &CommandServices,
    out: *mut CommandCost,
    tile: u32,
) {
    command.no_vehicle(out, TileIndex(tile));
}
/// `TunnelBridgeIsFree(tile, endtile, ignore)` for the native function, into `out`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_occupancy_wormhole(
    command: &CommandServices,
    out: *mut CommandCost,
    tile: u32,
    endtile: u32,
    ignore: Option<VehicleRef>,
) {
    command.wormhole_free(out, TileIndex(tile), TileIndex(endtile), ignore);
}

/// The command layer's checks, each writing its check's result into the command's cost.
impl CommandServices {
    /// `EnsureNoVehicleOnGround(tile)`.
    pub(crate) fn no_vehicle(&self, out: *mut CommandCost, tile: TileIndex) {
        command_cost(
            Cost(out, self.cost),
            ensure_no_vehicle_on_ground(&World::new(self.shared), tile),
        );
    }
    /// [`Self::no_vehicle`] for callers in files no pass has finished.
    pub(crate) fn no_vehicle_raw(&self, out: *mut CommandCost, tile: u32) {
        self.no_vehicle(out, TileIndex(tile));
    }
    /// `EnsureNoTrainOnTrackBits(tile, track_bits)`.
    pub(crate) fn no_train(&self, out: *mut CommandCost, tile: TileIndex, track_bits: TrackBits) {
        command_cost(
            Cost(out, self.cost),
            ensure_no_train_on_track_bits(self.shared.map(), tile, track_bits),
        );
    }
    /// `TunnelBridgeIsFree(tile, endtile, ignore)`.
    pub(crate) fn wormhole_free(
        &self,
        out: *mut CommandCost,
        tile: TileIndex,
        endtile: TileIndex,
        ignore: Option<VehicleRef>,
    ) {
        command_cost(
            Cost(out, self.cost),
            tunnel_bridge_is_free(self.shared.map(), tile, endtile, ignore),
        );
    }
    /// [`Self::wormhole_free`] with no ignored vehicle, for callers in files no pass has
    /// finished.
    pub(crate) fn wormhole_free_raw(&self, out: *mut CommandCost, tile: u32, endtile: u32) {
        self.wormhole_free(out, TileIndex(tile), TileIndex(endtile), None);
    }
}

/// The hash's operations and the three checks over real records on a fixture map. Every test
/// that indexes a record holds `town::tests::GLOBAL_LOCK`, which the train swap oracle also
/// holds, from its setup to the end of its cleanup, since the hash is process-wide.
#[cfg(test)]
#[allow(unsafe_code)]
mod index_tests {
    use super::*;
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation};
    use crate::services::Services;
    use std::cell::Cell;
    use std::sync::{MutexGuard, PoisonError};

    /// Tile (1, 1) of the fixture's 64x64 map. Its four corners are at height 1, so its
    /// highest pixel is 8.
    const TILE: TileIndex = TileIndex(1 + 64);
    /// Tile (3, 1), at height 0.
    const ENDTILE: TileIndex = TileIndex(3 + 64);
    const MAX_PIXEL_Z: i32 = 8;

    /// The hash held for one test, and the records the test made.
    struct Fixture {
        shared: &'static Services,
        records: Vec<VehicleRef>,
        _hash: MutexGuard<'static, ()>,
    }
    impl Fixture {
        /// The fixture's map, installed for this thread, with the hash locked.
        fn new() -> Self {
            const SIZE: u32 = 64 * 64;
            let hash = crate::town::tests::GLOBAL_LOCK
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let base: &'static mut [BaseTileLayout] = Box::leak(
                (0..SIZE)
                    .map(|tile| BaseTileLayout {
                        kind: 0,
                        height: u8::from(
                            [TILE.0, TILE.0 + 1, TILE.0 + 64, TILE.0 + 65].contains(&tile),
                        ),
                        m2: 0,
                        m1: 0,
                        m3: 0,
                        m4: 0,
                        m5: 0,
                    })
                    .collect(),
            );
            let extended: &'static mut [ExtendedTileLayout] = Box::leak(
                (0..SIZE)
                    .map(|_| ExtendedTileLayout {
                        m6: 0,
                        m7: 0,
                        m8: 0,
                    })
                    .collect(),
            );
            crate::map_func::fixture::set(
                TileTypeAllocation {
                    base: base.as_ptr(),
                    extended: extended.as_ptr(),
                    count: SIZE,
                    assertions: true,
                },
                6,
            );
            let mut shared = crate::test_tables::SHARED;
            shared.map = crate::map_func::fixture::statics();
            Self {
                shared: Box::leak(Box::new(shared)),
                records: Vec::new(),
                _hash: hash,
            }
        }
        /// A fresh record of `kind` on `tile` at height `z`, outside the hash.
        fn vehicle(&mut self, kind: VehicleType, tile: TileIndex, z: i32) -> VehicleRef {
            let v = VehicleRef::of(VehicleRecord::new(ptr::null_mut(), kind));
            v.set_tile(tile);
            v.set_z_pos(z);
            self.records.push(v);
            v
        }
        /// `v->UpdatePosition()`.
        fn insert(&self, v: VehicleRef) {
            update_position(v, self.shared.map());
        }
        fn ground(&self) -> Option<VehicleType> {
            ensure_no_vehicle_on_ground(&World::new(self.shared), TILE)
        }
    }
    impl Drop for Fixture {
        /// Remove every record from the hash, then free them.
        fn drop(&mut self) {
            for &v in &self.records {
                update_vehicle_tile_hash(v, ptr::null_mut());
            }
            for &v in &self.records {
                // SAFETY: The fixture's records, out of the hash, each freed once.
                unsafe { VehicleRecord::delete(v.0.as_ptr()) };
            }
        }
    }

    /// `VehiclesNearTileXY::Iterator::operator++` on an exhausted cursor fails the native
    /// `assert(this->current_veh != nullptr)` in `Increment` before it reads the vehicle's
    /// links; the isolated child ends at that assertion.
    #[test]
    fn near_advance_on_an_exhausted_cursor_fails_the_native_assertion() {
        if crate::services_assert::isolate_test_exit(
            "vehicle::occupancy::index_tests::near_advance_on_an_exhausted_cursor_fails_the_native_assertion",
        ) {
            return;
        }
        let mut it = Near {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
            hxmin: 0,
            hxmax: 0,
            hymin: 0,
            hymax: 0,
            hx: 0,
            hy: 0,
            current_veh: ptr::null_mut(),
        };
        // SAFETY: The failed assertion ends the child before any record is read.
        unsafe { it.advance() };
        unreachable!("the failed native assertion ends the child");
    }

    /// Head insertion links, removal that keeps the stale successor without reading the map
    /// size, and a reset that clears only current positions.
    #[test]
    fn index_links_removal_and_reset() {
        let mut f = Fixture::new();
        let (a, b) = (
            f.vehicle(VehicleType::VEH_ROAD, TILE, MAX_PIXEL_Z),
            f.vehicle(VehicleType::VEH_ROAD, TILE, MAX_PIXEL_Z),
        );
        // A fresh record is in no chain.
        assert!(a.hash_tile_next().is_null());
        assert!(a.hash_tile_prev().is_null());
        assert!(a.hash_tile_current().is_null());
        // SAFETY: The fixture's shared table and live records throughout.
        unsafe {
            openttd_rust_occupancy_update(f.shared, a, false);
            openttd_rust_occupancy_update(f.shared, b, false);
        }
        // A vehicle at the tile's highest pixel blocks the ground; one above it does not.
        assert_eq!(f.ground(), Some(VehicleType::VEH_ROAD));
        a.set_z_pos(MAX_PIXEL_Z + 1);
        b.set_z_pos(MAX_PIXEL_Z + 1);
        assert_eq!(f.ground(), None);
        // The later insertion heads the bucket.
        assert_eq!(b.hash_tile_next(), a.0.as_ptr());
        assert_eq!(a.hash_tile_prev(), b.hash_tile_next_address());
        // Removal reads no map size (it has no shared table) and leaves the original's stale
        // successor.
        // SAFETY: As above.
        unsafe { openttd_rust_occupancy_update(ptr::null(), b, true) };
        assert!(b.hash_tile_current().is_null());
        assert_eq!(b.hash_tile_next(), a.0.as_ptr());
        // Reset clears current positions only, keeping stale links. Each slot holds a shell
        // identity and its record, as a pooled vehicle's does.
        let shells = [0_u8; 2];
        let slot = |shell: &u8, v: VehicleRef| crate::pool::PoolSlot {
            shell: (&raw const *shell).cast_mut().cast(),
            record: v.0.as_ptr().cast(),
        };
        let descriptor = crate::pool::PoolDescriptor {
            slots: crate::industrytype::NativeVec::from_vec(vec![
                slot(&shells[0], a),
                slot(&shells[1], b),
            ]),
            used_bitmap: crate::industrytype::NativeVec::new(),
            first_free: 2,
            first_unused: 2,
            items: 2,
            checked: 0,
            cleaning: false,
            alloc_cache: ptr::null_mut(),
        };
        let descriptor_address = &raw const descriptor;
        let mut scheduler = crate::test_tables::SCHEDULER;
        scheduler.pool = &raw const descriptor_address;
        // SAFETY: The test's live descriptor, read through the scheduler table.
        unsafe { openttd_rust_occupancy_reset(&scheduler) };
        assert!(a.hash_tile_current().is_null());
        assert_eq!(b.hash_tile_next(), a.0.as_ptr());
        // Reinsertion makes the later vehicle the tile's first again.
        f.insert(a);
        f.insert(b);
        assert_eq!(OnTile::new(f.shared.map(), TILE).get(), Some(b));
    }

    /// `EnsureNoVehicleOnGround`'s two `continue` tests: a disaster vehicle and an aircraft
    /// shadow never block, and any other vehicle blocks with its own type only at or below the
    /// tile's highest pixel.
    #[test]
    fn ground_check_skips_disaster_vehicles_shadows_and_vehicles_above_the_ground() {
        let mut f = Fixture::new();
        let disaster = f.vehicle(VehicleType::VEH_DISASTER, TILE, 0);
        let shadow = f.vehicle(VehicleType::VEH_AIRCRAFT, TILE, 0);
        shadow.set_subtype(AircraftSubType::AIR_SHADOW.0);
        f.insert(disaster);
        f.insert(shadow);
        assert_eq!(f.ground(), None);

        let aircraft = f.vehicle(VehicleType::VEH_AIRCRAFT, TILE, MAX_PIXEL_Z + 1);
        aircraft.set_subtype(AircraftSubType::AIR_AIRCRAFT.0);
        f.insert(aircraft);
        assert_eq!(f.ground(), None);
        aircraft.set_z_pos(MAX_PIXEL_Z);
        assert_eq!(f.ground(), Some(VehicleType::VEH_AIRCRAFT));
        update_vehicle_tile_hash(aircraft, ptr::null_mut());

        let road = f.vehicle(VehicleType::VEH_ROAD, TILE, MAX_PIXEL_Z + 1);
        f.insert(road);
        assert_eq!(f.ground(), None);
        road.set_z_pos(MAX_PIXEL_Z);
        assert_eq!(f.ground(), Some(VehicleType::VEH_ROAD));
    }

    /// `TunnelBridgeIsFree`: trains, road vehicles and ships block and other types do not;
    /// the ignored vehicle does not block, but one behind it does; `endtile` is searched
    /// after `tile`.
    #[test]
    fn tunnel_bridge_check_finds_ground_vehicles_on_either_end_except_the_ignored_one() {
        let mut f = Fixture::new();
        let free =
            |f: &Fixture, ignore| tunnel_bridge_is_free(f.shared.map(), TILE, ENDTILE, ignore);
        let aircraft = f.vehicle(VehicleType::VEH_AIRCRAFT, TILE, 0);
        let disaster = f.vehicle(VehicleType::VEH_DISASTER, ENDTILE, 0);
        f.insert(aircraft);
        f.insert(disaster);
        assert_eq!(free(&f, None), None);

        for kind in [
            VehicleType::VEH_TRAIN,
            VehicleType::VEH_ROAD,
            VehicleType::VEH_SHIP,
        ] {
            let v = f.vehicle(kind, ENDTILE, 0);
            f.insert(v);
            assert_eq!(free(&f, None), Some(kind));
            assert_eq!(free(&f, Some(v)), None);
            update_vehicle_tile_hash(v, ptr::null_mut());
        }

        // A ship heads `endtile`'s bucket, behind the ignored train.
        let ship = f.vehicle(VehicleType::VEH_SHIP, ENDTILE, 0);
        let train = f.vehicle(VehicleType::VEH_TRAIN, ENDTILE, 0);
        f.insert(ship);
        f.insert(train);
        assert_eq!(free(&f, Some(train)), Some(VehicleType::VEH_SHIP));
        // With a vehicle on each end, the result is `tile`'s.
        let road = f.vehicle(VehicleType::VEH_ROAD, TILE, 0);
        f.insert(road);
        assert_eq!(free(&f, None), Some(VehicleType::VEH_ROAD));
    }

    thread_local! {
        /// The last string `CommandCost` received, or `None` after `CommandCost()`.
        static RESULT: Cell<Option<u32>> = const { Cell::new(None) };
    }
    extern "C" fn error(_: *mut CommandCost, message: u32) {
        RESULT.set(Some(message));
    }
    extern "C" fn zero(_: *mut CommandCost) {
        RESULT.set(None);
    }

    /// A check's result reaches the command's cost as `STR_ERROR_TRAIN_IN_THE_WAY` plus the
    /// vehicle type, or as an empty cost.
    #[test]
    fn command_check_writes_the_vehicle_type_after_the_train_error() {
        let mut f = Fixture::new();
        let mut costs = crate::test_tables::COMMAND_COSTS;
        costs.error = error;
        costs.zero = zero;
        let mut command = crate::test_tables::COMMANDS;
        command.shared = f.shared;
        command.cost = Box::leak(Box::new(costs));
        RESULT.set(Some(0));
        command.no_vehicle(ptr::null_mut(), TILE);
        assert_eq!(RESULT.get(), None);
        let ship = f.vehicle(VehicleType::VEH_SHIP, TILE, 0);
        f.insert(ship);
        command.no_vehicle(ptr::null_mut(), TILE);
        assert_eq!(RESULT.get(), Some(STR_ERROR_TRAIN_IN_THE_WAY.0 + 2));
    }
}
