/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `league_type.h`.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeagueTableID(pub u8);
impl LeagueTableID {
    pub const INVALID: Self = Self(u8::MAX);
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeagueTableElementID(pub u16);
impl LeagueTableElementID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct LeagueTableElement {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LeagueTable {
    _private: [u8; 0],
}
pub const LT_NONE: u8 = 0;
pub const LT_TILE: u8 = 1;
pub const LT_INDUSTRY: u8 = 2;
pub const LT_TOWN: u8 = 3;
pub const LT_COMPANY: u8 = 4;
pub const LT_STORY_PAGE: u8 = 5;

use crate::game_state::GameState;
use crate::pool::{PoolCore, PoolId, TypedPool};
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDLeagueTableRecord" ["league_type"] => pub struct LeagueTableRecord {
        pub shell: *mut LeagueTable => "LeagueTable *",
        pub index: LeagueTableID => "LeagueTableID",
        pub title: crate::industrytype::NativeVec<u8> => "RustEncodedString",
        pub header: crate::industrytype::NativeVec<u8> => "RustEncodedString",
        pub footer: crate::industrytype::NativeVec<u8> => "RustEncodedString",
    }
    "OpenTTDLeagueTableElementRecord" ["league_type"] => pub struct LeagueTableElementRecord {
        pub shell: *mut LeagueTableElement => "LeagueTableElement *",
        pub index: LeagueTableElementID => "LeagueTableElementID",
        pub table: LeagueTableID => "LeagueTableID",
        pub rating: i64,
        pub company: crate::company_type::CompanyID => "CompanyID",
        pub text: crate::industrytype::NativeVec<u8> => "RustEncodedString",
        pub score: crate::industrytype::NativeVec<u8> => "RustEncodedString",
        pub link_type: u8 => "LinkType",
        pub link_target: u32,
    }
}

impl PoolId for LeagueTableID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
impl LeagueTableID {
    pub const END: usize = 255;
}
static LEAGUE_TABLE_POOL: GameState<*mut PoolCore> = GameState::new(null_mut());
/// Bind the actual process-wide native pool during startup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_league_table_pool_bind(pool: *mut PoolCore) {
    unsafe {
        *LEAGUE_TABLE_POOL.get() = pool;
    }
}
pub(crate) unsafe fn league_table_can_allocate() -> bool {
    unsafe { PoolCore::can_allocate(*LEAGUE_TABLE_POOL.get(), 1) }
}
/// Allocate the beside record with the original shell's member defaults.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_league_table_record_new(
    shell: *mut LeagueTable,
    index: LeagueTableID,
) -> *mut LeagueTableRecord {
    Box::into_raw(Box::new(LeagueTableRecord {
        shell,
        index,
        title: crate::industrytype::NativeVec::new(),
        header: crate::industrytype::NativeVec::new(),
        footer: crate::industrytype::NativeVec::new(),
    }))
}
/// The native owner retires the record while its shell slot is still live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_league_table_record_delete(record: *mut LeagueTableRecord) {
    unsafe {
        drop(Box::from_raw(record));
    }
}

impl PoolId for LeagueTableElementID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
impl LeagueTableElementID {
    pub const END: usize = 64000;
}
static LEAGUE_TABLE_ELEMENT_POOL: GameState<*mut PoolCore> = GameState::new(null_mut());
/// Bind the actual process-wide native pool during startup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_league_table_element_pool_bind(pool: *mut PoolCore) {
    unsafe {
        *LEAGUE_TABLE_ELEMENT_POOL.get() = pool;
    }
}
pub(crate) unsafe fn league_table_element_pool()
-> TypedPool<LeagueTableElementID, LeagueTableElementRecord, LeagueTableElement> {
    unsafe { TypedPool::from_descriptor(PoolCore::descriptor(*LEAGUE_TABLE_ELEMENT_POOL.get())) }
}
pub(crate) unsafe fn league_table_element_can_allocate() -> bool {
    unsafe { PoolCore::can_allocate(*LEAGUE_TABLE_ELEMENT_POOL.get(), 1) }
}
/// Allocate the beside record with the original shell's member defaults.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_league_table_element_record_new(
    shell: *mut LeagueTableElement,
    index: LeagueTableElementID,
) -> *mut LeagueTableElementRecord {
    Box::into_raw(Box::new(LeagueTableElementRecord {
        shell,
        index,
        table: LeagueTableID::INVALID,
        rating: 0,
        company: crate::company_type::CompanyID::INVALID,
        text: crate::industrytype::NativeVec::new(),
        score: crate::industrytype::NativeVec::new(),
        link_type: LT_NONE,
        link_target: 0,
    }))
}
/// The native owner retires the record while its shell slot is still live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_league_table_element_record_delete(
    record: *mut LeagueTableElementRecord,
) {
    unsafe {
        drop(Box::from_raw(record));
    }
}
