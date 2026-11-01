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

extern "C" uint64_t openttd_orders_last_station(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetLastStationVisited().base();
}

extern "C" uint64_t openttd_orders_ownerless_station(uint16_t station, uint8_t if_valid) noexcept
{
	{
			BaseStation *bs = if_valid != 0 ? BaseStation::GetIfValid(StationID(station)) : BaseStation::Get(StationID(station));
			return bs != nullptr && bs->owner == OWNER_NONE;
		}
}

extern "C" uint64_t openttd_orders_vehicle_type(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetType();
}

extern "C" uint64_t openttd_orders_vehicle_status(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetVehStatus().base();
}



extern "C" uint64_t openttd_orders_ticks_per_second() noexcept
{
	return Ticks::TICKS_PER_SECOND;
}

extern "C" uint64_t openttd_orders_economy_fraction() noexcept
{
	return TimerGameEconomy::date_fract;
}

extern "C" uint64_t openttd_orders_maximum_date() noexcept
{
	return EconomyTime::MAX_DATE.base();
}

extern "C" uint64_t openttd_orders_timetable_year_limit() noexcept
{
	return TimerGameEconomy::DateAtStartOfYear(MAX_TIMETABLE_START_YEARS).base();
}

extern "C" uint64_t openttd_orders_stopped_or_crashed(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetVehStatus().Any({VehState::Stopped, VehState::Crashed});
}

extern "C" void *openttd_orders_allocate_list() noexcept
{
	return new OrderList();
}

extern "C" uint64_t openttd_orders_suppress_implicit(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return HasBit(v->GetGroundVehicleFlags(), GVF_SUPPRESS_IMPLICIT_ORDERS);
}

extern "C" uint64_t openttd_orders_shared_window(void *ctx, void *first) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return VehicleListIdentifier(VL_SHARED_ORDERS, v->GetType(), v->GetOwner(), static_cast<Vehicle *>(first)->index).ToWindowNumber();
}

extern "C" uint64_t openttd_orders_vehicle_id(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->index.base();
}


extern "C" uint64_t openttd_orders_display_speed(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetDisplayMaxSpeed();
}

extern "C" uint64_t openttd_orders_needs_service(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->NeedsServicing();
}

extern "C" uint64_t openttd_orders_airport_tile(uint16_t destination) noexcept
{
	return RustStationAirportTile(Station::Get(StationID(destination)));
}

extern "C" uint64_t openttd_orders_base_station_tile(uint16_t destination) noexcept
{
	return BaseStation::Get(StationID(destination))->xy.base();
}

extern "C" uint64_t openttd_orders_station_tile(uint16_t destination) noexcept
{
	return Station::Get(StationID(destination))->xy.base();
}

extern "C" uint64_t openttd_orders_depot_tile(uint16_t destination) noexcept
{
	return Depot::Get(DepotID(destination))->xy.base();
}

extern "C" uint64_t openttd_orders_destination_tile(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetDestTile().base();
}

extern "C" uint64_t openttd_orders_aircraft_flying(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return Aircraft::From(v)->state == FLYING;
}

extern "C" uint64_t openttd_orders_target_airport(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return Aircraft::From(v)->targetairport.base();
}

extern "C" uint64_t openttd_orders_waypoint_tile(uint16_t destination) noexcept
{
	return Waypoint::Get(StationID(destination))->xy.base();
}

extern "C" uint64_t openttd_orders_at_station(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return IsTileType(v->GetTile(), MP_STATION);
}

extern "C" uint64_t openttd_orders_valid_station(uint16_t destination) noexcept
{
	return RustStationIfValid<Station *>(destination) != nullptr;
}

extern "C" uint64_t openttd_orders_station_owner(uint16_t destination) noexcept
{
	return Station::Get(StationID(destination))->owner.base();
}





extern "C" uint64_t openttd_orders_station_error(void *ctx, uint16_t destination) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return GetVehicleCannotUseStationReason(v, Station::Get(StationID(destination)));
}



extern "C" uint64_t openttd_orders_valid_depot(uint16_t destination) noexcept
{
	return Depot::GetIfValid(DepotID(destination)) != nullptr;
}

