/* OpenTTD-Rust migration evidence, licensed under GPLv2.
 * Shared by the AI and GameScript, which alias their API classes to Api*.
 * Every record prints "INFRA <who> <name> <value>". */

function Record(who, name, value) {
	print("INFRA " + who + " " + name + " " + value);
}

function Num(value) {
	if (value == true) return 1;
	if (value == false) return 0;
	if (value == null) return "null";
	return value;
}

/* Tiles around both map ends and the 32-bit TileIndex conversion. */
function BoundaryTiles() {
	local size = ApiMap.GetMapSize();
	return [-1, 0, 1, ApiMap.GetMapSizeX() - 1, ApiMap.GetMapSizeX(), size - 1, size, size + 1,
		0x7FFFFFFF, 0xFFFFFFFF, 0x100000000, -0x80000000];
}

function ProbeMap(who) {
	Record(who, "map.size", ApiMap.GetMapSize() + " " + ApiMap.GetMapSizeX() + " " + ApiMap.GetMapSizeY());
	foreach (tile in BoundaryTiles()) {
		Record(who, "map.tile " + tile, Num(ApiMap.IsValidTile(tile)) + " " + ApiMap.GetTileX(tile) + " " +
			ApiMap.GetTileY(tile) + " " + ApiMap.DistanceFromEdge(tile) + " " +
			ApiMap.DistanceManhattan(tile, 0) + " " + ApiMap.DistanceMax(tile, 70) + " " +
			ApiMap.DistanceSquare(70, tile) + " " + ApiTile.GetDistanceManhattanToTile(tile, 70) + " " +
			ApiTile.GetDistanceSquareToTile(70, tile));
	}
	foreach (xy in [[0, 0], [5, 7], [-1, 0], [0, -1], [64, 0], [0, 64], [0x100000000, 3], [-5, -5]]) {
		Record(who, "map.index " + xy[0] + "," + xy[1], ApiMap.GetTileIndex(xy[0], xy[1]));
	}
}

function ProbeValues(who) {
	local slopes = "";
	local corners = "";
	for (local slope = -1; slope <= 0x101; slope++) {
		slopes += " " + Num(ApiTile.IsSteepSlope(slope)) + Num(ApiTile.IsHalftileSlope(slope)) + ":" + ApiTile.GetComplementSlope(slope);
	}
	foreach (slope in [0xFFFF, 0x10000, 0x1001F, 0x7FFFFFFF, -0x100]) {
		slopes += " " + Num(ApiTile.IsSteepSlope(slope)) + Num(ApiTile.IsHalftileSlope(slope)) + ":" + ApiTile.GetComplementSlope(slope);
	}
	Record(who, "tile.slopes", slopes);
	local tile = ApiMap.GetTileIndex(10, 10);
	foreach (corner in [-1, 0, 1, 2, 3, 4, 255, 256, 257, 0x100000002]) {
		corners += " " + ApiTile.GetCornerHeight(tile, corner);
	}
	Record(who, "tile.corners", corners);
	local costs = "";
	for (local build = -1; build <= 10; build++) costs += " " + ApiTile.GetBuildCost(build);
	Record(who, "tile.costs", costs);
	local transports = "";
	foreach (kind in [-1, 0, 1, 2, 3, 4, 256, 257]) {
		foreach (t in BoundaryTiles()) transports += Num(ApiTile.HasTransportType(t, kind));
		transports += " ";
	}
	Record(who, "tile.transport-boundary", transports);
}

