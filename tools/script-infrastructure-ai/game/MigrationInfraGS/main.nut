/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
ApiAccounting <- GSAccounting;
ApiController <- GSController;
ApiError <- GSError;
ApiMap <- GSMap;
ApiRail <- GSRail;
ApiRoad <- GSRoad;
ApiStation <- GSStation;
ApiTestMode <- GSTestMode;
ApiTile <- GSTile;
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
