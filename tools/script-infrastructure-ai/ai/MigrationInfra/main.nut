/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
ApiAccounting <- AIAccounting;
ApiController <- AIController;
ApiError <- AIError;
ApiMap <- AIMap;
ApiRail <- AIRail;
ApiRoad <- AIRoad;
ApiStation <- AIStation;
ApiTestMode <- AITestMode;
ApiTile <- AITile;
require("probe.nut");

class MigrationInfra extends AIController {
	function Start() {
		/* One full tile-loop cycle settles snow and desert ground first. */
		this.Sleep(300);
		/* Levelling and building in every climate needs the full loan. */
		AICompany.SetLoanAmount(AICompany.GetMaxLoanAmount());
		local free = Free();
		/* Company-owned single-bit road pieces and a company road owner. */
		AIRoad.SetCurrentRoadType(AIRoadTypeList(AIRoad.ROADTRAMTYPES_ROAD).Begin());
		Record("ai", "road", Num(AIRoad.BuildRoad(free + 6, free + 7)) + " " + AIError.GetLastErrorString());
		ProbeAll("ai", free);
		while (true) this.Sleep(100);
	}
}
