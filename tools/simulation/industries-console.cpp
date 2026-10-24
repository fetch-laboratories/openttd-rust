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
static bool IndustriesScenario(std::span<std::string_view>)
{
	Industry *i = nullptr;
	for (Industry *candidate : Industry::Iterate()) if (!candidate->accepted.empty() && IsValidCargoType(candidate->accepted[0].cargo)) { i = candidate; break; }
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
	SpriteGroup::CanAllocateItem();
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
