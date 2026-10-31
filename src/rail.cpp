/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail.cpp Implementation of rail specific functions. */

#include "stdafx.h"
#include "station_map.h"
#include "tunnelbridge_map.h"
#include "timer/timer_game_calendar.h"
#include "company_func.h"
#include "company_base.h"
#include "engine_base.h"

#include "table/track_data.h"

#include "rust/type_availability_ffi.h"

#include "rust/type_registry_ffi.h"

#include "safeguards.h"

/**
 * Get the RailType for this RailTypeInfo.
 * @return RailType in static RailTypeInfo definitions.
 */
RailType RailTypeInfo::Index() const
{
	extern RailTypeInfo _railtypes[RAILTYPE_END];
	size_t index = this - _railtypes;
	assert(index < RAILTYPE_END);
	return static_cast<RailType>(index);
}

/**
 * Return the rail type of tile, or INVALID_RAILTYPE if this is no rail tile.
 */
RailType GetTileRailType(Tile tile)
{
	switch (GetTileType(tile)) {
		case MP_RAILWAY:
			return GetRailType(tile);

		case MP_ROAD:
			/* rail/road crossing */
			if (IsLevelCrossing(tile)) return GetRailType(tile);
			break;

		case MP_STATION:
			if (HasStationRail(tile)) return GetRailType(tile);
			break;

		case MP_TUNNELBRIDGE:
			if (GetTunnelBridgeTransportType(tile) == TRANSPORT_RAIL) return GetRailType(tile);
			break;

		default:
			break;
	}
	return INVALID_RAILTYPE;
}

/**
 * Finds out if a company has a certain buildable railtype available.
 * @param company the company in question
 * @param railtype requested RailType
 * @return true if company has requested RailType available
 */
bool HasRailTypeAvail(const CompanyID company, const RailType railtype)
{
	return openttd_rust_has_rail_type(&GetRustTypeAvailabilityServices(), company.base(), railtype) != 0;
}

/**
 * Test if any buildable railtype is available for a company.
 * @param company the company in question
 * @return true if company has any RailTypes available
 */
bool HasAnyRailTypesAvail(const CompanyID company)
{
	return openttd_rust_has_any_rail_types(&GetRustTypeAvailabilityServices(), company.base());
}

/**
 * Validate functions for rail building.
 * @param rail the railtype to check.
 * @return true if the current company may build the rail.
 */
bool ValParamRailType(const RailType rail)
{
	return openttd_rust_val_rail_type(&GetRustSharedServices(), rail) != 0;
}

/**
 * Add the rail types that are to be introduced at the given date.
 * @param current The currently available railtypes.
 * @param date    The date for the introduction comparisons.
 * @return The rail types that should be available when date
 *         introduced rail types are taken into account as well.
 */
RailTypes AddDateIntroducedRailTypes(RailTypes current, TimerGameCalendar::Date date)
{
	return RailTypes{openttd_rust_add_date_rail_types(&GetRustTypeAvailabilityServices(), current.base(), date.base())};
}

/**
 * Get the rail types the given company can build.
 * @param company the company to get the rail types for.
 * @param introduces If true, include rail types introduced by other rail types
 * @return the rail types.
 */
RailTypes GetCompanyRailTypes(CompanyID company, bool introduces)
{
	return RailTypes{openttd_rust_company_rail_types( &GetRustTypeAvailabilityServices(), company.base(), introduces)};
}

/**
 * Get list of rail types, regardless of company availability.
 * @param introduces If true, include rail types introduced by other rail types
 * @return the rail types.
 */
RailTypes GetRailTypes(bool introduces)
{
	return RailTypes{openttd_rust_rail_types( &GetRustTypeAvailabilityServices(), introduces)};
}

/**
 * Get the rail type for a given label.
 * @param label the railtype label.
 * @param allow_alternate_labels Search in the alternate label lists as well.
 * @return the railtype.
 */
RailType GetRailTypeByLabel(RailTypeLabel label, bool allow_alternate_labels)
{
	return openttd_rust_rail_by_label(label, allow_alternate_labels);
}
