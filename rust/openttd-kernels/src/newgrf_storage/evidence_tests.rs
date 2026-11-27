/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Store, rollback and array-ownership policy of the persistent arrays (#736 W), and the
//! temporary registers' key wrap.
//!
//! Expectations are read from `PersistentStorageArray::StoreValue`, `ClearChanges`,
//! `BasePersistentStorageArray::SwitchMode` and `TemporaryStorageArray` of the pinned
//! original. The shells below stand in for the C++ `PersistentStorage` records: the
//! storage table's member leaves read them, as the C++ leaves read the real records.
use super::*;
use crate::tile_type::TileIndex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A C++ `BasePersistentStorageArray`'s members, which the leaves read.
#[repr(C)]
struct Shell {
    grfid: u32,
    feature: u8,
    tile: TileIndex,
}
extern "C" fn unused_can_allocate_item() -> u8 {
    unreachable!("the policy test creates no pool item")
}
extern "C" fn unused_new(_: u32, _: u8, _: u32) -> *mut PersistentStorageArray<256> {
    unreachable!("the policy test creates no pool item")
}
#[allow(unsafe_code, reason = "a load of the test's live shell")]
extern "C" fn shell_grfid(shell: *const BasePersistentStorageArray) -> u32 {
    // SAFETY: every array's shell is a live `Shell`.
    unsafe { (*shell.cast::<Shell>()).grfid }
}
#[allow(unsafe_code, reason = "a load of the test's live shell")]
extern "C" fn shell_feature(shell: *const BasePersistentStorageArray) -> u8 {
    // SAFETY: as `shell_grfid`.
    unsafe { (*shell.cast::<Shell>()).feature }
}
#[allow(unsafe_code, reason = "a load of the test's live shell")]
extern "C" fn shell_tile(shell: *const BasePersistentStorageArray) -> u32 {
    // SAFETY: as `shell_grfid`.
    unsafe { (*shell.cast::<Shell>()).tile.0 }
}
static SERVICES: Services = Services {
    can_allocate_item: unused_can_allocate_item,
    new: unused_new,
    grfid: shell_grfid,
    feature: shell_feature,
    tile: shell_tile,
};

static FAILED_ASSERTIONS: AtomicUsize = AtomicUsize::new(0);
extern "C" fn record_assertion(condition: bool) {
    if !condition {
        FAILED_ASSERTIONS.fetch_add(1, Ordering::SeqCst);
    }
}

/// The discard messages `SwitchMode` wrote since the last call.
fn discarded() -> Vec<String> {
    crate::debug::tests::take()
        .into_iter()
        .map(|(category, level, message)| {
            assert_eq!((category.as_str(), level), ("desync", 2));
            String::from_utf8(message).unwrap()
        })
        .collect()
}
fn message(shell: &Shell) -> String {
    format!(
        "warning: discarding persistent storage changes: Feature {}, GrfID {:08X}, Tile {}",
        shell.feature,
        shell.grfid.swap_bytes(),
        shell.tile.0
    )
}

/// `PersistentStorageArray<int32_t, 256>` and its old 16-entry form across the six mode
/// switches: changes outside the game loop and command scope, and inside test mode, are
/// backed up once, registered once and rolled back at the next switch, which writes one
/// `Debug(desync, 2)` line for each changed array in its shell's address order; changes
/// inside the game loop or a command stay. A save-time `ClearChanges` rolls back but
/// leaves the array registered; deletion unregisters it. A failed `SwitchMode`
/// precondition is asserted through the native policy and the switch goes on.
#[test]
#[allow(
    unsafe_code,
    reason = "the arrays' exports over the test's live shells"
)]
fn store_rollback_and_ownership_policy() {
    const NAME: &str = "newgrf_storage::evidence_tests::store_rollback_and_ownership_policy";
    if crate::services_assert::isolate_test_check(NAME, record_assertion) {
        return;
    }
    discarded();
    // Two shells in one allocation, so their address order is known.
    let shells = [
        Shell {
            grfid: 0x1234_5678,
            feature: 10,
            tile: TileIndex(7),
        },
        Shell {
            grfid: 0x0102_0304,
            feature: 13,
            tile: TileIndex(9),
        },
    ];
    let shell = |index: usize| {
        (&raw const shells[index])
            .cast_mut()
            .cast::<BasePersistentStorageArray>()
    };
    let first = openttd_rust_storage256_new(shell(0), &raw const SERVICES);
    let old = openttd_rust_storage16_new(shell(1), &raw const SERVICES);
    // SAFETY: both arrays live until their deletion below.
    let (store, get, store16, get16) = unsafe {
        (
            |pos, value| openttd_rust_storage256_store(first, pos, value),
            |pos| openttd_rust_storage256_get(first, pos),
            |pos, value| openttd_rust_storage16_store(old, pos, value),
            |pos| openttd_rust_storage16_get(old, pos),
        )
    };
    switch_mode(PersistentStorageMode::PSM_LEAVE_GAMELOOP, true);
    assert_eq!(discarded(), [] as [String; 0]);

    // Outside the game loop: temporary. Out-of-range and unchanged values store nothing.
    store(256, 99);
    store(0, 0);
    switch_mode(PersistentStorageMode::PSM_LEAVE_GAMELOOP, true);
    assert_eq!(discarded(), [] as [String; 0]);
    store(15, -8);
    store(15, -9);
    store16(15, 4);
    store16(16, 5);
    assert_eq!(
        (get(15), get(256), get(u32::MAX), get16(15), get16(16)),
        (-9, 0, 0, 4, 0)
    );
    switch_mode(PersistentStorageMode::PSM_ENTER_GAMELOOP, false);
    assert_eq!(discarded(), [message(&shells[0]), message(&shells[1])]);
    assert_eq!((get(15), get16(15)), (0, 0));

    // Inside the game loop and a command: persistent, with nothing registered.
    store(255, i32::MIN);
    switch_mode(PersistentStorageMode::PSM_ENTER_COMMAND, false);
    store(1, i32::MAX);
    switch_mode(PersistentStorageMode::PSM_LEAVE_COMMAND, false);
    assert_eq!(discarded(), [] as [String; 0]);
    // Test mode: temporary again, and rolled back on leaving it.
    switch_mode(PersistentStorageMode::PSM_ENTER_TESTMODE, false);
    store(255, 7);
    store(255, 8);
    assert_eq!(get(255), 8);
    switch_mode(PersistentStorageMode::PSM_LEAVE_TESTMODE, false);
    assert_eq!(discarded(), [message(&shells[0])]);
    assert_eq!((get(255), get(1)), (i32::MIN, i32::MAX));
    switch_mode(PersistentStorageMode::PSM_LEAVE_GAMELOOP, false);

    // The saver's `ClearChanges` rolls back and keeps the array in the set.
    store(1, -3);
    // SAFETY: the array is live.
    unsafe { openttd_rust_storage256_clear(first) };
    assert_eq!(get(1), i32::MAX);
    switch_mode(PersistentStorageMode::PSM_LEAVE_GAMELOOP, true);
    assert_eq!(discarded(), [message(&shells[0])]);
    assert_eq!(get(1), i32::MAX);

    // A deleted array leaves the set.
    store16(0, 1);
    // SAFETY: the array is not used again.
    unsafe { openttd_rust_storage16_delete(old) };
    switch_mode(PersistentStorageMode::PSM_LEAVE_GAMELOOP, true);
    assert_eq!(discarded(), [] as [String; 0]);
    assert_eq!(FAILED_ASSERTIONS.load(Ordering::SeqCst), 0);

    // `assert(ignore_prev_mode || command)` fails outside a command; the flag is still written.
    switch_mode(PersistentStorageMode::PSM_LEAVE_COMMAND, false);
    assert_eq!(FAILED_ASSERTIONS.load(Ordering::SeqCst), 1);
    switch_mode(PersistentStorageMode::PSM_ENTER_COMMAND, false);
    store(2, 3);
    switch_mode(PersistentStorageMode::PSM_LEAVE_COMMAND, false);
    assert_eq!(discarded(), [] as [String; 0]);
    assert_eq!((get(2), FAILED_ASSERTIONS.load(Ordering::SeqCst)), (3, 1));
    // SAFETY: the array is not used again.
    unsafe { openttd_rust_storage256_delete(first) };
}

