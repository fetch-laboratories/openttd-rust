/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Rail vehicle construction and depot consist transactions: the original `train_cmd.cpp`
//! bodies from `GetFreeWagonsInDepot` through `CmdSellRailWagon`. Records carry chain links
//! and moved fields; shells keep the rest. Rust owns control flow, the temporary backup lists
//! and the free-wagon order.
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]
use super::{CommonServices, ERROR, TransactionServices, build, construct, refit};
use crate::command_type::DoCommandFlag;
use crate::fleet::{DEFAULT_GROUP, GroupServices, INVALID_VEHICLE};
use crate::ground_vehicle as gv;
use crate::raw_field::{read_field, write_field};
use crate::services::{CommandCost, Cost};
use crate::tile_type::TILE_SIZE;
use crate::vehicle::OpenTTDVehicle;
use crate::vehicle::{CALLBACK_FAILED, OpenTTDEngine, RailVehicleTypes, VehicleRef};
use crate::vehicle_base::{is_articulated_part, is_front_engine};
use std::mem::MaybeUninit;
use std::ptr;

/// A nullable canonical record; native shell leaves receive `Ctx::sh`.
type R = *mut OpenTTDVehicle;

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
    pub train_if_valid: extern "C" fn(u32) -> *mut (),
    pub new_wagon: extern "C" fn(u32, *const OpenTTDEngine) -> *mut (),
    pub new_engine: extern "C" fn(u32, *const OpenTTDEngine) -> *mut (),
    pub engine_dates: extern "C" fn(*mut (), *const OpenTTDEngine),
    pub new_rear: extern "C" fn(*mut ()) -> *mut (),
    pub link_multihead: extern "C" fn(*mut (), *mut ()),
    pub invalidate_grf: extern "C" fn(*mut ()),
    pub unit: extern "C" fn(*mut ()) -> u16,
    pub use_unit: extern "C" fn(*mut (), u16),
    pub release_unit: extern "C" fn(*mut ()),
    pub head_windows: extern "C" fn(*mut ()),
    pub front_windows: extern "C" fn(*mut ()),
    pub company_dirty: extern "C" fn(),
    pub depot_changed: extern "C" fn(u32),
}

