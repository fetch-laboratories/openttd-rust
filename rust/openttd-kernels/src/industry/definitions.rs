/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry definition allocation and publication. Native specs hold tables and graphs;
//! mapping/override state is canonical Rust storage. All calls run under the game-state
//! mutex. References to manager storage end before native leaves or reentrant owners.
use super::{SpecHandle, TileSpec};
pub(crate) mod properties;
pub(crate) mod storage;
use crate::override_manager::{self, Mapping};
use std::ffi::c_void;
#[repr(C)]
pub struct Identity {
    pub grfid: u32,
    pub local: u16,
    pub substitute: u16,
}
#[repr(C)]
pub struct Availability {
    pub enabled: bool,
    pub has_file: bool,
}
#[repr(C)]
pub struct Counts {
    pub industries: usize,
    pub tiles: usize,
}
#[repr(C)]
pub struct Cargo {
    pub label: u32,
    pub mixed: bool,
    pub cargo: u8,
}
#[repr(C)]
pub struct Services {
    pub industry: unsafe extern "C" fn(u16) -> *mut SpecHandle,
    pub tile: unsafe extern "C" fn(u16) -> *mut TileSpec,
    pub identity: unsafe extern "C" fn(*const SpecHandle) -> Identity,
    pub tile_identity: unsafe extern "C" fn(*const TileSpec) -> Identity,
    pub availability: unsafe extern "C" fn(*const SpecHandle) -> Availability,
    pub set_enabled: unsafe extern "C" fn(*mut SpecHandle, bool),
    pub set_override: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub set_tile_override: unsafe extern "C" fn(*mut TileSpec, u16),
    pub set_tile_enabled: unsafe extern "C" fn(*mut TileSpec, bool),
    pub publish: unsafe extern "C" fn(u16, *mut SpecHandle),
    pub publish_tile: unsafe extern "C" fn(u16, *mut TileSpec),
    pub industry_full: unsafe extern "C" fn(),
    pub tile_full: unsafe extern "C" fn(),
    pub reset_industry: unsafe extern "C" fn(u16),
    pub clear_industry: unsafe extern "C" fn(u16),
    pub climate_available: unsafe extern "C" fn(*const SpecHandle, u8) -> bool,
    pub reset_tile: unsafe extern "C" fn(u16),
    pub clear_tile: unsafe extern "C" fn(u16),
    pub file: unsafe extern "C" fn(usize) -> *mut c_void,
    pub counts: unsafe extern "C" fn(*const c_void) -> Counts,
    pub file_industry: unsafe extern "C" fn(*mut c_void, usize) -> *mut SpecHandle,
    pub file_tile: unsafe extern "C" fn(*mut c_void, usize) -> *mut TileSpec,
    pub clear_file: unsafe extern "C" fn(*mut c_void),
    pub conflicting: unsafe extern "C" fn(*mut SpecHandle) -> *mut u8,
    pub invalid_name: unsafe extern "C" fn(*mut SpecHandle),
    pub produced: unsafe extern "C" fn(*const SpecHandle, usize) -> Cargo,
    pub accepted: unsafe extern "C" fn(*const SpecHandle, usize) -> Cargo,
    pub tile_accepted: unsafe extern "C" fn(*const TileSpec, usize) -> Cargo,
    pub set_produced: unsafe extern "C" fn(*mut SpecHandle, usize, u8),
    pub set_accepted: unsafe extern "C" fn(*mut SpecHandle, usize, u8),
    pub set_tile_accepted: unsafe extern "C" fn(*mut TileSpec, usize, u8),
}
pub unsafe fn allocate(local: u16, grfid: u32, substitute: u16, s: &Services) -> u16 {
    let manager = override_manager::industry();
    let (offset, maximum, invalid) =
        unsafe { ((*manager).offset, (*manager).maximum, (*manager).invalid) };
    for id in 0..maximum {
        if id < offset && unsafe { (&(*manager).entities)[usize::from(id)] } != invalid {
            continue;
        }
        let spec = unsafe { (s.industry)(id) };
        let status = unsafe { (s.availability)(spec) };
        if !status.enabled && !status.has_file {
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
pub unsafe fn allocate_tile(local: u16, grfid: u32, substitute: u16) -> u16 {
    let manager = override_manager::tile();
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
unsafe extern "C" fn tile_valid(_: *mut c_void, id: u16) -> bool {
    id != 0xff
}
pub unsafe fn publish(spec: *mut SpecHandle, s: &Services) {
    let identity = unsafe { (s.identity)(spec) };
    let manager = override_manager::industry();
    let invalid = unsafe { (*manager).invalid };
    // The original narrows GetID and AddEntityID to IndustryType before testing it.
    let mut id = unsafe { override_manager::industry_id(identity.local, identity.grfid) } as u8;
    if u16::from(id) == invalid {
        id = unsafe { allocate(identity.local, identity.grfid, identity.substitute, s) } as u8;
        unsafe {
            (s.set_override)(spec, invalid);
        }
    }
    if u16::from(id) == invalid {
        unsafe {
            (s.industry_full)();
        }
        return;
    }
    unsafe {
        (s.publish)(u16::from(id), spec);
        (s.set_enabled)((s.industry)(u16::from(id)), true);
    }
}
pub unsafe fn publish_tile(spec: *mut TileSpec, s: &Services) {
    let identity = unsafe { (s.tile_identity)(spec) };
    let manager = override_manager::tile();
    let (offset, invalid) = unsafe { ((*manager).offset, (*manager).invalid) };
    let id = unsafe { allocate_tile(identity.local, identity.grfid, identity.substitute) };
    if id == invalid {
        unsafe {
            (s.tile_full)();
        }
        return;
    }
    unsafe {
        (s.publish_tile)(id, spec);
    }
    for i in 0..offset {
        let matched = unsafe {
            (&(*manager).entities)[usize::from(i)] == identity.local
                && (&(*manager).grfids)[usize::from(i)] == identity.grfid
        };
        if !matched {
            continue;
        }
        let overridden = unsafe { (s.tile)(i) };
        unsafe {
            (s.set_tile_override)(overridden, id);
            (s.set_tile_enabled)(overridden, false);
        }
        unsafe {
            (&mut (*manager).entities)[usize::from(i)] = invalid;
            (&mut (*manager).grfids)[usize::from(i)] = 0;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_definition_allocate(
    local: u16,
    grfid: u32,
    substitute: u16,
    s: *const Services,
) -> u16 {
    unsafe { allocate(local, grfid, substitute, &*s) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_definition_publish(
    spec: *mut SpecHandle,
    s: *const Services,
) {
    unsafe {
        publish(spec, &*s);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_definition_publish(
    spec: *mut TileSpec,
    s: *const Services,
) {
    unsafe {
        publish_tile(spec, &*s);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_definition_reset(climate: u8, s: *const Services) {
    let s = unsafe { &*s };
    for id in 0..crate::industry_type::NEW_INDUSTRYOFFSET {
        unsafe {
            (s.reset_industry)(id);
        }
    }
    for id in
        crate::industry_type::NEW_INDUSTRYOFFSET..u16::from(crate::industry_type::NUM_INDUSTRYTYPES)
    {
        unsafe {
            (s.clear_industry)(id);
        }
    }
    for id in 0..crate::industry_type::NEW_INDUSTRYOFFSET {
        let spec = unsafe { (s.industry)(id) };
        let available = unsafe { (s.climate_available)(spec, climate) };
        unsafe {
            (s.set_enabled)(spec, available);
        }
    }
    for id in 0..crate::industry_type::NEW_INDUSTRYTILEOFFSET {
        unsafe {
            (s.reset_tile)(id);
        }
    }
    for id in crate::industry_type::NEW_INDUSTRYTILEOFFSET..crate::industry_type::NUM_INDUSTRYTILES
    {
        unsafe {
            (s.clear_tile)(id);
        }
    }
    unsafe {
        override_manager::openttd_rust_override_reset(override_manager::tile());
        override_manager::openttd_rust_override_reset(override_manager::industry());
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDIndustryDefinitionIdentity" ["industry"] => Identity { grfid, local, substitute };
    "OpenTTDIndustryDefinitionAvailability" ["industry"] => Availability { enabled, has_file };
    "OpenTTDIndustryDefinitionCounts" ["industry"] => Counts { industries, tiles };
    "OpenTTDIndustryDefinitionCargo" ["industry"] => Cargo { label, mixed, cargo };
    "OpenTTDIndustryDefinitionServices" ["industry"] => Services {
        industry, tile, identity, tile_identity, availability, set_enabled, set_override, set_tile_override, set_tile_enabled, publish, publish_tile, industry_full, tile_full, reset_industry, clear_industry, climate_available, reset_tile, clear_tile, file, counts, file_industry, file_tile, clear_file, conflicting, invalid_name, produced, accepted, tile_accepted, set_produced, set_accepted, set_tile_accepted
    };
}