/* Per-tile predicates over every map tile plus the boundary tiles. */
function TilePredicates() {
	return [
		["IsBuildable", function(t) { return ApiTile.IsBuildable(t); }],
		["IsSeaTile", function(t) { return ApiTile.IsSeaTile(t); }],
		["IsRiverTile", function(t) { return ApiTile.IsRiverTile(t); }],
		["IsWaterTile", function(t) { return ApiTile.IsWaterTile(t); }],
		["IsCoastTile", function(t) { return ApiTile.IsCoastTile(t); }],
		["IsStationTile", function(t) { return ApiTile.IsStationTile(t); }],
		["HasTreeOnTile", function(t) { return ApiTile.HasTreeOnTile(t); }],
		["IsFarmTile", function(t) { return ApiTile.IsFarmTile(t); }],
		["IsRockTile", function(t) { return ApiTile.IsRockTile(t); }],
		["IsRoughTile", function(t) { return ApiTile.IsRoughTile(t); }],
		["IsSnowTile", function(t) { return ApiTile.IsSnowTile(t); }],
		["IsDesertTile", function(t) { return ApiTile.IsDesertTile(t); }],
		["IsHouseTile", function(t) { return ApiTile.IsHouseTile(t); }],
		["GetTerrainType", function(t) { return ApiTile.GetTerrainType(t); }],
		["GetSlope", function(t) { return ApiTile.GetSlope(t); }],
		["GetMinHeight", function(t) { return ApiTile.GetMinHeight(t); }],
		["GetMaxHeight", function(t) { return ApiTile.GetMaxHeight(t); }],
		["GetCornerHeight.0", function(t) { return ApiTile.GetCornerHeight(t, 0); }],
		["GetCornerHeight.1", function(t) { return ApiTile.GetCornerHeight(t, 1); }],
		["GetCornerHeight.2", function(t) { return ApiTile.GetCornerHeight(t, 2); }],
		["GetCornerHeight.3", function(t) { return ApiTile.GetCornerHeight(t, 3); }],
		["GetOwner", function(t) { return ApiTile.GetOwner(t); }],
		["HasRail", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_RAIL); }],
		["HasRoad", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_ROAD); }],
		["HasWater", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_WATER); }],
		["HasAir", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_AIR); }],
		["GetTileX", function(t) { return ApiMap.GetTileX(t); }],
		["DistanceFromEdge", function(t) { return ApiMap.DistanceFromEdge(t); }],
		["BuildableRect", function(t) { return ApiTile.IsBuildableRectangle(t, 1 + t % 3, 1 + t % 2); }],
		["IsRailTile", function(t) { return ApiRail.IsRailTile(t); }],
		["IsLevelCrossingTile", function(t) { return ApiRail.IsLevelCrossingTile(t); }],
		["IsRailDepotTile", function(t) { return ApiRail.IsRailDepotTile(t); }],
		["IsRailStationTile", function(t) { return ApiRail.IsRailStationTile(t); }],
		["IsRailWaypointTile", function(t) { return ApiRail.IsRailWaypointTile(t); }],
		["GetRailTracks", function(t) { return ApiRail.GetRailTracks(t); }],
		["GetRailType", function(t) { return ApiRail.GetRailType(t); }],
		["GetRailDepotFrontTile", function(t) { return ApiRail.GetRailDepotFrontTile(t); }],
		["GetRailStationDirection", function(t) { return ApiRail.GetRailStationDirection(t); }],
		["GetSignalType.sw", function(t) { return ApiRail.GetSignalType(t, t + 1); }],
		["GetSignalType.ne", function(t) { return ApiRail.GetSignalType(t, t - 1); }],
		["GetSignalType.se", function(t) { return ApiRail.GetSignalType(t, t + ApiMap.GetMapSizeX()); }],
		["GetSignalType.nw", function(t) { return ApiRail.GetSignalType(t, t - ApiMap.GetMapSizeX()); }],
		["IsRoadTile", function(t) { return ApiRoad.IsRoadTile(t); }],
		["IsRoadDepotTile", function(t) { return ApiRoad.IsRoadDepotTile(t); }],
		["IsRoadStationTile", function(t) { return ApiRoad.IsRoadStationTile(t); }],
		["IsDriveThroughRoadStationTile", function(t) { return ApiRoad.IsDriveThroughRoadStationTile(t); }],
		["HasRoadType", function(t) { return Num(ApiRoad.HasRoadType(t, 0)) + 2 * Num(ApiRoad.HasRoadType(t, 1)) + 4 * Num(ApiRoad.HasRoadType(t, 70)); }],
		["AreRoadTilesConnected", function(t) {
			local sx = ApiMap.GetMapSizeX();
			return Num(ApiRoad.AreRoadTilesConnected(t, t + 1)) + 2 * Num(ApiRoad.AreRoadTilesConnected(t, t - 1)) + 4 * Num(ApiRoad.AreRoadTilesConnected(t, t + sx)) + 8 * Num(ApiRoad.AreRoadTilesConnected(t, t - sx));
		}],
		["CanBuildConnectedRoadPartsHere", function(t) {
			local sx = ApiMap.GetMapSizeX();
			return ApiRoad.CanBuildConnectedRoadPartsHere(t, t - 1, t + 1) + 3 * ApiRoad.CanBuildConnectedRoadPartsHere(t, t - sx, t + 1) + 9 * ApiRoad.CanBuildConnectedRoadPartsHere(t, t + sx, t - sx) + 27 * ApiRoad.CanBuildConnectedRoadPartsHere(t, t + 1, t + 1);
		}],
		["GetNeighbourRoadCount", function(t) { return ApiRoad.GetNeighbourRoadCount(t); }],
		["GetRoadDepotFrontTile", function(t) { return ApiRoad.GetRoadDepotFrontTile(t); }],
		["GetRoadStationFrontTile", function(t) { return ApiRoad.GetRoadStationFrontTile(t); }],
		["GetDriveThroughBackTile", function(t) { return ApiRoad.GetDriveThroughBackTile(t); }],
		["AreTilesConnected", function(t) {
			local sx = ApiMap.GetMapSizeX();
			local v = 0;
			foreach (i, pair in [[-1, 1], [1, -1], [-sx, sx], [sx, -sx], [-1, sx], [-1, -sx], [1, sx], [1, -sx], [-sx, 1], [sx, -1], [0, 1], [-1, 2]]) {
				if (ApiRail.AreTilesConnected(t + pair[0], t, t + pair[1])) v = v | (1 << i);
			}
			return v;
		}],
		["IsBridgeTile", function(t) { return ApiBridge.IsBridgeTile(t); }],
		["GetBridgeType", function(t) { return ApiBridge.GetBridgeType(t); }],
		["GetOtherBridgeEnd", function(t) { return ApiBridge.GetOtherBridgeEnd(t); }],
		["IsTunnelTile", function(t) { return ApiTunnel.IsTunnelTile(t); }],
		["GetOtherTunnelEnd", function(t) { return ApiTunnel.GetOtherTunnelEnd(t); }],
		["IsWaterDepotTile", function(t) { return ApiMarine.IsWaterDepotTile(t); }],
		["IsDockTile", function(t) { return ApiMarine.IsDockTile(t); }],
		["IsBuoyTile", function(t) { return ApiMarine.IsBuoyTile(t); }],
		["IsLockTile", function(t) { return ApiMarine.IsLockTile(t); }],
		["IsCanalTile", function(t) { return ApiMarine.IsCanalTile(t); }],
		["AreWaterTilesConnectedX", function(t) { return ApiMarine.AreWaterTilesConnected(t, t + 1); }],
		["AreWaterTilesConnectedY", function(t) { return ApiMarine.AreWaterTilesConnected(t, t + ApiMap.GetMapSizeX()); }],
		["AreWaterTilesConnectedBack", function(t) { return ApiMarine.AreWaterTilesConnected(t, t - 1); }],
	];
}