/// `TemporaryStorageArray`: a value reads back until the next `ClearChanges`, and after
/// 65,535 of them, when the key wraps to zero and every slot is reset, a value stored
/// before reads zero; one stored after reads back. A position past the array reads zero.
#[test]
fn temporary_storage_key_wraps() {
    let mut temporary = TemporaryStorageArray::<0x110>::new();
    temporary.store_value(0x10F, -123);
    assert_eq!(temporary.get_value(0x10F), -123);
    for _ in 0..65535 {
        temporary.clear_changes();
    }
    assert_eq!(temporary.get_value(0x10F), 0);
    temporary.store_value(0x10F, 456);
    assert_eq!(temporary.get_value(0x10F), 456);
    // The slot's old key is 1, which the key reaches again after the wrap: a stale
    // slot that the wrap did not reset would read back here.
    temporary.store_value(3, 9);
    for _ in 0..65535 {
        temporary.clear_changes();
    }
    temporary.store_value(0x110, 1);
    assert_eq!((temporary.get_value(3), temporary.get_value(0x110)), (0, 0));
    temporary.clear_changes();
    assert_eq!(temporary.get_value(0x10F), 0);
}

/// Stand-in `PersistentStorage` records for the tests of the `StorePSA` bodies, which
/// create storages through the storage table and store them in their records.
pub(crate) mod fixture {
    use super::{
        BasePersistentStorageArray, PersistentStorageArray, Services, Shell,
        openttd_rust_storage256_new, shell_feature, shell_grfid, shell_tile,
    };
    use crate::tile_type::TileIndex;
    use std::sync::Mutex;

