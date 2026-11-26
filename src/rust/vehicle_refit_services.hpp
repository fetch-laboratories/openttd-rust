/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle_refit_services.hpp Native refit observations, scalar stores and UI leaves. */
#include "services_road.h"
#include "services_cargo.h"

static OpenTTDRefitCargoKind VehicleRefitCargoKind(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.cargo = RustVehicleCargoType(handle), .subtype = RustVehicleCargoSubtype(v)};
}
const OpenTTDVehicleRefitServices &GetRustVehicleRefitServices() noexcept
{
	static const OpenTTDVehicleRefitServices services{
		.common = &GetRustVehicleCommonServices(),
		.selection = &GetRustVehicleSelectionServices(),
		.cargo = CargoStorageServices(),
		.train = &GetTrainServices(),
		.road = &GetRustRoadLeaves(),
		.aircraft = &GetRustAircraftLeaves(),
		.ship = &GetRustShipServices(),
		.road_ground = &_rust_ground_services,
		.engine = RustEngineByID,
		.capabilities = &GetRustEngineCapabilitiesServices(),
		.cargo_kind = VehicleRefitCargoKind,
		.set_cargo_kind = RustVehicleSetCargoKind,
		.expense = RustVehicleExpense,
		.invalidate_chain = RustVehicleInvalidateChain,
		.aircraft_handle = GetRustAircraftVehicle,
	};
	return services;
}

static_assert(to_underlying(EngineMiscFlag::AutoRefit) == 4);
