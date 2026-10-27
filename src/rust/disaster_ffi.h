/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file disaster_ffi.h Synchronous disaster controllers and typed native services. */
#ifndef OPENTTD_RUST_DISASTER_FFI_H
#define OPENTTD_RUST_DISASTER_FFI_H
#include "services_ffi.h"
#include "vehicle_ffi.h"
#include "aircraft_ffi.h"
#include "road_ffi.h"
#include "call_ffi.h"
struct OpenTTDDisasterState {
	uint32_t image_override;
	uint32_t target;
	uint16_t state;
	uint8_t flags;
};
struct OpenTTDDisasterVehicle {
	OpenTTDVehicle * handle;
	OpenTTDDisasterState * state;
	uint32_t id;
};
struct OpenTTDDisasterRoadTarget {
	OpenTTDVehicle * handle;
	OpenTTDRoadState * state;
	uint32_t id;
};
struct OpenTTDDisasterIndustryTarget {
	void * industry;
	uint16_t id;
};
struct OpenTTDDisasterIndustryLocation {
	uint32_t tile;
	uint16_t town;
};
struct OpenTTDDisasterStationTarget {
	uint32_t next;
	uint32_t tile;
	uint8_t airport_type;
};
struct OpenTTDDisasterServices {
	const OpenTTDSharedServices * shared;
	const OpenTTDVehicleServices * vehicles;
	const OpenTTDOccupancyCommands * occupancy;
	const OpenTTDEnvironmentServices * environment;
	const OpenTTDRailMapServices * rail_map;
	const OpenTTDAircraftLeaves * aircraft;
	const OpenTTDRoadLeaves * road;
	int32_t (OPENTTD_RUST_CALL *age)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(age);
	void (OPENTTD_RUST_CALL *write_age)(OpenTTDVehicle *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_age);
	void (OPENTTD_RUST_CALL *write_sprite)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_sprite);
	uint32_t (OPENTTD_RUST_CALL *first_sprite)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(first_sprite);
	void (OPENTTD_RUST_CALL *write_first_sprite)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_first_sprite);
	void (OPENTTD_RUST_CALL *write_subtype)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_subtype);
	void (OPENTTD_RUST_CALL *write_owner)(OpenTTDVehicle *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_owner);
	void (OPENTTD_RUST_CALL *set_next)(OpenTTDVehicle *, OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(set_next);
	OpenTTDDisasterVehicle (OPENTTD_RUST_CALL *next_part)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(next_part);
	void (OPENTTD_RUST_CALL *update_delta)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(update_delta);
	uint8_t (OPENTTD_RUST_CALL *toward)(OpenTTDVehicle *, int32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(toward);
	OpenTTDDisasterVehicle (OPENTTD_RUST_CALL *create)(int32_t, int32_t, uint8_t, uint8_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(create);
	OpenTTDDisasterVehicle (OPENTTD_RUST_CALL *resolve)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	OpenTTDDisasterVehicle (OPENTTD_RUST_CALL *next_disaster)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_disaster);
	OpenTTDDisasterRoadTarget (OPENTTD_RUST_CALL *next_road)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_road);
	OpenTTDDisasterRoadTarget (OPENTTD_RUST_CALL *road_target)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_target);
	bool (OPENTTD_RUST_CALL *front)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(front);
	uint32_t (OPENTTD_RUST_CALL *road_disaster)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(road_disaster);
	uint8_t (OPENTTD_RUST_CALL *breakdown_counter)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(breakdown_counter);
	void (OPENTTD_RUST_CALL *write_road_disaster)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(write_road_disaster);
	void (OPENTTD_RUST_CALL *write_breakdown)(OpenTTDVehicle *, uint8_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(write_breakdown);
	OpenTTDDisasterVehicle (OPENTTD_RUST_CALL *next_vehicle)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_vehicle);
	bool (OPENTTD_RUST_CALL *ground)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(ground);
	OpenTTDDisasterVehicle (OPENTTD_RUST_CALL *next_train)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_train);
	uint16_t (OPENTTD_RUST_CALL *road_count)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_count);
	bool (OPENTTD_RUST_CALL *human)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(human);
	OpenTTDDisasterStationTarget (OPENTTD_RUST_CALL *next_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_station);
	OpenTTDDisasterIndustryTarget (OPENTTD_RUST_CALL *next_industry)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_industry);
	OpenTTDDisasterIndustryTarget (OPENTTD_RUST_CALL *industry)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry);
	uint32_t (OPENTTD_RUST_CALL *industry_behaviour)(void *) noexcept OPENTTD_SERVICE_DEFAULT(industry_behaviour);
	OpenTTDDisasterIndustryLocation (OPENTTD_RUST_CALL *industry_location)(void *) noexcept OPENTTD_SERVICE_DEFAULT(industry_location);
	uint32_t (OPENTTD_RUST_CALL *industry_tile)(void *) noexcept OPENTTD_SERVICE_DEFAULT(industry_tile);
	uint16_t (OPENTTD_RUST_CALL *industry_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_index);
	bool (OPENTTD_RUST_CALL *industry_belongs)(void *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_belongs);
	void (OPENTTD_RUST_CALL *reset_industry_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_industry_tile);
	bool (OPENTTD_RUST_CALL *airport_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(airport_tile);
	void (OPENTTD_RUST_CALL *airport_blocks)(uint16_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(airport_blocks);
	void (OPENTTD_RUST_CALL *zeppelin_crashed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(zeppelin_crashed);
	void (OPENTTD_RUST_CALL *zeppelin_cleared)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(zeppelin_cleared);
	void (OPENTTD_RUST_CALL *road_crashed)(OpenTTDVehicle *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(road_crashed);
	void (OPENTTD_RUST_CALL *zeppelin_news)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(zeppelin_news);
	void (OPENTTD_RUST_CALL *small_ufo_news)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(small_ufo_news);
	void (OPENTTD_RUST_CALL *airplane_news)(uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(airplane_news);
	void (OPENTTD_RUST_CALL *helicopter_news)(uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(helicopter_news);
	void (OPENTTD_RUST_CALL *big_ufo_news)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(big_ufo_news);
	void (OPENTTD_RUST_CALL *coal_news)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(coal_news);
	void (OPENTTD_RUST_CALL *clear)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (OPENTTD_RUST_CALL *update_signals)() noexcept OPENTTD_SERVICE_DEFAULT(update_signals);
	bool (OPENTTD_RUST_CALL *disasters_enabled)() noexcept OPENTTD_SERVICE_DEFAULT(disasters_enabled);
	bool (OPENTTD_RUST_CALL *can_allocate)(size_t) noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	uint16_t (OPENTTD_RUST_CALL *town_index)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_index);
};
/* Tables are immutable and live through each synchronous entry and reentry.
 * Shell/owner accesses end before callbacks, especially deletion. No exception
 * crosses this boundary; environmental failures terminate in noexcept services. */
extern "C" {
OpenTTDDisasterState *OPENTTD_RUST_CALL openttd_rust_disaster_state_create(uint32_t);
void OPENTTD_RUST_CALL openttd_rust_disaster_state_destroy(OpenTTDDisasterState *);
uint16_t *OPENTTD_RUST_CALL openttd_rust_disaster_delay();
bool OPENTTD_RUST_CALL openttd_rust_disaster_tick(const OpenTTDDisasterServices *, OpenTTDDisasterVehicle);
void OPENTTD_RUST_CALL openttd_rust_disaster_daily(const OpenTTDDisasterServices *);
void OPENTTD_RUST_CALL openttd_rust_disaster_startup(const OpenTTDDisasterServices *);
void OPENTTD_RUST_CALL openttd_rust_disaster_construct(const OpenTTDDisasterServices *, OpenTTDDisasterVehicle, int32_t, int32_t, uint8_t, uint8_t);
void OPENTTD_RUST_CALL openttd_rust_disaster_release_industry(const OpenTTDDisasterServices *, uint16_t);
void OPENTTD_RUST_CALL openttd_rust_disaster_release_vehicle(const OpenTTDDisasterServices *, OpenTTDDisasterVehicle);
}
#endif
