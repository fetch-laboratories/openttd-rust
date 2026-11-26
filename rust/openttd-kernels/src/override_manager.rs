/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `OverrideManagerBase` of `newgrf_commons.cpp`, and the `GetID` and `AddEntityID` of the
//! managers that override them.
//!
//! C++ owns each manager object, which holds its Rust owner from construction to destruction.
//! The native facade and `SlObject` borrow the owner's vectors through the spans of
//! `openttd_rust_override_view`, which stay valid until assignment, which refreshes them, or
//! destruction. The game-state mutex serializes every access. `CheckValidNewID` is a native
//! virtual that may revisit the manager, so no reference to the manager is live across it;
//! every other entry ends its references before it returns.
use crate::game_state::GameState;
use std::ffi::c_void;

/// Maps an entity id stored on the map to a GRF file.
/// Entities are objects used ingame (houses, industries, industry tiles) for which we need to
/// correlate the ids from the grf files with the ones in the the savegames themselves.
/// An array of `EntityIDMapping` structs is saved with the savegame so that those GRFs can be
/// loaded in a different order, or removed safely. The index in the array is the entity's ID
/// stored on the map.
///
/// The substitute ID is the ID of an original entity that should be used instead if the GRF
/// containing the new entity is not available.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct EntityIDMapping {
    /// The GRF ID of the file the entity belongs to.
    pub grfid: u32,
    /// The entity ID within the GRF file.
    pub entity_id: u16,
    /// The (original) entity ID to use if this GRF is not available.
    pub substitute_id: u16,
}

/// `OverrideManagerBase`, the Rust owner a native manager holds.
#[derive(Clone)]
pub struct OverrideManagerBase {
    entity_overrides: Vec<u16>,
    grfid_overrides: Vec<u32>,
    /// What is the length of the original entity's array of specs.
    max_offset: u16,
    /// What is the amount of entities, old and new summed.
    max_entities: u16,
    /// ID used to detected invalid entities.
    invalid_id: u16,
    mappings: Vec<EntityIDMapping>,
}

/// The borrowed storage of a manager, which the native facade spans.
#[repr(C)]
pub struct View {
    mappings: *mut EntityIDMapping,
    entities: *mut u16,
    grfids: *mut u32,
}

impl OverrideManagerBase {
    /// `OverrideManagerBase::OverrideManagerBase`: constructor of generic class.
    /// `offset` is the end of original data for this entity, i.e. houses = 110; `maximum` the
    /// number of entities this manager can deal with, i.e. houses = 512; and `invalid` the ID
    /// used to identify an invalid entity id.
    pub(crate) fn new(offset: u16, maximum: u16, invalid: u16) -> Self {
        Self {
            max_offset: offset,
            max_entities: maximum,
            invalid_id: invalid,
            mappings: vec![EntityIDMapping::default(); usize::from(maximum)],
            entity_overrides: vec![invalid; usize::from(offset)],
            grfid_overrides: vec![0; usize::from(offset)],
        }
    }

    /// `max_offset`.
    pub(crate) fn max_offset(&self) -> u16 {
        self.max_offset
    }
    /// `max_entities`.
    pub(crate) fn max_entities(&self) -> u16 {
        self.max_entities
    }
    /// `invalid_id`.
    pub(crate) fn invalid_id(&self) -> u16 {
        self.invalid_id
    }
    /// `entity_overrides[i]`.
    pub(crate) fn entity_overrides(&self, i: u16) -> u16 {
        self.entity_overrides[usize::from(i)]
    }
    /// Store `entity_overrides[i]`.
    pub(crate) fn set_entity_overrides(&mut self, i: u16, local_id: u16) {
        self.entity_overrides[usize::from(i)] = local_id;
    }
    /// `grfid_overrides[i]`.
    pub(crate) fn grfid_overrides(&self, i: u16) -> u32 {
        self.grfid_overrides[usize::from(i)]
    }
    /// Store `grfid_overrides[i]`.
    pub(crate) fn set_grfid_overrides(&mut self, i: u16, grfid: u32) {
        self.grfid_overrides[usize::from(i)] = grfid;
    }
    /// `&mappings[id]`, whose fields a caller reads and writes in place.
    pub(crate) fn mappings(&mut self, id: u16) -> *mut EntityIDMapping {
        &raw mut self.mappings[usize::from(id)]
    }

