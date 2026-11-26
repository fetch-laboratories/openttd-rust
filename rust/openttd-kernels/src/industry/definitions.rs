/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry definition allocation, publication and reset over the Rust spec storage. All
//! calls run under the game-state mutex. No reference into spec or manager storage survives
//! a native leaf or a reentrant owner; specs move between storages by value.
use crate::debug::grf_msg;
use crate::industrytype::{self, EMPTY_INDUSTRY, EMPTY_TILE, IndustrySpec, IndustryTileSpec};
pub(crate) mod properties;
use crate::cargo_type::{CargoType, is_valid_cargo_type};
use crate::industry_type::{
    INDUSTRY_ORIGINAL_NUM_INPUTS, INDUSTRY_ORIGINAL_NUM_OUTPUTS, IndustryGfx, IndustryType,
};
use crate::newgrf::{CargoLabelChoice, GRFFile};
use crate::newgrf_commons::SubstituteSpecIdentity as Identity;
use crate::override_manager::{self, OverrideManagerBase};
use crate::table::strings::STR_NEWGRF_INVALID_INDUSTRYTYPE;
/// Native inputs: the original tables.
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub origin: *const IndustrySpec,
    pub origin_tiles: *const IndustryTileSpec,
    /// `AddStringForMapping` into a `StringID` field of a record.
    pub map_string: unsafe extern "C" fn(u16, *mut u32),
}
impl Services {
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: `GetIndustryDefinitionServices` points at the process-lifetime shared table.
        unsafe { &*self.shared }
    }
}
/// `IndustryOverrideManager::AddEntityID`: method to find an entity ID and to mark it as
/// reserved for the Industry to be included. Returns a free entity id (slotid) if ever one
/// has been found, or the invalid marker otherwise.
#[allow(
    clippy::cast_possible_truncation,
    reason = "IndustryOverrideManager::AddEntityID keeps the original's implicit C++ integer conversions"
)]
pub unsafe fn add_entity_id(
    manager: *mut OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
    substitute_id: u16,
) -> u16 {
    let (max_offset, max_entities, invalid_id) = unsafe {
        (
            (*manager).max_offset(),
            (*manager).max_entities(),
            (*manager).invalid_id(),
        )
    };
    // This entity hasn't been defined before, so give it an ID now.
    for id in 0..max_entities {
        // Skip overridden industries
        if id < max_offset && unsafe { (*manager).entity_overrides(id) } != invalid_id {
            continue;
        }
        // Get the real live industry. The original narrows the slot to IndustryType for
        // GetIndustrySpec.
        let inds = industrytype::get_industry_spec(IndustryType(id as u8));
        let (enabled, has_file) = unsafe { ((*inds).enabled, (*inds).grf_prop.has_grf_file()) };
        // This industry must be one that is not available(enabled), mostly because of climate.
        // And it must not already be used by a grf (grffile == nullptr).
        // So reserve this slot here, as it is the chosen one
        if !enabled && !has_file {
            let map = unsafe { (*manager).mappings(id) };
            if unsafe { (*map).entity_id } == 0 && unsafe { (*map).grfid } == 0 {
                // winning slot, mark it as been used
                unsafe {
                    (*map).entity_id = grf_local_id;
                    (*map).grfid = grfid;
                    (*map).substitute_id = substitute_id;
                }
                return id;
            }
        }
    }
    invalid_id
}

/// `IndustryOverrideManager::SetEntitySpec`: method to install the new industry data in its
/// proper slot, moving the per-file spec into it.
#[allow(
    clippy::cast_possible_truncation,
    reason = "IndustryOverrideManager::SetEntitySpec keeps the original's implicit C++ integer conversions"
)]
pub unsafe fn publish(manager: *mut OverrideManagerBase, inds: *mut IndustrySpec) {
    let props = unsafe { (*inds).grf_prop };
    let invalid_id = unsafe { (*manager).invalid_id() };
    // First step : We need to find if this industry is already specified in the savegame data.
    // The original narrows GetID and AddEntityID to IndustryType before testing it.
    let mut ind_id =
        IndustryType(unsafe { (*manager).industry_get_id(props.local_id, props.grfid) } as u8);
    if u16::from(ind_id.0) == invalid_id {
        // Not found.
        // Or it has already been overridden, so you've lost your place.
        // Or it is a simple substitute.
        // We need to find a free available slot
        ind_id = IndustryType(unsafe {
            add_entity_id(manager, props.local_id, props.grfid, props.subst_id)
        } as u8);
        unsafe { (*inds).grf_prop.override_id = invalid_id }; // make sure it will not be detected as overridden
    }
    if u16::from(ind_id.0) == invalid_id {
        grf_msg!(
            1,
            "Industry.SetEntitySpec: Too many industries allocated. Ignoring."
        );
        return;
    }
    // Now that we know we can use the given id, copy the spec to its final destination...
    let target = industrytype::get_industry_spec(ind_id);
    unsafe {
        *target = std::mem::replace(&mut *inds, EMPTY_INDUSTRY);
        // ... and mark it as usable
        (*target).enabled = true;
    }
}
/// `IndustryTileOverrideManager::SetEntitySpec`.
pub unsafe fn publish_tile(manager: *mut OverrideManagerBase, its: *mut IndustryTileSpec) {
    let props = unsafe { (*its).grf_prop };
    let (max_offset, invalid_id) = unsafe { ((*manager).max_offset(), (*manager).invalid_id()) };
    let indt_id = IndustryGfx(unsafe {
        override_manager::industry_tile_add_entity_id(
            manager,
            props.local_id,
            props.grfid,
            props.subst_id,
        )
    });
    if indt_id.0 == invalid_id {
        grf_msg!(
            1,
            "IndustryTile.SetEntitySpec: Too many industry tiles allocated. Ignoring."
        );
        return;
    }
    unsafe {
        *industrytype::get_industry_tile_spec(indt_id) = std::mem::replace(&mut *its, EMPTY_TILE);
    }
    // Now add the overrides.
    for i in 0..max_offset {
        let matched = unsafe {
            (*manager).entity_overrides(i) == props.local_id
                && (*manager).grfid_overrides(i) == props.grfid
        };
        if !matched {
            continue;
        }
        let overridden_its = industrytype::get_industry_tile_spec(IndustryGfx(i));
        unsafe {
            (*overridden_its).grf_prop.override_id = indt_id.0;
            (*overridden_its).enabled = false;
            (*manager).set_entity_overrides(i, invalid_id);
            (*manager).set_grfid_overrides(i, 0);
        }
    }
}
/// `GetCargoTypeByLabel(GetActiveCargoLabel(label))`.
fn translate(label: CargoLabelChoice) -> CargoType {
    crate::cargo_registry::get_cargo_type_by_label(crate::newgrf::get_active_cargo_label(label))
}
/// Apply default cargo translation map for unset cargo slots
fn translate_cargo(cargo: &mut [u8], labels: &[CargoLabelChoice]) {
    for (cargo, label) in cargo.iter_mut().zip(labels) {
        if !is_valid_cargo_type(*cargo) {
            *cargo = translate(*label).0;
        }
    }
}

