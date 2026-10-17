/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file company_adapter.cpp Typed noexcept shared-world services for the Rust company owner. */
#include "../stdafx.h"
#ifdef WITH_RUST
#include "company_adapter.h"
#include "services_rail.h"
#include "company_services.h"
#include "services_landscape.h"
#include "presentation_services.h"
#include "settings_services.h"
#include "../company_base.h"
#include "../company_func.h"
#include "../company_cmd.h"
#include "../company_gui.h"
#include "../economy_func.h"
#include "../economy_base.h"
#include "../command_func.h"
#include "../vehicle_base.h"
#include "../vehicle_func.h"
#include "../vehicle_cmd.h"
#include "../aircraft.h"
#include "../station_base.h"
#include "../waypoint_base.h"
#include "../subsidy_base.h"
#include "../subsidy_func.h"
#include "../town.h"
#include "../signs_base.h"
#include "../goal_base.h"
#include "../story_base.h"
#include "../landscape.h"
#include "../rail.h"
#include "../road.h"
#include "../road_func.h"
#include "../station_func.h"
#include "../rail_map.h"
#include "../road_map.h"
#include "../signal_func.h"
#include "../cargomonitor.h"
#include "../autoreplace_func.h"
#include "../group.h"
#include "../network/network.h"
#include "../network/network_func.h"
#include "../network/network_admin.h"
#include "../ai/ai.hpp"
#include "../game/game.hpp"
#include "../script/api/script_object.hpp"
#include "../script/api/script_event_types.hpp"
#include "../news_func.h"
#include "../strings_func.h"
#include "../sound_func.h"
#include "../texteff.hpp"
#include "../timer/timer_game_tick.h"
#include "../timer/timer.h"
#include "../window_func.h"
#include "../object.h"
#include "../water.h"
#include "../table/strings.h"
#include <memory>
#include "../safeguards.h"

extern void CompanyAdminRemove(CompanyID, CompanyRemoveReason);
extern const PriceBaseSpec _price_base_specs[];
extern TimeoutTimer<TimerGameTick> _new_competitor_timeout;
/* Identity services keep company_cmd.cpp's file-local helpers there. */
void RustCompanyGenerateName(uint8_t) noexcept;
uint8_t RustCompanyGenerateColour() noexcept;
void RustCompanySetColour(uint8_t, uint8_t) noexcept;
void RustCompanySetup(uint8_t, bool) noexcept;
void RustCompanyNetworkSpectate(uint32_t) noexcept;
void RustCompanyNetworkNew(uint8_t, uint32_t, bool) noexcept;
void RustCompanyNetworkOwn(uint8_t, uint32_t) noexcept;

/* Constants Rust uses as literals. */
static_assert(Ticks::DAY_TICKS == 74 && Ticks::TICKS_PER_SECOND == 37);
static_assert(CalendarTime::DAYS_IN_YEAR * 2 == 730 && PR_END == 71 && MAX_COMPANIES == 15);
static_assert(RAILTYPE_END == 64 && ROADTYPE_END == 63 && MAX_HISTORY_QUARTERS == 24 && EXPENSES_END == 13);
static_assert(CompanyID::Invalid().base() == 255 && OWNER_NONE.base() == 16 && OWNER_DEITY.base() == 18 && CRR_END == 3);
static_assert(CalendarTime::ORIGINAL_BASE_YEAR.base() == 1920 && CalendarTime::ORIGINAL_MAX_YEAR.base() == 2090);
static_assert(RATING_INITIAL == 500 && VEH_TRAIN == 0 && VEH_ROAD == 1 && VEH_SHIP == 2 && VEH_AIRCRAFT == 3);

/** Next pool item at or after @p from whose owner field is @p owner. */
template <typename T, typename F> static uint32_t NextOwned(uint8_t owner, uint32_t from, F field) noexcept
{
	for (const T *item : T::Iterate(from)) {
		if (field(item) == CompanyID(owner)) return item->index.base();
	}
	return UINT32_MAX;
}

