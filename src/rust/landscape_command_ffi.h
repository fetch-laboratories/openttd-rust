/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file landscape_command_ffi.h Whole landscape transaction owner. */
#ifndef RUST_LANDSCAPE_COMMAND_FFI_H
#define RUST_LANDSCAPE_COMMAND_FFI_H
#include "tile_owner_ffi.h"
#include "trees_ffi.h"
struct OpenTTDLandscapeCommandStrings {
	uint32_t canal, clear_limit, water, sea_level, high, edge, bridge, bridge_high, tunnel, terraform_limit, levelled;
};
struct OpenTTDLandscapeTileServices {
	void (*clear)(OpenTTDCommandCost *, uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*terraform)(OpenTTDCommandCost *, uint32_t, uint16_t, int32_t, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(terraform);
	const OpenTTDTileOwnerServices *owners;
	const OpenTTDTreeServices *trees;
	const OpenTTDIndustryTilePolicyServices *industry_policy;
	const OpenTTDIndustryCostServices *industry_costs;
	const OpenTTDIndustryGenerationServices *industry_generation;
};
/* Immutable tables and stack costs outlive synchronous serialized calls. Native
 * leaves neither throw nor retain pointers. No VM/save service is invoked. */
struct OpenTTDLandscapeCommandServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDCommandCostServices *costs;
	const OpenTTDCommandServices *commands;
	OpenTTDLandscapeCommandStrings strings;
	const OpenTTDLandscapeTileServices *tile_procs;
	const OpenTTDWaterCommandServices *water;
	bool (*on_water)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(on_water);
	bool (*coast)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(coast);
	bool (*buoy)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(buoy);
	bool (*canal)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(canal);
	uint8_t (*height_limit)() noexcept OPENTTD_SERVICE_DEFAULT(height_limit);
	uint8_t (*max_bridge_height)() noexcept OPENTTD_SERVICE_DEFAULT(max_bridge_height);
	bool (*freeform_edges)() noexcept OPENTTD_SERVICE_DEFAULT(freeform_edges);
	bool (*editor)() noexcept OPENTTD_SERVICE_DEFAULT(editor);
	bool (*exchange_generating)(bool) noexcept OPENTTD_SERVICE_DEFAULT(exchange_generating);
	bool (*tunnel_in_way)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(tunnel_in_way);
	void (*mark_dirty_height)(uint32_t, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(mark_dirty_height);
	void (*set_height)(uint32_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_height);
	bool (*paused)() noexcept OPENTTD_SERVICE_DEFAULT(paused);
};
const OpenTTDLandscapeCommandServices &GetRustLandscapeCommandServices() noexcept;
extern "C" {
void openttd_rust_landscape_clear(const OpenTTDLandscapeCommandServices *, OpenTTDCommandCost *, uint32_t, uint16_t);
int64_t openttd_rust_landscape_clear_area(const OpenTTDLandscapeCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint32_t, uint32_t, bool, uint16_t);
uint32_t openttd_rust_landscape_terraform(const OpenTTDLandscapeCommandServices *, OpenTTDCommandCost *, uint32_t, uint8_t, bool, uint16_t);
uint32_t openttd_rust_landscape_level(const OpenTTDLandscapeCommandServices *, OpenTTDCommandCost *, OpenTTDCommandCost *, OpenTTDCommandCost *, uint32_t, uint32_t, bool, uint8_t, uint16_t, int64_t *);
}
#endif /* RUST_LANDSCAPE_COMMAND_FFI_H */
