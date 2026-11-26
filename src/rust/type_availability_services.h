/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file type_availability_services.h Canonical native type-registry observations. */
#ifndef RUST_TYPE_AVAILABILITY_SERVICES_H
#define RUST_TYPE_AVAILABILITY_SERVICES_H
#include "type_availability_ffi.h"
#include "../rail.h"
#include "../road.h"
inline int32_t RustRoadTypeIntroductionDate(const RoadTypeInfo *rti) noexcept { return rti->introduction_date.base(); }
inline bool RustRoadTypeTownBuild(const RoadTypeInfo *rti) noexcept { return rti->flags.Test(RoadTypeFlag::TownBuild); }
#endif
