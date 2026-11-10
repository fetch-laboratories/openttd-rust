/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry servicing, news policy and specification arithmetic. Native news operations
//! retain formatting and publication. Shells, orders and specs remain live under the
//! game-state mutex; every owner borrow ends before a resolver or presentation call.
use super::{Builder, Industry, Pair, Services, produced, produced_count};
use crate::cargo_type::{CargoLabel, CargoType, is_valid_cargo_type};
use crate::company_type::CompanyID;
use crate::economy_type::{EconomyType, Price};
use crate::industrytype::{
    IndustryBehaviour, IndustryBehaviours, IndustryLifeType, IndustryLifeTypes,
};
use crate::newgrf_callbacks::{IndustryCallbackMask, IndustryCallbackMasks};
use crate::order_type::{OrderType, OrderUnloadType};
use crate::tile_type::{TileIndex, TileType};
use crate::vehicle_type::VehicleType;
use std::ffi::c_void;

/// The result of `WhoCanServiceIndustry`: 0 if nobody can service the industry, 2 if the
/// local company can service the industry, and 1 otherwise (only competitors can service the
/// industry). The native news callbacks take the same ordinals.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Servicing {
    Nobody,
    Other,
    Company,
}
/// Original `CanCargoServiceIndustry`: can the given cargo type be accepted or produced
/// by the industry?
fn can_cargo_service_industry(
    cargo: CargoType,
    shell: *mut c_void,
    ind: *mut Industry,
    s: &Services,
    c_accepts: &mut bool,
    c_produces: &mut bool,
) {
    if !is_valid_cargo_type(cargo.0) {
        return;
    }
    // Check for acceptance of cargo
    if unsafe { super::get_cargo_accepted(ind, cargo).is_some() }
        && !unsafe {
            super::resolver::industry_temporarily_refuses_cargo(shell, ind, cargo, &*s.resolver)
        }
    {
        *c_accepts = true;
    }
    // Check for produced cargo
    if unsafe { super::get_cargo_produced(ind, cargo).is_some() } {
        *c_produces = true;
    }
}
/// Original `WhoCanServiceIndustry`: compute who can service the industry.
///
/// Here, 'can service' means that they have trains and stations close enough
/// to the industry with the right cargo type and the right orders (ie has the
/// technical means).
pub(super) fn servicing(shell: *mut c_void, ind: *mut Industry, s: &Services) -> Servicing {
    if unsafe { (*super::stations_near(ind)).entries().is_empty() } {
        return Servicing::Nobody; // No stations found at all => nobody services
    }
    let shared = s.shared();
    let commands = unsafe { &*(shared.station_commands)() };
    let vehicles = unsafe { &*commands.vehicle };
    let grf = unsafe { &*(vehicles.newgrf)() };
    let orders = crate::orders::VisitOrders::new(unsafe { &*(vehicles.orders)() });
    let resolver = unsafe { &*s.resolver };
    let subsidies = unsafe { &*(resolver.lifecycle().subsidies)() };
    let local_company = || CompanyID((subsidies.local_company)());
    let mut result = Servicing::Nobody;
    let mut from = 0;
    while let Some(v) = (shared.vehicle_next)(from) {
        from = unsafe { (grf.id)(v.0.as_ptr()) }.wrapping_add(1);
        // Is it worthwhile to try this vehicle?
        if CompanyID(v.owner()) != local_company() && !matches!(result, Servicing::Nobody) {
            continue;
        }
        // Check whether it accepts the right kind of cargo
        let mut c_accepts = false;
        let mut c_produces = false;
        match VehicleType(v.vehicle_type()) {
            VehicleType::VEH_TRAIN if crate::vehicle_base::is_front_engine(true, v.subtype()) => {
                let mut part = Some(v);
                while let Some(u) = part {
                    can_cargo_service_industry(
                        CargoType(unsafe { (grf.cargo_type)(u.0.as_ptr()) }),
                        shell,
                        ind,
                        s,
                        &mut c_accepts,
                        &mut c_produces,
                    );
                    part = (u).next();
                }
            }
            VehicleType::VEH_ROAD | VehicleType::VEH_SHIP | VehicleType::VEH_AIRCRAFT => {
                can_cargo_service_industry(
                    CargoType(unsafe { (grf.cargo_type)(v.0.as_ptr()) }),
                    shell,
                    ind,
                    s,
                    &mut c_accepts,
                    &mut c_produces,
                );
            }
            _ => continue,
        }
        if !c_accepts && !c_produces {
            continue; // Wrong cargo
        }
        // Check orders of the vehicle.
        // We cannot check the first of shared orders only, since the first vehicle in such a chain
        // may have a different cargo type.
        let list = orders.list(v);
        if list.is_null() {
            continue;
        }
        for index in 0..orders.stored_count(list) {
            let o = orders.stored_order(list, index);
            if OrderType(o.kind()) == OrderType::OT_GOTO_STATION
                && OrderUnloadType(o.unload()) != OrderUnloadType::Transfer
            {
                // Vehicle visits a station to load or unload
                let st = (subsidies.station)(o.destination)
                    .cast_mut()
                    .cast::<crate::station::BaseStation>();
                crate::movement::native_assert(shared, !st.is_null());
                // Same cargo produced by industry is dropped here => not serviced by vehicle v
                if OrderUnloadType(o.unload()) == OrderUnloadType::Unload && !c_accepts {
                    break;
                }
                if unsafe {
                    (*super::stations_near(ind))
                        .entries()
                        .iter()
                        .any(|entry| entry.station == st)
                } {
                    if CompanyID(v.owner()) == local_company() {
                        return Servicing::Company; // Company services industry
                    }
                    result = Servicing::Other; // Competitor services industry
                }
            }
        }
    }
    result
}
/// The shell of `Industry::GetRandom`, or null when there is no industry.
pub(crate) fn random_industry(s: &Services) -> *mut c_void {
    random_entry(s).industry
}
/// `Industry::GetRandom` with the canonical owner: return a random valid industry.
#[allow(
    clippy::cast_possible_truncation,
    reason = "Industry::GetRandom keeps the original's implicit C++ integer conversions"
)]
pub(crate) fn random_entry(s: &Services) -> super::Entry {
    let count = unsafe { (s.num_items)() };
    if count == 0 {
        return super::Entry {
            industry: std::ptr::null_mut(),
            owner: std::ptr::null_mut(),
            id: 0,
        };
    }
    let mut num = s.shared().random_range(u32::from(count as u16));
    let mut from = 0;
    loop {
        // Make sure we have a valid industry
        let entry = unsafe { (s.next_tick)(from) };
        if num == 0 {
            return entry;
        }
        from = u32::from(entry.id) + 1;
        num -= 1;
    }
}
/// Original `GetCurrentTotalNumberOfIndustries`: get total number of industries existing in
/// the game.
#[allow(
    clippy::cast_possible_truncation,
    reason = "GetCurrentTotalNumberOfIndustries keeps the original's implicit C++ integer conversions"
)]
pub(super) fn total(builder: *const Builder) -> u32 {
    unsafe {
        (*builder).registry.iter().fold(0u32, |total, kinds| {
            total.wrapping_add(u32::from(kinds.len() as u16))
        })
    }
}
/// Original `IsTileForestIndustry`: check whether the tile is a forest.
pub(crate) fn is_tile_forest_industry(tile: TileIndex, s: &Services) -> bool {
    let shared = s.shared();
    let world = crate::map_func::World::new(shared);
    // Check for industry tile
    if TileType(world.types.read(tile.0)) != TileType::MP_INDUSTRY {
        return false;
    }
    let catchment = unsafe { &*(shared.station_catchment)() };
    let shell = crate::industry_map::get_industry_by_tile(
        &world.types,
        tile,
        catchment.industry_by_id,
        crate::movement::table(shared).native_assert,
    );
    let ind = (catchment.industry_owner)(shell);
    let spec = crate::industrytype::get_industry_spec(crate::industry_type::IndustryType(unsafe {
        (*ind).metadata.kind
    }));
    // Check for organic industry (i.e. not processing or extractive)
    if !IndustryLifeTypes(unsafe { (*spec).life_type }).test(IndustryLifeType::Organic) {
        return false;
    }
    // Check for wood production
    (0..produced_count(ind)).any(|slot| {
        let cargo = CargoType(unsafe { (*produced(ind, slot)).cargo });
        is_valid_cargo_type(cargo.0)
            && CargoLabel(crate::cargo_registry::cargo_label(cargo.0)) == CargoLabel::CT_WOOD
    })
}
/// # Safety
/// Live world map, pool and static native table under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_forest_abi(tile: u32, s: *const Services) -> u8 {
    u8::from(is_tile_forest_industry(TileIndex(tile), unsafe { &*s }))
}
/// The news item and script events announcing a new industry, as `AdvertiseIndustryOpening`.
pub(super) fn advertise(shell: *mut c_void, s: &Services) {
    unsafe {
        (s.opening_news)(shell);
        (s.opening_events)(shell);
    }
}
/// `IndustrySpec::IsRawIndustry`: is an industry with the spec a raw industry?
pub(crate) const fn is_raw_industry(life_type: IndustryLifeTypes) -> bool {
    life_type.any(
        IndustryLifeTypes::from_element(IndustryLifeType::Extractive)
            .with(IndustryLifeTypes::from_element(IndustryLifeType::Organic)),
    )
}
/// `IndustrySpec::IsProcessingIndustry`: is an industry with the spec a processing
/// industry?
pub(crate) const fn is_processing_industry(
    life_type: IndustryLifeTypes,
    behaviour: IndustryBehaviours,
) -> bool {
    // Lumber mills are neither raw nor processing
    life_type.test(IndustryLifeType::Processing) && !behaviour.test(IndustryBehaviour::CutTrees)
}
/// `IndustrySpec::UsesOriginalEconomy`: determines whether this industrytype uses
/// standard/newgrf production changes.
pub(crate) const fn uses_original_economy(
    callback_mask: IndustryCallbackMasks,
    economy_type: EconomyType,
) -> bool {
    economy_type.0 == EconomyType::ET_ORIGINAL.0
        || callback_mask.any(
            // production callbacks
            IndustryCallbackMasks::from_element(IndustryCallbackMask::Production256Ticks)
                .with(IndustryCallbackMasks::from_element(
                    IndustryCallbackMask::ProductionCargoArrival,
                ))
                // production change callbacks
                .with(IndustryCallbackMasks::from_element(
                    IndustryCallbackMask::MonthlyProdChange,
                ))
                .with(IndustryCallbackMasks::from_element(
                    IndustryCallbackMask::ProductionChange,
                ))
                .with(IndustryCallbackMasks::from_element(
                    IndustryCallbackMask::ProdChangeBuild,
                )),
        )
}
/// `IndustrySpec::GetConstructionCost`: get the cost for constructing this industry, under
/// the `raw_industry_construction` setting.
pub(crate) fn get_construction_cost(
    cost_multiplier: u8,
    life_type: IndustryLifeTypes,
    raw_industry_construction: u8,
) -> i64 {
    // Building raw industries like secondary uses different price base
    let price = if raw_industry_construction == 1 && is_raw_industry(life_type) {
        Price::PR_BUILD_INDUSTRY_RAW
    } else {
        Price::PR_BUILD_INDUSTRY
    };
    crate::company::price(usize::from(price.0)).saturating_mul(i64::from(cost_multiplier)) >> 8
}
/// `IndustrySpec::GetRemovalCost`: get the cost for removing this industry.
/// Take note that the cost will always be zero for non-grf industries.
/// Only if the grf author did specified a cost will it be applicable.
#[allow(
    clippy::cast_possible_wrap,
    reason = "IndustrySpec::GetRemovalCost keeps the original's implicit C++ integer conversions"
)]
pub(crate) fn removal_cost(removal_cost_multiplier: u32) -> i64 {
    // OverflowSafeInt::operator*= takes int, including the unsigned multiplier conversion.
    crate::company::price(usize::from(Price::PR_CLEAR_INDUSTRY.0))
        .saturating_mul(i64::from(removal_cost_multiplier as i32))
        >> 8
}
/// # Safety
/// Nonempty readable native span of layout-checked industry history pairs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_sum_history(
    history: *const Pair,
    count: usize,
) -> Pair {
    // SAFETY: The caller supplies the original nonempty initialized canonical pair span.
    sum_history(unsafe { std::slice::from_raw_parts(history, count) })
}
/// The `SumHistory` specialisations for `Industry::ProducedHistory` and
/// `Industry::AcceptedHistory`: average each field, clamped to `uint16_t`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "SumHistory keeps the original's implicit C++ integer conversions"
)]
pub(crate) fn sum_history(history: &[Pair]) -> Pair {
    let count = history.len();
    let a = history
        .iter()
        .fold(0u32, |sum, pair| sum.wrapping_add(u32::from(pair.first)));
    let b = history
        .iter()
        .fold(0u32, |sum, pair| sum.wrapping_add(u32::from(pair.second)));
    Pair {
        first: (a as usize / count).min(usize::from(u16::MAX)) as u16,
        second: (b as usize / count).min(usize::from(u16::MAX)) as u16,
    }
}
