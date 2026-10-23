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
#include "rust/tile_animation_services.h"
#include "rust/sound_services.h"
#include "rust/services_station.h"
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
		.station_base = RustAnimationStationBase, .as_station = RustAnimationStation, .location = RustStationLocation,
		.is_station = GetRustStationServices().is_station, .rail_randomisation = RustStationRandomisationAt, .road_randomisation = RustRoadStopRandomisationAt,
		.town = &GetRustTownServices(), .industries = &GetIndustryRustServices(),
		.construction = &GetIndustryConstructionServices(), .industry_tiles = &GetIndustryTileServices(),
		.house = &GetHouseAnimationBinding(), .industry = &GetIndustryAnimationBinding(), .station = &GetStationAnimationBinding(),
		.airport = &GetAirportAnimationBinding(), .object = &GetObjectAnimationBinding(), .roadstop = &GetRoadStopAnimationBinding(),
		.state = [](uint32_t t) noexcept { return GetAnimatedTileState(TileIndex{t}); },
		.set_state = [](uint32_t t, AnimatedTileState s) noexcept { SetAnimatedTileState(TileIndex{t}, s); },
		.frame = OpenTTDAnimationFrame, .ambient = OpenTTDAmbientSound,
		.house_spec = [](uint16_t h) noexcept -> const HouseSpec * { return HouseSpec::Get(h); },
		.road_frame = [](BaseStation *s, uint32_t t) noexcept { return s->GetRoadStopAnimationFrame(TileIndex{t}); },
		.set_road_frame = [](BaseStation *s, uint32_t t, uint8_t f) noexcept { return s->SetRoadStopAnimationFrame(TileIndex{t}, f); },
		.rail_area = OpenTTDAnimationRailArea,
		.rail_belongs = [](BaseStation *s, uint32_t t) noexcept { return s->TileBelongsToRailStation(TileIndex{t}); },
		.station_triggers = [](BaseStation *s, uint8_t t) noexcept { return s->cached_anim_triggers.Test(static_cast<StationAnimationTrigger>(t)); },
		.road_triggers = [](BaseStation *s, uint8_t t) noexcept { return s->cached_roadstop_anim_triggers.Test(static_cast<StationAnimationTrigger>(t)); },
		.rail_spec = RustAnimationRailSpec, .road_spec = RustAnimationRoadSpec, .airport_spec = RustAnimationAirportSpec,
		.industry_object = [](uint32_t t) noexcept { return Industry::GetByTile(TileIndex{t}); },
		.rail_spec_triggers = [](const StationSpec *s) noexcept { return s->animation.triggers.base(); },
		.road_spec_triggers = [](const RoadStopSpec *s) noexcept { return s->animation.triggers.base(); },
		.airport_spec_triggers = [](const AirportTileSpec *s) noexcept -> uint16_t { return s->animation.triggers.base(); },
		.rail_cargo = [](const StationSpec *s, uint8_t c) noexcept { return s->grf_prop.grffile->cargo_map[c]; },
		.road_cargo = [](const RoadStopSpec *s, uint8_t c) noexcept { return s->grf_prop.grffile->cargo_map[c]; },
		.airport_cargo = [](const AirportTileSpec *s, uint8_t c) noexcept { return s->grf_prop.grffile->cargo_map[c]; },
		.airport_grf = [](const AirportTileSpec *s) noexcept { return s->grf_prop.grffile != nullptr; },
		.airport_area = [](Station *s) noexcept -> OpenTTDAnimationArea { return {s->airport.tile.base(), s->airport.w, s->airport.h}; },
		.airport_belongs = [](Station *s, uint32_t t) noexcept { return s->TileBelongsToAirport(TileIndex{t}); },
		.road_count = [](BaseStation *s) noexcept { return s->custom_roadstop_tile_data.size(); },
		.road_tile = [](BaseStation *s, size_t i) noexcept { return s->custom_roadstop_tile_data[i].tile.base(); },
	};
	return services;
}
void AddAnimatedTile(TileIndex tile, bool dirty) { openttd_rust_animation_add(tile.base(), dirty); }
void DeleteAnimatedTile(TileIndex tile, bool immediate) { openttd_rust_animation_delete(tile.base(), immediate); }
void AnimateAnimatedTiles() { PerformanceAccumulator landscape_framerate(PFE_GL_LANDSCAPE); openttd_rust_animation_run(); }
void InitializeAnimatedTiles() { openttd_rust_animation_exchange(&GetRustAnimationServices()); openttd_rust_animation_initialize(); }
#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDAnimationServices.hpp"
bool CheckAbi_animated_tile()
{
	const auto &s = GetRustAnimationServices();
	return AbiServicesComplete(s) && AbiServicesComplete(*s.house) && AbiServicesComplete(*s.industry) &&
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
