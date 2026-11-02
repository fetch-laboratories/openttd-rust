/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file tree_cmd.cpp Handling of tree tiles. */

#include "stdafx.h"
#include "clear_map.h"
#include "landscape.h"
#include "tree_map.h"
#include "viewport_func.h"
#include "command_func.h"
#include "town.h"
#include "genworld.h"
#include "clear_func.h"
#include "company_func.h"
#include "sound_func.h"
#include "water.h"
#include "company_base.h"
#include "core/geometry_type.hpp"
#include "core/random_func.hpp"
#include "newgrf_generic.h"
#include "timer/timer_game_tick.h"
#include "tree_cmd.h"
#include "landscape_cmd.h"

#include "table/strings.h"
#include "table/tree_land.h"
#include "table/clear_land.h"

#include "rust/services_landscape.h"

#include "safeguards.h"

/** Where to place trees while in-game? */
enum ExtraTreePlacement : uint8_t {
	ETP_NO_SPREAD,           ///< Grow trees on tiles that have them but don't spread to new ones
	ETP_SPREAD_RAINFOREST,   ///< Grow trees on tiles that have them, only spread to new ones in rainforests
	ETP_SPREAD_ALL,          ///< Grow trees and spread them without restrictions
	ETP_NO_GROWTH_NO_SPREAD, ///< Don't grow trees and don't spread them at all
};

#include "rust/trees_ffi.h"
#include "rust/world_generation_ffi.h"
#include "rust/generation_services.hpp"
#include "rust/settings_services.h"
#include "rust/ground_services.h"
#include "rust/landscape_loop_ffi.h"
#include "rust/services_ffi.h"
#include "rust/services_map.h"
#include "rust/services_water.h"
#include <exception>
#include <utility>

static_assert(MP_CLEAR == 0 && MP_TREES == 4 && MP_WATER == 6);
static_assert(TREE_RAINFOREST == 20 && TREE_CACTUS == 27 && TREE_SUB_TROPICAL == 28 && TREE_TOYLAND == 32 && TREE_INVALID == 255 && TREE_COUNT_TEMPERATE == 12);
static_assert(to_underlying(LandscapeType::Temperate) == 0 && to_underlying(LandscapeType::Arctic) == 1 && to_underlying(LandscapeType::Tropic) == 2 && to_underlying(LandscapeType::Toyland) == 3);
static_assert(CLEAR_GRASS == 0 && CLEAR_ROUGH == 1 && CLEAR_ROCKS == 2 && CLEAR_FIELDS == 3 && CLEAR_DESERT == 5);
static_assert(TROPICZONE_NORMAL == 0 && TROPICZONE_DESERT == 1 && TROPICZONE_RAINFOREST == 2);
static_assert(TP_NONE == 0 && TP_ORIGINAL == 1 && TP_IMPROVED == 2);
static_assert(TREE_GROUND_GRASS == 0 && TREE_GROUND_ROUGH == 1 && TREE_GROUND_SNOW_DESERT == 2 && TREE_GROUND_SHORE == 3 && TREE_GROUND_ROUGH_SNOW == 4);
static_assert(to_underlying(TreeGrowthStage::Growing1) == 0 && to_underlying(TreeGrowthStage::Grown) == 3 && to_underlying(TreeGrowthStage::Dead) == 6);
static_assert(SND_42_RAINFOREST_1 == 66 && SND_43_RAINFOREST_2 == 67 && SND_44_RAINFOREST_3 == 68 && SND_48_RAINFOREST_4 == 72 && SND_34_ARCTIC_SNOW_1 == 52 && SND_39_ARCTIC_SNOW_2 == 57);
static_assert(MAP_HEIGHT_LIMIT_ORIGINAL == 15 && DIR_N == 0 && DIR_NW == 7);
static_assert(2 * (MAX_MAP_SIZE_BITS - MIN_MAP_SIZE_BITS) - 4 <= std::numeric_limits<uint8_t>::digits);

/** State of one CmdPlantTree or ClearTile_Trees call; Rust passes it back to the command services. */
struct OpenTTDTreeCommand {
	explicit OpenTTDTreeCommand(DoCommandFlags flags) : flags(flags) {}
	DoCommandFlags flags;
	Company *company = nullptr;
	CommandCost nested;
};

/** The AbortGenerateWorldSignal thrown by a progress update, held until the Rust entry returns. */
static std::exception_ptr _tree_generation_abort;