extern "C" uint64_t openttd_orders_depot_owner(uint16_t destination) noexcept
{
	return GetTileOwner(Depot::Get(DepotID(destination))->xy).base();
}

extern "C" uint64_t openttd_orders_rail_depot(uint16_t destination) noexcept
{
	return IsRailDepotTile(Depot::Get(DepotID(destination))->xy);
}

extern "C" uint64_t openttd_orders_road_depot(uint16_t destination) noexcept
{
	return Shared_is_road_depot(Depot::Get(DepotID(destination))->xy.base());
}

extern "C" uint64_t openttd_orders_ship_depot(uint16_t destination) noexcept
{
	return IsShipDepotTile(Depot::Get(DepotID(destination))->xy);
}

extern "C" uint64_t openttd_orders_valid_waypoint(uint16_t destination) noexcept
{
	return Waypoint::GetIfValid(StationID(destination)) != nullptr;
}

extern "C" uint64_t openttd_orders_waypoint_facilities(uint16_t destination) noexcept
{
	return Waypoint::Get(StationID(destination))->facilities.base();
}

extern "C" uint64_t openttd_orders_waypoint_owner(uint16_t destination) noexcept
{
	return Waypoint::Get(StationID(destination))->owner.base();
}

extern "C" uint64_t openttd_orders_list_capacity() noexcept
{
	return OrderList::CanAllocateItem();
}

extern "C" void *openttd_orders_next_backup(uint32_t *cursor) noexcept
{
	/* The original pool iterator advances its physical slot independently of
	 * the object's index, including during partially restored save loading. */
	while (*cursor < OrderBackup::GetPoolSize() && !OrderBackup::IsValidID(*cursor)) ++*cursor;
	if (*cursor >= OrderBackup::GetPoolSize()) return nullptr;
	return OrderBackup::Get((*cursor)++);
}

extern "C" void *openttd_orders_next_vehicle(uint32_t first) noexcept
{
	for (Vehicle *vehicle : Vehicle::Iterate(first)) return vehicle;
	return nullptr;
}

extern "C" uint64_t openttd_orders_aircraft_range(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return Aircraft::From(v)->acache.cached_max_range;
}

extern "C" uint64_t openttd_orders_aircraft_range_square(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return Aircraft::From(v)->acache.cached_max_range_sqr;
}

extern "C" uint64_t openttd_orders_backup_capacity() noexcept
{
	return OrderBackup::CanAllocateItem();
}

extern "C" void *openttd_orders_create_backup(void *ctx, uint32_t user) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return OrderBackup::CreateForRust(v, user);
}





extern "C" uint64_t openttd_orders_network_client() noexcept
{
	return _network_own_client_id;
}

extern "C" uint64_t openttd_orders_server_client() noexcept
{
	return CLIENT_ID_SERVER;
}

extern "C" uint64_t openttd_orders_default_group() noexcept
{
	return DEFAULT_GROUP.base();
}

extern "C" uint64_t openttd_orders_vehicle_tile(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetTile().base();
}

extern "C" uint64_t openttd_orders_vehicle_group(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->group_id.base();
}

extern "C" uint64_t openttd_orders_unique_backup_name(void *ctx) noexcept
{
	return IsUniqueVehicleName(static_cast<OrderBackup *>(ctx)->name);
}




extern "C" uint64_t openttd_orders_review_setting() noexcept
{
	return _settings_client.gui.order_review_system;
}

extern "C" uint64_t openttd_orders_local_owner(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return v->GetOwner() == _local_company;
}

extern "C" uint64_t openttd_orders_fast_aircraft(void *ctx) noexcept
{
	const Vehicle *v = static_cast<const Vehicle *>(ctx);
	return (RustAircraftInfoSubtype(AircraftVehInfo(EngineID(RustVehicleEngineID(v)))) & AIR_FAST) != 0;
}



extern "C" uint64_t openttd_orders_no_jet_crash() noexcept
{
	return _cheats.no_jetcrash.value;
}

extern "C" void openttd_orders_append_station(void *ctx, uint16_t station) noexcept
{
	static_cast<std::vector<StationID> *>(ctx)->push_back(StationID(station));
}

