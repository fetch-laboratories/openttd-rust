/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tgp.cpp OTTD Perlin Noise Landscape Generator, aka TerraGenesis Perlin */

#include "stdafx.h"
#include "clear_map.h"
#include "void_map.h"
#include "genworld.h"
#include "core/random_func.hpp"
#include "landscape_type.h"

#include "safeguards.h"

#include "rust/tgp_ffi.h"

OpenTTDTGPSettings RustTGPSettings() noexcept
{
	return {
		.log_x = Map::LogX(),
		.log_y = Map::LogY(),
		.terrain = _settings_game.difficulty.terrain_type,
		.custom_height = _settings_game.game_creation.custom_terrain_type,
		.height_limit = _settings_game.construction.map_height_limit,
		.smoothness = _settings_game.game_creation.tgen_smoothness,
		.landscape = static_cast<uint32_t>(_settings_game.game_creation.landscape),
		.variety = _settings_game.game_creation.variety,
		.sea = _settings_game.difficulty.quantity_sea_lakes,
		.custom_sea = _settings_game.game_creation.custom_sea_level,
		.freeform = _settings_game.construction.freeform_edges,
		.borders = _settings_game.game_creation.water_borders.base(),
		.seed = _settings_game.game_creation.generation_seed,
		.creation_log_x = _settings_game.game_creation.map_x,
		.creation_log_y = _settings_game.game_creation.map_y,
	};
}
