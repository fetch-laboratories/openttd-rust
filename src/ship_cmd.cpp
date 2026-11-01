/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file ship_cmd.cpp Handling of ships. */

#include "stdafx.h"
#include "ship.h"
#include "landscape.h"
#include "timetable.h"
#include "news_func.h"
#include "company_func.h"
#include "depot_base.h"
#include "station_base.h"
#include "newgrf_engine.h"
#include "pathfinder/yapf/yapf.h"
#include "pathfinder/yapf/yapf_ship_regions.h"
#include "newgrf_sound.h"
#include "strings_func.h"
#include "window_func.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "engine_base.h"
#include "company_base.h"
#include "tunnelbridge_map.h"
#include "zoom_func.h"
#include "framerate_type.h"
#include "industry.h"
#include "industry_map.h"
#include "ship_cmd.h"

#include "table/strings.h"

#include <unordered_set>

#include "rust/engine_capabilities_ffi.h"
#include "rust/services_ffi.h"
#include "rust/industry_shell_services.h"
#include "rust/vehicle_services.h"
#include "safeguards.h"

/** Max distance in tiles (as the crow flies) to search for depots when user clicks "go to depot". */
constexpr int MAX_SHIP_DEPOT_SEARCH_DISTANCE = 80;

/**
 * Determine the effective #WaterClass for a ship travelling on a tile.
 * @param tile Tile of interest
 * @return the waterclass to be used by the ship.
 */
WaterClass GetEffectiveWaterClass(TileIndex tile)
{
	if (HasTileWaterClass(tile)) return GetWaterClass(tile);
	if (IsTileType(tile, MP_TUNNELBRIDGE)) {
		assert(GetTunnelBridgeTransportType(tile) == TRANSPORT_WATER);
		return WaterClass::Canal;
	}
	if (IsTileType(tile, MP_RAILWAY)) {
		assert(GetRailGroundType(tile) == RailGroundType::HalfTileWater);
		return WaterClass::Sea;
	}
	NOT_REACHED();
}

static const uint16_t _ship_sprites[] = {0x0E5D, 0x0E55, 0x0E65, 0x0E6D};

template <>
bool IsValidImageIndex<VEH_SHIP>(uint8_t image_index)
{
	return image_index < lengthof(_ship_sprites);
}


static void GetShipIcon(EngineID engine, EngineImageType image_type, VehicleSpriteSeq *result)
{
	const Engine *e = Engine::Get(engine);
	uint8_t spritenum = e->VehInfo<ShipVehicleInfo>().image_index;

	if (IsCustomVehicleSpriteNum(spritenum)) {
		GetCustomVehicleIcon(engine, DIR_W, image_type, result);
		if (result->IsValid()) return;

		spritenum = e->original_image_index;
	}

	assert(IsValidImageIndex<VEH_SHIP>(spritenum));
	result->Set(DIR_W + _ship_sprites[spritenum]);
}

void DrawShipEngine(int left, int right, int preferred_x, int y, EngineID engine, PaletteID pal, EngineImageType image_type)
{
	VehicleSpriteSeq seq;
	GetShipIcon(engine, image_type, &seq);

	Rect rect;
	seq.GetBounds(&rect);
	preferred_x = Clamp(preferred_x,
			left - UnScaleGUI(rect.left),
			right - UnScaleGUI(rect.right));

	seq.Draw(preferred_x, y, pal, pal == PALETTE_CRASH);
}

/**
 * Get the size of the sprite of a ship sprite heading west (used for lists).
 * @param engine The engine to get the sprite from.
 * @param[out] width The width of the sprite.
 * @param[out] height The height of the sprite.
 * @param[out] xoffs Number of pixels to shift the sprite to the right.
 * @param[out] yoffs Number of pixels to shift the sprite downwards.
 * @param image_type Context the sprite is used in.
 */
void GetShipSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type)
{
	VehicleSpriteSeq seq;
	GetShipIcon(engine, image_type, &seq);

	Rect rect;
	seq.GetBounds(&rect);

	width  = UnScaleGUI(rect.Width());
	height = UnScaleGUI(rect.Height());
	xoffs  = UnScaleGUI(rect.left);
	yoffs  = UnScaleGUI(rect.top);
}

void Ship::GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const
{
	uint8_t spritenum = this->spritenum;

	if (image_type == EIT_ON_MAP) direction = this->GetRotation();

	if (IsCustomVehicleSpriteNum(spritenum)) {
		GetCustomVehicleSprite(this, direction, image_type, result);
		if (result->IsValid()) return;

		spritenum = this->GetEngine()->original_image_index;
	}

	assert(IsValidImageIndex<VEH_SHIP>(spritenum));
	result->Set(_ship_sprites[spritenum] + direction);
}


