/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine_services.h Shared native engine observations for Rust consumers. */
#ifndef RUST_ENGINE_SERVICES_H
#define RUST_ENGINE_SERVICES_H
#include "engine_ffi.h"
#include "engine_capabilities_ffi.h"
#include "vehicle_services.h"
#include "company_services.h"
#include "environment_services.h"
#include "../articulated_vehicles.h"
#include "../rail.h"
#include "services_rail.h"
#include "../road.h"
#include "../table/strings.h"
#include "services_cargo.h"
inline OpenTTDNewGRFCargoLabel RustEngineCargoLabel(const OpenTTDEngine *e) noexcept { return RustCargoLabel(EngineOf(e)->info.cargo_label); }
inline uint16_t RustEngineCallbackMask(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.callback_mask.base(); }
inline uint8_t RustEngineMiscFlags(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.misc_flags.base(); }
inline bool RustEngineIsTram(const OpenTTDEngine *e) noexcept { return EngineMiscFlags{RustEngineMiscFlags(e)}.Test(EngineMiscFlag::RoadIsTram); }
inline uint64_t RustEngineCargoRefitMask(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.refit_mask; }
inline uint8_t RustEngineRailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RailVehicleInfo>().capacity; }
inline uint8_t RustEngineRoadCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RoadVehicleInfo>().capacity; }
inline uint16_t RustEngineShipCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().capacity; }
inline uint16_t RustEnginePassengerCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().passenger_capacity; }
inline uint8_t RustEngineMailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().mail_capacity; }
inline bool RustEngineIsGroundVehicle(const OpenTTDEngine *e) noexcept { return EngineOf(e)->IsGroundVehicle(); }
inline const OpenTTDEngine *RustEngineByIDIfValid(uint16_t id) noexcept { return RustEngine(Engine::GetIfValid(EngineID(id))); }
inline uint8_t RustAircraftInfoSubtype(const AircraftVehicleInfo *info) noexcept { return info->subtype; }
inline uint8_t RustEngineAircraftSubtype(const OpenTTDEngine *e) noexcept { return RustAircraftInfoSubtype(&EngineOf(e)->VehInfo<AircraftVehicleInfo>()); }
inline const OpenTTDEngine *RustEngineByID(uint16_t id) noexcept { return RustEngine(Engine::Get(EngineID(id))); }
inline bool RustEngineHasArticulatedCallback(uint16_t engine) noexcept { return VehicleCallbackMasks(RustEngineCallbackMask(RustEngineByID(engine))).Test(VehicleCallbackMask::ArticEngine); }
inline uint8_t RustEngineAircraftCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().cost_factor; }
inline uint8_t RustEngineAircraftRunning(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().running_cost; }
inline uint8_t RustEngineRoadCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RoadVehicleInfo>().cost_factor; }
inline Price RustEngineRoadPrice(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RoadVehicleInfo>().running_cost_class; }
inline uint8_t RustEngineRoadRunning(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RoadVehicleInfo>().running_cost; }
inline uint8_t RustEngineShipCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().cost_factor; }
inline uint8_t RustEngineShipRunning(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().running_cost; }
inline uint8_t RustEngineTrainCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RailVehicleInfo>().cost_factor; }
inline Price RustEngineTrainPrice(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RailVehicleInfo>().running_cost_class; }
inline uint8_t RustEngineTrainRunning(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RailVehicleInfo>().running_cost; }
inline int32_t RustEngineLifeYears(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.lifelength.base(); }
inline bool RustEngineValidString(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.string_id != STR_NEWGRF_INVALID_ENGINE; }

inline uint16_t RustEngineCargoAge(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.cargo_age_period; }
inline uint16_t RustEngineAircraftRange(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().max_range; }
inline uint8_t RustEngineDefaultCargo(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetDefaultCargoType(); }
inline VehicleType RustEngineType(const OpenTTDEngine *e) noexcept { return EngineOf(e)->type; }
inline bool RustEngineExclusivePreview(const OpenTTDEngine *e) noexcept { return EngineOf(e)->flags.Test(EngineFlag::ExclusivePreview); }
inline void RustEngineSetGRFFile(const OpenTTDEngine *shell, const GRFFile *file) noexcept { RustSpecSetGRFFile(const_cast<Engine *>(EngineOf(shell)), file); }
inline bool RustEngineHasGRF(const OpenTTDEngine *e) noexcept { return RustSpecHasGRFFile(EngineOf(e)); }
inline uint8_t RustEngineRefitCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.refit_cost; }
inline int32_t RustEnginePriceShift(const OpenTTDEngine *e, uint32_t price) noexcept { return RustGRFPriceShift(RustEngineGRF(e), price); }
inline uint16_t RustEngineVariant(const Engine *e) noexcept { return e->info.variant_id.base(); }
inline uint16_t RustEngineVariant(const OpenTTDEngine *e) noexcept { return RustEngineVariant(EngineOf(e)); }

