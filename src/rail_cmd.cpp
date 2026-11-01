/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file rail_cmd.cpp Handling of rail tiles. */

#include "stdafx.h"
#include "viewport_func.h"
#include "command_func.h"
#include "depot_base.h"
#include "pathfinder/yapf/yapf_cache.h"
#include "newgrf_debug.h"
#include "newgrf_railtype.h"
#include "train.h"
#include "autoslope.h"
#include "water.h"
#include "tunnelbridge_map.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "tunnelbridge.h"
#include "elrail_func.h"
#include "town.h"
#include "pbs.h"
#include "company_base.h"
#include "core/backup_type.hpp"
#include "core/container_func.hpp"
#include "timer/timer_game_calendar.h"
#include "strings_func.h"
#include "company_gui.h"
#include "object_map.h"
#include "rail_cmd.h"
#include "landscape_cmd.h"

#include "table/strings.h"
#include "table/railtypes.h"
#include "table/track_land.h"

#include "rust/ground_services.h"
#include "rust/services_assert.h"
#include "rust/services_landscape.h"
#include "rust/environment_services.h"

#include "rust/movement_ffi.h"

#include "rust/type_registry_ffi.h"
#include "rust/type_availability_services.h"

#include "safeguards.h"

/** Helper type for lists/vectors of trains */
typedef std::vector<Train *> TrainList;

RailTypeInfo _railtypes[RAILTYPE_END];
static_assert(sizeof(RailTypes) == sizeof(uint64_t) && alignof(RailTypes) == alignof(uint64_t));
std::span<RailType> _sorted_railtypes;
RailTypes &_railtypes_hidden_mask = *reinterpret_cast<RailTypes *>(openttd_rust_rail_hidden_mask());

/** Enum holding the signal offset in the sprite sheet according to the side it is representing. */
enum SignalOffsets {
	SIGNAL_TO_SOUTHWEST,
	SIGNAL_TO_NORTHEAST,
	SIGNAL_TO_SOUTHEAST,
	SIGNAL_TO_NORTHWEST,
	SIGNAL_TO_EAST,
	SIGNAL_TO_WEST,
	SIGNAL_TO_SOUTH,
	SIGNAL_TO_NORTH,
};

/**
 * Reset all rail type information to its default values.
 */

void ResolveRailTypeGUISprites(RailTypeInfo *rti)
{
	SpriteID cursors_base = GetCustomRailSprite(rti, INVALID_TILE, RTSG_CURSORS);
	if (cursors_base != 0) {
		rti->gui_sprites.build_ns_rail = cursors_base +  0;
		rti->gui_sprites.build_x_rail  = cursors_base +  1;
		rti->gui_sprites.build_ew_rail = cursors_base +  2;
		rti->gui_sprites.build_y_rail  = cursors_base +  3;
		rti->gui_sprites.auto_rail     = cursors_base +  4;
		rti->gui_sprites.build_depot   = cursors_base +  5;
		rti->gui_sprites.build_tunnel  = cursors_base +  6;
		rti->gui_sprites.convert_rail  = cursors_base +  7;
		rti->cursor.rail_ns   = cursors_base +  8;
		rti->cursor.rail_swne = cursors_base +  9;
		rti->cursor.rail_ew   = cursors_base + 10;
		rti->cursor.rail_nwse = cursors_base + 11;
		rti->cursor.autorail  = cursors_base + 12;
		rti->cursor.depot     = cursors_base + 13;
		rti->cursor.tunnel    = cursors_base + 14;
		rti->cursor.convert   = cursors_base + 15;
	}

	/* Array of default GUI signal sprite numbers. */
	const SpriteID _signal_lookup[2][SIGTYPE_END] = {
		{SPR_IMG_SIGNAL_ELECTRIC_NORM,  SPR_IMG_SIGNAL_ELECTRIC_ENTRY, SPR_IMG_SIGNAL_ELECTRIC_EXIT,
		 SPR_IMG_SIGNAL_ELECTRIC_COMBO, SPR_IMG_SIGNAL_ELECTRIC_PBS,   SPR_IMG_SIGNAL_ELECTRIC_PBS_OWAY},

		{SPR_IMG_SIGNAL_SEMAPHORE_NORM,  SPR_IMG_SIGNAL_SEMAPHORE_ENTRY, SPR_IMG_SIGNAL_SEMAPHORE_EXIT,
		 SPR_IMG_SIGNAL_SEMAPHORE_COMBO, SPR_IMG_SIGNAL_SEMAPHORE_PBS,   SPR_IMG_SIGNAL_SEMAPHORE_PBS_OWAY},
	};

	for (SignalType type = SIGTYPE_BLOCK; type < SIGTYPE_END; type = (SignalType)(type + 1)) {
		for (SignalVariant var = SIG_ELECTRIC; var <= SIG_SEMAPHORE; var = (SignalVariant)(var + 1)) {
			SpriteID red   = GetCustomSignalSprite(rti, INVALID_TILE, type, var, SIGNAL_STATE_RED, true);
			SpriteID green = GetCustomSignalSprite(rti, INVALID_TILE, type, var, SIGNAL_STATE_GREEN, true);
			rti->gui_sprites.signals[type][var][0] = (red != 0)   ? red + SIGNAL_TO_SOUTH   : _signal_lookup[var][type];
			rti->gui_sprites.signals[type][var][1] = (green != 0) ? green + SIGNAL_TO_SOUTH : _signal_lookup[var][type] + 1;
		}
	}
}

/**
 * Compare railtypes based on their sorting order.
 * @param first  The railtype to compare to.
 * @param second The railtype to compare.
 * @return True iff the first should be sorted before the second.
 */
static bool CompareRailTypes(const RailType &first, const RailType &second)
{
	return GetRailTypeInfo(first)->sorting_order < GetRailTypeInfo(second)->sorting_order;
}

/**
 * Resolve sprites of custom rail types
 */

/**
 * Allocate a new rail type label
 */

const OpenTTDRailRegistryServices &GetRustRailRegistryServices() noexcept
{
	static const OpenTTDRailRegistryServices table = {
		.default_count = std::size(_original_railtypes),
		.spec = [](RailType rt) noexcept { return &_railtypes[rt]; },
		.type_label = [](RailType rt) noexcept { return RustRailTypeLabel(GetRailTypeInfo(rt)); },
		.labels = []() noexcept -> const RailTypeLabel * const * {
			static const auto labels = [] {
				std::array<const RailTypeLabel *, RAILTYPE_END> pointers{};
				for (size_t i = 0; i < pointers.size(); ++i) pointers[i] = &RustRailTypeLabel(&_railtypes[i]);
				return pointers;
			}();
			return labels.data();
		},
		.alternate = [](RailType rt, uint32_t label) noexcept { return GetRailTypeInfo(rt)->alternate_labels.contains(label); },
		.clear = [](RailTypeInfo *info) noexcept { *info = RailTypeInfo{}; },
		.copy_default = [](RailTypeInfo *info, RailType source) noexcept { *info = _original_railtypes[source]; },
		.resolve_gui = [](RailTypeInfo *info) noexcept { ResolveRailTypeGUISprites(info); },
		.publication = [](const RailTypeInfo *info) noexcept { return OpenTTDTypePublication{.label = RustRailTypeLabel(info), .hidden = RustRailTypeHidden(info)}; },
		.sort = [](RailType *data, size_t count) noexcept { std::sort(data, data + count, CompareRailTypes); },
		.publish_sorted = [](RailType *data, size_t count) noexcept { _sorted_railtypes = {data, count}; },
		.allocate = [](RailTypeInfo *info, const OpenTTDTypeAllocation *value) noexcept {
			info->label = value->label;
			info->alternate_labels.clear();
			info->compatible_railtypes = RailTypes{value->self};
			info->powered_railtypes = RailTypes{value->self};
			info->introduces_railtypes = RailTypes{value->self};
			info->sorting_order = value->order;
		},
	};
	return table;
}


static const uint8_t _track_sloped_sprites[14] = {
	14, 15, 22, 13,
	 0, 21, 17, 12,
	23,  0, 18, 20,
	19, 16
};


/*         4
 *     ---------
 *    |\       /|
 *    | \    1/ |
 *    |  \   /  |
 *    |   \ /   |
 *  16|    \    |32
 *    |   / \2  |
 *    |  /   \  |
 *    | /     \ |
 *    |/       \|
 *     ---------
 *         8
 */



/* MAP2 byte:    abcd???? => Signal On? Same coding as map3lo
 * MAP3LO byte:  abcd???? => Signal Exists?
 *               a and b are for diagonals, upper and left,
 *               one for each direction. (ie a == NE->SW, b ==
 *               SW->NE, or v.v., I don't know. b and c are
 *               similar for lower and right.
 * MAP2 byte:    ????abcd => Type of ground.
 * MAP3LO byte:  ????abcd => Type of rail.
 * MAP5:         00abcdef => rail
 *               01abcdef => rail w/ signals
 *               10uuuuuu => unused
 *               11uuuudd => rail depot
 */



/** Valid TrackBits on a specific (non-steep)-slope without foundation */

