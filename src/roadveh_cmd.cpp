/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file roadveh_cmd.cpp Handling of road vehicles. */

#include "stdafx.h"
#include "roadveh.h"
#include "command_func.h"
#include "error_func.h"
#include "news_func.h"
#include "station_base.h"
#include "company_func.h"
#include "articulated_vehicles.h"
#include "newgrf_sound.h"
#include "pathfinder/yapf/yapf.h"
#include "strings_func.h"
#include "tunnelbridge_map.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "depot_map.h"
#include "effectvehicle_func.h"
#include "roadstop_base.h"
#include "core/random_func.hpp"
#include "company_base.h"
#include "core/backup_type.hpp"
#include "newgrf.h"
#include "zoom_func.h"
#include "framerate_type.h"
#include "roadveh_cmd.h"
#include "road_cmd.h"
#include "newgrf_roadstop.h"

#include "table/strings.h"

#include "safeguards.h"

#include "rust/vehicle_services.h"
#include "rust/road_command_services.h"
#include "rust/settings_services.h"

static const OpenTTDRoadLeaves &GetRoadLeaves();
const OpenTTDRoadLeaves &GetRustRoadLeaves() noexcept { return GetRoadLeaves(); }
static OpenTTDRoadMaxSpeedRead RoadMaxSpeedRead(const RoadVehicle *u) noexcept;

/** Settings and map size a road entry may consult, read once per entry. */
static OpenTTDRoadEntry GetRoadEntry(const RoadVehicle *v) noexcept
{
	return {.ground = v->rust_ground.state, .acceleration_model = RustRoadAccelerationModel(),
		.road_side = RustRoadSide(), .queue = RustRoadQueue(), .map_log_x = static_cast<uint8_t>(Map::LogX())};
}

/** The front gate never observes movement settings or front-only fields for parts. */
OpenTTDRoadTickRead RustRoadTickRead(const RoadVehicle *v) noexcept
{
	OpenTTDRoadTickRead read{.tick = RustVehicleTick(v), .front = RustVehicleFront(v), .movement = {}, .order_time = nullptr, .running = nullptr, .status = nullptr};
	if (read.front) {
		read.movement = GetRoadEntry(v);
		read.order_time = RustVehicleOrderTimeAddress(v);
		read.running = RustVehicleRunningAddress(v);
		read.status = RustVehicleStatusAddress(v);
	}
	return read;
}

static const uint16_t _roadveh_images[] = {
	0xCD4, 0xCDC, 0xCE4, 0xCEC, 0xCF4, 0xCFC, 0xD0C, 0xD14,
	0xD24, 0xD1C, 0xD2C, 0xD04, 0xD1C, 0xD24, 0xD6C, 0xD74,
	0xD7C, 0xC14, 0xC1C, 0xC24, 0xC2C, 0xC34, 0xC3C, 0xC4C,
	0xC54, 0xC64, 0xC5C, 0xC6C, 0xC44, 0xC5C, 0xC64, 0xCAC,
	0xCB4, 0xCBC, 0xD94, 0xD9C, 0xDA4, 0xDAC, 0xDB4, 0xDBC,
	0xDCC, 0xDD4, 0xDE4, 0xDDC, 0xDEC, 0xDC4, 0xDDC, 0xDE4,
	0xE2C, 0xE34, 0xE3C, 0xC14, 0xC1C, 0xC2C, 0xC3C, 0xC4C,
	0xC5C, 0xC64, 0xC6C, 0xC74, 0xC84, 0xC94, 0xCA4
};

static const uint16_t _roadveh_full_adder[] = {
	 0,  88,   0,   0,   0,   0,  48,  48,
	48,  48,   0,   0,  64,  64,   0,  16,
	16,   0,  88,   0,   0,   0,   0,  48,
	48,  48,  48,   0,   0,  64,  64,   0,
	16,  16,   0,  88,   0,   0,   0,   0,
	48,  48,  48,  48,   0,   0,  64,  64,
	 0,  16,  16,   0,   8,   8,   8,   8,
	 0,   0,   0,   8,   8,   8,   8
};
static_assert(lengthof(_roadveh_images) == lengthof(_roadveh_full_adder));

template <>
bool IsValidImageIndex<VEH_ROAD>(uint8_t image_index)
{
	return image_index < lengthof(_roadveh_images);
}


