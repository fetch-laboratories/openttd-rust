/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_capabilities_services.hpp Native specs, pooled TEMP shell lifetime and shared services only. */
#include "engine_services.h"
#include "services_cargo.h"
#include "services_assert.h"
static OpenTTDVehicle *EngineTemporaryNew() noexcept { return RustVehicle(new Vehicle()); }
static void EngineTemporaryDelete(OpenTTDVehicle *v) noexcept { delete VehicleOf(v); }
static void EngineTemporarySetEngine(OpenTTDVehicle *v, uint16_t engine) noexcept { VehicleOf(v)->engine_type = EngineID(engine); }
static void EngineTemporarySetOwner(OpenTTDVehicle *v, uint8_t owner) noexcept { VehicleOf(v)->owner = Owner(owner); }
const OpenTTDEngineCapabilitiesServices &GetRustEngineCapabilitiesServices() noexcept
{
	static const OpenTTDEngineCapabilitiesServices services{
		.resolver = []() noexcept { return &GetRustVehicleResolverServices(); },
		.engine = RustEngineByID,
		.engine_if_valid = RustEngineByIDIfValid,
		.kind = RustEngineType,
		.id = RustEngineID,
		.default_cargo = RustEngineDefaultCargo,
		.callbacks = RustEngineCallbackMask,
		.misc = RustEngineMiscFlags,
		.refit_mask = RustEngineCargoRefitMask,
		.rail_capacity = RustEngineRailCapacity,
		.road_capacity = RustEngineRoadCapacity,
		.ship_capacity = RustEngineShipCapacity,
		.passenger_capacity = RustEnginePassengerCapacity,
		.mail_capacity = RustEngineMailCapacity,
		.rail_kind = Engine_rail_kind,
		.grf = RustEngineGRF,
		.version = RustGRFFileVersion,
		.new_engine = RustNewEngineID,
		.cargo_classes = RustCargoClasses,
		.multiplier = RustCargoMultiplier,
		.cargo_by_label = OpenTTDCargoByLabel,
		.pool_available = RustVehicleCanAllocate,
		.new_temporary = EngineTemporaryNew,
		.destroy_temporary = EngineTemporaryDelete,
		.set_engine = EngineTemporarySetEngine,
		.set_owner = EngineTemporarySetOwner,
		.current_company = OpenTTDCurrentCompany,
		.vehicle_engine = RustVehicleRead<uint16_t, RustVehicleEngineID>,
		.vehicle_cargo = RustVehicleCargoType,
		.vehicle_subtype = RustVehicleRead<uint8_t, RustVehicleCargoSubtype>,
		.has_articulated = RustVehicleHasArticulated,
		.next_articulated = RustVehicleNextArticulated,
		.check = RustAssertCondition,
#if !defined(NDEBUG) || defined(WITH_ASSERT)
		.assertions = true,
#else
		.assertions = false,
#endif
	};
	return services;
}
#include "service_table_check.hpp"
#include "abi-visit-OpenTTDEngineCapabilitiesServices.hpp"
bool CheckAbi_engine_capabilities() { return AbiServicesComplete(GetRustEngineCapabilitiesServices()); }

static_assert(CBID_VEHICLE_REFIT_CAPACITY == 0x15 && CBID_VEHICLE_ARTIC_ENGINE == 0x16);
static_assert(to_underlying(VehicleCallbackMask::RefitCapacity) == 3 && to_underlying(VehicleCallbackMask::ArticEngine) == 4 && to_underlying(VehicleCallbackMask::CargoSuffix) == 5);
static_assert(to_underlying(EngineMiscFlag::NoDefaultCargoMultiplier) == 5 && to_underlying(CargoClass::Passengers) == 0);
static_assert(PROP_TRAIN_CARGO_CAPACITY == 0x14 && PROP_ROADVEH_CARGO_CAPACITY == 0x0f && PROP_SHIP_CARGO_CAPACITY == 0x0d);
static_assert(PROP_AIRCRAFT_PASSENGER_CAPACITY == 0x0f && PROP_AIRCRAFT_MAIL_CAPACITY == 0x11 && ALL_CARGOTYPES == UINT64_MAX);
