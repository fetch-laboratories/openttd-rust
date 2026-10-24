/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Rail vehicle construction and depot consist transactions: the original
//! `train_cmd.cpp` bodies from `GetFreeWagonsInDepot` through `CmdSellRailWagon`.
//! Native shells keep their chain links and fields; Rust owns the control flow,
//! the temporary backup lists and the free-wagon order. Every shell access is a
//! short typed service call; none spans a callback, nested move or deletion.
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]
use super::{CommonServices, ERROR, TransactionServices, build, construct, refit};
use crate::command_type::DoCommandFlag;
use crate::fleet::{DEFAULT_GROUP, GroupServices, INVALID_VEHICLE};
use crate::ground_vehicle as gv;
use crate::services::{CommandCost, Cost};
use crate::tile_type::TILE_SIZE;
use crate::vehicle::{CALLBACK_FAILED, OpenTTDEngine, RailVehicleTypes, VehicleRef};
use crate::vehicle_base::{is_articulated_part, is_front_engine};
use std::mem::MaybeUninit;
use std::ptr;

/// Original error strings of this component not already in a shared table.
#[repr(C)]
pub struct Strings {
    pub rear_follow: u32,
    pub depot_only: u32,
    pub no_orders: u32,
}
#[repr(C)]
pub struct Services {
    pub cargo_capacity: &'static crate::cargo_storage::CapacityServices,
    pub strings: Strings,
    pub compatible: extern "C" fn(*const OpenTTDEngine, u32) -> bool,
    pub powered: extern "C" fn(*const OpenTTDEngine, u32) -> bool,
    pub new_wagon: extern "C" fn(u32, *const OpenTTDEngine) -> *mut (),
    pub new_engine: extern "C" fn(u32, *const OpenTTDEngine) -> *mut (),
    pub new_rear: extern "C" fn(*mut ()) -> *mut (),
    pub link_multihead: extern "C" fn(*mut (), *mut ()),
    pub subtype: extern "C" fn(*mut ()) -> u8,
    pub set_subtype: extern "C" fn(*mut (), u8),
    pub previous: extern "C" fn(*mut ()) -> *mut (),
    pub last: extern "C" fn(*mut ()) -> *mut (),
    pub set_next: extern "C" fn(*mut (), *mut ()),
    pub next_vehicle: extern "C" fn(*mut ()) -> *mut (),
    pub last_engine_part: extern "C" fn(*mut ()) -> *mut (),
    pub ground: &'static crate::vehicle::GroundServices,
    pub first_engine: extern "C" fn(*mut ()) -> u16,
    pub invalidate_grf: extern "C" fn(*mut ()),
    pub unknown_attach: extern "C" fn(*mut (), u16),
    pub train_limit: extern "C" fn() -> u16,
    pub unit: extern "C" fn(*mut ()) -> u16,
    pub use_unit: extern "C" fn(*mut (), u16),
    pub release_unit: extern "C" fn(*mut ()),
    pub head_windows: extern "C" fn(*mut ()),
    pub front_windows: extern "C" fn(*mut ()),
    pub company_dirty: extern "C" fn(),
    pub depot_changed: extern "C" fn(u32),
}

/// `CCF_ARRANGE`: `ConsistChangeFlag::Length | ConsistChangeFlag::Capacity`.
const CCF_ARRANGE: u8 = crate::train::CCF_ARRANGE;
const VEH_TRAIN: u8 = 0;

