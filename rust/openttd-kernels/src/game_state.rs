/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Process-wide Rust state for C++ globals and statics. GUI input and single-player commands
//! run on the main thread and the game loop on `ottd:game`; the native game-state mutex
//! serializes both, and startup runs before the game thread starts.

#![allow(unsafe_code)]

use std::cell::UnsafeCell;

pub(crate) struct GameState<T>(UnsafeCell<T>);
// SAFETY: Every access holds the native game-state mutex or runs at startup, so accesses
// never overlap and the mutex orders them across threads; each borrow ends before a
// callback that can reenter its owner.
unsafe impl<T> Sync for GameState<T> {}
impl<T> GameState<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }
    pub(crate) const fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// Registry of an immutable native service table that outlives every call through it.
impl<T> GameState<*const T> {
    /// Install `table` and return its predecessor, which is only reinstalled, never read.
    pub(crate) fn exchange(&self, table: *const T) -> *const T {
        // SAFETY: Serialized access to the slot itself; no borrow of either table.
        unsafe { std::ptr::replace(self.get(), table) }
    }
    /// # Safety
    /// A table is installed and outlives the returned borrow.
    pub(crate) unsafe fn table<'a>(&self) -> &'a T {
        // SAFETY: The caller's installation contract; the slot read is serialized.
        unsafe { &**self.get() }
    }
}
