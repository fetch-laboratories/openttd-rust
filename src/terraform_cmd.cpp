/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file terraform_cmd.cpp Commands related to terraforming. */

#include "stdafx.h"
#include "command_func.h"
#include "tunnel_map.h"
#include "bridge_map.h"
#include "viewport_func.h"
#include "genworld.h"
#include "object_base.h"
#include "company_base.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "terraform_cmd.h"
#include "landscape_cmd.h"

#include "table/strings.h"

#include "rust/landscape_command_ffi.h"
#include "rust/water_area_services.h"
#include "rust/command_cost_services.h"
static_assert(LM_LEVEL == 0 && LM_LOWER == 1 && LM_RAISE == 2);

#include "safeguards.h"


/**
 * Terraform land
 * @param flags for this command type
 * @param tile tile to terraform
 * @param slope corners to terraform (SLOPE_xxx)
 * @param dir_up direction; eg up (true) or down (false)
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, Money, TileIndex> CmdTerraformLand(DoCommandFlags flags, TileIndex tile, Slope slope, bool dir_up)
{
	CommandCost result;
	TileIndex error{openttd_rust_landscape_terraform(&GetRustLandscapeCommandServices(), RustCommandCost(&result), tile.base(), slope, dir_up, flags.base())};
	return {std::move(result), 0, error};
}


/**
 * Levels a selected (rectangle) area of land
 * @param flags for this command type
 * @param tile end tile of area-drag
 * @param start_tile start tile of area drag
 * @param diagonal Whether to use the Orthogonal (false) or Diagonal (true) iterator.
 * @param LevelMode Mode of leveling \c LevelMode.
 * @return the cost of this operation or an error
 */
std::tuple<CommandCost, Money, TileIndex> CmdLevelLand(DoCommandFlags flags, TileIndex tile, TileIndex start_tile, bool diagonal, LevelMode lm)
{
	CommandCost result, last, ret;
	OpenTTDWaterArea area;
	int64_t extra = 0;
	TileIndex error{openttd_rust_landscape_level(&GetRustLandscapeCommandServices(), RustCommandCost(&result), RustCommandCost(&last), RustCommandCost(&ret), tile.base(), start_tile.base(), diagonal, lm, flags.base(), &area, &extra)};
	return {std::move(result), Money(extra), error};
}
