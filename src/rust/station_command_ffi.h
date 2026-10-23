/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_command_ffi.h Station construction policy boundary (ABI 1060-1119). */
#ifndef RUST_STATION_COMMAND_FFI_H
#define RUST_STATION_COMMAND_FFI_H
#include "service_defaults.hpp"
#include "services_ffi.h"
#include "command_services_ffi.h"
#include "station_service_ffi.h"
#include "rail_command_ffi.h"
#include "train_reservation_ffi.h"
#include "ground_ffi.h"
#include "town_ffi.h"
#include "byte_span_ffi.h"
#include "vehicle_ffi.h"
#include "road_ffi.h"
#include "aircraft_ffi.h"
#include "ship_control_ffi.h"
enum class WaterClass : uint8_t;
struct Industry;
struct OpenTTDTileOwnerContext;
struct StationSpec;
struct RoadStopSpec;
struct BridgeableTileInfo;
struct OpenTTDStationArea { uint32_t tile; uint16_t w, h; };
struct OpenTTDStationBridgeInfo { const BridgeableTileInfo *data; size_t size; };
struct OpenTTDStationBounds { int32_t left, top, right, bottom; };
struct OpenTTDStationSpecIndex { uint8_t value; bool valid; };
struct OpenTTDRailStationSpecInfo { uint8_t platforms, lengths; bool slope_callback, availability_callback; };
struct OpenTTDWaypointLayout;
struct RoadStop;
struct OpenTTDRoadGroundServices;
struct OpenTTDRoadStationSpecCost { uint8_t multiplier; int32_t shift; };
struct OpenTTDRoadStationSpecInfo { uint8_t stop_type; bool drive_only, availability_callback; };
struct AirportSpec;
class TileIterator;
struct OpenTTDAirportSize { uint8_t w, h; };
struct OpenTTDAirportLayout { size_t size; uint8_t rotation; };
struct OpenTTDAirportLayoutTile { uint32_t tile; uint8_t gfx; };
struct OpenTTDAirportGeometry { OpenTTDStationArea area; const AirportSpec *spec; uint8_t rotation; };
struct OpenTTDStationCommandStrings {
	uint32_t demolish_bridge, flat_land, spread;
	uint32_t bridge_too_low[9];
	uint32_t too_many_stations, adjoins_multiple, remove_station, remove_rail_waypoint, remove_road_waypoint, remove_road_stop;
	uint32_t name_unique, unsuitable;
	uint32_t too_many_specs, disallowed_tracks, disallowed_length, remove_signals;
	uint32_t no_station;
	uint32_t drive_direction, drive_corner, drive_junction, drive_town, no_road, unsuitable_road, too_many_truck, too_many_bus;
	uint32_t refuse_noise, refuse_airport, close_airport, aircraft_in_way;
	uint32_t demolish_railroad, demolish_building, demolish_airport, demolish_cargo_tram, demolish_truck, demolish_passenger_tram, demolish_bus, buoy_in_way, demolish_dock, object_in_way, oilrig_name;
	uint32_t buoy_in_use, buoy_name, waypoint_adjoins_multiple, waypoint_close, waypoint_name;
};
/* Each pointer is borrowed synchronously under the game-state mutex. Specs and
 * bridge tables remain native and immutable during the entry. No reference to
 * station/map storage survives a callback. All leaves are direct noexcept calls;
 * nested commands may reenter Rust. Ordinary play cannot unwind these services. */
