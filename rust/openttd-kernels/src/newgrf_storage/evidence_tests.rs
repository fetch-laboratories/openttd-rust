/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Store, rollback and array-ownership policy of the persistent arrays (#736 W).
//!
//! Moved from `tools/simulation/storage-console.inc`, whose literal expectations passed
//! on the unchanged original (the reference role runs that fixture), and read against
//! `PersistentStorageArray::StoreValue`, `IndustriesScopeResolver::StorePSA`,
//! `AirportScopeResolver::StorePSA` and `TownScopeResolver::StorePSA` of the pinned
//! original. The leaves below stand in for the native records and pool: they prove the
//! Rust decisions and the order of its leaf calls, not `PersistentStorage`'s pool
//! behaviour, which the console keeps.
use super::*;
use crate::test_tables::SHARED;
use std::{
    ptr::{addr_of_mut, null, null_mut},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

type Owner = *mut Payload<256>;

/// One town's, one industry's and one airport's native record.
struct World {
    industry_valid: bool,
    industry: Owner,
    airport: Owner,
    town: Vec<(Owner, u32)>,
    pending_town: Option<(Owner, u32)>,
    industries_created: u32,
    airport_grfids: Vec<u32>,
}

static FAILED_CHECKS: AtomicUsize = AtomicUsize::new(0);
static DISCARDED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

extern "C" fn check(condition: bool) {
    if !condition {
        FAILED_CHECKS.fetch_add(1, Ordering::SeqCst);
    }
}
extern "C" fn discard(shell: *mut c_void) {
    DISCARDED.lock().unwrap().push(shell as usize);
}
fn new_owner() -> Owner {
    openttd_rust_storage256_new(Box::into_raw(Box::new(0_u8)).cast())
}
unsafe fn world<'a>(context: *const c_void) -> &'a mut World {
    unsafe { &mut *context.cast_mut().cast::<World>() }
}
extern "C" fn industry(context: *mut c_void) -> IndustryStorage {
    let w = unsafe { world(context) };
    IndustryStorage {
        owner: w.industry,
        valid: w.industry_valid,
    }
}
extern "C" fn create_industry(context: *mut c_void) -> Owner {
    let w = unsafe { world(context) };
    w.industries_created += 1;
    w.industry = new_owner();
    w.industry
}
extern "C" fn airport(context: *mut c_void) -> Owner {
    unsafe { world(context) }.airport
}
extern "C" fn create_airport(context: *mut c_void, grfid: u32) -> Owner {
    let w = unsafe { world(context) };
    w.airport_grfids.push(grfid);
    w.airport = new_owner();
    w.airport
}
extern "C" fn town_count(context: *const c_void) -> usize {
    unsafe { world(context) }.town.len()
}
extern "C" fn town_storage(context: *const c_void, index: usize) -> StorageReference {
    let (owner, grfid) = unsafe { world(context) }.town[index];
    StorageReference { owner, grfid }
}
extern "C" fn create_town(context: *mut c_void, grfid: u32) -> Owner {
    let w = unsafe { world(context) };
    let owner = new_owner();
    w.pending_town = Some((owner, grfid));
    owner
}
extern "C" fn append_town(context: *mut c_void, shell: *mut c_void) {
    let w = unsafe { world(context) };
    let (owner, grfid) = w.pending_town.take().expect("created before appended");
    assert_eq!(unsafe { (*owner).shell }, shell);
    w.town.push((owner, grfid));
}
extern "C" fn grf_id(grf: *const c_void) -> u32 {
    unsafe { grf.cast::<u32>().read() }
}

fn switch(mode: u8, ignore: bool) {
    unsafe { openttd_rust_storage_switch_mode(mode, ignore, check, discard) };
}
fn get(owner: Owner, pos: u32) -> i32 {
    unsafe { openttd_rust_storage256_get(owner, pos) }
}
fn register(value: i32) {
    unsafe { openttd_rust_storage_temp_store(0x100, value) };
}

impl World {
    fn new() -> Self {
        Self {
            industry_valid: true,
            industry: null_mut(),
            airport: null_mut(),
            town: Vec::new(),
            pending_town: None,
            industries_created: 0,
            airport_grfids: Vec::new(),
        }
    }
}
fn table(shared: *const crate::services::Services) -> Services {
    Services {
        shared,
        check,
        industry,
        create_industry,
        airport,
        create_airport,
        town_count,
        town_storage,
        create_town,
        append_town,
    }
}

#[test]
fn store_rollback_and_ownership_policy() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let mut shared = SHARED;
    shared.grf_file_id = grf_id;
    let table = table(&raw const shared);
    let s = &table;
    let mut w = World::new();
    let scope: *mut c_void = addr_of_mut!(w).cast();
    let (own, other) = (0x3931_2345_u32, 0x3935_4321_u32);
    let (grf, other_grf) = (
        (&raw const own).cast::<c_void>(),
        (&raw const other).cast::<c_void>(),
    );
    let town = |grf, readonly, pos, value| unsafe {
        openttd_rust_storage_town(s, scope, readonly, grf, pos, value);
    };
    let town_get =
        |grf, pos| unsafe { openttd_rust_storage_town_get(s, scope.cast_const(), grf, pos) };
    let ind = |pos, value| unsafe { openttd_rust_storage_industry(s, scope, pos, value) };
    let air = |station, grf, pos, value| unsafe {
        openttd_rust_storage_airport(s, station, grf, pos, value);
    };
    FAILED_CHECKS.store(0, Ordering::SeqCst);
    DISCARDED.lock().unwrap().clear();

    switch(PSM_LEAVE_GAMELOOP, true);
    switch(PSM_ENTER_COMMAND, false);

    // Null station, read-only town, absent file and another file's register write nothing.
    air(null_mut(), grf, 0, 7);
    town(grf, true, 0, 7);
    town(null(), false, 0, 7);
    register(123);
    town(grf, false, 0, 7);
    // Zero never creates an industry or airport array.
    ind(0, 0);
    air(scope, grf, 0, 0);
    // An invalid-ID industry writes nothing.
    let w_ptr = scope.cast::<World>();
    unsafe { (*w_ptr).industry_valid = false };
    ind(0, 7);
    unsafe { (*w_ptr).industry_valid = true };
    unsafe {
        assert_eq!((&*w_ptr).town.len(), 0);
        assert!((*w_ptr).industry.is_null() && (*w_ptr).airport.is_null());
    }

    // An airport without a file creates the default-file array.
    air(scope, null(), 256, 7);
    unsafe {
        assert_eq!((&*w_ptr).airport_grfids, [0]);
        openttd_rust_storage256_delete((*w_ptr).airport);
        (*w_ptr).airport = null_mut();
    }

    // Nonzero out-of-range writes allocate industry and airport; town allocates even for zero.
    register(-1);
    town(grf, false, 256, 0);
    ind(256, 7);
    air(scope, grf, 256, 7);
    unsafe {
        assert_eq!((&*w_ptr).town.len(), 1);
        assert_eq!((*w_ptr).industries_created, 1);
        assert_eq!((&*w_ptr).airport_grfids, [0, own]);
    }
    town(grf, false, 0, i32::MIN);
    town(grf, false, 255, i32::MAX);
    ind(15, -393);
    air(scope, grf, 255, 393);
    let (town_owner, industry_owner, airport_owner) =
        unsafe { ((&(*w_ptr).town)[0].0, (*w_ptr).industry, (*w_ptr).airport) };
    assert_eq!(get(industry_owner, 0), 0);
    assert_eq!(get(industry_owner, 15), -393);
    assert_eq!(get(industry_owner, 256), 0);
    assert_eq!(get(industry_owner, u32::MAX), 0);

    // An invalid-ID scope reads the existing array but cannot write it.
    unsafe { (*w_ptr).industry_valid = false };
    ind(15, 7);
    assert_eq!(get(industry_owner, 15), -393);
    unsafe { (*w_ptr).industry_valid = true };

    // Another file gets a distinct array; explicit GRFID reads may cross files.
    town(other_grf, false, 0, 99);
    unsafe { assert_eq!((&*w_ptr).town.len(), 2) };
    register(other.cast_signed());
    town(grf, false, 0, 77);
    assert_eq!(town_get(grf, 0), 99);
    register(own.cast_signed());
    assert_eq!(town_get(null(), 0), 0x8000_0000);
    register(-1);
    assert_eq!(town_get(null(), 0), 0);
    assert_eq!(town_get(grf, 0), 0x8000_0000);

    // Inside command test mode changes are temporary and roll back on leaving.
    switch(PSM_LEAVE_COMMAND, false);
    switch(PSM_ENTER_TESTMODE, false);
    town(grf, false, 0, 1);
    ind(15, 2);
    air(scope, grf, 255, 3);
    assert_eq!(get(industry_owner, 15), 2);
    assert!(DISCARDED.lock().unwrap().is_empty());
    switch(PSM_LEAVE_TESTMODE, false);
    assert_eq!(get(town_owner, 0), i32::MIN);
    assert_eq!(get(industry_owner, 15), -393);
    assert_eq!(get(airport_owner, 255), 393);

    // The registry is visited in pointer order, once per changed array.
    let mut visited = std::mem::take(&mut *DISCARDED.lock().unwrap());
    assert_eq!(visited.len(), 3);
    assert!(visited.is_sorted());
    visited.sort_unstable();
    visited.dedup();
    assert_eq!(visited.len(), 3);
    assert_eq!(FAILED_CHECKS.load(Ordering::SeqCst), 0);

    release(w_ptr, industry_owner, airport_owner);
}

fn release(world: *mut World, industry: Owner, airport: Owner) {
    unsafe {
        for (owner, _) in std::mem::take(&mut *addr_of_mut!((*world).town)) {
            openttd_rust_storage256_delete(owner);
        }
        openttd_rust_storage256_delete(industry);
        openttd_rust_storage256_delete(airport);
    }
}
