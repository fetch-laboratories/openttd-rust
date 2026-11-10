/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `tile_cmd.h`: vehicle tile-admission results and the tile-type cargo procedures.
#![allow(unsafe_code)]
use crate::cargo_type::{CargoArray, CargoTypes};
use crate::tile_type::{TileIndex, TileType};

/// Native `VehicleEnterTileState`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleEnterTileState(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "VehicleEnterTileState enumerator spelling"
)]
impl VehicleEnterTileState {
    /// The vehicle entered a station
    pub const EnteredStation: Self = Self(0);
    /// The vehicle either entered a bridge, tunnel or depot tile (this includes the last
    /// tile of the bridge/tunnel)
    pub const EnteredWormhole: Self = Self(1);
    /// The vehicle cannot enter the tile
    pub const CannotEnter: Self = Self(2);
}
/// Native `VehicleEnterTileStates`; elements are `VehicleEnterTileState` bit positions.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleEnterTileStates(pub u8);
impl VehicleEnterTileStates {
    /// Native bit set containing one element.
    #[must_use]
    pub const fn from_element(element: VehicleEnterTileState) -> Self {
        Self(1 << element.0)
    }
}
/// Raw adapter: `VehicleEnterTileState::EnteredStation` in `VehicleEnterTileStates`, read by
/// `movement.rs`, `road.rs` and `town/house.rs` until their passes.
pub const ENTERED_STATION: u8 =
    VehicleEnterTileStates::from_element(VehicleEnterTileState::EnteredStation).0;
/// Raw adapter: `VehicleEnterTileState::EnteredWormhole` in `VehicleEnterTileStates`.
pub const ENTERED_WORMHOLE: u8 =
    VehicleEnterTileStates::from_element(VehicleEnterTileState::EnteredWormhole).0;
/// Raw adapter: `VehicleEnterTileState::CannotEnter` in `VehicleEnterTileStates`.
pub const CANNOT_ENTER: u8 =
    VehicleEnterTileStates::from_element(VehicleEnterTileState::CannotEnter).0;

/// `AddAcceptedCargo`: the tile type's `add_accepted_cargo_proc`, with a fresh
/// dummy mask when the caller passes none. Only houses, industries and objects
/// have one.
pub(crate) fn add_accepted_cargo(
    tile: TileIndex,
    acceptance: &mut CargoArray,
    always_accepted: *mut CargoTypes,
    c: &crate::station::Ctx<'_>,
) {
    let kind = TileType((c.s.tile_type)(tile.0));
    if !matches!(
        kind,
        TileType::MP_HOUSE | TileType::MP_INDUSTRY | TileType::MP_OBJECT
    ) {
        return;
    }
    let mut dummy = CargoTypes(0);
    let always = if always_accepted.is_null() {
        &raw mut dummy
    } else {
        always_accepted
    }
    // The unfinished per-type procedures take the raw `CargoTypes` base.
    .cast::<u64>();
    let cargoes = acceptance.as_mut_ptr();
    // SAFETY: Live immutable tables; both outputs outlive the synchronous procs.
    unsafe {
        match kind {
            TileType::MP_HOUSE => {
                crate::town::house::openttd_rust_house_accepted_tile(
                    c.c.town, c.s, tile.0, cargoes, always,
                );
            }
            TileType::MP_INDUSTRY => {
                crate::industry::tile::add_accepted_cargo_industry(
                    tile,
                    &mut *cargoes.cast::<CargoArray>(),
                    &mut *always.cast::<CargoTypes>(),
                    &*c.c.industry_tile,
                );
            }
            _ => crate::objects::openttd_rust_object_cargo(c.c.object, tile.0, cargoes, always),
        }
    }
}

/// `AddProducedCargo`: the tile type's `add_produced_cargo_proc`. Only houses
/// and objects have one.
pub(crate) fn add_produced_cargo(
    tile: TileIndex,
    produced: &mut CargoArray,
    c: &crate::station::Ctx<'_>,
) {
    let cargoes = produced.as_mut_ptr();
    // SAFETY: Live immutable tables; the output outlives the synchronous procs.
    unsafe {
        match TileType((c.s.tile_type)(tile.0)) {
            TileType::MP_HOUSE => {
                crate::town::house::openttd_rust_house_produced(c.c.town, c.s, tile.0, cargoes);
            }
            TileType::MP_OBJECT => crate::objects::openttd_rust_object_cargo(
                c.c.object,
                tile.0,
                cargoes,
                std::ptr::null_mut(),
            ),
            _ => {}
        }
    }
}
