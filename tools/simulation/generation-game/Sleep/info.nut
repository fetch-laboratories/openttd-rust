/* OpenTTD-Rust generation evidence, licensed under GPLv2. */
class GenerationSleepInfo extends GSInfo {
	function GetAuthor() { return "OpenTTD-Rust"; }
	function GetName() { return "GenerationSleep"; }
	function GetShortName() { return "GWSl"; }
	function GetDescription() { return "Deterministic world generation startup."; }
	function GetVersion() { return 1; }
	function GetAPIVersion() { return "15"; }
	function GetDate() { return "2026-10-28"; }
	function CreateInstance() { return "GenerationSleep"; }
}
RegisterGS(GenerationSleepInfo());