inline void RustSetEngineInvalidString(const OpenTTDEngine *e) noexcept { const_cast<Engine *>(EngineOf(e))->info.string_id = STR_NEWGRF_INVALID_ENGINE; }
inline void RustSetEngineVariant(const OpenTTDEngine *e, uint16_t variant) noexcept { const_cast<Engine *>(EngineOf(e))->info.variant_id = EngineID{variant}; }
inline void RustSetEngineCargo(const OpenTTDEngine *e, CargoType cargo) noexcept { const_cast<Engine *>(EngineOf(e))->info.cargo_type = cargo; }
inline void RustSetEngineRefitMask(const OpenTTDEngine *e, CargoTypes mask) noexcept { const_cast<Engine *>(EngineOf(e))->info.refit_mask = mask; }
inline void RustDisableEngine(const OpenTTDEngine *e) noexcept { const_cast<Engine *>(EngineOf(e))->info.climates = {}; }
inline bool RustEngineOldRefittable(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().old_refittable; }
inline void RustSetEngineOldRefittable(const OpenTTDEngine *e, bool value) noexcept { const_cast<Engine *>(EngineOf(e))->VehInfo<ShipVehicleInfo>().old_refittable = value; }
inline void RustSetEngineRoadSpeed(const OpenTTDEngine *e, uint16_t speed) noexcept { const_cast<Engine *>(EngineOf(e))->VehInfo<RoadVehicleInfo>().max_speed = speed; }
inline void RustSetEngineRoadType(const OpenTTDEngine *e, RoadType rt) noexcept { const_cast<Engine *>(EngineOf(e))->VehInfo<RoadVehicleInfo>().roadtype = rt; }
inline void RustSetEngineIntro(const OpenTTDEngine *shell, int32_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.base_intro = TimerGameCalendar::Date{value}; }
inline void RustSetEngineDecay(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.decay_speed = value; }
inline void RustSetEngineLife(const OpenTTDEngine *shell, int32_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.lifelength = TimerGameCalendar::Year{value}; }
inline void RustSetEngineModelLife(const OpenTTDEngine *shell, int32_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.base_life = TimerGameCalendar::Year{value}; }
inline void RustSetEngineClimates(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.climates = LandscapeTypes{value}; }
inline void RustSetEngineLoadAmount(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.load_amount = value; }
inline void RustSetEngineCargoLabel(const OpenTTDEngine *shell, uint32_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.cargo_label = CargoLabel{value}; }
inline void RustSetEngineCallbacks(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.callback_mask = VehicleCallbackMasks{value}; }
inline void RustSetEngineRefitCost(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.refit_cost = value; }
inline void RustSetEngineRetire(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.retire_early = value; }
inline void RustSetEngineMisc(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.misc_flags = EngineMiscFlags{value}; }
inline void RustSetEngineExtra(const OpenTTDEngine *shell, uint32_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.extra_flags = static_cast<ExtraEngineFlags>(value); }
inline void RustSetEngineCargoAge(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->info.cargo_age_period = value; }
inline void RustSetEngineTrainAi(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().ai_passenger_only = value; }
inline void RustSetEngineTrainSpeed(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().max_speed = value; }
inline void RustSetEngineTrainPower(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().power = value; }
inline void RustSetEngineTrainKind(const OpenTTDEngine *shell, RailVehicleTypes value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().railveh_type = value; }
inline void RustSetEngineTrainRunning(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().running_cost = value; }
inline void RustSetEngineTrainPrice(const OpenTTDEngine *shell, Price value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().running_cost_class = value; }
inline void RustSetEngineTrainImage(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().image_index = value; }
inline void RustSetEngineTrainCapacity(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().capacity = value; }
inline void RustSetEngineTrainWeight(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().weight = value; }
inline void RustSetEngineTrainCost(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().cost_factor = value; }
inline void RustSetEngineTrainClass(const OpenTTDEngine *shell, EngineClass value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().engclass = value; }
inline void RustSetEngineTrainWagonPower(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().pow_wag_power = value; }
inline void RustSetEngineTrainTe(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().tractive_effort = value; }
inline void RustSetEngineTrainDrag(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().air_drag = value; }
inline void RustSetEngineTrainShorten(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().shorten_factor = value; }
inline void RustSetEngineTrainVisual(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().visual_effect = value; }
inline void RustSetEngineTrainWagonWeight(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().pow_wag_weight = value; }
inline void RustSetEngineTrainUser(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().user_def_data = value; }
inline void RustSetEngineTrainCurve(const OpenTTDEngine *shell, int16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>().curve_speed_mod = value; }
inline void RustSetEngineRoadRunning(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().running_cost = value; }
inline void RustSetEngineRoadPrice(const OpenTTDEngine *shell, Price value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().running_cost_class = value; }
inline void RustSetEngineRoadImage(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().image_index = value; }
inline void RustSetEngineRoadCapacity(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().capacity = value; }
inline void RustSetEngineRoadCost(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().cost_factor = value; }
inline void RustSetEngineRoadSound(const OpenTTDEngine *shell, SoundID value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().sfx = value; }
inline void RustSetEngineRoadPower(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().power = value; }
inline void RustSetEngineRoadWeight(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().weight = value; }
inline void RustSetEngineRoadTe(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().tractive_effort = value; }
inline void RustSetEngineRoadDrag(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().air_drag = value; }
inline void RustSetEngineRoadVisual(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().visual_effect = value; }
inline void RustSetEngineRoadShorten(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<RoadVehicleInfo>().shorten_factor = value; }
inline void RustSetEngineShipImage(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().image_index = value; }
inline void RustSetEngineShipCost(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().cost_factor = value; }
inline void RustSetEngineShipSpeed(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().max_speed = value; }
inline void RustSetEngineShipCapacity(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().capacity = value; }
inline void RustSetEngineShipRunning(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().running_cost = value; }
inline void RustSetEngineShipSound(const OpenTTDEngine *shell, SoundID value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().sfx = value; }
inline void RustSetEngineShipOcean(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().ocean_speed_frac = value; }
inline void RustSetEngineShipCanal(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().canal_speed_frac = value; }
inline void RustSetEngineShipVisual(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().visual_effect = value; }
inline void RustSetEngineShipAcceleration(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<ShipVehicleInfo>().acceleration = value; }
inline void RustSetEngineAircraftImage(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().image_index = value; }
inline void RustSetEngineAircraftSubtype(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().subtype = value; }
inline void RustSetEngineAircraftCost(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().cost_factor = value; }
inline void RustSetEngineAircraftSpeed(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().max_speed = value; }
inline void RustSetEngineAircraftAcceleration(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().acceleration = value; }
inline void RustSetEngineAircraftRunning(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().running_cost = value; }
inline void RustSetEngineAircraftPassengers(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().passenger_capacity = value; }
inline void RustSetEngineAircraftMail(const OpenTTDEngine *shell, uint8_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().mail_capacity = value; }
inline void RustSetEngineAircraftSound(const OpenTTDEngine *shell, SoundID value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().sfx = value; }
inline void RustSetEngineAircraftRange(const OpenTTDEngine *shell, uint16_t value) noexcept { const_cast<Engine *>(EngineOf(shell))->VehInfo<AircraftVehicleInfo>().max_range = value; }
inline void RustSetEngineListPosition(const OpenTTDEngine *e, uint16_t position) noexcept { const_cast<Engine *>(EngineOf(e))->list_position = position; }
template <typename Result, auto Read, typename Index = uint16_t>
static Result RustIndexedEngineRead(Index id) noexcept { return Read(RustEngineByID(static_cast<uint16_t>(id))); }
inline OpenTTDEngineCarrying RustEngineCarrying(const OpenTTDEngine *engine) noexcept
{
	OpenTTDEngineCarrying r{.kind = RustEngineType(engine), .ground_capacity = 0, .default_cargo = INVALID_CARGO};
	switch (r.kind) {
		case VEH_TRAIN: r.ground_capacity = RustEngineRailCapacity(engine); break;
		case VEH_ROAD: r.ground_capacity = RustEngineRoadCapacity(engine); break;
		default: break;
	}
	if ((r.kind == VEH_TRAIN || r.kind == VEH_ROAD) && r.ground_capacity == 0) return r;
	r.default_cargo = RustEngineDefaultCargo(engine);
	return r;
}
inline uint8_t RustEngineExtraFlags(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.extra_flags.base(); }
inline uint16_t RustEngineID(const OpenTTDEngine *e) noexcept { return EngineOf(e)->index.base(); }
inline OpenTTDEngineCapacityConfig RustEngineCapacityConfig(const OpenTTDEngine *engine) noexcept
{
	return {.id = RustEngineID(engine), .callbacks = RustEngineCallbackMask(engine), .misc = RustEngineMiscFlags(engine)};
}
inline int32_t RustEngineBaseIntro(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.base_intro.base(); }
inline bool RustEngineClimate(const Engine *e) noexcept { return e->info.climates.Test(OpenTTDLandscape()); }
inline bool RustEngineClimate(const OpenTTDEngine *e) noexcept { return RustEngineClimate(EngineOf(e)); }
inline uint8_t RustRailEngineClass(const RailVehicleInfo *info) noexcept { return info->engclass; }
inline RailVehicleTypes RustEngineRailKind(const Engine *e) noexcept { return e->VehInfo<RailVehicleInfo>().railveh_type; }
inline uint64_t RustEngineRailTypes(const Engine *e) noexcept { return e->VehInfo<RailVehicleInfo>().railtypes.base(); }
inline uint64_t RustEngineRailTypes(const OpenTTDEngine *e) noexcept { return RustEngineRailTypes(EngineOf(e)); }
inline RoadType RustEngineRoadType(const Engine *e) noexcept { return e->VehInfo<RoadVehicleInfo>().roadtype; }
inline RoadType RustEngineRoadType(const OpenTTDEngine *e) noexcept { return RustEngineRoadType(EngineOf(e)); }
/* Callers gate the vehicle type; inactive climate/wagon fields are not observed. */
inline OpenTTDEngineRailAvailability RustEngineRailAvailability(const OpenTTDEngine *shell) noexcept
{
	const Engine *e = EngineOf(shell);
	OpenTTDEngineRailAvailability info{.climate = RustEngineClimate(e), .kind = {}, .railtypes = 0};
	if (info.climate) {
		info.kind = RustEngineRailKind(e);
		if (info.kind != RAILVEH_WAGON) info.railtypes = RustEngineRailTypes(e);
	}
	return info;
}
inline OpenTTDEngineRoadAvailability RustEngineRoadAvailability(const OpenTTDEngine *shell) noexcept
{
	const Engine *e = EngineOf(shell);
	OpenTTDEngineRoadAvailability info{.climate = RustEngineClimate(e), .roadtype = INVALID_ROADTYPE};
	if (info.climate) info.roadtype = RustEngineRoadType(e);
	return info;
}
inline uint64_t RustEngineCompatibleRailTypes(const Engine *e) noexcept { return RustAllCompatibleRailTypes(RustEngineRailTypes(RustEngine(e))); }
/** Reserve a unit number and assign the returned native ID to the shell. */
inline void RustVehicleUseUnitNumber(Vehicle *v, uint16_t unit) noexcept { v->unitnumber = RustCompanyUseVehicleUnit(v->owner, v->type, unit); }
inline void RustVehicleUseCurrentUnit(void *shell) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	RustCompanyUseVehicleUnit(v->owner, v->type, RustVehicleUnitNumber(v));
}
inline bool RustEngineCompatibleRail(const OpenTTDEngine *e, uint32_t tile) noexcept { return RustRailCompatible(RailTypes{RustEngineRailTypes(e)}, RustRailType(tile)); }
inline bool RustEngineHasPowerOnRail(const OpenTTDEngine *e, uint32_t tile) noexcept { return RustRailPowered(RailTypes{RustEngineRailTypes(e)}, RustRailType(tile)); }
inline uint64_t RustEnginePoweredRoadTypes(const Engine *e) noexcept { return GetRoadTypeInfo(RustEngineRoadType(RustEngine(e)))->powered_roadtypes.base(); }
inline uint64_t FleetTransaction_rail_compatible(uint16_t id) noexcept { return RustEngineCompatibleRailTypes(Engine::Get(EngineID{id})); }
inline uint64_t FleetTransaction_road_powered(uint16_t id) noexcept { return RustEnginePoweredRoadTypes(Engine::Get(EngineID{id})); }
inline bool FleetTransaction_wagon(uint16_t id) noexcept { return RustEngineRailKind(Engine::Get(EngineID{id})) == RAILVEH_WAGON; }
inline RailVehicleTypes Engine_rail_kind(const OpenTTDEngine *e) noexcept { return RustEngineRailKind(EngineOf(e)); }
inline uint64_t Engine_compatible_railtypes(const OpenTTDEngine *e) noexcept { return RustEngineCompatibleRailTypes(EngineOf(e)); }
inline uint64_t Engine_powered_roadtypes(const OpenTTDEngine *e) noexcept { return RustEnginePoweredRoadTypes(EngineOf(e)); }
inline OpenTTDEngineCompany Engine_company_next(uint32_t from) noexcept
{
	Company *c = RustCompanyNext(from);
	return c == nullptr ? OpenTTDEngineCompany{nullptr, CompanyID::Invalid().base()} : OpenTTDEngineCompany{c, c->index.base()};
}
inline OpenTTDEngineVehicle Engine_vehicle_next(uint32_t from) noexcept
{
	const Vehicle *v = VehicleOf(RustNextVehicle(from));
	return v == nullptr ? OpenTTDEngineVehicle{nullptr, UINT32_MAX, 0, VEH_INVALID} : OpenTTDEngineVehicle{RustVehicle(v), RustVehicleIndexOpaque(v), RustVehicleOwner(v), RustVehicleVehicleType(v)};
}
inline uint16_t RustRailEnginePower(const RailVehicleInfo *info) noexcept { return info->power; }
inline uint16_t RustRailEngineWeight(const RailVehicleInfo *info) noexcept { return info->weight; }
inline uint16_t RustEngineTrainPower(const OpenTTDEngine *e) noexcept { return RustRailEnginePower(&EngineOf(e)->VehInfo<RailVehicleInfo>()); }
inline uint16_t RustEngineTrainWeight(const OpenTTDEngine *e) noexcept { return RustRailEngineWeight(&EngineOf(e)->VehInfo<RailVehicleInfo>()); }
inline EngineClass RustEngineTrainClass(const OpenTTDEngine *e) noexcept { return static_cast<EngineClass>(RustRailEngineClass(&EngineOf(e)->VehInfo<RailVehicleInfo>())); }
inline std::vector<BadgeID> *RustEngineBadges(const OpenTTDEngine *e) noexcept { return &const_cast<Engine *>(EngineOf(e))->badges; }
inline uint16_t RustGroundEnginePower(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RustRailEnginePower(RailVehInfo(v->engine_type)) : RoadVehInfo(v->engine_type)->power; }
inline uint16_t RustGroundEngineWeight(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RustRailEngineWeight(RailVehInfo(v->engine_type)) : RoadVehInfo(v->engine_type)->weight; }
inline uint8_t RustGroundEngineTE(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RailVehInfo(v->engine_type)->tractive_effort : RoadVehInfo(v->engine_type)->tractive_effort; }
inline uint8_t RustGroundEngineDrag(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RailVehInfo(v->engine_type)->air_drag : RoadVehInfo(v->engine_type)->air_drag; }
inline uint16_t RustEngineWagonPower(uint16_t engine) noexcept { return RailVehInfo(EngineID(engine))->pow_wag_power; }
inline uint8_t RustEngineWagonWeight(uint16_t engine) noexcept { return RailVehInfo(EngineID(engine))->pow_wag_weight; }
inline uint8_t RustEngineGRFVersionOpaque(const OpenTTDEngine *e) noexcept { return RustEngineGRFVersion(EngineOf(e)); }
inline void RustEngineArticulatedWarning(const OpenTTDEngine *e) noexcept { RustVehicleShowNewGRFError(EngineOf(e)->index, STR_NEWGRF_BUGGY, STR_NEWGRF_BUGGY_ARTICULATED_CARGO, GRFBug::VehRefit, false); }
inline OpenTTDEngineRef EngineRef(const Engine *e) noexcept
{
	if (e == nullptr) return {.shell = nullptr, .state = nullptr, .id = EngineID::Invalid().base(), .type = VEH_INVALID};
	return {.shell = RustEngine(e), .state = e->rust_lifecycle.state, .id = e->index.base(), .type = RustEngineType(RustEngine(e))};
}
inline OpenTTDEngineRef Engine_next(uint32_t from) noexcept
{
	for (Engine *e : Engine::Iterate(from)) return EngineRef(e);
	return EngineRef(nullptr);
}
inline OpenTTDEngineRef Engine_resolve_strict(uint16_t id) noexcept { return EngineRef(EngineOf(RustEngineByID(id))); }
inline OpenTTDEngineRef Engine_resolve(uint16_t id) noexcept { return EngineRef(EngineOf(RustEngineByIDIfValid(id))); }
#endif /* RUST_ENGINE_SERVICES_H */