use crate::train::CCF_ARRANGE;
use crate::vehicle_type::VEH_TRAIN;

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
// Field reads keep the original member-call shape at each use.
#[allow(clippy::unused_self)]
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
    /// Native shell of a nullable record, for shell-keyed leaves.
    fn sh(v: R) -> *mut () {
        VehicleRef::shell_from_record(v)
    }
    /// Canonical record of a nullable shell returned by a native leaf.
    fn rec(self, shell: *mut ()) -> R {
        (self.c.vehicles.record_from_shell)(shell.cast())
    }
    fn subtype(self, v: R) -> u8 {
        read_field!(v, subtype)
    }
    fn set_subtype(self, v: R, f: fn(u8) -> u8) {
        write_field!(v, subtype, f(self.subtype(v)));
    }
    fn is_engine(self, v: R) -> bool {
        gv::is_engine(self.subtype(v))
    }
    fn is_front(self, v: R) -> bool {
        is_front_engine(true, self.subtype(v))
    }
    fn is_articulated(self, v: R) -> bool {
        is_articulated_part(true, self.subtype(v))
    }
    fn is_multiheaded(self, v: R) -> bool {
        gv::is_multiheaded(self.subtype(v))
    }
    fn is_rear(self, v: R) -> bool {
        gv::is_rear_dualheaded(self.subtype(v))
    }
    fn length(self, v: R) -> u8 {
        read_field!(Self::vref(v).ground_cache(), cached_veh_length)
    }
    fn next(self, v: R) -> R {
        read_field!(v, next)
    }
    fn previous(self, v: R) -> R {
        read_field!(v, previous)
    }
    fn first(self, v: R) -> R {
        read_field!(v, first)
    }
    /// `Vehicle::Last`.
    fn last(self, v: R) -> R {
        Self::vref(v).last().0.as_ptr()
    }
    /// `Vehicle::GetNextVehicle`.
    fn next_vehicle(self, v: R) -> R {
        Self::vref(v)
            .next_vehicle()
            .map_or(ptr::null_mut(), |v| v.0.as_ptr())
    }
    /// `GroundVehicle::GetLastEnginePart`.
    fn last_engine_part(self, v: R) -> R {
        Self::vref(v).last_engine_part().0.as_ptr()
    }
    /// A non-null live record.
    fn vref(v: R) -> VehicleRef {
        VehicleRef(ptr::NonNull::new(v).unwrap())
    }
    fn set_next(self, v: R, next: R) {
        // SAFETY: The transaction retains every linked record until explicit deletion.
        unsafe { self.c.vehicles.set_next_record(v, next) };
    }
    fn id(self, v: R) -> u32 {
        unsafe { (self.g.vehicle_id)(Self::sh(v)) }
    }
    fn engine(self, v: R) -> u16 {
        Self::vref(v).engine_id()
    }
    fn tile(self, v: R) -> u32 {
        Self::vref(v).tile()
    }
    fn assert(self, condition: impl FnOnce() -> bool) {
        if self.w.assertions {
            self.w.check(condition());
        }
    }
    /// `Train::GetIfValid`.
    fn train(self, id: u32) -> R {
        self.rec((self.s.train_if_valid)(id))
    }
    fn consist_changed(self, v: R) {
        unsafe {
            crate::train::openttd_rust_train_consist_changed(
                self.r.train_handle(v),
                CCF_ARRANGE,
                self.r.train,
                self.t.shared,
            );
        }
    }
    fn update_group(self, v: R) {
        unsafe { crate::fleet::openttd_rust_fleet_update_train_group(self.g, Self::sh(v)) };
    }
    fn count_vehicle(self, v: R, delta: i32) {
        unsafe { crate::fleet::openttd_rust_fleet_count_vehicle(self.g, Self::sh(v), delta) };
    }
    /// `TestVehicleBuildProbability(v, v->engine_type, Reversed)` and its flag.
    fn reverse_on_build(self, v: R) {
        let probability = unsafe {
            crate::newgrf_vehicle::openttd_rust_vehicle_build_probability(
                &*(self.c.vehicles.newgrf)(),
                v,
                self.engine(v),
                crate::newgrf_vehicle::BUILD_PROBABILITY_REVERSED,
            )
        };
        if probability >= 0 {
            self.r
                .train_handle(v)
                .set_flag(crate::train::FLIPPED, probability != 0);
        }
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
        let v = handle.0.as_ptr();
        if handle.vehicle_type() != VEH_TRAIN {
            return;
        }
        if handle.status() & crate::vehicle_base::VS_CRASHED != 0 {
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
) -> R {
    let railtypes = crate::vehicle::EngineRef::of(e).railtypes();
    let tiletype = (x.b.refit.train.physics.rail_type)(tile);
    if (x.t.shared.all_compatible_railtypes)(railtypes) & (1 << tiletype.0) == 0 {
        (x.t.cost.error)(out, ERROR);
        return ptr::null_mut();
    }
    let mut v = ptr::null_mut();
    if flags & DoCommandFlag::EXECUTE != 0 {
        let a = x.b.construct.articulated;
        v = x.rec((x.s.new_wagon)(tile, e));
        (a.random_bits)(Ctx::sh(v), x.t.shared.random() as u16);
        unsafe { (x.g.set_membership)(Ctx::sh(v), DEFAULT_GROUP) };
        x.reverse_on_build(v);
        unsafe { super::articulated::add(a, Ctx::sh(v)) };
        unsafe {
            a.position(Ctx::sh(v));
        }
        let first = x.first(v);
        x.consist_changed(first);
        x.update_group(x.first(v));
        unsafe { super::articulated::consistency(a, Ctx::sh(v)) };
        for vehicle in free_wagons(x, tile) {
            if vehicle == x.id(v) {
                continue;
            }
            let w = x.rec(unsafe { (x.g.vehicle)(vehicle) });
            if x.engine(w) != x.engine(v) {
                continue;
            }
            if x.first(w) == v {
                continue;
            }
            let last = x.id(x.last(w));
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
    let u = x.rec(u);
    x.assert(|| x.is_engine(u));
    for vehicle in free_wagons(x, x.tile(u)) {
        if !unsafe { nested_move(b, out, DoCommandFlag::EXECUTE, vehicle, x.id(u), true) } {
            break;
        }
        crate::witness::hit(crate::witness::VT_RAIL_NORMALIZE);
    }
}

/// Original `AddRearEngineToMultiheadedTrain`.
fn add_rear_engine(x: Ctx, v: R) {
    let a = x.b.construct.articulated;
    let u = x.rec((x.s.new_rear)(Ctx::sh(v)));
    (a.random_bits)(Ctx::sh(u), x.t.shared.random() as u16);
    x.set_subtype(v, gv::set_multiheaded);
    x.set_subtype(u, gv::set_multiheaded);
    x.set_next(v, u);
    x.reverse_on_build(u);
    unsafe {
        a.position(Ctx::sh(u));
    }
    (x.s.link_multihead)(Ctx::sh(v), Ctx::sh(u));
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
    let kind = crate::vehicle::EngineRef::of(e).rail_kind();
    if kind == RailVehicleTypes::WAGON {
        return Ctx::sh(unsafe { build_wagon(x, out, flags, tile, e) });
    }
    let railtypes = crate::vehicle::EngineRef::of(e).railtypes();
    let tiletype = (b.refit.train.physics.rail_type)(tile);
    if !(b.refit.train.physics.powered)(railtypes, tiletype) {
        (x.t.cost.error)(out, ERROR);
        return ptr::null_mut();
    }
    let mut v = ptr::null_mut();
    if flags & DoCommandFlag::EXECUTE != 0 {
        let k: &construct::Services = x.b.construct;
        let a = k.articulated;
        let company = (x.t.shared.current_company)();
        v = x.rec((s.new_engine)(tile, e));
        (x.c.vehicles.set_max_age)(Ctx::vref(v), unsafe {
            crate::engine_capabilities::openttd_rust_engine_life_days(x.r.capabilities, e)
        });
        (s.engine_dates)(Ctx::sh(v), e);
        (a.random_bits)(Ctx::sh(v), x.t.shared.random() as u16);
        (k.service_flags)(
            Ctx::sh(v),
            crate::vehicle::EngineRef::of(e).exclusive_preview(),
            (x.t.interval_percent)(company),
        );
        unsafe { (x.g.set_membership)(Ctx::sh(v), DEFAULT_GROUP) };
        x.set_subtype(v, gv::set_front_engine);
        x.set_subtype(v, gv::set_engine);
        x.reverse_on_build(v);
        unsafe {
            a.position(Ctx::sh(v));
        }
        if kind == RailVehicleTypes::MULTIHEAD {
            crate::witness::hit(crate::witness::VT_RAIL_MULTIHEAD);
            add_rear_engine(x, v);
        } else {
            unsafe { super::articulated::add(a, Ctx::sh(v)) };
        }
        x.consist_changed(v);
        x.update_group(v);
        unsafe { super::articulated::consistency(a, Ctx::sh(v)) };
    }
    (x.t.cost.zero)(out);
    Ctx::sh(v)
}

/// Original `FindGoodVehiclePos`.
fn find_good_position(x: Ctx, src: R) -> R {
    let eng = x.engine(src);
    for vehicle in free_wagons(x, x.tile(src)) {
        let dst = x.rec(unsafe { (x.g.vehicle)(vehicle) });
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
fn backup(x: Ctx, mut t: R) -> Vec<R> {
    let mut list = Vec::new();
    while !t.is_null() {
        list.push(t);
        t = x.next(t);
    }
    list
}
/// Original `RestoreTrainBackup`.
fn restore(x: Ctx, list: &[R]) {
    if list.is_empty() {
        return;
    }
    let mut prev: R = ptr::null_mut();
    for &t in list {
        if prev.is_null() {
            // Make sure the head of the train is always the first in the chain.
            let previous = x.previous(t);
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
fn remove_from_consist(x: Ctx, part: R, chain: bool) {
    let tail = if chain {
        x.last(part)
    } else {
        x.last_engine_part(part)
    };
    let previous = x.previous(part);
    if !previous.is_null() {
        x.set_next(previous, x.next(tail));
    }
    x.set_next(tail, ptr::null_mut());
}
/// Original `InsertInConsist`.
fn insert_in_consist(x: Ctx, dst: R, chain: R) {
    // We do not want to add something in the middle of an articulated part.
    x.assert(|| {
        !dst.is_null() && {
            let next = x.next(dst);
            next.is_null() || !x.is_articulated(next)
        }
    });
    x.set_next(x.last(chain), x.next(dst));
    x.set_next(dst, chain);
}
/// Original `NormaliseDualHeads`.
fn normalise_dual_heads(x: Ctx, mut t: R) {
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
            let other = x.rec((x.r.selection.other_multihead)(Ctx::sh(t)));
            if u != other {
                remove_from_consist(x, other, false);
                crate::witness::hit(crate::witness::VT_RAIL_DUAL_HEADS);
                insert_in_consist(x, u, other);
            }
        }
        t = x.next_vehicle(t);
    }
}
/// Original `NormaliseSubtypes`.
fn normalise_subtypes(x: Ctx, chain: R) {
    if chain.is_null() {
        return;
    }
    x.assert(|| x.previous(chain).is_null());
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
fn check_new_train(x: Ctx, original_dst: R, dst: R, original_src: R, src: R) -> Option<u32> {
    let engine = |v: R| i32::from(!v.is_null() && x.is_engine(v));
    if engine(src) + engine(dst) - engine(original_src) - engine(original_dst) <= 0 {
        return None;
    }
    // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
    if (x.b.free_unit)(crate::vehicle::VehicleType::TRAIN)
        <= unsafe { x.t.shared.settings().max_trains.read() }
    {
        return None;
    }
    Some(x.b.too_many)
}
/// Original `CheckTrainAttachment`. An error leaves the chain split, exactly
/// as the original; every caller restores its backup.
fn check_train_attachment(x: Ctx, mut t: R) -> Option<u32> {
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
    (x.s.invalidate_grf)(Ctx::sh(head));
    let strings = &x.c.strings;
    while !t.is_null() {
        allowed_len -= i32::from(x.length(t));
        let next = x.next(t);
        x.set_next(t, ptr::null_mut());
        if !x.is_articulated(t) && !x.is_rear(t) {
            let cache = Ctx::vref(t).ground_cache();
            let first_engine = read_field!(cache, first_engine);
            write_field!(cache, first_engine, crate::engine_type::INVALID_ENGINE);
            (x.s.invalidate_grf)(Ctx::sh(t));
            let mut regs100 = MaybeUninit::<i32>::uninit();
            let callback = unsafe {
                crate::newgrf_vehicle::callback(
                    x.c.vehicles,
                    Some(Ctx::vref(t)),
                    x.engine(head),
                    crate::newgrf_callbacks::CBID_TRAIN_ALLOW_WAGON_ATTACH.0,
                    0,
                    head,
                    true,
                    regs100.as_mut_ptr(),
                    1,
                )
            };
            write_field!(Ctx::vref(t).ground_cache(), first_engine, first_engine);
            (x.s.invalidate_grf)(Ctx::sh(t));
            (x.s.invalidate_grf)(Ctx::sh(head));
            if callback != CALLBACK_FAILED {
                let mut error = strings.null_string;
                let grf_text = |n: u32| {
                    x.c.grf_text(Ctx::sh(head), strings.grf_text.0.wrapping_add(n))
                };
                if x.c.engine_grf_version(Ctx::vref(head)) < 8 {
                    if callback == 0xFD {
                        error = strings.incompatible_rail;
                    }
                    if callback < 0xFD {
                        error = grf_text(u32::from(callback));
                    }
                    if callback >= 0x100 {
                        x.c.vehicles.unknown_callback_result(
                            (x.c.vehicles.engine)(Ctx::vref(head)),
                            crate::newgrf_callbacks::CBID_TRAIN_ALLOW_WAGON_ATTACH,
                            callback,
                        );
                    }
                } else if callback < 0x400 {
                    error = grf_text(u32::from(callback));
                } else {
                    match callback {
                        0x400 | 0x401 => (),
                        0x40F => {
                            // The original reads register 100 only for this result.
                            error = x
                                .c
                                .grf_text(Ctx::sh(head), unsafe { regs100.assume_init() } as u32);
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
    original_dst: R,
    dst: R,
    original_src: R,
    src: R,
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
fn arrange_trains(x: Ctx, dst_head: &mut R, dst: R, src_head: &mut R, src: R, move_chain: bool) {
    if *src_head == *dst_head {
        *dst_head = ptr::null_mut();
    } else if dst_head.is_null() {
        *dst_head = src;
    }
    if src == *src_head {
        *src_head = if move_chain {
            ptr::null_mut()
        } else if x.is_multiheaded(src) {
            Ctx::vref(src)
                .next_unit()
                .map_or(ptr::null_mut(), |v| v.0.as_ptr())
        } else {
            x.next_vehicle(src)
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
fn normalise_train_head(x: Ctx, head: R) {
    if head.is_null() {
        return;
    }
    x.consist_changed(head);
    x.update_group(head);
    if !x.is_front(head) {
        return;
    }
    (x.s.head_windows)(Ctx::sh(head));
    if (x.s.unit)(Ctx::sh(head)) != 0 {
        return;
    }
    (x.s.use_unit)(
        Ctx::sh(head),
        (x.b.free_unit)(crate::vehicle::VehicleType::TRAIN),
    );
}
/// `new_head->orders = old->orders`.
fn take_orders(new_head: R, old: R) {
    let orders = crate::vehicle_base::order_list(old);
    let view = read_field!(new_head, orders);
    write_field!(view, orders, orders);
}
fn add_to_shared(x: Ctx, v: R, chain: R) {
    unsafe {
        crate::orders::openttd_rust_add_shared(Ctx::sh(v).cast(), Ctx::sh(chain).cast(), x.c.orders)
    };
}
/// `DeleteVehicleOrders(v)` with its default arguments.
fn delete_orders(x: Ctx, v: R) {
    unsafe {
        crate::orders::openttd_rust_delete_vehicle_orders(Ctx::sh(v).cast(), 0, 1, x.c.orders)
    };
}
fn mark_dirty(x: Ctx, v: R) {
    unsafe {
        crate::train::openttd_rust_train_mark_dirty(x.r.train_handle(v), x.r.train, x.t.shared);
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
    unsafe { x.t.commands.ownership(out, Ctx::vref(src).owner(), 0) };
    if !(cost.success)(out) {
        return;
    }
    if Ctx::vref(src).status() & crate::vehicle_base::VS_CRASHED != 0 {
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
        unsafe { x.t.commands.ownership(out, Ctx::vref(dst).owner(), 0) };
        if !(cost.success)(out) {
            return;
        }
        if Ctx::vref(dst).status() & crate::vehicle_base::VS_CRASHED != 0 {
            return (cost.error)(out, ERROR);
        }
    }
    let first_part = x.r.selection.first_engine_part;
    src = x.rec(first_part(Ctx::sh(src)));
    if !dst.is_null() {
        dst = x.rec(first_part(Ctx::sh(dst)));
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
        dst = x.last_engine_part(dst);
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
        && src == x.rec((x.r.selection.other_multihead)(Ctx::sh(dst)))
    {
        return (cost.zero)(out);
    }
    if !unsafe { (x.w.stopped_in_depot)(Ctx::sh(src_head)) } {
        return (cost.error)(out, s.strings.depot_only);
    }
    if !dst_head.is_null() && !unsafe { (x.w.stopped_in_depot)(Ctx::sh(dst_head)) } {
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
        (s.front_windows)(Ctx::sh(src));
        if !src_head.is_null() && x.is_front(src_head) {
            crate::witness::hit(crate::witness::VT_RAIL_HEAD_TRANSFER);
            take_orders(src_head, src);
            if !crate::vehicle_base::order_list(src_head).is_null() {
                add_to_shared(x, src_head, src);
            }
            unsafe { (x.w.copy_configuration)(Ctx::sh(src), Ctx::sh(src_head)) };
        }
        delete_orders(x, src);
        (s.release_unit)(Ctx::sh(src));
        (x.t.clear_name)(Ctx::sh(src));
    }
    if original_src_head != src && dst_head == src {
        crate::witness::hit(crate::witness::VT_RAIL_NEW_FRONT);
        unsafe {
            crate::fleet::openttd_rust_fleet_set_train_group(x.g, Ctx::sh(src), DEFAULT_GROUP);
        }
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
            crate::cargo_storage::cargo_capacity(
                s.cargo_capacity,
                ptr::NonNull::new(src_head).map(VehicleRef),
                None,
                0,
                0,
            );
            crate::cargo_storage::cargo_capacity(
                s.cargo_capacity,
                ptr::NonNull::new(dst_head).map(VehicleRef),
                None,
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
    let v = x.rec((x.r.selection.first_engine_part)(t));
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
    if crate::vehicle_base::order_list(first).is_null() && (x.c.orders.list_capacity)() == 0 {
        restore(x, &original);
        return (cost.error)(out, s.strings.no_orders);
    }
    (cost.expense_money)(out, x.c.strings.new_vehicle_expense, 0);
    let mut part = sell_head;
    while !part.is_null() {
        (cost.amount)(out, (x.c.value)(Ctx::sh(part)).saturating_neg());
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
            take_orders(new_head, first);
            add_to_shared(x, new_head, first);
            delete_orders(x, first);
            unsafe { (x.w.copy_configuration)(Ctx::sh(first), Ctx::sh(new_head)) };
        }
        x.count_vehicle(new_head, 1);
    } else if unsafe { (x.g.primary)(Ctx::sh(v)) } && backup_order {
        unsafe { crate::orders::openttd_rust_backup_create(Ctx::sh(v).cast(), user, x.c.orders) };
    }
    normalise_train_head(x, new_head);
    (s.depot_changed)(x.tile(v));
    // No shell access follows the deleting callback.
    (x.c.vehicles.destroy)(Ctx::vref(sell_head));
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
        cargo_capacity, strings, train_if_valid, new_wagon, new_engine, engine_dates, new_rear, link_multihead,
        invalidate_grf, unit, use_unit, release_unit, head_windows, front_windows, company_dirty, depot_changed,
    };
    "OpenTTDRailConstructionStrings" ["probe"] => Strings { rear_follow, depot_only, no_orders, };
}
