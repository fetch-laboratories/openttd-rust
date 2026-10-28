/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `NewGRF` payloads, the transactional registry and modes, and temporary registers. Statics
//! have no destructor, so they outlive native global storage and pool destruction.
#![allow(unsafe_code, clippy::missing_safety_doc, clippy::cast_sign_loss)]
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

type Check = extern "C" fn(bool);
/// Stable allocation owned by one native shell; aliases point at `storage`.
pub struct Payload<const N: usize> {
    storage: [i32; N],
    backup: Option<Box<[i32; N]>>,
    shell: *mut c_void,
}
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
unsafe fn clear<const N: usize>(owner: *mut c_void) {
    let owner = unsafe { &mut *owner.cast::<Payload<N>>() };
    if let Some(previous) = owner.backup.take() {
        owner.storage = *previous;
    }
}
unsafe fn store<const N: usize>(owner: *mut Payload<N>, pos: u32, value: i32, check: Check) {
    let pos = pos as usize;
    if pos >= N || unsafe { (*owner).storage[pos] } == value {
        return;
    }
    let persistent = unsafe {
        let r = &*OWNER.get();
        (r.gameloop || r.command) && !r.testmode
    };
    if persistent {
        check(unsafe { (*owner).backup.is_none() });
    } else if unsafe { (*owner).backup.is_none() } {
        let previous = Box::new(unsafe { (*owner).storage });
        unsafe {
            (*owner).backup = Some(previous);
            let shell = (*owner).shell;
            (*OWNER.get()).changed.insert(
                shell as usize,
                Changed {
                    owner: owner.cast(),
                    shell,
                    clear: clear::<N>,
                },
            );
        }
    }
    unsafe {
        (*owner).storage[pos] = value;
    }
}
macro_rules! payload_api {
    ($n:literal, $new:ident, $delete:ident, $data:ident, $store:ident, $get:ident, $clear:ident) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $new(shell: *mut c_void) -> *mut Payload<$n> {
            Box::into_raw(Box::new(Payload {
                storage: [0; $n],
                backup: None,
                shell,
            }))
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $delete(owner: *mut Payload<$n>) {
            unsafe {
                (*OWNER.get()).changed.remove(&((*owner).shell as usize));
                drop(Box::from_raw(owner));
            }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $data(owner: *mut Payload<$n>) -> *mut i32 {
            unsafe { std::ptr::addr_of_mut!((*owner).storage).cast() }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $store(
            owner: *mut Payload<$n>,
            pos: u32,
            value: i32,
            check: Check,
        ) {
            unsafe {
                store(owner, pos, value, check);
            }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $get(owner: *const Payload<$n>, pos: u32) -> i32 {
            if pos as usize >= $n {
                0
            } else {
                unsafe { (*owner).storage[pos as usize] }
            }
        }
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $clear(owner: *mut Payload<$n>) {
            unsafe {
                clear::<$n>(owner.cast());
            }
        }
    };
}
payload_api!(
    16,
    openttd_rust_storage16_new,
    openttd_rust_storage16_delete,
    openttd_rust_storage16_data,
    openttd_rust_storage16_store,
    openttd_rust_storage16_get,
    openttd_rust_storage16_clear
);
payload_api!(
    256,
    openttd_rust_storage256_new,
    openttd_rust_storage256_delete,
    openttd_rust_storage256_data,
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
pub struct StorageReference {
    owner: *mut Payload<256>,
    grfid: u32,
}
#[repr(C)]
pub struct IndustryStorage {
    owner: *mut Payload<256>,
    valid: bool,
}
#[repr(C)]
pub struct Services {
    check: Check,
    grfid: extern "C" fn(*const c_void) -> u32,
    industry: extern "C" fn(*mut c_void) -> IndustryStorage,
    create_industry: extern "C" fn(*mut c_void) -> *mut Payload<256>,
    pub(crate) airport: extern "C" fn(*mut c_void) -> *mut Payload<256>,
    create_airport: extern "C" fn(*mut c_void, u32) -> *mut Payload<256>,
    town_count: extern "C" fn(*const c_void) -> usize,
    town_storage: extern "C" fn(*const c_void, usize) -> StorageReference,
    create_town: extern "C" fn(*mut c_void, u32) -> *mut Payload<256>,
    append_town: extern "C" fn(*mut c_void, *mut c_void),
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
        let grfid = if grf.is_null() { 0 } else { (s.grfid)(grf) };
        owner = (s.create_airport)(station, grfid);
    }
    unsafe {
        store(owner, pos, value, s.check);
    }
}
fn town_find(s: &Services, town: *const c_void, grfid: u32) -> *mut Payload<256> {
    for index in 0..(s.town_count)(town) {
        let entry = (s.town_storage)(town, index);
        if entry.grfid == grfid {
            return entry.owner;
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
        grfid = (s.grfid)(grf);
    }
    if grfid != (s.grfid)(grf) {
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
    (s.append_town)(town, unsafe { (*owner).shell });
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
        grfid = (s.grfid)(grf);
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
        check, grfid, industry, create_industry, airport, create_airport,
        town_count, town_storage, create_town, append_town,
    };
    "OpenTTDStorageReference" ["storage"] => StorageReference { owner, grfid };
    "OpenTTDIndustryStorage" ["storage"] => IndustryStorage { owner, valid };
}
