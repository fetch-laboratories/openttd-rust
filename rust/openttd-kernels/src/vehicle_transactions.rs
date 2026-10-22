/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common vehicle transactions. Native shells, names and stack costs stay native;
//! Rust owns policy and transaction scratch. All accesses end before callbacks.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
pub(crate) mod articulated;
pub(crate) mod build;
pub(crate) mod clone;
mod clone_name;
pub(crate) mod construct;
pub(crate) mod mass;
pub(crate) mod refit;
use crate::command_type::DoCommandFlag;
use crate::fleet::GroupServices;
use crate::order_type::{OT_GOTO_WAYPOINT, OT_IMPLICIT};
use crate::services::{ByteSpan, CommandCost, CommandCostServices, CommandServices, Services};
use crate::strings_type::INVALID_STRING_ID as ERROR;
use crate::vehiclelist::{
    VL_DEPOT_LIST, VL_GROUP_LIST, VL_SHARED_ORDERS, VL_STANDARD, VL_STATION_LIST,
};

#[repr(C)]
pub struct Target {
    pub owner: u8,
    pub primary: bool,
}
#[repr(C)]
pub struct TransactionServices {
    pub shared: &'static Services,
    pub commands: &'static CommandServices,
    pub cost: &'static CommandCostServices,
    pub groups: &'static GroupServices,
    pub target: extern "C" fn(*mut ()) -> Target,
    pub name: extern "C" fn(*mut ()) -> ByteSpan,
    pub clear_name: extern "C" fn(*mut ()),
    pub set_name: extern "C" fn(*mut (), ByteSpan),
    pub name_dirty: extern "C" fn(*mut ()),
    pub interval_percent: extern "C" fn(u8) -> bool,
    pub interval_default: extern "C" fn(u8, u8) -> i32,
    pub interval_clamp: extern "C" fn(i32, bool) -> u16,
    pub set_interval: extern "C" fn(*mut (), u16, bool, bool),
    pub unique_name: u32,
    pub max_name: usize,
}

/// # Safety
/// The spans denote live readable bytes; no name mutation occurs during comparison.
unsafe fn equal(a: ByteSpan, b: ByteSpan) -> bool {
    a.size == b.size
        && (a.size == 0
            || unsafe {
                std::slice::from_raw_parts(a.data, a.size)
                    == std::slice::from_raw_parts(b.data, b.size)
            })
}
unsafe fn unique_name(s: &TransactionServices, name: ByteSpan) -> bool {
    let mut from = 0;
    loop {
        let v = unsafe { (s.groups.next_vehicle)(from) };
        if v.is_null() {
            return true;
        }
        let current = (s.name)(v.cast());
        if current.size != 0 && unsafe { equal(current, name) } {
            return false;
        }
        from = unsafe { (s.groups.vehicle_id)(v.cast()) }.wrapping_add(1);
    }
}
/// Resolve once, preserving the original primary check before ownership.
unsafe fn owned_primary(s: &TransactionServices, cost: *mut CommandCost, id: u32) -> (*mut (), u8) {
    let v = unsafe { (s.groups.vehicle)(id) };
    if v.is_null() {
        (s.cost.error)(cost, ERROR);
        return (v, 0);
    }
    let target = (s.target)(v);
    if !target.primary {
        (s.cost.error)(cost, ERROR);
        return (std::ptr::null_mut(), 0);
    }
    unsafe { (s.commands.ownership)(cost, target.owner, 0) };
    if !(s.cost.success)(cost) {
        return (std::ptr::null_mut(), 0);
    }
    (v, target.owner)
}
/// # Safety
/// Immutable tables and readable name span live for this synchronous call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_unique_name(
    s: &TransactionServices,
    name: ByteSpan,
) -> bool {
    unsafe { unique_name(s, name) }
}
/// # Safety
/// Tables, caller-owned native cost and readable text remain live. No mutable
/// native name view or Rust owner borrow survives a service call. Panics abort.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_rename(
    s: &TransactionServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    text: ByteSpan,
) {
    let (v, _) = unsafe { owned_primary(s, cost, id) };
    if v.is_null() {
        return;
    }
    if text.size != 0 {
        if (s.shared.utf8_length)(text.data, text.size) >= s.max_name {
            return (s.cost.error)(cost, ERROR);
        }
        if flags & DoCommandFlag::AUTO_REPLACE == 0 && !unsafe { unique_name(s, text) } {
            return (s.cost.error)(cost, s.unique_name);
        }
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        if text.size == 0 {
            (s.clear_name)(v);
        } else {
            (s.set_name)(v, text);
        }
        (s.name_dirty)(v);
    }
    (s.cost.zero)(cost);
}
/// # Safety
/// Immutable tables, native cost and resolved vehicle remain live for this call.
/// Setter and presentation leaves may reenter; no owner borrow spans them.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_service_interval(
    s: &TransactionServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    mut interval: u16,
    custom: bool,
    mut percent: bool,
) {
    let (v, owner) = unsafe { owned_primary(s, cost, id) };
    if v.is_null() {
        return;
    }
    // The original resolves the company only after ownership succeeds.
    if !custom {
        percent = (s.interval_percent)(owner);
    }
    if custom {
        if interval != (s.interval_clamp)(i32::from(interval), percent) {
            return (s.cost.error)(cost, ERROR);
        }
    } else {
        interval = (s.interval_default)(owner, unsafe { (s.groups.vehicle_type)(v) }) as u16;
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        (s.set_interval)(v, interval, custom, percent);
    }
    (s.cost.zero)(cost);
}

