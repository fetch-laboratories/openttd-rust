/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Types for recording game performance data (`framerate_type.h`).
//!
//! Each measurement calls the native entry point of the same name in `framerate_gui.cpp`,
//! which owns `_pf_data` and the performance timer until the GUI is ported. The native table
//! is bound once for the process; before it binds, every measurement is discarded, as a
//! measurement changes no game state.
//!
//! `_pf_data` is unsynchronised, as in the original, so each entry point is `unsafe`: its
//! caller runs on the thread that runs the original's statement (the game loop's, or world
//! generation's for the tile loop), serialized with every other framerate call by the
//! game-state mutex. A guard ends on the thread that
//! began it, so neither guard is `Send` or `Sync`; its contract covers its drop. A guard's native object is constructed
//! in, and destroyed from, storage that the caller keeps at one address until the guard drops;
//! the guard itself holds only references and may move.

use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicPtr, Ordering};

/// Elements of game performance that can be measured (`PerformanceElement`). The constants
/// are those Rust measures; the others arrive with their first Rust user.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PerformanceElement(u8);
impl PerformanceElement {
    /// The `uint8_t` the native entry points take.
    pub(crate) const fn base(self) -> u8 {
        self.0
    }
    /// Speed of gameloop processing.
    pub const PFE_GAMELOOP: Self = Self(0);
    /// Time spent processing cargo movement
    pub const PFE_GL_ECONOMY: Self = Self(1);
    /// Time spent processing trains
    pub const PFE_GL_TRAINS: Self = Self(2);
    /// Time spend processing road vehicles
    pub const PFE_GL_ROADVEHS: Self = Self(3);
    /// Time spent processing ships
    pub const PFE_GL_SHIPS: Self = Self(4);
    /// Time spent processing aircraft
    pub const PFE_GL_AIRCRAFT: Self = Self(5);
    /// Time spent processing other world features
    pub const PFE_GL_LANDSCAPE: Self = Self(6);
    /// Time spent waiting for link graph background jobs
    pub const PFE_GL_LINKGRAPH: Self = Self(7);
    /// Sum of all GS/AI scripts
    pub const PFE_ALLSCRIPTS: Self = Self(12);
}

/// Storage of a native `PerformanceMeasurer`, checked for size and alignment by C++.
#[repr(C, align(8))]
pub struct PerformanceMeasurerStorage {
    pub storage: [MaybeUninit<u8>; 16],
}
/// Storage of a native `PerformanceAccumulator`, checked for size and alignment by C++.
#[repr(C, align(8))]
pub struct PerformanceAccumulatorStorage {
    pub storage: [MaybeUninit<u8>; 16],
}

/// Native performance entry points.
#[repr(C)]
pub struct Services {
    /// `new (storage) PerformanceAccumulator(elem)`.
    pub accumulator_begin: extern "C" fn(*mut PerformanceAccumulatorStorage, u8),
    /// `~PerformanceAccumulator()` on the object in `storage`.
    pub accumulator_end: extern "C" fn(*mut PerformanceAccumulatorStorage),
    /// `PerformanceAccumulator::Reset(elem)`.
    pub accumulator_reset: extern "C" fn(u8),
    /// `new (storage) PerformanceMeasurer(elem)`.
    pub measurer_begin: extern "C" fn(*mut PerformanceMeasurerStorage, u8),
    /// `~PerformanceMeasurer()` on the object in `storage`.
    pub measurer_end: extern "C" fn(*mut PerformanceMeasurerStorage),
    /// `PerformanceMeasurer::Paused(elem)`.
    pub measurer_paused: extern "C" fn(u8),
    /// `PerformanceMeasurer::SetInactive(elem)`.
    pub measurer_set_inactive: extern "C" fn(u8),
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDFramerateServices" ["framerate"] => Services {
        accumulator_begin,
        accumulator_end,
        accumulator_reset,
        measurer_begin,
        measurer_end,
        measurer_paused,
        measurer_set_inactive,
    };
    "OpenTTDPerformanceAccumulator" ["framerate"] => PerformanceAccumulatorStorage { storage, };
    "OpenTTDPerformanceMeasurer" ["framerate"] => PerformanceMeasurerStorage { storage, };
}

extern "C" fn discard_storage<T>(_: *mut T) {}
extern "C" fn discard_begin<T>(_: *mut T, _: u8) {}
extern "C" fn discard_element(_: u8) {}
/// Services before the native table binds: each measurement is discarded.
static DISCARD: Services = Services {
    accumulator_begin: discard_begin,
    accumulator_end: discard_storage,
    accumulator_reset: discard_element,
    measurer_begin: discard_begin,
    measurer_end: discard_storage,
    measurer_paused: discard_element,
    measurer_set_inactive: discard_element,
};
static SERVICES: AtomicPtr<Services> = AtomicPtr::new((&raw const DISCARD).cast_mut());

