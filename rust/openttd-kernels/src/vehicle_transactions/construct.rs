/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Road vehicle and aircraft construction: original `CmdBuildRoadVehicle` and
//! `CmdBuildAircraft`. Native leaves allocate shells and store fields; Rust owns
//! the checks, RNG draws, articulated assembly, capacities, caches and chain.
#![allow(clippy::many_single_char_names)]
use super::{ERROR, articulated, build};
use crate::aircraft::{AIR_AIRCRAFT, AIR_HELICOPTER};
use crate::airport::HANGAR;
use crate::command_type::DoCommandFlag;
use crate::engine_type::AIR_CTOL;
use crate::services::CommandCost;
use crate::station::Station;
use crate::vehicle::{OpenTTDEngine, OpenTTDVehicle, VehicleType};
use std::ptr;

const HRS_ROTOR_STOPPED: u8 = 0;

#[repr(C)]
pub struct AircraftInfo {
    pub image: u8,
    pub passengers: u16,
    pub mail: u16,
    pub subtype: u8,
}
#[repr(C)]
pub struct Services {
    pub articulated: &'static articulated::Services,
    pub powered_roadtypes: extern "C" fn(*const OpenTTDEngine) -> u64,
    pub any_roadtype: extern "C" fn(u32, u64) -> bool,
    pub new_road: extern "C" fn(u32) -> *mut (),
    pub road_front: extern "C" fn(*mut (), *const OpenTTDEngine),
    pub station: extern "C" fn(u32) -> *const Station,
    pub hangar_tile:
        extern "C" fn(*const Station, u32, *const crate::station_cmd::AirportSpec) -> u32,
    pub new_aircraft: extern "C" fn() -> *mut (),
    pub aircraft_place: extern "C" fn(*mut (), *mut (), u32),
    pub aircraft_info: extern "C" fn(*const OpenTTDEngine) -> AircraftInfo,
    pub aircraft_properties: extern "C" fn(*mut (), *mut (), *const OpenTTDEngine, u8),
    pub rotor: extern "C" fn(*mut (), *mut (), *mut ()),
    pub mail_capacity_address: extern "C" fn(*mut ()) -> *mut u16,
    pub road_capacity: extern "C" fn(*mut (), *mut (), u16),
    pub set_cargo_type: extern "C" fn(*mut OpenTTDVehicle, u8),
    pub engine_common: extern "C" fn(*mut (), *const OpenTTDEngine),
    pub service_dates: extern "C" fn(*mut (), u16),
    pub build_stamp: extern "C" fn(*mut ()),
    pub exclusive_preview: extern "C" fn(*const OpenTTDEngine) -> bool,
    pub service_flags: extern "C" fn(*mut (), bool, bool),
    pub wrong_depot: u32,
}
/// # Safety
/// `v` is a live shell for the duration of the native call.
unsafe fn vehicle_ref(v: *mut ()) -> crate::vehicle::VehicleRef {
    crate::vehicle::VehicleRef(unsafe { ptr::NonNull::new_unchecked(v.cast()) })
}
/// Original `CmdBuildRoadVehicle`.
/// # Safety
/// Live native cost and engine; new shells stay live, and no owner access spans a callback.
pub(crate) unsafe fn road(
    s: &build::Services,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    e: *const OpenTTDEngine,
) -> *mut () {
    let k = s.construct;
    let a = k.articulated;
    let r = s.refit;
    let b = r.common.base;
    if !(k.any_roadtype)(tile, (k.powered_roadtypes)(e)) {
        (b.cost.error)(out, k.wrong_depot);
        return ptr::null_mut();
    }
    (b.cost.zero)(out);
    if flags & DoCommandFlag::EXECUTE == 0 {
        return ptr::null_mut();
    }
    crate::witness::hit(crate::witness::VT_ROAD);
    let company = (b.shared.current_company)();
    let v = (k.new_road)(tile);
    unsafe { crate::road::set_depot_state((r.road_state)(v.cast())) };
    let info = (a.road_info)(e, true);
    (a.set_part)(v, info.image, (a.default_cargo)(e), info.capacity, true);
    (k.engine_common)(v, e);
    (r.common.vehicles.set_max_age)(unsafe { vehicle_ref(v) }, unsafe {
        crate::engine_capabilities::openttd_rust_engine_life_days(r.capabilities, e)
    });
    (k.service_dates)(v, b.interval_default(company, VehicleType::ROAD.0) as u16);
    (k.build_stamp)(v);
    (a.random_bits)(v, b.shared.random() as u16);
    (k.road_front)(v, e);
    (k.service_flags)(v, (k.exclusive_preview)(e), (b.interval_percent)(company));
    unsafe { articulated::add(a, v) };
    (r.invalidate_chain)(v.cast());
    let mut u = v;
    while !u.is_null() {
        let capacity = unsafe {
            crate::engine_capabilities::openttd_rust_engine_capacity(
                r.capabilities,
                (r.vehicle_engine)(u.cast()),
                u.cast(),
                ptr::null_mut(),
            )
        };
        (k.road_capacity)(v, u, capacity as u16);
        u = unsafe { super::refit::next(r, u) };
    }
    unsafe {
        crate::road::openttd_rust_road_update_cache(
            vehicle_ref(v),
            (r.road_state)(v.cast()),
            false,
            r.road,
        );
    }
    if (r.road_acceleration)() != 0 {
        crate::ground_vehicle::cargo_changed(
            r.road_ground.physics,
            (r.road_ground.physics.first)(unsafe { vehicle_ref(v) }),
        );
    }
    unsafe {
        a.position(v);
    }
    unsafe { articulated::consistency(a, v) };
    v
}

