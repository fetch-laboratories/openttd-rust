/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationHostInfo extends AIInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "MigrationHost"; }
	function GetShortName() { return "HOST"; }
	function GetDescription() { return "Script host lifecycle, command and event observations."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-11-02"; }
	function CreateInstance() { return "MigrationHost"; }
	function UseAsRandomAI() { return false; }
}
RegisterAI(MigrationHostInfo());