#[repr(C)]
pub struct SelectionServices {
    pub groups: &'static GroupServices,
    pub transactions: &'static crate::fleet::transactions::Services,
    pub vehicles: &'static crate::vehicle::VehicleServices,
    pub in_depot: extern "C" fn(*mut ()) -> bool,
    pub free_wagon: extern "C" fn(*mut ()) -> bool,
    pub first_engine_part: extern "C" fn(*mut ()) -> *mut (),
    pub multiheaded: extern "C" fn(*mut ()) -> bool,
    pub other_multihead: extern "C" fn(*mut ()) -> *mut (),
    pub with_orders: extern "C" fn(
        *mut (),
        unsafe extern "C" fn(*mut (), *const ()) -> bool,
        unsafe extern "C" fn(*mut (), *const ()) -> bool,
        unsafe extern "C" fn(*mut (), *const ()),
    ),
    pub list_clear: extern "C" fn(*mut ()),
    pub list_push: extern "C" fn(*mut (), *mut ()),
}

unsafe fn vehicle_set(
    s: &SelectionServices,
    mut v: *mut (),
    mut count: u8,
    mut emit: impl FnMut(u32),
) {
    if unsafe { (s.groups.vehicle_type)(v) } != 0 {
        return;
    }
    v = (s.first_engine_part)(v);
    while !v.is_null() && count > 0 {
        loop {
            emit(unsafe { (s.groups.vehicle_id)(v) });
            if (s.multiheaded)(v) {
                emit(unsafe { (s.groups.vehicle_id)((s.other_multihead)(v)) });
            }
            let handle = crate::vehicle::VehicleRef(std::ptr::NonNull::new(v.cast()).unwrap());
            v = (s.vehicles.next)(handle).map_or(std::ptr::null_mut(), |v| v.0.as_ptr().cast());
            if v.is_null() || !unsafe { (s.transactions.articulated)(v) } {
                break;
            }
        }
        count -= 1;
    }
}
/// # Safety
/// A live vehicle and native selection set, with a synchronous include callback.
/// The callback may allocate the native set but cannot mutate vehicle shells.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_selection_set(
    s: &SelectionServices,
    v: *mut (),
    count: u8,
    set: *mut (),
    include: extern "C" fn(*mut (), u32),
) {
    unsafe { vehicle_set(s, v, count, |id| include(set, id)) };
}