/**
 * Checks if a track combination is valid on a specific slope and returns the needed foundation.
 *
 * @param tileh Tile slope.
 * @param bits  Trackbits.
 * @return Needed foundation or FOUNDATION_INVALID if track/slope combination is not allowed.
 */
Foundation GetRailFoundation(Slope tileh, TrackBits bits)
{
	return openttd_rust_rail_foundation(tileh, bits, RustAssertCondition);
}




/**
 * Called from water_cmd if a non-flat rail-tile gets flooded and should be converted to shore.
 * The function floods the lower halftile, if the tile has a halftile foundation.
 *
 * @param t The tile to flood.
 * @return true if something was flooded.
 */
bool FloodHalftile(TileIndex tile)
{
	assert(IsPlainRailTile(tile));
	OpenTTDGroundCall call;
	return openttd_rust_flood_halftile(tile.base(), &call, &GetRustWaterGroundServices(), &GetRustSharedServices());
}


/**
 * Get surface height in point (x,y)
 * On tiles with halftile foundations move (x,y) to a safe point wrt. track
 */
static uint GetSaveSlopeZ(uint x, uint y, Track track)
{
	switch (track) {
		case TRACK_UPPER: x &= ~0xF; y &= ~0xF; break;
		case TRACK_LOWER: x |=  0xF; y |=  0xF; break;
		case TRACK_LEFT:  x |=  0xF; y &= ~0xF; break;
		case TRACK_RIGHT: x &= ~0xF; y |=  0xF; break;
		default: break;
	}
	return GetSlopePixelZ(x, y);
}

static void DrawSingleSignal(TileIndex tile, const RailTypeInfo *rti, Track track, SignalState condition, SignalOffsets image, uint pos)
{
	bool side;
	switch (_settings_game.construction.train_signal_side) {
		case 0:  side = false;                                 break; // left
		case 2:  side = true;                                  break; // right
		default: side = _settings_game.vehicle.road_side != 0; break; // driving side
	}
	static const Point SignalPositions[2][12] = {
		{ // Signals on the left side
		/*  LEFT      LEFT      RIGHT     RIGHT     UPPER     UPPER */
			{ 8,  5}, {14,  1}, { 1, 14}, { 9, 11}, { 1,  0}, { 3, 10},
		/*  LOWER     LOWER     X         X         Y         Y     */
			{11,  4}, {14, 14}, {11,  3}, { 4, 13}, { 3,  4}, {11, 13}
		}, { // Signals on the right side
		/*  LEFT      LEFT      RIGHT     RIGHT     UPPER     UPPER */
			{14,  1}, {12, 10}, { 4,  6}, { 1, 14}, {10,  4}, { 0,  1},
		/*  LOWER     LOWER     X         X         Y         Y     */
			{14, 14}, { 5, 12}, {11, 13}, { 4,  3}, {13,  4}, { 3, 11}
		}
	};

	uint x = TileX(tile) * TILE_SIZE + SignalPositions[side][pos].x;
	uint y = TileY(tile) * TILE_SIZE + SignalPositions[side][pos].y;

	SignalType type       = GetSignalType(tile, track);
	SignalVariant variant = GetSignalVariant(tile, track);

	SpriteID sprite = GetCustomSignalSprite(rti, tile, type, variant, condition);
	if (sprite != 0) {
		sprite += image;
	} else {
		/* Normal electric signals are stored in a different sprite block than all other signals. */
		sprite = (type == SIGTYPE_BLOCK && variant == SIG_ELECTRIC) ? SPR_ORIGINAL_SIGNALS_BASE : SPR_SIGNALS_BASE - 16;
		sprite += type * 16 + variant * 64 + image * 2 + condition + (type > SIGTYPE_LAST_NOPBS ? 64 : 0);
	}

	AddSortableSpriteToDraw(sprite, PAL_NONE, x, y, GetSaveSlopeZ(x, y, track), {{}, {1, 1, BB_HEIGHT_UNDER_BRIDGE}, {}});
}

/** Offsets for drawing fences */
struct FenceOffset : SpriteBounds {
	Corner height_ref; ///< Corner to use height offset from.

	constexpr FenceOffset(Corner height_ref, int8_t origin_x, int8_t origin_y, uint8_t extent_x, uint8_t extent_y) :
		SpriteBounds({origin_x, origin_y, 0}, {extent_x, extent_y, 4}, {}), height_ref(height_ref) {}
};

/** Offsets for drawing fences */
static const FenceOffset _fence_offsets[] = {
	{ CORNER_INVALID,  0,  1, 16,  1 }, // RFO_FLAT_X_NW
	{ CORNER_INVALID,  1,  0,  1, 16 }, // RFO_FLAT_Y_NE
	{ CORNER_W,        8,  8,  1,  1 }, // RFO_FLAT_LEFT
	{ CORNER_N,        8,  8,  1,  1 }, // RFO_FLAT_UPPER
	{ CORNER_INVALID,  0,  1, 16,  1 }, // RFO_SLOPE_SW_NW
	{ CORNER_INVALID,  1,  0,  1, 16 }, // RFO_SLOPE_SE_NE
	{ CORNER_INVALID,  0,  1, 16,  1 }, // RFO_SLOPE_NE_NW
	{ CORNER_INVALID,  1,  0,  1, 16 }, // RFO_SLOPE_NW_NE
	{ CORNER_INVALID,  0, 15, 16,  1 }, // RFO_FLAT_X_SE
	{ CORNER_INVALID, 15,  0,  1, 16 }, // RFO_FLAT_Y_SW
	{ CORNER_E,        8,  8,  1,  1 }, // RFO_FLAT_RIGHT
	{ CORNER_S,        8,  8,  1,  1 }, // RFO_FLAT_LOWER
	{ CORNER_INVALID,  0, 15, 16,  1 }, // RFO_SLOPE_SW_SE
	{ CORNER_INVALID, 15,  0,  1, 16 }, // RFO_SLOPE_SE_SW
	{ CORNER_INVALID,  0, 15, 16,  1 }, // RFO_SLOPE_NE_SE
	{ CORNER_INVALID, 15,  0,  1, 16 }, // RFO_SLOPE_NW_SW
};

/**
 * Draw a track fence.
 * @param ti Tile drawing information.
 * @param base_image First fence sprite.
 * @param num_sprites Number of fence sprites.
 * @param rfo Fence to draw.
 */
static void DrawTrackFence(const TileInfo *ti, const PalSpriteID &psid, uint num_sprites, RailFenceOffset rfo)
{
	int z = ti->z;
	if (_fence_offsets[rfo].height_ref != CORNER_INVALID) {
		z += GetSlopePixelZInCorner(RemoveHalftileSlope(ti->tileh), _fence_offsets[rfo].height_ref);
	}
	AddSortableSpriteToDraw(psid.sprite + (rfo % num_sprites), psid.pal, ti->x, ti->y, z, _fence_offsets[rfo]);
}

/**
 * Draw fence at NW border matching the tile slope.
 */
static void DrawTrackFence_NW(const TileInfo *ti, const PalSpriteID &psid, uint num_sprites)
{
	RailFenceOffset rfo = RFO_FLAT_X_NW;
	if (ti->tileh & SLOPE_NW) rfo = (ti->tileh & SLOPE_W) ? RFO_SLOPE_SW_NW : RFO_SLOPE_NE_NW;
	DrawTrackFence(ti, psid, num_sprites, rfo);
}

/**
 * Draw fence at SE border matching the tile slope.
 */
static void DrawTrackFence_SE(const TileInfo *ti, const PalSpriteID &psid, uint num_sprites)
{
	RailFenceOffset rfo = RFO_FLAT_X_SE;
	if (ti->tileh & SLOPE_SE) rfo = (ti->tileh & SLOPE_S) ? RFO_SLOPE_SW_SE : RFO_SLOPE_NE_SE;
	DrawTrackFence(ti, psid, num_sprites, rfo);
}

/**
 * Draw fence at NE border matching the tile slope.
 */
static void DrawTrackFence_NE(const TileInfo *ti, const PalSpriteID &psid, uint num_sprites)
{
	RailFenceOffset rfo = RFO_FLAT_Y_NE;
	if (ti->tileh & SLOPE_NE) rfo = (ti->tileh & SLOPE_E) ? RFO_SLOPE_SE_NE : RFO_SLOPE_NW_NE;
	DrawTrackFence(ti, psid, num_sprites, rfo);
}

/**
 * Draw fence at SW border matching the tile slope.
 */
static void DrawTrackFence_SW(const TileInfo *ti, const PalSpriteID &psid, uint num_sprites)
{
	RailFenceOffset rfo = RFO_FLAT_Y_SW;
	if (ti->tileh & SLOPE_SW) rfo = (ti->tileh & SLOPE_S) ? RFO_SLOPE_SE_SW : RFO_SLOPE_NW_SW;
	DrawTrackFence(ti, psid, num_sprites, rfo);
}

/**
 * Draw track fences.
 * @param ti Tile drawing information.
 * @param rti Rail type information.
 */