/**
 * Check whether a roadvehicle is a bus
 * @return true if bus
 */
bool RoadVehicle::IsBus() const
{
	assert(this->IsFrontEngine());
	return IsCargoInClass(this->cargo_type, CargoClass::Passengers);
}

/**
 * Get the width of a road vehicle image in the GUI.
 * @param offset Additional offset for positioning the sprite; set to nullptr if not needed
 * @return Width in pixels
 */
int RoadVehicle::GetDisplayImageWidth(Point *offset) const
{
	int reference_width = ROADVEHINFO_DEFAULT_VEHICLE_WIDTH;

	if (offset != nullptr) {
		offset->x = ScaleSpriteTrad(reference_width) / 2;
		offset->y = 0;
	}
	return ScaleSpriteTrad(this->gcache.cached_veh_length * reference_width / VEHICLE_LENGTH);
}

static void GetRoadVehIcon(EngineID engine, EngineImageType image_type, VehicleSpriteSeq *result)
{
	const Engine *e = Engine::Get(engine);
	uint8_t spritenum = e->VehInfo<RoadVehicleInfo>().image_index;

	if (IsCustomVehicleSpriteNum(spritenum)) {
		GetCustomVehicleIcon(engine, DIR_W, image_type, result);
		if (result->IsValid()) return;

		spritenum = e->original_image_index;
	}

	assert(IsValidImageIndex<VEH_ROAD>(spritenum));
	result->Set(DIR_W + _roadveh_images[spritenum]);
}

void RoadVehicle::GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const
{
	uint8_t spritenum = this->spritenum;

	if (IsCustomVehicleSpriteNum(spritenum)) {
		if (spritenum == CUSTOM_VEHICLE_SPRITENUM_REVERSED) direction = ReverseDir(direction);
		GetCustomVehicleSprite(this, direction, image_type, result);
		if (result->IsValid()) return;

		spritenum = this->GetEngine()->original_image_index;
	}

	assert(IsValidImageIndex<VEH_ROAD>(spritenum));
	SpriteID sprite = direction + _roadveh_images[spritenum];

	if (this->cargo.StoredCount() >= this->Record().cargo_cap / 2U) sprite += _roadveh_full_adder[spritenum];

	result->Set(sprite);
}

/**
 * Draw a road vehicle engine.
 * @param left Left edge to draw within.
 * @param right Right edge to draw within.
 * @param preferred_x Preferred position of the engine.
 * @param y Vertical position of the engine.
 * @param engine Engine to draw
 * @param pal Palette to use.
 */
void DrawRoadVehEngine(int left, int right, int preferred_x, int y, EngineID engine, PaletteID pal, EngineImageType image_type)
{
	VehicleSpriteSeq seq;
	GetRoadVehIcon(engine, image_type, &seq);

	Rect rect;
	seq.GetBounds(&rect);
	preferred_x = Clamp(preferred_x,
			left - UnScaleGUI(rect.left),
			right - UnScaleGUI(rect.right));

	seq.Draw(preferred_x, y, pal, pal == PALETTE_CRASH);
}

/**
 * Get the size of the sprite of a road vehicle sprite heading west (used for lists).
 * @param engine The engine to get the sprite from.
 * @param[out] width The width of the sprite.
 * @param[out] height The height of the sprite.
 * @param[out] xoffs Number of pixels to shift the sprite to the right.
 * @param[out] yoffs Number of pixels to shift the sprite downwards.
 * @param image_type Context the sprite is used in.
 */
void GetRoadVehSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type)
{
	VehicleSpriteSeq seq;
	GetRoadVehIcon(engine, image_type, &seq);

	Rect rect;
	seq.GetBounds(&rect);

	width  = UnScaleGUI(rect.Width());
	height = UnScaleGUI(rect.Height());
	xoffs  = UnScaleGUI(rect.left);
	yoffs  = UnScaleGUI(rect.top);
}

/**
 * Get length of a road vehicle.
 * @param v Road vehicle to query length.
 * @return Length of the given road vehicle.
 */


/**
 * Update the cache of a road vehicle.
 * @param v Road vehicle needing an update of its cache.
 * @param same_length should length of vehicles stay the same?
 * @pre \a v must be first road vehicle.
 */
void RoadVehUpdateCache(RoadVehicle *v, bool same_length)
{
	openttd_rust_road_update_cache(RustVehicle(v), v->GetRustState(), same_length, &GetRoadLeaves());
}


