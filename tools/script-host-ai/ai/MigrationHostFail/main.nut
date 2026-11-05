/* OpenTTD-Rust migration evidence, licensed under GPLv2. */
class MigrationHostFail extends AIController {
	constructor() {
		print("HOST fail constructor " + AIController.GetTick());
		/* Commands are not allowed in a constructor; the script dies before Start. */
		AISign.BuildSign(AIMap.GetTileIndex(4, 4), "fail");
		print("HOST fail unreachable");
	}
	function Start() {
		print("HOST fail start");
	}
}
