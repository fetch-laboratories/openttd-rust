/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_availability_services.h Canonical native type-registry observations. */
#ifndef RUST_TYPE_AVAILABILITY_SERVICES_H
#define RUST_TYPE_AVAILABILITY_SERVICES_H
#include "type_availability_ffi.h"
#include "../rail.h"
#include "../road.h"
inline uint64_t RustRailTypeIntroduces(RailType rt) noexcept { return GetRailTypeInfo(rt)->introduces_railtypes.base(); }
inline uint64_t RustRoadTypeIntroduces(RoadType rt) noexcept { return GetRoadTypeInfo(rt)->introduces_roadtypes.base(); }
inline OpenTTDTypeIntroduction RustRailTypeIntroduction(RailType rt) noexcept
{
	const RailTypeInfo *rti = GetRailTypeInfo(rt);
	return {.label = rti->label, .date = rti->introduction_date.base(), .required = rti->introduction_required_railtypes.base(), .introduces = RustRailTypeIntroduces(rt)};
}
inline OpenTTDTypeIntroduction RustRoadTypeIntroduction(RoadType rt) noexcept
{
	const RoadTypeInfo *rti = GetRoadTypeInfo(rt);
	return {.label = rti->label, .date = rti->introduction_date.base(), .required = rti->introduction_required_roadtypes.base(), .introduces = RustRoadTypeIntroduces(rt)};
}
inline OpenTTDRoadTypeConstruction RustRoadTypeConstruction(RoadType rt) noexcept
{
	const RoadTypeInfo *rti = GetRoadTypeInfo(rt);
	return {.label = rti->label, .date = rti->introduction_date.base(), .hidden = rti->flags.Test(RoadTypeFlag::Hidden), .town_build = rti->flags.Test(RoadTypeFlag::TownBuild)};
}
inline uint64_t RustRailTypesHidden() noexcept { return _railtypes_hidden_mask.base(); }
inline uint64_t RustRoadTypesHidden() noexcept { return _roadtypes_hidden_mask.base(); }
inline uint64_t RustMaskForRoadTramType(RoadTramType rtt) noexcept { return GetMaskForRoadTramType(rtt).base(); }
#endif