static void DrawTrackDetails(const TileInfo *ti, const RailTypeInfo *rti, PaletteID pal)
{
	/* Base sprite for track fences.
	 * Note: Halftile slopes only have fences on the upper part. */
	uint num_sprites = 0;
	PalSpriteID psid{
		.sprite = GetCustomRailSprite(rti, ti->tile, RTSG_FENCES, IsHalftileSlope(ti->tileh) ? TCX_UPPER_HALFTILE : TCX_NORMAL, &num_sprites),
		.pal = pal,
	};
	if (psid.sprite == 0) {
		psid.sprite = SPR_TRACK_FENCE_FLAT_X;
		num_sprites = 8;
	}

	assert(num_sprites > 0);

	switch (GetRailGroundType(ti->tile)) {
		case RailGroundType::FenceNW:     DrawTrackFence_NW(ti, psid, num_sprites); break;
		case RailGroundType::FenceSE:     DrawTrackFence_SE(ti, psid, num_sprites); break;
		case RailGroundType::FenceSENW:   DrawTrackFence_NW(ti, psid, num_sprites);
		                                  DrawTrackFence_SE(ti, psid, num_sprites); break;
		case RailGroundType::FenceNE:     DrawTrackFence_NE(ti, psid, num_sprites); break;
		case RailGroundType::FenceSW:     DrawTrackFence_SW(ti, psid, num_sprites); break;
		case RailGroundType::FenceNESW:   DrawTrackFence_NE(ti, psid, num_sprites);
		                                  DrawTrackFence_SW(ti, psid, num_sprites); break;
		case RailGroundType::FenceVert1:  DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_LEFT);  break;
		case RailGroundType::FenceVert2:  DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_RIGHT); break;
		case RailGroundType::FenceHoriz1: DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_UPPER); break;
		case RailGroundType::FenceHoriz2: DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_LOWER); break;
		case RailGroundType::HalfTileWater: {
			Corner track_corner;
			if (IsHalftileSlope(ti->tileh)) {
				/* Steep slope or one-corner-raised slope with halftile foundation */
				track_corner = GetHalftileSlopeCorner(ti->tileh);
			} else {
				/* Three-corner-raised slope */
				track_corner = OppositeCorner(GetHighestSlopeCorner(ComplementSlope(ti->tileh)));
			}
			switch (track_corner) {
				case CORNER_W: DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_LEFT);  break;
				case CORNER_S: DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_LOWER); break;
				case CORNER_E: DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_RIGHT); break;
				case CORNER_N: DrawTrackFence(ti, psid, num_sprites, RFO_FLAT_UPPER); break;
				default: NOT_REACHED();
			}
			break;
		}
		default: break;
	}
}

/* SubSprite for drawing the track halftile of 'three-corners-raised'-sloped rail sprites. */
static const int INF = 1000; // big number compared to tilesprite size
static const SubSprite _halftile_sub_sprite[4] = {
	{ -INF    , -INF  , 32 - 33, INF     }, // CORNER_W, clip 33 pixels from right
	{ -INF    ,  0 + 7, INF    , INF     }, // CORNER_S, clip 7 pixels from top
	{ -31 + 33, -INF  , INF    , INF     }, // CORNER_E, clip 33 pixels from left
	{ -INF    , -INF  , INF    , 30 - 23 }  // CORNER_N, clip 23 pixels from bottom
};

static inline void DrawTrackSprite(SpriteID sprite, PaletteID pal, const TileInfo *ti, Slope s)
{
	DrawGroundSprite(sprite, pal, nullptr, 0, (ti->tileh & s) ? -8 : 0);
}

static void DrawTrackBitsOverlay(TileInfo *ti, TrackBits track, const RailTypeInfo *rti)
{
	RailGroundType rgt = GetRailGroundType(ti->tile);
	Foundation f = GetRailFoundation(ti->tileh, track);
	Corner halftile_corner = CORNER_INVALID;

	if (IsNonContinuousFoundation(f)) {
		/* Save halftile corner */
		halftile_corner = (f == FOUNDATION_STEEP_BOTH ? GetHighestSlopeCorner(ti->tileh) : GetHalftileFoundationCorner(f));
		/* Draw lower part first */
		track &= ~CornerToTrackBits(halftile_corner);
		f = (f == FOUNDATION_STEEP_BOTH ? FOUNDATION_STEEP_LOWER : FOUNDATION_NONE);
	}

	DrawFoundation(ti, f);
	/* DrawFoundation modifies ti */

	/* Draw ground */
	if (rgt == RailGroundType::HalfTileWater) {
		if (track != TRACK_BIT_NONE || IsSteepSlope(ti->tileh)) {
			/* three-corner-raised slope or steep slope with track on upper part */
			DrawShoreTile(ti->tileh);
		} else {
			/* single-corner-raised slope with track on upper part */
			DrawGroundSprite(SPR_FLAT_WATER_TILE, PAL_NONE);
		}
	} else {
		SpriteID image;

		switch (rgt) {
			case RailGroundType::Barren: image = SPR_FLAT_BARE_LAND; break;
			case RailGroundType::SnowOrDesert: image = SPR_FLAT_SNOW_DESERT_TILE; break;
			default: image = SPR_FLAT_GRASS_TILE; break;
		}

		image += SlopeToSpriteOffset(ti->tileh);

		DrawGroundSprite(image, PAL_NONE);
	}

	bool no_combine = ti->tileh == SLOPE_FLAT && rti->flags.Test(RailTypeFlag::NoSpriteCombine);
	SpriteID overlay = GetCustomRailSprite(rti, ti->tile, RTSG_OVERLAY);
	SpriteID ground = GetCustomRailSprite(rti, ti->tile, no_combine ? RTSG_GROUND_COMPLETE : RTSG_GROUND);
	TrackBits pbs = _settings_client.gui.show_track_reservation ? GetRailReservationTrackBits(ti->tile) : TRACK_BIT_NONE;

	if (track == TRACK_BIT_NONE) {
		/* Half-tile foundation, no track here? */
	} else if (no_combine) {
		/* Use trackbits as direct index from ground sprite, subtract 1
		 * because there is no sprite for no bits. */
		DrawGroundSprite(ground + track - 1, PAL_NONE);

		/* Draw reserved track bits */
		if (pbs & TRACK_BIT_X)     DrawGroundSprite(overlay + RTO_X, PALETTE_CRASH);
		if (pbs & TRACK_BIT_Y)     DrawGroundSprite(overlay + RTO_Y, PALETTE_CRASH);
		if (pbs & TRACK_BIT_UPPER) DrawTrackSprite(overlay + RTO_N, PALETTE_CRASH, ti, SLOPE_N);
		if (pbs & TRACK_BIT_LOWER) DrawTrackSprite(overlay + RTO_S, PALETTE_CRASH, ti, SLOPE_S);
		if (pbs & TRACK_BIT_RIGHT) DrawTrackSprite(overlay + RTO_E, PALETTE_CRASH, ti, SLOPE_E);
		if (pbs & TRACK_BIT_LEFT)  DrawTrackSprite(overlay + RTO_W, PALETTE_CRASH, ti, SLOPE_W);
	} else if (ti->tileh == SLOPE_NW && track == TRACK_BIT_Y) {
		DrawGroundSprite(ground + RTO_SLOPE_NW, PAL_NONE);
		if (pbs != TRACK_BIT_NONE) DrawGroundSprite(overlay + RTO_SLOPE_NW, PALETTE_CRASH);
	} else if (ti->tileh == SLOPE_NE && track == TRACK_BIT_X) {
		DrawGroundSprite(ground + RTO_SLOPE_NE, PAL_NONE);
		if (pbs != TRACK_BIT_NONE) DrawGroundSprite(overlay + RTO_SLOPE_NE, PALETTE_CRASH);
	} else if (ti->tileh == SLOPE_SE && track == TRACK_BIT_Y) {
		DrawGroundSprite(ground + RTO_SLOPE_SE, PAL_NONE);
		if (pbs != TRACK_BIT_NONE) DrawGroundSprite(overlay + RTO_SLOPE_SE, PALETTE_CRASH);
	} else if (ti->tileh == SLOPE_SW && track == TRACK_BIT_X) {
		DrawGroundSprite(ground + RTO_SLOPE_SW, PAL_NONE);
		if (pbs != TRACK_BIT_NONE) DrawGroundSprite(overlay + RTO_SLOPE_SW, PALETTE_CRASH);
	} else {
		switch (track) {
			/* Draw single ground sprite when not overlapping. No track overlay
			 * is necessary for these sprites. */
			case TRACK_BIT_X:     DrawGroundSprite(ground + RTO_X, PAL_NONE); break;
			case TRACK_BIT_Y:     DrawGroundSprite(ground + RTO_Y, PAL_NONE); break;
			case TRACK_BIT_UPPER: DrawTrackSprite(ground + RTO_N, PAL_NONE, ti, SLOPE_N); break;
			case TRACK_BIT_LOWER: DrawTrackSprite(ground + RTO_S, PAL_NONE, ti, SLOPE_S); break;
			case TRACK_BIT_RIGHT: DrawTrackSprite(ground + RTO_E, PAL_NONE, ti, SLOPE_E); break;
			case TRACK_BIT_LEFT:  DrawTrackSprite(ground + RTO_W, PAL_NONE, ti, SLOPE_W); break;
			case TRACK_BIT_CROSS: DrawGroundSprite(ground + RTO_CROSSING_XY, PAL_NONE); break;
			case TRACK_BIT_HORZ:  DrawTrackSprite(ground + RTO_N, PAL_NONE, ti, SLOPE_N);
			                      DrawTrackSprite(ground + RTO_S, PAL_NONE, ti, SLOPE_S); break;
			case TRACK_BIT_VERT:  DrawTrackSprite(ground + RTO_E, PAL_NONE, ti, SLOPE_E);
			                      DrawTrackSprite(ground + RTO_W, PAL_NONE, ti, SLOPE_W); break;

			default:
				/* We're drawing a junction tile */
				if ((track & TRACK_BIT_3WAY_NE) == 0) {
					DrawGroundSprite(ground + RTO_JUNCTION_SW, PAL_NONE);
				} else if ((track & TRACK_BIT_3WAY_SW) == 0) {
					DrawGroundSprite(ground + RTO_JUNCTION_NE, PAL_NONE);
				} else if ((track & TRACK_BIT_3WAY_NW) == 0) {
					DrawGroundSprite(ground + RTO_JUNCTION_SE, PAL_NONE);
				} else if ((track & TRACK_BIT_3WAY_SE) == 0) {
					DrawGroundSprite(ground + RTO_JUNCTION_NW, PAL_NONE);
				} else {
					DrawGroundSprite(ground + RTO_JUNCTION_NSEW, PAL_NONE);
				}

				/* Mask out PBS bits as we shall draw them afterwards anyway. */
				track &= ~pbs;

				/* Draw regular track bits */
				if (track & TRACK_BIT_X)     DrawGroundSprite(overlay + RTO_X, PAL_NONE);
				if (track & TRACK_BIT_Y)     DrawGroundSprite(overlay + RTO_Y, PAL_NONE);
				if (track & TRACK_BIT_UPPER) DrawGroundSprite(overlay + RTO_N, PAL_NONE);
				if (track & TRACK_BIT_LOWER) DrawGroundSprite(overlay + RTO_S, PAL_NONE);
				if (track & TRACK_BIT_RIGHT) DrawGroundSprite(overlay + RTO_E, PAL_NONE);
				if (track & TRACK_BIT_LEFT)  DrawGroundSprite(overlay + RTO_W, PAL_NONE);
		}

		/* Draw reserved track bits */
		if (pbs & TRACK_BIT_X)     DrawGroundSprite(overlay + RTO_X, PALETTE_CRASH);
		if (pbs & TRACK_BIT_Y)     DrawGroundSprite(overlay + RTO_Y, PALETTE_CRASH);
		if (pbs & TRACK_BIT_UPPER) DrawTrackSprite(overlay + RTO_N, PALETTE_CRASH, ti, SLOPE_N);
		if (pbs & TRACK_BIT_LOWER) DrawTrackSprite(overlay + RTO_S, PALETTE_CRASH, ti, SLOPE_S);
		if (pbs & TRACK_BIT_RIGHT) DrawTrackSprite(overlay + RTO_E, PALETTE_CRASH, ti, SLOPE_E);
		if (pbs & TRACK_BIT_LEFT)  DrawTrackSprite(overlay + RTO_W, PALETTE_CRASH, ti, SLOPE_W);
	}

	if (IsValidCorner(halftile_corner)) {
		DrawFoundation(ti, HalftileFoundation(halftile_corner));
		overlay = GetCustomRailSprite(rti, ti->tile, RTSG_OVERLAY, TCX_UPPER_HALFTILE);
		ground = GetCustomRailSprite(rti, ti->tile, RTSG_GROUND, TCX_UPPER_HALFTILE);

		/* Draw higher halftile-overlay: Use the sloped sprites with three corners raised. They probably best fit the lightning. */
		Slope fake_slope = SlopeWithThreeCornersRaised(OppositeCorner(halftile_corner));

		SpriteID image;
		switch (rgt) {
			case RailGroundType::Barren: image = SPR_FLAT_BARE_LAND; break;
			case RailGroundType::SnowOrDesert:
			case RailGroundType::HalfTileSnow: image = SPR_FLAT_SNOW_DESERT_TILE; break;
			default: image = SPR_FLAT_GRASS_TILE; break;
		}

		image += SlopeToSpriteOffset(fake_slope);

		DrawGroundSprite(image, PAL_NONE, &(_halftile_sub_sprite[halftile_corner]));

		track = CornerToTrackBits(halftile_corner);

		int offset;
		switch (track) {
			default: NOT_REACHED();
			case TRACK_BIT_UPPER: offset = RTO_N; break;
			case TRACK_BIT_LOWER: offset = RTO_S; break;
			case TRACK_BIT_RIGHT: offset = RTO_E; break;
			case TRACK_BIT_LEFT:  offset = RTO_W; break;
		}

		DrawTrackSprite(ground + offset, PAL_NONE, ti, fake_slope);
		if (_settings_client.gui.show_track_reservation && HasReservedTracks(ti->tile, track)) {
			DrawTrackSprite(overlay + offset, PALETTE_CRASH, ti, fake_slope);
		}
	}
}

