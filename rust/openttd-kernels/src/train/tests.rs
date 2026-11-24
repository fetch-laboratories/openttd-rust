/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Unit tests of the train bodies against expectations read from `train_cmd.cpp`.

/// `Train::From`'s type assertion holds the handle constructor: a road vehicle fails it once,
/// a train passes it and its handle carries the record's train owner.
mod handle_tests {
    use crate::vehicle::VehicleRef;
    use crate::vehicle_type::VehicleType;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Failed native assertions seen by the isolated child.
    static FAILED: AtomicU32 = AtomicU32::new(0);
    extern "C" fn record_failure(condition: bool) {
        if !condition {
            FAILED.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn from_vehicle_asserts_the_vehicle_type_and_carries_the_train_owner() {
        if crate::services_assert::isolate_test_check(
            "train::tests::handle_tests::from_vehicle_asserts_the_vehicle_type_and_carries_the_train_owner",
            record_failure,
        ) {
            return;
        }
        let record = |kind| {
            VehicleRef::of(crate::vehicle_base::openttd_rust_vehicle_record_new(
                std::ptr::null_mut(),
                kind,
            ))
        };
        let owner = std::ptr::NonNull::<crate::train_state::TrainState>::dangling().as_ptr();
        let train = record(VehicleType::VEH_TRAIN);
        // SAFETY: The test's fresh record, which it leaks.
        unsafe { (*train.0.as_ptr()).train = owner };
        let handle = super::super::Handle::from_vehicle(train);
        assert_eq!((handle.record, handle.owner), (train.0.as_ptr(), owner));
        assert_eq!(FAILED.load(Ordering::Relaxed), 0);
        let _ = super::super::Handle::from_vehicle(record(VehicleType::VEH_ROAD));
        assert_eq!(FAILED.load(Ordering::Relaxed), 1);
    }
}
