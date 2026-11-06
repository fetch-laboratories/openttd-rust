/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationInfra extends AIController {
	function Start() {
		local start = AIMap.GetTileIndex(0, -1);
		local end = AIMap.GetTileIndex(0, 1);
		/* These preconditions reject before the slope helper is reached. */
		if (AIRoad.CanBuildConnectedRoadParts(AITile.SLOPE_STEEP, [], start, start) != -1) throw "equal parts";
		if (AIRoad.CanBuildConnectedRoadParts(AITile.SLOPE_STEEP, [5], start, end) != -1) throw "existing orientation";
		if (AIRoad.CanBuildConnectedRoadParts(AITile.SLOPE_STEEP, [], 5, end) != -1) throw "start orientation";
		if (AIRoad.CanBuildConnectedRoadParts(AITile.SLOPE_STEEP, [], start, 5) != -1) throw "end orientation";
		print("INFRA road-slope fatal");
		AIRoad.CanBuildConnectedRoadParts(AITile.SLOPE_STEEP, [], start, end);
		print("INFRA road-slope returned");
		while (true) this.Sleep(100);
	}
}
