/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Cargo packets and cargo lists (`cargopacket.h`, `cargopacket.cpp`), the cargo actions that
//! move packets between lists (`cargoaction.cpp`), and the cargo transfers of autoreplace
//! (`autoreplace_cmd.cpp`). The packet pool and its shells stay native: a list's node holds the
//! shell C++ allocated, and the `packet` slot resolves the [`CargoPacket`] record the shell
//! owns.
//!
//! The contract every unsafe item here relies on: a list or packet pointer is a live
//! allocation; a packet's record address is stable until `destroy`; access is serialised by the
//! game-state mutex; and no reference to a list or a packet is alive across `create`,
//! `destroy`, a payment or an operation on another list. A mover's source and destination may
//! be the same list, so movers and actions take raw pointers and never hold `&mut` to two lists
//! at once. Save import stores shell handles without dereferencing them. Map reads use the
//! shared game-loop and command interval: these coordinate operations cannot reset or yield.
use crate::cargo_flow::FlowStatMap;
use crate::cargo_payment::NativePayment;
use crate::cargo_type::CargoType;
use crate::cargopacket::MoveToAction;
use crate::order_type::OrderUnloadType;
use crate::services_assert::{assert_with, not_reached};
use crate::source_type::Source;
use crate::station_base::GoodsEntry;
use crate::station_type::StationID;
use crate::tile_type::TileIndex;
use crate::vehicle::VehicleRef;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;

crate::abi::shared_records! {
    fn abi_record;
    /// `CargoPacket`'s fields, the record its native shell owns and native readers share in
    /// place.
    #[derive(Clone, Copy)]
    "OpenTTDCargoPacket" ["cargo_storage"] => pub struct CargoPacket {
        pub feeder_share: i64,
        pub source_xy: u32,
        pub count: u16,
        pub periods_in_transit: u16,
        pub first_station: u16,
        pub next_hop: u16,
        pub source_id: u16,
        pub travelled_x: i16,
        pub travelled_y: i16,
        pub source_type: u8,
        pub in_vehicle: u8,
    }
}
impl Default for CargoPacket {
    /// `CargoPacket()`: the member initialisers of `cargopacket.h`.
    fn default() -> Self {
        let source = Source {
            id: Source::Invalid,
            kind: crate::source_type::SourceType::Industry.0,
        };
        Self {
            feeder_share: 0,
            source_xy: crate::tile_type::INVALID_TILE,
            count: 0,
            periods_in_transit: 0,
            first_station: StationID::INVALID.0,
            next_hop: StationID::INVALID.0,
            source_id: source.id,
            travelled_x: 0,
            travelled_y: 0,
            source_type: source.kind,
            in_vehicle: 0,
        }
    }
}
/// The cached fields of a cargo list, as the save code reads and writes them.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fields {
    pub(crate) cargo_periods_in_transit: u64,
    pub(crate) feeder_share: i64,
    pub(crate) count: u32,
    pub(crate) reserved_count: u32,
    pub(crate) action_counts: [u32; 4],
}
/// The native packet pool, the payment and the flows a cargo list reaches.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Services {
    /// `CargoPacket::CanAllocateItem()`.
    pub(crate) can_allocate: extern "C" fn() -> u8,
    /// `GetCargoDeliveryState()`, the process-wide delivery state final payments fill.
    pub(crate) cargo_delivery_state: extern "C" fn() -> *mut crate::cargo_payment::Delivery,
    /// `new CargoPacket(fields)`: a new shell owning a copy of the record.
    pub(crate) create: extern "C" fn(*const CargoPacket) -> *mut c_void,
    /// `delete cp`.
    pub(crate) destroy: extern "C" fn(*mut c_void),
    /// `ge->HasData() ? &ge->GetData().flows : nullptr`.
    pub(crate) flow_map: extern "C" fn(*const GoodsEntry) -> *mut FlowStatMap,
    pub(crate) monitors: *const crate::cargomonitor::Services,
    /// `CargoPacket::state`: the record the shell owns.
    pub(crate) packet: extern "C" fn(*mut c_void) -> *mut CargoPacket,
    /// The next shell of `CargoPacket::Iterate()` after `shell`, from the first for null.
    pub(crate) packet_next: extern "C" fn(*mut c_void) -> *mut c_void,
    pub(crate) payment: *const crate::cargo_payment::Services,
    /// `CargoPayment::rust_state`.
    pub(crate) payment_state:
        extern "C" fn(*mut NativePayment) -> *mut crate::cargo_payment::Payment,
    pub(crate) shared: *const crate::services::Services,
}
impl Services {
    /// `RandomRange(limit)`.
    fn random_range(&self, limit: u32) -> u32 {
        self.shared().random_range(limit)
    }
    /// The shared services.
    #[allow(unsafe_code, reason = "reads the process-lifetime shared table")]
    pub(crate) fn shared(&self) -> &crate::services::Services {
        // SAFETY: The table names the immutable process-lifetime shared services.
        unsafe { &*self.shared }
    }
}
/// `StationCargoAmountMap`: an amount of cargo by origin station.
pub(crate) type StationCargoAmountMap = BTreeMap<StationID, u32>;

/// `std::list` and `MultiMap` node: a packet shell between its neighbours.
#[derive(Clone, Copy)]
struct Node {
    token: usize,
    /// The `MultiMap` key, a next hop; vehicle lists leave it unused.
    key: StationID,
    shell: *mut c_void,
    previous: Option<usize>,
    next: Option<usize>,
}
/// The first and last node of one `MultiMap` key.
#[derive(Clone, Copy)]
struct Ends {
    first: usize,
    last: usize,
}
/// `CargoList`: the packets of a vehicle or station, with their cached counts. The allocation
/// is the one the native list shell holds.
pub struct CargoList {
    pub(crate) fields: Fields,
    // Slots never move relative to their indices. Reuse happens only after erase,
    // when no source iterator can still refer to that node. Traversal follows
    // links, never arena allocation order; same-list insertion keeps existing
    // iterators valid even if growing the arena reallocates its backing storage.
    nodes: Vec<Option<Node>>,
    free: Vec<usize>,
    first: Option<usize>,
    last: Option<usize>,
    destinations: BTreeMap<StationID, Ends>,
    /// The `MultiMap`'s keys, including those whose packet list a save left empty.
    keys: BTreeSet<StationID>,
    /// Whether the native shell's `Tinst` is `VehicleCargoList`.
    vehicle: bool,
}
/// `VehicleCargoList`: a vehicle's packets, staged for transfer, delivery, keeping and
/// loading in that order.
#[repr(transparent)]
pub struct VehicleCargoList(CargoList);
/// `StationCargoList`: a station's packets by their next hop.
#[repr(transparent)]
pub struct StationCargoList(CargoList);

/// The typed list behind a native list handle.
enum Typed {
    Vehicle(*mut VehicleCargoList),
    Station(*mut StationCargoList),
}
impl CargoList {
    fn new(vehicle: bool) -> Self {
        Self {
            fields: Fields::default(),
            nodes: Vec::new(),
            free: Vec::new(),
            first: None,
            last: None,
            destinations: BTreeMap::new(),
            keys: BTreeSet::new(),
            vehicle,
        }
    }
    /// The list `l` as its native `Tinst`.
    #[allow(unsafe_code, reason = "reads the kind of the caller's live list")]
    unsafe fn typed(l: *mut Self) -> Typed {
        // SAFETY: The caller's live list; one field-sized read.
        if unsafe { (*l).vehicle } {
            Typed::Vehicle(l.cast())
        } else {
            Typed::Station(l.cast())
        }
    }
    fn node(&self, token: usize) -> Node {
        self.nodes[token].unwrap()
    }
    fn front(&self) -> Option<Node> {
        self.first.map(|token| self.node(token))
    }
    fn back(&self) -> Option<Node> {
        self.last.map(|token| self.node(token))
    }
    fn after(&self, token: usize) -> Option<Node> {
        self.node(token).next.map(|token| self.node(token))
    }
    /// Links a new node for `shell` between `previous` and `next`.
    fn link(
        &mut self,
        shell: *mut c_void,
        key: StationID,
        previous: Option<usize>,
        next: Option<usize>,
    ) -> usize {
        let token = self.free.pop().unwrap_or_else(|| {
            self.nodes.push(None);
            self.nodes.len() - 1
        });
        self.nodes[token] = Some(Node {
            token,
            key,
            shell,
            previous,
            next,
        });
        if let Some(previous) = previous {
            self.nodes[previous].as_mut().unwrap().next = Some(token);
        } else {
            self.first = Some(token);
        }
        if let Some(next) = next {
            self.nodes[next].as_mut().unwrap().previous = Some(token);
        } else {
            self.last = Some(token);
        }
        token
    }
    /// Unlinks the node at `token` and frees its slot.
    fn unlink(&mut self, token: usize) -> Node {
        let removed = self.nodes[token].take().unwrap();
        if let Some(previous) = removed.previous {
            self.nodes[previous].as_mut().unwrap().next = removed.next;
        } else {
            self.first = removed.next;
        }
        if let Some(next) = removed.next {
            self.nodes[next].as_mut().unwrap().previous = removed.previous;
        } else {
            self.last = removed.previous;
        }
        self.free.push(token);
        removed
    }
    /// `packets.clear()`.
    fn clear(&mut self) {
        self.nodes.clear();
        self.free.clear();
        self.first = None;
        self.last = None;
        self.destinations.clear();
        self.keys.clear();
    }
    /// `CargoList::~CargoList`: destroy the cargolist ("frees" all cargo packets).
    #[allow(
        unsafe_code,
        reason = "frees the caller's list and deletes its packets"
    )]
    unsafe fn destroy(l: *mut Self, s: &Services) {
        // SAFETY: The caller's live list, which no other reference covers; each read of a
        // node ends before the deletion of its packet.
        let mut it = unsafe { (*l).front() };
        while let Some(current) = it {
            it = unsafe { (*l).after(current.token) };
            (s.destroy)(current.shell);
        }
        // SAFETY: The list came from `openttd_rust_cargo_list_new` and is freed once.
        drop(unsafe { Box::from_raw(l) });
    }
    /// `CargoList::OnCleanPool`: empty the cargo list, but don't free the cargo packets; the
    /// cargo packets are cleaned by `CargoPacket`'s `CleanPool`.
    pub(crate) fn on_clean_pool(&mut self) {
        self.clear();
    }
    /// `CargoList::RemoveFromCache`: update the cached values to reflect the removal of this
    /// packet or part of it. Decreases count and `periods_in_transit`.
    fn remove_from_cache(&mut self, cp: &CargoPacket, count: u32) {
        assert_with(|| count <= u32::from(cp.count));
        self.fields.count = self.fields.count.wrapping_sub(count);
        self.fields.cargo_periods_in_transit = self
            .fields
            .cargo_periods_in_transit
            .wrapping_sub(u64::from(cp.periods_in_transit).wrapping_mul(u64::from(count)));
    }
    /// `CargoList::AddToCache`: update the cache to reflect adding of this packet. Increases
    /// count and `periods_in_transit`.
    fn add_to_cache(&mut self, cp: &CargoPacket) {
        self.fields.count = self.fields.count.wrapping_add(u32::from(cp.count));
        self.fields.cargo_periods_in_transit = self
            .fields
            .cargo_periods_in_transit
            .wrapping_add(u64::from(cp.periods_in_transit) * u64::from(cp.count));
    }
    /// `CargoList::TryMerge`: tries to merge the second packet into the first and return if
    /// that was successful. `cp` is the shell of the packet to be eliminated and `cp_state` its
    /// record; `are_mergable` is `Tinst::AreMergable`.
    #[allow(unsafe_code, reason = "merges and deletes the caller's live packets")]
    unsafe fn try_merge(
        s: &Services,
        icp: *mut CargoPacket,
        cp: *mut c_void,
        cp_state: *const CargoPacket,
        are_mergable: fn(&CargoPacket, &CargoPacket) -> bool,
    ) -> bool {
        // SAFETY: Two live, distinct packet records; the references end before the merge.
        let mergeable = unsafe {
            are_mergable(&*icp, &*cp_state)
                && u32::from((*icp).count) + u32::from((*cp_state).count)
                    <= u32::from(CargoPacket::MAX_COUNT)
        };
        if mergeable {
            // SAFETY: As above; the merge deletes `cp` last.
            unsafe { CargoPacket::merge(icp, s, cp, cp_state) };
            true
        } else {
            false
        }
    }
    /// `CargoList::PeriodsInTransit`: returns average number of cargo aging periods in
    /// transit for a cargo entity.
    pub(crate) fn periods_in_transit(&self) -> u32 {
        if self.fields.count == 0 {
            0
        } else {
            (self.fields.cargo_periods_in_transit / u64::from(self.fields.count)) as u32
        }
    }
    /// The first packet's `first_station`, or `StationID::Invalid()` for an empty list.
    #[allow(
        unsafe_code,
        reason = "reads the first live packet of the caller's list"
    )]
    unsafe fn front_first_station(&self, s: &Services) -> StationID {
        if self.fields.count == 0 {
            StationID::INVALID
        } else {
            // SAFETY: A list with cargo has a live front packet.
            StationID(unsafe { (*(s.packet)(self.front().unwrap().shell)).first_station })
        }
    }
}

