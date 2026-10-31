/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file economy.cpp Handling of the economy. */

#include "stdafx.h"
#include <ranges>
#include "company_func.h"
#include "command_func.h"
#include "industry.h"
#include "town.h"
#include "news_func.h"
#include "network/network.h"
#include "network/network_func.h"
#include "ai/ai.hpp"
#include "aircraft.h"
#include "train.h"
#include "newgrf_engine.h"
#include "engine_base.h"
#include "ground_vehicle.hpp"
#include "newgrf_cargo.h"
#include "newgrf_sound.h"
#include "newgrf_industrytiles.h"
#include "newgrf_station.h"
#include "newgrf_airporttiles.h"
#include "newgrf_roadstop.h"
#include "object.h"
#include "strings_func.h"
#include "vehicle_func.h"
#include "sound_func.h"
#include "autoreplace_func.h"
#include "company_gui.h"
#include "signs_base.h"
#include "subsidy_base.h"
#include "subsidy_func.h"
#include "station_base.h"
#include "waypoint_base.h"
#include "economy_base.h"
#include "core/pool_func.hpp"
#include "core/backup_type.hpp"
#include "core/container_func.hpp"
#include "cargo_type.h"
#include "water.h"
#include "game/game.hpp"
#include "cargomonitor.h"
#include "goal_base.h"
#include "story_base.h"
#include "linkgraph/refresh.h"
#include "company_cmd.h"
#include "economy_cmd.h"
#include "vehicle_cmd.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"

#include "table/strings.h"
#include "table/pricebase.h"

#include "rust/services_industry.h"
#include "rust/cargo_registry_ffi.h"
#include "rust/services_cargo.h"
#include "rust/company_adapter.h"
#include "rust/presentation_services.h"
#include "rust/tile_owner_services.h"
#include "rust/vehicle_services.h"
#include "rust/company_services.h"
#include "rust/settings_services.h"
#include "rust/industry_shell_services.h"
#include "rust/industry_resolver_ffi.h"
#include "rust/vehicle_visits_ffi.h"
#include "rust/services_station.h"
#include "rust/vehicle_refit_ffi.h"
#include "rust/command_cost_services.h"

#include "rust/clock_ffi.h"

#include "safeguards.h"


/* Initialize the cargo payment-pool */
CargoPaymentPool _cargo_payment_pool("CargoPayment");
INSTANTIATE_POOL_METHODS(CargoPayment)

/**
 * Multiply two integer values and shift the results to right.
 *
 * This function multiplies two integer values. The result is
 * shifted by the amount of shift to right.
 *
 * @param a The first integer
 * @param b The second integer
 * @param shift The amount to shift the value to right.
 * @return The shifted result
 */

/**
 * Score info, values used for computing the detailed performance rating.
 */
const ScoreInfo _score_info[] = {
	{     120, 100}, // SCORE_VEHICLES
	{      80, 100}, // SCORE_STATIONS
	{   10000, 100}, // SCORE_MIN_PROFIT
	{   50000,  50}, // SCORE_MIN_INCOME
	{  100000, 100}, // SCORE_MAX_INCOME
	{   40000, 400}, // SCORE_DELIVERED
	{       8,  50}, // SCORE_CARGO
	{10000000,  50}, // SCORE_MONEY
	{  250000,  50}, // SCORE_LOAN
	{       0,   0}  // SCORE_TOTAL
};

Economy &GetRustEconomy()
{
	static Economy *state = new (openttd_rust_economy_state()) Economy{};
	return *state;
}
Prices &GetRustPrices()
{
	static Money *prices = new (openttd_rust_economy_prices()) Prices{};
	return *reinterpret_cast<Prices *>(prices);
}
CompanyScoreParts &GetRustCompanyScores()
{
	static CompanyScoreParts *scores = new (openttd_rust_company_scores()) CompanyScoreParts{};
	return *scores;
}
/** Calendar year and difficulty read by inflation, price and startup entries. */
OpenTTDEconomySettings RustEconomySettings() noexcept
{
	const auto &d = _settings_game.difficulty;
	return {.year = TimerGameCalendar::year.base(), .max_loan = d.max_loan, .initial_interest = d.initial_interest,
		.vehicle_costs = d.vehicle_costs, .construction_cost = d.construction_cost, .inflation = _settings_game.economy.inflation};
}

