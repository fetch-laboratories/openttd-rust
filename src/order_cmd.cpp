/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file order_cmd.cpp Handling of orders. */

#include "stdafx.h"
#include "debug.h"
#include "command_func.h"
#include "company_func.h"
#include "news_func.h"
#include "strings_func.h"
#include "timetable.h"
#include "vehicle_func.h"
#include "depot_base.h"
#include "core/pool_func.hpp"
#include "aircraft.h"
#include "roadveh.h"
#include "station_base.h"
#include "waypoint_base.h"
#include "company_base.h"
#include "order_backup.h"
#include "cheat_type.h"
#include "order_cmd.h"
#include "train_cmd.h"
#include "timetable_cmd.h"
#include "group_cmd.h"
#include "network/network.h"
#include "network/network_func.h"

#include "table/strings.h"

#include "safeguards.h"

/* DestinationID must be at least as large as every these below, because it can
 * be any of them
 */
static_assert(sizeof(DestinationID) >= sizeof(DepotID));
static_assert(sizeof(DestinationID) >= sizeof(StationID));

#include "rust/environment_services.h"
#include "rust/settings_services.h"
#include "rust/services_map.h"
#include "rust/vehicle_services.h"
#include "rust/services_station.h"
#include "rust/station_command_ffi.h"
#include "rust/engine_services.h"
#include "rust/depot_ffi.h"
#include "rust/vehicle_ffi.h"
#include "rust/vehicle_transaction_ffi.h"
static_assert(sizeof(Order) == sizeof(OpenTTDOrderFields) && alignof(Order) == alignof(OpenTTDOrderFields));
static_assert(std::is_trivially_copyable_v<Order> && std::is_trivially_destructible_v<Order>);
static_assert(sizeof(VehicleFlags) == sizeof(uint16_t) && alignof(VehicleFlags) == alignof(uint16_t));
static_assert(sizeof(GroupID) == sizeof(uint16_t) && alignof(GroupID) == alignof(uint16_t));
static_assert(sizeof(DestinationID) == sizeof(uint16_t) && alignof(DestinationID) == alignof(uint16_t));
static_assert(sizeof(TileIndex) == sizeof(uint32_t) && alignof(TileIndex) == alignof(uint32_t));
static_assert(std::is_trivially_copyable_v<VehicleFlags> && std::is_trivially_destructible_v<VehicleFlags>);
static_assert(std::is_trivially_destructible_v<GroupID>);
static_assert(std::is_trivially_copyable_v<DestinationID> && std::is_trivially_destructible_v<DestinationID>);
static_assert(std::is_trivially_destructible_v<TileIndex>);
extern "C" void openttd_orders_construct(void *data, size_t length) noexcept
{
	Order *orders = static_cast<Order *>(data);
	for (size_t i = 0; i < length; ++i) new (orders + i) Order{};
}
extern "C" void openttd_orders_destroy(void *data, size_t length) noexcept
{
	Order *orders = static_cast<Order *>(data);
	for (size_t i = 0; i < length; ++i) std::destroy_at(orders + i);
}

/* The original pool iterator advances its physical slot independently of
 * the object's index, including during partially restored save loading. */
static void *RustOrderBackupNext(uint32_t *cursor) noexcept
{
	while (*cursor < OrderBackup::GetPoolSize() && !OrderBackup::IsValidID(*cursor)) ++*cursor;
	if (*cursor >= OrderBackup::GetPoolSize()) return nullptr;
	return OrderBackup::Get((*cursor)++);
}

static void RustOrderListDebugCheckSanity([[maybe_unused]] void *list) noexcept
{
#ifdef WITH_ASSERT
	static_cast<OrderList *>(list)->DebugCheckSanity();
#endif
}

