/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file group_cmd.cpp Handling of the engine groups */

#include "stdafx.h"
#include "command_func.h"
#include "train.h"
#include "vehiclelist.h"
#include "vehicle_func.h"
#include "autoreplace_base.h"
#include "autoreplace_func.h"
#include "string_func.h"
#include "company_func.h"
#include "core/pool_func.hpp"
#include "order_backup.h"
#include "group_cmd.h"

#include "table/strings.h"

#include "rust/fleet_group_services.hpp"
static CommandCost FleetGroupResult(uint32_t error) { return error == UINT32_MAX ? CommandCost() : CommandCost(error); }

#include "safeguards.h"

GroupPool _group_pool("Group");
INSTANTIATE_POOL_METHODS(Group)

/**
 * Clear all caches.
 */
void GroupStatistics::Clear()
{
	openttd_rust_fleet_stats_clear(this->state.state);
}

/**
 * Update children list for each group.
 */
void UpdateGroupChildren()
{
	openttd_rust_fleet_update_children(&FleetGroupServices());
}

/**
 * Get number of vehicles of a specific engine ID.
 * @param engine Engine ID.
 * @returns number of vehicles of this engine ID.
 */
uint16_t GroupStatistics::GetNumEngines(EngineID engine) const
{
	return openttd_rust_fleet_engine_count(this->state.state, engine.base());
}

/**
 * Returns the GroupStatistics for a specific group.
 * @param company Owner of the group.
 * @param id_g    GroupID of the group.
 * @param type    VehicleType of the vehicles in the group.
 * @return Statistics for the group.
 */
/* static */ GroupStatistics &GroupStatistics::Get(CompanyID company, GroupID id_g, VehicleType type)
{
	if (Group::IsValidID(id_g)) {
		Group *g = Group::Get(id_g);
		assert(g->owner == company);
		assert(g->vehicle_type == type);
		return g->statistics;
	}

	if (IsDefaultGroupID(id_g)) return Company::Get(company)->group_default[type];
	if (IsAllGroupID(id_g)) return Company::Get(company)->group_all[type];

	NOT_REACHED();
}

/**
 * Returns the GroupStatistic for the group of a vehicle.
 * @param v Vehicle.
 * @return GroupStatistics for the group of the vehicle.
 */
/* static */ GroupStatistics &GroupStatistics::Get(const Vehicle *v)
{
	return GroupStatistics::Get(v->GetOwner(), v->group_id, v->GetType());
}

/**
 * Returns the GroupStatistic for the ALL_GROUP of a vehicle type.
 * @param v Vehicle.
 * @return GroupStatistics for the ALL_GROUP of the vehicle type.
 */
/* static */ GroupStatistics &GroupStatistics::GetAllGroup(const Vehicle *v)
{
	return GroupStatistics::Get(v->GetOwner(), ALL_GROUP, v->GetType());
}

/**
 * Update all caches after loading a game, changing NewGRF, etc.
 */
/* static */ void GroupStatistics::UpdateAfterLoad()
{
	openttd_rust_fleet_update_afterload(&FleetGroupServices());
}

/**
 * Update num_vehicle when adding or removing a vehicle.
 * @param v Vehicle to count.
 * @param delta +1 to add, -1 to remove.
 */
/* static */ void GroupStatistics::CountVehicle(const Vehicle *v, int delta)
{
	assert(delta == 1 || delta == -1);
	openttd_rust_fleet_count_vehicle(&FleetGroupServices(), const_cast<Vehicle *>(v), delta);
}

/**
 * Update num_engines when adding/removing an engine.
 * @param v Engine to count.
 * @param delta +1 to add, -1 to remove.
 */
/* static */ void GroupStatistics::CountEngine(const Vehicle *v, int delta)
{
	assert(delta == 1 || delta == -1);
	openttd_rust_fleet_count_engine(&FleetGroupServices(), const_cast<Vehicle *>(v), delta);
}

/**
 * Add a vehicle's last year profit to the profit sum of its group.
 */
/* static */ void GroupStatistics::AddProfitLastYear(const Vehicle *v)
{
	openttd_rust_fleet_add_profit(&FleetGroupServices(), const_cast<Vehicle *>(v));
}

/**
 * Add a vehicle to the profit sum of its group.
 */
/* static */ void GroupStatistics::VehicleReachedMinAge(const Vehicle *v)
{
	openttd_rust_fleet_min_age(&FleetGroupServices(), const_cast<Vehicle *>(v));
}

/**
 * Recompute the profits for all groups.
 */
/* static */ void GroupStatistics::UpdateProfits()
{
	openttd_rust_fleet_update_profits(&FleetGroupServices());
}

