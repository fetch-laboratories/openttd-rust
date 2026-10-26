/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_effect_services.hpp Native shell and parsed-spec observations for emission. */
#include "vehicle_effect_ffi.h"
#include "engine_services.h"
#include "services_map.h"
#include "services_rail.h"

const OpenTTDVisualServices &GetRustVisualServices()
{
	static const OpenTTDVisualServices services{
		.vehicle = &GetRustVehicleServices(),
		.shared = &GetRustSharedServices(),
		.gate = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualGate {
			const Vehicle *v = VehicleOf(shell);
			return {.speed = RustVehicleSpeed(v), .status = RustVehicleStatus(v), .kind = RustVehicleVehicleType(v), .smoke = _settings_game.vehicle.smoke_amount, .primary = RustVehiclePrimary(v)};
		},
		.engine = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualEngine {
			Vehicle *v = VehicleOf(shell);
			const Engine *e = EngineOf(RustVehicleGetEngine(shell));
			OpenTTDVisualEngine result{.engine = RustEngine(e), .cache = &v->vcache.cached_vis_effect, .id = RustVehicleEngineID(v), .kind = RustEngineType(RustEngine(e)), .visual = 0, .rail_kind = 0, .engclass = 0, .callback = e->info.callback_mask.Test(VehicleCallbackMask::VisualEffect)};
			switch (e->type) {
				case VEH_TRAIN: result.visual = e->VehInfo<RailVehicleInfo>().visual_effect; result.rail_kind = RustEngineRailKind(e); result.engclass = e->VehInfo<RailVehicleInfo>().engclass; break;
				case VEH_ROAD: result.visual = e->VehInfo<RoadVehicleInfo>().visual_effect; break;
				case VEH_SHIP: result.visual = e->VehInfo<ShipVehicleInfo>().visual_effect; break;
				default: break;
			}
			return result;
		},
		.grf_version = RustEngineGRFVersionOpaque,
		.power_error = [](uint16_t engine) noexcept { ShowNewGrfVehicleError(EngineID(engine), STR_NEWGRF_BROKEN, STR_NEWGRF_BROKEN_POWERED_WAGON, GRFBug::VehPoweredWagon, false); },
		.part = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualPart {
			Vehicle *v = VehicleOf(shell);
			return {.cache = &v->vcache.cached_vis_effect, .tile = RustVehicleTile(v), .kind = RustVehicleVehicleType(v), .status = RustVehicleStatus(v), .tick = RustVehicleTick(v)};
		},
		.offset = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualOffset {
			const Vehicle *v = VehicleOf(shell);
			return {.direction = RustVehicleDirection(v), .length = v->type == VEH_TRAIN ? RustGroundCachedLength(Train::From(v)) : v->type == VEH_ROAD ? RustGroundCachedLength(RoadVehicle::From(v)) : uint8_t(VEHICLE_LENGTH), .flipped = v->type == VEH_TRAIN && Train::From(v)->GetTrainFlags().Test(VehicleRailFlag::Flipped)};
		},
		.train = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualTrain {
			const Train *v = Train::From(VehicleOf(shell));
			return {.tile = RustVehicleTile(v), .speed = RustVehicleSpeed(v), .reversing = v->GetTrainFlags().Test(VehicleRailFlag::Reversing), .front = v->IsFrontEngine()};
		},
		.station = RustIsRailStation,
		.should_stop = [](OpenTTDVehicle *shell, uint32_t tile) noexcept { const Train *v = Train::From(VehicleOf(shell)); return v->current_order.ShouldStopAtStation(v, GetStationIndex(TileIndex(tile))); },
		.depot = SharedIsDepotTile,
		.tunnel = [](uint32_t tile) noexcept { return IsTunnelTile(TileIndex(tile)); },
		.rail_types = [](OpenTTDVehicle *shell) noexcept { return Train::From(VehicleOf(shell))->GetRailTypes().base(); },
		.tile_rail = RustTileRailType,
		.powered = RustRailPoweredMask,
		.power_weight = [](OpenTTDVehicle *shell) noexcept -> OpenTTDVisualPowerWeight { const Train *v = Train::From(VehicleOf(shell)); return {.power = RustTrainCachedPower(v), .weight = v->gcache.cached_weight}; },
	};
	return services;
}
