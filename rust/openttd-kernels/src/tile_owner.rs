/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared tile-kind ownership dispatch. Immutable canonical tables are borrowed;
//! three distinct, constructed native costs and the full road context belong to
//! the outer C++ frame. No world borrow survives a callback. Panics abort and
//! native leaves are noexcept; AI startup remains outside Rust.
#![allow(unsafe_code, missing_docs, clippy::too_many_arguments)]
use crate::{industry, rail_cmd, road_cmd, services, station_cmd, water_cmd};

#[repr(C)]
pub struct Services {
    pub clear: extern "C" fn(u32, u8, u8),
    pub house: extern "C" fn(u32, u8, u8),
    pub industry: *const industry::TileServices,
    pub object: extern "C" fn(u32, u8, u8),
    pub rail: *const rail_cmd::RailCommandServices,
    pub shared: *const services::Services,
    pub station: *const station_cmd::CommandServicesTable,
    pub trees: extern "C" fn(u32, u8, u8),
    pub tunnel_bridge: extern "C" fn(u32, u8, u8),
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
/// `ChangeTileOwner`, retaining each original named tile handler.
/// # Safety
/// Tables and exclusive native scratch outlive the synchronous serialized call.
/// Slots may be reused only after a nested owner has returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_tile_change_owner(
    context: &Context,
    tile: u32,
    old: u8,
    new: u8,
) {
    // SAFETY: The native outer frame supplies the canonical tables and live slots.
    unsafe {
        let s = &*context.services;
        match ((*s.shared).tile_type)(tile) {
            0 => (s.clear)(tile, old, new),
            1 => rail_cmd::openttd_rust_rail_change_owner(s.rail, context.cost, tile, old, new),
            2 => {
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
            3 => (s.house)(tile, old, new),
            4 => (s.trees)(tile, old, new),
            5 => station_cmd::openttd_rust_station_change_owner(s.station, context, tile, old, new),
            6 => water_cmd::openttd_rust_water_change_owner(s.water, context.cost, tile, old, new),
            7 => (s.void)(tile, old, new),
            8 => {
                let owner = ((*s.industry).industry)(tile).owner;
                industry::openttd_rust_industry_change_owner(owner, old, new);
            }
            9 => (s.tunnel_bridge)(tile, old, new),
            10 => (s.object)(tile, old, new),
            // Like the native table, the map supplies only defined tile kinds.
            _ => std::hint::unreachable_unchecked(),
        }
    }
}