extern "C" void openttd_orders_command_error(void *ctx, uint32_t error, uint32_t detail) noexcept
{
	{
			static const StringID errors[] = {INVALID_STRING_ID, STR_ERROR_TIMETABLE_NOT_STOPPING_HERE, STR_ERROR_TIMETABLE_ONLY_WAIT_AT_STATIONS, STR_ERROR_TIMETABLE_NOT_STARTED, STR_ERROR_TIMETABLE_INCOMPLETE,
				STR_ERROR_CAN_T_ADD_ORDER, STR_ERROR_CAN_T_ADD_ORDER_SHARED, STR_ERROR_UNBUNCHING_NO_FULL_LOAD, STR_ERROR_UNBUNCHING_NO_UNBUNCHING_FULL_LOAD, STR_ERROR_UNBUNCHING_ONLY_ONE_ALLOWED, STR_ERROR_UNBUNCHING_NO_UNBUNCHING_CONDITIONAL,
				STR_ERROR_NO_RAIL_WAYPOINT, STR_ERROR_NO_ROAD_WAYPOINT, STR_ERROR_NO_BUOY, STR_ERROR_UNBUNCHING_NO_CONDITIONAL, STR_ERROR_TOO_MANY_ORDERS, STR_ERROR_NO_MORE_SPACE_FOR_ORDERS, STR_ERROR_CAN_T_COPY_SHARE_ORDER, STR_ERROR_AIRCRAFT_NOT_ENOUGH_RANGE};
			if (error == 0) *static_cast<CommandCost *>(ctx) = CMD_ERROR;
			else if (error == 5 || error == 6 || error == 17) *static_cast<CommandCost *>(ctx) = CommandCost(errors[error], StringID(detail));
			else if (error >= 11 && error <= 13) *static_cast<CommandCost *>(ctx) = CommandCost(STR_ERROR_CAN_T_ADD_ORDER, errors[error]);
			else *static_cast<CommandCost *>(ctx) = CommandCost(errors[error]);

		}
}

extern "C" void openttd_orders_timetable_dirty(void *ctx) noexcept
{
	SetWindowDirty(WC_VEHICLE_TIMETABLE, static_cast<Vehicle *>(ctx)->index);
}

extern "C" void openttd_orders_invalidate_order(void *ctx, int32_t data) noexcept
{
	InvalidateVehicleOrder(static_cast<Vehicle *>(ctx), static_cast<int>(data));
}

extern "C" void openttd_orders_delete_order_news(void *ctx) noexcept
{
	DeleteVehicleNews(static_cast<Vehicle *>(ctx)->index, AdviceType::Order);
}

extern "C" void openttd_orders_suppress_implicit_write(void *ctx, uint8_t suppress) noexcept
{
	{ uint16_t &flags = static_cast<Vehicle *>(ctx)->GetGroundVehicleFlags(); AssignBit(flags, GVF_SUPPRESS_IMPLICIT_ORDERS, suppress != 0);  }
}

extern "C" void openttd_orders_invalidate_vehicle_list(void *ctx) noexcept
{
	InvalidateWindowClassesData(GetWindowClassForVehicleType(static_cast<Vehicle *>(ctx)->GetType()), 0);
}

extern "C" void openttd_orders_close_shared_window(void *ctx, uint32_t window) noexcept
{
	CloseWindowById(GetWindowClassForVehicleType(static_cast<Vehicle *>(ctx)->GetType()), window);
}

extern "C" void openttd_orders_invalidate_shared_window(void *ctx, uint32_t window, int32_t data) noexcept
{
	InvalidateWindowData(GetWindowClassForVehicleType(static_cast<Vehicle *>(ctx)->GetType()), window, static_cast<int>(data));
}

extern "C" void openttd_orders_last_station_write(void *ctx, uint16_t station) noexcept
{
	static_cast<Vehicle *>(ctx)->SetLastStationVisited(StationID(station));
}

extern "C" void openttd_orders_dirty_vehicle_windows(void *ctx) noexcept
{
	SetWindowClassesDirty(GetWindowClassForVehicleType(static_cast<Vehicle *>(ctx)->GetType()));
}

