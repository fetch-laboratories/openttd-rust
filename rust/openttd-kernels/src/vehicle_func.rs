/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `vehicle_func.h`: the custom sprite numbers, `IsCompanyBuildableVehicleType`,
//! and both `CanVehicleUseStation` overloads (`vehicle.cpp`).
use crate::airport::{AirportFTAClassFlag, AirportFTAClassFlags};
use crate::engine_type::{AIR_CTOL, EngineID};
use crate::station_type::{StationFacilities, StationFacility};
use crate::vehicle::{EngineRef, VehicleRef};
use crate::vehicle_type::VehicleType;

/// Special values for `Vehicle::spritenum` and `(Aircraft|Rail|Road|Ship)VehicleInfo::image_index`.
/// The unscoped native enum uses an integer; stored sprite indices are bytes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CustomVehicleSpriteNum(pub u32);

impl CustomVehicleSpriteNum {
    /// Vehicle sprite from `NewGRF`.
    pub const CUSTOM_VEHICLE_SPRITENUM: Self = Self(0xfd);
    /// Vehicle sprite from `NewGRF` with reverse driving direction (from
    /// articulation callback).
    pub const CUSTOM_VEHICLE_SPRITENUM_REVERSED: Self = Self(0xfe);
}

/// Raw compatibility names for files no pass has finished; #734's closure removes them.
#[allow(clippy::cast_possible_truncation)] // Native enum stored in the sprite byte.
pub const CUSTOM_VEHICLE_SPRITENUM: u8 = CustomVehicleSpriteNum::CUSTOM_VEHICLE_SPRITENUM.0 as u8;
/// Raw `CUSTOM_VEHICLE_SPRITENUM_REVERSED`.
#[allow(clippy::cast_possible_truncation)] // Native enum stored in the sprite byte.
pub const CUSTOM_VEHICLE_SPRITENUM_REVERSED: u8 =
    CustomVehicleSpriteNum::CUSTOM_VEHICLE_SPRITENUM_REVERSED.0 as u8;

/// `CanVehicleUseStation(EngineID, const Station *)`: can this station be used
/// by the given engine type?
///
/// For road vehicles the vehicle is needed to determine whether it can use the
/// station. This function returns true for road vehicles when at least one of
/// the facilities is available.
pub(crate) fn can_engine_use_station(
    engine_type: EngineID,
    st: *const crate::station::BaseStation,
    s: &crate::newgrf_airport::Services,
) -> bool {
    let e = (s.engine_if_valid)(engine_type.0);
    crate::services_assert::assert_with(|| !e.is_null());

    match EngineRef::of(e).kind() {
        VehicleType::VEH_TRAIN => s.commands().facilities(st).test(StationFacility::Train),

        VehicleType::VEH_ROAD => {
            // For road vehicles we need the vehicle to know whether it can actually
            // use the station, but if it doesn't have facilities for RVs it is
            // certainly not possible that the station can be used.
            s.commands().facilities(st).any(
                StationFacilities::from_element(StationFacility::BusStop)
                    .with(StationFacilities::from_element(StationFacility::TruckStop)),
            )
        }

        VehicleType::VEH_SHIP => s.commands().facilities(st).test(StationFacility::Dock),

        VehicleType::VEH_AIRCRAFT => {
            s.commands().facilities(st).test(StationFacility::Airport) && {
                // `GetFTA()`, then the flag the engine selects, then its test.
                let fta = crate::newgrf_airport::station_fta(st.cast_mut(), s);
                let flag = if EngineRef::of(e).aircraft_subtype() & AIR_CTOL != 0 {
                    AirportFTAClassFlag::AIRPLANES
                } else {
                    AirportFTAClassFlag::HELICOPTERS
                };
                AirportFTAClassFlags((s.fta_flags)(fta)).test(flag)
            }
        }

        _ => false,
    }
}

/// `CanVehicleUseStation(const Vehicle *, const Station *)`: can this station be
/// used by the given vehicle?
pub(crate) fn can_vehicle_use_station(
    v: VehicleRef,
    st: *const crate::station::BaseStation,
    s: &crate::newgrf_airport::Services,
) -> bool {
    if v.vehicle_type() == VehicleType::VEH_ROAD {
        return !crate::road_stop::primary_for_vehicle(s.shared(), st.cast_mut(), v).is_null();
    }

    can_engine_use_station(v.engine_type(), st, s)
}

/// `IsCompanyBuildableVehicleType`: is the given vehicle type buildable by a company?
#[must_use]
pub fn is_company_buildable_vehicle_type(kind: VehicleType) -> bool {
    kind < VehicleType::VEH_COMPANY_END
}
