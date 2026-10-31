/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file clear_cmd.cpp Commands related to clear tiles. */

#include "stdafx.h"
#include "clear_map.h"
#include "command_func.h"
#include "landscape.h"
#include "genworld.h"
#include "viewport_func.h"
#include "core/random_func.hpp"
#include "newgrf_generic.h"
#include "landscape_cmd.h"

#include "table/strings.h"
#include "table/sprites.h"
#include "table/clear_land.h"

#include "rust/ground_ffi.h"
#include "rust/services_landscape.h"
#include "rust/environment_services.h"
#include "rust/command_services.h"

#include "safeguards.h"

static CommandCost ClearTile_Clear(TileIndex tile, DoCommandFlags flags)
{
	static constexpr Price clear_price_table[] = {
		PR_CLEAR_GRASS,
		PR_CLEAR_ROUGH,
		PR_CLEAR_ROCKS,
		PR_CLEAR_FIELDS,
		PR_CLEAR_ROUGH,
		PR_CLEAR_ROUGH,
	};
	CommandCost price(EXPENSES_CONSTRUCTION);

	ClearGround ground = GetClearGround(tile);
	uint8_t density = GetClearDensity(tile);
	if (IsSnowTile(tile)) {
		price.AddCost(_price[clear_price_table[ground]]);
		/* Add a little more for removing snow. */
		price.AddCost(std::abs(_price[PR_CLEAR_ROUGH] - _price[PR_CLEAR_GRASS]));
	} else if (ground != CLEAR_GRASS || density != 0) {
		price.AddCost(_price[clear_price_table[ground]]);
	}

	if (flags.Test(DoCommandFlag::Execute)) DoClearSquare(tile);

	return price;
}

void DrawClearLandTile(const TileInfo *ti, uint8_t set)
{
	DrawGroundSprite(SPR_FLAT_BARE_LAND + SlopeToSpriteOffset(ti->tileh) + set * 19, PAL_NONE);
}

void DrawHillyLandTile(const TileInfo *ti)
{
	if (ti->tileh != SLOPE_FLAT) {
		DrawGroundSprite(SPR_FLAT_ROUGH_LAND + SlopeToSpriteOffset(ti->tileh), PAL_NONE);
	} else {
		DrawGroundSprite(_landscape_clear_sprites_rough[GB(TileHash(ti->x, ti->y), 0, 3)], PAL_NONE);
	}
}

static void DrawClearLandFence(const TileInfo *ti)
{
	/* combine fences into one sprite object */
	StartSpriteCombine();

	SpriteBounds bounds{{}, {TILE_SIZE, TILE_SIZE, 4}, {}};

	bounds.extent.z += GetSlopeMaxPixelZ(ti->tileh);

	uint fence_nw = GetFence(ti->tile, DIAGDIR_NW);
	if (fence_nw != 0) {
		bounds.offset.x = 0;
		bounds.offset.y = -static_cast<int>(TILE_SIZE);
		bounds.offset.z = GetSlopePixelZInCorner(ti->tileh, CORNER_W);
		SpriteID sprite = _clear_land_fence_sprites[fence_nw - 1] + _fence_mod_by_tileh_nw[ti->tileh];
		AddSortableSpriteToDraw(sprite, PAL_NONE, *ti, bounds, false);
	}

	uint fence_ne = GetFence(ti->tile, DIAGDIR_NE);
	if (fence_ne != 0) {
		bounds.offset.x = -static_cast<int>(TILE_SIZE);
		bounds.offset.y = 0;
		bounds.offset.z = GetSlopePixelZInCorner(ti->tileh, CORNER_E);
		SpriteID sprite = _clear_land_fence_sprites[fence_ne - 1] + _fence_mod_by_tileh_ne[ti->tileh];
		AddSortableSpriteToDraw(sprite, PAL_NONE, *ti, bounds, false);
	}

	uint fence_sw = GetFence(ti->tile, DIAGDIR_SW);
	uint fence_se = GetFence(ti->tile, DIAGDIR_SE);

	if (fence_sw != 0 || fence_se != 0) {
		bounds.offset.x = 0;
		bounds.offset.y = 0;
		bounds.offset.z = GetSlopePixelZInCorner(ti->tileh, CORNER_S);

		if (fence_sw != 0) {
			SpriteID sprite = _clear_land_fence_sprites[fence_sw - 1] + _fence_mod_by_tileh_sw[ti->tileh];
			AddSortableSpriteToDraw(sprite, PAL_NONE, *ti, bounds, false);
		}

		if (fence_se != 0) {
			SpriteID sprite = _clear_land_fence_sprites[fence_se - 1] + _fence_mod_by_tileh_se[ti->tileh];
			AddSortableSpriteToDraw(sprite, PAL_NONE, *ti, bounds, false);

		}
	}
	EndSpriteCombine();
}

