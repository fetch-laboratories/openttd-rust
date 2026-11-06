/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NewGRF` payloads, the transactional registry and modes, and temporary registers. Statics
//! have no destructor, so they outlive native global storage and pool destruction.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_sign_loss
)]
use crate::game_state::GameState;
use std::{
    collections::BTreeMap,
    ffi::c_void,
    ops::Bound::{Excluded, Unbounded},
};

/// Native `PSM_ENTER_GAMELOOP`.
pub const PSM_ENTER_GAMELOOP: u8 = 0;
/// Native `PSM_LEAVE_GAMELOOP`.
pub const PSM_LEAVE_GAMELOOP: u8 = 1;
/// Native `PSM_ENTER_COMMAND`.
pub const PSM_ENTER_COMMAND: u8 = 2;
/// Native `PSM_LEAVE_COMMAND`.
pub const PSM_LEAVE_COMMAND: u8 = 3;
/// Native `PSM_ENTER_TESTMODE`.
pub const PSM_ENTER_TESTMODE: u8 = 4;
/// Native `PSM_LEAVE_TESTMODE`.
pub const PSM_LEAVE_TESTMODE: u8 = 5;

type Check = extern "C" fn(bool);
/// Native pool index; legacy arrays never publish one.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PersistentStorageID(pub u32);
impl PersistentStorageID {
    pub const INVALID: Self = Self(0x000F_FFFF);
}
impl crate::pool::PoolId for PersistentStorageID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
#[repr(C)]
pub struct PersistentStorageShell {
    _private: [u8; 0],
}
crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDPersistentStorageMetadata" ["storage"] => pub struct Metadata {
        pub grfid: u32,
        pub feature: u8 => "GrfSpecFeature",
        pub tile: crate::tile_type::TileIndex => "TileIndex",
    }
    "OpenTTDLegacyPersistentStorageRecord" ["storage"] => pub struct LegacyRecord {
        pub metadata: Metadata => "OpenTTDPersistentStorageMetadata",
        pub storage: [i32; 16] => "std::array<int32_t, 16>",
    }
    "OpenTTDPersistentStorageRecord" ["storage"] => pub struct PersistentStorageRecord {
        pub metadata: Metadata => "OpenTTDPersistentStorageMetadata",
        pub storage: [i32; 256] => "std::array<int32_t, 256>",
        pub shell: *mut PersistentStorageShell => "PersistentStorage *",
        pub index: PersistentStorageID => "PersistentStorageID",
    }
}
impl Metadata {
    fn new() -> Self {
        Self {
            grfid: 0,
            feature: 0xFF,
            tile: crate::tile_type::TileIndex::INVALID,
        }
    }
}
/// The two original fixed-array sizes share private transaction policy.
pub trait FixedData<const N: usize> {
    fn new() -> Self;
    fn metadata(&mut self) -> *mut Metadata;
    fn storage(&self) -> &[i32; N];
    fn storage_mut(&mut self) -> &mut [i32; N];
}
impl FixedData<16> for LegacyRecord {
    fn new() -> Self {
        Self {
            metadata: Metadata::new(),
            storage: [0; 16],
        }
    }
    fn metadata(&mut self) -> *mut Metadata {
        &raw mut self.metadata
    }
    fn storage(&self) -> &[i32; 16] {
        &self.storage
    }
    fn storage_mut(&mut self) -> &mut [i32; 16] {
        &mut self.storage
    }
}
impl FixedData<256> for PersistentStorageRecord {
    fn new() -> Self {
        Self {
            metadata: Metadata::new(),
            storage: [0; 256],
            shell: std::ptr::null_mut(),
            index: PersistentStorageID::INVALID,
        }
    }
    fn metadata(&mut self) -> *mut Metadata {
        &raw mut self.metadata
    }
    fn storage(&self) -> &[i32; 256] {
        &self.storage
    }
    fn storage_mut(&mut self) -> &mut [i32; 256] {
        &mut self.storage
    }
}
/// One allocation. Generated fixed data is the repr(C) prefix; backup and the
/// actual native `BasePersistentStorageArray` registry identity remain private.
#[repr(C)]
pub struct Payload<R: FixedData<N>, const N: usize> {
    record: R,
    backup: Option<Box<[i32; N]>>,
    registry_shell: *mut c_void,
}
pub type Payload16 = Payload<LegacyRecord, 16>;
pub type Payload256 = Payload<PersistentStorageRecord, 256>;
// Native inline views and record-to-owner resolution use these guaranteed repr(C) prefixes.
const _: () = assert!(std::mem::offset_of!(Payload16, record) == 0);
const _: () = assert!(std::mem::offset_of!(Payload256, record) == 0);
const _: () = assert!(std::mem::offset_of!(LegacyRecord, metadata) == 0);
const _: () = assert!(std::mem::offset_of!(PersistentStorageRecord, metadata) == 0);
#[derive(Clone, Copy)]
struct Changed {
    owner: *mut c_void,
    shell: *mut c_void,
    clear: unsafe fn(*mut c_void),
}
struct Runtime {
    gameloop: bool,
    command: bool,
    testmode: bool,
    changed: BTreeMap<usize, Changed>,
    temporary: [i32; 0x110],
    initialized: [u16; 0x110],
    key: u16,
}
static OWNER: GameState<Runtime> = GameState::new(Runtime {
    gameloop: false,
    command: false,
    testmode: false,
    changed: BTreeMap::new(),
    temporary: [0; 0x110],
    initialized: [0; 0x110],
    key: 1,
});
unsafe fn clear<R: FixedData<N>, const N: usize>(owner: *mut c_void) {
    let owner = unsafe { &mut *owner.cast::<Payload<R, N>>() };
    if let Some(previous) = owner.backup.take() {
        *owner.record.storage_mut() = *previous;
    }
}
unsafe fn store<R: FixedData<N>, const N: usize>(
    owner: *mut Payload<R, N>,
    pos: u32,
    value: i32,
    check: Check,
) {
    let pos = pos as usize;
    if pos >= N || unsafe { (*owner).record.storage()[pos] } == value {
        return;
    }
    let persistent = unsafe {
        let r = &*OWNER.get();
        (r.gameloop || r.command) && !r.testmode
    };
    if persistent {
        check(unsafe { (*owner).backup.is_none() });
    } else if unsafe { (*owner).backup.is_none() } {
        let previous = Box::new(unsafe { *(*owner).record.storage() });
        unsafe {
            (*owner).backup = Some(previous);
            let shell = (*owner).registry_shell;
            (*OWNER.get()).changed.insert(
                shell as usize,
                Changed {
                    owner: owner.cast(),
                    shell,
                    clear: clear::<R, N>,
                },
            );
        }
    }
    unsafe {
        (*owner).record.storage_mut()[pos] = value;
    }
}
macro_rules! payload_api {
    ($n:literal, $record:ty, $payload:ty, $new:ident, $delete:ident, $store:ident, $get:ident, $clear:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $new(
            shell: *mut c_void,
            metadata: *mut *mut Metadata,
        ) -> *mut $payload {
            let mut owner = Box::new(Payload {
                record: <$record>::new(),
                backup: None,
                registry_shell: shell,
            });
            unsafe {
                metadata.write(owner.record.metadata());
            }
            Box::into_raw(owner)
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $delete(owner: *mut $payload) {
            unsafe {
                (*OWNER.get())
                    .changed
                    .remove(&((*owner).registry_shell as usize));
                drop(Box::from_raw(owner));
            }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $store(owner: *mut $payload, pos: u32, value: i32, check: Check) {
            unsafe {
                store(owner, pos, value, check);
            }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $get(owner: *const $payload, pos: u32) -> i32 {
            if pos as usize >= $n {
                0
            } else {
                unsafe { (*owner).record.storage()[pos as usize] }
            }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $clear(owner: *mut $payload) {
            unsafe {
                clear::<$record, $n>(owner.cast());
            }
        }
    };
}
payload_api!(
    16,
    LegacyRecord,
    Payload16,
    openttd_rust_storage16_new,
    openttd_rust_storage16_delete,
    openttd_rust_storage16_store,
    openttd_rust_storage16_get,
    openttd_rust_storage16_clear
);
payload_api!(
    256,
    PersistentStorageRecord,
    Payload256,
    openttd_rust_storage256_new,
    openttd_rust_storage256_delete,
    openttd_rust_storage256_store,
    openttd_rust_storage256_get,
    openttd_rust_storage256_clear
);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_storage_switch_mode(
    mode: u8,
    ignore: bool,
    check: Check,
    discard: extern "C" fn(*mut c_void),
) {
    let (gameloop, command, testmode) = unsafe {
        let r = &*OWNER.get();
        (r.gameloop, r.command, r.testmode)
    };
    match mode {
        0 => {
            check(ignore || !gameloop);
            check(!command && !testmode);
            unsafe {
                (*OWNER.get()).gameloop = true;
            }
        }
        1 => {
            check(ignore || gameloop);
            check(!command && !testmode);
            unsafe {
                (*OWNER.get()).gameloop = false;
            }
        }
        2 => {
            check((ignore || !command) && !testmode);
            unsafe {
                (*OWNER.get()).command = true;
            }
        }
        3 => {
            check(ignore || command);
            unsafe {
                (*OWNER.get()).command = false;
            }
        }
        4 => {
            check(!command && (ignore || !testmode));
            unsafe {
                (*OWNER.get()).testmode = true;
            }
        }
        5 => {
            check(ignore || testmode);
            unsafe {
                (*OWNER.get()).testmode = false;
            }
        }
        _ => unreachable!(),
    }
    // Like the native pointer-ordered set: log, restore each array, then erase
    // the registry. In particular save-time clear does not erase an entry.
    let mut after = None;
    loop {
        let next = unsafe {
            let changed = &(*OWNER.get()).changed;
            after
                .map_or_else(
                    || changed.first_key_value(),
                    |key| changed.range((Excluded(key), Unbounded)).next(),
                )
                .map(|(&key, &entry)| (key, entry))
        };
        let Some((key, entry)) = next else {
            break;
        };
        discard(entry.shell);
        unsafe {
            (entry.clear)(entry.owner);
        }
        after = Some(key);
    }
    unsafe {
        (*OWNER.get()).changed.clear();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_storage_temp_store(pos: u32, value: i32) {
    let r = unsafe { &mut *OWNER.get() };
    if (pos as usize) < r.temporary.len() {
        r.temporary[pos as usize] = value;
        r.initialized[pos as usize] = r.key;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_storage_temp_get(pos: u32) -> i32 {
    let r = unsafe { &*OWNER.get() };
    if (pos as usize) < r.temporary.len() && r.initialized[pos as usize] == r.key {
        r.temporary[pos as usize]
    } else {
        0
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_storage_temp_clear() {
    let r = unsafe { &mut *OWNER.get() };
    r.key = r.key.wrapping_add(1);
    if r.key == 0 {
        r.initialized = [0; 0x110];
        r.key = 1;
    }
}

#[repr(C)]
pub struct IndustryStorage {
    owner: *mut Payload256,
    valid: bool,
}
#[repr(C)]
pub struct Services {
    shared: *const crate::services::Services,
    check: Check,
    create: extern "C" fn(u32, u8, u32) -> *mut PersistentStorageShell,
    destroy: extern "C" fn(*mut PersistentStorageShell),
    record: extern "C" fn(*const PersistentStorageShell) -> *mut PersistentStorageRecord,
    industry: extern "C" fn(*mut c_void) -> IndustryStorage,
    create_industry: extern "C" fn(*mut c_void) -> *mut Payload256,
    pub(crate) airport: extern "C" fn(*mut c_void) -> *mut Payload256,
    create_airport: extern "C" fn(*mut c_void, u32) -> *mut Payload256,
    town_count: extern "C" fn(*const c_void) -> usize,
    town_storage: extern "C" fn(*const c_void, usize) -> *mut PersistentStorageRecord,
    create_town: extern "C" fn(*mut c_void, u32) -> *mut Payload256,
    append_town: extern "C" fn(*mut c_void, *mut c_void),
}
impl Services {
    fn shared(&self) -> &crate::services::Services {
        // SAFETY: `GetRustStorageServices` points at the process-lifetime shared table.
        unsafe { &*self.shared }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_storage_industry(
    s: &Services,
    industry: *mut c_void,
    pos: u32,
    value: i32,
) {
    let input = (s.industry)(industry);
    if !input.valid {
        return;
    }
    let mut owner = input.owner;
    if owner.is_null() {
        if value == 0 {
            return;
        }
        owner = (s.create_industry)(industry);
    }
    unsafe {
        store(owner, pos, value, s.check);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_storage_airport(
    s: &Services,
    station: *mut c_void,
    grf: *const c_void,
    pos: u32,
    value: i32,
) {
    if station.is_null() {
        return;
    }
    let mut owner = (s.airport)(station);
    if owner.is_null() {
        if value == 0 {
            return;
        }
        let grfid = if grf.is_null() {
            0
        } else {
            (s.shared().grf_file_id)(grf)
        };
        owner = (s.create_airport)(station, grfid);
    }
    unsafe {
        store(owner, pos, value, s.check);
    }
}
fn town_find(s: &Services, town: *const c_void, grfid: u32) -> *mut Payload256 {
    for index in 0..(s.town_count)(town) {
        let entry = (s.town_storage)(town, index);
        // Serialized canonical owner resolution; no record borrow crosses the next call.
        if unsafe { (*entry).metadata.grfid } == grfid {
            // Payload is repr(C), with this generated record as its first field.
            return entry.cast();
        }
    }
    std::ptr::null_mut()
}
pub unsafe extern "C" fn openttd_rust_storage_town(
    s: &Services,
    town: *mut c_void,
    readonly: bool,
    grf: *const c_void,
    pos: u32,
    value: i32,
) {
    if readonly {
        return;
    }
    (s.check)(!town.is_null());
    if grf.is_null() {
        return;
    }
    let mut grfid = unsafe { openttd_rust_storage_temp_get(0x100) } as u32;
    if grfid == u32::MAX {
        grfid = (s.shared().grf_file_id)(grf);
    }
    if grfid != (s.shared().grf_file_id)(grf) {
        return;
    }
    let owner = town_find(s, town, grfid);
    if !owner.is_null() {
        unsafe {
            store(owner, pos, value, s.check);
        }
        return;
    }
    let owner = (s.create_town)(town, grfid);
    unsafe {
        store(owner, pos, value, s.check);
    }
    (s.append_town)(town, unsafe { (*owner).registry_shell });
}
pub unsafe extern "C" fn openttd_rust_storage_town_get(
    s: &Services,
    town: *const c_void,
    grf: *const c_void,
    pos: u32,
) -> u32 {
    let mut grfid = unsafe { openttd_rust_storage_temp_get(0x100) } as u32;
    if grfid == u32::MAX {
        if grf.is_null() {
            return 0;
        }
        grfid = (s.shared().grf_file_id)(grf);
    }
    let owner = town_find(s, town, grfid);
    if owner.is_null() {
        0
    } else {
        (unsafe { openttd_rust_storage256_get(owner, pos) }) as u32
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStorageServices" ["storage"] => Services {
        shared, check, create, destroy, record, industry, create_industry, airport, create_airport,
        town_count, town_storage, create_town, append_town,
    };
    "OpenTTDIndustryStorage" ["storage"] => IndustryStorage { owner, valid };
}