function ProbeTiles(who) {
	local predicates = TilePredicates();
	local hashes = [];
	local counts = [];
	foreach (p in predicates) { hashes.push(0); counts.push(0); }
	local tiles = BoundaryTiles();
	for (local t = 0; t < ApiMap.GetMapSize(); t++) tiles.push(t);
	foreach (t in tiles) {
		foreach (i, p in predicates) {
			local v = Num(p[1](t));
			hashes[i] = (hashes[i] * 31 + v) & 0x3FFFFFFF;
			if (v != 0) counts[i]++;
		}
	}
	foreach (i, p in predicates) Record(who, "sweep." + p[0], hashes[i] + " nonzero=" + counts[i]);
	foreach (t in [-1, 0, ApiMap.GetMapSize(), ApiMap.GetTileIndex(1, 1), ApiMap.GetTileIndex(62, 62)]) {
		local rects = "";
		foreach (wh in [[0, 1], [1, 0], [1, 1], [2, 2], [64, 1], [1, 64], [100, 100], [-1, 5], [0x7FFFFFFF, 1]]) {
			rects += Num(ApiTile.IsBuildableRectangle(t, wh[0], wh[1]));
		}
		Record(who, "tile.rect " + t, rects);
	}
}

/* One command attempt: result, last error and accumulated cost. */
function Command(who, name, attempt) {
	local costs = ApiAccounting();
	local result = Num(attempt());
	Record(who, "cmd." + name, result + " err=" + ApiError.GetLastErrorString() + " cost=" + costs.GetCosts());
}

function ProbeCommands(who, free) {
	local size = ApiMap.GetMapSize();
	local edge = ApiMap.GetTileIndex(ApiMap.GetMapSizeX() - 2, 5);
	Command(who, "raise-invalid", function() : (size) { return ApiTile.RaiseTile(size, ApiTile.SLOPE_N); });
	Command(who, "raise-void", function() { return ApiTile.RaiseTile(0, ApiTile.SLOPE_N); });
	Command(who, "raise", function() : (free) { return ApiTile.RaiseTile(free, ApiTile.SLOPE_N); });
	Command(who, "raise-bad-slope", function() : (free) { return ApiTile.RaiseTile(free, 0x1FF); });
	Command(who, "lower", function() : (free) { return ApiTile.LowerTile(free + 1, ApiTile.SLOPE_ELEVATED); });
	Command(who, "level-invalid-start", function() : (size, free) { return ApiTile.LevelTiles(size, free); });
	Command(who, "level-invalid-end", function() : (size, free) { return ApiTile.LevelTiles(free, size + 3); });
	Command(who, "level", function() : (free) { return ApiTile.LevelTiles(free, free + 2); });
	Command(who, "demolish-void", function() { return ApiTile.DemolishTile(0); });
	Command(who, "demolish", function() : (free) { return ApiTile.DemolishTile(free + 3); });
	Command(who, "tree-invalid", function() : (size) { return ApiTile.PlantTree(size); });
	Command(who, "tree", function() : (free) { return ApiTile.PlantTree(free + 4); });
	foreach (wh in [[0, 1], [1, 0], [1, 1], [20, 20], [21, 1], [1, 21], [3, 3]]) {
		local w = wh[0], h = wh[1];
		Command(who, "tree-rect " + w + "x" + h, function() : (free, w, h) { return ApiTile.PlantTreeRectangle(free + 5, w, h); });
	}
	Command(who, "tree-rect-edge", function() : (edge) { return ApiTile.PlantTreeRectangle(edge, 3, 2); });
	Command(who, "tree-rect-void", function() { return ApiTile.PlantTreeRectangle(0, 2, 2); });
}

function Free(start = null) {
	if (start == null) start = ApiMap.GetTileIndex(8, 8);
	for (local tile = start; tile < ApiMap.GetMapSize(); tile++) {
		local ok = true;
		for (local i = 0; i < 8 && ok; i++) {
			ok = ApiTile.IsBuildable(tile + i) && ApiTile.GetSlope(tile + i) == ApiTile.SLOPE_FLAT && !ApiTile.HasTreeOnTile(tile + i);
		}
		if (ok) return tile;
	}
	throw "No free tile";
}