    /// `OverrideManagerBase::Add`: since the entity IDs defined by the GRF file does not
    /// necessarily correlate to those used by the game, the IDs used for overriding old
    /// entities must be translated when the entity spec is set. `local_id` is the ID in the
    /// grf file, `grfid` the ID of the grf file and `entity_type` the original entity type.
    pub(crate) fn add(&mut self, local_id: u16, grfid: u32, entity_type: u32) {
        crate::services_assert::assert_with(|| entity_type < u32::from(self.max_offset));
        let entity_type = entity_type as usize;
        /* An override can be set only once */
        if self.entity_overrides[entity_type] != self.invalid_id {
            return;
        }
        self.entity_overrides[entity_type] = local_id;
        self.grfid_overrides[entity_type] = grfid;
    }

    /// `OverrideManagerBase::ResetMapping`: resets the mapping, which is used while
    /// initializing game.
    pub(crate) fn reset_mapping(&mut self) {
        self.mappings.fill(EntityIDMapping::default());
    }

    /// `OverrideManagerBase::ResetOverride`: resets the override, which is used while
    /// initializing game.
    pub(crate) fn reset_override(&mut self) {
        self.entity_overrides.fill(self.invalid_id);
        self.grfid_overrides.fill(0);
    }

    /// `OverrideManagerBase::GetID`: return the ID (if ever available) of a previously
    /// inserted entity. `grf_local_id` is the ID of this entity within the grfID and `grfid`
    /// the ID of the grf file. Returns the ID of the candidate, or the invalid flag item ID.
    pub(crate) fn get_id(&self, grf_local_id: u16, grfid: u32) -> u16 {
        for id in 0..self.max_entities {
            let map = &self.mappings[usize::from(id)];
            if map.entity_id == grf_local_id && map.grfid == grfid {
                return id;
            }
        }

        self.invalid_id
    }

    /// `IndustryOverrideManager::GetID`: return the ID (if ever available) of a previously
    /// inserted entity. `grf_local_id` is the ID of this entity within the grfID and `grfid`
    /// the ID of the grf file. Returns the ID of the candidate, or the invalid flag item ID.
    pub(crate) fn industry_get_id(&self, grf_local_id: u16, grfid: u32) -> u16 {
        let id = self.get_id(grf_local_id, grfid);
        if id != self.invalid_id {
            return id;
        }

        /* No mapping found, try the overrides */
        for id in 0..self.max_offset {
            if self.entity_overrides[usize::from(id)] == grf_local_id
                && self.grfid_overrides[usize::from(id)] == grfid
            {
                return id;
            }
        }

        self.invalid_id
    }

    /// `OverrideManagerBase::GetGRFID`: gives the GRFID of the file the entity belongs to.
    pub(crate) fn get_grfid(&self, entity_id: u16) -> u32 {
        self.mappings[usize::from(entity_id)].grfid
    }

    /// `OverrideManagerBase::GetSubstituteID`: gives the substitute of the entity, as
    /// specified by the grf file.
    pub(crate) fn get_substitute_id(&self, entity_id: u16) -> u16 {
        self.mappings[usize::from(entity_id)].substitute_id
    }
}

/// `OverrideManagerBase::AddEntityID`: reserves a place in the mapping array for an entity to
/// be installed. `grf_local_id` is an arbitrary id given by the grf's author, also known as
/// setid; `grfid` is the id of the grf file itself; `substitute_id` is the original entity
/// from which data is copied for the new one. `id` is the manager's `GetID(grf_local_id,
/// grfid)` and `check_valid_new_id` its `CheckValidNewID`. Returns the proper usable slot id,
/// or the invalid marker if none is found.
/// # Safety
/// `manager` is a live manager; no reference to it is live.
#[allow(unsafe_code)]
unsafe fn add_entity_id(
    manager: *mut OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
    substitute_id: u16,
    id: u16,
    mut check_valid_new_id: impl FnMut(u16) -> bool,
) -> u16 {
    /* Look to see if this entity has already been added. This is done
     * separately from the loop below in case a GRF has been deleted, and there
     * are any gaps in the array.
     */
    // SAFETY: The caller's contract; each access below ends before the check runs.
    if id != unsafe { (*manager).invalid_id } {
        return id;
    }

    /* This entity hasn't been defined before, so give it an ID now. */
    let mut id = unsafe { (*manager).max_offset };
    while id < unsafe { (*manager).max_entities } {
        let map = unsafe { (*manager).mappings(id) };

        if check_valid_new_id(id)
            && unsafe { (*map).entity_id } == 0
            && unsafe { (*map).grfid } == 0
        {
            unsafe {
                (*map).entity_id = grf_local_id;
                (*map).grfid = grfid;
                (*map).substitute_id = substitute_id;
            }
            return id;
        }
        id += 1;
    }

    unsafe { (*manager).invalid_id }
}

