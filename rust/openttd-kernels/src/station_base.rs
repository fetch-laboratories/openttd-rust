/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Constants shared from `station_base.h`.
#![allow(unsafe_code)]
use crate::station_type::StationID;
use crate::tile_type::TileIndex;

/// `INITIAL_STATION_RATING`: the initial rating of a cargo at a station.
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

    /// `GoodsEntry::HasRating`: does this cargo have a rating at this station?
    /// True if the cargo has a rating, i.e. cargo has been moved to the station.
    #[must_use]
    pub const fn has_rating(status: States) -> bool {
        status.test(State::Rating)
    }
}

/// Temporary raw adapter for `station_service.rs`; delegates to [`goods_entry::has_rating`].
pub(crate) fn has_rating(status: u8) -> bool {
    goods_entry::has_rating(goods_entry::States(status))
}
/// `GoodsEntry::HasVehicleEverTriedLoading`: reports whether a vehicle has
/// ever tried to load the cargo at this station.
pub(crate) fn has_vehicle_ever_tried_loading(last_speed: u8) -> bool {
    last_speed != 0
}

/// `Airport::GetSpec`: get the `AirportSpec` from the airport type of this
/// airport. If there is no airport (`tile == INVALID_TILE`) then return the
/// dummy `AirportSpec`.
pub(crate) fn airport_get_spec(
    tile: TileIndex,
    kind: u8,
    s: &crate::newgrf_airport::Services,
) -> *const crate::station_cmd::AirportSpec {
    if tile == TileIndex::INVALID {
        s.dummy_spec
    } else {
        crate::newgrf_airport::get(kind, s)
    }
}
/// Temporary raw adapter for `newgrf_airport.rs` and `aircraft.rs`.
pub(crate) fn airport_get_spec_raw(
    tile: u32,
    kind: u8,
    s: &crate::newgrf_airport::Services,
) -> *const crate::station_cmd::AirportSpec {
    airport_get_spec(TileIndex(tile), kind, s)
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
/// `GoodsEntry::GetVia(source, excluded, excluded2)`: get the best next hop
/// for a cargo packet from station `source`, optionally excluding one or two
/// stations. Returns the chosen next hop or `StationID::Invalid()` if none was
/// found.
/// # Safety
/// The map is live and serialized; the random callback changes only the RNG.
pub(crate) unsafe fn goods_get_via(
    map: *mut crate::cargo_flow::FlowMap,
    source: StationID,
    excluded: StationID,
    excluded2: StationID,
    random: impl FnMut() -> u32,
) -> StationID {
    // `!HasData()`.
    if map.is_null() {
        return StationID::INVALID;
    }
    // The fork's flow-map lookup: operation 1 is `flows.find(source)`.
    let entry = unsafe { crate::cargo_flow::openttd_rust_flow_map_at(map, u32::from(source.0), 1) };
    if entry.found == 0 {
        return StationID::INVALID;
    }
    let mut restricted = 0;
    // The fork's flow dispatch: mode 2 is `FlowStat::GetVia(excluded, excluded2)`.
    StationID(unsafe {
        crate::cargo_flow::flow_via(
            entry.flow,
            2,
            excluded.0,
            excluded2.0,
            &raw mut restricted,
            random,
        )
    })
}
/// Temporary raw adapter for `cargo_storage.rs`; delegates to [`goods_get_via`].
/// # Safety
/// As [`goods_get_via`].
pub(crate) unsafe fn goods_get_via_raw(
    map: *mut crate::cargo_flow::FlowMap,
    source: u16,
    excluded: u16,
    excluded2: u16,
    random: impl FnMut() -> u32,
) -> u16 {
    unsafe {
        goods_get_via(
            map,
            StationID(source),
            StationID(excluded),
            StationID(excluded2),
            random,
        )
    }
    .0
}

/// `Station::TileBelongsToRailStation` and its `Waypoint` override: whether
/// `tile` is a rail station (or rail waypoint) tile of station `index`.
pub(crate) fn tile_belongs_to_rail_station(
    s: &crate::services::Services,
    waypoint: bool,
    index: StationID,
    tile: TileIndex,
) -> bool {
    let world = crate::map_func::World::new(s);
    (if waypoint {
        world.is_rail_waypoint_tile(tile)
    } else {
        world.is_rail_station_tile(tile)
    }) && world.get_station_index(tile) == index
}
/// `Station::TileBelongsToAirport`: whether `tile` is an airport tile of
/// station `index`.
pub(crate) fn tile_belongs_to_airport(
    s: &crate::services::Services,
    index: StationID,
    tile: TileIndex,
) -> bool {
    let world = crate::map_func::World::new(s);
    world.is_airport_tile(tile) && world.get_station_index(tile) == index
}
