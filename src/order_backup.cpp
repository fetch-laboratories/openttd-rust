/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file order_backup.cpp Handling of order backups. */

#include "stdafx.h"
#include "command_func.h"
#include "core/pool_func.hpp"
#include "network/network.h"
#include "network/network_func.h"
#include "order_backup.h"
#include "vehicle_base.h"
#include "window_func.h"
#include "station_map.h"
#include "order_cmd.h"
#include "group_cmd.h"
#include "vehicle_func.h"

#include "table/strings.h"

#include "safeguards.h"

OrderBackupPool _order_backup_pool("BackupOrder");
INSTANTIATE_POOL_METHODS(OrderBackup)

OrderBackup::~OrderBackup() = default;

/**
 * Create an order backup for the given vehicle.
 * @param v    The vehicle to make a backup of.
 * @param user The user that is requesting the backup.
 */
OrderBackup::OrderBackup(const Vehicle *v, uint32_t user)
{
	openttd_rust_capture_backup(this, v, user, &GetRustOrdersLeaves());
}

/**
 * Restore the data of this order to the given vehicle.
 * @param v The vehicle to restore to.
 */

/**
 * Create an order backup for the given vehicle.
 * @param v    The vehicle to make a backup of.
 * @param user The user that is requesting the backup.
 * @note Will automatically remove any previous backups of this user.
 */
/* static */ void OrderBackup::Backup(const Vehicle *v, uint32_t user)
{
	openttd_rust_backup_create(const_cast<Vehicle *>(v), user, &GetRustOrdersLeaves());
}

/**
 * Restore the data of this order to the given vehicle.
 * @param v    The vehicle to restore to.
 * @param user The user that built the vehicle, thus wants to restore.
 * @note After restoration the backup will automatically be removed.
 */
/* static */ void OrderBackup::Restore(Vehicle *v, uint32_t user)
{
	openttd_rust_backup_restore(v, user, &GetRustOrdersLeaves());
}

/**
 * Reset an OrderBackup given a tile and user.
 * @param tile The tile associated with the OrderBackup.
 * @param user The user associated with the OrderBackup.
 * @note Must not be used from the GUI!
 */
/* static */ void OrderBackup::ResetOfUser(TileIndex tile, uint32_t user)
{
	openttd_rust_backup_clear_user(tile.base(), user, &GetRustOrdersLeaves());
}

/**
 * Clear an OrderBackup
 * @param flags For command.
 * @param tile  Tile related to the to-be-cleared OrderBackup.
 * @param user_id User that had the OrderBackup.
 * @return The cost of this operation or an error.
 */
CommandCost CmdClearOrderBackup(DoCommandFlags flags, TileIndex tile, ClientID user_id)
{
	openttd_rust_command_clear_backup(tile.base(), user_id, flags.Test(DoCommandFlag::Execute), &GetRustOrdersLeaves()); return CommandCost();
}

/**
 * Reset an user's OrderBackup if needed.
 * @param user The user associated with the OrderBackup.
 * @pre _network_server.
 * @note Must not be used from a command.
 */
/* static */ void OrderBackup::ResetUser(uint32_t user)
{
	assert(_network_server); openttd_rust_backup_reset(user, &GetRustOrdersLeaves());
}

/**
 * Reset the OrderBackups from GUI/game logic.
 * @param t        The tile of the order backup.
 * @param from_gui Whether the call came from the GUI, i.e. whether
 *                 it must be synced over the network.
 */
/* static */ void OrderBackup::Reset(TileIndex t, bool from_gui)
{
	openttd_rust_backup_tile_clear(t.base(), from_gui, &GetRustOrdersLeaves());
}

/**
 * Clear the group of all backups having this group ID.
 * @param group The group to clear.
 */
/* static */ void OrderBackup::ClearGroup(GroupID group)
{
	openttd_rust_backup_group_clear(group.base(), &GetRustOrdersLeaves());
}

/**
 * Clear/update the (clone) vehicle from an order backup.
 * @param v The vehicle to clear.
 * @pre v != nullptr
 * @note If it is not possible to set another vehicle as clone
 *       "example", then this backed up order will be removed.
 */
/* static */ void OrderBackup::ClearVehicle(const Vehicle *v)
{
	assert(v != nullptr); openttd_rust_backup_vehicle_clear(const_cast<Vehicle *>(v), &GetRustOrdersLeaves());
}

/**
 * Removes an order from all vehicles. Triggers when, say, a station is removed.
 * @param type The type of the order (OT_GOTO_[STATION|DEPOT|WAYPOINT]).
 * @param destination The destination. Can be a StationID, DepotID or WaypointID.
 * @param hangar Only used for airports in the destination.
 *               When false, remove airport and hangar orders.
 *               When true, remove either airport or hangar order.
 */
/* static */ void OrderBackup::RemoveOrder(OrderType type, DestinationID destination, bool hangar)
{
	openttd_rust_backup_destination_clear(type, destination.base(), hangar, &GetRustOrdersLeaves());
}
