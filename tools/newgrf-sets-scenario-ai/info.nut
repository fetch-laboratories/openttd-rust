class MigrationNewGRFSets extends AIInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "MigrationNewGRFSets"; }
	function GetShortName() { return "MNGS"; }
	function GetDescription() { return "Builds trains and buses from file-loaded NewGRF sets."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-10-27"; }
	function CreateInstance() { return "MigrationNewGRFSets"; }
	function UseAsRandomAI() { return false; }
}
RegisterAI(MigrationNewGRFSets());