impl CargoPacket {
    /// The record of the native shell `cp`.
    fn of(s: &Services, cp: *mut c_void) -> *mut Self {
        (s.packet)(cp)
    }
    /// `CargoPacket::Split`: split this packet in two and return the split off part, or null
    /// if no packet could be allocated!
    #[allow(unsafe_code, reason = "copies and writes the caller's live packet")]
    pub(crate) unsafe fn split(this: *mut Self, s: &Services, new_size: u32) -> *mut c_void {
        if (s.can_allocate)() == 0 {
            return std::ptr::null_mut();
        }
        // SAFETY: The caller's live packet; the reference ends before `create`.
        let (fs, mut cp_new) = unsafe { ((*this).get_feeder_share_part(new_size), *this) };
        // The split constructor `CargoPacket(new_size, fs, *this)`.
        cp_new.count = new_size as u16;
        cp_new.feeder_share = fs;
        let cp_new = (s.create)(&raw const cp_new);
        // SAFETY: As above; `create` made a new record and left this one in place.
        unsafe {
            (*this).feeder_share = (*this).feeder_share.saturating_sub(fs);
            (*this).count = (*this).count.wrapping_sub(new_size as u16);
        }
        cp_new
    }
    /// `CargoPacket::Merge`: merge another packet into this one. `cp` is its shell and
    /// `cp_state` its record.
    #[allow(
        unsafe_code,
        reason = "writes the caller's live packet and deletes the other"
    )]
    unsafe fn merge(this: *mut Self, s: &Services, cp: *mut c_void, cp_state: *const Self) {
        // SAFETY: Two live, distinct packet records; the writes end before the deletion.
        unsafe {
            (*this).count = (*this).count.wrapping_add((*cp_state).count);
            (*this).feeder_share = (*this)
                .feeder_share
                .saturating_add((*cp_state).feeder_share);
        }
        (s.destroy)(cp);
    }
    /// `CargoPacket::Reduce`: reduce the packet by the given amount and remove the feeder
    /// share.
    pub(crate) fn reduce(&mut self, count: u32) {
        assert_with(|| count < u32::from(self.count));
        self.feeder_share = self
            .feeder_share
            .saturating_sub(self.get_feeder_share_part(count));
        self.count = self.count.wrapping_sub(count as u16);
    }
    /// `CargoPacket::SetNextHop`: sets the station where the packet is supposed to go next.
    pub(crate) fn set_next_hop(&mut self, next_hop: StationID) {
        self.next_hop = next_hop.0;
    }
    /// `CargoPacket::UpdateLoadingTile`: update for the cargo being loaded on this tile.
    ///
    /// When a `CargoPacket` is created, it is moved to a station. But at that moment in time it
    /// is not known yet at which tile the cargo will be picked up. As this tile is used for
    /// payment information, we delay setting the `source_xy` till first pickup, getting a
    /// better idea where a cargo started from.
    ///
    /// Further more, we keep track of the amount of tiles the cargo moved inside a vehicle. This
    /// is used in `GetDistance()` below.
    pub(crate) fn update_loading_tile(
        &mut self,
        shared: &crate::services::Services,
        tile: TileIndex,
    ) {
        if self.source_xy == crate::tile_type::INVALID_TILE {
            self.source_xy = tile.0;
        }
        assert_with(|| self.in_vehicle == 0);
        self.in_vehicle = 1;
        let world = crate::map_func::World::new(shared);
        // We want to calculate the vector from tile-unload to tile-load. As we currently only
        // know the tile-load, add it. When we know the tile-unload, we subtract it.
        self.travelled_x = self
            .travelled_x
            .wrapping_add(world.dimensions.tile_x(tile) as i16);
        self.travelled_y = self
            .travelled_y
            .wrapping_add(world.dimensions.tile_y(tile) as i16);
    }
    /// `CargoPacket::UpdateUnloadingTile`: update for the cargo being unloaded on this tile.
    pub(crate) fn update_unloading_tile(
        &mut self,
        shared: &crate::services::Services,
        tile: TileIndex,
    ) {
        assert_with(|| self.in_vehicle != 0);
        self.in_vehicle = 0;
        let world = crate::map_func::World::new(shared);
        self.travelled_x = self
            .travelled_x
            .wrapping_sub(world.dimensions.tile_x(tile) as i16);
        self.travelled_y = self
            .travelled_y
            .wrapping_sub(world.dimensions.tile_y(tile) as i16);
    }
    /// `CargoPacket::AddFeederShare`: adds some feeder share to the packet.
    pub(crate) fn add_feeder_share(&mut self, new_share: i64) {
        self.feeder_share = self.feeder_share.saturating_add(new_share);
    }
    /// `CargoPacket::Count`: gets the number of 'items' in this packet.
    pub(crate) fn count(&self) -> u16 {
        self.count
    }
    /// `CargoPacket::GetFeederShare(part)`: gets part of the amount of money already paid to
    /// earlier vehicles in the feeder chain.
    pub(crate) fn get_feeder_share_part(&self, part: u32) -> i64 {
        self.feeder_share.saturating_mul(i64::from(part as i32)) / i64::from(i32::from(self.count))
    }
    /// `CargoPacket::GetPeriodsInTransit`: gets the number of cargo aging periods this cargo
    /// has been in transit.
    pub(crate) fn get_periods_in_transit(&self) -> u16 {
        self.periods_in_transit
    }
    /// `CargoPacket::GetSource`: gets the source of the packet for subsidy purposes.
    pub(crate) fn get_source(&self) -> Source {
        Source {
            id: self.source_id,
            kind: self.source_type,
        }
    }
    /// `CargoPacket::GetFirstStation`: gets the ID of the station where the cargo was loaded
    /// for the first time.
    pub(crate) fn get_first_station(&self) -> StationID {
        StationID(self.first_station)
    }
    /// `CargoPacket::GetNextHop`: gets the ID of station the cargo wants to go next.
    pub(crate) fn get_next_hop(&self) -> StationID {
        StationID(self.next_hop)
    }
    /// `CargoPacket::GetDistance`: get the current distance the cargo has travelled.
    pub(crate) fn get_distance(
        &self,
        shared: &crate::services::Services,
        current_tile: TileIndex,
    ) -> u32 {
        assert_with(|| self.source_xy != crate::tile_type::INVALID_TILE);
        assert_with(|| self.in_vehicle != 0);
        let world = crate::map_func::World::new(shared);
        // Distance is always requested when the cargo is still inside the vehicle. So first
        // finish the calculation for travelled to become a vector.
        let local_x = self
            .travelled_x
            .wrapping_sub(world.dimensions.tile_x(current_tile) as i16);
        let local_y = self
            .travelled_y
            .wrapping_sub(world.dimensions.tile_y(current_tile) as i16);
        // Cargo-movement is a vector that indicates how much the cargo has actually traveled in
        // a vehicle. This is the distance you get paid for. However, one could construct a
        // route where this vector would be really long. To not overpay the player, cap out at
        // the distance between source and destination.
        //
        // This way of calculating is to counter people moving cargo for free and instantly in
        // stations, where you deliver it in one part of a station and pick it up in another. By
        // using the actual distance traveled in a vehicle, using this trick doesn't give you
        // more money.
        //
        // However, especially in large networks with large transfer station, etc, one could
        // actually make the route a lot longer. In that case, use the actual distance between
        // source and destination.
        let distance_travelled =
            i32::from(local_x).unsigned_abs() + i32::from(local_y).unsigned_abs();
        let distance_source_dest = world
            .dimensions
            .distance_manhattan(TileIndex(self.source_xy), current_tile);
        distance_travelled.min(distance_source_dest)
    }
    /// `CargoPacket::InvalidateAllFrom(Source)`: invalidates (sets `source_id` to
    /// `INVALID_SOURCE`) all cargo packets from given source.
    #[allow(unsafe_code, reason = "writes each live packet of the native pool")]
    pub(crate) unsafe fn invalidate_all_from_source(s: &Services, src: Source) {
        let mut shell = (s.packet_next)(std::ptr::null_mut());
        while !shell.is_null() {
            let cp = Self::of(s, shell);
            // SAFETY: Each pool shell owns a live record; the reference ends before the next
            // call.
            let cp = unsafe { &mut *cp };
            if cp.get_source() == src {
                let mut source = cp.get_source();
                source.make_invalid();
                cp.source_id = source.id;
            }
            shell = (s.packet_next)(shell);
        }
    }
    /// `CargoPacket::InvalidateAllFrom(StationID)`: invalidates (sets source to
    /// `StationID::Invalid()`) all cargo packets from given station.
    #[allow(unsafe_code, reason = "writes each live packet of the native pool")]
    pub(crate) unsafe fn invalidate_all_from_station(s: &Services, sid: StationID) {
        let mut shell = (s.packet_next)(std::ptr::null_mut());
        while !shell.is_null() {
            let cp = Self::of(s, shell);
            // SAFETY: As in `invalidate_all_from_source`.
            let cp = unsafe { &mut *cp };
            if cp.get_first_station() == sid {
                cp.first_station = StationID::INVALID.0;
            }
            shell = (s.packet_next)(shell);
        }
    }
}

