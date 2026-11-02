/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file commands-console.cpp Production command entries with native input and unwind observations. */
#include "network/network_command.cpp" // Inspect the real server queue; relink replaces this object.
#include "ai/ai.hpp"
#include "clear_map.h"
#include "company_base.h"
#include "console_internal.h"
#include "economy_func.h"
#include "genworld.h"
#include "gfx_func.h"
#include "newgrf_storage.h"
#include "object_base.h"
#include "object_map.h"
#include "saveload/saveload.h"

static bool command_throw = false;
static uint command_diagnostics = 0;
static CompanyID command_allocated = CompanyID::Invalid();
struct CommandStartupFailure {};
extern "C" void __real__ZN2AI8StartNewE6PoolIDIh12CompanyIDTagLh15ELh255EE(CompanyID);
extern "C" void __wrap__ZN2AI8StartNewE6PoolIDIh12CompanyIDTagLh15ELh255EE(CompanyID id)
{
	if (command_throw) { command_allocated = id; throw CommandStartupFailure{}; }
	__real__ZN2AI8StartNewE6PoolIDIh12CompanyIDTagLh15ELh255EE(id);
}
extern "C" void __real__Z10DebugPrintSt17basic_string_viewIcSt11char_traitsIcEEiONSt7__cxx1112basic_stringIcS1_SaIcEEE(std::string_view, int, std::string &&);
extern "C" void __wrap__Z10DebugPrintSt17basic_string_viewIcSt11char_traitsIcEEiONSt7__cxx1112basic_stringIcS1_SaIcEEE(std::string_view category, int level, std::string &&message)
{
	/* Source locations differ between the native and Rust-backed templates. */
	if (command_throw && category == "misc" && level == 0 && message.ends_with(": Backed-up value was not restored!")) {
		command_diagnostics++;
		message = "command scope: Backed-up value was not restored!";
	}
	__real__Z10DebugPrintSt17basic_string_viewIcSt11char_traitsIcEEiONSt7__cxx1112basic_stringIcS1_SaIcEEE(category, level, std::move(message));
}
static void CommandNeed(bool value, int line)
{
	if (!value) { fmt::print(stderr, "INVOCATION failed {}\n", line); std::abort(); }
}
static uint command_callbacks = 0;
static void CommandObserve(Commands cmd, const CommandCost &cost, TileIndex tile)
{
	if (cmd == CMD_BUILD_OBJECT && cost.Succeeded()) {
		CommandNeed(IsObjectTypeTile(tile, OBJECT_OWNED_LAND) && Company::Get(_current_company)->last_build_coordinate == tile, __LINE__);
	}
	command_callbacks++;
	fmt::print(stderr, "INVOCATION callback {} {} {} {}\n", cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage(), _current_company.base());
}
/* Expose the production entry with a fixture callback absent from the network callback registry. */
template <Commands Cmd> struct ObservedCommand : Command<Cmd> { using Command<Cmd>::InternalPost; };
static bool CommandsScenario([[maybe_unused]] std::span<std::string_view> args)
{
	if (args.size() != 2 && args.size() != 3) return false;
	const CompanyID actor{0};
	AutoRestoreBackup<CompanyID> current(_current_company, actor), local(_local_company, actor);
	AutoRestoreBackup<bool> shift(_shift_pressed, false), network(_networking, false), server(_network_server, true);
	AutoRestoreBackup<PauseModes> pause(_pause_mode, PauseModes{});
	AutoRestoreBackup<CommandPauseLevel> level(_settings_game.construction.command_pause_level, CommandPauseLevel::AllActions);
	AutoRestoreBackup<Randomizer> interactive(_interactive_random, _interactive_random);
	_interactive_random.SetSeed(559);
	Company *company = Company::Get(actor);
#ifdef WITH_RUST
	auto &finance = company->Finances();
#else
	auto &finance = *company;
#endif
	finance.money = 1000000000;
	finance.current_loan = _economy.max_loan;
	TileIndex tile = INVALID_TILE;
	for (TileIndex t : Map::Iterate()) {
		if (TileX(t) < 2 || TileY(t) < 2 || TileX(t) >= Map::MaxX() - 1 || TileY(t) >= Map::MaxY() - 1) continue;
		if (IsTileType(t, MP_CLEAR) && GetTileSlope(t) == SLOPE_FLAT && GetTileZ(t) > 0) { tile = t; break; }
	}
	CommandNeed(tile != INVALID_TILE, __LINE__);
	auto post = [&] { return ObservedCommand<CMD_BUILD_OBJECT>::InternalPost(0, &CommandObserve, true, false, std::make_tuple(tile, OBJECT_OWNED_LAND, uint8_t{0})); };
	const Money money = finance.money;
	command_callbacks = 0;
	_pause_mode = {PauseMode::Normal};
	_settings_game.construction.command_pause_level = CommandPauseLevel::NoActions;
	CommandNeed(!post() && command_callbacks == 0 && IsTileType(tile, MP_CLEAR), __LINE__);
	fmt::print(stderr, "INVOCATION paused-rejected {}\n", _pause_mode.base());
	_shift_pressed = true;
	CommandNeed(post() && command_callbacks == 0 && IsTileType(tile, MP_CLEAR) && finance.money == money, __LINE__);
	CommandNeed(!_pause_mode.Test(PauseMode::CommandDuringPause), __LINE__);
	fmt::print(stderr, "INVOCATION shift-estimate {}\n", _pause_mode.base());
	_shift_pressed = false;
	_settings_game.construction.command_pause_level = CommandPauseLevel::AllActions;
	CommandNeed(post() && command_callbacks == 1 && IsObjectTypeTile(tile, OBJECT_OWNED_LAND), __LINE__);
	CommandNeed(_pause_mode.Test(PauseMode::CommandDuringPause), __LINE__);
	fmt::print(stderr, "INVOCATION pause-marked {}\n", _pause_mode.base());
	_pause_mode = {};
	CommandNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile).Succeeded(), __LINE__);
	MakeClear(tile, CLEAR_ROUGH, 3);
	/* NoTest must execute even when its test cost exceeds available cash. */
	finance.money = 0;
	auto cash = Command<CMD_CLEAR_AREA>::Unsafe<CommandCallback>(0, nullptr, true, false, tile, std::make_tuple(tile, tile, false));
	CommandNeed(std::get<0>(cash).Failed() && std::get<1>(cash) > 0 && IsTileType(tile, MP_CLEAR) && GetClearGround(tile) == CLEAR_ROUGH, __LINE__);
	fmt::print(stderr, "INVOCATION extra-cash {} {} {}\n", std::get<0>(cash).GetErrorMessage(), std::get<0>(cash).GetCost(), std::get<1>(cash));
	finance.money = money;
	CommandNeed(Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, tile).Succeeded(), __LINE__);
	_networking = true;
	CommandNeed(_local_wait_queue.empty(), __LINE__);
	const Money before_send = finance.money;
	CommandNeed(post() && command_callbacks == 1 && IsTileType(tile, MP_CLEAR) && finance.money == before_send, __LINE__);
	CommandNeed(_local_wait_queue.size() == 1, __LINE__);
	const auto packet = _local_wait_queue.front();
	CommandNeed(packet.cmd == CMD_BUILD_OBJECT && packet.company == actor && packet.callback == reinterpret_cast<CommandCallback *>(&CommandObserve), __LINE__);
	CommandNeed(packet.data == EndianBufferWriter<CommandDataBuffer>::FromValue(std::make_tuple(tile, OBJECT_OWNED_LAND, uint8_t{0})), __LINE__);
	fmt::print(stderr, "INVOCATION network-send {} {} {}\n", packet.cmd, packet.company.base(), packet.frame == _frame_counter_max + 1);
	_local_wait_queue.clear();
	CommandNeed(Command<CMD_BUILD_OBJECT>::PostFromNet(0, &CommandObserve, true, std::make_tuple(tile, OBJECT_OWNED_LAND, uint8_t{0})), __LINE__);
	CommandNeed(command_callbacks == 2 && IsObjectTypeTile(tile, OBJECT_OWNED_LAND) && _local_wait_queue.empty(), __LINE__);
	_networking = false;
	/* Throw at the native AI startup boundary after the real company allocation. */
	const auto count = Company::GetNumItems();
	command_throw = true;
	bool caught = false;
	try {
		ObservedCommand<CMD_COMPANY_CTRL>::InternalPost(0, &CommandObserve, true, false, std::make_tuple(CCA_NEW_AI, CompanyID::Invalid(), CompanyRemoveReason{}, ClientID{1}));
	} catch (const CommandStartupFailure &) { caught = true; }
	command_throw = false;
	CommandNeed(caught && command_diagnostics == 1 && command_callbacks == 2 && _current_company == actor && Company::GetNumItems() == count + 1, __LINE__);
	CommandNeed(Company::IsValidID(command_allocated) && Company::Get(command_allocated)->is_ai, __LINE__);
	{ RecursiveCommandCounter counter; CommandNeed(counter.IsTopLevel(), __LINE__); }
	/* The original leaves command storage mode active on this exception. */
	BasePersistentStorageArray::SwitchMode(PSM_LEAVE_COMMAND);
	fmt::print(stderr, "INVOCATION unwind {} {} {}\n", caught, _current_company.base(), command_allocated.base());
	fmt::print(stderr, "INVOCATION rng {} {} {} {}\n", _random.state[0], _random.state[1], _interactive_random.state[0], _interactive_random.state[1]);
	if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	std::fflush(stderr);
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("command_scenario", CommandsScenario);
}