/// `AddEntityID` of `_house_mngr`: `HouseOverrideManager` inherits
/// `OverrideManagerBase::AddEntityID`, `GetID` and `CheckValidNewID`.
/// # Safety
/// `manager` is a live manager; no reference to it is live.
#[allow(unsafe_code)]
pub(crate) unsafe fn house_add_entity_id(
    manager: *mut OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
    substitute_id: u16,
) -> u16 {
    // SAFETY: The caller's contract.
    unsafe {
        let id = (*manager).get_id(grf_local_id, grfid);
        add_entity_id(manager, grf_local_id, grfid, substitute_id, id, |_| true)
    }
}

/// `AddEntityID` of `_airport_mngr`: `AirportOverrideManager` inherits
/// `OverrideManagerBase::AddEntityID`, `GetID` and `CheckValidNewID`.
/// # Safety
/// `manager` is a live manager; no reference to it is live.
#[allow(unsafe_code)]
pub(crate) unsafe fn airport_add_entity_id(
    manager: *mut OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
    substitute_id: u16,
) -> u16 {
    // SAFETY: The caller's contract.
    unsafe {
        let id = (*manager).get_id(grf_local_id, grfid);
        add_entity_id(manager, grf_local_id, grfid, substitute_id, id, |_| true)
    }
}

/// `AddEntityID` of `_airporttile_mngr`: `AirportTileOverrideManager` inherits
/// `OverrideManagerBase::AddEntityID` and `GetID`, with its own `CheckValidNewID`.
/// # Safety
/// `manager` is a live manager; no reference to it is live.
#[allow(unsafe_code)]
pub(crate) unsafe fn airport_tile_add_entity_id(
    manager: *mut OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
    substitute_id: u16,
) -> u16 {
    // SAFETY: The caller's contract.
    unsafe {
        let id = (*manager).get_id(grf_local_id, grfid);
        add_entity_id(
            manager,
            grf_local_id,
            grfid,
            substitute_id,
            id,
            crate::newgrf_commons::airport_tile_check_valid_new_id,
        )
    }
}

/// `AddEntityID` of `_industile_mngr`: `IndustryTileOverrideManager` inherits
/// `OverrideManagerBase::AddEntityID` and `GetID`, with its own `CheckValidNewID`.
/// # Safety
/// `manager` is a live manager; no reference to it is live.
#[allow(unsafe_code)]
pub(crate) unsafe fn industry_tile_add_entity_id(
    manager: *mut OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
    substitute_id: u16,
) -> u16 {
    // SAFETY: The caller's contract.
    unsafe {
        let id = (*manager).get_id(grf_local_id, grfid);
        add_entity_id(
            manager,
            grf_local_id,
            grfid,
            substitute_id,
            id,
            crate::newgrf_commons::industry_tile_check_valid_new_id,
        )
    }
}

// The managers Rust callers name: C++ owns each global and registers its Rust owner when
// the global is constructed, before any entry runs.
static HOUSE_MNGR: GameState<*mut OverrideManagerBase> = GameState::new(std::ptr::null_mut());
static INDUSTRY_MNGR: GameState<*mut OverrideManagerBase> = GameState::new(std::ptr::null_mut());
static INDUSTILE_MNGR: GameState<*mut OverrideManagerBase> = GameState::new(std::ptr::null_mut());
static AIRPORT_MNGR: GameState<*mut OverrideManagerBase> = GameState::new(std::ptr::null_mut());
static AIRPORTTILE_MNGR: GameState<*mut OverrideManagerBase> = GameState::new(std::ptr::null_mut());

