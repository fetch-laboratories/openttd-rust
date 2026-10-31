/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file tunnelbridge_cmd.cpp
 * This file deals with tunnels and bridges (non-gui stuff)
 * @todo separate this file into two
 */

#include "stdafx.h"
#include "viewport_func.h"
#include "command_func.h"
#include "town.h"
#include "train.h"
#include "ship.h"
#include "roadveh.h"
#include "pathfinder/yapf/yapf_cache.h"
#include "newgrf_sound.h"
#include "autoslope.h"
#include "tunnelbridge_map.h"
#include "strings_func.h"
#include "timer/timer_game_calendar.h"
#include "clear_func.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "tunnelbridge.h"
#include "cheat_type.h"
#include "elrail_func.h"
#include "pbs.h"
#include "company_base.h"
#include "newgrf_railtype.h"
#include "newgrf_roadtype.h"
#include "object_base.h"
#include "water.h"
#include "company_gui.h"
#include "tunnelbridge_cmd.h"
#include "landscape_cmd.h"
#include "terraform_cmd.h"

#include "table/strings.h"
#include "table/bridge_land.h"

#include "rust/ground_ffi.h"
#include "rust/services_landscape.h"
#include "rust/environment_services.h"
#include "rust/tunnelbridge_command_ffi.h"
#include "rust/command_services.h"
#include "rust/company_adapter.h"
#include "rust/road_command_services.h"
#include "rust/services_rail.h"
#include "rust/settings_services.h"
#include "rust/services_bridge.h"
#include "rust/services_viewport.h"

#include "safeguards.h"

BridgeSpec _bridge[MAX_BRIDGES]; ///< The specification of all bridges.

TileIndex GetBuildTunnelEndTile()
{
	return TileIndex{openttd_rust_tunnel_endtile()};
}

const OpenTTDTunnelBridgeServices &GetRustTunnelBridgeServices() noexcept
{
	static const OpenTTDTunnelBridgeServices services{
		.rail = &GetRustRailCommandServices(),
		.road = &GetRustRoadCommandServices(),
		.road_ground = &GetRustRoadGroundServices(),
		.water = &GetRustWaterCommandServices(),
		.company = &GetRustCompanyServices(),
		.availability = [](BridgeType type) noexcept -> OpenTTDBridgeAvailability { const auto *b = GetBridgeSpec(type); return {.year = b->avail_year.base(), .min = b->min_length, .max = b->max_length}; },
		.speed = RustBridgeSpecSpeed,
		.price = [](BridgeType type) noexcept -> uint16_t { return GetBridgeSpec(type)->price; },
		.max_bridge_length = RustMaxBridgeLength,
		.max_bridge_height = RustMaxBridgeHeight,
		.max_tunnel_length = RustMaxTunnelLength,
		.crossing_tunnels = []() noexcept -> bool { return _cheats.crossing_tunnels.value; },
		.other_bridge_end = RustOtherBridgeEnd,
		.northern_bridge_end = [](uint32_t t) noexcept -> uint32_t { return GetNorthernBridgeEnd(TileIndex{t}).base(); },
		.bridge_type = RustBridgeType,
		.is_tunnel = RustIsTunnel,
		.tunnel_in_way = [](uint32_t t, int32_t z, DiagDirection d) noexcept -> bool { return IsTunnelInWayDir(TileIndex{t}, z, d); },
		.set_middle = [](uint32_t t, Axis a) noexcept -> void { SetBridgeMiddle(TileIndex{t}, a); },
		.clear_middle = RustClearBridgeMiddle,
		.set_reservation = RustSetTunnelBridgeReservation,
		.make_rail_bridge = [](uint32_t t, uint8_t o, BridgeType b, DiagDirection d, RailType rt) noexcept -> void { MakeRailBridgeRamp(TileIndex{t}, Owner{o}, b, d, rt); },
		.make_road_bridge = [](uint32_t t, uint8_t o, uint8_t road, uint8_t tram, BridgeType b, DiagDirection d, RoadType rt, RoadType tt) noexcept -> void { MakeRoadBridgeRamp(TileIndex{t}, Owner{o}, Owner{road}, Owner{tram}, b, d, rt, tt); },
		.make_aqueduct = [](uint32_t t, uint8_t o, DiagDirection d) noexcept -> void { MakeAqueductBridgeRamp(TileIndex{t}, Owner{o}, d); },
		.make_rail_tunnel = [](uint32_t t, uint8_t o, DiagDirection d, RailType rt) noexcept -> void { MakeRailTunnel(TileIndex{t}, Owner{o}, d, rt); },
		.make_road_tunnel = [](uint32_t t, uint8_t o, DiagDirection d, RoadType rt, RoadType tt) noexcept -> void { MakeRoadTunnel(TileIndex{t}, Owner{o}, d, rt, tt); },
		.mark_bridge = [](uint32_t start, uint32_t end, DiagDirection d, uint32_t z) noexcept -> void { MarkBridgeDirty(TileIndex{start}, TileIndex{end}, d, z); },
		.mark_dirty_height = RustMarkTileDirtyHeight,
		.terraform = [](OpenTTDCommandCost *out, uint16_t flags, uint32_t t, Slope slope) noexcept -> void { *reinterpret_cast<CommandCost *>(out) = std::get<0>(Command<CMD_TERRAFORM_LAND>::Do(DoCommandFlags{flags}, TileIndex{t}, slope, false)); },
		.objects = &GetRustObjectServices(),
		.strings = {
			.bridge_border = STR_ERROR_BRIDGE_THROUGH_MAP_BORDER,
			.map_edge = STR_ERROR_TOO_CLOSE_TO_EDGE_OF_MAP,
			.same_end = STR_ERROR_CAN_T_START_AND_END_ON,
			.straight = STR_ERROR_START_AND_END_MUST_BE_IN,
			.too_long = STR_ERROR_BRIDGE_TOO_LONG,
			.wrong_slope = STR_ERROR_LAND_SLOPED_IN_WRONG_DIRECTION,
			.unequal_height = STR_ERROR_BRIDGEHEADS_NOT_SAME_HEIGHT,
			.demolish_bridge = STR_ERROR_MUST_DEMOLISH_BRIDGE_FIRST,
			.already_built = STR_ERROR_ALREADY_BUILT,
			.authority = STR_ERROR_LOCAL_AUTHORITY_REFUSES_TO_ALLOW_THIS,
			.other_owner = STR_ERROR_AREA_IS_OWNED_BY_ANOTHER,
			.too_low = STR_ERROR_BRIDGE_TOO_LOW_FOR_TERRAIN,
			.too_high = STR_ERROR_BRIDGE_TOO_HIGH_FOR_TERRAIN,
			.tunnel_site = STR_ERROR_SITE_UNSUITABLE_FOR_TUNNEL,
			.water = STR_ERROR_CAN_T_BUILD_ON_WATER,
			.tunnel_border = STR_ERROR_TUNNEL_THROUGH_MAP_BORDER,
			.tunnel_way = STR_ERROR_ANOTHER_TUNNEL_IN_THE_WAY,
			.tunnel_long = STR_ERROR_TUNNEL_TOO_LONG,
			.excavate = STR_ERROR_UNABLE_TO_EXCAVATE_LAND,
			.demolish_tunnel = STR_ERROR_MUST_DEMOLISH_TUNNEL_FIRST,
		},
	};
	return services;
}

/** Z position of the bridge sprites relative to bridge height (downwards) */
static const int BRIDGE_Z_START = 3;


/**
 * Mark bridge tiles dirty.
 * Note: The bridge does not need to exist, everything is passed via parameters.
 * @param begin Start tile.
 * @param end End tile.
 * @param direction Direction from \a begin to \a end.
 * @param bridge_height Bridge height level.
 */
void MarkBridgeDirty(TileIndex begin, TileIndex end, DiagDirection direction, uint bridge_height)
{
	TileIndexDiff delta = TileOffsByDiagDir(direction);
	for (TileIndex t = begin; t != end; t += delta) {
		MarkTileDirtyByTile(t, bridge_height - TileHeight(t));
	}
	MarkTileDirtyByTile(end);
}

