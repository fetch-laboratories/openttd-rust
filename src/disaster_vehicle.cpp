/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file disaster_vehicle.cpp
 *
 * All disaster/easter egg vehicles are handled here.
 * The general flow of control for the disaster vehicles is as follows:
 * <ol>
 * <li>Initialize the disaster in a disaster specific way (eg start position,
 *     possible target, etc.) Disaster_XXX_Init() function
 * <li>Add a subtype to a disaster, which is an index into the function array
 *     that handles the vehicle's ticks.
 * <li>Run the disaster vehicles each tick until their target has been reached,
 *     this happens in the DisasterTick_XXX() functions. In here, a vehicle's
 *     state is kept by v->state variable. Each achieved sub-target
 *     will increase this value, and the last one will remove the disaster itself
 * </ol>
 */


#include "stdafx.h"

#include "aircraft.h"
#include "disaster_vehicle.h"
#include "industry.h"
#include "station_base.h"
#include "command_func.h"
#include "news_func.h"
#include "town.h"
#include "company_func.h"
#include "strings_func.h"
#include "viewport_func.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "effectvehicle_func.h"
#include "roadveh.h"
#include "train.h"
#include "ai/ai.hpp"
#include "game/game.hpp"
#include "company_base.h"
#include "core/random_func.hpp"
#include "core/backup_type.hpp"
#include "landscape_cmd.h"
#include "timer/timer.h"
#include "timer/timer_game_economy.h"

#include "table/strings.h"

#include "safeguards.h"

#include "rust/disaster_adapter.hpp"

void DisasterVehicle::UpdateDeltaXY()
{
	this->bounds = {{-1, -1, 0}, {2, 2, 5}, {}};
}
