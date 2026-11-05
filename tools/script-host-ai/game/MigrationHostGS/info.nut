/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationHostGSInfo extends GSInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "MigrationHostGS"; }
	function GetShortName() { return "HSGS"; }
	function GetDescription() { return "GameScript host, command mode and event observations."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-11-02"; }
	function CreateInstance() { return "MigrationHostGS"; }
}
RegisterGS(MigrationHostGSInfo());