/// Resolved immutable service tables; holds no world or owner borrow.
#[derive(Clone, Copy)]
struct Ctx {
    s: &'static Services,
    b: &'static build::Services,
    r: &'static refit::Services,
    c: &'static CommonServices,
    t: &'static TransactionServices,
    g: &'static GroupServices,
    w: &'static crate::fleet::transactions::Services,
}
impl Ctx {
    fn new(b: &'static build::Services) -> Self {
        let s = b.rail;
        let r = b.refit;
        let c = r.common;
        Self {
            s,
            b,
            r,
            c,
            t: c.base,
            g: c.base.groups,
            w: c.transactions,
        }
    }
    // Every helper below is one synchronous native leaf over a live shell.
    fn subtype(self, v: *mut ()) -> u8 {
        (self.s.subtype)(v)
    }
    fn set_subtype(self, v: *mut (), f: fn(u8) -> u8) {
        (self.s.set_subtype)(v, f(self.subtype(v)));
    }
    fn is_engine(self, v: *mut ()) -> bool {
        gv::is_engine(self.subtype(v))
    }
    fn is_front(self, v: *mut ()) -> bool {
        is_front_engine(true, self.subtype(v))
    }
    fn is_articulated(self, v: *mut ()) -> bool {
        is_articulated_part(true, self.subtype(v))
    }
    fn is_multiheaded(self, v: *mut ()) -> bool {
        gv::is_multiheaded(self.subtype(v))
    }
    fn is_rear(self, v: *mut ()) -> bool {
        gv::is_rear_dualheaded(self.subtype(v))
    }
    fn length(self, v: *mut ()) -> u8 {
        (self.s.ground.cached_length)(unsafe { super::vehicle_ref(v) })
    }
    fn next(self, v: *mut ()) -> *mut () {
        unsafe { (self.g.next_part)(v) }
    }
    fn first(self, v: *mut ()) -> *mut () {
        unsafe { (self.w.first)(v) }
    }
    fn set_next(self, v: *mut (), next: *mut ()) {
        (self.s.set_next)(v, next);
    }
    fn id(self, v: *mut ()) -> u32 {
        unsafe { (self.g.vehicle_id)(v) }
    }
    fn engine(self, v: *mut ()) -> u16 {
        unsafe { (self.g.vehicle_engine)(v) }
    }
    fn tile(self, v: *mut ()) -> u32 {
        (self.c.vehicles.tile)(unsafe { super::vehicle_ref(v) })
    }
    fn assert(self, condition: impl FnOnce() -> bool) {
        if self.w.assertions {
            unsafe { (self.w.check)(condition()) };
        }
    }
    /// `Train::GetIfValid`.
    fn train(self, id: u32) -> *mut () {
        let v = unsafe { (self.g.vehicle)(id) };
        if v.is_null() || unsafe { (self.g.vehicle_type)(v) } != VEH_TRAIN {
            return ptr::null_mut();
        }
        v
    }
    fn consist_changed(self, v: *mut ()) {
        unsafe {
            crate::train::openttd_rust_train_consist_changed(
                (self.r.train_handle)(v.cast()),
                CCF_ARRANGE,
                self.r.train,
                self.t.shared,
            );
        }
    }
    fn update_group(self, v: *mut ()) {
        unsafe { crate::fleet::openttd_rust_fleet_update_train_group(self.g, v) };
    }
    fn count_vehicle(self, v: *mut (), delta: i32) {
        unsafe { crate::fleet::openttd_rust_fleet_count_vehicle(self.g, v, delta) };
    }
    /// `TestVehicleBuildProbability(v, v->engine_type, Reversed)` and its flag.
    fn reverse_on_build(self, v: *mut ()) {
        let a = self.b.construct.articulated;
        let probability = (a.reverse)(v, self.engine(v));
        if probability.present {
            (self.r.train_handle)(v.cast()).set_flag(crate::train::FLIPPED, probability.value);
        }
    }
    /// `Vehicle::orders` of the canonical Rust-owned order view.
    fn orders(self, v: *mut ()) -> *mut std::ffi::c_void {
        unsafe { (*(self.c.orders.vehicle)(v.cast())).orders }
    }
}

/// Original `GetFreeWagonsInDepot`: IDs sorted for consistency across clients.
fn free_wagons(x: Ctx, tile: u32) -> Vec<u32> {
    struct Visit {
        x: Ctx,
        free: Vec<u32>,
    }
    unsafe extern "C" fn visit(ctx: *mut std::ffi::c_void, handle: VehicleRef) {
        // SAFETY: synchronous traversal; only the caller-local list is mutated.
        let c = unsafe { &mut *ctx.cast::<Visit>() };
        let v = handle.0.as_ptr().cast();
        if unsafe { (c.x.g.vehicle_type)(v) } != VEH_TRAIN {
            return;
        }
        if unsafe { (c.x.w.crashed)(v) } {
            return;
        }
        if !gv::is_free_wagon(c.x.subtype(v)) {
            return;
        }
        c.free.push(c.x.id(v));
    }
    let mut ctx = Visit {
        x,
        free: Vec::new(),
    };
    unsafe {
        x.c.vehicles
            .for_each_on_tile(tile, |v| visit(ptr::from_mut(&mut ctx).cast(), v));
    }
    ctx.free.sort_unstable();
    ctx.free
}

