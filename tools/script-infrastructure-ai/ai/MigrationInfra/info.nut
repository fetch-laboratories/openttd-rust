/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationInfraInfo extends AIInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "MigrationInfra"; }
	function GetShortName() { return "INFR"; }
	function GetDescription() { return "Infrastructure script API observations."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-11-05"; }
	function CreateInstance() { return "MigrationInfra"; }
	function UseAsRandomAI() { return false; }
}
RegisterAI(MigrationInfraInfo());
