/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `FreeUnitIDGenerator`: Company owns each bitmap until member destruction.
//! Entries hold the game-state mutex. Native assertion leaves cannot mutate the
//! bitmap. Copy observes the source before replacing destination storage.
#![allow(unsafe_code)]
use crate::industrytype::NativeVec;

crate::abi::shared_records! {
    fn abi_record;
    /// Canonical bitmap words, with native `size_t` width.
    #[derive(Clone, Default)]
    "OpenTTDUnitNumberState" ["vehicle_maintenance"] => pub struct UnitNumbers {
        /// No stored aliases survive growth, assignment or Company destruction.
        pub used_bitmap: NativeVec<usize> => "RustVector<std::size_t>",
    }
}
impl UnitNumbers {
    const BITS: usize = usize::BITS as usize;

    /// `NextID` does not claim the returned number; narrowing preserves wrap.
    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn next(&self) -> u16 {
        for (index, &word) in self.used_bitmap.as_slice().iter().enumerate() {
            let available = !word;
            if available != 0 {
                return (index * Self::BITS + available.trailing_zeros() as usize + 1) as u16;
            }
        }
        (self.used_bitmap.len() * Self::BITS + 1) as u16
    }

    /// `UseID` ignores the two invalid sentinels and grows with zero words.
    pub(crate) fn use_id(&mut self, id: u16) -> u16 {
        if id == 0 || id == u16::MAX {
            return id;
        }
        let index = usize::from(id - 1);
        let slot = index / Self::BITS;
        if slot >= self.used_bitmap.len() {
            self.used_bitmap.resize(slot + 1, 0);
        }
        // SAFETY: resize above makes this slot live.
        unsafe { *self.used_bitmap.as_mut_ptr().add(slot) |= 1 << (index % Self::BITS) };
        id
    }

    /// `ReleaseID` retains the native assertion before the bitmap write.
    pub(crate) unsafe fn release(&mut self, id: u16, check: impl FnOnce(bool)) {
        if id == 0 || id == u16::MAX {
            return;
        }
        let index = usize::from(id - 1);
        let slot = index / Self::BITS;
        if slot >= self.used_bitmap.len() {
            check(false);
        }
        // SAFETY: valid native calls release a previously used number; invalid
        // asserts-off input retains the source's out-of-bounds precondition.
        unsafe { *self.used_bitmap.as_mut_ptr().add(slot) &= !(1 << (index % Self::BITS)) };
    }
}

/// Copy the bitmap into an initialized destination, preserving self-assignment.
///
/// # Safety
/// Both descriptors are initialized live Company members under the game-state
/// mutex; they are independent unless their pointers are identical. Native
/// value-initialization is a valid empty descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_unit_numbers_copy(
    destination: *mut UnitNumbers,
    source: *const UnitNumbers,
) {
    if !std::ptr::eq(destination, source) {
        let copy = unsafe { (*source).clone() };
        unsafe { *destination = copy };
    }
}
/// Destroy a Company member's Rust bitmap.
///
/// # Safety
/// `state` is initialized and exclusively owned, and is never read after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_unit_numbers_drop(state: *mut UnitNumbers) {
    unsafe { std::ptr::drop_in_place(state) };
}
/// Find the lowest unused unit without claiming it.
///
/// # Safety
/// `state` is initialized and live under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_unit_numbers_next(state: *const UnitNumbers) -> u16 {
    unsafe { (*state).next() }
}
/// Claim a unit, growing the canonical bitmap when necessary.
///
/// # Safety
/// `state` is an initialized exclusive Company member under the game-state mutex.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_unit_numbers_use(state: *mut UnitNumbers, id: u16) -> u16 {
    unsafe { (*state).use_id(id) }
}
/// Release a unit through the original native assertion policy.
///
/// # Safety
/// `state` is an initialized exclusive Company member; non-sentinel `id` belongs
/// to an allocated word. `check` cannot mutate the bitmap or unwind.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_unit_numbers_release(
    state: *mut UnitNumbers,
    id: u16,
    check: extern "C" fn(bool),
) {
    unsafe { (*state).release(id, |condition| check(condition)) };
}

#[cfg(test)]
mod tests {
    use super::UnitNumbers;

    #[test]
    fn sentinels_holes_word_growth_and_independent_copies() {
        let mut units = UnitNumbers::default();
        assert_eq!(units.next(), 1);
        assert_eq!(units.use_id(0), 0);
        assert_eq!(units.use_id(u16::MAX), u16::MAX);
        assert!(units.used_bitmap.is_empty());
        for id in 1..=130 {
            assert_eq!(units.next(), id);
            assert_eq!(units.next(), id); // Merely asking never claims it.
            assert_eq!(units.use_id(id), id);
        }
        unsafe { units.release(64, |valid| assert!(valid)) };
        unsafe { units.release(129, |valid| assert!(valid)) };
        assert_eq!(units.next(), 64);
        let mut copy = units.clone();
        units.use_id(64);
        assert_eq!(units.next(), 129);
        assert_eq!(copy.next(), 64);
        copy.use_id(64);
        assert_eq!(copy.next(), 129);
        for id in 1..u16::MAX {
            units.use_id(id);
        }
        assert_eq!(units.next(), u16::MAX);
        // The invalid sentinel remains unclaimed even at exhaustion.
        units.use_id(u16::MAX);
        assert_eq!(units.next(), u16::MAX);
        unsafe { units.release(0, |_| panic!("sentinel cannot assert")) };
        unsafe { units.release(u16::MAX, |_| panic!("sentinel cannot assert")) };
        unsafe { units.release(7, |valid| assert!(valid)) };
        assert_eq!(units.next(), 7);
    }

    #[test]
    #[should_panic(expected = "native ReleaseID range assertion")]
    fn release_asserts_before_accessing_an_absent_word() {
        unsafe {
            UnitNumbers::default().release(1, |valid| {
                assert!(valid, "native ReleaseID range assertion");
            });
        }
    }
}
