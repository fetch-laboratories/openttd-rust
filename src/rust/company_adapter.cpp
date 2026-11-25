/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file company_adapter.cpp Typed noexcept callees and Company members for the Rust company owner. */
#include "../stdafx.h"
#include "../ai/ai.hpp"
#include "../cargotype.h"
#include "../command_func.h"
#include "../company_base.h"
#include "../company_cmd.h"
#include "../company_func.h"
#include "../company_gui.h"
#include "../company_manager_face.h"
#include "../economy_func.h"
#include "../goal_base.h"
#include "../network/network.h"
#include "../network/network_admin.h"
#include "../network/network_func.h"
#include "../rail.h"
#include "../road_func.h"
#include "../script/api/script_object.hpp"
#include "../settings_func.h"
#include "../signs_base.h"
#include "../sound_func.h"
#include "../station_func.h"
#include "../story_base.h"
#include "../table/strings.h"
#include "../timer/timer.h"
#include "../timer/timer_game_tick.h"
#include "../vehicle_base.h"
#include "../water.h"
#include "../waypoint_base.h"
#include "../window_func.h"
#include "company_adapter.h"
#include "company_identity_ffi.h"
#include "company_services.h"
#include "services_cargo.h"
#include "clock_ffi.h"
#include "command_services.h"
#include "engine_ffi.h"
#include "environment_services.h"
#include "services_objectives.h"
#include "services_station.h"
#include "settings_table_ffi.h"
#include "station_lifecycle_ffi.h"
#include "subsidy_ffi.h"
#include "vehicle_services.h"
#include "../safeguards.h"

extern void CompanyAdminRemove(CompanyID, CompanyRemoveReason);
extern const PriceBaseSpec _price_base_specs[];
extern TimeoutTimer<TimerGameTick> _new_competitor_timeout;
extern std::string _company_manager_face;

/* Constants Rust names as its own definitions. */
static_assert(Ticks::DAY_TICKS == 74 && Ticks::TICKS_PER_SECOND == 37);
static_assert(PR_END == 71 && MAX_COMPANIES == 15 && MAX_HISTORY_QUARTERS == 24 && EXPENSES_END == 13 && SCORE_END == 10);
static_assert(CompanyID::Invalid().base() == 255 && OWNER_NONE.base() == 16 && OWNER_DEITY.base() == 18 && CRR_END == 3);
static_assert(OWNER_END.base() == 19 && NETWORK_PUBLIC_KEY_LENGTH == 65 && CALCA_ADD == 0 && CALCA_REMOVE == 1 && CCA_NEW == 0 && CCA_NEW_AI == 1 && CCA_DELETE == 2);
static_assert(to_underlying(LoanCommand::Interval) == 0 && to_underlying(LoanCommand::Max) == 1 && to_underlying(LoanCommand::Amount) == 2);
static_assert(PCAT_NONE == 0 && PCAT_RUNNING == 1 && PCAT_CONSTRUCTION == 2 && LOAN_INTERVAL == 10000 && INITIAL_LOAN == 100000 && MAX_LOAN_LIMIT == 2000000000);
static_assert(sizeof(Owner) == sizeof(uint8_t));

/* Finance services. */
static CompanyFinances *Fin_company_finances(uint8_t id) noexcept
{
	Company *c = Company::GetIfValid(id);
	return c == nullptr ? nullptr : &c->Finances();
}
extern CompanyMask _dirty_company_finances;
/* InvalidateCompanyWindows(Company::Get(id)) by the company's ID, without the pool lookup:
 * it reads only the index. */
static void Fin_invalidate_company_windows(uint8_t id) noexcept { _dirty_company_finances.Set(CompanyID(id)); }

const OpenTTDCompanyFinanceServices &GetRustCompanyFinanceServices() noexcept
{
	static const OpenTTDCompanyFinanceServices services{
		.current_company = RustCurrentCompanyID,
		.set_current_company = RustSetCurrentCompanyID,
		.networking = RustNetworking,
		.company_finances = Fin_company_finances,
		.company_get_finances = RustCompanyGetFinances,
		.invalidate_company_windows = Fin_invalidate_company_windows,
	};
	return services;
}

const OpenTTDClockCompany &GetRustClockCompanyServices() noexcept
{
	static const OpenTTDClockCompany services{.services = &GetRustCompanyServices()};
	return services;
}