/// `Command<CMD_MOVE_RAIL_VEHICLE>::Do` from a Rust caller, by vehicle ID.
pub(crate) unsafe fn nested_move(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: u16,
    src: u32,
    dest: u32,
    chain: bool,
) -> bool {
    let t = b.refit.common.base;
    crate::command::nested(
        t.commands,
        t.shared,
        Cost(out, t.cost),
        0,
        flags,
        |f| unsafe {
            move_vehicle(b, out, f, src, dest, chain);
        },
    );
    (t.cost.success)(out)
}

/// Original `CmdBuildRailWagon`. `out` doubles as the scratch result of the
/// nested free-chain moves; the original returns a fresh zero cost after them.
unsafe fn build_wagon(
    x: Ctx,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    e: *const OpenTTDEngine,
) -> *mut () {
    if !(x.s.compatible)(e, tile) {
        (x.t.cost.error)(out, ERROR);
        return ptr::null_mut();
    }
    let mut v = ptr::null_mut();
    if flags & DoCommandFlag::EXECUTE != 0 {
        let a = x.b.construct.articulated;
        v = (x.s.new_wagon)(tile, e);
        (a.random_bits)(v, x.t.shared.random() as u16);
        unsafe { (x.g.set_membership)(v, DEFAULT_GROUP) };
        x.reverse_on_build(v);
        unsafe { super::articulated::add(a, v) };
        unsafe {
            a.position(v);
        }
        let first = x.first(v);
        x.consist_changed(first);
        x.update_group(x.first(v));
        unsafe { super::articulated::consistency(a, v) };
        for vehicle in free_wagons(x, tile) {
            if vehicle == x.id(v) {
                continue;
            }
            let w = unsafe { (x.g.vehicle)(vehicle) };
            if x.engine(w) != x.engine(v) {
                continue;
            }
            if x.first(w) == v {
                continue;
            }
            let last = x.id((x.s.last)(w));
            if unsafe { nested_move(x.b, out, DoCommandFlag::EXECUTE, x.id(v), last, true) } {
                crate::witness::hit(crate::witness::VT_RAIL_WAGON_JOIN);
                break;
            }
        }
    }
    (x.t.cost.zero)(out);
    v
}

/// Original `NormalizeTrainVehInDepot`; `out` is a free caller-owned result.
pub(crate) unsafe fn normalize(b: &'static build::Services, out: *mut CommandCost, u: *mut ()) {
    let x = Ctx::new(b);
    x.assert(|| x.is_engine(u));
    for vehicle in free_wagons(x, x.tile(u)) {
        if !unsafe { nested_move(b, out, DoCommandFlag::EXECUTE, vehicle, x.id(u), true) } {
            break;
        }
        crate::witness::hit(crate::witness::VT_RAIL_NORMALIZE);
    }
}

/// Original `AddRearEngineToMultiheadedTrain`.
fn add_rear_engine(x: Ctx, v: *mut ()) {
    let a = x.b.construct.articulated;
    let u = (x.s.new_rear)(v);
    (a.random_bits)(u, x.t.shared.random() as u16);
    x.set_subtype(v, gv::set_multiheaded);
    x.set_subtype(u, gv::set_multiheaded);
    x.set_next(v, u);
    x.reverse_on_build(u);
    unsafe {
        a.position(u);
    }
    (x.s.link_multihead)(v, u);
}