/// A cargo action applied to one packet at a time: `MaxMove()` and `operator()`.
trait CargoAction {
    /// `MaxMove`: returns how much more cargo can be moved with this action.
    fn max_move(&self) -> u32;
    /// `operator()`: apply the action to the packet `cp`; true if the packet left its list.
    ///
    /// # Safety
    /// The module's contract; `cp` is a live packet of the action's source list.
    #[allow(unsafe_code, reason = "each action moves the caller's live packet")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool;
}
/// `CargoRemoval`: abstract action of removing cargo from a vehicle or a station.
struct CargoRemoval<'a, Tsource> {
    s: &'a Services,
    /// Source of the cargo.
    source: *mut Tsource,
    /// Maximum amount of cargo to be removed with this action.
    max_move: u32,
}
impl<Tsource> CargoRemoval<'_, Tsource> {
    /// `CargoRemoval::Preprocess`: determines the amount of cargo to be removed from a packet
    /// and removes that from the metadata of the list.
    fn preprocess(&mut self, cp: &CargoPacket) -> u32 {
        if self.max_move >= u32::from(cp.count()) {
            self.max_move -= u32::from(cp.count());
            u32::from(cp.count())
        } else {
            let ret = self.max_move;
            self.max_move = 0;
            ret
        }
    }
    /// `CargoRemoval::Postprocess`: finalize cargo removal. Either delete the packet or reduce
    /// it. Returns true if the packet was deleted, false if it was reduced.
    #[allow(unsafe_code, reason = "reduces or deletes the caller's live packet")]
    unsafe fn postprocess(&self, cp: *mut c_void, cp_state: *mut CargoPacket, remove: u32) -> bool {
        // SAFETY: The caller's live packet; the reference ends before the deletion.
        if remove == u32::from(unsafe { (*cp_state).count() }) {
            (self.s.destroy)(cp);
            true
        } else {
            // SAFETY: As above.
            unsafe { (*cp_state).reduce(remove) };
            false
        }
    }
}
impl CargoAction for CargoRemoval<'_, VehicleCargoList> {
    fn max_move(&self) -> u32 {
        self.max_move
    }
    /// `CargoRemoval<VehicleCargoList>::operator()`: removes some cargo from a
    /// `VehicleCargoList`. Returns true if the packet was completely delivered, false if only
    /// part of it was.
    #[allow(unsafe_code, reason = "removes the caller's live packet from its list")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let cp_state = CargoPacket::of(self.s, cp);
        // SAFETY: The module's contract; each reference ends before the next call.
        let remove = self.preprocess(unsafe { &*cp_state });
        unsafe {
            (*self.source).remove_from_meta(&*cp_state, MoveToAction::MTA_KEEP, remove);
            self.postprocess(cp, cp_state, remove)
        }
    }
}
/// `CargoDelivery`: action of final delivery of cargo.
struct CargoDelivery<'a> {
    removal: CargoRemoval<'a, VehicleCargoList>,
    /// Current tile cargo delivery is happening.
    current_tile: TileIndex,
    /// Payment object where payments will be registered.
    payment: *mut NativePayment,
    /// The cargo type of the cargo.
    cargo: CargoType,
}
impl CargoAction for CargoDelivery<'_> {
    fn max_move(&self) -> u32 {
        self.removal.max_move
    }
    /// `CargoDelivery::operator()`: delivers some cargo. Returns true if the packet was
    /// completely delivered, false if only part of it was.
    #[allow(
        unsafe_code,
        reason = "delivers the caller's live packet from its list"
    )]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let s = self.removal.s;
        let cp_state = CargoPacket::of(s, cp);
        // SAFETY: The module's contract; each reference ends before the payment.
        let remove = self.removal.preprocess(unsafe { &*cp_state });
        unsafe {
            (*self.removal.source).remove_from_meta(&*cp_state, MoveToAction::MTA_DELIVER, remove);
            crate::cargo_payment::pay_final_delivery(
                &crate::cargo_payment::FinalDelivery::of(s, self.payment),
                self.cargo,
                cp_state,
                remove,
                self.current_tile,
            );
            self.removal.postprocess(cp, cp_state, remove)
        }
    }
}
/// `CargoMovement`: abstract action for moving cargo from one list to another.
struct CargoMovement<'a, Tsource, Tdest> {
    s: &'a Services,
    /// Source of the cargo.
    source: *mut Tsource,
    /// Destination for the cargo.
    destination: *mut Tdest,
    /// Maximum amount of cargo to be moved with this action.
    max_move: u32,
}
impl<Tsource, Tdest> CargoMovement<'_, Tsource, Tdest> {
    /// `CargoMovement::Preprocess`: decides if a packet needs to be split. Returns either new
    /// packet if splitting was necessary or the given one otherwise.
    #[allow(unsafe_code, reason = "splits the caller's live packet")]
    unsafe fn preprocess(&mut self, cp: *mut c_void, cp_state: *mut CargoPacket) -> *mut c_void {
        // SAFETY: The caller's live packet; the read ends before the split.
        let count = u32::from(unsafe { (*cp_state).count() });
        if self.max_move < count {
            // SAFETY: As above.
            let cp = unsafe { CargoPacket::split(cp_state, self.s, self.max_move) };
            self.max_move = 0;
            cp
        } else {
            self.max_move -= count;
            cp
        }
    }
    /// The record of `cp_new`, the packet `preprocess` returned for `cp`.
    fn state(
        &self,
        cp: *mut c_void,
        cp_state: *mut CargoPacket,
        cp_new: *mut c_void,
    ) -> *mut CargoPacket {
        if cp_new == cp {
            cp_state
        } else {
            CargoPacket::of(self.s, cp_new)
        }
    }
}
/// `CargoTransfer`: action of transferring cargo from a vehicle to a station.
struct CargoTransfer<'a> {
    movement: CargoMovement<'a, VehicleCargoList, StationCargoList>,
    /// Current tile cargo unloading is happening.
    current_tile: TileIndex,
}
impl CargoAction for CargoTransfer<'_> {
    fn max_move(&self) -> u32 {
        self.movement.max_move
    }
    /// `CargoTransfer::operator()`: transfers some cargo from a vehicle to a station. Returns
    /// true if the packet was completely reserved, false if part of it was.
    #[allow(unsafe_code, reason = "moves the caller's live packet between lists")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let m = &mut self.movement;
        let cp_state = CargoPacket::of(m.s, cp);
        // SAFETY: The module's contract; each reference ends before the next call.
        let cp_new = unsafe { m.preprocess(cp, cp_state) };
        if cp_new.is_null() {
            return false;
        }
        let new_state = m.state(cp, cp_state, cp_new);
        unsafe {
            (*new_state).update_unloading_tile(m.s.shared(), self.current_tile);
            (*m.source).remove_from_meta(
                &*new_state,
                MoveToAction::MTA_TRANSFER,
                u32::from((*new_state).count()),
            );
            // No transfer credits here as they were already granted during Stage().
            let next = (*new_state).get_next_hop();
            StationCargoList::append(m.destination, m.s, cp_new, next);
        }
        cp_new == cp
    }
}
/// `CargoLoad`: action of loading cargo from a station onto a vehicle.
struct CargoLoad<'a> {
    movement: CargoMovement<'a, StationCargoList, VehicleCargoList>,
    /// Current tile cargo loading is happening.
    current_tile: TileIndex,
}
impl CargoAction for CargoLoad<'_> {
    fn max_move(&self) -> u32 {
        self.movement.max_move
    }
    /// `CargoLoad::operator()`: loads some cargo onto a vehicle. Returns true if the packet
    /// was completely loaded, false if part of it was.
    #[allow(unsafe_code, reason = "moves the caller's live packet between lists")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let m = &mut self.movement;
        let cp_state = CargoPacket::of(m.s, cp);
        // SAFETY: The module's contract; each reference ends before the next call.
        let cp_new = unsafe { m.preprocess(cp, cp_state) };
        if cp_new.is_null() {
            return false;
        }
        let new_state = m.state(cp, cp_state, cp_new);
        unsafe {
            (*new_state).update_loading_tile(m.s.shared(), self.current_tile);
            (*m.source)
                .0
                .remove_from_cache(&*new_state, u32::from((*new_state).count()));
            VehicleCargoList::append(m.destination, m.s, cp_new, MoveToAction::MTA_KEEP);
        }
        cp_new == cp
    }
}
/// `CargoReservation`: action of reserving cargo from a station to be loaded onto a vehicle.
struct CargoReservation<'a> {
    load: CargoLoad<'a>,
}
impl CargoAction for CargoReservation<'_> {
    fn max_move(&self) -> u32 {
        self.load.movement.max_move
    }
    /// `CargoReservation::operator()`: reserves some cargo for loading. Returns true if the
    /// packet was completely reserved, false if part of it was.
    #[allow(unsafe_code, reason = "moves the caller's live packet between lists")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let current_tile = self.load.current_tile;
        let m = &mut self.load.movement;
        let cp_state = CargoPacket::of(m.s, cp);
        // SAFETY: The module's contract; each reference ends before the next call.
        let cp_new = unsafe { m.preprocess(cp, cp_state) };
        if cp_new.is_null() {
            return false;
        }
        let new_state = m.state(cp, cp_state, cp_new);
        unsafe {
            (*new_state).update_loading_tile(m.s.shared(), current_tile);
            let source = &mut (*m.source).0;
            let count = u32::from((*new_state).count());
            source.fields.reserved_count = source.fields.reserved_count.wrapping_add(count);
            source.remove_from_cache(&*new_state, count);
            VehicleCargoList::append(m.destination, m.s, cp_new, MoveToAction::MTA_LOAD);
        }
        cp_new == cp
    }
}
/// `CargoReturn`: action of returning previously reserved cargo from the vehicle to the
/// station.
struct CargoReturn<'a> {
    movement: CargoMovement<'a, VehicleCargoList, StationCargoList>,
    /// Current tile cargo unloading is happening.
    current_tile: TileIndex,
    next: StationID,
}
impl CargoAction for CargoReturn<'_> {
    fn max_move(&self) -> u32 {
        self.movement.max_move
    }
    /// `CargoReturn::operator()`: returns some reserved cargo. Returns true if the packet was
    /// completely returned, false if part of it was.
    #[allow(unsafe_code, reason = "moves the caller's live packet between lists")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let m = &mut self.movement;
        let cp_state = CargoPacket::of(m.s, cp);
        // SAFETY: The module's contract; each reference ends before the next call.
        let mut cp_new = unsafe { m.preprocess(cp, cp_state) };
        if cp_new.is_null() {
            cp_new = cp;
        }
        let new_state = m.state(cp, cp_state, cp_new);
        unsafe {
            let count = u32::from((*new_state).count());
            assert_with(|| count <= (*m.destination).0.fields.reserved_count);
            (*new_state).update_unloading_tile(m.s.shared(), self.current_tile);
            (*m.source).remove_from_meta(&*new_state, MoveToAction::MTA_LOAD, count);
            let destination = &mut (*m.destination).0;
            destination.fields.reserved_count =
                destination.fields.reserved_count.wrapping_sub(count);
            StationCargoList::append(m.destination, m.s, cp_new, self.next);
        }
        cp_new == cp
    }
}
/// `CargoShift`: action of shifting cargo from one vehicle to another.
struct CargoShift<'a> {
    movement: CargoMovement<'a, VehicleCargoList, VehicleCargoList>,
}
impl CargoAction for CargoShift<'_> {
    fn max_move(&self) -> u32 {
        self.movement.max_move
    }
    /// `CargoShift::operator()`: shifts some cargo from a vehicle to another one. Returns true
    /// if the packet was completely shifted, false if part of it was.
    #[allow(unsafe_code, reason = "moves the caller's live packet between lists")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let m = &mut self.movement;
        let cp_state = CargoPacket::of(m.s, cp);
        // SAFETY: The module's contract; each reference ends before the next call.
        let mut cp_new = unsafe { m.preprocess(cp, cp_state) };
        if cp_new.is_null() {
            cp_new = cp;
        }
        let new_state = m.state(cp, cp_state, cp_new);
        unsafe {
            (*m.source).remove_from_meta(
                &*new_state,
                MoveToAction::MTA_KEEP,
                u32::from((*new_state).count()),
            );
            VehicleCargoList::append(m.destination, m.s, cp_new, MoveToAction::MTA_KEEP);
        }
        cp_new == cp
    }
}
/// `CargoReroute`: action of rerouting cargo between different cargo lists and/or next hops.
struct CargoReroute<'a, Tlist> {
    movement: CargoMovement<'a, Tlist, Tlist>,
    avoid: StationID,
    avoid2: StationID,
    /// `&ge->GetData().flows`, null where the entry has no data, which `ge->GetVia` reads.
    flows: *mut FlowStatMap,
}
impl<Tlist> CargoReroute<'_, Tlist> {
    /// `ge->GetVia(source, avoid, avoid2)`.
    #[allow(unsafe_code, reason = "reads the station's live flows")]
    fn get_via(&self, source: StationID) -> StationID {
        let s = self.movement.s;
        // SAFETY: The flows of the goods entry the reroute was made for, live for its call.
        unsafe {
            crate::station_base::goods_get_via(
                self.flows,
                source,
                self.avoid,
                self.avoid2,
                |limit| s.random_range(limit),
            )
        }
    }
}
/// `StationCargoReroute`: action of rerouting cargo in a station.
type StationCargoReroute<'a> = CargoReroute<'a, StationCargoList>;
impl CargoAction for StationCargoReroute<'_> {
    fn max_move(&self) -> u32 {
        self.movement.max_move
    }
    /// `StationCargoReroute::operator()`: reroutes some cargo from one Station sublist to
    /// another. Returns true if the packet was completely rerouted, false if part of it was.
    #[allow(unsafe_code, reason = "moves the caller's live packet between lists")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let s = self.movement.s;
        let cp_state = CargoPacket::of(s, cp);
        // SAFETY: The module's contract; each reference ends before the next call, and the
        // source and destination are each borrowed alone.
        let mut cp_new = unsafe { self.movement.preprocess(cp, cp_state) };
        if cp_new.is_null() {
            cp_new = cp;
        }
        let new_state = self.movement.state(cp, cp_state, cp_new);
        let next = self.get_via(unsafe { (*new_state).get_first_station() });
        assert_with(|| next != self.avoid && next != self.avoid2);
        let m = &self.movement;
        unsafe {
            if m.source != m.destination {
                (*m.source)
                    .0
                    .remove_from_cache(&*new_state, u32::from((*new_state).count()));
                (*m.destination).0.add_to_cache(&*new_state);
            }
            // Legal, as insert doesn't invalidate iterators in the MultiMap, however this might
            // insert the packet between range.first and range.second (which might be end()).
            // This is why we check for GetKey above to avoid infinite loops.
            (*m.destination).insert(next, cp_new);
        }
        cp_new == cp
    }
}
/// `VehicleCargoReroute`: action of rerouting cargo staged for transfer in a vehicle.
type VehicleCargoReroute<'a> = CargoReroute<'a, VehicleCargoList>;
impl<'a> VehicleCargoReroute<'a> {
    /// `VehicleCargoReroute(source, dest, max_move, avoid, avoid2, ge)`.
    #[allow(unsafe_code, reason = "reads the caller's live source list")]
    unsafe fn new(
        s: &'a Services,
        source: *mut VehicleCargoList,
        dest: *mut VehicleCargoList,
        max_move: u32,
        avoid: StationID,
        avoid2: StationID,
        flows: *mut FlowStatMap,
    ) -> Self {
        // SAFETY: The caller's live source list; one count read.
        assert_with(|| max_move <= unsafe { (*source).action_count(MoveToAction::MTA_TRANSFER) });
        Self {
            movement: CargoMovement {
                s,
                source,
                destination: dest,
                max_move,
            },
            avoid,
            avoid2,
            flows,
        }
    }
}
impl CargoAction for VehicleCargoReroute<'_> {
    fn max_move(&self) -> u32 {
        self.movement.max_move
    }
    /// `VehicleCargoReroute::operator()`: reroutes some cargo in a `VehicleCargoList`. Returns
    /// true if the packet was completely rerouted, false if part of it was.
    #[allow(unsafe_code, reason = "moves the caller's live packet between lists")]
    unsafe fn call(&mut self, cp: *mut c_void) -> bool {
        let s = self.movement.s;
        let cp_state = CargoPacket::of(s, cp);
        // SAFETY: The module's contract; each reference ends before the next call, and the
        // source and destination are each borrowed alone.
        let mut cp_new = unsafe { self.movement.preprocess(cp, cp_state) };
        if cp_new.is_null() {
            cp_new = cp;
        }
        let new_state = self.movement.state(cp, cp_state, cp_new);
        unsafe {
            let next_hop = (*new_state).get_next_hop();
            if next_hop == self.avoid || next_hop == self.avoid2 {
                let via = self.get_via((*new_state).get_first_station());
                // The original sets the next hop on `cp`, even where `cp_new` is a split part.
                (*cp_state).set_next_hop(via);
            }
            let m = &self.movement;
            if m.source != m.destination {
                (*m.source).remove_from_meta(
                    &*new_state,
                    MoveToAction::MTA_TRANSFER,
                    u32::from((*new_state).count()),
                );
                (*m.destination).add_to_meta(&*new_state, MoveToAction::MTA_TRANSFER);
            }
            // Legal, as front pushing doesn't invalidate iterators in std::list.
            (*m.destination).push_front(cp_new);
        }
        cp_new == cp
    }
}

