/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Rail vehicle construction and depot consist transactions: the original `train_cmd.cpp`
//! bodies from `GetFreeWagonsInDepot` through `CmdSellRailWagon`. Records carry chain links
//! and moved fields; shells keep the rest. Rust owns control flow, the temporary backup lists
//! and the free-wagon order.
#![allow(clippy::many_single_char_names, clippy::too_many_lines)]
use super::{CommonServices, ERROR, TransactionServices, build, construct, refit};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::fleet::{DEFAULT_GROUP, GroupServices, INVALID_VEHICLE};
use crate::ground_vehicle as gv;
use crate::network_type::ClientID;
use crate::raw_field::{read_field, write_field};
use crate::services::{CommandCost, Cost};
use crate::tile_type::TILE_SIZE;
use crate::tile_type::TileIndex;
use crate::vehicle::OpenTTDVehicle;
use crate::vehicle::{CALLBACK_FAILED, OpenTTDEngine, RailVehicleTypes, VehicleRef};
use crate::vehicle_base::{is_articulated_part, is_front_engine};
use crate::vehicle_type::VehicleID;
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

/// `GetFreeWagonsInDepot`: get a list of free wagons in a depot.
fn get_free_wagons_in_depot(x: Ctx, tile: TileIndex) -> Vec<u32> {
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
            .for_each_on_tile(tile.0, |v| visit(ptr::from_mut(&mut ctx).cast(), v));
    }
    /* Sort by vehicle index for consistency across clients. */
    ctx.free.sort_unstable();
    ctx.free
}

/// `Command<CMD_MOVE_RAIL_VEHICLE>::Do` from a Rust caller; whether it succeeded.
pub(crate) unsafe fn nested_move_rail_vehicle(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdMoveRailVehicle,
) -> bool {
    let t = b.refit.common.base;
    crate::command::nested(
        t.commands,
        t.shared,
        Cost(out, t.cost),
        0,
        flags.0,
        |f| unsafe {
            cmd_move_rail_vehicle(b, out, DoCommandFlags(f), cmd);
        },
    );
    (t.cost.success)(out)
}
/// Raw adapter: the nested `CmdMoveRailVehicle` over base values, for
/// `fleet/transactions.rs` until its pass.
pub(crate) unsafe fn nested_move(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: u16,
    src: u32,
    dest: u32,
    chain: bool,
) -> bool {
    unsafe {
        nested_move_rail_vehicle(
            b,
            out,
            DoCommandFlags(flags),
            CmdMoveRailVehicle {
                src_veh: VehicleID(src),
                dest_veh: VehicleID(dest),
                move_chain: chain,
            },
        )
    }
}

/// `CmdBuildRailWagon`: build a railroad wagon. `out` doubles as the scratch
/// result of the nested free-chain moves; the original returns a fresh zero
/// cost after them.
unsafe fn cmd_build_rail_wagon(
    x: Ctx,
    out: *mut CommandCost,
    flags: DoCommandFlags,
    tile: TileIndex,
    e: *const OpenTTDEngine,
) -> R {
    /* Check that the wagon can drive on the track in question */
    let railtypes = crate::vehicle::EngineRef::of(e).railtypes();
    let tiletype = (x.b.refit.train.physics.rail_type)(tile.0);
    if (x.t.shared.all_compatible_railtypes)(railtypes) & (1 << tiletype.0) == 0 {
        (x.t.cost.error)(out, ERROR);
        return ptr::null_mut();
    }
    let mut v = ptr::null_mut();
    if flags.test(DoCommandFlag::Execute) {
        let a = x.b.construct.articulated;
        // The native allocation clears gcache.first_engine: needs to be set before first callback.
        v = x.rec((x.s.new_wagon)(tile.0, e));
        (a.random_bits)(Ctx::sh(v), x.t.shared.random() as u16);
        unsafe { (x.g.set_membership)(Ctx::sh(v), DEFAULT_GROUP) };
        x.reverse_on_build(v);
        unsafe { super::articulated::add_articulated_parts(a, Ctx::sh(v)) };
        unsafe {
            a.position(Ctx::sh(v));
        }
        let first = x.first(v);
        x.consist_changed(first);
        x.update_group(x.first(v));
        unsafe { super::articulated::check_consistency_of_articulated_vehicle(a, Ctx::sh(v)) };
        /* Try to connect the vehicle to one of free chains of wagons. */
        for vehicle in get_free_wagons_in_depot(x, tile) {
            if vehicle == x.id(v) {
                continue;
            }
            let w = x.rec(unsafe { (x.g.vehicle)(vehicle) });
            if x.engine(w) != x.engine(v) {
                continue; // Must be same type
            }
            if x.first(w) == v {
                continue; // Don't connect to ourself
            }
            let last = x.id(x.last(w));
            if unsafe {
                nested_move_rail_vehicle(
                    x.b,
                    out,
                    DoCommandFlags::from_element(DoCommandFlag::Execute),
                    CmdMoveRailVehicle {
                        src_veh: VehicleID(x.id(v)),
                        dest_veh: VehicleID(last),
                        move_chain: true,
                    },
                )
            } {
                crate::witness::hit(crate::witness::VT_RAIL_WAGON_JOIN);
                break;
            }
        }
    }
    (x.t.cost.zero)(out);
    v
}