/* Rail type availability, compatibility and the current type, around the 64-type range. */
function ProbeRailTypes(who) {
	local types = "";
	foreach (rt in [-1, 0, 1, 2, 3, 4, 63, 64, 255, 256, 0x100000001]) {
		types += " " + rt + ":" + Num(ApiRail.IsRailTypeAvailable(rt)) + ApiError.GetLastErrorString();
		foreach (track in [0, 1, 2, 3, 64, 256]) types += Num(ApiRail.TrainCanRunOnRail(rt, track)) + ":" + Num(ApiRail.TrainHasPowerOnRail(rt, track)) + ",";
		types += " speed=" + ApiRail.GetMaxSpeed(rt);
		for (local build = -1; build <= 5; build++) types += " cost=" + ApiRail.GetBuildCost(rt, build);
	}
	Record(who, "rail.types", types);
	local current = "" + ApiRail.GetCurrentRailType();
	foreach (rt in [64, -1, 3, 1, 0, 256]) {
		ApiRail.SetCurrentRailType(rt);
		current += " " + ApiRail.GetCurrentRailType();
	}
	Record(who, "rail.current", current);
}

/* Rail command preconditions and results; the rail area is a free row of eight tiles. */
function ProbeRail(who, r) {
	local sx = ApiMap.GetMapSizeX();
	local size = ApiMap.GetMapSize();
	Command(who, "rail-track-zero", function() : (r, sx) { return ApiRail.BuildRailTrack(r + 2 * sx, 0); });
	Command(who, "rail-track-two", function() : (r, sx) { return ApiRail.BuildRailTrack(r + 2 * sx, 3); });
	Command(who, "rail-track-high", function() : (r, sx) { return ApiRail.BuildRailTrack(r + 2 * sx, 64); });
	Command(who, "rail-track-invalid", function() : (size) { return ApiRail.BuildRailTrack(size, 1); });
	Command(who, "rail-track", function() : (r, sx) { return ApiRail.BuildRailTrack(r + 2 * sx, ApiRail.RAILTRACK_NE_SW); });
	Command(who, "rail-track-diag", function() : (r, sx) { return ApiRail.BuildRailTrack(r + 2 * sx + 1, ApiRail.RAILTRACK_NW_NE); });
	Command(who, "rail-drag-invalid", function() : (r, size) { return ApiRail.BuildRail(r, r + 1, size); });
	Command(who, "rail-drag-near", function() : (r) { return ApiRail.BuildRail(r, r + 2, r + 5); });
	Command(who, "rail-drag-same", function() : (r) { return ApiRail.BuildRail(r, r + 1, r + 1); });
	Command(who, "rail-drag-diagonal", function() : (r, sx) { return ApiRail.BuildRail(r, r + 1, r + 3 + 3 * sx); });
	Command(who, "rail-drag-bend", function() : (r, sx) { return ApiRail.BuildRail(r + sx, r + 1 + sx, r + 2 + 2 * sx); });
	Command(who, "rail-drag", function() : (r) { return ApiRail.BuildRail(r, r + 1, r + 6); });
	Command(who, "rail-drag-back", function() : (r) { return ApiRail.BuildRail(r + 7, r + 6, r); });
	Command(who, "rail-drag-y", function() : (r, sx) { return ApiRail.BuildRail(r + 7 - sx, r + 7, r + 7 + 3 * sx); });
	foreach (kind in [-1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 255]) {
		local k = kind;
		Command(who, "rail-signal " + k, function() : (r, k) { return ApiRail.BuildSignal(r + 2, r + 1, k); });
		Record(who, "rail-signal-type " + k, ApiRail.GetSignalType(r + 2, r + 1) + " " + ApiRail.GetSignalType(r + 2, r + 3));
	}
	Command(who, "rail-signal-far", function() : (r) { return ApiRail.BuildSignal(r + 3, r + 1, 0); });
	Command(who, "rail-signal-off", function() : (r, sx) { return ApiRail.BuildSignal(r + 3, r + 3 + sx, 0); });
	Command(who, "rail-signal-remove-none", function() : (r) { return ApiRail.RemoveSignal(r + 4, r + 3); });
	Command(who, "rail-signal-remove", function() : (r) { return ApiRail.RemoveSignal(r + 2, r + 1); });
	Command(who, "rail-signal-keep", function() : (r) { return ApiRail.BuildSignal(r + 5, r + 6, 4); });
	Command(who, "rail-depot-same", function() : (r, sx) { return ApiRail.BuildRailDepot(r + 3 * sx, r + 3 * sx); });
	Command(who, "rail-depot-skew", function() : (r, sx) { return ApiRail.BuildRailDepot(r + 3 * sx, r + 4 * sx + 1); });
	Command(who, "rail-depot", function() : (r, sx) { return ApiRail.BuildRailDepot(r + 3 * sx + 1, r + 3 * sx); });
	Command(who, "rail-depot-north", function() : (r, sx) { return ApiRail.BuildRailDepot(r + 4 * sx + 7, r + 3 * sx + 7); });
	Command(who, "rail-station-dir", function() : (r, sx) { return ApiRail.BuildRailStation(r + 5 * sx, ApiRail.RAILTRACK_NW_NE, 1, 3, ApiStation.STATION_NEW); });
	Command(who, "rail-station-zero", function() : (r, sx) { return ApiRail.BuildRailStation(r + 5 * sx, ApiRail.RAILTRACK_NE_SW, 0, 3, ApiStation.STATION_NEW); });
	Command(who, "rail-station-long", function() : (r, sx) { return ApiRail.BuildRailStation(r + 5 * sx, ApiRail.RAILTRACK_NE_SW, 1, 256, ApiStation.STATION_NEW); });
	Command(who, "rail-station-bad", function() : (r, sx) { return ApiRail.BuildRailStation(r + 5 * sx, ApiRail.RAILTRACK_NE_SW, 1, 3, 9999); });
	Command(who, "rail-station", function() : (r, sx) { return ApiRail.BuildRailStation(r + 5 * sx, ApiRail.RAILTRACK_NE_SW, 1, 3, ApiStation.STATION_NEW); });
	Command(who, "rail-station-grf", function() : (r, sx) { return ApiRail.BuildNewGRFRailStation(r + 5 * sx + 4, ApiRail.RAILTRACK_NW_SE, 2, 2, ApiStation.STATION_NEW, 0, -1, -1, 10, true); });
	Command(who, "rail-waypoint-bare", function() : (r, sx) { return ApiRail.BuildRailWaypoint(r + 6 * sx); });
	Command(who, "rail-waypoint-diag", function() : (r, sx) { return ApiRail.BuildRailWaypoint(r + 2 * sx + 1); });
	Command(who, "rail-waypoint", function() : (r) { return ApiRail.BuildRailWaypoint(r + 3); });
	Command(who, "rail-waypoint-2", function() : (r) { return ApiRail.BuildRailWaypoint(r + 6); });
	Command(who, "rail-convert-invalid", function() : (r, size) { return ApiRail.ConvertRailType(r, size, 0); });
	Command(who, "rail-convert-type", function() : (r) { return ApiRail.ConvertRailType(r, r + 6, 64); });
	Command(who, "rail-convert", function() : (r) { return ApiRail.ConvertRailType(r, r + 6, ApiRail.GetCurrentRailType()); });
	Command(who, "rail-remove-track-bare", function() : (r, sx) { return ApiRail.RemoveRailTrack(r + 6 * sx, 1); });
	Command(who, "rail-remove-track-two", function() : (r, sx) { return ApiRail.RemoveRailTrack(r + 2 * sx, 3); });
	Command(who, "rail-remove-track-missing", function() : (r, sx) { return ApiRail.RemoveRailTrack(r + 2 * sx, 2); });
	Command(who, "rail-remove-track", function() : (r, sx) { return ApiRail.RemoveRailTrack(r + 2 * sx + 1, ApiRail.RAILTRACK_NW_NE); });
	Command(who, "rail-remove-drag-far", function() : (r) { return ApiRail.RemoveRail(r + 7, r + 5, r); });
	Command(who, "rail-remove-drag", function() : (r, sx) { return ApiRail.RemoveRail(r + 7 - sx, r + 7, r + 7 + 2 * sx); });
	Command(who, "road-crossing", function() : (r, sx) { return ApiRoad.BuildRoad(r + 4 - sx, r + 4 + sx); });
	Command(who, "road-crossing-2", function() : (r, sx) { return ApiRoad.BuildRoad(r + 1 - sx, r + 1 + sx); });
	Record(who, "rail-crossing", Num(ApiRail.IsLevelCrossingTile(r + 4)) + " " + ApiRail.GetRailTracks(r + 4) + " " + Num(ApiRail.AreTilesConnected(r + 3, r + 4, r + 5)));
	Command(who, "rail-remove-crossing", function() : (r) { return ApiRail.RemoveRailTrack(r + 1, ApiRail.RAILTRACK_NE_SW); });
	Command(who, "rail-remove-rect", function() : (r, sx) { return ApiRail.RemoveRailStationTileRectangle(r + 5 * sx, r + 5 * sx + 2, false); });
	Command(who, "rail-remove-waypoint", function() : (r) { return ApiRail.RemoveRailWaypointTileRectangle(r + 6, r + 6, true); });
}

