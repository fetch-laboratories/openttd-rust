/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
ApiAccounting <- GSAccounting;
ApiBridge <- GSBridge;
ApiBridgeList <- GSBridgeList;
ApiBridgeList_Length <- GSBridgeList_Length;
ApiController <- GSController;
ApiError <- GSError;
ApiMap <- GSMap;
ApiMarine <- GSMarine;
ApiRail <- GSRail;
ApiRailTypeList <- GSRailTypeList;
ApiRoad <- GSRoad;
ApiRoadTypeList <- GSRoadTypeList;
ApiStation <- GSStation;
ApiTestMode <- GSTestMode;
ApiTile <- GSTile;
ApiTileList <- GSTileList;
ApiTunnel <- GSTunnel;
require("probe.nut");

class MigrationInfraGS extends GSController {
	function Start() {
		/* Let the AI company build its road before company-mode probes. */
		this.Sleep(500);
		local free = Free();
		ProbeAll("gs.deity", free);
		local company = GSCompanyMode(0);
		ProbeAll("gs.company", free);
		/* Release the previous mode first; its destructor restores deity. */
		company = null;
		company = GSCompanyMode(14);
		ProbeAll("gs.invalid", free);
		company = null;
		while (true) this.Sleep(100);
	}
}
