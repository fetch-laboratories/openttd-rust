/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file animated_tile.cpp Everything related to animated tiles. */

#include "stdafx.h"
#include "animated_tile_func.h"
#include "animated_tile_map.h"
#include "tile_cmd.h"
#include "viewport_func.h"
#include "framerate_type.h"

#include "safeguards.h"

#ifdef WITH_RUST
#include "rust/animation_ffi.h"
#include "rust/station_resolver_ffi.h"
#include "rust/tile_animation_services.h"
#include "rust/services_sound.h"
#include "rust/services_station.h"
#include "rust/services_industry.h"
#include "rust/services_cargo.h"
#include "rust/station_service_ffi.h"
#include "station_base.h"
#include "newgrf_station.h"
#include "newgrf_roadstop.h"
#include "newgrf_airporttiles.h"
#include "house.h"
#include "industry.h"

static const OpenTTDAnimationServices &GetRustAnimationServices() noexcept
{
	static const OpenTTDAnimationServices services{
		.shared = &GetRustSharedServices(),
		.station_base = RustStationGet<BaseStation *>, .as_station = RustAnimationStation, .location = RustStationLocation,
		.is_station = RustStationIsExpected<BaseStation *>, .roadstop_resolver = &GetRoadStopResolverServices(),
		.town = &GetRustTownServices(), .industries = &GetIndustryRustServices(),
		.construction = &GetIndustryConstructionServices(), .industry_tiles = &GetIndustryTileServices(),
		.house = &GetHouseAnimationBinding(), .industry = &GetIndustryAnimationBinding(), .station = &GetStationAnimationBinding(),
		.airport = &GetAirportAnimationBinding(),
		.airport_runtime = &GetAirportResolverServices(), .object = &GetObjectAnimationBinding(), .roadstop = &GetRoadStopAnimationBinding(),
		.state = [](uint32_t t) noexcept { return GetAnimatedTileState(TileIndex{t}); },
		.set_state = [](uint32_t t, AnimatedTileState s) noexcept { SetAnimatedTileState(TileIndex{t}, s); },
		.house_spec = RustHouseSpec,
		.road_frame = RustRoadStopAnimationFrame,
		.set_road_frame = [](BaseStation *s, uint32_t t, uint8_t f) noexcept { return s->SetRoadStopAnimationFrame(TileIndex{t}, f); },
		.rail_belongs = [](BaseStation *s, uint32_t t) noexcept { return s->TileBelongsToRailStation(TileIndex{t}); },
		.station_triggers = RustStationHasAnimationTrigger,
		.road_triggers = RustRoadStopHasAnimationTrigger,
		.rail_spec = RustAnimationRailSpec,
		.industry_object = RustIndustryByTile,
		.rail_spec_triggers = RustStationAnimationTriggers,
		.road_spec_triggers = RustRoadStopAnimationTriggers,
		.airport_spec_triggers = [](const AirportTileSpec *s) noexcept { return s->animation.triggers.base(); },
		.rail_cargo = RustSpecCargoMap<StationSpec>,
		.road_cargo = RustSpecCargoMap<RoadStopSpec>,
		.airport_cargo = RustSpecCargoMap<AirportTileSpec>,
		.airport_grf = RustSpecHasGRFFile<AirportTileSpec>,
		.airport_area = [](Station *s) noexcept -> OpenTTDAnimationArea { return {s->airport.tile.base(), s->airport.w, s->airport.h}; },
		.airport_belongs = RustStationAirportMember,
		.road_count = RustRoadStopTileDataCount,
		.road_tile = RustRoadStopDataTile,
	};
	return services;
}
void AddAnimatedTile(TileIndex tile, bool dirty) { openttd_rust_animation_add(tile.base(), dirty); }
void DeleteAnimatedTile(TileIndex tile, bool immediate) { openttd_rust_animation_delete(tile.base(), immediate); }

/* InitializeGame calls this for generated, intro and loaded worlds before
 * commands, PBS, cargo/visits, animation ticks or purchase/render resolvers run.
 * All direct Rust station entries share this process-wide serialized table. */