/// `NormalizeTrainVehInDepot`: move all free vehicles in the depot to the
/// train; `out` is a free caller-owned result.
pub(crate) unsafe fn normalize_train_veh_in_depot(
    b: &'static build::Services,
    out: *mut CommandCost,
    u: *mut (),
) {
    let x = Ctx::new(b);
    let u = x.rec(u);
    x.assert(|| x.is_engine(u));
    for vehicle in get_free_wagons_in_depot(x, TileIndex(x.tile(u))) {
        if !unsafe {
            nested_move_rail_vehicle(
                b,
                out,
                DoCommandFlags::from_element(DoCommandFlag::Execute),
                CmdMoveRailVehicle {
                    src_veh: VehicleID(vehicle),
                    dest_veh: VehicleID(x.id(u)),
                    move_chain: true,
                },
            )
        } {
            break;
        }
        crate::witness::hit(crate::witness::VT_RAIL_NORMALIZE);
    }
}

/// `AddRearEngineToMultiheadedTrain`: build the rear engine of a multiheaded train.
fn add_rear_engine_to_multiheaded_train(x: Ctx, v: R) {
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
    /* Now we need to link the front and rear engines together */
    (x.s.link_multihead)(Ctx::sh(v), Ctx::sh(u));
}

/// `CmdBuildRailVehicle`: build a railroad vehicle at the depot `tile`; returns
/// the new vehicle, or null when only testing or on error.
///
/// # Safety
/// Live native result and engine; new shells stay live and no shell access
/// spans a callback.
pub(crate) unsafe fn cmd_build_rail_vehicle(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: DoCommandFlags,
    tile: TileIndex,
    e: *const OpenTTDEngine,
) -> *mut () {
    let x = Ctx::new(b);
    let s = x.s;
    let kind = crate::vehicle::EngineRef::of(e).rail_kind();
    if kind == RailVehicleTypes::WAGON {
        return Ctx::sh(unsafe { cmd_build_rail_wagon(x, out, flags, tile, e) });
    }
    /* Check if depot and new engine uses the same kind of tracks *
     * We need to see if the engine got power on the tile to avoid electric engines in non-electric depots */
    let railtypes = crate::vehicle::EngineRef::of(e).railtypes();
    let tiletype = (b.refit.train.physics.rail_type)(tile.0);
    if !(b.refit.train.physics.powered)(railtypes, tiletype) {
        (x.t.cost.error)(out, ERROR);
        return ptr::null_mut();
    }
    let mut v = ptr::null_mut();
    if flags.test(DoCommandFlag::Execute) {
        let k: &construct::Services = x.b.construct;
        let a = k.articulated;
        let company = (x.t.shared.current_company)();
        // The native allocation clears gcache.first_engine: needs to be set before first callback.
        v = x.rec((s.new_engine)(tile.0, e));
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
            add_rear_engine_to_multiheaded_train(x, v);
        } else {
            unsafe { super::articulated::add_articulated_parts(a, Ctx::sh(v)) };
        }
        x.consist_changed(v);
        x.update_group(v);
        unsafe { super::articulated::check_consistency_of_articulated_vehicle(a, Ctx::sh(v)) };
    }
    (x.t.cost.zero)(out);
    Ctx::sh(v)
}