/**
 * Calculate the value of the assets of a company.
 *
 * @param c The company to calculate the value of.
 * @return The value of the assets of the company.
 */

/**
 * Calculate the value of the company. That is the value of all
 * assets (vehicles, stations) and money (including loan),
 * except when including_loan is \c false which is useful when
 * we want to calculate the value for bankruptcy.
 * @param c the company to get the value of.
 * @param including_loan include the loan in the company value.
 * @return the value of the company.
 */
Money CalculateCompanyValue(const Company *c, bool including_loan)
{
	return openttd_rust_company_value(&GetRustCompanyServices(), &c->Finances(), c->index.base(), including_loan);
}

/**
 * Calculate what you have to pay to take over a company.
 *
 * This is different from bankruptcy and company value, and involves a few
 * more parameters to make it more realistic.
 *
 * You have to pay for:
 * - The value of all the assets in the company.
 * - The loan the company has (the investors really want their money back).
 * - The profit for the next two years (if positive) based on the last four quarters.
 *
 * And on top of that, they walk away with all the money they have in the bank.
 *
 * @param c the company to get the value of.
 * @return The value of the company.
 */
Money CalculateHostileTakeoverValue(const Company *c)
{
	return openttd_rust_company_hostile_takeover_value(&GetRustCompanyServices(), &c->Finances(), c->index.base());
}

/**
 * if update is set to true, the economy is updated with this score
 *  (also the house is updated, should only be true in the on-tick event)
 * @param update the economy with calculated score
 * @param c company been evaluated
 * @return actual score of this company
 *
 */
int UpdateCompanyRatingAndValue(Company *c, bool update)
{
	return openttd_rust_company_update_rating(&GetRustCompanyServices(), &c->Finances(), c->index.base(), update);
}

/**
 * Change the ownership of all the items of a company.
 * @param old_owner The company that gets removed.
 * @param new_owner The company to merge to, or INVALID_OWNER to remove the company.
 */
void ChangeOwnershipOfCompanyItems(Owner old_owner, Owner new_owner)
{
	RustTileOwnerContext ownership;
	openttd_rust_company_change_ownership(&GetRustCompanyServices(), &ownership.view, old_owner.base(), new_owner.base());
}

/**
 * Check for bankruptcy of a company. Called every three months.
 * @param c Company to check.
 */

/**
 * Update the finances of all companies.
 * Pay for the stations, update the history graph, update ratings and company values, and deal with bankruptcy.
 */

/**
 * Add monthly inflation
 * @param check_year Shall the inflation get stopped after 170 years?
 * @return true if inflation is maxed and nothing was changed
 */
bool AddInflation(bool check_year)
{
	return openttd_rust_economy_add_inflation(check_year, TimerGameCalendar::year.base());
}

/**
 * Computes all prices, payments and maximum loan.
 */
void RecomputePrices()
{
	const OpenTTDEconomySettings settings = RustEconomySettings();
	openttd_rust_economy_recompute_prices(&GetRustCompanyServices(), &settings);
}

/** Let all companies pay the monthly interest on their loan. */



/**
 * Reset changes to the price base multipliers.
 */
void ResetPriceBaseMultipliers()
{
	openttd_rust_economy_reset_price_multipliers();
}

/**
 * Change a price base by the given factor.
 * The price base is altered by factors of two.
 * NewBaseCost = OldBaseCost * 2^n
 * @param price Index of price base to change.
 * @param factor Amount to change by.
 */
void SetPriceBaseMultiplier(Price price, int factor)
{
	assert(price < PR_END);
	openttd_rust_economy_set_price_multiplier(price, factor);
}

/**
 * Initialize the variables that will maintain the daily industry change system.
 * @param init_counter specifies if the counter is required to be initialized
 */
void StartupIndustryDailyChanges(bool init_counter)
{
	openttd_rust_industry_daily_start(RustIndustryBuilderState(), Map::LogX() + Map::LogY(), init_counter);
}


/**
 * Resets economy to initial values
 */
void InitializeEconomy()
{
	openttd_rust_economy_initialize();
}

