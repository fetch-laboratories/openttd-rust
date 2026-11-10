/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Cargo packet fields, ordered lists, metadata and cargo actions; pool allocation and
//! identity stay native. Payment can call getters, so every raw owner access is short. Save
//! import stores unresolved shell handles without dereferencing them; the pointer contract is
//! in `cargo_storage_ffi.h`. Packet field addresses stay stable until destruction; native
//! aliases and Rust accesses are serialized by the game-state mutex. Map reads use the shared
//! game-loop/command interval: these synchronous coordinate operations cannot reset or yield.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
use crate::cargopacket::{CargoPacket, MoveToAction};
use crate::services_assert::check;
use crate::vehicle::VehicleRef;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;

/// Native cargo Source, passed by value without packing its ID and type.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Source {
    pub(crate) id: u16,
    pub(crate) source_type: u8,
}
crate::abi::shared_records! {
    fn abi_record;
    /// Canonical packet allocation, shared in place with native readers and CAPA.
    #[derive(Clone, Copy)]
    "OpenTTDCargoPacket" ["cargo_storage"] => pub struct Packet {
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
impl Default for Packet {
    fn default() -> Self {
        Self {
            feeder_share: 0,
            source_xy: crate::tile_type::INVALID_TILE,
            count: 0,
            periods_in_transit: 0,
            first_station: INVALID,
            next_hop: INVALID,
            source_id: INVALID,
            travelled_x: 0,
            travelled_y: 0,
            source_type: 0,
            in_vehicle: 0,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Fields {
    pub(crate) cargo_periods_in_transit: u64,
    pub(crate) feeder_share: i64,
    pub(crate) count: u32,
    pub(crate) reserved_count: u32,
    pub(crate) action_counts: [u32; 4],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Services {
    pub(crate) can_allocate: extern "C" fn() -> u8,
    pub(crate) create: extern "C" fn(*const Packet) -> *mut c_void,
    pub(crate) packet: extern "C" fn(*mut c_void) -> *mut Packet,
    pub(crate) destroy: extern "C" fn(*mut c_void),
    pub(crate) pay: extern "C" fn(*mut c_void, u8, u8, *const c_void, u32, u32) -> i64,
    pub(crate) origin: extern "C" fn(*mut c_void, u16, u32, u8),
    pub(crate) flow_map: extern "C" fn(*const c_void) -> *mut crate::cargo_flow::FlowMap,
    pub(crate) shared: *const crate::services::Services,
    pub(crate) packet_next: extern "C" fn(*mut c_void) -> *mut c_void,
}
impl Services {
    fn random_draw(&self) -> u32 {
        unsafe { &*self.shared }.random()
    }
}
#[derive(Clone, Copy)]
struct Node {
    token: usize,
    key: u16,
    shell: *mut c_void,
    previous: Option<usize>,
    next: Option<usize>,
}
#[derive(Clone, Copy)]
struct Ends {
    first: usize,
    last: usize,
}
pub struct List {
    pub(crate) fields: Fields,
    // Slots never move relative to their indices. Reuse happens only after erase,
    // when no source iterator can still refer to that node. Traversal follows
    // links, never arena allocation order; same-list insertion keeps existing
    // iterators valid even if growing the arena reallocates its backing storage.
    nodes: Vec<Option<Node>>,
    free: Vec<usize>,
    first: Option<usize>,
    last: Option<usize>,
    destinations: BTreeMap<u16, Ends>,
    keys: BTreeSet<u16>,
    vehicle: bool,
}
// Native next-hop spans can alias C++ storage. Read individual uint16_t fields;
// never build a Rust slice/reference that would survive a reentering callback.
#[derive(Clone, Copy)]
struct NextHops {
    data: *const u16,
    length: usize,
}
impl NextHops {
    unsafe fn at(self, index: usize) -> u16 {
        unsafe { self.data.add(index).read() }
    }
    unsafe fn contains(self, key: u16) -> bool {
        (0..self.length).any(|index| unsafe { self.at(index) } == key)
    }
}
const INVALID: u16 = u16::MAX;

pub(crate) fn share(p: Packet, amount: u32) -> i64 {
    p.feeder_share.saturating_mul(i64::from(amount as i32)) / i64::from(i32::from(p.count))
}
unsafe fn read(p: *const Packet) -> Packet {
    unsafe { p.read() }
}
unsafe fn packet(s: Services, shell: *mut c_void) -> Packet {
    unsafe { read((s.packet)(shell)) }
}
unsafe fn split(p: *mut Packet, s: Services, amount: u32) -> *mut c_void {
    if (s.can_allocate)() == 0 {
        return std::ptr::null_mut();
    }
    let original = unsafe { read(p) };
    let fs = share(original, amount);
    let mut part = original;
    part.count = amount as u16;
    part.feeder_share = fs;
    let shell = (s.create)(&raw const part);
    unsafe {
        (*p).feeder_share = original.feeder_share.saturating_sub(fs);
        (*p).count = original.count.wrapping_sub(amount as u16);
    }
    shell
}
unsafe fn reduce(p: *mut Packet, amount: u32) {
    let old = unsafe { read(p) };
    if amount >= u32::from(old.count) {
        check(false);
    }
    unsafe {
        (*p).feeder_share = old.feeder_share.saturating_sub(share(old, amount));
        (*p).count = old.count.wrapping_sub(amount as u16);
    }
}
unsafe fn tile(p: *mut Packet, s: Services, at: u32, loading: bool) {
    // Delay setting the origin until first pickup: creation at a station does
    // not yet tell us the tile at which the cargo will be picked up.
    unsafe {
        if loading {
            if (*p).source_xy == u32::MAX {
                (*p).source_xy = at;
            }
            if (*p).in_vehicle != 0 {
                check(false);
            }
            (*p).in_vehicle = 1;
        } else {
            if (*p).in_vehicle == 0 {
                check(false);
            }
            (*p).in_vehicle = 0;
        }
        // SAFETY: the synchronous service table keeps the shared map view live;
        // these coordinate reads neither call back nor yield.
        let world = crate::map_func::World::new(&*s.shared);
        let x = world.dimensions.tile_x(at) as i16;
        let y = world.dimensions.tile_y(at) as i16;
        if loading {
            // Add the known loading tile now; subtract the unloading tile later
            // to obtain the vector of travel inside a vehicle.
            (*p).travelled_x = (*p).travelled_x.wrapping_add(x);
            (*p).travelled_y = (*p).travelled_y.wrapping_add(y);
        } else {
            (*p).travelled_x = (*p).travelled_x.wrapping_sub(x);
            (*p).travelled_y = (*p).travelled_y.wrapping_sub(y);
        }
    }
}
unsafe fn fields(l: *const List) -> Fields {
    unsafe { (*l).fields }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoSource" ["probe"] => Source {
        id,
        source_type => type,
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
        create,
        packet,
        destroy,
        pay,
        origin,
        flow_map,
        shared,
        packet_next,
    };
    "OpenTTDCargoCapacityVehicle" ["probe"] => Vehicle {
        list,
        cargo,
    };
    "OpenTTDCargoCapacityServices" ["probe"] => CapacityServices {
        read,
        cargo,
    };
}

#[cfg(test)]
unsafe fn nodes(l: *const List) -> Vec<Node> {
    let mut result = Vec::new();
    let mut node = unsafe { first(l) };
    while let Some(current) = node {
        result.push(current);
        node = unsafe { after(l, current.token) };
    }
    result
}
unsafe fn first(l: *const List) -> Option<Node> {
    unsafe { (*l).first.map(|token| (&(*l).nodes)[token].unwrap()) }
}
unsafe fn last(l: *const List) -> Option<Node> {
    unsafe { (*l).last.map(|token| (&(*l).nodes)[token].unwrap()) }
}
unsafe fn after(l: *const List, token: usize) -> Option<Node> {
    unsafe {
        (&(*l).nodes)[token]
            .unwrap()
            .next
            .map(|token| (&(*l).nodes)[token].unwrap())
    }
}
unsafe fn before(l: *const List, token: usize) -> Option<Node> {
    unsafe {
        (&(*l).nodes)[token]
            .unwrap()
            .previous
            .map(|token| (&(*l).nodes)[token].unwrap())
    }
}
unsafe fn destination(l: *const List, key: u16, reverse: bool) -> Option<Node> {
    unsafe {
        (*l).destinations
            .get(&key)
            .map(|ends| (&(*l).nodes)[if reverse { ends.last } else { ends.first }].unwrap())
    }
}
unsafe fn erase(l: *mut List, token: usize) {
    unsafe {
        let l = &mut *l;
        let removed = l.nodes[token].take().unwrap();
        if let Some(previous) = removed.previous {
            l.nodes[previous].as_mut().unwrap().next = removed.next;
        } else {
            l.first = removed.next;
        }
        if let Some(next) = removed.next {
            l.nodes[next].as_mut().unwrap().previous = removed.previous;
        } else {
            l.last = removed.previous;
        }
        if !l.vehicle {
            let ends = l.destinations.get_mut(&removed.key).unwrap();
            if ends.first == token && ends.last == token {
                l.destinations.remove(&removed.key);
                l.keys.remove(&removed.key);
            } else {
                if ends.first == token {
                    ends.first = removed.next.unwrap();
                }
                if ends.last == token {
                    ends.last = removed.previous.unwrap();
                }
            }
        }
        l.free.push(token);
    }
}
unsafe fn insert(l: *mut List, shell: *mut c_void, key: u16, before: Option<usize>, front: bool) {
    unsafe {
        let l = &mut *l;
        let next = if before.is_some() {
            before
        } else if l.vehicle {
            if front { l.first } else { None }
        } else if let Some(ends) = l.destinations.get(&key) {
            l.nodes[ends.last].unwrap().next
        } else {
            l.destinations
                .range(key..)
                .next()
                .map(|(_, ends)| ends.first)
        };
        let previous = next.map_or(l.last, |token| l.nodes[token].unwrap().previous);
        let token = l.free.pop().unwrap_or_else(|| {
            l.nodes.push(None);
            l.nodes.len() - 1
        });
        let n = Node {
            token,
            key,
            shell,
            previous,
            next,
        };
        l.nodes[token] = Some(n);
        if let Some(previous) = previous {
            l.nodes[previous].as_mut().unwrap().next = Some(token);
        } else {
            l.first = Some(token);
        }
        if let Some(next) = next {
            l.nodes[next].as_mut().unwrap().previous = Some(token);
        } else {
            l.last = Some(token);
        }
        if !l.vehicle {
            l.keys.insert(key);
            l.destinations
                .entry(key)
                .and_modify(|ends| ends.last = token)
                .or_insert(Ends {
                    first: token,
                    last: token,
                });
        }
    }
}
unsafe fn add_cache(l: *mut List, p: Packet, action: Option<usize>) {
    unsafe {
        let f = &mut (*l).fields;
        f.count = f.count.wrapping_add(u32::from(p.count));
        f.cargo_periods_in_transit = f
            .cargo_periods_in_transit
            .wrapping_add(u64::from(p.periods_in_transit) * u64::from(p.count));
        if (*l).vehicle {
            f.feeder_share = f.feeder_share.saturating_add(p.feeder_share);
        }
        if let Some(a) = action {
            f.action_counts[a] = f.action_counts[a].wrapping_add(u32::from(p.count));
        }
    }
}
unsafe fn remove_cache(l: *mut List, p: Packet, amount: u32, action: Option<usize>) {
    unsafe {
        let f = &mut (*l).fields;
        f.count = f.count.wrapping_sub(amount);
        f.cargo_periods_in_transit = f
            .cargo_periods_in_transit
            .wrapping_sub(u64::from(p.periods_in_transit) * u64::from(amount));
        if (*l).vehicle {
            f.feeder_share = f.feeder_share.saturating_sub(share(p, amount));
        }
        if let Some(a) = action {
            f.action_counts[a] = f.action_counts[a].wrapping_sub(amount);
        }
    }
}
/// Checks whether the second packet can merge into the first, including `MAX_COUNT`.
fn mergeable(a: Packet, b: Packet) -> bool {
    a.source_xy == b.source_xy
        && a.periods_in_transit == b.periods_in_transit
        && a.first_station == b.first_station
        && a.source_id == b.source_id
        && a.source_type == b.source_type
        && u32::from(a.count) + u32::from(b.count) <= u32::from(CargoPacket::MAX_COUNT)
}
unsafe fn merge(p: *mut Packet, s: Services, other: *mut c_void) {
    let b = unsafe { packet(s, other) };
    unsafe {
        (*p).count = (*p).count.wrapping_add(b.count);
        (*p).feeder_share = (*p).feeder_share.saturating_add(b.feeder_share);
    }
    (s.destroy)(other);
}
/// Appends a packet, trying to merge it with another in its range first.
/// Vehicle packets may be appended for keeping or loading; reserved packets must
/// be loaded before adding directly to the keep range. Station packets join the
/// range with the same next station. After appending, the packet may no longer exist.
unsafe fn append(l: *mut List, s: Services, shell: *mut c_void, key: u16) {
    let p = unsafe { packet(s, shell) };
    let vehicle = unsafe { (*l).vehicle };
    unsafe {
        add_cache(
            l,
            p,
            if vehicle {
                Some(usize::from(key))
            } else {
                None
            },
        );
    }
    if vehicle && unsafe { fields(l).count } == u32::from(p.count) {
        unsafe {
            insert(l, shell, key, None, false);
        }
        return;
    }
    let mut sum = u32::from(p.count);
    let mut node = if vehicle {
        unsafe { last(l) }
    } else {
        unsafe { destination(l, key, true) }
    };
    while let Some(n) = node {
        let existing = unsafe { packet(s, n.shell) };
        if mergeable(existing, p) {
            unsafe {
                merge((s.packet)(n.shell), s, shell);
            }
            return;
        }
        if vehicle {
            sum = sum.wrapping_add(u32::from(existing.count));
            if sum >= unsafe { fields(l).action_counts[usize::from(key)] } {
                break;
            }
        }
        node = unsafe { before(l, n.token) };
        if !vehicle && node.is_some_and(|n| n.key != key) {
            break;
        }
    }
    // The packet could not be merged with another one.
    unsafe {
        insert(l, shell, key, None, false);
    }
}
unsafe fn forced_flow(
    s: Services,
    ge: *const c_void,
    origin: u16,
    station: u16,
    next: NextHops,
) -> u16 {
    use crate::cargo_flow::{
        flow_via, openttd_rust_flow_change, openttd_rust_flow_clone, openttd_rust_flow_destroy,
        openttd_rust_flow_read,
    };
    let map = (s.flow_map)(ge);
    let original = if map.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { crate::cargo_flow::openttd_rust_flow_map_at(map, u32::from(origin), 1).flow }
    };
    if original.is_null() {
        return INVALID;
    }
    let flow = unsafe { openttd_rust_flow_clone(original) };
    unsafe {
        openttd_rust_flow_change(flow, 0, station, i32::MIN as u32);
    }
    for index in (0..next.length).rev() {
        let station = unsafe { next.at(index) };
        if unsafe { openttd_rust_flow_read(flow, 2, 0) } == 0 {
            break;
        }
        unsafe {
            openttd_rust_flow_change(flow, 0, station, i32::MIN as u32);
        }
    }
    let mut restricted = 0;
    let via = if unsafe { openttd_rust_flow_read(flow, 2, 0) } == 0 {
        INVALID
    } else {
        unsafe { flow_via(flow, 0, 0, 0, &raw mut restricted, || s.random_draw()) }
    };
    unsafe {
        openttd_rust_flow_destroy(flow);
    }
    via
}
/// Chooses the action to perform with this packet at the current and next stations.
fn choose(p: Packet, via: u16, station: u16, accepted: bool, next: NextHops) -> MoveToAction {
    if via == INVALID {
        if accepted && p.first_station != station {
            MoveToAction::MTA_DELIVER
        } else {
            MoveToAction::MTA_KEEP
        }
    } else if via == station {
        MoveToAction::MTA_DELIVER
    // SAFETY: the caller supplies a live next-hop span for this synchronous choice.
    } else if unsafe { next.contains(via) } {
        MoveToAction::MTA_KEEP
    } else {
        MoveToAction::MTA_TRANSFER
    }
}
/// Moves cargo between adjacent designations without changing the packets.
/// Moving from `MTA_DELIVER` to `MTA_TRANSFER` also updates the next station.
pub(crate) unsafe fn reassign(
    list: *mut List,
    services: Services,
    from: MoveToAction,
    to: MoveToAction,
    amount: u32,
) -> u32 {
    let meta = unsafe { fields(list) };
    let amount = amount.min(meta.action_counts[from.0 as usize]);
    if from == MoveToAction::MTA_DELIVER && to == MoveToAction::MTA_TRANSFER {
        let mut sum = 0_u32;
        let mut node = unsafe { first(list) };
        while sum < meta.action_counts[MoveToAction::MTA_TRANSFER.0 as usize].wrapping_add(amount) {
            let current = node.unwrap();
            let next = unsafe { after(list, current.token) };
            let owner = (services.packet)(current.shell);
            sum = sum.wrapping_add(u32::from(unsafe { read(owner) }.count));
            if sum > meta.action_counts[MoveToAction::MTA_TRANSFER.0 as usize] {
                if sum
                    > meta.action_counts[MoveToAction::MTA_TRANSFER.0 as usize].wrapping_add(amount)
                {
                    // Historical C++ expression adds max_move (rather than subtracts).
                    let part = unsafe {
                        split(
                            owner,
                            services,
                            sum.wrapping_sub(
                                meta.action_counts[MoveToAction::MTA_TRANSFER.0 as usize],
                            )
                            .wrapping_add(amount),
                        )
                    };
                    sum = sum.wrapping_sub(u32::from(unsafe { packet(services, part) }.count));
                    unsafe {
                        insert(list, part, 0, next.map(|node| node.token), false);
                    }
                }
                unsafe {
                    (*owner).next_hop = INVALID;
                }
            }
            node = next;
        }
    }
    unsafe {
        (*list).fields.action_counts[from.0 as usize] =
            (*list).fields.action_counts[from.0 as usize].wrapping_sub(amount);
        (*list).fields.action_counts[to.0 as usize] =
            (*list).fields.action_counts[to.0 as usize].wrapping_add(amount);
    }
    amount
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_new(p: *const Packet) -> *mut Packet {
    Box::into_raw(Box::new(if p.is_null() {
        Packet::default()
    } else {
        // SAFETY: a non-null input points to a live canonical packet for this copy.
        let packet = unsafe { read(p) };
        // The copy is the split constructor `CargoPacket(count, feeder_share, original)`.
        if packet.count == 0 {
            check(false);
        }
        packet
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_destroy(p: *mut Packet) {
    unsafe {
        drop(Box::from_raw(p));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_tile(
    p: *mut Packet,
    s: *const Services,
    at: u32,
    loading: u8,
) {
    unsafe {
        tile(p, *s, at, loading != 0);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_feeder(p: *mut Packet, share: i64) {
    unsafe {
        (*p).feeder_share = (*p).feeder_share.saturating_add(share);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_share(p: *const Packet, amount: u32) -> i64 {
    share(unsafe { read(p) }, amount)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_distance(
    p: *const Packet,
    s: *const Services,
    at: u32,
) -> u32 {
    let p = unsafe { read(p) };
    if p.source_xy == crate::tile_type::INVALID_TILE {
        check(false);
    }
    if p.in_vehicle == 0 {
        check(false);
    }
    let s = unsafe { *s };
    // SAFETY: the synchronous service table keeps the shared map view live;
    // these coordinate reads neither call back nor yield.
    let world = crate::map_func::World::new(unsafe { &*s.shared });
    let x = world.dimensions.tile_x(at);
    let y = world.dimensions.tile_y(at);
    // Distance is requested while the cargo is still inside the vehicle.
    // Finish the travel vector locally without changing the packet.
    let dx = p.travelled_x.wrapping_sub(x as i16);
    let dy = p.travelled_y.wrapping_sub(y as i16);
    let travelled = i32::from(dx).unsigned_abs() + i32::from(dy).unsigned_abs();
    // Free movement within a station must not increase payment. Cap vehicle
    // travel at the direct source-to-destination distance to avoid overpayment
    // on routes with large transfer stations or long detours.
    let direct = world.dimensions.distance_manhattan(p.source_xy, at);
    travelled.min(direct)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_merge(
    p: *mut Packet,
    s: *const Services,
    other: *mut c_void,
) {
    unsafe {
        merge(p, *s, other);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_list_new(vehicle: u8) -> *mut List {
    Box::into_raw(Box::new(List {
        fields: Fields::default(),
        nodes: Vec::new(),
        free: Vec::new(),
        first: None,
        last: None,
        destinations: BTreeMap::new(),
        keys: BTreeSet::new(),
        vehicle: vehicle != 0,
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_destroy(l: *mut List, s: *const Services) {
    let s = unsafe { *s };
    let mut node = unsafe { first(l) };
    while let Some(current) = node {
        node = unsafe { after(l, current.token) };
        (s.destroy)(current.shell);
    }
    unsafe {
        drop(Box::from_raw(l));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_clear(l: *mut List) {
    unsafe {
        (*l).nodes.clear();
        (*l).free.clear();
        (*l).first = None;
        (*l).last = None;
        (*l).destinations.clear();
        (*l).keys.clear();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_export(l: *const List, out: *mut Fields) {
    unsafe {
        out.write(fields(l));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_import(l: *mut List, input: *const Fields) {
    unsafe {
        (*l).fields = *input;
    }
}
// Scalar queries read only the canonical fields they need. All arithmetic
// follows the original unsigned C++ widths, including metadata loaded from saves.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_count(l: *const List) -> u32 {
    unsafe { (*l).fields.count }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_reserved(l: *const List) -> u32 {
    unsafe { (*l).fields.reserved_count }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_set_reserved(l: *mut List, count: u32) {
    unsafe {
        (*l).fields.reserved_count = count;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_station_total(l: *const List) -> u32 {
    unsafe { (*l).fields.count.wrapping_add((*l).fields.reserved_count) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_periods(l: *const List) -> u32 {
    let count = unsafe { (*l).fields.count };
    if count == 0 {
        0
    } else {
        (unsafe { (*l).fields.cargo_periods_in_transit } / u64::from(count)) as u32
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_feeder(l: *const List) -> i64 {
    unsafe { (*l).fields.feeder_share }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_action_count(l: *const List, action: u8) -> u32 {
    unsafe {
        (&raw const (*l).fields.action_counts)
            .cast::<u32>()
            .add(usize::from(action))
            .read()
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_stored(l: *const List) -> u32 {
    unsafe {
        (*l).fields
            .count
            .wrapping_sub((*l).fields.action_counts[MoveToAction::MTA_LOAD.0 as usize])
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_unload(l: *const List) -> u32 {
    unsafe {
        (*l).fields.action_counts[MoveToAction::MTA_TRANSFER.0 as usize]
            .wrapping_add((*l).fields.action_counts[MoveToAction::MTA_DELIVER.0 as usize])
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_remaining(l: *const List) -> u32 {
    unsafe {
        (*l).fields.action_counts[MoveToAction::MTA_KEEP.0 as usize]
            .wrapping_add((*l).fields.action_counts[MoveToAction::MTA_LOAD.0 as usize])
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_key_count(l: *const List) -> usize {
    unsafe { (*l).keys.len() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_snapshot(
    l: *const List,
    context: *mut c_void,
    output: extern "C" fn(*mut c_void, u16, *mut c_void),
) {
    let empty: Vec<u16> = unsafe {
        (*l).keys
            .iter()
            .copied()
            .filter(|key| !(*l).destinations.contains_key(key))
            .collect()
    };
    for key in empty {
        output(context, key, std::ptr::null_mut());
    }
    let mut node = unsafe { first(l) };
    while let Some(current) = node {
        node = unsafe { after(l, current.token) };
        output(context, current.key, current.shell);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_insert(
    l: *mut List,
    key: u16,
    shell: *mut c_void,
) {
    unsafe {
        if shell.is_null() {
            (*l).keys.insert(key);
        } else {
            insert(l, shell, key, None, false);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_rebuild(l: *mut List, s: *const Services) {
    let s = unsafe { *s };
    unsafe {
        (*l).fields.count = 0;
        (*l).fields.cargo_periods_in_transit = 0;
        (*l).fields.feeder_share = 0;
    }
    let mut node = unsafe { first(l) };
    while let Some(current) = node {
        node = unsafe { after(l, current.token) };
        unsafe {
            add_cache(l, packet(s, current.shell), None);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_append(
    l: *mut List,
    s: *const Services,
    shell: *mut c_void,
    key: u16,
) {
    unsafe {
        append(l, *s, shell, key);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_first(l: *const List, s: *const Services) -> u16 {
    if unsafe { fields(l) }.count == 0 {
        INVALID
    } else {
        unsafe { packet(*s, first(l).unwrap().shell) }.first_station
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_has(
    l: *const List,
    next: *const u16,
    length: usize,
) -> u8 {
    let next = NextHops { data: next, length };
    let keys = unsafe { &(*l).keys };
    u8::from(
        (0..length).any(|index| keys.contains(&unsafe { next.at(index) }))
            || keys.contains(&INVALID),
    )
}

/// Scheduler accounting includes every packet lookup, erased for ordinary callers.
pub(crate) unsafe fn age<const COUNTED: bool>(l: *mut List, s: *const Services) {
    let s = unsafe { *s };
    let mut node = unsafe { first(l) };
    while let Some(current) = node {
        node = unsafe { after(l, current.token) };
        let p = (s.packet)(current.shell);
        if COUNTED {
            crate::witness::hit(crate::witness::VEHICLE_CROSSINGS);
            crate::witness::hit(crate::witness::VEHICLE_PACKET_LOOKUPS);
            crate::witness::add(
                crate::witness::VEHICLE_OBSERVED_BYTES,
                std::mem::size_of_val(&p) as u64,
            );
        }
        let old = unsafe { read(p) };
        // If we're at the maximum, then we can't increase no more.
        if old.periods_in_transit == u16::MAX {
            continue;
        }
        unsafe {
            (*p).periods_in_transit += 1;
            (*l).fields.cargo_periods_in_transit = (*l)
                .fields
                .cargo_periods_in_transit
                .wrapping_add(u64::from(old.count));
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_keep(l: *mut List) {
    unsafe {
        (*l).fields.action_counts = [0, 0, (*l).fields.count, 0];
    }
}

/// Stages cargo for unloading, sorting transferred, delivered and kept packets
/// into consecutive chunks and updating the counts to reflect their sizes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_stage(
    l: *mut List,
    s: *const Services,
    accepted: u8,
    station: u16,
    next: *const u16,
    length: usize,
    unload: u8,
    ge: *const c_void,
    cargo: u8,
    payment: *mut c_void,
    at: u32,
) -> u8 {
    let s = unsafe { *s };
    let next_slice = NextHops { data: next, length };
    unsafe {
        (*l).fields.action_counts[MoveToAction::MTA_TRANSFER.0 as usize] = 0;
        (*l).fields.action_counts[MoveToAction::MTA_DELIVER.0 as usize] = 0;
        (*l).fields.action_counts[MoveToAction::MTA_KEEP.0 as usize] = 0;
    }
    let mut deliver = None;
    let mut n = unsafe { first(l) };
    let mut sum = 0_u32;
    while sum < unsafe { fields(l) }.count {
        let current = n.unwrap();
        n = unsafe { after(l, current.token) };
        unsafe {
            erase(l, current.token);
        }
        let p = (s.packet)(current.shell);
        let mut via = INVALID;
        let mut restricted = 0;
        let mut observed = unsafe { read(p) };
        let mut action;
        if unload == 4 {
            action = MoveToAction::MTA_KEEP;
        } else if unload == 1 && accepted != 0 && observed.first_station != station {
            action = MoveToAction::MTA_DELIVER;
        } else if unload == 1 || unload == 2 {
            action = MoveToAction::MTA_TRANSFER;
            // We cannot send the cargo to any possible next hop or the current station.
            via = unsafe { forced_flow(s, ge, observed.first_station, station, next_slice) };
        } else {
            let map = (s.flow_map)(ge);
            // Rewrite an invalid source station to some random other one to avoid
            // keeping the cargo in the vehicle forever.
            if observed.first_station == INVALID && !map.is_null() {
                let first = unsafe { crate::cargo_flow::openttd_rust_flow_map_at(map, 0, 0) };
                if first.found != 0 {
                    observed.first_station = first.origin;
                    unsafe {
                        (*p).first_station = observed.first_station;
                    }
                }
            }
            let flow = if map.is_null() {
                std::ptr::null_mut()
            } else {
                unsafe {
                    crate::cargo_flow::openttd_rust_flow_map_at(
                        map,
                        u32::from(observed.first_station),
                        1,
                    )
                    .flow
                }
            };
            via = if flow.is_null() {
                INVALID
            } else {
                unsafe {
                    crate::cargo_flow::flow_via(flow, 1, 0, 0, &raw mut restricted, || {
                        s.random_draw()
                    })
                }
            };
            action = choose(observed, via, station, accepted != 0, next_slice);
            if restricted != 0 && action == MoveToAction::MTA_TRANSFER {
                // If the flow is restricted we can't transfer to it. Choose an
                // unrestricted one instead.
                via = unsafe {
                    crate::cargo_flow::flow_via(flow, 0, 0, 0, &raw mut restricted, || {
                        s.random_draw()
                    })
                };
                action = choose(observed, via, station, accepted != 0, next_slice);
            }
        }
        match action {
            MoveToAction::MTA_KEEP => {
                unsafe {
                    insert(l, current.shell, 0, None, false);
                }
                if deliver.is_none() {
                    deliver = unsafe { last(l).map(|n| n.token) };
                }
            }
            MoveToAction::MTA_DELIVER => unsafe {
                insert(l, current.shell, 0, deliver, false);
            },
            MoveToAction::MTA_TRANSFER => {
                unsafe {
                    insert(l, current.shell, 0, None, true);
                }
                // Add feeder share here to allow reusing the field for next station.
                let credit = (s.pay)(
                    payment,
                    0,
                    cargo,
                    current.shell,
                    u32::from(observed.count),
                    at,
                );
                unsafe {
                    (*p).feeder_share = (*p).feeder_share.saturating_add(credit);
                    (*l).fields.feeder_share = (*l).fields.feeder_share.saturating_add(credit);
                    (*p).next_hop = via;
                }
            }
            _ => unreachable!(),
        }
        let count = u32::from(unsafe { read(p) }.count);
        unsafe {
            (*l).fields.action_counts[action.0 as usize] =
                (*l).fields.action_counts[action.0 as usize].wrapping_add(count);
        }
        sum = sum.wrapping_add(count);
    }
    let f = unsafe { fields(l) };
    u8::from(
        f.action_counts[MoveToAction::MTA_DELIVER.0 as usize] > 0
            || f.action_counts[MoveToAction::MTA_TRANSFER.0 as usize] > 0,
    )
}

// Action selectors: 0 return, 1 transfer, 2 deliver, 3 shift, 4 remove vehicle,
// 5 reroute vehicle, 6 reserve, 7 load, 8 reroute station, 9 truncate station,
// 10 unload dispatch. Walker tokens reproduce stable source iterator identity.
unsafe fn action(
    src: *mut List,
    dst: *mut List,
    s: Services,
    mode: u8,
    remaining: &mut u32,
    n: Node,
    avoid: u16,
    avoid2: u16,
    ge: *const c_void,
    cargo: u8,
    payment: *mut c_void,
    at: u32,
) -> bool {
    let original = unsafe { packet(s, n.shell) };
    if mode == 2 || mode == 4 {
        let remove = (*remaining).min(u32::from(original.count));
        *remaining = remaining.wrapping_sub(remove);
        unsafe {
            remove_cache(
                src,
                original,
                remove,
                Some(
                    if mode == 2 {
                        MoveToAction::MTA_DELIVER
                    } else {
                        MoveToAction::MTA_KEEP
                    }
                    .0 as usize,
                ),
            );
        }
        if mode == 2 {
            (s.pay)(payment, 1, cargo, n.shell, remove, at);
        }
        if remove == u32::from(original.count) {
            (s.destroy)(n.shell);
            return true;
        }
        unsafe {
            reduce((s.packet)(n.shell), remove);
        }
        return false;
    }
    let mut cp = n.shell;
    if *remaining < u32::from(original.count) {
        cp = unsafe { split((s.packet)(cp), s, *remaining) };
        *remaining = 0;
    } else {
        *remaining = remaining.wrapping_sub(u32::from(original.count));
    }
    if cp.is_null() {
        if mode == 1 || mode == 6 || mode == 7 {
            return false;
        }
        cp = n.shell;
    }
    let p = unsafe { packet(s, cp) };
    let count = u32::from(p.count);
    match mode {
        0 | 1 => {
            unsafe {
                tile((s.packet)(cp), s, at, false);
                remove_cache(
                    src,
                    p,
                    count,
                    Some(
                        if mode == 0 {
                            MoveToAction::MTA_LOAD
                        } else {
                            MoveToAction::MTA_TRANSFER
                        }
                        .0 as usize,
                    ),
                );
            }
            if mode == 0 {
                unsafe {
                    (*dst).fields.reserved_count = (*dst).fields.reserved_count.wrapping_sub(count);
                }
            }
            unsafe {
                append(dst, s, cp, if mode == 0 { avoid } else { p.next_hop });
            }
        }
        3 => unsafe {
            remove_cache(src, p, count, Some(MoveToAction::MTA_KEEP.0 as usize));
            append(dst, s, cp, u16::from(MoveToAction::MTA_KEEP.0));
        },
        5 => {
            if p.next_hop == avoid || p.next_hop == avoid2 {
                let via = unsafe {
                    crate::station_base::goods_get_via_raw(
                        (s.flow_map)(ge),
                        p.first_station,
                        avoid,
                        avoid2,
                        || s.random_draw(),
                    )
                };
                // Original mutates cp, even when cp_new is a partial split.
                unsafe {
                    (*(s.packet)(n.shell)).next_hop = via;
                }
            }
            if src != dst {
                unsafe {
                    remove_cache(src, p, count, Some(MoveToAction::MTA_TRANSFER.0 as usize));
                    add_cache(dst, p, Some(MoveToAction::MTA_TRANSFER.0 as usize));
                }
            }
            unsafe {
                insert(dst, cp, 0, None, true);
            }
        }
        6 | 7 => {
            unsafe {
                tile((s.packet)(cp), s, at, true);
            }
            if mode == 6 {
                unsafe {
                    (*src).fields.reserved_count = (*src).fields.reserved_count.wrapping_add(count);
                }
            }
            unsafe {
                remove_cache(src, p, count, None);
                append(
                    dst,
                    s,
                    cp,
                    if mode == 6 {
                        u16::from(MoveToAction::MTA_LOAD.0)
                    } else {
                        u16::from(MoveToAction::MTA_KEEP.0)
                    },
                );
            }
        }
        8 => {
            let via = unsafe {
                crate::station_base::goods_get_via_raw(
                    (s.flow_map)(ge),
                    p.first_station,
                    avoid,
                    avoid2,
                    || s.random_draw(),
                )
            };
            if src != dst {
                unsafe {
                    remove_cache(src, p, count, None);
                    add_cache(dst, p, None);
                }
            }
            unsafe {
                insert(dst, cp, via, None, false);
            }
        }
        _ => unreachable!(),
    }
    cp == n.shell
}
unsafe fn walk(
    src: *mut List,
    dst: *mut List,
    s: Services,
    mode: u8,
    remaining: &mut u32,
    key: Option<u16>,
    avoid: u16,
    avoid2: u16,
    ge: *const c_void,
    cargo: u8,
    payment: *mut c_void,
    at: u32,
) {
    let reverse = mode == 0 || mode == 3 || mode == 4;
    let mut n = if reverse {
        unsafe { last(src) }
    } else if let Some(key) = key {
        unsafe { destination(src, key, false) }
    } else {
        unsafe { first(src) }
    };
    while *remaining > 0 {
        let Some(current) = n else {
            break;
        };
        if key.is_some_and(|key| current.key != key) {
            break;
        }
        let next = if reverse {
            unsafe { before(src, current.token) }
        } else {
            unsafe { after(src, current.token) }
        };
        let removed = unsafe {
            action(
                src, dst, s, mode, remaining, current, avoid, avoid2, ge, cargo, payment, at,
            )
        };
        if removed {
            unsafe {
                erase(src, current.token);
            }
        } else {
            break;
        }
        n = next;
    }
}
pub(crate) unsafe fn truncate_station(
    l: *mut List,
    s: Services,
    max_move: u32,
    origins: *mut c_void,
) -> u32 {
    let max_move = max_move.min(unsafe { fields(l) }.count);
    let previous = unsafe { fields(l) }.count;
    let mut moved = 0_u32;
    let mut loop_count = 0_u32;
    while max_move > moved {
        let mut n = unsafe { first(l) };
        while let Some(current) = n {
            n = unsafe { after(l, current.token) };
            let p = unsafe { packet(s, current.shell) };
            if previous > max_move
                && unsafe { &*s.shared }.random_range(previous) < previous - max_move
            {
                if !origins.is_null() && loop_count == 0 {
                    (s.origin)(origins, p.first_station, u32::from(p.count), 0);
                }
                continue;
            }
            let diff = max_move.wrapping_sub(moved);
            if u32::from(p.count) > diff {
                if diff > 0 {
                    unsafe {
                        remove_cache(l, p, diff, None);
                        reduce((s.packet)(current.shell), diff);
                    }
                    moved = moved.wrapping_add(diff);
                }
                if loop_count > 0 {
                    if !origins.is_null() {
                        (s.origin)(origins, p.first_station, diff, 1);
                    }
                    return moved;
                }
                if !origins.is_null() {
                    (s.origin)(
                        origins,
                        p.first_station,
                        u32::from(unsafe { packet(s, current.shell) }.count),
                        0,
                    );
                }
            } else {
                unsafe {
                    erase(l, current.token);
                }
                if !origins.is_null() && loop_count > 0 {
                    (s.origin)(origins, p.first_station, u32::from(p.count), 1);
                }
                moved = moved.wrapping_add(u32::from(p.count));
                unsafe {
                    remove_cache(l, p, u32::from(p.count), None);
                }
                (s.destroy)(current.shell);
            }
        }
        loop_count = loop_count.wrapping_add(1);
    }
    moved
}
/// Original `VehicleCargoList::Truncate`, callable by Rust transaction owners.
/// # Safety
/// Live serialized vehicle cargo owner and immutable native services. No borrow
/// of this or another cargo owner survives the packet/deletion callbacks.
pub(crate) unsafe fn truncate_vehicle(src: *mut List, s: Services, amount: u32) -> u32 {
    let amount = amount.min(unsafe { openttd_rust_cargo_list_count(src) });
    let mut remaining = amount;
    unsafe {
        walk(
            src,
            std::ptr::null_mut(),
            s,
            4,
            &mut remaining,
            None,
            INVALID,
            INVALID,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            u32::MAX,
        );
    }
    amount
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_move(
    src: *mut List,
    dst: *mut List,
    s: *const Services,
    mode: u8,
    amount: u32,
    avoid: u16,
    avoid2: u16,
    next: *const u16,
    length: usize,
    ge: *const c_void,
    cargo: u8,
    payment: *mut c_void,
    at: u32,
) -> u32 {
    let s = unsafe { *s };
    if mode == 4 {
        return unsafe { truncate_vehicle(src, s, amount) };
    }
    if mode == 9 {
        return unsafe { truncate_station(src, s, amount, payment) };
    }
    if mode == 10 {
        let mut moved = 0_u32;
        let f = unsafe { fields(src) };
        if f.action_counts[MoveToAction::MTA_TRANSFER.0 as usize] > 0 {
            let amount = f.action_counts[MoveToAction::MTA_TRANSFER.0 as usize].min(amount);
            let mut remaining = amount;
            unsafe {
                walk(
                    src,
                    dst,
                    s,
                    1,
                    &mut remaining,
                    None,
                    avoid,
                    avoid2,
                    ge,
                    cargo,
                    payment,
                    at,
                );
            }
            moved = moved.wrapping_add(amount);
        }
        let f = unsafe { fields(src) };
        if f.action_counts[MoveToAction::MTA_TRANSFER.0 as usize] == 0
            && f.action_counts[MoveToAction::MTA_DELIVER.0 as usize] > 0
            && moved < amount
        {
            let amount = f.action_counts[MoveToAction::MTA_DELIVER.0 as usize].min(amount - moved);
            let mut remaining = amount;
            unsafe {
                walk(
                    src,
                    dst,
                    s,
                    2,
                    &mut remaining,
                    None,
                    avoid,
                    avoid2,
                    ge,
                    cargo,
                    payment,
                    at,
                );
            }
            moved = moved.wrapping_add(amount);
        }
        return moved;
    }
    if mode == 6 || mode == 7 || mode == 8 {
        if mode == 7 {
            let reserved =
                unsafe { fields(dst) }.action_counts[MoveToAction::MTA_LOAD.0 as usize].min(amount);
            if reserved > 0 {
                unsafe {
                    (*src).fields.reserved_count =
                        (*src).fields.reserved_count.wrapping_sub(reserved);
                    reassign(
                        dst,
                        s,
                        MoveToAction::MTA_LOAD,
                        MoveToAction::MTA_KEEP,
                        reserved,
                    );
                }
                return reserved;
            }
        }
        let mut remaining = amount;
        if mode == 8 {
            unsafe {
                walk(
                    src,
                    dst,
                    s,
                    mode,
                    &mut remaining,
                    Some(avoid),
                    avoid,
                    avoid2,
                    ge,
                    cargo,
                    payment,
                    at,
                );
            }
        } else {
            let next = NextHops { data: next, length };
            for index in (0..length).rev() {
                let key = unsafe { next.at(index) };
                unsafe {
                    walk(
                        src,
                        dst,
                        s,
                        mode,
                        &mut remaining,
                        Some(key),
                        avoid,
                        avoid2,
                        ge,
                        cargo,
                        payment,
                        at,
                    );
                }
                if remaining == 0 {
                    break;
                }
            }
            if remaining > 0 {
                unsafe {
                    walk(
                        src,
                        dst,
                        s,
                        mode,
                        &mut remaining,
                        Some(INVALID),
                        avoid,
                        avoid2,
                        ge,
                        cargo,
                        payment,
                        at,
                    );
                }
            }
        }
        return amount.wrapping_sub(remaining);
    }
    let f = unsafe { fields(src) };
    let amount = amount.min(match mode {
        0 => f.action_counts[MoveToAction::MTA_LOAD.0 as usize],
        5 => f.action_counts[MoveToAction::MTA_TRANSFER.0 as usize],
        _ => f.count,
    });
    let mut remaining = amount;
    unsafe {
        walk(
            src,
            dst,
            s,
            mode,
            &mut remaining,
            None,
            avoid,
            avoid2,
            ge,
            cargo,
            payment,
            at,
        );
    }
    amount
}

/// Traverse the typed pool in ID order; the native hook supplies identity only.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_invalidate(s: *const Services, value: u16, kind: u8) {
    let services = unsafe { *s };
    let mut shell = (services.packet_next)(std::ptr::null_mut());
    while !shell.is_null() {
        let owner = (services.packet)(shell);
        let observed = unsafe { read(owner) };
        if kind == u8::MAX {
            if observed.first_station == value {
                unsafe {
                    (*owner).first_station = INVALID;
                }
            }
        } else if observed.source_id == value && observed.source_type == kind {
            unsafe {
                (*owner).source_id = INVALID;
            }
        }
        shell = (services.packet_next)(shell);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Vehicle {
    pub(crate) list: *mut List,
    pub(crate) cargo: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CapacityServices {
    /// The shell's cargo list and cargo type.
    pub(crate) read: extern "C" fn(*mut c_void, *mut Vehicle),
    pub(crate) cargo: *const Services,
}
/// One part's cargo fields, and its capacity, train kind and articulation.
struct CapacityPart {
    list: *mut List,
    cargo: u8,
    capacity: u16,
    train: bool,
    articulated: bool,
}
fn vehicle(services: CapacityServices, v: VehicleRef) -> CapacityPart {
    let mut fields = Vehicle::default();
    (services.read)(v.shell(), &raw mut fields);
    CapacityPart {
        list: fields.list,
        cargo: fields.cargo,
        capacity: crate::raw_field::read_field!(v.0.as_ptr(), cargo_cap),
        train: v.vehicle_type() == crate::vehicle_type::VEH_TRAIN,
        articulated: v.is_articulated_part(),
    }
}
/// `Train::From(v)->other_multiheaded_part`, as a nullable record.
fn other(v: Option<VehicleRef>) -> Option<VehicleRef> {
    v.and_then(VehicleRef::other_multiheaded_part)
}
/// Callers invoke `ConsistChanged` only after this function returns:
/// it may reorder/destroy world parts and reenter their cargo owners.
pub(crate) unsafe fn cargo_capacity(
    s: *const CapacityServices,
    old: Option<VehicleRef>,
    new: Option<VehicleRef>,
    mode: u8,
    part_chain: u8,
) -> u8 {
    let services = unsafe { *s };
    let cargo = unsafe { *services.cargo };
    let transfer = mode != 0;
    let mut src = old;
    while let Some(src_part) = src {
        let source = vehicle(services, src_part);
        let count = unsafe { fields(source.list) }.count;
        if transfer
            && part_chain == 0
            && source.train
            && src != old
            && src != other(old)
            && !source.articulated
        {
            src = src_part.last_engine_part().next();
            continue;
        }
        if (!transfer && count <= u32::from(source.capacity))
            || (transfer && (source.cargo >= 64 || count == 0))
        {
            src = src_part.next();
            continue;
        }
        let mut spread = count.wrapping_sub(u32::from(source.capacity));
        let mut dest = if transfer { new } else { old };
        while let Some(dest_part) = dest
            && (if transfer {
                unsafe { fields(source.list) }.count > 0
            } else {
                spread != 0
            })
        {
            let destination = vehicle(services, dest_part);
            if transfer
                && part_chain == 0
                && destination.train
                && dest != new
                && dest != other(new)
                && !destination.articulated
            {
                dest = dest_part.last_engine_part().next();
                continue;
            }
            let held = unsafe { fields(destination.list) }.count;
            if destination.cargo == source.cargo
                && (transfer || held < u32::from(destination.capacity))
            {
                let available = u32::from(destination.capacity).wrapping_sub(held);
                let amount = available.min(if transfer {
                    unsafe { fields(source.list) }.count
                } else {
                    spread
                });
                if amount > 0 {
                    let mut remaining = amount;
                    unsafe {
                        walk(
                            source.list,
                            destination.list,
                            cargo,
                            3,
                            &mut remaining,
                            None,
                            INVALID,
                            INVALID,
                            std::ptr::null(),
                            0,
                            std::ptr::null_mut(),
                            0,
                        );
                    }
                    if !transfer {
                        spread = spread.wrapping_sub(amount);
                    }
                }
            }
            dest = dest_part.next();
        }
        if !transfer {
            let count = unsafe { fields(source.list) }.count;
            if count > u32::from(source.capacity) {
                let mut remove = count - u32::from(source.capacity);
                unsafe {
                    walk(
                        source.list,
                        std::ptr::null_mut(),
                        cargo,
                        4,
                        &mut remove,
                        None,
                        INVALID,
                        INVALID,
                        std::ptr::null(),
                        0,
                        std::ptr::null_mut(),
                        0,
                    );
                }
            }
        }
        src = src_part.next();
    }
    u8::from(
        transfer
            && part_chain != 0
            && new.is_some_and(|v| v.vehicle_type() == crate::vehicle_type::VEH_TRAIN),
    )
}

/// `VehicleCargoList::Reroute`, with already-resolved canonical owners.
/// # Safety
/// All pointers are live for this synchronous call, with no outstanding owner borrow.
pub(crate) unsafe fn reroute_vehicle(
    list: *mut List,
    s: &Services,
    ge: *const crate::station_base::GoodsEntry,
    avoid: u16,
    avoid2: u16,
) {
    unsafe {
        openttd_rust_cargo_list_move(
            list,
            list,
            s,
            5,
            u32::MAX,
            avoid,
            avoid2,
            std::ptr::null(),
            0,
            ge.cast(),
            0,
            std::ptr::null_mut(),
            crate::tile_type::INVALID_TILE,
        );
    }
}
/// `StationCargoList::Reroute`, with already-resolved canonical owners.
/// # Safety
/// Same serialized owner contract as `reroute_vehicle`.
pub(crate) unsafe fn reroute_station(
    list: *mut List,
    s: &Services,
    ge: *const crate::station_base::GoodsEntry,
    avoid: u16,
    avoid2: u16,
) {
    unsafe {
        openttd_rust_cargo_list_move(
            list,
            list,
            s,
            8,
            u32::MAX,
            avoid,
            avoid2,
            std::ptr::null(),
            0,
            ge.cast(),
            0,
            std::ptr::null_mut(),
            crate::tile_type::INVALID_TILE,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    thread_local! {
        static CAPACITY_CALLS: RefCell<Vec<(&'static str, u8)>> = const { RefCell::new(Vec::new()) };
        static ALLOCATE: Cell<bool> = const { Cell::new(true) };
        static POOL: RefCell<Vec<*mut c_void>> = const { RefCell::new(Vec::new()) };
        static PAYMENT_LIST: Cell<*mut List> = const { Cell::new(std::ptr::null_mut()) };
        static PAYMENTS: RefCell<Vec<(u8, u32, u32)>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn can_allocate() -> u8 {
        ALLOCATE.with(|flag| u8::from(flag.get()))
    }
    extern "C" fn create(fields: *const Packet) -> *mut c_void {
        let shell = Box::into_raw(Box::new(unsafe { *fields })).cast();
        POOL.with(|pool| pool.borrow_mut().push(shell));
        shell
    }
    extern "C" fn owner(shell: *mut c_void) -> *mut Packet {
        shell.cast()
    }
    extern "C" fn destroy(shell: *mut c_void) {
        POOL.with(|pool| pool.borrow_mut().retain(|&entry| entry != shell));
        unsafe {
            drop(Box::from_raw(shell.cast::<Packet>()));
        }
    }
    extern "C" fn pay(
        _: *mut c_void,
        final_delivery: u8,
        _: u8,
        _: *const c_void,
        amount: u32,
        _: u32,
    ) -> i64 {
        // Reentry observes the list before Stage's action count commit, and after
        // final delivery's metadata removal, exactly like native payment getters.
        let list = PAYMENT_LIST.with(Cell::get);
        let held = unsafe { openttd_rust_cargo_list_count(list) };
        PAYMENTS.with(|payments| payments.borrow_mut().push((final_delivery, amount, held)));
        7
    }
    extern "C" fn origin(_: *mut c_void, _: u16, _: u32, _: u8) {}
    extern "C" fn flow_map(context: *const c_void) -> *mut crate::cargo_flow::FlowMap {
        context.cast_mut().cast()
    }
    fn flow_fixture(via: u16) -> *mut crate::cargo_flow::FlowMap {
        let map = crate::cargo_flow::openttd_rust_flow_map_new();
        let flow = crate::cargo_flow::openttd_rust_flow_new(via, 1, 0);
        unsafe {
            for origin in [10, 11] {
                crate::cargo_flow::openttd_rust_flow_map_insert(map, origin, flow);
            }
            crate::cargo_flow::openttd_rust_flow_destroy(flow);
        }
        map
    }
    extern "C" fn draw(_: *mut c_void) -> u32 {
        0
    }
    extern "C" fn no_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        0
    }
    thread_local! {
        static SHARED: crate::services::Services = crate::services::fixture(std::ptr::null_mut(), draw, no_industry);
    }
    extern "C" fn next_packet(shell: *mut c_void) -> *mut c_void {
        POOL.with(|pool| {
            let pool = pool.borrow();
            let index = if shell.is_null() {
                0
            } else {
                pool.iter().position(|&entry| entry == shell).unwrap() + 1
            };
            pool.get(index).copied().unwrap_or(std::ptr::null_mut())
        })
    }
    fn services() -> Services {
        crate::map_func::fixture::set(
            crate::map_func::TileTypeAllocation {
                base: std::ptr::null(),
                extended: std::ptr::null(),
                count: 256 * 256,
                assertions: true,
            },
            8,
        );
        Services {
            can_allocate,
            create,
            packet: owner,
            destroy,
            pay,
            origin,
            flow_map,
            shared: SHARED.with(std::ptr::from_ref),
            packet_next: next_packet,
        }
    }
    fn cargo(count: u16, first: u16, feeder: i64) -> *mut c_void {
        let fields = Packet {
            feeder_share: feeder,
            source_xy: 5,
            count,
            periods_in_transit: 2,
            first_station: first,
            next_hop: INVALID,
            source_id: 9,
            travelled_x: 5,
            travelled_y: 0,
            source_type: 0,
            in_vehicle: 1,
        };
        create(&raw const fields)
    }
    /// A packet waiting at a station: not yet in a vehicle (`UpdateLoadingTile`'s precondition).
    fn station_cargo(count: u16, first: u16, feeder: i64) -> *mut c_void {
        let shell = cargo(count, first, feeder);
        // SAFETY: the fixture shell was just created and owns its live packet.
        unsafe { (*owner(shell)).in_vehicle = 0 };
        shell
    }
    fn list(vehicle: bool) -> *mut List {
        openttd_rust_cargo_list_new(u8::from(vehicle))
    }
    unsafe fn clean(list: *mut List) {
        let service = services();
        unsafe {
            openttd_rust_cargo_list_destroy(list, &raw const service);
        }
    }

    extern "C" fn deny_allocation() -> u8 {
        CAPACITY_CALLS.with(|calls| calls.borrow_mut().push(("packet", 0)));
        0
    }

    extern "C" fn unexpected_allocation(_: *const Packet) -> *mut c_void {
        panic!("denied Split must return before allocating a packet")
    }

    #[test]
    fn split_denied_preserves_packet_before_share_arithmetic() {
        // Captured from the unchanged 15.3 CargoPacket::Split body at
        // 14ec60f248547d4d062a1160f0fc26d742319888 using the retiring
        // cargo-split-reference.cpp reference branch, before its deletion.
        let service = Services {
            can_allocate: deny_allocation,
            create: unexpected_allocation,
            ..services()
        };
        let mut cases = 0;
        for row in include_str!("cargo_storage/split-denied-oracle.txt").lines() {
            let row: Vec<i64> = row.split_whitespace().map(|v| v.parse().unwrap()).collect();
            let [count, feeder, amount, calls] = row[..] else {
                panic!("invalid original split observation")
            };
            let mut packet = Packet {
                count: u16::try_from(count).unwrap(),
                feeder_share: feeder,
                ..Packet::default()
            };
            CAPACITY_CALLS.with(|log| log.borrow_mut().clear());
            // SAFETY: The stack packet and synchronous fixture table remain live.
            let part = unsafe { split(&raw mut packet, service, u32::try_from(amount).unwrap()) };
            assert!(part.is_null());
            assert_eq!(i64::from(packet.count), count);
            assert_eq!(packet.feeder_share, feeder);
            CAPACITY_CALLS
                .with(|log| assert_eq!(i64::try_from(log.borrow().len()).unwrap(), calls));
            cases += 1;
        }
        assert_eq!(cases, 48);
    }

    thread_local! {
        static FAILED_CHECKS: Cell<u32> = const { Cell::new(0) };
    }
    extern "C" fn record_failed_check(condition: bool) {
        if !condition {
            FAILED_CHECKS.with(|failed| failed.set(failed.get() + 1));
        }
    }
    extern "C" fn abort_failed_check(condition: bool) {
        if !condition {
            std::process::abort();
        }
    }
    /// Failed native checks on this thread while running `operation`. Other tests
    /// keep failing fast afterwards.
    fn failed_checks(operation: impl FnOnce()) -> u32 {
        crate::services_assert::openttd_rust_assert_bind(record_failed_check);
        FAILED_CHECKS.with(|failed| failed.set(0));
        operation();
        crate::services_assert::openttd_rust_assert_bind(abort_failed_check);
        FAILED_CHECKS.with(Cell::get)
    }

    #[test]
    fn cargo_packet_preconditions_call_the_native_check_once() {
        let service = services();
        let mut packet = Packet {
            count: 4,
            feeder_share: 8,
            ..Packet::default()
        };
        let p = &raw mut packet;
        // SAFETY: the stack packet and the fixture map outlive each synchronous call.
        unsafe {
            // `Reduce`: count < this->count.
            assert_eq!(failed_checks(|| reduce(p, 3)), 0);
            assert_eq!(((*p).count, (*p).feeder_share), (1, 2));
            assert_eq!(failed_checks(|| reduce(p, 1)), 1);
            (*p).count = 4;
            // `GetDistance`: a valid source, then in a vehicle.
            let at = 3 + 4 * 256;
            let both = failed_checks(|| {
                openttd_rust_cargo_packet_distance(p, &raw const service, at);
            });
            assert_eq!(both, 2);
            // `UpdateUnloadingTile`: in a vehicle.
            assert_eq!(failed_checks(|| tile(p, service, at, false)), 1);
            // `UpdateLoadingTile`: not yet in a vehicle; it sets the missing source.
            assert_eq!(failed_checks(|| tile(p, service, at, true)), 0);
            assert_eq!(((*p).source_xy, (*p).in_vehicle), (at, 1));
            let distance = openttd_rust_cargo_packet_distance(p, &raw const service, at);
            assert_eq!(failed_checks(|| tile(p, service, at, true)), 1);
            assert_eq!(distance, 0);
            // The split constructor: count != 0.
            (*p).count = 0;
            let copy = Cell::new(std::ptr::null_mut());
            assert_eq!(
                failed_checks(|| copy.set(openttd_rust_cargo_packet_new(p))),
                1
            );
            openttd_rust_cargo_packet_destroy(copy.get());
        }
    }

    #[test]
    fn canonical_packet_defaults_and_rectangular_map_coordinates() {
        let service = services();
        crate::map_func::fixture::set(
            crate::map_func::TileTypeAllocation {
                base: std::ptr::null(),
                extended: std::ptr::null(),
                count: 512 * 256,
                assertions: true,
            },
            9,
        );
        // SAFETY: null selects default construction without reading an input packet.
        let packet = unsafe { openttd_rust_cargo_packet_new(std::ptr::null()) };
        // SAFETY: this test owns the live packet until its final destruction.
        let observed = unsafe { read(packet) };
        assert_eq!(observed.source_xy, u32::MAX);
        assert_eq!(observed.first_station, INVALID);
        assert_eq!(observed.next_hop, INVALID);
        assert_eq!(observed.source_id, INVALID);
        assert_eq!(observed.count, 0);
        assert_eq!(observed.periods_in_transit, 0);
        assert_eq!(observed.feeder_share, 0);
        assert_eq!(observed.source_type, 0);
        assert_eq!(observed.in_vehicle, 0);
        assert_eq!((observed.travelled_x, observed.travelled_y), (0, 0));
        // A station transfer covers (20, 20) without travel in a vehicle.
        // SAFETY: the test owns the live packet and the installed rectangular map.
        unsafe { tile(packet, service, 5 + 10 * 512, true) };
        // SAFETY: the same packet and map remain live after loading.
        unsafe { tile(packet, service, 20 + 30 * 512, false) };
        // SAFETY: the same packet and map remain live after unloading.
        unsafe { tile(packet, service, 40 + 50 * 512, true) };
        // SAFETY: the owned packet remains live; this read retains no borrow.
        let observed = unsafe { read(packet) };
        assert_eq!(observed.source_xy, 5 + 10 * 512);
        assert_eq!((observed.travelled_x, observed.travelled_y), (25, 30));
        // SAFETY: packet, service table and map remain live for the synchronous read.
        let distance = unsafe {
            openttd_rust_cargo_packet_distance(packet, &raw const service, 100 + 100 * 512)
        };
        assert_eq!(distance, 145);
        // Paid distance cannot exceed the direct source-to-destination distance.
        // SAFETY: the same packet, service table and map remain live.
        let distance =
            unsafe { openttd_rust_cargo_packet_distance(packet, &raw const service, 5 + 10 * 512) };
        assert_eq!(distance, 0);
        // SAFETY: this test destroys its owned packet exactly once, after all reads.
        unsafe { openttd_rust_cargo_packet_destroy(packet) };
    }

    #[test]
    fn partial_reserve_load_return_and_feeder_rounding() {
        let service = services();
        let station = list(false);
        let vehicle = list(true);
        unsafe {
            append(station, service, station_cargo(17, 10, 23), 100);
            let next = [100];
            assert_eq!(
                openttd_rust_cargo_list_move(
                    station,
                    vehicle,
                    &raw const service,
                    6,
                    7,
                    INVALID,
                    INVALID,
                    next.as_ptr(),
                    1,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    5
                ),
                7
            );
            assert_eq!(fields(station).reserved_count, 7);
            assert_eq!(fields(station).count, 10);
            assert_eq!(fields(vehicle).action_counts, [0, 0, 0, 7]);
            assert_eq!(fields(vehicle).feeder_share, 9);
            assert_eq!(
                openttd_rust_cargo_list_move(
                    station,
                    vehicle,
                    &raw const service,
                    7,
                    3,
                    INVALID,
                    INVALID,
                    next.as_ptr(),
                    1,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    5
                ),
                3
            );
            assert_eq!(fields(vehicle).action_counts, [0, 0, 3, 4]);
            assert_eq!(
                openttd_rust_cargo_list_move(
                    vehicle,
                    station,
                    &raw const service,
                    0,
                    4,
                    100,
                    INVALID,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    5
                ),
                4
            );
            assert_eq!(fields(vehicle).count, 3);
            assert_eq!(fields(station).count, 14);
            assert_eq!(fields(station).reserved_count, 0);
            clean(vehicle);
            clean(station);
        }
    }
    #[test]
    fn stage_order_and_payment_getter_reentry() {
        let service = services();
        let vehicle = list(true);
        let station = list(false);
        PAYMENT_LIST.with(|slot| slot.set(vehicle));
        PAYMENTS.with(|slot| slot.borrow_mut().clear());
        let flows = crate::cargo_flow::openttd_rust_flow_map_new();
        unsafe {
            for (origin, via) in [(10, 100), (11, 200), (12, 300)] {
                let flow = crate::cargo_flow::openttd_rust_flow_new(via, 1, 0);
                crate::cargo_flow::openttd_rust_flow_map_insert(flows, origin, flow);
                crate::cargo_flow::openttd_rust_flow_destroy(flow);
            }
            for first in [10, 11, 12] {
                append(
                    vehicle,
                    service,
                    cargo(8, first, 0),
                    u16::from(MoveToAction::MTA_KEEP.0),
                );
            }
            let next = [100];
            assert_eq!(
                openttd_rust_cargo_list_stage(
                    vehicle,
                    &raw const service,
                    1,
                    200,
                    next.as_ptr(),
                    1,
                    0,
                    flows.cast(),
                    0,
                    std::ptr::null_mut(),
                    5
                ),
                1
            );
            assert_eq!(fields(vehicle).action_counts, [8, 8, 8, 0]);
            let order: Vec<u16> = nodes(vehicle)
                .iter()
                .map(|node| packet(service, node.shell).first_station)
                .collect();
            assert_eq!(order, [12, 11, 10]);
            assert_eq!(fields(vehicle).feeder_share, 7);
            assert_eq!(
                openttd_rust_cargo_list_move(
                    vehicle,
                    station,
                    &raw const service,
                    10,
                    12,
                    INVALID,
                    INVALID,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    5
                ),
                12
            );
            assert_eq!(fields(vehicle).action_counts, [0, 4, 8, 0]);
            assert_eq!(fields(station).count, 8);
            PAYMENTS.with(|payments| assert_eq!(*payments.borrow(), [(0, 8, 24), (1, 4, 12)]));
            clean(vehicle);
            clean(station);
            crate::cargo_flow::openttd_rust_flow_map_destroy(flows);
        }
    }
    #[test]
    fn same_list_reroute_and_partial_packet_identity() {
        let service = services();
        unsafe {
            for via in [10_u16, 90] {
                let flows = flow_fixture(via);
                let station = list(false);
                append(station, service, station_cargo(8, 10, 0), 70);
                append(station, service, station_cargo(8, 11, 0), 70);
                assert_eq!(
                    openttd_rust_cargo_list_move(
                        station,
                        station,
                        &raw const service,
                        8,
                        10,
                        70,
                        INVALID,
                        std::ptr::null(),
                        0,
                        flows.cast(),
                        0,
                        std::ptr::null_mut(),
                        5
                    ),
                    10
                );
                let selected: u32 = nodes(station)
                    .iter()
                    .filter(|node| node.key == via)
                    .map(|node| u32::from(packet(service, node.shell).count))
                    .sum();
                assert_eq!(selected, 10);
                assert_eq!(fields(station).count, 16);
                clean(station);
                crate::cargo_flow::openttd_rust_flow_map_destroy(flows);
            }
            let vehicle = list(true);
            let original = cargo(8, 10, 0);
            (*owner(original)).next_hop = 70;
            append(
                vehicle,
                service,
                original,
                u16::from(MoveToAction::MTA_KEEP.0),
            );
            (*vehicle).fields.action_counts = [8, 0, 0, 0];
            let via = 90_u16;
            let flows = flow_fixture(via);
            assert_eq!(
                openttd_rust_cargo_list_move(
                    vehicle,
                    vehicle,
                    &raw const service,
                    5,
                    3,
                    70,
                    INVALID,
                    std::ptr::null(),
                    0,
                    flows.cast(),
                    0,
                    std::ptr::null_mut(),
                    5
                ),
                3
            );
            assert_eq!((*owner(original)).next_hop, 90);
            assert_eq!(packet(service, nodes(vehicle)[0].shell).next_hop, 70);
            assert_eq!(fields(vehicle).action_counts, [8, 0, 0, 0]);
            clean(vehicle);
            crate::cargo_flow::openttd_rust_flow_map_destroy(flows);
        }
    }
    #[test]
    fn merge_age_invalidation_and_pool_failure_fallback() {
        let service = services();
        unsafe {
            let station = list(false);
            let vehicle = list(true);
            append(station, service, station_cargo(8, 10, 3), 100);
            append(station, service, station_cargo(9, 10, 4), 100);
            assert_eq!(nodes(station).len(), 1);
            let next = [100];
            openttd_rust_cargo_list_move(
                station,
                vehicle,
                &raw const service,
                7,
                17,
                INVALID,
                INVALID,
                next.as_ptr(),
                1,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                5,
            );
            age::<false>(vehicle, &raw const service);
            assert_eq!(fields(vehicle).cargo_periods_in_transit, 51);
            openttd_rust_cargo_invalidate(&raw const service, 9, 0);
            assert_eq!(
                packet(service, first(vehicle).unwrap().shell).source_id,
                INVALID
            );
            ALLOCATE.with(|flag| flag.set(false));
            let destination = list(true);
            assert_eq!(
                openttd_rust_cargo_list_move(
                    vehicle,
                    destination,
                    &raw const service,
                    3,
                    3,
                    INVALID,
                    INVALID,
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    5
                ),
                3
            );
            assert_eq!(fields(vehicle).count, 0);
            assert_eq!(fields(destination).count, 17); // original whole-packet fallback
            ALLOCATE.with(|flag| flag.set(true));
            clean(destination);
            clean(vehicle);
            clean(station);
        }
    }
    #[test]
    fn all_forced_unloading_selectors() {
        let service = services();
        for (selector, action) in [
            (4, MoveToAction::MTA_KEEP),
            (1, MoveToAction::MTA_DELIVER),
            (2, MoveToAction::MTA_TRANSFER),
        ] {
            let vehicle = list(true);
            PAYMENT_LIST.with(|slot| slot.set(vehicle));
            unsafe {
                append(
                    vehicle,
                    service,
                    cargo(8, 10, 0),
                    u16::from(MoveToAction::MTA_KEEP.0),
                );
                openttd_rust_cargo_list_stage(
                    vehicle,
                    &raw const service,
                    1,
                    200,
                    std::ptr::null(),
                    0,
                    selector,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    5,
                );
                let meta = fields(vehicle);
                assert_eq!(meta.action_counts[action.0 as usize], 8);
                assert_eq!(meta.action_counts.iter().sum::<u32>(), 8);
                clean(vehicle);
            }
        }
    }

    /// One train part: its shell-side cargo fields and its canonical record.
    struct Part {
        id: u8,
        list: *mut List,
        cargo: u8,
        record: *mut crate::vehicle_base::VehicleRecord,
        train: Box<crate::train_state::TrainState>,
    }
    extern "C" fn capacity_read(shell: *mut c_void, output: *mut Vehicle) {
        let part = unsafe { &*shell.cast::<Part>() };
        CAPACITY_CALLS.with(|calls| calls.borrow_mut().push(("read", part.id)));
        unsafe {
            output.write(Vehicle {
                list: part.list,
                cargo: part.cargo,
            });
        }
    }
    // The record keeps the boxed part address as its shell.
    #[allow(clippy::unnecessary_box_returns)]
    fn part(cap: u16) -> Box<Part> {
        let mut part = Box::new(Part {
            id: 0,
            list: list(true),
            cargo: 0,
            record: std::ptr::null_mut(),
            train: Box::default(),
        });
        let shell = std::ptr::from_mut(&mut *part).cast();
        part.record = crate::vehicle_base::openttd_rust_vehicle_record_new(
            shell,
            crate::vehicle_type::VehicleType::TRAIN,
        );
        unsafe {
            (*part.record).cargo_cap = cap;
            (*part.record).train = &raw mut *part.train;
        }
        part
    }
    fn link(parts: &[Box<Part>]) {
        for pair in parts.windows(2) {
            unsafe { crate::vehicle_base::set_next(pair[0].record, pair[1].record) };
        }
    }
    fn articulate(part: &Part) {
        unsafe { (*part.record).subtype |= 1 << crate::vehicle_base::GVSF_ARTICULATED_PART };
    }
    fn head(part: &Part) -> Option<VehicleRef> {
        std::ptr::NonNull::new(part.record).map(VehicleRef)
    }
    fn release(part: &Part) {
        unsafe {
            clean(part.list);
            crate::vehicle_base::openttd_rust_vehicle_record_delete(part.record);
        }
    }
    #[test]
    fn destination_count_keeps_empty_imported_keys() {
        unsafe {
            let station = list(false);
            openttd_rust_cargo_list_insert(station, 100, std::ptr::null_mut());
            openttd_rust_cargo_list_insert(station, 200, std::ptr::null_mut());
            openttd_rust_cargo_list_insert(station, 100, std::ptr::null_mut());
            assert_eq!(openttd_rust_cargo_list_key_count(station), 2);
            assert_eq!(openttd_rust_cargo_list_count(station), 0);
            assert_eq!(openttd_rust_cargo_list_has(station, std::ptr::null(), 0), 0);
            let next = [200, 100];
            assert_eq!(
                openttd_rust_cargo_list_has(station, next.as_ptr(), next.len()),
                1
            );
            openttd_rust_cargo_list_insert(station, INVALID, std::ptr::null_mut());
            assert_eq!(openttd_rust_cargo_list_key_count(station), 3);
            assert_eq!(openttd_rust_cargo_list_has(station, std::ptr::null(), 0), 1);
            openttd_rust_cargo_list_clear(station);
            assert_eq!(openttd_rust_cargo_list_key_count(station), 0);
            clean(station);
        }
    }

    #[test]
    fn capacity_autoreplace_single_engine_skips_foreign_parts_in_order() {
        let service = services();
        let capacity = CapacityServices {
            read: capacity_read,
            cargo: &raw const service,
        };
        let mut source: Vec<_> = (0..5).map(|_| part(1)).collect();
        let mut destination: Vec<_> = (0..5).map(|_| part(1)).collect();
        for parts in [&source, &destination] {
            link(parts);
            let (front, rear) = (parts[0].record, parts[4].record);
            unsafe { (*(*front).train).other_multiheaded_part = rear };
            articulate(&parts[1]);
            articulate(&parts[3]);
        }
        for (index, entry) in source.iter_mut().enumerate() {
            entry.id = index as u8 + 1;
        }
        for (index, entry) in destination.iter_mut().enumerate() {
            entry.id = index as u8 + 11;
        }
        unsafe {
            for (index, entry) in source.iter().enumerate() {
                append(
                    entry.list,
                    service,
                    cargo(1, index as u16 + 10, 0),
                    u16::from(MoveToAction::MTA_KEEP.0),
                );
            }
            CAPACITY_CALLS.with(|calls| calls.borrow_mut().clear());
            assert_eq!(
                cargo_capacity(
                    &raw const capacity,
                    head(&source[0]),
                    head(&destination[0]),
                    1,
                    0,
                ),
                0
            );
            assert_eq!(
                source
                    .iter()
                    .map(|entry| fields(entry.list).count)
                    .collect::<Vec<_>>(),
                [0, 0, 1, 1, 0]
            );
            assert_eq!(
                destination
                    .iter()
                    .map(|entry| fields(entry.list).count)
                    .collect::<Vec<_>>(),
                [1, 1, 0, 0, 1]
            );
            // Foreign engines skip to after their last articulated part: 3 skips 4, 13 skips 14.
            CAPACITY_CALLS.with(|calls| {
                assert_eq!(
                    calls.borrow().iter().map(|&(_, id)| id).collect::<Vec<_>>(),
                    [1, 11, 2, 11, 12, 3, 5, 11, 12, 13, 15]
                );
            });
        }
        for entry in source.iter().chain(&destination) {
            release(entry);
        }
    }

    #[test]
    fn capacity_shrink_and_autoreplace_consist_transfer() {
        let service = services();
        let capacity = CapacityServices {
            read: capacity_read,
            cargo: &raw const service,
        };
        let source = [part(3), part(6)];
        link(&source);
        unsafe {
            append(
                source[0].list,
                service,
                cargo(8, 10, 8),
                u16::from(MoveToAction::MTA_KEEP.0),
            );
            append(
                source[1].list,
                service,
                cargo(1, 11, 1),
                u16::from(MoveToAction::MTA_KEEP.0),
            );
            assert_eq!(
                cargo_capacity(&raw const capacity, head(&source[0]), None, 0, 0),
                0
            );
            assert_eq!(fields(source[0].list).count, 3);
            assert_eq!(fields(source[1].list).count, 6);
            let mut replacement = [part(4), part(4)];
            link(&replacement);
            assert_eq!(
                cargo_capacity(
                    &raw const capacity,
                    head(&source[0]),
                    head(&replacement[0]),
                    1,
                    1
                ),
                1
            );
            assert_eq!(fields(source[0].list).count, 0);
            assert_eq!(fields(source[1].list).count, 1);
            assert_eq!(fields(replacement[0].list).count, 4);
            assert_eq!(fields(replacement[1].list).count, 4);
            (*replacement[0].record).cargo_cap = 2;
            replacement[1].cargo = 1;
            cargo_capacity(&raw const capacity, head(&replacement[0]), None, 0, 0);
            assert_eq!(fields(replacement[0].list).count, 2);
            assert_eq!(fields(replacement[1].list).count, 4);
            for entry in source.iter().chain(&replacement) {
                release(entry);
            }
        }
    }
}

/// The behavioural checks of the retired `tools/simulation/cargo-list-scaling.cpp` (#736 F):
/// lists of 1000 to 64000 packets, far larger than the play corpus, keep packet identity,
/// order and counts through `NoUnload` staging, first-hit appends, reverse removal, keyed
/// loads and next-hop queries, reading each appended or loaded packet a bounded number of
/// times. Expected values are the probe's own checks; its timings were not observations.
#[cfg(test)]
mod scaling_tests {
    use super::*;
    use crate::test_tables::{CARGO_STORAGE, SHARED};
    use std::cell::Cell;

    thread_local! {
        static READS: Cell<u32> = const { Cell::new(0) };
    }
    extern "C" fn packet_of(shell: *mut c_void) -> *mut Packet {
        READS.set(READS.get() + 1);
        shell.cast()
    }
    extern "C" fn destroy_packet(shell: *mut c_void) {
        // SAFETY: Every shell in these lists is a packet from `openttd_rust_cargo_packet_new`.
        unsafe { openttd_rust_cargo_packet_destroy(shell.cast()) };
    }
    extern "C" fn collect(context: *mut c_void, _: u16, shell: *mut c_void) {
        if !shell.is_null() {
            // SAFETY: `snapshot`'s own vector.
            unsafe { (*context.cast::<Vec<*mut c_void>>()).push(shell) };
        }
    }
    fn snapshot(list: *const List) -> Vec<*mut c_void> {
        let mut shells = Vec::new();
        // SAFETY: A live list; the callback writes only the local vector.
        unsafe { openttd_rust_cargo_list_snapshot(list, (&raw mut shells).cast(), collect) };
        shells
    }
    fn packet_aged(periods: u32) -> *mut c_void {
        // SAFETY: A null input allocates a default packet.
        let packet = unsafe { openttd_rust_cargo_packet_new(std::ptr::null()) };
        // SAFETY: Allocated above; the list takes it.
        unsafe {
            (*packet).count = 1;
            (*packet).periods_in_transit = periods as u16;
        }
        packet.cast()
    }
    fn exported(list: *const List) -> Fields {
        let mut fields = Fields::default();
        // SAFETY: A live list and a local output.
        unsafe { openttd_rust_cargo_list_export(list, &raw mut fields) };
        fields
    }
    fn imported(list: *mut List, count: u32, periods: u64, keep: u32) {
        let mut fields = Fields {
            count,
            cargo_periods_in_transit: periods,
            ..Fields::default()
        };
        fields.action_counts[2] = keep;
        // SAFETY: A live list and a local input.
        unsafe { openttd_rust_cargo_list_import(list, &raw const fields) };
    }

    /// The vehicle list: 7 `NoUnload` stages keep the order; 1024 first-hit appends merge
    /// into the newest packet with 4 reads each; a reverse removal empties the list.
    fn check_vehicle(s: &Services, size: u32) {
        let vehicle = openttd_rust_cargo_list_new(1);
        // SAFETY: The list is live until destroyed at the end; every shell is a fresh packet.
        unsafe {
            for i in 0..size {
                openttd_rust_cargo_list_insert(vehicle, 0, packet_aged(i));
            }
            imported(
                vehicle,
                size,
                u64::from(size) * u64::from(size - 1) / 2,
                size,
            );
            let expected = snapshot(vehicle);
            for round in 0..7 {
                openttd_rust_cargo_list_stage(
                    vehicle,
                    s,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    4,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    0,
                );
                assert!(snapshot(vehicle) == expected, "{size}: stage {round} order");
            }
            READS.set(0);
            for _ in 0..1024 {
                openttd_rust_cargo_list_append(vehicle, s, packet_aged(size - 1), 2);
            }
            assert_eq!(READS.get(), 4096, "{size}: first-hit append reads");
            assert!(snapshot(vehicle) == expected, "{size}: append order");
            let fields = exported(vehicle);
            assert_eq!(
                (fields.count, fields.action_counts[2]),
                (size + 1024, size + 1024)
            );
            openttd_rust_cargo_list_move(
                vehicle,
                std::ptr::null_mut(),
                s,
                4,
                size + 1024,
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
            );
            assert!(
                snapshot(vehicle).is_empty(),
                "{size}: removal empties the list"
            );
            let fields = exported(vehicle);
            assert_eq!(
                (
                    fields.count,
                    fields.action_counts[2],
                    fields.cargo_periods_in_transit
                ),
                (0, 0, 0),
                "{size}: removed counts"
            );
            openttd_rust_cargo_list_destroy(vehicle, s);
        }
    }

    /// The station list: keyed next-hop queries, 1024 first-hit appends with 4 reads each,
    /// and a 1025-piece keyed load of only the front packet with 4 reads.
    fn check_station(s: &Services, size: u32) {
        let station = openttd_rust_cargo_list_new(0);
        // SAFETY: Both lists are live until destroyed at the end; every shell is a fresh packet.
        unsafe {
            openttd_rust_cargo_list_insert(station, 0, packet_aged(size));
            for i in 0..size {
                openttd_rust_cargo_list_insert(station, (i + 2) as u16, packet_aged(i));
            }
            let periods = u64::from(size) * u64::from(size - 1) / 2 + u64::from(size);
            imported(station, size + 1, periods, 0);
            let mut expected = snapshot(station);
            let (last_key, missing_key) = ((size + 1) as u16, 1_u16);
            for _ in 0..1024 {
                assert_eq!(
                    openttd_rust_cargo_list_has(station, &raw const last_key, 1),
                    1
                );
                assert_eq!(
                    openttd_rust_cargo_list_has(station, &raw const missing_key, 1),
                    0
                );
            }
            openttd_rust_cargo_list_insert(station, u16::MAX, std::ptr::null_mut());
            assert_eq!(openttd_rust_cargo_list_has(station, std::ptr::null(), 0), 1);
            READS.set(0);
            for _ in 0..1024 {
                openttd_rust_cargo_list_append(station, s, packet_aged(size), 0);
            }
            assert_eq!(READS.get(), 4096, "{size}: station first-hit append reads");
            assert!(
                snapshot(station) == expected,
                "{size}: station append order"
            );
            let vehicle = openttd_rust_cargo_list_new(1);
            let next = 0_u16;
            READS.set(0);
            let moved = openttd_rust_cargo_list_move(
                station,
                vehicle,
                s,
                7,
                1025,
                0,
                0,
                &raw const next,
                1,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
            );
            assert_eq!((moved, READS.get()), (1025, 4), "{size}: keyed load, reads");
            assert!(snapshot(vehicle) == [expected[0]], "{size}: loaded packet");
            expected.remove(0);
            assert!(snapshot(station) == expected, "{size}: station remainder");
            assert_eq!(exported(station).count, size, "{size}: station count");
            openttd_rust_cargo_list_destroy(vehicle, s);
            openttd_rust_cargo_list_destroy(station, s);
        }
    }

    #[test]
    fn large_lists_keep_identity_order_and_bounded_reads() {
        crate::map_func::fixture::set(
            crate::map_func::TileTypeAllocation {
                base: std::ptr::null(),
                extended: std::ptr::null(),
                count: 65536,
                assertions: false,
            },
            8,
        );
        let mut shared = SHARED;
        shared.map = crate::map_func::fixture::statics();
        let mut s = CARGO_STORAGE;
        s.packet = packet_of;
        s.destroy = destroy_packet;
        s.shared = &raw const shared;
        for size in [1000, 4000, 16000, 64000] {
            check_vehicle(&s, size);
            check_station(&s, size);
        }
    }
}
