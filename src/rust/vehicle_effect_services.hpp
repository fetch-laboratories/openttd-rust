/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_effect_services.hpp Native shell and parsed-spec observations for emission. */
#include "vehicle_effect_ffi.h"
#include "engine_services.h"
#include "settings_services.h"
#include "services_map.h"
#include "services_rail.h"

const OpenTTDVisualServices &GetRustVisualServices()
{
	static const OpenTTDVisualServices services{
		.vehicle = &GetRustVehicleServices(),
		.shared = &GetRustSharedServices(),
		.smoke = RustVehicleSmokeAmount,
		.engine = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualEngine {
			const Engine *e = EngineOf(RustVehicleGetEngine(shell));
			OpenTTDVisualEngine result{.engine = RustEngine(e), .kind = RustEngineType(RustEngine(e)), .visual = 0, .callback = VehicleCallbackMasks(RustEngineCallbackMask(RustEngine(e))).Test(VehicleCallbackMask::VisualEffect)};
			switch (result.kind) {
				case VEH_TRAIN: result.visual = e->VehInfo<RailVehicleInfo>().visual_effect; break;
				case VEH_ROAD: result.visual = e->VehInfo<RoadVehicleInfo>().visual_effect; break;
				case VEH_SHIP: result.visual = e->VehInfo<ShipVehicleInfo>().visual_effect; break;
				default: break;
			}
			return result;
		},
		.defaults = [](const OpenTTDEngine *shell) noexcept -> OpenTTDVisualDefaults { const Engine *e = EngineOf(shell); return {.rail_kind = RustEngineRailKind(e), .engclass = RustRailEngineClass(&e->VehInfo<RailVehicleInfo>())}; },
		.grf_version = RustEngineGRFVersionOpaque,
		.power_error = [](uint16_t engine) noexcept { RustVehicleShowNewGRFError(EngineID(engine), STR_NEWGRF_BROKEN, STR_NEWGRF_BROKEN_POWERED_WAGON, GRFBug::VehPoweredWagon, false); },
		.station = RustIsRailStation,
		.depot = SharedIsDepotTile,
		.tunnel = RustIsTunnelTile,
		.tile_rail = RustTileRailType,
		.powered = RustRailPoweredMask,
	};
	return services;
}
