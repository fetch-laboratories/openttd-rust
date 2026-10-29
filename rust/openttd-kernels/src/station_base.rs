/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Constants shared from `station_base.h`.
#![allow(unsafe_code)]

/// `INITIAL_STATION_RATING`.
pub const INITIAL_STATION_RATING: u8 = 175;

/// `GoodsEntry::State` positions from the native header.
pub const GE_ACCEPTANCE: u8 = 0;
pub const GE_RATING: u8 = 1;
pub const GE_EVER_ACCEPTED: u8 = 2;
pub const GE_LAST_MONTH: u8 = 3;
pub const GE_CURRENT_MONTH: u8 = 4;
pub const GE_ACCEPTED_BIGTICK: u8 = 5;
/// `GoodsEntry::HasRating`.
pub(crate) fn has_rating(status: u8) -> bool {
    status & (1 << GE_RATING) != 0
}
/// `GoodsEntry::HasVehicleEverTriedLoading`.
pub(crate) fn has_vehicle_ever_tried_loading(last_speed: u8) -> bool {
    last_speed != 0
}

/// `Airport::GetSpec`, including the dummy spec when there is no airport.
pub(crate) fn airport_get_spec(
    tile: u32,
    kind: u8,
    s: &crate::newgrf_airport::Services,
) -> *const crate::station_cmd::AirportSpec {
    if tile == crate::tile_type::INVALID_TILE {
        s.dummy_spec
    } else {
        crate::newgrf_airport::get(kind, s)
    }
}

/// `Airport::HasHangar`, after its canonical spec selection.
pub(crate) fn airport_has_hangar(
    st: *const crate::station::BaseStation,
    s: &crate::newgrf_airport::Services,
) -> bool {
    (s.has_hangar)(st, crate::newgrf_airport::station_spec(st.cast_mut(), s))
}

/// Native `GoodsEntry`; storage remains in the station shell.
#[repr(C)]
pub struct GoodsEntry {
    _private: [u8; 0],
}
/// `GoodsEntry::GetVia(source, excluded, excluded2)` over its resolved flow map.
/// # Safety
/// The map is live and serialized; the random callback changes only the RNG.
pub(crate) unsafe fn goods_get_via(
    map: *mut crate::cargo_flow::FlowMap,
    source: u16,
    excluded: u16,
    excluded2: u16,
    random: extern "C" fn() -> u32,
) -> u16 {
    if map.is_null() {
        return crate::station_type::INVALID_STATION;
    }
    let entry = unsafe { crate::cargo_flow::openttd_rust_flow_map_at(map, u32::from(source), 1) };
    if entry.found == 0 {
        return crate::station_type::INVALID_STATION;
    }
    let mut restricted = 0;
    unsafe {
        crate::cargo_flow::openttd_rust_flow_via(
            entry.flow,
            2,
            excluded,
            excluded2,
            &raw mut restricted,
            random,
        )
    }
}
