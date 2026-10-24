/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_refit_services.hpp Native refit observations, scalar stores and UI leaves. */
#include "services_road.h"
#include "services_cargo.h"

static OpenTTDRefitCargo VehicleRefitCargo(const OpenTTDEngine *e, uint8_t cargo) noexcept { return {.classes = RustCargoClasses(cargo), .local = RustGRFCargoMap(EngineOf(e)->GetGRF(), cargo)}; }
static OpenTTDRefitEngine VehicleRefitEngine(const OpenTTDEngine *e) noexcept { return {.mask = RustEngineCargoRefitMask(e), .automatic = EngineOf(e)->info.misc_flags.Test(EngineMiscFlag::AutoRefit)}; }
static OpenTTDRefitCargoKind VehicleRefitCargoKind(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.cargo = RustVehicleCargoType(handle), .subtype = RustVehicleCargoSubtype(v)};
}
static void VehicleRefitWrite(OpenTTDVehicle *handle, uint8_t cargo, uint16_t cap, uint8_t subtype) noexcept
{
	RustVehicleSetCargoType(handle, cargo);
	RustVehicleSetCargoCap(handle, cap);
	RustVehicleSetCargoSubtype(handle, subtype);
}
static OpenTTDRoadState *VehicleRefitRoadState(OpenTTDVehicle *v) noexcept { return RoadStateOf(RoadVehicle::From(VehicleOf(v))); }
static void VehicleRefitDetailsUI(OpenTTDVehicle *v) noexcept { InvalidateWindowData(WC_VEHICLE_DETAILS, VehicleOf(v)->index); }
static void VehicleRefitListsUI(uint8_t type) noexcept { InvalidateWindowClassesData(GetWindowClassForVehicleType(static_cast<VehicleType>(type)), 0); }
const OpenTTDVehicleRefitServices &GetRustVehicleRefitServices() noexcept
{
	static const OpenTTDVehicleRefitServices services{
		.common = &GetRustVehicleCommonServices(),
		.selection = &GetRustVehicleSelectionServices(),
		.cargo = CargoStorageServices(),
		.scheduler = &GetRustVehicleSchedulerServices(),
		.train = &GetTrainServices(),
		.road = &GetRustRoadLeaves(),
		.aircraft = &GetRustAircraftLeaves(),
		.ship = &GetRustShipServices(),
		.road_ground = &GetRustGroundServices<RoadVehicle>(),
		.prices = {
			.train = PR_BUILD_VEHICLE_TRAIN, .wagon = PR_BUILD_VEHICLE_WAGON,
			.road = PR_BUILD_VEHICLE_ROAD, .ship = PR_BUILD_VEHICLE_SHIP, .aircraft = PR_BUILD_VEHICLE_AIRCRAFT,
			.train_run = EXPENSES_TRAIN_RUN, .road_run = EXPENSES_ROADVEH_RUN,
			.ship_run = EXPENSES_SHIP_RUN, .aircraft_run = EXPENSES_AIRCRAFT_RUN,
			.auto_refit = CCF_AUTOREFIT.base(), .refit = CCF_REFIT.base(),
		},
		.engine = RustEngineByID,
		.vehicle_engine = RustVehicleGetEngine,
		.has_grf = RustEngineHasGRF,
		.refit_cost = RustEngineRefitCost,
		.refit_cargo = VehicleRefitCargo,
		.engine_kind = RustEngineType,
		.rail_kind = Engine_rail_kind,
		.price_shift = RustEnginePriceShift,
		.capabilities = &GetRustEngineCapabilitiesServices(),
		.refit_engine = VehicleRefitEngine,
		.cargo_kind = VehicleRefitCargoKind,
		.set_cargo_kind = RustVehicleSetCargoKind,
		.expense = RustVehicleExpense,
		.invalidate_chain = RustVehicleInvalidateChain,
		.commit = VehicleRefitCommit,
		.set_refit_cap = RustVehicleSetRefitCap,
		.refit_cap = RustVehicleRefitCap,
		.cargo_cap = RustVehicleCargoCap,
		.set_capacity = RustVehicleSetCargoCap,
		.write_refit = VehicleRefitWrite,
		.mail_cargo = OpenTTDMailCargo,
		.train_handle = RustVehicleTrain,
		.road_state = VehicleRefitRoadState,
		.aircraft_handle = GetRustAircraftVehicle,
		.road_acceleration = RustRoadAccelerationModel,
		.details_ui = VehicleRefitDetailsUI,
		.lists_ui = VehicleRefitListsUI,
	};
	return services;
}
