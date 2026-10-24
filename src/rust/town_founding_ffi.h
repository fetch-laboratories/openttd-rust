/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_founding_ffi.h Synchronous founding policy and native exception frame. */
#ifndef RUST_TOWN_FOUNDING_FFI_H
#define RUST_TOWN_FOUNDING_FFI_H
#include "service_defaults.hpp"
#include "command_services_ffi.h"
#include "town_name_ffi.h"
#include "../clear_map.h"
#include "../tree_map.h"
#include "../cargotype.h"
#include "../command_type.h"
#include <exception>
#include <optional>
struct OpenTTDRoadCommandContext;
struct OpenTTDTownFoundingFrame {
	OpenTTDTownNameFrame names;
	std::optional<TownNameParams> parameters;
	std::exception_ptr cancellation;
	Money extra_money = 0;
	TownID town_id = TownID::Invalid();
};
struct OpenTTDTownNameMetadata { uint32_t grfid; uint16_t type; };
/** Only set_progress/increase_progress can signal ordinary cancellation. Each
 * catches AbortGenerateWorldSignal in the native frame; Rust drops scratch and
 * returns before C++ rethrows. All other services are direct noexcept calls.
 * World reentry may mutate any owner; Rust retains no owner borrow across it. */
/** Strong native IDs cross only as their documented wire representations:
 * TileIndex uint32_t, TownID uint16_t, CompanyID uint8_t. Native wrappers
 * explicitly construct IDs and expose .base(); enums retain their native type. */
struct OpenTTDTownFoundingServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDTownLeaves *growth;
	const OpenTTDTownNameServices *names;
	const OpenTTDCommandCostServices *cost;
	OpenTTDTownRef (*allocate)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	OpenTTDTownRef (*resolve)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(resolve);
	bool (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	bool (*close_to_town)(uint32_t, uint) noexcept OPENTTD_SERVICE_DEFAULT(close_to_town);
	bool (*flat)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(flat);
	uint (*water_distance)(uint32_t, bool) noexcept OPENTTD_SERVICE_DEFAULT(water_distance);
	uint (*scale_size)(uint) noexcept OPENTTD_SERVICE_DEFAULT(scale_size);
	uint (*scale_land)(uint) noexcept OPENTTD_SERVICE_DEFAULT(scale_land);
	TownFounding (*found_setting)() noexcept OPENTTD_SERVICE_DEFAULT(found_setting);
	TownLayout (*layout_setting)() noexcept OPENTTD_SERVICE_DEFAULT(layout_setting);
	uint8_t (*initial_city_size)() noexcept OPENTTD_SERVICE_DEFAULT(initial_city_size);
	uint8_t (*town_density)() noexcept OPENTTD_SERVICE_DEFAULT(town_density);
	uint16_t (*custom_number)() noexcept OPENTTD_SERVICE_DEFAULT(custom_number);
	uint8_t (*larger_towns)() noexcept OPENTTD_SERVICE_DEFAULT(larger_towns);
	const TownNameParams *(*parameters)(OpenTTDTownFoundingFrame *) noexcept OPENTTD_SERVICE_DEFAULT(parameters);
	OpenTTDTownNameMetadata (*metadata)(const TownNameParams *) noexcept OPENTTD_SERVICE_DEFAULT(metadata);
	void (*insert_town)(Town *) noexcept OPENTTD_SERVICE_DEFAULT(insert_town);
	bool (*cargo_effect_exists)(TownAcceptanceEffect) noexcept OPENTTD_SERVICE_DEFAULT(cargo_effect_exists);
	void (*prepare_rename)(Town *) noexcept OPENTTD_SERVICE_DEFAULT(prepare_rename);
	void (*name_changed)(Town *, OpenTTDByteSpan) noexcept OPENTTD_SERVICE_DEFAULT(name_changed);
	void (*update_virt)(Town *) noexcept OPENTTD_SERVICE_DEFAULT(update_virt);
	void (*directory_rebuild)() noexcept OPENTTD_SERVICE_DEFAULT(directory_rebuild);
	void (*directory_resort)() noexcept OPENTTD_SERVICE_DEFAULT(directory_resort);
	void (*update_noise)() noexcept OPENTTD_SERVICE_DEFAULT(update_noise);
	void (*delete_town)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_town);
	void (*assert_random_context)() noexcept OPENTTD_SERVICE_DEFAULT(assert_random_context);
	void (*assert_allocation)() noexcept OPENTTD_SERVICE_DEFAULT(assert_allocation);
	void (*initialize_cost)(OpenTTDCommandCost *, int) noexcept OPENTTD_SERVICE_DEFAULT(initialize_cost);
	const OpenTTDCommandServices *command;
	void (*money_deficit)(OpenTTDTownFoundingFrame *, const OpenTTDCommandCost *) noexcept OPENTTD_SERVICE_DEFAULT(money_deficit);
	void (*set_result)(OpenTTDTownFoundingFrame *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_result);
	bool (*exchange_generating)(bool) noexcept OPENTTD_SERVICE_DEFAULT(exchange_generating);
	void (*founded_news)(uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(founded_news);
	void (*founded_events)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(founded_events);
	void (*clear_station_names)() noexcept OPENTTD_SERVICE_DEFAULT(clear_station_names);
	void (*clear_industry_names)() noexcept OPENTTD_SERVICE_DEFAULT(clear_industry_names);
	void (*update_station_coords)() noexcept OPENTTD_SERVICE_DEFAULT(update_station_coords);
	OpenTTDByteSpan (*cached_name)(const Town *) noexcept OPENTTD_SERVICE_DEFAULT(cached_name);
	bool (*set_progress)(OpenTTDTownFoundingFrame *, uint) noexcept OPENTTD_SERVICE_DEFAULT(set_progress);
	bool (*increase_progress)(OpenTTDTownFoundingFrame *) noexcept OPENTTD_SERVICE_DEFAULT(increase_progress);
	void (*no_town_error)() noexcept OPENTTD_SERVICE_DEFAULT(no_town_error);
	Randomizer *random;
	StringID edge_error, close_error, unsuitable_error, unique_error, too_many_error, no_space_error;
};
const OpenTTDTownFoundingServices &GetRustTownFoundingServices() noexcept;
extern "C" {
void openttd_rust_town_found(const OpenTTDTownFoundingServices *, const OpenTTDRoadCommandContext *, OpenTTDTownFoundingFrame *, OpenTTDTownNameFrame *, OpenTTDCommandCost *, uint32_t, TownSize, bool, TownLayout, bool, uint32_t, OpenTTDByteSpan, bool);
uint openttd_rust_town_default_number(const OpenTTDTownFoundingServices *);
bool openttd_rust_town_generate(const OpenTTDTownFoundingServices *, const OpenTTDRoadCommandContext *, OpenTTDTownFoundingFrame *, OpenTTDTownNameFrame *, TownLayout, bool, uint, bool *);
void openttd_rust_town_rename(const OpenTTDTownFoundingServices *, OpenTTDCommandCost *, uint16_t, OpenTTDByteSpan, bool);
}
#endif /* RUST_TOWN_FOUNDING_FFI_H */