/**
 * Returns which of the 5 junction-'Rail underlays' to use for the given track bits.
 * See https://newgrf-specs.tt-wiki.net/wiki/Action3/Railtypes for the cases.
 * @pre The track bits describe a junction, so at least two bits are set.
 * @param track The track bits to consider.
 * @return The offset in the five junction ground sprites.
 */
static int GetJunctionGroundSpriteOffset(TrackBits track)
{
	/* If none of the tracks end up in the NE corner, return the ground sprite
	 * where the NE of the tile is not covered. Repeat for the other directions.
	 * What remains are junctions where all directions are covered. */
	if ((track & TRACK_BIT_3WAY_NE) == 0) return 0;
	if ((track & TRACK_BIT_3WAY_SW) == 0) return 1;
	if ((track & TRACK_BIT_3WAY_NW) == 0) return 2;
	if ((track & TRACK_BIT_3WAY_SE) == 0) return 3;
	return 4;
}

/**
 * Draw ground sprite and track bits
 * @param ti TileInfo
 * @param track TrackBits to draw
 */
static void DrawTrackBits(TileInfo *ti, TrackBits track)
{
	const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(ti->tile));

	if (rti->UsesOverlay()) {
		DrawTrackBitsOverlay(ti, track, rti);
		return;
	}

	RailGroundType rgt = GetRailGroundType(ti->tile);
	Foundation f = GetRailFoundation(ti->tileh, track);
	Corner halftile_corner = CORNER_INVALID;

	if (IsNonContinuousFoundation(f)) {
		/* Save halftile corner */
		halftile_corner = (f == FOUNDATION_STEEP_BOTH ? GetHighestSlopeCorner(ti->tileh) : GetHalftileFoundationCorner(f));
		/* Draw lower part first */
		track &= ~CornerToTrackBits(halftile_corner);
		f = (f == FOUNDATION_STEEP_BOTH ? FOUNDATION_STEEP_LOWER : FOUNDATION_NONE);
	}

	DrawFoundation(ti, f);
	/* DrawFoundation modifies ti */

	SpriteID image;
	PaletteID pal = PAL_NONE;
	const SubSprite *sub = nullptr;
	bool junction = false;

	/* Select the sprite to use. */
	if (track == 0) {
		/* Clear ground (only track on halftile foundation) */
		if (rgt == RailGroundType::HalfTileWater) {
			if (IsSteepSlope(ti->tileh)) {
				DrawShoreTile(ti->tileh);
				image = 0;
			} else {
				image = SPR_FLAT_WATER_TILE;
			}
		} else {
			switch (rgt) {
				case RailGroundType::Barren: image = SPR_FLAT_BARE_LAND; break;
				case RailGroundType::SnowOrDesert: image = SPR_FLAT_SNOW_DESERT_TILE; break;
				default: image = SPR_FLAT_GRASS_TILE; break;
			}
			image += SlopeToSpriteOffset(ti->tileh);
		}
	} else {
		if (ti->tileh != SLOPE_FLAT) {
			/* track on non-flat ground */
			image = _track_sloped_sprites[ti->tileh - 1] + rti->base_sprites.track_y;
		} else {
			/* track on flat ground */
			switch (track) {
				/* single track, select combined track + ground sprite*/
				case TRACK_BIT_Y:     image = rti->base_sprites.track_y;     break;
				case TRACK_BIT_X:     image = rti->base_sprites.track_y + 1; break;
				case TRACK_BIT_UPPER: image = rti->base_sprites.track_y + 2; break;
				case TRACK_BIT_LOWER: image = rti->base_sprites.track_y + 3; break;
				case TRACK_BIT_RIGHT: image = rti->base_sprites.track_y + 4; break;
				case TRACK_BIT_LEFT:  image = rti->base_sprites.track_y + 5; break;
				case TRACK_BIT_CROSS: image = rti->base_sprites.track_y + 6; break;

				/* double diagonal track, select combined track + ground sprite*/
				case TRACK_BIT_HORZ:  image = rti->base_sprites.track_ns;     break;
				case TRACK_BIT_VERT:  image = rti->base_sprites.track_ns + 1; break;

				/* junction, select only ground sprite, handle track sprite later */
				default:
					junction = true;
					image = rti->base_sprites.ground + GetJunctionGroundSpriteOffset(track);
					break;
			}
		}

		switch (rgt) {
			case RailGroundType::Barren: pal = PALETTE_TO_BARE_LAND; break;
			case RailGroundType::SnowOrDesert: image += rti->snow_offset; break;
			case RailGroundType::HalfTileWater: {
				/* three-corner-raised slope */
				DrawShoreTile(ti->tileh);
				Corner track_corner = OppositeCorner(GetHighestSlopeCorner(ComplementSlope(ti->tileh)));
				sub = &(_halftile_sub_sprite[track_corner]);
				break;
			}
			default: break;
		}
	}

	if (image != 0) DrawGroundSprite(image, pal, sub);

	/* Draw track pieces individually for junction tiles */
	if (junction) {
		if (track & TRACK_BIT_X)     DrawGroundSprite(rti->base_sprites.single_x, PAL_NONE);
		if (track & TRACK_BIT_Y)     DrawGroundSprite(rti->base_sprites.single_y, PAL_NONE);
		if (track & TRACK_BIT_UPPER) DrawGroundSprite(rti->base_sprites.single_n, PAL_NONE);
		if (track & TRACK_BIT_LOWER) DrawGroundSprite(rti->base_sprites.single_s, PAL_NONE);
		if (track & TRACK_BIT_LEFT)  DrawGroundSprite(rti->base_sprites.single_w, PAL_NONE);
		if (track & TRACK_BIT_RIGHT) DrawGroundSprite(rti->base_sprites.single_e, PAL_NONE);
	}

	/* PBS debugging, draw reserved tracks darker */
	if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation) {
		/* Get reservation, but mask track on halftile slope */
		TrackBits pbs = GetRailReservationTrackBits(ti->tile) & track;
		if (pbs & TRACK_BIT_X) {
			if (ti->tileh == SLOPE_FLAT || ti->tileh == SLOPE_ELEVATED) {
				DrawGroundSprite(rti->base_sprites.single_x, PALETTE_CRASH);
			} else {
				DrawGroundSprite(_track_sloped_sprites[ti->tileh - 1] + rti->base_sprites.single_sloped - 20, PALETTE_CRASH);
			}
		}
		if (pbs & TRACK_BIT_Y) {
			if (ti->tileh == SLOPE_FLAT || ti->tileh == SLOPE_ELEVATED) {
				DrawGroundSprite(rti->base_sprites.single_y, PALETTE_CRASH);
			} else {
				DrawGroundSprite(_track_sloped_sprites[ti->tileh - 1] + rti->base_sprites.single_sloped - 20, PALETTE_CRASH);
			}
		}
		if (pbs & TRACK_BIT_UPPER) DrawGroundSprite(rti->base_sprites.single_n, PALETTE_CRASH, nullptr, 0, ti->tileh & SLOPE_N ? -(int)TILE_HEIGHT : 0);
		if (pbs & TRACK_BIT_LOWER) DrawGroundSprite(rti->base_sprites.single_s, PALETTE_CRASH, nullptr, 0, ti->tileh & SLOPE_S ? -(int)TILE_HEIGHT : 0);
		if (pbs & TRACK_BIT_LEFT)  DrawGroundSprite(rti->base_sprites.single_w, PALETTE_CRASH, nullptr, 0, ti->tileh & SLOPE_W ? -(int)TILE_HEIGHT : 0);
		if (pbs & TRACK_BIT_RIGHT) DrawGroundSprite(rti->base_sprites.single_e, PALETTE_CRASH, nullptr, 0, ti->tileh & SLOPE_E ? -(int)TILE_HEIGHT : 0);
	}

	if (IsValidCorner(halftile_corner)) {
		DrawFoundation(ti, HalftileFoundation(halftile_corner));

		/* Draw higher halftile-overlay: Use the sloped sprites with three corners raised. They probably best fit the lightning. */
		Slope fake_slope = SlopeWithThreeCornersRaised(OppositeCorner(halftile_corner));
		image = _track_sloped_sprites[fake_slope - 1] + rti->base_sprites.track_y;
		pal = PAL_NONE;
		switch (rgt) {
			case RailGroundType::Barren: pal = PALETTE_TO_BARE_LAND; break;
			case RailGroundType::SnowOrDesert:
			case RailGroundType::HalfTileSnow: image += rti->snow_offset; break; // higher part has snow in this case too
			default: break;
		}
		DrawGroundSprite(image, pal, &(_halftile_sub_sprite[halftile_corner]));

		if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation && HasReservedTracks(ti->tile, CornerToTrackBits(halftile_corner))) {
			static const uint8_t _corner_to_track_sprite[] = {3, 1, 2, 0};
			DrawGroundSprite(_corner_to_track_sprite[halftile_corner] + rti->base_sprites.single_n, PALETTE_CRASH, nullptr, 0, -(int)TILE_HEIGHT);
		}
	}
}