static const OpenTTDOrdersLeaves _rust_orders_leaves{
	.allocate_list = []() noexcept -> void * { return new OrderList(); },
	.backup = [](void *backup) noexcept { return static_cast<OrderBackup *>(backup)->GetRustState(); },
	.backup_can_allocate_item = []() noexcept -> uint8_t { return OrderBackup::CanAllocateItem(); },
	.backup_consist = [](void *backup) noexcept { return static_cast<OrderBackup *>(backup)->rust_orders.get(); },
	.backup_vector = [](void *backup) noexcept { return static_cast<OrderBackup *>(backup)->GetRustOrders(); },
	.create_backup = [](void *v, uint32_t user) noexcept -> void * { return OrderBackup::CreateForRust(static_cast<const Vehicle *>(v), user); },
	.delete_backup = [](void *backup) noexcept { delete static_cast<OrderBackup *>(backup); },
	.delete_list = [](void *list) noexcept { delete static_cast<OrderList *>(list); },
	.list = [](void *list) noexcept { return static_cast<OrderList *>(list)->GetRustState(); },
	.list_can_allocate_item = []() noexcept -> uint8_t { return OrderList::CanAllocateItem(); },
	.next_backup = RustOrderBackupNext,
	.vector = [](void *list) noexcept { return static_cast<OrderList *>(list)->GetRustOrders(); },
	.backup_clear_name = [](void *backup) noexcept { static_cast<OrderBackup *>(backup)->name.clear(); },
	.backup_name = [](const void *backup) noexcept -> OpenTTDByteSpan { const std::string &name = static_cast<const OrderBackup *>(backup)->name; return {reinterpret_cast<const uint8_t *>(name.data()), name.size()}; },
	.backup_service_interval = [](const void *backup) noexcept { return static_cast<const OrderBackup *>(backup)->service_interval; },
	.backup_set_name = [](void *backup, const void *v) noexcept { static_cast<OrderBackup *>(backup)->name = static_cast<const Vehicle *>(v)->name; },
	.backup_set_service_interval = [](void *backup, uint16_t interval) noexcept { static_cast<OrderBackup *>(backup)->service_interval = interval; },
	.vehicle_set_name = RustOpaqueVehicleCopyName<OrderBackup>,
	.vehicle_set_service_interval = RustOpaqueVehicleInvoke<RustVehicleSetServiceInterval, uint16_t>,
	.base_station_get_if_valid = [](uint16_t index) noexcept { return BaseStation::GetIfValid(StationID(index)); },
	.waypoint_get_if_valid = [](uint16_t index) noexcept -> BaseStation * { return Waypoint::GetIfValid(StationID(index)); },
	.debug_check_sanity = RustOrderListDebugCheckSanity,
	.get_vehicle_cannot_use_station_reason = [](void *v, const BaseStation *st) noexcept -> uint32_t { return GetVehicleCannotUseStationReason(static_cast<const Vehicle *>(v), static_cast<const Station *>(st)); },
	.invalidate_vehicle_order = [](void *v, int32_t data) noexcept { InvalidateVehicleOrder(static_cast<const Vehicle *>(v), data); },
	.append_station = [](void *next_station, uint16_t station) noexcept { static_cast<std::vector<StationID> *>(next_station)->push_back(StationID(station)); },
	.clear_order_backup_unsafe = [](uint32_t tile, uint32_t user) noexcept { Command<CMD_CLEAR_ORDER_BACKUP>::Unsafe<CommandCallback>(STR_NULL, nullptr, true, false, TileIndex(tile), CommandTraits<CMD_CLEAR_ORDER_BACKUP>::Args{TileIndex(tile), static_cast<ClientID>(user)}); },
	.base = &GetRustVehicleServices(),
	.depots = &GetRustDepotServices(),
	.scheduler = &GetRustVehicleSchedulerServices(),
	.station_commands = &GetRustStationCommandServices(),
	.vehicle_transactions = &GetRustVehicleTransactionServices(),
};
const OpenTTDOrdersLeaves &GetRustOrdersLeaves() { return _rust_orders_leaves; }


OrderListPool _orderlist_pool("OrderList");
INSTANTIATE_POOL_METHODS(OrderList)

