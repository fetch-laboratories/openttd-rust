/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_command_ffi.h Station construction policy boundary (ABI 1060-1119). */
#ifndef RUST_STATION_COMMAND_FFI_H
#define RUST_STATION_COMMAND_FFI_H
#include "services_ffi.h"
#include "command_services_ffi.h"
#include "station_service_ffi.h"
#include "rail_command_ffi.h"
#include "train_reservation_ffi.h"
#include "town_ffi.h"
#include "byte_span_ffi.h"
#include "vehicle_ffi.h"
#include "road_ffi.h"
#include "aircraft_ffi.h"
#include "ship_control_ffi.h"
enum class WaterClass : uint8_t;
struct Industry;
struct StationSpec;
struct RoadStopSpec;
struct BridgeableTileInfo;
struct OpenTTDStationArea { uint32_t tile; uint16_t w, h; };
struct OpenTTDStationBridgeInfo { const BridgeableTileInfo *data; size_t size; };
struct OpenTTDStationBounds { int32_t left, top, right, bottom; };
struct OpenTTDStationSpecIndex { uint8_t value; bool valid; };
struct OpenTTDRailStationSpecInfo { uint8_t platforms, lengths; bool slope_callback, availability_callback; };
struct RoadStop;
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
	uint8_t (*spread)() noexcept;
	OpenTTDStationBridgeInfo (*default_bridge_info)(uint8_t) noexcept;
	OpenTTDStationBridgeInfo (*rail_bridge_info)(const StationSpec *) noexcept;
	OpenTTDStationBridgeInfo (*road_bridge_info)(const RoadStopSpec *) noexcept;
	uint32_t (*southern_bridge_end)(uint32_t) noexcept;
	int32_t (*bridge_height)(uint32_t) noexcept;
	OpenTTDStationArea (*train_area)(const BaseStation *) noexcept;
	const StationSpec *(*rail_spec)(uint32_t) noexcept;
	const RoadStopSpec *(*road_spec)(uint32_t) noexcept;
	const OpenTTDStationLinks *links;
	OpenTTDStationBounds (*bounds)(BaseStation *) noexcept;
	uint32_t (*location)(const BaseStation *) noexcept;
	void (*move_sign)(BaseStation *, uint32_t) noexcept;
	void (*node_location)(OpenTTDLinkGraph *, uint16_t, uint32_t) noexcept;
	void (*virtual_coord)(BaseStation *) noexcept;
	void (*catchment)(BaseStation *, bool) noexcept;
	void (*catchment_dirty)() noexcept;
	void (*facility_widget)(BaseStation *, uint8_t) noexcept;
	void (*select_invalidate)() noexcept;
	void (*radius)(uint32_t, uint32_t, void *, void (*)(void *, BaseStation *)) noexcept;
	bool (*station_available)() noexcept;
	BaseStation *(*new_station)(uint32_t) noexcept;
	void (*index_insert)(BaseStation *) noexcept;
	void (*set_town)(BaseStation *, const OpenTTDTownHandle *) noexcept;
	void (*default_name)(BaseStation *, uint32_t, uint8_t) noexcept;
	void (*have_rating)(BaseStation *, uint8_t) noexcept;
	void (*before_add_rect)(OpenTTDCommandCost *, BaseStation *, OpenTTDStationArea, uint8_t) noexcept;
	bool (*road_waypoint)(BaseStation *) noexcept;
	bool (*name_equals)(BaseStation *, const uint8_t *, size_t) noexcept;
	void (*set_name)(BaseStation *, const uint8_t *, size_t) noexcept;
	bool (*rect_contains)(BaseStation *, uint32_t) noexcept;
	void (*flip_airport_closed)(BaseStation *) noexcept;
	void (*closed_widget)(BaseStation *) noexcept;
	bool (*distant_join)() noexcept;
	uint32_t (*rail_class_count)() noexcept;
	bool (*rail_waypoint_class)(uint16_t) noexcept;
	uint32_t (*rail_spec_count)(uint16_t) noexcept;
	const StationSpec *(*class_rail_spec)(uint16_t, uint16_t) noexcept;
	OpenTTDRailStationSpecInfo (*rail_spec_info)(const StationSpec *) noexcept;
	void (*station_slope_check)(OpenTTDCommandCost *, uint32_t, uint32_t, const StationSpec *, uint8_t, uint8_t, uint8_t) noexcept;
	OpenTTDByteSpan (*rail_layout)(const StationSpec *, uint8_t, uint8_t) noexcept;
	uint16_t (*rail_callback)(uint16_t, uint32_t, uint32_t, const StationSpec *, BaseStation *, uint32_t) noexcept;
	uint32_t (*platform_info)(uint8_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept;
	OpenTTDStationSpecIndex (*allocate_rail_spec)(const StationSpec *, BaseStation *) noexcept;
	void (*assign_rail_spec)(const StationSpec *, BaseStation *, uint8_t) noexcept;
	void (*deallocate_rail_spec)(BaseStation *, uint32_t) noexcept;
	bool (*rail_boolean)(const StationSpec *, uint16_t, uint16_t) noexcept;
	void (*rail_callback_error)(const StationSpec *, uint16_t, uint16_t) noexcept;
	void (*set_train_area)(BaseStation *, OpenTTDStationArea) noexcept;
	void (*add_facility)(BaseStation *, uint8_t, uint32_t) noexcept;
	void (*rail_triggers)(BaseStation *, const StationSpec *) noexcept;
	bool (*reserved)(uint32_t) noexcept;
	Train *(*last_train)(Train *) noexcept;
	OpenTTDTrainHandle (*train_handle)(const Train *) noexcept;
	const OpenTTDTrainReservationLeaves *reservation;
	void (*platform_reservation)(uint32_t, uint8_t, bool) noexcept;
	bool (*compatible_tiles)(uint32_t, uint32_t) noexcept;
	uint32_t (*custom_rail_index)(uint32_t) noexcept;
	void (*make_rail_station)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t, uint8_t) noexcept;
	void (*close_station_inspect)(uint32_t) noexcept;
	void (*set_custom_rail_index)(uint32_t, uint8_t) noexcept;
	void (*set_random_bits)(uint32_t, uint8_t) noexcept;
	void (*set_gfx)(uint32_t, uint8_t) noexcept;
	void (*rail_animation)(BaseStation *, uint32_t) noexcept;
	size_t (*tile_flag_count)(const StationSpec *) noexcept;
	uint8_t (*tile_flags)(const StationSpec *, uint8_t) noexcept;
	void (*set_tile_flags)(uint32_t, bool, bool, bool) noexcept;
	void (*station_infrastructure)(uint8_t, uint32_t) noexcept;
	void (*mark_tiles)(BaseStation *, bool) noexcept;
	void (*after_remove_tile)(BaseStation *, uint32_t) noexcept;
	void (*reset_facilities)(BaseStation *, uint8_t) noexcept;
	void (*orders_dirty)() noexcept;
	const OpenTTDVehicleServices *vehicle;
	bool (*road_type_valid)(uint8_t) noexcept;
	bool (*road_type_is_road)(uint8_t) noexcept;
	bool (*powered_road)(uint8_t, uint8_t) noexcept;
	bool (*stop_on_town_road)() noexcept;
	bool (*stop_on_competitor_road)() noexcept;
	uint32_t (*road_class_count)() noexcept;
	bool (*road_waypoint_class)(uint16_t) noexcept;
	uint32_t (*road_spec_count)(uint16_t) noexcept;
	const RoadStopSpec *(*class_road_spec)(uint16_t, uint16_t) noexcept;
	OpenTTDRoadStationSpecInfo (*road_spec_info)(const RoadStopSpec *) noexcept;
	int64_t (*road_spec_build_cost)(const RoadStopSpec *, uint8_t) noexcept;
	int64_t (*road_spec_clear_cost)(const RoadStopSpec *, uint8_t) noexcept;
	bool (*stops_available)(size_t) noexcept;
	OpenTTDStationSpecIndex (*allocate_road_spec)(const RoadStopSpec *, BaseStation *) noexcept;
	void (*assign_road_spec)(const RoadStopSpec *, BaseStation *, uint8_t) noexcept;
	void (*deallocate_road_spec)(BaseStation *, uint32_t) noexcept;
	uint16_t (*road_callback)(const RoadStopSpec *, uint8_t, uint8_t) noexcept;
	bool (*road_boolean)(const RoadStopSpec *, uint16_t) noexcept;
	void (*road_triggers)(BaseStation *, const RoadStopSpec *) noexcept;
	RoadStop *(*new_stop)(uint32_t) noexcept;
	RoadStop *(*get_stop)(uint32_t, bool) noexcept;
	RoadStop *(*stop_head)(BaseStation *, bool) noexcept;
	void (*set_stop_head)(BaseStation *, bool, RoadStop *) noexcept;
	RoadStop *(*stop_next)(const RoadStop *) noexcept;
	void (*set_stop_next)(RoadStop *, RoadStop *) noexcept;
	uint32_t (*stop_tile)(const RoadStop *) noexcept;
	void (*add_stop_area)(BaseStation *, bool, uint32_t) noexcept;
	void (*clear_stop_area)(BaseStation *, bool) noexcept;
	void (*before_add_tile)(OpenTTDCommandCost *, BaseStation *, uint32_t, uint8_t) noexcept;
	void (*make_drive_stop)(uint32_t, uint8_t, uint8_t, uint8_t, uint16_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept;
	void (*make_bay_stop)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t, uint8_t, uint8_t) noexcept;
	void (*stop_make_drive)(RoadStop *) noexcept;
	void (*stop_clear_drive)(RoadStop *) noexcept;
	void (*delete_stop)(RoadStop *) noexcept;
	void (*set_custom_road_index)(uint32_t, uint8_t) noexcept;
	uint32_t (*custom_road_index)(uint32_t) noexcept;
	void (*stop_random_bits)(BaseStation *, uint32_t, uint8_t) noexcept;
	void (*road_animation)(BaseStation *, uint32_t) noexcept;
	void (*remove_stop_tile_data)(BaseStation *, uint32_t) noexcept;
	void (*close_stop_inspect)(uint32_t) noexcept;
	OpenTTDStationArea (*road_waypoint_area)(BaseStation *) noexcept;
	void (*set_road_waypoint_area)(BaseStation *, OpenTTDStationArea) noexcept;
	void (*vehicles_on_tile)(uint32_t, void *, void (*)(void *, OpenTTDVehicle *)) noexcept;
	OpenTTDRoadState *(*road_vehicle_owner)(OpenTTDVehicle *) noexcept;
	uint32_t (*order_station_location)(OpenTTDVehicle *, uint16_t) noexcept;
	const OpenTTDAircraftLeaves *aircraft;
	const AirportSpec *(*airport_spec)(uint8_t) noexcept;
	bool (*airport_available)(const AirportSpec *) noexcept;
	size_t (*airport_layout_count)(const AirportSpec *) noexcept;
	bool (*airport_in_bounds)(const AirportSpec *, uint8_t, uint32_t) noexcept;
	OpenTTDAirportSize (*airport_size)(const AirportSpec *) noexcept;
	OpenTTDAirportLayout (*airport_layout)(const AirportSpec *, uint8_t) noexcept;
	OpenTTDAirportLayoutTile (*airport_layout_tile)(const AirportSpec *, uint8_t, uint32_t, size_t) noexcept;
	uint32_t (*iterator_tile)(TileIterator *) noexcept;
	void (*iterator_next)(TileIterator *) noexcept;
	const OpenTTDTownHandle *(*calc_closest_town)(uint32_t, uint32_t) noexcept;
	uint8_t (*airport_noise)(const AirportSpec *) noexcept;
	bool (*noise_enabled)() noexcept;
	uint16_t (*town_max_noise)(const OpenTTDTownHandle *) noexcept;
	const OpenTTDTownHandle *(*station_town)(BaseStation *) noexcept;
	uint32_t (*airport_tile)(const BaseStation *) noexcept;
	uint8_t (*airport_type)(const BaseStation *) noexcept;
	OpenTTDAirportGeometry (*airport_geometry)(BaseStation *) noexcept;
	bool (*airport_airplanes)(uint8_t) noexcept;
	void (*set_airport_details)(BaseStation *, uint8_t, uint8_t, uint8_t) noexcept;
	void (*make_airport)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept;
	void (*airport_add)(BaseStation *, uint32_t) noexcept;
	bool (*airport_animated)(uint8_t) noexcept;
	void (*airport_animation)(BaseStation *, uint32_t) noexcept;
	void (*airport_infrastructure)(uint8_t, uint32_t) noexcept;
	void (*station_view_invalidate)(BaseStation *, int32_t) noexcept;
	void (*town_view_dirty)(const OpenTTDTownHandle *) noexcept;
	OpenTTDAircraftVehicle (*aircraft_vehicle)(OpenTTDVehicle *) noexcept;
	uint32_t (*hangar_count)(BaseStation *) noexcept;
	uint32_t (*hangar_tile)(BaseStation *, uint32_t) noexcept;
	void (*close_hangar)(uint32_t) noexcept;
	void (*close_airport_tile_inspect)(uint32_t) noexcept;
	void (*delete_airport_storage)(BaseStation *) noexcept;
	void (*after_remove_rect)(BaseStation *, OpenTTDStationArea) noexcept;
	void (*clear_airport)(BaseStation *) noexcept;
	void (*close_airport_inspect)(BaseStation *) noexcept;
	const OpenTTDShipLeaves *ship;
	WaterClass (*water_class)(uint32_t) noexcept;
	void (*water_restore)(uint32_t, uint8_t) noexcept;
	TrackBits (*water_tracks)(uint32_t, uint8_t) noexcept;
	void (*set_docking)(uint32_t, bool) noexcept;
	OpenTTDStationArea (*ship_area)(BaseStation *) noexcept;
	void (*set_ship_area)(BaseStation *, OpenTTDStationArea) noexcept;
	void (*ship_area_add)(BaseStation *, uint32_t) noexcept;
	void (*docking_clear)(BaseStation *) noexcept;
	void (*docking_add)(BaseStation *, uint32_t) noexcept;
	void (*water_infrastructure)(uint8_t, uint32_t) noexcept;
	void (*make_dock)(uint32_t, uint8_t, uint16_t, uint8_t, uint8_t) noexcept;
	BaseStation *(*neutral_for_tile)(uint32_t) noexcept;
	void (*leave_station)(OpenTTDVehicle *) noexcept;
	void (*oilrig_unavailable)(uint32_t) noexcept;
	void (*associate_industry)(BaseStation *, uint32_t) noexcept;
	void (*make_oilrig)(uint32_t, uint16_t, uint8_t) noexcept;
	void (*set_station_owner)(BaseStation *, uint8_t) noexcept;
	void (*set_airport_kind_rotation)(BaseStation *, uint8_t, uint8_t) noexcept;
	void (*set_facilities)(BaseStation *, uint8_t) noexcept;
	void (*set_build_date)(BaseStation *, int32_t) noexcept;
	void (*with_nearby)(BaseStation *, void *, void (*)(void *, BaseStation *)) noexcept;
	Industry *(*associated_industry)(BaseStation *) noexcept;
	BaseStation *(*industry_neutral)(Industry *) noexcept;
	void (*clear_industry_neutral)(Industry *) noexcept;
	OpenTTDStationArea (*industry_area)(Industry *) noexcept;
	void (*remove_buoy)(OpenTTDCommandCost *, uint32_t, uint16_t) noexcept;
	bool (*autoslope_axis)(uint32_t, int32_t, uint8_t, uint8_t) noexcept;
	void (*set_road_owner)(uint32_t, uint8_t, uint8_t) noexcept;
	void (*station_list_classes_invalidate)() noexcept;
};
const OpenTTDStationCommandServices &GetRustStationCommandServices() noexcept;
extern "C" {
void openttd_rust_station_check_buildable(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, int32_t *, bool, bool);
void openttd_rust_station_rail_bridge_ok(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, const StationSpec *, uint8_t, uint8_t);
void openttd_rust_station_road_bridge_ok(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, const RoadStopSpec *, uint8_t, uint8_t);
void openttd_rust_station_default_bridge_ok(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, uint8_t);
void openttd_rust_station_build_above(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint32_t, int32_t);
OpenTTDStationArea openttd_rust_station_expand_rail(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, const BaseStation *, OpenTTDStationArea);
uint8_t openttd_rust_station_airport_noise(const OpenTTDStationCommandServices *, uint8_t, uint32_t);
void openttd_rust_station_tiles_changed(const OpenTTDStationCommandServices *, BaseStation *, bool, uint8_t);
void openttd_rust_station_rename(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, const uint8_t *, size_t);
uint16_t openttd_rust_station_move_name(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint32_t);
void openttd_rust_station_open_close_airport(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t);
void openttd_rust_station_build_rail(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, uint8_t, uint16_t, uint16_t, uint16_t, bool);
void openttd_rust_station_set_rail_flags(const OpenTTDStationCommandServices *, uint32_t, const StationSpec *);
void openttd_rust_station_remove_rail_area(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint32_t, bool, bool);
void openttd_rust_station_road_cost(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDStationArea, uint16_t, bool, uint8_t, const RoadStopSpec *, uint8_t, uint8_t, uint16_t *, uint8_t, int64_t);
void openttd_rust_station_build_road(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint16_t, uint32_t, uint8_t, uint8_t, uint8_t, bool, uint8_t, uint8_t, uint16_t, uint16_t, uint16_t, bool);
void openttd_rust_station_remove_road_tile(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint32_t, uint16_t, int32_t, bool);
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
void openttd_rust_station_change_owner(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint32_t, uint8_t, uint8_t);
BaseStation *openttd_rust_station_join_waypoint(const OpenTTDStationCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, bool, OpenTTDStationArea, bool);
}
#endif /* RUST_STATION_COMMAND_FFI_H */
