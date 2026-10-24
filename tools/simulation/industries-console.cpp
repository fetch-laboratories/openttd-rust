/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file industries-console.cpp Real Action2 programs on both games' industry specs. */
#include "stdafx.h"
#include "console_internal.h"
#include "industry.h"
#include "newgrf_industrytiles.h"
#include "newgrf_cargo.h"
#include "town.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "safeguards.h"

static DeterministicSpriteGroupAdjust Read(uint8_t variable, uint8_t parameter = 0, uint32_t mask = UINT32_MAX, DeterministicSpriteGroupAdjustOperation operation = DSGA_OP_RST)
{
	return {.operation = operation, .type = DSGA_TYPE_NONE, .variable = variable, .parameter = parameter, .shift_num = 0, .and_mask = mask};
}
static void Store(DeterministicSpriteGroup *group, uint16_t reg, uint32_t value)
{
	group->adjusts.push_back(Read(0x1A, 0, value));
	group->adjusts.push_back(Read(0x1A, 0, reg, DSGA_OP_STO));
}
static DeterministicSpriteGroup *Query(uint8_t variable, uint8_t parameter, const SpriteGroup *result, uint32_t filter = 0)
{
	auto *group = new DeterministicSpriteGroup();
	group->var_scope = VSG_SCOPE_SELF;
	group->size = DSG_SIZE_DWORD;
	Store(group, 0x100, 0);
	Store(group, 0x101, filter);
	group->adjusts.push_back(Read(variable, parameter));
	group->adjusts.push_back(Read(0x1A, 0, 0x102, DSGA_OP_STO));
	group->default_result.group = result;
	return group;
}
/** Query the public virtuals, including unavailable and empty-slot results. */
static bool VariableCases(Industry *i, Industry *temporary, TileIndex tile, IndustryGfx gfx, GRFFile &file)
{
	IndustriesResolverObject live(tile, i, i->type), missing(INVALID_TILE, nullptr, i->type), invalid(INVALID_TILE, i, i->type), prototype(tile, temporary, i->type);
	IndustryTileResolverObject tile_live(gfx, tile, i), tile_temporary(gfx, tile, temporary);
	IndustryTileScopeResolver tile_null(live, nullptr, tile);
	std::vector<uint8_t> variables{0x7A, 0x7C, 0xB0, 0xB3, 0xB4, 0xFF};
	for (uint8_t v = 0x40; v <= 0x47; ++v) variables.push_back(v);
	for (uint8_t v = 0x60; v <= 0x71; ++v) variables.push_back(v);
	for (uint8_t v = 0x80; v <= 0xAC; ++v) variables.push_back(v);
	uint count = 0;
	for (auto *object : {&live, &missing, &invalid, &prototype}) for (bool incoming : {false, true}) for (uint8_t v : variables) {
		object->SetRegister(0x100, 0); object->SetRegister(0x101, 0x100);
		bool available = incoming;
		uint32_t value = object->industries_scope.GetVariable(v, 0, available);
		fmt::print(stderr, "INDUSTRY-RESOLVER variable {} {} {} {} {}\n", count++, v, incoming, available, value);
	}
	for (auto *scope : {&tile_live.indtile_scope, &tile_temporary.indtile_scope, &tile_null}) for (bool incoming : {false, true}) for (uint8_t v : {0x40, 0x41, 0x42, 0x43, 0x44, 0x60, 0x61, 0x62, 0x7A, 0xFF}) {
		if (scope == &tile_null && (v == 0x43 || v == 0x62)) continue; // The source dereferences the industry for these.
		bool available = incoming;
		uint32_t value = scope->GetVariable(v, 0, available);
		fmt::print(stderr, "INDUSTRY-RESOLVER tile-variable {} {} {} {} {}\n", count++, v, incoming, available, value);
	}
	for (uint8_t version : {6, 7, 8}) {
		file.grf_version = version;
		IndustriesResolverObject location(tile, temporary, i->type, 0xFEDCBA98, CBID_INDUSTRY_LOCATION);
		for (uint8_t v : {0x80, 0x81, 0x82, 0x86, 0x87, 0x88, 0x89, 0x8A, 0x8B, 0x8D, 0x8F}) {
			bool available = false;
			fmt::print(stderr, "INDUSTRY-RESOLVER location-variable {} {} {}\n", version, v, location.industries_scope.GetVariable(v, 0, available));
		}
		for (uint32_t parameter : {0U, 1U, 0xFFU, 0x101U}) for (uint8_t v : {0x60, 0x61, 0x62, 0x63, 0x65, 0x66}) {
			bool available = true;
			fmt::print(stderr, "INDUSTRY-RESOLVER nearby {} {} {} {}\n", version, v, parameter, live.industries_scope.GetVariable(v, parameter, available));
		}
	}
	file.grf_version = 8;
	{
		Backup<Owner> founder(i->founder);
		for (Owner owner : {Owner(0), OWNER_NONE, INVALID_OWNER}) {
			i->founder = owner; bool available = true;
			fmt::print(stderr, "INDUSTRY-RESOLVER founder {} {}\n", owner.base(), live.industries_scope.GetVariable(0x45, 0, available));
		}
	}
	GRFFile caller; caller.grfid = file.grfid + 1; caller.grf_version = 8; caller.cargo_list = file.cargo_list;
	std::ranges::reverse(caller.cargo_list); live.grffile = &caller;
	for (uint32_t parameter : {0U, 1U, 31U, 63U, 64U, 255U, 256U}) for (uint8_t v : {0x69,0x6A,0x6B,0x6C,0x6D,0x6E,0x6F,0x70,0x71}) {
		bool available = true;
		fmt::print(stderr, "INDUSTRY-RESOLVER caller-cargo {} {} {}\n", v, parameter, live.industries_scope.GetVariable(v, parameter, available));
	}
	Backup<std::vector<EntityIDMapping>> mapping(_industry_mngr.mappings);
	_industry_mngr.mappings[i->type] = {file.grfid, 17, i->type}; live.grffile = &file;
	for (uint32_t id : {0U, UINT32_MAX, file.grfid, file.grfid + 1}) for (uint32_t filter : {0U, 1U, 0x100U, 0x101U}) {
		live.SetRegister(0x100, id); live.SetRegister(0x101, filter); bool available = true;
		fmt::print(stderr, "INDUSTRY-RESOLVER mapping {} {} {}\n", id, filter, live.industries_scope.GetVariable(0x68, id == 0 ? i->type : 17, available));
	}
	fmt::print(stderr, "INDUSTRY-RESOLVER variable-cases {}\n", count);
	return count > 600;
}
static bool CountWrapCases(Industry *i)
{
	std::vector<Industry *> sisters;
	Industry::CanAllocateItem(260);
	for (uint n = 0; n < 260; ++n) {
		auto *other = new Industry(); other->location.tile = i->location.tile + 1; other->location.w = 0;
		other->type = i->type; other->town = i->town; other->selected_layout = 127;
		Industry::industries[i->type].insert(other->index); sisters.push_back(other);
	}
	IndustriesResolverObject object(i->location.tile, i, i->type);
	object.SetRegister(0x100, 0); object.SetRegister(0x101, 127); bool available = true;
	uint32_t unfiltered = object.industries_scope.GetVariable(0x67, i->type, available);
	uint32_t filtered = object.industries_scope.GetVariable(0x68, i->type, available);
	bool equal = (unfiltered >> 16) == 255 && (unfiltered & 0xFFFF) == 1 && (filtered >> 16) == 4 && (filtered & 0xFFFF) == 1;
	for (auto *other : sisters) { Industry::industries[i->type].erase(other->index); delete other; }
	fmt::print(stderr, "INDUSTRY-RESOLVER count-wrap {}\n", equal);
	return equal;
}
static bool TileIdentityCases(Industry *i, Industry *temporary, TileIndex tile, GRFFile &file, const SpriteGroup *group)
{
	IndustryGfx original = GetCleanIndustryGfx(tile);
	constexpr IndustryGfx custom = NEW_INDUSTRYTILEOFFSET + 1, replacement = NEW_INDUSTRYTILEOFFSET + 2;
	auto *old_spec = const_cast<IndustryTileSpec *>(GetIndustryTileSpec(original));
	auto *new_spec = const_cast<IndustryTileSpec *>(GetIndustryTileSpec(custom));
	auto *override_spec = const_cast<IndustryTileSpec *>(GetIndustryTileSpec(replacement));
	Backup<IndustryTileSpec> old_backup(*old_spec), new_backup(*new_spec), override_backup(*override_spec);
	bool equal = true;
	for (IndustryGfx gfx : {original, custom}) for (bool groups : {false, true}) for (uint32_t grfid : {file.grfid, file.grfid + 1}) {
		SetIndustryGfx(tile, gfx);
		auto *s = gfx == original ? old_spec : new_spec;
		s->grf_prop.grfid = grfid; s->grf_prop.local_id = 19; s->grf_prop.subst_id = 17;
		s->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, groups ? group : nullptr);
		old_spec->grf_prop.override_id = groups ? replacement : INVALID_INDUSTRYTILE;
		override_spec->grf_prop.grfid = grfid; override_spec->grf_prop.local_id = 21;
		uint32_t value = GetIndustryIDAtOffset(tile, i, file.grfid);
		uint32_t expected = gfx == original && !groups ? 0xFF00U | original : !groups ? 0xFF11U : grfid != file.grfid ? 0xFFFEU : gfx == original ? 21U : 19U;
		equal &= value == expected;
		fmt::print(stderr, "INDUSTRY-RESOLVER tile-id {} {} {} {}\n", gfx, groups, grfid, value);
	}
	SetIndustryGfx(tile, original);
	Backup<TileArea> location(temporary->location);
	temporary->location.tile = tile + 1;
	IndustryTileResolverObject negative(original, tile, temporary); bool available = true;
	fmt::print(stderr, "INDUSTRY-RESOLVER relative-negative {}\n", negative.indtile_scope.GetVariable(0x43, 0, available));
	equal &= GetIndustryIDAtOffset(tile + TileDiffXY(20, 20), i, file.grfid) == 0xFFFF;
	fmt::print(stderr, "INDUSTRY-RESOLVER tile-identities {}\n", equal);
	return equal;
}

