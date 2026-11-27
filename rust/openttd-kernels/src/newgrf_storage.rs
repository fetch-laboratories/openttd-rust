/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Functionality related to the temporary and persistent storage arrays for NewGRFs
//! (`newgrf_storage.h`, `newgrf_storage.cpp`).
//!
//! A [`PersistentStorageArray`] is owned by its C++ shell, a `PersistentStorageArray`
//! that keeps `grfid`, `feature` and `tile` and the `storage` reference the saver writes
//! through, and lives from the shell's construction to its destruction. The C++ shell
//! aliases `storage`, so Rust reaches an array only through a raw pointer, with
//! field-sized loads and stores. The mode flags and the set of changed arrays are
//! game state, touched only under the game-state mutex. These statics have no
//! destructor, so they outlive native global storage and pool destruction.

use crate::debug::debug;
use crate::game_state::GameState;
use crate::services_assert;
use std::collections::BTreeMap;
use std::ffi::c_void;

/// Mode switches to the behaviour of persistent storage array.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentStorageMode(pub u8);
impl PersistentStorageMode {
    /// Enter the gameloop, changes will be permanent.
    pub const PSM_ENTER_GAMELOOP: Self = Self(0);
    /// Leave the gameloop, changes will be temporary.
    pub const PSM_LEAVE_GAMELOOP: Self = Self(1);
    /// Enter command scope, changes will be permanent.
    pub const PSM_ENTER_COMMAND: Self = Self(2);
    /// Leave command scope, revert to previous mode.
    pub const PSM_LEAVE_COMMAND: Self = Self(3);
    /// Enter command test mode, changes will be temporary.
    pub const PSM_ENTER_TESTMODE: Self = Self(4);
    /// Leave command test mode, revert to previous mode.
    pub const PSM_LEAVE_TESTMODE: Self = Self(5);
}

/// The C++ storage leaves: the pool, and the members of the
/// `BasePersistentStorageArray` shell that holds an array.
#[repr(C)]
pub struct Services {
    /// `PersistentStorage::CanAllocateItem()`.
    can_allocate_item: extern "C" fn() -> bool,
    /// `new PersistentStorage(grfid, feature, tile)`: the new storage's array.
    new: extern "C" fn(u32, u8, u32) -> *mut PersistentStorageArray<256>,
    /// `BasePersistentStorageArray::grfid`.
    grfid: extern "C" fn(*const c_void) -> u32,
    /// `BasePersistentStorageArray::feature`.
    feature: extern "C" fn(*const c_void) -> u8,
    /// `BasePersistentStorageArray::tile`.
    tile: extern "C" fn(*const c_void) -> u32,
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStorageServices" ["storage"] => Services { can_allocate_item, new => new_storage, grfid, feature, tile };
}
/// `BasePersistentStorageArray`: what every array holds whatever its size, which
/// [`PersistentStorageArray`] starts with.
#[repr(C)]
struct BasePersistentStorageArray {
    /// The C++ `BasePersistentStorageArray` that owns this array. It has no original:
    /// the original's array is that object.
    shell: *mut c_void,
    /// The storage table, which reads the shell's members.
    services: *const Services,
}

/// Class for persistent storage of data.
/// On #ClearChanges that data is either reverted or saved.
#[repr(C)]
pub struct PersistentStorageArray<const SIZE: usize> {
    base: BasePersistentStorageArray,
    /// Memory for the storage array
    storage: [i32; SIZE],
    /// Temporary memory to store previous state so it can be reverted, e.g. for command tests.
    prev_storage: Option<Box<[i32; SIZE]>>,
}

/// A changed array in `_changed_storage_arrays`, with its `ClearChanges`.
#[derive(Clone, Copy)]
struct ChangedArray {
    array: *mut BasePersistentStorageArray,
    clear_changes: unsafe fn(*mut BasePersistentStorageArray),
}