/// `VehicleCargoList::Stage`'s parameters.
pub(crate) struct StageArgs<'a> {
    /// If the cargo will be accepted at the station.
    pub(crate) accepted: bool,
    /// ID of the station.
    pub(crate) current_station: StationID,
    /// ID of the station the vehicle will go to next.
    pub(crate) next_station: &'a [StationID],
    /// `OrderUnloadType` that will apply to the unload operation.
    pub(crate) unload_type: OrderUnloadType,
    /// `GoodsEntry` for getting the flows.
    pub(crate) ge: *const GoodsEntry,
    /// The cargo type of the cargo.
    pub(crate) cargo: CargoType,
    /// Payment object for registering transfers.
    pub(crate) payment: *mut NativePayment,
    /// Current tile the cargo handling is happening on.
    pub(crate) current_tile: TileIndex,
}

impl VehicleCargoList {
    /// The vehicle list behind a native list handle the caller knows is a vehicle's.
    pub(crate) fn of(l: *mut CargoList) -> *mut Self {
        l.cast()
    }
    /// `packets.push_back(cp)`.
    fn push_back(&mut self, cp: *mut c_void) {
        let previous = self.0.last;
        self.0.link(cp, StationID(0), previous, None);
    }
    /// `packets.push_front(cp)`.
    fn push_front(&mut self, cp: *mut c_void) {
        let next = self.0.first;
        self.0.link(cp, StationID(0), None, next);
    }
    /// `packets.insert(it, cp)`: `cp` before `it`, `None` being `end()`.
    fn insert(&mut self, it: Option<usize>, cp: *mut c_void) {
        let previous = it.map_or(self.0.last, |token| self.0.node(token).previous);
        self.0.link(cp, StationID(0), previous, it);
    }
    /// `packets.erase(it)`.
    fn erase(&mut self, it: usize) {
        self.0.unlink(it);
    }
    /// `VehicleCargoList::Append`: appends the given cargo packet. Tries to merge it with
    /// another one in the packets list. If no fitting packet is found, appends it. You can only
    /// append packets to the ranges of packets designated for keeping or loading. Furthermore
    /// if there are already packets reserved for loading you cannot directly add packets to the
    /// "keep" list. You first have to load the reserved ones. After appending this packet may
    /// not exist anymore! `action` is either `MTA_KEEP` if you want to add the packet directly
    /// or `MTA_LOAD` if you want to reserve it first.
    ///
    /// # Safety
    /// The module's contract; `cp` is a live packet in no list.
    #[allow(
        unsafe_code,
        reason = "adds the caller's live packet to the caller's list"
    )]
    pub(crate) unsafe fn append(
        this: *mut Self,
        s: &Services,
        cp: *mut c_void,
        action: MoveToAction,
    ) {
        assert_with(|| !cp.is_null());
        // SAFETY: The module's contract; each reference ends before the next call.
        assert_with(|| {
            action == MoveToAction::MTA_LOAD
                || (action == MoveToAction::MTA_KEEP
                    && unsafe { (*this).action_count(MoveToAction::MTA_LOAD) } == 0)
        });
        let cp_state = CargoPacket::of(s, cp);
        unsafe {
            (*this).add_to_meta(&*cp_state, action);
            if (*this).0.fields.count == u32::from((*cp_state).count()) {
                (*this).push_back(cp);
                return;
            }
        }
        let mut sum = u32::from(unsafe { (*cp_state).count() });
        let mut it = unsafe { (*this).0.back() };
        while let Some(current) = it {
            let icp = CargoPacket::of(s, current.shell);
            if unsafe { CargoList::try_merge(s, icp, cp, cp_state, Self::are_mergable) } {
                return;
            }
            sum = sum.wrapping_add(u32::from(unsafe { (*icp).count() }));
            if sum >= unsafe { (*this).action_count(action) } {
                unsafe { (*this).push_back(cp) };
                return;
            }
            it = current
                .previous
                .map(|token| unsafe { (*this).0.node(token) });
        }
        not_reached();
    }
    /// `VehicleCargoList::ShiftCargo`: shifts cargo from the front of the packet list and
    /// applies some action to it. If the action returns true the cargo packet will be removed
    /// from the list. Otherwise it will be kept and the loop will be aborted.
    #[allow(unsafe_code, reason = "applies the action to the caller's live list")]
    unsafe fn shift_cargo(this: *mut Self, mut action: impl CargoAction) {
        // SAFETY: The module's contract; each reference ends before the action.
        let mut it = unsafe { (*this).0.first };
        while let Some(token) = it
            && action.max_move() > 0
        {
            let cp = unsafe { (*this).0.node(token) }.shell;
            if unsafe { action.call(cp) } {
                it = unsafe { (*this).0.node(token) }.next;
                unsafe { (*this).erase(token) };
            } else {
                break;
            }
        }
    }
    /// `VehicleCargoList::PopCargo`: pops cargo from the back of the packet list and applies
    /// some action to it. If the action returns true the cargo packet will be removed from the
    /// list. Otherwise it will be kept and the loop will be aborted.
    #[allow(unsafe_code, reason = "applies the action to the caller's live list")]
    unsafe fn pop_cargo(this: *mut Self, mut action: impl CargoAction) {
        // SAFETY: The module's contract; each reference ends before the action.
        let (Some(mut it), Some(begin)) = (unsafe { (*this).0.last }, unsafe { (*this).0.first })
        else {
            return;
        };
        while action.max_move() > 0 {
            let cp = unsafe { (*this).0.node(it) }.shell;
            if unsafe { action.call(cp) } {
                if it != begin {
                    let previous = unsafe { (*this).0.node(it) }.previous.unwrap();
                    unsafe { (*this).erase(it) };
                    it = previous;
                } else {
                    unsafe { (*this).erase(it) };
                    break;
                }
            } else {
                break;
            }
        }
    }
    /// `VehicleCargoList::RemoveFromCache`: update the cached values to reflect the removal of
    /// this packet or part of it. Decreases count, feeder share and `periods_in_transit`.
    fn remove_from_cache(&mut self, cp: &CargoPacket, count: u32) {
        self.0.fields.feeder_share = self
            .0
            .fields
            .feeder_share
            .saturating_sub(cp.get_feeder_share_part(count));
        self.0.remove_from_cache(cp, count);
    }
    /// `VehicleCargoList::AddToCache`: update the cache to reflect adding of this packet.
    /// Increases count, feeder share and `periods_in_transit`.
    fn add_to_cache(&mut self, cp: &CargoPacket) {
        self.0.fields.feeder_share = self.0.fields.feeder_share.saturating_add(cp.feeder_share);
        self.0.add_to_cache(cp);
    }
    /// `VehicleCargoList::RemoveFromMeta`: removes a packet or part of it from the metadata.
    fn remove_from_meta(&mut self, cp: &CargoPacket, action: MoveToAction, count: u32) {
        assert_with(|| count <= self.action_count(action));
        self.assert_count_consistency();
        self.remove_from_cache(cp, count);
        let counts = &mut self.0.fields.action_counts;
        counts[usize::from(action.0)] = counts[usize::from(action.0)].wrapping_sub(count);
        self.assert_count_consistency();
    }
    /// `VehicleCargoList::AddToMeta`: adds a packet to the metadata.
    fn add_to_meta(&mut self, cp: &CargoPacket, action: MoveToAction) {
        self.assert_count_consistency();
        self.add_to_cache(cp);
        let counts = &mut self.0.fields.action_counts;
        counts[usize::from(action.0)] =
            counts[usize::from(action.0)].wrapping_add(u32::from(cp.count));
        self.assert_count_consistency();
    }
    /// `VehicleCargoList::AssertCountConsistency`: assert that the designation counts add up.
    fn assert_count_consistency(&self) {
        assert_with(|| {
            let counts = &self.0.fields.action_counts;
            counts[usize::from(MoveToAction::MTA_KEEP.0)]
                .wrapping_add(counts[usize::from(MoveToAction::MTA_DELIVER.0)])
                .wrapping_add(counts[usize::from(MoveToAction::MTA_TRANSFER.0)])
                .wrapping_add(counts[usize::from(MoveToAction::MTA_LOAD.0)])
                == self.0.fields.count
        });
    }
    /// `VehicleCargoList::AgeCargo`: ages the all cargo in this list. `COUNTED` adds each
    /// packet lookup to the scheduler's witness counters.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(unsafe_code, reason = "writes the caller's live list and its packets")]
    pub(crate) unsafe fn age_cargo<const COUNTED: bool>(this: *mut Self, s: &Services) {
        // SAFETY: The module's contract; each reference ends before the next call.
        let mut it = unsafe { (*this).0.front() };
        while let Some(current) = it {
            it = unsafe { (*this).0.after(current.token) };
            let cp = CargoPacket::of(s, current.shell);
            if COUNTED {
                crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
                crate::witness::hit(crate::witness::VEHICLE_PACKET_LOOKUPS);
                crate::witness::add(
                    crate::witness::VEHICLE_OBSERVED_BYTES,
                    std::mem::size_of_val(&cp) as u64,
                );
            }
            let cp = unsafe { &mut *cp };
            // If we're at the maximum, then we can't increase no more.
            if cp.periods_in_transit == u16::MAX {
                continue;
            }
            cp.periods_in_transit += 1;
            let fields = unsafe { &mut (*this).0.fields };
            fields.cargo_periods_in_transit = fields
                .cargo_periods_in_transit
                .wrapping_add(u64::from(cp.count));
        }
    }
    /// `VehicleCargoList::ChooseAction`: choose action to be performed with the given cargo
    /// packet. `cargo_next` is the next hop the cargo wants to pass, `current_station` the
    /// current station of the vehicle carrying the cargo, `accepted` if the cargo is accepted
    /// at the current station and `next_station` the next station(s) the vehicle may stop at.
    fn choose_action(
        cp: &CargoPacket,
        cargo_next: StationID,
        current_station: StationID,
        accepted: bool,
        next_station: &[StationID],
    ) -> MoveToAction {
        if cargo_next == StationID::INVALID {
            if accepted && cp.get_first_station() != current_station {
                MoveToAction::MTA_DELIVER
            } else {
                MoveToAction::MTA_KEEP
            }
        } else if cargo_next == current_station {
            MoveToAction::MTA_DELIVER
        } else if next_station.contains(&cargo_next) {
            MoveToAction::MTA_KEEP
        } else {
            MoveToAction::MTA_TRANSFER
        }
    }
    /// `VehicleCargoList::Stage`: stages cargo for unloading. The cargo is sorted so that
    /// packets to be transferred, delivered or kept are in consecutive chunks in the list. At
    /// the same time the designation counts are updated to reflect the size of those chunks.
    /// Returns if any cargo will be unloaded.
    ///
    /// # Safety
    /// The module's contract; `args.ge` is the live goods entry and `args.payment` the live
    /// payment of the unloading vehicle.
    #[allow(
        unsafe_code,
        reason = "reorders the caller's live list and pays its transfers"
    )]
    pub(crate) unsafe fn stage(this: *mut Self, s: &Services, args: &StageArgs<'_>) -> bool {
        // SAFETY: The module's contract; each reference ends before the next call.
        unsafe {
            (*this).assert_count_consistency();
            assert_with(|| (*this).action_count(MoveToAction::MTA_LOAD) == 0);
            let counts = &mut (*this).0.fields.action_counts;
            counts[usize::from(MoveToAction::MTA_KEEP.0)] = 0;
            counts[usize::from(MoveToAction::MTA_DELIVER.0)] = 0;
            counts[usize::from(MoveToAction::MTA_TRANSFER.0)] = 0;
        }
        let mut deliver: Option<usize> = None;
        let mut it = unsafe { (*this).0.first };
        let mut sum = 0_u32;
        let flows = (s.flow_map)(args.ge);
        // SAFETY: The entry's flows, which nothing in the loop changes.
        let flows = if flows.is_null() {
            None
        } else {
            Some(unsafe { &*flows })
        };
        let force_keep = args.unload_type == OrderUnloadType::NoUnload;
        let force_unload = args.unload_type == OrderUnloadType::Unload;
        let force_transfer = args.unload_type == OrderUnloadType::Transfer
            || args.unload_type == OrderUnloadType::Unload;
        assert_with(|| unsafe { (*this).0.fields.count } > 0 || it.is_none());
        while sum < unsafe { (*this).0.fields.count } {
            let current = unsafe { (*this).0.node(it.unwrap()) };
            it = current.next;
            unsafe { (*this).erase(current.token) };
            let cp = current.shell;
            let cp_state = CargoPacket::of(s, cp);
            let mut cargo_next = StationID::INVALID;
            let mut action;
            if force_keep {
                action = MoveToAction::MTA_KEEP;
            } else if force_unload
                && args.accepted
                && unsafe { (*cp_state).get_first_station() } != args.current_station
            {
                action = MoveToAction::MTA_DELIVER;
            } else if force_transfer {
                action = MoveToAction::MTA_TRANSFER;
                // We cannot send the cargo to any of the possible next hops and also not to the
                // current station.
                let flow_it =
                    flows.and_then(|flows| flows.find(unsafe { (*cp_state).get_first_station() }));
                if let Some(flow) = flow_it {
                    let mut new_shares = flow.clone();
                    new_shares.change_share(args.current_station, i32::MIN);
                    for &station in args.next_station.iter().rev() {
                        if new_shares.get_shares().is_empty() {
                            break;
                        }
                        new_shares.change_share(station, i32::MIN);
                    }
                    cargo_next = if new_shares.get_shares().is_empty() {
                        StationID::INVALID
                    } else {
                        new_shares.get_via(|limit| s.random_range(limit))
                    };
                } else {
                    cargo_next = StationID::INVALID;
                }
            } else {
                // Rewrite an invalid source station to some random other one to avoid keeping
                // the cargo in the vehicle forever.
                if unsafe { (*cp_state).get_first_station() } == StationID::INVALID
                    && let Some(first) = flows.and_then(FlowStatMap::begin)
                {
                    unsafe { (*cp_state).first_station = first.0 };
                }
                let mut restricted = false;
                let flow_it =
                    flows.and_then(|flows| flows.find(unsafe { (*cp_state).get_first_station() }));
                cargo_next = flow_it.map_or(StationID::INVALID, |flow| {
                    flow.get_via_with_restricted(&mut restricted, |limit| s.random_range(limit))
                });
                action = Self::choose_action(
                    unsafe { &*cp_state },
                    cargo_next,
                    args.current_station,
                    args.accepted,
                    args.next_station,
                );
                if restricted && action == MoveToAction::MTA_TRANSFER {
                    // If the flow is restricted we can't transfer to it. Choose an unrestricted
                    // one instead.
                    cargo_next = flow_it.unwrap().get_via(|limit| s.random_range(limit));
                    action = Self::choose_action(
                        unsafe { &*cp_state },
                        cargo_next,
                        args.current_station,
                        args.accepted,
                        args.next_station,
                    );
                }
            }
            match action {
                MoveToAction::MTA_KEEP => unsafe {
                    (*this).push_back(cp);
                    if deliver.is_none() {
                        deliver = (*this).0.last;
                    }
                },
                MoveToAction::MTA_DELIVER => unsafe { (*this).insert(deliver, cp) },
                MoveToAction::MTA_TRANSFER => {
                    unsafe { (*this).push_front(cp) };
                    // Add feeder share here to allow reusing field for next station.
                    let share = unsafe {
                        crate::cargo_payment::pay_transfer(
                            (s.payment_state)(args.payment),
                            &*s.payment,
                            s.shared(),
                            args.cargo,
                            cp_state,
                            u32::from((*cp_state).count()),
                            args.current_tile,
                        )
                    };
                    unsafe {
                        (*cp_state).add_feeder_share(share);
                        let fields = &mut (*this).0.fields;
                        fields.feeder_share = fields.feeder_share.saturating_add(share);
                        (*cp_state).next_hop = cargo_next.0;
                    }
                }
                _ => not_reached(),
            }
            let count = u32::from(unsafe { (*cp_state).count() });
            unsafe {
                let counts = &mut (*this).0.fields.action_counts;
                counts[usize::from(action.0)] = counts[usize::from(action.0)].wrapping_add(count);
            }
            sum = sum.wrapping_add(count);
        }
        unsafe {
            (*this).assert_count_consistency();
            (*this).action_count(MoveToAction::MTA_DELIVER) > 0
                || (*this).action_count(MoveToAction::MTA_TRANSFER) > 0
        }
    }
    /// `VehicleCargoList::InvalidateCache`: invalidates the cached data and rebuild it.
    #[allow(unsafe_code, reason = "reads the live packets of the caller's list")]
    unsafe fn invalidate_cache(this: *mut Self, s: &Services) {
        // SAFETY: The module's contract; each reference ends before the next call.
        unsafe {
            (*this).0.fields.feeder_share = 0;
            (*this).0.fields.count = 0;
            (*this).0.fields.cargo_periods_in_transit = 0;
        }
        let mut it = unsafe { (*this).0.front() };
        while let Some(current) = it {
            it = unsafe { (*this).0.after(current.token) };
            let cp = CargoPacket::of(s, current.shell);
            unsafe { (*this).add_to_cache(&*cp) };
        }
    }
    /// `VehicleCargoList::Reassign<Tfrom, Tto>`: moves some cargo from one designation to
    /// another. You can only move between adjacent designations. E.g. you can keep cargo that
    /// was previously reserved (`MTA_LOAD`), but you can't reserve cargo that's marked as to be
    /// delivered. Furthermore, as this method doesn't change the actual packets, you cannot
    /// move cargo from or to `MTA_TRANSFER`; the specialisation for (`MTA_DELIVER`,
    /// `MTA_TRANSFER`) does that, and takes care of the next station the cargo wants to visit.
    /// Returns the amount of cargo actually reassigned.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(unsafe_code, reason = "splits the live packets of the caller's list")]
    pub(crate) unsafe fn reassign(
        this: *mut Self,
        s: &Services,
        from: MoveToAction,
        to: MoveToAction,
        max_move: u32,
    ) -> u32 {
        // SAFETY: The module's contract; each reference ends before the next call.
        let max_move = unsafe { (*this).action_count(from) }.min(max_move);
        if from == MoveToAction::MTA_DELIVER && to == MoveToAction::MTA_TRANSFER {
            let transfer = unsafe { (*this).action_count(MoveToAction::MTA_TRANSFER) };
            let mut sum = 0_u32;
            let mut it = unsafe { (*this).0.first };
            while sum < transfer.wrapping_add(max_move) {
                let current = unsafe { (*this).0.node(it.unwrap()) };
                it = current.next;
                let cp = CargoPacket::of(s, current.shell);
                sum = sum.wrapping_add(u32::from(unsafe { (*cp).count() }));
                if sum <= transfer {
                    continue;
                }
                if sum > transfer.wrapping_add(max_move) {
                    // The original writes `sum - action_counts[MTA_TRANSFER] + max_move`.
                    let cp_split = unsafe {
                        CargoPacket::split(cp, s, sum.wrapping_sub(transfer).wrapping_add(max_move))
                    };
                    sum = sum.wrapping_sub(u32::from(unsafe {
                        (*CargoPacket::of(s, cp_split)).count()
                    }));
                    unsafe { (*this).insert(it, cp_split) };
                }
                unsafe { (*cp).set_next_hop(StationID::INVALID) };
            }
        }
        unsafe {
            let counts = &mut (*this).0.fields.action_counts;
            counts[usize::from(from.0)] = counts[usize::from(from.0)].wrapping_sub(max_move);
            counts[usize::from(to.0)] = counts[usize::from(to.0)].wrapping_add(max_move);
        }
        max_move
    }
    /// `VehicleCargoList::Return`: returns reserved cargo to the station and removes it from
    /// the cache. `next` is the ID of the next station the cargo wants to go to and
    /// `current_tile` the tile the cargo handling is happening on. Returns the amount of cargo
    /// actually returned.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(unsafe_code, reason = "moves packets between the caller's live lists")]
    pub(crate) unsafe fn return_(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        dest: *mut StationCargoList,
        next: StationID,
        current_tile: TileIndex,
    ) -> u32 {
        // SAFETY: The module's contract.
        let max_move = unsafe { (*this).action_count(MoveToAction::MTA_LOAD) }.min(max_move);
        unsafe {
            Self::pop_cargo(
                this,
                CargoReturn {
                    movement: CargoMovement {
                        s,
                        source: this,
                        destination: dest,
                        max_move,
                    },
                    current_tile,
                    next,
                },
            );
        }
        max_move
    }
    /// `VehicleCargoList::Shift`: shifts cargo between two vehicles. Returns the amount of
    /// cargo actually moved.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(unsafe_code, reason = "moves packets between the caller's live lists")]
    pub(crate) unsafe fn shift(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        dest: *mut VehicleCargoList,
    ) -> u32 {
        // SAFETY: The module's contract.
        let max_move = unsafe { (*this).0.fields.count }.min(max_move);
        unsafe {
            Self::pop_cargo(
                this,
                CargoShift {
                    movement: CargoMovement {
                        s,
                        source: this,
                        destination: dest,
                        max_move,
                    },
                },
            );
        }
        max_move
    }
    /// `VehicleCargoList::Unload`: unloads cargo at the given station. Deliver or transfer,
    /// depending on the ranges defined by the designation counts. Returns the amount of cargo
    /// actually unloaded.
    ///
    /// # Safety
    /// The module's contract; `payment` is the live payment of the unloading vehicle.
    #[allow(
        unsafe_code,
        reason = "moves and delivers packets of the caller's live lists"
    )]
    pub(crate) unsafe fn unload(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        dest: *mut StationCargoList,
        cargo: CargoType,
        payment: *mut NativePayment,
        current_tile: TileIndex,
    ) -> u32 {
        let mut moved = 0_u32;
        // SAFETY: The module's contract; each count read ends before the action.
        if unsafe { (*this).action_count(MoveToAction::MTA_TRANSFER) } > 0 {
            let r#move = unsafe { (*this).action_count(MoveToAction::MTA_TRANSFER) }.min(max_move);
            unsafe {
                Self::shift_cargo(
                    this,
                    CargoTransfer {
                        movement: CargoMovement {
                            s,
                            source: this,
                            destination: dest,
                            max_move: r#move,
                        },
                        current_tile,
                    },
                );
            }
            moved = moved.wrapping_add(r#move);
        }
        if unsafe { (*this).action_count(MoveToAction::MTA_TRANSFER) } == 0
            && unsafe { (*this).action_count(MoveToAction::MTA_DELIVER) } > 0
            && moved < max_move
        {
            let r#move =
                unsafe { (*this).action_count(MoveToAction::MTA_DELIVER) }.min(max_move - moved);
            unsafe {
                Self::shift_cargo(
                    this,
                    CargoDelivery {
                        removal: CargoRemoval {
                            s,
                            source: this,
                            max_move: r#move,
                        },
                        current_tile,
                        payment,
                        cargo,
                    },
                );
            }
            moved = moved.wrapping_add(r#move);
        }
        moved
    }
    /// `VehicleCargoList::Truncate`: truncates the cargo in this list to the given amount. It
    /// leaves the first cargo entities and removes `max_move` from the back of the list.
    /// Returns the amount of entities actually moved.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(unsafe_code, reason = "deletes packets of the caller's live list")]
    pub(crate) unsafe fn truncate(this: *mut Self, s: &Services, max_move: u32) -> u32 {
        // SAFETY: The module's contract.
        let max_move = unsafe { (*this).0.fields.count }.min(max_move);
        unsafe {
            Self::pop_cargo(
                this,
                CargoRemoval {
                    s,
                    source: this,
                    max_move,
                },
            );
        }
        max_move
    }
    /// `VehicleCargoList::Reroute`: routes packets with station `avoid` as next hop to a
    /// different place. `dest` is the list to prepend the cargo to, `avoid2` an additional
    /// station to exclude from routing and `ge` the `GoodsEntry` to get the routing info from.
    ///
    /// # Safety
    /// The module's contract; `ge` is a live goods entry.
    #[allow(unsafe_code, reason = "moves packets between the caller's live lists")]
    pub(crate) unsafe fn reroute(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        dest: *mut VehicleCargoList,
        avoid: StationID,
        avoid2: StationID,
        ge: *const GoodsEntry,
    ) -> u32 {
        // SAFETY: The module's contract.
        let max_move = unsafe { (*this).action_count(MoveToAction::MTA_TRANSFER) }.min(max_move);
        unsafe {
            let action =
                VehicleCargoReroute::new(s, this, dest, max_move, avoid, avoid2, (s.flow_map)(ge));
            Self::shift_cargo(this, action);
        }
        max_move
    }
    /// `VehicleCargoList::KeepAll`: marks all cargo in the vehicle as to be kept.
    pub(crate) fn keep_all(&mut self) {
        let counts = &mut self.0.fields.action_counts;
        // `action_counts[MTA_DELIVER] = action_counts[MTA_TRANSFER] = action_counts[MTA_LOAD] =
        // 0` stores right to left.
        counts[usize::from(MoveToAction::MTA_LOAD.0)] = 0;
        counts[usize::from(MoveToAction::MTA_TRANSFER.0)] = 0;
        counts[usize::from(MoveToAction::MTA_DELIVER.0)] = 0;
        counts[usize::from(MoveToAction::MTA_KEEP.0)] = self.0.fields.count;
    }
    /// `VehicleCargoList::GetFirstStation`: returns the first station of the first cargo
    /// packet in this list.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(
        unsafe_code,
        reason = "reads the first live packet of the caller's list"
    )]
    pub(crate) unsafe fn get_first_station(&self, s: &Services) -> StationID {
        // SAFETY: The module's contract.
        unsafe { self.0.front_first_station(s) }
    }
    /// `VehicleCargoList::GetFeederShare`: returns total sum of the feeder share for all
    /// packets.
    pub(crate) fn get_feeder_share(&self) -> i64 {
        self.0.fields.feeder_share
    }
    /// `VehicleCargoList::ActionCount`: returns the amount of cargo designated for a given
    /// purpose.
    pub(crate) fn action_count(&self, action: MoveToAction) -> u32 {
        self.0.fields.action_counts[usize::from(action.0)]
    }
    /// `VehicleCargoList::StoredCount`: returns sum of cargo on board the vehicle (ie not only
    /// reserved).
    pub(crate) fn stored_count(&self) -> u32 {
        self.0
            .fields
            .count
            .wrapping_sub(self.action_count(MoveToAction::MTA_LOAD))
    }
    /// `VehicleCargoList::TotalCount`: returns sum of cargo, including reserved cargo.
    pub(crate) fn total_count(&self) -> u32 {
        self.0.fields.count
    }
    /// `VehicleCargoList::ReservedCount`: returns sum of reserved cargo.
    pub(crate) fn reserved_count(&self) -> u32 {
        self.action_count(MoveToAction::MTA_LOAD)
    }
    /// `VehicleCargoList::UnloadCount`: returns sum of cargo to be moved out of the vehicle at
    /// the current station.
    pub(crate) fn unload_count(&self) -> u32 {
        self.action_count(MoveToAction::MTA_TRANSFER)
            .wrapping_add(self.action_count(MoveToAction::MTA_DELIVER))
    }
    /// `VehicleCargoList::RemainingCount`: returns the sum of cargo to be kept in the vehicle
    /// at the current station.
    pub(crate) fn remaining_count(&self) -> u32 {
        self.action_count(MoveToAction::MTA_KEEP)
            .wrapping_add(self.action_count(MoveToAction::MTA_LOAD))
    }
    /// `CargoList::PeriodsInTransit`.
    pub(crate) fn periods_in_transit(&self) -> u32 {
        self.0.periods_in_transit()
    }
    /// `VehicleCargoList::AreMergable`: are the two packets mergeable in the context of a list
    /// of `CargoPacket`s for a vehicle?
    fn are_mergable(cp1: &CargoPacket, cp2: &CargoPacket) -> bool {
        cp1.source_xy == cp2.source_xy
            && cp1.periods_in_transit == cp2.periods_in_transit
            && cp1.first_station == cp2.first_station
            && cp1.get_source() == cp2.get_source()
    }
}