/* The first tile of a buildable 9x8 block at or after start, or start. */
function FreeBlock(start) {
	for (local tile = start; tile < ApiMap.GetMapSize(); tile++) {
		if (ApiTile.IsBuildableRectangle(tile, 9, 8)) return tile;
	}
	return start;
}

/* Road type availability and the connected-part lookup over every non-aborting slope. */
function ProbeRoadTypes(who) {
	local types = "";
	foreach (rt in [-1, 0, 1, 2, 62, 63, 64, 255, 256]) {
		types += " " + rt + ":" + Num(ApiRoad.IsRoadTypeAvailable(rt)) + ApiError.GetLastErrorString();
		local info = "" + ApiRoad.GetMaxSpeed(rt) + ":" + ApiError.GetLastErrorString();
		foreach (build in [-1, 0, 1, 2, 3, 4, 256]) {
			info += " " + build + ":" + ApiRoad.GetBuildCost(rt, build) + ":" + ApiError.GetLastErrorString();
		}
		foreach (other in [-1, 0, 1, 2, 62, 63, 64, 255, 256]) {
			info += " " + other + ":" + Num(ApiRoad.RoadVehHasPowerOnRoad(rt, other)) + ":" + ApiError.GetLastErrorString();
			info += ":" + Num(ApiRoad.RoadVehCanRunOnRoad(rt, other)) + ":" + ApiError.GetLastErrorString();
		}
		Record(who, "road.info " + rt, info);
	}
	Record(who, "road.types", types);
	local current = "" + ApiRoad.GetCurrentRoadType();
	foreach (rt in [64, -1, 1, 256, 0]) {
		ApiRoad.SetCurrentRoadType(rt);
		current += " " + ApiRoad.GetCurrentRoadType();
	}
	Record(who, "road.current", current);
	local sx = ApiMap.GetMapSizeX();
	local nw = ApiMap.GetTileIndex(0, -1), sw = ApiMap.GetTileIndex(1, 0), se = ApiMap.GetTileIndex(0, 1), ne = ApiMap.GetTileIndex(-1, 0);
	local parts = [nw, sw, se, ne];
	local slopes = [];
	for (local slope = 0; slope < 16; slope++) slopes.push(slope);
	foreach (slope in [0x17, 0x1B, 0x1D, 0x1E, 0x20, 0x100]) slopes.push(slope);
	foreach (slope in slopes) {
		local text = "";
		foreach (existing in [[], [nw], [sw], [se, nw], [ne, sw], [nw, sw], [5]]) {
			foreach (a in parts) {
				foreach (b in parts) text += ApiRoad.CanBuildConnectedRoadParts(slope, existing, a, b) + 1;
			}
			text += ",";
		}
		text += ApiRoad.CanBuildConnectedRoadParts(slope, [], 7, nw) + 1;
		Record(who, "road.parts " + slope, text);
	}
}