static void DrawSignals(TileIndex tile, TrackBits rails, const RailTypeInfo *rti)
{
	auto MAYBE_DRAW_SIGNAL = [&](uint8_t signalbit, SignalOffsets image, uint pos, Track track) {
		if (IsSignalPresent(tile, signalbit)) DrawSingleSignal(tile, rti, track, GetSingleSignalState(tile, signalbit), image, pos);
	};

	if (!(rails & TRACK_BIT_Y)) {
		if (!(rails & TRACK_BIT_X)) {
			if (rails & TRACK_BIT_LEFT) {
				MAYBE_DRAW_SIGNAL(2, SIGNAL_TO_NORTH, 0, TRACK_LEFT);
				MAYBE_DRAW_SIGNAL(3, SIGNAL_TO_SOUTH, 1, TRACK_LEFT);
			}
			if (rails & TRACK_BIT_RIGHT) {
				MAYBE_DRAW_SIGNAL(0, SIGNAL_TO_NORTH, 2, TRACK_RIGHT);
				MAYBE_DRAW_SIGNAL(1, SIGNAL_TO_SOUTH, 3, TRACK_RIGHT);
			}
			if (rails & TRACK_BIT_UPPER) {
				MAYBE_DRAW_SIGNAL(3, SIGNAL_TO_WEST, 4, TRACK_UPPER);
				MAYBE_DRAW_SIGNAL(2, SIGNAL_TO_EAST, 5, TRACK_UPPER);
			}
			if (rails & TRACK_BIT_LOWER) {
				MAYBE_DRAW_SIGNAL(1, SIGNAL_TO_WEST, 6, TRACK_LOWER);
				MAYBE_DRAW_SIGNAL(0, SIGNAL_TO_EAST, 7, TRACK_LOWER);
			}
		} else {
			MAYBE_DRAW_SIGNAL(3, SIGNAL_TO_SOUTHWEST, 8, TRACK_X);
			MAYBE_DRAW_SIGNAL(2, SIGNAL_TO_NORTHEAST, 9, TRACK_X);
		}
	} else {
		MAYBE_DRAW_SIGNAL(3, SIGNAL_TO_SOUTHEAST, 10, TRACK_Y);
		MAYBE_DRAW_SIGNAL(2, SIGNAL_TO_NORTHWEST, 11, TRACK_Y);
	}
}