/**
 * Update the caches of this ship.
 */
void Ship::UpdateCache()
{
	openttd_rust_ship_cache(RustVehicle(this), this->index.base(), &GetRustShipServices(), &GetRustSharedServices());
}

Money Ship::GetRunningCost() const
{
	return openttd_rust_engine_vehicle_running_cost(&GetRustEngineCapabilitiesServices(), RustEngine(this->GetEngine()), RustVehicle(this));
}

/** Calendar day handler. */
void Ship::OnNewCalendarDay()
{
	openttd_rust_ship_calendar_day(this->index.base(), RustVehicle(this), &GetRustShipServices(), &GetRustSharedServices());
}

/** Economy day handler. */
void Ship::OnNewEconomyDay()
{
	openttd_rust_ship_economy_day(this->index.base(), RustVehicle(this), &GetRustShipServices(), &GetRustSharedServices());
}

Trackdir Ship::GetVehicleTrackdir() const
{
	return static_cast<Trackdir>(openttd_rust_ship_trackdir(this->index.base(), &GetRustShipServices(), &GetRustSharedServices()));
}

void Ship::MarkDirty()
{
	this->colourmap = PAL_NONE;
	this->UpdateViewport(true, false);
	this->UpdateCache();
}

void Ship::PlayLeaveStationSound(bool force) const
{
	if (PlayVehicleSound(this, VSE_START, force)) return;
	SndPlayVehicleFx(ShipVehInfo(this->GetEngineType())->sfx, this);
}

TileIndex Ship::GetOrderStationLocation(StationID station)
{
	return TileIndex(static_cast<uint32_t>(openttd_rust_ship_station_destination(this->index.base(), station.base(), RustVehicle(this), &GetRustShipServices(), &GetRustSharedServices())));
}

void Ship::UpdateDeltaXY()
{
	static constexpr SpriteBounds ship_bounds[DIR_END] = {
		{{ -3,  -3, 0}, { 6,  6, 6}, {}}, // N
		{{-16,  -3, 0}, {32,  6, 6}, {}}, // NE
		{{ -3,  -3, 0}, { 6,  6, 6}, {}}, // E
		{{ -3, -16, 0}, { 6, 32, 6}, {}}, // SE
		{{ -3,  -3, 0}, { 6,  6, 6}, {}}, // S
		{{-16,  -3, 0}, {32,  6, 6}, {}}, // SW
		{{ -3,  -3, 0}, { 6,  6, 6}, {}}, // W
		{{ -3, -16, 0}, { 6, 32, 6}, {}}, // NW
	};

	this->bounds = ship_bounds[this->GetRotation()];

	if (this->GetDirection() != this->GetRotation()) {
		/* If we are rotating, then it is possible the ship was moved to its next position. In that
		 * case, because we are still showing the old direction, the ship will appear to glitch sideways
		 * slightly. We can work around this by applying an additional offset to make the ship appear
		 * where it was before it moved. */
		this->bounds.origin.x -= this->Record().x_pos - this->GetRotationX();
		this->bounds.origin.y -= this->Record().y_pos - this->GetRotationY();
	}
}


bool IsShipDestinationTile(TileIndex tile, StationID station) { return openttd_rust_ship_is_destination( tile.base(), station.base(), &GetRustShipServices(), &GetRustSharedServices()); }

bool Ship::Tick()
{
	PerformanceAccumulator framerate(PFE_GL_SHIPS);
	return openttd_rust_ship_tick(this->index.base(), RustVehicle(this), &GetRustShipServices(), &GetRustSharedServices());
}

void Ship::SetDestTile(TileIndex tile)
{
	openttd_rust_ship_destination(this->index.base(), tile.base(), &GetRustShipServices(), &GetRustSharedServices());
}


ClosestDepot Ship::FindClosestDepot()
{
	auto depot = openttd_rust_ship_find_depot(this->index.base(), MAX_SHIP_DEPOT_SEARCH_DISTANCE, &GetRustShipServices(), &GetRustSharedServices());
	return depot.valid ? ClosestDepot(TileIndex(depot.tile), DepotID(depot.id)) : ClosestDepot();
}

#include "rust/ship_control_services.h"

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDShipLeaves.hpp"
bool CheckAbi_ship_cmd() { return AbiServicesComplete(GetRustShipServices()); }