/// `BasePersistentStorageArray::gameloop`.
static GAMELOOP: GameState<bool> = GameState::new(false);
/// `BasePersistentStorageArray::command`.
static COMMAND: GameState<bool> = GameState::new(false);
/// `BasePersistentStorageArray::testmode`.
static TESTMODE: GameState<bool> = GameState::new(false);
/// The changed storage arrays, ordered by their shells' addresses as the original's
/// set of pointers is.
static CHANGED_STORAGE_ARRAYS: GameState<BTreeMap<usize, ChangedArray>> =
    GameState::new(BTreeMap::new());

/// Check whether currently changes to the storage shall be persistent or
/// temporary till the next call to ClearChanges().
#[allow(
    unsafe_code,
    reason = "loads of the mode flags under the game-state mutex"
)]
fn are_changes_persistent() -> bool {
    // SAFETY: game state, under the mutex; no borrow is kept.
    unsafe { (*GAMELOOP.get() || *COMMAND.get()) && !*TESTMODE.get() }
}

/// Add the changed storage array to the list of changed arrays.
/// This is done so we only have to revert/save the changed
/// arrays, which saves quite a few clears, etc. after callbacks.
#[allow(
    unsafe_code,
    reason = "an insert into the changed set under the game-state mutex"
)]
fn add_changed_persistent_storage(storage: ChangedArray) {
    // SAFETY: game state, under the mutex; the borrow ends with the insert.
    unsafe {
        let shell = (*storage.array).shell as usize;
        (*CHANGED_STORAGE_ARRAYS.get()).insert(shell, storage);
    }
}

#[allow(
    unsafe_code,
    reason = "field-sized loads and stores of arrays the C++ shell aliases"
)]
impl<const SIZE: usize> PersistentStorageArray<SIZE> {
    /// The array a shell constructs.
    fn new(shell: *mut c_void, services: *const Services) -> Self {
        Self {
            base: BasePersistentStorageArray { shell, services },
            storage: [0; SIZE],
            prev_storage: None,
        }
    }

    /// Stores some value at a given position.
    /// If there is no backup of the data that backup is made and then
    /// we write the data.
    ///
    /// # Safety
    /// `this` is a live array.
    unsafe fn store_value(this: *mut Self, pos: u32, value: i32) {
        /* Out of the scope of the array */
        if pos as usize >= SIZE {
            return;
        }

        /* The value hasn't changed, so we pretend nothing happened.
         * Saves a few cycles and such and it's pretty easy to check. */
        // SAFETY: the caller's contract; `pos` is in range.
        if unsafe { (*this).storage[pos as usize] } == value {
            return;
        }

        /* We do not have made a backup; lets do so */
        if are_changes_persistent() {
            // SAFETY: the caller's contract.
            services_assert::assert_with(|| unsafe { (*this).prev_storage.is_none() });
        } else if unsafe { (*this).prev_storage.is_none() } {
            // SAFETY: the caller's contract; the copy ends before the store.
            unsafe { (*this).prev_storage = Some(Box::new((*this).storage)) };

            /* We only need to register ourselves when we made the backup
             * as that is the only time something will have changed */
            add_changed_persistent_storage(ChangedArray {
                array: this.cast(),
                clear_changes: Self::clear_changes_of,
            });
        }

        // SAFETY: the caller's contract; `pos` is in range.
        unsafe { (*this).storage[pos as usize] = value };
    }

    /// Gets the value from a given position.
    /// @return the data from that position
    ///
    /// # Safety
    /// `this` is a live array.
    unsafe fn get_value(this: *const Self, pos: u32) -> i32 {
        /* Out of the scope of the array */
        if pos as usize >= SIZE {
            return 0;
        }

        // SAFETY: the caller's contract; `pos` is in range.
        unsafe { (*this).storage[pos as usize] }
    }

    /// `ClearChanges`: discard temporary changes.
    ///
    /// # Safety
    /// `this` is a live array.
    unsafe fn clear_changes(this: *mut Self) {
        // SAFETY: the caller's contract.
        unsafe {
            if let Some(prev_storage) = (*this).prev_storage.take() {
                (*this).storage = *prev_storage;
            }
        }
    }

