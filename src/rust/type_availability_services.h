/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_availability_services.h Canonical native type-registry observations. */
#ifndef RUST_TYPE_AVAILABILITY_SERVICES_H
#define RUST_TYPE_AVAILABILITY_SERVICES_H
#include "type_availability_ffi.h"
#include "../rail.h"
#include "../road.h"
inline const RailTypeLabel &RustRailTypeLabel(const RailTypeInfo *rti) noexcept { return rti->label; }
inline const RoadTypeLabel &RustRoadTypeLabel(const RoadTypeInfo *rti) noexcept { return rti->label; }
inline bool RustRailTypeHidden(const RailTypeInfo *rti) noexcept { return rti->flags.Test(RailTypeFlag::Hidden); }
inline bool RustRoadTypeHidden(const RoadTypeInfo *rti) noexcept { return rti->flags.Test(RoadTypeFlag::Hidden); }
inline uint64_t RustRailTypeIntroduces(const RailTypeInfo *rti) noexcept { return rti->introduces_railtypes.base(); }
inline uint64_t RustRoadTypeIntroduces(const RoadTypeInfo *rti) noexcept { return rti->introduces_roadtypes.base(); }
inline int32_t RustRoadTypeIntroductionDate(const RoadTypeInfo *rti) noexcept { return rti->introduction_date.base(); }
inline bool RustRoadTypeTownBuild(const RoadTypeInfo *rti) noexcept { return rti->flags.Test(RoadTypeFlag::TownBuild); }
inline OpenTTDTypeIntroduction RustRailTypeIntroduction(RailType rt) noexcept
{
	const RailTypeInfo *rti = GetRailTypeInfo(rt);
	return {.label = RustRailTypeLabel(rti), .date = rti->introduction_date.base(), .required = rti->introduction_required_railtypes.base(), .introduces = RustRailTypeIntroduces(rti)};
}
inline OpenTTDTypeIntroduction RustRoadTypeIntroduction(RoadType rt) noexcept
{
	const RoadTypeInfo *rti = GetRoadTypeInfo(rt);
	return {.label = RustRoadTypeLabel(rti), .date = RustRoadTypeIntroductionDate(rti), .required = rti->introduction_required_roadtypes.base(), .introduces = RustRoadTypeIntroduces(rti)};
}
inline OpenTTDRoadTypeConstruction RustRoadTypeConstruction(RoadType rt) noexcept
{
	const RoadTypeInfo *rti = GetRoadTypeInfo(rt);
	return {.label = RustRoadTypeLabel(rti), .date = RustRoadTypeIntroductionDate(rti), .hidden = RustRoadTypeHidden(rti), .town_build = RustRoadTypeTownBuild(rti)};
}
#endif
