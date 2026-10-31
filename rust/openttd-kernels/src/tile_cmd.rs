/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `tile_cmd.h` admission result bits.
#![allow(unsafe_code)]
/// `VehicleEnterTileState::EnteredStation` in `VehicleEnterTileStates`.
pub const ENTERED_STATION: u8 = 1;
/// `VehicleEnterTileState::EnteredWormhole` in `VehicleEnterTileStates`.
pub const ENTERED_WORMHOLE: u8 = 2;
/// `VehicleEnterTileState::CannotEnter` in `VehicleEnterTileStates`.
pub const CANNOT_ENTER: u8 = 4;

/// `AddAcceptedCargo`: the tile type's `add_accepted_cargo_proc`, with a fresh
/// dummy mask when the caller passes none. Only houses, industries and objects
/// have one.
pub(crate) fn add_accepted_cargo(
    tile: u32,
    acceptance: &mut [u32; 64],
    always_accepted: *mut u64,
    c: &crate::station::Ctx<'_>,
) {
    let kind = (c.s.tile_type)(tile);
    if !matches!(
        kind,
        crate::tile_type::MP_HOUSE | crate::tile_type::MP_INDUSTRY | crate::tile_type::MP_OBJECT
    ) {
        return;
    }
    let mut dummy = 0_u64;
    let always = if always_accepted.is_null() {
        &raw mut dummy
    } else {
        always_accepted
    };
    let cargoes = acceptance.as_mut_ptr();
    // SAFETY: Live immutable tables; both outputs outlive the synchronous procs.
    unsafe {
        match kind {
            crate::tile_type::MP_HOUSE => {
                crate::town::house::openttd_rust_house_accepted_tile(
                    c.c.town, c.s, tile, cargoes, always,
                );
            }
            crate::tile_type::MP_INDUSTRY => {
                crate::industry::tile::openttd_rust_industry_tile_acceptance(
                    tile,
                    cargoes,
                    always,
                    c.c.industry_tile,
                );
            }
            _ => crate::objects::openttd_rust_object_cargo(c.c.object, tile, cargoes, always),
        }
    }
}

/// `AddProducedCargo`: the tile type's `add_produced_cargo_proc`. Only houses
/// and objects have one.
pub(crate) fn add_produced_cargo(tile: u32, produced: &mut [u32; 64], c: &crate::station::Ctx<'_>) {
    let cargoes = produced.as_mut_ptr();
    // SAFETY: Live immutable tables; the output outlives the synchronous procs.
    unsafe {
        match (c.s.tile_type)(tile) {
            crate::tile_type::MP_HOUSE => {
                crate::town::house::openttd_rust_house_produced(c.c.town, c.s, tile, cargoes);
            }
            crate::tile_type::MP_OBJECT => crate::objects::openttd_rust_object_cargo(
                c.c.object,
                tile,
                cargoes,
                std::ptr::null_mut(),
            ),
            _ => {}
        }
    }
}