/// Original `CmdBuildRailVehicle`.
/// # Safety
/// Live native result and engine; new shells stay live and no shell access
/// spans a callback.
pub(crate) unsafe fn build(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    e: *const OpenTTDEngine,
) -> *mut () {
    let x = Ctx::new(b);
    let s = x.s;
    let kind = (x.r.rail_kind)(e);
    if kind == RailVehicleTypes::WAGON {
        return unsafe { build_wagon(x, out, flags, tile, e) };
    }
    if !(s.powered)(e, tile) {
        (x.t.cost.error)(out, ERROR);
        return ptr::null_mut();
    }
    let mut v = ptr::null_mut();
    if flags & DoCommandFlag::EXECUTE != 0 {
        let k: &construct::Services = x.b.construct;
        let a = k.articulated;
        let company = (x.t.shared.current_company)();
        v = (s.new_engine)(tile, e);
        (a.random_bits)(v, x.t.shared.random() as u16);
        (k.service_flags)(v, (k.exclusive_preview)(e), (x.t.interval_percent)(company));
        unsafe { (x.g.set_membership)(v, DEFAULT_GROUP) };
        x.set_subtype(v, gv::set_front_engine);
        x.set_subtype(v, gv::set_engine);
        x.reverse_on_build(v);
        unsafe {
            a.position(v);
        }
        if kind == RailVehicleTypes::MULTIHEAD {
            crate::witness::hit(crate::witness::VT_RAIL_MULTIHEAD);
            add_rear_engine(x, v);
        } else {
            unsafe { super::articulated::add(a, v) };
        }
        x.consist_changed(v);
        x.update_group(v);
        unsafe { super::articulated::consistency(a, v) };
    }
    (x.t.cost.zero)(out);
    v
}

/// Original `FindGoodVehiclePos`.
fn find_good_position(x: Ctx, src: *mut ()) -> *mut () {
    let eng = x.engine(src);
    for vehicle in free_wagons(x, x.tile(src)) {
        let dst = unsafe { (x.g.vehicle)(vehicle) };
        let mut t = dst;
        while x.engine(t) == eng {
            t = x.next(t);
            if t.is_null() {
                return dst;
            }
        }
    }
    ptr::null_mut()
}

/// Original `MakeTrainBackup`: a local list, as the original's `TrainList`.
fn backup(x: Ctx, mut t: *mut ()) -> Vec<*mut ()> {
    let mut list = Vec::new();
    while !t.is_null() {
        list.push(t);
        t = x.next(t);
    }
    list
}
/// Original `RestoreTrainBackup`.
fn restore(x: Ctx, list: &[*mut ()]) {
    if list.is_empty() {
        return;
    }
    let mut prev: *mut () = ptr::null_mut();
    for &t in list {
        if prev.is_null() {
            // Make sure the head of the train is always the first in the chain.
            let previous = (x.s.previous)(t);
            if !previous.is_null() {
                x.set_next(previous, ptr::null_mut());
            }
        } else {
            x.set_next(prev, t);
        }
        prev = t;
    }
}
/// Original `RemoveFromConsist`.
fn remove_from_consist(x: Ctx, part: *mut (), chain: bool) {
    let tail = if chain {
        (x.s.last)(part)
    } else {
        (x.s.last_engine_part)(part)
    };
    let previous = (x.s.previous)(part);
    if !previous.is_null() {
        x.set_next(previous, x.next(tail));
    }
    x.set_next(tail, ptr::null_mut());
}
/// Original `InsertInConsist`.
fn insert_in_consist(x: Ctx, dst: *mut (), chain: *mut ()) {
    // We do not want to add something in the middle of an articulated part.
    x.assert(|| {
        !dst.is_null() && {
            let next = x.next(dst);
            next.is_null() || !x.is_articulated(next)
        }
    });
    x.set_next((x.s.last)(chain), x.next(dst));
    x.set_next(dst, chain);
}
/// Original `NormaliseDualHeads`.
fn normalise_dual_heads(x: Ctx, mut t: *mut ()) {
    while !t.is_null() {
        if x.is_multiheaded(t) && x.is_engine(t) {
            let mut u = t;
            loop {
                let next = x.next(u);
                if next.is_null() || x.is_engine(next) {
                    break;
                }
                u = next;
            }
            let other = (x.r.selection.other_multihead)(t);
            if u != other {
                remove_from_consist(x, other, false);
                crate::witness::hit(crate::witness::VT_RAIL_DUAL_HEADS);
                insert_in_consist(x, u, other);
            }
        }
        t = (x.s.next_vehicle)(t);
    }
}
/// Original `NormaliseSubtypes`.
fn normalise_subtypes(x: Ctx, chain: *mut ()) {
    if chain.is_null() {
        return;
    }
    x.assert(|| (x.s.previous)(chain).is_null());
    if gv::is_wagon(x.subtype(chain)) {
        x.set_subtype(chain, gv::set_free_wagon);
    } else {
        x.assert(|| x.is_engine(chain));
        x.set_subtype(chain, gv::set_front_engine);
    }
    let mut t = x.next(chain);
    while !t.is_null() {
        x.set_subtype(t, gv::clear_free_wagon);
        x.set_subtype(t, gv::clear_front_engine);
        t = x.next(t);
    }
}