static void DrawTile_Track(TileInfo *ti)
{
	const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(ti->tile));
	BridgePillarFlags blocked_pillars{};
	PaletteID pal = GetCompanyPalette(GetTileOwner(ti->tile));

	if (IsPlainRail(ti->tile)) {
		TrackBits rails = GetTrackBits(ti->tile);

		DrawTrackBits(ti, rails);

		if (HasBit(_display_opt, DO_FULL_DETAIL)) DrawTrackDetails(ti, rti, pal);

		if (HasRailCatenaryDrawn(GetRailType(ti->tile))) DrawRailCatenary(ti);

		if (HasSignals(ti->tile)) DrawSignals(ti->tile, rails, rti);

		if (IsBridgeAbove(ti->tile)) {
			if ((rails & TRACK_BIT_3WAY_NE) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeNE);
			if ((rails & TRACK_BIT_3WAY_SE) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeSE);
			if ((rails & TRACK_BIT_3WAY_SW) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeSW);
			if ((rails & TRACK_BIT_3WAY_NW) != 0) blocked_pillars.Set(BridgePillarFlag::EdgeNW);
		}
	} else {
		/* draw depot */
		const DrawTileSprites *dts;
		DiagDirection dir = GetRailDepotDirection(ti->tile);

		if (ti->tileh != SLOPE_FLAT) DrawFoundation(ti, FOUNDATION_LEVELED);

		if (IsInvisibilitySet(TO_BUILDINGS)) {
			/* Draw rail instead of depot */
			dts = &_depot_invisible_gfx_table[dir];
		} else {
			dts = &_depot_gfx_table[dir];
		}

		SpriteID image;
		if (rti->UsesOverlay()) {
			image = SPR_FLAT_GRASS_TILE;
		} else {
			image = dts->ground.sprite;
			if (image != SPR_FLAT_GRASS_TILE) image += rti->GetRailtypeSpriteOffset();
		}

		/* Adjust ground tile for desert and snow. */
		if (IsSnowRailGround(ti->tile)) {
			if (image != SPR_FLAT_GRASS_TILE) {
				image += rti->snow_offset; // tile with tracks
			} else {
				image = SPR_FLAT_SNOW_DESERT_TILE; // flat ground
			}
		}

		DrawGroundSprite(image, GroundSpritePaletteTransform(image, PAL_NONE, pal));

		if (rti->UsesOverlay()) {
			SpriteID ground = GetCustomRailSprite(rti, ti->tile, RTSG_GROUND);

			switch (GetRailDepotDirection(ti->tile)) {
				case DIAGDIR_NE:
					if (!IsInvisibilitySet(TO_BUILDINGS)) break;
					[[fallthrough]];
				case DIAGDIR_SW:
					DrawGroundSprite(ground + RTO_X, PAL_NONE);
					break;
				case DIAGDIR_NW:
					if (!IsInvisibilitySet(TO_BUILDINGS)) break;
					[[fallthrough]];
				case DIAGDIR_SE:
					DrawGroundSprite(ground + RTO_Y, PAL_NONE);
					break;
				default:
					break;
			}

			if (_settings_client.gui.show_track_reservation && HasDepotReservation(ti->tile)) {
				SpriteID overlay = GetCustomRailSprite(rti, ti->tile, RTSG_OVERLAY);

				switch (GetRailDepotDirection(ti->tile)) {
					case DIAGDIR_NE:
						if (!IsInvisibilitySet(TO_BUILDINGS)) break;
						[[fallthrough]];
					case DIAGDIR_SW:
						DrawGroundSprite(overlay + RTO_X, PALETTE_CRASH);
						break;
					case DIAGDIR_NW:
						if (!IsInvisibilitySet(TO_BUILDINGS)) break;
						[[fallthrough]];
					case DIAGDIR_SE:
						DrawGroundSprite(overlay + RTO_Y, PALETTE_CRASH);
						break;
					default:
						break;
				}
			}
		} else {
			/* PBS debugging, draw reserved tracks darker */
			if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation && HasDepotReservation(ti->tile)) {
				switch (GetRailDepotDirection(ti->tile)) {
					case DIAGDIR_NE:
						if (!IsInvisibilitySet(TO_BUILDINGS)) break;
						[[fallthrough]];
					case DIAGDIR_SW:
						DrawGroundSprite(rti->base_sprites.single_x, PALETTE_CRASH);
						break;
					case DIAGDIR_NW:
						if (!IsInvisibilitySet(TO_BUILDINGS)) break;
						[[fallthrough]];
					case DIAGDIR_SE:
						DrawGroundSprite(rti->base_sprites.single_y, PALETTE_CRASH);
						break;
					default:
						break;
				}
			}
		}
		int depot_sprite = GetCustomRailSprite(rti, ti->tile, RTSG_DEPOT);
		int relocation = depot_sprite != 0 ? depot_sprite - SPR_RAIL_DEPOT_SE_1 : rti->GetRailtypeSpriteOffset();

		if (HasRailCatenaryDrawn(GetRailType(ti->tile))) DrawRailCatenary(ti);

		DrawRailTileSeq(ti, dts, TO_BUILDINGS, relocation, 0, pal);
		/* Depots can't have bridges above so no blocked pillars. */
	}
	DrawBridgeMiddle(ti, blocked_pillars);
}

void DrawTrainDepotSprite(int x, int y, int dir, RailType railtype)
{
	const DrawTileSprites *dts = &_depot_gfx_table[dir];
	const RailTypeInfo *rti = GetRailTypeInfo(railtype);
	SpriteID image = rti->UsesOverlay() ? SPR_FLAT_GRASS_TILE : dts->ground.sprite;
	uint32_t offset = rti->GetRailtypeSpriteOffset();

	if (image != SPR_FLAT_GRASS_TILE) image += offset;
	PaletteID palette = GetCompanyPalette(_local_company);

	DrawSprite(image, PAL_NONE, x, y);

	if (rti->UsesOverlay()) {
		SpriteID ground = GetCustomRailSprite(rti, INVALID_TILE, RTSG_GROUND);

		switch (dir) {
			case DIAGDIR_SW: DrawSprite(ground + RTO_X, PAL_NONE, x, y); break;
			case DIAGDIR_SE: DrawSprite(ground + RTO_Y, PAL_NONE, x, y); break;
			default: break;
		}
	}
	int depot_sprite = GetCustomRailSprite(rti, INVALID_TILE, RTSG_DEPOT);
	if (depot_sprite != 0) offset = depot_sprite - SPR_RAIL_DEPOT_SE_1;

	DrawRailTileSeqInGUI(x, y, dts, offset, 0, palette);
}



const OpenTTDRailServices &GetRustRailGroundServices() noexcept
{
	static const OpenTTDRailServices services{
		.tracks = RustRailTracks,
		.ground = RustRailGround,
		.set_ground = RustSetRailGroundTyped,
		.snow = [](uint32_t index) noexcept { auto [z, slope] = RustTileSlopeZ(index); bool plain = RustPlainRailTile(index); return OpenTTDRailSnowGround{.z = z, .slope = slope, .tracks = plain ? RustRailTracks(index) : TRACK_BIT_NONE, .plain = plain}; },
		.fences = [](uint32_t index) noexcept { bool plain = RustPlainRailTile(index); return OpenTTDRailFenceGround{.tracks = plain ? RustRailTracks(index) : TRACK_BIT_NONE, .owner = plain ? RustTileOwnerValue(index) : uint8_t{}, .plain = plain}; },

		.owned_land = [](uint32_t t) noexcept { return IsObjectType(TileIndex{t}, OBJECT_OWNED_LAND); },
	};
	return services;
}



static bool ClickTile_Track(TileIndex tile)
{
	if (!IsRailDepot(tile)) return false;

	ShowDepotWindow(tile, VEH_TRAIN);
	return true;
}

static void GetTileDesc_Track(TileIndex tile, TileDesc &td)
{
	const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(tile));
	td.rail_speed = rti->max_speed;
	td.railtype = rti->strings.name;
	td.owner[0] = GetTileOwner(tile);
	switch (GetRailTileType(tile)) {
		case RailTileType::Normal:
			td.str = STR_LAI_RAIL_DESCRIPTION_TRACK;
			break;

		case RailTileType::Signals: {
			static const StringID signal_type[6][6] = {
				{
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_SIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_PRESIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_EXITSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_COMBOSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_NOENTRYSIGNALS
				},
				{
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_PRESIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRESIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_EXITSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_COMBOSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_NOENTRYSIGNALS
				},
				{
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_EXITSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_EXITSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_EXITSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_EXIT_COMBOSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_EXIT_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_EXIT_NOENTRYSIGNALS
				},
				{
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_COMBOSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_COMBOSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_EXIT_COMBOSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_COMBOSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_COMBO_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_COMBO_NOENTRYSIGNALS
				},
				{
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_EXIT_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_COMBO_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PBSSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PBS_NOENTRYSIGNALS
				},
				{
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NORMAL_NOENTRYSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PRE_NOENTRYSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_EXIT_NOENTRYSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_COMBO_NOENTRYSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_PBS_NOENTRYSIGNALS,
					STR_LAI_RAIL_DESCRIPTION_TRACK_WITH_NOENTRYSIGNALS
				}
			};

			SignalType primary_signal;
			SignalType secondary_signal;
			if (HasSignalOnTrack(tile, TRACK_UPPER)) {
				primary_signal = GetSignalType(tile, TRACK_UPPER);
				secondary_signal = HasSignalOnTrack(tile, TRACK_LOWER) ? GetSignalType(tile, TRACK_LOWER) : primary_signal;
			} else {
				secondary_signal = primary_signal = GetSignalType(tile, TRACK_LOWER);
			}

			td.str = signal_type[secondary_signal][primary_signal];
			break;
		}

		case RailTileType::Depot:
			td.str = STR_LAI_RAIL_DESCRIPTION_TRAIN_DEPOT;
			if (_settings_game.vehicle.train_acceleration_model != AM_ORIGINAL) {
				if (td.rail_speed > 0) {
					td.rail_speed = std::min<uint16_t>(td.rail_speed, 61);
				} else {
					td.rail_speed = 61;
				}
			}
			td.build_date = Depot::GetByTile(tile)->build_date;
			break;

		default:
			NOT_REACHED();
	}
}




/**
 * Compute number of ticks when next wagon will leave a depot.
 * Negative means next wagon should have left depot n ticks before.
 * @param v vehicle outside (leaving) the depot
 * @return number of ticks when the next wagon will leave
 */
int TicksToLeaveDepot(const Train *v)
{
	return openttd_rust_movement_depot_ticks(&GetRustMovementServices(), TrainHandle(v));
}

/**
 * Tile callback routine when vehicle enters tile
 * @see vehicle_enter_tile_proc
 */