/// Original `CmdBuildAircraft`, including its shadow and helicopter rotor.
/// # Safety
/// As `road`; the station of a hangar tile is live for the whole call.
pub(crate) unsafe fn aircraft(
    s: &build::Services,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    e: *const OpenTTDEngine,
) -> *mut () {
    let k = s.construct;
    let a = k.articulated;
    let r = s.refit;
    let c = r.common;
    let b = c.base;
    let st = (k.station)(tile);
    if !crate::vehicle_func::can_engine_use_station(
        (crate::newgrf_airport::runtime().engine_id)(e),
        st.cast(),
        crate::newgrf_airport::runtime(),
    ) {
        (b.cost.error)(out, ERROR);
        return ptr::null_mut();
    }
    let spec =
        crate::newgrf_airport::station_spec(st.cast_mut().cast(), crate::newgrf_airport::runtime());
    let tile = (k.hangar_tile)(st, tile, spec);
    (b.cost.zero)(out);
    if flags & DoCommandFlag::EXECUTE == 0 {
        return ptr::null_mut();
    }
    crate::witness::hit(crate::witness::VT_AIRCRAFT);
    let company = (b.shared.current_company)();
    let v = (k.new_aircraft)();
    let u = (k.new_aircraft)();
    (k.aircraft_place)(v, u, tile);
    let info = (k.aircraft_info)(e);
    (a.set_part)(v, info.image, (a.default_cargo)(e), info.passengers, true);
    (r.set_refit_cap)(u.cast(), 0);
    let mail = crate::cargo_registry::openttd_rust_cargo_by_label(crate::cargo_type::CT_MAIL);
    if (c.valid_cargo)(mail) {
        (k.set_cargo_type)(u.cast(), mail);
        (r.set_capacity)(u.cast(), info.mail);
    }
    (b.clear_name)(v);
    (k.engine_common)(v, e);
    (r.common.vehicles.set_max_age)(unsafe { vehicle_ref(v) }, unsafe {
        crate::engine_capabilities::openttd_rust_engine_life_days(r.capabilities, e)
    });
    let subtype = if info.subtype & AIR_CTOL != 0 {
        AIR_AIRCRAFT
    } else {
        AIR_HELICOPTER
    };
    (k.aircraft_properties)(v, u, e, subtype);
    let hv = (r.aircraft_handle)(v.cast());
    let hu = (r.aircraft_handle)(u.cast());
    unsafe {
        (r.aircraft.update_delta)(hv);
        (r.aircraft.update_delta)(hu);
    }
    let pos = crate::airport::get_vehicle_pos_on_build(tile, crate::newgrf_airport::runtime());
    let target = (b.shared.station_index)(tile);
    // SAFETY: The new aircraft's sole private owner; no callback runs meanwhile.
    unsafe {
        (*hv.state).pos = pos;
        (*hv.state).state = HANGAR;
        (*hv.state).previous_pos = pos;
        (*hv.state).targetairport = target;
    }
    (k.service_dates)(
        v,
        b.interval_default(company, VehicleType::AIRCRAFT.0) as u16,
    );
    (k.build_stamp)(v);
    (k.build_stamp)(u);
    (a.random_bits)(v, b.shared.random() as u16);
    (a.random_bits)(u, b.shared.random() as u16);
    (k.service_flags)(v, (k.exclusive_preview)(e), (b.interval_percent)(company));
    (r.invalidate_chain)(v.cast());
    // Source passes the live mail shell field, so callbacks see each optional write immediately.
    let capacity = unsafe {
        crate::engine_capabilities::openttd_rust_engine_capacity(
            r.capabilities,
            e,
            v.cast(),
            (k.mail_capacity_address)(u),
        )
    };
    (r.set_capacity)(v.cast(), capacity as u16);
    (r.invalidate_chain)(v.cast());
    unsafe { crate::aircraft::openttd_rust_aircraft_cache(r.aircraft, hv, true) };
    unsafe {
        a.position(v);
        a.position(u);
    }
    if subtype == AIR_HELICOPTER {
        crate::witness::hit(crate::witness::VT_ROTOR);
        let w = (k.new_aircraft)();
        (k.rotor)(w, v, u);
        (a.random_bits)(w, b.shared.random() as u16);
        let hw = (r.aircraft_handle)(w.cast());
        // SAFETY: As above, for the new rotor's private owner.
        unsafe {
            (*hw.state).state = HRS_ROTOR_STOPPED;
            (r.aircraft.update_delta)(hw);
        }
        unsafe {
            a.position(w);
        }
    }
    v
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAircraftInfo" ["probe"] => AircraftInfo { image, passengers, mail, subtype, };
    "OpenTTDVehicleConstructServices" ["probe"] => Services {
        articulated, powered_roadtypes, any_roadtype, new_road, road_front, station,
        hangar_tile, new_aircraft, aircraft_place, aircraft_info, aircraft_properties,
        rotor, mail_capacity_address, road_capacity, set_cargo_type, engine_common, service_dates,
        build_stamp, exclusive_preview, service_flags, wrong_depot,
    };
}