/// Original `CheckNewTrain`; `Some` is the `CommandCost(StringID)` error.
fn check_new_train(
    x: Ctx,
    original_dst: *mut (),
    dst: *mut (),
    original_src: *mut (),
    src: *mut (),
) -> Option<u32> {
    let engine = |v: *mut ()| i32::from(!v.is_null() && x.is_engine(v));
    if engine(src) + engine(dst) - engine(original_src) - engine(original_dst) <= 0 {
        return None;
    }
    if (x.b.free_unit)(crate::vehicle::VehicleType::TRAIN) <= (x.s.train_limit)() {
        return None;
    }
    Some(x.b.too_many)
}
/// Original `CheckTrainAttachment`. An error leaves the chain split, exactly
/// as the original; every caller restores its backup.
fn check_train_attachment(x: Ctx, mut t: *mut ()) -> Option<u32> {
    if t.is_null() || x.next(t).is_null() {
        return None;
    }
    // Unsigned `uint8_t * uint` arithmetic, then the original's `int` conversion.
    let mut allowed_len = (u32::from(unsafe { (x.w.max_length)() }) * TILE_SIZE)
        .wrapping_sub(u32::from(x.length(t)))
        .cast_signed();
    if !x.is_engine(t) {
        t = x.next(t);
        while !t.is_null() {
            allowed_len -= i32::from(x.length(t));
            t = x.next(t);
        }
        if allowed_len < 0 {
            return Some(x.w.too_long);
        }
        return None;
    }
    let head = t;
    let mut prev = t;
    t = x.next(t);
    x.set_next(prev, ptr::null_mut());
    (x.s.invalidate_grf)(head);
    let strings = &x.c.strings;
    while !t.is_null() {
        allowed_len -= i32::from(x.length(t));
        let next = x.next(t);
        x.set_next(t, ptr::null_mut());
        if !x.is_articulated(t) && !x.is_rear(t) {
            let first_engine = (x.s.first_engine)(t);
            (x.s.ground.write_first_engine)(
                unsafe { super::vehicle_ref(t) },
                crate::vehicle::INVALID_ENGINE,
            );
            (x.s.invalidate_grf)(t);
            let mut regs100 = MaybeUninit::<i32>::uninit();
            let callback = unsafe {
                crate::newgrf_vehicle::callback(
                    x.c.vehicles,
                    super::vehicle_ref(t),
                    (x.c.vehicles.engine_id)(super::vehicle_ref(head)),
                    crate::newgrf_callbacks::CBID_TRAIN_ALLOW_WAGON_ATTACH.0,
                    0,
                    head.cast(),
                    true,
                    regs100.as_mut_ptr(),
                    1,
                )
            };
            (x.s.ground.write_first_engine)(unsafe { super::vehicle_ref(t) }, first_engine);
            (x.s.invalidate_grf)(t);
            (x.s.invalidate_grf)(head);
            if callback != CALLBACK_FAILED {
                let mut error = strings.null_string;
                let grf_text = |n: u32| (x.c.grf_string)(head, strings.grf_text.0.wrapping_add(n));
                if (x.c.grf_version)(head) < 8 {
                    if callback == 0xFD {
                        error = strings.incompatible_rail;
                    }
                    if callback < 0xFD {
                        error = grf_text(u32::from(callback));
                    }
                    if callback >= 0x100 {
                        (x.s.unknown_attach)(head, callback);
                    }
                } else if callback < 0x400 {
                    error = grf_text(u32::from(callback));
                } else {
                    match callback {
                        0x400 | 0x401 => (),
                        0x40F => {
                            // The original reads register 100 only for this result.
                            error = (x.c.grf_string)(head, unsafe { regs100.assume_init() } as u32);
                        }
                        _ => error = strings.incompatible_rail,
                    }
                }
                if error != strings.null_string {
                    return Some(error);
                }
            }
        }
        x.set_next(prev, t);
        prev = t;
        t = next;
    }
    if allowed_len < 0 {
        return Some(x.w.too_long);
    }
    None
}
/// Original `ValidateTrains`.
fn validate_trains(
    x: Ctx,
    original_dst: *mut (),
    dst: *mut (),
    original_src: *mut (),
    src: *mut (),
    check_limit: bool,
) -> Option<u32> {
    // Source attachment errors take precedence over destination errors.
    let error = check_train_attachment(x, src);
    if error.is_some() {
        return error;
    }
    let error = check_train_attachment(x, dst);
    if error.is_some() {
        return error;
    }
    if check_limit {
        check_new_train(x, original_dst, dst, original_src, src)
    } else {
        None
    }
}
/// Original `ArrangeTrains`.
fn arrange_trains(
    x: Ctx,
    dst_head: &mut *mut (),
    dst: *mut (),
    src_head: &mut *mut (),
    src: *mut (),
    move_chain: bool,
) {
    if *src_head == *dst_head {
        *dst_head = ptr::null_mut();
    } else if dst_head.is_null() {
        *dst_head = src;
    }
    if src == *src_head {
        *src_head = if move_chain {
            ptr::null_mut()
        } else if x.is_multiheaded(src) {
            unsafe { (x.w.next_unit)(src) }
        } else {
            (x.s.next_vehicle)(src)
        };
    }
    remove_from_consist(x, src, move_chain);
    if *dst_head != src {
        insert_in_consist(x, dst, src);
    }
    normalise_dual_heads(x, *src_head);
    normalise_dual_heads(x, *dst_head);
}
/// Original `NormaliseTrainHead`.
fn normalise_train_head(x: Ctx, head: *mut ()) {
    if head.is_null() {
        return;
    }
    x.consist_changed(head);
    x.update_group(head);
    if !x.is_front(head) {
        return;
    }
    (x.s.head_windows)(head);
    if (x.s.unit)(head) != 0 {
        return;
    }
    (x.s.use_unit)(head, (x.b.free_unit)(crate::vehicle::VehicleType::TRAIN));
}
/// `new_head->orders = old->orders`, through the canonical order view.
fn take_orders(x: Ctx, new_head: *mut (), old: *mut ()) {
    let orders = x.orders(old);
    unsafe { (*(x.c.orders.vehicle)(new_head.cast())).orders = orders };
}
fn add_to_shared(x: Ctx, v: *mut (), chain: *mut ()) {
    unsafe { crate::orders::openttd_rust_add_shared(v.cast(), chain.cast(), x.c.orders) };
}
/// `DeleteVehicleOrders(v)` with its default arguments.
fn delete_orders(x: Ctx, v: *mut ()) {
    unsafe { crate::orders::openttd_rust_delete_vehicle_orders(v.cast(), 0, 1, x.c.orders) };
}
fn mark_dirty(x: Ctx, v: *mut ()) {
    unsafe {
        crate::train::openttd_rust_train_mark_dirty(
            (x.r.train_handle)(v.cast()),
            x.r.train,
            x.t.shared,
        );
    }
}