/**
 * Determine a certain price
 * @param index Price base
 * @param cost_factor Price factor
 * @param grf_file NewGRF to use local price multipliers from.
 * @param shift Extra bit shifting after the computation
 * @return Price
 */
Money GetPrice(Price index, uint cost_factor, const GRFFile *grf_file, int shift)
{
	/* GRF storage stays canonical C++; observe its scalar at the original point. */
	if (index >= PR_END) return 0;
	shift += RustGRFPriceShift(grf_file, index);
	return openttd_rust_economy_price(index, cost_factor, shift);
}

/* Leaves only read/write canonical world storage or call existing shared services.
 * NewGRF sprite-group resolution cannot reenter these owners or execute commands.
 * No exception crosses Rust; allocation/debug/UI failures terminate here. */
static const OpenTTDCargoServices _cargo_services = {
	.spec = [](uint8_t cargo, OpenTTDCargoSpec *out) noexcept {
		const CargoSpec *cs = RustCargoSpec(cargo);
		*out = {cs->current_payment.base(), uint8_t(cs->IsValid()), uint8_t(cs->callback_mask.Test(CargoCallbackMask::ProfitCalc)), cs->transit_periods[0], cs->transit_periods[1]};
	},
	.registry = &GetRustCargoRegistryServices(),
	.station_company = RustStationOwnerByte<const Station>,
	.town_acceptance_effect = RustCargoTownAcceptanceEffect,
	.station_town = RustStationStationTown<const Station>,
	.resolver = &GetIndustryResolverServices(),
	.subsidies = RustSubsidyServices,
	.industry_delivery = &GetIndustryDeliveryServices(),
	.last_station = RustVehicleLastStationVisited,
	.vehicle_owner = RustVehicleOwnerOf,
	.detach_payment = RustVehicleDetachPayment,
	.finance = &GetRustCompanyFinanceServices(),
	.vehicles = &GetRustVehicleServices(),
	.local_company = RustIsLocalCompany,
	.expense = RustVehicleExpense,
	.profit = RustVehicleRead<int64_t, RustVehicleProfit>,
	.x = RustVehicleRead<int32_t, RustVehicleX>,
	.y = RustVehicleRead<int32_t, RustVehicleY>,
	.z = RustVehicleRead<int32_t, RustVehicleZ>,
	.cost_animation = RustCostOrIncomeAnimation,
	.feeder_animation = RustFeederIncomeAnimation,
	.subsidy_multiplier = RustSubsidyMultiplier,
	.feeder_payment_share = RustFeederPaymentShare,
};

static OpenTTDCargoDelivery *GetCargoDeliveryState()
{
	static const std::unique_ptr<OpenTTDCargoDelivery, decltype(&openttd_rust_cargo_delivery_destroy)> state(openttd_rust_cargo_delivery_new(), openttd_rust_cargo_delivery_destroy);
	return state.get();
}

Money GetTransportedGoodsIncome(uint num_pieces, uint dist, uint16_t transit_periods, CargoType cargo_type)
{
	return openttd_rust_cargo_income(&_cargo_services, num_pieces, dist, transit_periods, cargo_type);
}

CargoPayment::CargoPayment() : rust_state(openttd_rust_cargo_payment_new(nullptr, StationID::Invalid().base())) {}
CargoPayment::CargoPayment(Vehicle *front) : rust_state(openttd_rust_cargo_payment_new(RustVehicle(front), RustVehicleLastStationVisited(RustVehicle(front)))) {}
CargoPayment::~CargoPayment() { openttd_rust_cargo_payment_destroy(this->rust_state, &_cargo_services, CleaningPool()); }
Vehicle *CargoPayment::GetFront() const { return VehicleOf(openttd_rust_cargo_payment_front(this->rust_state)); }
void CargoPayment::AfterLoad()
{
	this->GetFront()->cargo_payment = this;
	openttd_rust_cargo_payment_afterload(this->rust_state, &_cargo_services);
}
void CargoPayment::PayFinalDelivery(CargoType cargo, const CargoPacket *cp, uint count, TileIndex current_tile)
{
	assert(int(count) > 0);
	Source source = cp->GetSource();
	openttd_rust_cargo_payment_final(this->rust_state, GetCargoDeliveryState(), &_cargo_services, cargo, cp->RustOwner(), count, cp->GetDistance(current_tile), cp->GetPeriodsInTransit(), source, &GetRustCargoMonitorServices());
}
Money CargoPayment::PayTransfer(CargoType cargo, const CargoPacket *cp, uint count, TileIndex current_tile)
{
	return openttd_rust_cargo_payment_transfer(this->rust_state, &_cargo_services, cargo, cp->RustOwner(), count, cp->GetDistance(current_tile), cp->GetPeriodsInTransit());
}

