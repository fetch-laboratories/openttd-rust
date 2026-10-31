/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine.cpp Base for all engine handling. */

#include "stdafx.h"
#include "core/container_func.hpp"
#include "company_func.h"
#include "command_func.h"
#include "news_func.h"
#include "aircraft.h"
#include "newgrf.h"
#include "newgrf_engine.h"
#include "strings_func.h"
#include "core/random_func.hpp"
#include "window_func.h"
#include "autoreplace_gui.h"
#include "string_func.h"
#include "ai/ai.hpp"
#include "core/pool_func.hpp"
#include "engine_gui.h"
#include "engine_func.h"
#include "engine_base.h"
#include "company_base.h"
#include "vehicle_func.h"
#include "articulated_vehicles.h"
#include "error.h"
#include "engine_base.h"
#include "timer/timer.h"
#include "timer/timer_game_tick.h"
#include "timer/timer_game_calendar.h"

#include "table/strings.h"
#include "table/engines.h"

void ShowEnginePreviewWindow(EngineID engine);
#include "rust/engine_adapter.hpp"
#include "rust/engine_capabilities_services.hpp"

#include "rust/engine_loading_ffi.h"

#include "rust/clock_ffi.h"

#include "safeguards.h"

EnginePool _engine_pool("Engine");
INSTANTIATE_POOL_METHODS(Engine)

EngineOverrideManager _engine_mngr;

/**
 * Year that engine aging stops. Engines will not reduce in reliability
 * and no more engines will be introduced
 */

/** Number of engines of each vehicle type in original engine data */
const uint8_t _engine_counts[4] = {
	lengthof(_orig_rail_vehicle_info),
	lengthof(_orig_road_vehicle_info),
	lengthof(_orig_ship_vehicle_info),
	lengthof(_orig_aircraft_vehicle_info),
};

/** Offset of the first engine of each vehicle type in original engine data */
const uint8_t _engine_offsets[4] = {
	0,
	lengthof(_orig_rail_vehicle_info),
	lengthof(_orig_rail_vehicle_info) + lengthof(_orig_road_vehicle_info),
	lengthof(_orig_rail_vehicle_info) + lengthof(_orig_road_vehicle_info) + lengthof(_orig_ship_vehicle_info),
};

static_assert(lengthof(_orig_rail_vehicle_info) + lengthof(_orig_road_vehicle_info) + lengthof(_orig_ship_vehicle_info) + lengthof(_orig_aircraft_vehicle_info) == lengthof(_orig_engine_info));

Engine::Engine(VehicleType type, uint16_t local_id)
{
	this->type = type;
	this->grf_prop.local_id = local_id;
	this->list_position = local_id;
	this->preview_company = CompanyID::Invalid();
	this->display_last_variant = EngineID::Invalid();

	/* Check if this base engine is within the original engine data range */
	if (local_id >= _engine_counts[type]) {
		/* Initialise default type-specific information. */
		switch (type) {
			case VEH_TRAIN: this->vehicle_info.emplace<RailVehicleInfo>(); break;
			case VEH_ROAD: this->vehicle_info.emplace<RoadVehicleInfo>(); break;
			case VEH_SHIP: this->vehicle_info.emplace<ShipVehicleInfo>(); break;
			case VEH_AIRCRAFT: this->vehicle_info.emplace<AircraftVehicleInfo>(); break;
			default: break;
		}
		/* Set model life to maximum to make wagons available */
		this->info.base_life = TimerGameCalendar::Year{0xFF};
		/* Aircraft must have CT_INVALID as default, as there is no property */
		this->info.cargo_type = INVALID_CARGO;
		this->info.cargo_label = (type == VEH_AIRCRAFT) ? CT_INVALID : CT_PASSENGERS;
		/* Set cargo aging period to the default value. */
		this->info.cargo_age_period = Ticks::CARGO_AGING_TICKS;
		/* Not a variant */
		this->info.variant_id = EngineID::Invalid();
		return;
	}

	/* Copy the original engine info for this slot */
	this->info = _orig_engine_info[_engine_offsets[type] + local_id];

	/* Copy the original engine data for this slot */
	switch (type) {
		default: NOT_REACHED();

		case VEH_TRAIN: {
			RailVehicleInfo &rvi = this->vehicle_info.emplace<RailVehicleInfo>(_orig_rail_vehicle_info[local_id]);
			this->original_image_index = rvi.image_index;
			this->info.string_id = STR_VEHICLE_NAME_TRAIN_ENGINE_RAIL_KIRBY_PAUL_TANK_STEAM + local_id;

			/* Set the default model life of original wagons to "infinite" */
			if (rvi.railveh_type == RAILVEH_WAGON) this->info.base_life = TimerGameCalendar::Year{0xFF};

			break;
		}

		case VEH_ROAD: {
			RoadVehicleInfo &rvi = this->vehicle_info.emplace<RoadVehicleInfo>(_orig_road_vehicle_info[local_id]);
			this->original_image_index = rvi.image_index;
			this->info.string_id = STR_VEHICLE_NAME_ROAD_VEHICLE_MPS_REGAL_BUS + local_id;
			break;
		}

		case VEH_SHIP: {
			ShipVehicleInfo &svi = this->vehicle_info.emplace<ShipVehicleInfo>(_orig_ship_vehicle_info[local_id]);
			this->original_image_index = svi.image_index;
			this->info.string_id = STR_VEHICLE_NAME_SHIP_MPS_OIL_TANKER + local_id;
			break;
		}

		case VEH_AIRCRAFT: {
			AircraftVehicleInfo &avi = this->vehicle_info.emplace<AircraftVehicleInfo>(_orig_aircraft_vehicle_info[local_id]);
			this->original_image_index = avi.image_index;
			this->info.string_id = STR_VEHICLE_NAME_AIRCRAFT_SAMPSON_U52 + local_id;
			break;
		}
	}
}

