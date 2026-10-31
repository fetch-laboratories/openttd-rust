/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_spatial_fixture.hpp Fixture-only calls into the selected Rust owners. */
#ifndef STATION_SPATIAL_FIXTURE_HPP
#define STATION_SPATIAL_FIXTURE_HPP
#include "../rust/services_ffi.h"
#include "../rust/station_command_ffi.h"
#include "../rust/town_ffi.h"
#include "../rust/road_stop_ffi.h"
extern "C" {
void openttd_test_station_tree_sequence(const OpenTTDSharedServices *, void *, void (*)(void *, uint32_t, uint16_t));
void openttd_test_station_insert(const OpenTTDSharedServices *, BaseStation *);
void openttd_test_station_move_sign(const OpenTTDSharedServices *, BaseStation *, uint32_t);
void openttd_test_station_radius(const OpenTTDSharedServices *, uint32_t, uint32_t, void *, void (*)(void *, BaseStation *));
void openttd_test_station_rating_around(const OpenTTDSharedServices *, uint32_t, uint8_t, int32_t, uint32_t);
void openttd_test_station_modify_rating(OpenTTDStationService *, int32_t);
RoadStop *openttd_test_station_primary_stop(const OpenTTDSharedServices *, BaseStation *, OpenTTDVehicle *);
RoadStop *openttd_test_station_next_stop(const OpenTTDSharedServices *, const RoadStop *, OpenTTDVehicle *);
OpenTTDStationArea openttd_test_station_rail_area(const OpenTTDSharedServices *, BaseStation *, uint32_t, uint8_t);
uint32_t openttd_test_town_stations(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t);
}
#endif