/**
 * 'Free' the order
 * @note ONLY use on "current_order" vehicle orders!
 */
void Order::Free()
{
	openttd_rust_order_free(this);
}

/**
 * Makes this order a Go To Station order.
 * @param destination the station to go to.
 */
void Order::MakeGoToStation(StationID destination)
{
	openttd_rust_order_station(this, destination.base());
}

/**
 * Makes this order a Go To Depot order.
 * @param destination   the depot to go to.
 * @param order         is this order a 'default' order, or an overridden vehicle order?
 * @param non_stop_type how to get to the depot?
 * @param action        what to do in the depot?
 * @param cargo         the cargo type to change to.
 */
void Order::MakeGoToDepot(DestinationID destination, OrderDepotTypeFlags order, OrderNonStopFlags non_stop_type, OrderDepotActionFlags action, CargoType cargo)
{
	openttd_rust_order_depot(this, destination.base(), order.base(), non_stop_type.base(), action.base(), cargo);
}

/**
 * Makes this order a Go To Waypoint order.
 * @param destination the waypoint to go to.
 */
void Order::MakeGoToWaypoint(StationID destination)
{
	openttd_rust_order_waypoint(this, destination.base());
}

/**
 * Makes this order a Loading order.
 * @param ordered is this an ordered stop?
 */
void Order::MakeLoading(bool ordered)
{
	openttd_rust_order_loading(this, ordered);
}

/**
 * Makes this order a Dummy order.
 */
void Order::MakeDummy()
{
	openttd_rust_order_dummy(this);
}

/**
 * Makes this order an conditional order.
 * @param order the order to jump to.
 */
void Order::MakeConditional(VehicleOrderID order)
{
	openttd_rust_order_conditional(this, order);
}

/**
 * Makes this order an implicit order.
 * @param destination the station to go to.
 */
void Order::MakeImplicit(StationID destination)
{
	openttd_rust_order_implicit(this, destination.base());
}

/**
 * Make this depot/station order also a refit order.
 * @param cargo   the cargo type to change to.
 * @pre IsType(OT_GOTO_DEPOT) || IsType(OT_GOTO_STATION).
 */
void Order::SetRefit(CargoType cargo)
{
	openttd_rust_order_refit(this, cargo);
}

/**
 * Does this order have the same type, flags and destination?
 * @param other the second order to compare to.
 * @return true if the type, flags and destination match.
 */
bool Order::Equals(const Order &other) const
{
	return openttd_rust_order_equals(const_cast<Order *>(this), &other) != 0;
}

/**
 * Pack this order into a 16 bits integer as close to the TTD
 * representation as possible.
 * @return the TTD-like packed representation.
 */
uint16_t Order::MapOldOrder() const
{
	return openttd_rust_order_convert(const_cast<Order *>(this));
}

/**
 *
 * Updates the widgets of a vehicle which contains the order-data
 *
 */
void InvalidateVehicleOrder(const Vehicle *v, int data)
{
	SetWindowDirty(WC_VEHICLE_VIEW, v->index);

	if (data != 0) {
		/* Calls SetDirty() too */
		InvalidateWindowData(WC_VEHICLE_ORDERS,    v->index, data);
		InvalidateWindowData(WC_VEHICLE_TIMETABLE, v->index, data);
		return;
	}

	SetWindowDirty(WC_VEHICLE_ORDERS,    v->index);
	SetWindowDirty(WC_VEHICLE_TIMETABLE, v->index);
}

/**
 *
 * Assign data to an order (from another order)
 *   This function makes sure that the index is maintained correctly
 * @param other the data to copy (except next pointer).
 *
 */
void Order::AssignOrder(const Order &other)
{
	openttd_rust_order_assign(this, &other);
}

/**
 * Recomputes everything.
 * @param chain first order in the chain
 * @param v one of vehicle that is using this orderlist
 */
void OrderList::Initialize(Vehicle *v)
{
	openttd_rust_list_init(this, RustVehicle(v), &_rust_orders_leaves);
}