/**
 * Checks whether the engine is a valid (non-articulated part of an) engine.
 * @return true if enabled
 */
bool Engine::IsEnabled() const
{
	return this->info.string_id != STR_NEWGRF_INVALID_ENGINE && this->info.climates.Test(_settings_game.game_creation.landscape);
}

/**
 * Retrieve the GRF ID of the NewGRF the engine is tied to.
 * This is the GRF providing the Action 3.
 * @return GRF ID of the associated NewGRF.
 */
uint32_t Engine::GetGRFID() const
{
	const GRFFile *file = this->GetGRF();
	return file == nullptr ? 0 : file->grfid;
}

/**
 * Determines whether an engine can carry something.
 * A vehicle cannot carry anything if its capacity is zero, or none of the possible cargoes is available in the climate.
 * @return true if the vehicle can carry something.
 */
bool Engine::CanCarryCargo() const
{
	return openttd_rust_engine_can_carry(&GetRustEngineCapabilitiesServices(), RustEngine(this));
}


/**
 * Determines capacity of a given vehicle from scratch.
 * For aircraft the main capacity is determined. Mail might be present as well.
 * @param v Vehicle of interest; nullptr in purchase list
 * @param mail_capacity returns secondary cargo (mail) capacity of aircraft
 * @return Capacity
 */
uint Engine::DetermineCapacity(const Vehicle *v, uint16_t *mail_capacity) const
{
	return openttd_rust_engine_capacity(&GetRustEngineCapabilitiesServices(), RustEngine(this), v, mail_capacity);
}

/**
 * Return how much the running costs of this engine are.
 * @return Yearly running cost of the engine.
 */
Money Engine::GetRunningCost() const
{
	Price base_price;
	uint cost_factor;
	switch (this->type) {
		case VEH_ROAD:
			base_price = this->VehInfo<RoadVehicleInfo>().running_cost_class;
			if (base_price == INVALID_PRICE) return 0;
			cost_factor = GetEngineProperty(this->index, PROP_ROADVEH_RUNNING_COST_FACTOR, this->VehInfo<RoadVehicleInfo>().running_cost);
			break;

		case VEH_TRAIN:
			base_price = this->VehInfo<RailVehicleInfo>().running_cost_class;
			if (base_price == INVALID_PRICE) return 0;
			cost_factor = GetEngineProperty(this->index, PROP_TRAIN_RUNNING_COST_FACTOR, this->VehInfo<RailVehicleInfo>().running_cost);
			break;

		case VEH_SHIP:
			base_price = PR_RUNNING_SHIP;
			cost_factor = GetEngineProperty(this->index, PROP_SHIP_RUNNING_COST_FACTOR, this->VehInfo<ShipVehicleInfo>().running_cost);
			break;

		case VEH_AIRCRAFT:
			base_price = PR_RUNNING_AIRCRAFT;
			cost_factor = GetEngineProperty(this->index, PROP_AIRCRAFT_RUNNING_COST_FACTOR, this->VehInfo<AircraftVehicleInfo>().running_cost);
			break;

		default: NOT_REACHED();
	}

	return GetPrice(base_price, cost_factor, this->GetGRF(), -8);
}