void InitializeAnimatedTiles() { openttd_rust_airport_exchange(&GetAirportResolverServices()); openttd_rust_animation_exchange(&GetRustAnimationServices()); openttd_rust_station_resolver_exchange(&GetStationResolverServices()); openttd_rust_animation_initialize(); }
#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDAnimationServices.hpp"
#include "abi-visit-OpenTTDRoadStopResolverServices.hpp"
#include "abi-visit-OpenTTDHouseAnimation.hpp"
#include "abi-visit-OpenTTDIndustryAnimation.hpp"
#include "abi-visit-OpenTTDStationAnimation.hpp"
#include "abi-visit-OpenTTDAirportAnimation.hpp"
#include "abi-visit-OpenTTDObjectAnimation.hpp"
#include "abi-visit-OpenTTDRoadStopAnimation.hpp"
bool CheckAbi_animated_tile()
{
	const auto &s = GetRustAnimationServices();
	return AbiServicesComplete(s) && AbiServicesComplete(*s.roadstop_resolver) && AbiServicesComplete(*s.house) && AbiServicesComplete(*s.industry) &&
		AbiServicesComplete(*s.station) && AbiServicesComplete(*s.airport) && AbiServicesComplete(*s.object) && AbiServicesComplete(*s.roadstop);
}

#else
/** The table/list with animated tiles. */
std::vector<TileIndex> _animated_tiles;

/**
 * Stops animation on the given tile.
 * @param tile the tile to remove
 * @param immediate immediately delete the tile from the animated tile list instead of waiting for the next tick.
 */
void DeleteAnimatedTile(TileIndex tile, bool immediate)
{
	if (immediate) {
		if (GetAnimatedTileState(tile) == AnimatedTileState::None) return;

		/* The tile may be switched to a non-animatable tile soon, so we should remove it from the
		 * animated tile list early. */
		SetAnimatedTileState(tile, AnimatedTileState::None);

		/* To avoid having to move everything after this tile in the animated tile list, look for this tile
		 * in the animated tile list and replace with last entry if not last. */
		auto it = std::ranges::find(_animated_tiles, tile);
		if (it == std::end(_animated_tiles)) return;

		if (std::next(it) != std::end(_animated_tiles)) *it = _animated_tiles.back();
		_animated_tiles.pop_back();

		return;
	}

	/* If the tile was animated, mark it for deletion from the tile list on the next animation loop. */
	if (GetAnimatedTileState(tile) == AnimatedTileState::Animated) SetAnimatedTileState(tile, AnimatedTileState::Deleted);
}

/**
 * Add the given tile to the animated tile table (if it does not exist yet).
 * @param tile the tile to make animated
 * @param mark_dirty whether to also mark the tile dirty.
 */
void AddAnimatedTile(TileIndex tile, bool mark_dirty)
{
	if (mark_dirty) MarkTileDirtyByTile(tile);

	const AnimatedTileState state = GetAnimatedTileState(tile);

	/* Tile is already animated so nothing needs to happen. */
	if (state == AnimatedTileState::Animated) return;

	/* Tile has no previous animation state, so add to the tile list. If the state is anything
	 * other than None (e.g. Deleted) then the tile will still be in the list and does not need to be added again. */
	if (state == AnimatedTileState::None) _animated_tiles.push_back(tile);

	SetAnimatedTileState(tile, AnimatedTileState::Animated);
}

/**
 * Animate all tiles in the animated tile list, i.e.\ call AnimateTile on them.
 */
void AnimateAnimatedTiles()
{
	PerformanceAccumulator landscape_framerate(PFE_GL_LANDSCAPE);

	for (auto it = std::begin(_animated_tiles); it != std::end(_animated_tiles); /* nothing */) {
		TileIndex &tile = *it;

		if (GetAnimatedTileState(tile) != AnimatedTileState::Animated) {
			/* Tile should not be animated any more, mark it as not animated and erase it from the list. */
			SetAnimatedTileState(tile, AnimatedTileState::None);

			/* Removing the last entry, no need to swap and continue. */
			if (std::next(it) == std::end(_animated_tiles)) {
				_animated_tiles.pop_back();
				break;
			}

			/* Replace the current list entry with the back of the list to avoid moving elements. */
			*it = _animated_tiles.back();
			_animated_tiles.pop_back();
			continue;
		}

		AnimateTile(tile);
		++it;
	}
}

/**
 * Initialize all animated tile variables to some known begin point
 */
void InitializeAnimatedTiles()
{
	_animated_tiles.clear();
}

#endif /* WITH_RUST */
