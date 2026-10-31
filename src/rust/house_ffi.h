/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file house_ffi.h Typed native services for the Rust house lifecycle owner. */
#ifndef RUST_HOUSE_FFI_H
#define RUST_HOUSE_FFI_H
struct GRFFile;
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
struct OpenTTDHouseRemoval { uint8_t factor; uint16_t rating; };
struct OpenTTDHouseAcceptance { uint8_t accepts[16], amounts[16]; };
struct OpenTTDHouseReseed { uint32_t reseed; uint8_t used; };
struct OpenTTDHouseErrors { uint32_t demolish, is_protected, refuses, sloped, bridge, flat, no_town; };
/* Synchronous noexcept services; callbacks and commands may reenter Rust,
 * which holds no owner or map reference across any of them. */
struct OpenTTDHouseServices {
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCommandServices *commands;
	const OpenTTDTownAuthorityServices *authority;
	OpenTTDHouseEntry (*entry)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(entry);
	OpenTTDHouseLoopTile (*loop_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(loop_tile);
	uint16_t (*house_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(house_type);
	bool (*completed)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(completed);
	bool (*is_protected)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(is_protected);
	OpenTTDHouseLift (*lift)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(lift);
	OpenTTDHouseRandom (*random)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random);
	OpenTTDHouseConstruction (*construction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(construction);
	OpenTTDHouseCache (*cache)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(cache);
	OpenTTDHouseAge (*age)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(age);
	uint8_t (*processing)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(processing);
	uint16_t (*clean_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(clean_type);
	OpenTTDTownRef (*town)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(town);
	OpenTTDHouseLoopSpec (*loop_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(loop_spec);
	OpenTTDHouseAnimationSpec (*animation_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(animation_spec);
	OpenTTDHouseNewSpec (*new_spec)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(new_spec);
	OpenTTDHouseProduction (*production)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(production);
	bool (*sprite_groups)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(sprite_groups);
	OpenTTDHouseRemoval (*removal)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(removal);
	OpenTTDHouseAcceptance (*acceptance)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(acceptance);
	uint64_t (*watched)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(watched);
	void (*inc_construction)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(inc_construction);
	void (*reset_age)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(reset_age);
	void (*set_age)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_age);
	void (*set_lift_destination)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_lift_destination);
	void (*set_lift_position)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_lift_position);
	void (*halt_lift)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(halt_lift);
	void (*dec_processing)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(dec_processing);
	void (*set_processing)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_processing);
	void (*set_random_bits)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_random_bits);
	void (*set_random_triggers)(uint32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_random_triggers);
	void (*set_type)(uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_type);

	void (*delete_inspect)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_inspect);
	const GRFFile *(*cargo_file)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_file);
	bool (*convert_boolean)(uint16_t, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(convert_boolean);
	bool (*convert_8bit)(uint16_t, uint16_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(convert_8bit);

	uint32_t (*scale_cargo)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(scale_cargo);
	uint8_t (*cargogen_mode)() noexcept OPENTTD_SERVICE_DEFAULT(cargogen_mode);
	uint8_t (*place_houses)() noexcept OPENTTD_SERVICE_DEFAULT(place_houses);
	bool (*company_human)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(company_human);
	OpenTTDHouseErrors errors;
};
/* Resolver and AnimationBase services live beside their native templates. */

extern "C" {
void openttd_rust_house_accepted_tile(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t, uint32_t *, uint64_t *);
void openttd_rust_house_accepted_prototype(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint16_t, uint32_t *);
void openttd_rust_house_tile_loop(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile);
void openttd_rust_house_animate(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile);
void openttd_rust_house_place(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, OpenTTDCommandCost *, uint16_t flags, uint32_t tile, uint16_t house, bool is_protected, bool replace);
void openttd_rust_house_accepted(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile, uint16_t house, Town *town, uint32_t *acceptance, uint64_t *always);
void openttd_rust_house_produced(const OpenTTDTownLeaves *, const OpenTTDSharedServices *, uint32_t tile, uint32_t *produced);
void openttd_rust_house_rebuild_caches(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
void openttd_rust_house_update_all(const OpenTTDTownLeaves *, const OpenTTDSharedServices *);
}
#endif /* RUST_HOUSE_FFI_H */
