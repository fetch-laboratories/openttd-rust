/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_spritegroup.cpp Handling of primarily NewGRF action 2. */

#include "stdafx.h"
#include "debug.h"
#include "newgrf_spritegroup.h"
#include "newgrf_profiling.h"
#include "core/pool_func.hpp"
#include "rust/newgrf_spritegroup_ffi.h"
#include "rust/newgrf_storage_ffi.h"

#include "safeguards.h"

SpriteGroupPool _spritegroup_pool("SpriteGroup");
INSTANTIATE_POOL_METHODS(SpriteGroup)


/**
 * Process registers and the construction stage into the sprite layout.
 * The passed construction stage might get reset to zero, if it gets incorporated into the layout
 * during the preprocessing.
 * @param object ResolverObject owning the temporary storage.
 * @param[in,out] stage Construction stage (0-3), or nullptr if not applicable.
 * @return sprite layout to draw.
 */
SpriteLayoutProcessor TileLayoutSpriteGroup::ProcessRegisters(const ResolverObject &object, uint8_t *stage) const
{
	if (!this->dts.NeedsPreprocessing()) {
		if (stage != nullptr && this->dts.consistent_max_offset > 0) *stage = GetConstructionStageOffset(*stage, this->dts.consistent_max_offset);
		return SpriteLayoutProcessor(this->dts);
	}

	uint8_t actual_stage = stage != nullptr ? *stage : 0;
	SpriteLayoutProcessor result(this->dts, 0, 0, 0, actual_stage, false);
	result.ProcessRegisters(object, 0, 0);

	/* Stage has been processed by PrepareLayout(), set it to zero. */
	if (stage != nullptr) *stage = 0;

	return result;
}
