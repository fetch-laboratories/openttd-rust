/* OpenTTD-Rust generation evidence, licensed under GPLv2. */
class GenerationCapInfo extends GSInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "GenerationCap"; }
	function GetShortName() { return "GWCa"; }
	function GetDescription() { return "Deterministic world generation startup."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-10-28"; }
	function CreateInstance() { return "GenerationCap"; }
}
RegisterGS(GenerationCapInfo());
