/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "command_cost_services.h"
#include "services_ffi.h"
#include "company_services.h"
#include "engine_services.h"
#include "vehicle_services.h"

/* Named shared services and full native CommandCost storage. All callbacks are
 * noexcept, retain original move/AddCost behavior, and permit ordinary reentry.
 * No Rust state access spans them; environmental failures terminate. */
struct NativeFleetCosts {
	CommandCost result, replace, build, copy, temporary;
	SavedRandomSeeds seeds;
	OpenTTDFleetCosts View() { return {&result, &replace, &build, &copy, &temporary, &seeds}; }
};
static void FleetTransaction_cost_zero(void *out) noexcept { GetRustCommandCostServices().zero(RustCommandCost(static_cast<CommandCost *>(out))); }
static void FleetTransaction_cost_vehicles(void *out) noexcept { GetRustCommandCostServices().expense_money(RustCommandCost(static_cast<CommandCost *>(out)), EXPENSES_NEW_VEHICLES, 0); }
static void FleetTransaction_cost_error(void *out, uint32_t error) noexcept { GetRustCommandCostServices().error(RustCommandCost(static_cast<CommandCost *>(out)), error); }
static void FleetTransaction_cost_add(void *out, void *item) noexcept { GetRustCommandCostServices().add(RustCommandCost(static_cast<CommandCost *>(out)), RustCommandCost(static_cast<CommandCost *>(item))); }
static void FleetTransaction_cost_move(void *out, void *item) noexcept { GetRustCommandCostServices().move(RustCommandCost(static_cast<CommandCost *>(out)), RustCommandCost(static_cast<CommandCost *>(item))); }
static void FleetTransaction_cost_amount(void *out, int64_t amount) noexcept { GetRustCommandCostServices().amount(RustCommandCost(static_cast<CommandCost *>(out)), amount); }
static bool FleetTransaction_success(void *out) noexcept { return GetRustCommandCostServices().success(RustCommandCost(static_cast<CommandCost *>(out))); }
static uint32_t FleetTransaction_error(void *out) noexcept { return GetRustCommandCostServices().error_message(RustCommandCost(static_cast<CommandCost *>(out))); }
static int64_t FleetTransaction_money(void *out) noexcept { return GetRustCommandCostServices().money(RustCommandCost(static_cast<CommandCost *>(out))); }
static bool FleetTransaction_rear(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return Train::From(v)->IsRearDualheaded(); }
static bool FleetTransaction_articulated(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->IsArticulatedPart(); }
static bool FleetTransaction_crashed(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->vehstatus.Test(VehState::Crashed); }
static bool FleetTransaction_stopped(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->vehstatus.Test(VehState::Stopped); }
static bool FleetTransaction_chain_depot(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->IsChainInDepot(); }
static void * FleetTransaction_next_unit(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return Train::From(v)->GetNextUnit(); }
static void * FleetTransaction_prev_unit(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return Train::From(v)->GetPrevUnit(); }
static uint16_t FleetTransaction_length(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return Train::From(v)->gcache.cached_total_length; }
static bool FleetTransaction_flipped(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return Train::From(v)->GetTrainFlags().Test(VehicleRailFlag::Flipped); }
static uint8_t FleetTransaction_cargo_type(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->cargo_type; }
static bool FleetTransaction_stopped_in_depot(void *shell) noexcept { return static_cast<Vehicle *>(shell)->IsStoppedInDepot(); }
static uint8_t FleetTransaction_max_length() noexcept { return _settings_game.vehicle.max_train_length; }
/* Original assertion sites; Rust evaluates their conditions only when assertions is set. */
static void FleetTransaction_check([[maybe_unused]] bool condition) noexcept { assert(condition); }
#if !defined(NDEBUG) || defined(WITH_ASSERT)
static constexpr bool FLEET_ASSERTIONS = true;
#else
static constexpr bool FLEET_ASSERTIONS = false;
#endif
static bool FleetTransaction_needs_renew(void *shell, bool settings) noexcept { return static_cast<Vehicle *>(shell)->NeedsAutorenewing(Company::Get(OpenTTDCurrentCompany()), settings); }
static bool FleetTransaction_engine_valid(uint16_t id) noexcept { return Engine::IsValidID(EngineID(id)); }
static bool FleetTransaction_company_valid(uint8_t id) noexcept { return GetRustSharedServices().company_valid(id); }
static bool FleetTransaction_engine_buildable(uint16_t id, uint8_t type, uint8_t company) noexcept { return IsEngineBuildable(EngineID(id), static_cast<VehicleType>(type), CompanyID(company)); }
static bool FleetTransaction_tram(uint16_t id) noexcept { return Engine::Get(EngineID(id))->info.misc_flags.Test(EngineMiscFlag::RoadIsTram); }
static uint8_t FleetTransaction_plane(uint16_t id) noexcept { return AircraftVehInfo(EngineID(id))->subtype & AIR_CTOL; }
static void FleetTransaction_refit_masks(uint16_t id, uint64_t *union_mask, uint64_t *available) noexcept { GetArticulatedRefitMasks(EngineID(id), true, union_mask, available); }
static uint64_t FleetTransaction_vehicle_cargo(void *shell, uint8_t *cargo) noexcept { return GetCargoTypesOfArticulatedVehicle(static_cast<Vehicle *>(shell), cargo); }
static uint64_t FleetTransaction_default_cargo(uint16_t id) noexcept { return GetCargoTypesOfArticulatedParts(EngineID(id)); }
static void * FleetTransaction_orders(void *shell) noexcept { return static_cast<Vehicle *>(shell)->orders; }
static size_t FleetTransaction_order_count(void *orders) noexcept { return static_cast<OrderList *>(orders)->GetOrders().size(); }
static uint8_t FleetTransaction_order_count_id(void *orders) noexcept { return static_cast<OrderList *>(orders)->GetNumOrders(); }
static void * FleetTransaction_order_at(void *orders, size_t index) noexcept { return &static_cast<OrderList *>(orders)->GetOrders()[index]; }
static bool FleetTransaction_order_refit(void *order) noexcept { return static_cast<Order *>(order)->IsRefit(); }
static bool FleetTransaction_order_auto(void *order) noexcept { return static_cast<Order *>(order)->IsAutoRefit(); }
static uint8_t FleetTransaction_order_cargo(void *order) noexcept { return static_cast<Order *>(order)->GetRefitCargo(); }
static bool FleetTransaction_local() noexcept { return IsLocalCompany(); }
static void FleetTransaction_refit_news(void *shell, int32_t order) noexcept { auto *v = static_cast<Vehicle *>(shell); VehicleID id = (v->type == VEH_TRAIN) ? RustVehicleFirst(v)->index : v->index; EncodedString headline = order != -1 ? GetEncodedString(STR_NEWS_VEHICLE_AUTORENEW_FAILED, id, STR_ERROR_AUTOREPLACE_INCOMPATIBLE_REFIT, order + 1) : GetEncodedString(STR_NEWS_VEHICLE_AUTORENEW_FAILED, id, STR_ERROR_AUTOREPLACE_INCOMPATIBLE_CARGO, CargoSpec::Get(v->cargo_type)->name); AddVehicleAdviceNewsItem(AdviceType::AutorenewFailed, std::move(headline), id); }
static void * FleetTransaction_build(void *out, void *shell, uint16_t engine) noexcept { auto *v = static_cast<Vehicle *>(shell); VehicleID id; std::tie(*static_cast<CommandCost *>(out), id, std::ignore, std::ignore, std::ignore) = Command<CMD_BUILD_VEHICLE>::Do({DoCommandFlag::Execute, DoCommandFlag::AutoReplace}, v->tile, EngineID(engine), true, INVALID_CARGO, INVALID_CLIENT_ID); return static_cast<CommandCost *>(out)->Failed() ? nullptr : Vehicle::Get(id); }
static void FleetTransaction_refit(void *out, void *shell, uint8_t cargo, uint8_t subtype) noexcept { *static_cast<CommandCost *>(out) = std::get<0>(Command<CMD_REFIT_VEHICLE>::Do(DoCommandFlag::Execute, static_cast<Vehicle *>(shell)->index, cargo, subtype, false, false, 0)); }
static uint8_t FleetTransaction_subtype(void *old, void *replacement, uint8_t cargo) noexcept { return GetBestFittingSubType(static_cast<Vehicle *>(old), static_cast<Vehicle *>(replacement), cargo); }
static bool FleetTransaction_reverse_probability(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return TestVehicleBuildProbability(v, v->engine_type, BuildProbabilityType::Reversed).has_value(); }
static void FleetTransaction_reverse(void *shell) noexcept { Command<CMD_REVERSE_TRAIN_DIRECTION>::Do(DoCommandFlag::Execute, static_cast<Vehicle *>(shell)->index, true); }
static void FleetTransaction_start_stop(void *out, void *shell, bool evaluate) noexcept { *static_cast<CommandCost *>(out) = Command<CMD_START_STOP_VEHICLE>::Do({DoCommandFlag::Execute, DoCommandFlag::AutoReplace}, static_cast<Vehicle *>(shell)->index, evaluate); }
static void FleetTransaction_move(void *out, void *shell, void *after, uint32_t flags, bool whole) noexcept { *static_cast<CommandCost *>(out) = Command<CMD_MOVE_RAIL_VEHICLE>::Do(DoCommandFlags(static_cast<uint16_t>(flags)).Set(DoCommandFlag::NoCargoCapacityCheck), static_cast<Vehicle *>(shell)->index, after == nullptr ? VehicleID::Invalid() : static_cast<Vehicle *>(after)->index, whole); }
static void FleetTransaction_sell(void *out, void *shell, uint32_t flags) noexcept { *static_cast<CommandCost *>(out) = Command<CMD_SELL_VEHICLE>::Do(DoCommandFlags(static_cast<uint16_t>(flags)), static_cast<Vehicle *>(shell)->index, false, false, INVALID_CLIENT_ID); }
static void FleetTransaction_clone_order(void *out, void *old, void *replacement) noexcept { *static_cast<CommandCost *>(out) = Command<CMD_CLONE_ORDER>::Do(DoCommandFlag::Execute, CO_SHARE, static_cast<Vehicle *>(replacement)->index, static_cast<Vehicle *>(old)->index); }
static void FleetTransaction_copy_group(void *out, void *old, void *replacement) noexcept { *static_cast<CommandCost *>(out) = std::get<0>(Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlag::Execute, static_cast<Vehicle *>(old)->group_id, static_cast<Vehicle *>(replacement)->index, false, VehicleListIdentifier{})); }
static void FleetTransaction_copy_configuration(void *old, void *replacement) noexcept { static_cast<Vehicle *>(replacement)->CopyVehicleConfigAndStatistics(static_cast<Vehicle *>(old)); }
static void FleetTransaction_viewports(void *old, void *replacement) noexcept { ChangeVehicleViewports(static_cast<Vehicle *>(old)->index, static_cast<Vehicle *>(replacement)->index); }
static void FleetTransaction_view_window(void *old, void *replacement) noexcept { ChangeVehicleViewWindow(static_cast<Vehicle *>(old)->index, static_cast<Vehicle *>(replacement)->index); }
static void FleetTransaction_news(void *old, void *replacement) noexcept { ChangeVehicleNews(static_cast<Vehicle *>(old)->index, static_cast<Vehicle *>(replacement)->index); }
static void FleetTransaction_transfer_cargo(void *old, void *replacement, bool chain) noexcept { TransferCargo(static_cast<Vehicle *>(old), static_cast<Vehicle *>(replacement), chain); }
static void FleetTransaction_capacity(void *shell) noexcept { CheckCargoCapacity(static_cast<Vehicle *>(shell)); }
static void FleetTransaction_event(void *old, void *replacement) noexcept { auto *v = static_cast<Vehicle *>(old); AI::NewEvent(v->owner, new ScriptEventVehicleAutoReplaced(v->index, static_cast<Vehicle *>(replacement)->index)); }
static void FleetTransaction_save_rng(void *seeds) noexcept { *static_cast<SavedRandomSeeds *>(seeds) = std::bit_cast<SavedRandomSeeds>(GetRustRandomServices().save()); }
static void FleetTransaction_restore_rng(void *seeds) noexcept { GetRustRandomServices().restore(std::bit_cast<OpenTTDRandomSeeds>(*static_cast<SavedRandomSeeds *>(seeds))); }
static void FleetTransaction_rule_window(uint16_t engine, uint16_t group) noexcept { InvalidateAutoreplaceWindow(EngineID(engine), GroupID(group)); }
static const OpenTTDFleetTransactionServices _fleet_transaction_services{
	.cost_zero = FleetTransaction_cost_zero,
	.cost_vehicles = FleetTransaction_cost_vehicles,
	.cost_error = FleetTransaction_cost_error,
	.cost_add = FleetTransaction_cost_add,
	.cost_move = FleetTransaction_cost_move,
	.cost_amount = FleetTransaction_cost_amount,
	.success = FleetTransaction_success,
	.error = FleetTransaction_error,
	.money = FleetTransaction_money,
	.commands = &GetRustCommandServices(),
	.rear = FleetTransaction_rear,
	.articulated = FleetTransaction_articulated,
	.crashed = FleetTransaction_crashed,
	.stopped = FleetTransaction_stopped,
	.chain_depot = FleetTransaction_chain_depot,
	.first = RustVehicleFirstOpaque,
	.next_unit = FleetTransaction_next_unit,
	.prev_unit = FleetTransaction_prev_unit,
	.length = FleetTransaction_length,
	.flipped = FleetTransaction_flipped,
	.cargo_type = FleetTransaction_cargo_type,
	.can_carry = FleetTransaction_can_carry,
	.stopped_in_depot = FleetTransaction_stopped_in_depot,
	.max_length = FleetTransaction_max_length,
	.check = FleetTransaction_check,
	.needs_renew = FleetTransaction_needs_renew,
	.engine_valid = FleetTransaction_engine_valid,
	.company_valid = FleetTransaction_company_valid,
	.engine_buildable = FleetTransaction_engine_buildable,
	.rail_compatible = FleetTransaction_rail_compatible,
	.road_powered = FleetTransaction_road_powered,
	.wagon = FleetTransaction_wagon,
	.tram = FleetTransaction_tram,
	.plane = FleetTransaction_plane,
	.refit_mask = FleetTransaction_refit_mask,
	.refit_masks = FleetTransaction_refit_masks,
	.vehicle_cargo = FleetTransaction_vehicle_cargo,
	.default_cargo = FleetTransaction_default_cargo,
	.orders = FleetTransaction_orders,
	.order_count = FleetTransaction_order_count,
	.order_count_id = FleetTransaction_order_count_id,
	.order_at = FleetTransaction_order_at,
	.order_refit = FleetTransaction_order_refit,
	.order_auto = FleetTransaction_order_auto,
	.order_cargo = FleetTransaction_order_cargo,
	.local = FleetTransaction_local,
	.refit_news = FleetTransaction_refit_news,
	.build = FleetTransaction_build,
	.refit = FleetTransaction_refit,
	.subtype = FleetTransaction_subtype,
	.reverse_probability = FleetTransaction_reverse_probability,
	.reverse = FleetTransaction_reverse,
	.start_stop = FleetTransaction_start_stop,
	.move = FleetTransaction_move,
	.sell = FleetTransaction_sell,
	.clone_order = FleetTransaction_clone_order,
	.copy_group = FleetTransaction_copy_group,
	.copy_configuration = FleetTransaction_copy_configuration,
	.viewports = FleetTransaction_viewports,
	.view_window = FleetTransaction_view_window,
	.news = FleetTransaction_news,
	.transfer_cargo = FleetTransaction_transfer_cargo,
	.capacity = FleetTransaction_capacity,
	.event = FleetTransaction_event,
	.save_rng = FleetTransaction_save_rng,
	.restore_rng = FleetTransaction_restore_rng,
	.rule_window = FleetTransaction_rule_window,
	.assertions = FLEET_ASSERTIONS,
	.unavailable = STR_ERROR_RAIL_VEHICLE_NOT_AVAILABLE,
	.too_long = STR_ERROR_TRAIN_TOO_LONG,
	.too_long_replacement = STR_ERROR_TRAIN_TOO_LONG_AFTER_REPLACEMENT,
	.nothing = STR_ERROR_AUTOREPLACE_NOTHING_TO_DO,
};