/// `_house_mngr`.
#[allow(unsafe_code)]
pub(crate) fn house_mngr() -> *mut OverrideManagerBase {
    // SAFETY: The module's serialization contract.
    unsafe { *HOUSE_MNGR.get() }
}
/// `_industry_mngr`.
#[allow(unsafe_code)]
pub(crate) fn industry_mngr() -> *mut OverrideManagerBase {
    // SAFETY: The module's serialization contract.
    unsafe { *INDUSTRY_MNGR.get() }
}
/// `_industile_mngr`.
#[allow(unsafe_code)]
pub(crate) fn industile_mngr() -> *mut OverrideManagerBase {
    // SAFETY: The module's serialization contract.
    unsafe { *INDUSTILE_MNGR.get() }
}
/// `_airport_mngr`.
#[allow(unsafe_code)]
pub(crate) fn airport_mngr() -> *mut OverrideManagerBase {
    // SAFETY: The module's serialization contract.
    unsafe { *AIRPORT_MNGR.get() }
}
/// `_airporttile_mngr`.
#[allow(unsafe_code)]
pub(crate) fn airporttile_mngr() -> *mut OverrideManagerBase {
    // SAFETY: The module's serialization contract.
    unsafe { *AIRPORTTILE_MNGR.get() }
}

/// Register the Rust owner of `_house_mngr`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_house_register_manager(manager: *mut OverrideManagerBase) {
    // SAFETY: The module's serialization contract.
    unsafe { *HOUSE_MNGR.get() = manager };
}
/// Register the Rust owner of `_industry_mngr`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_register_manager(manager: *mut OverrideManagerBase) {
    // SAFETY: The module's serialization contract.
    unsafe { *INDUSTRY_MNGR.get() = manager };
}
/// Register the Rust owner of `_industile_mngr`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_industry_tile_register_manager(manager: *mut OverrideManagerBase) {
    // SAFETY: The module's serialization contract.
    unsafe { *INDUSTILE_MNGR.get() = manager };
}
/// Register the Rust owner of `_airport_mngr`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_register_manager(manager: *mut OverrideManagerBase) {
    // SAFETY: The module's serialization contract.
    unsafe { *AIRPORT_MNGR.get() = manager };
}
/// Register the Rust owner of `_airporttile_mngr`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_tile_register_manager(manager: *mut OverrideManagerBase) {
    // SAFETY: The module's serialization contract.
    unsafe { *AIRPORTTILE_MNGR.get() = manager };
}