/// `FinaliseIndustriesArray`.
pub unsafe fn finalise(s: &Services) {
    let mut file_index = 0;
    loop {
        let file: *const GRFFile = (s.shared().grf_file_at_or_null)(file_index).cast();
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
                unsafe { publish(override_manager::industry_mngr(), spec) };
            }
            // A null or disabled spec is skipped.
        }
        let count = unsafe { industrytype::file_specs(file) }.tiles.len();
        for i in 0..count {
            if let Some(spec) = (unsafe { industrytype::file_specs(file) }.tiles[i].as_deref_mut())
                .map(std::ptr::from_mut)
            {
                unsafe { publish_tile(override_manager::industile_mngr(), spec) };
            }
        }
        // Won't be used again
        industrytype::clear_file(file);
        file_index += 1;
    }
    for id in 0..IndustryType::NUM_INDUSTRYTYPES.0 {
        let spec = industrytype::get_industry_spec(IndustryType(id));
        let (enabled, has_file, grfid) = unsafe {
            (
                (*spec).enabled,
                (*spec).grf_prop.has_grf_file(),
                (*spec).grf_prop.grfid,
            )
        };
        if enabled && has_file {
            for i in 0..unsafe { (*spec).conflicting.len() } {
                let kind = unsafe { (*spec).conflicting[i] };
                let mapped =
                    crate::industry::resolver::map_new_grf_industry_type(IndustryType(kind), grfid)
                        .0;
                unsafe { (*spec).conflicting[i] = mapped };
            }
        }
        if !enabled {
            unsafe { (*spec).name = STR_NEWGRF_INVALID_INDUSTRYTYPE.0 };
        }
        // Apply default cargo translation map for unset cargo slots
        let mut produced = unsafe { (*spec).produced_cargo };
        let labels = unsafe { (*spec).produced_cargo_label };
        translate_cargo(&mut produced[..INDUSTRY_ORIGINAL_NUM_OUTPUTS], &labels);
        unsafe { (*spec).produced_cargo = produced };
        let mut accepted = unsafe { (*spec).accepts_cargo };
        let labels = unsafe { (*spec).accepts_cargo_label };
        translate_cargo(&mut accepted[..INDUSTRY_ORIGINAL_NUM_INPUTS], &labels);
        unsafe { (*spec).accepts_cargo = accepted };
    }
    for gfx in 0..IndustryGfx::NUM_INDUSTRYTILES.0 {
        let spec = industrytype::get_industry_tile_spec(IndustryGfx(gfx));
        // Apply default cargo translation map for unset cargo slots
        let mut accepted = unsafe { (*spec).accepts_cargo };
        let labels = unsafe { (*spec).accepts_cargo_label };
        translate_cargo(&mut accepted[..INDUSTRY_ORIGINAL_NUM_INPUTS], &labels);
        unsafe { (*spec).accepts_cargo = accepted };
    }
}

/// `ResetIndustries` with the override managers.
pub(crate) unsafe fn reset(climate: crate::landscape_type::LandscapeType, s: &Services) {
    unsafe {
        industrytype::reset(s.shared, climate, s.origin, s.origin_tiles);
        /* Reset any overrides that have been set. */
        (*override_manager::industile_mngr()).reset_override();
        (*override_manager::industry_mngr()).reset_override();
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDSubstituteSpecIdentity" ["industry"] => Identity { grfid, local, substitute };
    "OpenTTDIndustryDefinitionServices" ["industry"] => Services {
        shared, origin, origin_tiles, map_string
    };
}