    /// A `PersistentStorage`: its members, then the array it owns.
    #[repr(C)]
    struct Record {
        shell: Shell,
        array: *mut PersistentStorageArray<256>,
    }
    /// The members of each storage `new` created, in a process each test of the bodies
    /// has to itself.
    static CREATED: Mutex<Vec<(u32, u8, u32)>> = Mutex::new(Vec::new());
    extern "C" fn can_allocate_item() -> u8 {
        1
    }
    extern "C" fn new(grfid: u32, feature: u8, tile: u32) -> *mut PersistentStorageArray<256> {
        CREATED.lock().unwrap().push((grfid, feature, tile));
        storage(grfid, feature, TileIndex(tile))
    }
    /// The storage table over the stand-in records.
    pub(crate) static STORAGE: Services = Services {
        can_allocate_item,
        new,
        grfid: shell_grfid,
        feature: shell_feature,
        tile: shell_tile,
    };
    /// A storage with these members, which lives for the process, as its array.
    pub(crate) fn storage(
        grfid: u32,
        feature: u8,
        tile: TileIndex,
    ) -> *mut PersistentStorageArray<256> {
        let record = Box::leak(Box::new(Record {
            shell: Shell {
                grfid,
                feature,
                tile,
            },
            array: std::ptr::null_mut(),
        }));
        record.array = openttd_rust_storage256_new((&raw mut *record).cast(), &raw const STORAGE);
        record.array
    }
    /// The members of each storage the table's `new` created since the last call.
    pub(crate) fn created() -> Vec<(u32, u8, u32)> {
        std::mem::take(&mut *CREATED.lock().unwrap())
    }
    /// The array of a storage this fixture made, from its shell.
    #[allow(unsafe_code, reason = "a load of a live stand-in record")]
    pub(crate) fn array_of(
        shell: *mut BasePersistentStorageArray,
    ) -> *mut PersistentStorageArray<256> {
        // SAFETY: every shell this fixture hands out is a leaked `Record`.
        unsafe { (*shell.cast::<Record>()).array }
    }
}
