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
		.gate = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualGate {
			const Vehicle *v = VehicleOf(shell);
			return {.speed = RustVehicleSpeed(v), .status = RustVehicleStatus(v), .smoke = RustVehicleSmokeAmount(), .primary = RustVehiclePrimary(v)};
		},
		.show = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualShow {
			const Vehicle *v = VehicleOf(shell);
			return {.speed = RustVehicleSpeed(v), .kind = RustVehicleVehicleType(v), .smoke = RustVehicleSmokeAmount()};
		},
		.engine = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualEngine {
			Vehicle *v = VehicleOf(shell);
			const Engine *e = EngineOf(RustVehicleGetEngine(shell));
			OpenTTDVisualEngine result{.engine = RustEngine(e), .cache = RustVehicleVisualCache(v), .id = RustVehicleEngineID(v), .kind = RustEngineType(RustEngine(e)), .visual = 0, .callback = VehicleCallbackMasks(RustEngineCallbackMask(RustEngine(e))).Test(VehicleCallbackMask::VisualEffect)};
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
		.part = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualPart {
			Vehicle *v = VehicleOf(shell);
			return {.cache = RustVehicleVisualCache(v), .tile = RustVehicleTile(v), .kind = RustVehicleVehicleType(v), .status = RustVehicleStatus(v), .tick = RustVehicleTick(v)};
		},
		.bridge_position = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualBridgePosition { const Vehicle *v = VehicleOf(shell); return {.x = RustVehicleX(v), .y = RustVehicleY(v)}; },
		.offset = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualOffset {
			const Vehicle *v = VehicleOf(shell);
			const VehicleType kind = RustVehicleVehicleType(v);
			return {.direction = RustVehicleDirection(v), .length = kind == VEH_TRAIN ? RustGroundCachedLength(Train::From(v)) : kind == VEH_ROAD ? RustGroundCachedLength(RoadVehicle::From(v)) : uint8_t(VEHICLE_LENGTH)};
		},
		.train = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualTrain {
			const Train *v = Train::From(VehicleOf(shell));
			return {.handle = TrainHandle(v), .tile = RustVehicleTile(v), .speed = RustVehicleSpeed(v), .front = RustVehicleFront(v)};
		},
		.station = RustIsRailStation,
		.depot = SharedIsDepotTile,
		.tunnel = RustIsTunnelTile,
		.tile_rail = RustTileRailType,
		.powered = RustRailPoweredMask,
		.power_weight = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualPowerWeight { const Train *v = Train::From(VehicleOf(shell)); return {.power = RustTrainCachedPower(v), .weight = RustTrainCachedWeight(v)}; },
	};
	return services;
}