/// `OverrideManagerBase::OverrideManagerBase`, for the native constructor.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_override_new(
    offset: u16,
    maximum: u16,
    invalid: u16,
) -> *mut OverrideManagerBase {
    Box::into_raw(Box::new(OverrideManagerBase::new(offset, maximum, invalid)))
}
/// The native copy constructor's owner.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_clone(
    manager: *const OverrideManagerBase,
) -> *mut OverrideManagerBase {
    // SAFETY: The caller's contract.
    Box::into_raw(Box::new(unsafe { (*manager).clone() }))
}
/// The native copy assignment's owner.
/// # Safety
/// `manager` and `source` are live managers.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_assign(
    manager: *mut OverrideManagerBase,
    source: *const OverrideManagerBase,
) {
    if std::ptr::eq(manager, source) {
        return;
    }
    // SAFETY: The caller's contract; the two managers are distinct.
    let (source, target) = unsafe { (&*source, &mut *manager) };
    target.entity_overrides.clone_from(&source.entity_overrides);
    target.grfid_overrides.clone_from(&source.grfid_overrides);
    target.max_offset = source.max_offset;
    target.max_entities = source.max_entities;
    target.invalid_id = source.invalid_id;
    target.mappings.clone_from(&source.mappings);
}
/// The native destructor's owner.
/// # Safety
/// `manager` is a live manager that nothing borrows any more.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_delete(manager: *mut OverrideManagerBase) {
    // SAFETY: The caller's contract; `openttd_rust_override_new` or `_clone` allocated it.
    drop(unsafe { Box::from_raw(manager) });
}
/// The storage the native facade spans.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_view(manager: *mut OverrideManagerBase) -> View {
    // SAFETY: The caller's contract.
    let m = unsafe { &mut *manager };
    View {
        mappings: m.mappings.as_mut_ptr(),
        entities: m.entity_overrides.as_mut_ptr(),
        grfids: m.grfid_overrides.as_mut_ptr(),
    }
}
/// `OverrideManagerBase::Add`.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_add(
    manager: *mut OverrideManagerBase,
    local_id: u16,
    grfid: u32,
    entity_type: u32,
) {
    // SAFETY: The caller's contract.
    unsafe { (*manager).add(local_id, grfid, entity_type) };
}
/// `OverrideManagerBase::ResetMapping`.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_reset_mapping(manager: *mut OverrideManagerBase) {
    // SAFETY: The caller's contract.
    unsafe { (*manager).reset_mapping() };
}
/// `OverrideManagerBase::ResetOverride`.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_reset(manager: *mut OverrideManagerBase) {
    // SAFETY: The caller's contract.
    unsafe { (*manager).reset_override() };
}
/// `OverrideManagerBase::GetID`.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_get(
    manager: *const OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
) -> u16 {
    // SAFETY: The caller's contract.
    unsafe { (*manager).get_id(grf_local_id, grfid) }
}
/// `IndustryOverrideManager::GetID`.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_definition_id(
    manager: *const OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
) -> u16 {
    // SAFETY: The caller's contract.
    unsafe { (*manager).industry_get_id(grf_local_id, grfid) }
}
/// `OverrideManagerBase::GetGRFID`.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_grfid(
    manager: *const OverrideManagerBase,
    entity_id: u16,
) -> u32 {
    // SAFETY: The caller's contract.
    unsafe { (*manager).get_grfid(entity_id) }
}
/// `OverrideManagerBase::GetSubstituteID`.
/// # Safety
/// `manager` is a live manager.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_substitute(
    manager: *const OverrideManagerBase,
    entity_id: u16,
) -> u16 {
    // SAFETY: The caller's contract.
    unsafe { (*manager).get_substitute_id(entity_id) }
}
/// `OverrideManagerBase::AddEntityID` for a native manager: `existing` is its `GetID`
/// result and `valid` calls its `CheckValidNewID` on `native`.
/// # Safety
/// `manager` is the live owner of the live native manager `native`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_override_allocate(
    manager: *mut OverrideManagerBase,
    grf_local_id: u16,
    grfid: u32,
    substitute_id: u16,
    native: *mut c_void,
    existing: u16,
    valid: unsafe extern "C" fn(*mut c_void, u16) -> u8,
) -> u16 {
    // SAFETY: The caller's contract; no reference to the manager is live across `valid`.
    unsafe {
        add_entity_id(
            manager,
            grf_local_id,
            grfid,
            substitute_id,
            existing,
            |id| valid(native, id) != 0,
        )
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "EntityIDMapping" ["industry"] => EntityIDMapping { grfid, entity_id, substitute_id };
    "OpenTTDOverrideView" ["industry"] => View { mappings, entities, grfids };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static FAILED_ASSERTIONS: AtomicU32 = AtomicU32::new(0);
    extern "C" fn record_assertion(condition: bool) {
        if !condition {
            FAILED_ASSERTIONS.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// `OverrideManagerBase::Add` asserts `entity_type < max_offset` before its first read;
    /// an entity type of `max_offset` ends there, with no store.
    #[test]
    fn add_of_the_max_offset_ends_at_its_assertion() {
        if crate::services_assert::isolate_test_exit(
            "override_manager::tests::add_of_the_max_offset_ends_at_its_assertion",
        ) {
            return;
        }
        let mut manager = OverrideManagerBase::new(3, 10, 0xFF);
        manager.add(7, 123, 3);
        unreachable!("Add returned past its failed assertion");
    }

    /// `OverrideManagerBase::Add` of an original entity type stores the override once: an
    /// override can be set only once, so a second `Add` of the type changes nothing.
    #[test]
    fn add_stores_an_override_only_once() {
        if crate::services_assert::isolate_test_check(
            "override_manager::tests::add_stores_an_override_only_once",
            record_assertion,
        ) {
            return;
        }
        let mut manager = OverrideManagerBase::new(3, 10, 0xFF);
        manager.add(7, 123, 2);
        manager.add(8, 456, 2);
        assert_eq!(
            (manager.entity_overrides(2), manager.grfid_overrides(2)),
            (7, 123)
        );
        for i in 0..2 {
            assert_eq!(
                (manager.entity_overrides(i), manager.grfid_overrides(i)),
                (0xFF, 0)
            );
        }
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 0);
    }
}
