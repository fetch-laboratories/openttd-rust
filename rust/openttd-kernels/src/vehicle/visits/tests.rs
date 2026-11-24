/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tests of the station and depot visits.
use super::*;

#[test]
fn autorefit_restore_reuses_storage_and_restores_payload() {
    // These identities are only compared, never dereferenced by restore.
    let vehicle = VehicleRef(std::ptr::NonNull::dangling());
    let mut frame = Refresher {
        vehicle,
        capacities: [37; CargoType::NUM_CARGO.0 as usize],
        refit_capacities: vec![
            RefitDesc {
                cargo: CargoType(2),
                capacity: 113,
                remaining: 71,
            },
            RefitDesc {
                cargo: CargoType(9),
                capacity: 211,
                remaining: 0,
            },
        ],
        cargo: CargoType(9),
        allow_merge: true,
        is_full_loading: true,
    };
    // The original makes one backup allocation, outside the cargo loop.
    let backup = frame.clone();
    let storage = frame.refit_capacities.as_ptr();
    let capacity = frame.refit_capacities.capacity();
    for _ in 0..CargoType::NUM_CARGO.0 {
        let mut other_identity = 0_u8;
        frame.vehicle = VehicleRef(std::ptr::NonNull::from(&mut other_identity).cast());
        frame.capacities.fill(0);
        frame.refit_capacities.fill(RefitDesc {
            cargo: CargoType(0),
            capacity: 0,
            remaining: 0,
        });
        frame.cargo = CargoType(0);
        frame.allow_merge = false;
        frame.is_full_loading = false;
        frame.restore(&backup);
        assert_eq!(frame.refit_capacities.as_ptr(), storage);
        assert_eq!(frame.refit_capacities.capacity(), capacity);
        assert_eq!(frame.refit_capacities.len(), backup.refit_capacities.len());
        assert_eq!(frame.vehicle.0, vehicle.0);
        assert_eq!(frame.capacities, backup.capacities);
        for (actual, expected) in frame.refit_capacities.iter().zip(&backup.refit_capacities) {
            assert_eq!(
                (actual.cargo, actual.capacity, actual.remaining),
                (expected.cargo, expected.capacity, expected.remaining)
            );
        }
        assert_eq!(frame.cargo, backup.cargo);
        assert_eq!(frame.allow_merge, backup.allow_merge);
        assert_eq!(frame.is_full_loading, backup.is_full_loading);
    }
}