    /// [`Self::clear_changes`] of the array a changed-set entry names.
    unsafe fn clear_changes_of(array: *mut BasePersistentStorageArray) {
        // SAFETY: the entry was made by `store_value` of a live array of this size,
        // which its destructor erases before it frees the array.
        unsafe { Self::clear_changes(array.cast()) }
    }

    /// `~BasePersistentStorageArray`: remove references to use, then free the array.
    ///
    /// # Safety
    /// `this` came from `Box::into_raw` and is not used again.
    unsafe fn delete(this: *mut Self) {
        // SAFETY: the caller's contract; game state, under the mutex.
        unsafe {
            let shell = (*this).base.shell as usize;
            (*CHANGED_STORAGE_ARRAYS.get()).remove(&shell);
            drop(Box::from_raw(this));
        }
    }
}

/// `BasePersistentStorageArray::SwitchMode`: clear temporary changes made since the
/// last call to SwitchMode, and set whether subsequent changes shall be persistent or
/// temporary.
///
/// `mode` is the mode switch affecting temporary/persistent changes, and
/// `ignore_prev_mode` disables some sanity checks for exceptional call circumstances.
#[allow(
    unsafe_code,
    reason = "loads and stores of the mode flags and the changed set"
)]
pub fn switch_mode(mode: PersistentStorageMode, ignore_prev_mode: bool) {
    // SAFETY (each block): game state, under the mutex; each load or store ends at once.
    let gameloop = || unsafe { *GAMELOOP.get() };
    let command = || unsafe { *COMMAND.get() };
    let testmode = || unsafe { *TESTMODE.get() };
    match mode {
        PersistentStorageMode::PSM_ENTER_GAMELOOP => {
            services_assert::assert_with(|| ignore_prev_mode || !gameloop());
            services_assert::assert_with(|| !command() && !testmode());
            unsafe { *GAMELOOP.get() = true };
        }

        PersistentStorageMode::PSM_LEAVE_GAMELOOP => {
            services_assert::assert_with(|| ignore_prev_mode || gameloop());
            services_assert::assert_with(|| !command() && !testmode());
            unsafe { *GAMELOOP.get() = false };
        }

        PersistentStorageMode::PSM_ENTER_COMMAND => {
            services_assert::assert_with(|| (ignore_prev_mode || !command()) && !testmode());
            unsafe { *COMMAND.get() = true };
        }

        PersistentStorageMode::PSM_LEAVE_COMMAND => {
            services_assert::assert_with(|| ignore_prev_mode || command());
            unsafe { *COMMAND.get() = false };
        }

        PersistentStorageMode::PSM_ENTER_TESTMODE => {
            services_assert::assert_with(|| !command() && (ignore_prev_mode || !testmode()));
            unsafe { *TESTMODE.get() = true };
        }

        PersistentStorageMode::PSM_LEAVE_TESTMODE => {
            services_assert::assert_with(|| ignore_prev_mode || testmode());
            unsafe { *TESTMODE.get() = false };
        }

        _ => services_assert::not_reached(),
    }

    /* Discard all temporary changes */
    // The set is taken out, which leaves it empty as the original's `clear()` after
    // the loop does: neither `ClearChanges` nor `Debug` touches the set.
    // SAFETY: game state, under the mutex; the borrow ends with the take.
    let changed = std::mem::take(unsafe { &mut *CHANGED_STORAGE_ARRAYS.get() });
    for it in changed.into_values() {
        // SAFETY: each entry names a live array, which its destructor erases.
        let base = unsafe { &*it.array };
        // SAFETY: the array's storage table lives for the process.
        let services = unsafe { &*base.services };
        debug!(
            desync,
            2,
            "warning: discarding persistent storage changes: Feature {}, GrfID {:08X}, Tile {}",
            (services.feature)(base.shell),
            (services.grfid)(base.shell).swap_bytes(),
            (services.tile)(base.shell)
        );
        // SAFETY: as above.
        unsafe { (it.clear_changes)(it.array) };
    }
}

