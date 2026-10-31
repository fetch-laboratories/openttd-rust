/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file town_command_ffi.h Town command and destruction services. */
#ifndef RUST_TOWN_COMMAND_FFI_H
#define RUST_TOWN_COMMAND_FFI_H
#include "town_founding_ffi.h"
#include "cargo_storage_ffi.h"
#include "../object_type.h"
struct OpenTTDObjectServices;
struct OpenTTDSubsidyServices;
struct OpenTTDRailMapServices;
struct BaseStation;
struct Object;
struct OpenTTDTownPoolRelation { uint32_t next; const Town *town; };
struct OpenTTDTownStationCursor { BaseStation *shell; uint32_t next; };
/** All entries are synchronous/noexcept. Shells are borrowed only until deletion;
 * no owner borrow crosses commands, storage destruction, news or event enqueue.
 * post_destruct runs after pool removal and never reacquires the old town ID.
 * Save/load decoding stays native and commits partially decoded encoded text. */
struct OpenTTDTownCommandServices {
	const OpenTTDTownFoundingServices *founding;
	const OpenTTDRailMapServices *rail;
	const OpenTTDCargoStorageServices *cargo;
	const OpenTTDObjectServices *objects;
	const OpenTTDSubsidyServices *subsidies;
	bool (*fund_roads)() noexcept OPENTTD_SERVICE_DEFAULT(fund_roads);
	bool (*fund_buildings)() noexcept OPENTTD_SERVICE_DEFAULT(fund_buildings);
	bool (*exclusive_rights)() noexcept OPENTTD_SERVICE_DEFAULT(exclusive_rights);
	bool (*bribe)() noexcept OPENTTD_SERVICE_DEFAULT(bribe);
	void (*invalidate_view)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_view);
	void (*invalidate_authority)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(invalidate_authority);
	void (*modify_station_rating)(uint32_t, uint8_t, int, uint) noexcept OPENTTD_SERVICE_DEFAULT(modify_station_rating);
	void (*road_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(road_news);
	void (*road_events)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(road_events);
	void (*rights_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(rights_news);
	void (*rights_events)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(rights_events);
	void (*station_views_dirty)() noexcept OPENTTD_SERVICE_DEFAULT(station_views_dirty);
	OpenTTDTownStationCursor (*next_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_station);
	const OpenTTDTownHandle *(*station_town)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_town);
	uint8_t (*station_owner)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_owner);
	bool (*station_airport)(const Station *) noexcept OPENTTD_SERVICE_DEFAULT(station_airport);
	uint8_t (*airport_type)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_type);
	uint32_t (*airport_tile)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(airport_tile);
	const OpenTTDStationService *(*station_activity_owner)(const OpenTTDStation *) noexcept OPENTTD_SERVICE_DEFAULT(station_activity_owner);
	bool (*local_company)() noexcept OPENTTD_SERVICE_DEFAULT(local_company);
	void (*bribe_error)() noexcept OPENTTD_SERVICE_DEFAULT(bribe_error);
	OpenTTDTownPoolRelation (*next_waypoint)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_waypoint);
	OpenTTDTownPoolRelation (*next_depot)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_depot);
	OpenTTDTownPoolRelation (*next_industry)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_industry);
	void (*set_object_town)(Object *, Town *) noexcept OPENTTD_SERVICE_DEFAULT(set_object_town);
	Town *(*industry_town_by_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_town_by_tile);
	void (*remove_town_sign)(Town *) noexcept OPENTTD_SERVICE_DEFAULT(remove_town_sign);
	void (*delete_town)(Town *) noexcept OPENTTD_SERVICE_DEFAULT(delete_town);
	bool (*cleaning_pool)() noexcept OPENTTD_SERVICE_DEFAULT(cleaning_pool);
	void (*close_view)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(close_view);
	void (*close_graph)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(close_graph);
	size_t (*storage_count)(Town *) noexcept OPENTTD_SERVICE_DEFAULT(storage_count);
	void (*delete_storage)(Town *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_storage);
	void (*clear_storage)(Town *) noexcept OPENTTD_SERVICE_DEFAULT(clear_storage);
	void (*delete_inspect)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_inspect);
	bool assertions;
	uint32_t too_many_objects, no_statue_place;
};
const OpenTTDTownCommandServices &GetRustTownCommandServices() noexcept;
extern "C" {
void openttd_rust_town_cargo_goal(const OpenTTDTownCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, TownAcceptanceEffect, uint32_t);
void openttd_rust_town_text_command(const OpenTTDTownCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, OpenTTDByteSpan);
void openttd_rust_town_growth_command(const OpenTTDTownCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint16_t);
void openttd_rust_town_rating_command(const OpenTTDTownCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint8_t, int16_t);
void openttd_rust_town_expand_command(const OpenTTDTownCommandServices *, const OpenTTDRoadCommandContext *, OpenTTDCommandCost *, uint16_t, uint16_t, uint32_t, uint8_t);
void openttd_rust_town_delete_command(const OpenTTDTownCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t);
uint8_t openttd_rust_town_action_cost(uint8_t);
uint8_t openttd_rust_town_action_mask(const OpenTTDTownCommandServices *, uint8_t, OpenTTDTownRef);
void openttd_rust_town_action(const OpenTTDTownCommandServices *, OpenTTDCommandCost *, uint16_t, uint16_t, uint8_t);
void openttd_rust_town_destruct(const OpenTTDTownCommandServices *, OpenTTDTownRef);
void openttd_rust_town_post_destruct(const OpenTTDTownCommandServices *);
}
#endif /* RUST_TOWN_COMMAND_FFI_H */
