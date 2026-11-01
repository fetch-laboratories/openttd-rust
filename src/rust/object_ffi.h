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
struct OpenTTDObjectPrice { uint32_t factor; int32_t shift; };
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
struct Town;
enum class WaterClass : uint8_t;
enum class ObjectAnimationTrigger : uint8_t;
struct OpenTTDObjectGenerationFrame;
struct OpenTTDObjectArea { uint32_t tile; uint16_t width, height; };
struct OpenTTDObjectIdentity { Object *object; OpenTTDObjectArea area; uint16_t kind; };
struct OpenTTDObjectAllocation { Object *object; uint32_t id; };
struct OpenTTDObjectNext { Object *object; uint32_t id, tile; };
struct OpenTTDObjectLoopPolicy { const ObjectSpec *spec; Object *object; uint32_t north; uint16_t flags, kind; };
struct OpenTTDObjectPolicy { uint16_t flags; uint8_t callbacks; };
struct OpenTTDObjectFootprint { uint16_t flags; uint8_t callbacks, size; };
struct OpenTTDObjectClearance { uint16_t flags; uint8_t height; };
struct OpenTTDObjectAdmission { uint8_t views, height; };
struct OpenTTDObjectAreaShape { uint8_t views, size; };
struct OpenTTDObjectGeneration { uint16_t flags; uint8_t amount, views; };
struct OpenTTDObjectSlopeResult { uint16_t callback; std::array<int32_t, 16> registers; uint8_t version; };
struct OpenTTDObjectStrings {
	uint32_t too_many, no_town, edge_sub, water, edge, on_water, bridge, flat;
	uint32_t own, built, limit, hq, object;
};
enum class HouseZone : uint8_t;
struct ResolverObject;
struct OpenTTDObjectDefinition { uint32_t grfid; uint16_t local_id; uint8_t view; bool has_file; };
struct OpenTTDObjectQueryServices {
	uint8_t (*random_bits)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(random_bits);
	uint32_t (*index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(index);
	Object *(*by_tile)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(by_tile);
	OpenTTDObjectDefinition (*definition)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(definition);
	uint32_t (*object_index)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(object_index);
	uint16_t (*kind)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(kind);
	uint32_t (*north)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(north);
	const Town *(*town)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(town);
	uint32_t (*town_tile)(const OpenTTDTownHandle *) noexcept OPENTTD_SERVICE_DEFAULT(town_tile);
	int32_t (*date)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(date);
	uint8_t (*colour)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(colour);
	uint8_t (*view)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(view);
	bool (*version8)(const ResolverObject *) noexcept OPENTTD_SERVICE_DEFAULT(version8);
	uint16_t (*override_id)(uint16_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(override_id);
	uint32_t (*manhattan)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(manhattan);
	uint32_t (*square)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(square);
	uint16_t (*triggers)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(triggers);

	void (*unhandled)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(unhandled);
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
	size_t (*spec_count)() noexcept OPENTTD_SERVICE_DEFAULT(spec_count);
	const ObjectSpec * (*spec)(uint16_t type) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	uint8_t (*callbacks)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(callbacks);
	OpenTTDObjectPolicy (*colour_policy)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(colour_policy);
	uint16_t (*flags)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(flags);
	OpenTTDObjectFootprint (*footprint)(const ObjectSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(footprint);
	OpenTTDObjectClearance (*clearance)(uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(clearance);
	OpenTTDObjectAreaShape (*area_shape)(const ObjectSpec *) noexcept OPENTTD_SERVICE_DEFAULT(area_shape);
	OpenTTDObjectAdmission (*admission)(const ObjectSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(admission);
	OpenTTDObjectGeneration (*generation)(const ObjectSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(generation);
	bool (*can_allocate)() noexcept OPENTTD_SERVICE_DEFAULT(can_allocate);
	OpenTTDObjectAllocation (*allocate)(uint16_t type, const Town *town, OpenTTDObjectArea area, uint8_t view) noexcept OPENTTD_SERVICE_DEFAULT(allocate);
	uint16_t (*object_type)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(object_type);
	OpenTTDObjectLoopPolicy (*loop_policy)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(loop_policy);
	OpenTTDObjectIdentity (*identity)(uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(identity);
	OpenTTDObjectArea (*location)(Object *object) noexcept OPENTTD_SERVICE_DEFAULT(location);
	OpenTTDObjectNext (*next)(uint32_t from) noexcept OPENTTD_SERVICE_DEFAULT(next);
	void (*set_colour)(Object *o, uint8_t colour) noexcept OPENTTD_SERVICE_DEFAULT(set_colour);
	uint8_t (*recolour)(uint8_t owner, bool two) noexcept OPENTTD_SERVICE_DEFAULT(recolour);
	void (*colour_error)(const ObjectSpec *spec, uint16_t result) noexcept OPENTTD_SERVICE_DEFAULT(colour_error);
	void (*make)(uint32_t tile, uint8_t owner, uint32_t id, WaterClass water, uint32_t random) noexcept OPENTTD_SERVICE_DEFAULT(make);
	void (*delete_object)(Object *o) noexcept OPENTTD_SERVICE_DEFAULT(delete_object);
	const OpenTTDObjectQueryServices *query;
	const OpenTTDObjectResolverServices *resolver;
	void (*slope_error)(OpenTTDCommandCost *out, const OpenTTDObjectSlopeResult *result, const ObjectSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(slope_error);
	OpenTTDObjectPrice (*build_price)(const ObjectSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(build_price);
	OpenTTDObjectPrice (*clear_price)(const ObjectSpec *spec) noexcept OPENTTD_SERVICE_DEFAULT(clear_price);
	void (*set_hq)(uint8_t owner, uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(set_hq);
	OpenTTDTownRef (*statue_town)(Object *) noexcept OPENTTD_SERVICE_DEFAULT(statue_town);
	void (*close_inspect)(uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(close_inspect);
	bool (*progress_set)(OpenTTDObjectGenerationFrame *frame, uint32_t total) noexcept OPENTTD_SERVICE_DEFAULT(progress_set);
	bool (*progress_increase)(OpenTTDObjectGenerationFrame *frame) noexcept OPENTTD_SERVICE_DEFAULT(progress_increase);
	bool (*normal)() noexcept OPENTTD_SERVICE_DEFAULT(normal);
	bool (*bulldozer)() noexcept OPENTTD_SERVICE_DEFAULT(bulldozer);
	WaterClass (*water_class)(uint32_t tile) noexcept OPENTTD_SERVICE_DEFAULT(water_class);
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
uint32_t openttd_rust_object_variable(const OpenTTDObjectServices *, const ResolverObject *, Object *, const ObjectSpec *, uint32_t, uint8_t, uint8_t, uint32_t, bool *);
bool openttd_rust_object_trigger_tile(const OpenTTDObjectServices *, Object *, uint32_t, ObjectAnimationTrigger, const ObjectSpec *);
bool openttd_rust_object_trigger(const OpenTTDObjectServices *, Object *, ObjectAnimationTrigger, const ObjectSpec *);
void openttd_rust_object_counts_reset();
void openttd_rust_object_count_increment(uint16_t);
void openttd_rust_object_count_decrement(uint16_t);
uint16_t openttd_rust_object_count(uint16_t);
bool openttd_rust_object_indirectly_cleared(uint32_t, uint32_t);
OpenTTDObjectHiddenArea openttd_rust_object_tunnel_hide(uint32_t, uint32_t);
void openttd_rust_object_restore_first(OpenTTDObjectHiddenArea);
void openttd_rust_object_build(const OpenTTDObjectServices * services, uint16_t kind, uint32_t tile, uint8_t owner, const Town * town, uint8_t view);
void openttd_rust_object_update_hq(const OpenTTDObjectServices * services, uint32_t tile, uint32_t score);
void openttd_rust_object_colours(const OpenTTDObjectServices * services, uint8_t company);
void openttd_rust_object_build_command(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, uint16_t flags, uint32_t tile, uint16_t kind, uint8_t view);
void openttd_rust_object_build_area(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * ret, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, OpenTTDCommandCost * last, uint16_t flags, uint32_t tile, uint32_t start, uint16_t kind, uint8_t view, bool diagonal);
void openttd_rust_object_cargo(const OpenTTDObjectServices * services, uint32_t tile, uint32_t * cargo, uint64_t * accepted);
void openttd_rust_object_tile_loop(const OpenTTDObjectServices * services, OpenTTDGroundCall * call, uint32_t tile);
bool openttd_rust_object_generate(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, OpenTTDObjectGenerationFrame * frame);
void openttd_rust_object_terraform(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, uint32_t tile, uint16_t flags, int32_t z, Slope slope);
void openttd_rust_object_build_above(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, uint32_t tile, uint16_t flags, int32_t height);
}
#endif /* RUST_OBJECT_FFI_H */
