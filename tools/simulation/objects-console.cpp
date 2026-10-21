/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file objects-console.cpp Identical inputs to unchanged original and Rust-backed bodies. */
#define GetObjectCallback ObjectProbeCallback
#define SetGeneratingWorldProgress ObjectProbeSetProgress
#define IncreaseGeneratingWorldProgress ObjectProbeIncreaseProgress
#include "object_cmd.cpp"
#undef GetObjectCallback
#undef SetGeneratingWorldProgress
#undef IncreaseGeneratingWorldProgress
#include "console_internal.h"
#include "saveload/saveload.h"
#ifdef WITH_RUST
static void ResetScratch() { openttd_rust_command_before(&GetRustCommandServices(), true, false); }
#else
class AbortGenerateWorldSignal {};
struct HiddenObjectArea { size_t index; uint32_t first_tile; };
static void ResetScratch() { _cleared_object_areas.clear(); }
static HiddenObjectArea HideObjectForTunnel(TileIndex t) {
	auto *p = FindClearedObject(t); size_t i = p == nullptr ? _cleared_object_areas.size() : p - _cleared_object_areas.data();
	if (p == nullptr) _cleared_object_areas.emplace_back(t, TileArea(t, 1, 1));
	uint32_t first = _cleared_object_areas[i].first_tile.base(); _cleared_object_areas[i].first_tile = INVALID_TILE; return {i, first};
}
static void RestoreObjectFirstTile(HiddenObjectArea h) { _cleared_object_areas[h.index].first_tile = TileIndex(h.first_tile); }
static bool IsObjectIndirectlyCleared(TileIndex t) { auto *p = FindClearedObject(t); return p != nullptr && p->first_tile != t; }
#endif
uint16_t GetObjectCallback(CallbackID, uint32_t, uint32_t, const ObjectSpec *, Object *, TileIndex, std::span<int32_t>, uint8_t);
void SetGeneratingWorldProgress(GenWorldProgress, uint);
void IncreaseGeneratingWorldProgress(GenWorldProgress);
static int cancellation = -1;
static bool callbacks = false;
uint16_t ObjectProbeCallback(CallbackID id, uint32_t a, uint32_t b, const ObjectSpec *s, Object *o, TileIndex t, std::span<int32_t> regs, uint8_t view)
{
	if (!callbacks) return GetObjectCallback(id, a, b, s, o, t, regs, view);
	std::ranges::fill(regs, 0);
	if (id == CBID_OBJECT_LAND_SLOPE_CHECK) {
		fmt::print(stderr, "OBJECT callback-before-clear {}\n", IsObjectTypeTile(t, OBJECT_OWNED_LAND));
		return s->grf_prop.grffile->grf_version < 8 ? 0 : 0x400;
	}
	return id == CBID_OBJECT_COLOUR ? 0xA5 : 0;
}
void ObjectProbeSetProgress(GenWorldProgress kind, uint total)
{
	if (cancellation == 0) throw AbortGenerateWorldSignal{};
	if (cancellation < 0) SetGeneratingWorldProgress(kind, total);
}
void ObjectProbeIncreaseProgress(GenWorldProgress kind)
{
	if (cancellation == 1) throw AbortGenerateWorldSignal{};
	IncreaseGeneratingWorldProgress(kind);
}
static void Result(const char *name, const CommandCost &cost)
{
	fmt::print(stderr, "OBJECT cost {} {} {} {}\n", name, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage());
}
template<typename Tile> static void Loop(Tile tile) {
	if constexpr (std::is_invocable_v<decltype(&TileLoop_Object), Tile>) TileLoop_Object(tile); else TileLoop_Object(tile.base());
}
/** Exercise native resolver entry points and callbacks identically in both games. */
struct ObjectPolicyGroup : SpriteGroup {
	ObjectPolicyGroup() {} // Preserve the pool-assigned index during construction.
	mutable uint32_t calls = 0, word = 0, trigger = 0;
	mutable Randomizer expected{};
	mutable bool equal = true;
	bool disable = false, refresh = true;
	ResolverResult Resolve(ResolverObject &ro) const override {
		auto &scope = static_cast<ObjectResolverObject &>(ro).object_scope;
		equal &= ro.callback == CBID_OBJECT_ANIMATION_TRIGGER && ro.callback_param1 == word && ro.callback_param2 == trigger;
		fmt::print(stderr, "OBJECT animation-callback {} {} {}\n", calls++, scope.tile.base(), ro.callback_param1);
		equal &= scope.GetRandomBits() == (IsTileType(scope.tile, MP_OBJECT) ? GetObjectRandomBits(scope.tile) : 0);
		if (refresh) SB(word, 0, 16, expected.Next());
		if (disable) const_cast<ObjectSpec *>(scope.spec)->animation.triggers = {};
		return CallbackResult(0xFD);
	}
};
static void ObjectPolicyProbe(Object *o, ObjectSpec *spec)
{
	Backup<ObjectSpec> backup(*spec); Backup<TileArea> location(o->location);
	GRFFile file; file.grf_version = 8; file.grfid = 0x12345678;
	spec->grf_prop.grffile = &file; spec->grf_prop.grfid = file.grfid; spec->grf_prop.local_id = 17;
	ObjectResolverObject live(spec, o, o->location.tile), empty(spec, nullptr, o->location.tile, 3), invalid(spec, nullptr, INVALID_TILE, 2);
	for (auto *r : {&live, &empty, &invalid}) for (bool incoming : {false, true}) {
		for (uint8_t var : {0x40,0x41,0x42,0x43,0x44,0x45,0x46,0x47,0x48,0x60,0x61,0x62,0x63,0x64,0x7A,0xFF}) {
			bool available = incoming; uint32_t value = r->object_scope.GetVariable(var, 0, available);
			fmt::print(stderr, "OBJECT query {} {} {} {} {}\n", r == &live ? 0 : r == &empty ? 1 : 2, var, incoming, available, value);
		}
		fmt::print(stderr, "OBJECT random {}\n", r->object_scope.GetRandomBits());
	}
	for (TileIndex t : {o->location.tile + 1, INVALID_TILE}) {
		ObjectResolverObject other(spec, o, t);
		for (uint8_t var : {0x40,0x42,0x47,0x48,0x64}) { bool a = true; fmt::print(stderr, "OBJECT query-live-tile {} {} {}\n", t.base(), var, other.object_scope.GetVariable(var, 0, a)); }
	}
	Backup<std::vector<EntityIDMapping>> mappings(_object_mngr.mappings);
	_object_mngr.mappings[o->type] = {file.grfid, 17, o->type};
	_object_mngr.mappings[OBJECT_LIGHTHOUSE] = {file.grfid + 1, 17, OBJECT_LIGHTHOUSE};
	for (uint32_t grfid : {0U, UINT32_MAX, file.grfid, file.grfid + 1, 0x87654321U}) {
		live.SetRegister(0x100, grfid); bool available = false;
		fmt::print(stderr, "OBJECT query-count {} {}\n", grfid, live.object_scope.GetVariable(0x64, grfid == 0 ? o->type : 17, available));
	}
	bool same = true;
	for (uint32_t p : {0U, 1U, 0xFFU, 0x101U}) for (uint8_t var : {0x60,0x61,0x62,0x63}) {
		bool available = true; fmt::print(stderr, "OBJECT query-near {} {} {}\n", var, p, live.object_scope.GetVariable(var, p, available));
	}
	for (bool has_file : {false, true}) for (uint32_t grfid : {file.grfid, file.grfid + 1}) {
		spec->grf_prop.grffile = has_file ? &file : nullptr; spec->grf_prop.grfid = grfid;
		bool available = true; fmt::print(stderr, "OBJECT query-id {} {} {}\n", has_file, grfid, live.object_scope.GetVariable(0x60, 0, available));
	}
	for (uint8_t version : {7, 8}) { file.grf_version = version; bool a = true; fmt::print(stderr, "OBJECT query-version {} {}\n", version, live.object_scope.GetVariable(0x62, 1, a)); }
	spec->grf_prop.grffile = &file;
	SpriteGroup::CanAllocateItem();
	auto *group = new ObjectPolicyGroup(); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, group);
	Randomizer expected = _random; expected.Next(); spec->animation.triggers = {};
	same &= !TriggerObjectTileAnimation(o, o->location.tile, ObjectAnimationTrigger::Built, spec);
	same &= std::ranges::equal(_random.state, expected.state);
	same &= !TriggerObjectAnimation(o, ObjectAnimationTrigger::Built, spec) && std::ranges::equal(_random.state, expected.state);
	group->refresh = false; group->trigger = 1; group->expected = _random; group->word = group->expected.Next(); spec->animation.triggers = {ObjectAnimationTrigger::TileLoop};
	same &= TriggerObjectTileAnimation(o, o->location.tile, ObjectAnimationTrigger::TileLoop, spec) && group->calls == 1 && group->equal && std::ranges::equal(_random.state, group->expected.state);
	group->refresh = true; group->trigger = 0;
	o->location.w = o->location.h = 2;
	for (bool disable : {false, true}) {
		group->calls = 0; group->disable = disable; group->expected = _random; group->word = group->expected.Next();
		spec->animation.triggers = {ObjectAnimationTrigger::Built};
		bool result = TriggerObjectAnimation(o, ObjectAnimationTrigger::Built, spec);
		same &= result == !disable && group->calls == (disable ? 1U : 4U) && group->equal && std::ranges::equal(_random.state, group->expected.state);
	}
	delete group;
	fmt::print(stderr, "OBJECT query-animation-order {}\n", same);
}
static bool ObjectsScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	Company *c = *Company::Iterate().begin();
	Backup<CompanyID> company(_current_company, c->index);
	if (args[1] == "inspect") {
		std::array<uint16_t, NUM_OBJECTS> counts{};
		for (const Object *o : Object::Iterate()) counts[o->type]++;
		bool equal = true;
		for (ObjectType type = 0; type < NUM_OBJECTS; type++) equal &= counts[type] == Object::GetTypeCount(type);
		fmt::print(stderr, "OBJECT afterload-counts {}\n", equal);
		return true;
	}
