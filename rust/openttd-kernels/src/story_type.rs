/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `story_type.h`.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoryPageID(pub u16);
impl StoryPageID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoryPageElementID(pub u16);
impl StoryPageElementID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct StoryPage {
    _private: [u8; 0],
}
#[repr(C)]
pub struct StoryPageElement {
    _private: [u8; 0],
}

use crate::game_state::GameState;
use crate::pool::{PoolCore, PoolDescriptor, PoolId, TypedPool};
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDStoryPageRecord" ["story_type"] => pub struct StoryPageRecord {
        pub shell: *mut StoryPage => "StoryPage *",
        pub index: StoryPageID => "StoryPageID",
        pub sort_value: u32,
        pub date: i32 => "TimerGameCalendar::Date",
        pub company: crate::company_type::CompanyID => "CompanyID",
        pub title: crate::industrytype::NativeVec<u8> => "RustEncodedString",
    }
    "OpenTTDStoryPageElementRecord" ["story_type"] => pub struct StoryPageElementRecord {
        pub shell: *mut StoryPageElement => "StoryPageElement *",
        pub index: StoryPageElementID => "StoryPageElementID",
        pub sort_value: u32,
        pub page: StoryPageID => "StoryPageID",
        pub r#type: u8 => "StoryPageElementType",
        pub referenced_id: u32,
        pub text: crate::industrytype::NativeVec<u8> => "RustEncodedString",
    }
}

impl PoolId for StoryPageID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
impl StoryPageID {
    pub const END: usize = 64000;
}
static STORY_PAGE_POOL: GameState<*mut PoolCore> = GameState::new(null_mut());
/// Bind the actual process-wide native pool during startup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_pool_bind(pool: *mut PoolCore) {
    unsafe {
        *STORY_PAGE_POOL.get() = pool;
    }
}
pub(crate) unsafe fn story_page_pool() -> TypedPool<StoryPageID, StoryPageRecord, StoryPage> {
    unsafe { TypedPool::from_descriptor(PoolCore::descriptor(*STORY_PAGE_POOL.get())) }
}
pub(crate) unsafe fn story_page_descriptor() -> *const PoolDescriptor {
    unsafe { PoolCore::descriptor(*STORY_PAGE_POOL.get()) }
}
pub(crate) unsafe fn story_page_can_allocate() -> bool {
    unsafe { PoolCore::can_allocate(*STORY_PAGE_POOL.get(), 1) }
}
/// Allocate the beside record with the original shell's member defaults.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_story_page_record_new(
    shell: *mut StoryPage,
    index: StoryPageID,
) -> *mut StoryPageRecord {
    Box::into_raw(Box::new(StoryPageRecord {
        shell,
        index,
        sort_value: 0,
        date: 0,
        company: crate::company_type::CompanyID::INVALID,
        title: crate::industrytype::NativeVec::new(),
    }))
}
/// The native owner retires the record while its shell slot is still live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_record_delete(record: *mut StoryPageRecord) {
    unsafe {
        drop(Box::from_raw(record));
    }
}

impl PoolId for StoryPageElementID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
impl StoryPageElementID {
    pub const END: usize = 64000;
}
static STORY_PAGE_ELEMENT_POOL: GameState<*mut PoolCore> = GameState::new(null_mut());
/// Bind the actual process-wide native pool during startup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_element_pool_bind(pool: *mut PoolCore) {
    unsafe {
        *STORY_PAGE_ELEMENT_POOL.get() = pool;
    }
}
pub(crate) unsafe fn story_page_element_pool()
-> TypedPool<StoryPageElementID, StoryPageElementRecord, StoryPageElement> {
    unsafe { TypedPool::from_descriptor(PoolCore::descriptor(*STORY_PAGE_ELEMENT_POOL.get())) }
}
pub(crate) unsafe fn story_page_element_descriptor() -> *const PoolDescriptor {
    unsafe { PoolCore::descriptor(*STORY_PAGE_ELEMENT_POOL.get()) }
}
pub(crate) unsafe fn story_page_element_can_allocate() -> bool {
    unsafe { PoolCore::can_allocate(*STORY_PAGE_ELEMENT_POOL.get(), 1) }
}
/// Allocate the beside record with the original shell's member defaults.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_story_page_element_record_new(
    shell: *mut StoryPageElement,
    index: StoryPageElementID,
) -> *mut StoryPageElementRecord {
    Box::into_raw(Box::new(StoryPageElementRecord {
        shell,
        index,
        sort_value: 0,
        page: StoryPageID(0),
        r#type: 0,
        referenced_id: 0,
        text: crate::industrytype::NativeVec::new(),
    }))
}
/// The native owner retires the record while its shell slot is still live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_element_record_delete(
    record: *mut StoryPageElementRecord,
) {
    unsafe {
        drop(Box::from_raw(record));
    }
}
