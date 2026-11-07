/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Constants shared from `station_base.h`.
#![allow(unsafe_code)]

/// `INITIAL_STATION_RATING`.
pub const INITIAL_STATION_RATING: u8 = 175;

/// Original nested `GoodsEntry` status vocabulary.
pub mod goods_entry {
    /// `GoodsEntry::State` bit positions.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct State(pub u8);
    #[allow(non_upper_case_globals)]
    impl State {
        /// Currently accepts final deliveries; refreshed from surrounding-tile
        /// acceptance >= 8/8 every `STATION_ACCEPTANCE_TICKS`.
        pub const Acceptance: Self = Self(0);
        /// Has a rating because cargo has waited here, supplied by surrounding
        /// tiles or unloaded/transferred without final delivery. Cleared after
        /// 255 * `STATION_RATING_TICKS` without a pickup.
        pub const Rating: Self = Self(1);
        /// A vehicle has made a final delivery here. Never cleared.
        pub const EverAccepted: Self = Self(2);
        /// Final delivery last month; copied from `CurrentMonth` at month start.
        pub const LastMonth: Self = Self(3);
        /// Final delivery this month; reset at the beginning of each month.
        pub const CurrentMonth: Self = Self(4);
        /// Final delivery in the current `STATION_ACCEPTANCE_TICKS` interval;
        /// reset every `STATION_ACCEPTANCE_TICKS` ticks.
        pub const AcceptedBigtick: Self = Self(5);
    }

    /// `GoodsEntry::States`, retaining all bits in the saved status byte.
    #[repr(transparent)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct States(pub u8);
    impl States {
        /// Native `EnumBitSet::Test`.
        #[must_use]
        pub const fn test(self, element: State) -> bool {
            self.0 & (1 << element.0) != 0
        }
    }
}

/// `GoodsEntry::HasRating`.
pub(crate) fn has_rating(status: u8) -> bool {
    goods_entry::States(status).test(goods_entry::State::Rating)
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
    random: impl FnMut() -> u32,
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
        crate::cargo_flow::flow_via(
            entry.flow,
            2,
            excluded,
            excluded2,
            &raw mut restricted,
            random,
        )
    }
}