/* Rust owns the rail track, depot and signal commands and the rail clear,
 * owner, terraform and build-above tile procs (rail_cmd.rs). Map, pools, company
 * counters and every CommandCost stay native; the original bodies remain
 * under #ifndef WITH_RUST above. */
#include "rust/rail_command_ffi.h"
#include "rust/services_water.h"
#include "rust/services_newgrf.h"
#include "rust/command_cost_services.h"
#include "rust/company_services.h"
#include "rust/settings_services.h"
#include "rust/vehicle_services.h"
#include "tilearea_type.h"
#include "window_func.h"

static_assert(PR_BUILD_SIGNALS == 3 && PR_BUILD_DEPOT_TRAIN == 5 && PR_CLEAR_ROUGH == 23 && PR_CLEAR_SIGNALS == 28);
static_assert(PR_CLEAR_DEPOT_TRAIN == 30 && PR_CLEAR_WATER == 34 && PR_BUILD_FOUNDATION == 57);
static_assert(VALID_LEVEL_CROSSING_SLOPES == 0x6CA1 && INVALID_ROADTYPE == 63 && ROAD_X == 10 && ROAD_Y == 5);
static_assert(to_underlying(RailGroundType::Barren) == 0 && to_underlying(RailGroundType::HalfTileWater) == 13);
static_assert(to_underlying(RailTileType::Normal) == 0 && to_underlying(RailTileType::Signals) == 1 && to_underlying(RailTileType::Depot) == 3);
static_assert(LEVELCROSSING_TRACKBIT_FACTOR == 2 && TUNNELBRIDGE_TRACKBIT_FACTOR == 4 && TILE_AXIAL_DISTANCE == 192 && TILE_CORNER_DISTANCE == 128);
static_assert(OWNER_WATER.base() == 0x11 && OWNER_DEITY.base() == 0x12 && OT_LOADING == 3 && SIGTYPE_LAST == 5);
static_assert(FOUNDATION_RAIL_W == 10 && FOUNDATION_RAIL_N == 13 && FOUNDATION_INVALID == 0xFF);

#include "rust/rail_command_services.h"

static CommandCost &RailCost(OpenTTDCommandCost *cost) noexcept { return *reinterpret_cast<CommandCost *>(cost); }

static void RailBuildSingle(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, uint8_t rt, uint8_t track, bool auto_remove) noexcept
{
	RailCost(out) = Command<CMD_BUILD_SINGLE_RAIL>::Do(DoCommandFlags{flags}, TileIndex{tile}, static_cast<RailType>(rt), static_cast<Track>(track), auto_remove);
}
static void RailRemoveSingle(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, uint8_t track) noexcept
{
	RailCost(out) = Command<CMD_REMOVE_SINGLE_RAIL>::Do(DoCommandFlags{flags}, TileIndex{tile}, static_cast<Track>(track));
}
static void RailBuildSignal(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, uint8_t track, uint8_t sigtype, uint8_t sigvar, bool convert, bool skip, bool ctrl, uint8_t cycle_start, uint8_t cycle_stop, uint8_t num_dir_cycle, uint8_t signals_copy) noexcept
{
	RailCost(out) = Command<CMD_BUILD_SINGLE_SIGNAL>::Do(DoCommandFlags{flags}, TileIndex{tile}, static_cast<Track>(track), static_cast<SignalType>(sigtype), static_cast<SignalVariant>(sigvar), convert, skip, ctrl, static_cast<SignalType>(cycle_start), static_cast<SignalType>(cycle_stop), num_dir_cycle, signals_copy);
}
static void RailRemoveSignal(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, uint8_t track) noexcept
{
	RailCost(out) = Command<CMD_REMOVE_SINGLE_SIGNAL>::Do(DoCommandFlags{flags}, TileIndex{tile}, static_cast<Track>(track));
}
static void RailConvert(OpenTTDCommandCost *out, uint16_t flags, uint32_t tile, uint32_t area_start, uint8_t rt, bool diagonal) noexcept
{
	RailCost(out) = Command<CMD_CONVERT_RAIL>::Do(DoCommandFlags{flags}, TileIndex{tile}, TileIndex{area_start}, static_cast<RailType>(rt), diagonal);
}

static OpenTTDRailTrainParking RailParking(Train *v) noexcept
{
	return {.track = v->GetTrack(), .status = RustVehicleStatus(v), .speed = RustVehicleSpeed(v), .order = RustVehicleOrderType(v)};
}
static void RailMakeRoad(uint32_t tile, OpenTTDCrossingRoad r) noexcept
{
	MakeRoadNormal(TileIndex{tile}, static_cast<RoadBits>(r.bits), static_cast<RoadType>(r.road_type), static_cast<RoadType>(r.tram_type), TownID(r.town), Owner(r.road_owner), Owner(r.tram_owner));
}
static void RailMakeCrossing(uint32_t tile, uint8_t road, uint8_t tram, uint8_t rail, uint8_t axis, uint8_t rt, uint8_t road_rt, uint8_t tram_rt, uint16_t town) noexcept
{
	MakeRoadCrossing(TileIndex{tile}, Owner(road), Owner(tram), Owner(rail), static_cast<Axis>(axis), static_cast<RailType>(rt), static_cast<RoadType>(road_rt), static_cast<RoadType>(tram_rt), TownID(town));
}
static void RailMakeShore(uint32_t tile) noexcept
{
	bool docking = OpenTTDIsDockingTile(tile);
	OpenTTDMakeShore(tile);
	OpenTTDSetDockingTile(tile, docking);
}
static void RailDepotWindows(uint32_t tile) noexcept
{
	InvalidateWindowData(WC_VEHICLE_DEPOT, TileIndex{tile});
	InvalidateWindowData(WC_BUILD_VEHICLE, TileIndex{tile});
}
static void RailIncludeTrain(OpenTTDRailConvertFrame *frame, OpenTTDVehicle *v) noexcept { include(frame->affected, RustVehicleFirst(Train::From(VehicleOf(v)))); }
static void RailConsistChanged(OpenTTDRailConvertFrame *frame) noexcept
{
	for (Train *v : frame->affected) {
		v->ConsistChanged(CCF_TRACK);
	}
}