static FindDepotData FindClosestRoadDepot(const RoadVehicle *v, int max_distance)
{
	if (IsRoadDepotTile(v->GetTile())) return FindDepotData(v->GetTile(), 0);

	return YapfRoadVehicleFindNearestDepot(v, max_distance);
}

ClosestDepot RoadVehicle::FindClosestDepot()
{
	FindDepotData rfdd = FindClosestRoadDepot(this, 0);
	if (rfdd.best_length == UINT_MAX) return ClosestDepot();

	return ClosestDepot(rfdd.tile, GetDepotIndex(rfdd.tile));
}

/**
 * Turn a roadvehicle around.
 * @param flags operation to perform
 * @param veh_id vehicle ID to turn
 * @return the cost of this operation or an error
 */
CommandCost CmdTurnRoadVeh(DoCommandFlags flags, VehicleID veh_id)
{
	RoadVehicle *v = RoadVehicle::GetIfValid(veh_id);
	if (v == nullptr) return CMD_ERROR;

	if (!v->IsPrimaryVehicle()) return CMD_ERROR;

	CommandCost ret = CheckOwnership(v->GetOwner());
	if (ret.Failed()) return ret;

	if (!openttd_rust_road_turn(RustVehicle(v), v->GetRustState(), flags.Test(DoCommandFlag::Execute), &GetRoadLeaves())) return CMD_ERROR;

	return CommandCost();
}


void RoadVehicle::MarkDirty()
{
	for (RoadVehicle *v = this; v != nullptr; v = v->Next()) {
		v->colourmap = PAL_NONE;
		v->UpdateViewport(true, false);
	}
	this->CargoChanged();
}

void RoadVehicle::UpdateDeltaXY()
{
	/* Set common defaults. */
	this->bounds = {{-1, -1, 0}, {3, 3, 6}, {}};

	if (!IsDiagonalDirection(this->GetDirection())) {
		static const Point _sign_table[] = {
			/* x, y */
			{-1, -1}, // DIR_N
			{-1,  1}, // DIR_E
			{ 1,  1}, // DIR_S
			{ 1, -1}, // DIR_W
		};

		int half_shorten = (VEHICLE_LENGTH - this->gcache.cached_veh_length) / 2;

		/* For all straight directions, move the bound box to the centre of the vehicle, but keep the size. */
		this->bounds.offset.x -= half_shorten * _sign_table[DirToDiagDir(this->GetDirection())].x;
		this->bounds.offset.y -= half_shorten * _sign_table[DirToDiagDir(this->GetDirection())].y;
	} else {
		/* Unlike trains, road vehicles do not have their offsets moved to the centre. */
		switch (this->GetDirection()) {
				/* Shorten southern corner of the bounding box according the vehicle length. */
			case DIR_NE:
				this->bounds.origin.x = -3;
				this->bounds.extent.x = this->gcache.cached_veh_length;
				this->bounds.offset.x = 1;
				break;

			case DIR_NW:
				this->bounds.origin.y = -3;
				this->bounds.extent.y = this->gcache.cached_veh_length;
				this->bounds.offset.y = 1;
				break;

				/* Move northern corner of the bounding box down according to vehicle length. */
			case DIR_SW:
				this->bounds.origin.x = -3 + (VEHICLE_LENGTH - this->gcache.cached_veh_length);
				this->bounds.extent.x = this->gcache.cached_veh_length;
				this->bounds.offset.x = 1 - (VEHICLE_LENGTH - this->gcache.cached_veh_length);
				break;

			case DIR_SE:
				this->bounds.origin.y = -3 + (VEHICLE_LENGTH - this->gcache.cached_veh_length);
				this->bounds.extent.y = this->gcache.cached_veh_length;
				this->bounds.offset.y = 1 - (VEHICLE_LENGTH - this->gcache.cached_veh_length);
				break;

			default:
				NOT_REACHED();
		}
	}
}

/**
 * Calculates the maximum speed of the vehicle under its current conditions.
 * @return Maximum speed of the vehicle.
 */
int RoadVehicle::GetCurrentMaxSpeed() const
{
	OpenTTDRoadMaxSpeedRead read = RoadMaxSpeedRead(this);
	return openttd_rust_road_max_speed(RustVehicle(this), this->GetRustState(), RustRoadAccelerationModel(), &read, &GetRoadLeaves());
}

