/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Initial industry probability, forced placement and weighted traversal, with scratch arrays
//! on the invocation stack. Only progress cancellation returns early, to a native stack
//! exception frame.
use super::{Builder, Services, TYPES, place};
use crate::industrytype;

pub use crate::industrytype::IndustrySpec as SpecHandle;
#[repr(C)]
pub struct GenerationContext {
    _private: [u8; 0],
}
#[repr(C)]
pub struct GenerationInfo {
    pub appear: u8,
    pub enabled: bool,
    pub layouts: bool,
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
fn probability(
    kind: u8,
    water: Option<bool>,
    forced: &mut bool,
    s: &Services,
    g: &GenerationServices,
) -> u32 {
    let spec = industrytype::industry(u16::from(kind));
    let behaviour = water.map(|_| unsafe { (*spec).behaviour });
    if water.is_some_and(|water| (behaviour.unwrap_or(0) & (1 << 2) != 0) != water) {
        return 0; // Source deliberately leaves the force output untouched here.
    }
    let (enabled, layouts) = unsafe { ((*spec).enabled, !(*spec).layouts.is_empty()) };
    let climate = unsafe { s.shared().settings().climate.read() };
    let appear = unsafe { (*spec).appear_creation[usize::from(climate)] };
    if !enabled || !layouts || (!(g.environment.editor)() && unsafe { (g.density)() } == 0) {
        *forced = false;
        return 0;
    }
    let chance = unsafe {
        super::resolver::openttd_rust_industry_probability(kind, 0, u32::from(appear), s.resolver)
    };
    if chance == 0 {
        *forced = false;
        return 0;
    }
    let chance = chance.wrapping_mul(16);
    // CHECK_REFINERY and CHECK_OIL_RIG place industries at the map edge.
    let chance = if matches!(unsafe { (*spec).check_proc }, 2 | 8) {
        unsafe { (g.scale_by_size_1d)(chance) }
    } else {
        unsafe { (s.scale_by_map_size)(chance) }
    };
    *forced = chance > 0
        && behaviour.unwrap_or_else(|| unsafe { (*spec).behaviour }) & (1 << 16) == 0
        && !(g.environment.editor)();
    chance
}
struct Probabilities {
    probs: [u32; TYPES],
    force_one: [bool; TYPES],
    total: u64,
    num_forced: u32,
}
fn probabilities(water: bool, s: &Services, g: &GenerationServices) -> Probabilities {
    let mut p = Probabilities {
        probs: [0; TYPES],
        force_one: [false; TYPES],
        total: 0,
        num_forced: 0,
    };
    for kind in 0..TYPES {
        p.probs[kind] = probability(kind as u8, Some(water), &mut p.force_one[kind], s, g);
        p.total = p.total.wrapping_add(u64::from(p.probs[kind]));
        if p.force_one[kind] {
            p.num_forced = p.num_forced.wrapping_add(1);
        }
    }
    p
}
fn wanted(s: &Services, g: &GenerationServices) -> u32 {
    const AMOUNTS: [u16; 7] = [0, 0, 10, 25, 55, 80, 0];
    let density = if (g.environment.editor)() {
        2
    } else {
        unsafe { (g.density)() }
    };
    if density == 6 {
        unsafe { (g.custom_number)() }.min(64000)
    } else {
        unsafe { (s.scale_by_map_size)(u32::from(AMOUNTS[density as usize])) }.min(64000)
    }
}
fn initial(
    kind: u8,
    water: bool,
    hard: bool,
    context: *mut GenerationContext,
    s: &Services,
    g: &GenerationServices,
) -> bool {
    let company = unsafe { (s.company_none)() };
    if !unsafe { (g.increase_progress)(context, water) } {
        // Native Backup<CompanyID> would restore during cancellation unwinding.
        unsafe { (s.restore_company)(company) };
        return false;
    }
    place(kind, 0, hard, s);
    unsafe { (s.restore_company)(company) };
    true
}
/// # Safety
/// Live builder and call-scoped native cancellation frame. Tables remain valid;
/// cancellation callbacks catch only the named ordinary generation abort signal.
// Keep the wanted-number services inside the source nonzero-total branch.
#[allow(clippy::manual_checked_ops)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_generate_abi(
    builder: *mut Builder,
    context: *mut GenerationContext,
    s: *const Services,
    g: *const GenerationServices,
) -> u8 {
    let s = unsafe { &*s };
    let g = unsafe { &*g };
    if !(g.environment.editor)() && unsafe { (g.density)() } == 0 {
        return u8::from(true);
    }
    let land = probabilities(false, s, g);
    let water = probabilities(true, s, g);
    for (on_water, p) in [(false, &land), (true, &water)] {
        let mut amount = 0;
        let total = land.total.wrapping_add(water.total);
        if total > 0 {
            amount = (p.total.wrapping_mul(u64::from(wanted(s, g))) / total) as u32;
        }
        if !on_water && unsafe { (g.density)() } != 6 {
            amount = unsafe { (g.scale_by_land_proportion)(amount) };
        }
        if p.total == 0 || amount < p.num_forced {
            amount = p.num_forced;
        }
        if !unsafe { (g.set_progress)(context, on_water, amount) } {
            return u8::from(false);
        }
        for kind in 0..TYPES {
            if p.force_one[kind] {
                amount = amount.wrapping_sub(1);
                if !initial(kind as u8, on_water, true, context, s, g) {
                    return u8::from(false);
                }
            }
        }
        for _ in 0..amount {
            let mut r = s.shared().random_range(p.total as u32);
            let mut kind = 0;
            while r >= p.probs[kind] {
                r = r.wrapping_sub(p.probs[kind]);
                kind += 1;
            }
            if !initial(kind as u8, on_water, false, context, s, g) {
                return u8::from(false);
            }
        }
    }
    unsafe { super::openttd_rust_industry_build_reset(builder, s) };
    u8::from(true)
}
/// # Safety
/// Tables and returned immutable specs remain live for this invocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_check(
    s: *const Services,
    g: *const GenerationServices,
) {
    let s = unsafe { &*s };
    let g = unsafe { &*g };
    let mut count = 0;
    for kind in 0..TYPES {
        if unsafe { (s.type_count)(kind as u8) } > 0 {
            continue;
        }
        let mut forced = false;
        let chance = probability(kind as u8, None, &mut forced, s, g);
        if chance == 0 || !forced {
            continue;
        }
        unsafe { (g.missing_warning)(industrytype::industry(u16::from(kind as u8))) };
        count += 1;
        if count >= 3 {
            break;
        }
    }
}