/**
 * Mark bridge tiles dirty.
 * @param tile Bridge head.
 */
void MarkBridgeDirty(TileIndex tile)
{
	MarkBridgeDirty(tile, GetOtherTunnelBridgeEnd(tile), GetTunnelBridgeDirection(tile), GetBridgeHeight(tile));
}

/** Reset the data been eventually changed by the grf loaded. */
void ResetBridges()
{
	std::ranges::copy(_orig_bridge, std::begin(_bridge));
}

/**
 * Calculate the price factor for building a long bridge.
 * Basically the cost delta is 1,1, 1, 2,2, 3,3,3, 4,4,4,4, 5,5,5,5,5, 6,6,6,6,6,6,  7,7,7,7,7,7,7,  8,8,8,8,8,8,8,8,
 * @param length Length of the bridge.
 * @return Price factor for the bridge.
 */
int CalcBridgeLenCostFactor(int length)
{
	return openttd_rust_bridge_length_cost(length);
}

/**
 * Get the foundation for a bridge.
 * @param tileh The slope to build the bridge on.
 * @param axis The axis of the bridge entrance.
 * @return The foundation required.
 */
Foundation GetBridgeFoundation(Slope tileh, Axis axis)
{
	return openttd_rust_bridge_foundation(tileh, axis);
}

/**
 * Determines if the track on a bridge ramp is flat or goes up/down.
 *
 * @param tileh Slope of the tile under the bridge head
 * @param axis Orientation of bridge
 * @return true iff the track is flat.
 */
bool HasBridgeFlatRamp(Slope tileh, Axis axis)
{
	return openttd_rust_bridge_flat_ramp(&GetRustSharedServices(), tileh, axis);
}

/**
 * Test if bridge piece uses a custom sprite table.
 * @param bridge_type Bridge type.
 * @param piece Bridge piece.
 * @return True iff a custom sprite table is used for the bridge piece.
 */
static bool BridgeHasCustomSpriteTable(BridgeType bridge_type, BridgePieces piece)
{
	assert(piece < NUM_BRIDGE_PIECES);
	const BridgeSpec *bridge = GetBridgeSpec(bridge_type);
	return piece < bridge->sprite_table.size() && !bridge->sprite_table[piece].empty();
}

/**
 * Get the sprite table for a rail/road bridge piece.
 * @param bridge_type Bridge type.
 * @param piece Bridge piece.
 * @return Sprite table for the bridge piece.
 */
static std::span<const PalSpriteID> GetBridgeSpriteTable(BridgeType bridge_type, BridgePieces piece)
{
	assert(piece < NUM_BRIDGE_PIECES);

	const BridgeSpec *bridge = GetBridgeSpec(bridge_type);
	if (piece < bridge->sprite_table.size() && !bridge->sprite_table[piece].empty()) return bridge->sprite_table[piece];

	return _bridge_sprite_table[bridge_type][piece];
}

/**
 * Get the sprite table transport type base offset for a rail/road bridge.
 * @param transport_type Transport type of bridge.
 * @param ramp Tile of bridge ramp.
 * @return Offset for transport type.
 */
static uint8_t GetBridgeSpriteTableBaseOffset(TransportType transport_type, TileIndex ramp)
{
	switch (transport_type) {
		case TRANSPORT_RAIL: return GetRailTypeInfo(GetRailType(ramp))->bridge_offset;
		case TRANSPORT_ROAD: return 8;
		default: NOT_REACHED();
	}
}

/**
 * Get bridge sprite table base offset for the ramp part of bridge.
 * @param diagdir Direction of ramp.
 * @return Offset for direction.
 */
static uint8_t GetBridgeRampDirectionBaseOffset(DiagDirection diagdir)
{
	/* Bridge ramps are ordered SW, SE, NE, NW instead of NE, SE, SW, NW. */
	static constexpr uint8_t ramp_offsets[DIAGDIR_END] = {2, 1, 0, 3};
	return ramp_offsets[diagdir];
}

/**
 * Get bridge sprite table base offset for the middle part of bridge.
 * @param axis Axis of bridge.
 * @return Offset for axis.
 */
static uint8_t GetBridgeMiddleAxisBaseOffset(Axis axis)
{
	return axis == AXIS_X ? 0 : 4;
}

/**
 * Determines the foundation for the bridge head, and tests if the resulting slope is valid.
 *
 * @param bridge_piece Direction of the bridge head.
 * @param axis Axis of the bridge
 * @param tileh Slope of the tile under the north bridge head; returns slope on top of foundation
 * @param z TileZ corresponding to tileh, gets modified as well
 * @return Error or cost for bridge foundation
 */

/**
 * Is a bridge of the specified type and length available?
 * @param bridge_type Wanted type of bridge.
 * @param bridge_len  Wanted length of the bridge.
 * @param flags       Type of operation.
 * @return A succeeded (the requested bridge is available) or failed (it cannot be built) command.
 */
CommandCost CheckBridgeAvailability(BridgeType bridge_type, uint bridge_len, DoCommandFlags flags)
{
	CommandCost cost;
	openttd_rust_bridge_available(&GetRustTunnelBridgeServices(), RustCommandCost(&cost), bridge_type, bridge_len, flags.base());
	return cost;
}

/**
 * Calculate the base cost of clearing a tunnel/bridge per tile.
 * @param tile Start tile of the tunnel/bridge.
 * @return How much clearing this tunnel/bridge costs per tile.
 */


/**
 * Build a Bridge
 * @param flags type of operation
 * @param tile_end end tile
 * @param tile_start start tile
 * @param transport_type transport type.
 * @param bridge_type bridge type (hi bh)
 * @param road_rail_type rail type or road types.
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildBridge(DoCommandFlags flags, TileIndex tile_end, TileIndex tile_start, TransportType transport_type, BridgeType bridge_type, uint8_t road_rail_type)
{
	CommandCost cost, ret, north, south;
	openttd_rust_build_bridge(&GetRustTunnelBridgeServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&north), RustCommandCost(&south), flags.base(), tile_end.base(), tile_start.base(), transport_type, bridge_type, road_rail_type);
	return cost;
}


/**
 * Build Tunnel.
 * @param flags type of operation
 * @param start_tile start tile of tunnel
 * @param transport_type transport type
 * @param road_rail_type railtype or roadtype
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildTunnel(DoCommandFlags flags, TileIndex start_tile, TransportType transport_type, uint8_t road_rail_type)
{
	CommandCost cost, ret;
	openttd_rust_build_tunnel(&GetRustTunnelBridgeServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), start_tile.base(), transport_type, road_rail_type);
	return cost;
}


/**
 * Are we allowed to remove the tunnel or bridge at \a tile?
 * @param tile End point of the tunnel or bridge.
 * @return A succeeded command if the tunnel or bridge may be removed, a failed command otherwise.
 */

/**
 * Remove a tunnel from the game, update town rating, etc.
 * @param tile Tile containing one of the endpoints of the tunnel.
 * @param flags Command flags.
 * @return Succeeded or failed command.
 */


/**
 * Remove a bridge from the game, update town rating, etc.
 * @param tile Tile containing one of the endpoints of the bridge.
 * @param flags Command flags.
 * @return Succeeded or failed command.
 */

/**
 * Remove a tunnel or a bridge from the game.
 * @param tile Tile containing one of the endpoints.
 * @param flags Command flags.
 * @return Succeeded or failed command.
 */
static CommandCost ClearTile_TunnelBridge(TileIndex tile, DoCommandFlags flags)
{
	CommandCost cost, ret;
	openttd_rust_tunnelbridge_clear(&GetRustTunnelBridgeServices(), RustCommandCost(&cost), RustCommandCost(&ret), tile.base(), flags.base());
	return cost;
}