static void DrawTile_Clear(TileInfo *ti)
{
	ClearGround real_ground = GetClearGround(ti->tile);
	ClearGround ground = IsSnowTile(ti->tile) ? CLEAR_SNOW : real_ground;

	switch (ground) {
		case CLEAR_GRASS:
			DrawClearLandTile(ti, GetClearDensity(ti->tile));
			break;

		case CLEAR_ROUGH:
			DrawHillyLandTile(ti);
			break;

		case CLEAR_ROCKS:
			DrawGroundSprite((HasGrfMiscBit(GrfMiscBit::SecondRockyTileSet) && (TileHash(ti->x, ti->y) & 1) ? SPR_FLAT_ROCKY_LAND_2 : SPR_FLAT_ROCKY_LAND_1) + SlopeToSpriteOffset(ti->tileh), PAL_NONE);
			break;

		case CLEAR_FIELDS:
			DrawGroundSprite(_clear_land_sprites_farmland[GetFieldType(ti->tile)] + SlopeToSpriteOffset(ti->tileh), PAL_NONE);
			DrawClearLandFence(ti);
			break;

		case CLEAR_SNOW: {
			uint8_t density = GetClearDensity(ti->tile);
			DrawGroundSprite(_clear_land_sprites_snow_desert[density] + SlopeToSpriteOffset(ti->tileh), PAL_NONE);
			if (real_ground == CLEAR_ROCKS) {
				/* There 4 levels of snowy overlay rocks, each with 19 sprites. */
				++density;
				DrawGroundSprite(SPR_OVERLAY_ROCKS_BASE + (density * 19) + SlopeToSpriteOffset(ti->tileh), PAL_NONE);
			}
			break;
		}

		case CLEAR_DESERT:
			DrawGroundSprite(_clear_land_sprites_snow_desert[GetClearDensity(ti->tile)] + SlopeToSpriteOffset(ti->tileh), PAL_NONE);
			break;
	}

	DrawBridgeMiddle(ti, {});
}



const OpenTTDClearServices &GetRustClearServices() noexcept
{
	static const OpenTTDClearServices services{
		.field_tile = RustClearFieldTile, .density = RustClearDensity, .counter = RustClearCounter, .snow = RustSnowTile, .surface = RustClearSurface,
		.set_ground = [](uint32_t t, ClearGround g, uint32_t d) noexcept { SetClearGroundDensity(TileIndex{t}, g, d); },
		.set_density = [](uint32_t t, uint32_t d) noexcept { SetClearDensity(TileIndex{t}, d); },
		.set_counter = [](uint32_t t, uint32_t c) noexcept { RustSetClearCounter(Tile{TileIndex{t}}, c); },
		.clear_snow = [](uint32_t t) noexcept { ClearSnow(TileIndex{t}); },
		.make_clear = RustMakeClear,
		.fences = [](uint32_t index) noexcept { Tile t{TileIndex{index}}; return OpenTTDFieldFences{.ne = RustFieldFence(t, DIAGDIR_NE), .se = RustFieldFence(t, DIAGDIR_SE), .sw = RustFieldFence(t, DIAGDIR_SW), .nw = RustFieldFence(t, DIAGDIR_NW)}; },
		.field = [](uint32_t index) noexcept { Tile t{TileIndex{index}}; return OpenTTDFieldGround{.industry = RustFieldIndustry(t).base(), .type = GetFieldType(t)}; },
		.set_field = [](uint32_t t, uint32_t f) noexcept { SetFieldType(TileIndex{t}, f); },
		.water_class = RustWaterClass,
	};
	return services;
}


static void GetTileDesc_Clear(TileIndex tile, TileDesc &td)
{
	/* Each pair holds a normal and a snowy ClearGround description. */
	static constexpr std::pair<StringID, StringID> clear_land_str[] = {
		{STR_LAI_CLEAR_DESCRIPTION_GRASS,      STR_LAI_CLEAR_DESCRIPTION_SNOWY_GRASS},
		{STR_LAI_CLEAR_DESCRIPTION_ROUGH_LAND, STR_LAI_CLEAR_DESCRIPTION_SNOWY_ROUGH_LAND},
		{STR_LAI_CLEAR_DESCRIPTION_ROCKS,      STR_LAI_CLEAR_DESCRIPTION_SNOWY_ROCKS},
		{STR_LAI_CLEAR_DESCRIPTION_FIELDS,     STR_EMPTY},
		{STR_EMPTY,                            STR_EMPTY}, // CLEAR_SNOW does not appear in the map.
		{STR_LAI_CLEAR_DESCRIPTION_DESERT,     STR_EMPTY},
	};

	if (!IsSnowTile(tile) && IsClearGround(tile, CLEAR_GRASS) && GetClearDensity(tile) == 0) {
		td.str = STR_LAI_CLEAR_DESCRIPTION_BARE_LAND;
	} else {
		const auto &[name, snowy_name] = clear_land_str[GetClearGround(tile)];
		td.str = IsSnowTile(tile) ? snowy_name : name;
	}
	td.owner[0] = GetTileOwner(tile);
}

static void ChangeTileOwner_Clear(TileIndex, Owner, Owner)
{
	return;
}



extern const TileTypeProcs _tile_type_clear_procs = {
	DrawTile_Clear,           ///< draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	ClearTile_Clear,          ///< clear_tile_proc
	nullptr,                     ///< add_accepted_cargo_proc
	GetTileDesc_Clear,        ///< get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                     ///< click_tile_proc
	nullptr,                 // Rust scheduler calls the owner directly.
	ChangeTileOwner_Clear,    ///< change_tile_owner_proc
	nullptr,                     ///< add_produced_cargo_proc
	nullptr,                     ///< vehicle_enter_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	nullptr,                 // Rust tunnel and bridge construction owns the check.
};