/* Road command preconditions and results in a second free block. */
function ProbeRoad(who, d) {
	local sx = ApiMap.GetMapSizeX();
	local size = ApiMap.GetMapSize();
	Command(who, "road-same", function() : (d) { return ApiRoad.BuildRoad(d, d); });
	Command(who, "road-invalid", function() : (d, size) { return ApiRoad.BuildRoad(d, size); });
	Command(who, "road-skew", function() : (d, sx) { return ApiRoad.BuildRoad(d, d + 2 + sx); });
	Command(who, "road", function() : (d) { return ApiRoad.BuildRoad(d, d + 7); });
	Command(who, "road-full", function() : (d, sx) { return ApiRoad.BuildRoadFull(d + 2 * sx + 6, d + 2 * sx); });
	Command(who, "road-oneway", function() : (d, sx) { return ApiRoad.BuildOneWayRoad(d + 3, d + 3 + 4 * sx); });
	Command(who, "road-oneway-full", function() : (d, sx) { return ApiRoad.BuildOneWayRoadFull(d + 5 + 4 * sx, d + 5); });
	Command(who, "road-depot-same", function() : (d) { return ApiRoad.BuildRoadDepot(d + 1 + 3 * ApiMap.GetMapSizeX(), d + 1 + 3 * ApiMap.GetMapSizeX()); });
	Command(who, "road-depot-skew", function() : (d, sx) { return ApiRoad.BuildRoadDepot(d + 1 + sx, d + 2); });
	Command(who, "road-depot", function() : (d, sx) { return ApiRoad.BuildRoadDepot(d + 1 + sx, d + 1); });
	Command(who, "road-depot-west", function() : (d, sx) { return ApiRoad.BuildRoadDepot(d + 7 + 2 * sx, d + 6 + 2 * sx); });
	Command(who, "road-stop-type", function() : (d, sx) { return ApiRoad.BuildRoadStation(d + 4 + sx, d + 4, 2, ApiStation.STATION_NEW); });
	Command(who, "road-stop-station", function() : (d, sx) { return ApiRoad.BuildRoadStation(d + 4 + sx, d + 4, 0, 9999); });
	Command(who, "road-stop-skew", function() : (d, sx) { return ApiRoad.BuildRoadStation(d + 4 + sx, d + 5, 0, ApiStation.STATION_NEW); });
	Command(who, "road-stop", function() : (d, sx) { return ApiRoad.BuildRoadStation(d + 4 + sx, d + 4, 0, ApiStation.STATION_NEW); });
	Command(who, "road-stop-truck", function() : (d, sx) { return ApiRoad.BuildRoadStation(d + 6 - sx, d + 6, 1, ApiStation.STATION_JOIN_ADJACENT); });
	Command(who, "road-drive-through", function() : (d, sx) { return ApiRoad.BuildDriveThroughRoadStation(d + 2, d + 3, 0, ApiStation.STATION_NEW); });
	Command(who, "road-drive-through-y", function() : (d, sx) { return ApiRoad.BuildDriveThroughRoadStation(d + 3 + 2 * sx, d + 3 + sx, 1, ApiStation.STATION_NEW); });
	Record(who, "road-front", ApiRoad.GetRoadStationFrontTile(d + 4 + sx) + " " + ApiRoad.GetRoadStationFrontTile(d + 2) + " " + ApiRoad.GetDriveThroughBackTile(d + 2) + " " + ApiRoad.GetRoadDepotFrontTile(d + 1 + sx));
	Command(who, "road-convert-invalid", function() : (d, size) { return ApiRoad.ConvertRoadType(d, size, 0); });
	Command(who, "road-convert-type", function() : (d) { return ApiRoad.ConvertRoadType(d, d + 7, 64); });
	Command(who, "road-convert", function() : (d) { return ApiRoad.ConvertRoadType(d, d + 7, 0); });
	Command(who, "road-remove-same", function() : (d) { return ApiRoad.RemoveRoad(d, d); });
	Command(who, "road-remove", function() : (d) { return ApiRoad.RemoveRoad(d + 7, d + 6); });
	Command(who, "road-remove-full", function() : (d, sx) { return ApiRoad.RemoveRoadFull(d + 2 * sx, d + 2 * sx + 2); });
	Command(who, "road-remove-depot-bare", function() : (d, sx) { return ApiRoad.RemoveRoadDepot(d + 5 * sx); });
	Command(who, "road-remove-depot-road", function() : (d) { return ApiRoad.RemoveRoadDepot(d + 1); });
	Command(who, "road-remove-depot", function() : (d, sx) { return ApiRoad.RemoveRoadDepot(d + 7 + 2 * sx); });
	Command(who, "road-remove-station-bare", function() : (d, sx) { return ApiRoad.RemoveRoadStation(d + 5 * sx); });
	Command(who, "road-remove-station-rail", function() : (d, sx) { return ApiRoad.RemoveRoadStation(d + 2 * sx + 1); });
	Command(who, "road-remove-station", function() : (d, sx) { return ApiRoad.RemoveRoadStation(d + 6 - sx); });
}