static const OpenTTDStationLoading _rust_station_loading = {
	.control = [](OpenTTDVehicle *handle) noexcept -> OpenTTDLoadingControl {
		Vehicle *v = VehicleOf(handle);
		bool refit = v->current_order.IsRefit();
		return {
			.ticks = RustVehicleLoadUnloadTicks(handle),
			.vehicle_type = v->type,
			.unload_type = to_underlying(RustVehicleOrderUnloadType(v)),
			.load_type = to_underlying(RustVehicleOrderLoadType(v)),
			.refit_cargo = refit ? v->current_order.GetRefitCargo() : uint8_t(0),
			.refit = refit,
			.full_load = v->current_order.IsFullLoadOrder(),
			.stop = v->vehicle_flags.Test(VehicleFlag::StopLoading),
			.gradual = _settings_game.order.gradual_loading,
			.improved = _settings_game.order.improved_load,
		};
	},
	.part = [](OpenTTDVehicle *handle) noexcept -> OpenTTDLoadingPart {
		Vehicle *v = VehicleOf(handle);
		uint16_t capacity = RustVehicleCargoCap(handle);
		return {
			.capacity = capacity,
			.cargo = capacity != 0 ? v->cargo_type : uint8_t(0),
			.vehicle_type = v->type,
			.unloading = capacity != 0 && v->vehicle_flags.Test(VehicleFlag::CargoUnloading),
		};
	},
	.set_refit_capacity_and_stats = [](OpenTTDVehicle *handle, OpenTTDVehicle *front_handle, uint8_t kind, uint16_t capacity, bool refitted) noexcept -> OpenTTDLoadingStats {
		Vehicle *v = VehicleOf(handle);
		Vehicle *front = VehicleOf(front_handle);
		uint8_t cargo = 0;
		if (refitted) {
			cargo = v->cargo_type;
			capacity = RustVehicleCargoCap(handle);
		}
		RustVehicleSetRefitCap(handle, capacity);
		int speed;
		switch (kind) {
			case VEH_TRAIN:
			case VEH_SHIP: speed = front->vcache.cached_max_speed; break;
			case VEH_ROAD: speed = front->vcache.cached_max_speed / 2; break;
			case VEH_AIRCRAFT: speed = Aircraft::From(front)->GetSpeedOldUnits(); break;
			default: NOT_REACHED();
		}
		uint32_t stored = v->cargo.StoredCount();
		return {
			.stored = stored,
			.load_count = capacity != stored ? v->cargo.ActionCount(VehicleCargoList::MTA_LOAD) : 0,
			.speed = speed,
			.age = (TimerGameCalendar::year - front->build_year).base(),
			.capacity = capacity,
			.cargo = cargo,
		};
	},
	.wait_observation = [](OpenTTDVehicle *handle) noexcept -> OpenTTDLoadingWait {
		Vehicle *v = VehicleOf(handle);
		return {
			v->lateness_counter, v->current_order.GetTimetabledWait()
		};
	},
	.load_engine = [](OpenTTDVehicle *handle) noexcept -> OpenTTDLoadEngine {
		const Engine *engine = VehicleOf(handle)->GetEngine();
		return {RustEngine(engine), engine->info.load_amount};
	},
	.grf_version = [](const OpenTTDEngine *handle) noexcept -> uint8_t {
		return RustEngineGRFVersion(EngineOf(handle));
	},
	.engine_load_callback = [](const OpenTTDEngine *handle) noexcept -> bool {
		return EngineOf(handle)->info.callback_mask.Test(VehicleCallbackMask::LoadAmount);
	},
	.engine_no_multiplier = [](const OpenTTDEngine *handle) noexcept -> bool {
		return EngineOf(handle)->info.misc_flags.Test(EngineMiscFlag::NoDefaultCargoMultiplier);
	},
	.delivery_remaining = [](OpenTTDVehicle *handle) noexcept -> OpenTTDDeliveryRemaining {
		Vehicle *v = VehicleOf(handle);
		return {
			v->cargo.RemainingCount(), v->cargo.ActionCount(VehicleCargoList::MTA_DELIVER), RustVehicleCargoCap(handle)
		};
	},
	.cargo_stored = [](OpenTTDVehicle *handle) noexcept -> OpenTTDCargoStored {
		Vehicle *v = VehicleOf(handle);
		return {
			v->cargo.StoredCount(), RustVehicleCargoCap(handle)
		};
	},
	.cargo_reservation = [](OpenTTDVehicle *handle) noexcept -> OpenTTDCargoReservation {
		Vehicle *v = VehicleOf(handle);
		return {
			static_cast<uint64_t>(EngInfo(v->engine_type)->refit_mask), v->cargo.ReservedCount(), RustVehicleCargoCap(handle), v->cargo_type
		};
	},
	.cargo_capacity = [](OpenTTDVehicle *handle) noexcept -> OpenTTDCargoCapacity {
		Vehicle *v = VehicleOf(handle);
		return {
			RustVehicleCargoOwner(v), RustVehicleCargoCap(handle), v->cargo_type
		};
	},
	.vehicle_type = GetRustVehicleServices().vehicle_type,
	.cargo_type = RustVehicleCargoType,
	.cargo_cap = RustVehicleCargoCap,
	.stored_count = [](OpenTTDVehicle *handle) noexcept -> uint32_t {
		Vehicle *v = VehicleOf(handle); return v->cargo.StoredCount();
	},
	.unload_count = [](OpenTTDVehicle *handle) noexcept -> uint32_t {
		Vehicle *v = VehicleOf(handle); return v->cargo.UnloadCount();
	},
	.deliver_count = [](OpenTTDVehicle *handle) noexcept -> uint32_t {
		Vehicle *v = VehicleOf(handle);
		return v->cargo.ActionCount(VehicleCargoList::MTA_DELIVER);
	},
	.transfer_count = [](OpenTTDVehicle *handle) noexcept -> uint32_t {
		Vehicle *v = VehicleOf(handle);
		return v->cargo.ActionCount(VehicleCargoList::MTA_TRANSFER);
	},
	.load_count = [](OpenTTDVehicle *handle) noexcept -> uint32_t {
		Vehicle *v = VehicleOf(handle);
		return v->cargo.ActionCount(VehicleCargoList::MTA_LOAD);
	},
	.unload_type = RustVehicleRead<uint8_t, RustVehicleOrderUnloadType>,
	.has_orders = [](OpenTTDVehicle *handle) noexcept -> bool {
		Vehicle *v = VehicleOf(handle); return v->orders != nullptr;
	},
	.load_unload_ticks = RustVehicleLoadUnloadTicks,
	.status = GetRustVehicleServices().status,
	.has_articulated_part = RustVehicleHasArticulated,
	.is_articulated_part = RustVehicleRead<bool, RustVehicleArticulated>,
	.is_rear_dualheaded = [](OpenTTDVehicle *handle) noexcept -> bool {
		Vehicle *v = VehicleOf(handle);
		return Train::From(v)->IsRearDualheaded();
	},
	.is_normal_aircraft = [](OpenTTDVehicle *handle) noexcept -> bool {
		Vehicle *v = VehicleOf(handle);
		return Aircraft::From(v)->IsNormalAircraft();
	},
	.is_multiheaded = [](OpenTTDVehicle *handle) noexcept -> bool {
		Vehicle *v = VehicleOf(handle); return Train::From(v)->IsMultiheaded();
	},
	.cargo_multiplier = [](OpenTTDVehicle *handle) noexcept -> uint16_t {
		Vehicle *v = VehicleOf(handle);
		return CargoSpec::Get(v->cargo_type)->multiplier;
	},
	.cached_total_length = [](OpenTTDVehicle *handle) noexcept -> uint16_t {
		Vehicle *v = VehicleOf(handle);
		return RustGroundCachedTotalLength(v->GetGroundVehicleCache());
	},
	.full_load_order = [](OpenTTDVehicle *handle) noexcept -> bool {
		Vehicle *v = VehicleOf(handle); return v->current_order.IsFullLoadOrder();
	},
	.vehicle_owner = RustVehicleOwnerOf,
	.passenger_cargo = [](OpenTTDVehicle *handle) noexcept -> bool {
		Vehicle *v = VehicleOf(handle);
		return IsCargoInClass(v->cargo_type, CargoClass::Passengers);
	},
	.station_length_penalty = []() noexcept -> bool {
		return _settings_game.order.station_length_loading_penalty;
	},
	.current_company = GetRustCompanyFinanceServices().current_company,
	.vehicle = &GetRustVehicleServices(),
	.total_count = [](OpenTTDVehicle *handle) noexcept -> uint32_t {
		Vehicle *v = VehicleOf(handle); return v->cargo.TotalCount();
	},
	.order_time = GetRustVehicleServices().order_time,
	.refit_mask = [](OpenTTDVehicle *handle) noexcept -> uint64_t {
		Vehicle *v = VehicleOf(handle);
		return static_cast<uint64_t>(EngInfo(v->engine_type)->refit_mask);
	},
	.set_cargo_unloading = [](OpenTTDVehicle *handle, bool value) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		front->vehicle_flags.Set(VehicleFlag::CargoUnloading, value);
	},
	.set_loading_finished = [](OpenTTDVehicle *handle, bool value) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		front->vehicle_flags.Set(VehicleFlag::LoadingFinished, value);
	},
	.set_stop = [](OpenTTDVehicle *handle, bool value) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		front->vehicle_flags.Set(VehicleFlag::StopLoading, value);
	},
	.set_load_ticks = [](OpenTTDVehicle *handle, uint16_t value) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		front->load_unload_ticks = value;
	},
	.set_speed = GetRustVehicleServices().write_speed,
	.subtract_profit = [](OpenTTDVehicle *handle, int64_t value) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		front->profit_this_year -= value;
	},
	.unknown_load_result = [](OpenTTDVehicle *handle, uint16_t value) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		ErrorUnknownCallbackResult(front->GetEngine()->GetGRFID(), CBID_VEHICLE_LOAD_AMOUNT, value);
	},
	.vehicle_dirty = [](OpenTTDVehicle *handle) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		SetWindowDirty(GetWindowClassForVehicleType(front->type), front->owner);
		SetWindowDirty(WC_VEHICLE_DETAILS, front->index);
		front->MarkDirty();
	},
	.indicator = [](OpenTTDVehicle *handle) noexcept -> bool {
		const Vehicle *front = VehicleOf(handle);
		return _game_mode != GM_MENU && (_settings_client.gui.loading_indicators > (uint)(front->owner != _local_company && _local_company != COMPANY_SPECTATOR));
	},
	.show_indicator = [](OpenTTDVehicle *handle, uint8_t percent, uint32_t percent_up_down) noexcept -> void {
		Vehicle *front = VehicleOf(handle);
		if (front->fill_percent_te_id == INVALID_TE_ID) {
			front->fill_percent_te_id = ShowFillingPercent(front->x_pos, front->y_pos, front->z_pos + 20, percent, percent_up_down);
		} else {
			UpdateFillingPercent(front->fill_percent_te_id, percent, percent_up_down);
		}
	},
	.set_current_company = GetRustCompanyFinanceServices().set_current_company,
	.next_vehicle = GetRustVehicleServices().next,
	.next_articulated = RustVehicleNextArticulated,
	.other_multiheaded = [](OpenTTDVehicle *handle) noexcept -> OpenTTDVehicle * {
		Vehicle *v = VehicleOf(handle);
		return RustVehicle(Train::From(v)->other_multiheaded_part);
	},
	.first_engine_part = [](OpenTTDVehicle *handle) noexcept -> OpenTTDVehicle * {
		Vehicle *v = VehicleOf(handle); return RustVehicle(v->GetFirstEnginePart());
	},
	.first_vehicle = [](OpenTTDVehicle *handle) noexcept -> OpenTTDVehicle * {
		Vehicle *v = VehicleOf(handle); return RustVehicle(v->First());
	},
	.previous_vehicle = [](OpenTTDVehicle *handle) noexcept -> OpenTTDVehicle * {
		Vehicle *v = VehicleOf(handle); return RustVehicle(v->Previous());
	},
	.with_next_stations = [](OpenTTDVehicle *handle, void *context, OpenTTDStationScope body) noexcept {
		std::vector<StationID> next;
		VehicleOf(handle)->GetNextStoppingStation(next);
		body(context, &next);
	},
	.stage = [](OpenTTDStation *station, OpenTTDVehicle *vehicle, uint8_t unload, void *next_stations, OpenTTDNativeCargoPayment *payment) noexcept -> void {
		Station *st = static_cast<Station *>(station);
		Vehicle *v = VehicleOf(vehicle);
		GoodsEntry &ge = st->goods[v->cargo_type];
		std::span<const StationID> next = *static_cast<std::vector<StationID> *>(next_stations);
		v->cargo.Stage(ge.status.Test(GoodsEntry::State::Acceptance), st->index, next, static_cast<OrderUnloadType>(unload), &ge, v->cargo_type, static_cast<CargoPayment *>(payment), v->GetCargoTile());
	},
	.return_cargo = [](OpenTTDStation *station, OpenTTDVehicle *vehicle, uint32_t amount) noexcept -> void {
		Station *st = static_cast<Station *>(station);
		Vehicle *v = VehicleOf(vehicle);
		GoodsEntry &ge = st->goods[v->cargo_type];
		v->cargo.Return(amount, &ge.GetOrCreateData().cargo, StationID::Invalid(), v->GetCargoTile());
	},
	.unload_cargo = [](OpenTTDStation *station, OpenTTDVehicle *vehicle, uint32_t amount, OpenTTDNativeCargoPayment *payment) noexcept -> uint32_t {
		Station *st = static_cast<Station *>(station);
		Vehicle *v = VehicleOf(vehicle);
		GoodsEntry &ge = st->goods[v->cargo_type];
		return v->cargo.Unload(amount, &ge.GetOrCreateData().cargo, v->cargo_type, static_cast<CargoPayment *>(payment), v->GetCargoTile());
	},
	.load_cargo = [](OpenTTDStation *station, OpenTTDVehicle *vehicle, uint32_t amount, void *next_stations) noexcept -> uint32_t {
		Station *st = static_cast<Station *>(station);
		Vehicle *v = VehicleOf(vehicle);
		GoodsEntry &ge = st->goods[v->cargo_type];
		std::span<const StationID> next = *static_cast<std::vector<StationID> *>(next_stations);
		return ge.GetOrCreateData().cargo.Load(amount, &v->cargo, next, v->GetCargoTile());
	},
	.reserve_cargo = [](OpenTTDStation *station, OpenTTDVehicle *vehicle, uint32_t amount, void *next_stations) noexcept -> void {
		Station *st = static_cast<Station *>(station);
		Vehicle *v = VehicleOf(vehicle);
		GoodsEntry &ge = st->goods[v->cargo_type];
		std::span<const StationID> next = *static_cast<std::vector<StationID> *>(next_stations);
		ge.GetOrCreateData().cargo.Reserve(amount, &v->cargo, next, v->GetCargoTile());
	},
	.deliver_to_transfer = [](OpenTTDVehicle *vehicle, uint32_t amount) noexcept -> void {
		VehicleOf(vehicle)->cargo.Reassign<VehicleCargoList::MTA_DELIVER, VehicleCargoList::MTA_TRANSFER>(amount);
	},
	.deliver_to_keep = [](OpenTTDVehicle *vehicle, uint32_t amount) noexcept -> void {
		VehicleOf(vehicle)->cargo.Reassign<VehicleCargoList::MTA_DELIVER, VehicleCargoList::MTA_KEEP>(amount);
	},
	.has_cargo_for = [](OpenTTDStation *station, uint8_t cargo, void *next_stations) noexcept -> bool {
		GoodsEntry &ge = static_cast<Station *>(station)->goods[cargo];
		return ge.HasData() && ge.GetData().cargo.HasCargoFor(*static_cast<std::vector<StationID> *>(next_stations));
	},
	.vehicle_tile = GetRustVehicleServices().tile,
	.station_cargo_total = [](OpenTTDStation *station, uint8_t cargo) noexcept -> uint32_t {
		return static_cast<Station *>(station)->goods[cargo].GetData().cargo.TotalCount();
	},


	.get_payment = [](OpenTTDVehicle *handle) noexcept -> OpenTTDNativeCargoPayment * {
		return VehicleOf(handle)->cargo_payment;
	},
	.create_payment = [](OpenTTDVehicle *handle) noexcept -> OpenTTDNativeCargoPayment * {
		Vehicle *v = VehicleOf(handle);
		assert(v->cargo_payment == nullptr);
		static_assert(CargoPaymentPool::MAX_SIZE == VehiclePool::MAX_SIZE);
		assert(CargoPayment::CanAllocateItem());
		v->cargo_payment = new CargoPayment(v);
		return v->cargo_payment;
	},
	.delete_payment = [](OpenTTDNativeCargoPayment *handle) noexcept -> void {
		delete static_cast<CargoPayment *>(handle);
	},
	.flush_delivery = []() noexcept -> void {
		openttd_rust_cargo_delivery_flush(GetCargoDeliveryState(), &_cargo_services);
	},
	.station_dirty = [](OpenTTDStation *station) noexcept -> void {
		Station *st = static_cast<Station *>(station);
		st->MarkTilesDirty(true);
		SetWindowDirty(WC_STATION_VIEW, st->index);
		SetWindowDirty(WC_STATION_LIST, st->owner);
	},
};