/**
 * Recomputes Timetable duration.
 * Split out into a separate function so it can be used by afterload.
 */
void OrderList::RecalculateTimetableDuration()
{
	openttd_rust_list_recalculate(this, &_rust_orders_leaves);
}

/**
 * Free a complete order chain.
 * @param keep_orderlist If this is true only delete the orders, otherwise also delete the OrderList.
 * @note do not use on "current_order" vehicle orders!
 */
void OrderList::FreeChain(bool keep_orderlist)
{
	openttd_rust_list_free(this, keep_orderlist, &_rust_orders_leaves);
}

/**
 * Recursively determine the next deterministic station to stop at.
 * @param v The vehicle we're looking at.
 * @param first Order to start searching at or INVALID_VEH_ORDER_ID to start at cur_implicit_order_index + 1.
 * @param hops Number of orders we have already looked at.
 * @return Next stopping station or StationID::Invalid().
 * @pre The vehicle is currently loading and v->last_station_visited is meaningful.
 * @note This function may draw a random number. Don't use it from the GUI.
 */
void OrderList::GetNextStoppingStation(std::vector<StationID> &next_station, const Vehicle *v, VehicleOrderID first, uint hops) const
{
	openttd_rust_list_stopping(const_cast<OrderList *>(this), &next_station, RustVehicle(v), first, hops, &_rust_orders_leaves);
}

/**
 * Insert a new order into the order chain.
 * @param new_order is the order to insert into the chain.
 * @param index is the position where the order is supposed to be inserted.
 */
void OrderList::InsertOrderAt(Order &&order, VehicleOrderID index)
{
	openttd_rust_list_insert(this, &order, index, &_rust_orders_leaves);
}


/**
 * Checks whether all orders of the list have a filled timetable.
 * @return whether all orders have a filled timetable.
 */
bool OrderList::IsCompleteTimetable() const
{
	return openttd_rust_list_complete(const_cast<OrderList *>(this), &_rust_orders_leaves) != 0;
}

#ifdef WITH_ASSERT
/**
 * Checks for internal consistency of order list. Triggers assertion if something is wrong.
 */
void OrderList::DebugCheckSanity() const
{
	VehicleOrderID check_num_orders = 0;
	VehicleOrderID check_num_manual_orders = 0;
	uint check_num_vehicles = 0;
	TimerGameTick::Ticks check_timetable_duration = 0;
	TimerGameTick::Ticks check_total_duration = 0;

	Debug(misc, 6, "Checking OrderList {} for sanity...", this->index);

	for (const Order &o : this->orders) {
		++check_num_orders;
		if (!o.IsType(OT_IMPLICIT)) ++check_num_manual_orders;
		check_timetable_duration += o.GetTimetabledWait() + o.GetTimetabledTravel();
		check_total_duration += o.GetWaitTime() + o.GetTravelTime();
	}
	assert(this->GetNumOrders() == check_num_orders);
	assert(this->num_manual_orders == check_num_manual_orders);
	assert(this->timetable_duration == check_timetable_duration);
	assert(this->total_duration == check_total_duration);

	for (const Vehicle *v = this->first_shared; v != nullptr; v = v->NextShared()) {
		++check_num_vehicles;
		assert(v->orders == this);
	}
	assert(this->num_vehicles == check_num_vehicles);
	Debug(misc, 6, "... detected {} orders ({} manual), {} vehicles, {} timetabled, {} total",
			(uint)this->GetNumOrders(), (uint)this->num_manual_orders,
			this->num_vehicles, this->timetable_duration, this->total_duration);
}
#endif


/**
 * Returns a tile somewhat representing the order destination (not suitable for pathfinding).
 * @param v The vehicle to get the location for.
 * @param airport Get the airport tile and not the station location for aircraft.
 * @return destination of order, or INVALID_TILE if none.
 */
