/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town_founding_ffi.h Synchronous founding policy and native exception frame. */
#ifndef RUST_TOWN_FOUNDING_FFI_H
#define RUST_TOWN_FOUNDING_FFI_H
#include "town_name_ffi.h"
#include "../clear_map.h"
#include "../tree_map.h"
#include "../cargotype.h"
#include "../command_type.h"
#include <exception>
#include <optional>
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
	OpenTTDTownRef (*allocate)(uint32_t) noexcept;
	OpenTTDTownRef (*resolve)(uint16_t) noexcept;
	bool (*can_allocate)() noexcept;
	bool (*close_to_town)(uint32_t, uint) noexcept;
	bool (*flat)(uint32_t) noexcept;
	uint (*water_distance)(uint32_t, bool) noexcept;
	uint (*scale_size)(uint) noexcept;
	uint (*scale_land)(uint) noexcept;
	TownFounding (*found_setting)() noexcept;
	TownLayout (*layout_setting)() noexcept;
	uint8_t (*initial_city_size)() noexcept;
	uint8_t (*town_density)() noexcept;
	uint16_t (*custom_number)() noexcept;
	uint8_t (*larger_towns)() noexcept;
	const TownNameParams *(*parameters)(OpenTTDTownFoundingFrame *) noexcept;
	OpenTTDTownNameMetadata (*metadata)(const TownNameParams *) noexcept;
	void (*initialize_counts)(Town *) noexcept;
	void (*insert_town)(Town *) noexcept;
	bool (*cargo_effect_exists)(TownAcceptanceEffect) noexcept;
	void (*prepare_rename)(Town *) noexcept;
	void (*name_changed)(Town *, OpenTTDByteSpan) noexcept;
	void (*update_virt)(Town *) noexcept;
	void (*directory_rebuild)() noexcept;
	void (*directory_resort)() noexcept;
	void (*update_noise)() noexcept;
	void (*rebuild_tree)() noexcept;
	void (*delete_town)(uint16_t) noexcept;
	void (*assert_random_context)() noexcept;
	void (*assert_allocation)() noexcept;
	void (*initialize_cost)(OpenTTDCommandCost *, int) noexcept;
	bool (*sufficient_money)(const OpenTTDCommandCost *) noexcept;
	void (*money_deficit)(OpenTTDTownFoundingFrame *, const OpenTTDCommandCost *) noexcept;
	void (*set_result)(OpenTTDTownFoundingFrame *, uint16_t) noexcept;
	bool (*exchange_generating)(bool) noexcept;
	void (*nearest_town_roads)(bool) noexcept;
	void (*founded_news)(uint16_t, uint32_t) noexcept;
	void (*founded_events)(uint16_t) noexcept;
	void (*clear_station_names)() noexcept;
	void (*clear_industry_names)() noexcept;
	void (*update_station_coords)() noexcept;
	OpenTTDByteSpan (*cached_name)(const Town *) noexcept;
	bool (*set_progress)(OpenTTDTownFoundingFrame *, uint) noexcept;
	bool (*increase_progress)(OpenTTDTownFoundingFrame *) noexcept;
	void (*no_town_error)() noexcept;
	size_t (*town_count)() noexcept;
	Randomizer *random;
	StringID edge_error, close_error, unsuitable_error, unique_error, too_many_error, no_space_error;
};
const OpenTTDTownFoundingServices &GetRustTownFoundingServices() noexcept;
extern "C" {
void openttd_rust_town_found(const OpenTTDTownFoundingServices *, OpenTTDTownFoundingFrame *, OpenTTDTownNameFrame *, OpenTTDCommandCost *, uint32_t, TownSize, bool, TownLayout, bool, uint32_t, OpenTTDByteSpan, bool);
uint openttd_rust_town_default_number(const OpenTTDTownFoundingServices *);
bool openttd_rust_town_generate(const OpenTTDTownFoundingServices *, OpenTTDTownFoundingFrame *, OpenTTDTownNameFrame *, TownLayout, bool, uint, bool *);
void openttd_rust_town_rename(const OpenTTDTownFoundingServices *, OpenTTDCommandCost *, uint16_t, OpenTTDByteSpan, bool);
}
#endif /* RUST_TOWN_FOUNDING_FFI_H */