/**
 * Return how much a new engine costs.
 * @return Cost of the engine.
 */
Money Engine::GetCost() const
{
	Price base_price;
	uint cost_factor;
	switch (this->type) {
		case VEH_ROAD:
			base_price = PR_BUILD_VEHICLE_ROAD;
			cost_factor = GetEngineProperty(this->index, PROP_ROADVEH_COST_FACTOR, this->VehInfo<RoadVehicleInfo>().cost_factor);
			break;

		case VEH_TRAIN:
			if (this->VehInfo<RailVehicleInfo>().railveh_type == RAILVEH_WAGON) {
				base_price = PR_BUILD_VEHICLE_WAGON;
				cost_factor = GetEngineProperty(this->index, PROP_TRAIN_COST_FACTOR, this->VehInfo<RailVehicleInfo>().cost_factor);
			} else {
				base_price = PR_BUILD_VEHICLE_TRAIN;
				cost_factor = GetEngineProperty(this->index, PROP_TRAIN_COST_FACTOR, this->VehInfo<RailVehicleInfo>().cost_factor);
			}
			break;

		case VEH_SHIP:
			base_price = PR_BUILD_VEHICLE_SHIP;
			cost_factor = GetEngineProperty(this->index, PROP_SHIP_COST_FACTOR, this->VehInfo<ShipVehicleInfo>().cost_factor);
			break;

		case VEH_AIRCRAFT:
			base_price = PR_BUILD_VEHICLE_AIRCRAFT;
			cost_factor = GetEngineProperty(this->index, PROP_AIRCRAFT_COST_FACTOR, this->VehInfo<AircraftVehicleInfo>().cost_factor);
			break;

		default: NOT_REACHED();
	}

	return GetPrice(base_price, cost_factor, this->GetGRF(), -8);
}

/**
 * Returns max speed of the engine for display purposes
 * @return max speed in km-ish/h
 */
uint Engine::GetDisplayMaxSpeed() const
{
	switch (this->type) {
		case VEH_TRAIN:
			return GetEngineProperty(this->index, PROP_TRAIN_SPEED, this->VehInfo<RailVehicleInfo>().max_speed);

		case VEH_ROAD: {
			uint max_speed = GetEngineProperty(this->index, PROP_ROADVEH_SPEED, 0);
			return (max_speed != 0) ? max_speed * 2 : this->VehInfo<RoadVehicleInfo>().max_speed / 2;
		}

		case VEH_SHIP:
			return GetEngineProperty(this->index, PROP_SHIP_SPEED, this->VehInfo<ShipVehicleInfo>().max_speed) / 2;

		case VEH_AIRCRAFT: {
			uint max_speed = GetEngineProperty(this->index, PROP_AIRCRAFT_SPEED, 0);
			if (max_speed != 0) {
				return (max_speed * 128) / 10;
			}
			return this->VehInfo<AircraftVehicleInfo>().max_speed;
		}

		default: NOT_REACHED();
	}
}

/**
 * Returns the power of the engine for display
 * and sorting purposes.
 * Only trains and road vehicles have power
 * @return power in display units hp
 */
uint Engine::GetPower() const
{
	/* Only trains and road vehicles have 'power'. */
	switch (this->type) {
		case VEH_TRAIN:
			return GetEngineProperty(this->index, PROP_TRAIN_POWER, this->VehInfo<RailVehicleInfo>().power);
		case VEH_ROAD:
			return GetEngineProperty(this->index, PROP_ROADVEH_POWER, this->VehInfo<RoadVehicleInfo>().power) * 10;

		default: NOT_REACHED();
	}
}

/**
 * Returns the weight of the engine for display purposes.
 * For dual-headed train-engines this is the weight of both heads
 * @return weight in display units metric tons
 */
uint Engine::GetDisplayWeight() const
{
	/* Only trains and road vehicles have 'weight'. */
	switch (this->type) {
		case VEH_TRAIN:
			return GetEngineProperty(this->index, PROP_TRAIN_WEIGHT, this->VehInfo<RailVehicleInfo>().weight) << (this->VehInfo<RailVehicleInfo>().railveh_type == RAILVEH_MULTIHEAD ? 1 : 0);
		case VEH_ROAD:
			return GetEngineProperty(this->index, PROP_ROADVEH_WEIGHT, this->VehInfo<RoadVehicleInfo>().weight) / 4;

		default: NOT_REACHED();
	}
}

