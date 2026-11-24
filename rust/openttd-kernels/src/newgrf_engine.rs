/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared constants from `newgrf_engine.h`.
/// `VEHICLEINFO_FULL_VEHICLE_WIDTH`.
pub const VEHICLEINFO_FULL_VEHICLE_WIDTH: u32 = 32;

/// `TRAININFO_DEFAULT_VEHICLE_WIDTH`.
pub const TRAININFO_DEFAULT_VEHICLE_WIDTH: u32 = 29;

/// `UsesWagonOverride`: check if a wagon is currently using a wagon override.
#[allow(
    unsafe_code,
    reason = "the resolver binding over a live vehicle record"
)]
pub(crate) fn uses_wagon_override(
    s: &crate::newgrf_vehicle::Services,
    v: crate::vehicle::VehicleRef,
) -> bool {
    crate::services_assert::assert_with(|| {
        v.vehicle_type() == crate::vehicle_type::VehicleType::VEH_TRAIN
    });
    // SAFETY: A live record and the immutable binding; one field read.
    !unsafe { (s.cached_override)(v.0.as_ptr()) }.is_null()
}