/**
 * Prepare the vehicle to be unloaded.
 * @param front_v the vehicle to be unloaded
 */

/**
 * Load/unload the vehicles in this station according to the order
 * they entered.
 * @param st the station to do the loading/unloading for
 */
const OpenTTDStationLoading &GetRustStationLoading() noexcept { return _rust_station_loading; }

void LoadUnloadStation(Station *st)
{
	CommandCost cost, temporary;
	openttd_rust_station_load(st->rust_service.get(), &GetRustStationServices(), &_rust_station_loading, &GetRustVehicleVisits(), RustCommandCost(&cost), RustCommandCost(&temporary));
}

OpenTTDEconomyMonth RustEconomyClockMonth() noexcept
{
	return {.month = TimerGameEconomy::month, .infinite_money = _settings_game.difficulty.infinite_money,
		.maintenance = _settings_game.economy.infrastructure_maintenance, .fluctuating = _settings_game.difficulty.economy};
}

/**
 * Every calendar month update of inflation.
 */
static const IntervalTimer<TimerGameCalendar> _calendar_inflation_monthly({TimerGameCalendar::MONTH, TimerGameCalendar::Priority::COMPANY}, RustClockCallback{openttd_rust_clock_economy_calendar_month, &GetRustClockCompanyServices()});


