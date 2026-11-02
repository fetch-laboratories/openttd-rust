/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationHostFailInfo extends AIInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "MigrationHostFail"; }
	function GetShortName() { return "HSTF"; }
	function GetDescription() { return "A script whose constructor issues a command and fails to start."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-11-02"; }
	function CreateInstance() { return "MigrationHostFail"; }
	function UseAsRandomAI() { return false; }
}
RegisterAI(MigrationHostFailInfo());
