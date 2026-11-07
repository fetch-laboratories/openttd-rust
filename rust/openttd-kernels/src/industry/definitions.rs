/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry definition allocation, publication and reset over the Rust spec storage. All
//! calls run under the game-state mutex. No reference into spec or manager storage survives
//! a native leaf or a reentrant owner; specs move between storages by value.
use crate::debug::grf_msg;
use crate::industrytype::{self, EMPTY_INDUSTRY, EMPTY_TILE, IndustrySpec, IndustryTileSpec};
pub(crate) mod properties;
use crate::industry_type::{
    INDUSTRY_ORIGINAL_NUM_INPUTS, INDUSTRY_ORIGINAL_NUM_OUTPUTS, IndustryType,
};
use crate::newgrf::CargoLabel;
use crate::newgrf_commons::SubstituteSpecIdentity as Identity;
use crate::override_manager::{self, Mapping};
use std::ffi::c_void;
/// Native inputs: the original tables and the string ids Rust cannot name.
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub origin: *const IndustrySpec,
    pub origin_tiles: *const IndustryTileSpec,
    pub invalid_name: u32,
    /// `AddStringForMapping` into a `StringID` field of a record.
    pub map_string: unsafe extern "C" fn(u16, *mut u32),
}
impl Services {
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: `GetIndustryDefinitionServices` points at the process-lifetime shared table.
        unsafe { &*self.shared }
    }
}
pub unsafe fn allocate(
    manager: *mut override_manager::Manager,
    local: u16,
    grfid: u32,
    substitute: u16,
) -> u16 {
    let (offset, maximum, invalid) =
        unsafe { ((*manager).offset, (*manager).maximum, (*manager).invalid) };
    for id in 0..maximum {
        if id < offset && unsafe { (&(*manager).entities)[usize::from(id)] } != invalid {
            continue;
        }
        // The original narrows the slot to IndustryType for GetIndustrySpec.
        let spec = industrytype::industry(u16::from(id as u8));
        let (enabled, has_file) = unsafe { ((*spec).enabled, (*spec).grf_prop.has_grf_file()) };
        if !enabled && !has_file {
            let map = unsafe { &mut (&mut (*manager).mappings)[usize::from(id)] };
            if map.entity_id == 0 && map.grfid == 0 {
                *map = Mapping {
                    grfid,
                    entity_id: local,
                    substitute_id: substitute,
                };
                return id;
            }
        }
    }
    invalid
}
pub unsafe fn allocate_tile_for(
    manager: *mut override_manager::Manager,
    local: u16,
    grfid: u32,
    substitute: u16,
) -> u16 {
    let existing = unsafe { override_manager::get(manager, local, grfid) };
    // IndustryTileOverrideManager::CheckValidNewID excludes the water-check marker.
    unsafe {
        crate::override_manager::openttd_rust_override_allocate(
            manager,
            local,
            grfid,
            substitute,
            std::ptr::null_mut(),
            existing,
            tile_valid,
        )
    }
}
pub unsafe fn allocate_tile(local: u16, grfid: u32, substitute: u16) -> u16 {
    unsafe { allocate_tile_for(override_manager::tile(), local, grfid, substitute) }
}

unsafe extern "C" fn tile_valid(_: *mut c_void, id: u16) -> u8 {
    u8::from(crate::newgrf_commons::industry_tile_check_valid_new_id(id))
}
/// `IndustryOverrideManager::SetEntitySpec`, moving the per-file spec into its slot.
pub unsafe fn publish(manager: *mut override_manager::Manager, spec: *mut IndustrySpec) {
    let props = unsafe { (*spec).grf_prop };
    let invalid = unsafe { (*manager).invalid };
    // The original narrows GetID and AddEntityID to IndustryType before testing it.
    let mut id = IndustryType(unsafe {
        override_manager::industry_id_for(manager, props.local_id, props.grfid)
    } as u8);
    if u16::from(id.0) == invalid {
        id = IndustryType(
            unsafe { allocate(manager, props.local_id, props.grfid, props.subst_id) } as u8,
        );
        unsafe { (*spec).grf_prop.override_id = invalid };
    }
    if u16::from(id.0) == invalid {
        grf_msg!(
            1,
            "Industry.SetEntitySpec: Too many industries allocated. Ignoring."
        );
        return;
    }
    let target = industrytype::industry(u16::from(id.0));
    unsafe {
        *target = std::mem::replace(&mut *spec, EMPTY_INDUSTRY);
        (*target).enabled = true;
    }
}
/// `IndustryTileOverrideManager::SetEntitySpec`.
pub unsafe fn publish_tile(manager: *mut override_manager::Manager, spec: *mut IndustryTileSpec) {
    let props = unsafe { (*spec).grf_prop };
    let (offset, invalid) = unsafe { ((*manager).offset, (*manager).invalid) };
    let id = unsafe { allocate_tile_for(manager, props.local_id, props.grfid, props.subst_id) };
    if id == invalid {
        grf_msg!(
            1,
            "IndustryTile.SetEntitySpec: Too many industry tiles allocated. Ignoring."
        );
        return;
    }
    unsafe {
        *industrytype::tile(id) = std::mem::replace(&mut *spec, EMPTY_TILE);
    }
    for i in 0..offset {
        let matched = unsafe {
            (&(*manager).entities)[usize::from(i)] == props.local_id
                && (&(*manager).grfids)[usize::from(i)] == props.grfid
        };
        if !matched {
            continue;
        }
        let overridden = industrytype::tile(i);
        unsafe {
            (*overridden).grf_prop.override_id = id;
            (*overridden).enabled = false;
            (&mut (*manager).entities)[usize::from(i)] = invalid;
            (&mut (*manager).grfids)[usize::from(i)] = 0;
        }
    }
}
fn translate(label: CargoLabel) -> u8 {
    use crate::cargo_registry::openttd_rust_cargo_by_label;
    openttd_rust_cargo_by_label(crate::newgrf::get_active_cargo_label(label, |label| {
        openttd_rust_cargo_by_label(label)
    }))
}
/// Replace each invalid cargo slot with its default label's translation.
fn translate_cargo(cargo: &mut [u8], labels: &[CargoLabel]) {
    for (cargo, label) in cargo.iter_mut().zip(labels) {
        if !crate::cargo_type::is_valid_cargo_type(*cargo) {
            *cargo = translate(*label);
        }
    }
}

