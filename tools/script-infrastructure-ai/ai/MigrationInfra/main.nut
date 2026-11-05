/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
ApiAccounting <- AIAccounting;
ApiController <- AIController;
ApiError <- AIError;
ApiMap <- AIMap;
ApiTestMode <- AITestMode;
ApiTile <- AITile;
require("probe.nut");

class MigrationInfra extends AIController {
	function Start() {
		ProbeAll("ai", Free());
		while (true) this.Sleep(100);
	}
}
