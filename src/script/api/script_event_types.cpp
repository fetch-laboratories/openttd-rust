/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file script_event_types.cpp Implementation of all EventTypes. */

#include "../../stdafx.h"
#include "script_event_types.hpp"
#include "script_vehicle.hpp"
#include "script_log.hpp"
#include "../../strings_func.h"
#include "../../settings_type.h"
#include "../../engine_base.h"
#include "../../articulated_vehicles.h"
#include "../../string_func.h"
#include "../../economy_cmd.h"
#include "../../engine_cmd.h"
#include "../../3rdparty/nlohmann/json.hpp"

#include "table/strings.h"

#include "../admin_conversion.hpp"

#include "../../safeguards.h"

bool ScriptEventEnginePreview::IsEngineValid() const
{
	const Engine *e = ::Engine::GetIfValid(this->engine);
	return e != nullptr && e->IsEnabled();
}

std::optional<std::string> ScriptEventEnginePreview::GetName()
{
	if (!this->IsEngineValid()) return std::nullopt;

	return ::StrMakeValid(::GetString(STR_ENGINE_NAME, this->engine), {});
}

CargoType ScriptEventEnginePreview::GetCargoType()
{
	if (!this->IsEngineValid()) return INVALID_CARGO;
	CargoArray cap = ::GetCapacityOfArticulatedParts(this->engine);

	auto it = std::max_element(std::cbegin(cap), std::cend(cap));
	if (*it == 0) return INVALID_CARGO;

	return CargoType(std::distance(std::cbegin(cap), it));
}

int32_t ScriptEventEnginePreview::GetCapacity()
{
	if (!this->IsEngineValid()) return -1;
	const Engine *e = ::Engine::Get(this->engine);
	switch (e->type) {
		case VEH_ROAD:
		case VEH_TRAIN: {
			CargoArray capacities = GetCapacityOfArticulatedParts(this->engine);
			for (uint &cap : capacities) {
				if (cap != 0) return cap;
			}
			return -1;
		}

		case VEH_SHIP:
		case VEH_AIRCRAFT:
			return e->GetDisplayDefaultCapacity();

		default: NOT_REACHED();
	}
}

int32_t ScriptEventEnginePreview::GetMaxSpeed()
{
	if (!this->IsEngineValid()) return -1;
	const Engine *e = ::Engine::Get(this->engine);
	int32_t max_speed = e->GetDisplayMaxSpeed(); // km-ish/h
	if (e->type == VEH_AIRCRAFT) max_speed /= _settings_game.vehicle.plane_speed;
	return max_speed;
}

Money ScriptEventEnginePreview::GetPrice()
{
	if (!this->IsEngineValid()) return -1;
	return ::Engine::Get(this->engine)->GetCost();
}

Money ScriptEventEnginePreview::GetRunningCost()
{
	if (!this->IsEngineValid()) return -1;
	return ::Engine::Get(this->engine)->GetRunningCost();
}

int32_t ScriptEventEnginePreview::GetVehicleType()
{
	if (!this->IsEngineValid()) return ScriptVehicle::VT_INVALID;
	switch (::Engine::Get(this->engine)->type) {
		case VEH_ROAD:     return ScriptVehicle::VT_ROAD;
		case VEH_TRAIN:    return ScriptVehicle::VT_RAIL;
		case VEH_SHIP:     return ScriptVehicle::VT_WATER;
		case VEH_AIRCRAFT: return ScriptVehicle::VT_AIR;
		default: NOT_REACHED();
	}
}

bool ScriptEventEnginePreview::AcceptPreview()
{
	EnforceCompanyModeValid(false);
	if (!this->IsEngineValid()) return false;
	return ScriptObject::Command<CMD_WANT_ENGINE_PREVIEW>::Do(this->engine);
}

bool ScriptEventCompanyAskMerger::AcceptMerger()
{
	EnforceCompanyModeValid(false);
	return ScriptObject::Command<CMD_BUY_COMPANY>::Do(ScriptCompany::FromScriptCompanyID(this->owner), false);
}

ScriptEventAdminPort::ScriptEventAdminPort(const std::string &json) :
		ScriptEvent(ET_ADMIN_PORT),
		json(json)
{
}


SQInteger ScriptEventAdminPort::GetObject(HSQUIRRELVM vm)
{
	auto json = nlohmann::json::parse(this->json, nullptr, false);

	ScriptAdminConvertFromJSON(json, vm);
	return 1;
}
