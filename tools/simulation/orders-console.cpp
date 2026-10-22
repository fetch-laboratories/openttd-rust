/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file orders-console.cpp Identical native command adapter for the orders scenarios. */
#include "stdafx.h"
#include "console_internal.h"
#include "command_func.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "depot_base.h"
#include "network/network.h"
#include "order_backup.h"
#include "order_cmd.h"
#include "order_func.h"
#include "saveload/saveload.h"
#include "timetable_cmd.h"
#include "cargo_type.h"
#include "vehicle_cmd.h"
#include "vehicle_func.h"
#include "roadveh.h"
#include "station_base.h"

static size_t BackupCount()
{
	size_t count = 0;
	for (OrderBackup *backup : OrderBackup::Iterate()) { (void)backup; ++count; }
	return count;
}

static void CheckCommand(const char *name, const CommandCost &cost)
{
	fmt::print(stderr, "ORDERS command {} {}\n", name, cost.Succeeded());
	if (cost.Failed()) { fmt::print(stderr, "ORDERS failure {} {}\n", name, cost.GetErrorMessage()); std::abort(); }
}

static void State(const char *name, const Vehicle *v)
{
	fmt::print(stderr, "ORDERS state {} {} {} {} {} {} {} {} {} {} {}\n", name, v->index.base(), v->GetNumOrders(), v->orders == nullptr ? 0 : v->orders->GetNumVehicles(), v->cur_real_order_index, v->cur_implicit_order_index, v->round_trip_time, v->depot_unbunching_last_departure, v->depot_unbunching_next_departure, v->timetable_start, v->lateness_counter);
}

static bool VisitsScenario(std::span<std::string_view> args)
{
	if (args[1] != "visits") return false;
	/* Declared inputs on padhattan-ridge-2000's running fleet (#349): each order
	 * change is an ordinary command, checked against the vehicle it expects. */
	auto vehicle = [](uint16_t index, VehicleType type) {
		Vehicle *u = Vehicle::GetIfValid(VehicleID(index));
		if (u == nullptr || u->type != type || !u->IsPrimaryVehicle()) std::abort();
		return u;
	};
	Backup<CompanyID> company(_current_company, CompanyID::Begin());
	auto station = [&](Vehicle *u, VehicleOrderID index, uint16_t destination) {
		Order order;
		order.MakeGoToStation(StationID(destination));
		order.SetStopLocation(OrderStopLocation::FarEnd);
		CheckCommand("visit-station", Command<CMD_INSERT_ORDER>::Do(DoCommandFlag::Execute, u->index, index, order));
	};
	/* Refit to goods at the first stop: a new goods graph, then joining 20, with
	 * aircraft mail narrowing and the refit reset at the next station. */
	Vehicle *refit = vehicle(44, VEH_AIRCRAFT);
	CheckCommand("refit", Command<CMD_ORDER_REFIT>::Do(DoCommandFlag::Execute, refit->index, 0, 5));
	/* Autorefit at the second stop predicts every refittable cargo. */
	Vehicle *autorefit = vehicle(30, VEH_AIRCRAFT);
	CheckCommand("autorefit", Command<CMD_ORDER_REFIT>::Do(DoCommandFlag::Execute, autorefit->index, 1, CARGO_AUTO_REFIT));
	/* A conditional load-percentage branch back to the first stop. */
	Vehicle *conditional = vehicle(18, VEH_ROAD);
	Order branch;
	branch.MakeConditional(0);
	CheckCommand("conditional", Command<CMD_INSERT_ORDER>::Do(DoCommandFlag::Execute, conditional->index, 1, branch));
	CheckCommand("condition", Command<CMD_MODIFY_ORDER>::Do(DoCommandFlag::Execute, conditional->index, 1, MOF_COND_VARIABLE, to_underlying(OrderConditionVariable::LoadPercentage)));
	CheckCommand("comparator", Command<CMD_MODIFY_ORDER>::Do(DoCommandFlag::Execute, conditional->index, 1, MOF_COND_COMPARATOR, to_underlying(OrderConditionComparator::LessThan)));
	CheckCommand("value", Command<CMD_MODIFY_ORDER>::Do(DoCommandFlag::Execute, conditional->index, 1, MOF_COND_VALUE, 50));
	/* Full load against a short timetabled round trip. */
	Vehicle *full = vehicle(2, VEH_SHIP);
	if (full->GetOrder(0)->GetLoadType() != OrderLoadType::FullLoadAny) CheckCommand("full-load", Command<CMD_MODIFY_ORDER>::Do(DoCommandFlag::Execute, full->index, 0, MOF_LOAD, to_underlying(OrderLoadType::FullLoadAny)));
	for (VehicleOrderID index = 0; index < 2; ++index) {
		CheckCommand("full-wait", Command<CMD_CHANGE_TIMETABLE>::Do(DoCommandFlag::Execute, full->index, index, MTF_WAIT_TIME, 30));
		CheckCommand("full-travel", Command<CMD_CHANGE_TIMETABLE>::Do(DoCommandFlag::Execute, full->index, index, MTF_TRAVEL_TIME, 30));
	}
	/* Merge goods {4,6,13} with the smaller {10,12}, then the smaller passenger
	 * ship graph {24,25} into {0,1,16}. */
	station(vehicle(7, VEH_ROAD), 2, 12);
	station(vehicle(42, VEH_SHIP), 4, 1);
	/* Stopped buses whose next hop leaves their graph: their link goes stale and
	 * the station tick's refresh may not merge. */
	for (uint16_t index : {3, 4}) {
		Vehicle *bus = vehicle(index, VEH_ROAD);
		station(bus, 2, 8);
		CheckCommand("stop", Command<CMD_START_STOP_VEHICLE>::Do(DoCommandFlag::Execute, bus->index, false));
	}
	fmt::print(stderr, "VISITS setup {} {} {}\n", refit->GetNumOrders(), conditional->GetNumOrders(), full->orders->GetTotalDuration());
	company.Restore();
	return true;
}

