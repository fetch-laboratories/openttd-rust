/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engines-console.cpp Identical typed lifecycle inputs for both game bodies. */
#include "stdafx.h"
#include "console_internal.h"
#include "command_func.h"
#include "company_base.h"
#include "company_func.h"
#include "core/backup_type.hpp"
#include "engine_base.h"
#include "engine_cmd.h"
#include "engine_func.h"
#include "vehicle_base.h"
#include "settings_type.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_manager.h"
#include "saveload/saveload.h"
#include "console_func.h"
/* Compile the unchanged native loader here to reach its private chunk reader. */
#include "saveload/saveload.cpp"

void CalendarEnginesMonthlyLoop();
void ClearEnginesHiddenFlagOfCompany(CompanyID);
[[noreturn]] static void FixtureAbort(int line) { fmt::print(stderr, "ENGINE setup-failure {}\n", line); std::fflush(stderr); std::abort(); }
static void Cost(const char *label, const CommandCost &cost)
{
	fmt::print(stderr, "ENGINE cost {} {} {} {}\n", label, cost.Succeeded(), cost.GetCost(), cost.GetErrorMessage());
}
static uint8_t &BlockPreview(Company *c)
{
#ifdef WITH_RUST
	return c->Finances().block_preview;
#else
	return c->block_preview;
#endif
}
static std::string_view Name(const Engine *e)
{
#ifdef WITH_RUST
	return e->name.View();
#else
	return e->name;
#endif
}
static Engine *UnusedEngine(EngineID except = EngineID::Invalid())
{
	for (Engine *e : Engine::IterateType(VEH_TRAIN)) {
		if (e->index == except || e->VehInfo<RailVehicleInfo>().railveh_type == RAILVEH_WAGON || !e->IsEnabled()) continue;
		bool used = false;
		for (Vehicle *v : Vehicle::Iterate()) if (v->engine_type == e->index) used = true;
		if (!used) return e;
	}
	FixtureAbort(__LINE__);
}
#include "subsidies-console.inc"
#include "cargomonitor-console.inc"
static bool EnginesScenario(std::span<std::string_view> args)
{
	if (args.size() < 2) return false;
	if (args[1].starts_with("monitors")) return MonitorScenario(args);
	if (args[1].starts_with("subsidies")) return SubsidyScenario(args);
	Company *c = nullptr;
	for (Vehicle *v : Vehicle::Iterate()) if (v->type == VEH_TRAIN && v->GetEngine()->CanCarryCargo()) { c = Company::Get(v->owner); break; }
	if (c == nullptr) FixtureAbort(__LINE__);
	Backup<CompanyID> company(_current_company, c->index);
	Engine *e = UnusedEngine();
	Engine *variant = UnusedEngine(e->index);
	if (args[1] == "inspect") {
		bool found = false;
		for (Engine *loaded : Engine::Iterate()) if (Name(loaded) == "Migration engine") found = loaded->company_hidden.Test(c->index) && loaded->company_avail.Test(c->index);
		fmt::print(stderr, "ENGINE reloaded {}\n", found);
	} else if (args[1] == "commands") {
		e->company_avail.Set(c->index);
		e->flags.Set(EngineFlag::Available);
		bool special_bits = true;
		for (uint8_t id : {16, 17, 32, 33, 48, 49, 63}) special_bits &= !IsEngineBuildable(e->index, e->type, CompanyID{id});
		fmt::print(stderr, "ENGINE company-bits {}\n", special_bits);
		e->company_hidden.Set(c->index);
		ClearEnginesHiddenFlagOfCompany(CompanyID{16});
		fmt::print(stderr, "ENGINE clear-special {}\n", e->company_hidden.Test(c->index));
		e->company_hidden.Reset(c->index);
		Cost("rename-test", Command<CMD_RENAME_ENGINE>::Do({}, e->index, "Migration engine"));
		Cost("rename", Command<CMD_RENAME_ENGINE>::Do(DoCommandFlag::Execute, e->index, "Migration engine"));
		Cost("duplicate", Command<CMD_RENAME_ENGINE>::Do(DoCommandFlag::Execute, variant->index, "Migration engine"));
		Cost("hide-test", Command<CMD_SET_VEHICLE_VISIBILITY>::Do({}, e->index, true));
		Cost("hide", Command<CMD_SET_VEHICLE_VISIBILITY>::Do(DoCommandFlag::Execute, e->index, true));
		fmt::print(stderr, "ENGINE hidden {}\n", e->company_hidden.Test(c->index));
		Cost("show", Command<CMD_SET_VEHICLE_VISIBILITY>::Do(DoCommandFlag::Execute, e->index, false));
		company.Change(OWNER_DEITY);
		Cost("disable-test", Command<CMD_ENGINE_CTRL>::Do({}, e->index, c->index, false));
		Cost("disable", Command<CMD_ENGINE_CTRL>::Do(DoCommandFlag::Execute, e->index, c->index, false));
		fmt::print(stderr, "ENGINE disabled {}\n", !e->company_avail.Test(c->index));
		Cost("enable", Command<CMD_ENGINE_CTRL>::Do(DoCommandFlag::Execute, e->index, c->index, true));
		company.Change(c->index);
		Cost("reload-hide", Command<CMD_SET_VEHICLE_VISIBILITY>::Do(DoCommandFlag::Execute, e->index, true));
		if (args.size() == 3) SaveOrLoad(std::string(args[2]), SLO_SAVE, DFT_GAME_FILE, NO_DIRECTORY, false);
	} else if (args[1] == "preview") {
		/* Input specs attach a joined variant; stock saves contain no NewGRF. */
		variant->info.variant_id = e->index;
		variant->info.extra_flags.Set(ExtraEngineFlag::JoinPreview);
		for (Engine *other : Engine::Iterate()) other->flags.Reset(EngineFlag::ExclusivePreview);
		for (Company *other : Company::Iterate()) BlockPreview(other) = 0;
		e->flags = EngineFlag::ExclusivePreview;
		e->company_avail = {};
		e->preview_asked = {};
		e->preview_company = CompanyID::Invalid();
		variant->company_avail = {};
		TimerGameCalendar::SetDate(TimerGameCalendar::date, 73);
		TimerManager<TimerGameCalendar>::Elapsed(1);
		fmt::print(stderr, "ENGINE offered {} {}\n", e->preview_company == c->index, e->preview_wait == 20);
		Cost("accept-test", Command<CMD_WANT_ENGINE_PREVIEW>::Do({}, e->index));
		Cost("accept", Command<CMD_WANT_ENGINE_PREVIEW>::Do(DoCommandFlag::Execute, e->index));
		fmt::print(stderr, "ENGINE joined {} {}\n", e->company_avail.Test(c->index), variant->company_avail.Test(c->index));
		e->intro_date = TimerGameCalendar::date - 365;
		CalendarEnginesMonthlyLoop();
		fmt::print(stderr, "ENGINE penalty {} {}\n", BlockPreview(c) == 20, e->flags.Test(EngineFlag::Available));
	} else if (args[1] == "reliability") {
		_settings_game.vehicle.never_expire_vehicles = false;
		/* Exercise the native Calendar::Year domain beyond byte-sized NewGRF inputs. */
		e->info.base_life = TimerGameCalendar::Year{511};
		StartupOneEngine(e, TimerGameCalendar::ConvertDateToYMD(TimerGameCalendar::date), 0);
		fmt::print(stderr, "ENGINE startup-life {}\n", e->duration_phase_2 > 5000);
		e->age = 60;
		e->duration_phase_1 = 10;
		e->duration_phase_2 = 20;
		e->duration_phase_3 = 10;
		e->reliability_final = 123;
		e->company_avail.Set();
		variant->display_last_variant = e->index;
		CalcEngineReliability(e, false);
		fmt::print(stderr, "ENGINE retired {} {} {}\n", e->company_avail.None(), e->reliability == 123, variant->display_last_variant == EngineID::Invalid());
		_settings_game.vehicle.never_expire_vehicles = true;
		e->company_avail.Set();
		e->reliability_max = 345;
		CalcEngineReliability(e, false);
		fmt::print(stderr, "ENGINE never-expire {} {}\n", e->company_avail.Any(), e->reliability == 345);
		variant->info.variant_id = e->index;
		variant->info.extra_flags.Set(ExtraEngineFlag::SyncReliability);
		variant->duration_phase_1 = e->duration_phase_1;
		variant->duration_phase_2 = e->duration_phase_2;
		variant->duration_phase_3 = e->duration_phase_3;
		variant->reliability_max = e->reliability_max;
		CalcEngineReliability(variant, true);
		fmt::print(stderr, "ENGINE synced {}\n", variant->reliability == e->reliability);
		/* Declared zero-life/1980 specs keep the original default cutoff at 2050. */
		for (Engine *other : Engine::Iterate()) {
			other->info.lifelength = TimerGameCalendar::Year{0};
			other->info.base_intro = TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{1980}, 0, 1);
		}
		SetYearEngineAgingStops();
		TimerGameCalendar::SetDate(TimerGameCalendar::ConvertYMDToDate(TimerGameCalendar::Year{2050}, 0, 1), 0);
		int32_t age = e->age;
		CalendarEnginesMonthlyLoop();
		fmt::print(stderr, "ENGINE cutoff {}\n", e->age == age);
	} else {
		FixtureAbort(__LINE__);
	}
	company.Restore();
	return true;
}
extern "C" void __real__Z22IConsoleStdLibRegisterv();
extern "C" void __wrap__Z22IConsoleStdLibRegisterv()
{
	__real__Z22IConsoleStdLibRegisterv();
	IConsole::CmdRegister("engines_scenario", EnginesScenario);
}