/**
 * Update autoreplace_defined and autoreplace_finished of all statistics of a company.
 * @param company Company to update statistics for.
 */
/* static */ void GroupStatistics::UpdateAutoreplace(CompanyID company)
{
	openttd_rust_fleet_update_autoreplace(&FleetGroupServices(), company.base());
}

/**
 * Update the num engines of a groupID. Decrease the old one and increase the new one
 * @note called in SetTrainGroupID and UpdateTrainGroupID
 * @param v     Vehicle we have to update
 * @param old_g index of the old group
 * @param new_g index of the new group
 */




/**
 * Propagate a livery change to a group's children, and optionally update cached vehicle colourmaps.
 * @param g Group to propagate colours to children.
 * @param reset_cache Reset colourmap of vehicles in this group.
 */

/**
 * Update group liveries for a company. This is called when the LS_DEFAULT scheme is changed, to update groups with
 * colours set to default.
 * @param c Company to update.
 */


/**
 * Create a new vehicle group.
 * @param flags type of operation
 * @param vt vehicle type
 * @param parent_group parent groupid
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, GroupID> CmdCreateGroup(DoCommandFlags flags, VehicleType vt, GroupID parent_group)
{
	uint16_t id;
	uint32_t error = openttd_rust_fleet_create_group(&FleetGroupServices(), flags.base(), vt, parent_group.base(), &id);
	return {FleetGroupResult(error), GroupID(id)};
}


/**
 * Add all vehicles in the given group to the default group and then deletes the group.
 * @param flags type of operation
 * @param group_id index of group
 * @return the cost of this operation or an error
 */
CommandCost CmdDeleteGroup(DoCommandFlags flags, GroupID group_id)
{
	return FleetGroupResult(openttd_rust_fleet_delete_group(&FleetGroupServices(), flags.base(), group_id.base()));
}

/**
 * Alter a group
 * @param flags type of operation
 * @param mode Operation to perform.
 * @param group_id GroupID
 * @param parent_id parent group index
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdAlterGroup(DoCommandFlags flags, AlterGroupMode mode, GroupID group_id, GroupID parent_id, const std::string &text)
{
	return FleetGroupResult(openttd_rust_fleet_alter_group(&FleetGroupServices(), flags.base(), to_underlying(mode), group_id.base(), parent_id.base(), reinterpret_cast<const uint8_t *>(text.data()), text.size()));
}


/**
 * Do add a vehicle to a group.
 * @param v Vehicle to add.
 * @param new_g Group to add to.
 */

/**
 * Add a vehicle to a group
 * @param flags type of operation
 * @param group_id index of group
 * @param veh_id vehicle to add to a group
 * @param add_shared Add shared vehicles as well.
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, GroupID> CmdAddVehicleGroup(DoCommandFlags flags, GroupID group_id, VehicleID veh_id, bool add_shared, const VehicleListIdentifier &vli)
{
	FleetVehicleListContext context{{}, vli};
	uint16_t id;
	uint32_t error = openttd_rust_fleet_add_vehicle_group(&FleetGroupServices(), flags.base(), group_id.base(), veh_id.base(), add_shared, &context, vli.Valid(), &id);
	return {FleetGroupResult(error), GroupID(id)};
}

/**
 * Add all shared vehicles of all vehicles from a group
 * @param flags type of operation
 * @param id_g index of group
 * @param type type of vehicles
 * @return the cost of this operation or an error
 */
CommandCost CmdAddSharedVehicleGroup(DoCommandFlags flags, GroupID id_g, VehicleType type)
{
	return FleetGroupResult(openttd_rust_fleet_add_shared_group(&FleetGroupServices(), flags.base(), id_g.base(), type));
}


/**
 * Remove all vehicles from a group
 * @param flags type of operation
 * @param group_id index of group
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveAllVehiclesGroup(DoCommandFlags flags, GroupID group_id)
{
	return FleetGroupResult(openttd_rust_fleet_remove_vehicles_group(&FleetGroupServices(), flags.base(), group_id.base()));
}

/**
 * Set the livery for a vehicle group.
 * @param flags     Command flags.
 * @param group_id Group ID.
 * @param primary Set primary instead of secondary colour
 * @param colour Colour.
 */
CommandCost CmdSetGroupLivery(DoCommandFlags flags, GroupID group_id, bool primary, Colours colour)
{
	return FleetGroupResult(openttd_rust_fleet_set_livery(&FleetGroupServices(), flags.base(), group_id.base(), primary, colour));
}