/// A `PersistentStorage`, as its array. A leaf that reads a storage member returns the
/// array; one that stores a storage takes the shell, which the handle reads from it.
#[derive(Clone, Copy)]
pub(crate) struct PersistentStorageRef(*mut PersistentStorageArray<256>);
#[allow(unsafe_code, reason = "field loads and leaf calls of a live storage")]
impl PersistentStorageRef {
    /// The storage of an array a leaf returns, or `None` for a null member.
    ///
    /// # Safety
    /// A non-null `array` is a live storage's array, which stays live while the
    /// handle is used.
    pub(crate) unsafe fn of(array: *mut PersistentStorageArray<256>) -> Option<Self> {
        (!array.is_null()).then_some(Self(array))
    }

    /// The storage of an array the original dereferences without a test, such as an
    /// element of a town's `psa_list`.
    ///
    /// # Safety
    /// `array` is a live storage's array, which stays live while the handle is used.
    pub(crate) unsafe fn of_live(array: *mut PersistentStorageArray<256>) -> Self {
        Self(array)
    }

    /// `new PersistentStorage(grfid, feature, tile)`.
    pub(crate) fn new(
        services: &Services,
        grfid: u32,
        feature: crate::newgrf::GrfSpecFeature,
        tile: crate::tile_type::TileIndex,
    ) -> Self {
        Self((services.new)(grfid, feature.0, tile.0))
    }

    /// `PersistentStorage::CanAllocateItem()`.
    pub(crate) fn can_allocate_item(services: &Services) -> bool {
        (services.can_allocate_item)()
    }

    /// The C++ `PersistentStorage`, as a `BasePersistentStorageArray`.
    pub(crate) fn shell(self) -> *mut c_void {
        // SAFETY: the handle's contract.
        unsafe { (*self.0).base.shell }
    }

    /// `grfid`.
    pub(crate) fn grfid(self) -> u32 {
        // SAFETY: the handle's contract; the storage table lives for the process.
        unsafe { ((*(*self.0).base.services).grfid)((*self.0).base.shell) }
    }

    /// `StoreValue(pos, value)`.
    pub(crate) fn store_value(self, pos: u32, value: i32) {
        // SAFETY: the handle's contract.
        unsafe { PersistentStorageArray::store_value(self.0, pos, value) }
    }

    /// `GetValue(pos)`.
    pub(crate) fn get_value(self, pos: u32) -> i32 {
        // SAFETY: the handle's contract.
        unsafe { PersistentStorageArray::get_value(self.0, pos) }
    }
}

/// Class for temporary storage of data.
/// On #ClearChanges that data is always zero-ed.
pub(crate) struct TemporaryStorageArray<const SIZE: usize> {
    /// Memory for the storage array
    storage: [i32; SIZE],
    /// Storage has been assigned, if this equals 'init_key'.
    init: [u16; SIZE],
    /// Magic key to 'init'.
    init_key: u16,
}
impl<const SIZE: usize> TemporaryStorageArray<SIZE> {
    /// An array with nothing assigned.
    pub(crate) const fn new() -> Self {
        Self {
            storage: [0; SIZE],
            init: [0; SIZE],
            init_key: 1,
        }
    }

    /// Stores some value at a given position.
    pub(crate) fn store_value(&mut self, pos: u32, value: i32) {
        /* Out of the scope of the array */
        if pos as usize >= SIZE {
            return;
        }

        self.storage[pos as usize] = value;
        self.init[pos as usize] = self.init_key;
    }

    /// Gets the value from a given position.
    /// @return the data from that position
    pub(crate) fn get_value(&self, pos: u32) -> i32 {
        /* Out of the scope of the array */
        if pos as usize >= SIZE {
            return 0;
        }

        if self.init[pos as usize] != self.init_key {
            /* Unassigned since last call to ClearChanges */
            return 0;
        }

        self.storage[pos as usize]
    }