impl StationCargoList {
    /// The station list behind a native list handle the caller knows is a station's.
    pub(crate) fn of(l: *mut CargoList) -> *mut Self {
        l.cast()
    }
    /// `packets.Insert(key, cp)`: insert `cp` at the end of the range with `key`.
    fn insert(&mut self, key: StationID, cp: *mut c_void) {
        let l = &mut self.0;
        let next = if let Some(ends) = l.destinations.get(&key) {
            l.node(ends.last).next
        } else {
            l.destinations
                .range(key..)
                .next()
                .map(|(_, ends)| ends.first)
        };
        let previous = next.map_or(l.last, |token| l.node(token).previous);
        let token = l.link(cp, key, previous, next);
        l.keys.insert(key);
        l.destinations
            .entry(key)
            .and_modify(|ends| ends.last = token)
            .or_insert(Ends {
                first: token,
                last: token,
            });
    }
    /// `packets.erase(it)`.
    fn erase(&mut self, it: usize) {
        let l = &mut self.0;
        let removed = l.unlink(it);
        let ends = l.destinations.get_mut(&removed.key).unwrap();
        if ends.first == it && ends.last == it {
            l.destinations.remove(&removed.key);
            l.keys.remove(&removed.key);
        } else {
            if ends.first == it {
                ends.first = removed.next.unwrap();
            }
            if ends.last == it {
                ends.last = removed.previous.unwrap();
            }
        }
    }
    /// `StationCargoList::Append`: appends the given cargo packet to the range of packets with
    /// the same next station. After appending this packet may not exist anymore!
    ///
    /// # Safety
    /// The module's contract; `cp` is a live packet in no list.
    #[allow(
        unsafe_code,
        reason = "adds the caller's live packet to the caller's list"
    )]
    pub(crate) unsafe fn append(this: *mut Self, s: &Services, cp: *mut c_void, next: StationID) {
        assert_with(|| !cp.is_null());
        let cp_state = CargoPacket::of(s, cp);
        // SAFETY: The module's contract; each reference ends before the next call.
        unsafe { (*this).0.add_to_cache(&*cp_state) };
        let mut it = unsafe { (*this).0.destinations.get(&next) }
            .map(|ends| unsafe { (*this).0.node(ends.last) });
        while let Some(current) = it {
            let icp = CargoPacket::of(s, current.shell);
            if unsafe { CargoList::try_merge(s, icp, cp, cp_state, Self::are_mergable) } {
                return;
            }
            it = current
                .previous
                .map(|token| unsafe { (*this).0.node(token) })
                .filter(|node| node.key == next);
        }
        // The packet could not be merged with another one.
        unsafe { (*this).insert(next, cp) };
    }
    /// `StationCargoList::ShiftCargo(action, next)`: shifts cargo from the front of the packet
    /// list for a specific station and applies some action to it. If the action returns true
    /// the cargo packet will be removed from the list. Otherwise it will be kept and the loop
    /// will be aborted. Returns true if all packets with the given next hop have been removed,
    /// false otherwise.
    #[allow(unsafe_code, reason = "applies the action to the caller's live list")]
    unsafe fn shift_cargo_next(
        this: *mut Self,
        action: &mut impl CargoAction,
        next: StationID,
    ) -> bool {
        // SAFETY: The module's contract; each reference ends before the action.
        let mut it = unsafe { (*this).0.destinations.get(&next) }.map(|ends| ends.first);
        while let Some(token) = it {
            let current = unsafe { (*this).0.node(token) };
            if current.key != next {
                break;
            }
            if action.max_move() == 0 {
                return false;
            }
            if unsafe { action.call(current.shell) } {
                it = unsafe { (*this).0.node(token) }.next;
                unsafe { (*this).erase(token) };
            } else {
                return false;
            }
        }
        true
    }
    /// `StationCargoList::ShiftCargo(action, next, include_invalid)`: shifts cargo from the
    /// front of the packet list for a specific station and optional also from the list for
    /// "any station", then applies some action to it. `include_invalid` is if cargo from the
    /// `StationID::Invalid()` list should be used if necessary. Returns the amount of cargo
    /// actually moved.
    #[allow(unsafe_code, reason = "applies the action to the caller's live list")]
    unsafe fn shift_cargo(
        this: *mut Self,
        mut action: impl CargoAction,
        next: &[StationID],
        include_invalid: bool,
    ) -> u32 {
        let max_move = action.max_move();
        // SAFETY: The module's contract.
        for &station in next.iter().rev() {
            unsafe { Self::shift_cargo_next(this, &mut action, station) };
            if action.max_move() == 0 {
                break;
            }
        }
        if include_invalid && action.max_move() > 0 {
            unsafe { Self::shift_cargo_next(this, &mut action, StationID::INVALID) };
        }
        max_move - action.max_move()
    }
    /// `StationCargoList::Truncate`: truncates where each destination loses roughly the same
    /// percentage of its cargo. This is done by randomizing the selection of packets to be
    /// removed. Optionally count the cargo by origin station. Returns the amount of cargo
    /// actually moved.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(
        unsafe_code,
        reason = "reduces and deletes packets of the caller's live list"
    )]
    pub(crate) unsafe fn truncate(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        mut cargo_per_source: Option<&mut StationCargoAmountMap>,
    ) -> u32 {
        // SAFETY: The module's contract; each reference ends before the next call.
        let max_move = max_move.min(unsafe { (*this).0.fields.count });
        let prev_count = unsafe { (*this).0.fields.count };
        let mut moved = 0_u32;
        let mut r#loop = 0_u32;
        let do_count = cargo_per_source.is_some();
        while max_move > moved {
            let mut it = unsafe { (*this).0.first };
            while let Some(token) = it {
                let current = unsafe { (*this).0.node(token) };
                let cp = CargoPacket::of(s, current.shell);
                if prev_count > max_move && s.random_range(prev_count) < prev_count - max_move {
                    if do_count && r#loop == 0 {
                        add_amount(
                            &mut cargo_per_source,
                            unsafe { &*cp },
                            u32::from(unsafe { (*cp).count }),
                        );
                    }
                    it = current.next;
                    continue;
                }
                let diff = max_move.wrapping_sub(moved);
                if u32::from(unsafe { (*cp).count }) > diff {
                    if diff > 0 {
                        unsafe {
                            (*this).0.remove_from_cache(&*cp, diff);
                            (*cp).reduce(diff);
                        }
                        moved = moved.wrapping_add(diff);
                    }
                    if r#loop > 0 {
                        if do_count {
                            sub_amount(&mut cargo_per_source, unsafe { &*cp }, diff);
                        }
                        return moved;
                    }
                    if do_count {
                        add_amount(
                            &mut cargo_per_source,
                            unsafe { &*cp },
                            u32::from(unsafe { (*cp).count }),
                        );
                    }
                    it = current.next;
                } else {
                    it = current.next;
                    unsafe { (*this).erase(token) };
                    if do_count && r#loop > 0 {
                        sub_amount(
                            &mut cargo_per_source,
                            unsafe { &*cp },
                            u32::from(unsafe { (*cp).count }),
                        );
                    }
                    moved = moved.wrapping_add(u32::from(unsafe { (*cp).count }));
                    unsafe { (*this).0.remove_from_cache(&*cp, u32::from((*cp).count)) };
                    (s.destroy)(current.shell);
                }
            }
            r#loop = r#loop.wrapping_add(1);
        }
        moved
    }
    /// `StationCargoList::Reserve`: reserves cargo for loading onto the vehicle. `next_station`
    /// is the next station(s) the loading vehicle will visit and `current_tile` the tile the
    /// cargo handling is happening on. Returns the amount of cargo actually reserved.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(unsafe_code, reason = "moves packets between the caller's live lists")]
    pub(crate) unsafe fn reserve(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        dest: *mut VehicleCargoList,
        next_station: &[StationID],
        current_tile: TileIndex,
    ) -> u32 {
        let action = CargoReservation {
            load: CargoLoad {
                movement: CargoMovement {
                    s,
                    source: this,
                    destination: dest,
                    max_move,
                },
                current_tile,
            },
        };
        // SAFETY: The module's contract.
        unsafe { Self::shift_cargo(this, action, next_station, true) }
    }
    /// `StationCargoList::Load`: loads cargo onto a vehicle. If the vehicle has reserved cargo
    /// load that. Otherwise load cargo from the station. Returns the amount of cargo actually
    /// loaded. Vehicles may or may not reserve, depending on their orders. The two modes of
    /// loading are exclusive, though. If cargo is reserved we don't need to load unreserved
    /// cargo.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(unsafe_code, reason = "moves packets between the caller's live lists")]
    pub(crate) unsafe fn load(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        dest: *mut VehicleCargoList,
        next_station: &[StationID],
        current_tile: TileIndex,
    ) -> u32 {
        // SAFETY: The module's contract; each reference ends before the next call.
        let r#move = unsafe { (*dest).action_count(MoveToAction::MTA_LOAD) }.min(max_move);
        if r#move > 0 {
            unsafe {
                let fields = &mut (*this).0.fields;
                fields.reserved_count = fields.reserved_count.wrapping_sub(r#move);
                VehicleCargoList::reassign(
                    dest,
                    s,
                    MoveToAction::MTA_LOAD,
                    MoveToAction::MTA_KEEP,
                    r#move,
                );
            }
            r#move
        } else {
            let action = CargoLoad {
                movement: CargoMovement {
                    s,
                    source: this,
                    destination: dest,
                    max_move,
                },
                current_tile,
            };
            unsafe { Self::shift_cargo(this, action, next_station, true) }
        }
    }
    /// `StationCargoList::Reroute`: routes packets with station `avoid` as next hop to a
    /// different place. `dest` is the list to append the cargo to, `avoid2` an additional
    /// station to exclude from routing and `ge` the `GoodsEntry` to get the routing info from.
    ///
    /// # Safety
    /// The module's contract; `ge` is a live goods entry.
    #[allow(unsafe_code, reason = "moves packets between the caller's live lists")]
    pub(crate) unsafe fn reroute(
        this: *mut Self,
        s: &Services,
        max_move: u32,
        dest: *mut StationCargoList,
        avoid: StationID,
        avoid2: StationID,
        ge: *const GoodsEntry,
    ) -> u32 {
        let action = StationCargoReroute {
            movement: CargoMovement {
                s,
                source: this,
                destination: dest,
                max_move,
            },
            avoid,
            avoid2,
            flows: (s.flow_map)(ge),
        };
        // SAFETY: The module's contract.
        unsafe { Self::shift_cargo(this, action, &[avoid], false) }
    }
    /// `StationCargoList::InvalidateCache`: `CargoList::InvalidateCache` with the station's
    /// `AddToCache`.
    #[allow(unsafe_code, reason = "reads the live packets of the caller's list")]
    unsafe fn invalidate_cache(this: *mut Self, s: &Services) {
        // SAFETY: The module's contract; each reference ends before the next call.
        unsafe {
            (*this).0.fields.count = 0;
            (*this).0.fields.cargo_periods_in_transit = 0;
        }
        let mut it = unsafe { (*this).0.front() };
        while let Some(current) = it {
            it = unsafe { (*this).0.after(current.token) };
            let cp = CargoPacket::of(s, current.shell);
            unsafe { (*this).0.add_to_cache(&*cp) };
        }
    }
    /// `StationCargoList::HasCargoFor`: are there packets waiting for any of the given next
    /// stations?
    pub(crate) fn has_cargo_for(&self, next: &[StationID]) -> bool {
        for station in next {
            if self.0.keys.contains(station) {
                return true;
            }
        }
        // Packets for StationID::Invalid() can go anywhere.
        self.0.keys.contains(&StationID::INVALID)
    }
    /// `StationCargoList::GetFirstStation`: returns first station of the first cargo packet in
    /// this list.
    ///
    /// # Safety
    /// The module's contract.
    #[allow(
        unsafe_code,
        reason = "reads the first live packet of the caller's list"
    )]
    pub(crate) unsafe fn get_first_station(&self, s: &Services) -> StationID {
        // SAFETY: The module's contract.
        unsafe { self.0.front_first_station(s) }
    }
    /// `StationCargoList::ReservedCount`: returns sum of cargo reserved for loading onto
    /// vehicles.
    pub(crate) fn reserved_count(&self) -> u32 {
        self.0.fields.reserved_count
    }
    /// `StationCargoList::TotalCount`: returns total count of cargo at the station, including
    /// cargo which is already reserved for loading.
    pub(crate) fn total_count(&self) -> u32 {
        self.0
            .fields
            .count
            .wrapping_add(self.0.fields.reserved_count)
    }
    /// `CargoList::PeriodsInTransit`.
    pub(crate) fn periods_in_transit(&self) -> u32 {
        self.0.periods_in_transit()
    }
    /// `CargoList::OnCleanPool`.
    pub(crate) fn on_clean_pool(&mut self) {
        self.0.on_clean_pool();
    }
    /// `StationCargoList::AreMergable`: are the two packets mergeable in the context of a list
    /// of `CargoPacket`s for a station?
    fn are_mergable(cp1: &CargoPacket, cp2: &CargoPacket) -> bool {
        cp1.source_xy == cp2.source_xy
            && cp1.periods_in_transit == cp2.periods_in_transit
            && cp1.first_station == cp2.first_station
            && cp1.get_source() == cp2.get_source()
    }
}
/// `(*cargo_per_source)[cp->first_station] += amount`.
fn add_amount(map: &mut Option<&mut StationCargoAmountMap>, cp: &CargoPacket, amount: u32) {
    let entry = map
        .as_mut()
        .unwrap()
        .entry(cp.get_first_station())
        .or_insert(0);
    *entry = entry.wrapping_add(amount);
}
/// `(*cargo_per_source)[cp->first_station] -= amount`.
fn sub_amount(map: &mut Option<&mut StationCargoAmountMap>, cp: &CargoPacket, amount: u32) {
    let entry = map
        .as_mut()
        .unwrap()
        .entry(cp.get_first_station())
        .or_insert(0);
    *entry = entry.wrapping_sub(amount);
}

