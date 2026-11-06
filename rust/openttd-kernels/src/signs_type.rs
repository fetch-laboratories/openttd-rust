/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `signs_type.h`.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignID(pub u16);
impl SignID {
    pub const INVALID: Self = Self(u16::MAX);
}
/// Opaque native `Sign`; valid only while its native pool shell lives.
#[repr(C)]
pub struct Sign {
    _private: [u8; 0],
}
pub const MAX_LENGTH_SIGN_NAME_CHARS: usize = 32;

use crate::game_state::GameState;
use crate::pool::{PoolCore, PoolId, TypedPool};
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDSignRecord" ["signs_type"] => pub struct SignRecord {
        pub shell: *mut Sign,
        pub index: SignID => "SignID",
        pub name: crate::industrytype::NativeVec<u8> => "RustVector<uint8_t>",
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub owner: crate::company_type::Owner => "Owner",
    }
}

impl PoolId for SignID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
static POOL: GameState<*mut PoolCore> = GameState::new(null_mut());
/// Bind the actual native pool at startup, before allocation or command dispatch.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_sign_pool_bind(pool: *mut PoolCore) {
    unsafe { *POOL.get() = pool };
}
pub(crate) unsafe fn pool() -> TypedPool<SignID, SignRecord, Sign> {
    unsafe { TypedPool::from_descriptor(PoolCore::descriptor(*POOL.get())) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_sign_record_new(shell: *mut Sign, index: SignID) -> *mut SignRecord {
    Box::into_raw(Box::new(SignRecord {
        shell,
        index,
        name: Default::default(),
        x: 0,
        y: 0,
        z: 0,
        owner: crate::company_type::CompanyID(0xff),
    }))
}
/// The shell has completed its destructor and retired its live slot's record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_sign_record_delete(record: *mut SignRecord) {
    unsafe { drop(Box::from_raw(record)) };
}

pub(crate) unsafe fn can_allocate() -> bool {
    unsafe { PoolCore::can_allocate(*POOL.get(), 1) }
}
