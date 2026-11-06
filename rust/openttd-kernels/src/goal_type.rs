/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Types and constants from `goal_type.h`.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GoalID(pub u16);
impl GoalID {
    pub const INVALID: Self = Self(u16::MAX);
}
#[repr(C)]
pub struct Goal {
    _private: [u8; 0],
}
pub const GT_NONE: u8 = 0;
pub const GT_TILE: u8 = 1;
pub const GT_INDUSTRY: u8 = 2;
pub const GT_TOWN: u8 = 3;
pub const GT_COMPANY: u8 = 4;
pub const GT_STORY_PAGE: u8 = 5;
pub const GOAL_QUESTION_BUTTON_COUNT: u8 = 18;
pub const GQT_QUESTION: u8 = 0;
pub const GQT_END: u8 = 4;

use crate::game_state::GameState;
use crate::pool::{PoolCore, PoolDescriptor, PoolId, TypedPool};
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    "OpenTTDGoalRecord" ["goal_type"] => pub struct GoalRecord {
        pub shell: *mut Goal => "Goal *",
        pub index: GoalID => "GoalID",
        pub company: crate::company_type::CompanyID => "CompanyID",
        pub r#type: u8 => "GoalType",
        pub dst: u32 => "GoalTypeID",
        pub text: crate::industrytype::NativeVec<u8> => "RustEncodedString",
        pub progress: crate::industrytype::NativeVec<u8> => "RustEncodedString",
        pub completed: bool,
    }
}

impl PoolId for GoalID {
    fn index(self) -> usize {
        self.0 as usize
    }
}
impl GoalID {
    pub const END: usize = 64000;
}
static GOAL_POOL: GameState<*mut PoolCore> = GameState::new(null_mut());
/// Bind the actual process-wide native pool during startup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_goal_pool_bind(pool: *mut PoolCore) {
    unsafe {
        *GOAL_POOL.get() = pool;
    }
}
pub(crate) unsafe fn goal_pool() -> TypedPool<GoalID, GoalRecord, Goal> {
    unsafe { TypedPool::from_descriptor(PoolCore::descriptor(*GOAL_POOL.get())) }
}
pub(crate) unsafe fn goal_descriptor() -> *const PoolDescriptor {
    unsafe { PoolCore::descriptor(*GOAL_POOL.get()) }
}
pub(crate) unsafe fn goal_can_allocate() -> bool {
    unsafe { PoolCore::can_allocate(*GOAL_POOL.get(), 1) }
}
/// Allocate the beside record with the original shell's member defaults.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_goal_record_new(shell: *mut Goal, index: GoalID) -> *mut GoalRecord {
    Box::into_raw(Box::new(GoalRecord {
        shell,
        index,
        company: crate::company_type::CompanyID::INVALID,
        r#type: GT_NONE,
        dst: 0,
        text: crate::industrytype::NativeVec::new(),
        progress: crate::industrytype::NativeVec::new(),
        completed: false,
    }))
}
/// The native owner retires the record while its shell slot is still live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_goal_record_delete(record: *mut GoalRecord) {
    unsafe {
        drop(Box::from_raw(record));
    }
}