/// Original `CmdMoveRailVehicle`.
/// # Safety
/// Live caller-owned native result; vehicles stay live through the command.
pub(crate) unsafe fn move_vehicle(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: u16,
    src_veh: u32,
    dest_veh: u32,
    move_chain: bool,
) {
    let x = Ctx::new(b);
    let s = x.s;
    let cost = x.t.cost;
    let mut src = x.train(src_veh);
    if src.is_null() {
        return (cost.error)(out, ERROR);
    }
    unsafe { (x.t.commands.ownership)(out, (x.g.vehicle_owner)(src), 0) };
    if !(cost.success)(out) {
        return;
    }
    if unsafe { (x.w.crashed)(src) } {
        return (cost.error)(out, ERROR);
    }
    let mut dst;
    if dest_veh == INVALID_VEHICLE {
        dst = if x.is_engine(src) || flags & DoCommandFlag::AUTO_REPLACE != 0 {
            ptr::null_mut()
        } else {
            find_good_position(x, src)
        };
    } else {
        dst = x.train(dest_veh);
        if dst.is_null() {
            return (cost.error)(out, ERROR);
        }
        unsafe { (x.t.commands.ownership)(out, (x.g.vehicle_owner)(dst), 0) };
        if !(cost.success)(out) {
            return;
        }
        if unsafe { (x.w.crashed)(dst) } {
            return (cost.error)(out, ERROR);
        }
    }
    let first_part = x.r.selection.first_engine_part;
    src = first_part(src);
    if !dst.is_null() {
        dst = first_part(dst);
    }
    if src == dst {
        return (cost.zero)(out);
    }
    let mut src_head = x.first(src);
    let mut dst_head;
    if dst.is_null() {
        dst_head = ptr::null_mut();
    } else {
        dst_head = x.first(dst);
        if x.tile(dst_head) != x.tile(src_head) {
            return (cost.error)(out, ERROR);
        }
        dst = (s.last_engine_part)(dst);
    }
    if x.is_rear(src) {
        return (cost.error)(out, s.strings.rear_follow);
    }
    if move_chain && src_head == dst_head {
        return (cost.zero)(out);
    }
    if !move_chain
        && !dst.is_null()
        && x.is_rear(dst)
        && src == (x.r.selection.other_multihead)(dst)
    {
        return (cost.zero)(out);
    }
    if !unsafe { (x.w.stopped_in_depot)(src_head) } {
        return (cost.error)(out, s.strings.depot_only);
    }
    if !dst_head.is_null() && !unsafe { (x.w.stopped_in_depot)(dst_head) } {
        return (cost.error)(out, s.strings.depot_only);
    }
    let original_src = backup(x, src_head);
    let original_dst = backup(x, dst_head);
    let original_src_head = src_head;
    let original_dst_head = if dst_head == src_head {
        ptr::null_mut()
    } else {
        dst_head
    };
    let original_src_head_front_engine = x.is_front(original_src_head);
    let original_dst_head_front_engine =
        !original_dst_head.is_null() && x.is_front(original_dst_head);
    arrange_trains(x, &mut dst_head, dst, &mut src_head, src, move_chain);
    // Autoreplace reverts to the original, assumed valid, state.
    if flags & DoCommandFlag::AUTO_REPLACE == 0
        && let Some(error) = validate_trains(
            x,
            original_dst_head,
            dst_head,
            original_src_head,
            src_head,
            true,
        )
    {
        crate::witness::hit(crate::witness::VT_RAIL_MOVE_RESTORE);
        restore(x, &original_src);
        restore(x, &original_dst);
        return (cost.error)(out, error);
    }
    if flags & DoCommandFlag::EXECUTE == 0 {
        restore(x, &original_src);
        restore(x, &original_dst);
        return (cost.zero)(out);
    }
    if original_src_head_front_engine {
        x.count_vehicle(original_src_head, -1);
    }
    if original_dst_head_front_engine {
        x.count_vehicle(original_dst_head, -1);
    }
    normalise_subtypes(x, src_head);
    normalise_subtypes(x, dst_head);
    if src == original_src_head && x.is_engine(src) && !x.is_front(src) {
        // Cases #2 and #3: the front engine gets trashed.
        crate::witness::hit(crate::witness::VT_RAIL_FRONT_TRASHED);
        (s.front_windows)(src);
        if !src_head.is_null() && x.is_front(src_head) {
            crate::witness::hit(crate::witness::VT_RAIL_HEAD_TRANSFER);
            take_orders(x, src_head, src);
            if !x.orders(src_head).is_null() {
                add_to_shared(x, src_head, src);
            }
            unsafe { (x.w.copy_configuration)(src, src_head) };
        }
        delete_orders(x, src);
        (s.release_unit)(src);
        (x.t.clear_name)(src);
    }
    if original_src_head != src && dst_head == src {
        crate::witness::hit(crate::witness::VT_RAIL_NEW_FRONT);
        unsafe { crate::fleet::openttd_rust_fleet_set_train_group(x.g, src, DEFAULT_GROUP) };
        (s.company_dirty)();
    }
    normalise_train_head(x, src_head);
    normalise_train_head(x, dst_head);
    if !src_head.is_null() && x.is_front(src_head) {
        x.count_vehicle(src_head, 1);
    }
    if !dst_head.is_null() && x.is_front(dst_head) {
        x.count_vehicle(dst_head, 1);
    }
    if flags & DoCommandFlag::NO_CARGO_CAPACITY_CHECK == 0 {
        unsafe {
            crate::cargo_storage::openttd_rust_cargo_capacity(
                s.cargo_capacity,
                src_head.cast(),
                ptr::null_mut(),
                0,
                0,
            );
            crate::cargo_storage::openttd_rust_cargo_capacity(
                s.cargo_capacity,
                dst_head.cast(),
                ptr::null_mut(),
                0,
                0,
            );
        }
    }
    if !src_head.is_null() {
        mark_dirty(x, x.first(src_head));
    }
    if !dst_head.is_null() {
        mark_dirty(x, x.first(dst_head));
    }
    (s.depot_changed)(x.tile(src));
    (cost.zero)(out);
}

