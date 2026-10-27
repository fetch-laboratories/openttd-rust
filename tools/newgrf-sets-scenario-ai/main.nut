/* OpenTTD-Rust migration evidence, licensed under GPLv2.
 * Runs identically in both builds; its commands depend only on the map and seed. */
class MigrationNewGRFSets extends AIController {
	function Check(label, ok) {
		print("NEWGRF-SETS " + label + " " + ok);
		if (!ok) throw label + ": " + AIError.GetLastErrorString();
	}
	function Engine(type, cargo, wagon, rail) {
		foreach (id, unused in AIEngineList(type)) {
			if (!AIEngine.IsBuildable(id)) continue;
			if (type == AIVehicle.VT_ROAD) {
				if (AIEngine.HasPowerOnRoad(id, AIRoad.ROADTYPE_ROAD) && AIEngine.CanRefitCargo(id, cargo)) return id;
			} else if (AIEngine.IsWagon(id) == wagon && AIEngine.CanRunOnRail(id, rail)
					&& (wagon ? AIEngine.CanRefitCargo(id, cargo) : AIEngine.HasPowerOnRail(id, rail))) {
				return id;
			}
		}
		return -1;
	}
	function Vehicle(depot, engine, cargo) {
		local vehicle = AIVehicle.BuildVehicle(depot, engine);
		this.Check("vehicle-" + engine, AIVehicle.IsValidVehicle(vehicle));
		if (cargo != null && AIEngine.GetCargoType(engine) != cargo) this.Check("refit-" + engine, AIVehicle.RefitVehicle(vehicle, cargo));
		return vehicle;
	}
	function Run(vehicle, a, b) {
		this.Check("orders", AIOrder.AppendOrder(vehicle, a, AIOrder.OF_NONE) && AIOrder.AppendOrder(vehicle, b, AIOrder.OF_NONE));
		this.Check("start", AIVehicle.StartStopVehicle(vehicle));
	}
	/* Depot, a three-tile station, five plain tracks and a second station along X. */
	function Rail(center, cargo) {
		local rail = AIRailTypeList().Begin();
		AIRail.SetCurrentRailType(rail);
		local cx = AIMap.GetTileX(center), cy = AIMap.GetTileY(center);
		for (local r = 2; r < 20; r++) for (local y = cy - r; y <= cy + r; y++) for (local x = cx - r; x <= cx + r; x += (y == cy - r || y == cy + r) ? 1 : 2 * r) {
			if (x < 1 || y < 1 || x + 12 >= AIMap.GetMapSizeX() || y + 1 >= AIMap.GetMapSizeY()) continue;
			local start = AIMap.GetTileIndex(x, y);
			if (!AITile.IsBuildableRectangle(start, 12, 1)) continue;
			local flat = true;
			for (local i = 0; i < 12; i++) flat = flat && AITile.GetSlope(start + i) == AITile.SLOPE_FLAT && AITile.GetMinHeight(start + i) == AITile.GetMinHeight(start);
			if (!flat) continue;
			local t = function(i) : (start) { return start + i; };
			this.Check("rail-depot", AIRail.BuildRailDepot(t(0), t(1)));
			this.Check("rail-station-a", AIRail.BuildRailStation(t(1), AIRail.RAILTRACK_NE_SW, 1, 3, AIStation.STATION_NEW));
			this.Check("rail-track", AIRail.BuildRail(t(3), t(4), t(8)));
			this.Check("rail-station-b", AIRail.BuildRailStation(t(9), AIRail.RAILTRACK_NE_SW, 1, 3, AIStation.STATION_NEW));
			local engine = this.Engine(AIVehicle.VT_RAIL, cargo, false, rail), wagon = this.Engine(AIVehicle.VT_RAIL, cargo, true, rail);
			this.Check("rail-engines", engine != -1 && wagon != -1);
			local train = this.Vehicle(t(0), engine, null);
			for (local i = 0; i < 2; i++) this.Check("wagon", AIVehicle.MoveWagon(this.Vehicle(t(0), wagon, cargo), 0, train, 0));
			this.Run(train, t(1), t(9));
			return;
		}
		this.Check("rail-site", false);
	}
	/* Two drive-through stops on town roads and a depot beside the road. */
	function Road(center, cargo) {
		AIRoad.SetCurrentRoadType(AIRoad.ROADTYPE_ROAD);
		local stops = [], depot = -1;
		local tiles = AITileList();
		local cx = AIMap.GetTileX(center), cy = AIMap.GetTileY(center);
		tiles.AddRectangle(AIMap.GetTileIndex(max(1, cx - 12), max(1, cy - 12)), AIMap.GetTileIndex(min(AIMap.GetMapSizeX() - 2, cx + 12), min(AIMap.GetMapSizeY() - 2, cy + 12)));
		tiles.Valuate(AIMap.DistanceManhattan, center);
		tiles.Sort(AIList.SORT_BY_VALUE, true);
		foreach (tile, unused in tiles) {
			if (stops.len() == 2 && depot != -1) break;
			foreach (step in [1, -1, AIMap.GetMapSizeX(), -AIMap.GetMapSizeX()]) {
				if (stops.len() < 2 && AIRoad.IsRoadTile(tile) && (stops.len() == 0 || AIMap.DistanceManhattan(stops[0], tile) >= 5)
						&& AIRoad.BuildDriveThroughRoadStation(tile, tile + step, AIRoad.ROADVEHTYPE_BUS, AIStation.STATION_NEW)) stops.append(tile);
				if (depot == -1 && AIRoad.IsRoadTile(tile + step) && AITile.IsBuildable(tile)
						&& AIRoad.BuildRoadDepot(tile, tile + step) && AIRoad.BuildRoad(tile, tile + step)) depot = tile;
			}
		}
		this.Check("road-route " + stops.len() + " " + depot, stops.len() == 2 && depot != -1);
		local engine = this.Engine(AIVehicle.VT_ROAD, cargo, false, null);
		this.Check("bus-engine", engine != -1);
		for (local i = 0; i < 2; i++) this.Run(this.Vehicle(depot, engine, cargo), stops[i], stops[1 - i]);
	}
	function Start() {
		AIController.SetCommandDelay(1);
		this.Check("loan", AICompany.SetLoanAmount(AICompany.GetMaxLoanAmount()));
		local cargo = AICargoList();
		cargo.Valuate(AICargo.GetTownEffect);
		cargo.KeepValue(AICargo.TE_PASSENGERS);
		local towns = AITownList();
		towns.Valuate(AITown.GetPopulation);
		towns.Sort(AIList.SORT_BY_VALUE, false);
		local center = AITown.GetLocation(towns.Begin());
		this.Rail(center, cargo.Begin());
		this.Road(center, cargo.Begin());
		print("NEWGRF-SETS ready");
		while (true) this.Sleep(1000);
	}
}