/* The items of a list in its iteration order, and its count. */
function ListText(list) {
	local text = list.Count() + ":";
	foreach (item, value in list) text += " " + item;
	return text;
}

/* Bridge specifications, prices, lengths and the bridge, type and tile lists. */
function ProbeLists(who) {
	local text = "";
	foreach (bt in [-1, 0, 1, 2, 5, 9, 12, 13, 14, 255, 256, 0x100000001]) {
		text += " " + bt + ":" + Num(ApiBridge.IsValidBridge(bt)) + "," + ApiBridge.GetMaxSpeed(bt) + "," + ApiBridge.GetMinLength(bt) + "," + ApiBridge.GetMaxLength(bt);
		/* CalcBridgeLenCostFactor loops once per tile of length, so lengths stay small. */
		foreach (length in [-1, 0, 1, 2, 3, 5, 100, 1000]) text += "," + ApiBridge.GetPrice(bt, length);
	}
	Record(who, "bridge.specs", text);
	text = "";
	foreach (bt in [-1, 0, 1, 2, 3, 4, 5, 255, 256, 0x100000001]) text += " " + bt + ":" + ApiMarine.GetBuildCost(bt);
	Record(who, "marine.costs", text);
	Record(who, "list.bridge", ListText(ApiBridgeList()));
	foreach (length in [-1, 0, 1, 2, 3, 4, 5, 10, 100, 0x100000000]) Record(who, "list.bridge-length " + length, ListText(ApiBridgeList_Length(length)));
	Record(who, "list.railtype", ListText(ApiRailTypeList()) + " " + ApiError.GetLastErrorString());
	foreach (rtts in [0, 1, 2, 3, 8, -1]) Record(who, "list.roadtype " + rtts, ListText(ApiRoadTypeList(rtts)) + " " + ApiError.GetLastErrorString());
	local sx = ApiMap.GetMapSizeX();
	local size = ApiMap.GetMapSize();
	local cases = [
		["rect", 1 + sx, 4 + 3 * sx], ["rect-swapped", 4 + 3 * sx, 1 + sx], ["rect-anti", 4 + sx, 1 + 3 * sx],
		["rect-one", 5 + 5 * sx, 5 + 5 * sx], ["rect-void", 0, 3 + 3 * sx], ["rect-invalid", 1 + sx, size],
		["rect-edge", size - sx - 4, size - 2 * sx - 2],
	];
	foreach (c in cases) {
		local list = ApiTileList();
		list.AddRectangle(c[1], c[2]);
		local added = ListText(list);
		list.RemoveRectangle(c[1] + 1, c[2]);
		list.RemoveTile(c[1]);
		list.AddTile(c[2] + sx);
		list.AddTile(size);
		list.AddTile(0);
		list.RemoveTile(-1);
		Record(who, "list.tile " + c[0], added + " | " + ListText(list));
	}
}

