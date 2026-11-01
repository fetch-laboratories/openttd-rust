/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file autoreplace.cpp Management of replacement lists. */

#include "stdafx.h"
#include "command_func.h"
#include "group.h"
#include "autoreplace_base.h"
#include "core/pool_func.hpp"

#include "safeguards.h"

/** The pool of autoreplace "orders". */
EngineRenewPool _enginerenew_pool("EngineRenew");
INSTANTIATE_POOL_METHODS(EngineRenew)

/**
 * Retrieves the EngineRenew that specifies the replacement of the given
 * engine type from the given renewlist
 */

/**
 * Remove all engine replacement settings for the company.
 * @param  erl The renewlist for a given company.
 * @return The new renewlist for the company.
 */
void RemoveAllEngineReplacement(EngineRenewList *erl)
{
	openttd_rust_fleet_remove_all_replacements(&FleetGroupServices(), reinterpret_cast<void **>(erl));
}

/**
 * Retrieve the engine replacement in a given renewlist for an original engine type.
 * @param erl The renewlist to search in.
 * @param engine Engine type to be replaced.
 * @param group The group related to this replacement.
 * @param[out] replace_when_old Set to true if the replacement should be done when old.
 * @return The engine type to replace with, or EngineID::Invalid() if no
 * replacement is in the list.
 */
EngineID EngineReplacement(EngineRenewList erl, EngineID engine, GroupID group, bool *replace_when_old)
{
	return EngineID(openttd_rust_fleet_replacement(&FleetGroupServices(), const_cast<EngineRenew *>(erl), engine.base(), group.base(), replace_when_old));
}

/**
 * Add an engine replacement to the given renewlist.
 * @param erl The renewlist to add to.
 * @param old_engine The original engine type.
 * @param new_engine The replacement engine type.
 * @param group The group related to this replacement.
 * @param replace_when_old Replace when old or always?
 * @param flags The calling command flags.
 * @return 0 on success, CMD_ERROR on failure.
 */
CommandCost AddEngineReplacement(EngineRenewList *erl, EngineID old_engine, EngineID new_engine, GroupID group, bool replace_when_old, DoCommandFlags flags)
{
	uint32_t error = openttd_rust_fleet_add_replacement(&FleetGroupServices(), reinterpret_cast<void **>(erl), old_engine.base(), new_engine.base(), group.base(), replace_when_old, flags.base());
	return error == UINT32_MAX ? CommandCost() : CommandCost(error);
}

/**
 * Remove an engine replacement from a given renewlist.
 * @param erl The renewlist from which to remove the replacement
 * @param engine The original engine type.
 * @param group The group related to this replacement.
 * @param flags The calling command flags.
 * @return 0 on success, CMD_ERROR on failure.
 */
CommandCost RemoveEngineReplacement(EngineRenewList *erl, EngineID engine, GroupID group, DoCommandFlags flags)
{
	uint32_t error = openttd_rust_fleet_remove_replacement(&FleetGroupServices(), reinterpret_cast<void **>(erl), engine.base(), group.base(), flags.base());
	return error == UINT32_MAX ? CommandCost() : CommandCost(error);
}
