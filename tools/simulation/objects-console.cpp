/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file objects-console.cpp Identical inputs to unchanged original and Rust-backed bodies. */
#include "saveload/saveload.cpp" // Unchanged native reader/dispatcher; relink replaces this object.
#define SetGeneratingWorldProgress ObjectProbeSetProgress
#define IncreaseGeneratingWorldProgress ObjectProbeIncreaseProgress
#include "object_cmd.cpp"
#undef SetGeneratingWorldProgress
#undef IncreaseGeneratingWorldProgress
#include "console_internal.h"
#include "saveload/saveload.h"
#ifdef WITH_RUST
using HiddenObjectArea = OpenTTDObjectHiddenArea;
static HiddenObjectArea HideObjectForTunnel(TileIndex t) { return openttd_rust_object_tunnel_hide(t.base(), GetRustSharedServices().map_log_x()); }
static void RestoreObjectFirstTile(HiddenObjectArea h) { openttd_rust_object_restore_first(h); }
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
#ifdef WITH_RUST
		TileIndex tile{scope.context.tile}; const ObjectSpec *spec = scope.context.spec;
#else
		TileIndex tile = scope.tile; const ObjectSpec *spec = scope.spec;
#endif
		equal &= ro.callback == CBID_OBJECT_ANIMATION_TRIGGER && ro.callback_param1 == word && ro.callback_param2 == trigger;
		fmt::print(stderr, "OBJECT animation-callback {} {} {}\n", calls++, tile.base(), ro.callback_param1);
		equal &= scope.GetRandomBits() == (IsTileType(tile, MP_OBJECT) ? GetObjectRandomBits(tile) : 0);
		if (refresh) SB(word, 0, 16, expected.Next());
		if (disable) const_cast<ObjectSpec *>(spec)->animation.triggers = {};
		return CallbackResult(0xFD);
	}
};
struct ObjectClearOrderGroup : SpriteGroup {
	ResolverResult Resolve(ResolverObject &ro) const override {
		if (ro.callback != CBID_OBJECT_LAND_SLOPE_CHECK) return CallbackResult(0xA5);
		auto &scope = static_cast<ObjectResolverObject &>(ro).object_scope;
#ifdef WITH_RUST
		TileIndex tile{scope.context.tile};
#else
		TileIndex tile = scope.tile;
#endif
		fmt::print(stderr, "OBJECT callback-before-clear {}\n", IsObjectTypeTile(tile, OBJECT_OWNED_LAND));
		return CallbackResult(ro.grffile->grf_version < 8 ? 0 : 0x400);
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
	spec->flags.Set(ObjectFlag::Animation);
	spec->animation.triggers = {ObjectAnimationTrigger::TileLoopNorth};
	group->calls = 0;
	group->disable = false;
	group->trigger = to_underlying(ObjectAnimationTrigger::TileLoopNorth);
	group->expected = _random;
	group->expected.Next(); // The inactive single-tile TileLoop trigger still draws.
	group->word = group->expected.Next();
	Loop(o->location.tile);
	same &= group->calls == 4 && group->equal && std::ranges::equal(_random.state, group->expected.state);

	delete group;
	fmt::print(stderr, "OBJECT query-animation-order {}\n", same);
}
static void ObjectNeed(bool value, int line)
{
	if (!value) { fmt::print(stderr, "OBJECT invocation-failed {}\n", line); std::abort(); }
}
/** Actual deterministic/procedure/randomized Action2, reconstructed after load. */
static void ObjectInvocationProbe(TileIndex tile, bool reloaded)
{
	extern std::vector<ObjectSpec> _object_specs;
	const auto saved_count = _object_specs.size(); const ObjectType kind = saved_count;
	ObjectSpec fixture = *ObjectSpec::Get(OBJECT_TRANSMITTER); _object_specs.push_back(fixture);
	ObjectSpec *spec = &_object_specs.back();
	std::array<uint8_t, 4> heights{}; uint corner = 0;
	for (TileIndex t : TileArea(tile, 2, 2)) { heights[corner++] = TileHeight(t); SetTileHeight(t, 4); }
	GRFFile file; file.grfid = 0x43004300; file.grf_version = 8;
	spec->grf_prop.grffile = &file; spec->grf_prop.grfid = file.grfid; spec->grf_prop.local_id = 23;
	spec->flags = {}; spec->views = 4; spec->size = OBJECT_SIZE_1X1; spec->climate = LandscapeTypes::All();
	spec->introduction_date = TimerGameCalendar::date - 400; spec->end_of_life_date = TimerGameCalendar::date + 400;
	spec->callback_mask = {ObjectCallbackMask::SlopeCheck, ObjectCallbackMask::Colour, ObjectCallbackMask::Autoslope};
	std::vector<SpriteGroup *> groups;
	auto result = [&](uint16_t value) { ObjectNeed(SpriteGroup::CanAllocateItem(), __LINE__); auto *g = new CallbackResultSpriteGroup(value); groups.push_back(g); return g; };
	auto program = [&](VarSpriteGroupScope scope, const SpriteGroup *end) { ObjectNeed(SpriteGroup::CanAllocateItem(), __LINE__); auto *g = new DeterministicSpriteGroup(); groups.push_back(g); g->size = DSG_SIZE_DWORD; g->var_scope = scope; g->default_result = {false, end}; return g; };
	auto *slope = result(0x400), *frame = result(1), *autoslope = result(0), *procedure_result = result(0x77);
	auto *procedure = program(VSG_SCOPE_SELF, procedure_result);
	procedure->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x48, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}, {.operation = DSGA_OP_RST, .variable = 0x10, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x101}, {.operation = DSGA_OP_RST, .variable = 0x18, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x102}};
	ObjectNeed(SpriteGroup::CanAllocateItem(), __LINE__); auto *random = new RandomizedSpriteGroup(); groups.push_back(random);
	random->var_scope = VSG_SCOPE_SELF; random->groups = {result(0xA4), result(0xA5)};
	auto *live_root = program(VSG_SCOPE_SELF, frame), *purchase_root = program(VSG_SCOPE_SELF, frame);
	for (auto *root : {live_root, purchase_root}) {
		root->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x7E, .and_mask = UINT32_MAX, .subroutine = procedure}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x103}, {.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = root == live_root ? 111U : 222U}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x104}, {.operation = DSGA_OP_RST, .variable = 0x0C, .and_mask = UINT32_MAX}};
		root->ranges = {{{false, slope}, CBID_OBJECT_LAND_SLOPE_CHECK, CBID_OBJECT_LAND_SLOPE_CHECK}, {{false, random}, CBID_OBJECT_COLOUR, CBID_OBJECT_COLOUR}, {{false, autoslope}, CBID_OBJECT_AUTOSLOPE, CBID_OBJECT_AUTOSLOPE}};
	}
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, live_root); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, purchase_root);
	auto tested = Command<CMD_BUILD_OBJECT>::Do({}, tile, kind, 3); Result("invocation-test", tested); ObjectNeed(tested.Succeeded(), __LINE__);
	auto built = Command<CMD_BUILD_OBJECT>::Do(DoCommandFlag::Execute, tile, kind, 3); Result("invocation-build", built); ObjectNeed(built.Succeeded(), __LINE__);
	Object *object = Object::GetByTile(tile);
	ObjectNeed(object->colour == 0xA4, __LINE__);
	ObjectNeed(GetObjectCallback(CBID_OBJECT_COLOUR, 0, 0, spec, object, tile) == 0xA4 + (GetObjectRandomBits(tile) & 1), __LINE__);
	std::array<int32_t, 5> regs{};
	auto callback = GetObjectCallback(CBID_OBJECT_LAND_SLOPE_CHECK, 0x12345678, 0x87654321, spec, nullptr, tile, regs, 2);
	ObjectNeed(callback == 0x400 && regs == std::array<int32_t, 5>{2, 0x12345678, static_cast<int32_t>(0x87654321), 0x77, 222}, __LINE__);
	ObjectResolverObject live(spec, object, tile), purchase(spec, nullptr, tile, 2), invalid(spec, nullptr, INVALID_TILE);
	ObjectNeed(live.root_spritegroup == live_root && purchase.root_spritegroup == purchase_root && invalid.root_spritegroup == purchase_root, __LINE__);
	ObjectNeed(live.GetFeature() == GSF_OBJECTS && live.GetDebugID() == 23 && live.callback == CBID_NO_CALLBACK && live.callback_param1 == 0 && live.callback_param2 == 0, __LINE__);
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, nullptr);
	ObjectResolverObject fallback(spec, nullptr, tile); ObjectNeed(fallback.root_spritegroup == live_root, __LINE__);
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, nullptr);
	ObjectResolverObject missing(spec, object, tile); regs.fill(0x1234); ObjectNeed(missing.ResolveCallback(regs) == CALLBACK_FAILED && regs[0] == 0x1234, __LINE__);
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, live_root); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, purchase_root);
	fmt::print(stderr, "OBJECT invocation-roots {} {} {}\n", reloaded, callback, object->colour);
	/* The procedure reenters self queries; the parent program enters integrated
	 * town persistent storage and preserves readonly construction semantics. */
	auto *parent = program(VSG_SCOPE_PARENT, slope);
	parent->adjusts = {{.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x100}, {.operation = DSGA_OP_RST, .variable = 0x1A, .and_mask = 0x430}, {.operation = DSGA_OP_STOP, .variable = 0x1A, .and_mask = 3}, {.operation = DSGA_OP_RST, .variable = 0x7C, .parameter = 3, .and_mask = UINT32_MAX}, {.operation = DSGA_OP_STO, .variable = 0x1A, .and_mask = 0x101}};
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, parent); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, parent);
	ObjectResolverObject writable(spec, object, tile), readonly(spec, nullptr, tile);
	ObjectNeed(writable.ResolveCallback(regs) == 0x400 && regs[1] == 0x430, __LINE__);
	auto *town = object->town; auto *cached = writable.GetScope(VSG_SCOPE_PARENT);
	object->town = nullptr; ObjectNeed(writable.GetScope(VSG_SCOPE_PARENT) == cached && writable.ResolveCallback(regs) == 0x400, __LINE__);
	ObjectResolverObject retry(spec, object, tile);
	ObjectNeed(retry.GetScope(VSG_SCOPE_PARENT, 239) == retry.ResolverObject::GetScope(VSG_SCOPE_PARENT, 239) && retry.ResolveCallback({}) == CALLBACK_FAILED, __LINE__);
	object->town = town; ObjectNeed(retry.ResolveCallback(regs) == 0x400 && regs[1] == 0x430, __LINE__);
	parent->adjusts[2].and_mask = 0x431; ObjectNeed(readonly.ResolveCallback(regs) == 0x400 && regs[1] == 0x430, __LINE__);
	ObjectNeed(writable.GetScope(VSG_SCOPE_RELATIVE, 127) == writable.ResolverObject::GetScope(VSG_SCOPE_RELATIVE, 127), __LINE__);
	fmt::print(stderr, "OBJECT invocation-parent {} {}\n", town->index.base(), regs[1]);
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, live_root); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, purchase_root);
	Backup<bool> autosloping(_settings_game.construction.autoslope, true);
	Result("invocation-autoslope", TerraformTile_Object(tile, {}, GetTileZ(tile), GetTileSlope(tile)));
	spec->flags.Set(ObjectFlag::Animation); spec->animation.frames = 3; spec->animation.status = AnimationStatus::Looping; spec->animation.speed = 0;
	spec->animation.triggers = {ObjectAnimationTrigger::Built};
	ObjectNeed(TriggerObjectAnimation(object, ObjectAnimationTrigger::Built, spec) && GetAnimationFrame(tile) == 1, __LINE__);
	spec->callback_mask.Set({ObjectCallbackMask::AnimationSpeed, ObjectCallbackMask::AnimationNextFrame});
	AnimateNewObjectTile(tile); ObjectNeed(GetAnimationFrame(tile) == 1, __LINE__);
	fmt::print(stderr, "OBJECT invocation-animation {}\n", GetAnimationFrame(tile));
	/* Eligibility is observed through actual command test and execute admission.
	 * The temporary editor/world-generation modes use the original owner rules. */
	spec->flags = {}; spec->callback_mask = {}; spec->animation.triggers = {};
	Backup<bool> bulldozer(_cheats.magic_bulldozer.value, true);
	ObjectNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile).Succeeded(), __LINE__);
	for (auto mode : {GM_NORMAL, GM_EDITOR}) for (bool generating : {false, true}) for (uint8_t boundary = 0; boundary < 8; boundary++) {
		Backup<GameMode> game_mode(_game_mode, mode); Backup<bool> world(_generating_world, generating);
		Backup<CompanyID> company(_current_company, mode == GM_EDITOR || generating ? OWNER_NONE : _current_company);
		spec->flags = {}; spec->views = boundary == 0 ? 0 : 4;
		spec->climate = boundary == 1 ? LandscapeTypes{} : LandscapeTypes::All();
		spec->introduction_date = TimerGameCalendar::date - (boundary == 2 ? 0 : 400);
		spec->end_of_life_date = TimerGameCalendar::date + (boundary == 3 ? 0 : boundary == 4 ? -36 : boundary == 5 ? -35 : 400);
		if (boundary >= 6) spec->flags.Set(boundary == 6 ? ObjectFlag::OnlyInScenedit : ObjectFlag::OnlyInGame);
		auto test = Command<CMD_BUILD_OBJECT>::Do({}, tile, kind, 0);
		auto execute = Command<CMD_BUILD_OBJECT>::Do(DoCommandFlag::Execute, tile, kind, 0);
		ObjectNeed(test.Succeeded() == execute.Succeeded(), __LINE__);
		fmt::print(stderr, "OBJECT invocation-eligibility {} {} {} {} {} {} {} {}\n", to_underlying(mode), generating, boundary, spec->IsEnabled(), spec->IsEverAvailable(), spec->WasEverAvailable(), spec->IsAvailable(), execute.Succeeded());
		if (execute.Succeeded()) ObjectNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile).Succeeded(), __LINE__);
	}
	/* Leave a real object in the save with transient groups/specs detached. The
	 * inspect path repeats this installation after native load, never serializes it. */
	_object_specs.resize(saved_count);
	ObjectNeed(Command<CMD_BUILD_OBJECT>::Do(DoCommandFlag::Execute, tile, OBJECT_TRANSMITTER, 0).Succeeded(), __LINE__);
	corner = 0; for (TileIndex t : TileArea(tile, 2, 2)) SetTileHeight(t, heights[corner++]);
	for (auto *group : groups) delete group;
	fmt::print(stderr, "OBJECT invocation-complete {}\n", reloaded);
}

