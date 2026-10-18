/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Canonical cargo packet fields, ordered lists, metadata and cargo actions.
//! Pool allocation and identity remain native. All raw owner accesses are short:
//! payment can call getters, so no reference into an owner survives any service.
//! Save import stores unresolved shell handles without dereferencing them. The
//! complete safety/lifetime contract is in `cargo_storage_ffi.h`; panics abort.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;

/// Native cargo Source, passed by value without packing its ID and type.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Source {
    pub(crate) id: u16,
    pub(crate) source_type: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Packet {
    pub(crate) feeder_share: i64,
    pub(crate) source_xy: u32,
    pub(crate) count: u16,
    pub(crate) periods_in_transit: u16,
    pub(crate) first_station: u16,
    pub(crate) next_hop: u16,
    pub(crate) source_id: u16,
    pub(crate) travelled_x: i16,
    pub(crate) travelled_y: i16,
    pub(crate) source_type: u8,
    pub(crate) in_vehicle: u8,
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
    pub(crate) random: extern "C" fn(u32) -> u32,
    pub(crate) coordinate: extern "C" fn(u32, u8) -> u32,
    pub(crate) flow:
        extern "C" fn(*const c_void, u8, u16, u16, u16, *const u16, usize, *mut u8) -> u16,
    pub(crate) pay: extern "C" fn(*mut c_void, u8, u8, *const c_void, u32, u32) -> i64,
    pub(crate) origin: extern "C" fn(*mut c_void, u16, u32, u8),
    pub(crate) flow_owner: extern "C" fn(*const c_void, u16) -> *mut crate::cargo_flow::Flow,
    pub(crate) random_draw: extern "C" fn() -> u32,
    pub(crate) packet_next: extern "C" fn(*mut c_void) -> *mut c_void,
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
    fields: Fields,
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
const TRANSFER: usize = 0;
const DELIVER: usize = 1;
const KEEP: usize = 2;
const LOAD: usize = 3;
const INVALID: u16 = u16::MAX;

fn share(p: Packet, amount: u32) -> i64 {
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
    unsafe {
        (*p).feeder_share = old.feeder_share.saturating_sub(share(old, amount));
        (*p).count = old.count.wrapping_sub(amount as u16);
    }
}
unsafe fn tile(p: *mut Packet, s: Services, at: u32, loading: bool) {
    let x = (s.coordinate)(at, 0) as i16;
    let y = (s.coordinate)(at, 1) as i16;
    unsafe {
        if loading {
            if (*p).source_xy == u32::MAX {
                (*p).source_xy = at;
            }
            (*p).in_vehicle = 1;
            (*p).travelled_x = (*p).travelled_x.wrapping_add(x);
            (*p).travelled_y = (*p).travelled_y.wrapping_add(y);
        } else {
            (*p).in_vehicle = 0;
            (*p).travelled_x = (*p).travelled_x.wrapping_sub(x);
            (*p).travelled_y = (*p).travelled_y.wrapping_sub(y);
        }
    }
}
unsafe fn fields(l: *const List) -> Fields {
    unsafe { (*l).fields }
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
fn mergeable(a: Packet, b: Packet) -> bool {
    a.source_xy == b.source_xy
        && a.periods_in_transit == b.periods_in_transit
        && a.first_station == b.first_station
        && a.source_id == b.source_id
        && a.source_type == b.source_type
        && u16::try_from(u32::from(a.count) + u32::from(b.count)).is_ok()
}
unsafe fn merge(p: *mut Packet, s: Services, other: *mut c_void) {
    let b = unsafe { packet(s, other) };
    unsafe {
        (*p).count = (*p).count.wrapping_add(b.count);
        (*p).feeder_share = (*p).feeder_share.saturating_add(b.feeder_share);
    }
    (s.destroy)(other);
}
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
        openttd_rust_flow_change, openttd_rust_flow_clone, openttd_rust_flow_destroy,
        openttd_rust_flow_read, openttd_rust_flow_via,
    };
    let original = (s.flow_owner)(ge, origin);
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
        unsafe { openttd_rust_flow_via(flow, 0, 0, 0, &raw mut restricted, s.random_draw) }
    };
    unsafe {
        openttd_rust_flow_destroy(flow);
    }
    via
}
fn choose(p: Packet, via: u16, station: u16, accepted: bool, next: NextHops) -> usize {
    if via == INVALID {
        if accepted && p.first_station != station {
            DELIVER
        } else {
            KEEP
        }
    } else if via == station {
        DELIVER
    } else if unsafe { next.contains(via) } {
        KEEP
    } else {
        TRANSFER
    }
}
unsafe fn reassign(
    list: *mut List,
    services: Services,
    from: usize,
    to: usize,
    amount: u32,
) -> u32 {
    let meta = unsafe { fields(list) };
    let amount = amount.min(meta.action_counts[from]);
    if from == DELIVER && to == TRANSFER {
        let mut sum = 0_u32;
        let mut node = unsafe { first(list) };
        while sum < meta.action_counts[TRANSFER].wrapping_add(amount) {
            let current = node.unwrap();
            let next = unsafe { after(list, current.token) };
            let owner = (services.packet)(current.shell);
            sum = sum.wrapping_add(u32::from(unsafe { read(owner) }.count));
            if sum > meta.action_counts[TRANSFER] {
                if sum > meta.action_counts[TRANSFER].wrapping_add(amount) {
                    // Historical C++ expression adds max_move (rather than subtracts).
                    let part = unsafe {
                        split(
                            owner,
                            services,
                            sum.wrapping_sub(meta.action_counts[TRANSFER])
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
        (*list).fields.action_counts[from] =
            (*list).fields.action_counts[from].wrapping_sub(amount);
        (*list).fields.action_counts[to] = (*list).fields.action_counts[to].wrapping_add(amount);
    }
    amount
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_new(p: *const Packet) -> *mut Packet {
    Box::into_raw(Box::new(unsafe { read(p) }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_destroy(p: *mut Packet) {
    unsafe {
        drop(Box::from_raw(p));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_export(p: *const Packet, out: *mut Packet) {
    unsafe {
        out.write(read(p));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_import(p: *mut Packet, input: *const Packet) {
    unsafe {
        p.write(read(input));
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_set(p: *mut Packet, field: u8, value: u32) {
    unsafe {
        match field {
            0 => (*p).next_hop = value as u16,
            1 => (*p).first_station = value as u16,
            2 => (*p).source_id = value as u16,
            _ => unreachable!(),
        }
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
    let s = unsafe { *s };
    let x = (s.coordinate)(at, 0);
    let y = (s.coordinate)(at, 1);
    let dx = p.travelled_x.wrapping_sub(x as i16);
    let dy = p.travelled_y.wrapping_sub(y as i16);
    let travelled = i32::from(dx).unsigned_abs() + i32::from(dy).unsigned_abs();
    let direct =
        (s.coordinate)(p.source_xy, 0).abs_diff(x) + (s.coordinate)(p.source_xy, 1).abs_diff(y);
    travelled.min(direct)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_packet_split(
    p: *mut Packet,
    s: *const Services,
    amount: u32,
) -> *mut c_void {
    unsafe { split(p, *s, amount) }
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
pub unsafe extern "C" fn openttd_rust_cargo_packet_reduce(p: *mut Packet, amount: u32) {
    unsafe {
        reduce(p, amount);
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
            .wrapping_sub((*l).fields.action_counts[LOAD])
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_unload(l: *const List) -> u32 {
    unsafe { (*l).fields.action_counts[TRANSFER].wrapping_add((*l).fields.action_counts[DELIVER]) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_remaining(l: *const List) -> u32 {
    unsafe { (*l).fields.action_counts[KEEP].wrapping_add((*l).fields.action_counts[LOAD]) }
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_age(l: *mut List, s: *const Services) {
    unsafe { age::<false>(l, s) };
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_list_reassign(
    l: *mut List,
    s: *const Services,
    from: u8,
    to: u8,
    amount: u32,
) -> u32 {
    unsafe { reassign(l, *s, usize::from(from), usize::from(to), amount) }
}

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
        (*l).fields.action_counts[TRANSFER] = 0;
        (*l).fields.action_counts[DELIVER] = 0;
        (*l).fields.action_counts[KEEP] = 0;
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
            action = KEEP;
        } else if unload == 1 && accepted != 0 && observed.first_station != station {
            action = DELIVER;
        } else if unload == 1 || unload == 2 {
            action = TRANSFER;
            via = unsafe { forced_flow(s, ge, observed.first_station, station, next_slice) };
        } else {
            if observed.first_station == INVALID
                && (s.flow)(ge, 4, 0, 0, 0, std::ptr::null(), 0, &raw mut restricted) != 0
            {
                observed.first_station =
                    (s.flow)(ge, 3, 0, 0, 0, std::ptr::null(), 0, &raw mut restricted);
                unsafe {
                    (*p).first_station = observed.first_station;
                }
            }
            via = (s.flow)(
                ge,
                0,
                observed.first_station,
                0,
                0,
                std::ptr::null(),
                0,
                &raw mut restricted,
            );
            action = choose(observed, via, station, accepted != 0, next_slice);
            if restricted != 0 && action == TRANSFER {
                via = (s.flow)(
                    ge,
                    5,
                    observed.first_station,
                    0,
                    0,
                    std::ptr::null(),
                    0,
                    &raw mut restricted,
                );
                action = choose(observed, via, station, accepted != 0, next_slice);
            }
        }
        match action {
            KEEP => {
                unsafe {
                    insert(l, current.shell, 0, None, false);
                }
                if deliver.is_none() {
                    deliver = unsafe { last(l).map(|n| n.token) };
                }
            }
            DELIVER => unsafe {
                insert(l, current.shell, 0, deliver, false);
            },
            TRANSFER => {
                unsafe {
                    insert(l, current.shell, 0, None, true);
                }
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
            (*l).fields.action_counts[action] =
                (*l).fields.action_counts[action].wrapping_add(count);
        }
        sum = sum.wrapping_add(count);
    }
    let f = unsafe { fields(l) };
    u8::from(f.action_counts[DELIVER] > 0 || f.action_counts[TRANSFER] > 0)
}

// Action selectors: 0 return,1 transfer,2 deliver,3 shift,4 remove vehicle,
// 5 reroute vehicle,6 reserve,7 load,8 reroute station,9 truncate station,
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
                Some(if mode == 2 { DELIVER } else { KEEP }),
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
                remove_cache(src, p, count, Some(if mode == 0 { LOAD } else { TRANSFER }));
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
            remove_cache(src, p, count, Some(KEEP));
            append(dst, s, cp, KEEP as u16);
        },
        5 => {
            if p.next_hop == avoid || p.next_hop == avoid2 {
                let mut restricted = 0;
                let via = (s.flow)(
                    ge,
                    2,
                    p.first_station,
                    avoid,
                    avoid2,
                    std::ptr::null(),
                    0,
                    &raw mut restricted,
                );
                // Original mutates cp, even when cp_new is a partial split.
                unsafe {
                    (*(s.packet)(n.shell)).next_hop = via;
                }
            }
            if src != dst {
                unsafe {
                    remove_cache(src, p, count, Some(TRANSFER));
                    add_cache(dst, p, Some(TRANSFER));
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
                    if mode == 6 { LOAD as u16 } else { KEEP as u16 },
                );
            }
        }
        8 => {
            let mut restricted = 0;
            let via = (s.flow)(
                ge,
                2,
                p.first_station,
                avoid,
                avoid2,
                std::ptr::null(),
                0,
                &raw mut restricted,
            );
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
unsafe fn truncate_station(l: *mut List, s: Services, max_move: u32, origins: *mut c_void) -> u32 {
    let max_move = max_move.min(unsafe { fields(l) }.count);
    let previous = unsafe { fields(l) }.count;
    let mut moved = 0_u32;
    let mut loop_count = 0_u32;
    while max_move > moved {
        let mut n = unsafe { first(l) };
        while let Some(current) = n {
            n = unsafe { after(l, current.token) };
            let p = unsafe { packet(s, current.shell) };
            if previous > max_move && (s.random)(previous) < previous - max_move {
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
    if mode == 9 {
        return unsafe { truncate_station(src, s, amount, payment) };
    }
    if mode == 10 {
        let mut moved = 0_u32;
        let f = unsafe { fields(src) };
        if f.action_counts[TRANSFER] > 0 {
            let amount = f.action_counts[TRANSFER].min(amount);
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
        if f.action_counts[TRANSFER] == 0 && f.action_counts[DELIVER] > 0 && moved < amount {
            let amount = f.action_counts[DELIVER].min(amount - moved);
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
            let reserved = unsafe { fields(dst) }.action_counts[LOAD].min(amount);
            if reserved > 0 {
                unsafe {
                    (*src).fields.reserved_count =
                        (*src).fields.reserved_count.wrapping_sub(reserved);
                    reassign(dst, s, LOAD, KEEP, reserved);
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
        0 => f.action_counts[LOAD],
        5 => f.action_counts[TRANSFER],
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
    pub(crate) capacity: u16,
    pub(crate) cargo: u8,
    pub(crate) train: u8,
    pub(crate) articulated: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CapacityServices {
    pub(crate) read: extern "C" fn(*mut c_void, *mut Vehicle),
    pub(crate) next_part: extern "C" fn(*mut c_void) -> *mut c_void,
    pub(crate) last_engine_part: extern "C" fn(*mut c_void) -> *mut c_void,
    pub(crate) other_multiheaded_part: extern "C" fn(*mut c_void) -> *mut c_void,
    pub(crate) cargo: *const Services,
}
fn vehicle(services: CapacityServices, shell: *mut c_void) -> Vehicle {
    let mut fields = Vehicle::default();
    (services.read)(shell, &raw mut fields);
    fields
}
/// The native facade invokes `ConsistChanged` only after this function returns:
/// it may reorder/destroy world parts and reenter their cargo owners.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_capacity(
    s: *const CapacityServices,
    old: *mut c_void,
    new: *mut c_void,
    mode: u8,
    part_chain: u8,
) -> u8 {
    let services = unsafe { *s };
    let cargo = unsafe { *services.cargo };
    let transfer = mode != 0;
    let mut src = old;
    while !src.is_null() {
        let source = vehicle(services, src);
        let count = unsafe { fields(source.list) }.count;
        if transfer
            && part_chain == 0
            && source.train != 0
            && src != old
            && src != (services.other_multiheaded_part)(old)
            && source.articulated == 0
        {
            src = (services.next_part)((services.last_engine_part)(src));
            continue;
        }
        if (!transfer && count <= u32::from(source.capacity))
            || (transfer && (source.cargo >= 64 || count == 0))
        {
            src = (services.next_part)(src);
            continue;
        }
        let mut spread = count.wrapping_sub(u32::from(source.capacity));
        let mut dest = if transfer { new } else { old };
        while !dest.is_null()
            && (if transfer {
                unsafe { fields(source.list) }.count > 0
            } else {
                spread != 0
            })
        {
            let destination = vehicle(services, dest);
            if transfer
                && part_chain == 0
                && destination.train != 0
                && dest != new
                && dest != (services.other_multiheaded_part)(new)
                && destination.articulated == 0
            {
                dest = (services.next_part)((services.last_engine_part)(dest));
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
            dest = (services.next_part)(dest);
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
        src = (services.next_part)(src);
    }
    u8::from(transfer && part_chain != 0 && vehicle(services, new).train != 0)
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
    extern "C" fn random(limit: u32) -> u32 {
        limit - 1
    }
    extern "C" fn coordinate(tile: u32, axis: u8) -> u32 {
        if axis == 0 { tile & 255 } else { tile >> 8 }
    }
    extern "C" fn flow(
        context: *const c_void,
        mode: u8,
        origin: u16,
        _: u16,
        _: u16,
        _: *const u16,
        _: usize,
        restricted: *mut u8,
    ) -> u16 {
        unsafe {
            *restricted = 0;
        }
        if mode == 2 {
            unsafe { *context.cast::<u16>() }
        } else {
            match origin {
                10 => 100,
                11 => 200,
                12 => 300,
                _ => INVALID,
            }
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
    extern "C" fn flow_owner(_: *const c_void, _: u16) -> *mut crate::cargo_flow::Flow {
        std::ptr::null_mut()
    }
    extern "C" fn draw() -> u32 {
        0
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
        Services {
            can_allocate,
            create,
            packet: owner,
            destroy,
            random,
            coordinate,
            flow,
            pay,
            origin,
            flow_owner,
            random_draw: draw,
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
    fn list(vehicle: bool) -> *mut List {
        openttd_rust_cargo_list_new(u8::from(vehicle))
    }
    unsafe fn clean(list: *mut List) {
        let service = services();
        unsafe {
            openttd_rust_cargo_list_destroy(list, &raw const service);
        }
    }

    #[test]
    fn partial_reserve_load_return_and_feeder_rounding() {
        let service = services();
        let station = list(false);
        let vehicle = list(true);
        unsafe {
            append(station, service, cargo(17, 10, 23), 100);
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
        unsafe {
            for first in [10, 11, 12] {
                append(vehicle, service, cargo(8, first, 0), KEEP as u16);
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
                    std::ptr::null(),
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
        }
    }
    #[test]
    fn same_list_reroute_and_partial_packet_identity() {
        let service = services();
        unsafe {
            for via in [10_u16, 90] {
                let station = list(false);
                append(station, service, cargo(8, 10, 0), 70);
                append(station, service, cargo(8, 11, 0), 70);
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
                        (&raw const via).cast(),
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
            }
            let vehicle = list(true);
            let original = cargo(8, 10, 0);
            (*owner(original)).next_hop = 70;
            append(vehicle, service, original, KEEP as u16);
            (*vehicle).fields.action_counts = [8, 0, 0, 0];
            let via = 90_u16;
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
                    (&raw const via).cast(),
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
        }
    }
    #[test]
    fn merge_age_invalidation_and_pool_failure_fallback() {
        let service = services();
        unsafe {
            let station = list(false);
            let vehicle = list(true);
            append(station, service, cargo(8, 10, 3), 100);
            append(station, service, cargo(9, 10, 4), 100);
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
            openttd_rust_cargo_list_age(vehicle, &raw const service);
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
        for (selector, action) in [(4, KEEP), (1, DELIVER), (2, TRANSFER)] {
            let vehicle = list(true);
            PAYMENT_LIST.with(|slot| slot.set(vehicle));
            unsafe {
                append(vehicle, service, cargo(8, 10, 0), KEEP as u16);
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
                assert_eq!(meta.action_counts[action], 8);
                assert_eq!(meta.action_counts.iter().sum::<u32>(), 8);
                clean(vehicle);
            }
        }
    }

    struct Part {
        id: u8,
        next: *mut Part,
        last: *mut Part,
        other: *mut Part,
        list: *mut List,
        cap: u16,
        cargo: u8,
        train: u8,
        articulated: u8,
    }
    extern "C" fn capacity_read(shell: *mut c_void, output: *mut Vehicle) {
        let part = unsafe { &*shell.cast::<Part>() };
        CAPACITY_CALLS.with(|calls| calls.borrow_mut().push(("read", part.id)));
        unsafe {
            output.write(Vehicle {
                list: part.list,
                capacity: part.cap,
                cargo: part.cargo,
                train: part.train,
                articulated: part.articulated,
            });
        }
    }
    extern "C" fn capacity_next_part(shell: *mut c_void) -> *mut c_void {
        let part = unsafe { &*shell.cast::<Part>() };
        CAPACITY_CALLS.with(|calls| calls.borrow_mut().push(("next", part.id)));
        part.next.cast()
    }
    extern "C" fn capacity_last_engine_part(shell: *mut c_void) -> *mut c_void {
        let part = unsafe { &*shell.cast::<Part>() };
        CAPACITY_CALLS.with(|calls| calls.borrow_mut().push(("last", part.id)));
        part.last.cast()
    }
    extern "C" fn capacity_other_multiheaded_part(shell: *mut c_void) -> *mut c_void {
        let part = unsafe { &*shell.cast::<Part>() };
        CAPACITY_CALLS.with(|calls| calls.borrow_mut().push(("other", part.id)));
        part.other.cast()
    }
    fn part(cap: u16) -> Part {
        Part {
            id: 0,
            next: std::ptr::null_mut(),
            last: std::ptr::null_mut(),
            other: std::ptr::null_mut(),
            list: list(true),
            cap,
            cargo: 0,
            train: 1,
            articulated: 0,
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
            next_part: capacity_next_part,
            last_engine_part: capacity_last_engine_part,
            other_multiheaded_part: capacity_other_multiheaded_part,
            cargo: &raw const service,
        };
        let mut source = std::array::from_fn::<_, 5, _>(|_| part(1));
        let mut destination = std::array::from_fn::<_, 5, _>(|_| part(1));
        for parts in [&mut source, &mut destination] {
            let base = parts.as_mut_ptr();
            for (index, entry) in parts.iter_mut().enumerate() {
                entry.id = index as u8 + 1;
                if index < 4 {
                    entry.next = unsafe { base.add(index + 1) };
                }
            }
            parts[0].other = unsafe { base.add(4) };
            parts[1].articulated = 1;
            parts[2].last = unsafe { base.add(3) };
            parts[3].articulated = 1;
        }
        for entry in &mut destination {
            entry.id += 10;
        }
        unsafe {
            for (index, entry) in source.iter().enumerate() {
                append(
                    entry.list,
                    service,
                    cargo(1, index as u16 + 10, 0),
                    KEEP as u16,
                );
            }
            CAPACITY_CALLS.with(|calls| calls.borrow_mut().clear());
            assert_eq!(
                openttd_rust_cargo_capacity(
                    &raw const capacity,
                    source.as_mut_ptr().cast(),
                    destination.as_mut_ptr().cast(),
                    1,
                    0,
                ),
                0
            );
            assert_eq!(
                source.each_ref().map(|entry| fields(entry.list).count),
                [0, 0, 1, 1, 0]
            );
            assert_eq!(
                destination.each_ref().map(|entry| fields(entry.list).count),
                [1, 1, 0, 0, 1]
            );
            CAPACITY_CALLS.with(|calls| {
                assert_eq!(
                    *calls.borrow(),
                    vec![
                        ("read", 1),
                        ("read", 11),
                        ("next", 11),
                        ("next", 1),
                        ("read", 2),
                        ("other", 1),
                        ("read", 11),
                        ("next", 11),
                        ("read", 12),
                        ("other", 11),
                        ("next", 12),
                        ("next", 2),
                        ("read", 3),
                        ("other", 1),
                        ("last", 3),
                        ("next", 4),
                        ("read", 5),
                        ("other", 1),
                        ("read", 11),
                        ("next", 11),
                        ("read", 12),
                        ("other", 11),
                        ("next", 12),
                        ("read", 13),
                        ("other", 11),
                        ("last", 13),
                        ("next", 14),
                        ("read", 15),
                        ("other", 11),
                        ("next", 15),
                        ("next", 5),
                    ]
                );
            });
            for entry in source.iter().chain(&destination) {
                clean(entry.list);
            }
        }
    }

    #[test]
    fn capacity_shrink_and_autoreplace_consist_transfer() {
        let service = services();
        let capacity = CapacityServices {
            read: capacity_read,
            next_part: capacity_next_part,
            last_engine_part: capacity_last_engine_part,
            other_multiheaded_part: capacity_other_multiheaded_part,
            cargo: &raw const service,
        };
        let mut source = part(3);
        let mut tail = part(6);
        source.next = &raw mut tail;
        unsafe {
            append(source.list, service, cargo(8, 10, 8), KEEP as u16);
            append(tail.list, service, cargo(1, 11, 1), KEEP as u16);
            assert_eq!(
                openttd_rust_cargo_capacity(
                    &raw const capacity,
                    (&raw mut source).cast(),
                    std::ptr::null_mut(),
                    0,
                    0
                ),
                0
            );
            assert_eq!(fields(source.list).count, 3);
            assert_eq!(fields(tail.list).count, 6);
            let mut replacement = part(4);
            let mut replacement_tail = part(4);
            replacement.next = &raw mut replacement_tail;
            assert_eq!(
                openttd_rust_cargo_capacity(
                    &raw const capacity,
                    (&raw mut source).cast(),
                    (&raw mut replacement).cast(),
                    1,
                    1
                ),
                1
            );
            assert_eq!(fields(source.list).count, 0);
            assert_eq!(fields(tail.list).count, 1);
            assert_eq!(fields(replacement.list).count, 4);
            assert_eq!(fields(replacement_tail.list).count, 4);
            replacement.cap = 2;
            replacement_tail.cargo = 1;
            openttd_rust_cargo_capacity(
                &raw const capacity,
                (&raw mut replacement).cast(),
                std::ptr::null_mut(),
                0,
                0,
            );
            assert_eq!(fields(replacement.list).count, 2);
            assert_eq!(fields(replacement_tail.list).count, 4);
            clean(source.list);
            clean(tail.list);
            clean(replacement.list);
            clean(replacement_tail.list);
        }
    }
}