/// The vehicle part whose cargo the two autoreplace bodies move: its list, cargo type and
/// capacity.
struct CargoPart {
    cargo: *mut VehicleCargoList,
    cargo_type: CargoType,
    cargo_cap: u16,
}
impl CargoPart {
    fn of(v: VehicleRef, s: &crate::newgrf_vehicle::Services) -> Self {
        Self {
            cargo: v.cargo(s),
            cargo_type: v.cargo_type(s),
            cargo_cap: v.cargo_cap(),
        }
    }
}
/// `CheckCargoCapacity`: check the capacity of all vehicles in a chain and spread cargo if
/// needed.
///
/// # Safety
/// The module's contract; `v` is a live chain with live cargo lists.
#[allow(
    unsafe_code,
    reason = "moves and deletes packets of the chain's live lists"
)]
pub(crate) unsafe fn check_cargo_capacity(
    v: Option<VehicleRef>,
    s: &crate::newgrf_vehicle::Services,
) {
    assert_with(|| v.is_none_or(|v| v.first() == v));
    let storage = s.storage();
    let mut src = v;
    while let Some(src_part) = src {
        let source = CargoPart::of(src_part, s);
        // SAFETY: The module's contract; each count read ends before the next call.
        assert_with(|| unsafe {
            (*source.cargo).total_count() == (*source.cargo).action_count(MoveToAction::MTA_KEEP)
        });
        // Do we need to more cargo away?
        if unsafe { (*source.cargo).total_count() } <= u32::from(source.cargo_cap) {
            src = src_part.next();
            continue;
        }
        // We need to move a particular amount. Try that on the other vehicles.
        let mut to_spread = unsafe { (*source.cargo).total_count() } - u32::from(source.cargo_cap);
        let mut dest = v;
        while let Some(dest_part) = dest
            && to_spread != 0
        {
            let destination = CargoPart::of(dest_part, s);
            assert_with(|| unsafe {
                (*destination.cargo).total_count()
                    == (*destination.cargo).action_count(MoveToAction::MTA_KEEP)
            });
            if unsafe { (*destination.cargo).total_count() } >= u32::from(destination.cargo_cap)
                || destination.cargo_type != source.cargo_type
            {
                dest = dest_part.next();
                continue;
            }
            let amount = to_spread.min(
                u32::from(destination.cargo_cap) - unsafe { (*destination.cargo).total_count() },
            );
            unsafe { VehicleCargoList::shift(source.cargo, storage, amount, destination.cargo) };
            to_spread -= amount;
            dest = dest_part.next();
        }
        // Any left-overs will be thrown away, but not their feeder share.
        if u32::from(source.cargo_cap) < unsafe { (*source.cargo).total_count() } {
            let amount = unsafe { (*source.cargo).total_count() } - u32::from(source.cargo_cap);
            unsafe { VehicleCargoList::truncate(source.cargo, storage, amount) };
        }
        src = src_part.next();
    }
}
/// `TransferCargo` up to its last statement: transfer cargo from a single (articulated) old
/// vehicle to the new vehicle chain. `part_of_chain` is true if `old_veh` is part of a chain
/// and `new_head` is the head of that chain. Returns whether the caller runs the original's
/// last statement, `ConsistChanged(CCF_LOADUNLOAD)` on `new_head`.
///
/// # Safety
/// The module's contract; both are live chains with live cargo lists.
#[allow(unsafe_code, reason = "moves packets between the chains' live lists")]
pub(crate) unsafe fn transfer_cargo(
    old_veh: VehicleRef,
    new_head: VehicleRef,
    part_of_chain: bool,
    s: &crate::newgrf_vehicle::Services,
) -> bool {
    assert_with(|| !part_of_chain || new_head.is_primary_vehicle());
    let storage = s.storage();
    let mut src = Some(old_veh);
    while let Some(src_part) = src {
        let source = CargoPart::of(src_part, s);
        // SAFETY: The module's contract; each count read ends before the next call.
        assert_with(|| unsafe {
            (*source.cargo).total_count() == (*source.cargo).action_count(MoveToAction::MTA_KEEP)
        });
        if !part_of_chain
            && src_part.vehicle_type() == crate::vehicle_type::VehicleType::TRAIN
            && src_part != old_veh
            && Some(src_part) != old_veh.other_multiheaded_part()
            && !src_part.is_articulated_part()
        {
            // Skip vehicles, which do not belong to old_veh.
            src = src_part.last_engine_part().next();
            continue;
        }
        if source.cargo_type >= CargoType::NUM_CARGO
            || unsafe { (*source.cargo).total_count() } == 0
        {
            src = src_part.next();
            continue;
        }
        // Find free space in the new chain.
        let mut dest = Some(new_head);
        while let Some(dest_part) = dest
            && unsafe { (*source.cargo).total_count() } > 0
        {
            let destination = CargoPart::of(dest_part, s);
            assert_with(|| unsafe {
                (*destination.cargo).total_count()
                    == (*destination.cargo).action_count(MoveToAction::MTA_KEEP)
            });
            if !part_of_chain
                && dest_part.vehicle_type() == crate::vehicle_type::VehicleType::TRAIN
                && dest_part != new_head
                && Some(dest_part) != new_head.other_multiheaded_part()
                && !dest_part.is_articulated_part()
            {
                // Skip vehicles, which do not belong to new_head.
                dest = dest_part.last_engine_part().next();
                continue;
            }
            if destination.cargo_type != source.cargo_type {
                dest = dest_part.next();
                continue;
            }
            // `dest->cargo_cap - dest->cargo.TotalCount()` is unsigned and wraps.
            let amount = unsafe { (*source.cargo).total_count() }.min(
                u32::from(destination.cargo_cap)
                    .wrapping_sub(unsafe { (*destination.cargo).total_count() }),
            );
            // `amount <= 0` on the unsigned amount.
            if amount == 0 {
                dest = dest_part.next();
                continue;
            }
            unsafe { VehicleCargoList::shift(source.cargo, storage, amount, destination.cargo) };
            dest = dest_part.next();
        }
        src = src_part.next();
    }
    // Update train weight etc., the old vehicle will be sold anyway.
    part_of_chain && new_head.vehicle_type() == crate::vehicle_type::VehicleType::TRAIN
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoSource" ["probe"] => Source {
        id,
        kind => type,
    };
    "OpenTTDCargoListFields" ["probe"] => Fields {
        cargo_periods_in_transit,
        feeder_share,
        count,
        reserved_count,
        action_counts,
    };
    "OpenTTDCargoStorageServices" ["probe"] => Services {
        can_allocate,
        cargo_delivery_state,
        create,
        destroy,
        flow_map,
        monitors,
        packet,
        packet_next,
        payment,
        payment_state,
        shared,
    };
}

