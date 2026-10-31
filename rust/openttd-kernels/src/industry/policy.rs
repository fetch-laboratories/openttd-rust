/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry servicing, news policy and specification arithmetic. Native news operations
//! retain formatting and publication. Shells, orders and specs remain live under the
//! game-state mutex; every owner borrow ends before a resolver or presentation call.
use super::{Builder, Industry, Pair, Services, produced, produced_count};
use crate::industry_type::{CUT_TREES, LIFE_EXTRACTIVE, LIFE_ORGANIC, LIFE_PROCESSING};
use crate::order_type::{OT_GOTO_STATION, OUFB_TRANSFER, OUFB_UNLOAD};
use crate::vehicle_type::{VEH_AIRCRAFT, VEH_ROAD, VEH_SHIP, VEH_TRAIN};
use std::ffi::c_void;

/// Native industry-news servicing category, independent of the presentation enum.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Servicing {
    Nobody,
    Other,
    Company,
}
fn cargo_service(
    cargo: u8,
    shell: *mut c_void,
    owner: *mut Industry,
    s: &Services,
    accepts: &mut bool,
    produces: &mut bool,
) {
    if !super::valid(cargo) {
        return;
    }
    if unsafe { super::get_cargo_accepted(owner, cargo).is_some() }
        && !unsafe { super::resolver::refuses(shell, owner, cargo, &*s.resolver) }
    {
        *accepts = true;
    }
    if unsafe { super::get_cargo_produced(owner, cargo).is_some() } {
        *produces = true;
    }
}
pub(super) fn servicing(shell: *mut c_void, owner: *mut Industry, s: &Services) -> Servicing {
    if unsafe { (*super::stations_near(owner)).entries().is_empty() } {
        return Servicing::Nobody;
    }
    let shared = s.shared();
    let commands = unsafe { &*(shared.station_commands)() };
    let vehicles = unsafe { &*commands.vehicle };
    let grf = unsafe { &*(vehicles.newgrf)() };
    let orders = crate::orders::VisitOrders::new(unsafe { &*(vehicles.orders)() });
    let resolver = unsafe { &*s.resolver };
    let subsidies = unsafe { &*(resolver.lifecycle().subsidies)() };
    let catchment = unsafe { &*(shared.station_catchment)() };
    let stations = unsafe { &*catchment.station };
    let mut result = Servicing::Nobody;
    let mut from = 0;
    while let Some(v) = (shared.vehicle_next)(from) {
        from = unsafe { (grf.id)(v.0.as_ptr()) }.wrapping_add(1);
        if (vehicles.owner)(v) != (subsidies.local_company)()
            && !matches!(result, Servicing::Nobody)
        {
            continue;
        }
        let mut accepts = false;
        let mut produces = false;
        match (vehicles.vehicle_type)(v) {
            VEH_TRAIN if crate::vehicle_base::is_front_engine(true, (vehicles.subtype)(v)) => {
                let mut part = Some(v);
                while let Some(u) = part {
                    cargo_service(
                        unsafe { (grf.cargo_type)(u.0.as_ptr()) },
                        shell,
                        owner,
                        s,
                        &mut accepts,
                        &mut produces,
                    );
                    part = (vehicles.next)(u);
                }
            }
            VEH_ROAD | VEH_SHIP | VEH_AIRCRAFT => cargo_service(
                unsafe { (grf.cargo_type)(v.0.as_ptr()) },
                shell,
                owner,
                s,
                &mut accepts,
                &mut produces,
            ),
            _ => continue,
        }
        if !accepts && !produces {
            continue;
        }
        let mut index = 0;
        while let Some(order) = orders.get(v, index) {
            if order.kind() == OT_GOTO_STATION && order.unload() != OUFB_TRANSFER {
                let station = (stations.get)(order.destination);
                crate::movement::native_assert(shared, !station.is_null());
                if order.unload() == OUFB_UNLOAD && !accepts {
                    break;
                }
                if unsafe {
                    (*super::stations_near(owner))
                        .entries()
                        .iter()
                        .any(|entry| entry.station == station)
                } {
                    if (vehicles.owner)(v) == (subsidies.local_company)() {
                        return Servicing::Company;
                    }
                    result = Servicing::Other;
                }
            }
            index = index.wrapping_add(1);
        }
    }
    result
}
pub(crate) fn random_industry(s: &Services) -> *mut c_void {
    let count = unsafe { (s.num_items)() };
    if count == 0 {
        return std::ptr::null_mut();
    }
    let mut num = s.shared().random_range(u32::from(count as u16));
    let mut from = 0;
    loop {
        let entry = unsafe { (s.next_tick)(from) };
        if num == 0 {
            return entry.industry;
        }
        from = u32::from(entry.id) + 1;
        num -= 1;
    }
}
pub(super) fn total(builder: *const Builder) -> u32 {
    unsafe {
        (*builder).registry.iter().fold(0u32, |total, kinds| {
            total.wrapping_add(u32::from(kinds.len() as u16))
        })
    }
}
pub(crate) fn forest(tile: u32, s: &Services) -> bool {
    let shared = s.shared();
    let world = crate::map_func::World::new(shared);
    if world.types.read(tile) != crate::tile_type::MP_INDUSTRY {
        return false;
    }
    let catchment = unsafe { &*(shared.station_catchment)() };
    let shell = crate::industry_map::get_industry_by_tile(
        &world.types,
        tile,
        catchment.industry_by_id,
        crate::movement::table(shared).native_assert,
    );
    let owner = (catchment.industry_owner)(shell);
    let resolver = unsafe { &*s.resolver };
    let spec = resolver.spec(unsafe { (*owner).metadata.kind });
    if unsafe { ((*resolver.generation).spec_life)(spec) } & LIFE_ORGANIC == 0 {
        return false;
    }
    (0..produced_count(owner)).any(|slot| {
        let cargo = unsafe { (*produced(owner, slot)).cargo };
        super::valid(cargo)
            && crate::cargo_registry::cargo_label(cargo) == crate::cargo_type::CT_WOOD
    })
}
/// # Safety
/// Live world map, pool and static native table under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_forest(tile: u32, s: *const Services) -> bool {
    forest(tile, unsafe { &*s })
}
pub(super) fn advertise(shell: *mut c_void, s: &Services) {
    unsafe {
        (s.opening_news)(shell);
        (s.opening_events)(shell);
    }
}
pub(super) const fn spec_raw(life: u8) -> bool {
    life & (LIFE_EXTRACTIVE | LIFE_ORGANIC) != 0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_spec_raw(life: u8) -> bool {
    spec_raw(life)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_spec_processing(life: u8, behaviour: u32) -> bool {
    life & LIFE_PROCESSING != 0 && behaviour & CUT_TREES == 0
}
pub(super) const fn original_economy(callbacks: u16, economy: u8) -> bool {
    economy == 0 || callbacks & super::construction::ORIGINAL_CALLBACKS != 0
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_original_economy(callbacks: u16, economy: u8) -> bool {
    original_economy(callbacks, economy)
}
pub(super) fn construction_cost(multiplier: u8, life: u8, raw_setting: u8) -> i64 {
    let price = if raw_setting == 1 && spec_raw(life) {
        crate::economy_type::PR_BUILD_INDUSTRY_RAW
    } else {
        crate::economy_type::PR_BUILD_INDUSTRY
    };
    crate::company::price(price).saturating_mul(i64::from(multiplier)) >> 8
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_construction_cost(
    multiplier: u8,
    life: u8,
    raw_setting: u8,
) -> i64 {
    construction_cost(multiplier, life, raw_setting)
}
pub(super) fn removal_cost(multiplier: u32) -> i64 {
    // OverflowSafeInt::operator*= takes int, including the unsigned multiplier conversion.
    crate::company::price(crate::economy_type::PR_CLEAR_INDUSTRY)
        .saturating_mul(i64::from(multiplier as i32))
        >> 8
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_removal_cost(multiplier: u32) -> i64 {
    removal_cost(multiplier)
}
/// # Safety
/// Nonempty readable native span of layout-checked industry history pairs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_sum_history(
    history: *const Pair,
    count: usize,
) -> Pair {
    sum_history(unsafe { std::slice::from_raw_parts(history, count) })
}
pub(super) fn sum_history(history: &[Pair]) -> Pair {
    let count = history.len();
    let a = history
        .iter()
        .fold(0u32, |sum, pair| sum.wrapping_add(u32::from(pair.first)));
    let b = history
        .iter()
        .fold(0u32, |sum, pair| sum.wrapping_add(u32::from(pair.second)));
    Pair {
        first: (a as usize / count).min(65535) as u16,
        second: (b as usize / count).min(65535) as u16,
    }
}
