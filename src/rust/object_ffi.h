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
struct OpenTTDObjectLoopFrame;
struct OpenTTDObjectGenerationFrame;
struct OpenTTDObjectArea { uint32_t tile; uint16_t width, height; };
struct OpenTTDObjectIdentity { Object *object; OpenTTDObjectArea area; uint16_t kind; };
struct OpenTTDObjectReference { Object *object; uint16_t kind; };
struct OpenTTDObjectAllocation { Object *object; uint32_t id; };
struct OpenTTDObjectNext { Object *object; uint32_t id, tile; };
struct OpenTTDObjectLoopPolicy { const ObjectSpec *spec; Object *object; uint32_t north; uint16_t flags, kind; };
struct OpenTTDObjectPolicy { uint16_t flags; uint8_t callbacks; };
struct OpenTTDObjectFootprint { uint16_t flags; uint8_t callbacks, size; };
struct OpenTTDObjectClearance { uint16_t flags; uint8_t height; };
struct OpenTTDObjectAdmission { uint8_t views, height; };
struct OpenTTDObjectAreaShape { uint8_t views, size; };
struct OpenTTDObjectGeneration { uint16_t flags; uint8_t amount, views; bool ever; };
struct OpenTTDObjectSlopeResult { uint16_t callback; std::array<int32_t, 16> registers; uint8_t version; };
struct OpenTTDObjectStrings {
	uint32_t too_many, no_town, edge_sub, water, edge, on_water, bridge, flat;
	uint32_t own, built, limit, hq, object;
};
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
	size_t (*spec_count)() noexcept;
	const ObjectSpec * (*spec)(uint16_t type) noexcept;
	uint8_t (*callbacks)(const ObjectSpec *) noexcept;
	OpenTTDObjectPolicy (*colour_policy)(Object *) noexcept;
	uint16_t (*flags)(const ObjectSpec *) noexcept;
	OpenTTDObjectFootprint (*footprint)(const ObjectSpec *spec) noexcept;
	OpenTTDObjectClearance (*clearance)(const ObjectSpec *) noexcept;
	bool (*available)(const ObjectSpec *) noexcept;
	bool (*ever)(const ObjectSpec *) noexcept;
	OpenTTDObjectAreaShape (*area_shape)(const ObjectSpec *) noexcept;
	OpenTTDObjectAdmission (*admission)(const ObjectSpec *spec) noexcept;
	OpenTTDObjectGeneration (*generation)(const ObjectSpec *spec) noexcept;
	bool (*can_allocate)() noexcept;
	OpenTTDObjectAllocation (*allocate)(uint16_t type, const Town *town, OpenTTDObjectArea area, uint8_t view) noexcept;
	const Town * (*nearest_town)(uint32_t tile) noexcept;
	uint16_t (*object_type)(uint32_t) noexcept;
	OpenTTDObjectLoopPolicy (*loop_policy)(uint32_t) noexcept;
	OpenTTDObjectReference (*reference)(uint32_t) noexcept;
	OpenTTDObjectIdentity (*identity)(uint32_t tile) noexcept;
	OpenTTDObjectArea (*location)(uint32_t tile) noexcept;
	OpenTTDObjectNext (*next)(uint32_t from) noexcept;
	void (*set_colour)(Object *o, uint8_t colour) noexcept;
	uint8_t (*recolour)(uint8_t owner, bool two) noexcept;
	uint16_t (*colour_callback)(const ObjectSpec *spec, Object *o, uint8_t colour, uint32_t tile) noexcept;
	void (*colour_error)(const ObjectSpec *spec, uint16_t result) noexcept;
	void (*make)(uint32_t tile, uint8_t owner, uint32_t id, WaterClass water, uint32_t random) noexcept;
	void (*delete_object)(Object *o) noexcept;
	void (*trigger)(Object *o, ObjectAnimationTrigger trigger, const ObjectSpec *spec) noexcept;
	void (*trigger_tile)(Object *o, uint32_t tile, const ObjectSpec *spec) noexcept;
	OpenTTDObjectSlopeResult (*slope_callback)(const ObjectSpec *spec, uint32_t tile, Slope slope, uint32_t offset, uint8_t view) noexcept;
	void (*slope_error)(OpenTTDCommandCost *out, const OpenTTDObjectSlopeResult *result, const ObjectSpec *spec) noexcept;
	void (*buildable)(OpenTTDCommandCost *out, uint32_t tile, int32_t *z) noexcept;
	OpenTTDObjectPrice (*build_price)(const ObjectSpec *spec) noexcept;
	OpenTTDObjectPrice (*clear_price)(const ObjectSpec *spec) noexcept;
	void (*set_hq)(uint8_t owner, uint32_t tile) noexcept;
	OpenTTDTownRef (*statue_town)(Object *) noexcept;
	void (*hq_begin)(OpenTTDObjectLoopFrame *frame, uint32_t tile, uint32_t level) noexcept;
	void (*hq_deliver)(OpenTTDObjectLoopFrame *frame, uint8_t cargo, uint32_t amount, uint8_t owner) noexcept;
	void (*hq_end)(OpenTTDObjectLoopFrame *frame) noexcept;
	uint16_t (*autoslope_callback)(const ObjectSpec *spec, Object *o, uint32_t tile) noexcept;
	bool (*boolean_callback)(const ObjectSpec *, uint16_t, uint16_t) noexcept;
	bool (*enabled)(const ObjectSpec *spec) noexcept;
	int32_t (*bridge_top)(uint32_t tile) noexcept;
	void (*close_inspect)(uint32_t tile) noexcept;
	bool (*progress_set)(OpenTTDObjectGenerationFrame *frame, uint32_t total) noexcept;
	bool (*progress_increase)(OpenTTDObjectGenerationFrame *frame) noexcept;
	bool (*normal)() noexcept;
	bool (*freeform)() noexcept;
	bool (*bulldozer)() noexcept;
	WaterClass (*water_class)(uint32_t tile) noexcept;
	const OpenTTDWaterRestorationServices *restoration;
	void (*set_owner)(uint32_t tile, uint8_t owner) noexcept;
	uint32_t (*scale_size_1d)(uint32_t n) noexcept;
	uint8_t (*passengers)() noexcept;
	uint8_t (*mail)() noexcept;
	uint32_t (*scale_cargo)(uint32_t amount) noexcept;
};
const OpenTTDObjectServices &GetRustObjectServices() noexcept;
/* All calls are serialized by the game-state mutex, including across the GUI and
 * game threads. Indices survive append. No element pointer or allocation escapes;
 * callbacks may reenter only after every Rust state borrow has ended. */