extern "C" void openttd_orders_capture_backup_metadata(void *ctx, void *source_context) noexcept
{
	{ OrderBackup *backup = static_cast<OrderBackup *>(ctx); const Vehicle *source = static_cast<const Vehicle *>(source_context); backup->name = source->name; backup->service_interval = source->service_interval;  }
}

extern "C" void openttd_orders_clear_backup_name(void *ctx) noexcept
{
	static_cast<OrderBackup *>(ctx)->name.clear();
}

extern "C" void openttd_orders_restore_backup_metadata(void *ctx, void *backup_context) noexcept
{
	{ Vehicle *vehicle = static_cast<Vehicle *>(ctx); const OrderBackup *backup = static_cast<const OrderBackup *>(backup_context); vehicle->name = backup->name; vehicle->service_interval = backup->service_interval;  }
}

extern "C" void openttd_orders_order_news(void *ctx, uint32_t message) noexcept
{
	{ static const StringID messages[] = {INVALID_STRING_ID, STR_NEWS_VEHICLE_HAS_VOID_ORDER, STR_NEWS_VEHICLE_HAS_INVALID_ENTRY, STR_NEWS_PLANE_USES_TOO_SHORT_RUNWAY, STR_NEWS_VEHICLE_HAS_DUPLICATE_ENTRY, STR_NEWS_VEHICLE_HAS_TOO_FEW_ORDERS}; const Vehicle *vehicle = static_cast<const Vehicle *>(ctx); AddVehicleAdviceNewsItem(AdviceType::Order, GetEncodedString(messages[message], vehicle->index), vehicle->index);  }
}

extern "C" void openttd_orders_debug_list([[maybe_unused]] void *ctx) noexcept
{
#ifdef WITH_ASSERT
			if (static_cast<Vehicle *>(ctx)->orders != nullptr) static_cast<Vehicle *>(ctx)->orders->DebugCheckSanity();
#endif

}

extern "C" void openttd_orders_assert_departure_range([[maybe_unused]] int32_t lower, [[maybe_unused]] int32_t upper) noexcept
{
	assert(static_cast<int32_t>(lower) <= static_cast<int32_t>(upper));
}

extern "C" void openttd_orders_delete_list(void *ctx) noexcept
{
	delete static_cast<OrderList *>(ctx);
}


extern "C" void openttd_orders_reverse_train(void *ctx) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(ctx);
	Command<CMD_REVERSE_TRAIN_DIRECTION>::Do(DoCommandFlag::Execute, v->index, false);
}

extern "C" void openttd_orders_next_airport(void *ctx) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(ctx);
	AircraftNextAirportPos_and_Order(Aircraft::From(v));
}

extern "C" void openttd_orders_closest_depot(void *ctx, OpenTTDOrdersClosest *result) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(ctx);
	{ ClosestDepot closest = v->FindClosestDepot(); *static_cast<OpenTTDOrdersClosest *>(result) = {closest.location.base(), closest.destination.base(), closest.reverse, closest.found};  }
}

extern "C" void openttd_orders_share_command(void *ctx, const void *source) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(ctx);
	Command<CMD_CLONE_ORDER>::Do(DoCommandFlag::Execute, CO_SHARE, v->index, static_cast<const Vehicle *>(source)->index);
}

extern "C" void openttd_orders_group_command(void *ctx, uint16_t group) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(ctx);
	Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlag::Execute, GroupID(group), v->index, false, VehicleListIdentifier{});
}

extern "C" void openttd_orders_delete_backup(void *ctx) noexcept
{
	delete static_cast<OrderBackup *>(ctx);
}

extern "C" void openttd_orders_clear_backup_gui(uint32_t tile, uint32_t user) noexcept
{
	Command<CMD_CLEAR_ORDER_BACKUP>::Unsafe<CommandCallback>(STR_NULL, nullptr, true, false, TileIndex(tile), CommandTraits<CMD_CLEAR_ORDER_BACKUP>::Args{TileIndex(tile), static_cast<ClientID>(user)});
}

extern "C" void openttd_orders_clear_backup_post(uint32_t user) noexcept
{
	Command<CMD_CLEAR_ORDER_BACKUP>::Post(TileIndex{}, static_cast<ClientID>(user));
}