/* Bridge, tunnel and marine command preconditions and results in a free block. */
function ProbeWater(who, b) {
	local sx = ApiMap.GetMapSizeX();
	local size = ApiMap.GetMapSize();
	Command(who, "bridge-same", function() : (b) { return ApiBridge.BuildBridge(1, 0, b, b); });
	Command(who, "bridge-invalid", function() : (b, size) { return ApiBridge.BuildBridge(1, 0, b, size); });
	Command(who, "bridge-skew", function() : (b, sx) { return ApiBridge.BuildBridge(1, 0, b, b + 3 + 3 * sx); });
	Command(who, "bridge-vehicle", function() : (b, sx) { return ApiBridge.BuildBridge(3, 0, b + sx, b + 5 * sx); });
	Command(who, "bridge-type", function() : (b, sx) { return ApiBridge.BuildBridge(1, 13, b + 1, b + 1 + 5 * sx); });
	Command(who, "bridge-road", function() : (b, sx) { return ApiBridge.BuildBridge(1, 0, b + 1, b + 1 + 5 * sx); });
	Command(who, "bridge-rail", function() : (b, sx) { return ApiBridge.BuildBridge(0, 0, b + 3 + 5 * sx, b + 3); });
	Command(who, "bridge-water", function() : (b, sx) { return ApiBridge.BuildBridge(2, 0, b + 5, b + 5 + 5 * sx); });
	Record(who, "bridge-ends", ApiBridge.GetOtherBridgeEnd(b + 1) + " " + ApiBridge.GetOtherBridgeEnd(b + 3) + " " + ApiBridge.GetBridgeType(b + 1));
	Command(who, "bridge-remove-bare", function() : (b, sx) { return ApiBridge.RemoveBridge(b + 7 + 7 * sx); });
	Command(who, "bridge-remove", function() : (b) { return ApiBridge.RemoveBridge(b + 3); });
	local slope = -1;
	for (local t = 0; t < size && slope == -1; t++) {
		if (ApiTile.IsBuildable(t) && ApiTunnel.GetOtherTunnelEnd(t) != ApiMap.TILE_INVALID) slope = t;
	}
	Record(who, "tunnel-start", slope + " " + ApiTunnel.GetOtherTunnelEnd(slope));
	Command(who, "tunnel-invalid", function() : (size) { return ApiTunnel.BuildTunnel(1, size); });
	Command(who, "tunnel-water", function() : (slope) { return ApiTunnel.BuildTunnel(2, slope); });
	Command(who, "tunnel-flat", function() : (b, sx) { return ApiTunnel.BuildTunnel(1, b + 6 * sx); });
	Command(who, "tunnel-rail", function() : (slope) { return ApiTunnel.BuildTunnel(0, slope); });
	Command(who, "tunnel-road", function() : (slope) { return ApiTunnel.BuildTunnel(1, slope); });
	Record(who, "tunnel-ends", Num(ApiTunnel.IsTunnelTile(slope)) + " " + ApiTunnel.GetOtherTunnelEnd(slope));
	Command(who, "tunnel-remove-bare", function() : (b, sx) { return ApiTunnel.RemoveTunnel(b + 7 + 7 * sx); });
	Command(who, "tunnel-remove", function() : (slope) { return ApiTunnel.RemoveTunnel(slope); });
	Command(who, "canal", function() : (b, sx) { return ApiMarine.BuildCanal(b + 6 + 6 * sx); });
	Command(who, "canal-2", function() : (b, sx) { return ApiMarine.BuildCanal(b + 7 + 6 * sx); });
	Command(who, "canal-3", function() : (b, sx) { return ApiMarine.BuildCanal(b + 6 + 7 * sx); });
	Command(who, "canal-invalid", function() : (size) { return ApiMarine.BuildCanal(size); });
	Command(who, "depot-same", function() : (b, sx) { return ApiMarine.BuildWaterDepot(b + 6 + 6 * sx, b + 6 + 6 * sx); });
	Command(who, "depot-diagonal", function() : (b, sx) { return ApiMarine.BuildWaterDepot(b + 6 + 6 * sx, b + 7 + 7 * sx); });
	Command(who, "depot-invalid", function() : (b, sx, size) { return ApiMarine.BuildWaterDepot(b + 6 + 6 * sx, size); });
	Command(who, "depot", function() : (b, sx) { return ApiMarine.BuildWaterDepot(b + 6 + 6 * sx, b + 7 + 6 * sx); });
	Command(who, "buoy", function() : (b, sx) { return ApiMarine.BuildBuoy(b + 6 + 7 * sx); });
	Command(who, "lock", function() : (slope) { return ApiMarine.BuildLock(slope); });
	Command(who, "dock", function() : (slope) { return ApiMarine.BuildDock(slope, ApiStation.STATION_NEW); });
	Record(who, "water-connected", Num(ApiMarine.AreWaterTilesConnected(b + 6 + 6 * sx, b + 7 + 6 * sx)) + Num(ApiMarine.AreWaterTilesConnected(b + 6 + 6 * sx, b + 6 + 7 * sx)) + Num(ApiMarine.AreWaterTilesConnected(b + 6 + 6 * sx, b + 8 + 6 * sx)));
	Command(who, "depot-remove-bare", function() : (b) { return ApiMarine.RemoveWaterDepot(b); });
	Command(who, "buoy-remove", function() : (b, sx) { return ApiMarine.RemoveBuoy(b + 6 + 7 * sx); });
	Command(who, "canal-remove", function() : (b, sx) { return ApiMarine.RemoveCanal(b + 7 + 6 * sx); });
}

function ProbeAll(who, free) {
	Record(who, "free", free);
	ProbeMap(who);
	ProbeValues(who);
	ProbeTiles(who);
	local rail = FreeBlock(ApiMap.GetTileIndex(2, 2));
	Record(who, "rail-area", rail);
	local road = FreeBlock(rail + 10);
	Record(who, "road-area", road);
	local water = FreeBlock(road + 10);
	Record(who, "water-area", water);
	ProbeRailTypes(who);
	ProbeRoadTypes(who);
	ProbeLists(who);
	local mode = ApiTestMode();
	ProbeCommands(who + ".test", free);
	ProbeRail(who + ".test", rail);
	ProbeRoad(who + ".test", road);
	ProbeWater(who + ".test", water);
	mode = null;
	ProbeCommands(who + ".exec", free);
	Command(who + ".exec", "rail-level", function() : (rail) { return ApiTile.LevelTiles(rail, rail + 8 + 7 * ApiMap.GetMapSizeX()); });
	ProbeRail(who + ".exec", rail);
	Command(who + ".exec", "road-level", function() : (road) { return ApiTile.LevelTiles(road, road + 8 + 7 * ApiMap.GetMapSizeX()); });
	ProbeRoad(who + ".exec", road);
	Command(who + ".exec", "water-level", function() : (water) { return ApiTile.LevelTiles(water, water + 8 + 7 * ApiMap.GetMapSizeX()); });
	ProbeWater(who + ".exec", water);
	ProbeLists(who + ".after");
	ProbeTiles(who + ".after");
	Record(who, "done", ApiController.GetTick());
}
