/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationInfraGSInfo extends GSInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "MigrationInfraGS"; }
	function GetShortName() { return "INFG"; }
	function GetDescription() { return "Infrastructure script API observations in deity and company modes."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-11-05"; }
	function CreateInstance() { return "MigrationInfraGS"; }
}
RegisterGS(MigrationInfraGSInfo());
