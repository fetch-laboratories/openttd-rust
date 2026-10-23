/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file water_area_services.h Shared native area-iterator stack storage. */
#ifndef RUST_WATER_AREA_SERVICES_H
#define RUST_WATER_AREA_SERVICES_H
#include "../tilearea_type.h"
#include <optional>
#include <variant>
struct OpenTTDWaterArea {
	std::optional<std::variant<OrthogonalTileIterator, DiagonalTileIterator>> iter;
};
#endif /* RUST_WATER_AREA_SERVICES_H */