struct OpenTTDStationCommandServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDStationServices *station;
	const OpenTTDRailCommandServices *rail;
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCommandServices *command;
	const OpenTTDTownAuthorityServices *authority;
	OpenTTDStationCommandStrings strings;
	uint8_t (*spread)() noexcept OPENTTD_SERVICE_DEFAULT(spread);
	OpenTTDStationBridgeInfo (*default_bridge_info)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(default_bridge_info);
	OpenTTDStationBridgeInfo (*rail_bridge_info)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(rail_bridge_info);
	OpenTTDStationBridgeInfo (*road_bridge_info)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(road_bridge_info);
	uint32_t (*southern_bridge_end)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(southern_bridge_end);
	OpenTTDStationArea (*train_area)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(train_area);
	const StationSpec *(*rail_spec)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_spec);
	const RoadStopSpec *(*road_spec)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_spec);
	const OpenTTDStationLinks *links;
	OpenTTDStationBounds (*bounds)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(bounds);
	uint32_t (*location)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(location);
	void (*move_sign)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(move_sign);
	void (*virtual_coord)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(virtual_coord);
	void (*catchment)(BaseStation *, bool) noexcept OPENTTD_SERVICE_DEFAULT(catchment);
	void (*catchment_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(catchment_dirty);
	void (*facility_widget)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(facility_widget);
	void (*select_invalidate)() noexcept OPENTTD_SERVICE_DEFAULT(select_invalidate);
	void (*radius)(uint32_t, uint32_t, void *, void (*)(void *, BaseStation *)) noexcept OPENTTD_SERVICE_DEFAULT(radius);
	bool (*station_available)() noexcept OPENTTD_SERVICE_DEFAULT(station_available);
	BaseStation *(*new_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_station);
	void (*index_insert)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(index_insert);
	void (*set_town)(BaseStation *, const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(set_town);
	void (*default_name)(BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(default_name);
	void (*have_rating)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(have_rating);
	void (*before_add_rect)(OpenTTDCommandCost *, BaseStation *, OpenTTDStationArea, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(before_add_rect);
	bool (*road_waypoint)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(road_waypoint);
	bool (*name_equals)(BaseStation *, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(name_equals);
	void (*set_name)(BaseStation *, const uint8_t *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(set_name);
	bool (*rect_contains)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rect_contains);
	void (*flip_airport_closed)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(flip_airport_closed);
	void (*closed_widget)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(closed_widget);
	bool (*distant_join)() noexcept OPENTTD_SERVICE_DEFAULT(distant_join);
	uint32_t (*rail_class_count)() noexcept OPENTTD_SERVICE_DEFAULT(rail_class_count);
	bool (*rail_waypoint_class)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_waypoint_class);
	uint32_t (*rail_spec_count)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_spec_count);
	const StationSpec *(*class_rail_spec)(uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(class_rail_spec);
	OpenTTDRailStationSpecInfo (*rail_spec_info)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(rail_spec_info);
	void (*station_slope_check)(OpenTTDCommandCost *, uint32_t, uint32_t, const StationSpec *, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(station_slope_check);
	OpenTTDByteSpan (*rail_layout)(const StationSpec *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_layout);
	uint16_t (*rail_callback)(uint16_t, uint32_t, uint32_t, const StationSpec *, BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_callback);
	uint32_t (*platform_info)(uint8_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(platform_info);
	OpenTTDStationSpecIndex (*allocate_rail_spec)(const StationSpec *, BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(allocate_rail_spec);
	void (*assign_rail_spec)(const StationSpec *, BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(assign_rail_spec);
	void (*deallocate_rail_spec)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(deallocate_rail_spec);
	bool (*rail_boolean)(const StationSpec *, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_boolean);
	void (*rail_callback_error)(const StationSpec *, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_callback_error);
	void (*set_train_area)(BaseStation *, OpenTTDStationArea) noexcept OPENTTD_SERVICE_DEFAULT(set_train_area);
	void (*add_facility)(BaseStation *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(add_facility);
	void (*rail_triggers)(BaseStation *, const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(rail_triggers);
	bool (*reserved)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reserved);
	Train *(*last_train)(Train *) noexcept OPENTTD_SERVICE_DEFAULT(last_train);
	const OpenTTDTrainServices *train;
	const OpenTTDTrainReservationLeaves *reservation;
	bool (*compatible_tiles)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(compatible_tiles);
	uint32_t (*custom_rail_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(custom_rail_index);
	void (*make_rail_station)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_rail_station);
	void (*close_station_inspect)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(close_station_inspect);
	void (*set_custom_rail_index)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_custom_rail_index);
	void (*set_random_bits)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_random_bits);
	void (*set_gfx)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_gfx);

	size_t (*tile_flag_count)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(tile_flag_count);
	uint8_t (*tile_flags)(const StationSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_flags);
	void (*set_tile_flags)(uint32_t, bool, bool, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_tile_flags);
	void (*station_infrastructure)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_infrastructure);
	void (*mark_tiles)(BaseStation *, bool) noexcept OPENTTD_SERVICE_DEFAULT(mark_tiles);
	void (*after_remove_tile)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(after_remove_tile);
	void (*reset_facilities)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_facilities);
	void (*orders_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(orders_dirty);
	const OpenTTDVehicleServices *vehicle;
	bool (*stop_on_town_road)() noexcept OPENTTD_SERVICE_DEFAULT(stop_on_town_road);
	bool (*stop_on_competitor_road)() noexcept OPENTTD_SERVICE_DEFAULT(stop_on_competitor_road);
	uint32_t (*road_class_count)() noexcept OPENTTD_SERVICE_DEFAULT(road_class_count);
	bool (*road_waypoint_class)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(road_waypoint_class);
	uint32_t (*road_spec_count)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(road_spec_count);
	const RoadStopSpec *(*class_road_spec)(uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(class_road_spec);
	OpenTTDRoadStationSpecInfo (*road_spec_info)(const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(road_spec_info);
	OpenTTDRoadStationSpecCost (*road_spec_build_cost)(const RoadStopSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_spec_build_cost);
	OpenTTDRoadStationSpecCost (*road_spec_clear_cost)(const RoadStopSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_spec_clear_cost);
	bool (*stops_available)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(stops_available);
	OpenTTDStationSpecIndex (*allocate_road_spec)(const RoadStopSpec *, BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(allocate_road_spec);
	void (*assign_road_spec)(const RoadStopSpec *, BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(assign_road_spec);
	void (*deallocate_road_spec)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(deallocate_road_spec);
	uint16_t (*road_callback)(const RoadStopSpec *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_callback);
	bool (*road_boolean)(const RoadStopSpec *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(road_boolean);
	void (*road_triggers)(BaseStation *, const RoadStopSpec *) noexcept OPENTTD_SERVICE_DEFAULT(road_triggers);
	RoadStop *(*new_stop)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_stop);
	void (*set_stop_head)(BaseStation *, bool, RoadStop *) noexcept OPENTTD_SERVICE_DEFAULT(set_stop_head);
	void (*set_stop_next)(RoadStop *, RoadStop *) noexcept OPENTTD_SERVICE_DEFAULT(set_stop_next);
	void (*add_stop_area)(BaseStation *, bool, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(add_stop_area);
	void (*clear_stop_area)(BaseStation *, bool) noexcept OPENTTD_SERVICE_DEFAULT(clear_stop_area);
	void (*before_add_tile)(OpenTTDCommandCost *, BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(before_add_tile);
	void (*make_drive_stop)(uint32_t, uint8_t, uint8_t, uint8_t, uint16_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_drive_stop);
	void (*make_bay_stop)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_bay_stop);
	void (*delete_stop)(RoadStop *) noexcept OPENTTD_SERVICE_DEFAULT(delete_stop);
	void (*set_custom_road_index)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_custom_road_index);
	uint32_t (*custom_road_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(custom_road_index);
	void (*stop_random_bits)(BaseStation *, uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(stop_random_bits);

	void (*remove_stop_tile_data)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(remove_stop_tile_data);
	void (*close_stop_inspect)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(close_stop_inspect);
	OpenTTDStationArea (*road_waypoint_area)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(road_waypoint_area);
	void (*set_road_waypoint_area)(BaseStation *, OpenTTDStationArea) noexcept OPENTTD_SERVICE_DEFAULT(set_road_waypoint_area);
	OpenTTDRoadState *(*road_vehicle_owner)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_vehicle_owner);
	uint32_t (*order_station_location)(OpenTTDVehicle *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(order_station_location);
	const OpenTTDAircraftLeaves *aircraft;
	const AirportSpec *(*airport_spec)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_spec);
	bool (*airport_available)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_available);
	size_t (*airport_layout_count)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_layout_count);
	bool (*airport_in_bounds)(const AirportSpec *, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_in_bounds);
	OpenTTDAirportSize (*airport_size)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_size);
	OpenTTDAirportLayout (*airport_layout)(const AirportSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_layout);
	OpenTTDAirportLayoutTile (*airport_layout_tile)(const AirportSpec *, uint8_t, uint32_t, size_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_layout_tile);
	uint32_t (*iterator_tile)(TileIterator *) noexcept OPENTTD_SERVICE_DEFAULT(iterator_tile);
	void (*iterator_next)(TileIterator *) noexcept OPENTTD_SERVICE_DEFAULT(iterator_next);
	uint8_t (*airport_noise)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_noise);
	bool (*noise_enabled)() noexcept OPENTTD_SERVICE_DEFAULT(noise_enabled);
	uint16_t (*town_max_noise)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_max_noise);
	const OpenTTDTownHandle *(*station_town)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_town);
	uint32_t (*airport_tile)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_tile);
	uint8_t (*airport_type)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_type);
	OpenTTDAirportGeometry (*airport_geometry)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_geometry);
	bool (*airport_airplanes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_airplanes);
	void (*set_airport_details)(BaseStation *, uint8_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_airport_details);
	void (*make_airport)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_airport);
	void (*airport_add)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_add);
	bool (*airport_animated)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_animated);

	void (*airport_infrastructure)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_infrastructure);
	void (*station_view_invalidate)(BaseStation *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(station_view_invalidate);
	void (*town_view_dirty)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_view_dirty);
	OpenTTDAircraftVehicle (*aircraft_vehicle)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_vehicle);
	uint32_t (*hangar_count)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(hangar_count);
	uint32_t (*hangar_tile)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(hangar_tile);
	void (*close_hangar)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(close_hangar);
	void (*close_airport_tile_inspect)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(close_airport_tile_inspect);
	void (*delete_airport_storage)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(delete_airport_storage);
	void (*after_remove_rect)(BaseStation *, OpenTTDStationArea) noexcept OPENTTD_SERVICE_DEFAULT(after_remove_rect);
	void (*clear_airport)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(clear_airport);
	void (*close_airport_inspect)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(close_airport_inspect);
	const OpenTTDShipLeaves *ship;
	const OpenTTDClearServices *clear;
	const OpenTTDWaterRestorationServices *restoration;
	void (*set_docking)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_docking);
	OpenTTDStationArea (*ship_area)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(ship_area);
	void (*set_ship_area)(BaseStation *, OpenTTDStationArea) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_area);
	void (*ship_area_add)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(ship_area_add);
	void (*docking_clear)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(docking_clear);
	void (*docking_add)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(docking_add);
	void (*water_infrastructure)(uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(water_infrastructure);
	void (*make_dock)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_dock);
	BaseStation *(*neutral_for_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(neutral_for_tile);
	void (*oilrig_unavailable)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(oilrig_unavailable);
	void (*associate_industry)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(associate_industry);
	void (*make_oilrig)(uint32_t, uint16_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(make_oilrig);
	void (*set_station_owner)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_station_owner);
	void (*set_airport_kind_rotation)(BaseStation *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_airport_kind_rotation);
	void (*set_facilities)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_facilities);
	void (*set_build_date)(BaseStation *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_build_date);
	void (*with_nearby)(BaseStation *, void *, void (*)(void *, BaseStation *)) noexcept OPENTTD_SERVICE_DEFAULT(with_nearby);
	Industry *(*associated_industry)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(associated_industry);
	BaseStation *(*industry_neutral)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(industry_neutral);
	void (*clear_industry_neutral)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(clear_industry_neutral);
	OpenTTDStationArea (*industry_area)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(industry_area);
	bool (*autoslope_axis)(uint32_t, int32_t, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(autoslope_axis);
	void (*station_list_classes_invalidate)() noexcept OPENTTD_SERVICE_DEFAULT(station_list_classes_invalidate);
	void (*clear_cached_name)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(clear_cached_name);
	void (*clear_name)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(clear_name);
	void (*make_buoy)(uint32_t, uint16_t, WaterClass) noexcept OPENTTD_SERVICE_DEFAULT(make_buoy);
	void (*make_rail_waypoint)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t, RailType) noexcept OPENTTD_SERVICE_DEFAULT(make_rail_waypoint);
	BaseStation *(*new_waypoint)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(new_waypoint);
	Axis (*rail_axis)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(rail_axis);
	const OpenTTDRoadGroundServices *road_ground;
	void (*road_waypoint_area_add)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_waypoint_area_add);
	void (*set_facility_bits)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_facility_bits);
	void (*set_location)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_location);
	void (*set_rail_station_reservation)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(set_rail_station_reservation);
	void (*set_road_waypoint)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(set_road_waypoint);
	void (*set_string_id)(BaseStation *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_string_id);
	uint32_t (*string_id)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(string_id);
	bool (*tile_flat)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(tile_flat);
	bool (*waypoint_available)() noexcept OPENTTD_SERVICE_DEFAULT(waypoint_available);
	void (*waypoint_default_name)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(waypoint_default_name);
	uint8_t (*waypoint_layout_gfx)(OpenTTDWaypointLayout *, bool) noexcept OPENTTD_SERVICE_DEFAULT(waypoint_layout_gfx);
	void (*waypoint_layout_init)(OpenTTDWaypointLayout *, const StationSpec *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(waypoint_layout_init);
	void (*waypoint_view_invalidate)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(waypoint_view_invalidate);

};
const OpenTTDStationCommandServices &GetRustStationCommandServices() noexcept;
extern "C" {
void openttd_rust_station_check_buildable(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, int32_t *, bool, bool);
void openttd_rust_station_road_bridge_ok(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, const RoadStopSpec *, uint8_t, uint8_t);
void openttd_rust_station_build_above(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, int32_t);
uint8_t openttd_rust_station_airport_noise(const OpenTTDStationCommandServices *, uint8_t, uint32_t);
void openttd_rust_station_tiles_changed(const OpenTTDStationCommandServices *, BaseStation *, bool, uint8_t);
void openttd_rust_station_rename(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, const uint8_t *, size_t);
uint16_t openttd_rust_station_move_name(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint32_t);
void openttd_rust_station_open_close_airport(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t);
void openttd_rust_station_build_rail(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint8_t, uint16_t, uint16_t, uint16_t, bool);
void openttd_rust_station_set_rail_flags(const OpenTTDStationCommandServices *, uint32_t, const StationSpec *);
void openttd_rust_station_remove_rail_area(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, bool, bool);
void openttd_rust_station_build_road(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, uint8_t, uint8_t, uint16_t, uint16_t, uint16_t, bool);
void openttd_rust_station_remove_road_area(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, bool);
const OpenTTDTownHandle *openttd_rust_station_airport_nearest(const OpenTTDStationCommandServices *, const AirportSpec *, uint8_t, uint32_t, TileIterator *, uint32_t *);
void openttd_rust_station_update_airport_noise(const OpenTTDStationCommandServices *);
void openttd_rust_station_build_airport(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint16_t, bool);
void openttd_rust_station_build_dock(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint16_t, bool);
void openttd_rust_station_update_docking(const OpenTTDStationCommandServices *, BaseStation *);
void openttd_rust_station_remove_docking_tile(const OpenTTDStationCommandServices *, uint32_t);
void openttd_rust_station_clear_docking_neighbours(const OpenTTDStationCommandServices *, uint32_t);
bool openttd_rust_station_possible_docking(const OpenTTDStationCommandServices *, uint32_t);
void openttd_rust_station_check_docking(const OpenTTDStationCommandServices *, uint32_t);
void openttd_rust_station_build_oilrig(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t);
void openttd_rust_station_delete_oilrig(const OpenTTDStationCommandServices *, uint32_t);
bool openttd_rust_station_in_orders(const OpenTTDStationCommandServices *, uint16_t, bool, uint8_t);
void openttd_rust_station_clear(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint32_t, uint16_t);
void openttd_rust_station_terraform(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, uint16_t, int32_t, uint8_t);
}
#endif /* RUST_STATION_COMMAND_FFI_H */
