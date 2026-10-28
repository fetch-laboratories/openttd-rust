/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationCommands extends AIInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "MigrationCommands"; }
	function GetShortName() { return "CMDS"; }
	function GetDescription() { return "Seeded script command comparisons."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-10-28"; }
	function CreateInstance() { return "MigrationCommands"; }
	function UseAsRandomAI() { return false; }
}
RegisterAI(MigrationCommands());