#include "animation-console.hpp"
static bool ObjectsScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1].starts_with("anit-")) AnimationLoadProbe(args[1].substr(5));
	Company *c = *Company::Iterate().begin();
	Backup<CompanyID> company(_current_company, c->index);
	if (args[1] == "inspect") {
		AnimationReloadObservation();
		TileIndex invocation_tile = INVALID_TILE;
		for (const Object *o : Object::Iterate()) if (o->type == OBJECT_TRANSMITTER) { invocation_tile = o->location.tile; break; }
		ObjectNeed(invocation_tile != INVALID_TILE, __LINE__);
		ObjectNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, invocation_tile).Succeeded(), __LINE__);
		ObjectInvocationProbe(invocation_tile, true);
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
		ObjectNeed(SpriteGroup::CanAllocateItem(), __LINE__); auto *clear_group = new ObjectClearOrderGroup();
		spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, clear_group); spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, nullptr);
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
		AnimationBindingsProbe();
		backup.Restore(); delete clear_group;
		Backup<bool> generating(_generating_world, true);
		for (cancellation = 0; cancellation < 2; cancellation++) {
			bool caught = false;
			try { GenerateObjects(); } catch (AbortGenerateWorldSignal &) { caught = true; }
			fmt::print(stderr, "OBJECT cancellation {} {}\n", cancellation, caught);
		}
		cancellation = -1;
		AnimationLifecycleProbe(tile);
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
		TileIndex sea = INVALID_TILE; for (TileIndex t : Map::Iterate()) if (sea == INVALID_TILE && IsTileType(t, MP_WATER) && IsSea(t) && GetTileSlopeZ(t) == std::tuple{SLOPE_FLAT, 0}) sea = t;
		MakeCanal(tile + 7, OWNER_NONE, Random()); MakeRiver(tile + 8, Random()); fmt::print(stderr, "OBJECT water-clear"); /* Clearing restores each class, then ownership redispatches. */
		for (TileIndex t : {tile + 7, tile + 8, sea}) { Object::CanAllocateItem(); BuildObject(OBJECT_OWNED_LAND, t, c->index); ChangeTileOwner(t, c->index, INVALID_OWNER); fmt::print(stderr, " {} {}", to_underlying(GetWaterClass(t)), GetTileOwner(t).base()); }
		fmt::print(stderr, "\n");
	}
	ObjectInvocationProbe(tile + TileDiffXY(8, 8), false);
	uint index = 0; for (TileIndex t : TileArea(tile - TileDiffXY(3, 3), 14, 14)) SetTileHeight(t, heights[index++]);
	if (args.size() == 3) {
		for (auto t : AnimationQueue()) SetAnimatedTileState(t, AnimatedTileState::None);
		InitializeAnimatedTiles();
		for (auto t : AnimationFixtureTiles()) AddAnimatedTile(t, false);
		AnimationReloadObservation();
		SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	}
	std::fflush(stderr); return true;
}
#include "landscape-console.hpp"
#include "generation-console.hpp"
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("objects_scenario", ObjectsScenario);
	IConsole::CmdRegister("landscape_scenario", LandscapeScenario);
	IConsole::CmdRegister("generation_scenario", GenerationScenario);
}
