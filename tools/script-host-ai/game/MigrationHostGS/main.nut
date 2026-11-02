/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationHostGS extends GSController {
	phase = "constructed";
	loaded = "none";
	companies = [];
	function Log(text) {
		print("HOST gs " + GSController.GetTick() + " " + text);
	}
	function Save() {
		this.Log("save " + this.phase);
		return { phase = this.phase, tick = GSController.GetTick() };
	}
	function Load(version, data) {
		this.loaded = data.phase + "@" + data.tick;
		print("HOST gs load " + version + " " + this.loaded);
	}
	function Events() {
		local text = "";
		while (GSEventController.IsEventWaiting()) {
			local event = GSEventController.GetNextEvent();
			text += " " + event.GetEventType();
			if (event.GetEventType() == GSEvent.ET_COMPANY_NEW) {
				local company = GSEventCompanyNew.Convert(event).GetCompanyID();
				this.companies.append(company);
				text += ":" + company;
			}
		}
		this.Log("events" + text);
	}
	function Start() {
		this.Log("start loaded=" + this.loaded + " ops=" + this.GetOpsTillSuspend());
		this.Events();
		if (this.loaded != "none") {
			while (true) { this.Sleep(50); this.Log("idle " + this.phase); this.Events(); }
		}
		/* World generation runs every command as an immediate callback; the AIs start after it. */
		this.phase = "generation";
		while (this.companies.len() == 0) { this.Sleep(1); this.Events(); }
		this.phase = "typed";
		local tile = GSMap.GetTileIndex(6, 6);
		local goal = GSGoal.New(GSCompany.COMPANY_INVALID, "goal", GSGoal.GT_NONE, 0);
		local page = GSStoryPage.New(GSCompany.COMPANY_INVALID, "page");
		local element = GSStoryPage.NewElement(page, GSStoryPage.SPET_TEXT, 0, "element");
		local table = GSLeagueTable.New("table", "header", "footer");
		local row = GSLeagueTable.NewElement(table, 5, GSCompany.COMPANY_INVALID, "row", "score", GSLeagueTable.LINK_NONE, 0);
		this.Log("typed " + goal + " " + page + " " + element + " " + table + " " + row);

		/* Asynchronous commands return in the same tick and charge opcodes. */
		this.phase = "async";
		{
			local async = GSAsyncMode(true);
			local tick = this.GetTick();
			local sign = GSSign.BuildSign(tile, "async");
			this.Log("async " + sign + " " + (this.GetTick() - tick) + " " + GSSign.IsValidSign(sign));
			{
				local sync = GSAsyncMode(false);
				tick = this.GetTick();
				sign = GSSign.BuildSign(tile, "sync");
				this.Log("sync " + sign + " " + (this.GetTick() - tick));
			}
		}
		{
			local test = GSTestMode();
			this.Log("test " + GSSign.BuildSign(tile, "test") + " " + GSError.GetLastError());
		}

		/* Company mode switches the acting company and restores the deity. */
		this.phase = "company";
		{
			local sign;
			{
				local mode = GSCompanyMode(this.companies[0]);
				sign = GSSign.BuildSign(tile, "company");
				this.Log("company " + sign + " " + GSSign.GetOwner(sign) + " " + GSCompany.ResolveCompanyID(GSCompany.COMPANY_SELF));
			}
			sign = GSSign.BuildSign(tile, "deity");
			this.Log("deity " + sign + " " + GSSign.GetOwner(sign));
		}
		{
			local mode = GSCompanyMode(GSCompany.COMPANY_INVALID);
			this.Log("invalid " + GSSign.BuildSign(tile, "invalid") + " " + GSError.GetLastError());
		}
		this.Log("restored " + GSSign.BuildSign(tile, "restored"));

		this.phase = "idle";
		while (this.GetTick() < 2600) { this.Sleep(100); this.Events(); }

		/* Break pauses the instance and the game with developer tools enabled. */
		this.phase = "break";
		this.Log("break " + this.GetOpsTillSuspend());
		GSController.Break("host");
		this.Log("after break");
	}
}