extern "C" {
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
void openttd_rust_object_build_area(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * ret, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, OpenTTDCommandCost * test, OpenTTDCommandCost * last, uint16_t flags, uint32_t tile, uint32_t start, uint16_t kind, uint8_t view, bool diagonal);
void openttd_rust_object_clear(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * tmp, uint32_t tile, uint16_t flags);
void openttd_rust_object_cargo(const OpenTTDObjectServices * services, uint32_t tile, uint32_t * cargo, uint64_t * accepted);
void openttd_rust_object_tile_loop(const OpenTTDObjectServices * services, OpenTTDObjectLoopFrame * frame, OpenTTDGroundCall * call, uint32_t tile);
bool openttd_rust_object_generate(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, OpenTTDCommandCost * tmp, OpenTTDCommandCost * aux, OpenTTDObjectGenerationFrame * frame);
void openttd_rust_object_change_owner(const OpenTTDObjectServices * services, uint32_t tile, uint8_t old, uint8_t new_owner);
void openttd_rust_object_terraform(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, uint32_t tile, uint16_t flags, int32_t z, Slope slope);
void openttd_rust_object_build_above(const OpenTTDObjectServices * services, OpenTTDCommandCost * out, uint32_t tile, uint16_t flags, int32_t height);
}
#endif /* RUST_OBJECT_FFI_H */
