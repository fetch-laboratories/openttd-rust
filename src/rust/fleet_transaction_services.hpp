/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "command_cost_services.h"
#include "engine_services.h"
#include "services_ffi.h"
#include "shared_services_table.h"
#include "vehicle_refit_ffi.h"
#include "vehicle_services.h"

/* The tables of the Rust autoreplace bodies and the native bodies they call that are not
 * ported. Every callback is noexcept and holds no Rust state across its call. */
static void FleetTransaction_change_vehicle_news(uint32_t from_index, uint32_t to_index) noexcept { ChangeVehicleNews(VehicleID(from_index), VehicleID(to_index)); }
static void FleetTransaction_change_vehicle_view_window(uint32_t from_index, uint32_t to_index) noexcept { ChangeVehicleViewWindow(VehicleID(from_index), VehicleID(to_index)); }
static void FleetTransaction_change_vehicle_viewports(uint32_t from_index, uint32_t to_index) noexcept { ChangeVehicleViewports(VehicleID(from_index), VehicleID(to_index)); }
static void FleetTransaction_copy_vehicle_config_and_statistics(OpenTTDVehicle *v, OpenTTDVehicle *src) noexcept { VehicleOf(v)->CopyVehicleConfigAndStatistics(VehicleOf(src)); }
static uint8_t FleetTransaction_get_best_fitting_sub_type(OpenTTDVehicle *v_from, OpenTTDVehicle *v_for, uint8_t dest_cargo_type) noexcept { return GetBestFittingSubType(VehicleOf(v_from), VehicleOf(v_for), CargoType(dest_cargo_type)); }
static void FleetTransaction_invalidate_autoreplace_window(uint16_t e, uint16_t id_g) noexcept { InvalidateAutoreplaceWindow(EngineID(e), GroupID(id_g)); }
static const OpenTTDFleetTransactionServices _fleet_transaction_services{
	.build = GetRustVehicleBuildServices,
	.capabilities = &GetRustEngineCapabilitiesServices(),
	.commands = &GetRustCommandServices(),
	.vehicles = &GetRustVehicleServices(),
	.change_vehicle_news = FleetTransaction_change_vehicle_news,
	.change_vehicle_view_window = FleetTransaction_change_vehicle_view_window,
	.change_vehicle_viewports = FleetTransaction_change_vehicle_viewports,
	.copy_vehicle_config_and_statistics = FleetTransaction_copy_vehicle_config_and_statistics,
	.get_best_fitting_sub_type = FleetTransaction_get_best_fitting_sub_type,
	.invalidate_autoreplace_window = FleetTransaction_invalidate_autoreplace_window,
};