/// `FindGoodVehiclePos`: find a free wagon chain in the depot whose engines
/// match `src`'s, to drag it onto.
fn find_good_vehicle_pos(x: Ctx, src: R) -> R {
    let eng = x.engine(src);
    for vehicle in get_free_wagons_in_depot(x, TileIndex(x.tile(src))) {
        let dst = x.rec(unsafe { (x.g.vehicle)(vehicle) });
        /* check so all vehicles in the line have the same engine. */
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

/// `MakeTrainBackup`: make a backup of a train into a train list; a local list,
/// as the original's `TrainList`.
fn make_train_backup(x: Ctx, mut t: R) -> Vec<R> {
    let mut list = Vec::new();
    while !t.is_null() {
        list.push(t);
        t = x.next(t);
    }
    list
}
/// `RestoreTrainBackup`: restore the train from the backup list.
fn restore_train_backup(x: Ctx, list: &[R]) {
    /* No train, nothing to do. */
    if list.is_empty() {
        return;
    }
    let mut prev: R = ptr::null_mut();
    /* Iterate over the list and rebuild it. */
    for &t in list {
        if prev.is_null() {
            /* Make sure the head of the train is always the first in the chain. */
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
/// `RemoveFromConsist`: unlink a part, or the chain from it, from its train.
fn remove_from_consist(x: Ctx, part: R, chain: bool) {
    let tail = if chain {
        x.last(part)
    } else {
        x.last_engine_part(part)
    };
    /* Unlink at the front, but make it point to the next
     * vehicle after the to be remove part. */
    let previous = x.previous(part);
    if !previous.is_null() {
        x.set_next(previous, x.next(tail));
    }
    /* Unlink at the back */
    x.set_next(tail, ptr::null_mut());
}
/// `InsertInConsist`: inserts a chain into the train at dst.
fn insert_in_consist(x: Ctx, dst: R, chain: R) {
    /* We do not want to add something in the middle of an articulated part. */
    x.assert(|| {
        !dst.is_null() && {
            let next = x.next(dst);
            next.is_null() || !x.is_articulated(next)
        }
    });
    x.set_next(x.last(chain), x.next(dst));
    x.set_next(dst, chain);
}
/// `NormaliseDualHeads`: normalise the dual heads in the train, i.e. if one is
/// missing move that one to this train.
fn normalise_dual_heads(x: Ctx, mut t: R) {
    while !t.is_null() {
        if x.is_multiheaded(t) && x.is_engine(t) {
            /* Make sure that there are no free cars before next engine */
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
                /* Remove the part from the 'wrong' train */
                remove_from_consist(x, other, false);
                crate::witness::hit(crate::witness::VT_RAIL_DUAL_HEADS);
                /* And add it to the 'right' train */
                insert_in_consist(x, u, other);
            }
        }
        t = x.next_vehicle(t);
    }
}
/// `NormaliseSubtypes`: normalise the sub types of the parts in this chain.
fn normalise_subtypes(x: Ctx, chain: R) {
    /* Nothing to do */
    if chain.is_null() {
        return;
    }
    /* We must be the first in the chain. */
    x.assert(|| x.previous(chain).is_null());
    /* Set the appropriate bits for the first in the chain. */
    if gv::is_wagon(x.subtype(chain)) {
        x.set_subtype(chain, gv::set_free_wagon);
    } else {
        x.assert(|| x.is_engine(chain));
        x.set_subtype(chain, gv::set_front_engine);
    }
    /* Now clear the bits for the rest of the chain */
    let mut t = x.next(chain);
    while !t.is_null() {
        x.set_subtype(t, gv::clear_free_wagon);
        x.set_subtype(t, gv::clear_front_engine);
        t = x.next(t);
    }
}

/// `CheckNewTrain`: check/validate whether we may actually build a new train;
/// `Some` is the `CommandCost(StringID)` error.
fn check_new_train(x: Ctx, original_dst: R, dst: R, original_src: R, src: R) -> Option<u32> {
    /* Just add 'new' engines and subtract the original ones.
     * If that's less than or equal to 0 we can be sure we did
     * not add any engines (read: trains) along the way. */
    let engine = |v: R| i32::from(!v.is_null() && x.is_engine(v));
    if engine(src) + engine(dst) - engine(original_src) - engine(original_dst) <= 0 {
        return None;
    }
    /* Get a free unit number and check whether it's within the bounds.
     * There will always be a maximum of one new train. */
    // SAFETY: The catalog addresses the process-wide setting; one field-sized load.
    if (x.b.free_unit)(crate::vehicle::VehicleType::TRAIN)
        <= unsafe { x.t.shared.settings().max_trains.read() }
    {
        return None;
    }
    Some(x.b.too_many)
}
/// `CheckTrainAttachment`: check whether the train parts can be attached. An
/// error leaves the chain split, exactly as the original; every caller
/// restores its backup.
fn check_train_attachment(x: Ctx, mut t: R) -> Option<u32> {
    /* No multi-part train, no need to check. */
    if t.is_null() || x.next(t).is_null() {
        return None;
    }
    /* The maximum length for a train. For each part we decrease this by one
     * and if the result is negative the train is simply too long. */
    // Unsigned `uint8_t * uint` arithmetic, then the original's `int` conversion.
    let mut allowed_len = (u32::from(x.w.max_train_length()) * TILE_SIZE)
        .wrapping_sub(u32::from(x.length(t)))
        .cast_signed();
    /* For free-wagon chains, check if they are within the max_train_length limit. */
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
    /* Break the prev -> t link so it always holds within the loop. */
    t = x.next(t);
    x.set_next(prev, ptr::null_mut());
    /* Make sure the cache is cleared. */
    (x.s.invalidate_grf)(Ctx::sh(head));
    let strings = &x.c.strings;
    while !t.is_null() {
        allowed_len -= i32::from(x.length(t));
        let next = x.next(t);
        /* Unlink the to-be-added piece; it is already unlinked from the previous
         * part due to the fact that the prev -> t link is broken. */
        x.set_next(t, ptr::null_mut());
        /* Don't check callback for articulated or rear dual headed parts */
        if !x.is_articulated(t) && !x.is_rear(t) {
            /* Back up and clear the first_engine data to avoid using wagon override group */
            let cache = Ctx::vref(t).ground_cache();
            let first_engine = read_field!(cache, first_engine);
            write_field!(cache, first_engine, crate::engine_type::INVALID_ENGINE);
            /* We don't want the cache to interfere. head's cache is cleared before
             * the loop and after each callback does not need to be cleared here. */
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
            /* Restore original first_engine data */
            write_field!(Ctx::vref(t).ground_cache(), first_engine, first_engine);
            /* We do not want to remember any cached variables from the test run */
            (x.s.invalidate_grf)(Ctx::sh(t));
            (x.s.invalidate_grf)(Ctx::sh(head));
            if callback != CALLBACK_FAILED {
                /* A failing callback means everything is okay */
                let mut error = strings.null_string;
                let grf_text = |n: u32| {
                    x.c.grf_text_raw(Ctx::sh(head), strings.grf_text.0.wrapping_add(n))
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
                        // 0x400: allow if railtypes match (always the case for OpenTTD); 0x401: allow
                        0x400 | 0x401 => (),
                        0x40F => {
                            // The original reads register 100 only for this result.
                            error =
                                x.c.grf_text_raw(Ctx::sh(head), unsafe { regs100.assume_init() }
                                    as u32);
                        }
                        // unknown reason, or 0x402: disallow attaching
                        _ => error = strings.incompatible_rail,
                    }
                }
                if error != strings.null_string {
                    return Some(error);
                }
            }
        }
        /* And link it to the new part. */
        x.set_next(prev, t);
        prev = t;
        t = next;
    }
    if allowed_len < 0 {
        return Some(x.w.too_long);
    }
    None
}
/// `ValidateTrains`: validate whether we are going to create valid trains.
fn validate_trains(
    x: Ctx,
    original_dst: R,
    dst: R,
    original_src: R,
    src: R,
    check_limit: bool,
) -> Option<u32> {
    /* Check whether we may actually construct the trains. */
    let error = check_train_attachment(x, src);
    if error.is_some() {
        return error;
    }
    let error = check_train_attachment(x, dst);
    if error.is_some() {
        return error;
    }
    /* Check whether we need to build a new train. */
    if check_limit {
        check_new_train(x, original_dst, dst, original_src, src)
    } else {
        None
    }
}
/// `ArrangeTrains`: arrange the trains in the wanted way.
fn arrange_trains(x: Ctx, dst_head: &mut R, dst: R, src_head: &mut R, src: R, move_chain: bool) {
    /* First determine the front of the two resulting trains */
    if *src_head == *dst_head {
        /* If we aren't moving part(s) to a new train, we are just moving the
         * front back and there is not destination head. */
        *dst_head = ptr::null_mut();
    } else if dst_head.is_null() {
        /* If we are moving to a new train the head of the move train would become
         * the head of the new vehicle. */
        *dst_head = src;
    }
    if src == *src_head {
        /* If we are moving the front of a train then we are, in effect, creating
         * a new head for the train. Point to that. Unless we are moving the whole
         * train in which case there is not 'source' train anymore.
         * In case we are a multiheaded part we want the complete thing to come
         * with us, so src->GetNextUnit(), however... when we are e.g. a wagon
         * that is followed by a rear multihead we do not want to include that. */
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
    /* Now it's just simply removing the part that we are going to move from the
     * source train and *if* the destination is a not a new train add the chain
     * at the destination location. */
    remove_from_consist(x, src, move_chain);
    if *dst_head != src {
        insert_in_consist(x, dst, src);
    }
    /* Now normalise the dual heads, that is move the dual heads around in such
     * a way that the head and rear of a dual head are in the same train */
    normalise_dual_heads(x, *src_head);
    normalise_dual_heads(x, *dst_head);
}
/// `NormaliseTrainHead`: normalise the head of the train again, i.e. that is
/// tell the world that we have changed and update all kinds of variables.
fn normalise_train_head(x: Ctx, head: R) {
    /* Not much to do! */
    if head.is_null() {
        return;
    }
    /* Tell the 'world' the train changed. */
    x.consist_changed(head);
    x.update_group(head);
    /* Not a front engine, i.e. a free wagon chain. No need to do more. */
    if !x.is_front(head) {
        return;
    }
    /* Update the refit button and window */
    (x.s.head_windows)(Ctx::sh(head));
    /* If we don't have a unit number yet, set one. */
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

/// `CmdMoveRailVehicle`'s arguments.
#[derive(Clone, Copy)]
pub(crate) struct CmdMoveRailVehicle {
    /// Source vehicle index.
    pub(crate) src_veh: VehicleID,
    /// What wagon to put the source wagon AFTER, `INVALID_VEHICLE` to make a new line.
    pub(crate) dest_veh: VehicleID,
    /// Move all vehicles following the source vehicle.
    pub(crate) move_chain: bool,
}
/// `CmdMoveRailVehicle`: move a rail vehicle around inside the depot.
/// `DoCommandFlag::AutoReplace` is set when autoreplace tries to undo its
/// modifications or moves vehicles to temporary locations inside the depot.
///
/// # Safety
/// Live caller-owned native result; vehicles stay live through the command.
pub(crate) unsafe fn cmd_move_rail_vehicle(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: DoCommandFlags,
    cmd: CmdMoveRailVehicle,
) {
    let CmdMoveRailVehicle {
        src_veh,
        dest_veh,
        move_chain,
    } = cmd;
    let x = Ctx::new(b);
    let s = x.s;
    let cost = x.t.cost;
    let mut src = x.train(src_veh.0);
    if src.is_null() {
        return (cost.error)(out, ERROR);
    }
    unsafe { x.t.commands.ownership(out, Ctx::vref(src).owner(), 0) };
    if !(cost.success)(out) {
        return;
    }
    /* Do not allow moving crashed vehicles inside the depot, it is likely to cause asserts later */
    if Ctx::vref(src).status() & crate::vehicle_base::VS_CRASHED != 0 {
        return (cost.error)(out, ERROR);
    }
    /* if nothing is selected as destination, try and find a matching vehicle to drag to. */
    let mut dst;
    if dest_veh.0 == INVALID_VEHICLE {
        dst = if x.is_engine(src) || flags.test(DoCommandFlag::AutoReplace) {
            ptr::null_mut()
        } else {
            find_good_vehicle_pos(x, src)
        };
    } else {
        dst = x.train(dest_veh.0);
        if dst.is_null() {
            return (cost.error)(out, ERROR);
        }
        unsafe { x.t.commands.ownership(out, Ctx::vref(dst).owner(), 0) };
        if !(cost.success)(out) {
            return;
        }
        /* Do not allow appending to crashed vehicles, too */
        if Ctx::vref(dst).status() & crate::vehicle_base::VS_CRASHED != 0 {
            return (cost.error)(out, ERROR);
        }
    }
    /* if an articulated part is being handled, deal with its parent vehicle */
    let first_part = x.r.selection.first_engine_part;
    src = x.rec(first_part(Ctx::sh(src)));
    if !dst.is_null() {
        dst = x.rec(first_part(Ctx::sh(dst)));
    }
    /* don't move the same vehicle.. */
    if src == dst {
        return (cost.zero)(out);
    }
    /* locate the head of the two chains */
    let mut src_head = x.first(src);
    let mut dst_head;
    if dst.is_null() {
        dst_head = ptr::null_mut();
    } else {
        dst_head = x.first(dst);
        if x.tile(dst_head) != x.tile(src_head) {
            return (cost.error)(out, ERROR);
        }
        /* Now deal with articulated part of destination wagon */
        dst = x.last_engine_part(dst);
    }
    if x.is_rear(src) {
        return (cost.error)(out, s.strings.rear_follow);
    }
    /* When moving all wagons, we can't have the same src_head and dst_head */
    if move_chain && src_head == dst_head {
        return (cost.zero)(out);
    }
    /* When moving a multiheaded part to be place after itself, bail out. */
    if !move_chain
        && !dst.is_null()
        && x.is_rear(dst)
        && src == x.rec((x.r.selection.other_multihead)(Ctx::sh(dst)))
    {
        return (cost.zero)(out);
    }
    /* Check if all vehicles in the source train are stopped inside a depot. */
    if !unsafe { (x.w.stopped_in_depot)(Ctx::sh(src_head)) } {
        return (cost.error)(out, s.strings.depot_only);
    }
    /* Check if all vehicles in the destination train are stopped inside a depot. */
    if !dst_head.is_null() && !unsafe { (x.w.stopped_in_depot)(Ctx::sh(dst_head)) } {
        return (cost.error)(out, s.strings.depot_only);
    }
    /* First make a backup of the order of the trains. That way we can do
     * whatever we want with the order and later on easily revert. */
    let original_src = make_train_backup(x, src_head);
    let original_dst = make_train_backup(x, dst_head);
    /* Also make backup of the original heads as ArrangeTrains can change them.
     * For the destination head we do not care if it is the same as the source
     * head because in that case it's just a copy. */
    let original_src_head = src_head;
    let original_dst_head = if dst_head == src_head {
        ptr::null_mut()
    } else {
        dst_head
    };
    /* We want this information from before the rearrangement, but execute this after the validation.
     * original_src_head can't be nullptr; src is by definition != nullptr, so src_head can't be nullptr as
     * src->GetFirst() always yields non-nullptr, so eventually original_src_head != nullptr as well. */
    let original_src_head_front_engine = x.is_front(original_src_head);
    let original_dst_head_front_engine =
        !original_dst_head.is_null() && x.is_front(original_dst_head);
    /* (Re)arrange the trains in the wanted arrangement. */
    arrange_trains(x, &mut dst_head, dst, &mut src_head, src, move_chain);
    /* If the autoreplace flag is set we do not need to test for the validity
     * because we are going to revert the train to its original state. As we
     * assume the original state was correct autoreplace can skip this. */
    if !flags.test(DoCommandFlag::AutoReplace)
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
        /* Restore the train we had. */
        restore_train_backup(x, &original_src);
        restore_train_backup(x, &original_dst);
        return (cost.error)(out, error);
    }
    /* do it? */
    if !flags.test(DoCommandFlag::Execute) {
        /* We don't want to execute what we're just tried. */
        restore_train_backup(x, &original_src);
        restore_train_backup(x, &original_dst);
        return (cost.zero)(out);
    }
    /* Remove old heads from the statistics */
    if original_src_head_front_engine {
        x.count_vehicle(original_src_head, -1);
    }
    if original_dst_head_front_engine {
        x.count_vehicle(original_dst_head, -1);
    }
    /* First normalise the sub types of the chains. */
    normalise_subtypes(x, src_head);
    normalise_subtypes(x, dst_head);
    /* There are 14 different cases:
     *  1) front engine gets moved to a new train, it stays a front engine.
     *     a) the 'next' part is a wagon that becomes a free wagon chain.
     *     b) the 'next' part is an engine that becomes a front engine.
     *     c) there is no 'next' part, nothing else happens
     *  2) front engine gets moved to another train, it is not a front engine anymore
     *     a) the 'next' part is a wagon that becomes a free wagon chain.
     *     b) the 'next' part is an engine that becomes a front engine.
     *     c) there is no 'next' part, nothing else happens
     *  3) front engine gets moved to later in the current train, it is not a front engine anymore.
     *     a) the 'next' part is a wagon that becomes a free wagon chain.
     *     b) the 'next' part is an engine that becomes a front engine.
     *  4) free wagon gets moved
     *     a) the 'next' part is a wagon that becomes a free wagon chain.
     *     b) the 'next' part is an engine that becomes a front engine.
     *     c) there is no 'next' part, nothing else happens
     *  5) non front engine gets moved and becomes a new train, nothing else happens
     *  6) non front engine gets moved within a train / to another train, nothing happens
     *  7) wagon gets moved, nothing happens
     */
    if src == original_src_head && x.is_engine(src) && !x.is_front(src) {
        /* Cases #2 and #3: the front engine gets trashed. */
        crate::witness::hit(crate::witness::VT_RAIL_FRONT_TRASHED);
        (s.front_windows)(Ctx::sh(src));
        if !src_head.is_null() && x.is_front(src_head) {
            /* Cases #?b: Transfer order, unit number and other stuff
             * to the new front engine. */
            crate::witness::hit(crate::witness::VT_RAIL_HEAD_TRANSFER);
            take_orders(src_head, src);
            if !crate::vehicle_base::order_list(src_head).is_null() {
                add_to_shared(x, src_head, src);
            }
            unsafe { (x.w.copy_configuration)(Ctx::sh(src), Ctx::sh(src_head)) };
        }
        /* Remove stuff not valid anymore for non-front engines. */
        delete_orders(x, src);
        (s.release_unit)(Ctx::sh(src));
        (x.t.clear_name)(Ctx::sh(src));
    }
    /* We weren't a front engine but are becoming one. So
     * we should be put in the default group. */
    if original_src_head != src && dst_head == src {
        crate::witness::hit(crate::witness::VT_RAIL_NEW_FRONT);
        unsafe {
            crate::fleet::openttd_rust_fleet_set_train_group(x.g, Ctx::sh(src), DEFAULT_GROUP);
        }
        (s.company_dirty)();
    }
    /* Handle 'new engine' part of cases #1b, #2b, #3b, #4b and #5 in NormaliseTrainHead. */
    normalise_train_head(x, src_head);
    normalise_train_head(x, dst_head);
    /* Add new heads to statistics.
     * This should be done after NormaliseTrainHead due to engine total limit checks in GetFreeUnitNumber. */
    if !src_head.is_null() && x.is_front(src_head) {
        x.count_vehicle(src_head, 1);
    }
    if !dst_head.is_null() && x.is_front(dst_head) {
        x.count_vehicle(dst_head, 1);
    }
    if !flags.test(DoCommandFlag::NoCargoCapacityCheck) {
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
    /* We are undoubtedly changing something in the depot and train list. */
    (s.depot_changed)(x.tile(src));
    (cost.zero)(out);
}

/// `CmdSellRailWagon`: sell a (single) train wagon/engine, or the chain from
/// it when `sell_chain`.
///
/// # Safety
/// Live caller-owned native result and train shell; the sold chain is deleted
/// last, after every shell access.
pub(crate) unsafe fn cmd_sell_rail_wagon(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: DoCommandFlags,
    t: *mut (),
    sell_chain: bool,
    backup_order: bool,
    user: ClientID,
) {
    let x = Ctx::new(b);
    let s = x.s;
    let cost = x.t.cost;
    let v = x.rec((x.r.selection.first_engine_part)(t));
    let first = x.first(v);
    if x.is_rear(v) {
        return (cost.error)(out, s.strings.rear_follow);
    }
    /* First make a backup of the order of the train. That way we can do
     * whatever we want with the order and later on easily revert. */
    let original = make_train_backup(x, first);
    /* We need to keep track of the new head and the head of what we're going to sell. */
    let mut new_head = first;
    let mut sell_head = ptr::null_mut();
    /* Split the train in the wanted way. */
    arrange_trains(
        x,
        &mut sell_head,
        ptr::null_mut(),
        &mut new_head,
        v,
        sell_chain,
    );
    /* We don't need to validate the second train; it's going to be sold. */
    if let Some(error) = validate_trains(
        x,
        ptr::null_mut(),
        ptr::null_mut(),
        first,
        new_head,
        !flags.test(DoCommandFlag::AutoReplace),
    ) {
        crate::witness::hit(crate::witness::VT_RAIL_SELL_RESTORE);
        /* Restore the train we had. */
        restore_train_backup(x, &original);
        return (cost.error)(out, error);
    }
    if crate::vehicle_base::order_list(first).is_null() && (x.c.orders.list_capacity)() == 0 {
        /* Restore the train we had. */
        restore_train_backup(x, &original);
        return (cost.error)(out, s.strings.no_orders);
    }
    (cost.expense_money)(out, x.c.strings.new_vehicle_expense, 0);
    let mut part = sell_head;
    while !part.is_null() {
        (cost.amount)(out, (x.c.value)(Ctx::sh(part)).saturating_neg());
        part = x.next(part);
    }
    /* do it? */
    if !flags.test(DoCommandFlag::Execute) {
        /* We don't want to execute what we're just tried. */
        restore_train_backup(x, &original);
        return;
    }
    /* First normalise the sub types of the chain. */
    normalise_subtypes(x, new_head);
    if v == first && !sell_chain && !new_head.is_null() && x.is_front(new_head) {
        if x.is_engine(v) {
            /* We are selling the front engine. In this case we want to
             * 'give' the order, unit number and such to the new head. */
            crate::witness::hit(crate::witness::VT_RAIL_SELL_TRANSFER);
            take_orders(new_head, first);
            add_to_shared(x, new_head, first);
            delete_orders(x, first);
            /* Copy other important data from the front engine */
            unsafe { (x.w.copy_configuration)(Ctx::sh(first), Ctx::sh(new_head)) };
        }
        x.count_vehicle(new_head, 1); // after copying over the profit, if required
    } else if unsafe { (x.g.primary)(Ctx::sh(v)) } && backup_order {
        unsafe {
            crate::orders::openttd_rust_backup_create(Ctx::sh(v).cast(), user.0, x.c.orders);
        };
    }
    /* We need to update the information about the train. */
    normalise_train_head(x, new_head);
    /* We are undoubtedly changing something in the depot and train list. */
    (s.depot_changed)(x.tile(v));
    /* Actually delete the sold 'goods' */
    // No shell access follows the deleting callback.
    (x.c.vehicles.destroy)(Ctx::vref(sell_head));
}

/// Command entry for `CmdMoveRailVehicle`.
/// # Safety
/// As `cmd_move_rail_vehicle`; the static table outlives the call. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_train_move(
    b: &'static build::Services,
    out: *mut CommandCost,
    flags: u16,
    src_veh: u32,
    dest_veh: u32,
    move_chain: bool,
) {
    unsafe {
        cmd_move_rail_vehicle(
            b,
            out,
            DoCommandFlags(flags),
            CmdMoveRailVehicle {
                src_veh: VehicleID(src_veh),
                dest_veh: VehicleID(dest_veh),
                move_chain,
            },
        );
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRailConstructionServices" ["probe"] => Services {
        cargo_capacity, strings, train_if_valid, new_wagon, new_engine, engine_dates, new_rear, link_multihead,
        invalidate_grf, unit, use_unit, release_unit, head_windows, front_windows, company_dirty, depot_changed,
    };
    "OpenTTDRailConstructionStrings" ["probe"] => Strings { rear_follow, depot_only, no_orders, };
}