const OpenTTDRailCommandServices &GetRustRailCommandServices() noexcept
{
	static const OpenTTDRailCommandServices services{
		.shared = &GetRustSharedServices(),
		.rail = &GetRustRailMapServices(),
		.cost = &GetRustCommandCostServices(),
		.command = &GetRustCommandServices(),
		.signal = &GetRustSignalServices(),
		.strings = {
			.impossible_combination = STR_ERROR_IMPOSSIBLE_TRACK_COMBINATION,
			.already_built = STR_ERROR_ALREADY_BUILT,
			.build_on_water = STR_ERROR_CAN_T_BUILD_ON_WATER,
			.sloped_wrong = STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION,
			.remove_signals_first = STR_ERROR_MUST_REMOVE_SIGNALS_FIRST,
			.road_works = STR_ERROR_ROAD_WORKS_IN_PROGRESS,
			.oneway_road = STR_ERROR_CROSSING_ON_ONEWAY_ROAD,
			.crossing_rail = STR_ERROR_CROSSING_DISALLOWED_RAIL,
			.crossing_road = STR_ERROR_CROSSING_DISALLOWED_ROAD,
			.no_track = STR_ERROR_THERE_IS_NO_RAILROAD_TRACK,
			.owned_by = STR_ERROR_OWNED_BY,
			.flat_land = STR_ERROR_FLAT_LAND_REQUIRED,
			.demolish_bridge = STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST,
			.no_suitable_track = STR_ERROR_NO_SUITABLE_RAILROAD_TRACK,
			.no_signals = STR_ERROR_THERE_ARE_NO_SIGNALS,
			.owned_by_another = STR_ERROR_AREA_IS_OWNED_BY_ANOTHER,
			.remove_track = STR_ERROR_MUST_REMOVE_RAILROAD_TRACK,
			.demolish_building = STR_ERROR_BUILDING_MUST_BE_DEMOLISHED,
			.ship_in_way = STR_ERROR_SHIP_IN_THE_WAY,
		},
		.compatible_rail = RustRailCompatibleLegacy,
		.powered_rail = RustRailPoweredLegacy,
		.train_powered = [](Train *v, uint8_t rt) noexcept { return RustRailPowered(v->GetRailTypes(), static_cast<RailType>(rt)); },
		.rail_no_crossings = [](uint8_t rt) noexcept { return RailNoLevelCrossings(static_cast<RailType>(rt)); },
		.road_no_crossing = [](uint8_t rt) noexcept { return RoadNoLevelCrossing(static_cast<RoadType>(rt)); },
		.rail_build_cost = [](uint8_t rt) noexcept { return RailBuildCost(static_cast<RailType>(rt)).base(); },
		.rail_clear_cost = [](uint8_t rt) noexcept { return RailClearCost(static_cast<RailType>(rt)).base(); },
		.rail_convert_cost = [](uint8_t from, uint8_t to) noexcept { return RailConvertCost(static_cast<RailType>(from), static_cast<RailType>(to)).base(); },
		.crossing_with_competitor = RustCrossingWithCompetitor,
		.disable_elrails = RustDisableElrails,
		.slope_z_in_corner = [](uint8_t tileh, uint8_t corner) noexcept -> int32_t { return GetSlopeZInCorner(static_cast<Slope>(tileh), static_cast<Corner>(corner)); },
		.entrance_edge = [](uint32_t tile, int32_t z, uint8_t tileh, uint8_t dir) noexcept { return AutoslopeCheckForEntranceEdge(TileIndex{tile}, z, static_cast<Slope>(tileh), static_cast<DiagDirection>(dir)); },
		.build_rail = RailBuildSingle,
		.remove_rail = RailRemoveSingle,
		.build_signal = RailBuildSignal,
		.remove_signal = RailRemoveSignal,
		.convert_rail = RailConvert,
		.rail_infrastructure = RustAddRailInfrastructure,
		.road_infrastructure = RustAddRoadInfrastructure,
		.signal_infrastructure = RustAddSignalInfrastructure,

		.parking = RailParking,
		.train_position = [](Train *v) noexcept -> OpenTTDRailTrainPosition { return {.tile = RustVehicleTile(v), .trackdir = GetRustSharedServices().train_trackdir(v)}; },
		.layout_changed = [](uint32_t tile, uint8_t track) noexcept { YapfNotifyTrackLayoutChange(TileIndex{tile}, static_cast<Track>(track)); },
		.mark_crossings = [](uint32_t tile, uint8_t axis) noexcept { MarkDirtyAdjacentLevelCrossingTiles(TileIndex{tile}, static_cast<Axis>(axis)); },
		.crossing_removed = [](uint32_t tile, uint8_t axis) noexcept { UpdateAdjacentLevelCrossingTilesOnLevelCrossingRemoval(TileIndex{tile}, static_cast<Axis>(axis)); },
		.make_crossing = RailMakeCrossing,
		.make_road = RailMakeRoad,
		.depot_available = []() noexcept { return Depot::CanAllocateItem(); },
		.new_depot = [](uint32_t tile) noexcept { return (new Depot(TileIndex{tile}))->index.base(); },
		.depot_name = [](uint16_t id) noexcept { MakeDefaultName(Depot::Get(DepotID(id))); },
		.delete_depot = [](uint32_t tile) noexcept { delete Depot::GetByTile(TileIndex{tile}); },
		.depot_windows = RailDepotWindows,
		.close_inspect = OpenTTDDeleteRailInspect,
		.make_shore = RailMakeShore,
		.area_begin = [](OpenTTDRailConvertFrame *frame, uint32_t start, uint32_t end, bool diagonal) noexcept { frame->iter = TileIterator::Create(TileIndex{start}, TileIndex{end}, diagonal); },
		.area_tile = [](OpenTTDRailConvertFrame *frame) noexcept { return (**frame->iter).base(); },
		.area_next = [](OpenTTDRailConvertFrame *frame) noexcept { ++(*frame->iter); },
		.area_contains = [](uint32_t start, uint32_t end, bool diagonal, uint32_t tile) noexcept {
			if (diagonal) return DiagonalTileArea(TileIndex{start}, TileIndex{end}).Contains(TileIndex{tile});
			return OrthogonalTileArea(TileIndex{start}, TileIndex{end}).Contains(TileIndex{tile});
		},
		.include_train = RailIncludeTrain,
		.consist_changed = RailConsistChanged,
	};
	return services;
}

static const OpenTTDRailCommandServices *RailServices() noexcept { return &GetRustRailCommandServices(); }

CommandCost CmdBuildSingleRail(DoCommandFlags flags, TileIndex tile, RailType railtype, Track track, bool auto_remove_signals)
{
	CommandCost cost, ret, other;
	openttd_rust_rail_build_single(RailServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&other), flags.base(), tile.base(), railtype, track, auto_remove_signals);
	return cost;
}

CommandCost CmdRemoveSingleRail(DoCommandFlags flags, TileIndex tile, Track track)
{
	CommandCost cost, ret;
	openttd_rust_rail_remove_single(RailServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), track);
	return cost;
}

static CommandCost RailTrackDrag(DoCommandFlags flags, TileIndex tile, TileIndex end_tile, RailType railtype, Track track, bool remove, bool auto_remove_signals, bool fail_on_obstacle)
{
	CommandCost total, last_error, ret;
	openttd_rust_rail_track_drag(RailServices(), RustCommandCost(&total), RustCommandCost(&last_error), RustCommandCost(&ret), flags.base(), tile.base(), end_tile.base(), railtype, track, remove, auto_remove_signals, fail_on_obstacle);
	return total;
}

CommandCost CmdBuildRailroadTrack(DoCommandFlags flags, TileIndex end_tile, TileIndex start_tile, RailType railtype, Track track, bool auto_remove_signals, bool fail_on_obstacle)
{
	return RailTrackDrag(flags, start_tile, end_tile, railtype, track, false, auto_remove_signals, fail_on_obstacle);
}

CommandCost CmdRemoveRailroadTrack(DoCommandFlags flags, TileIndex end_tile, TileIndex start_tile, Track track)
{
	return RailTrackDrag(flags, start_tile, end_tile, INVALID_RAILTYPE, track, true, false, false);
}

CommandCost CmdBuildTrainDepot(DoCommandFlags flags, TileIndex tile, RailType railtype, DiagDirection dir)
{
	CommandCost cost, ret;
	openttd_rust_rail_build_depot(RailServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), railtype, dir);
	return cost;
}

CommandCost CmdBuildSingleSignal(DoCommandFlags flags, TileIndex tile, Track track, SignalType sigtype, SignalVariant sigvar, bool convert_signal, bool skip_existing_signals, bool ctrl_pressed, SignalType cycle_start, SignalType cycle_stop, uint8_t num_dir_cycle, uint8_t signals_copy)
{
	CommandCost cost, ret;
	openttd_rust_rail_build_signal(RailServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), track, sigtype, sigvar, convert_signal, skip_existing_signals, ctrl_pressed, cycle_start, cycle_stop, num_dir_cycle, signals_copy);
	return cost;
}

static CommandCost RailSignalDrag(DoCommandFlags flags, TileIndex tile, TileIndex end_tile, Track track, SignalType sigtype, SignalVariant sigvar, bool mode, bool remove, bool autofill, bool minimise_gaps, int signal_density)
{
	CommandCost total, last_error, ret;
	openttd_rust_rail_signal_drag(RailServices(), RustCommandCost(&total), RustCommandCost(&last_error), RustCommandCost(&ret), flags.base(), tile.base(), end_tile.base(), track, sigtype, sigvar, mode, remove, autofill, minimise_gaps, signal_density);
	return total;
}

CommandCost CmdBuildSignalTrack(DoCommandFlags flags, TileIndex tile, TileIndex end_tile, Track track, SignalType sigtype, SignalVariant sigvar, bool mode, bool autofill, bool minimise_gaps, uint8_t signal_density)
{
	return RailSignalDrag(flags, tile, end_tile, track, sigtype, sigvar, mode, false, autofill, minimise_gaps, signal_density);
}

CommandCost CmdRemoveSingleSignal(DoCommandFlags flags, TileIndex tile, Track track)
{
	CommandCost cost;
	openttd_rust_rail_remove_signal(RailServices(), RustCommandCost(&cost), flags.base(), tile.base(), track);
	return cost;
}

CommandCost CmdRemoveSignalTrack(DoCommandFlags flags, TileIndex tile, TileIndex end_tile, Track track, bool autofill)
{
	return RailSignalDrag(flags, tile, end_tile, track, SIGTYPE_BLOCK, SIG_ELECTRIC, false, true, autofill, false, 1);
}

CommandCost CmdConvertRail(DoCommandFlags flags, TileIndex tile, TileIndex area_start, RailType totype, bool diagonal)
{
	OpenTTDRailConvertFrame frame;
	CommandCost cost, error, ret;
	openttd_rust_rail_convert(RailServices(), &frame, RustCommandCost(&cost), RustCommandCost(&error), RustCommandCost(&ret), flags.base(), tile.base(), area_start.base(), totype, diagonal);
	return cost;
}



static CommandCost CheckBuildAbove_Track(TileIndex tile, DoCommandFlags flags, Axis, int)
{
	CommandCost cost;
	openttd_rust_rail_build_above(RailServices(), RustCommandCost(&cost), flags.base(), tile.base());
	return cost;
}

extern const TileTypeProcs _tile_type_rail_procs = {
	DrawTile_Track,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	nullptr,                     // add_accepted_cargo_proc
	GetTileDesc_Track,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	ClickTile_Track,          // click_tile_proc
	nullptr,                 // Rust scheduler calls the owner directly.
	nullptr,                 // Rust ownership dispatcher calls the owner directly.
	nullptr,                     // add_produced_cargo_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                 // Rust landscape commands call the owner directly.
	CheckBuildAbove_Track, // check_build_above_proc
};

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDRailCommandServices.hpp"
bool CheckAbi_rail_cmd() { return AbiServicesComplete(GetRustRailCommandServices()); }
