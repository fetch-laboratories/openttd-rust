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
inline uint16_t RustEngineCallbackMask(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.callback_mask.base(); }
inline uint8_t RustEngineMiscFlags(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.misc_flags.base(); }
inline uint64_t RustEngineCargoRefitMask(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.refit_mask; }
inline uint8_t RustEngineRailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RailVehicleInfo>().capacity; }
inline uint8_t RustEngineRoadCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RoadVehicleInfo>().capacity; }
inline uint16_t RustEngineShipCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().capacity; }
inline uint16_t RustEnginePassengerCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().passenger_capacity; }
inline uint8_t RustEngineMailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().mail_capacity; }
inline bool RustEngineIsGroundVehicle(const OpenTTDEngine *e) noexcept { return EngineOf(e)->IsGroundVehicle(); }
inline uint16_t RustNewEngineID(const GRFFile *file, VehicleType type, uint16_t id) noexcept { return GetNewEngineID(file, type, id).base(); }
inline const OpenTTDEngine *RustEngineByIDIfValid(uint16_t id) noexcept { return RustEngine(Engine::GetIfValid(EngineID(id))); }
inline uint8_t RustAircraftInfoSubtype(const AircraftVehicleInfo *info) noexcept { return info->subtype; }
inline uint8_t RustEngineAircraftSubtype(const OpenTTDEngine *e) noexcept { return RustAircraftInfoSubtype(&EngineOf(e)->VehInfo<AircraftVehicleInfo>()); }
inline const OpenTTDEngine *RustEngineByID(uint16_t id) noexcept { return RustEngine(Engine::Get(EngineID(id))); }
inline bool RustEngineHasArticulatedCallback(uint16_t engine) noexcept { return VehicleCallbackMasks(RustEngineCallbackMask(RustEngineByID(engine))).Test(VehicleCallbackMask::ArticEngine); }
inline TimerGameCalendar::Date RustEngineLifeLength(const Engine *e) noexcept { return e->GetLifeLengthInDays(); }
inline int64_t RustEngineCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetCost().base(); }
inline uint16_t RustEngineCargoAge(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.cargo_age_period; }
inline uint16_t RustEngineAircraftRange(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().max_range; }
inline uint8_t RustEngineDefaultCargo(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetDefaultCargoType(); }
inline VehicleType RustEngineType(const OpenTTDEngine *e) noexcept { return EngineOf(e)->type; }
inline bool RustEngineExclusivePreview(const OpenTTDEngine *e) noexcept { return EngineOf(e)->flags.Test(EngineFlag::ExclusivePreview); }
inline bool RustEngineHasGRF(const OpenTTDEngine *e) noexcept { return RustEngineGRF(e) != nullptr; }
inline uint8_t RustEngineRefitCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.refit_cost; }
inline int32_t RustEnginePriceShift(const OpenTTDEngine *e, uint32_t price) noexcept { return RustGRFPriceShift(RustEngineGRF(e), price); }
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
inline uint16_t RustGroundEnginePower(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RailVehInfo(v->engine_type)->power : RoadVehInfo(v->engine_type)->power; }
inline uint16_t RustGroundEngineWeight(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RailVehInfo(v->engine_type)->weight : RoadVehInfo(v->engine_type)->weight; }
inline uint8_t RustGroundEngineTE(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RailVehInfo(v->engine_type)->tractive_effort : RoadVehInfo(v->engine_type)->tractive_effort; }
inline uint8_t RustGroundEngineDrag(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->type == VEH_TRAIN ? RailVehInfo(v->engine_type)->air_drag : RoadVehInfo(v->engine_type)->air_drag; }
inline uint16_t RustEngineWagonPower(uint16_t engine) noexcept { return RailVehInfo(EngineID(engine))->pow_wag_power; }
inline uint8_t RustEngineWagonWeight(uint16_t engine) noexcept { return RailVehInfo(EngineID(engine))->pow_wag_weight; }
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
inline OpenTTDEngineRef Engine_resolve(uint16_t id) noexcept { return EngineRef(EngineOf(RustEngineByIDIfValid(id))); }
#endif /* RUST_ENGINE_SERVICES_H */