unsafe fn depot_list<F: FnMut(*mut (), bool)>(
    s: &SelectionServices,
    kind: u8,
    tile: u32,
    wagons: bool,
    individual: bool,
    mut emit: F,
) {
    struct Visit<'a, F> {
        s: &'a SelectionServices,
        kind: u8,
        wagons: bool,
        individual: bool,
        emit: F,
    }
    unsafe extern "C" fn visit<F: FnMut(*mut (), bool)>(
        ctx: *mut std::ffi::c_void,
        handle: crate::vehicle::VehicleRef,
    ) {
        // Callback runs synchronously; emit mutates only caller-local selection.
        let c = unsafe { &mut *ctx.cast::<Visit<'_, F>>() };
        let v = handle.0.as_ptr().cast();
        if unsafe { (c.s.groups.vehicle_type)(v) } != c.kind || !(c.s.in_depot)(v) {
            return;
        }
        if c.kind == 0 {
            if unsafe { (c.s.transactions.articulated)(v) || (c.s.transactions.rear)(v) } {
                return;
            }
            if c.wagons && (c.s.free_wagon)(unsafe { (c.s.transactions.first)(v) }) {
                if c.individual || (c.s.free_wagon)(v) {
                    (c.emit)(v, true);
                }
                return;
            }
        }
        if unsafe { (c.s.groups.primary)(v) } {
            (c.emit)(v, false);
        }
    }
    let mut ctx = Visit {
        s,
        kind,
        wagons,
        individual,
        emit: &mut emit,
    };
    unsafe {
        (s.vehicles.vehicles_on_tile)(tile, std::ptr::from_mut(&mut ctx).cast(), visit::<&mut F>);
    };
}
/// # Safety
/// Native engine/wagon vectors remain live; they may alias exactly as in the
/// original depot-sale path. Presentation vectors own their native allocations.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_depot_list(
    s: &SelectionServices,
    kind: u8,
    tile: u32,
    engines: *mut (),
    wagons: *mut (),
    individual: bool,
) {
    (s.list_clear)(engines);
    if !wagons.is_null() && engines != wagons {
        (s.list_clear)(wagons);
    }
    unsafe {
        depot_list(s, kind, tile, !wagons.is_null(), individual, |v, wagon| {
            (s.list_push)(if wagon { wagons } else { engines }, v);
        });
    };
}