/** Callees that hold no Rust borrow. */
static Company *Co_allocate_company(uint8_t company, bool is_ai) noexcept
{
	if (CompanyID(company) == CompanyID::Invalid()) return new Company(STR_SV_UNNAMED, is_ai);
	return new (CompanyID(company)) Company(STR_SV_UNNAMED, is_ai);
}
static bool Co_allow_list_add(Company *c, OpenTTDByteSpan key) noexcept { return c->allow_list.Add({reinterpret_cast<const char *>(key.data), key.size}); }
static bool Co_allow_list_remove(Company *c, OpenTTDByteSpan key) noexcept { return c->allow_list.Remove({reinterpret_cast<const char *>(key.data), key.size}); }
template <typename T> static OpenTTDCompanyPoolCursor Co_next(uint32_t from) noexcept
{
	for (T *item : T::Iterate(from)) return {.item = item, .index = item->index.base()};
	return {.item = nullptr, .index = UINT32_MAX};
}
static OpenTTDCompanyPoolCursor Co_next_waypoint(uint32_t from) noexcept
{
	if (Waypoint *wp = OpenTTDNextWaypoint(from)) return {.item = static_cast<BaseStation *>(wp), .index = RustBaseStationID(wp)};
	return {.item = nullptr, .index = UINT32_MAX};
}
static bool Co_parse_company_manager_face(Company *c) noexcept
{
	if (_company_manager_face.empty()) return false;
	auto cmf = ParseCompanyManagerFaceCode(_company_manager_face);
	if (!cmf.has_value()) return false;
	c->face = std::move(*cmf);
	return true;
}
static void Co_send_company_manager_face(Company *c) noexcept
{
	if (!_company_manager_face.empty()) {
		auto cmf = ParseCompanyManagerFaceCode(_company_manager_face);
		if (cmf.has_value()) {
			Command<CMD_SET_COMPANY_MANAGER_FACE>::SendNet(STR_NULL, c->index, cmf->style, cmf->bits);
		}
	}
}
static uint16_t Co_use_free_unit(Company *c, uint8_t type) noexcept
{
	auto &unitidgen = c->freeunits[static_cast<VehicleType>(type)];
	return unitidgen.UseID(unitidgen.NextID());
}