/// Bind the native table during static initialization; return the native table it replaced,
/// or null when it replaced the discarding services.
///
/// # Safety
/// `services` lives for the process.
#[allow(
    unsafe_code,
    reason = "the binding export and its process-lifetime pointer contract"
)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_framerate_bind(services: *const Services) -> *const Services {
    let previous = SERVICES
        .swap(services.cast_mut(), Ordering::AcqRel)
        .cast_const();
    if std::ptr::eq(previous, &raw const DISCARD) {
        std::ptr::null()
    } else {
        previous
    }
}

/// The bound table, or the discarding services before the native table binds.
#[allow(
    unsafe_code,
    reason = "the dereference of the process-lifetime table pointer"
)]
fn services() -> &'static Services {
    // SAFETY: Either `DISCARD` or a process-lifetime table from `openttd_rust_framerate_bind`.
    unsafe { &*SERVICES.load(Ordering::Acquire) }
}

/// RAII class for measuring simple elements of performance (`PerformanceMeasurer`).
/// Construct an object with the appropriate element parameter when processing begins,
/// time is automatically taken when the object goes out of scope again.
///
/// Call Paused at the start of a frame if the processing of this element is paused.
#[must_use]
pub(crate) struct PerformanceMeasurer<'a> {
    storage: &'a mut MaybeUninit<PerformanceMeasurerStorage>,
    /// The table its begin used; its end uses the same.
    services: &'static Services,
    thread: PhantomData<*const ()>,
}
#[allow(
    unsafe_code,
    reason = "the entry points' serialized-access contract on unsynchronised `_pf_data`"
)]
impl<'a> PerformanceMeasurer<'a> {
    /// Begin a cycle of a measured element.
    ///
    /// # Safety
    /// Called on the thread that runs the original's statement, serialized with every other
    /// framerate call by the game-state mutex, which also drops the guard.
    pub(crate) unsafe fn new(
        storage: &'a mut MaybeUninit<PerformanceMeasurerStorage>,
        elem: PerformanceElement,
    ) -> Self {
        let services = services();
        (services.measurer_begin)(storage.as_mut_ptr(), elem.base());
        Self {
            storage,
            services,
            thread: PhantomData,
        }
    }
    /// Mark a performance element as not currently in use.
    ///
    /// # Safety
    /// Called on the thread that runs the original's statement, serialized with every other
    /// framerate call by the game-state mutex.
    pub(crate) unsafe fn set_inactive(elem: PerformanceElement) {
        (services().measurer_set_inactive)(elem.base());
    }
    /// Indicate that a cycle of "pause" where no processing occurs.
    ///
    /// # Safety
    /// Called on the thread that runs the original's statement, serialized with every other
    /// framerate call by the game-state mutex.
    pub(crate) unsafe fn paused(elem: PerformanceElement) {
        (services().measurer_paused)(elem.base());
    }
}
impl Drop for PerformanceMeasurer<'_> {
    /// Finish a cycle of a measured element and store the measurement taken.
    fn drop(&mut self) {
        (self.services.measurer_end)(self.storage.as_mut_ptr());
    }
}

/// RAII class for measuring multi-step elements of performance (`PerformanceAccumulator`).
/// At the beginning of a frame, call Reset on the element, then construct an object in the
/// scope where each processing cycle happens. The measurements are summed between resets.
///
/// Usually `StateGameLoop` is an appropriate function to place Reset calls in, but for
/// elements with more isolated scopes it can also be appropriate to Reset somewhere else.
/// An example is the `CallVehicleTicks` function where all the vehicle type elements are reset.
///
/// The [`PerformanceMeasurer::paused`] function can also be used with elements otherwise
/// measured with this class.
#[must_use]
pub(crate) struct PerformanceAccumulator<'a> {
    storage: &'a mut MaybeUninit<PerformanceAccumulatorStorage>,
    /// The table its begin used; its end uses the same.
    services: &'static Services,
    thread: PhantomData<*const ()>,
}
#[allow(
    unsafe_code,
    reason = "the entry points' serialized-access contract on unsynchronised `_pf_data`"
)]
impl<'a> PerformanceAccumulator<'a> {
    /// Begin measuring one block of the accumulating value.
    ///
    /// # Safety
    /// Called on the thread that runs the original's statement, serialized with every other
    /// framerate call by the game-state mutex, which also drops the guard.
    pub(crate) unsafe fn new(
        storage: &'a mut MaybeUninit<PerformanceAccumulatorStorage>,
        elem: PerformanceElement,
    ) -> Self {
        let services = services();
        (services.accumulator_begin)(storage.as_mut_ptr(), elem.base());
        Self {
            storage,
            services,
            thread: PhantomData,
        }
    }
    /// Store the previous accumulator value and reset for a new cycle of accumulating
    /// measurements. This function must be called once per frame, otherwise measurements
    /// are not collected.
    ///
    /// # Safety
    /// Called on the thread that runs the original's statement, serialized with every other
    /// framerate call by the game-state mutex.
    pub(crate) unsafe fn reset(elem: PerformanceElement) {
        (services().accumulator_reset)(elem.base());
    }
}
impl Drop for PerformanceAccumulator<'_> {
    /// Finish and add one block of the accumulating value.
    fn drop(&mut self) {
        (self.services.accumulator_end)(self.storage.as_mut_ptr());
    }
}