unsafe fn generate_list<F: FnMut(*mut ())>(
    s: &SelectionServices,
    kind: u8,
    vehicle_type: u8,
    company: u8,
    index: u32,
    mut emit: F,
) -> bool {
    if kind == VL_STATION_LIST || kind == VL_DEPOT_LIST {
        struct Find<'a, F> {
            s: &'a SelectionServices,
            vehicle_type: u8,
            depot: bool,
            destination: u16,
            emit: F,
        }
        unsafe extern "C" fn vehicle<F>(ctx: *mut (), v: *const ()) -> bool {
            let c = unsafe { &*ctx.cast::<Find<'_, F>>() };
            unsafe { (c.s.groups.vehicle_type)(v.cast_mut()) == c.vehicle_type }
        }
        unsafe extern "C" fn order<F>(ctx: *mut (), o: *const ()) -> bool {
            let c = unsafe { &*ctx.cast::<Find<'_, F>>() };
            // Native Order is the aliased canonical Rust-owned scalar record.
            let o = crate::orders::OrderHandle(o.cast());
            unsafe {
                if c.depot {
                    o.is_type(crate::vehicle::OT_GOTO_DEPOT)
                        && !o.nearest_depot()
                        && o.destination() == c.destination
                } else {
                    (o.is_type(crate::vehicle::OT_GOTO_STATION)
                        || o.is_type(OT_GOTO_WAYPOINT)
                        || o.is_type(OT_IMPLICIT))
                        && o.destination() == c.destination
                }
            }
        }
        unsafe extern "C" fn visit<F: FnMut(*mut ())>(ctx: *mut (), v: *const ()) {
            let c = unsafe { &mut *ctx.cast::<Find<'_, F>>() };
            (c.emit)(v.cast_mut());
        }
        let mut ctx = Find {
            s,
            vehicle_type,
            depot: kind == VL_DEPOT_LIST,
            destination: index as u16,
            emit: &mut emit,
        };
        (s.with_orders)(
            std::ptr::from_mut(&mut ctx).cast(),
            vehicle::<&mut F>,
            order::<&mut F>,
            visit::<&mut F>,
        );
        return true;
    }
    if kind == VL_SHARED_ORDERS {
        let mut v = unsafe { (s.groups.vehicle)(index) };
        if v.is_null()
            || unsafe { (s.groups.vehicle_type)(v) != vehicle_type || !(s.groups.primary)(v) }
        {
            return false;
        }
        while !v.is_null() {
            emit(v);
            v = unsafe { (s.groups.next_shared)(v) };
        }
        return true;
    }
    if kind != VL_STANDARD && kind != VL_GROUP_LIST {
        return false;
    }
    let mut from = 0;
    loop {
        let v = unsafe { (s.groups.next_vehicle)(from) }.cast::<()>();
        if v.is_null() {
            break;
        }
        // Preserve the distinct predicate order of group and standard lists.
        if unsafe { (s.groups.vehicle_type)(v) } == vehicle_type {
            let selected = if kind == VL_GROUP_LIST && index as u16 != crate::fleet::ALL_GROUP {
                unsafe {
                    (s.groups.primary)(v)
                        && (s.groups.vehicle_owner)(v) == company
                        && crate::fleet::openttd_rust_fleet_contains(
                            s.groups,
                            (s.groups.vehicle_group)(v),
                            index as u16,
                        )
                }
            } else {
                unsafe { (s.groups.vehicle_owner)(v) == company && (s.groups.primary)(v) }
            };
            if selected {
                emit(v);
            }
        }
        from = unsafe { (s.groups.vehicle_id)(v) }.wrapping_add(1);
    }
    true
}
/// # Safety
/// The native presentation list is live and remains valid across append leaves.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_generate_list(
    s: &SelectionServices,
    list: *mut (),
    kind: u8,
    vehicle_type: u8,
    company: u8,
    index: u32,
) -> bool {
    (s.list_clear)(list);
    unsafe {
        generate_list(s, kind, vehicle_type, company, index, |v| {
            (s.list_push)(list, v);
        })
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct GrfStringId(pub u32);
#[repr(C)]
pub struct AircraftStartState {
    pub state: u8,
    pub flags: u8,
}
#[repr(C)]
pub struct TransactionStrings {
    pub destroyed: u32,
    pub stopped_in_depot: u32,
    pub no_power: u32,
    pub in_flight: u32,
    pub incompatible_rail: u32,
    pub null_string: u32,
    pub grf_text: GrfStringId,
    pub takeoff: u8,
    pub terminal7: u8,
    pub direct_descent: u8,
    pub new_vehicle_expense: u8,
    pub invalid_client: u32,
    pub share_orders: u8,
    pub copy_orders: u8,
    pub cargo_count: u8,
    pub invalid_cargo: u8,
}
#[repr(C)]
pub struct CommonServices {
    pub base: &'static TransactionServices,
    pub transactions: &'static crate::fleet::transactions::Services,
    pub vehicles: &'static crate::vehicle::VehicleServices,
    pub orders: &'static crate::orders::Leaves,
    pub strings: TransactionStrings,
    pub train_power: extern "C" fn(*mut ()) -> u32,
    pub aircraft_start: extern "C" fn(*mut ()) -> AircraftStartState,
    pub startstop_callback: extern "C" fn(*mut (), *mut i32) -> u16,
    pub grf_version: extern "C" fn(*mut ()) -> u8,
    pub grf_string: extern "C" fn(*mut (), GrfStringId) -> u32,
    pub waiting_news: extern "C" fn(u32),
    pub flip_stopped: extern "C" fn(*mut ()),
    pub startstop_ui: extern "C" fn(*mut ()),
    pub sell_rail: extern "C" fn(*mut CommandCost, u16, *mut (), bool, bool, u32),
    pub value: extern "C" fn(*mut ()) -> i64,
    pub valid_cargo: extern "C" fn(u8) -> bool,
}
/// # Safety
/// `v` denotes a live shell, established by the caller's source preconditions.
unsafe fn vehicle_ref(v: *mut ()) -> crate::vehicle::VehicleRef {
    crate::vehicle::VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(v.cast()) })
}
pub(crate) unsafe fn sell(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    chain: bool,
    backup: bool,
    client: u32,
) {
    let b = s.base;
    let v = unsafe { (b.groups.vehicle)(id) };
    if v.is_null() {
        return (b.cost.error)(cost, ERROR);
    }
    let front = unsafe { (s.transactions.first)(v) };
    unsafe { (b.commands.ownership)(cost, (b.groups.vehicle_owner)(front), 0) };
    if !(b.cost.success)(cost) {
        return;
    }
    if unsafe { (s.transactions.crashed)(front) } {
        return (b.cost.error)(cost, s.strings.destroyed);
    }
    if !unsafe { (s.transactions.stopped_in_depot)(front) } {
        return (b.cost.error)(
            cost,
            s.strings.stopped_in_depot + u32::from(unsafe { (b.groups.vehicle_type)(front) }),
        );
    }
    if unsafe { (b.groups.vehicle_type)(v) } == 0 {
        return (s.sell_rail)(cost, flags, v, chain, backup, client);
    }
    (b.cost.expense_money)(
        cost,
        s.strings.new_vehicle_expense,
        (s.value)(front).saturating_neg(),
    );
    if flags & DoCommandFlag::EXECUTE != 0 {
        if unsafe { (b.groups.primary)(front) } && backup {
            unsafe { crate::orders::openttd_rust_backup_create(front.cast(), client, s.orders) };
        }
        // No owner access or retained reference crosses the deleting callback.
        (s.vehicles.destroy)(unsafe { vehicle_ref(front) });
    }
}
pub(crate) unsafe fn startstop(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    mut evaluate: bool,
) {
    if flags & DoCommandFlag::AUTO_REPLACE == 0 {
        evaluate = true;
    }
    let b = s.base;
    let (v, _) = unsafe { owned_primary(b, cost, id) };
    if v.is_null() {
        return;
    }
    let handle = unsafe { vehicle_ref(v) };
    let status = (s.vehicles.status)(handle);
    if status & crate::vehicle::VS_CRASHED != 0 {
        return (b.cost.error)(cost, s.strings.destroyed);
    }
    match unsafe { (b.groups.vehicle_type)(v) } {
        0 => {
            if status & crate::vehicle::VS_STOPPED != 0 && (s.train_power)(v) == 0 {
                return (b.cost.error)(cost, s.strings.no_power);
            }
        }
        1 | 2 => (),
        3 => {
            let state = (s.aircraft_start)(v);
            if (state.state >= s.strings.takeoff && state.state < s.strings.terminal7)
                || state.flags & s.strings.direct_descent != 0
            {
                return (b.cost.error)(cost, s.strings.in_flight);
            }
        }
        _ => return (b.cost.error)(cost, ERROR),
    }
    if evaluate {
        let mut register100 = std::mem::MaybeUninit::<i32>::uninit();
        let callback = (s.startstop_callback)(v, register100.as_mut_ptr());
        let mut error = s.strings.null_string;
        if callback != crate::vehicle::CALLBACK_FAILED {
            if (s.grf_version)(v) < 8 {
                if callback < 0x400 && callback & 0xff != 0xff {
                    error = (s.grf_string)(
                        v,
                        GrfStringId(s.strings.grf_text.0.wrapping_add(u32::from(callback))),
                    );
                }
            } else if callback < 0x400 {
                error = (s.grf_string)(
                    v,
                    GrfStringId(s.strings.grf_text.0.wrapping_add(u32::from(callback))),
                );
            } else {
                match callback {
                    0x400 => (),
                    0x40f => {
                        // The original reads register 100 only for this result.
                        error = (s.grf_string)(
                            v,
                            GrfStringId(unsafe { register100.assume_init() } as u32),
                        );
                    }
                    _ => error = s.strings.incompatible_rail,
                }
            }
        }
        if error != s.strings.null_string {
            return (b.cost.error)(cost, error);
        }
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        if unsafe { (s.transactions.stopped_in_depot)(v) }
            && flags & DoCommandFlag::AUTO_REPLACE == 0
        {
            (s.waiting_news)(id);
        }
        (s.flip_stopped)(v);
        if unsafe { (b.groups.vehicle_type)(v) } != 0 {
            (s.vehicles.write_speed)(handle, 0);
        }
        (s.vehicles.reset_depot_unbunching)(handle);
        // Native presentation may reenter owners; no Rust owner borrow spans it.
        (unsafe { &*s.vehicles.maintenance }.mark_dirty)(handle);
        (s.startstop_ui)(v);
    }
    (b.cost.zero)(cost);
}
/// # Safety
/// Immutable service tables and native stack cost remain live; shell ownership
/// follows the original command preconditions. No borrow survives deletion/reentry.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_sell(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    chain: bool,
    backup: bool,
    client: u32,
) {
    unsafe { sell(s, cost, flags, id, chain, backup, client) };
}
/// # Safety
/// Immutable service tables and stack cost remain live for this synchronous entry.
/// Register 100 follows the original successful `NewGRF` callback preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_startstop(
    s: &CommonServices,
    cost: *mut CommandCost,
    flags: u16,
    id: u32,
    evaluate: bool,
) {
    unsafe { startstop(s, cost, flags, id, evaluate) };
}
