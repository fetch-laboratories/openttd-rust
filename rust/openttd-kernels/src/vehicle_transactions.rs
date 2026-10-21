/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common vehicle transactions. Native shells, names and stack costs stay native;
//! Rust owns policy and transaction scratch. All accesses end before callbacks.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::too_many_arguments
)]
use crate::fleet::GroupServices;
use crate::services::{ByteSpan, CommandCost, CommandCostServices, CommandServices, Services};

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
    pub interval_default: extern "C" fn(u8, u8) -> u16,
    pub interval_clamp: extern "C" fn(i32, bool) -> u16,
    pub set_interval: extern "C" fn(*mut (), u16, bool, bool),
    pub unique_name: u32,
    pub max_name: usize,
}
const EXECUTE: u16 = 1;
const AUTOREPLACE: u16 = 1 << 6;
const ERROR: u32 = 0xffff;

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
    unsafe { (s.commands.ownership)(cost, target.owner) };
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
        if flags & AUTOREPLACE == 0 && !unsafe { unique_name(s, text) } {
            return (s.cost.error)(cost, s.unique_name);
        }
    }
    if flags & EXECUTE != 0 {
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
        interval = (s.interval_default)(owner, unsafe { (s.groups.vehicle_type)(v) });
    }
    if flags & EXECUTE != 0 {
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
    if kind == 2 || kind == 3 {
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
                    (o.is_type(crate::vehicle::OT_GOTO_STATION) || o.is_type(6) || o.is_type(8))
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
            depot: kind == 3,
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
    if kind == 1 {
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
    if kind != 0 && kind != 4 {
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
            let selected = if kind == 4 && index as u16 != 65533 {
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
