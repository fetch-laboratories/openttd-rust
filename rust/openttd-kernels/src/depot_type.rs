/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `depot_type.h`.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DepotID(pub u16);
/// Opaque native `Depot`; valid only while its native pool shell lives.
#[repr(C)]
pub struct Depot {
    _private: [u8; 0],
}
pub const MAX_LENGTH_DEPOT_NAME_CHARS: usize = 32;

use crate::game_state::GameState;
use crate::pool::{PoolCore, PoolDescriptor, PoolId, TypedPool};
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDDepotRecord" ["depot_type"] => pub struct DepotRecord {
        pub shell: *mut Depot => "Depot *",
        pub index: DepotID => "DepotID",
        pub town_cn: u16,
        pub xy: crate::tile_type::TileIndex => "TileIndex",
        pub town: *mut crate::town_type::TownHandle => "Town *",
        pub name: crate::industrytype::NativeVec<u8> => "RustVector<uint8_t>",
        pub build_date: i32 => "TimerGameCalendar::Date",
    }
}

impl PoolId for DepotID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
static POOL: GameState<*mut PoolCore> = GameState::new(null_mut());
/// Bind the actual native pool at startup, before allocation or command dispatch.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_depot_pool_bind(pool: *mut PoolCore) -> *mut PoolCore {
    unsafe { std::ptr::replace(POOL.get(), pool) }
}
pub(crate) unsafe fn pool() -> TypedPool<DepotID, DepotRecord, Depot> {
    unsafe { TypedPool::from_descriptor(PoolCore::descriptor(*POOL.get())) }
}
pub(crate) unsafe fn descriptor() -> *const PoolDescriptor {
    unsafe { PoolCore::descriptor(*POOL.get()) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_depot_record_new(
    shell: *mut Depot,
    index: DepotID,
) -> *mut DepotRecord {
    Box::into_raw(Box::new(DepotRecord {
        shell,
        index,
        town_cn: 0,
        xy: crate::tile_type::TileIndex(u32::MAX),
        town: std::ptr::null_mut(),
        name: Default::default(),
        build_date: 0,
    }))
}
/// The shell has completed its destructor and retired its live slot's record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_depot_record_delete(record: *mut DepotRecord) {
    unsafe { drop(Box::from_raw(record)) };
}

pub(crate) unsafe fn can_allocate() -> bool {
    unsafe { PoolCore::can_allocate(*POOL.get(), 1) }
}
