/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Common build transaction, including speculative construction and RNG rollback.
use super::{ERROR, construct, mass, rail, refit};
use crate::command_type::DoCommandFlag;
use crate::services::{CommandCost, Cost, RandomServices};
use crate::vehicle::{OpenTTDEngine, RailVehicleTypes, VehicleType};
use std::ptr;
#[repr(C)]
pub struct Result {
    pub id: u32,
    pub capacity: refit::Capacity,
}
impl Default for Result {
    fn default() -> Self {
        Self {
            id: crate::fleet::INVALID_VEHICLE,
            capacity: refit::Capacity::default(),
        }
    }
}
#[repr(C)]
pub struct Services {
    pub refit: &'static refit::Services,
    pub mass: &'static mass::Services,
    pub random: &'static RandomServices,
    pub depot_kind: extern "C" fn(u32) -> VehicleType,
    pub engine_cost: extern "C" fn(*const OpenTTDEngine) -> i64,
    pub default_cargo: extern "C" fn(*const OpenTTDEngine) -> u8,
    pub articulate_count: extern "C" fn(u16, bool) -> u32,
    pub pool_available: extern "C" fn(usize) -> bool,
    pub free_unit: extern "C" fn(VehicleType) -> u16,
    pub construct: &'static construct::Services,
    pub rail: &'static rail::Services,
    pub ship_tile: extern "C" fn(u32) -> u32,
    pub new_ship: extern "C" fn(u32) -> *mut (),
    pub set_purchase: extern "C" fn(*mut (), u16, i64),
    pub articulated_capacity: extern "C" fn(u16, *mut u32),
    pub display_capacity: extern "C" fn(*const OpenTTDEngine, *mut u16) -> u32,
    pub is_engine: extern "C" fn(*mut ()) -> bool,
    pub presentation: extern "C" fn(*mut (), VehicleType),
    pub use_unit: extern "C" fn(*mut ()),
    pub too_many: u32,
    pub unavailable: u32,
}