TileIndex Order::GetLocation(const Vehicle *v, bool airport) const
{
	return TileIndex(openttd_rust_order_location(this, RustVehicle(v), airport, &GetRustOrdersLeaves()));
}

/**
 * Get the distance between two orders of a vehicle. Conditional orders are resolved
 * and the bigger distance of the two order branches is returned.
 * @param prev Origin order.
 * @param cur Destination order.
 * @param v The vehicle to get the distance for.
 * @param conditional_depth Internal param for resolving conditional orders.
 * @return Maximum distance between the two orders.
 */
uint GetOrderDistance(VehicleOrderID prev, VehicleOrderID cur, const Vehicle *v, int conditional_depth)
{
	return openttd_rust_order_distance(prev, cur, RustVehicle(v), conditional_depth, &GetRustOrdersLeaves());
}

/**
 * Add an order to the orderlist of a vehicle.
 * @param flags operation to perform
 * @param veh ID of the vehicle
 * @param sel_ord the selected order (if any). If the last order is given,
 *                        the order will be inserted before that one
 *                        the maximum vehicle order id is 254.
 * @param new_order order to insert
 * @return the cost of this operation or an error
 */
CommandCost CmdInsertOrder(DoCommandFlags flags, VehicleID veh, VehicleOrderID sel_ord, const Order &new_order)
{
	CommandCost result; openttd_rust_command_insert(flags.base(), veh.base(), sel_ord, &new_order, &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Insert a new order but skip the validation.
 * @param v       The vehicle to insert the order to.
 * @param new_o   The new order.
 * @param sel_ord The position the order should be inserted at.
 */
void InsertOrder(Vehicle *v, Order &&new_o, VehicleOrderID sel_ord)
{
	openttd_rust_insert_order(RustVehicle(v), &new_o, sel_ord, &GetRustOrdersLeaves());
}


/**
 * Delete an order from the orderlist of a vehicle.
 * @param flags operation to perform
 * @param veh_id the ID of the vehicle
 * @param sel_ord the order to delete (max 255)
 * @return the cost of this operation or an error
 */
CommandCost CmdDeleteOrder(DoCommandFlags flags, VehicleID veh_id, VehicleOrderID sel_ord)
{
	CommandCost result; openttd_rust_command_delete(flags.base(), veh_id.base(), sel_ord, &result, &GetRustOrdersLeaves()); return result;
}


/**
 * Goto order of order-list.
 * @param flags operation to perform
 * @param veh_id The ID of the vehicle which order is skipped
 * @param sel_ord the selected order to which we want to skip
 * @return the cost of this operation or an error
 */
CommandCost CmdSkipToOrder(DoCommandFlags flags, VehicleID veh_id, VehicleOrderID sel_ord)
{
	CommandCost result; openttd_rust_command_skip(flags.base(), veh_id.base(), sel_ord, &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Move an order inside the orderlist
 * @param flags operation to perform
 * @param veh the ID of the vehicle
 * @param moving_order the order to move
 * @param target_order the target order
 * @return the cost of this operation or an error
 * @note The target order will move one place down in the orderlist
 *  if you move the order upwards else it'll move it one place down
 */
CommandCost CmdMoveOrder(DoCommandFlags flags, VehicleID veh, VehicleOrderID moving_order, VehicleOrderID target_order)
{
	CommandCost result; openttd_rust_command_move(flags.base(), veh.base(), moving_order, target_order, &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Modify an order in the orderlist of a vehicle.
 * @param flags operation to perform
 * @param veh ID of the vehicle
 * @param sel_ord the selected order (if any). If the last order is given,
 *                the order will be inserted before that one
 *                the maximum vehicle order id is 254.
 * @param mof what data to modify (@see ModifyOrderFlags)
 * @param data the data to modify
 * @return the cost of this operation or an error
 */
CommandCost CmdModifyOrder(DoCommandFlags flags, VehicleID veh, VehicleOrderID sel_ord, ModifyOrderFlags mof, uint16_t data)
{
	CommandCost result; openttd_rust_command_modify(flags.base(), veh.base(), sel_ord, mof, data, &result, &GetRustOrdersLeaves()); return result;
}


/**
 * Clone/share/copy an order-list of another vehicle.
 * @param flags operation to perform
 * @param action action to perform
 * @param veh_dst destination vehicle to clone orders to
 * @param veh_src source vehicle to clone orders from, if any (none for CO_UNSHARE)
 * @return the cost of this operation or an error
 */
CommandCost CmdCloneOrder(DoCommandFlags flags, CloneOptions action, VehicleID veh_dst, VehicleID veh_src)
{
	CommandCost result; openttd_rust_command_clone(flags.base(), action, veh_dst.base(), veh_src.base(), &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Add/remove refit orders from an order
 * @param flags operation to perform
 * @param veh VehicleIndex of the vehicle having the order
 * @param order_number number of order to modify
 * @param cargo CargoType
 * @return the cost of this operation or an error
 */
CommandCost CmdOrderRefit(DoCommandFlags flags, VehicleID veh, VehicleOrderID order_number, CargoType cargo)
{
	CommandCost result; openttd_rust_command_refit(flags.base(), veh.base(), order_number, cargo, &result, &GetRustOrdersLeaves()); return result;
}


/**
 * Removes an order from all vehicles. Triggers when, say, a station is removed.
 * @param type The type of the order (OT_GOTO_[STATION|DEPOT|WAYPOINT]).
 * @param destination The destination. Can be a StationID, DepotID or WaypointID.
 * @param hangar Only used for airports in the destination.
 *               When false, remove airport and hangar orders.
 *               When true, remove either airport or hangar order.
 */
void RemoveOrderFromAllVehicles(OrderType type, DestinationID destination, bool hangar)
{
	openttd_rust_remove_destination(type, destination.base(), hangar, &GetRustOrdersLeaves());
}


/**
 * Delete all orders from a vehicle
 * @param v                   Vehicle whose orders to reset
 * @param keep_orderlist      If true, do not free the order list, only empty it.
 * @param reset_order_indices If true, reset cur_implicit_order_index and cur_real_order_index
 *                            and cancel the current full load order (if the vehicle is loading).
 *                            If false, _you_ have to make sure the order indices are valid after
 *                            your messing with them!
 */
void DeleteVehicleOrders(Vehicle *v, bool keep_orderlist, bool reset_order_indices)
{
	openttd_rust_delete_vehicle_orders(RustVehicle(v), keep_orderlist, reset_order_indices, &GetRustOrdersLeaves());
}

/**
 * Clamp the service interval to the correct min/max. The actual min/max values
 * depend on whether it's in days, minutes, or percent.
 * @param interval The proposed service interval.
 * @param ispercent Whether the interval is a percent.
 * @return The service interval clamped to use the chosen units.
 */
uint16_t GetServiceIntervalClamped(int interval, bool ispercent)
{
	/* Service intervals are in percents. */
	if (ispercent) return Clamp(interval, MIN_SERVINT_PERCENT, MAX_SERVINT_PERCENT);

	/* Service intervals are in minutes. */
	if (TimerGameEconomy::UsingWallclockUnits(_game_mode == GM_MENU)) return Clamp(interval, MIN_SERVINT_MINUTES, MAX_SERVINT_MINUTES);

	/* Service intervals are in days. */
	return Clamp(interval, MIN_SERVINT_DAYS, MAX_SERVINT_DAYS);
}


/**
 * Handle the orders of a vehicle and determine the next place
 * to go to if needed.
 * @param v the vehicle to do this for.
 * @return true *if* the vehicle is eligible for reversing
 *              (basically only when leaving a station).
 */
bool ProcessOrders(Vehicle *v)
{
	return openttd_rust_process_orders(RustVehicle(v), &GetRustOrdersLeaves()) != 0;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDOrdersLeaves.hpp"
bool CheckAbi_order_cmd() { return AbiServicesComplete(_rust_orders_leaves); }
