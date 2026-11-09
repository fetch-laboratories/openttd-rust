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
		.power_error = [](uint16_t engine) noexcept { RustVehicleShowNewGRFError(EngineID(engine), STR_NEWGRF_BROKEN, STR_NEWGRF_BROKEN_POWERED_WAGON, GRFBug::VehPoweredWagon, false); },
		.depot = SharedIsDepotTile,
		.tunnel = RustIsTunnelTile,
		.powered = RustRailPoweredMask,
	};
	return services;
}

static_assert(to_underlying(VehicleCallbackMask::VisualEffect) == 0);