/* Finance services. */
static bool Fin_networking() noexcept { return _networking; }
static CompanyFinances *Fin_company_finances(uint8_t id) noexcept
{
	Company *c = Company::GetIfValid(id);
	return c == nullptr ? nullptr : &c->Finances();
}
static void Fin_invalidate_company_windows(uint8_t id) noexcept { InvalidateCompanyWindows(Company::Get(id)); }
static void Fin_give_money_message(uint8_t dest, int64_t amount) noexcept
{
	CompanyID d(dest);
	NetworkTextMessage(NETWORK_ACTION_GIVE_MONEY, GetDrawStringCompanyColour(_current_company), false, GetString(STR_COMPANY_NAME, _current_company), GetString(STR_COMPANY_NAME, d), amount);
}
static void Fin_money_animation(uint32_t t, int64_t amount) noexcept
{
	TileIndex tile(t);
	ShowCostOrIncomeAnimation(TileX(tile) * TILE_SIZE, TileY(tile) * TILE_SIZE, GetTilePixelZ(tile), amount);
}

/* Company identity and lifecycle services. */
static bool Co_company_is_ai(uint8_t id) noexcept { return Company::Get(id)->is_ai; }
static size_t Co_company_count() noexcept { return Company::GetNumItems(); }
static bool Co_can_allocate_company() noexcept { return Company::CanAllocateItem(); }
static bool Co_network_server() noexcept { return _network_server; }
static OpenTTDCompanyVehicleCounts Co_company_vehicle_counts(uint8_t id) noexcept
{
	/* Special owners are not companies; the original takeover limit is not reached for them. */
	const Company *c = Company::GetIfValid(id);
	if (c == nullptr) return {};
	return {c->group_all[VEH_TRAIN].num_vehicle, c->group_all[VEH_ROAD].num_vehicle, c->group_all[VEH_SHIP].num_vehicle, c->group_all[VEH_AIRCRAFT].num_vehicle};
}
static OpenTTDCompanyVehicleCounts Co_vehicle_limits() noexcept
{
	return {RustTrainLimit(), RustRoadVehicleLimit(), RustShipLimit(), RustAircraftLimit()};
}
static void Co_company_admin_update(uint8_t id) noexcept { CompanyAdminUpdate(Company::Get(id)); }
static void Co_company_in_trouble(uint8_t id) noexcept
{
	CompanyID cid(id);
	auto cni = std::make_unique<CompanyNewsInformation>(STR_NEWS_COMPANY_IN_TROUBLE_TITLE, Company::Get(cid));
	EncodedString headline = GetEncodedString(STR_NEWS_COMPANY_IN_TROUBLE_DESCRIPTION, cni->company_name);
	AddCompanyNewsItem(std::move(headline), std::move(cni));
	AI::BroadcastNewEvent(new ScriptEventCompanyInTrouble(cid));
	Game::NewEvent(new ScriptEventCompanyInTrouble(cid));
}
static void Co_post_company_delete(uint8_t id) noexcept { Command<CMD_COMPANY_CTRL>::Post(CCA_DELETE, CompanyID(id), CRR_BANKRUPT, INVALID_CLIENT_ID); }
static void Co_update_company_hq(uint8_t id, int32_t score) noexcept { UpdateCompanyHQ(Company::Get(id)->location_of_HQ, score); }
static void Co_performance_detail_dirty() noexcept { SetWindowDirty(WC_PERFORMANCE_DETAIL, 0); }
static void Co_company_graphs_dirty() noexcept
{
	SetWindowDirty(WC_INCOME_GRAPH, 0);
	SetWindowDirty(WC_OPERATING_PROFIT, 0);
	SetWindowDirty(WC_DELIVERED_CARGO, 0);
	SetWindowDirty(WC_PERFORMANCE_HISTORY, 0);
	SetWindowDirty(WC_COMPANY_VALUE, 0);
	SetWindowDirty(WC_COMPANY_LEAGUE, 0);
}
static void Co_company_infrastructure(uint8_t id, OpenTTDCompanyInfrastructure *out) noexcept
{
	const CompanyInfrastructure &i = Company::Get(id)->infrastructure;
	for (RailType rt = RAILTYPE_BEGIN; rt < RAILTYPE_END; rt++) out->rail[rt] = i.rail[rt];
	out->road_is_road = 0;
	for (RoadType rt = ROADTYPE_BEGIN; rt < ROADTYPE_END; rt++) {
		out->road[rt] = i.road[rt];
		if (RoadTypeIsRoad(rt)) out->road_is_road |= uint64_t{1} << rt;
	}
	out->rail_total = i.GetRailTotal();
	out->road_total = i.GetRoadTotal();
	out->tram_total = i.GetTramTotal();
	out->signal = i.signal;
	out->water = i.water;
	out->station = i.station;
}
static int64_t Co_rail_maintenance_cost(uint8_t rt, uint32_t num, uint32_t total) noexcept { return RailMaintenanceCost(static_cast<RailType>(rt), num, total).base(); }
static int64_t Co_signal_maintenance_cost(uint32_t num) noexcept { return SignalMaintenanceCost(num).base(); }
static int64_t Co_road_maintenance_cost(uint8_t rt, uint32_t num, uint32_t total) noexcept { return RoadMaintenanceCost(static_cast<RoadType>(rt), num, total).base(); }
static int64_t Co_canal_maintenance_cost(uint32_t num) noexcept { return CanalMaintenanceCost(num).base(); }
static int64_t Co_station_maintenance_cost(uint32_t num) noexcept { return StationMaintenanceCost(num).base(); }
static int64_t Co_airport_maintenance_cost(uint8_t id) noexcept { return AirportMaintenanceCost(CompanyID(id)).base(); }
static void Co_recession_news(bool begin) noexcept
{
	AddNewsItem(GetEncodedString(begin ? STR_NEWS_BEGIN_OF_RECESSION : STR_NEWS_END_OF_RECESSION), NewsType::Economy, NewsStyle::Normal, {});
}
static OpenTTDCompanyPriceBase Co_price_base(uint32_t price) noexcept { return {_price_base_specs[price].start_price.base(), static_cast<uint8_t>(_price_base_specs[price].category)}; }
static uint32_t Co_cargo_next(uint32_t from, int64_t *initial_payment) noexcept
{
	for (const CargoSpec *cs : CargoSpec::Iterate(from)) {
		*initial_payment = cs->initial_payment;
		return cs->Index();
	}
	return UINT32_MAX;
}
static void Co_set_cargo_payment(uint32_t cargo, int64_t payment) noexcept { CargoSpec::Get(static_cast<CargoType>(cargo))->current_payment = payment; }
static void Co_price_windows_dirty() noexcept
{
	SetWindowClassesDirty(WC_BUILD_VEHICLE);
	SetWindowClassesDirty(WC_REPLACE_VEHICLE);
	SetWindowClassesDirty(WC_VEHICLE_DETAILS);
	SetWindowClassesDirty(WC_COMPANY_INFRASTRUCTURE);
	InvalidateWindowData(WC_PAYMENT_RATES, 0);
}
static void Co_industry_daily_changes(bool init_counter) noexcept { StartupIndustryDailyChanges(init_counter); }
static void Co_clear_cargo_monitors(uint8_t id) noexcept
{
	ClearCargoPickupMonitoring(CompanyID(id));
	ClearCargoDeliveryMonitoring(CompanyID(id));
}
static void Co_clear_all_cargo_monitors() noexcept
{
	ClearCargoPickupMonitoring();
	ClearCargoDeliveryMonitoring();
}
static void Co_ask_merger(uint8_t best, uint8_t id, int64_t value) noexcept { AI::NewEvent(CompanyID(best), new ScriptEventCompanyAskMerger(CompanyID(id), value)); }
static void Co_show_buy_company(uint8_t id) noexcept { ShowBuyCompanyDialog(CompanyID(id), false); }
static uint32_t Co_script_random_next(uint32_t max) noexcept { return ScriptObject::GetRandomizer(OWNER_NONE).Next(max); }
static void Co_reset_competitor_timeout(uint32_t ticks) noexcept { _new_competitor_timeout.Reset({TimerGameTick::Priority::COMPETITOR_TIMEOUT, ticks}); }
static void Co_finances_dirty(uint8_t id) noexcept { InvalidateWindowData(WC_FINANCES, id); }
static void Co_show_company_finances(uint8_t id) noexcept { ShowCompanyFinances(CompanyID(id)); }
static void Co_new_year_sound(bool bad) noexcept { SndPlayFx(bad ? SND_01_BAD_YEAR : SND_00_GOOD_YEAR); }
static OpenTTDCompanyRef Co_allocate_company(uint8_t requested, bool is_ai) noexcept
{
	Company *c = requested == CompanyID::Invalid().base() ? new Company(STR_SV_UNNAMED, is_ai) : new (CompanyID(requested)) Company(STR_SV_UNNAMED, is_ai);
	return {&c->Finances(), c->index.base()};
}
static void Co_new_company_events(uint8_t id) noexcept
{
	AI::BroadcastNewEvent(new ScriptEventCompanyNew(CompanyID(id)), CompanyID(id));
	Game::NewEvent(new ScriptEventCompanyNew(CompanyID(id)));
}
static void Co_company_league_dirty() noexcept { InvalidateWindowData(WC_COMPANY_LEAGUE, 0, 0); }
static void Co_company_ctrl_windows() noexcept
{
	InvalidateWindowClassesData(WC_GAME_OPTIONS);
	InvalidateWindowClassesData(WC_SCRIPT_SETTINGS);
	InvalidateWindowClassesData(WC_SCRIPT_LIST);
}
static void Co_close_network_status() noexcept { CloseWindowById(WC_NETWORK_STATUS_WINDOW, WN_NETWORK_STATUS_WINDOW_JOIN); }
static void Co_assert_new_ai_slot([[maybe_unused]] uint8_t id) noexcept { assert(CompanyID(id) == CompanyID::Invalid() || !Company::IsValidID(id)); }
static void Co_company_bankrupt_news(uint8_t id) noexcept
{
	auto cni = std::make_unique<CompanyNewsInformation>(STR_NEWS_COMPANY_BANKRUPT_TITLE, Company::Get(id));
	EncodedString headline = GetEncodedString(STR_NEWS_COMPANY_BANKRUPT_DESCRIPTION, cni->company_name);
	AddCompanyNewsItem(std::move(headline), std::move(cni));
}
/* AI::Stop resets the instance; ScriptInstance's destructor marks shutdown before
 * releasing the VM and the mode objects suppress their errors, so it does not throw. */
