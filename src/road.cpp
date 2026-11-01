/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file road.cpp Generic road related functions. */

#include "stdafx.h"
#include "rail_map.h"
#include "road_map.h"
#include "water_map.h"
#include "genworld.h"
#include "company_func.h"
#include "company_base.h"
#include "engine_base.h"
#include "timer/timer_game_calendar.h"
#include "landscape.h"
#include "road.h"
#include "road_func.h"
#include "roadveh.h"

#include "rust/type_availability_ffi.h"

#include "rust/type_registry_ffi.h"

#include "safeguards.h"

/**
 * Get the RoadType for this RoadTypeInfo.
 * @return RoadType in static RoadTypeInfo definitions.
 */
RoadType RoadTypeInfo::Index() const
{
	extern RoadTypeInfo _roadtypes[ROADTYPE_END];
	size_t index = this - _roadtypes;
	assert(index < ROADTYPE_END);
	return static_cast<RoadType>(index);
}

/**
 * Finds out, whether given company has a given RoadType available for construction.
 * @param company ID of company
 * @param roadtypet RoadType to test
 * @return true if company has the requested RoadType available
 */
bool HasRoadTypeAvail(const CompanyID company, RoadType roadtype)
{
	return openttd_rust_has_road_type(&GetRustSharedServices(), &GetRustTypeAvailabilityServices(), company.base(), roadtype) != 0;
}

/**
 * Test if any buildable RoadType is available for a company.
 * @param company the company in question
 * @return true if company has any RoadTypes available
 */
bool HasAnyRoadTypesAvail(CompanyID company, RoadTramType rtt)
{
	return openttd_rust_has_any_road_types(&GetRustTypeAvailabilityServices(), company.base(), rtt);
}

/**
 * Validate functions for rail building.
 * @param roadtype road type to check.
 * @return true if the current company may build the road.
 */
bool ValParamRoadType(RoadType roadtype)
{
	return openttd_rust_val_road_type(&GetRustSharedServices(), roadtype) != 0;
}

/**
 * Add the road types that are to be introduced at the given date.
 * @param rt      Roadtype
 * @param current The currently available roadtypes.
 * @param date    The date for the introduction comparisons.
 * @return The road types that should be available when date
 *         introduced road types are taken into account as well.
 */
RoadTypes AddDateIntroducedRoadTypes(RoadTypes current, TimerGameCalendar::Date date)
{
	return RoadTypes{openttd_rust_add_date_road_types(&GetRustTypeAvailabilityServices(), current.base(), date.base())};
}

/**
 * Get the road types the given company can build.
 * @param company the company to get the road types for.
 * @param introduces If true, include road types introduced by other road types
 * @return the road types.
 */
RoadTypes GetCompanyRoadTypes(CompanyID company, bool introduces)
{
	return RoadTypes{openttd_rust_company_road_types( &GetRustTypeAvailabilityServices(), company.base(), introduces)};
}

/**
 * Get list of road types, regardless of company availability.
 * @param introduces If true, include road types introduced by other road types
 * @return the road types.
 */
RoadTypes GetRoadTypes(bool introduces)
{
	return RoadTypes{openttd_rust_road_types( &GetRustTypeAvailabilityServices(), introduces)};
}

/**
 * Get the road type for a given label.
 * @param label the roadtype label.
 * @param allow_alternate_labels Search in the alternate label lists as well.
 * @return the roadtype.
 */
RoadType GetRoadTypeByLabel(RoadTypeLabel label, bool allow_alternate_labels)
{
	return openttd_rust_road_by_label(label, allow_alternate_labels);
}