/**
 * Draw a single pillar sprite.
 * @param psid      Pillarsprite
 * @param x         Pillar X
 * @param y         Pillar Y
 * @param z         Pillar Z
 * @param w         Bounding box size in X direction
 * @param h         Bounding box size in Y direction
 * @param subsprite Optional subsprite for drawing halfpillars
 */
static inline void DrawPillar(const PalSpriteID &psid, int x, int y, int z, uint8_t w, uint8_t h, const SubSprite *subsprite)
{
	static const int PILLAR_Z_OFFSET = TILE_HEIGHT - BRIDGE_Z_START; ///< Start offset of pillar wrt. bridge (downwards)
	AddSortableSpriteToDraw(psid.sprite, psid.pal, x, y, z, {{0, 0, -PILLAR_Z_OFFSET}, {w, h, BB_HEIGHT_UNDER_BRIDGE}, {0, 0, PILLAR_Z_OFFSET}}, IsTransparencySet(TO_BRIDGES), subsprite);
}

/**
 * Draw two bridge pillars (north and south).
 * @param z_bottom Bottom Z
 * @param z_top    Top Z
 * @param psid     Pillarsprite
 * @param x        Pillar X
 * @param y        Pillar Y
 * @param w        Bounding box size in X direction
 * @param h        Bounding box size in Y direction
 * @return Reached Z at the bottom
 */
static int DrawPillarColumn(int z_bottom, int z_top, const PalSpriteID &psid, int x, int y, int w, int h)
{
	int cur_z;
	for (cur_z = z_top; cur_z >= z_bottom; cur_z -= TILE_HEIGHT) {
		DrawPillar(psid, x, y, cur_z, w, h, nullptr);
	}
	return cur_z;
}

/**
 * Draws the pillars under high bridges.
 *
 * @param psid Image and palette of a bridge pillar.
 * @param ti #TileInfo of current bridge-middle-tile.
 * @param axis Orientation of bridge.
 * @param drawfarpillar Whether to draw the pillar at the back
 * @param x Sprite X position of front pillar.
 * @param y Sprite Y position of front pillar.
 * @param z_bridge Absolute height of bridge bottom.
 */
static void DrawBridgePillars(const PalSpriteID &psid, const TileInfo *ti, Axis axis, bool drawfarpillar, int x, int y, int z_bridge)
{
	static const int bounding_box_size[2]  = {16, 2}; ///< bounding box size of pillars along bridge direction
	static const int back_pillar_offset[2] = { 0, 9}; ///< sprite position offset of back facing pillar

	static const int INF = 1000; ///< big number compared to sprite size
	static const SubSprite half_pillar_sub_sprite[2][2] = {
		{ {  -14, -INF, INF, INF }, { -INF, -INF, -15, INF } }, // X axis, north and south
		{ { -INF, -INF,  15, INF }, {   16, -INF, INF, INF } }, // Y axis, north and south
	};

	if (psid.sprite == 0) return;

	/* Determine ground height under pillars */
	DiagDirection south_dir = AxisToDiagDir(axis);
	int z_front_north = ti->z;
	int z_back_north = ti->z;
	int z_front_south = ti->z;
	int z_back_south = ti->z;
	GetSlopePixelZOnEdge(ti->tileh, south_dir, z_front_south, z_back_south);
	GetSlopePixelZOnEdge(ti->tileh, ReverseDiagDir(south_dir), z_front_north, z_back_north);

	/* Shared height of pillars */
	int z_front = std::max(z_front_north, z_front_south);
	int z_back = std::max(z_back_north, z_back_south);

	/* x and y size of bounding-box of pillars */
	int w = bounding_box_size[axis];
	int h = bounding_box_size[OtherAxis(axis)];
	/* sprite position of back facing pillar */
	int x_back = x - back_pillar_offset[axis];
	int y_back = y - back_pillar_offset[OtherAxis(axis)];

	/* Draw front pillars */
	int bottom_z = DrawPillarColumn(z_front, z_bridge, psid, x, y, w, h);
	if (z_front_north < z_front) DrawPillar(psid, x, y, bottom_z, w, h, &half_pillar_sub_sprite[axis][0]);
	if (z_front_south < z_front) DrawPillar(psid, x, y, bottom_z, w, h, &half_pillar_sub_sprite[axis][1]);

	/* Draw back pillars, skip top two parts, which are hidden by the bridge */
	int z_bridge_back = z_bridge - 2 * (int)TILE_HEIGHT;
	if (drawfarpillar && (z_back_north <= z_bridge_back || z_back_south <= z_bridge_back)) {
		bottom_z = DrawPillarColumn(z_back, z_bridge_back, psid, x_back, y_back, w, h);
		if (z_back_north < z_back) DrawPillar(psid, x_back, y_back, bottom_z, w, h, &half_pillar_sub_sprite[axis][0]);
		if (z_back_south < z_back) DrawPillar(psid, x_back, y_back, bottom_z, w, h, &half_pillar_sub_sprite[axis][1]);
	}
}

/**
 * Retrieve the sprites required for catenary on a road/tram bridge.
 * @param rti              RoadTypeInfo for the road or tram type to get catenary for
 * @param head_tile        Bridge head tile with roadtype information
 * @param offset           Sprite offset identifying flat to sloped bridge tiles
 * @param head             Are we drawing bridge head?
 * @param[out] spr_back    Back catenary sprite to use
 * @param[out] spr_front   Front catenary sprite to use
 */
static void GetBridgeRoadCatenary(const RoadTypeInfo *rti, TileIndex head_tile, int offset, bool head, SpriteID &spr_back, SpriteID &spr_front)
{
	static const SpriteID back_offsets[6]  = { 95,  96,  99, 102, 100, 101 };
	static const SpriteID front_offsets[6] = { 97,  98, 103, 106, 104, 105 };

	/* Simplified from DrawRoadTypeCatenary() to remove all the special cases required for regular ground road */
	spr_back = GetCustomRoadSprite(rti, head_tile, ROTSG_CATENARY_BACK, head ? TCX_NORMAL : TCX_ON_BRIDGE);
	spr_front = GetCustomRoadSprite(rti, head_tile, ROTSG_CATENARY_FRONT, head ? TCX_NORMAL : TCX_ON_BRIDGE);
	if (spr_back == 0 && spr_front == 0) {
		spr_back = SPR_TRAMWAY_BASE + back_offsets[offset];
		spr_front = SPR_TRAMWAY_BASE + front_offsets[offset];
	} else {
		if (spr_back != 0) spr_back += 23 + offset;
		if (spr_front != 0) spr_front += 23 + offset;
	}
}

/**
 * Draws the road and trambits over an already drawn (lower end) of a bridge.
 * @param head_tile    bridge head tile with roadtype information
 * @param x            the x of the bridge
 * @param y            the y of the bridge
 * @param z            the z of the bridge
 * @param offset       sprite offset identifying flat to sloped bridge tiles
 * @param head         are we drawing bridge head?
 * @param is_custom_layout Set if the bridge uses a custom sprite layout.
 */