    /// `ClearChanges`.
    pub(crate) fn clear_changes(&mut self) {
        /* Increment init_key to invalidate all storage */
        self.init_key = self.init_key.wrapping_add(1);
        if self.init_key == 0 {
            /* When init_key wraps around, we need to reset everything */
            self.init = [0; SIZE];
            self.init_key = 1;
        }
    }
}

macro_rules! storage_exports {
    ($size:literal, $new:ident, $delete:ident, $data:ident, $store:ident, $get:ident, $clear:ident) => {
        /// The array of a constructed `PersistentStorageArray` shell.
        #[allow(unsafe_code, reason = "export; the shell's constructor")]
        #[unsafe(no_mangle)]
        pub extern "C" fn $new(
            shell: *mut c_void,
            services: *const Services,
        ) -> *mut PersistentStorageArray<$size> {
            Box::into_raw(Box::new(PersistentStorageArray::new(shell, services)))
        }
        /// `~PersistentStorageArray`.
        ///
        /// # Safety
        /// The shell's array, which it does not use again.
        #[allow(unsafe_code, reason = "export; the shell's destructor")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $delete(array: *mut PersistentStorageArray<$size>) {
            // SAFETY: the caller's contract.
            unsafe { PersistentStorageArray::delete(array) }
        }
        /// The shell's `storage` reference.
        ///
        /// # Safety
        /// A live array.
        #[allow(unsafe_code, reason = "export; the address of a live array's storage")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $data(array: *mut PersistentStorageArray<$size>) -> *mut i32 {
            // SAFETY: the caller's contract.
            unsafe { (&raw mut (*array).storage).cast() }
        }
        /// `StoreValue`.
        ///
        /// # Safety
        /// A live array.
        #[allow(unsafe_code, reason = "export; the shell's StoreValue")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $store(
            array: *mut PersistentStorageArray<$size>,
            pos: u32,
            value: i32,
        ) {
            // SAFETY: the caller's contract.
            unsafe { PersistentStorageArray::store_value(array, pos, value) }
        }
        /// `GetValue`.
        ///
        /// # Safety
        /// A live array.
        #[allow(unsafe_code, reason = "export; the shell's GetValue")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $get(
            array: *const PersistentStorageArray<$size>,
            pos: u32,
        ) -> i32 {
            // SAFETY: the caller's contract.
            unsafe { PersistentStorageArray::get_value(array, pos) }
        }
        /// `ClearChanges`.
        ///
        /// # Safety
        /// A live array.
        #[allow(unsafe_code, reason = "export; the shell's ClearChanges")]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $clear(array: *mut PersistentStorageArray<$size>) {
            // SAFETY: the caller's contract.
            unsafe { PersistentStorageArray::clear_changes(array) }
        }
    };
}
storage_exports!(
    16,
    openttd_rust_storage16_new,
    openttd_rust_storage16_delete,
    openttd_rust_storage16_data,
    openttd_rust_storage16_store,
    openttd_rust_storage16_get,
    openttd_rust_storage16_clear
);
storage_exports!(
    256,
    openttd_rust_storage256_new,
    openttd_rust_storage256_delete,
    openttd_rust_storage256_data,
    openttd_rust_storage256_store,
    openttd_rust_storage256_get,
    openttd_rust_storage256_clear
);

/// `BasePersistentStorageArray::SwitchMode`.
#[allow(unsafe_code, reason = "export; the C++ SwitchMode")]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_storage_switch_mode(mode: u8, ignore_prev_mode: bool) {
    switch_mode(PersistentStorageMode(mode), ignore_prev_mode);
}

#[cfg(test)]
pub(crate) mod evidence_tests;

/// Native `PersistentStorageID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersistentStorageID(pub u32);

impl PersistentStorageID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(0xFF000);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(0xFFFFF);
}
