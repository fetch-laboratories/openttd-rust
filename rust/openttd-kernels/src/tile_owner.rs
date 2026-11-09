/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Tile-kind ownership dispatch over immutable tables. Three distinct constructed native costs and
//! the road context belong to the outer C++ frame; AI startup stays in C++.
#![allow(unsafe_code, missing_docs, clippy::too_many_arguments)]
use crate::company_type::{CompanyID, Owner};
use crate::tile_type::{TileIndex, TileType};
use crate::{industry, objects, rail_cmd, road_cmd, services, station_cmd, water_cmd};

#[repr(C)]
pub struct Services {
    pub clear: extern "C" fn(u32, u8, u8),
    pub house: extern "C" fn(u32, u8, u8),
    pub industry: *const industry::TileServices,
    pub objects: *const objects::ObjectServices,
    pub rail: *const rail_cmd::RailCommandServices,
    pub shared: *const services::Services,
    pub station: *const station_cmd::CommandServicesTable,
    pub trees: extern "C" fn(u32, u8, u8),
    pub tunnel_bridge: *const crate::tunnelbridge_cmd::TunnelBridgeServices,
    pub void: extern "C" fn(u32, u8, u8),
    pub water: *const water_cmd::WaterCommandServices,
}
#[repr(C)]
pub struct Context {
    pub services: *const Services,
    pub cost: *mut services::CommandCost,
    pub ret: *mut services::CommandCost,
    pub error: *mut services::CommandCost,
    pub road: *const road_cmd::CommandContext,
}
/// `ChangeTileOwner`: change the owner of a tile from `old_owner` to `new_owner`, through
/// the tile type's `change_tile_owner_proc`, retaining each original named tile handler.
///
/// # Safety
/// Tables and exclusive native scratch outlive the synchronous serialized call.
/// Slots may be reused only after a nested owner has returned.
pub(crate) unsafe fn change_tile_owner(
    context: &Context,
    tile: TileIndex,
    old_owner: Owner,
    new_owner: Owner,
) {
    let (tile, old, new) = (tile.0, old_owner.0, new_owner.0);
    // SAFETY: The native outer frame supplies the canonical tables and live slots.
    unsafe {
        let s = &*context.services;
        match TileType(((*s.shared).tile_type)(tile)) {
            TileType::MP_CLEAR => (s.clear)(tile, old, new),
            TileType::MP_RAILWAY => {
                rail_cmd::openttd_rust_rail_change_owner(s.rail, context.cost, tile, old, new);
            }
            TileType::MP_ROAD => {
                let road = &*context.road;
                road_cmd::openttd_rust_road_change_owner(
                    road.services,
                    road.cost,
                    road.ret,
                    tile,
                    old,
                    new,
                );
            }
            TileType::MP_HOUSE => (s.house)(tile, old, new),
            TileType::MP_TREES => (s.trees)(tile, old, new),
            TileType::MP_STATION => {
                station_cmd::openttd_rust_station_change_owner(s.station, context, tile, old, new);
            }
            TileType::MP_WATER => {
                water_cmd::openttd_rust_water_change_owner(s.water, context.cost, tile, old, new);
            }
            TileType::MP_VOID => (s.void)(tile, old, new),
            TileType::MP_INDUSTRY => {
                let owner = ((*s.industry).industry)(tile).owner;
                industry::change_tile_owner_industry(owner, old_owner, new_owner);
            }
            TileType::MP_TUNNELBRIDGE => {
                crate::tunnelbridge_cmd::change_owner(
                    s.tunnel_bridge,
                    context.cost,
                    tile,
                    old,
                    new,
                );
            }
            TileType::MP_OBJECT => {
                objects::openttd_rust_object_change_owner(s.objects, context, tile, old, new);
            }
            // Like the native table, the map supplies only defined tile kinds.
            _ => std::hint::unreachable_unchecked(),
        }
    }
}
/// # Safety
/// As [`change_tile_owner`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tile_change_owner(
    context: &Context,
    tile: u32,
    old: u8,
    new: u8,
) {
    unsafe { change_tile_owner(context, TileIndex(tile), CompanyID(old), CompanyID(new)) };
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTileOwnerServices" ["landscape"] => Services {
        clear,
        house,
        industry,
        objects,
        rail,
        shared,
        station,
        trees,
        tunnel_bridge,
        void => void_tile,
        water,
    };
    "OpenTTDTileOwnerContext" ["landscape"] => Context {
        services,
        cost,
        ret,
        error,
        road,
    };
}