static OpenTTDTreePlantObservation TreePlantObservation(uint32_t index) noexcept
{
	TileIndex tile{index};
	OpenTTDTreePlantObservation o{};
	o.tile_type = SharedTileType(index);
	o.zone = GetTropicZone(tile);
	if (o.tile_type == MP_CLEAR) {
		o.bridge = IsBridgeAbove(tile);
		o.ground = RustClearGround(Tile{tile});
		o.density = static_cast<uint8_t>(RustClearDensity(index));
		o.snow = RustSnowTile(index);
	} else if (o.tile_type == MP_WATER) {
		o.bridge = IsBridgeAbove(tile);
		o.coast = RustIsCoast(index) && !IsSlopeWithOneCornerRaised(GetTileSlope(tile));
	}
	return o;
}

static OpenTTDTreeObservation TreeObservation(uint32_t index) noexcept
{
	TileIndex tile{index};
	return {
		.ground = RustTreeGround(Tile{tile}),
		.density = static_cast<uint8_t>(GetTreeDensity(tile)),
		.species = GetTreeType(tile),
		.count = static_cast<uint8_t>(GetTreeCount(tile)),
		.growth = to_underlying(GetTreeGrowth(tile)),
		.zone = GetTropicZone(tile),
	};
}

/** Run one progress update; on world-generation abort keep the exception and return true. */
template <typename Update>
static bool TreeProgress(Update update) noexcept
{
	try {
		update();
	} catch (...) {
		_tree_generation_abort = std::current_exception();
		return true;
	}
	return false;
}

static void TreeTownRating(uint16_t flags, uint32_t tile, bool up) noexcept
{
	Town *t = ClosestTownFromTile(TileIndex{tile}, _settings_game.economy.dist_local_authority);
	if (t == nullptr) return;
	if (up) {
		ChangeTownRating(t, RATING_TREE_UP_STEP, RATING_TREE_MAXIMUM, DoCommandFlags{flags});
	} else {
		ChangeTownRating(t, RATING_TREE_DOWN_STEP, RATING_TREE_MINIMUM, DoCommandFlags{flags});
	}
}

static int32_t TreeCommandBegin(OpenTTDTreeCommand *ctx) noexcept
{
	ctx->company = (_game_mode != GM_EDITOR) ? Company::GetIfValid(_current_company) : nullptr;
	int limit = (ctx->company == nullptr ? INT32_MAX : GB(ctx->company->Finances().tree_limit, 16, 16));
	return limit;
}

static const OpenTTDTreeServices _tree_services{
	.plant_observation = TreePlantObservation,
	.tree_observation = TreeObservation,
	.tree_count = [](uint32_t tile) noexcept { return GetTreeCount(TileIndex{tile}); },
	.clear_observation = [](uint32_t index) noexcept {
		TileIndex tile{index};
		return OpenTTDTreeClearObservation{.count = GetTreeCount(tile), .species = GetTreeType(tile)};
	},
	.sin = [](float v) noexcept { return sinf(v); },
	.cos = [](float v) noexcept { return cosf(v); },
	.make_tree = [](uint32_t tile, uint8_t type, uint32_t count, uint8_t growth, uint8_t ground, uint32_t density) noexcept {
		MakeTree(TileIndex{tile}, static_cast<TreeType>(type), count, static_cast<TreeGrowthStage>(growth), static_cast<TreeGround>(ground), density);
	},
	.set_ground_density = RustSetTreeGroundDensityValue,
	.add_count = [](uint32_t tile, int32_t count) noexcept { AddTreeCount(TileIndex{tile}, count); },
	.add_growth = [](uint32_t tile, int32_t growth) noexcept { AddTreeGrowth(TileIndex{tile}, growth); },
	.set_growth = [](uint32_t tile, uint8_t growth) noexcept { SetTreeGrowth(TileIndex{tile}, static_cast<TreeGrowthStage>(growth)); },
	.make_clear = RustMakeClearValue,
	.progress = []() noexcept { return TreeProgress([] { RustIncreaseGeneratingWorldProgress(GWP_TREE); }); },
	.progress_total = [](uint32_t total) noexcept { return TreeProgress([total] { RustSetGeneratingWorldProgress(GWP_TREE, total); }); },
	.town_rating = TreeTownRating,
	.command_begin = TreeCommandBegin,
	.command_debit = [](OpenTTDTreeCommand *ctx) noexcept { if (ctx->company != nullptr) ctx->company->Finances().tree_limit -= 1 << 16; },
};

/** Rethrow a world-generation abort after the Rust entry returned. */
static void RethrowTreeGenerationAbort(bool aborted)
{
	if (aborted) std::rethrow_exception(std::exchange(_tree_generation_abort, nullptr));
}