/// # Safety
/// All tables and three distinct constructed native costs live through the call.
/// Native allocations produce live shells. Every owner access ends before any
/// callback; speculative destruction leaves only the original returned ID value.
#[allow(clippy::too_many_lines, clippy::many_single_char_names)]
pub(crate) unsafe fn build(
    s: &'static Services,
    out: *mut CommandCost,
    temporary: *mut CommandCost,
    refit_temporary: *mut CommandCost,
    flags: u16,
    tile: u32,
    engine: u16,
    use_free: bool,
    cargo: u8,
    client: u32,
) -> Result {
    let r = s.refit;
    let c = r.common;
    let b = c.base;
    let value = Cost(out, b.cost);
    let company = (b.shared.current_company)();
    if !(s.mass.is_depot)(tile) || !crate::tile_map::is_tile_owner(b.shared, tile, company) {
        value.error(ERROR);
        return Result::default();
    }
    let kind = (s.depot_kind)(tile);
    if !unsafe { (c.transactions.engine_buildable)(engine, kind.0, company) } {
        value.error(s.unavailable.wrapping_add(u32::from(kind.0)));
        return Result::default();
    }
    if cargo >= c.strings.cargo_count && (c.valid_cargo)(cargo) {
        value.error(ERROR);
        return Result::default();
    }
    let e = (r.engine)(engine);
    (b.cost.expense_money)(out, c.strings.new_vehicle_expense, (s.engine_cost)(e));
    let default_cargo = (s.default_cargo)(e);
    if !(c.valid_cargo)(default_cargo) {
        value.error(ERROR);
        return Result::default();
    }
    let refitting = (c.valid_cargo)(cargo) && cargo != default_cargo;
    let count = match kind.0 {
        0 => {
            u32::from((r.rail_kind)(e) == RailVehicleTypes::MULTIHEAD)
                + 1
                + (s.articulate_count)(engine, false)
        }
        1 => 1 + (s.articulate_count)(engine, false),
        2 => 1,
        3 => {
            if unsafe { (c.transactions.plane)(engine) } != 0 {
                2
            } else {
                3
            }
        }
        _ => unreachable!("depot kind is a buildable vehicle type"),
    };
    if !(s.pool_available)(count as usize) {
        value.error(s.too_many);
        return Result::default();
    }
    let unit = if flags & (DoCommandFlag::QUERY_COST | DoCommandFlag::AUTO_REPLACE) != 0
        || (kind == VehicleType::TRAIN && (r.rail_kind)(e) == RailVehicleTypes::WAGON)
    {
        0
    } else {
        (s.free_unit)(kind)
    };
    if unit == u16::MAX {
        value.error(s.too_many);
        return Result::default();
    }
    let subflags = if refitting && flags & DoCommandFlag::EXECUTE == 0 {
        flags | DoCommandFlag::EXECUTE | DoCommandFlag::AUTO_REPLACE
    } else {
        flags
    };
    let seeds = unsafe { (s.random.save)() };
    let v = match kind.0 {
        0 => unsafe { rail::build(s, temporary, subflags, tile, e) },
        1 => unsafe { construct::road(s, temporary, subflags, tile, e) },
        2 => {
            let north = (s.ship_tile)(tile);
            let v = if subflags & DoCommandFlag::EXECUTE != 0 {
                let v = (s.new_ship)(north);
                let id = unsafe { (b.groups.vehicle_id)(v) };
                unsafe {
                    crate::ship_control::openttd_rust_ship_build(
                        id,
                        u32::from(engine),
                        r.ship,
                        b.shared,
                    );
                };
                v
            } else {
                ptr::null_mut()
            };
            (b.cost.zero)(temporary);
            v
        }
        3 => unsafe { construct::aircraft(s, temporary, subflags, tile, e) },
        _ => unreachable!("depot kind is a buildable vehicle type"),
    };
    value.add(Cost(temporary, b.cost));
    let mut result = Result::default();
    // This is deliberately one success test, before refitting. A failed refit
    // still follows the original accounting, temporary sale and restoration.
    if value.ok() {
        if subflags & DoCommandFlag::EXECUTE != 0 {
            (s.set_purchase)(v, unit, value.money());
            result.id = unsafe { (b.groups.vehicle_id)(v) };
        }
        if refitting {
            result.capacity = unsafe {
                refit::refit(
                    r,
                    temporary,
                    refit_temporary,
                    flags,
                    (b.groups.vehicle_id)(v),
                    cargo,
                    0,
                    false,
                    false,
                    1,
                )
            };
            value.add(Cost(temporary, b.cost));
        } else if kind == VehicleType::TRAIN || kind == VehicleType::ROAD {
            (s.articulated_capacity)(engine, result.capacity.cargo.as_mut_ptr());
            result.capacity.total = result.capacity.cargo[usize::from(default_cargo)];
        } else {
            result.capacity.total = (s.display_capacity)(e, &raw mut result.capacity.mail);
            result.capacity.cargo[usize::from(default_cargo)] = result.capacity.total;
            let mail = (r.mail_cargo)();
            if (c.valid_cargo)(mail) {
                result.capacity.cargo[usize::from(mail)] = u32::from(result.capacity.mail);
            }
        }
        if flags & DoCommandFlag::EXECUTE != 0 {
            if kind == VehicleType::TRAIN
                && use_free
                && flags & DoCommandFlag::AUTO_REPLACE == 0
                && (s.is_engine)(v)
            {
                // The speculative-build result in `temporary` was consumed above.
                unsafe { rail::normalize(s, temporary, v) };
            }
            (s.presentation)(v, kind);
        }
        if subflags & DoCommandFlag::EXECUTE != 0 {
            unsafe {
                crate::fleet::openttd_rust_fleet_count_engine(b.groups, v, 1);
                crate::fleet::openttd_rust_fleet_update_autoreplace(
                    b.groups,
                    (b.shared.current_company)(),
                );
            }
            if unsafe { (b.groups.primary)(v) } {
                unsafe { crate::fleet::openttd_rust_fleet_count_vehicle(b.groups, v, 1) };
                if subflags & DoCommandFlag::AUTO_REPLACE == 0 {
                    unsafe {
                        crate::orders::openttd_rust_backup_restore(v.cast(), client, c.orders)
                    };
                }
            }
            (s.use_unit)(v);
        }
        if flags != subflags {
            let id = unsafe { (b.groups.vehicle_id)(v) };
            crate::command::nested(
                b.commands,
                b.shared,
                Cost(temporary, b.cost),
                0,
                DoCommandFlag::EXECUTE,
                |nested_flags| unsafe {
                    super::sell(
                        c,
                        temporary,
                        nested_flags,
                        id,
                        false,
                        false,
                        c.strings.invalid_client,
                    );
                },
            );
        }
    }
    if flags != subflags {
        crate::witness::hit(crate::witness::VT_RESTORE);
        unsafe { (s.random.restore)(seeds) };
    }
    result
}

/// # Safety
/// Same synchronous stack-cost and native-shell contract as `build`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_vehicle_build(
    s: &'static Services,
    out: *mut CommandCost,
    temporary: *mut CommandCost,
    refit_temporary: *mut CommandCost,
    flags: u16,
    tile: u32,
    engine: u16,
    use_free: bool,
    cargo: u8,
    client: u32,
) -> Result {
    unsafe {
        build(
            s,
            out,
            temporary,
            refit_temporary,
            flags,
            tile,
            engine,
            use_free,
            cargo,
            client,
        )
    }
}