/// `FinaliseIndustriesArray`.
pub unsafe fn finalise(s: &Services) {
    let mut file_index = 0;
    loop {
        let file = (s.shared().grf_file_at_or_null)(file_index);
        if file.is_null() {
            break;
        }
        let count = unsafe { industrytype::file_specs(file) }.industries.len();
        for i in 0..count {
            let Some(spec) = (unsafe { industrytype::file_specs(file) }.industries[i]
                .as_deref_mut())
            .map(std::ptr::from_mut) else {
                continue;
            };
            if unsafe { (*spec).enabled } {
                unsafe { publish(override_manager::industry(), spec) };
            }
        }
        let count = unsafe { industrytype::file_specs(file) }.tiles.len();
        for i in 0..count {
            if let Some(spec) = (unsafe { industrytype::file_specs(file) }.tiles[i].as_deref_mut())
                .map(std::ptr::from_mut)
            {
                unsafe { publish_tile(override_manager::tile(), spec) };
            }
        }
        industrytype::clear_file(file);
        file_index += 1;
    }
    for id in 0..u16::from(crate::industry_type::NUM_INDUSTRYTYPES) {
        let spec = industrytype::industry(id);
        let (enabled, has_file, grfid) = unsafe {
            (
                (*spec).enabled,
                (*spec).grf_prop.has_grf_file(),
                (*spec).grf_prop.grfid,
            )
        };
        if enabled && has_file {
            for i in 0..3 {
                let kind = unsafe { (*spec).conflicting[i] };
                let mapped = crate::industry::resolver::map_type(kind, grfid);
                unsafe { (*spec).conflicting[i] = mapped };
            }
        }
        if !enabled {
            unsafe { (*spec).name = s.invalid_name };
        }
        let mut produced = unsafe { (*spec).produced_cargo };
        let labels = unsafe { (*spec).produced_cargo_label };
        translate_cargo(&mut produced[..INDUSTRY_ORIGINAL_NUM_OUTPUTS], &labels);
        unsafe { (*spec).produced_cargo = produced };
        let mut accepted = unsafe { (*spec).accepts_cargo };
        let labels = unsafe { (*spec).accepts_cargo_label };
        translate_cargo(&mut accepted[..INDUSTRY_ORIGINAL_NUM_INPUTS], &labels);
        unsafe { (*spec).accepts_cargo = accepted };
    }
    for gfx in 0..crate::industry_type::NUM_INDUSTRYTILES {
        let spec = industrytype::tile(gfx);
        let mut accepted = unsafe { (*spec).accepts_cargo };
        let labels = unsafe { (*spec).accepts_cargo_label };
        translate_cargo(&mut accepted[..INDUSTRY_ORIGINAL_NUM_INPUTS], &labels);
        unsafe { (*spec).accepts_cargo = accepted };
    }
}

/// `ResetIndustries` with the override managers.
pub(crate) unsafe fn reset(climate: u8, s: &Services) {
    unsafe {
        industrytype::reset(s.shared, climate, s.origin, s.origin_tiles);
        override_manager::openttd_rust_override_reset(override_manager::tile());
        override_manager::openttd_rust_override_reset(override_manager::industry());
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDSubstituteSpecIdentity" ["industry"] => Identity { grfid, local, substitute };
    "OpenTTDIndustryDefinitionServices" ["industry"] => Services {
        shared, origin, origin_tiles, invalid_name, map_string
    };
}
