/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! NOSAVE vehicle tile index. Native shells own only its stable intrusive links.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use super::{OpenTTDVehicle, VehicleRef, VehicleServices};
use crate::aircraft::AIR_SHADOW;
use crate::game_state::GameState;
use crate::strings_type::StringID;
use crate::tile_type::TileIndex;
use crate::track_type::TrackBits;
use crate::vehicle_type::VehicleType;
use crate::{map_func::Map, tile_type::TILE_SIZE};
use std::ptr;

// Size of the hash, 6 = 64 x 64, 7 = 128 x 128. Larger sizes will (in theory) reduce hash
// lookup times at the expense of memory usage.
const TILE_HASH_BITS: u8 = 7;
// Resolution of the hash, 0 = 1*1 tile, 1 = 2*2 tiles, 2 = 4*4 tiles, etc.
// Profiling results show that 0 is fastest.
const TILE_HASH_RES: u8 = 0;
const TILE_HASH_SIZE: u32 = 1 << TILE_HASH_BITS;
const TILE_HASH_MASK: u32 = TILE_HASH_SIZE - 1;
const TOTAL_TILE_HASH_SIZE: usize = (TILE_HASH_SIZE * TILE_HASH_SIZE) as usize;
type Shell = *mut OpenTTDVehicle;
/// Field layout is shared, not the Vehicle layout. Native construction zeros it;
/// the Rust owner alone changes links. Shell addresses never move while indexed.
#[repr(C)]
pub struct Links {
    pub next: Shell,
    pub prev: *mut Shell,
    pub current: *mut Shell,
}
static VEHICLE_TILE_HASH: GameState<[Shell; TOTAL_TILE_HASH_SIZE]> =
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
fn get_tile_hash(x: u32, y: u32) -> *mut Shell {
    VEHICLE_TILE_HASH
        .get()
        .cast::<Shell>()
        .wrapping_add(compose_tile_hash(get_tile_hash_1d(x), get_tile_hash_1d(y)))
}
#[inline]
fn get_tile_hash_for_tile(map: Map, tile: TileIndex) -> *mut Shell {
    get_tile_hash(map.tile_x(tile), map.tile_y(tile))
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
#[inline]
pub(crate) unsafe fn update(s: &VehicleServices, v: VehicleRef, remove: bool) {
    if remove {
        unsafe { update_vehicle_tile_hash(s, v, (s.tile_links)(v), ptr::null_mut()) };
    } else {
        let map = Map::from_log_x((s.map_log_x)());
        unsafe { update_position(s, v, map) };
    }
}
/// The caller's dimensions cover only a serialized synchronous no-reset/no-yield
/// interval. No map storage is exposed. Links resolve once here; native link access neither reenters nor mutates state.
#[inline]
pub(crate) unsafe fn update_position(s: &VehicleServices, v: VehicleRef, map: Map) {
    let links = (s.tile_links)(v);
    unsafe {
        update_vehicle_tile_hash(
            s,
            v,
            links,
            get_tile_hash_for_tile(map, TileIndex(v.tile())),
        );
    };
}
#[inline]
unsafe fn update_vehicle_tile_hash(
    s: &VehicleServices,
    v: VehicleRef,
    links: *mut Links,
    new_hash: *mut Shell,
) {
    let old_hash = unsafe { (*links).current };
    if old_hash == new_hash {
        crate::witness::hit(crate::witness::OCCUPANCY_SAME);
        return;
    }
    unsafe {
        // Remove from the old position in the hash table
        if !old_hash.is_null() {
            crate::witness::hit(crate::witness::OCCUPANCY_REMOVE);
            let n = (*links).next;
            if !n.is_null() {
                (*((s.tile_links)(handle(n)))).prev = (*links).prev;
            }
            *(*links).prev = (*links).next;
        }
        // Insert vehicle at beginning of the new position in the hash table
        if !new_hash.is_null() {
            crate::witness::hit(crate::witness::OCCUPANCY_INSERT);
            (*links).next = *new_hash;
            if !(*links).next.is_null() {
                (*((s.tile_links)(handle((*links).next)))).prev = ptr::addr_of_mut!((*links).next);
            }
            (*links).prev = new_hash;
            *new_hash = v.0.as_ptr();
        }
        // Remember current hash position
        (*links).current = new_hash;
    }
}
/// Tile-index portion of `ResetVehicleHash`.
/// No pool allocation/deletion/yield occurs during reset. Retain stale links.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_reset(s: &super::scheduler::SchedulerServices) {
    crate::witness::hit(crate::witness::OCCUPANCY_RESET);
    let pool = (s.pool)();
    let vs = unsafe { &*s.vehicle };
    for i in 0..unsafe { *pool.first_unused } {
        let record = unsafe { (*pool.slots.add(i)).record };
        if !record.is_null() {
            unsafe {
                let v = VehicleRef::of(record.cast());
                (*((vs.tile_links)(v))).current = ptr::null_mut();
            }
        }
    }
    unsafe {
        ptr::write_bytes(
            VEHICLE_TILE_HASH.get().cast::<Shell>(),
            0,
            TOTAL_TILE_HASH_SIZE,
        );
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
/// Iterate over all vehicles on a tile; no reference into index storage is retained.
/// The order is non-deterministic. Processing must not depend on the order.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OnTile {
    pub tile: u32,
    pub current: Shell,
}
impl OnTile {
    /// Iterator constructor. Find first vehicle on tile.
    pub(crate) unsafe fn new(s: &VehicleServices, tile: u32) -> Self {
        let map = Map::from_log_x((s.map_log_x)());
        let mut it = Self {
            tile,
            current: unsafe { *get_tile_hash_for_tile(map, TileIndex(tile)) },
        };
        unsafe {
            it.skip_false_matches(s);
        }
        it
    }
    /// Advance until it reaches a vehicle on the correct tile or the end.
    unsafe fn skip_false_matches(&mut self, s: &VehicleServices) {
        while !self.current.is_null()
            && TileIndex(unsafe { handle(self.current) }.tile()) != TileIndex(self.tile)
        {
            crate::witness::hit(crate::witness::OCCUPANCY_ALIAS);
            self.current = unsafe { next(s, self.current) };
        }
    }
    /// Advance to the next potential vehicle (which may be on a different tile),
    /// then skip false matches, as in `VehiclesOnTile::Iterator::operator++`.
    pub(crate) unsafe fn advance(&mut self, s: &VehicleServices) {
        self.current = unsafe { next(s, self.current) };
        unsafe {
            self.skip_false_matches(s);
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
/// Iterate over all vehicles near a given world coordinate.
/// This only works for vehicles with proper `Vehicle::Tile`, so only ground vehicles outside wormholes.
/// The order is non-deterministic. Processing must not depend on the order.
/// Integer fields preserve native unsigned arithmetic and
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
    /// Iterator constructor. Find first vehicle near (x, y).
    #[inline]
    pub(crate) unsafe fn new(s: &VehicleServices, x: i32, y: i32, distance: u32) -> Self {
        crate::witness::hit(crate::witness::OCCUPANCY_NEAR);
        // There are no negative tile coordinates. Preserve unsigned arithmetic before clamping.
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
            // Scan all unless the distance below selects a smaller hash area.
            hxmax: TILE_HASH_MASK,
            hymin: 0,
            hymax: TILE_HASH_MASK,
            hx: 0,
            hy: 0,
            current: ptr::null_mut(),
        };
        if distance.wrapping_mul(2) < TILE_HASH_MASK * TILE_SIZE {
            // Hash area to scan
            it.hxmin = get_tile_hash_1d(it.left as u32 / TILE_SIZE);
            it.hxmax = get_tile_hash_1d(it.right as u32 / TILE_SIZE);
            it.hymin = get_tile_hash_1d(it.top as u32 / TILE_SIZE);
            it.hymax = get_tile_hash_1d(it.bottom as u32 / TILE_SIZE);
        }
        it.hx = it.hxmin;
        it.hy = it.hymin;
        it.current = unsafe { *get_tile_hash(it.hx, it.hy) };
        unsafe {
            it.skip_empty_buckets();
            it.skip_false_matches(s);
        }
        it
    }
    /// Advance until we reach a non-empty bucket, or the end.
    #[inline]
    unsafe fn skip_empty_buckets(&mut self) {
        while self.current.is_null() {
            if self.hx != self.hxmax {
                self.hx = inc_tile_hash_1d(self.hx);
            } else if self.hy != self.hymax {
                self.hx = self.hxmin;
                self.hy = inc_tile_hash_1d(self.hy);
            } else {
                return;
            }
            self.current = unsafe { *get_tile_hash(self.hx, self.hy) };
        }
    }
    /// Advance the internal state to the next potential vehicle.
    #[inline]
    unsafe fn increment(&mut self, links: *mut Links) {
        // skip_false_matches proves a non-null current; public advancement asserts before
        // resolving links, as the original Increment does before dereferencing.
        self.current = unsafe { (*links).next };
        unsafe {
            self.skip_empty_buckets();
        }
    }
    /// Advance until it reaches a vehicle within the search area.
    #[inline]
    unsafe fn skip_false_matches(&mut self, s: &VehicleServices) {
        while let Some(v) = self.get() {
            let links = (s.tile_links)(v);
            if crate::geometry_type::rect_contains(
                self.left,
                self.top,
                self.right,
                self.bottom,
                v.x(),
                v.y(),
            ) {
                break;
            }
            unsafe {
                self.increment(links);
            }
        }
    }
    /// `VehiclesNearTileXY::Iterator::operator++`: increment, then skip false matches.
    #[inline]
    pub(crate) unsafe fn advance(&mut self, s: &VehicleServices) {
        // Resolve afresh AFTER the visitor; never reuse a skip_false_matches observation
        // across callbacks that can change the current shell's next link.
        if s.assertions {
            assert!(!self.current.is_null());
        }
        let links = (s.tile_links)(unsafe { handle(self.current) });
        unsafe {
            self.increment(links);
            self.skip_false_matches(s);
        }
    }
    #[inline]
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
    "OpenTTDOccupancyCommands" ["probe"] => Commands { vehicles, max_pixel_z, costs, vehicle_error, };
    "OpenTTDVehicleTileLinks" ["probe"] => Links { next, prev, current, };
    "OpenTTDVehicleOnTile" ["probe"] => OnTile { tile, current, };
    "OpenTTDVehicleNear" ["probe"] => Near { left, top, right, bottom, hxmin, hxmax, hymin, hymax, hx, hy, current, };
}

/// Native stack costs retain their complete representation. These services are
/// immutable; all traversal and command refusal policy belongs to this owner.
#[repr(C)]
pub struct Commands {
    pub vehicles: &'static VehicleServices,
    pub max_pixel_z: extern "C" fn(u32) -> i32,
    pub costs: &'static crate::services::CommandCostServices,
    pub vehicle_error: u32,
}
impl Commands {
    fn result(&self, out: *mut crate::services::CommandCost, vehicle: Option<u8>) {
        if let Some(vehicle_type) = vehicle.map(VehicleType) {
            let error = StringID(self.vehicle_error.wrapping_add(u32::from(vehicle_type.0)));
            (self.costs.error)(out, error.0);
        } else {
            (self.costs.zero)(out);
        }
    }
}
/// `EnsureNoVehicleOnGround`: ensure there is no vehicle at the ground at the given position.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_ground(
    c: &Commands,
    out: *mut crate::services::CommandCost,
    tile: u32,
) {
    let s = c.vehicles;
    let tile = TileIndex(tile);
    let z = (c.max_pixel_z)(tile.0);
    let mut it = unsafe { OnTile::new(s, tile.0) };
    // Value v is not safe in MP games, however, it is used to generate a local
    // error message only (which may be different for different machines).
    // Such a message does not affect MP synchronisation.
    while let Some(v) = it.get() {
        let vehicle_type = VehicleType(v.vehicle_type());
        if vehicle_type != VehicleType::DISASTER
            && !(vehicle_type == VehicleType::AIRCRAFT && v.subtype() == AIR_SHADOW)
            && v.z() <= z
        {
            c.result(out, Some(vehicle_type.0));
            return;
        }
        unsafe {
            it.advance(s);
        }
    }
    c.result(out, None);
}
/// `TunnelBridgeIsFree`: find a vehicle in a tunnel / bridge, ignoring the given vehicle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_wormhole(
    c: &Commands,
    out: *mut crate::services::CommandCost,
    tile: u32,
    end: u32,
    ignore: *const OpenTTDVehicle,
) {
    c.result(out, unsafe {
        wormhole_blocker(c.vehicles, tile, end, ignore)
    });
}
pub(crate) unsafe fn wormhole_blocker(
    s: &VehicleServices,
    tile: u32,
    end: u32,
    ignore: *const OpenTTDVehicle,
) -> Option<u8> {
    for t in [TileIndex(tile), TileIndex(end)] {
        let mut it = unsafe { OnTile::new(s, t.0) };
        // Value v is not safe in MP games, however, it is used to generate a local
        // error message only (which may be different for different machines).
        // Such a message does not affect MP synchronisation.
        while let Some(v) = it.get() {
            let vehicle_type = VehicleType(v.vehicle_type());
            if [VehicleType::TRAIN, VehicleType::ROAD, VehicleType::SHIP].contains(&vehicle_type)
                && !ptr::eq(v.0.as_ptr(), ignore)
            {
                return Some(vehicle_type.0);
            }
            unsafe {
                it.advance(s);
            }
        }
    }
    None
}
/// `EnsureNoTrainOnTrackBits`: test if a vehicle interacts with the specified track bits.
/// All track bits interact except parallel `TRACK_BIT_HORZ` or `TRACK_BIT_VERT`.
/// Returns the type of the vehicle in the way, whose `STR_ERROR_TRAIN_IN_THE_WAY + type`
/// error fails the command, or `None` when no train interacts.
pub(crate) fn ensure_no_train_on_track_bits(
    s: &VehicleServices,
    tile: TileIndex,
    track_bits: TrackBits,
) -> Option<VehicleType> {
    // Value v is not safe in MP games, however, it is used to generate a local
    // error message only (which may be different for different machines).
    // Such a message does not affect MP synchronisation.
    // SAFETY: The native iterator's current-shell contract; no owner borrow spans the walk.
    unsafe {
        let mut it = OnTile::new(s, tile.0);
        while let Some(v) = it.get() {
            if VehicleType(v.vehicle_type()) == VehicleType::TRAIN {
                let track = TrackBits((s.train_handle)(v.0.as_ptr()).track());
                if track == track_bits || crate::track_func::tracks_overlap(track.0 | track_bits.0)
                {
                    return Some(VehicleType(v.vehicle_type()));
                }
            }
            it.advance(s);
        }
    }
    None
}
/// Export shim of [`ensure_no_train_on_track_bits`] for the native facade.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_occupancy_train(
    c: &Commands,
    out: *mut crate::services::CommandCost,
    tile: u32,
    bits: u8,
) {
    let blocker = ensure_no_train_on_track_bits(c.vehicles, TileIndex(tile), TrackBits(bits));
    c.result(out, blocker.map(|vehicle_type| vehicle_type.0));
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

/// The occupancy index: the ground check against the tile's highest pixel, head insertion
/// links, removal that keeps the stale successor without reading the map size, and a reset
/// that clears only current positions. No other test sets `tile_links`, so none shares the
/// process-wide tile index.
#[cfg(test)]
mod index_tests {
    use super::*;
    use crate::test_tables::{COMMAND_COSTS, SCHEDULER, VEHICLE};
    use crate::vehicle_base::VehicleRecord;
    use std::cell::Cell;

    /// Tile (1, 1) of the fixture's 64x64 map, with its highest corner at pixel height 8.
    const TILE: u32 = 1 + 64;
    const MAX_PIXEL_Z: i32 = 8;
    const VEHICLE_ERROR: u32 = 100;

    thread_local! {
        /// The two vehicles' native `tile_links` members, by record.
        static LINKS: Cell<[(*mut VehicleRecord, *mut Links); 2]> =
            const { Cell::new([(ptr::null_mut(), ptr::null_mut()); 2]) };
        static RESULT: Cell<Option<u32>> = const { Cell::new(None) };
        static SLOTS: Cell<*const crate::pool::PoolSlot> = const { Cell::new(ptr::null()) };
        static FIRST_UNUSED: Cell<usize> = const { Cell::new(0) };
    }
    extern "C" fn tile_links(v: VehicleRef) -> *mut Links {
        LINKS
            .get()
            .iter()
            .find(|(record, _)| *record == v.0.as_ptr())
            .unwrap()
            .1
    }
    extern "C" fn map_log_x() -> u32 {
        6
    }
    extern "C" fn no_map_read() -> u32 {
        panic!("removal resolves no map dimensions")
    }
    extern "C" fn max_pixel_z(tile: u32) -> i32 {
        assert_eq!(tile, TILE);
        MAX_PIXEL_Z
    }
    extern "C" fn error(_: *mut crate::services::CommandCost, message: u32) {
        RESULT.set(Some(message));
    }
    extern "C" fn zero(_: *mut crate::services::CommandCost) {
        RESULT.set(None);
    }
    extern "C" fn pool() -> crate::vehicle::scheduler::PoolCursor {
        crate::vehicle::scheduler::PoolCursor {
            slots: SLOTS.get(),
            slots_len: 2,
            first_unused: FIRST_UNUSED.with(Cell::as_ptr),
        }
    }

    fn road_vehicle(z: i32) -> *mut VehicleRecord {
        let record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            ptr::null_mut(),
            VehicleType::ROAD,
        );
        // SAFETY: The fresh record.
        unsafe {
            (*record).tile = TileIndex(TILE);
            (*record).z_pos = z;
        }
        record
    }
    fn set_z(records: [*mut VehicleRecord; 2], z: i32) {
        for record in records {
            // SAFETY: The test's live records.
            unsafe { (*record).z_pos = z };
        }
    }
    /// `EnsureNoVehicleOnGround(tile)`: the error, or `None` when the ground is free.
    fn ground(commands: &Commands) -> Option<u32> {
        // SAFETY: The test's live index, records and tables.
        unsafe { openttd_rust_occupancy_ground(commands, ptr::null_mut(), TILE) };
        RESULT.get()
    }

    #[test]
    fn index_links_removal_and_reset() {
        let mut links = [
            Links {
                next: ptr::null_mut(),
                prev: ptr::null_mut(),
                current: ptr::null_mut(),
            },
            Links {
                next: ptr::null_mut(),
                prev: ptr::null_mut(),
                current: ptr::null_mut(),
            },
        ];
        let (a, b) = (road_vehicle(MAX_PIXEL_Z), road_vehicle(MAX_PIXEL_Z));
        let (la, lb): (*mut Links, *mut Links) = (&raw mut links[0], &raw mut links[1]);
        LINKS.set([(a, la), (b, lb)]);
        let mut vehicles = VEHICLE;
        vehicles.tile_links = tile_links;
        vehicles.map_log_x = map_log_x;
        let vehicles: &'static VehicleServices = Box::leak(Box::new(vehicles));
        let mut costs = COMMAND_COSTS;
        costs.error = error;
        costs.zero = zero;
        let commands = Commands {
            vehicles,
            max_pixel_z,
            costs: Box::leak(Box::new(costs)),
            vehicle_error: VEHICLE_ERROR,
        };
        let (va, vb) = (VehicleRef::of(a), VehicleRef::of(b));
        // SAFETY: The test's live records, links and services throughout.
        unsafe {
            update(vehicles, va, false);
            update(vehicles, vb, false);
            // A vehicle at the tile's highest pixel blocks the ground; one above it does not.
            assert_eq!(
                ground(&commands),
                Some(VEHICLE_ERROR + u32::from(VehicleType::ROAD.0))
            );
            set_z([a, b], MAX_PIXEL_Z + 1);
            assert_eq!(ground(&commands), None);
            // The later insertion heads the bucket.
            assert_eq!((*lb).next, a);
            assert_eq!((*la).prev, &raw mut (*lb).next);
            // Removal reads no map size and leaves the source's stale successor.
            let mut removal = VEHICLE;
            removal.tile_links = tile_links;
            removal.map_log_x = no_map_read;
            update(&removal, vb, true);
            assert!((*lb).current.is_null());
            assert_eq!((*lb).next, a);
            // Reset clears current positions only, keeping stale links.
            let slots = [
                crate::pool::PoolSlot {
                    shell: ptr::null_mut(),
                    record: a.cast(),
                },
                crate::pool::PoolSlot {
                    shell: ptr::null_mut(),
                    record: b.cast(),
                },
            ];
            SLOTS.set(slots.as_ptr());
            FIRST_UNUSED.set(2);
            let mut scheduler = SCHEDULER;
            scheduler.pool = pool;
            scheduler.vehicle = vehicles;
            openttd_rust_occupancy_reset(&scheduler);
            assert!((*la).current.is_null());
            assert_eq!((*lb).next, a);
            // Reinsertion makes the later vehicle the tile's first again.
            update(vehicles, va, false);
            update(vehicles, vb, false);
            assert_eq!(
                OnTile::new(vehicles, TILE).get().map(|v| v.0.as_ptr()),
                Some(b)
            );
            update(vehicles, vb, true);
            update(vehicles, va, true);
            for record in [a, b] {
                crate::vehicle_base::openttd_rust_vehicle_record_delete(record);
            }
        }
    }
}
