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

// Canonical Company lookups finish before any bitmap borrow. The original
// operations have no mutating callback between acquiring a bitmap and using it.
unsafe fn vehicle_numbers(
    s: &crate::company_identity::Services,
    owner: u8,
    kind: u8,
) -> *mut UnitNumbers {
    let company = (s.company)(crate::company_type::CompanyID(owner));
    (s.vehicle_numbers)(company, crate::vehicle_type::VehicleType(kind))
}
unsafe fn group_numbers(s: &crate::company_identity::Services, owner: u8) -> *mut UnitNumbers {
    let company = (s.company)(crate::company_type::CompanyID(owner));
    (s.group_numbers)(company)
}

/// `GetFreeUnitNumber`: limit test precedes non-claiming bitmap selection.
/// Tables and Company-owned records are live under the game-state mutex.
pub(crate) unsafe fn free_vehicle_number(
    groups: &crate::fleet::GroupServices,
    kind: crate::vehicle_type::VehicleType,
) -> u16 {
    let settings = unsafe { &*groups.shared }.settings();
    let maximum = settings.vehicle_limit(kind);
    let owner = unsafe { (groups.current_company)() };
    let stats = unsafe { (groups.stats)(owner, crate::fleet::ALL_GROUP, kind.0) };
    let count = crate::raw_field::read_field!(stats.cast::<crate::fleet::StatsFields>(), vehicles);
    if count >= maximum {
        return u16::MAX;
    }
    let numbers = unsafe { vehicle_numbers(groups.vehicles.companies(), owner, kind.0) };
    unsafe { (*numbers).next() }
}

/// Reserve the supplied vehicle's number without changing its record.
pub(crate) unsafe fn use_vehicle(
    s: &crate::company_identity::Services,
    v: crate::vehicle::VehicleRef,
    unit: u16,
) -> u16 {
    let numbers = unsafe { vehicle_numbers(s, v.owner(), v.vehicle_type()) };
    unsafe { (*numbers).use_id(unit) }
}

/// Company takeover assigns a fresh number after changing ownership/statistics.
pub(crate) unsafe fn assign_next_vehicle(
    s: &crate::company_identity::Services,
    v: crate::vehicle::VehicleRef,
    owner: u8,
) {
    let numbers = unsafe { vehicle_numbers(s, owner, v.vehicle_type()) };
    let next = unsafe { (*numbers).next() };
    let unit = unsafe { (*numbers).use_id(next) };
    v.write_unit_number(unit);
}

/// `Vehicle::ReleaseUnitNumber`, including its subsequent zero store.
pub(crate) unsafe fn release_vehicle(
    s: &crate::company_identity::Services,
    v: crate::vehicle::VehicleRef,
) {
    let numbers = unsafe { vehicle_numbers(s, v.owner(), v.vehicle_type()) };
    unsafe {
        (*numbers).release(v.unit_number(), |condition| {
            ((*s.shared.movement).native_assert)(condition);
        });
    }
    v.write_unit_number(0);
}

/// `freegroups.UseID(freegroups.NextID())`.
pub(crate) unsafe fn next_group(s: &crate::company_identity::Services, owner: u8) -> u16 {
    let numbers = unsafe { group_numbers(s, owner) };
    let next = unsafe { (*numbers).next() };
    unsafe { (*numbers).use_id(next) }
}

/// Release a group number through the shared native assertion leaf.
pub(crate) unsafe fn release_group(s: &crate::company_identity::Services, owner: u8, unit: u16) {
    let numbers = unsafe { group_numbers(s, owner) };
    unsafe {
        (*numbers).release(unit, |condition| {
            ((*s.shared.movement).native_assert)(condition);
        });
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