/**
 * Every economy month update of company economic data, plus economy fluctuations.
 */
static const IntervalTimer<TimerGameEconomy> _economy_companies_monthly({ TimerGameEconomy::MONTH, TimerGameEconomy::Priority::COMPANY }, RustClockCallback{openttd_rust_clock_economy_month, &GetRustClockCompanyServices()});



/**
 * Buy up another company.
 * When a competing company is gone bankrupt you get the chance to purchase
 * that company.
 * @todo currently this only works for AI companies
 * @param flags type of operation
 * @param target_company company to buy up
 * @param hostile_takeover whether to buy up the company even if it is not bankrupt
 * @return the cost of this operation or an error
 */
CommandCost CmdBuyCompany(DoCommandFlags flags, CompanyID target_company, bool hostile_takeover)
{
	RustTileOwnerContext ownership;
	return RustCompanyCost(openttd_rust_company_buy(&GetRustCompanyServices(), &ownership.view, target_company.base(), hostile_takeover, flags.Test(DoCommandFlag::Execute)));
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDCargoServices.hpp"
#include "abi-visit-OpenTTDStationLoading.hpp"
bool CheckAbi_economy()
{
	return AbiServicesComplete(_cargo_services) &&
		AbiServicesComplete(_rust_station_loading);
}
