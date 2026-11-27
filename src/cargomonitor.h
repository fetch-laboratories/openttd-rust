/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargomonitor.h Cargo transport monitoring declarations. */

#ifndef CARGOMONITOR_H
#define CARGOMONITOR_H

#include "cargo_type.h"
#include "company_func.h"
#include "industry.h"
#include "town.h"
#include "core/overflowsafe_type.hpp"

struct Station;

/**
 * Unique number for a company / cargo type / (town or industry).
 * Encoding is as follows:
 * - bits 0-15 town or industry number
 * - bit 16 is set if it is an industry number (else it is a town number).
 * - bits 19-23 Cargo type.
 * - bits 24-31 %Company number.
 */
typedef uint32_t CargoMonitorID; ///< Type of the cargo monitor number.

/** Map type for storing and updating active cargo monitor numbers and their amounts. */
#include "rust/cargomonitor_ffi.h"


/* The encoding constants and their static assertions are cargomonitor.rs's. */


/**
 * Encode a cargo monitor for pickup or delivery at an industry.
 * @param company Company performing the transport.
 * @param ctype Cargo type being transported.
 * @param ind %Industry providing or accepting the cargo.
 * @return The encoded cargo/company/industry number.
 */
inline CargoMonitorID EncodeCargoIndustryMonitor(CompanyID company, CargoType ctype, IndustryID ind)
{
	return openttd_rust_monitor_encode_industry(company.base(), ctype, ind.base());
}

/**
 * Encode a cargo monitoring number for pickup or delivery at a town.
 * @param company %Company performing the transport.
 * @param ctype Cargo type being transported.
 * @param town %Town providing or accepting the cargo.
 * @return The encoded cargo/company/town number.
 */
inline CargoMonitorID EncodeCargoTownMonitor(CompanyID company, CargoType ctype, TownID town)
{
	return openttd_rust_monitor_encode_town(company.base(), ctype, town.base());
}

/**
 * Extract the company from the cargo monitor.
 * @param num Cargo monitoring number to decode.
 * @return The extracted company id.
 */
inline CompanyID DecodeMonitorCompany(CargoMonitorID num)
{
	return CompanyID(openttd_rust_monitor_company(num));
}

/**
 * Extract the cargo type from the cargo monitor.
 * @param num Cargo monitoring number to decode.
 * @return The extracted cargo type.
 */
inline CargoType DecodeMonitorCargoType(CargoMonitorID num)
{
	return openttd_rust_monitor_cargo(num);
}

/**
 * Does the cargo number monitor an industry or a town?
 * @param num Cargo monitoring number to decode.
 * @return true if monitoring an industry, false if monitoring a town.
 */

/**
 * Extract the industry number from the cargo monitor.
 * @param num Cargo monitoring number to decode.
 * @return The extracted industry id, or #IndustryID::Invalid() if the number does not monitor an industry.
 */
inline IndustryID DecodeMonitorIndustry(CargoMonitorID num)
{
	return IndustryID(openttd_rust_monitor_industry(num));
}

/**
 * Extract the town number from the cargo monitor.
 * @param num Cargo monitoring number to decode.
 * @return The extracted town id, or #TownID::Invalid() if the number does not monitor a town.
 */
inline TownID DecodeMonitorTown(CargoMonitorID num)
{
	return TownID(openttd_rust_monitor_town(num));
}

void ClearCargoPickupMonitoring(CompanyID company = INVALID_OWNER);
void ClearCargoDeliveryMonitoring(CompanyID company = INVALID_OWNER);
int32_t GetDeliveryAmount(CargoMonitorID monitor, bool keep_monitoring);
int32_t GetPickupAmount(CargoMonitorID monitor, bool keep_monitoring);

#endif /* CARGOMONITOR_H */
