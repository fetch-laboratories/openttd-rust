/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared constants from `vehicle_func.h`.
#![allow(unsafe_code)]
/// Native custom sprite sentinel.
pub const CUSTOM_VEHICLE_SPRITENUM: u8 = 0xfd;
/// Native custom sprite sentinel.
pub const CUSTOM_VEHICLE_SPRITENUM_REVERSED: u8 = 0xfe;

/// Original engine overload of `CanVehicleUseStation`. Facility/type gates stay
/// before airport selection; no world or engine borrow spans a native service.
pub(crate) fn can_engine_use_station(
    engine: u16,
    st: *const crate::station::BaseStation,
    s: &crate::newgrf_airport::Services,
) -> bool {
    let e = (s.engine_if_valid)(engine);
    (s.check)(!e.is_null());
    let kind = crate::vehicle::EngineRef::of(e).kind().0;
    match kind {
        0 => {
            crate::station_record::facilities(st.cast_mut().cast())
                & crate::station_type::FACILITY_TRAIN
                != 0
        }
        1 => {
            crate::station_record::facilities(st.cast_mut().cast())
                & (crate::station_type::FACILITY_BUS | crate::station_type::FACILITY_TRUCK)
                != 0
        }
        2 => {
            crate::station_record::facilities(st.cast_mut().cast())
                & crate::station_type::FACILITY_DOCK
                != 0
        }
        3 => {
            crate::station_record::facilities(st.cast_mut().cast())
                & crate::station_type::FACILITY_AIRPORT
                != 0
                && {
                    let fta = crate::newgrf_airport::station_fta(st.cast_mut(), s);
                    let flag = if crate::vehicle::EngineRef::of(e).aircraft_subtype()
                        & crate::engine_type::AIR_CTOL
                        != 0
                    {
                        crate::airport::FTA_AIRPLANES
                    } else {
                        crate::airport::FTA_HELICOPTERS
                    };
                    (s.fta_flags)(fta) & flag != 0
                }
        }
        _ => false,
    }
}
/// Original vehicle overload, preserving its road-vehicle early return.
/// SAFETY: original live vehicle/station preconditions and game-state serialization.
pub(crate) unsafe fn can_vehicle_use_station(
    v: *const crate::vehicle::OpenTTDVehicle,
    st: *const crate::station::BaseStation,
    s: &crate::newgrf_airport::Services,
) -> bool {
    let vehicle =
        crate::vehicle::VehicleRef(unsafe { std::ptr::NonNull::new_unchecked(v.cast_mut()) });
    if vehicle.vehicle_type() == 1 {
        return !crate::road_stop::primary_for_vehicle(s.shared(), st.cast_mut(), vehicle)
            .is_null();
    }
    can_engine_use_station(vehicle.engine_id(), st, s)
}

/// Original type admission for a company's vehicles.
#[must_use]
pub const fn is_company_buildable_vehicle_type(kind: u8) -> bool {
    kind < crate::vehicle_type::VEH_COMPANY_END
}
