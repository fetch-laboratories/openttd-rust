/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Deferred refit computation and commit. Active frames are owned by their stack
//! transaction; only cleared inactive frames remain in process-wide scratch.
#![allow(clippy::too_many_lines, clippy::many_single_char_names)]
use super::{CommonServices, ERROR, SelectionServices, vehicle_ref, vehicle_set};
use crate::command_type::DoCommandFlag;
use crate::economy_func::get_price;
use crate::game_state::GameState;
use crate::services::{CommandCost, ExpenseType};
use crate::vehicle::{OpenTTDEngine, OpenTTDVehicle, RailVehicleTypes, VehicleType};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Capacity {
    pub total: u32,
    pub mail: u16,
    pub cargo: [u32; 64],
}
impl Default for Capacity {
    fn default() -> Self {
        Self {
            total: 0,
            mail: 0,
            cargo: [0; 64],
        }
    }
}
#[repr(C)]
pub struct CargoKind {
    pub cargo: u8,
    pub subtype: u8,
}
#[repr(C)]
pub struct Commit {
    pub cargo: u8,
    pub capacity: u16,
}
#[repr(C)]
pub struct Engine {
    pub mask: u64,
    pub automatic: bool,
}
#[repr(C)]
pub struct Cargo {
    pub classes: u16,
    pub local: u8,
}
#[repr(C)]
pub struct Prices {
    pub train: u32,
    pub wagon: u32,
    pub road: u32,
    pub ship: u32,
    pub aircraft: u32,
    pub train_run: u8,
    pub road_run: u8,
    pub ship_run: u8,
    pub aircraft_run: u8,
    pub auto_refit: u8,
    pub refit: u8,
}
#[repr(C)]
pub struct Services {
    pub common: &'static CommonServices,
    pub selection: &'static SelectionServices,
    pub cargo: &'static crate::cargo_storage::Services,
    pub scheduler: &'static crate::vehicle::scheduler::SchedulerServices,
    pub train: &'static crate::train::Leaves,
    pub road: &'static crate::road::Leaves,
    pub aircraft: &'static crate::aircraft::Leaves,
    pub ship: &'static crate::ship_control::Leaves,
    pub road_ground: &'static crate::vehicle::GroundServices,
    pub prices: Prices,
    pub engine: extern "C" fn(u16) -> *const OpenTTDEngine,
    pub vehicle_engine: extern "C" fn(*mut OpenTTDVehicle) -> *const OpenTTDEngine,
    pub has_grf: extern "C" fn(*const OpenTTDEngine) -> bool,
    pub refit_cost: extern "C" fn(*const OpenTTDEngine) -> u8,
    pub refit_cargo: extern "C" fn(*const OpenTTDEngine, u8) -> Cargo,
    pub engine_kind: extern "C" fn(*const OpenTTDEngine) -> VehicleType,
    pub rail_kind: extern "C" fn(*const OpenTTDEngine) -> RailVehicleTypes,
    pub price_shift: extern "C" fn(*const OpenTTDEngine, u32) -> i32,
    pub can_carry: extern "C" fn(*const OpenTTDEngine) -> bool,
    pub refit_engine: extern "C" fn(*const OpenTTDEngine) -> Engine,
    pub capacity: extern "C" fn(*const OpenTTDEngine, *mut OpenTTDVehicle, *mut u16) -> u32,
    pub cargo_kind: extern "C" fn(*mut OpenTTDVehicle) -> CargoKind,
    pub set_cargo_kind: extern "C" fn(*mut OpenTTDVehicle, u8, u8),
    pub expense: extern "C" fn(*mut OpenTTDVehicle, bool) -> ExpenseType,
    pub invalidate_chain: extern "C" fn(*mut OpenTTDVehicle),
    pub commit: extern "C" fn(*mut OpenTTDVehicle) -> Commit,
    pub set_refit_cap: extern "C" fn(*mut OpenTTDVehicle, u16),
    pub refit_cap: extern "C" fn(*mut OpenTTDVehicle) -> u16,
    pub cargo_cap: extern "C" fn(*mut OpenTTDVehicle) -> u16,
    pub set_capacity: extern "C" fn(*mut OpenTTDVehicle, u16),
    pub write_refit: extern "C" fn(*mut OpenTTDVehicle, u8, u16, u8),
    pub mail_cargo: extern "C" fn() -> u8,
    pub train_handle: extern "C" fn(*mut OpenTTDVehicle) -> crate::train::Handle,
    pub road_state: extern "C" fn(*mut OpenTTDVehicle) -> *mut crate::road::State,
    pub aircraft_handle: extern "C" fn(*mut OpenTTDVehicle) -> crate::aircraft::Vehicle,
    pub road_acceleration: extern "C" fn() -> u8,
    pub details_ui: extern "C" fn(*mut OpenTTDVehicle),
    pub lists_ui: extern "C" fn(u8),
}
#[derive(Clone, Copy)]
struct Result {
    vehicle: *mut (),
    capacity: u32,
    mail: u32,
    subtype: u8,
}
#[derive(Default)]
struct Frame {
    selection: Vec<u32>,
    results: Vec<Result>,
}
static IDLE: GameState<Vec<Frame>> = GameState::new(Vec::new());
struct Transaction(Frame);
impl Transaction {
    fn new() -> Self {
        // SAFETY: Serialized short access; an active frame is removed from the pool.
        Self(unsafe { (&mut *IDLE.get()).pop() }.unwrap_or_default())
    }
}
impl Drop for Transaction {
    fn drop(&mut self) {
        self.0.selection.clear();
        self.0.results.clear();
        // SAFETY: Raw IDs/handles have no dropping callbacks. No borrow survives
        // publishing the now-empty frame; nested commands own distinct frames.
        unsafe {
            (&mut *IDLE.get()).push(std::mem::take(&mut self.0));
        }
    }
}
unsafe fn cost_factor(
    s: &Services,
    v: *mut (),
    engine: u16,
    cargo: u8,
    subtype: u8,
) -> (i32, bool) {
    let e = (s.engine)(engine);
    if (s.has_grf)(e) {
        let cs = (s.refit_cargo)(e, cargo);
        let param = (u32::from(cs.classes) << 16) | (u32::from(subtype) << 8) | u32::from(cs.local);
        let result = unsafe {
            crate::newgrf_vehicle::callback(
                s.common.vehicles,
                std::ptr::NonNull::new(v.cast()).map(crate::vehicle::VehicleRef),
                engine,
                crate::newgrf_callbacks::CBID_VEHICLE_REFIT_COST.0,
                param,
                std::ptr::null_mut(),
                false,
                std::ptr::null_mut(),
                0,
            )
        };
        if result != crate::vehicle::CALLBACK_FAILED {
            let mut factor = i32::from(result & 0x3fff);
            if factor >= 0x2000 {
                factor -= 0x4000;
            }
            return (factor, result & (1 << 14) != 0);
        }
    }
    let factor = (s.refit_cost)(e);
    let changed = v.is_null() || unsafe { (s.common.transactions.cargo_type)(v) } != cargo;
    (if changed { i32::from(factor) } else { 0 }, factor == 0)
}
pub(super) unsafe fn get_cost(
    s: &Services,
    out: *mut CommandCost,
    v: *mut (),
    engine: u16,
    cargo: u8,
    subtype: u8,
) -> bool {
    let e = (s.engine)(engine);
    let (mut factor, automatic) = unsafe { cost_factor(s, v, engine, cargo, subtype) };
    let (price, expense) = match (s.engine_kind)(e) {
        VehicleType::SHIP => (s.prices.ship, s.prices.ship_run),
        VehicleType::ROAD => (s.prices.road, s.prices.road_run),
        VehicleType::AIRCRAFT => (s.prices.aircraft, s.prices.aircraft_run),
        VehicleType::TRAIN => {
            let price = if (s.rail_kind)(e) == RailVehicleTypes::WAGON {
                s.prices.wagon
            } else {
                s.prices.train
            };
            factor = factor.wrapping_mul(2);
            (price, s.prices.train_run)
        }
        _ => unreachable!("original NOT_REACHED vehicle type"),
    };
    let negative = factor < 0;
    let amount = get_price(price, factor.unsigned_abs(), (s.price_shift)(e, price), -10);
    (s.common.base.cost.expense_money)(
        out,
        expense,
        if negative {
            amount.saturating_neg()
        } else {
            amount
        },
    );
    automatic
}
pub(super) unsafe fn next(s: &Services, v: *mut ()) -> *mut () {
    (s.common.vehicles.next)(unsafe { vehicle_ref(v) })
        .map_or(ptr::null_mut(), |v| v.0.as_ptr().cast())
}
unsafe fn truncate(s: &Services, v: *mut (), cap: u16) {
    let owner = (s.scheduler.cargo_owner)(unsafe { vehicle_ref(v) });
    if unsafe { crate::cargo_storage::openttd_rust_cargo_list_count(owner) } > u32::from(cap) {
        let amount = unsafe { crate::cargo_storage::openttd_rust_cargo_list_count(owner) }
            .wrapping_sub(u32::from(cap));
        unsafe {
            crate::cargo_storage::truncate_vehicle(owner, *s.cargo, amount);
        }
    }
}
unsafe fn refit_chain(
    s: &Services,
    cost: *mut CommandCost,
    temp: *mut CommandCost,
    mut v: *mut (),
    only: bool,
    count: u8,
    cargo: u8,
    subtype: u8,
    flags: u16,
    automatic: bool,
) -> Capacity {
    let c = s.common;
    let b = c.base;
    (b.cost.expense_money)(cost, (s.expense)(v.cast(), false).0, 0);
    let mut total = Capacity::default();
    let mut mail_total = 0_u32;
    let mut transaction = Transaction::new();
    if !only {
        unsafe {
            vehicle_set(
                s.selection,
                v,
                if count == 0 { u8::MAX } else { count },
                |id| {
                    if !transaction.0.selection.contains(&id) {
                        transaction.0.selection.push(id);
                    }
                },
            );
        }
        v = unsafe { (c.transactions.first)(v) };
    }
    (s.invalidate_chain)(v.cast());
    let mut actual_subtype = subtype;
    while !v.is_null() {
        if !unsafe { (c.transactions.articulated)(v) } {
            actual_subtype = subtype;
        }
        if unsafe { (b.groups.vehicle_type)(v) } == VehicleType::TRAIN.0
            && !only
            && !transaction
                .0
                .selection
                .contains(&unsafe { (b.groups.vehicle_id)(v) })
        {
            v = unsafe { next(s, v) };
            continue;
        }
        let e = (s.vehicle_engine)(v.cast());
        if !(s.can_carry)(e) {
            v = if only {
                ptr::null_mut()
            } else {
                unsafe { next(s, v) }
            };
            continue;
        }
        let info = (s.refit_engine)(e);
        let refittable = info.mask & (1_u64 << cargo) != 0 && (!automatic || info.automatic);
        if !refittable && unsafe { (c.transactions.cargo_type)(v) } != cargo {
            let amount = (s.capacity)(e, v.cast(), ptr::null_mut());
            if amount > 0 {
                let original = unsafe { (c.transactions.cargo_type)(v) };
                total.cargo[usize::from(original)] =
                    total.cargo[usize::from(original)].wrapping_add(amount);
            }
            v = if only {
                ptr::null_mut()
            } else {
                unsafe { next(s, v) }
            };
            continue;
        }
        if actual_subtype == 0xff {
            actual_subtype = unsafe { (c.transactions.subtype)(v, v, cargo) };
        }
        let old = (s.cargo_kind)(v.cast());
        if refittable {
            (s.set_cargo_kind)(v.cast(), cargo, actual_subtype);
        }
        let mut mail = 0_u16;
        let amount = (s.capacity)(e, v.cast(), &raw mut mail);
        total.total = total.total.wrapping_add(amount);
        mail_total = mail_total.wrapping_add(u32::from(mail));
        total.cargo[usize::from(cargo)] = total.cargo[usize::from(cargo)].wrapping_add(amount);
        let mail_cargo = (s.mail_cargo)();
        if (s.common.valid_cargo)(mail_cargo) {
            total.cargo[usize::from(mail_cargo)] =
                total.cargo[usize::from(mail_cargo)].wrapping_add(u32::from(mail));
        }
        if refittable {
            (s.set_cargo_kind)(v.cast(), old.cargo, old.subtype);
            let engine = unsafe { (b.groups.vehicle_engine)(v) };
            let allowed = unsafe { get_cost(s, temp, v, engine, cargo, actual_subtype) };
            if automatic && flags & DoCommandFlag::QUERY_COST == 0 && !allowed {
                total.total = total.total.wrapping_sub(amount);
                mail_total = mail_total.wrapping_sub(u32::from(mail));
                if unsafe { (c.transactions.cargo_type)(v) } == cargo {
                    total.total = total.total.wrapping_add(u32::from((s.cargo_cap)(v.cast())));
                    if unsafe { (b.groups.vehicle_type)(v) } == VehicleType::AIRCRAFT.0 {
                        mail_total = mail_total
                            .wrapping_add(u32::from((s.cargo_cap)(unsafe { next(s, v) }.cast())));
                    }
                }
            } else {
                (b.cost.add)(cost, temp);
                transaction.0.results.push(Result {
                    vehicle: v,
                    capacity: amount,
                    mail: u32::from(mail),
                    subtype: actual_subtype,
                });
            }
        }
        v = if only {
            ptr::null_mut()
        } else {
            unsafe { next(s, v) }
        };
    }
    if flags & DoCommandFlag::EXECUTE != 0 {
        for index in 0..transaction.0.results.len() {
            // Copy only the private result; no Vec borrow crosses a callback.
            let result = transaction.0.results[index];
            let v = result.vehicle;
            let old = (s.commit)(v.cast());
            let cap = if old.cargo == cargo {
                (result.capacity as u16).min(old.capacity)
            } else {
                0
            };
            (s.set_refit_cap)(v.cast(), cap);
            unsafe {
                truncate(s, v, cap);
            }
            (s.write_refit)(v.cast(), cargo, result.capacity as u16, result.subtype);
            if unsafe { (b.groups.vehicle_type)(v) } == VehicleType::AIRCRAFT.0 {
                let shadow = unsafe { next(s, v) };
                if c.transactions.assertions {
                    unsafe {
                        (c.transactions.check)(!shadow.is_null());
                    }
                }
                let cap = (s.refit_cap)(shadow.cast()).min(result.mail as u16);
                (s.set_refit_cap)(shadow.cast(), cap);
                (s.set_capacity)(shadow.cast(), result.mail as u16);
                unsafe {
                    truncate(s, shadow, cap);
                }
            }
        }
    }
    total.mail = mail_total as u16;
    total
}
pub(crate) unsafe fn refit(
    s: &Services,
    cost: *mut CommandCost,
    temp: *mut CommandCost,
    flags: u16,
    id: u32,
    cargo: u8,
    subtype: u8,
    automatic: bool,
    mut only: bool,
    count: u8,
) -> Capacity {
    let c = s.common;
    let b = c.base;
    let v = unsafe { (b.groups.vehicle)(id) };
    if v.is_null() {
        (b.cost.error)(cost, ERROR);
        return Capacity::default();
    }
    let kind = unsafe { (b.groups.vehicle_type)(v) };
    if !unsafe { (b.groups.buildable_type)(kind) } {
        (b.cost.error)(cost, ERROR);
        return Capacity::default();
    }
    let front = unsafe { (c.transactions.first)(v) };
    unsafe {
        (b.commands.ownership)(cost, (b.groups.vehicle_owner)(front), 0);
    }
    if !(b.cost.success)(cost) {
        return Capacity::default();
    }
    let free = kind == VehicleType::TRAIN.0 && (s.selection.free_wagon)(front);
    if v != front && matches!(VehicleType(kind), VehicleType::SHIP | VehicleType::AIRCRAFT) {
        (b.cost.error)(cost, ERROR);
        return Capacity::default();
    }
    let loading = || {
        if automatic {
            let state = unsafe { (c.orders.vehicle)(front.cast()) };
            unsafe {
                crate::orders::OrderHandle(ptr::addr_of!((*state).current).cast())
                    .is_type(crate::vehicle::OT_LOADING)
            }
        } else {
            false
        }
    };
    if flags & DoCommandFlag::QUERY_COST == 0
        && !free
        && (!automatic || !loading())
        && !unsafe { (c.transactions.stopped_in_depot)(front) }
    {
        (b.cost.error)(
            cost,
            c.strings.stopped_in_depot + u32::from(unsafe { (b.groups.vehicle_type)(front) }),
        );
        return Capacity::default();
    }
    if unsafe { (c.transactions.crashed)(front) } {
        (b.cost.error)(cost, c.strings.destroyed);
        return Capacity::default();
    }
    if cargo >= c.strings.cargo_count {
        (b.cost.error)(cost, ERROR);
        return Capacity::default();
    }
    only |= matches!(
        VehicleType(unsafe { (b.groups.vehicle_type)(front) }),
        VehicleType::SHIP | VehicleType::AIRCRAFT
    );
    let result = unsafe {
        refit_chain(
            s, cost, temp, v, only, count, cargo, subtype, flags, automatic,
        )
    };
    if flags & DoCommandFlag::EXECUTE != 0 {
        match VehicleType(unsafe { (b.groups.vehicle_type)(v) }) {
            VehicleType::TRAIN => unsafe {
                crate::train::openttd_rust_train_consist_changed(
                    (s.train_handle)(front.cast()),
                    if automatic {
                        s.prices.auto_refit
                    } else {
                        s.prices.refit
                    },
                    s.train,
                    b.shared,
                );
            },
            VehicleType::ROAD => {
                unsafe {
                    crate::road::openttd_rust_road_update_cache(
                        vehicle_ref(front),
                        (s.road_state)(front.cast()),
                        automatic,
                        s.road,
                    );
                }
                if (s.road_acceleration)() != 0 {
                    crate::ground_vehicle::cargo_changed(
                        s.road_ground.physics,
                        (s.road_ground.physics.first)(unsafe { vehicle_ref(front) }),
                    );
                }
            }
            VehicleType::SHIP => {
                (s.invalidate_chain)(v.cast());
                unsafe {
                    crate::ship_control::openttd_rust_ship_cache(
                        vehicle_ref(v),
                        (b.groups.vehicle_id)(v),
                        s.ship,
                        b.shared,
                    );
                }
            }
            VehicleType::AIRCRAFT => {
                (s.invalidate_chain)(v.cast());
                unsafe {
                    crate::aircraft::openttd_rust_aircraft_cache(
                        s.aircraft,
                        (s.aircraft_handle)(v.cast()),
                        true,
                    );
                }
            }
            _ => unreachable!("original NOT_REACHED vehicle type"),
        }
        (unsafe { &*c.vehicles.maintenance }.mark_dirty)(unsafe { vehicle_ref(front) });
        if !free {
            (s.details_ui)(front.cast());
            (s.lists_ui)(unsafe { (b.groups.vehicle_type)(v) });
        }
        unsafe {
            (b.groups.depot_dirty)(front);
        }
    } else {
        (s.invalidate_chain)(v.cast());
    }
    result
}
/// # Safety
/// Immutable tables and distinct native stack cost slots remain live. The
/// serialized owner and callback preconditions match the original commands.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_refit(
    s: &Services,
    cost: *mut CommandCost,
    temp: *mut CommandCost,
    flags: u16,
    id: u32,
    cargo: u8,
    subtype: u8,
    automatic: bool,
    only: bool,
    count: u8,
) -> Capacity {
    unsafe {
        refit(
            s, cost, temp, flags, id, cargo, subtype, automatic, only, count,
        )
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleRefitCapacity" ["probe"] => Capacity {
        total, mail, cargo,
    };
    "OpenTTDRefitCargoKind" ["probe"] => CargoKind {
        cargo, subtype,
    };
    "OpenTTDRefitCommit" ["probe"] => Commit {
        cargo, capacity,
    };
    "OpenTTDRefitEngine" ["probe"] => Engine {
        mask, automatic,
    };
    "OpenTTDRefitCargo" ["probe"] => Cargo {
        classes, local,
    };
    "OpenTTDRefitPrices" ["probe"] => Prices {
        train, wagon, road, ship, aircraft, train_run, road_run, ship_run, aircraft_run, auto_refit, refit,
    };
    "OpenTTDVehicleRefitServices" ["probe"] => Services {
        common, selection, cargo, scheduler, train, road, aircraft, ship, road_ground, prices, engine, vehicle_engine, has_grf, refit_cost, refit_cargo, engine_kind, rail_kind, price_shift, can_carry, refit_engine, capacity, cargo_kind, set_cargo_kind, expense, invalidate_chain, commit, set_refit_cap, refit_cap, cargo_cap, set_capacity, write_refit, mail_cargo, train_handle, road_state, aircraft_handle, road_acceleration, details_ui, lists_ui,
    };
}