static bool CallbackCases(Industry *i, IndustrySpec *spec, GRFFile &file)
{
	bool equal = true;
	Backup<IndustryCallbackMasks> mask(spec->callback_mask);
	for (uint8_t version : {7, 8}) for (uint16_t result : {0, 1, 255, 256, 65535}) {
		file.grf_version = version;
		SpriteGroup::CanAllocateItem(); auto *answer = new CallbackResultSpriteGroup(result);
		spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, answer);
		spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, answer);
		spec->callback_mask.Set(IndustryCallbackMask::Probability);
		uint32_t probability = GetIndustryProbabilityCallback(i->type, IACT_USERCREATION, 73);
		uint32_t expected = result == CALLBACK_FAILED || (version >= 8 && result == 256) ? 73 : version < 8 ? (result == 0 ? 73 : 0) : result;
		equal &= probability == expected;
		if (result <= 1 || result == CALLBACK_FAILED) {
			spec->callback_mask.Set(IndustryCallbackMask::RefuseCargo);
			bool refused = IndustryTemporarilyRefusesCargo(i, i->accepted[0].cargo);
			equal &= refused == (result == 0);
			fmt::print(stderr, "INDUSTRY-RESOLVER refusal-result {} {} {}\n", version, result, refused);
		}
		fmt::print(stderr, "INDUSTRY-RESOLVER probability {} {} {}\n", version, result, probability);
		delete answer;
	}
	file.grf_version = 8;
	if (i->produced.empty() || !IsValidCargoType(i->produced[0].cargo)) return false;
	Backup<uint16_t> input(i->accepted[0].waiting), output(i->produced[0].waiting);
	Backup<uint8_t> level(i->prod_level);
	i->accepted[0].waiting = 48; i->produced[0].waiting = 7; i->prod_level = 16;
	spec->callback_mask.Set(IndustryCallbackMask::ProductionCargoArrival);
	SpriteGroup::CanAllocateItem(); auto *production = new IndustryProductionSpriteGroup();
	production->version = 1; production->num_input = production->num_output = 1; production->subtract_input[0] = 0x102; production->add_output[0] = 0x103; production->again = 4;
	SpriteGroup::CanAllocateItem(); auto *program = new DeterministicSpriteGroup(); program->size = DSG_SIZE_DWORD;
	Store(program, 0x103, 5); Store(program, 4, 0);
	program->adjusts.push_back(Read(0x10)); program->adjusts.push_back(Read(0x1A, 0, 0x105, DSGA_OP_STO));
	program->adjusts.push_back(Read(0x40)); program->adjusts.push_back(Read(0x1A, 0, 0x102, DSGA_OP_STO)); program->default_result.group = production;
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, program);
	Backup<IndustryBehaviours> behaviour(spec->behaviour); spec->behaviour.Reset(IndustryBehaviour::ProdMultiHandling); spec->behaviour.Set(IndustryBehaviour::ProdCallbackRandom);
	Randomizer expected_rng = _random; uint32_t expected_random = expected_rng.Next();
	IndustryProductionCallback(i, 0);
	ResolverObject registers(&file);
	bool parameters = static_cast<uint32_t>(registers.GetRegister(0x105)) == expected_random && std::ranges::equal(_random.state, expected_rng.state);
	equal &= parameters;
	fmt::print(stderr, "INDUSTRY-RESOLVER production-params {}\n", parameters);
	equal &= i->accepted[0].waiting == 0 && i->produced[0].waiting == 12;
	fmt::print(stderr, "INDUSTRY-RESOLVER production {} {}\n", i->accepted[0].waiting, i->produced[0].waiting);
	delete program; delete production;
	fmt::print(stderr, "INDUSTRY-RESOLVER callbacks {}\n", equal);
	return equal;
}
static bool RandomCases(Industry *i, IndustryTileSpec *spec, IndustryGfx gfx, TileIndex tile)
{
	SpriteGroup::CanAllocateItem(2); auto *zero = new CallbackResultSpriteGroup(0); auto *one = new CallbackResultSpriteGroup(1);
	SpriteGroup::CanAllocateItem(2); auto *parent = new RandomizedSpriteGroup(); auto *self = new RandomizedSpriteGroup();
	parent->var_scope = VSG_SCOPE_PARENT; parent->triggers = 1; parent->lowest_randbit = 9; parent->groups = {zero, one};
	self->var_scope = VSG_SCOPE_SELF; self->triggers = 1; self->lowest_randbit = 2; self->groups = {parent, parent};
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, self);
	bool equal = true;
	for (uint16_t bits : {0, 0x200, 0xFFFF}) {
		i->random = bits;
		IndustryTileResolverObject object(gfx, tile, i, CBID_RANDOM_TRIGGER); object.SetWaitingRandomTriggers(IndustryRandomTriggers(1));
		object.ResolveRerandomisation();
		equal &= object.reseed[VSG_SCOPE_SELF] == 4 && object.reseed[VSG_SCOPE_PARENT] == 0x200 && object.GetUsedRandomTriggers().base() == 1;
		std::array<int32_t, 1> regs{}; object.callback = CBID_INDTILE_SHAPE_CHECK;
		equal &= object.ResolveCallback(regs) == ((bits & 0x200) != 0 ? 1 : 0);
		fmt::print(stderr, "INDUSTRY-RESOLVER random-query {} {} {}\n", bits, object.GetReseedSum(), object.indtile_scope.GetRandomBits());
	}
	for (uint8_t bits : {0, 4, 255}) {
		SetIndustryRandomBits(tile, bits); SetIndustryRandomTriggers(tile, IndustryRandomTriggers(2));
		TriggerIndustryTileRandomisation(tile, IndustryRandomTrigger::TileLoop);
		fmt::print(stderr, "INDUSTRY-RESOLVER reseed {} {} {} {} {}\n", GetIndustryRandomBits(tile), GetIndustryRandomTriggers(tile).base(), i->random, _random.state[0], _random.state[1]);
	}
	delete self; delete parent; delete one; delete zero;
	fmt::print(stderr, "INDUSTRY-RESOLVER randomized {}\n", equal);
	return equal;
}