/**
 * Returns the tractive effort of the engine for display purposes.
 * For dual-headed train-engines this is the tractive effort of both heads
 * @return tractive effort in display units kN
 */
uint Engine::GetDisplayMaxTractiveEffort() const
{
	/* Only trains and road vehicles have 'tractive effort'. */
	switch (this->type) {
		case VEH_TRAIN:
			return (GROUND_ACCELERATION * this->GetDisplayWeight() * GetEngineProperty(this->index, PROP_TRAIN_TRACTIVE_EFFORT, this->VehInfo<RailVehicleInfo>().tractive_effort)) / 256;
		case VEH_ROAD:
			return (GROUND_ACCELERATION * this->GetDisplayWeight() * GetEngineProperty(this->index, PROP_ROADVEH_TRACTIVE_EFFORT, this->VehInfo<RoadVehicleInfo>().tractive_effort)) / 256;

		default: NOT_REACHED();
	}
}

/**
 * Returns the vehicle's (not model's!) life length in days.
 * @return the life length
 */
TimerGameCalendar::Date Engine::GetLifeLengthInDays() const
{
	/* Assume leap years; this gives the player a bit more than the given amount of years, but never less. */
	return TimerGameCalendar::Date{(this->info.lifelength + _settings_game.vehicle.extend_vehicle_life).base() * CalendarTime::DAYS_IN_LEAP_YEAR};
}

/**
 * Get the range of an aircraft type.
 * @return Range of the aircraft type in tiles or 0 if unlimited range.
 */
uint16_t Engine::GetRange() const
{
	switch (this->type) {
		case VEH_AIRCRAFT:
			return GetEngineProperty(this->index, PROP_AIRCRAFT_RANGE, this->VehInfo<AircraftVehicleInfo>().max_range);

		default: NOT_REACHED();
	}
}

/**
 * Get the name of the aircraft type for display purposes.
 * @return Aircraft type string.
 */
StringID Engine::GetAircraftTypeText() const
{
	switch (this->type) {
		case VEH_AIRCRAFT:
			switch (this->VehInfo<AircraftVehicleInfo>().subtype) {
				case AIR_HELI: return STR_LIVERY_HELICOPTER;
				case AIR_CTOL: return STR_LIVERY_SMALL_PLANE;
				case AIR_CTOL | AIR_FAST: return STR_LIVERY_LARGE_PLANE;
				default: NOT_REACHED();
			}

		default: NOT_REACHED();
	}
}

/**
 * Check whether the engine variant chain is hidden in the GUI for the given company.
 * @param c Company to check.
 * @return \c true iff the engine variant chain is hidden in the GUI for the given company.
 */
bool Engine::IsVariantHidden(CompanyID c) const
{
	return openttd_rust_engine_variant_hidden(&EngineServices(), EngineRef(this), c.base());
}

/**
 * Initializes the #EngineOverrideManager with the default engines.
 */
void EngineOverrideManager::ResetToDefaultMapping()
{
	openttd_rust_engine_mapping_defaults(_engine_counts);
}

/**
 * Looks up an EngineID in the EngineOverrideManager
 * @param type Vehicle type
 * @param grf_local_id The local id in the newgrf
 * @param grfid The GrfID that defines the scope of grf_local_id.
 *              If a newgrf overrides the engines of another newgrf, the "scope grfid" is the ID of the overridden newgrf.
 *              If dynamic_engines is disabled, all newgrf share the same ID scope identified by INVALID_GRFID.
 * @return The engine ID if present, or EngineID::Invalid() if not.
 */
EngineID EngineOverrideManager::GetID(VehicleType type, uint16_t grf_local_id, uint32_t grfid)
{
	return EngineID{openttd_rust_engine_mapping_get(type, grf_local_id, grfid)};
}

/**
 * Look for an unreserved EngineID matching the local id, and reserve it if found.
 * @param type Vehicle type
 * @param grf_local_id The local id in the newgrf
 * @param grfid The GrfID that defines the scope of grf_local_id.
 *              If a newgrf overrides the engines of another newgrf, the "scope grfid" is the ID of the overridden newgrf.
 *              If dynamic_engines is disabled, all newgrf share the same ID scope identified by INVALID_GRFID.
 * @param static_access Whether to actually reserve the EngineID.
 * @return The engine ID if present and now reserved, or EngineID::Invalid() if not.
 */
