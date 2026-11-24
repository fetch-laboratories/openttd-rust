/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `vehicle_func.h`: the custom sprite numbers, `IsCompanyBuildableVehicleType`,
//! both `CanVehicleUseStation` overloads (`vehicle.cpp`), `GetNewVehiclePosResult` and
//! the loops over `VehiclesOnTile` and `VehiclesNearTileXY`.
//!
//! The loops keep the native iterators' lifetime contract: a visitor may reenter owners but
//! cannot delete the current shell, and no owner reference spans a visitor.
use crate::airport::{AirportFTAClassFlag, AirportFTAClassFlags};
use crate::engine_type::{AIR_CTOL, EngineID};
use crate::map_func::Map;
use crate::station_type::{StationFacilities, StationFacility};
use crate::tile_type::TileIndex;
use crate::vehicle::occupancy::{Near, OnTile};
use crate::vehicle::{EngineRef, VehicleRef, VehicleServices};
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

/// `CountVehiclesInChain`: count the number of vehicles in a chain.
pub(crate) fn count_vehicles_in_chain(v: VehicleRef) -> u32 {
    let mut count = 0_u32;
    let mut v = Some(v);
    while let Some(u) = v {
        count += 1;
        v = u.next();
    }
    count
}

/// Position information of a vehicle after it moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GetNewVehiclePosResult {
    /// x position of the vehicle after moving.
    pub x: i32,
    /// y position of the vehicle after moving.
    pub y: i32,
    /// Current tile of the vehicle.
    pub old_tile: TileIndex,
    /// Tile of the vehicle after moving.
    pub new_tile: TileIndex,
}

/// `for (Vehicle *v : VehiclesOnTile(tile))`: iterate over all vehicles on a tile.
/// The order is non-deterministic. You have to make sure, that your processing is not order
/// dependant.
#[inline]
pub(crate) fn vehicles_on_tile(
    vs: &VehicleServices,
    map: Map,
    tile: TileIndex,
    mut visit: impl FnMut(VehicleRef),
) {
    has_vehicle_on_tile(vs, map, tile, |v| {
        visit(v);
        false
    });
}
/// [`vehicles_on_tile`] for callers in files no pass has finished.
#[inline]
pub(crate) fn vehicles_on_tile_raw(
    vs: &VehicleServices,
    map: Map,
    tile: u32,
    visit: impl FnMut(VehicleRef),
) {
    vehicles_on_tile(vs, map, TileIndex(tile), visit);
}

/// `HasVehicleOnTile`: loop over vehicles on a tile, and check whether a predicate is true
/// for any of them.
#[allow(unsafe_code)]
#[inline]
pub(crate) fn has_vehicle_on_tile(
    vs: &VehicleServices,
    map: Map,
    tile: TileIndex,
    mut predicate: impl FnMut(VehicleRef) -> bool,
) -> bool {
    // SAFETY: The module's iterator contract; no index borrow spans `predicate`.
    unsafe {
        let mut it = OnTile::new(vs, map, tile.0);
        while let Some(v) = it.get() {
            if predicate(v) {
                return true;
            }
            it.advance(vs);
        }
    }
    false
}
/// [`has_vehicle_on_tile`] for callers in files no pass has finished.
#[inline]
pub(crate) fn has_vehicle_on_tile_raw(
    vs: &VehicleServices,
    map: Map,
    tile: u32,
    predicate: impl FnMut(VehicleRef) -> bool,
) -> bool {
    has_vehicle_on_tile(vs, map, TileIndex(tile), predicate)
}

/// `for (Vehicle *v : VehiclesNearTileXY(x, y, max_dist))`: iterate over all vehicles near a
/// given world coordinate.
/// This only works for vehicles with proper `Vehicle::Tile`, so only ground vehicles outside
/// wormholes. The order is non-deterministic. You have to make sure, that your processing is
/// not order dependant.
#[inline]
pub(crate) fn vehicles_near_tile_xy(
    vs: &VehicleServices,
    x: i32,
    y: i32,
    max_dist: u32,
    mut visit: impl FnMut(VehicleRef),
) {
    has_vehicle_near_tile_xy(vs, x, y, max_dist, |v| {
        visit(v);
        false
    });
}

/// `HasVehicleNearTileXY`: loop over vehicles near a given world coordinate, and check
/// whether a predicate is true for any of them.
/// This only works for vehicles with proper `Vehicle::Tile`, so only ground vehicles outside
/// wormholes.
#[allow(unsafe_code)]
#[inline]
pub(crate) fn has_vehicle_near_tile_xy(
    vs: &VehicleServices,
    x: i32,
    y: i32,
    max_dist: u32,
    mut predicate: impl FnMut(VehicleRef) -> bool,
) -> bool {
    // SAFETY: The module's iterator contract; no index borrow spans `predicate`.
    unsafe {
        let mut it = Near::new(vs, x, y, max_dist);
        while let Some(v) = it.get() {
            if predicate(v) {
                return true;
            }
            it.advance(vs);
        }
    }
    false
}

#[cfg(test)]
#[allow(unsafe_code, reason = "the test links and frees its own records")]
mod chain_tests {
    use crate::vehicle::VehicleRef;
    use crate::vehicle_type::VehicleType;

    /// `CountVehiclesInChain` counts the given vehicle and each `Next()` after it.
    #[test]
    fn count_vehicles_in_chain_counts_from_the_given_vehicle_to_the_end() {
        let records: Vec<_> = (0..4)
            .map(|_| {
                crate::vehicle_base::openttd_rust_vehicle_record_new(
                    std::ptr::null_mut(),
                    VehicleType::VEH_TRAIN,
                )
            })
            .collect();
        for pair in records.windows(2) {
            // SAFETY: The test's fresh records.
            unsafe { (*pair[0]).next = pair[1] };
        }
        for (i, &record) in records.iter().enumerate() {
            assert_eq!(
                super::count_vehicles_in_chain(VehicleRef::of(record)),
                4 - i as u32
            );
        }
        for record in records {
            // SAFETY: Each record came from `openttd_rust_vehicle_record_new`.
            unsafe { crate::vehicle_base::openttd_rust_vehicle_record_delete(record) };
        }
    }
}