static bool IndustriesScenario(std::span<std::string_view>)
{
	Industry *i = nullptr;
	for (Industry *candidate : Industry::Iterate()) if (!candidate->accepted.empty() && IsValidCargoType(candidate->accepted[0].cargo) && !candidate->produced.empty() && IsValidCargoType(candidate->produced[0].cargo)) { i = candidate; break; }
	if (i == nullptr) std::abort();
	TileIndex tile = INVALID_TILE;
	for (TileIndex t : i->location) if (i->TileBelongsToIndustry(t)) { tile = t; break; }
	if (tile == INVALID_TILE) std::abort();
	const IndustryGfx gfx = GetIndustryGfx(tile);
	auto *spec = const_cast<IndustrySpec *>(GetIndustrySpec(i->type));
	auto *tile_spec = const_cast<IndustryTileSpec *>(GetIndustryTileSpec(gfx));
	Backup<IndustrySpec> spec_backup(*spec);
	Backup<IndustryTileSpec> tile_backup(*tile_spec);
	GRFFile file; file.grf_version = 8; file.grfid = 0x12345678;
	for (CargoType cargo = 0; cargo < NUM_CARGO; cargo++) { file.cargo_map[cargo] = cargo; file.cargo_list.push_back(CargoSpec::Get(cargo)->label); }
	spec->grf_prop.grffile = &file; spec->grf_prop.grfid = file.grfid; spec->grf_prop.local_id = 17;
	tile_spec->grf_prop.grffile = &file; tile_spec->grf_prop.grfid = file.grfid; tile_spec->grf_prop.local_id = 19;
	SpriteGroup::CanAllocateItem(6);
	auto *zero = new CallbackResultSpriteGroup(0);
	auto *one = new CallbackResultSpriteGroup(1);
	auto *count = Query(0x67, i->type, zero);
	auto *filtered = Query(0x68, i->type, zero, 0x100);
	auto *location = Query(0x8F, 0, zero);
	auto *relative = Query(0x43, 0, one);
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, count);
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, location);
	tile_spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, relative);
	tile_spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Purchase, one);
	bool equal = true;
	std::array<int32_t, 4> regs{};
	const auto rng = _random;
	auto result = GetIndustryCallback(CBID_INDUSTRY_REFUSE_CARGO, 0, 0, i, i->type, tile, regs);
	equal &= result == 0 && (static_cast<uint32_t>(regs[2]) >> 16) == Industry::GetIndustryTypeCount(i->type);
	fmt::print(stderr, "INDUSTRY-RESOLVER var67 {} {}\n", result, regs[2]);
	spec->grf_prop.SetSpriteGroup(StandardSpriteGroup::Default, filtered);
	result = GetIndustryCallback(CBID_INDUSTRY_REFUSE_CARGO, 0, 0, i, i->type, tile, regs);
	fmt::print(stderr, "INDUSTRY-RESOLVER var68 {} {}\n", result, regs[2]);
	spec->callback_mask.Set(IndustryCallbackMask::RefuseCargo);
	equal &= IndustryTemporarilyRefusesCargo(i, i->accepted[0].cargo);
	fmt::print(stderr, "INDUSTRY-RESOLVER refusal true\n");
	Industry temporary;
	temporary.index = IndustryID::Invalid(); temporary.location.tile = tile; temporary.location.w = 0; temporary.type = i->type;
	temporary.town = i->town; temporary.selected_layout = 23; temporary.random = 0xA55A; temporary.founder = OWNER_NONE;
	IndustriesResolverObject construction(tile, &temporary, i->type, 0x87654321, CBID_INDUSTRY_LOCATION);
	result = construction.ResolveCallback(regs);
	equal &= result == 0 && static_cast<uint32_t>(regs[2]) == 0x87654321;
	fmt::print(stderr, "INDUSTRY-RESOLVER location {} {}\n", result, static_cast<uint32_t>(regs[2]));
	IndustriesResolverObject live(tile, i, i->type), missing(INVALID_TILE, nullptr, i->type), nearby(tile, nullptr, i->type);
	IndustryTileResolverObject live_tile(gfx, tile, i), temporary_tile(gfx, tile, &temporary);
	equal &= live.root_spritegroup == filtered && missing.root_spritegroup == location;
	equal &= live_tile.root_spritegroup == relative && temporary_tile.root_spritegroup == one;
	equal &= live.GetTown() == live.GetTown() && !live.GetTown()->readonly && construction.GetTown()->readonly && nearby.GetTown()->readonly && missing.GetTown() == nullptr;
	equal &= live.GetScope(VSG_SCOPE_SELF) == &live.industries_scope && live.GetScope(VSG_SCOPE_PARENT) == live.GetTown();
	equal &= live_tile.GetScope(VSG_SCOPE_SELF) == &live_tile.indtile_scope && live_tile.GetScope(VSG_SCOPE_PARENT) == &live_tile.ind_scope;
	result = GetIndustryTileCallback(CBID_INDTILE_SHAPE_CHECK, 0, 0, gfx, i, tile, regs);
	equal &= result == 1 && static_cast<uint32_t>(regs[2]) == (((TileY(tile) - TileY(i->location.tile)) & 15) << 20 | ((TileX(tile) - TileX(i->location.tile)) & 15) << 16 | static_cast<uint8_t>(TileY(tile) - TileY(i->location.tile)) << 8 | static_cast<uint8_t>(TileX(tile) - TileX(i->location.tile)));
	equal &= temporary_tile.indtile_scope.GetRandomBits() == 0 && temporary_tile.indtile_scope.GetRandomTriggers() == 0;
	equal &= std::ranges::equal(_random.state, rng.state);
	fmt::print(stderr, "INDUSTRY-RESOLVER checkpoint {}\n", equal);
	equal &= VariableCases(i, &temporary, tile, gfx, file);
	equal &= CountWrapCases(i);
	equal &= TileIdentityCases(i, &temporary, tile, file, zero);
	equal &= CallbackCases(i, spec, file);
	equal &= RandomCases(i, tile_spec, gfx, tile);
	fmt::print(stderr, "INDUSTRY-RESOLVER complete {}\n", equal);
	delete relative; delete location; delete filtered; delete count; delete one; delete zero;
	std::fflush(stderr);
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("industries_scenario", IndustriesScenario);
}