static void Co_stop_ai(uint8_t id) noexcept { AI::Stop(CompanyID(id)); }
static void Co_delete_company(uint8_t id) noexcept { delete Company::Get(id); }
static void Co_company_removed(uint8_t id, uint8_t reason) noexcept
{
	CompanyID c_index(id);
	AI::BroadcastNewEvent(new ScriptEventCompanyBankrupt(c_index));
	Game::NewEvent(new ScriptEventCompanyBankrupt(c_index));
	CompanyAdminRemove(c_index, static_cast<CompanyRemoveReason>(reason));
	if (StoryPage::GetNumItems() == 0 || Goal::GetNumItems() == 0) InvalidateWindowData(WC_MAIN_TOOLBAR, 0);
	InvalidateWindowData(WC_CLIENT_LIST, 0);
}
static void Co_merger_news(uint8_t id, bool hostile) noexcept
{
	CompanyID ci(id);
	Company *c = Company::Get(ci);
	auto cni = std::make_unique<CompanyNewsInformation>(STR_NEWS_COMPANY_MERGER_TITLE, c, Company::Get(_current_company));
	EncodedString headline = hostile
		? GetEncodedString(STR_NEWS_MERGER_TAKEOVER_TITLE, cni->company_name, cni->other_company_name)
		: GetEncodedString(STR_NEWS_COMPANY_MERGER_DESCRIPTION, cni->company_name, cni->other_company_name, c->Finances().bankrupt_value);
	AddCompanyNewsItem(std::move(headline), std::move(cni));
	AI::BroadcastNewEvent(new ScriptEventCompanyMerger(ci, _current_company));
	Game::NewEvent(new ScriptEventCompanyMerger(ci, _current_company));
}
static void Co_acquisition_windows(uint8_t id) noexcept
{
	CloseCompanyWindows(CompanyID(id));
	InvalidateWindowClassesData(WC_TRAINS_LIST, 0);
	InvalidateWindowClassesData(WC_SHIPS_LIST, 0);
	InvalidateWindowClassesData(WC_ROADVEH_LIST, 0);
	InvalidateWindowClassesData(WC_AIRCRAFT_LIST, 0);
	InvalidateWindowData(WC_CLIENT_LIST, 0);
}

