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
inline uint16_t RustEngineCallbackMask(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.callback_mask.base(); }
inline uint8_t RustEngineMiscFlags(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.misc_flags.base(); }
inline bool RustEngineIsTram(const OpenTTDEngine *e) noexcept { return EngineMiscFlags{RustEngineMiscFlags(e)}.Test(EngineMiscFlag::RoadIsTram); }
inline uint8_t RustEngineRailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RailVehicleInfo>().capacity; }
inline uint8_t RustEngineRoadCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<RoadVehicleInfo>().capacity; }
inline uint16_t RustEngineShipCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<ShipVehicleInfo>().capacity; }
inline uint16_t RustEnginePassengerCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().passenger_capacity; }
inline uint8_t RustEngineMailCapacity(const OpenTTDEngine *e) noexcept { return EngineOf(e)->VehInfo<AircraftVehicleInfo>().mail_capacity; }
inline const OpenTTDEngine *RustEngineByIDIfValid(uint16_t id) noexcept { return RustEngine(Engine::GetIfValid(EngineID(id))); }
inline uint8_t RustAircraftInfoSubtype(const AircraftVehicleInfo *info) noexcept { return info->subtype; }
inline const OpenTTDEngine *RustEngineByID(uint16_t id) noexcept { return RustEngine(Engine::Get(EngineID(id))); }
inline bool RustEngineHasArticulatedCallback(uint16_t engine) noexcept { return VehicleCallbackMasks(RustEngineCallbackMask(RustEngineByID(engine))).Test(VehicleCallbackMask::ArticEngine); }

inline uint16_t RustEngineCargoAge(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().info.cargo_age_period; }
inline uint8_t RustEngineDefaultCargo(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetDefaultCargoType(); }
inline VehicleType RustEngineType(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().type; }
inline bool RustEngineExclusivePreview(const OpenTTDEngine *e) noexcept { return EngineOf(e)->Record().flags.Test(EngineFlag::ExclusivePreview); }

template <typename Result, auto Read, typename Index = uint16_t>
static Result RustIndexedEngineRead(Index id) noexcept { return Read(RustEngineByID(static_cast<uint16_t>(id))); }
inline uint16_t RustEngineID(const OpenTTDEngine *e) noexcept { return EngineOf(e)->index.base(); }
inline uint8_t RustRailEngineClass(const RailVehicleInfo *info) noexcept { return info->engclass; }
inline RailVehicleTypes RustEngineRailKind(const Engine *e) noexcept { return e->VehInfo<RailVehicleInfo>().railveh_type; }
inline uint64_t RustEngineRailTypes(const Engine *e) noexcept { return e->VehInfo<RailVehicleInfo>().railtypes.base(); }
inline uint64_t RustEngineRailTypes(const OpenTTDEngine *e) noexcept { return RustEngineRailTypes(EngineOf(e)); }
inline RoadType RustEngineRoadType(const Engine *e) noexcept { return e->VehInfo<RoadVehicleInfo>().roadtype; }
inline RoadType RustEngineRoadType(const OpenTTDEngine *e) noexcept { return RustEngineRoadType(EngineOf(e)); }
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
inline uint16_t RustGroundEnginePower(OpenTTDVehicle *shell) noexcept { const Vehicle *v = VehicleOf(shell); return v->GetType() == VEH_TRAIN ? RustRailEnginePower(RailVehInfo(v->GetEngineType())) : RoadVehInfo(v->GetEngineType())->power; }
inline uint16_t RustEngineWagonPower(uint16_t engine) noexcept { return RailVehInfo(EngineID(engine))->pow_wag_power; }
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