/**
 * Set group flag for a group and its sub-groups.
 * @param g initial group.
 * @param set 1 to set or 0 to clear protection.
 */

/**
 * (Un)set group flag from a group
 * @param flags type of operation
 * @param group_id index of group array
 * @param flag flag to set, by value not bit.
 * @param value value to set the flag to.
 * @param recursive to apply to sub-groups.
 * @return the cost of this operation or an error
 */
CommandCost CmdSetGroupFlag(DoCommandFlags flags, GroupID group_id, GroupFlag flag, bool value, bool recursive)
{
	return FleetGroupResult(openttd_rust_fleet_flag_command(&FleetGroupServices(), flags.base(), group_id.base(), to_underlying(flag), value, recursive));
}

/**
 * Affect the groupID of a train to new_g.
 * @note called in CmdAddVehicleGroup and CmdMoveRailVehicle
 * @param v     First vehicle of the chain.
 * @param new_g index of array group
 */
void SetTrainGroupID(Train *v, GroupID new_g)
{
	/* The original asserts after its validity return; Rust repeats that return. */
	assert(!(Group::IsValidID(new_g) || IsDefaultGroupID(new_g)) || v->IsFrontEngine() || IsDefaultGroupID(new_g));
	openttd_rust_fleet_set_train_group(&FleetGroupServices(), v, new_g.base());
}


/**
 * Recalculates the groupID of a train. Should be called each time a vehicle is added
 * to/removed from the chain,.
 * @note this needs to be called too for 'wagon chains' (in the depot, without an engine)
 * @note Called in CmdBuildRailVehicle, CmdBuildRailWagon, CmdMoveRailVehicle, CmdSellRailWagon
 * @param v First vehicle of the chain.
 */
void UpdateTrainGroupID(Train *v)
{
	assert(v->IsFrontEngine() || v->IsFreeWagon());
	openttd_rust_fleet_update_train_group(&FleetGroupServices(), v);
}

/**
 * Get the number of engines with EngineID id_e in the group with GroupID
 * id_g and its sub-groups.
 * @param company The company the group belongs to
 * @param id_g The GroupID of the group used
 * @param id_e The EngineID of the engine to count
 * @return The number of engines with EngineID id_e in the group
 */
uint GetGroupNumEngines(CompanyID company, GroupID id_g, EngineID id_e)
{
	return openttd_rust_fleet_sum_engines(&FleetGroupServices(), company.base(), id_g.base(), id_e.base());
}

/**
 * Get the number of vehicles in the group with GroupID
 * id_g and its sub-groups.
 * @param company The company the group belongs to
 * @param id_g The GroupID of the group used
 * @param type The vehicle type of the group
 * @return The number of vehicles in the group
 */
uint GetGroupNumVehicle(CompanyID company, GroupID id_g, VehicleType type)
{
	return openttd_rust_fleet_sum_vehicles(&FleetGroupServices(), company.base(), id_g.base(), type);
}

/**
 * Get the number of vehicles above profit minimum age in the group with GroupID
 * id_g and its sub-groups.
 * @param company The company the group belongs to
 * @param id_g The GroupID of the group used
 * @param type The vehicle type of the group
 * @return The number of vehicles above profit minimum age in the group
 */
uint GetGroupNumVehicleMinAge(CompanyID company, GroupID id_g, VehicleType type)
{
	return openttd_rust_fleet_sum_min_age(&FleetGroupServices(), company.base(), id_g.base(), type);
}

/**
 * Get last year's profit of vehicles above minimum age
 * for the group with GroupID id_g and its sub-groups.
 * @param company The company the group belongs to
 * @param id_g The GroupID of the group used
 * @param type The vehicle type of the group
 * @return Last year's profit of vehicles above minimum age for the group
 */
Money GetGroupProfitLastYearMinAge(CompanyID company, GroupID id_g, VehicleType type)
{
	return openttd_rust_fleet_sum_profit(&FleetGroupServices(), company.base(), id_g.base(), type);
}

void RemoveAllGroupsForCompany(const CompanyID company)
{
	openttd_rust_fleet_remove_company_groups(&FleetGroupServices(), company.base());
}


/**
 * Test if GroupID group is a descendant of (or is) GroupID search
 * @param search The GroupID to search in
 * @param group The GroupID to search for
 * @return True iff group is search or a descendant of search
 */
bool GroupIsInGroup(GroupID search, GroupID group)
{
	return openttd_rust_fleet_contains(&FleetGroupServices(), search.base(), group.base());
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDFleetGroupServices.hpp"
bool CheckAbi_group_cmd() { return AbiServicesComplete(FleetGroupServices()); }