EngineID EngineOverrideManager::UseUnreservedID(VehicleType type, uint16_t grf_local_id, uint32_t grfid, bool static_access)
{
	return EngineID{openttd_rust_engine_mapping_use(type, grf_local_id, grfid, static_access)};
}

void EngineOverrideManager::SetID(VehicleType type, uint16_t grf_local_id, uint32_t grfid, uint8_t substitute_id, EngineID engine)
{
	openttd_rust_engine_mapping_set(type, grf_local_id, grfid, substitute_id, engine.base());
}

/**
 * Tries to reset the engine mapping to match the current NewGRF configuration.
 * This is only possible when there are currently no vehicles in the game.
 * @return false if resetting failed due to present vehicles.
 */
bool EngineOverrideManager::ResetToCurrentNewGRFConfig()
{
	return openttd_rust_engine_reset_config(&GetRustEngineLoadingServices());
}

/**
 * Initialise the engine pool with the data from the original vehicles.
 */
void SetupEngines()
{
	openttd_rust_engine_setup(&GetRustEngineLoadingServices());
}

/**
 * Determine whether an engine type is a wagon (and not a loco).
 * @param index %Engine getting queried.
 * @return Whether the queried engine is a wagon.
 */

/**
 * Ensure engine is not set as the last used variant for any other engine.
 * @param engine_id Engine being removed.
 * @param type      Type of engine.
 */

/**
 * Update #Engine::reliability and (if needed) update the engine GUIs.
 * @param e %Engine to update.
 */
void CalcEngineReliability(Engine *e, bool new_month)
{
	openttd_rust_engine_reliability(&EngineServices(), EnginePrototype(e), new_month);
}

/** Compute the value for #_year_engine_aging_stops. */
void SetYearEngineAgingStops()
{
	openttd_rust_engine_aging_stops(&EngineServices());
}

/**
 * Start/initialise one engine.
 * @param e The engine to initialise.
 * @param aging_date The date used for age calculations.
 * @param seed Random seed.
 */
void StartupOneEngine(Engine *e, const TimerGameCalendar::YearMonthDay &aging_ymd, uint32_t seed)
{
	openttd_rust_engine_start_one(&EngineServices(), EnginePrototype(e), {.year = aging_ymd.year.base(), .month = aging_ymd.month, .day = aging_ymd.day}, seed);
}

/**
 * Start/initialise all our engines. Must be called whenever there are changes
 * to the NewGRF config.
 */
void StartupEngines()
{
	openttd_rust_engine_start_all(&EngineServices());
}

/**
 * Allows engine \a eid to be used by a company \a company.
 * @param eid The engine to enable.
 * @param company The company to allow using the engine.
 */

/**
 * Forbids engine \a eid to be used by a company \a company.
 * @param eid The engine to disable.
 * @param company The company to forbid using the engine.
 */

/**
 * Company \a company accepts engine \a eid for preview.
 * @param eid Engine being accepted (is under preview).
 * @param company Current company previewing the engine.
 * @param recursion_depth Recursion depth to avoid infinite loop.
 */

/**
 * Get the best company for an engine preview.
 * @param e Engine to preview.
 * @return Best company if it exists, #CompanyID::Invalid() otherwise.
 */

/**
 * Checks if a vehicle type is disabled for all/ai companies.
 * @param type The vehicle type which shall be checked.
 * @param ai If true, check if the type is disabled for AI companies, otherwise check if
 *           the vehicle type is disabled for human companies.
 * @return Whether or not a vehicle type is disabled.
 */

/** Daily check to offer an exclusive engine preview to the companies. */
static const IntervalTimer<TimerGameCalendar> _calendar_engines_daily({TimerGameCalendar::DAY, TimerGameCalendar::Priority::ENGINE}, RustClockCallback{openttd_rust_clock_engine_daily, &EngineServices()});


/**
 * Clear the 'hidden' flag for all engines of a new company.
 * @param cid Company being created.
 */
void ClearEnginesHiddenFlagOfCompany(CompanyID cid)
{
	openttd_rust_engine_clear_hidden(&EngineServices(), cid.base());
}

/**
 * Set the visibility of an engine.
 * @param flags Operation to perform.
 * @param engine_id Engine id..
 * @param hide Set for hidden, unset for visible.
 * @return The cost of this operation or an error.
 */
CommandCost CmdSetVehicleVisibility(DoCommandFlags flags, EngineID engine_id, bool hide)
{
	return openttd_rust_engine_visibility(&EngineServices(), Engine_resolve(engine_id.base()), hide, flags.Test(DoCommandFlag::Execute)) == 0 ? CommandCost() : CMD_ERROR;
}