static void DrawBridgeRoadBits(TileIndex head_tile, int x, int y, int z, int offset, bool head, bool is_custom_layout)
{
	RoadType road_rt = GetRoadTypeRoad(head_tile);
	RoadType tram_rt = GetRoadTypeTram(head_tile);
	const RoadTypeInfo *road_rti = road_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(road_rt);
	const RoadTypeInfo *tram_rti = tram_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(tram_rt);

	SpriteID seq_back[4] = { 0 };
	bool trans_back[4] = { false };
	SpriteID seq_front[4] = { 0 };
	bool trans_front[4] = { false };

	static const SpriteID overlay_offsets[6] = {   0,   1,  11,  12,  13,  14 };
	if (head || !IsInvisibilitySet(TO_BRIDGES)) {
		/* Road underlay takes precedence over tram */
		trans_back[0] = !head && IsTransparencySet(TO_BRIDGES);
		if (road_rti != nullptr) {
			if (road_rti->UsesOverlay()) {
				seq_back[0] = GetCustomRoadSprite(road_rti, head_tile, ROTSG_BRIDGE, head ? TCX_NORMAL : TCX_ON_BRIDGE) + offset;
			} else if (is_custom_layout) {
				/* For custom layouts draw a custom bridge deck. */
				seq_back[0] = SPR_BRIDGE_DECKS_ROAD + offset;
			}
		} else if (tram_rti != nullptr) {
			if (tram_rti->UsesOverlay()) {
				seq_back[0] = GetCustomRoadSprite(tram_rti, head_tile, ROTSG_BRIDGE, head ? TCX_NORMAL : TCX_ON_BRIDGE) + offset;
			} else {
				seq_back[0] = SPR_TRAMWAY_BRIDGE + offset;
			}
		}

		/* Draw road overlay */
		trans_back[1] = !head && IsTransparencySet(TO_BRIDGES);
		if (road_rti != nullptr) {
			if (road_rti->UsesOverlay()) {
				seq_back[1] = GetCustomRoadSprite(road_rti, head_tile, ROTSG_OVERLAY, head ? TCX_NORMAL : TCX_ON_BRIDGE);
				if (seq_back[1] != 0) seq_back[1] += overlay_offsets[offset];
			}
		}

		/* Draw tram overlay */
		trans_back[2] = !head && IsTransparencySet(TO_BRIDGES);
		if (tram_rti != nullptr) {
			if (tram_rti->UsesOverlay()) {
				seq_back[2] = GetCustomRoadSprite(tram_rti, head_tile, ROTSG_OVERLAY, head ? TCX_NORMAL : TCX_ON_BRIDGE);
				if (seq_back[2] != 0) seq_back[2] += overlay_offsets[offset];
			} else if (road_rti != nullptr) {
				seq_back[2] = SPR_TRAMWAY_OVERLAY + overlay_offsets[offset];
			}
		}

		/* Road catenary takes precedence over tram */
		trans_back[3] = IsTransparencySet(TO_CATENARY);
		trans_front[0] = IsTransparencySet(TO_CATENARY);
		if (road_rti != nullptr && HasRoadCatenaryDrawn(road_rt)) {
			GetBridgeRoadCatenary(road_rti, head_tile, offset, head, seq_back[3], seq_front[0]);
		} else if (tram_rti != nullptr && HasRoadCatenaryDrawn(tram_rt)) {
			GetBridgeRoadCatenary(tram_rti, head_tile, offset, head, seq_back[3], seq_front[0]);
		}
	}

	static constexpr SpriteBounds back_bounds[6] = {
		{{}, {0, TILE_SIZE, 40}, {}},
		{{}, {TILE_SIZE, 0, 40}, {}},
		{{}, {TILE_SIZE, 0, 40}, {}},
		{{}, {0, TILE_SIZE, 40}, {}},
		{{}, {TILE_SIZE, 0, 40}, {}},
		{{}, {0, TILE_SIZE, 40}, {}},
	};

	/* The sprites under the vehicles are drawn as SpriteCombine. StartSpriteCombine() has already been called
	 * The bounding boxes here are the same as for bridge front/roof */
	for (uint i = 0; i < lengthof(seq_back); ++i) {
		if (seq_back[i] != 0) {
			AddSortableSpriteToDraw(seq_back[i], PAL_NONE, x, y, z, back_bounds[offset], trans_back[i]);
		}
	}

	/* Start a new SpriteCombine for the front part */
	EndSpriteCombine();
	StartSpriteCombine();

	static constexpr SpriteBounds front_bounds[6] = {
		{{15, 0, 0}, {0, TILE_SIZE, 40}, {-15, 0, 0}},
		{{0, 15, 0}, {TILE_SIZE, 0, 40}, {0, -15, 0}},
		{{0, 15, 0}, {TILE_SIZE, 0, 40}, {0, -15, 0}},
		{{15, 0, 0}, {0, TILE_SIZE, 40}, {-15, 0, 0}},
		{{0, 15, 0}, {TILE_SIZE, 0, 40}, {0, -15, 0}},
		{{15, 0, 0}, {0, TILE_SIZE, 40}, {-15, 0, 0}},
	};

	for (uint i = 0; i < lengthof(seq_front); ++i) {
		if (seq_front[i] != 0) {
			AddSortableSpriteToDraw(seq_front[i], PAL_NONE, x, y, z, front_bounds[offset], trans_front[i]);
		}
	}
}

/**
 * Draws a tunnel of bridge tile.
 * For tunnels, this is rather simple, as you only need to draw the entrance.
 * Bridges are a bit more complex. base_offset is where the sprite selection comes into play
 * and it works a bit like a bitmask.<p> For bridge heads:
 * @param ti TileInfo of the structure to draw
 * <ul><li>Bit 0: direction</li>
 * <li>Bit 1: northern or southern heads</li>
 * <li>Bit 2: Set if the bridge head is sloped</li>
 * <li>Bit 3 and more: Railtype Specific subset</li>
 * </ul>
 * Please note that in this code, "roads" are treated as railtype 1, whilst the real railtypes are 0, 2 and 3
 */
