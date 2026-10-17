/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file house_ffi.h Typed native services for the Rust house lifecycle owner. */
#ifndef RUST_HOUSE_FFI_H
#define RUST_HOUSE_FFI_H
#include "town_ffi.h"
#include <cstddef>
#include <cstdint>
/* Lean header: the standalone town protocol fixture includes it without game headers. */
struct Town;
struct Station;
using OpenTTDStation = Station;
struct OpenTTDCommandServices;

/** Narrow observations, acquired only at the phase that uses these fields. */
struct OpenTTDHouseLoopTile { uint8_t age; bool completed, lift_has_destination; };
struct OpenTTDHouseLift { uint8_t position, destination; bool has_destination; };
struct OpenTTDHouseRandom { uint8_t bits, triggers; };
struct OpenTTDHouseConstruction { uint16_t house; uint8_t tick; bool completed; };
struct OpenTTDHouseCache { uint16_t house, town; bool completed; };
struct OpenTTDHouseAge { uint8_t age; bool completed; };
struct OpenTTDHouseEntry { OpenTTDTownRef town; uint16_t house; };
struct OpenTTDHouseLoopSpec { uint16_t callbacks; uint8_t flags, population, mail, minimum_life; };
struct OpenTTDHouseAnimationSpec { uint16_t callbacks; uint8_t extra_flags; };
struct OpenTTDHouseNewSpec { uint16_t callbacks; uint8_t flags, processing_time; };
struct OpenTTDHouseProduction { uint16_t callbacks; uint8_t population, mail; };
struct OpenTTDHouseRemoval { int64_t cost; uint16_t rating; };
struct OpenTTDHouseAcceptance { uint8_t accepts[16], amounts[16]; };
struct OpenTTDHouseReseed { uint32_t reseed; uint8_t used; };
struct OpenTTDHouseCargo { uint8_t cargo; uint16_t multiplier; };
struct OpenTTDHouseErrors { uint32_t demolish, is_protected, refuses, sloped, bridge, flat, no_town; };
/* Synchronous noexcept services; callbacks and commands may reenter Rust,
 * which holds no owner or map reference across any of them. */
