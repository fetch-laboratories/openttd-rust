/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_act2.cpp NewGRF Action 0x02 handler. */

#include "../stdafx.h"
#include <ranges>
#include "../debug.h"
#include "../newgrf_engine.h"
#include "../newgrf_cargo.h"
#include "../error.h"
#include "../vehicle_base.h"
#include "../road.h"
#include "newgrf_bytereader.h"
#include "newgrf_internal.h"

#include "table/strings.h"

#include "../safeguards.h"

#include "../rust/newgrf_act2_services.inc"

void MapSpriteMappingRecolour(PalSpriteID *sprite)
{
	openttd_rust_action2_recolour(&GetRustAction2Services(), sprite);
}