/* Ownership transfer services. */
static void Co_clients_to_spectators(uint8_t id) noexcept { NetworkClientsToSpectators(CompanyID(id)); }
static void Co_set_local_company(uint8_t id) noexcept { SetLocalCompany(CompanyID(id)); }
static uint32_t Co_subsidy_next_awarded(uint8_t owner, uint32_t from) noexcept { return NextOwned<Subsidy>(owner, from, [](const Subsidy *s) { return s->awarded; }); }
static void Co_delete_subsidy(uint32_t id) noexcept { delete Subsidy::Get(id); }
static void Co_set_subsidy_awarded(uint32_t id, uint8_t owner) noexcept { Subsidy::Get(id)->awarded = CompanyID(owner); }
static void Co_rebuild_subsidy_cache() noexcept { RebuildSubsidisedSourceAndDestinationCache(); }
static uint32_t Co_town_next(uint32_t from, uint8_t old, OpenTTDCompanyTownRights *out) noexcept
{
	for (const Town *t : Town::Iterate(from)) {
		*out = {t->ratings[CompanyID(old)], t->have_ratings.base(), t->exclusive_counter, t->exclusivity.base()};
		return t->index.base();
	}
	return UINT32_MAX;
}
static int16_t Co_town_rating(uint32_t id, uint8_t owner) noexcept { return Town::Get(id)->ratings[CompanyID(owner)]; }
static void Co_set_town_rating(uint32_t id, uint8_t owner, int16_t rating, bool have) noexcept
{
	Town *t = Town::Get(id);
	if (have) t->have_ratings.Set(CompanyID(owner));
	t->ratings[CompanyID(owner)] = rating;
	if (!have) t->have_ratings.Reset(CompanyID(owner));
}
static void Co_set_town_exclusivity(uint32_t id, uint8_t owner, uint8_t counter) noexcept
{
	Town *t = Town::Get(id);
	t->exclusive_counter = counter;
	t->exclusivity = CompanyID(owner);
}
static uint32_t Co_vehicle_next_owned(uint8_t owner, uint32_t from, uint8_t *type) noexcept
{
	for (const Vehicle *v : Vehicle::Iterate(from)) {
		if (v->owner != CompanyID(owner)) continue;
		*type = v->type;
		return v->index.base();
	}
	return UINT32_MAX;
}
static bool Co_aircraft_is_normal(uint32_t id) noexcept { return Aircraft::From(Vehicle::Get(id))->IsNormalAircraft(); }
static int64_t Co_vehicle_value(uint32_t id) noexcept { return Vehicle::Get(id)->value.base(); }
static bool Co_vehicle_is_primary(uint32_t id) noexcept { return Vehicle::Get(id)->IsPrimaryVehicle(); }
static OpenTTDCompanyVehicleProfit Co_vehicle_profit(uint32_t id) noexcept
{
	const Vehicle *v = Vehicle::Get(id);
	return {v->profit_last_year.base(), v->economy_age.base()};
}
static uint32_t Co_vehicle_previous(uint32_t id) noexcept
{
	const Vehicle *u = Vehicle::Get(id)->Previous();
	return u == nullptr ? UINT32_MAX : u->index.base();
}
static OpenTTDCompanyVehicleGroupFlags Co_vehicle_group_flags(uint32_t id) noexcept
{
	const Vehicle *v = Vehicle::Get(id);
	return {v->IsEngineCountable(), v->IsPrimaryVehicle()};
}
static bool Co_vehicle_service_interval_is_custom(uint32_t id) noexcept { return Vehicle::Get(id)->ServiceIntervalIsCustom(); }
static void Co_delete_vehicle(uint32_t id) noexcept { delete Vehicle::Get(id); }
static void Co_count_group_engine(uint32_t id, int32_t delta) noexcept { GroupStatistics::CountEngine(Vehicle::Get(id), delta); }
static void Co_count_group_vehicle(uint32_t id, int32_t delta) noexcept { GroupStatistics::CountVehicle(Vehicle::Get(id), delta); }
/** The original's CompanyServiceInterval argument evaluation and nested command. */
static void Co_reset_service_interval(uint32_t id, uint8_t company) noexcept
{
	const Vehicle *v = Vehicle::Get(id);
	const Company *c = Company::Get(company);
	int interval = CompanyServiceInterval(c, v->type);
	Command<CMD_CHANGE_SERVICE_INT>::Do({DoCommandFlag::Execute, DoCommandFlag::Bankrupt}, v->index, interval, false, c->settings.vehicle.servint_ispercent);
}
/** Owner write and its cache invalidation, consecutive in the original. */
static void Co_set_vehicle_owner(uint32_t id, uint8_t owner) noexcept
{
	Vehicle *v = Vehicle::Get(id);
	v->owner = CompanyID(owner);
	v->colourmap = PAL_NONE;
	v->InvalidateNewGRFCache();
}
static void Co_assign_unit_number(uint32_t id, uint8_t company) noexcept
{
	Vehicle *v = Vehicle::Get(id);
	auto &unitidgen = Company::Get(company)->freeunits[v->type];
	v->unitnumber = unitidgen.UseID(unitidgen.NextID());
}
static void Co_remove_engine_replacements(uint8_t id) noexcept { RemoveAllEngineReplacementForCompany(Company::Get(id)); }
static uint32_t Co_group_next_owned(uint8_t owner, uint32_t from) noexcept { return NextOwned<Group>(owner, from, [](const Group *g) { return g->owner; }); }
static void Co_delete_group(uint32_t id) noexcept { delete Group::Get(id); }
static void Co_transfer_group(uint32_t id, uint8_t owner) noexcept
{
	Group *g = Group::Get(id);
	Company *c = Company::Get(owner);
	g->owner = c->index;
	g->number = c->freegroups.UseID(c->freegroups.NextID());
}
static void Co_copy_service_interval_defaults(uint8_t old_owner, uint8_t new_owner) noexcept
{
	Company *old_company = Company::Get(old_owner);
	const Company *new_company = Company::Get(new_owner);
	old_company->settings.vehicle.servint_aircraft = new_company->settings.vehicle.servint_aircraft;
	old_company->settings.vehicle.servint_trains = new_company->settings.vehicle.servint_trains;
	old_company->settings.vehicle.servint_roadveh = new_company->settings.vehicle.servint_roadveh;
	old_company->settings.vehicle.servint_ships = new_company->settings.vehicle.servint_ships;
	old_company->settings.vehicle.servint_ispercent = new_company->settings.vehicle.servint_ispercent;
}
static void Co_update_autoreplace(uint8_t id) noexcept { GroupStatistics::UpdateAutoreplace(CompanyID(id)); }
static uint32_t Co_map_size() noexcept { return Map::Size(); }
static void Co_change_tile_owner(uint32_t tile, uint8_t old_owner, uint8_t new_owner) noexcept { ChangeTileOwner(TileIndex(tile), CompanyID(old_owner), CompanyID(new_owner)); }
static OpenTTDCompanySignalTile Co_signal_tile(uint32_t t) noexcept
{
	TileIndex tile(t);
	OpenTTDCompanySignalTile r{IsTileType(tile, MP_RAILWAY), false, 0, false, 0};
	r.crossing = !r.railway && IsLevelCrossingTile(tile);
	/* House and industry tiles have no owner; only the two tested kinds are read. */
	if (r.railway || r.crossing) r.owner = RustTileOwnerValue(t);
	r.has_signals = r.railway && RustRailHasSignals(t);
	if (r.has_signals) r.tracks = static_cast<uint8_t>(RustRailTracks(t));
	return r;
}
static void Co_add_track_to_signal_buffer(uint32_t tile, uint8_t track, uint8_t owner) noexcept { AddTrackToSignalBuffer(TileIndex(tile), static_cast<Track>(track), CompanyID(owner)); }
static void Co_update_level_crossing(uint32_t tile) noexcept { UpdateLevelCrossing(TileIndex(tile)); }
static void Co_add_airport_infrastructure(uint8_t new_owner, uint8_t old_owner) noexcept { Company::Get(new_owner)->infrastructure.airport += Company::Get(old_owner)->infrastructure.airport; }
static uint32_t Co_station_next_owned(uint8_t owner, uint32_t from) noexcept { return NextOwned<Station>(owner, from, [](const Station *st) { return st->owner; }); }
static uint32_t Co_station_facility_count(uint32_t id) noexcept { return Station::Get(id)->facilities.Count(); }
static OpenTTDCompanyStationTimes Co_station_times(uint32_t id) noexcept
{
	const Station *st = Station::Get(id);
	return {st->time_since_load, st->time_since_unload};
}
static void Co_set_station_owner(uint32_t id, uint8_t owner) noexcept { Station::Get(id)->owner = CompanyID(owner); }
static uint32_t Co_waypoint_next_owned(uint8_t owner, uint32_t from) noexcept { return NextOwned<Waypoint>(owner, from, [](const Waypoint *wp) { return wp->owner; }); }
static void Co_set_waypoint_owner(uint32_t id, uint8_t owner) noexcept { Waypoint::Get(id)->owner = CompanyID(owner); }
static uint32_t Co_sign_next_owned(uint8_t owner, uint32_t from) noexcept { return NextOwned<Sign>(owner, from, [](const Sign *si) { return si->owner; }); }
static void Co_set_sign_owner(uint32_t id, uint8_t owner) noexcept { Sign::Get(id)->owner = CompanyID(owner); }
static uint32_t Co_goal_next_owned(uint8_t owner, uint32_t from) noexcept { return NextOwned<Goal>(owner, from, [](const Goal *g) { return g->company; }); }
static void Co_delete_goal(uint32_t id) noexcept { delete Goal::Get(id); }
static uint32_t Co_story_page_next_owned(uint8_t owner, uint32_t from) noexcept { return NextOwned<StoryPage>(owner, from, [](const StoryPage *sp) { return sp->company; }); }
static void Co_delete_story_page(uint32_t id) noexcept { delete StoryPage::Get(id); }
static void Co_change_window_owner(uint8_t old_owner, uint8_t new_owner) noexcept { ChangeWindowOwner(CompanyID(old_owner), CompanyID(new_owner)); }