#ifdef WITH_RUST
	auto &finances = c->Finances();
#else
	auto &finances = *c;
#endif
	finances.money = 1000000000;
	finances.build_object_limit = finances.clear_limit = UINT32_MAX;
	TileIndex tile = INVALID_TILE; std::vector<uint8_t> heights;
	for (uint y = 2; y + 14 < Map::MaxY() && tile == INVALID_TILE; y++) for (uint x = 2; x + 14 < Map::MaxX(); x++) {
		TileArea area(TileXY(x, y), 14, 14);
		bool clear = true; for (TileIndex t : area) clear &= IsTileType(t, MP_CLEAR); if (!clear) continue;
		tile = TileXY(x + 3, y + 3);
		for (TileIndex t : area) { heights.push_back(TileHeight(t)); SetTileHeight(t, 4); }
		break;
	}
	if (tile == INVALID_TILE) std::abort();
	auto build = [&](ObjectType type, TileIndex t) { return Command<CMD_BUILD_OBJECT>::Do(DoCommandFlag::Execute, t, type, 0); };
	if (args[1] == "gaps") {
		auto before = Object::GetTypeCount(63999);
		for (uint i = 0; i < 65536; i++) Object::IncTypeCount(63999);
		Object::CanAllocateItem(); Object *discarded = new Object(); discarded->type = 63999; delete discarded;
		fmt::print(stderr, "OBJECT count-wrap {}\n", Object::GetTypeCount(63999) == before);
		for (uint i = 0; i < 65536; i++) Object::DecTypeCount(63999);
		Backup<bool> bulldozer(_cheats.magic_bulldozer.value, true);
		Result("scratch-hq", build(OBJECT_HQ, tile)); Result("scratch-clear", Command<CMD_LANDSCAPE_CLEAR>::Do({}, tile));
		auto hidden = HideObjectForTunnel(tile + 1);
		for (uint i = 0; i < 4096; i++) HideObjectForTunnel(tile + TileDiffXY(0, 3) + i);
		RestoreObjectFirstTile(hidden); auto first = HideObjectForTunnel(tile + 1); RestoreObjectFirstTile(first);
		fmt::print(stderr, "OBJECT scratch-growth {} {}\n", !IsObjectIndirectlyCleared(tile), first.index == hidden.index && first.first_tile == tile.base());
		Result("scratch-remove", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile));
		GRFFile file; file.grf_version = 8;
		ObjectSpec *spec = const_cast<ObjectSpec *>(ObjectSpec::Get(OBJECT_TRANSMITTER));
		Backup<ObjectSpec> backup(*spec);
		spec->flags = {}; spec->callback_mask = {ObjectCallbackMask::SlopeCheck, ObjectCallbackMask::Colour}; spec->grf_prop.grffile = &file;
		callbacks = true;
		for (uint8_t version : {7, 8}) {
			file.grf_version = version; Result("owned", build(OBJECT_OWNED_LAND, tile));
			ResetScratch(); SetBridgeMiddle(tile, AXIS_X);
			RecursiveCommandCounter nested;
			Result("bridge-after-clear", CmdBuildObject(DoCommandFlag::Execute, tile, OBJECT_TRANSMITTER, 0));
			fmt::print(stderr, "OBJECT cleared-before-bridge {}\n", IsTileType(tile, MP_CLEAR));
			ClearBridgeMiddle(tile);
		}
		Result("colour", build(OBJECT_TRANSMITTER, tile));
		fmt::print(stderr, "OBJECT callback-colour {}\n", Object::GetByTile(tile)->colour == 0xA5);
		ObjectPolicyProbe(Object::GetByTile(tile), spec);
		callbacks = false; backup.Restore();
		Backup<bool> generating(_generating_world, true);
		for (cancellation = 0; cancellation < 2; cancellation++) {
			bool caught = false;
			try { GenerateObjects(); } catch (AbortGenerateWorldSignal &) { caught = true; }
			fmt::print(stderr, "OBJECT cancellation {} {}\n", cancellation, caught);
		}
		cancellation = -1;
	} else {
		Result("hq-test", Command<CMD_BUILD_OBJECT>::Do({}, tile, OBJECT_HQ, 0));
		Result("hq", build(OBJECT_HQ, tile));
		Result("same-hq", build(OBJECT_HQ, tile));
		TileIndex relocated = tile + 3; Result("relocate", build(OBJECT_HQ, relocated));
		UpdateCompanyHQ(relocated, 1000); UpdateObjectColours(c);
		Backup<bool> autoslope(_settings_game.construction.autoslope, true);
		Result("hq-terraform", TerraformTile_Object(relocated, {}, GetTileMaxZ(relocated) - 1, SLOPE_W));
		Result("hq-build-above", CheckBuildAbove_Object(relocated, {}, AXIS_X, GetTileMaxZ(relocated) + 10));
		CargoArray accepted{}, produced{}; CargoTypes always = 0;
		AddAcceptedCargo_Object(relocated, accepted, always); AddProducedCargo_Object(relocated, produced);
		fmt::print(stderr, "OBJECT hq-cargo {} {} {}\n", accepted[GetCargoTypeByLabel(CT_PASSENGERS)], accepted[GetCargoTypeByLabel(CT_MAIL)], produced[GetCargoTypeByLabel(CT_MAIL)]);
		for (uint i = 0; i < 256; i++) Loop(relocated);
		Result("area", Command<CMD_BUILD_OBJECT_AREA>::Do(DoCommandFlag::Execute, tile + TileDiffXY(2, 5), tile + TileDiffXY(0, 5), OBJECT_OWNED_LAND, 0, false));
		Result("diagonal", Command<CMD_BUILD_OBJECT_AREA>::Do(DoCommandFlag::Execute, tile + TileDiffXY(2, 7), tile + TileDiffXY(0, 7), OBJECT_OWNED_LAND, 0, true));
		TileIndex owned = tile + TileDiffXY(0, 5); ObjectID old = Object::GetByTile(owned)->index;
		Result("owned-terraform", TerraformTile_Object(owned, {}, GetTileZ(owned), SLOPE_FLAT));
		Result("clear", Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, owned)); Result("reuse", build(OBJECT_OWNED_LAND, owned));
		fmt::print(stderr, "OBJECT id-reused {}\n", old == Object::GetByTile(owned)->index);
		ChangeTileOwner_Object(owned, c->index, c->index);
		Object::CanAllocateItem(); BuildObject(OBJECT_STATUE, tile + TileDiffXY(5, 5), c->index);
		ChangeTileOwner_Object(tile + TileDiffXY(5, 5), c->index, INVALID_OWNER);
	}
	uint index = 0; for (TileIndex t : TileArea(tile - TileDiffXY(3, 3), 14, 14)) SetTileHeight(t, heights[index++]);
	if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	std::fflush(stderr); return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("objects_scenario", ObjectsScenario);
}