static void DrawTile_TunnelBridge(TileInfo *ti)
{
	TransportType transport_type = GetTunnelBridgeTransportType(ti->tile);
	DiagDirection tunnelbridge_direction = GetTunnelBridgeDirection(ti->tile);

	if (IsTunnel(ti->tile)) {
		/* Front view of tunnel bounding boxes:
		 *
		 *   122223  <- BB_Z_SEPARATOR
		 *   1    3
		 *   1    3                1,3 = empty helper BB
		 *   1    3                  2 = SpriteCombine of tunnel-roof and catenary (tram & elrail)
		 *
		 */

		 /* Tunnel sprites are positioned at 15,15, but the bounding box covers most of the tile. */
		static constexpr SpriteBounds roof_bounds[DIAGDIR_END] = {
			{{0, 1, BB_Z_SEPARATOR}, {TILE_SIZE, TILE_SIZE - 1, 1}, {TILE_SIZE - 1, TILE_SIZE - 2, -BB_Z_SEPARATOR}}, // NE
			{{1, 0, BB_Z_SEPARATOR}, {TILE_SIZE - 1, TILE_SIZE, 1}, {TILE_SIZE - 2, TILE_SIZE - 1, -BB_Z_SEPARATOR}}, // SE
			{{0, 1, BB_Z_SEPARATOR}, {TILE_SIZE, TILE_SIZE - 1, 1}, {TILE_SIZE - 1, TILE_SIZE - 2, -BB_Z_SEPARATOR}}, // SW
			{{1, 0, BB_Z_SEPARATOR}, {TILE_SIZE - 1, TILE_SIZE, 1}, {TILE_SIZE - 2, TILE_SIZE - 1, -BB_Z_SEPARATOR}}, // NW
		};

		/* Catenary sprites are positioned at 0,0, with the same bounding box as above. */
		static constexpr SpriteBounds catenary_bounds[DIAGDIR_END] = {
			{{0, 1, BB_Z_SEPARATOR}, {TILE_SIZE, TILE_SIZE - 1, 1}, {0, -1, -BB_Z_SEPARATOR}}, // NE
			{{1, 0, BB_Z_SEPARATOR}, {TILE_SIZE - 1, TILE_SIZE, 1}, {-1, 0, -BB_Z_SEPARATOR}}, // SE
			{{0, 1, BB_Z_SEPARATOR}, {TILE_SIZE, TILE_SIZE - 1, 1}, {0, -1, -BB_Z_SEPARATOR}}, // SW
			{{1, 0, BB_Z_SEPARATOR}, {TILE_SIZE - 1, TILE_SIZE, 1}, {-1, 0, -BB_Z_SEPARATOR}}, // NW
		};

		static constexpr SpriteBounds rear_sep[DIAGDIR_END] = {
			{{}, {TILE_SIZE, 1, TILE_HEIGHT}, {}}, // NE
			{{}, {1, TILE_SIZE, TILE_HEIGHT}, {}}, // SE
			{{}, {TILE_SIZE, 1, TILE_HEIGHT}, {}}, // SW
			{{}, {1, TILE_SIZE, TILE_HEIGHT}, {}}, // NW
		};

		static constexpr SpriteBounds front_sep[DIAGDIR_END] = {
			{{0, TILE_SIZE - 1, 0}, {TILE_SIZE, 1, TILE_HEIGHT}, {}}, // NE
			{{TILE_SIZE - 1, 0, 0}, {1, TILE_SIZE, TILE_HEIGHT}, {}}, // SE
			{{0, TILE_SIZE - 1, 0}, {TILE_SIZE, 1, TILE_HEIGHT}, {}}, // SW
			{{TILE_SIZE - 1, 0, 0}, {1, TILE_SIZE, TILE_HEIGHT}, {}}, // NW
		};

		bool catenary = false;

		SpriteID image;
		SpriteID railtype_overlay = 0;
		if (transport_type == TRANSPORT_RAIL) {
			const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(ti->tile));
			image = rti->base_sprites.tunnel;
			if (rti->UsesOverlay()) {
				/* Check if the railtype has custom tunnel portals. */
				railtype_overlay = GetCustomRailSprite(rti, ti->tile, RTSG_TUNNEL_PORTAL);
				if (railtype_overlay != 0) image = SPR_RAILTYPE_TUNNEL_BASE; // Draw blank grass tunnel base.
			}
		} else {
			image = SPR_TUNNEL_ENTRY_REAR_ROAD;
		}

		if (HasTunnelBridgeSnowOrDesert(ti->tile)) image += railtype_overlay != 0 ? 8 : 32;

		image += tunnelbridge_direction * 2;
		DrawGroundSprite(image, PAL_NONE);

		if (transport_type == TRANSPORT_ROAD) {
			RoadType road_rt = GetRoadTypeRoad(ti->tile);
			RoadType tram_rt = GetRoadTypeTram(ti->tile);
			const RoadTypeInfo *road_rti = road_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(road_rt);
			const RoadTypeInfo *tram_rti = tram_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(tram_rt);
			uint sprite_offset = DiagDirToAxis(tunnelbridge_direction) == AXIS_X ? 1 : 0;
			bool draw_underlay = true;

			/* Road underlay takes precedence over tram */
			if (road_rti != nullptr) {
				if (road_rti->UsesOverlay()) {
					SpriteID ground = GetCustomRoadSprite(road_rti, ti->tile, ROTSG_TUNNEL);
					if (ground != 0) {
						DrawGroundSprite(ground + tunnelbridge_direction, PAL_NONE);
						draw_underlay = false;
					}
				}
			} else {
				if (tram_rti->UsesOverlay()) {
					SpriteID ground = GetCustomRoadSprite(tram_rti, ti->tile, ROTSG_TUNNEL);
					if (ground != 0) {
						DrawGroundSprite(ground + tunnelbridge_direction, PAL_NONE);
						draw_underlay = false;
					}
				}
			}

			DrawRoadOverlays(ti, PAL_NONE, road_rti, tram_rti, sprite_offset, sprite_offset, draw_underlay);

			/* Road catenary takes precedence over tram */
			SpriteID catenary_sprite_base = 0;
			if (road_rti != nullptr && HasRoadCatenaryDrawn(road_rt)) {
				catenary_sprite_base = GetCustomRoadSprite(road_rti, ti->tile, ROTSG_CATENARY_FRONT);
				if (catenary_sprite_base == 0) {
					catenary_sprite_base = SPR_TRAMWAY_TUNNEL_WIRES;
				} else {
					catenary_sprite_base += 19;
				}
			} else if (tram_rti != nullptr && HasRoadCatenaryDrawn(tram_rt)) {
				catenary_sprite_base = GetCustomRoadSprite(tram_rti, ti->tile, ROTSG_CATENARY_FRONT);
				if (catenary_sprite_base == 0) {
					catenary_sprite_base = SPR_TRAMWAY_TUNNEL_WIRES;
				} else {
					catenary_sprite_base += 19;
				}
			}

			if (catenary_sprite_base != 0) {
				catenary = true;
				StartSpriteCombine();
				AddSortableSpriteToDraw(catenary_sprite_base + tunnelbridge_direction, PAL_NONE, *ti, catenary_bounds[tunnelbridge_direction], IsTransparencySet(TO_CATENARY));
			}
		} else {
			const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(ti->tile));
			if (rti->UsesOverlay()) {
				SpriteID surface = GetCustomRailSprite(rti, ti->tile, RTSG_TUNNEL);
				if (surface != 0) DrawGroundSprite(surface + tunnelbridge_direction, PAL_NONE);
			}

			/* PBS debugging, draw reserved tracks darker */
			if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation && HasTunnelBridgeReservation(ti->tile)) {
				if (rti->UsesOverlay()) {
					SpriteID overlay = GetCustomRailSprite(rti, ti->tile, RTSG_OVERLAY);
					DrawGroundSprite(overlay + RTO_X + DiagDirToAxis(tunnelbridge_direction), PALETTE_CRASH);
				} else {
					DrawGroundSprite(DiagDirToAxis(tunnelbridge_direction) == AXIS_X ? rti->base_sprites.single_x : rti->base_sprites.single_y, PALETTE_CRASH);
				}
			}

			if (HasRailCatenaryDrawn(GetRailType(ti->tile))) {
				/* Maybe draw pylons on the entry side */
				DrawRailCatenary(ti);

				catenary = true;
				StartSpriteCombine();
				/* Draw wire above the ramp */
				DrawRailCatenaryOnTunnel(ti);
			}
		}

		if (railtype_overlay != 0 && !catenary) StartSpriteCombine();

		AddSortableSpriteToDraw(image + 1, PAL_NONE, *ti, roof_bounds[tunnelbridge_direction], false);
		/* Draw railtype tunnel portal overlay if defined. */
		if (railtype_overlay != 0) AddSortableSpriteToDraw(railtype_overlay + tunnelbridge_direction, PAL_NONE, *ti, roof_bounds[tunnelbridge_direction], false);

		if (catenary || railtype_overlay != 0) EndSpriteCombine();

		/* Add helper BB for sprite sorting that separates the tunnel from things beside of it. */
		AddSortableSpriteToDraw(SPR_EMPTY_BOUNDING_BOX, PAL_NONE, *ti, rear_sep[tunnelbridge_direction]);
		AddSortableSpriteToDraw(SPR_EMPTY_BOUNDING_BOX, PAL_NONE, *ti, front_sep[tunnelbridge_direction]);

		DrawBridgeMiddle(ti, BridgePillarFlag::EdgeNE + tunnelbridge_direction);
	} else { // IsBridge(ti->tile)
		DrawFoundation(ti, GetBridgeFoundation(ti->tileh, DiagDirToAxis(tunnelbridge_direction)));
		bool is_custom_layout = false; // Set if rail/road bridge uses a custom layout.

		uint base_offset = GetBridgeRampDirectionBaseOffset(tunnelbridge_direction);
		std::span<const PalSpriteID> psid;
		if (transport_type != TRANSPORT_WATER) {
			BridgeType bridge_type = GetBridgeType(ti->tile);
			if (ti->tileh == SLOPE_FLAT) base_offset += 4; // sloped bridge head
			base_offset += GetBridgeSpriteTableBaseOffset(transport_type, ti->tile);
			psid = GetBridgeSpriteTable(bridge_type, BRIDGE_PIECE_HEAD);
			is_custom_layout = BridgeHasCustomSpriteTable(bridge_type, BRIDGE_PIECE_HEAD);
		} else {
			psid = _aqueduct_sprite_table_heads;
		}
		psid = psid.subspan(base_offset, 1);

		if (!HasTunnelBridgeSnowOrDesert(ti->tile)) {
			TileIndex next = ti->tile + TileOffsByDiagDir(tunnelbridge_direction);
			if (ti->tileh != SLOPE_FLAT && ti->z == 0 && HasTileWaterClass(next) && GetWaterClass(next) == WaterClass::Sea) {
				DrawShoreTile(ti->tileh);
			} else {
				DrawClearLandTile(ti, 3);
			}
		} else {
			DrawGroundSprite(SPR_FLAT_SNOW_DESERT_TILE + SlopeToSpriteOffset(ti->tileh), PAL_NONE);
		}

		/* draw ramp */

		/* Draw Trambits and PBS Reservation as SpriteCombine */
		if (transport_type == TRANSPORT_ROAD || transport_type == TRANSPORT_RAIL) StartSpriteCombine();

		/* HACK set the height of the BB of a sloped ramp to 1 so a vehicle on
		 * it doesn't disappear behind it
		 */
		/* Bridge heads are drawn solid no matter how invisibility/transparency is set */
		AddSortableSpriteToDraw(psid[0].sprite, psid[0].pal, *ti, {{}, {TILE_SIZE, TILE_SIZE, static_cast<uint8_t>(ti->tileh == SLOPE_FLAT ? 0 : TILE_HEIGHT)}, {}});

		if (transport_type == TRANSPORT_ROAD) {
			uint offset = tunnelbridge_direction;
			int z = ti->z;
			if (ti->tileh != SLOPE_FLAT) {
				offset = (offset + 1) & 1;
				z += TILE_HEIGHT;
			} else {
				offset += 2;
			}

			/* DrawBridgeRoadBits() calls EndSpriteCombine() and StartSpriteCombine() */
			DrawBridgeRoadBits(ti->tile, ti->x, ti->y, z, offset, true, is_custom_layout);

			EndSpriteCombine();
		} else if (transport_type == TRANSPORT_RAIL) {
			const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(ti->tile));
			if (is_custom_layout || rti->UsesOverlay()) {
				SpriteID surface = rti->UsesOverlay() ? GetCustomRailSprite(rti, ti->tile, RTSG_BRIDGE) : rti->base_sprites.bridge_deck;
				if (surface != 0) {
					if (HasBridgeFlatRamp(ti->tileh, DiagDirToAxis(tunnelbridge_direction))) {
						AddSortableSpriteToDraw(surface + ((DiagDirToAxis(tunnelbridge_direction) == AXIS_X) ? RTBO_X : RTBO_Y), PAL_NONE, *ti, {{0, 0, TILE_HEIGHT}, {TILE_SIZE, TILE_SIZE, 0}, {}});
					} else {
						AddSortableSpriteToDraw(surface + RTBO_SLOPE + tunnelbridge_direction, PAL_NONE, *ti, {{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT}, {}});
					}
				}
			}

			/* PBS debugging, draw reserved tracks darker */
			if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation && HasTunnelBridgeReservation(ti->tile)) {
				if (rti->UsesOverlay()) {
					SpriteID overlay = GetCustomRailSprite(rti, ti->tile, RTSG_OVERLAY);
					if (HasBridgeFlatRamp(ti->tileh, DiagDirToAxis(tunnelbridge_direction))) {
						AddSortableSpriteToDraw(overlay + RTO_X + DiagDirToAxis(tunnelbridge_direction), PALETTE_CRASH, *ti, {{0, 0, TILE_HEIGHT}, {TILE_SIZE, TILE_SIZE, 0}, {}});
					} else {
						AddSortableSpriteToDraw(overlay + RTO_SLOPE_NE + tunnelbridge_direction, PALETTE_CRASH, *ti, {{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT}, {}});
					}
				} else {
					if (HasBridgeFlatRamp(ti->tileh, DiagDirToAxis(tunnelbridge_direction))) {
						AddSortableSpriteToDraw(DiagDirToAxis(tunnelbridge_direction) == AXIS_X ? rti->base_sprites.single_x : rti->base_sprites.single_y, PALETTE_CRASH, *ti, {{0, 0, TILE_HEIGHT}, {TILE_SIZE, TILE_SIZE, 0}, {}});
					} else {
						AddSortableSpriteToDraw(rti->base_sprites.single_sloped + tunnelbridge_direction, PALETTE_CRASH, *ti, {{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT}, {}});
					}
				}
			}

			EndSpriteCombine();
			if (HasRailCatenaryDrawn(GetRailType(ti->tile))) {
				DrawRailCatenary(ti);
			}
		}

		BridgePillarFlags blocked_pillars;
		if (DiagDirToAxis(tunnelbridge_direction) == AXIS_X) {
			blocked_pillars = {BridgePillarFlag::EdgeSW, BridgePillarFlag::EdgeNE};
		} else {
			blocked_pillars = {BridgePillarFlag::EdgeNW, BridgePillarFlag::EdgeSE};
		}
		DrawBridgeMiddle(ti, blocked_pillars);
	}
}