/// A recording framerate table shared by the tests of every caller.
#[cfg(test)]
pub(crate) mod testing {
    use super::{
        PerformanceAccumulatorStorage, PerformanceElement, PerformanceMeasurerStorage, Services,
    };
    use std::cell::{Cell, RefCell};

    /// One native framerate call, in the order the code under test made it. An end carries
    /// the element its begin wrote into the storage, and must end the innermost live scope at
    /// the address that scope began at.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Effect {
        AccumulatorBegin(PerformanceElement),
        AccumulatorEnd(PerformanceElement),
        AccumulatorReset(PerformanceElement),
        MeasurerBegin(PerformanceElement),
        MeasurerEnd(PerformanceElement),
        MeasurerPaused(PerformanceElement),
        MeasurerSetInactive(PerformanceElement),
    }

    thread_local! {
        static EFFECTS: RefCell<Vec<Effect>> = const { RefCell::new(Vec::new()) };
        static OBSERVER: Cell<Option<fn(Effect)>> = const { Cell::new(None) };
        static LIVE: RefCell<Vec<usize>> = const { RefCell::new(Vec::new()) };
    }
    fn begin(storage: *mut [std::mem::MaybeUninit<u8>; 16], elem: u8) {
        write(storage, elem);
        LIVE.with(|l| l.borrow_mut().push(storage.addr()));
    }
    fn end(storage: *mut [std::mem::MaybeUninit<u8>; 16]) -> PerformanceElement {
        let began = LIVE.with(|l| l.borrow_mut().pop());
        assert_eq!(began, Some(storage.addr()), "a scope ends where it began");
        read(storage)
    }
    fn push(effect: Effect) {
        if let Some(observe) = OBSERVER.get() {
            observe(effect);
        }
        EFFECTS.with(|e| e.borrow_mut().push(effect));
    }
    /// Call `observe` with each of this thread's framerate calls as it is made, so a test can
    /// place it among its own observations.
    pub(crate) fn observe(observe: Option<fn(Effect)>) {
        OBSERVER.set(observe);
    }

    #[allow(
        unsafe_code,
        reason = "the recording begin stores its element in the caller's storage"
    )]
    fn write(storage: *mut [std::mem::MaybeUninit<u8>; 16], elem: u8) {
        // SAFETY: Production passes the caller's live 16-byte storage.
        unsafe { (*storage)[0] = std::mem::MaybeUninit::new(elem) };
    }
    #[allow(
        unsafe_code,
        reason = "the recording end reads the element its begin stored"
    )]
    fn read(storage: *mut [std::mem::MaybeUninit<u8>; 16]) -> PerformanceElement {
        // SAFETY: The storage its begin initialised, still at the same address.
        PerformanceElement(unsafe { (*storage)[0].assume_init() })
    }
    extern "C" fn accumulator_begin(storage: *mut PerformanceAccumulatorStorage, elem: u8) {
        begin(storage.cast(), elem);
        push(Effect::AccumulatorBegin(PerformanceElement(elem)));
    }
    extern "C" fn accumulator_end(storage: *mut PerformanceAccumulatorStorage) {
        push(Effect::AccumulatorEnd(end(storage.cast())));
    }
    extern "C" fn accumulator_reset(elem: u8) {
        push(Effect::AccumulatorReset(PerformanceElement(elem)));
    }
    extern "C" fn measurer_begin(storage: *mut PerformanceMeasurerStorage, elem: u8) {
        begin(storage.cast(), elem);
        push(Effect::MeasurerBegin(PerformanceElement(elem)));
    }
    extern "C" fn measurer_end(storage: *mut PerformanceMeasurerStorage) {
        push(Effect::MeasurerEnd(end(storage.cast())));
    }
    extern "C" fn measurer_paused(elem: u8) {
        push(Effect::MeasurerPaused(PerformanceElement(elem)));
    }
    extern "C" fn measurer_set_inactive(elem: u8) {
        push(Effect::MeasurerSetInactive(PerformanceElement(elem)));
    }
    static RECORDING: Services = Services {
        accumulator_begin,
        accumulator_end,
        accumulator_reset,
        measurer_begin,
        measurer_end,
        measurer_paused,
        measurer_set_inactive,
    };

    /// Bind the recording table for the process; return and clear this thread's recorded
    /// calls. A test calls it before the calls it observes.
    pub(crate) fn take() -> Vec<Effect> {
        super::SERVICES.store(
            (&raw const RECORDING).cast_mut(),
            std::sync::atomic::Ordering::Release,
        );
        EFFECTS.with(|e| std::mem::take(&mut *e.borrow_mut()))
    }
}