struct OpenTTDHouseServices {
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCommandServices *commands;
	const OpenTTDTownAuthorityServices *authority;
	OpenTTDHouseEntry (*entry)(uint32_t) noexcept;
	OpenTTDHouseLoopTile (*loop_tile)(uint32_t) noexcept;
	uint16_t (*house_type)(uint32_t) noexcept;
	bool (*completed)(uint32_t) noexcept;
	bool (*is_protected)(uint32_t) noexcept;
	OpenTTDHouseLift (*lift)(uint32_t) noexcept;
	OpenTTDHouseRandom (*random)(uint32_t) noexcept;
	OpenTTDHouseConstruction (*construction)(uint32_t) noexcept;
	OpenTTDHouseCache (*cache)(uint32_t) noexcept;
	OpenTTDHouseAge (*age)(uint32_t) noexcept;
	uint8_t (*processing)(uint32_t) noexcept;
	uint16_t (*clean_type)(uint32_t) noexcept;
	OpenTTDTownRef (*town)(uint32_t) noexcept;
	OpenTTDHouseLoopSpec (*loop_spec)(uint16_t) noexcept;
	OpenTTDHouseAnimationSpec (*animation_spec)(uint16_t) noexcept;
	OpenTTDHouseNewSpec (*new_spec)(uint16_t) noexcept;
	OpenTTDHouseProduction (*production)(uint16_t) noexcept;
	bool (*sprite_groups)(uint16_t) noexcept;
	OpenTTDHouseRemoval (*removal)(uint16_t) noexcept;
	OpenTTDHouseAcceptance (*acceptance)(uint16_t) noexcept;
	uint64_t (*watched)(uint16_t) noexcept;
	uint16_t (*substitute)(uint16_t) noexcept;
	void (*inc_construction)(uint32_t) noexcept;
	void (*reset_age)(uint32_t) noexcept;
	void (*set_age)(uint32_t, uint8_t) noexcept;
	void (*set_lift_destination)(uint32_t, uint8_t) noexcept;
	void (*set_lift_position)(uint32_t, uint8_t) noexcept;
	void (*halt_lift)(uint32_t) noexcept;
	void (*dec_processing)(uint32_t) noexcept;
	void (*set_processing)(uint32_t, uint8_t) noexcept;
	void (*set_random_bits)(uint32_t, uint8_t) noexcept;
	void (*set_random_triggers)(uint32_t, uint8_t) noexcept;
	void (*set_type)(uint32_t, uint16_t) noexcept;
	void (*animate_new)(uint32_t) noexcept;
	void (*delete_inspect)(uint32_t) noexcept;
	uint16_t (*callback)(uint16_t, uint32_t, uint32_t, uint16_t, Town *, uint32_t, bool) noexcept;
	uint8_t (*cargo_translation)(uint16_t, uint8_t) noexcept;
	bool (*convert_boolean)(uint16_t, uint16_t, uint16_t) noexcept;
	bool (*convert_8bit)(uint16_t, uint16_t, uint16_t) noexcept;
	OpenTTDHouseReseed (*resolve_reseed)(uint16_t, uint32_t, Town *, uint8_t) noexcept;
	void (*change_frame)(uint16_t, uint16_t, Town *, uint32_t, uint32_t, uint32_t, uint64_t) noexcept;
	OpenTTDStation *(*next_near_station)(Town *, OpenTTDStation *) noexcept;
	void (*erase_near_station)(Town *, OpenTTDStation *) noexcept;
	bool (*station_catchment)(const OpenTTDStation *, uint32_t) noexcept;
	bool (*station_covers_town)(const OpenTTDStation *, uint16_t) noexcept;
	uint32_t (*move_goods)(void *, uint8_t, uint32_t, uint16_t) noexcept;
	OpenTTDHouseCargo (*production_cargo)(uint8_t, uint32_t) noexcept;
	void (*add_cargo)(void *, uint8_t, uint32_t) noexcept;
	uint8_t (*food_cargo)() noexcept;
	uint32_t (*scale_cargo)(uint32_t) noexcept;
	uint8_t (*cargogen_mode)() noexcept;
	uint8_t (*place_houses)() noexcept;
	bool (*company_human)(uint8_t) noexcept;
	void (*init_counts)() noexcept;
	void (*rebuild_tree)() noexcept;
	OpenTTDHouseErrors errors;
};
/* Resolver and AnimationBase services live beside their native templates. */
OpenTTDHouseReseed OpenTTDHouseResolveReseed(uint16_t house, uint32_t tile, Town *town, uint8_t waiting) noexcept;
void OpenTTDHouseChangeFrame(uint16_t callback, uint16_t house, Town *town, uint32_t tile, uint32_t random, uint32_t trigger, uint64_t cargoes) noexcept;
void OpenTTDAnimateNewHouse(uint32_t tile) noexcept;

extern "C" {
void openttd_rust_house_tile_loop(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile, void *finder);
void openttd_rust_house_animate(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile);
void openttd_rust_house_clear(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, OpenTTDCommandCost *, uint32_t tile, uint16_t flags);
void openttd_rust_house_terraform(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, OpenTTDCommandCost *, uint32_t tile, uint16_t flags, int32_t z_new, uint8_t slope);
void openttd_rust_house_place(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, OpenTTDCommandCost *, uint16_t flags, uint32_t tile, uint16_t house, bool is_protected, bool replace);
void openttd_rust_house_accepted(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile, uint16_t house, Town *town, void *acceptance, uint64_t *always);
void openttd_rust_house_produced(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile, void *produced);
void openttd_rust_house_watched(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile, uint64_t cargoes);
void openttd_rust_house_rebuild_caches(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
void openttd_rust_house_update_all(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
void openttd_rust_house_age_all(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
}
/* Town leaves/shared services for native house entry points outside town_cmd.cpp. */
const OpenTTDTownLeaves &GetRustTownServices() noexcept;
#endif /* RUST_HOUSE_FFI_H */
