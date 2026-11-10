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
static HOUSE: Registered = Registered(UnsafeCell::new(ptr::null_mut()));
static INDUSTRY: Registered = Registered(UnsafeCell::new(ptr::null_mut()));
static TILE: Registered = Registered(UnsafeCell::new(ptr::null_mut()));
pub fn house() -> *mut Manager {
    unsafe { *HOUSE.0.get() }
}
pub fn industry() -> *mut Manager {
    unsafe { *INDUSTRY.0.get() }
}
pub fn tile() -> *mut Manager {
    unsafe { *TILE.0.get() }
}
static AIRPORT: Registered = Registered(UnsafeCell::new(ptr::null_mut()));
static AIRPORT_TILE: Registered = Registered(UnsafeCell::new(ptr::null_mut()));
pub fn airport() -> *mut Manager {
    unsafe { *AIRPORT.0.get() }
}
pub fn airport_tile() -> *mut Manager {
    unsafe { *AIRPORT_TILE.0.get() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_register_manager(manager: *mut Manager) {
    unsafe {
        *AIRPORT.0.get() = manager;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_tile_register_manager(manager: *mut Manager) {
    unsafe {
        *AIRPORT_TILE.0.get() = manager;
    }
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
    unsafe { add(&mut *manager, local, grfid, entity as usize) };
}
/// `OverrideManagerBase::Add`: the entity of `local` in `grfid` overrides `entity`.
fn add(m: &mut Manager, local: u16, grfid: u32, entity: usize) {
    /* An override can be set only once */
    if m.entities[entity] != m.invalid {
        return;
    }
    m.entities[entity] = local;
    m.grfids[entity] = grfid;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_reset_mapping(manager: *mut Manager) {
    unsafe {
        (*manager).mappings.fill(Mapping::default());
    }
}
/// `OverrideManagerBase::ResetOverride`: resets the override, which is used while
/// initializing game.
/// # Safety
/// `manager` is a live override manager with no other borrow.
pub(crate) unsafe fn reset_override(manager: *mut Manager) {
    let m = unsafe { &mut *manager };
    m.entities.fill(m.invalid);
    m.grfids.fill(0);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_reset(manager: *mut Manager) {
    unsafe { reset_override(manager) };
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
    valid: unsafe extern "C" fn(*mut c_void, u16) -> u8,
) -> u16 {
    unsafe {
        add_entity_id(&mut *manager, local, grfid, substitute, existing, |id| {
            valid(native, id) != 0
        })
    }
}
/// `OverrideManagerBase::AddEntityID`: the ID for the entity of `local` in `grfid`,
/// allocating one with substitute `substitute` when it has none. `existing` is
/// `GetID(local, grfid)` and `valid` the manager's `CheckValidNewID`.
fn add_entity_id(
    m: &mut Manager,
    local: u16,
    grfid: u32,
    substitute: u16,
    existing: u16,
    mut valid: impl FnMut(u16) -> bool,
) -> u16 {
    /* Look to see if this entity has already been added. This is done
     * separately from the loop below in case a GRF has been deleted, and there
     * are any gaps in the array.
     */
    let id = existing;
    let invalid = m.invalid;
    if id != invalid {
        return id;
    }
    /* This entity hasn't been defined before, so give it an ID now. */
    let (offset, maximum) = (m.offset, m.maximum);
    for id in offset..maximum {
        if !valid(id) {
            continue;
        }
        let map = &mut m.mappings[usize::from(id)];
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
pub unsafe fn allocate_house(
    manager: *mut Manager,
    local: u16,
    grfid: u32,
    substitute: u16,
) -> u16 {
    let existing = unsafe { get(manager, local, grfid) };
    // HouseOverrideManager inherits OverrideManagerBase::CheckValidNewID.
    add_entity_id(
        unsafe { &mut *manager },
        local,
        grfid,
        substitute,
        existing,
        |_| true,
    )
}
pub unsafe fn pending_override(manager: *const Manager, id: u16) -> (u16, u32) {
    let m = unsafe { &*manager };
    (m.entities[usize::from(id)], m.grfids[usize::from(id)])
}
pub unsafe fn clear_override(manager: *mut Manager, id: u16) {
    let m = unsafe { &mut *manager };
    m.entities[usize::from(id)] = m.invalid;
    m.grfids[usize::from(id)] = 0;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_house_register_manager(manager: *mut Manager) {
    unsafe {
        *HOUSE.0.get() = manager;
    }
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
