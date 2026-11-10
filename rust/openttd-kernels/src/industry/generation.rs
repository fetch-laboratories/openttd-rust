/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Initial industry probability, forced placement and weighted traversal, with scratch arrays
//! on the invocation stack. Only progress cancellation returns early, to a native stack
//! exception frame.
use super::{Builder, Services, TYPES, place_industry};
use crate::industry_type::{IndustryID, IndustryType};
use crate::industrytype::{self, CheckProc, IndustryBehaviour, IndustryBehaviours};
use crate::newgrf_industries::IndustryAvailabilityCallType;
use crate::settings_type::IndustryDensity;

pub use crate::industrytype::IndustrySpec as SpecHandle;
#[repr(C)]
pub struct GenerationContext {
    _private: [u8; 0],
}
#[repr(C)]
pub struct GenerationServices {
    pub environment: crate::services::EnvironmentServices,
    pub density: unsafe extern "C" fn() -> u8,
    pub custom_number: unsafe extern "C" fn() -> u32,
    pub scale_by_size_1d: unsafe extern "C" fn(u32) -> u32,
    pub scale_by_land_proportion: unsafe extern "C" fn(u32) -> u32,
    pub set_progress: unsafe extern "C" fn(*mut GenerationContext, bool, u32) -> bool,
    pub increase_progress: unsafe extern "C" fn(*mut GenerationContext, bool) -> bool,
    pub missing_warning: unsafe extern "C" fn(*const SpecHandle),
}
/// Original `GetScaledIndustryGenerationProbability`: compute the appearance probability for
/// an industry during map creation. `water` selects land-based or water-based industry
/// types, or both when `None`; `force_at_least_one` returns whether at least one instance
/// should be forced on map creation.
fn get_scaled_industry_generation_probability(
    it: IndustryType,
    water: Option<bool>,
    force_at_least_one: &mut bool,
    s: &Services,
    g: &GenerationServices,
) -> u32 {
    let ind_spc = industrytype::get_industry_spec(it);
    let behaviour = water.map(|_| IndustryBehaviours(unsafe { (*ind_spc).behaviour }));
    if water.is_some_and(|water| {
        (behaviour
            .unwrap_or(IndustryBehaviours::EMPTY)
            .test(IndustryBehaviour::BuiltOnWater))
            != water
    }) {
        return 0; // The original leaves `force_at_least_one` untouched here.
    }
    let (enabled, layouts) = unsafe { ((*ind_spc).enabled, !(*ind_spc).layouts.is_empty()) };
    let landscape = s.landscape_type();
    let chance = unsafe { (*ind_spc).appear_creation[usize::from(landscape.0)] };
    if !enabled
        || !layouts
        || (!(g.environment.editor)()
            && IndustryDensity(unsafe { (g.density)() }) == IndustryDensity::ID_FUND_ONLY)
    {
        *force_at_least_one = false;
        return 0;
    }
    let chance = unsafe {
        super::resolver::get_industry_probability_callback(
            it,
            IndustryAvailabilityCallType::IACT_MAPGENERATION,
            u32::from(chance),
            &*s.resolver,
        )
    };
    if chance == 0 {
        *force_at_least_one = false;
        return 0;
    }
    let chance = chance.wrapping_mul(16); // to increase precision
    // We want industries appearing at coast to appear less often on bigger maps, as length of coast increases slower than map area.
    // For simplicity we scale in both cases, though scaling the probabilities of all industries has no effect.
    let chance = if matches!(
        CheckProc(unsafe { (*ind_spc).check_proc }),
        CheckProc::CHECK_REFINERY | CheckProc::CHECK_OIL_RIG
    ) {
        unsafe { (g.scale_by_size_1d)(chance) }
    } else {
        s.scale_by_map_size(chance)
    };
    *force_at_least_one = chance > 0
        && !behaviour
            .unwrap_or_else(|| IndustryBehaviours(unsafe { (*ind_spc).behaviour }))
            .test(IndustryBehaviour::NoBuildMapCreation)
        && !(g.environment.editor)();
    chance
}
/// Original `IndustryGenerationProbabilities`.
struct IndustryGenerationProbabilities {
    probs: [u32; TYPES],
    force_one: [bool; TYPES],
    total: u64,
    num_forced: u32,
}
/// Original `GetScaledProbabilities`: get scaled industry generation probabilities, of land
/// or water industries.
fn get_scaled_probabilities(
    water: bool,
    s: &Services,
    g: &GenerationServices,
) -> IndustryGenerationProbabilities {
    let mut p = IndustryGenerationProbabilities {
        probs: [0; TYPES],
        force_one: [false; TYPES],
        total: 0,
        num_forced: 0,
    };
    for it in 0..IndustryType::NUM_INDUSTRYTYPES.0 {
        let index = usize::from(it);
        p.probs[index] = get_scaled_industry_generation_probability(
            IndustryType(it),
            Some(water),
            &mut p.force_one[index],
            s,
            g,
        );
        p.total = p.total.wrapping_add(u64::from(p.probs[index]));
        if p.force_one[index] {
            p.num_forced = p.num_forced.wrapping_add(1);
        }
    }
    p
}
/// Original `GetNumberOfIndustries`: get wanted number of industries on the map.
fn get_number_of_industries(s: &Services, g: &GenerationServices) -> u32 {
    // Number of industries on a 256x256 map.
    const NUMOF_INDUSTRY_TABLE: [u16; IndustryDensity::ID_END.0 as usize] = [
        0,  // none
        0,  // minimal
        10, // very low
        25, // low
        55, // normal
        80, // high
        0,  // custom
    ];
    let difficulty = if (g.environment.editor)() {
        IndustryDensity::ID_VERY_LOW
    } else {
        IndustryDensity(unsafe { (g.density)() })
    };
    if difficulty == IndustryDensity::ID_CUSTOM {
        unsafe { (g.custom_number)() }.min(u32::from(IndustryID::END.0))
    } else {
        s.scale_by_map_size(u32::from(NUMOF_INDUSTRY_TABLE[usize::from(difficulty.0)]))
            .min(u32::from(IndustryID::END.0))
    }
}
/// Original `PlaceInitialIndustry`: try to build a industry on the map. `try_hard` tries very
/// hard to find a place (used to place at least one industry per type).
fn place_initial_industry(
    r#type: IndustryType,
    water: bool,
    try_hard: bool,
    context: *mut GenerationContext,
    s: &Services,
    g: &GenerationServices,
) -> bool {
    let cur_company = unsafe { (s.company_none)() };
    if !unsafe { (g.increase_progress)(context, water) } {
        // Native Backup<CompanyID> would restore during cancellation unwinding.
        unsafe { (s.restore_company)(cur_company) };
        return false;
    }
    place_industry(
        r#type,
        IndustryAvailabilityCallType::IACT_MAPGENERATION,
        try_hard,
        s,
    );
    unsafe { (s.restore_company)(cur_company) };
    true
}
/// Original `GenerateIndustries`: this function will create random industries during game
/// creation. It will scale the amount of industries by mapsize and difficulty level.
///
/// # Safety
/// Live builder and call-scoped native cancellation frame. Tables remain valid;
/// cancellation callbacks catch only the named ordinary generation abort signal.
// Keep the wanted-number services inside the source nonzero-total branch.
#[allow(clippy::manual_checked_ops)]
#[unsafe(no_mangle)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "GenerateIndustries keeps the original's implicit C++ integer conversions"
)]
pub unsafe extern "C" fn openttd_rust_industry_generate_abi(
    builder: *mut Builder,
    context: *mut GenerationContext,
    s: *const Services,
    g: *const GenerationServices,
) -> u8 {
    let s = unsafe { &*s };
    let g = unsafe { &*g };
    if !(g.environment.editor)()
        && IndustryDensity(unsafe { (g.density)() }) == IndustryDensity::ID_FUND_ONLY
    {
        return u8::from(true); // No industries in the game.
    }
    // Get the probabilities for all industries. This is done first as we need the total of
    // both land and water for scaling later.
    let lprob = get_scaled_probabilities(false, s, g);
    let wprob = get_scaled_probabilities(true, s, g);
    // Run generation twice, for land and water industries in turn.
    for (water, p) in [(false, &lprob), (true, &wprob)] {
        // Total number of industries scaled by land/water proportion.
        let mut total_amount = 0;
        let total = lprob.total.wrapping_add(wprob.total);
        if total > 0 {
            total_amount = (p
                .total
                .wrapping_mul(u64::from(get_number_of_industries(s, g)))
                / total) as u32;
        }
        // Scale land-based industries to the land proportion, unless the player has set a custom industry count.
        if !water && IndustryDensity(unsafe { (g.density)() }) != IndustryDensity::ID_CUSTOM {
            total_amount = unsafe { (g.scale_by_land_proportion)(total_amount) };
        }
        // Ensure that forced industries are generated even if the scaled amounts are too low.
        if p.total == 0 || total_amount < p.num_forced {
            // Only place the forced ones
            total_amount = p.num_forced;
        }
        if !unsafe { (g.set_progress)(context, water, total_amount) } {
            return u8::from(false);
        }
        // Try to build one industry per type independent of any probabilities
        for it in 0..IndustryType::NUM_INDUSTRYTYPES.0 {
            if p.force_one[usize::from(it)] {
                total_amount = total_amount.wrapping_sub(1);
                if !place_initial_industry(IndustryType(it), water, true, context, s, g) {
                    return u8::from(false);
                }
            }
        }
        // Add the remaining industries according to their probabilities
        for _ in 0..total_amount {
            let mut r = s.shared().random_range(p.total as u32);
            let mut it = IndustryType(0);
            while r >= p.probs[usize::from(it.0)] {
                r = r.wrapping_sub(p.probs[usize::from(it.0)]);
                it.0 += 1;
            }
            if !place_initial_industry(it, water, false, context, s, g) {
                return u8::from(false);
            }
        }
    }
    super::industry_build_data_reset(builder);
    u8::from(true)
}
/// Original `CheckIndustries`: verify whether the generated industries are complete, and
/// warn the user if not. The builder, tables and returned immutable specs remain live for
/// this invocation.
pub(crate) fn check_industries(
    builder: *const super::Builder,
    s: &Services,
    g: &GenerationServices,
) {
    let mut count = 0;
    for it in 0..IndustryType::NUM_INDUSTRYTYPES.0 {
        if super::get_industry_type_count(builder, IndustryType(it), s) > 0 {
            continue; // Types of existing industries can be skipped.
        }
        let mut force_at_least_one = false;
        let chance = get_scaled_industry_generation_probability(
            IndustryType(it),
            None,
            &mut force_at_least_one,
            s,
            g,
        );
        if chance == 0 || !force_at_least_one {
            continue; // Types that are not available can be skipped.
        }
        unsafe { (g.missing_warning)(industrytype::get_industry_spec(IndustryType(it))) };
        count += 1;
        if count >= 3 {
            break; // Don't swamp the user with errors.
        }
    }
}
/// `CheckIndustries`.
///
/// # Safety
/// The live builder and tables remain live for this invocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_check(
    builder: *mut super::Builder,
    s: *const Services,
    g: *const GenerationServices,
) {
    check_industries(builder, unsafe { &*s }, unsafe { &*g });
}