OpenTTDTreeGenerateSettings TreeGenerateSettings() noexcept
{
	return {
		.size = Map::Size(), .size_x = Map::SizeX(), .size_y = Map::SizeY(),
		.climate = to_underlying(_settings_game.game_creation.landscape),
		.placer = _settings_game.game_creation.tree_placer,
		.height_limit = RustMapHeightLimit(),
		.editor = _game_mode == GM_EDITOR,
		.freeform_edges = _settings_game.construction.freeform_edges,
	};
}

void PlaceTree(TileIndex tile, uint32_t r, bool keep_density)
{
	openttd_rust_trees_place(tile.base(), r, keep_density, to_underlying(_settings_game.game_creation.landscape), &_tree_services, &GetRustSharedServices());
}

void PlaceTreesRandomly()
{
	RethrowTreeGenerationAbort(openttd_rust_trees_place_randomly(TreeGenerateSettings(), &_tree_services, &GetRustSharedServices()));
}

std::exception_ptr TakeTreeGenerationException() noexcept
{
	return std::exchange(_tree_generation_abort, nullptr);
}

CommandCost CmdPlantTree(DoCommandFlags flags, TileIndex tile, TileIndex start_tile, uint8_t tree_to_plant, bool diagonal)
{
	OpenTTDTreeCommand ctx(flags);
	const OpenTTDTreeCommandSettings settings{
		.build_price = static_cast<int64_t>(_price[PR_BUILD_TREES]),
		.size = Map::Size(),
		.climate = to_underlying(_settings_game.game_creation.landscape),
		.editor = _game_mode == GM_EDITOR,
		.execute = flags.Test(DoCommandFlag::Execute),
		.company_valid = Company::IsValidID(_current_company),
	};
	auto result = openttd_rust_trees_cmd_plant(&ctx, tile.base(), start_tile.base(), tree_to_plant, diagonal, settings, &_tree_services, &GetRustSharedServices(), RustCommandCost(&ctx.nested), &GetRustCommandServices(), flags.base());
	if (result.status == 1) return CMD_ERROR;
	if (result.status == 2) return ctx.nested;
	static const StringID messages[] = {INVALID_STRING_ID, STR_ERROR_TREE_ALREADY_HERE, STR_ERROR_TREE_PLANT_LIMIT_REACHED,
		STR_ERROR_CAN_T_BUILD_ON_WATER, STR_ERROR_SITE_UNSUITABLE, STR_ERROR_TREE_WRONG_TERRAIN_FOR_TREE_TYPE};
	return result.cost == 0 ? CommandCost(messages[result.message]) : CommandCost(EXPENSES_OTHER, Money(result.cost));
}

const OpenTTDTreeServices &GetRustTreeServices() noexcept { return _tree_services; }

OpenTTDTreeLoopSettings GetRustTreeLoopSettings() noexcept
{
	return {
		.tick_counter = TimerGameTick::counter,
		.size_x = Map::SizeX(),
		.climate = to_underlying(_settings_game.game_creation.landscape),
		.extra = _settings_game.construction.extra_tree_placement,
		.ambient = _settings_client.sound.ambient,
		.ambient_callback = RustAmbientSoundEnabled(),
	};
}

OpenTTDTreeTickSettings RustTreeTickSettings() noexcept
{
	return {
		.tick_counter = TimerGameTick::counter,
		.size = Map::Size(),
		.climate = to_underlying(_settings_game.game_creation.landscape),
		.extra = _settings_game.construction.extra_tree_placement,
	};
}



void InitializeTrees() { openttd_rust_trees_initialize(); }


/**
 * Place some trees in a radius around a tile.
 * The trees are placed in an quasi-normal distribution around the indicated tile, meaning that while
 * the radius does define a square, the distribution inside the square will be roughly circular.
 * @note This function the interactive RNG and must only be used in editor and map generation.
 * @param tile      Tile to place trees around.
 * @param treetype  Type of trees to place. Must be a valid tree type for the climate.
 * @param radius    Maximum distance (on each axis) from tile to place trees.
 * @param count     Maximum number of trees to place.
 * @param set_zone  Whether to create a rainforest zone when placing rainforest trees.
 * @return Number of trees actually placed.
 */
uint PlaceTreeGroupAroundTile(TileIndex tile, TreeType treetype, uint radius, uint count, bool set_zone)
{
	return openttd_rust_trees_place_group(tile.base(), treetype, radius, count, set_zone, &_tree_services, &GetRustSharedServices(), &GetRustEnvironmentServices(), &GetRustWorldServices());
}


