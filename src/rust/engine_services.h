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
inline OpenTTDNewGRFCargoLabel RustEngineCargoLabel(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.cargo_label; }
inline uint16_t RustEngineCallbackMask(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.callback_mask.base(); }
inline uint8_t RustEngineMiscFlags(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.misc_flags.base(); }
inline bool RustEngineIsTram(const OpenTTDEngine *e) noexcept { return EngineMiscFlags{RustEngineMiscFlags(e)}.Test(EngineMiscFlag::RoadIsTram); }
inline uint64_t RustEngineCargoRefitMask(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.refit_mask; }
inline uint8_t RustEngineRailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RailVehicleInfo>().capacity; }
inline uint8_t RustEngineRoadCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RoadVehicleInfo>().capacity; }
inline uint16_t RustEngineShipCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().capacity; }
inline uint16_t RustEnginePassengerCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().passenger_capacity; }
inline uint8_t RustEngineMailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().mail_capacity; }
inline const OpenTTDEngine *RustEngineByIDIfValid(uint16_t id) noexcept { return RustEngine(Engine::GetIfValid(EngineID(id))); }
inline uint8_t RustAircraftInfoSubtype(const AircraftVehicleInfo *info) noexcept { return info->subtype; }
inline uint8_t RustEngineAircraftSubtype(const OpenTTDEngine *e) noexcept { return RustAircraftInfoSubtype(&EngineOf(e)->VehInfo<AircraftVehicleInfo>()); }
inline const OpenTTDEngine *RustEngineByID(uint16_t id) noexcept { return RustEngine(Engine::Get(EngineID(id))); }
inline bool RustEngineHasArticulatedCallback(uint16_t engine) noexcept { return VehicleCallbackMasks(RustEngineCallbackMask(RustEngineByID(engine))).Test(VehicleCallbackMask::ArticEngine); }
inline StringID *RustEngineString(const OpenTTDEngine *e) noexcept { return &const_cast<Engine *>(EngineOf(e))->Record().info.string_id; }

inline uint16_t RustEngineCargoAge(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.cargo_age_period; }
inline uint8_t RustEngineDefaultCargo(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetDefaultCargoType(); }
inline VehicleType RustEngineType(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().type; }
inline bool RustEngineExclusivePreview(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().flags.Test(EngineFlag::ExclusivePreview); }
inline bool RustEngineHasGRF(const OpenTTDEngine *e) noexcept { return e->bindings.grf_prop.grffile != nullptr; }
inline uint8_t RustEngineRefitCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.refit_cost; }
inline uint16_t RustEngineVariant(const Engine *e) noexcept { return e->Record().info.variant_id.base(); }
inline uint16_t RustEngineVariant(const OpenTTDEngine *e) noexcept { return RustEngineVariant(EngineOf(e)); }

inline void RustSetEngineInvalidString(const OpenTTDEngine *e) noexcept { *RustEngineString(e) = STR_NEWGRF_INVALID_ENGINE; }
inline void RustSetEngineVariant(const OpenTTDEngine *e, uint16_t variant) noexcept { const_cast<Engine *>(EngineOf(e))->Record().info.variant_id = EngineID{variant}; }
inline void RustSetEngineCargo(const OpenTTDEngine *e, CargoType cargo) noexcept { const_cast<Engine *>(EngineOf(e))->Record().info.cargo_type = cargo; }
inline void RustSetEngineRefitMask(const OpenTTDEngine *e, CargoTypes mask) noexcept { const_cast<Engine *>(EngineOf(e))->Record().info.refit_mask = mask; }
inline void RustDisableEngine(const OpenTTDEngine *e) noexcept { const_cast<Engine *>(EngineOf(e))->Record().info.climates = {}; }
inline bool RustEngineOldRefittable(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().old_refittable; }
inline void RustSetEngineOldRefittable(const OpenTTDEngine *e, bool value) noexcept { const_cast<Engine *>(EngineOf(e))->VehInfo<ShipVehicleInfo>().old_refittable = value; }
inline void RustSetEngineRoadSpeed(const OpenTTDEngine *e, uint16_t speed) noexcept { const_cast<Engine *>(EngineOf(e))->VehInfo<RoadVehicleInfo>().max_speed = speed; }
inline void RustSetEngineRoadType(const OpenTTDEngine *e, RoadType rt) noexcept { const_cast<Engine *>(EngineOf(e))->VehInfo<RoadVehicleInfo>().roadtype = rt; }
inline uint16_t RustEngineRoadSound(const OpenTTDEngine *shell) noexcept { return RoadVehInfo(EngineOf(shell)->index)->sfx; }
inline void RustSetEngineListPosition(const OpenTTDEngine *e, uint16_t position) noexcept { const_cast<Engine *>(EngineOf(e))->Record().list_position = position; }
template <typename Result, auto Read, typename Index = uint16_t>
static Result RustIndexedEngineRead(Index id) noexcept { return Read(RustEngineByID(static_cast<uint16_t>(id))); }
inline uint16_t RustEngineID(const OpenTTDEngine *e) noexcept { return EngineOf(e)->index.base(); }
inline bool RustEngineClimate(const Engine *e) noexcept { return e->Record().info.climates.Test(OpenTTDLandscape()); }
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
inline void RustVehicleUseUnitNumber(Vehicle *v, uint16_t unit) noexcept { v->unitnumber = RustCompanyUseVehicleUnit(v->GetOwner(), v->GetType(), unit); }
inline void RustVehicleUseCurrentUnit(void *shell) noexcept
{
	Vehicle *v = static_cast<Vehicle *>(shell);
	RustCompanyUseVehicleUnit(v->GetOwner(), v->GetType(), RustVehicleUnitNumber(v));
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
inline uint16_t RustGroundEnginePower(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->GetType() == VEH_TRAIN ? RustRailEnginePower(RailVehInfo(v->GetEngineType())) : RoadVehInfo(v->GetEngineType())->power; }
inline uint16_t RustGroundEngineWeight(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->GetType() == VEH_TRAIN ? RustRailEngineWeight(RailVehInfo(v->GetEngineType())) : RoadVehInfo(v->GetEngineType())->weight; }
inline uint8_t RustGroundEngineTE(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->GetType() == VEH_TRAIN ? RailVehInfo(v->GetEngineType())->tractive_effort : RoadVehInfo(v->GetEngineType())->tractive_effort; }
inline uint8_t RustGroundEngineDrag(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->GetType() == VEH_TRAIN ? RailVehInfo(v->GetEngineType())->air_drag : RoadVehInfo(v->GetEngineType())->air_drag; }
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
