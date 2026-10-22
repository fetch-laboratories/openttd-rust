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
#include "vehicle_services.h"
#include "company_services.h"
#include "environment_services.h"
#include "../articulated_vehicles.h"
#include "../rail.h"
#include "../rail_map.h"
#include "../road.h"
inline const OpenTTDEngine *RustEngineByID(uint16_t id) noexcept { return RustEngine(Engine::Get(EngineID(id))); }
inline bool RustEngineHasArticulatedCallback(uint16_t engine) noexcept { return EngInfo(EngineID(engine))->callback_mask.Test(VehicleCallbackMask::ArticEngine); }
inline void RustArticulatedCapacity(uint16_t engine, uint32_t *out) noexcept { auto capacity = GetCapacityOfArticulatedParts(EngineID(engine)); std::copy(capacity.begin(), capacity.end(), out); }
inline int64_t RustEngineCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetCost().base(); }
inline uint8_t RustEngineDefaultCargo(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetDefaultCargoType(); }
inline VehicleType RustEngineType(const OpenTTDEngine *e) noexcept { return EngineOf(e)->type; }
inline bool RustEngineExclusivePreview(const OpenTTDEngine *e) noexcept { return EngineOf(e)->flags.Test(EngineFlag::ExclusivePreview); }
inline bool RustEngineHasGRF(const OpenTTDEngine *e) noexcept { return EngineOf(e)->GetGRF() != nullptr; }
inline uint8_t RustEngineRefitCost(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.refit_cost; }
inline int32_t RustEnginePriceShift(const OpenTTDEngine *e, uint32_t price) noexcept { return RustGRFPriceShift(EngineOf(e)->GetGRF(), price); }
inline uint32_t RustEngineDetermineCapacity(const OpenTTDEngine *e, OpenTTDVehicle *v, uint16_t *mail) noexcept { return EngineOf(e)->DetermineCapacity(VehicleOf(v), mail); }
template <typename Result, auto Read, typename Index = uint16_t>
static Result RustIndexedEngineRead(Index id) noexcept { return Read(RustEngineByID(static_cast<uint16_t>(id))); }
inline uint8_t RustEngineExtraFlags(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.extra_flags.base(); }
inline uint16_t RustEngineID(const OpenTTDEngine *e) noexcept { return EngineOf(e)->index.base(); }
inline int32_t RustEngineBaseIntro(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.base_intro.base(); }
inline bool RustEngineClimate(const OpenTTDEngine *e) noexcept { return EngineOf(e)->info.climates.Test(OpenTTDLandscape()); }
inline bool RustEngineCanCarry(const Engine *e) noexcept { return e->CanCarryCargo(); }
inline uint64_t RustEngineRefitMask(EngineID id, bool initial) noexcept { return GetUnionOfArticulatedRefitMasks(id, initial); }
inline RailVehicleTypes RustEngineRailKind(const Engine *e) noexcept { return e->VehInfo<RailVehicleInfo>().railveh_type; }
inline uint64_t RustEngineCompatibleRailTypes(const Engine *e) noexcept { return GetAllCompatibleRailTypes(e->VehInfo<RailVehicleInfo>().railtypes).base(); }
/** `v->unitnumber = Company::Get(v->owner)->freeunits[v->type].UseID(unit)`. */
inline void RustVehicleUseUnitNumber(Vehicle *v, uint16_t unit) noexcept { v->unitnumber = Company::Get(v->owner)->freeunits[v->type].UseID(unit); }
inline bool RustEngineCompatibleRail(const OpenTTDEngine *e, uint32_t tile) noexcept { return IsCompatibleRail(EngineOf(e)->VehInfo<RailVehicleInfo>().railtypes, GetRailType(TileIndex(tile))); }
inline bool RustEngineHasPowerOnRail(const OpenTTDEngine *e, uint32_t tile) noexcept { return HasPowerOnRail(EngineOf(e)->VehInfo<RailVehicleInfo>().railtypes, GetRailType(TileIndex(tile))); }
inline uint64_t RustEnginePoweredRoadTypes(const Engine *e) noexcept { return GetRoadTypeInfo(e->VehInfo<RoadVehicleInfo>().roadtype)->powered_roadtypes.base(); }
inline bool FleetTransaction_can_carry(void *shell) noexcept { return RustEngineCanCarry(static_cast<Vehicle *>(shell)->GetEngine()); }
inline uint64_t FleetTransaction_rail_compatible(uint16_t id) noexcept { return RustEngineCompatibleRailTypes(Engine::Get(EngineID{id})); }
inline uint64_t FleetTransaction_road_powered(uint16_t id) noexcept { return RustEnginePoweredRoadTypes(Engine::Get(EngineID{id})); }
inline bool FleetTransaction_wagon(uint16_t id) noexcept { return RustEngineRailKind(Engine::Get(EngineID{id})) == RAILVEH_WAGON; }
inline uint64_t FleetTransaction_refit_mask(uint16_t id, bool initial) noexcept { return RustEngineRefitMask(EngineID{id}, initial); }
inline bool Engine_can_carry(const OpenTTDEngine *e) noexcept { return RustEngineCanCarry(EngineOf(e)); }
inline uint64_t Engine_refit_mask(const OpenTTDEngine *e) noexcept { return RustEngineRefitMask(EngineOf(e)->index, true); }
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
	return v == nullptr ? OpenTTDEngineVehicle{nullptr, UINT32_MAX, 0, VEH_INVALID} : OpenTTDEngineVehicle{RustVehicle(v), v->index.base(), RustVehicleOwner(v), RustVehicleVehicleType(v)};
}
#endif /* RUST_ENGINE_SERVICES_H */
