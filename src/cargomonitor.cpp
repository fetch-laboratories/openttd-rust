/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargomonitor.cpp Implementation of the cargo transport monitoring. */

#include "stdafx.h"
#include "cargomonitor.h"
#include "station_base.h"

#include "safeguards.h"

#include "rust/cargomonitor_ffi.h"
#include "rust/services_station.h"
#include "rust/services_assert.h"
#include "rust/town_services.h"
const OpenTTDCargoMonitorServices &GetRustCargoMonitorServices() noexcept
{
	static const OpenTTDCargoMonitorServices services = {
		.station_town = RustStationTownID,
		.check = RustAssertCondition,
	};
	return services;
}
void ClearCargoPickupMonitoring(CompanyID company) { openttd_rust_monitor_clear_pickups(company.base()); }
void ClearCargoDeliveryMonitoring(CompanyID company) { openttd_rust_monitor_clear_deliveries(company.base()); }
int32_t GetPickupAmount(CargoMonitorID monitor, bool keep) { return openttd_rust_monitor_pickup(monitor, keep); }
int32_t GetDeliveryAmount(CargoMonitorID monitor, bool keep) { return openttd_rust_monitor_delivery(monitor, keep); }
#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCargoMonitorServices.hpp"
bool CheckAbi_cargomonitor() { return AbiServicesComplete(GetRustCargoMonitorServices()); }
