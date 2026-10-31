/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file aircraft_cmd.cpp
 * This file deals with aircraft and airport movements functionalities
 */

#include "stdafx.h"
#include "aircraft.h"
#include "landscape.h"
#include "news_func.h"
#include "newgrf_engine.h"
#include "newgrf_sound.h"
#include "error_func.h"
#include "strings_func.h"
#include "command_func.h"
#include "window_func.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "cheat_type.h"
#include "company_base.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "company_func.h"
#include "effectvehicle_func.h"
#include "station_base.h"
#include "engine_base.h"
#include "core/random_func.hpp"
#include "core/backup_type.hpp"
#include "zoom_func.h"
#include "disaster_vehicle.h"
#include "newgrf_airporttiles.h"
#include "framerate_type.h"
#include "aircraft_cmd.h"
#include "vehicle_cmd.h"

#include "table/strings.h"

#include "rust/engine_capabilities_ffi.h"
#include "rust/vehicle_services.h"
#include "safeguards.h"

void Aircraft::UpdateDeltaXY()
{
	this->bounds = {{-1, -1, 0}, {2, 2, 0}, {}};

	switch (this->subtype) {
		default: NOT_REACHED();

		case AIR_AIRCRAFT:
		case AIR_HELICOPTER:
			switch (this->state) {
				default: break;
				case ENDTAKEOFF:
				case LANDING:
				case HELILANDING:
				case FLYING:
					/* Bounds are not centred on the aircraft. */
					this->bounds.extent.x = 24;
					this->bounds.extent.y = 24;
					break;
			}
			this->bounds.extent.z = 5;
			break;

		case AIR_SHADOW:
			this->bounds.extent.z = 1;
			this->bounds.origin = {};
			break;

		case AIR_ROTOR:
			this->bounds.extent.z = 1;
			break;
	}
}


static const SpriteID _aircraft_sprite[] = {
	0x0EB5, 0x0EBD, 0x0EC5, 0x0ECD,
	0x0ED5, 0x0EDD, 0x0E9D, 0x0EA5,
	0x0EAD, 0x0EE5, 0x0F05, 0x0F0D,
	0x0F15, 0x0F1D, 0x0F25, 0x0F2D,
	0x0EED, 0x0EF5, 0x0EFD, 0x0F35,
	0x0E9D, 0x0EA5, 0x0EAD, 0x0EB5,
	0x0EBD, 0x0EC5
};

template <>
bool IsValidImageIndex<VEH_AIRCRAFT>(uint8_t image_index)
{
	return image_index < lengthof(_aircraft_sprite);
}

/** Helicopter rotor animation states */
enum HelicopterRotorStates : uint8_t {
	HRS_ROTOR_STOPPED,
	HRS_ROTOR_MOVING_1,
	HRS_ROTOR_MOVING_2,
	HRS_ROTOR_MOVING_3,
};

/**
 * Find the nearest hangar to v
 * StationID::Invalid() is returned, if the company does not have any suitable
 * airports (like helipads only)
 * @param v vehicle looking for a hangar
 * @return the StationID if one is found, otherwise, StationID::Invalid()
 */

void Aircraft::GetImage(Direction direction, EngineImageType image_type, VehicleSpriteSeq *result) const
{
	uint8_t spritenum = this->spritenum;

	if (IsCustomVehicleSpriteNum(spritenum)) {
		GetCustomVehicleSprite(this, direction, image_type, result);
		if (result->IsValid()) return;

		spritenum = this->GetEngine()->original_image_index;
	}

	assert(IsValidImageIndex<VEH_AIRCRAFT>(spritenum));
	result->Set(direction + _aircraft_sprite[spritenum]);
}

void GetRotorImage(const Aircraft *v, EngineImageType image_type, VehicleSpriteSeq *result)
{
	assert(v->subtype == AIR_HELICOPTER);

	const Aircraft *w = v->Next()->Next();
	if (IsCustomVehicleSpriteNum(v->spritenum)) {
		GetCustomRotorSprite(v, image_type, result);
		if (result->IsValid()) return;
	}

	/* Return standard rotor sprites if there are no custom sprites for this helicopter */
	result->Set(SPR_ROTOR_STOPPED + w->state);
}

static void GetAircraftIcon(EngineID engine, EngineImageType image_type, VehicleSpriteSeq *result)
{
	const Engine *e = Engine::Get(engine);
	uint8_t spritenum = e->VehInfo<AircraftVehicleInfo>().image_index;

	if (IsCustomVehicleSpriteNum(spritenum)) {
		GetCustomVehicleIcon(engine, DIR_W, image_type, result);
		if (result->IsValid()) return;

		spritenum = e->original_image_index;
	}

	assert(IsValidImageIndex<VEH_AIRCRAFT>(spritenum));
	result->Set(DIR_W + _aircraft_sprite[spritenum]);
}

void DrawAircraftEngine(int left, int right, int preferred_x, int y, EngineID engine, PaletteID pal, EngineImageType image_type)
{
	VehicleSpriteSeq seq;
	GetAircraftIcon(engine, image_type, &seq);

	Rect rect;
	seq.GetBounds(&rect);
	preferred_x = Clamp(preferred_x,
			left - UnScaleGUI(rect.left),
			right - UnScaleGUI(rect.right));

	seq.Draw(preferred_x, y, pal, pal == PALETTE_CRASH);

	if (!(AircraftVehInfo(engine)->subtype & AIR_CTOL)) {
		VehicleSpriteSeq rotor_seq;
		GetCustomRotorIcon(engine, image_type, &rotor_seq);
		if (!rotor_seq.IsValid()) rotor_seq.Set(SPR_ROTOR_STOPPED);
		rotor_seq.Draw(preferred_x, y - ScaleSpriteTrad(5), PAL_NONE, false);
	}
}

/**
 * Get the size of the sprite of an aircraft sprite heading west (used for lists).
 * @param engine The engine to get the sprite from.
 * @param[out] width The width of the sprite.
 * @param[out] height The height of the sprite.
 * @param[out] xoffs Number of pixels to shift the sprite to the right.
 * @param[out] yoffs Number of pixels to shift the sprite downwards.
 * @param image_type Context the sprite is used in.
 */
void GetAircraftSpriteSize(EngineID engine, uint &width, uint &height, int &xoffs, int &yoffs, EngineImageType image_type)
{
	VehicleSpriteSeq seq;
	GetAircraftIcon(engine, image_type, &seq);

	Rect rect;
	seq.GetBounds(&rect);

	width  = UnScaleGUI(rect.Width());
	height = UnScaleGUI(rect.Height());
	xoffs  = UnScaleGUI(rect.left);
	yoffs  = UnScaleGUI(rect.top);
}



Money Aircraft::GetRunningCost() const
{
	return openttd_rust_engine_vehicle_running_cost(&GetRustEngineCapabilitiesServices(), RustEngine(this->GetEngine()), RustVehicle(this));
}


void Aircraft::MarkDirty()
{
	this->colourmap = PAL_NONE;
	this->UpdateViewport(true, false);
	if (this->subtype == AIR_HELICOPTER) {
		GetRotorImage(this, EIT_ON_MAP, &this->Next()->Next()->sprite_cache.sprite_seq);
	}
}


#include "rust/aircraft_adapter.hpp"

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDAircraftLeaves.hpp"
bool CheckAbi_aircraft_cmd() { return AbiServicesComplete(_aircraft_leaves); }
