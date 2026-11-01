/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_internal_vehicle.h NewGRF internal processing state for vehicles. */

#ifndef NEWGRF_INTERNAL_VEHICLE_H
#define NEWGRF_INTERNAL_VEHICLE_H

#include "../newgrf.h"
#include "../engine_base.h"
#include "../vehicle_func.h"
#include "newgrf_internal.h"


Engine *GetNewEngine(const GRFFile *file, VehicleType type, uint16_t internal_id, bool static_access = false);

/**
 * Helper to check whether an image index is valid for a particular NewGRF vehicle.
 * @tparam T The type of vehicle.
 * @param image_index The image index to check.
 * @return True iff the image index is valid, or CUSTOM_VEHICLE_SPRITENUM (use new graphics).
 */
template <VehicleType T>
static inline bool IsValidNewGRFImageIndex(uint8_t image_index) noexcept
{
	return image_index == CUSTOM_VEHICLE_SPRITENUM || IsValidImageIndex<T>(image_index);
}


#endif /* NEWGRF_INTERNAL_VEHICLE_H */
