/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
ApiAccounting <- GSAccounting;
ApiController <- GSController;
ApiError <- GSError;
ApiMap <- GSMap;
ApiTestMode <- GSTestMode;
ApiTile <- GSTile;
require("probe.nut");

class MigrationInfraGS extends GSController {
	function Start() {
		/* Let the AI company start before company-mode probes. */
		this.Sleep(10);
		local free = Free();
		ProbeAll("gs.deity", free);
		local company = GSCompanyMode(0);
		ProbeAll("gs.company", free);
		company = GSCompanyMode(14);
		ProbeAll("gs.invalid", free);
		company = null;
		while (true) this.Sleep(100);
	}
}