struct TreeListEnt : PalSpriteID {
	int8_t x, y;
};

static void DrawTile_Trees(TileInfo *ti)
{
	switch (GetTreeGround(ti->tile)) {
		case TREE_GROUND_SHORE: DrawShoreTile(ti->tileh); break;
		case TREE_GROUND_GRASS: DrawClearLandTile(ti, GetTreeDensity(ti->tile)); break;
		case TREE_GROUND_ROUGH: DrawHillyLandTile(ti); break;
		default: DrawGroundSprite(_clear_land_sprites_snow_desert[GetTreeDensity(ti->tile)] + SlopeToSpriteOffset(ti->tileh), PAL_NONE); break;
	}

	/* Do not draw trees when the invisible trees setting is set */
	if (IsInvisibilitySet(TO_TREES)) return;

	uint tmp = CountBits(ti->tile.base() + ti->x + ti->y);
	uint index = GB(tmp, 0, 2) + (GetTreeType(ti->tile) << 2);

	/* different tree styles above one of the grounds */
	if ((GetTreeGround(ti->tile) == TREE_GROUND_SNOW_DESERT || GetTreeGround(ti->tile) == TREE_GROUND_ROUGH_SNOW) &&
			GetTreeDensity(ti->tile) >= 2 &&
			IsInsideMM(index, TREE_SUB_ARCTIC << 2, TREE_RAINFOREST << 2)) {
		index += 164 - (TREE_SUB_ARCTIC << 2);
	}

	assert(index < lengthof(_tree_layout_sprite));

	const PalSpriteID *s = _tree_layout_sprite[index];
	const TreePos *d = _tree_layout_xy[GB(tmp, 2, 2)];

	/* combine trees into one sprite object */
	StartSpriteCombine();

	TreeListEnt te[4];

	/* put the trees to draw in a list */
	uint trees = GetTreeCount(ti->tile);

	for (uint i = 0; i < trees; i++) {
		SpriteID sprite = s[0].sprite + (i == trees - 1 ? to_underlying(GetTreeGrowth(ti->tile)) : 3);
		PaletteID pal = s[0].pal;

		te[i].sprite = sprite;
		te[i].pal    = pal;
		te[i].x = d->x;
		te[i].y = d->y;
		s++;
		d++;
	}

	/* draw them in a sorted way */
	int z = ti->z + GetSlopeMaxPixelZ(ti->tileh) / 2;

	for (; trees > 0; trees--) {
		uint min = te[0].x + te[0].y;
		uint mi = 0;

		for (uint i = 1; i < trees; i++) {
			if ((uint)(te[i].x + te[i].y) < min) {
				min = te[i].x + te[i].y;
				mi = i;
			}
		}

		SpriteBounds bounds{{}, {TILE_SIZE, TILE_SIZE, 48}, {te[mi].x, te[mi].y, 0}};
		AddSortableSpriteToDraw(te[mi].sprite, te[mi].pal, ti->x, ti->y, z, bounds, IsTransparencySet(TO_TREES));

		/* replace the removed one with the last one */
		te[mi] = te[trees - 1];
	}

	EndSpriteCombine();
}





static void GetTileDesc_Trees(TileIndex tile, TileDesc &td)
{
	TreeType tt = GetTreeType(tile);

	if (IsInsideMM(tt, TREE_RAINFOREST, TREE_CACTUS)) {
		td.str = STR_LAI_TREE_NAME_RAINFOREST;
	} else {
		td.str = tt == TREE_CACTUS ? STR_LAI_TREE_NAME_CACTUS_PLANTS : STR_LAI_TREE_NAME_TREES;
	}

	td.owner[0] = GetTileOwner(tile);
}



static void ChangeTileOwner_Trees(TileIndex, Owner, Owner)
{
	/* not used */
}




extern const TileTypeProcs _tile_type_trees_procs = {
	DrawTile_Trees,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	nullptr,                     // add_accepted_cargo_proc
	GetTileDesc_Trees,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                     // click_tile_proc
	nullptr,                 // Rust scheduler calls the owner directly.
	ChangeTileOwner_Trees,    // change_tile_owner_proc
	nullptr,                     // add_produced_cargo_proc
	nullptr,                     // vehicle_enter_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	nullptr, // check_build_above_proc
};

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDTreeServices.hpp"
bool CheckAbi_tree_cmd() { return AbiServicesComplete(_tree_services); }
