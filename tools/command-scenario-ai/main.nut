/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
require("parameters.nut");
class MigrationCommands extends AIController {
	state = 1;
	function Draw(n) {
		// Park-Miller stays independent of the game's shared Random stream.
		this.state = (this.state * 48271) % 2147483647;
		return this.state % n;
	}
	function Arguments(values) {
		local text = "";
		foreach (value in values) text += (text == "" ? "" : ",") + value;
		return text;
	}
	function Setup(ok) {
		print("COMMAND-SETUP " + ok);
		if (!ok) throw AIError.GetLastErrorString();
	}
	function Start() {
		AIController.SetCommandDelay(1);
		AIRoad.SetCurrentRoadType(AIRoad.ROADTYPE_ROAD);
		local railtype = AIRailTypeList().Begin();
		AIRail.SetCurrentRailType(railtype);
		local width = AIMap.GetMapSizeX(), size = AIMap.GetMapSize();
		local base = -1;
		for (local tile = width + 1; tile < size - 4 * width; tile++) {
			if (tile % width > width - 6) continue;
			local clear = true;
			for (local y = 0; y < 4; y++) for (local x = 0; x < 4; x++) {
				local t = tile + y * width + x;
				if (!AITile.IsBuildable(t) || AITile.GetSlope(t) != AITile.SLOPE_FLAT || AITile.GetMinHeight(t) != AITile.GetMinHeight(tile)) clear = false;
			}
			if (clear) { base = tile; break; }
		}
		if (base == -1) throw "No flat bootstrap patch";
		local depot = base, stop = base + width, rail = base + 2 * width, earth = base + 3 * width;
		this.Setup(AIRoad.BuildRoadDepot(depot, depot + 1));
		local engine = -1;
		foreach (id, unused in AIEngineList(AIVehicle.VT_ROAD)) {
			if (AIEngine.IsBuildable(id) && AIEngine.GetCargoType(id) == 0) { engine = id; break; }
		}
		if (engine == -1) throw "No passenger road engine";
		local bus = AIVehicle.BuildVehicle(depot, engine);
		this.Setup(AIVehicle.IsValidVehicle(bus));
		this.Setup(AIOrder.AppendOrder(bus, depot, AIOrder.OF_NONE));
		this.Setup(AIOrder.AppendOrder(bus, depot, AIOrder.OF_SERVICE_IF_NEEDED));
		local vehicles = [bus], anchors = [[base, rail, stop, earth], [depot], [0, width - 1, size - width, size - 1]];
		foreach (id, unused in AIVehicleList()) { vehicles.append(id); anchors[1].append(AIVehicle.GetLocation(id)); }
		for (local tile = 0; tile < size; tile++) {
			if (AIRail.IsRailTile(tile) || AIRoad.IsRoadTile(tile) || AITile.IsStationTile(tile)) anchors[0].append(tile);
		}
		print("COMMAND-BEGIN " + COMMAND_SEED + " " + COMMAND_COUNT + " " + bus + " " + base);
		this.state = COMMAND_SEED;
		for (local index = 0; index < COMMAND_COUNT; index++) {
			local family = index % 6, round = index / 6;
			local op = this.Draw(6), context = this.Draw(3);
			local tile = anchors[context][this.Draw(anchors[context].len())];
			local dx = this.Draw(5) - 2, dy = this.Draw(5) - 2;
			local x = tile % width + dx, y = tile / width + dy;
			// Keep out-of-map coordinates as an invalid tile, never wrap a row.
			tile = x < 0 || y < 0 || x >= width || y >= AIMap.GetMapSizeY() ? size : AIMap.GetTileIndex(x, y);
			local front = tile + (this.Draw(2) == 0 ? 1 : width);
			local vehicle = vehicles[this.Draw(vehicles.len())], a = this.Draw(5), b = this.Draw(5);
			local name, args, value, ok;
			if (family == 0) {
				local track = 1 << this.Draw(6);
				if (round < 2) { op = 0; tile = rail; track = AIRail.RAILTRACK_NE_SW; }
				name = ["rail.build", "rail.remove", "rail.convert", "rail.depot", "rail.signal", "rail.remove-signal"][op];
				switch (op) {
					case 0: args = this.Arguments([tile, track]); value = AIRail.BuildRailTrack(tile, track); break;
					case 1: args = this.Arguments([tile, track]); value = AIRail.RemoveRailTrack(tile, track); break;
					case 2: args = this.Arguments([tile, tile, railtype]); value = AIRail.ConvertRailType(tile, tile, railtype); break;
					case 3: args = this.Arguments([tile, front]); value = AIRail.BuildRailDepot(tile, front); break;
					case 4: args = this.Arguments([tile, front, AIRail.SIGNALTYPE_PBS]); value = AIRail.BuildSignal(tile, front, AIRail.SIGNALTYPE_PBS); break;
					case 5: args = this.Arguments([tile, front]); value = AIRail.RemoveSignal(tile, front); break;
				}
			} else if (family == 1) {
				if (round < 2) { op = 0; tile = base + 1; front = base + 2; }
				name = ["road.build", "road.remove", "road.convert", "road.depot", "road.oneway", "road.remove-depot"][op];
				switch (op) {
					case 0: args = this.Arguments([tile, front]); value = AIRoad.BuildRoad(tile, front); break;
					case 1: args = this.Arguments([tile, front]); value = AIRoad.RemoveRoad(tile, front); break;
					case 2: args = this.Arguments([tile, tile, AIRoad.ROADTYPE_ROAD]); value = AIRoad.ConvertRoadType(tile, tile, AIRoad.ROADTYPE_ROAD); break;
					case 3: args = this.Arguments([tile, front]); value = AIRoad.BuildRoadDepot(tile, front); break;
					case 4: args = this.Arguments([tile, front]); value = AIRoad.BuildOneWayRoad(tile, front); break;
					case 5: args = this.Arguments([tile]); value = AIRoad.RemoveRoadDepot(tile); break;
				}
			} else if (family == 2) {
				if (round < 2) { op = 0; tile = stop; front = stop + 1; }
				name = ["station.bus", "station.truck", "station.drive-through", "station.remove-road", "station.rail", "station.remove-rail"][op];
				switch (op) {
					case 0: args = this.Arguments([tile, front, AIRoad.ROADVEHTYPE_BUS, AIStation.STATION_NEW]); value = AIRoad.BuildRoadStation(tile, front, AIRoad.ROADVEHTYPE_BUS, AIStation.STATION_NEW); break;
					case 1: args = this.Arguments([tile, front, AIRoad.ROADVEHTYPE_TRUCK, AIStation.STATION_NEW]); value = AIRoad.BuildRoadStation(tile, front, AIRoad.ROADVEHTYPE_TRUCK, AIStation.STATION_NEW); break;
					case 2: args = this.Arguments([tile, front, AIRoad.ROADVEHTYPE_BUS, AIStation.STATION_NEW]); value = AIRoad.BuildDriveThroughRoadStation(tile, front, AIRoad.ROADVEHTYPE_BUS, AIStation.STATION_NEW); break;
					case 3: args = this.Arguments([tile]); value = AIRoad.RemoveRoadStation(tile); break;
					case 4: args = this.Arguments([tile, AIRail.RAILTRACK_NE_SW, a + 1, b + 1, AIStation.STATION_NEW]); value = AIRail.BuildRailStation(tile, AIRail.RAILTRACK_NE_SW, a + 1, b + 1, AIStation.STATION_NEW); break;
					case 5: args = this.Arguments([tile, front, a % 2 == 0]); value = AIRail.RemoveRailStationTileRectangle(tile, front, a % 2 == 0); break;
				}
			} else if (family == 3) {
				if (round < 4) { op = round; vehicle = bus; a = round == 3 ? 255 : 0; b = 1; }
				local flags = round == 2 ? AIOrder.OF_NONE : AIOrder.OF_SERVICE_IF_NEEDED;
				name = ["order.move", "order.skip", "order.flags", "order.remove", "order.append", "order.conditional"][op];
				switch (op) {
					case 0: args = this.Arguments([vehicle, a, b]); value = AIOrder.MoveOrder(vehicle, a, b); break;
					case 1: args = this.Arguments([vehicle, a]); value = AIOrder.SkipToOrder(vehicle, a); break;
					case 2: args = this.Arguments([vehicle, a, flags]); value = AIOrder.SetOrderFlags(vehicle, a, flags); break;
					case 3: args = this.Arguments([vehicle, a]); value = AIOrder.RemoveOrder(vehicle, a); break;
					case 4: args = this.Arguments([vehicle, tile, AIOrder.OF_NONE]); value = AIOrder.AppendOrder(vehicle, tile, AIOrder.OF_NONE); break;
					case 5: args = this.Arguments([vehicle, a]); value = AIOrder.AppendConditionalOrder(vehicle, a); break;
				}
			} else if (family == 4) {
				if (round < 2) { op = round; vehicle = round == 0 ? bus : AIVehicle.VEHICLE_INVALID; }
				name = ["vehicle.start-stop", "vehicle.reverse", "vehicle.depot", "vehicle.refit", "vehicle.build", "vehicle.clone"][op];
				switch (op) {
					case 0: args = this.Arguments([vehicle]); value = AIVehicle.StartStopVehicle(vehicle); break;
					case 1: args = this.Arguments([vehicle]); value = AIVehicle.ReverseVehicle(vehicle); break;
					case 2: args = this.Arguments([vehicle]); value = AIVehicle.SendVehicleToDepot(vehicle); break;
					case 3: args = this.Arguments([vehicle, a]); value = AIVehicle.RefitVehicle(vehicle, a); break;
					case 4: args = this.Arguments([tile, engine]); value = AIVehicle.BuildVehicle(tile, engine); break;
					case 5: args = this.Arguments([tile, vehicle, false]); value = AIVehicle.CloneVehicle(tile, vehicle, false); break;
				}
			} else {
				if (round < 2) { op = round; tile = round == 0 ? earth : 0; }
				local slope = 1 << (a % 4);
				name = ["terraform.raise", "terraform.lower", "terraform.level", "terraform.clear", "terraform.raise", "terraform.lower"][op];
				switch (op) {
					case 0: case 4: args = this.Arguments([tile, slope]); value = AITile.RaiseTile(tile, slope); break;
					case 1: case 5: args = this.Arguments([tile, slope]); value = AITile.LowerTile(tile, slope); break;
					case 2: args = this.Arguments([tile, front]); value = AITile.LevelTiles(tile, front); break;
					case 3: args = this.Arguments([tile]); value = AITile.DemolishTile(tile); break;
				}
			}
			ok = family == 4 && op >= 4 ? value != AIVehicle.VEHICLE_INVALID : value;
			print("COMMAND " + index + " " + name + " " + (round < 4 ? "bootstrap" : ["infrastructure", "vehicle", "edge"][context]) + ":" + args + " " + value + " " + (ok ? "OK" : AIError.GetLastErrorString()));
			this.Sleep(1);
		}
		print("COMMAND-END " + COMMAND_COUNT + " " + this.state);
		while (true) this.Sleep(100);
	}
}