const OpenTTDCompanyServices &GetRustCompanyServices() noexcept
{
	/* Construct process-lifetime C++ views before Rust accesses their storage. */
	(void)GetRustEconomy(); (void)GetRustPrices(); (void)GetRustCompanyScores();
	static const OpenTTDCompanyServices services{
		.cost = &GetRustCommandCostServices(),
		.finance = &GetRustCompanyFinanceServices(),
		.identity = &GetRustCompanyIdentityServices(),
		.shared = &GetRustSharedServices(),
		.engines = []() noexcept { return &EngineServices(); },
		.objectives = GetRustObjectivesServices,
		.settings = GetRustSettingServices,
		.station_lifecycle = RustStationLifecycleServices,
		.subsidies = RustSubsidyServices,
		.ai_can_start_new = []() noexcept { return AI::CanStartNew(); },
		/* AI::Stop resets the instance; ScriptInstance's destructor marks shutdown before
		 * releasing the VM and the mode objects suppress their errors, so it does not throw. */
		.ai_stop = [](uint8_t company) noexcept { AI::Stop(CompanyID(company)); },
		.allocate_company = Co_allocate_company,
		.allow_list_add = Co_allow_list_add,
		.allow_list_remove = Co_allow_list_remove,
		.can_allocate_company = []() noexcept { return Company::CanAllocateItem(); },
		.canal_maintenance_cost = [](uint32_t num) noexcept { return CanalMaintenanceCost(num).base(); },
		.cargo_initial_payment = RustCargoInitialPayment,
		.cargo_set_current_payment = RustCargoSetCurrentPayment,
		.change_window_owner = [](uint8_t old_owner, uint8_t new_owner) noexcept { ChangeWindowOwner(CompanyID(old_owner), CompanyID(new_owner)); },
		.close_company_windows = [](uint8_t company) noexcept { CloseCompanyWindows(CompanyID(company)); },
		.close_construction_windows = []() noexcept { CloseConstructionWindows(); },
		.company_admin_remove = [](uint8_t company, uint8_t reason) noexcept { CompanyAdminRemove(CompanyID(company), static_cast<CompanyRemoveReason>(reason)); },
		.company_count = []() noexcept { return Company::GetNumItems(); },
		.delete_company = [](Company *c) noexcept { delete c; },
		.delete_goal = [](Goal *g) noexcept { delete g; },
		.delete_story_page = [](StoryPage *sp) noexcept { delete sp; },
		.draw_string_company_colour = [](uint8_t company) noexcept { return static_cast<uint16_t>(GetDrawStringCompanyColour(CompanyID(company))); },
		.group_all = RustCompanyGroupAll,
		.location_of_hq = RustCompanyLocationOfHQ,
		.network_admin_company_new = [](Company *c) noexcept { NetworkAdminCompanyNew(c); },
		.network_clients_to_spectators = [](uint8_t company) noexcept { NetworkClientsToSpectators(CompanyID(company)); },
		.network_server = RustNetworkServer,
		.new_competitor_timeout_has_fired = []() noexcept { return _new_competitor_timeout.HasFired(); },
		.new_competitor_timeout_reset = [](uint32_t ticks) noexcept { _new_competitor_timeout.Reset({TimerGameTick::Priority::COMPETITOR_TIMEOUT, ticks}); },
		.next_goal = Co_next<Goal>,
		.next_sign = Co_next<Sign>,
		.next_story_page = Co_next<StoryPage>,
		.next_waypoint = Co_next_waypoint,
		.parse_company_manager_face = Co_parse_company_manager_face,
		.price_base_category = [](uint8_t price) noexcept { return static_cast<uint8_t>(_price_base_specs[price].category); },
		.price_base_start_price = [](uint8_t price) noexcept { return _price_base_specs[price].start_price.base(); },
		.rail_maintenance_cost = [](uint8_t railtype, uint32_t num, uint32_t total_num) noexcept { return RailMaintenanceCost(static_cast<RailType>(railtype), num, total_num).base(); },
		.road_maintenance_cost = [](uint8_t roadtype, uint32_t num, uint32_t total_num) noexcept { return RoadMaintenanceCost(static_cast<RoadType>(roadtype), num, total_num).base(); },
		.script_random_next = [](uint32_t max) noexcept { return ScriptObject::GetRandomizer(OWNER_NONE).Next(max); },
		.send_company_manager_face = Co_send_company_manager_face,
		.set_avail_railtypes = [](Company *c, uint64_t types) noexcept { c->avail_railtypes = RailTypes{types}; },
		.set_avail_roadtypes = [](Company *c, uint64_t types) noexcept { c->avail_roadtypes = RoadTypes{types}; },
		.set_default_company_settings = [](uint8_t company) noexcept { SetDefaultCompanySettings(CompanyID(company)); },
		.set_inaugurated_year = [](Company *c, int32_t year) noexcept { c->inaugurated_year = TimerGameEconomy::Year{year}; },
		.set_inaugurated_year_calendar = [](Company *c, int32_t year) noexcept { c->inaugurated_year_calendar = TimerGameCalendar::Year{year}; },
		.set_unitnumber = RustVehicleSetUnitNumber,
		.show_buy_company_dialog = [](uint8_t company, bool hostile_takeover) noexcept { ShowBuyCompanyDialog(CompanyID(company), hostile_takeover); },
		.show_company_finances = [](uint8_t company) noexcept { ShowCompanyFinances(CompanyID(company)); },
		.sign_owner = [](Sign *si) noexcept { return reinterpret_cast<uint8_t *>(&si->owner); },
		.signal_maintenance_cost = [](uint32_t num) noexcept { return SignalMaintenanceCost(num).base(); },
		.snd_play_fx = [](uint8_t sound) noexcept { SndPlayFx(static_cast<SoundFx>(sound)); },
		.station_maintenance_cost = [](uint32_t num) noexcept { return StationMaintenanceCost(num).base(); },
		.sync_company_settings = []() noexcept { SyncCompanySettings(); },
		.use_free_group = [](Company *c) noexcept { return c->freegroups.UseID(c->freegroups.NextID()); },
		.use_free_unit = Co_use_free_unit,
	};
	return services;
}

#include "service_table_check.hpp"
#include "abi-visit-OpenTTDCompanyServices.hpp"
#include "abi-visit-OpenTTDCompanyFinanceServices.hpp"
bool CheckAbi_company_adapter()
{
	return AbiServicesComplete(GetRustCompanyServices()) &&
		AbiServicesComplete(GetRustCompanyFinanceServices());
}