const OpenTTDCompanyFinanceServices &GetRustCompanyFinanceServices() noexcept
{
	static const OpenTTDCompanyFinanceServices services{
		.current_company = RustCurrentCompanyID, .set_current_company = RustSetCurrentCompanyID,
		.networking = Fin_networking, .company_finances = Fin_company_finances,
		.invalidate_company_windows = Fin_invalidate_company_windows,
		.give_money_message = Fin_give_money_message, .money_animation = Fin_money_animation,
	};
	return services;
}

const OpenTTDCompanyServices &GetRustCompanyServices() noexcept
{
	/* Construct process-lifetime C++ views before Rust accesses their storage. */
	(void)GetRustEconomy(); (void)GetRustPrices(); (void)GetRustCompanyScores();
	static const OpenTTDCompanyServices services{
		.finance = &GetRustCompanyFinanceServices(), .shared = &GetRustSharedServices(),
		.company_next = RustCompanyRefNext, .company_is_ai = Co_company_is_ai,
		.company_count = Co_company_count, .can_allocate_company = Co_can_allocate_company,
		.local_company = RustLocalCompanyID, .network_server = Co_network_server,
		.company_vehicle_counts = Co_company_vehicle_counts, .vehicle_limits = Co_vehicle_limits,
		.company_admin_update = Co_company_admin_update, .company_in_trouble = Co_company_in_trouble,
		.post_company_delete = Co_post_company_delete, .update_company_hq = Co_update_company_hq,
		.performance_detail_dirty = Co_performance_detail_dirty, .company_graphs_dirty = Co_company_graphs_dirty,
		.company_infrastructure = Co_company_infrastructure,
		.rail_maintenance_cost = Co_rail_maintenance_cost, .signal_maintenance_cost = Co_signal_maintenance_cost,
		.road_maintenance_cost = Co_road_maintenance_cost, .canal_maintenance_cost = Co_canal_maintenance_cost,
		.station_maintenance_cost = Co_station_maintenance_cost, .airport_maintenance_cost = Co_airport_maintenance_cost,
		.recession_news = Co_recession_news, .price_base = Co_price_base,
		.cargo_next = Co_cargo_next, .set_cargo_payment = Co_set_cargo_payment,
		.price_windows_dirty = Co_price_windows_dirty, .industry_daily_changes = Co_industry_daily_changes,
		.clear_cargo_monitors = Co_clear_cargo_monitors, .clear_all_cargo_monitors = Co_clear_all_cargo_monitors,
		.generate_company_name = RustCompanyGenerateName, .ask_merger = Co_ask_merger,
		.is_interactive_company = RustIsInteractiveCompany, .show_buy_company = Co_show_buy_company,
		.script_random_next = Co_script_random_next, .reset_competitor_timeout = Co_reset_competitor_timeout,
		.finances_dirty = Co_finances_dirty, .show_company_finances = Co_show_company_finances,
		.new_year_sound = Co_new_year_sound, .generate_company_colour = RustCompanyGenerateColour,
		.allocate_company = Co_allocate_company, .set_company_colour = RustCompanySetColour,
		.setup_new_company = RustCompanySetup, .new_company_events = Co_new_company_events,
		.company_league_dirty = Co_company_league_dirty, .company_ctrl_windows = Co_company_ctrl_windows,
		.close_network_status = Co_close_network_status, .network_spectate = RustCompanyNetworkSpectate,
		.network_company_new = RustCompanyNetworkNew, .network_own_company = RustCompanyNetworkOwn,
		.assert_new_ai_slot = Co_assert_new_ai_slot, .company_bankrupt_news = Co_company_bankrupt_news,
		.stop_ai = Co_stop_ai, .delete_company = Co_delete_company, .company_removed = Co_company_removed,
		.merger_news = Co_merger_news, .acquisition_windows = Co_acquisition_windows,
		.clients_to_spectators = Co_clients_to_spectators, .set_local_company = Co_set_local_company,
		.subsidy_next_awarded = Co_subsidy_next_awarded, .delete_subsidy = Co_delete_subsidy,
		.set_subsidy_awarded = Co_set_subsidy_awarded, .rebuild_subsidy_cache = Co_rebuild_subsidy_cache,
		.town_next = Co_town_next, .town_rating = Co_town_rating,
		.set_town_rating = Co_set_town_rating, .set_town_exclusivity = Co_set_town_exclusivity,
		.vehicle_next_owned = Co_vehicle_next_owned, .aircraft_is_normal = Co_aircraft_is_normal,
		.vehicle_value = Co_vehicle_value, .vehicle_is_primary = Co_vehicle_is_primary,
		.vehicle_profit = Co_vehicle_profit, .vehicle_previous = Co_vehicle_previous,
		.vehicle_group_flags = Co_vehicle_group_flags, .vehicle_service_interval_is_custom = Co_vehicle_service_interval_is_custom,
		.delete_vehicle = Co_delete_vehicle, .count_group_engine = Co_count_group_engine,
		.count_group_vehicle = Co_count_group_vehicle, .reset_service_interval = Co_reset_service_interval,
		.set_vehicle_owner = Co_set_vehicle_owner, .assign_unit_number = Co_assign_unit_number,
		.remove_engine_replacements = Co_remove_engine_replacements, .group_next_owned = Co_group_next_owned,
		.delete_group = Co_delete_group, .transfer_group = Co_transfer_group,
		.copy_service_interval_defaults = Co_copy_service_interval_defaults, .update_autoreplace = Co_update_autoreplace,
		.map_size = Co_map_size, .change_tile_owner = Co_change_tile_owner,
		.signal_tile = Co_signal_tile, .has_signal_on_track = RustRailHasSignalTrack,
		.add_track_to_signal_buffer = Co_add_track_to_signal_buffer, .update_level_crossing = Co_update_level_crossing,
		.update_signals_in_buffer = RustUpdateSignals, .add_airport_infrastructure = Co_add_airport_infrastructure,
		.station_next_owned = Co_station_next_owned, .station_facility_count = Co_station_facility_count,
		.station_times = Co_station_times, .set_station_owner = Co_set_station_owner,
		.waypoint_next_owned = Co_waypoint_next_owned, .set_waypoint_owner = Co_set_waypoint_owner,
		.sign_next_owned = Co_sign_next_owned, .set_sign_owner = Co_set_sign_owner,
		.goal_next_owned = Co_goal_next_owned, .delete_goal = Co_delete_goal,
		.story_page_next_owned = Co_story_page_next_owned, .delete_story_page = Co_delete_story_page,
		.change_window_owner = Co_change_window_owner, .mark_whole_screen_dirty = RustScreenDirty,
	};
	return services;
}

CommandCost RustCompanyCost(const OpenTTDCompanyCost &x)
{
	switch (x.error) {
		case 0: return CommandCost(static_cast<ExpensesType>(x.expense), Money(x.cost));
		case 1: return CMD_ERROR;
		case 2: return CommandCostWithParam(STR_ERROR_MAXIMUM_PERMITTED_LOAN, Money(x.param));
		case 3: return CommandCost(STR_ERROR_LOAN_ALREADY_REPAID);
		case 4: return CommandCostWithParam(STR_ERROR_CURRENCY_REQUIRED, Money(x.param));
		case 5: return CommandCost(STR_ERROR_INSUFFICIENT_FUNDS);
		case 6: return CommandCost(STR_ERROR_TOO_MANY_VEHICLES_IN_GAME);
		default: NOT_REACHED();
	}
}
#endif /* WITH_RUST */