/**
 * Delete last vehicle of a chain road vehicles.
 * @param v First roadvehicle.
 */




/**
 * Road vehicle chain has crashed.
 * @param v First roadvehicle.
 * @return whether the chain still exists.
 */


uint RoadVehicle::Crash(bool flooded)
{
	return openttd_rust_road_crash(RustVehicle(this), this->GetRustState(), flooded, &GetRoadLeaves());
}






TileIndex RoadVehicle::GetOrderStationLocation(StationID station)
{
	if (station == this->GetLastStationVisited()) this->SetLastStationVisited(StationID::Invalid());

	const Station *st = Station::Get(station);
	if (!CanVehicleUseStation(this, st)) {
		/* There is no stop left at the station, so don't even TRY to go there */
		this->IncrementRealOrderIndex();
		return TileIndex{};
	}

	return st->xy;
}








/**
 * A road vehicle arrives at a station. If it is the first time, create a news item.
 * @param v  Road vehicle that arrived.
 * @param st Station where the road vehicle arrived.
 */






/**
 * Check if overtaking is possible on a piece of track
 *
 * @param od Information about the tile and the involved vehicles
 * @return true if we have to abort overtaking
 */








/**
 * Returns direction to for a road vehicle to take or
 * INVALID_TRACKDIR if the direction is currently blocked
 * @param v        the Vehicle to do the pathfinding for
 * @param tile     the where to start the pathfinding
 * @param enterdir the direction the vehicle enters the tile from
 * @return the Trackdir to take
 */



bool RoadVehLeaveDepot(RoadVehicle *v, bool first)
{
	OpenTTDRoadEntry entry = GetRoadEntry(v);
	return openttd_rust_road_leave_depot(RustVehicle(v), v->GetRustState(), first, &entry, &GetRoadLeaves());
}



/**
 * Can a tram track build without destruction on the given tile?
 * @param c the company that would be building the tram tracks
 * @param t the tile to build on.
 * @param rt the tram type to build.
 * @param r the road bits needed.
 * @return true when a track track can be build on 't'
 */


bool IndividualRoadVehicleController(RoadVehicle *v, const RoadVehicle *prev)
{
	OpenTTDRoadEntry entry = GetRoadEntry(v);
	OpenTTDRoadCommandSlot road;
	return openttd_rust_road_individual(RustVehicle(v), v->GetRustState(), RustVehicle(prev), prev == nullptr ? nullptr : prev->GetRustState(), &entry, &GetRoadLeaves(), &road);
}



Money RoadVehicle::GetRunningCost() const
{
	return openttd_rust_road_running_cost(RustVehicle(this), this->GetRustState(), &GetRoadLeaves());
}

bool RoadVehicle::Tick()
{
	PerformanceAccumulator framerate(PFE_GL_ROADVEHS);
	const auto read = RustRoadTickRead(this);
	OpenTTDRoadCommandSlot road;
	return openttd_rust_road_tick(RustVehicle(this), this->GetRustState(), &read, &GetRoadLeaves(), &road);
}

void RoadVehicle::SetDestTile(TileIndex tile)
{
	openttd_rust_road_set_dest(RustVehicle(this), this->GetRustState(), tile.base(), this->GetDestTile().base(), &GetRoadLeaves());
}



/** Calander day handler */
void RoadVehicle::OnNewCalendarDay()
{
	openttd_rust_road_calendar_day(RustVehicle(this), RustVehicleFront(this), &GetRoadLeaves());
}

/** Economy day handler. */
void RoadVehicle::OnNewEconomyDay()
{
	openttd_rust_road_economy_day(RustVehicle(this), this->GetRustState(), RustVehicleFront(this), &GetRoadLeaves());
}

Trackdir RoadVehicle::GetVehicleTrackdir() const
{
	return static_cast<Trackdir>(openttd_rust_road_trackdir(RustVehicle(this), this->GetRustState(), &GetRoadLeaves()));
}

uint16_t RoadVehicle::GetMaxWeight() const
{
	return openttd_rust_ground_weight(&GetRustGroundPhysicsServices(), this, true);
}

#include "rust/road_services.h"



#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRoadLeaves.hpp"
bool CheckAbi_roadveh_cmd() { return AbiServicesComplete(GetRoadLeaves()); }