/// Original `CmdSellRailWagon`.
/// # Safety
/// Live caller-owned native result and train shell; the sold chain is deleted
/// last, after every shell access.
pub(crate) unsafe fn sell(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: u16,
    t: *mut (),
    sell_chain: bool,
    backup_order: bool,
    user: u32,
) {
    let x = Ctx::new(b);
    let s = x.s;
    let cost = x.t.cost;
    let v = (x.r.selection.first_engine_part)(t);
    let first = x.first(v);
    if x.is_rear(v) {
        return (cost.error)(out, s.strings.rear_follow);
    }
    let original = backup(x, first);
    let mut new_head = first;
    let mut sell_head = ptr::null_mut();
    arrange_trains(
        x,
        &mut sell_head,
        ptr::null_mut(),
        &mut new_head,
        v,
        sell_chain,
    );
    if let Some(error) = validate_trains(
        x,
        ptr::null_mut(),
        ptr::null_mut(),
        first,
        new_head,
        flags & DoCommandFlag::AUTO_REPLACE == 0,
    ) {
        crate::witness::hit(crate::witness::VT_RAIL_SELL_RESTORE);
        restore(x, &original);
        return (cost.error)(out, error);
    }
    if x.orders(first).is_null() && (x.c.orders.list_capacity)() == 0 {
        restore(x, &original);
        return (cost.error)(out, s.strings.no_orders);
    }
    (cost.expense_money)(out, x.c.strings.new_vehicle_expense, 0);
    let mut part = sell_head;
    while !part.is_null() {
        (cost.amount)(out, (x.c.value)(part).saturating_neg());
        part = x.next(part);
    }
    if flags & DoCommandFlag::EXECUTE == 0 {
        restore(x, &original);
        return;
    }
    normalise_subtypes(x, new_head);
    if v == first && !sell_chain && !new_head.is_null() && x.is_front(new_head) {
        if x.is_engine(v) {
            crate::witness::hit(crate::witness::VT_RAIL_SELL_TRANSFER);
            take_orders(x, new_head, first);
            add_to_shared(x, new_head, first);
            delete_orders(x, first);
            unsafe { (x.w.copy_configuration)(first, new_head) };
        }
        x.count_vehicle(new_head, 1);
    } else if unsafe { (x.g.primary)(v) } && backup_order {
        unsafe { crate::orders::openttd_rust_backup_create(v.cast(), user, x.c.orders) };
    }
    normalise_train_head(x, new_head);
    (s.depot_changed)(x.tile(v));
    // No shell access follows the deleting callback.
    (x.c.vehicles.destroy)(unsafe { super::vehicle_ref(sell_head) });
}

/// Command entry for `CmdMoveRailVehicle`.
/// # Safety
/// As `move_vehicle`; the static table outlives the call. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_move(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: u16,
    src_veh: u32,
    dest_veh: u32,
    move_chain: bool,
) {
    unsafe { move_vehicle(b, out, flags, src_veh, dest_veh, move_chain) };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRailConstructionServices" ["probe"] => Services {
        cargo_capacity, strings, compatible, powered, new_wagon, new_engine, new_rear, link_multihead, subtype,
        set_subtype, previous, last, set_next, next_vehicle, last_engine_part, ground, first_engine,
        invalidate_grf, unknown_attach, train_limit, unit,
        use_unit, release_unit, head_windows, front_windows, company_dirty, depot_changed,
    };
    "OpenTTDRailConstructionStrings" ["probe"] => Strings { rear_follow, depot_only, no_orders, };
}