extern "C" void openttd_orders_missing_aircraft_orders(void *ctx) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(ctx);
	HandleMissingAircraftOrders(Aircraft::From(v));
}

extern "C" void openttd_orders_change_timetable_command(void *ctx, uint8_t index, uint8_t field, uint16_t value) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(ctx);
	Command<CMD_CHANGE_TIMETABLE>::Do(DoCommandFlag::Execute, v->index, static_cast<VehicleOrderID>(index), static_cast<ModifyTimetableFlags>(field), static_cast<uint16_t>(value));
}
static const OpenTTDOrdersLeaves _rust_orders_leaves{
	.vehicle = [](void *v) noexcept { return static_cast<Vehicle *>(v)->GetRustOrderState(); },
	.consist = [](void *v) noexcept { return static_cast<Vehicle *>(v)->rust_orders.get(); },
	.list = [](void *list) noexcept { return static_cast<OrderList *>(list)->GetRustState(); },
	.vector = [](void *list) noexcept { return static_cast<OrderList *>(list)->GetRustOrders(); },
	.backup = [](void *backup) noexcept { return static_cast<OrderBackup *>(backup)->GetRustState(); },
	.backup_vector = [](void *backup) noexcept { return static_cast<OrderBackup *>(backup)->GetRustOrders(); },
	.backup_consist = [](void *backup) noexcept { return static_cast<OrderBackup *>(backup)->rust_orders.get(); },
	.first_vehicle = RustVehicleFirstOpaque,
	.last_station = openttd_orders_last_station,
	.ownerless_station = openttd_orders_ownerless_station,
	.vehicle_type = openttd_orders_vehicle_type,
	.vehicle_status = openttd_orders_vehicle_status,
	.tick_counter = openttd_orders_tick_counter,
	.primary_vehicle = openttd_orders_primary_vehicle,
	.ticks_per_second = openttd_orders_ticks_per_second,
	.unit_number = RustOpaqueVehicleRead<uint64_t, RustVehicleUnitNumber>,
	.economy_fraction = openttd_orders_economy_fraction,
	.maximum_date = openttd_orders_maximum_date,
	.timetable_year_limit = openttd_orders_timetable_year_limit,
	.stopped_or_crashed = openttd_orders_stopped_or_crashed,
	.allocate_list = openttd_orders_allocate_list,
	.suppress_implicit = openttd_orders_suppress_implicit,
	.shared_window = openttd_orders_shared_window,
	.vehicle_id = openttd_orders_vehicle_id,
	.reliability = RustOpaqueVehicleRead<uint64_t, RustVehicleReliability>,
	.display_speed = openttd_orders_display_speed,
	.age_years = openttd_orders_age_years,
	.needs_service = openttd_orders_needs_service,
	.remaining_years = openttd_orders_remaining_years,
	.airport_tile = openttd_orders_airport_tile,
	.base_station_tile = openttd_orders_base_station_tile,
	.station_tile = openttd_orders_station_tile,
	.depot_tile = openttd_orders_depot_tile,
	.distance = OpenTTDLegacyOrderDistance,
	.station_location = RustVehicleLegacyOrderStationLocation,
	.destination_tile = openttd_orders_destination_tile,
	.aircraft_flying = openttd_orders_aircraft_flying,
	.target_airport = openttd_orders_target_airport,
	.waypoint_tile = openttd_orders_waypoint_tile,
	.at_station = openttd_orders_at_station,
	.tile_station = RustOpaqueVehicleTileRead<uint64_t, MapSvcStationIndex>,
	.ship_station_tile = openttd_orders_ship_station_tile,
	.valid_station = openttd_orders_valid_station,
	.station_owner = openttd_orders_station_owner,
	.station_error = openttd_orders_station_error,
	.valid_depot = openttd_orders_valid_depot,
	.depot_owner = openttd_orders_depot_owner,
	.rail_depot = openttd_orders_rail_depot,
	.road_depot = openttd_orders_road_depot,
	.ship_depot = openttd_orders_ship_depot,
	.valid_waypoint = openttd_orders_valid_waypoint,
	.waypoint_facilities = openttd_orders_waypoint_facilities,
	.waypoint_owner = openttd_orders_waypoint_owner,
	.list_capacity = openttd_orders_list_capacity,
	.next_backup = openttd_orders_next_backup,
	.next_vehicle = openttd_orders_next_vehicle,
	.aircraft_range = openttd_orders_aircraft_range,
	.aircraft_range_square = openttd_orders_aircraft_range_square,
	.backup_capacity = openttd_orders_backup_capacity,
	.create_backup = openttd_orders_create_backup,
	.networking = RustSettingRead<uint64_t, RustNetworking>,
	.network_server = RustNetworkServer,
	.network_client = openttd_orders_network_client,
	.server_client = openttd_orders_server_client,
	.default_group = openttd_orders_default_group,
	.vehicle_tile = openttd_orders_vehicle_tile,
	.vehicle_group = openttd_orders_vehicle_group,
	.unique_backup_name = openttd_orders_unique_backup_name,
	.review_setting = openttd_orders_review_setting,
	.local_owner = openttd_orders_local_owner,
	.fast_aircraft = openttd_orders_fast_aircraft,
	.no_jet_crash = openttd_orders_no_jet_crash,
	.append_station = openttd_orders_append_station,
	.station_commands = &GetRustStationCommandServices(),
	.command_error = openttd_orders_command_error,
	.timetable_dirty = openttd_orders_timetable_dirty,
	.invalidate_order = openttd_orders_invalidate_order,
	.delete_order_news = openttd_orders_delete_order_news,
	.suppress_implicit_write = openttd_orders_suppress_implicit_write,
	.invalidate_vehicle_list = openttd_orders_invalidate_vehicle_list,
	.close_shared_window = openttd_orders_close_shared_window,
	.invalidate_shared_window = openttd_orders_invalidate_shared_window,
	.last_station_write = openttd_orders_last_station_write,
	.dirty_vehicle_windows = openttd_orders_dirty_vehicle_windows,
	.capture_backup_metadata = openttd_orders_capture_backup_metadata,
	.clear_backup_name = openttd_orders_clear_backup_name,
	.restore_backup_metadata = openttd_orders_restore_backup_metadata,
	.order_news = openttd_orders_order_news,
	.debug_list = openttd_orders_debug_list,
	.assert_departure_range = openttd_orders_assert_departure_range,
	.delete_list = openttd_orders_delete_list,
	.reverse_train = openttd_orders_reverse_train,
	.next_airport = openttd_orders_next_airport,
	.set_destination = openttd_orders_set_destination,
	.closest_depot = openttd_orders_closest_depot,
	.share_command = openttd_orders_share_command,
	.group_command = openttd_orders_group_command,
	.delete_backup = openttd_orders_delete_backup,
	.clear_backup_gui = openttd_orders_clear_backup_gui,
	.clear_backup_post = openttd_orders_clear_backup_post,
	.missing_aircraft_orders = openttd_orders_missing_aircraft_orders,
	.change_timetable_command = openttd_orders_change_timetable_command,
	.base = &GetRustVehicleServices(),
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
 * Makes this order a Leave Station order.
 */
void Order::MakeLeaveStation()
{
	openttd_rust_order_leave(this);
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
	openttd_rust_list_init(this, v, &_rust_orders_leaves);
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
	if (openttd_rust_list_free(this, keep_orderlist, &_rust_orders_leaves)) delete this;
}

/**
 * Get the next order which will make the given vehicle stop at a station
 * or refit at a depot or evaluate a non-trivial condition.
 * @param next The order to start looking at.
 * @param hops The number of orders we have already looked at.
 * @return Either of
 *         \li a station order
 *         \li a refitting depot order
 *         \li a non-trivial conditional order
 *         \li INVALID_VEH_ORDER_ID if the vehicle won't stop anymore.
 */
VehicleOrderID OrderList::GetNextDecisionNode(VehicleOrderID next, uint hops) const
{
	return openttd_rust_list_decision(const_cast<OrderList *>(this), next, hops, &_rust_orders_leaves);
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
	openttd_rust_list_stopping(const_cast<OrderList *>(this), const_cast<Vehicle *>(v), first, hops, &next_station, &_rust_orders_leaves);
}

/**
 * Insert a new order into the order chain.
 * @param new_order is the order to insert into the chain.
 * @param index is the position where the order is supposed to be inserted.
 */
void OrderList::InsertOrderAt(Order &&order, VehicleOrderID index)
{
	openttd_rust_list_insert(this, index, &order, &_rust_orders_leaves);
}


/**
 * Remove an order from the order list and delete it.
 * @param index is the position of the order which is to be deleted.
 */
void OrderList::DeleteOrderAt(VehicleOrderID index)
{
	openttd_rust_list_delete(this, index, &_rust_orders_leaves);
}

/**
 * Move an order to another position within the order list.
 * @param from is the zero-based position of the order to move.
 * @param to is the zero-based position where the order is moved to.
 */
void OrderList::MoveOrder(VehicleOrderID from, VehicleOrderID to)
{
	openttd_rust_list_move(this, from, to, &_rust_orders_leaves);
}

/**
 * Removes the vehicle from the shared order list.
 * @note This is supposed to be called when the vehicle is still in the chain
 * @param v vehicle to remove from the list
 */
void OrderList::RemoveVehicle(Vehicle *v)
{
	openttd_rust_list_remove_vehicle(this, v, &_rust_orders_leaves);
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
 * Checks whether the order goes to a station or not, i.e. whether the
 * destination is a station
 * @param v the vehicle to check for
 * @param o the order to check
 * @return true if the destination is a station
 */

/**
 * Delete all news items regarding defective orders about a vehicle
 * This could kill still valid warnings (for example about void order when just
 * another order gets added), but assume the company will notice the problems,
 * when they're changing the orders.
 */

/**
 * Returns a tile somewhat representing the order destination (not suitable for pathfinding).
 * @param v The vehicle to get the location for.
 * @param airport Get the airport tile and not the station location for aircraft.
 * @return destination of order, or INVALID_TILE if none.
 */
TileIndex Order::GetLocation(const Vehicle *v, bool airport) const
{
	return TileIndex(openttd_rust_order_location(const_cast<Vehicle *>(v), airport, this, &GetRustOrdersLeaves()));
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
	return openttd_rust_order_distance(const_cast<Vehicle *>(v), prev, cur, conditional_depth, &GetRustOrdersLeaves());
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
	CommandCost result; openttd_rust_command_insert(Vehicle::GetIfValid(veh), sel_ord, flags.Test(DoCommandFlag::Execute), &new_order, &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Insert a new order but skip the validation.
 * @param v       The vehicle to insert the order to.
 * @param new_o   The new order.
 * @param sel_ord The position the order should be inserted at.
 */
void InsertOrder(Vehicle *v, Order &&new_o, VehicleOrderID sel_ord)
{
	openttd_rust_insert_order(v, sel_ord, &new_o, &GetRustOrdersLeaves());
}

/**
 * Declone an order-list
 * @param *dst delete the orders of this vehicle
 * @param flags execution flags
 */

/**
 * Delete an order from the orderlist of a vehicle.
 * @param flags operation to perform
 * @param veh_id the ID of the vehicle
 * @param sel_ord the order to delete (max 255)
 * @return the cost of this operation or an error
 */
CommandCost CmdDeleteOrder(DoCommandFlags flags, VehicleID veh_id, VehicleOrderID sel_ord)
{
	CommandCost result; openttd_rust_command_delete(Vehicle::GetIfValid(veh_id), sel_ord, flags.Test(DoCommandFlag::Execute), &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Cancel the current loading order of the vehicle as the order was deleted.
 * @param v the vehicle
 */

/**
 * Delete an order but skip the parameter validation.
 * @param v       The vehicle to delete the order from.
 * @param sel_ord The id of the order to be deleted.
 */
void DeleteOrder(Vehicle *v, VehicleOrderID sel_ord)
{
	openttd_rust_delete_order(v, sel_ord, &GetRustOrdersLeaves());
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
	CommandCost result; openttd_rust_command_skip(Vehicle::GetIfValid(veh_id), sel_ord, flags.Test(DoCommandFlag::Execute), &result, &GetRustOrdersLeaves()); return result;
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
	CommandCost result; openttd_rust_command_move(Vehicle::GetIfValid(veh), moving_order, target_order, flags.Test(DoCommandFlag::Execute), &result, &GetRustOrdersLeaves()); return result;
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
	CommandCost result; openttd_rust_command_modify(Vehicle::GetIfValid(veh), sel_ord, mof, data, flags.Test(DoCommandFlag::Execute), &result, &GetRustOrdersLeaves()); return result;
}

/**
 * Check if an aircraft has enough range for an order list.
 * @param v_new Aircraft to check.
 * @param v_order Vehicle currently holding the order list.
 * @param first First order in the source order list.
 * @return True if the aircraft has enough range for the orders, false otherwise.
 */

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
	CommandCost result; openttd_rust_command_clone(Vehicle::GetIfValid(veh_dst), Vehicle::GetIfValid(veh_src), action, flags.Test(DoCommandFlag::Execute), &result, &GetRustOrdersLeaves()); return result;
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
	CommandCost result; openttd_rust_command_refit(Vehicle::GetIfValid(veh), order_number, cargo, flags.Test(DoCommandFlag::Execute), &result, &GetRustOrdersLeaves()); return result;
}


/**
 *
 * Check the orders of a vehicle, to see if there are invalid orders and stuff
 *
 */
void CheckOrders(const Vehicle *v)
{
	openttd_rust_check_orders(const_cast<Vehicle *>(v), &GetRustOrdersLeaves());
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
 * Checks if a vehicle has a depot in its order list.
 * @return True iff at least one order is a depot order.
 */
bool Vehicle::HasDepotOrder() const
{
	return openttd_rust_has_depot(const_cast<Vehicle *>(this), &GetRustOrdersLeaves()) != 0;
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
	openttd_rust_delete_vehicle_orders(v, keep_orderlist, reset_order_indices, &GetRustOrdersLeaves());
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
 *
 * Check if a vehicle has any valid orders
 *
 * @return false if there are no valid orders
 * @note Conditional orders are not considered valid destination orders
 *
 */

/**
 * Process a conditional order and determine the next order.
 * @param order the order the vehicle currently has
 * @param v the vehicle to update
 * @return index of next order to jump to, or INVALID_VEH_ORDER_ID to use the next order
 */
VehicleOrderID ProcessConditionalOrder(const Order *order, const Vehicle *v)
{
	return openttd_rust_order_condition(const_cast<Vehicle *>(v), order, &GetRustOrdersLeaves());
}

/**
 * Update the vehicle's destination tile from an order.
 * @param order the order the vehicle currently has
 * @param v the vehicle to update
 * @param conditional_depth the depth (amount of steps) to go with conditional orders. This to prevent infinite loops.
 * @param pbs_look_ahead Whether we are forecasting orders for pbs reservations in advance. If true, the order indices must not be modified.
 */
bool UpdateOrderDest(Vehicle *v, const Order *order, int conditional_depth, bool pbs_look_ahead)
{
	return openttd_rust_order_destination(v, conditional_depth, pbs_look_ahead, order, &GetRustOrdersLeaves()) != 0;
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
	return openttd_rust_process_orders(v, &GetRustOrdersLeaves()) != 0;
}

/**
 * Check whether the given vehicle should stop at the given station
 * based on this order and the non-stop settings.
 * @param v       the vehicle that might be stopping.
 * @param station the station to stop at.
 * @return true if the vehicle should stop.
 */
bool Order::ShouldStopAtStation(const Vehicle *v, StationID station) const
{
	return openttd_rust_order_should_stop(const_cast<Order *>(this), v->GetLastStationVisited().base(), station.base()) != 0;
}

bool Order::CanLoadOrUnload() const
{
	return openttd_rust_order_can_load_unload(const_cast<Order *>(this)) != 0;
}

/**
 * A vehicle can leave the current station with cargo if:
 * 1. it can load cargo here OR
 * 2a. it could leave the last station with cargo AND
 * 2b. it doesn't have to unload all cargo here.
 */
bool Order::CanLeaveWithCargo(bool has_cargo) const
{
	return openttd_rust_order_can_leave(const_cast<Order *>(this), has_cargo) != 0;
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDOrdersLeaves.hpp"
bool CheckAbi_order_cmd() { return AbiServicesComplete(_rust_orders_leaves); }
