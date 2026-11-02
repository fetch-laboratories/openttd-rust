/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationHost extends AIController {
	phase = "constructed";
	loaded = "none";
	constructor() {
		print("HOST ai constructor " + AIController.GetTick());
	}
	function Log(text) {
		print("HOST ai " + AIController.GetTick() + " " + text);
	}
	function Save() {
		this.Log("save " + this.phase);
		return { phase = this.phase, tick = AIController.GetTick() };
	}
	function Load(version, data) {
		this.loaded = data.phase + "@" + data.tick;
		print("HOST ai load " + version + " " + this.loaded);
	}
	function Events() {
		local text = "";
		while (AIEventController.IsEventWaiting()) {
			local event = AIEventController.GetNextEvent();
			text += " " + event.GetEventType();
			if (event.GetEventType() == AIEvent.ET_COMPANY_NEW) text += ":" + AIEventCompanyNew.Convert(event).GetCompanyID();
		}
		this.Log("events" + text);
	}
	function Free() {
		for (local tile = AIMap.GetTileIndex(8, 8); tile < AIMap.GetMapSize(); tile++) {
			if (AITile.IsBuildable(tile) && AITile.GetSlope(tile) == AITile.SLOPE_FLAT && !AITile.HasTreeOnTile(tile)) return tile;
		}
		throw "No free tile";
	}
	function Start() {
		this.Log("start loaded=" + this.loaded + " ops=" + this.GetOpsTillSuspend());
		if (this.loaded != "none") {
			/* A reload resumes from saved data, then idles until the run ends. */
			this.Events();
			while (true) { this.Sleep(50); this.Log("idle " + this.phase); }
		}
		this.Events();
		this.phase = "sleep";
		this.Sleep(5);
		this.Log("slept 5");
		this.Sleep(0);
		this.Log("slept 0");

		/* Typed callbacks under a three-tick command delay. */
		this.phase = "commands";
		local tile = this.Free();
		AIController.SetCommandDelay(3);
		local sign = AISign.BuildSign(tile, "host");
		this.Log("sign " + sign + " " + AISign.IsValidSign(sign));
		AIController.SetCommandDelay(1);
		local first = AIGroup.CreateGroup(AIVehicle.VT_ROAD, AIGroup.GROUP_INVALID);
		local second = AIGroup.CreateGroup(AIVehicle.VT_ROAD, first);
		this.Log("groups " + first + " " + second + " " + AIGroup.GetParent(second));
		local named = AIGroup.SetName(first, "Host");
		/* A group cannot become a child of its own child. */
		local recursion = AIGroup.SetParent(first, second);
		this.Log("error " + named + " " + recursion + " " + AIError.GetLastError() + " " + AIError.GetLastErrorString());

		/* Estimates and executed costs, nested modes and their restoration. */
		this.phase = "costs";
		local costs = AIAccounting();
		local estimate;
		{
			local test = AITestMode();
			estimate = AITile.PlantTree(tile);
			{
				local exec = AIExecMode();
				this.Log("nested exec " + AISign.BuildSign(tile, "nested"));
			}
			this.Log("test " + estimate + " " + costs.GetCosts() + " " + AITile.HasTreeOnTile(tile));
		}
		local planted = AITile.PlantTree(tile);
		this.Log("exec " + planted + " " + costs.GetCosts() + " " + AITile.HasTreeOnTile(tile));
		this.Log("rand " + AIBase.Rand() + " " + AIBase.RandRange(1000));

		/* The opcode budget suspends a busy loop between ticks. */
		this.phase = "busy";
		local before = this.GetTick();
		local sum = 0;
		for (local i = 0; i < 40000; i++) sum += i % 7;
		this.Log("busy " + sum + " " + (this.GetTick() - before));
		this.Events();

		this.phase = "idle";
		for (local i = 0; i < 4; i++) { this.Sleep(25); this.Log("idle " + i); }

		/* Releasing a mode object out of order is fatal. */
		this.phase = "death";
		this.Log("death");
		local test = AITestMode();
		local exec = AIExecMode();
		test = null;
		this.Log("unreachable");
	}
}