/**
 * Accept an engine prototype. XXX - it is possible that the top-company
 * changes while you are waiting to accept the offer? Then it becomes invalid
 * @param flags operation to perform
 * @param engine_id engine-prototype offered
 * @return the cost of this operation or an error
 */
CommandCost CmdWantEnginePreview(DoCommandFlags flags, EngineID engine_id)
{
	return openttd_rust_engine_want_preview(&EngineServices(), Engine_resolve(engine_id.base()), flags.Test(DoCommandFlag::Execute)) == 0 ? CommandCost() : CMD_ERROR;
}

/**
 * Allow or forbid a specific company to use an engine
 * @param flags operation to perform
 * @param engine_id engine id
 * @param company_id Company to allow/forbid the use of an engine.
 * @param allow false to forbid, true to allow.
 * @return the cost of this operation or an error
 */
CommandCost CmdEngineCtrl(DoCommandFlags flags, EngineID engine_id, CompanyID company_id, bool allow)
{
	return openttd_rust_engine_control(&EngineServices(), Engine_resolve(engine_id.base()), company_id.base(), allow, flags.Test(DoCommandFlag::Execute)) == 0 ? CommandCost() : CMD_ERROR;
}

/**
 * An engine has become available for general use.
 * Also handle the exclusive engine preview contract.
 * @param e Engine generally available as of now.
 */

/** Monthly update of the availability, reliability, and preview offers of the engines. */
void CalendarEnginesMonthlyLoop()
{
	openttd_rust_engine_monthly(&EngineServices());
}

static const IntervalTimer<TimerGameCalendar> _calendar_engines_monthly({TimerGameCalendar::MONTH, TimerGameCalendar::Priority::ENGINE}, RustClockCallback{openttd_rust_clock_engine_monthly, &EngineServices()});


/**
 * Is \a name still free as name for an engine?
 * @param name New name of an engine.
 * @return \c false if the name is being used already, else \c true.
 */

/**
 * Rename an engine.
 * @param flags operation to perform
 * @param engine_id engine ID to rename
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameEngine(DoCommandFlags flags, EngineID engine_id, const std::string &text)
{
	uint8_t result = openttd_rust_engine_rename(&EngineServices(), Engine_resolve(engine_id.base()), {reinterpret_cast<const uint8_t *>(text.data()), text.size()}, flags.Test(DoCommandFlag::Execute));
	return result == 0 ? CommandCost() : result == 2 ? CommandCost(STR_ERROR_NAME_MUST_BE_UNIQUE) : CMD_ERROR;
}


/**
 * Check if an engine is buildable.
 * @param engine  index of the engine to check.
 * @param type    the type the engine should be.
 * @param company index of the company.
 * @return True if an engine is valid, of the specified type, and buildable by
 *              the given company.
 */
bool IsEngineBuildable(EngineID engine, VehicleType type, CompanyID company)
{
	return openttd_rust_engine_buildable(&EngineServices(), Engine_resolve(engine.base()), type, company.base());
}

/**
 * Check if an engine is refittable.
 * Note: Likely you want to use IsArticulatedVehicleRefittable().
 * @param engine index of the engine to check.
 * @return true if the engine is refittable.
 */
bool IsEngineRefittable(EngineID engine)
{
	return openttd_rust_engine_refittable(&GetRustEngineCapabilitiesServices(), engine.base());
}

/**
 * Check for engines that have an appropriate availability.
 */
void CheckEngines()
{
	openttd_rust_engine_check(&EngineServices());
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDEngineServices.hpp"
#include "rust/vehicle_properties_ffi.h"
#include "abi-visit-OpenTTDEngineLoadingServices.hpp"
#include "abi-visit-OpenTTDVehicleProperties.hpp"
#include "abi-visit-OpenTTDTypeAvailabilityServices.hpp"
bool CheckAbi_engine() { return AbiServicesComplete(GetRustVehicleProperties()) && AbiServicesComplete(GetRustEngineLoadingServices()) && AbiServicesComplete(EngineServices()) && AbiServicesComplete(GetRustTypeAvailabilityServices()); }

std::span<const EngineIDMapping> EngineOverrideManager::Mappings(VehicleType type) const
{
	auto view = openttd_rust_engine_mappings(type);
	return {view.data, view.size};
}
void EngineOverrideManager::ClearMapping() { openttd_rust_engine_mapping_clear(); }