/**
 * Compute bridge piece. Computes the bridge piece to display depending on the position inside the bridge.
 * bridges pieces sequence (middle parts).
 * Note that it is not covering the bridge heads, which are always referenced by the same sprite table.
 * bridge len 1: BRIDGE_PIECE_NORTH
 * bridge len 2: BRIDGE_PIECE_NORTH  BRIDGE_PIECE_SOUTH
 * bridge len 3: BRIDGE_PIECE_NORTH  BRIDGE_PIECE_MIDDLE_ODD   BRIDGE_PIECE_SOUTH
 * bridge len 4: BRIDGE_PIECE_NORTH  BRIDGE_PIECE_INNER_NORTH  BRIDGE_PIECE_INNER_SOUTH  BRIDGE_PIECE_SOUTH
 * bridge len 5: BRIDGE_PIECE_NORTH  BRIDGE_PIECE_INNER_NORTH  BRIDGE_PIECE_MIDDLE_EVEN  BRIDGE_PIECE_INNER_SOUTH  BRIDGE_PIECE_SOUTH
 * bridge len 6: BRIDGE_PIECE_NORTH  BRIDGE_PIECE_INNER_NORTH  BRIDGE_PIECE_INNER_SOUTH  BRIDGE_PIECE_INNER_NORTH  BRIDGE_PIECE_INNER_SOUTH  BRIDGE_PIECE_SOUTH
 * bridge len 7: BRIDGE_PIECE_NORTH  BRIDGE_PIECE_INNER_NORTH  BRIDGE_PIECE_INNER_SOUTH  BRIDGE_PIECE_MIDDLE_ODD   BRIDGE_PIECE_INNER_NORTH  BRIDGE_PIECE_INNER_SOUTH  BRIDGE_PIECE_SOUTH
 * #0 - always as first, #1 - always as last (if len>1)
 * #2,#3 are to pair in order
 * for odd bridges: #5 is going in the bridge middle if on even position, #4 on odd (counting from 0)
 * @param north Northernmost tile of bridge
 * @param south Southernmost tile of bridge
 * @return Index of bridge piece
 */
static BridgePieces CalcBridgePiece(uint north, uint south)
{
	if (north == 1) {
		return BRIDGE_PIECE_NORTH;
	} else if (south == 1) {
		return BRIDGE_PIECE_SOUTH;
	} else if (north < south) {
		return north & 1 ? BRIDGE_PIECE_INNER_SOUTH : BRIDGE_PIECE_INNER_NORTH;
	} else if (north > south) {
		return south & 1 ? BRIDGE_PIECE_INNER_NORTH : BRIDGE_PIECE_INNER_SOUTH;
	} else {
		return north & 1 ? BRIDGE_PIECE_MIDDLE_EVEN : BRIDGE_PIECE_MIDDLE_ODD;
	}
}

/**
 * Get pillar information for a bridge middle tile.
 * @param tile Tile of bridge middle.
 * @param rampnorth Northern ramp tile of bridge.
 * @param rampsouth Southern ramp tile of bridge.
 * @param type Bridge type.
 * @param transport_type Transport type of bridge.
 * @return Pillar flags for bridge middle.
 */