#[cfg(test)]
mod tests {
    use super::testing::{Effect, take};
    use super::{PerformanceAccumulator, PerformanceElement as E, PerformanceMeasurer};
    use std::mem::MaybeUninit;

    #[test]
    #[allow(
        unsafe_code,
        reason = "the test calls the entry points under their serialized-access contract"
    )]
    fn each_function_passes_its_element_to_its_entry_point() {
        take();
        // SAFETY: Each test thread makes only its own framerate calls, on the recording table.
        unsafe {
            PerformanceMeasurer::paused(E::PFE_GL_SHIPS);
            PerformanceMeasurer::set_inactive(E::PFE_GL_LINKGRAPH);
            PerformanceAccumulator::reset(E::PFE_GL_AIRCRAFT);
        }
        assert_eq!(
            take(),
            [
                Effect::MeasurerPaused(E::PFE_GL_SHIPS),
                Effect::MeasurerSetInactive(E::PFE_GL_LINKGRAPH),
                Effect::AccumulatorReset(E::PFE_GL_AIRCRAFT),
            ]
        );
    }

    #[test]
    #[allow(
        unsafe_code,
        reason = "the test calls the entry points under their serialized-access contract"
    )]
    fn a_guard_begins_at_new_and_ends_at_drop() {
        take();
        let mut storage = MaybeUninit::uninit();
        // SAFETY: As above; the guard drops on this thread.
        let measurer = unsafe { PerformanceMeasurer::new(&mut storage, E::PFE_ALLSCRIPTS) };
        assert_eq!(take(), [Effect::MeasurerBegin(E::PFE_ALLSCRIPTS)]);
        drop(measurer);
        assert_eq!(take(), [Effect::MeasurerEnd(E::PFE_ALLSCRIPTS)]);
        let mut storage = MaybeUninit::uninit();
        // SAFETY: As above.
        let accumulator = unsafe { PerformanceAccumulator::new(&mut storage, E::PFE_GL_LANDSCAPE) };
        assert_eq!(take(), [Effect::AccumulatorBegin(E::PFE_GL_LANDSCAPE)]);
        drop(accumulator);
        assert_eq!(take(), [Effect::AccumulatorEnd(E::PFE_GL_LANDSCAPE)]);
    }

    #[test]
    #[allow(
        unsafe_code,
        reason = "the test calls the entry points under their serialized-access contract"
    )]
    fn nested_guards_end_in_reverse_order() {
        take();
        {
            let mut outer = MaybeUninit::uninit();
            // SAFETY: As above; both guards drop on this thread.
            let _outer = unsafe { PerformanceMeasurer::new(&mut outer, E::PFE_GAMELOOP) };
            let mut inner = MaybeUninit::uninit();
            let _inner = unsafe { PerformanceAccumulator::new(&mut inner, E::PFE_GL_LANDSCAPE) };
        }
        assert_eq!(
            take(),
            [
                Effect::MeasurerBegin(E::PFE_GAMELOOP),
                Effect::AccumulatorBegin(E::PFE_GL_LANDSCAPE),
                Effect::AccumulatorEnd(E::PFE_GL_LANDSCAPE),
                Effect::MeasurerEnd(E::PFE_GAMELOOP),
            ]
        );
    }

    #[test]
    #[allow(
        unsafe_code,
        reason = "the test calls the entry points under their serialized-access contract"
    )]
    fn a_guard_ends_when_its_function_returns_early() {
        fn measured(fail: bool) -> Result<(), ()> {
            let mut storage = MaybeUninit::uninit();
            // SAFETY: As above; the guard drops on this thread.
            let _framerate = unsafe { PerformanceMeasurer::new(&mut storage, E::PFE_GL_ECONOMY) };
            if fail { Err(()) } else { Ok(()) }?;
            // SAFETY: As above.
            unsafe { PerformanceAccumulator::reset(E::PFE_GL_TRAINS) };
            Ok(())
        }
        take();
        assert_eq!(measured(true), Err(()));
        assert_eq!(
            take(),
            [
                Effect::MeasurerBegin(E::PFE_GL_ECONOMY),
                Effect::MeasurerEnd(E::PFE_GL_ECONOMY),
            ]
        );
    }
}
