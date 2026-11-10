/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Road vehicle and aircraft construction: original `CmdBuildRoadVehicle` and
//! `CmdBuildAircraft`. Native leaves allocate shells and store fields; Rust owns
//! the checks, RNG draws, articulated assembly, capacities, caches and chain.
#![allow(clippy::many_single_char_names)]
use super::{ERROR, articulated, build, vehicle_ref};
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
pub struct Services {
    pub articulated: &'static articulated::Services,
    pub new_road: extern "C" fn(u32) -> *mut (),
    pub road_front: extern "C" fn(*mut (), crate::services::RoadType, u64),
    pub station: extern "C" fn(u32) -> *const Station,
    pub hangar_tile:
        extern "C" fn(*const Station, u32, *const crate::station_cmd::AirportSpec) -> u32,
    pub new_aircraft: extern "C" fn() -> *mut (),
    pub aircraft_place: extern "C" fn(*mut (), *mut (), u32),
    pub aircraft_properties: extern "C" fn(*mut (), *mut (), u8, u16, u8),
    pub rotor: extern "C" fn(*mut (), *mut (), *mut ()),
    pub mail_capacity_address: extern "C" fn(*mut ()) -> *mut u16,
    pub road_capacity: extern "C" fn(*mut (), *mut (), u16),
    pub set_cargo_type: extern "C" fn(*mut OpenTTDVehicle, u8),
    pub engine_common: extern "C" fn(*mut (), u16, u16, u16),
    pub service_dates: extern "C" fn(*mut (), u16),
    pub build_stamp: extern "C" fn(*mut ()),
    pub service_flags: extern "C" fn(*mut (), bool, bool),
    pub wrong_depot: u32,
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
    if !crate::road_map::has_tile_any_road_type(
        &crate::map_func::World::new(b.shared),
        tile,
        crate::vehicle::EngineRef::of(e).powered_roadtypes(b.shared),
    ) {
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
    unsafe {
        crate::road::set_depot_state((r.road_state)(
            super::vehicle_ref(v, r.common.vehicles).0.as_ptr(),
        ));
    };
    let engine = crate::vehicle::EngineRef::of(e);
    (a.set_part)(
        v,
        engine.ground_image(VehicleType::ROAD),
        engine.default_cargo(),
        u16::from(engine.ground_capacity(VehicleType::ROAD)),
        true,
    );
    (k.engine_common)(
        v,
        (r.capabilities.id)(e),
        engine.reliability(),
        engine.reliability_decay(),
    );
    (r.common.vehicles.set_max_age)(unsafe { vehicle_ref(v, r.common.vehicles) }, unsafe {
        crate::engine_capabilities::openttd_rust_engine_life_days(r.capabilities, e)
    });
    (k.service_dates)(
        v,
        b.interval_default_raw(company, VehicleType::ROAD.0) as u16,
    );
    (k.build_stamp)(v);
    (a.random_bits)(v, b.shared.random() as u16);
    (k.road_front)(
        v,
        crate::services::RoadType(engine.roadtype()),
        engine.powered_roadtypes(b.shared),
    );
    (k.service_flags)(v, engine.exclusive_preview(), (b.interval_percent)(company));
    unsafe { articulated::add(a, v) };
    (r.invalidate_chain)(
        unsafe { super::vehicle_ref(v, r.common.vehicles) }
            .0
            .as_ptr(),
    );
    let mut u = v;
    while !u.is_null() {
        let capacity = unsafe {
            crate::engine_capabilities::openttd_rust_engine_capacity(
                r.capabilities,
                r.vehicle_engine(super::vehicle_ref(u, r.common.vehicles).0.as_ptr()),
                super::vehicle_ref(u, r.common.vehicles).0.as_ptr(),
                ptr::null_mut(),
            )
        };
        (k.road_capacity)(v, u, capacity as u16);
        u = unsafe { super::refit::next(r, u) };
    }
    unsafe {
        crate::road::openttd_rust_road_update_cache(
            vehicle_ref(v, r.common.vehicles),
            (r.road_state)(super::vehicle_ref(v, r.common.vehicles).0.as_ptr()),
            false,
            r.road,
        );
    }
    if r.road_acceleration_model() != 0 {
        crate::ground_vehicle::cargo_changed(
            r.road_ground.physics,
            unsafe { vehicle_ref(v, r.common.vehicles) }.first(),
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
#[allow(clippy::too_many_lines)]
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
    let engine = crate::vehicle::EngineRef::of(e);
    (a.set_part)(
        v,
        engine.aircraft_image(),
        engine.default_cargo(),
        engine.passenger_capacity(),
        true,
    );
    crate::vehicle::VehicleRef::of(
        unsafe { super::vehicle_ref(u, r.common.vehicles) }
            .0
            .as_ptr(),
    )
    .write_refit_cap(0);
    let mail = crate::cargo_registry::openttd_rust_cargo_by_label(crate::cargo_type::CT_MAIL);
    if (c.valid_cargo)(mail) {
        (k.set_cargo_type)(
            unsafe { super::vehicle_ref(u, r.common.vehicles) }
                .0
                .as_ptr(),
            mail,
        );
        crate::vehicle::VehicleRef::of(
            unsafe { super::vehicle_ref(u, r.common.vehicles) }
                .0
                .as_ptr(),
        )
        .write_cargo_cap(u16::from(engine.mail_capacity()));
    }
    (b.clear_name)(v);
    (k.engine_common)(
        v,
        (r.capabilities.id)(e),
        engine.reliability(),
        engine.reliability_decay(),
    );
    (r.common.vehicles.set_max_age)(unsafe { vehicle_ref(v, r.common.vehicles) }, unsafe {
        crate::engine_capabilities::openttd_rust_engine_life_days(r.capabilities, e)
    });
    let subtype = if engine.aircraft_subtype() & AIR_CTOL != 0 {
        AIR_AIRCRAFT
    } else {
        AIR_HELICOPTER
    };
    (k.aircraft_properties)(
        v,
        u,
        engine.aircraft_acceleration(),
        (r.capabilities.id)(e),
        subtype,
    );
    unsafe { c.vehicles.set_next_shell(v.cast(), u.cast()) };
    let hv = (r.aircraft_handle)(
        unsafe { super::vehicle_ref(v, r.common.vehicles) }
            .0
            .as_ptr(),
    );
    let hu = (r.aircraft_handle)(
        unsafe { super::vehicle_ref(u, r.common.vehicles) }
            .0
            .as_ptr(),
    );
    (r.common.vehicles.update_delta)(crate::vehicle::VehicleRef::of(hv.handle));
    (r.common.vehicles.update_delta)(crate::vehicle::VehicleRef::of(hu.handle));
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
        b.interval_default_raw(company, VehicleType::AIRCRAFT.0) as u16,
    );
    (k.build_stamp)(v);
    (k.build_stamp)(u);
    (a.random_bits)(v, b.shared.random() as u16);
    (a.random_bits)(u, b.shared.random() as u16);
    (k.service_flags)(v, engine.exclusive_preview(), (b.interval_percent)(company));
    (r.invalidate_chain)(
        unsafe { super::vehicle_ref(v, r.common.vehicles) }
            .0
            .as_ptr(),
    );
    // Source passes the live mail shell field, so callbacks see each optional write immediately.
    let capacity = unsafe {
        crate::engine_capabilities::openttd_rust_engine_capacity(
            r.capabilities,
            e,
            super::vehicle_ref(v, r.common.vehicles).0.as_ptr(),
            (k.mail_capacity_address)(u),
        )
    };
    crate::vehicle::VehicleRef::of(
        unsafe { super::vehicle_ref(v, r.common.vehicles) }
            .0
            .as_ptr(),
    )
    .write_cargo_cap(capacity as u16);
    (r.invalidate_chain)(
        unsafe { super::vehicle_ref(v, r.common.vehicles) }
            .0
            .as_ptr(),
    );
    unsafe { crate::aircraft::openttd_rust_aircraft_cache(r.aircraft, hv, true) };
    unsafe {
        a.position(v);
        a.position(u);
    }
    if subtype == AIR_HELICOPTER {
        crate::witness::hit(crate::witness::VT_ROTOR);
        let w = (k.new_aircraft)();
        (k.rotor)(w, v, u);
        unsafe { c.vehicles.set_next_shell(u.cast(), w.cast()) };
        (a.random_bits)(w, b.shared.random() as u16);
        let hw = (r.aircraft_handle)(
            unsafe { super::vehicle_ref(w, r.common.vehicles) }
                .0
                .as_ptr(),
        );
        // SAFETY: As above, for the new rotor's private owner.
        unsafe {
            (*hw.state).state = HRS_ROTOR_STOPPED;
            (r.common.vehicles.update_delta)(crate::vehicle::VehicleRef::of(hw.handle));
        }
        unsafe {
            a.position(w);
        }
    }
    v
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDVehicleConstructServices" ["probe"] => Services {
        articulated, new_road, road_front, station,
        hangar_tile, new_aircraft, aircraft_place, aircraft_properties,
        rotor, mail_capacity_address, road_capacity, set_cargo_type, engine_common, service_dates,
        build_stamp, service_flags, wrong_depot,
    };
}
