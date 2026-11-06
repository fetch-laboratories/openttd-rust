/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file object_ffi.h Canonical object counts and command scratch. */
#ifndef RUST_OBJECT_FFI_H
#define RUST_OBJECT_FFI_H
#include <cstddef>
#include <cstdint>
#include <array>
struct OpenTTDObjectHiddenArea { size_t index; uint32_t first_tile; };
#include "services_ffi.h"
#include "object_resolver_ffi.h"
#include "command_services_ffi.h"
#include "company_ffi.h"
#include "town_ffi.h"
#include "cargo_storage_ffi.h"
#include "water_regions_ffi.h"
struct OpenTTDGroundCall;
struct OpenTTDTownFoundingServices;
struct OpenTTDIndustryTileServices;
struct OpenTTDWaterRestorationServices;
struct ObjectSpec;
struct Object;
struct OpenTTDObjectRecord;
struct Town;
enum class WaterClass : uint8_t;
enum class ObjectAnimationTrigger : uint8_t;
struct OpenTTDObjectGenerationFrame;
struct OpenTTDObjectArea { uint32_t tile; uint16_t width, height; };
struct OpenTTDObjectSlopeResult { uint16_t callback; std::array<int32_t, 16> registers; uint8_t version; };
struct OpenTTDObjectStrings {
	uint32_t too_many, no_town, edge_sub, water, edge, on_water, bridge, flat;
	uint32_t own, built, limit, hq, object;
	uint32_t transmitter, lighthouse, statue, land, headquarters;
};
enum class HouseZone : uint8_t;
struct ResolverObject;
struct OpenTTDObjectQueryServices {
	uint8_t (*random_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	uint32_t (*index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(index);
	uint32_t (*town_tile)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_tile);
	bool (*version8)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(version8);
	uint32_t (*manhattan)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(manhattan);
	uint32_t (*square)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(square);

};
const OpenTTDObjectQueryServices &GetRustObjectQueryServices() noexcept;
struct OpenTTDObjectServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandCostServices *cost;
	const OpenTTDCommandServices *command;
	const OpenTTDCompanyServices *company;
	const OpenTTDCargoStorageServices *cargo;
	const OpenTTDTownLeaves *town;
	const OpenTTDTownFoundingServices *founding;
	const OpenTTDIndustryTileServices *industry_tiles;
	OpenTTDObjectStrings strings;
	OpenTTDObjectRecord *(*allocate)() noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	uint8_t (*recolour)(uint8_t owner, bool two) noexcept OPENTTD_SERVICE_DEFAULT(recolour);
	void (*colour_error)(const ObjectSpec *spec, uint16_t result) noexcept OPENTTD_SERVICE_DEFAULT(colour_error);
	void (*make)(uint32_t tile, uint8_t owner, uint32_t id, WaterClass water, uint32_t random) noexcept OPENTTD_SERVICE_DEFAULT(make);
	void (*delete_object)(OpenTTDObjectRecord *o) noexcept OPENTTD_SERVICE_DEFAULT(delete_object);
	const OpenTTDObjectQueryServices *query;
	const OpenTTDObjectResolverServices *resolver;
	int32_t (*price_shift)(const GRFFile *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(price_shift);
	void (*slope_error)(OpenTTDCommandCost *out, const OpenTTDObjectSlopeResult *result, const ObjectSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(slope_error);
	void (*set_hq)(uint8_t owner, uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(set_hq);
	void (*close_inspect)(uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(close_inspect);
	bool (*progress_set)(OpenTTDObjectGenerationFrame *frame, uint32_t total) noexcept OPENTTD_SERVICE_DEFAULT(progress_set);
	bool (*progress_increase)(OpenTTDObjectGenerationFrame *frame) noexcept OPENTTD_SERVICE_DEFAULT(progress_increase);
	bool (*normal)() noexcept OPENTTD_SERVICE_DEFAULT(normal);
	bool (*bulldozer)() noexcept OPENTTD_SERVICE_DEFAULT(bulldozer);
	const OpenTTDWaterRestorationServices *restoration;
	void (*set_owner)(uint32_t tile, uint8_t owner) noexcept OPENTTD_SERVICE_DEFAULT(set_owner);
	uint32_t (*scale_size_1d)(uint32_t n) noexcept OPENTTD_SERVICE_DEFAULT(scale_size_1d);
	uint32_t (*scale_cargo)(uint32_t amount) noexcept OPENTTD_SERVICE_DEFAULT(scale_cargo);
};
/* All calls are serialized by the game-state mutex, including across the GUI and
 * game threads. Indices survive append. No element pointer or allocation escapes;
 * callbacks may reenter only after every Rust state borrow has ended. */
extern "C" {
uint32_t openttd_rust_object_random_bits(const OpenTTDObjectServices *, uint32_t);
uint32_t openttd_rust_object_variable(const OpenTTDObjectServices *, const ResolverObject *, OpenTTDObjectRecord *, const ObjectSpec *, uint32_t, uint8_t, uint8_t, uint32_t, bool *);
uint8_t openttd_rust_object_trigger_tile_abi(const OpenTTDObjectServices *, OpenTTDObjectRecord *, uint32_t, ObjectAnimationTrigger, const ObjectSpec *);
extern "C++" inline bool openttd_rust_object_trigger_tile(const OpenTTDObjectServices *a0, OpenTTDObjectRecord *a1, uint32_t a2, ObjectAnimationTrigger a3, const ObjectSpec *a4) { return openttd_rust_object_trigger_tile_abi(a0, a1, a2, a3, a4) != 0; }
uint8_t openttd_rust_object_trigger_abi(const OpenTTDObjectServices *, OpenTTDObjectRecord *, ObjectAnimationTrigger, const ObjectSpec *);
extern "C++" inline bool openttd_rust_object_trigger(const OpenTTDObjectServices *a0, OpenTTDObjectRecord *a1, ObjectAnimationTrigger a2, const ObjectSpec *a3) { return openttd_rust_object_trigger_abi(a0, a1, a2, a3) != 0; }
void openttd_rust_object_counts_reset();
void openttd_rust_object_count_increment(uint16_t);
void openttd_rust_object_count_decrement(uint16_t);
uint16_t openttd_rust_object_count(uint16_t);
uint8_t openttd_rust_object_indirectly_cleared_abi(uint32_t, uint32_t);
extern "C++" inline bool openttd_rust_object_indirectly_cleared(uint32_t a0, uint32_t a1) { return openttd_rust_object_indirectly_cleared_abi(a0, a1) != 0; }
OpenTTDObjectHiddenArea openttd_rust_object_tunnel_hide(uint32_t, uint32_t);
void openttd_rust_object_restore_first(OpenTTDObjectHiddenArea);
void openttd_rust_object_build(const OpenTTDObjectServices * services, uint16_t kind, uint32_t tile, uint8_t owner, const Town * town, uint8_t view);
void openttd_rust_object_update_hq(const OpenTTDObjectServices * services, uint32_t tile, uint32_t score);
void openttd_rust_object_colours(const OpenTTDObjectServices * services, uint8_t company);
void openttd_rust_object_build_command(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, uint16_t flags, uint32_t tile, uint16_t kind, uint8_t view);
void openttd_rust_object_build_area(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * ret, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, OpenTTDCommandCost * last, uint16_t flags, uint32_t tile, uint32_t start, uint16_t kind, uint8_t view, bool diagonal);
void openttd_rust_object_cargo(const OpenTTDObjectServices * services, uint32_t tile, uint32_t * cargo, uint64_t * accepted);
void openttd_rust_object_tile_loop(const OpenTTDObjectServices * services, OpenTTDGroundCall * call, uint32_t tile);
uint8_t openttd_test_object_generate(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, OpenTTDObjectGenerationFrame * frame);
void openttd_rust_object_terraform(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, uint32_t tile, uint16_t flags, int32_t z, Slope slope);
void openttd_rust_object_build_above(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, uint32_t tile, uint16_t flags, int32_t height);
}
#endif /* RUST_OBJECT_FFI_H */
