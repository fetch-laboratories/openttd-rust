/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file signs-console.cpp Production sign and depot-name command results and records. */
#include "stdafx.h"
#include "console_internal.h"
#include "command_func.h"
#include "company_func.h"
#include "company_base.h"
#include "core/backup_type.hpp"
#include "core/random_func.hpp"
#include "depot_base.h"
#include "depot_cmd.h"
#include "depot_map.h"
#include "openttd.h"
#include "saveload/saveload.h"
#include "signs_base.h"
#include "signs_cmd.h"
#include "town.h"
#include "viewport_kdtree.h"

static std::string Hex(std::string_view text)
{
	std::string out = text.empty() ? "-" : "";
	for (unsigned char c : text) out += fmt::format("{:02x}", c);
	return out;
}
static std::string Utf8(size_t chars)
{
	std::string text;
	for (size_t i = 0; i < chars; ++i) text += "\xC3\xA9";
	return text;
}
template <Commands C, typename... Args> static auto Observe(std::string_view name, DoCommandFlags flags, Args &&...args)
{
	auto result = Command<C>::Do(flags, std::forward<Args>(args)...);
	if constexpr (requires { std::get<0>(result); }) {
		fmt::print(stderr, "SIGNS result {} {} {} {}\n", name, std::get<0>(result).Succeeded(), std::get<0>(result).GetErrorMessage(), std::get<1>(result).base());
	} else {
		fmt::print(stderr, "SIGNS result {} {} {}\n", name, result.Succeeded(), result.GetErrorMessage());
	}
	return result;
}
static void State(bool interactive)
{
	for (const Sign *si : Sign::Iterate()) fmt::print(stderr, "SIGNS sign {} {} {} {} {} {}\n", si->index, si->owner, si->x, si->y, si->z, Hex(si->name));
	for (const Depot *d : Depot::Iterate()) fmt::print(stderr, "SIGNS depot {} {} {} {} {}\n", d->index, d->xy, d->town == nullptr ? -1 : static_cast<int>(d->town->index.base()), d->town_cn, Hex(d->name));
	fmt::print(stderr, "SIGNS rng {} {}\n", _random.state[0], _random.state[1]);
	/* Saves omit the interactive stream; each process seeds it afresh. */
	if (interactive) fmt::print(stderr, "SIGNS interactive-rng {} {}\n", _interactive_random.state[0], _interactive_random.state[1]);
	fmt::print(stderr, "SIGNS state-complete\n");
}
static void SignCommands(CompanyID company, CompanyID other)
{
	const auto run = DoCommandFlag::Execute;
	const TileIndex tile = TileXY(20, 30), moved = TileXY(Map::MaxX() - 1, 7);
	Observe<CMD_PLACE_SIGN>("place-test", {}, tile, std::string("Alpha"));
	const SignID alpha = std::get<1>(Observe<CMD_PLACE_SIGN>("place", run, tile, std::string("Alpha")));
	Observe<CMD_PLACE_SIGN>("place-utf8-31", run, moved, Utf8(31));
	Observe<CMD_PLACE_SIGN>("place-utf8-32", run, moved, Utf8(32));
	const SignID blank = std::get<1>(Observe<CMD_PLACE_SIGN>("place-empty", run, tile, std::string()));
	_current_company = OWNER_DEITY;
	const SignID deity = std::get<1>(Observe<CMD_PLACE_SIGN>("place-deity", run, moved, std::string("Deity")));
	_current_company = company;
	Observe<CMD_RENAME_SIGN>("rename-deity-refused", run, deity, std::string("Mine"));
	Observe<CMD_MOVE_SIGN>("move-deity-refused", run, deity, tile);
	Observe<CMD_RENAME_SIGN>("delete-deity-refused", run, deity, std::string());
	{
		Backup<GameMode> mode(_game_mode, GM_EDITOR);
		Observe<CMD_PLACE_SIGN>("place-editor", run, tile, std::string("Editor"));
		Observe<CMD_RENAME_SIGN>("rename-deity-editor", run, deity, std::string("Edited"));
		Observe<CMD_MOVE_SIGN>("move-deity-editor", run, deity, tile);
		Observe<CMD_MOVE_SIGN>("move-company-editor", run, alpha, moved);
		mode.Restore();
	}
	_current_company = other;
	Observe<CMD_RENAME_SIGN>("rename-other-test", {}, alpha, std::string("Beta"));
	Observe<CMD_RENAME_SIGN>("rename-other", run, alpha, std::string("Beta"));
	Observe<CMD_MOVE_SIGN>("move-other-test", {}, blank, moved);
	Observe<CMD_MOVE_SIGN>("move-other", run, blank, moved);
	_current_company = company;
	Observe<CMD_RENAME_SIGN>("rename-utf8-31", run, alpha, Utf8(31));
	Observe<CMD_RENAME_SIGN>("rename-utf8-32", run, alpha, Utf8(32));
	Observe<CMD_RENAME_SIGN>("rename-invalid", run, SignID::Invalid(), std::string("X"));
	Observe<CMD_MOVE_SIGN>("move-invalid", run, SignID(SignID::End().base() - 1), tile);
	Observe<CMD_RENAME_SIGN>("delete-test", {}, blank, std::string());
	Observe<CMD_RENAME_SIGN>("delete", run, blank, std::string());
	Observe<CMD_RENAME_SIGN>("delete-again", run, blank, std::string());
	Observe<CMD_PLACE_SIGN>("place-reuse", run, tile, std::string("Reuse"));
	std::vector<SignID> fill;
	while (Sign::CanAllocateItem()) fill.push_back(std::get<1>(Command<CMD_PLACE_SIGN>::Do(run, tile, std::string("F"))));
	fmt::print(stderr, "SIGNS filled {}\n", fill.size());
	Observe<CMD_PLACE_SIGN>("place-full-test", {}, tile, std::string("Full"));
	Observe<CMD_PLACE_SIGN>("place-full", run, tile, std::string(64, 'x'));
	/* Per-command removal is quadratic in the viewport kdtree; "delete" covers that path. */
	for (SignID id : fill) delete Sign::Get(id);
	RebuildViewportKdtree();
}
static void DepotCommands()
{
	const auto run = DoCommandFlag::Execute;
	std::map<VehicleType, std::vector<Depot *>> kinds;
	for (Depot *d : Depot::Iterate()) kinds[GetDepotVehicleType(d->xy)].push_back(d);
	fmt::print(stderr, "SIGNS depot-kinds {}\n", kinds.size());
	Depot *first = kinds.begin()->second.front();
	Observe<CMD_RENAME_DEPOT>("depot-invalid", run, DepotID::Invalid(), std::string("X"));
	for (auto &[type, depots] : kinds) {
		Depot *d = depots.front();
		_current_company = GetTileOwner(d->xy);
		const std::string label = fmt::format("depot-{}", to_underlying(type));
		Observe<CMD_RENAME_DEPOT>(label + "-test", {}, d->index, std::string("Shed"));
		Observe<CMD_RENAME_DEPOT>(label + (d == first ? "-name" : "-duplicate"), run, d->index, std::string("Shed"));
		Observe<CMD_RENAME_DEPOT>(label + "-utf8-31", {}, d->index, Utf8(31));
		Observe<CMD_RENAME_DEPOT>(label + "-utf8-32", {}, d->index, Utf8(32));
		_current_company = GetTileOwner(d->xy) == CompanyID::Begin() ? CompanyID(1) : CompanyID::Begin();
		Observe<CMD_RENAME_DEPOT>(label + "-foreign", run, d->index, std::string("Theirs"));
		_current_company = GetTileOwner(d->xy);
		/* Resetting the newest depot of a town takes its lowest free default number. */
		Depot *last = depots.back();
		Observe<CMD_RENAME_DEPOT>(label + "-reset", run, last->index, std::string());
		fmt::print(stderr, "SIGNS reset {} {} {}\n", label, last->index, last->town_cn);
	}
	_current_company = GetTileOwner(first->xy);
	Observe<CMD_RENAME_DEPOT>("depot-reset-named", run, first->index, std::string());
	Observe<CMD_RENAME_DEPOT>("depot-name-freed", run, first->index, std::string("Shed"));
}
static bool SignsScenario(std::span<std::string_view> args)
{
	if (args.size() == 2 && args[1] == "inspect") {
		State(false);
		return true;
	}
	if (args.size() != 3 || args[1] != "commands") return false;
	Backup<CompanyID> actor(_current_company);
	/* The save restores only _random; UI randomness is an explicit fixture input. */
	_interactive_random.SetSeed(0x586);
	/* Sign ownership is presentation only; any other actor ID exercises the transfer. */
	const CompanyID company = (*Company::Iterate().begin())->index, other = CompanyID(company.base() + 1);
	_current_company = company;
	SignCommands(company, other);
	DepotCommands();
	State(true);
	if (SaveOrLoad(args[2], SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false) != SL_OK) std::abort();
	fmt::print(stderr, "SIGNS commands-complete\n");
	actor.Restore();
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("signs_scenario", SignsScenario);
}