static BridgePillarFlags GetBridgeTilePillarFlags(TileIndex tile, TileIndex rampnorth, TileIndex rampsouth, BridgeType type, TransportType transport_type)
{
	if (transport_type == TRANSPORT_WATER) return BRIDGEPILLARFLAGS_ALL_CORNERS;

	const BridgeSpec *spec = GetBridgeSpec(type);
	if (!spec->ctrl_flags.Test(BridgeSpec::ControlFlag::InvalidPillarFlags)) {
		BridgePieces piece = CalcBridgePiece(GetTunnelBridgeLength(tile, rampnorth) + 1, GetTunnelBridgeLength(tile, rampsouth) + 1);
		Axis axis = TileX(rampnorth) == TileX(rampsouth) ? AXIS_Y : AXIS_X;

		return spec->pillar_flags[piece][axis == AXIS_Y ? 1 : 0];
	}

	return BRIDGEPILLARFLAGS_ALL_CORNERS;
}

/**
 * Draw the middle bits of a bridge.
 * @param ti Tile information of the tile to draw it on.
 * @param blocked_pillars Mask of pillar corners and edges blocked by tile below the bridge.
 */
void DrawBridgeMiddle(const TileInfo *ti, BridgePillarFlags blocked_pillars)
{
	/* Sectional view of bridge bounding boxes:
	 *
	 *  1           2                                1,2 = SpriteCombine of Bridge front/(back&floor) and RoadCatenary
	 *  1           2                                  3 = empty helper BB
	 *  1     7     2                                4,5 = pillars under higher bridges
	 *  1 6 88888 6 2                                  6 = elrail-pylons
	 *  1 6 88888 6 2                                  7 = elrail-wire
	 *  1 6 88888 6 2  <- TILE_HEIGHT                  8 = rail-vehicle on bridge
	 *  3333333333333  <- BB_Z_SEPARATOR
	 *                 <- unused
	 *    4       5    <- BB_HEIGHT_UNDER_BRIDGE
	 *    4       5
	 *    4       5
	 *
	 */

	if (!IsBridgeAbove(ti->tile)) return;

	TileIndex rampnorth = GetNorthernBridgeEnd(ti->tile);
	TileIndex rampsouth = GetSouthernBridgeEnd(ti->tile);
	TransportType transport_type = GetTunnelBridgeTransportType(rampsouth);
	Axis axis = GetBridgeAxis(ti->tile);
	BridgePillarFlags pillars;
	bool is_custom_layout; // Set if rail/road bridge uses a custom layout.

	uint base_offset = GetBridgeMiddleAxisBaseOffset(axis);
	std::span<const PalSpriteID> psid;
	bool drawfarpillar;
	if (transport_type != TRANSPORT_WATER) {
		BridgeType bridge_type = GetBridgeType(rampsouth);
		BridgePieces bridge_piece = CalcBridgePiece(GetTunnelBridgeLength(ti->tile, rampnorth) + 1, GetTunnelBridgeLength(ti->tile, rampsouth) + 1);
		drawfarpillar = !HasBit(GetBridgeSpec(bridge_type)->flags, 0);
		base_offset += GetBridgeSpriteTableBaseOffset(transport_type, rampsouth);
		psid = GetBridgeSpriteTable(bridge_type, bridge_piece);
		pillars = GetBridgeTilePillarFlags(ti->tile, rampnorth, rampsouth, bridge_type, transport_type);
		is_custom_layout = BridgeHasCustomSpriteTable(bridge_type, bridge_piece);
	} else {
		drawfarpillar = true;
		psid = _aqueduct_sprite_table_middle;
		pillars = BRIDGEPILLARFLAGS_ALL_CORNERS;
	}
	psid = psid.subspan(base_offset, 3);

	int x = ti->x;
	int y = ti->y;
	uint bridge_z = GetBridgePixelHeight(rampsouth);
	int z = bridge_z - BRIDGE_Z_START;

	/* Add a bounding box that separates the bridge from things below it. */
	AddSortableSpriteToDraw(SPR_EMPTY_BOUNDING_BOX, PAL_NONE, x, y, bridge_z - TILE_HEIGHT + BB_Z_SEPARATOR, {{}, {TILE_SIZE, TILE_SIZE, 1}, {}});

	/* Draw Trambits as SpriteCombine */
	if (transport_type == TRANSPORT_ROAD || transport_type == TRANSPORT_RAIL) StartSpriteCombine();

	/* Draw floor and far part of bridge*/
	if (!IsInvisibilitySet(TO_BRIDGES)) {
		if (axis == AXIS_X) {
			AddSortableSpriteToDraw(psid[0].sprite, psid[0].pal, x, y, z, {{0, 0, BRIDGE_Z_START}, {TILE_SIZE, 1, 40}, {0, 0, -BRIDGE_Z_START}}, IsTransparencySet(TO_BRIDGES));
		} else {
			AddSortableSpriteToDraw(psid[0].sprite, psid[0].pal, x, y, z, {{0, 0, BRIDGE_Z_START}, {1, TILE_SIZE, 40}, {0, 0, -BRIDGE_Z_START}}, IsTransparencySet(TO_BRIDGES));
		}
	}

	if (transport_type == TRANSPORT_ROAD) {
		/* DrawBridgeRoadBits() calls EndSpriteCombine() and StartSpriteCombine() */
		DrawBridgeRoadBits(rampsouth, x, y, bridge_z, axis ^ 1, false, is_custom_layout);
	} else if (transport_type == TRANSPORT_RAIL) {
		const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(rampsouth));
		if ((is_custom_layout || rti->UsesOverlay()) && !IsInvisibilitySet(TO_BRIDGES)) {
			SpriteID surface = rti->UsesOverlay() ? GetCustomRailSprite(rti, rampsouth, RTSG_BRIDGE, TCX_ON_BRIDGE) : rti->base_sprites.bridge_deck;
			if (surface != 0) {
				AddSortableSpriteToDraw(surface + axis, PAL_NONE, x, y, bridge_z, {{}, {TILE_SIZE, TILE_SIZE, 0}, {}}, IsTransparencySet(TO_BRIDGES));
			}
		}

		if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation && !IsInvisibilitySet(TO_BRIDGES) && HasTunnelBridgeReservation(rampnorth)) {
			if (rti->UsesOverlay()) {
				SpriteID overlay = GetCustomRailSprite(rti, rampnorth, RTSG_OVERLAY, TCX_ON_BRIDGE);
				AddSortableSpriteToDraw(overlay + RTO_X + axis, PALETTE_CRASH, ti->x, ti->y, bridge_z, {{}, {TILE_SIZE, TILE_SIZE, 0}, {}}, IsTransparencySet(TO_BRIDGES));
			} else {
				AddSortableSpriteToDraw(axis == AXIS_X ? rti->base_sprites.single_x : rti->base_sprites.single_y, PALETTE_CRASH, ti->x, ti->y, bridge_z, {{}, {TILE_SIZE, TILE_SIZE, 0}, {}}, IsTransparencySet(TO_BRIDGES));
			}
		}

		EndSpriteCombine();

		if (HasRailCatenaryDrawn(GetRailType(rampsouth))) {
			DrawRailCatenaryOnBridge(ti);
		}
	}

	/* draw roof, the component of the bridge which is logically between the vehicle and the camera */
	if (!IsInvisibilitySet(TO_BRIDGES)) {
		if (axis == AXIS_X) {
			y += 12;
			if (psid[1].sprite & SPRITE_MASK) AddSortableSpriteToDraw(psid[1].sprite, psid[1].pal, x, y, z, {{0, 3, BRIDGE_Z_START}, {TILE_SIZE, 1, 40}, {0, -3, -BRIDGE_Z_START}}, IsTransparencySet(TO_BRIDGES));
		} else {
			x += 12;
			if (psid[1].sprite & SPRITE_MASK) AddSortableSpriteToDraw(psid[1].sprite, psid[1].pal, x, y, z, {{3, 0, BRIDGE_Z_START}, {1, TILE_SIZE, 40}, {-3, 0, -BRIDGE_Z_START}}, IsTransparencySet(TO_BRIDGES));
		}
	}

	/* Draw TramFront as SpriteCombine */
	if (transport_type == TRANSPORT_ROAD) EndSpriteCombine();

	/* Do not draw anything more if bridges are invisible */
	if (IsInvisibilitySet(TO_BRIDGES)) return;

	if (blocked_pillars.Any(pillars)) return;
	DrawBridgePillars(psid[2], ti, axis, drawfarpillar, x, y, z);
}