static bool OrdersScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1].starts_with("visits")) return VisitsScenario(args);
	Vehicle *v = Vehicle::Get(VehicleID(126));
	Backup<CompanyID> company(_current_company, v->owner);
	if (args[1] == "restore") {
		fmt::print(stderr, "ORDERS backups before {}\n", BackupCount());
		CheckCommand("clear-for-restore", Command<CMD_DELETE_ORDER>::Do(DoCommandFlag::Execute, v->index, VehicleOrderID(255)));
		bool shared = args.size() == 3 && args[2] == "shared";
		OrderBackup::Restore(v, shared ? 102 : 101);
		State(shared ? "shared-restored" : "unique-restored", v);
		fmt::print(stderr, "ORDERS backups after {}\n", BackupCount());
		company.Restore();
		return true;
	}
	if (args[1] == "inspect") {
		fmt::print(stderr, "ORDERS backups offline {}\n", BackupCount());
		State("reload", v);
		company.Restore();
		return true;
	}
	if (args[1] == "implicit-wrap") {
		Order station;
		for (const Order &order : v->Orders()) if (order.IsType(OT_GOTO_STATION)) { station = order; break; }
		if (!station.IsType(OT_GOTO_STATION)) std::abort();
		CheckCommand("implicit-clear", Command<CMD_DELETE_ORDER>::Do(DoCommandFlag::Execute, v->index, VehicleOrderID(255)));
		if (!OrderList::CanAllocateItem()) std::abort();
		Order implicit;
		implicit.MakeImplicit(station.GetDestination().ToStationID());
		InsertOrder(v, Order(implicit), 0);
		InsertOrder(v, Order(station), 1);
		InsertOrder(v, Order(implicit), 2);
		v->cur_implicit_order_index = 2;
		v->cur_real_order_index = 1;
		ClrBit(v->GetGroundVehicleFlags(), GVF_SUPPRESS_IMPLICIT_ORDERS);
		v->DeleteUnreachedImplicitOrders();
		State("implicit-wrap", v);
		company.Restore();
		return true;
	}
	if (args[1] == "oversized") {
		Order station;
		for (const Order &order : v->Orders()) if (order.IsType(OT_GOTO_STATION)) { station = order; break; }
		if (!station.IsType(OT_GOTO_STATION)) std::abort();
		CheckCommand("oversized-clear", Command<CMD_DELETE_ORDER>::Do(DoCommandFlag::Execute, v->index, VehicleOrderID(255)));
		if (!OrderList::CanAllocateItem()) std::abort();
		/* SlOrders accepts more than 255 stored orders. GetNumOrders narrows,
		 * while GetOrderDistance indexes the full span. Supply that live
		 * post-load shape through the same native storage API. */
		v->orders = new OrderList(v);
		for (size_t index = 0; index < 300; ++index) v->orders->InsertOrderAt(Order(station), 255);
		Order conditional;
		conditional.MakeConditional(200);
		conditional.SetConditionVariable(OrderConditionVariable::Unconditionally);
		v->Orders()[0] = conditional;
		fmt::print(stderr, "ORDERS oversized {} {}\n", v->Orders().size(), GetOrderDistance(1, 0, v));
		company.Restore();
		return true;
	}
	if (args[1] == "setup") {
		if (!v->IsInDepot()) std::abort();
		/* Retain the player's legal station destinations before detaching the list. */
		std::vector<Order> stations;
		for (const Order &order : v->Orders()) if (order.IsType(OT_GOTO_STATION)) stations.emplace_back(order);
		if (stations.size() < 2) std::abort();
		CheckCommand("clear", Command<CMD_DELETE_ORDER>::Do(DoCommandFlag::Execute, v->index, VehicleOrderID(255)));
		for (uint8_t index = 0; index < 2; ++index) {
			Order order;
			order.MakeGoToStation(stations[index].GetDestination().ToStationID());
			order.SetStopLocation(OrderStopLocation::FarEnd);
			CheckCommand("insert", Command<CMD_INSERT_ORDER>::Do(DoCommandFlag::Execute, v->index, index, order));
			CheckCommand("wait", Command<CMD_CHANGE_TIMETABLE>::Do(DoCommandFlag::Execute, v->index, index, MTF_WAIT_TIME, 43 + index));
			CheckCommand("travel", Command<CMD_CHANGE_TIMETABLE>::Do(DoCommandFlag::Execute, v->index, index, MTF_TRAVEL_TIME, 74 + index));
		}
		CheckCommand("bulk", Command<CMD_BULK_CHANGE_TIMETABLE>::Do(DoCommandFlag::Execute, v->index, MTF_TRAVEL_TIME, 148));
		CheckCommand("autofill", Command<CMD_AUTOFILL_TIMETABLE>::Do(DoCommandFlag::Execute, v->index, true, true));
		CheckCommand("autofill-off", Command<CMD_AUTOFILL_TIMETABLE>::Do(DoCommandFlag::Execute, v->index, false, false));
		CheckCommand("start", Command<CMD_SET_TIMETABLE_START>::Do(DoCommandFlag::Execute, v->index, false, TimerGameTick::counter + 100));
		OrderBackup::Backup(v, 101);
		auto [cost, peer_id] = Command<CMD_CLONE_VEHICLE>::Do(DoCommandFlag::Execute, v->tile, v->index, true);
		CheckCommand("clone", cost);
		Vehicle *peer = Vehicle::Get(peer_id);
		CheckCommand("group-start", Command<CMD_SET_TIMETABLE_START>::Do(DoCommandFlag::Execute, v->index, true, TimerGameTick::counter + 200));
		CheckCommand("on-time", Command<CMD_SET_VEHICLE_ON_TIME>::Do(DoCommandFlag::Execute, v->index, true));
		const Depot *depot = nullptr;
		for (Depot *candidate : Depot::Iterate()) if (candidate->xy == v->tile) depot = candidate;
		if (depot == nullptr) std::abort();
		Order order;
		order.MakeGoToDepot(depot->index, OrderDepotTypeFlag::PartOfOrders, {}, OrderDepotActionFlag::Unbunch, CARGO_NO_REFIT);
		CheckCommand("unbunch", Command<CMD_INSERT_ORDER>::Do(DoCommandFlag::Execute, v->index, 2, order));
		/* Declared typed inputs: both real depot peers have initialized trip samples. */
		v->round_trip_time = 4000;
		peer->round_trip_time = 6000;
		v->depot_unbunching_last_departure = TimerGameTick::counter - 3000;
		v->cur_real_order_index = v->cur_implicit_order_index = 2;
		v->current_order = *v->GetOrder(2);
		v->dest_tile = v->tile;
		v->vehstatus.Reset(VehState::Stopped);
		peer->vehstatus.Reset(VehState::Stopped);
		VehicleEnterDepot(v);
		v->vehstatus.Reset(VehState::Stopped);
		v->LeaveUnbunchingDepot();
		fmt::print(stderr, "ORDERS waiting {} {}\n", v->IsWaitingForUnbunching(), peer->IsWaitingForUnbunching());
		State("departure", v);
		State("peer", peer);
		v->vehstatus.Set(VehState::Stopped);
		peer->vehstatus.Set(VehState::Stopped);
		OrderBackup::Backup(v, 102);
		fmt::print(stderr, "ORDERS backups captured {}\n", BackupCount());
		if (args.size() >= 3) {
			bool shared = args.size() == 4 && args[3] == "shared";
			bool multi = args.size() == 4 && args[3] == "multi";
			/* Exercise each serialized form at index zero: the pinned original's
			 * value-initialized load constructor loses nonzero pool indices. */
			if (shared) {
				OrderBackup::ResetOfUser(INVALID_TILE, 101);
				OrderBackup::ResetOfUser(INVALID_TILE, 102);
				OrderBackup::Backup(v, 102);
			} else if (!multi) {
				OrderBackup::ResetOfUser(INVALID_TILE, 102);
			}
			fmt::print(stderr, "ORDERS backups serialized {} {}\n", multi ? "multi" : shared ? "shared" : "unique", BackupCount());
			Backup<bool> networking(_networking, true);
			Backup<bool> server(_network_server, true);
			if (SaveOrLoad(args[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
			server.Restore();
			networking.Restore();
		}
	} else if (args[1] == "active") {
		CheckCommand("remove-unbunch", Command<CMD_DELETE_ORDER>::Do(DoCommandFlag::Execute, v->index, 2));
		Order conditional;
		conditional.MakeConditional(1);
		conditional.SetConditionVariable(OrderConditionVariable::Unconditionally);
		CheckCommand("conditional", Command<CMD_INSERT_ORDER>::Do(DoCommandFlag::Execute, v->index, 0, conditional));
		Order implicit;
		implicit.MakeImplicit(v->GetOrder(1)->GetDestination().ToStationID());
		InsertOrder(v, std::move(implicit), 1);
		v->cur_real_order_index = v->cur_implicit_order_index = 0;
		v->current_order.Free();
		v->vehstatus.Reset(VehState::Stopped);
		ProcessOrders(v);
		v->vehstatus.Set(VehState::Stopped);
		State("conditional-active", v);
		fmt::print(stderr, "ORDERS active {} {} {}\n", v->GetOrder(0)->GetType(), v->GetOrder(1)->GetType(), v->current_order.GetType());
	}
	company.Restore();
	return true;
}

extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("orders_scenario", OrdersScenario);
}

extern "C" bool __real__Z13AfterLoadGamev();
extern "C" bool __wrap__Z13AfterLoadGamev()
{
	if (std::getenv("OPENTTD_ORDERS_CLIENT") == nullptr) return __real__Z13AfterLoadGamev();
	Backup<bool> networking(_networking, true);
	Backup<bool> server(_network_server, false);
	size_t before = BackupCount();
	bool okay = __real__Z13AfterLoadGamev();
	fmt::print(stderr, "ORDERS client-fixups {} {}\n", before, BackupCount());
	server.Restore();
	networking.Restore();
	return okay;
}
