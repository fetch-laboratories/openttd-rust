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
		["GetCornerHeight", function(t) { return ApiTile.GetCornerHeight(t, t % 4) * 7 + ApiTile.GetCornerHeight(t, 3 - t % 4); }],
		["GetOwner", function(t) { return ApiTile.GetOwner(t); }],
		["HasRail", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_RAIL); }],
		["HasRoad", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_ROAD); }],
		["HasWater", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_WATER); }],
		["HasAir", function(t) { return ApiTile.HasTransportType(t, ApiTile.TRANSPORT_AIR); }],
		["GetTileX", function(t) { return ApiMap.GetTileX(t); }],
		["DistanceFromEdge", function(t) { return ApiMap.DistanceFromEdge(t); }],
		["BuildableRect", function(t) { return ApiTile.IsBuildableRectangle(t, 1 + t % 3, 1 + t % 2); }],
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
			if (v != 0 && v != -1) counts[i]++;
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
	Command(who, "raise-invalid", function() { return ApiTile.RaiseTile(size, ApiTile.SLOPE_N); });
	Command(who, "raise-void", function() { return ApiTile.RaiseTile(0, ApiTile.SLOPE_N); });
	Command(who, "raise", function() { return ApiTile.RaiseTile(free, ApiTile.SLOPE_N); });
	Command(who, "raise-bad-slope", function() { return ApiTile.RaiseTile(free, 0x1FF); });
	Command(who, "lower", function() { return ApiTile.LowerTile(free + 1, ApiTile.SLOPE_ELEVATED); });
	Command(who, "level-invalid-start", function() { return ApiTile.LevelTiles(size, free); });
	Command(who, "level-invalid-end", function() { return ApiTile.LevelTiles(free, size + 3); });
	Command(who, "level", function() { return ApiTile.LevelTiles(free, free + 2); });
	Command(who, "demolish-void", function() { return ApiTile.DemolishTile(0); });
	Command(who, "demolish", function() { return ApiTile.DemolishTile(free + 3); });
	Command(who, "tree-invalid", function() { return ApiTile.PlantTree(size); });
	Command(who, "tree", function() { return ApiTile.PlantTree(free + 4); });
	foreach (wh in [[0, 1], [1, 0], [1, 1], [20, 20], [21, 1], [1, 21], [3, 3]]) {
		local w = wh[0], h = wh[1];
		Command(who, "tree-rect " + w + "x" + h, function() { return ApiTile.PlantTreeRectangle(free + 5, w, h); });
	}
	Command(who, "tree-rect-edge", function() { return ApiTile.PlantTreeRectangle(edge, 3, 2); });
	Command(who, "tree-rect-void", function() { return ApiTile.PlantTreeRectangle(0, 2, 2); });
}

function Free() {
	for (local tile = ApiMap.GetTileIndex(8, 8); tile < ApiMap.GetMapSize(); tile++) {
		local ok = true;
		for (local i = 0; i < 8 && ok; i++) {
			ok = ApiTile.IsBuildable(tile + i) && ApiTile.GetSlope(tile + i) == ApiTile.SLOPE_FLAT && !ApiTile.HasTreeOnTile(tile + i);
		}
		if (ok) return tile;
	}
	throw "No free tile";
}

function ProbeAll(who, free) {
	Record(who, "free", free);
	ProbeMap(who);
	ProbeValues(who);
	ProbeTiles(who);
	local mode = ApiTestMode();
	ProbeCommands(who + ".test", free);
	mode = null;
	ProbeCommands(who + ".exec", free);
	ProbeTiles(who + ".after");
	Record(who, "done", ApiController.GetTick());
}