static void GetTileDesc_TunnelBridge(TileIndex tile, TileDesc &td)
{
	TransportType tt = GetTunnelBridgeTransportType(tile);

	if (IsTunnel(tile)) {
		td.str = (tt == TRANSPORT_RAIL) ? STR_LAI_TUNNEL_DESCRIPTION_RAILROAD : STR_LAI_TUNNEL_DESCRIPTION_ROAD;
	} else { // IsBridge(tile)
		td.str = (tt == TRANSPORT_WATER) ? STR_LAI_BRIDGE_DESCRIPTION_AQUEDUCT : GetBridgeSpec(GetBridgeType(tile))->transport_name[tt];
	}
	td.owner[0] = GetTileOwner(tile);

	Owner road_owner = INVALID_OWNER;
	Owner tram_owner = INVALID_OWNER;
	RoadType road_rt = (tt == TRANSPORT_ROAD) ? GetRoadTypeRoad(tile) : INVALID_ROADTYPE;
	RoadType tram_rt = (tt == TRANSPORT_ROAD) ? GetRoadTypeTram(tile) : INVALID_ROADTYPE;
	if (road_rt != INVALID_ROADTYPE) {
		const RoadTypeInfo *rti = GetRoadTypeInfo(road_rt);
		td.roadtype = rti->strings.name;
		td.road_speed = rti->max_speed / 2;
		road_owner = GetRoadOwner(tile, RTT_ROAD);
	}
	if (tram_rt != INVALID_ROADTYPE) {
		const RoadTypeInfo *rti = GetRoadTypeInfo(tram_rt);
		td.tramtype = rti->strings.name;
		td.tram_speed = rti->max_speed / 2;
		tram_owner = GetRoadOwner(tile, RTT_TRAM);
	}

	/* Is there a mix of owners? */
	if ((tram_owner != INVALID_OWNER && tram_owner != td.owner[0]) ||
			(road_owner != INVALID_OWNER && road_owner != td.owner[0])) {
		uint i = 1;
		if (road_owner != INVALID_OWNER) {
			td.owner_type[i] = STR_LAND_AREA_INFORMATION_ROAD_OWNER;
			td.owner[i] = road_owner;
			i++;
		}
		if (tram_owner != INVALID_OWNER) {
			td.owner_type[i] = STR_LAND_AREA_INFORMATION_TRAM_OWNER;
			td.owner[i] = tram_owner;
		}
	}

	if (tt == TRANSPORT_RAIL) {
		const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(tile));
		td.rail_speed = rti->max_speed;
		td.railtype = rti->strings.name;

		if (!IsTunnel(tile)) {
			uint16_t spd = GetBridgeSpec(GetBridgeType(tile))->speed;
			/* rail speed special-cases 0 as unlimited, hides display of limit etc. */
			if (spd == UINT16_MAX) spd = 0;
			if (td.rail_speed == 0 || spd < td.rail_speed) {
				td.rail_speed = spd;
			}
		}
	} else if (tt == TRANSPORT_ROAD && !IsTunnel(tile)) {
		uint16_t spd = GetBridgeSpec(GetBridgeType(tile))->speed;
		/* road speed special-cases 0 as unlimited, hides display of limit etc. */
		if (spd == UINT16_MAX) spd = 0;
		if (road_rt != INVALID_ROADTYPE && (td.road_speed == 0 || spd < td.road_speed)) td.road_speed = spd;
		if (tram_rt != INVALID_ROADTYPE && (td.tram_speed == 0 || spd < td.tram_speed)) td.tram_speed = spd;
	}
}


const OpenTTDTunnelServices &GetRustTunnelGroundServices() noexcept
{
	static const OpenTTDTunnelServices services{
		.snow = [](uint32_t index) noexcept { TileIndex t{index}; return OpenTTDTunnelGround{.z = RustTunnelBridgeIsBridge(Tile{t}) ? OpenTTDTileMaxZ(t) : RustTileZ(t), .snow = RustTunnelSnowOrDesert(Tile{t})}; },
		.snow_or_desert = RustTunnelSnowOrDesert,
		.set_snow_or_desert = [](uint32_t t, bool value) noexcept { SetTunnelBridgeSnowOrDesert(TileIndex{t}, value); },
	};
	return services;
}



/**
 * Helper to prepare the ground vehicle when entering a bridge. This get called
 * when entering the bridge, at the last frame of travel on the bridge head.
 * Our calling function gets called before UpdateInclination/UpdateZPosition,
 * which normally controls the Z-coordinate. However, in the wormhole of the
 * bridge the vehicle is in a strange state so UpdateInclination does not get
 * called for the wormhole of the bridge and as such the going up/down bits
 * would remain set. As such, this function clears those. In doing so, the call
 * to UpdateInclination will not update the Z-coordinate, so that has to be
 * done here as well.
 * @param gv The ground vehicle entering the bridge.
 */

/**
 * Frame when the 'enter tunnel' sound should be played. This is the second
 * frame on a tile, so the sound is played shortly after entering the tunnel
 * tile, while the vehicle is still visible.
 */

/**
 * Frame when a vehicle should be hidden in a tunnel with a certain direction.
 * This differs per direction, because of visibility / bounding box issues.
 * Note that direction, in this case, is the direction leading into the tunnel.
 * When entering a tunnel, hide the vehicle when it reaches the given frame.
 * When leaving a tunnel, show the vehicle when it is one frame further
 * to the 'outside', i.e. at (TILE_SIZE-1) - (frame) + 1
 */


static CommandCost TerraformTile_TunnelBridge(TileIndex tile, DoCommandFlags flags, int z_new, Slope tileh_new)
{
	CommandCost cost, ret;
	openttd_rust_tunnelbridge_terraform(&GetRustTunnelBridgeServices(), RustCommandCost(&cost), RustCommandCost(&ret), tile.base(), flags.base(), z_new, tileh_new);
	return cost;
}

static CommandCost CheckBuildAbove_TunnelBridge(TileIndex tile, DoCommandFlags flags, Axis axis, int height)
{
	CommandCost cost;
	openttd_rust_tunnelbridge_build_above(&GetRustTunnelBridgeServices(), RustCommandCost(&cost), tile.base(), flags.base(), axis, height);
	return cost;
}

extern const TileTypeProcs _tile_type_tunnelbridge_procs = {
	DrawTile_TunnelBridge,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	ClearTile_TunnelBridge,          // clear_tile_proc
	nullptr,                            // add_accepted_cargo_proc
	GetTileDesc_TunnelBridge,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr,                            // click_tile_proc
	nullptr,                 // Rust scheduler calls the owner directly.
	nullptr,                         // Rust dispatcher calls the owner directly.
	nullptr,                            // add_produced_cargo_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr, // Rust movement owner dispatches directly.
	TerraformTile_TunnelBridge,      // terraform_tile_proc
	CheckBuildAbove_TunnelBridge, // check_build_above_proc
};

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDTunnelBridgeServices.hpp"
bool CheckAbi_tunnelbridge_cmd()
{
	return AbiServicesComplete(GetRustTunnelBridgeServices());
}
static_assert(PR_BUILD_BRIDGE == 4 && PR_BUILD_TUNNEL == 8 && PR_CLEAR_BRIDGE == 29 && PR_CLEAR_TUNNEL == 33 && PR_BUILD_AQUEDUCT == 62 && PR_CLEAR_AQUEDUCT == 63);
static_assert(std::is_same_v<BridgeType, uint32_t>);
static_assert(MAX_BRIDGES == 13 && TUNNELBRIDGE_TRACKBIT_FACTOR == 4);
static_assert(RATING_TUNNEL_BRIDGE_UP_STEP == 50 && RATING_TUNNEL_BRIDGE_DOWN_STEP == -250 && RATING_TUNNEL_BRIDGE_MINIMUM == 0 && RATING_MAXIMUM == 1000);