/// `CargoPacket(...)`: a new record with the original's defaults for null, or a copy of `p`
/// for the split constructor `CargoPacket(count, feeder_share, original)`.
///
/// # Safety
/// A non-null `p` is a live record.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's record")]
pub unsafe extern "C" fn openttd_rust_cargo_packet_new(p: *const CargoPacket) -> *mut CargoPacket {
    Box::into_raw(Box::new(if p.is_null() {
        CargoPacket::default()
    } else {
        // SAFETY: The caller's live record.
        let cp = unsafe { *p };
        assert_with(|| cp.count != 0);
        cp
    }))
}
/// `~CargoPacket`: frees a record `openttd_rust_cargo_packet_new` made.
///
/// # Safety
/// `p` came from `openttd_rust_cargo_packet_new` and is freed once.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the caller's allocation"
)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_destroy(p: *mut CargoPacket) {
    // SAFETY: The caller's own allocation, freed once.
    drop(unsafe { Box::from_raw(p) });
}
/// `CargoPacket::UpdateLoadingTile`.
///
/// # Safety
/// `p` is a live record and `s` the live storage table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's record")]
pub unsafe extern "C" fn openttd_rust_cargo_packet_update_loading_tile(
    p: *mut CargoPacket,
    s: *const Services,
    tile: u32,
) {
    // SAFETY: The caller's live record and table.
    unsafe { (*p).update_loading_tile((*s).shared(), TileIndex(tile)) };
}
/// `CargoPacket::GetDistance`.
///
/// # Safety
/// `p` is a live record and `s` the live storage table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's record")]
pub unsafe extern "C" fn openttd_rust_cargo_packet_distance(
    p: *const CargoPacket,
    s: *const Services,
    tile: u32,
) -> u32 {
    // SAFETY: The caller's live record and table.
    unsafe { (*p).get_distance((*s).shared(), TileIndex(tile)) }
}
/// `CargoPacket::InvalidateAllFrom(Source)`.
///
/// # Safety
/// `s` is the live storage table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the native pool")]
pub unsafe extern "C" fn openttd_rust_cargo_invalidate(s: *const Services, id: u16, kind: u8) {
    // SAFETY: The caller's live table.
    unsafe { CargoPacket::invalidate_all_from_source(&*s, Source { id, kind }) };
}
/// `CargoList()`: a new list for a vehicle (`vehicle` 1) or a station.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol")]
pub extern "C" fn openttd_rust_cargo_list_new(vehicle: u8) -> *mut CargoList {
    Box::into_raw(Box::new(CargoList::new(vehicle != 0)))
}
/// `~CargoList`.
///
/// # Safety
/// `l` came from `openttd_rust_cargo_list_new` and is destroyed once; `s` is the live table.
#[unsafe(no_mangle)]
#[allow(
    unsafe_code,
    reason = "the exported symbol and the caller's allocation"
)]
pub unsafe extern "C" fn openttd_rust_cargo_list_destroy(l: *mut CargoList, s: *const Services) {
    // SAFETY: The caller's own list and live table.
    unsafe { CargoList::destroy(l, &*s) };
}
/// `CargoList::OnCleanPool`, and the clear of `ImportPackets`.
///
/// # Safety
/// `l` is a live list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_clear(l: *mut CargoList) {
    // SAFETY: The caller's live list.
    unsafe { (*l).on_clean_pool() };
}
/// `CargoList::Export`: the cached fields, for the save code.
///
/// # Safety
/// `l` is a live list and `out` a writable record.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_export(l: *const CargoList, out: *mut Fields) {
    // SAFETY: The caller's live list and output.
    unsafe { out.write((*l).fields) };
}
/// `CargoList::ImportMeta`: the cached fields, from the save code.
///
/// # Safety
/// `l` is a live list and `input` a live record.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_import(l: *mut CargoList, input: *const Fields) {
    // SAFETY: The caller's live list and input.
    unsafe { (*l).fields = *input };
}
// Count readers read only the cached fields they need. All arithmetic follows the original
// unsigned widths, including metadata loaded from saves.
/// `VehicleCargoList::TotalCount` and `StationCargoList::AvailableCount`.
///
/// # Safety
/// `l` is a live list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_count(l: *const CargoList) -> u32 {
    // SAFETY: The caller's live list.
    unsafe { (*l).fields.count }
}
/// `StationCargoList::ReservedCount`.
///
/// # Safety
/// `l` is a live station list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_reserved(l: *const CargoList) -> u32 {
    // SAFETY: The caller's live station list.
    unsafe { (*StationCargoList::of(l.cast_mut())).reserved_count() }
}
/// `StationCargoList::SetReservedCount`, for the save code.
///
/// # Safety
/// `l` is a live station list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_set_reserved(l: *mut CargoList, count: u32) {
    // SAFETY: The caller's live station list.
    unsafe { (*l).fields.reserved_count = count };
}
/// `StationCargoList::TotalCount`.
///
/// # Safety
/// `l` is a live station list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_station_total(l: *const CargoList) -> u32 {
    // SAFETY: The caller's live station list.
    unsafe { (*StationCargoList::of(l.cast_mut())).total_count() }
}
/// `CargoList::PeriodsInTransit`.
///
/// # Safety
/// `l` is a live list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_periods(l: *const CargoList) -> u32 {
    // SAFETY: The caller's live list.
    unsafe { (*l).periods_in_transit() }
}
/// `VehicleCargoList::GetFeederShare`.
///
/// # Safety
/// `l` is a live vehicle list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_feeder(l: *const CargoList) -> i64 {
    // SAFETY: The caller's live vehicle list.
    unsafe { (*VehicleCargoList::of(l.cast_mut())).get_feeder_share() }
}
/// `VehicleCargoList::ActionCount`.
///
/// # Safety
/// `l` is a live vehicle list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_action_count(
    l: *const CargoList,
    action: u8,
) -> u32 {
    // SAFETY: The caller's live vehicle list.
    unsafe { (*VehicleCargoList::of(l.cast_mut())).action_count(MoveToAction(action)) }
}
/// `VehicleCargoList::StoredCount`.
///
/// # Safety
/// `l` is a live vehicle list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_stored(l: *const CargoList) -> u32 {
    // SAFETY: The caller's live vehicle list.
    unsafe { (*VehicleCargoList::of(l.cast_mut())).stored_count() }
}
/// `StationCargoList::MapSize`: the number of the `MultiMap`'s keys.
///
/// # Safety
/// `l` is a live station list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_key_count(l: *const CargoList) -> usize {
    // SAFETY: The caller's live station list.
    unsafe { (*l).keys.len() }
}
/// `CargoList::Packets`: each packet to `output` in list order with its key, after each key
/// whose packet list is empty with a null packet.
///
/// # Safety
/// `l` is a live list; `output` writes only `context`.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_snapshot(
    l: *const CargoList,
    context: *mut c_void,
    output: extern "C" fn(*mut c_void, u16, *mut c_void),
) {
    // SAFETY: The caller's live list, which `output` does not change.
    let l = unsafe { &*l };
    let empty: Vec<StationID> = l
        .keys
        .iter()
        .copied()
        .filter(|key| !l.destinations.contains_key(key))
        .collect();
    for key in empty {
        output(context, key.0, std::ptr::null_mut());
    }
    let mut it = l.front();
    while let Some(current) = it {
        it = l.after(current.token);
        output(context, current.key.0, current.shell);
    }
}
/// `CargoList::ImportPackets`' insertion: `packets.push_back(cp)` for a vehicle list;
/// `packets.Insert(key, cp)` for a station list, or the key alone for a null `shell`. The shell
/// is not dereferenced.
///
/// # Safety
/// `l` is a live list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_insert(
    l: *mut CargoList,
    key: u16,
    shell: *mut c_void,
) {
    // SAFETY: The caller's live list.
    match unsafe { CargoList::typed(l) } {
        Typed::Vehicle(l) => unsafe { (*l).push_back(shell) },
        Typed::Station(l) if shell.is_null() => unsafe {
            (*l).0.keys.insert(StationID(key));
        },
        Typed::Station(l) => unsafe { (*l).insert(StationID(key), shell) },
    }
}
/// `CargoList::InvalidateCache`.
///
/// # Safety
/// `l` is a live list and `s` the live table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_rebuild(l: *mut CargoList, s: *const Services) {
    // SAFETY: The caller's live list and table.
    match unsafe { CargoList::typed(l) } {
        Typed::Vehicle(l) => unsafe { VehicleCargoList::invalidate_cache(l, &*s) },
        Typed::Station(l) => unsafe { StationCargoList::invalidate_cache(l, &*s) },
    }
}
/// `VehicleCargoList::GetFirstStation` and `StationCargoList::GetFirstStation`.
///
/// # Safety
/// `l` is a live list and `s` the live table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_first(
    l: *const CargoList,
    s: *const Services,
) -> u16 {
    // SAFETY: The caller's live list and table.
    match unsafe { CargoList::typed(l.cast_mut()) } {
        Typed::Vehicle(l) => unsafe { (*l).get_first_station(&*s) }.0,
        Typed::Station(l) => unsafe { (*l).get_first_station(&*s) }.0,
    }
}
/// `VehicleCargoList::KeepAll`.
///
/// # Safety
/// `l` is a live vehicle list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_cargo_list_keep(l: *mut CargoList) {
    // SAFETY: The caller's live vehicle list.
    unsafe { (*VehicleCargoList::of(l)).keep_all() };
}
/// `VehicleCargoList::Append`.
///
/// # Safety
/// `l` is a live vehicle list, `s` the live table and `cp` a live packet in no list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_vehicle_cargo_list_append(
    l: *mut CargoList,
    s: *const Services,
    cp: *mut c_void,
    action: u8,
) {
    // SAFETY: The caller's live list, table and packet.
    unsafe { VehicleCargoList::append(VehicleCargoList::of(l), &*s, cp, MoveToAction(action)) };
}
/// `StationCargoList::Append`.
///
/// # Safety
/// `l` is a live station list, `s` the live table and `cp` a live packet in no list.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_station_cargo_list_append(
    l: *mut CargoList,
    s: *const Services,
    cp: *mut c_void,
    next: u16,
) {
    // SAFETY: The caller's live list, table and packet.
    unsafe { StationCargoList::append(StationCargoList::of(l), &*s, cp, StationID(next)) };
}
/// `VehicleCargoList::Truncate`.
///
/// # Safety
/// `l` is a live vehicle list and `s` the live table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_vehicle_cargo_list_truncate(
    l: *mut CargoList,
    s: *const Services,
    max_move: u32,
) -> u32 {
    // SAFETY: The caller's live list and table.
    unsafe { VehicleCargoList::truncate(VehicleCargoList::of(l), &*s, max_move) }
}
/// `StationCargoList::Truncate(max_move)`, counting no source.
///
/// # Safety
/// `l` is a live station list and `s` the live table.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's list")]
pub unsafe extern "C" fn openttd_rust_station_cargo_list_truncate(
    l: *mut CargoList,
    s: *const Services,
    max_move: u32,
) -> u32 {
    // SAFETY: The caller's live list and table.
    unsafe { StationCargoList::truncate(StationCargoList::of(l), &*s, max_move, None) }
}
/// `StationCargoList::Reserve`, for the orders console.
///
/// # Safety
/// `l` is a live station list, `dest` a live vehicle list, `s` the live table and `next`
/// `length` live station IDs.
#[unsafe(no_mangle)]
#[allow(unsafe_code, reason = "the exported symbol and the caller's lists")]
pub unsafe extern "C" fn openttd_rust_station_cargo_list_reserve(
    l: *mut CargoList,
    s: *const Services,
    max_move: u32,
    dest: *mut CargoList,
    next: *const u16,
    length: usize,
    tile: u32,
) -> u32 {
    // SAFETY: The caller's live lists, table and station IDs, which no callback writes;
    // `StationID` is a transparent `u16`.
    unsafe {
        let next = if length == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(next.cast::<StationID>(), length)
        };
        StationCargoList::reserve(
            StationCargoList::of(l),
            &*s,
            max_move,
            VehicleCargoList::of(dest),
            next,
            TileIndex(tile),
        )
    }
}
