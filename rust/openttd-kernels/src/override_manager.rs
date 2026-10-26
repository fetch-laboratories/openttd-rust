/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `OverrideManagerBase` owns stable vectors. Native facades and `SlObject` borrow their storage
//! until assignment or destruction; assignment refreshes native spans. The game-state mutex serializes access. End
//! every reference before virtual native calls, which may revisit the manager. FFI never unwinds.
#![allow(unsafe_code)]
use std::{cell::UnsafeCell, ffi::c_void, ptr};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Mapping {
    pub grfid: u32,
    pub entity_id: u16,
    pub substitute_id: u16,
}
#[derive(Clone)]
pub struct Manager {
    pub mappings: Vec<Mapping>,
    pub entities: Vec<u16>,
    pub grfids: Vec<u32>,
    pub offset: u16,
    pub maximum: u16,
    pub invalid: u16,
}
#[repr(C)]
pub struct View {
    mappings: *mut Mapping,
    entities: *mut u16,
    grfids: *mut u32,
}
struct Registered(UnsafeCell<*mut Manager>);
// Access is serialized by the native game-state mutex, including initialization.
unsafe impl Sync for Registered {}
static INDUSTRY: Registered = Registered(UnsafeCell::new(ptr::null_mut()));
static TILE: Registered = Registered(UnsafeCell::new(ptr::null_mut()));
pub fn industry() -> *mut Manager {
    unsafe { *INDUSTRY.0.get() }
}
pub fn tile() -> *mut Manager {
    unsafe { *TILE.0.get() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_new(
    offset: u16,
    maximum: u16,
    invalid: u16,
) -> *mut Manager {
    Box::into_raw(Box::new(Manager {
        mappings: vec![Mapping::default(); usize::from(maximum)],
        entities: vec![invalid; usize::from(offset)],
        grfids: vec![0; usize::from(offset)],
        offset,
        maximum,
        invalid,
    }))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_clone(manager: *const Manager) -> *mut Manager {
    Box::into_raw(Box::new(unsafe { (&*manager).clone() }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_assign(
    manager: *mut Manager,
    source: *const Manager,
) {
    if std::ptr::eq(manager, source) {
        return;
    }
    let source = unsafe { &*source };
    let target = unsafe { &mut *manager };
    target.entities.clone_from(&source.entities);
    target.grfids.clone_from(&source.grfids);
    target.offset = source.offset;
    target.maximum = source.maximum;
    target.invalid = source.invalid;
    target.mappings.clone_from(&source.mappings);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_delete(manager: *mut Manager) {
    drop(unsafe { Box::from_raw(manager) });
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_view(manager: *mut Manager) -> View {
    let m = unsafe { &mut *manager };
    View {
        mappings: m.mappings.as_mut_ptr(),
        entities: m.entities.as_mut_ptr(),
        grfids: m.grfids.as_mut_ptr(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_add(
    manager: *mut Manager,
    local: u16,
    grfid: u32,
    entity: u32,
) {
    let m = unsafe { &mut *manager };
    let i = entity as usize;
    if m.entities[i] != m.invalid {
        return;
    }
    m.entities[i] = local;
    m.grfids[i] = grfid;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_reset_mapping(manager: *mut Manager) {
    unsafe {
        (*manager).mappings.fill(Mapping::default());
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_reset(manager: *mut Manager) {
    let m = unsafe { &mut *manager };
    m.entities.fill(m.invalid);
    m.grfids.fill(0);
}
pub unsafe fn get(manager: *const Manager, local: u16, grfid: u32) -> u16 {
    let m = unsafe { &*manager };
    for id in 0..m.maximum {
        let map = m.mappings[usize::from(id)];
        if map.entity_id == local && map.grfid == grfid {
            return id;
        }
    }
    m.invalid
}
pub unsafe fn industry_id_for(manager: *const Manager, local: u16, grfid: u32) -> u16 {
    let id = unsafe { get(manager, local, grfid) };
    let m = unsafe { &*manager };
    if id != m.invalid {
        return id;
    }
    for id in 0..m.offset {
        if m.entities[usize::from(id)] == local && m.grfids[usize::from(id)] == grfid {
            return id;
        }
    }
    m.invalid
}
pub unsafe fn industry_id(local: u16, grfid: u32) -> u16 {
    unsafe { industry_id_for(industry(), local, grfid) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_get(
    manager: *const Manager,
    local: u16,
    grfid: u32,
) -> u16 {
    unsafe { get(manager, local, grfid) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_definition_id(
    manager: *const Manager,
    local: u16,
    grfid: u32,
) -> u16 {
    unsafe { industry_id_for(manager, local, grfid) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_grfid(manager: *const Manager, id: u16) -> u32 {
    unsafe { (&(*manager).mappings)[usize::from(id)].grfid }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_substitute(manager: *const Manager, id: u16) -> u16 {
    unsafe { (&(*manager).mappings)[usize::from(id)].substitute_id }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_allocate(
    manager: *mut Manager,
    local: u16,
    grfid: u32,
    substitute: u16,
    native: *mut c_void,
    existing: u16,
    valid: unsafe extern "C" fn(*mut c_void, u16) -> bool,
) -> u16 {
    let id = existing;
    let invalid = unsafe { (*manager).invalid };
    if id != invalid {
        return id;
    }
    let (offset, maximum) = unsafe { ((*manager).offset, (*manager).maximum) };
    for id in offset..maximum {
        if !unsafe { valid(native, id) } {
            continue;
        }
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
    invalid
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_register_manager(manager: *mut Manager) {
    unsafe {
        *INDUSTRY.0.get() = manager;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_register_manager(manager: *mut Manager) {
    unsafe {
        *TILE.0.get() = manager;
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "EntityIDMapping" ["industry"] => Mapping { grfid, entity_id, substitute_id };
    "OpenTTDOverrideView" ["industry"] => View { mappings, entities, grfids };
}
