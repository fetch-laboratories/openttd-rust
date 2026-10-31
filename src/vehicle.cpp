/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file vehicle.cpp Base implementations of all vehicles. */

#include "stdafx.h"
#include "error.h"
#include "roadveh.h"
#include "ship.h"
#include "spritecache.h"
#include "timetable.h"
#include "viewport_func.h"
#include "news_func.h"
#include "command_func.h"
#include "company_func.h"
#include "train.h"
#include "aircraft.h"
#include "newgrf_debug.h"
#include "newgrf_sound.h"
#include "newgrf_station.h"
#include "group_gui.h"
#include "strings_func.h"
#include "zoom_func.h"
#include "vehicle_func.h"
#include "autoreplace_func.h"
#include "autoreplace_gui.h"
#include "station_base.h"
#include "ai/ai.hpp"
#include "depot_func.h"
#include "network/network.h"
#include "core/pool_func.hpp"
#include "economy_base.h"
#include "articulated_vehicles.h"
#include "roadstop_base.h"
#include "core/random_func.hpp"
#include "core/backup_type.hpp"
#include "core/container_func.hpp"
#include "order_backup.h"
#include "sound_func.h"
#include "effectvehicle_func.h"
#include "effectvehicle_base.h"
#include "vehiclelist.h"
#include "bridge_map.h"
#include "tunnel_map.h"
#include "depot_map.h"
#include "gamelog.h"
#include "linkgraph/linkgraph.h"
#include "linkgraph/refresh.h"
#include "framerate_type.h"
#include "autoreplace_cmd.h"
#include "misc_cmd.h"
#include "train_cmd.h"
#include "vehicle_cmd.h"
#include "newgrf_roadstop.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_tick.h"

#include "table/strings.h"

#include "rust/vehicle_maintenance_ffi.h"
#include "rust/presentation_services.h"
#include "rust/vehicle_services.h"
#include "rust/company_identity_ffi.h"
#include "rust/command_cost_services.h"
#include "rust/vehicle_visits_ffi.h"
#include "rust/services_station.h"
#include "rust/services_linkgraph.h"
#include "rust/vehicle_transaction_ffi.h"

#include "rust/movement_ffi.h"

#include "rust/vehicle_effect_services.hpp"
#include "safeguards.h"

static_assert(VehicleFlags{VehicleFlag::LoadingFinished}.base() == 1 && VehicleFlags{VehicleFlag::StopLoading}.base() == 64);
static_assert(VehicleRailFlags{VehicleRailFlag::Reversed}.base() == 128 && VehicleRailFlags{VehicleRailFlag::LeavingStation}.base() == 512);
static_assert(CCF_ARRANGE.base() == 3 && TRACK_BIT_DEPOT == 0x80 && VehicleCargoList::MTA_LOAD == 3);
static_assert(OrderDepotTypeFlags{OrderDepotTypeFlag::PartOfOrders}.base() == 2 && OrderDepotActionFlags{OrderDepotActionFlag::Halt}.base() == 1 && OrderDepotActionFlags{OrderDepotActionFlag::NearestDepot}.base() == 2);
static_assert(to_underlying(OrderLoadType::FullLoad) == 2 && to_underlying(OrderLoadType::FullLoadAny) == 3);
static OpenTTDVisitCapacity VehicleVisitCapacity(OpenTTDVehicle *handle) noexcept
{
	return {.capacity = RustVehicleCargoCap(handle), .remaining = RustVehicleRefitCap(handle), .cargo = RustVehicleCargoType(handle)};
}
static OpenTTDVisitPrediction VehicleVisitPrediction(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	return {.loading_tick = v->last_loading_tick, .last_loading = RustVehicleLastLoadingStation(handle), .last_station = RustVehicleLastStationVisited(handle), .ticks = RustVehicleLoadUnloadTicks(handle)};
}
static void VehicleVisitArrivalDirty(OpenTTDVehicle *handle) noexcept
{
	Vehicle *v = VehicleOf(handle);
	SetWindowDirty(GetWindowClassForVehicleType(v->type), v->owner);
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP);
	RustVehicleDetailsDirty(handle);
	SetWindowDirty(WC_STATION_VIEW, v->last_station_visited);

	RustStationMarkTilesDirty(Station::Get(v->last_station_visited), true);
}
static void VehicleVisitReservationLog() noexcept { Debug(misc, 1, "cancelling cargo reservation"); }
static void VehicleVisitDepotWindows(OpenTTDVehicle *handle, bool invalidate) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	if (invalidate) RustVehicleInvalidateDepot(v);
	RustVehicleDirtyDepot(v);
}
static void VehicleVisitShipDepotViewport(OpenTTDVehicle *handle) noexcept
{
	Ship *ship = Ship::From(VehicleOf(handle));
	RustVehicleUpdateViewport(ship, true, true);
	RustVehicleDirtyDepot(ship);
}
static void VehicleVisitWithCosts(void *context, void (*scope)(void *, void *)) noexcept
{
	CommandCost cost, temporary;
	OpenTTDVisitCosts costs{reinterpret_cast<OpenTTDCommandCost *>(&cost), reinterpret_cast<OpenTTDCommandCost *>(&temporary)};
	scope(context, &costs);
}
static void VehicleVisitRefitFailedNews(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	if (v->owner == _local_company) {
		/* Notify the user that we stopped the vehicle */
		AddVehicleAdviceNewsItem(AdviceType::RefitFailed, GetEncodedString(STR_NEWS_ORDER_REFIT_FAILED, v->index), v->index);
	}
}
static void VehicleVisitCostAnimation(OpenTTDVehicle *handle, int64_t cost) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	if (v->owner == _local_company) RustCostOrIncomeAnimation(v->x_pos, v->y_pos, v->z_pos, cost);
}
static void VehicleVisitWaitingNews(OpenTTDVehicle *handle) noexcept
{
	const Vehicle *v = VehicleOf(handle);
	/* Announce that the vehicle is waiting to players and AIs. */
	if (v->owner == _local_company) {
		AddVehicleAdviceNewsItem(AdviceType::VehicleWaiting, GetEncodedString(STR_NEWS_TRAIN_IS_WAITING + v->type, v->index), v->index);
	}
	AI::NewEvent(v->owner, new ScriptEventVehicleWaitingInDepot(v->index));
}
const OpenTTDVehicleVisits &GetRustVehicleVisits() noexcept
{
	static const OpenTTDVehicleVisits services{
		.shared = &GetRustSharedServices(),
		.station = &GetRustStationServices(),
		.links = &GetRustStationLinks(),
		.loading = &GetRustStationLoading(),
		.refit = &GetRustVehicleRefitServices(),
		.schedule = &LinkGraphSchedule::services,
		.capacity = VehicleVisitCapacity,
		.prediction = VehicleVisitPrediction,
		.display_speed = RustVehicleDisplaySpeed,
		.location = RustStationLocation,
		.percent = {.up = STR_PERCENT_UP, .down = STR_PERCENT_DOWN, .up_down = STR_PERCENT_UP_DOWN, .none = STR_PERCENT_NONE},
		.set_last_loading = RustVehicleSetLastLoadingStation,
		.set_loading_tick = RustVehicleSetLastLoadingTick,
		.arrival_dirty = VehicleVisitArrivalDirty,
		.station_cargo = RustStationCargoOwner,
		.reservation_log = VehicleVisitReservationLog,
		.hide_filling = RustVehicleHideFilling,
		.set_occupancy = RustVehicleSetTripOccupancy,
		.leave_sound = RustVehiclePlayLeaveSound,
		.depot_windows = VehicleVisitDepotWindows,
		.ship_depot_viewport = VehicleVisitShipDepotViewport,
		.view_invalidate = RustVehicleVisitInvalidateView,
		.with_costs = VehicleVisitWithCosts,
		.refit_failed_news = VehicleVisitRefitFailedNews,
		.cost_animation = VehicleVisitCostAnimation,
		.waiting_news = VehicleVisitWaitingNews,
	};
	return services;
}

/* Number of bits in the hash to use from each vehicle coord */
static const uint GEN_HASHX_BITS = 6;
static const uint GEN_HASHY_BITS = 6;

/* Size of each hash bucket */
static const uint GEN_HASHX_BUCKET_BITS = 7;
static const uint GEN_HASHY_BUCKET_BITS = 6;

/* Compute hash for vehicle coord */
static inline uint GetViewportHashX(int x)
{
	return GB(x, GEN_HASHX_BUCKET_BITS + ZOOM_BASE_SHIFT, GEN_HASHX_BITS);
}

static inline uint GetViewportHashY(int y)
{
	return GB(y, GEN_HASHY_BUCKET_BITS + ZOOM_BASE_SHIFT, GEN_HASHY_BITS) << GEN_HASHX_BITS;
}

static inline uint GetViewportHash(int x, int y)
{
	return GetViewportHashX(x) + GetViewportHashY(y);
}

/* Maximum size until hash repeats */
static const uint GEN_HASHX_SIZE = 1 << (GEN_HASHX_BUCKET_BITS + GEN_HASHX_BITS + ZOOM_BASE_SHIFT);
static const uint GEN_HASHY_SIZE = 1 << (GEN_HASHY_BUCKET_BITS + GEN_HASHY_BITS + ZOOM_BASE_SHIFT);

/* Increments to reach next bucket in hash table */
static const uint GEN_HASHX_INC = 1;
static const uint GEN_HASHY_INC = 1 << GEN_HASHX_BITS;

/* Mask to wrap-around buckets */
static const uint GEN_HASHX_MASK =  (1 << GEN_HASHX_BITS) - 1;
static const uint GEN_HASHY_MASK = ((1 << GEN_HASHY_BITS) - 1) << GEN_HASHX_BITS;


/** The pool with all our precious vehicles. */
VehiclePool _vehicle_pool("Vehicle");
INSTANTIATE_POOL_METHODS(Vehicle)


/**
 * Determine shared bounds of all sprites.
 * @param[out] bounds Shared bounds.
 */
void VehicleSpriteSeq::GetBounds(Rect *bounds) const
{
	bounds->left = bounds->top = bounds->right = bounds->bottom = 0;
	for (uint i = 0; i < this->count; ++i) {
		const Sprite *spr = GetSprite(this->seq[i].sprite, SpriteType::Normal);
		if (i == 0) {
			bounds->left = spr->x_offs;
			bounds->top  = spr->y_offs;
			bounds->right  = spr->width  + spr->x_offs - 1;
			bounds->bottom = spr->height + spr->y_offs - 1;
		} else {
			if (spr->x_offs < bounds->left) bounds->left = spr->x_offs;
			if (spr->y_offs < bounds->top)  bounds->top  = spr->y_offs;
			int right  = spr->width  + spr->x_offs - 1;
			int bottom = spr->height + spr->y_offs - 1;
			if (right  > bounds->right)  bounds->right  = right;
			if (bottom > bounds->bottom) bounds->bottom = bottom;
		}
	}
}

/**
 * Draw the sprite sequence.
 * @param x X position
 * @param y Y position
 * @param default_pal Vehicle palette
 * @param force_pal Whether to ignore individual palettes, and draw everything with \a default_pal.
 */
void VehicleSpriteSeq::Draw(int x, int y, PaletteID default_pal, bool force_pal) const
{
	for (uint i = 0; i < this->count; ++i) {
		PaletteID pal = force_pal || !this->seq[i].pal ? default_pal : this->seq[i].pal;
		DrawSprite(this->seq[i].sprite, pal, x, y);
	}
}

/**
 * Function to tell if a vehicle needs to be autorenewed
 * @param *c The vehicle owner
 * @param use_renew_setting Should the company renew setting be considered?
 * @return true if the vehicle is old enough for replacement
 */
bool Vehicle::NeedsAutorenewing(const Company *c, bool use_renew_setting) const
{
	return openttd_rust_vehicle_needs_renew(RustVehicle(this), &GetRustVehicleServices(), c, use_renew_setting);
}

/**
 * Service a vehicle and all subsequent vehicles in the consist
 *
 * @param *v The vehicle or vehicle chain being serviced
 */
void VehicleServiceInDepot(Vehicle *v)
{
	assert(v != nullptr);
	openttd_rust_vehicle_service_depot(RustVehicle(v), &GetRustVehicleServices());
}

/**
 * Check if the vehicle needs to go to a depot in near future (if a opportunity presents itself) for service or replacement.
 *
 * @see NeedsAutomaticServicing()
 * @return true if the vehicle should go to a depot if a opportunity presents itself.
 */
bool Vehicle::NeedsServicing() const
{
	return openttd_rust_vehicle_needs_service(RustVehicle(this), &GetRustVehicleServices());
}

/**
 * Checks if the current order should be interrupted for a service-in-depot order.
 * @see NeedsServicing()
 * @return true if the current order should be interrupted.
 */
bool Vehicle::NeedsAutomaticServicing() const
{
	return openttd_rust_vehicle_needs_automatic_service(RustVehicle(this), &GetRustVehicleServices());
}

uint Vehicle::Crash(bool)
{
	assert(!this->vehstatus.Test(VehState::Crashed));
	assert(this->Previous() == nullptr); // IsPrimaryVehicle fails for free-wagon-chains

	uint pass = 0;
	/* Stop the vehicle. */
	if (this->IsPrimaryVehicle()) this->vehstatus.Set(VehState::Stopped);
	/* crash all wagons, and count passengers */
	for (Vehicle *v = this; v != nullptr; v = v->Next()) {
		/* We do not transfer reserver cargo back, so TotalCount() instead of StoredCount() */
		if (IsCargoInClass(v->cargo_type, CargoClass::Passengers)) pass += v->cargo.TotalCount();
		v->vehstatus.Set(VehState::Crashed);
		v->MarkAllViewportsDirty();
	}

	/* Dirty some windows */
	InvalidateWindowClassesData(GetWindowClassForVehicleType(this->type), 0);
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, this->index, WID_VV_START_STOP);
	SetWindowDirty(WC_VEHICLE_DETAILS, this->index);
	SetWindowDirty(WC_VEHICLE_DEPOT, this->tile);

	delete this->cargo_payment;
	assert(this->cargo_payment == nullptr); // cleared by ~CargoPayment

	return RandomRange(pass + 1); // Randomise deceased passengers.
}


/**
 * Displays a "NewGrf Bug" error message for a engine, and pauses the game if not networking.
 * @param engine The engine that caused the problem
 * @param part1  Part 1 of the error message, taking the grfname as parameter 1
 * @param part2  Part 2 of the error message, taking the engine as parameter 2
 * @param bug_type Flag to check and set in grfconfig
 * @param critical Shall the "OpenTTD might crash"-message be shown when the player tries to unpause?
 */
void ShowNewGrfVehicleError(EngineID engine, StringID part1, StringID part2, GRFBug bug_type, bool critical)
{
	const Engine *e = Engine::Get(engine);
	GRFConfig *grfconfig = GetGRFConfig(e->GetGRFID());

	/* Missing GRF. Nothing useful can be done in this situation. */
	if (grfconfig == nullptr) return;

	if (!grfconfig->grf_bugs.Test(bug_type)) {
		grfconfig->grf_bugs.Set(bug_type);
		ShowErrorMessage(GetEncodedString(part1, grfconfig->GetName()),
			GetEncodedString(part2, std::monostate{}, engine), WL_CRITICAL);
		if (!_networking) Command<CMD_PAUSE>::Do(DoCommandFlag::Execute, critical ? PauseMode::Error : PauseMode::Normal, true);
	}

	/* debug output */
	Debug(grf, 0, "{}", StrMakeValid(GetString(part1, grfconfig->GetName())));

	Debug(grf, 0, "{}", StrMakeValid(GetString(part2, std::monostate{}, engine)));
}

/**
 * Logs a bug in GRF and shows a warning message if this
 * is for the first time this happened.
 * @param u first vehicle of chain
 */
void VehicleLengthChanged(const Vehicle *u)
{
	/* show a warning once for each engine in whole game and once for each GRF after each game load */
	const Engine *engine = u->GetEngine();
	uint32_t grfid = engine->grf_prop.grfid;
	GRFConfig *grfconfig = GetGRFConfig(grfid);
	if (_gamelog.GRFBugReverse(grfid, engine->grf_prop.local_id) || !grfconfig->grf_bugs.Test(GRFBug::VehLength)) {
		ShowNewGrfVehicleError(u->engine_type, STR_NEWGRF_BROKEN, STR_NEWGRF_BROKEN_VEHICLE_LENGTH, GRFBug::VehLength, true);
	}
}

/**
 * Vehicle constructor.
 * @param type Type of the new vehicle.
 */
Vehicle::Vehicle(VehicleType type)
{
	this->type               = type;
	this->coord.left         = INVALID_COORD;
	this->sprite_cache.old_coord.left = INVALID_COORD;
	this->group_id           = DEFAULT_GROUP;
	this->fill_percent_te_id = INVALID_TE_ID;
	this->first              = this;
	this->colourmap          = PAL_NONE;
	this->cargo_age_counter  = 1;
	this->last_station_visited = StationID::Invalid();
	this->last_loading_station = StationID::Invalid();
}

VehiclesOnTile::Iterator::Iterator(TileIndex tile) : cursor(openttd_rust_occupancy_on_tile(&GetRustVehicleServices(), tile.base())) {}
void VehiclesOnTile::Iterator::Increment() { openttd_rust_occupancy_on_advance(&GetRustVehicleServices(), &this->cursor); }
VehiclesNearTileXY::Iterator::Iterator(int32_t x, int32_t y, uint max_dist) : cursor(openttd_rust_occupancy_near(&GetRustVehicleServices(), x, y, max_dist)) {}
void VehiclesNearTileXY::Iterator::Increment() { openttd_rust_occupancy_near_advance(&GetRustVehicleServices(), &this->cursor); }

/**
 * Ensure there is no vehicle at the ground at the given position.
 * @param tile Position to examine.
 * @return Succeeded command (ground is free) or failed command (a vehicle is found).
 */
CommandCost EnsureNoVehicleOnGround(TileIndex tile)
{
	CommandCost out;
	openttd_rust_occupancy_ground(GetRustOccupancyCommands(), RustCommandCost(&out), tile.base());
	return out;
}

/**
 * Finds vehicle in tunnel / bridge
 * @param tile first end
 * @param endtile second end
 * @param ignore Ignore this vehicle when searching
 * @return Succeeded command (if tunnel/bridge is free) or failed command (if a vehicle is using the tunnel/bridge).
 */
CommandCost TunnelBridgeIsFree(TileIndex tile, TileIndex endtile, const Vehicle *ignore)
{
	CommandCost out;
	openttd_rust_occupancy_wormhole(GetRustOccupancyCommands(), RustCommandCost(&out), tile.base(), endtile.base(), RustVehicle(ignore));
	return out;
}

/**
 * Tests if a vehicle interacts with the specified track bits.
 * All track bits interact except parallel #TRACK_BIT_HORZ or #TRACK_BIT_VERT.
 *
 * @param tile The tile.
 * @param track_bits The track bits.
 * @return \c true if no train that interacts, is found. \c false if a train is found.
 */
CommandCost EnsureNoTrainOnTrackBits(TileIndex tile, TrackBits track_bits)
{
	CommandCost out;
	openttd_rust_occupancy_train(GetRustOccupancyCommands(), RustCommandCost(&out), tile.base(), track_bits);
	return out;
}


static std::array<Vehicle *, 1 << (GEN_HASHX_BITS + GEN_HASHY_BITS)> _vehicle_viewport_hash{};

static void UpdateVehicleViewportHash(Vehicle *v, int x, int y, int old_x, int old_y)
{
	Vehicle **old_hash, **new_hash;

	new_hash = (x == INVALID_COORD) ? nullptr : &_vehicle_viewport_hash[GetViewportHash(x, y)];
	old_hash = (old_x == INVALID_COORD) ? nullptr : &_vehicle_viewport_hash[GetViewportHash(old_x, old_y)];

	if (old_hash == new_hash) return;

	/* remove from hash table? */
	if (old_hash != nullptr) {
		if (v->hash_viewport_next != nullptr) v->hash_viewport_next->hash_viewport_prev = v->hash_viewport_prev;
		*v->hash_viewport_prev = v->hash_viewport_next;
	}

	/* insert into hash table? */
	if (new_hash != nullptr) {
		v->hash_viewport_next = *new_hash;
		if (v->hash_viewport_next != nullptr) v->hash_viewport_next->hash_viewport_prev = &v->hash_viewport_next;
		v->hash_viewport_prev = new_hash;
		*new_hash = v;
	}
}

void ResetVehicleHash()
{
	openttd_rust_occupancy_reset(&GetRustVehicleSchedulerServices());
	_vehicle_viewport_hash.fill(nullptr);
}

void ResetVehicleColourMap()
{
	for (Vehicle *v : Vehicle::Iterate()) { v->colourmap = PAL_NONE; }
}

/**
 * List of vehicles that should check for autoreplace this tick.
 * Mapping of vehicle -> leave depot immediately after autoreplace.
 */
#include "rust/fleet_pending_services.hpp"
static void ClearPendingAutoreplace() { openttd_rust_fleet_pending_clear(); }

void InitializeVehicles()
{
	openttd_rust_effect_bind(GetRustEffectCreationServices());
	openttd_rust_visual_bind(&GetRustVisualServices());
	ClearPendingAutoreplace();
	ResetVehicleHash();
}

uint CountVehiclesInChain(const Vehicle *v)
{
	uint count = 0;
	do count++; while ((v = v->Next()) != nullptr);
	return count;
}

/**
 * Check if a vehicle is counted in num_engines in each company struct
 * @return true if the vehicle is counted in num_engines
 */
bool Vehicle::IsEngineCountable() const
{
	switch (this->type) {
		case VEH_AIRCRAFT: return Aircraft::From(this)->IsNormalAircraft(); // don't count plane shadows and helicopter rotors
		case VEH_TRAIN:
			return !this->IsArticulatedPart() && // tenders and other articulated parts
					!Train::From(this)->IsRearDualheaded(); // rear parts of multiheaded engines
		case VEH_ROAD: return RoadVehicle::From(this)->IsFrontEngine();
		case VEH_SHIP: return true;
		default: return false; // Only count company buildable vehicles
	}
}

/**
 * Check whether Vehicle::engine_type has any meaning.
 * @return true if the vehicle has a usable engine type.
 */
bool Vehicle::HasEngineType() const
{
	switch (this->type) {
		case VEH_AIRCRAFT: return Aircraft::From(this)->IsNormalAircraft();
		case VEH_TRAIN:
		case VEH_ROAD:
		case VEH_SHIP: return true;
		default: return false;
	}
}

/**
 * Retrieves the engine of the vehicle.
 * @return Engine of the vehicle.
 * @pre HasEngineType() == true
 */
const Engine *Vehicle::GetEngine() const
{
	return Engine::Get(this->engine_type);
}

/**
 * Retrieve the NewGRF the vehicle is tied to.
 * This is the GRF providing the Action 3 for the engine type.
 * @return NewGRF associated to the vehicle.
 */
const GRFFile *Vehicle::GetGRF() const
{
	return this->GetEngine()->GetGRF();
}

/**
 * Retrieve the GRF ID of the NewGRF the vehicle is tied to.
 * This is the GRF providing the Action 3 for the engine type.
 * @return GRF ID of the associated NewGRF.
 */
uint32_t Vehicle::GetGRFID() const
{
	return this->GetEngine()->GetGRFID();
}

/**
 * Shift all dates by given interval.
 * This is useful if the date has been modified with the cheat menu.
 * @param interval Number of days to be added or subtracted.
 */
void Vehicle::ShiftDates(TimerGameEconomy::Date interval)
{
	this->date_of_last_service = std::max(this->date_of_last_service + interval, TimerGameEconomy::Date(0));
	/* date_of_last_service_newgrf is not updated here as it must stay stable
	 * for vehicles outside of a depot. */
}

/**
 * Handle the pathfinding result, especially the lost status.
 * If the vehicle is now lost and wasn't previously fire an
 * event to the AIs and a news message to the user. If the
 * vehicle is not lost anymore remove the news message.
 * @param path_found Whether the vehicle has a path to its destination.
 */
void Vehicle::HandlePathfindingResult(bool path_found)
{
	if (path_found) {
		/* Route found, is the vehicle marked with "lost" flag? */
		if (!this->vehicle_flags.Test(VehicleFlag::PathfinderLost)) return;

		/* Clear the flag as the PF's problem was solved. */
		this->vehicle_flags.Reset(VehicleFlag::PathfinderLost);
		SetWindowWidgetDirty(WC_VEHICLE_VIEW, this->index, WID_VV_START_STOP);
		InvalidateWindowClassesData(GetWindowClassForVehicleType(this->type));
		/* Delete the news item. */
		DeleteVehicleNews(this->index, AdviceType::VehicleLost);
		return;
	}

	/* Were we already lost? */
	if (this->vehicle_flags.Test(VehicleFlag::PathfinderLost)) return;

	/* It is first time the problem occurred, set the "lost" flag. */
	this->vehicle_flags.Set(VehicleFlag::PathfinderLost);
	SetWindowWidgetDirty(WC_VEHICLE_VIEW, this->index, WID_VV_START_STOP);
	InvalidateWindowClassesData(GetWindowClassForVehicleType(this->type));

	/* Unbunching data is no longer valid. */
	this->ResetDepotUnbunching();

	/* Notify user about the event. */
	AI::NewEvent(this->owner, new ScriptEventVehicleLost(this->index));
	if (_settings_client.gui.lost_vehicle_warn && this->owner == _local_company) {
		AddVehicleAdviceNewsItem(AdviceType::VehicleLost, GetEncodedString(STR_NEWS_VEHICLE_IS_LOST, this->index), this->index);
	}
}

/** Destroy all stuff that (still) needs the virtual functions to work properly */
void Vehicle::PreDestructor()
{
	if (CleaningPool()) return;

	if (Station::IsValidID(this->last_station_visited)) {
		Station *st = Station::Get(this->last_station_visited);
		st->loading_vehicles.remove(this);

		HideFillingPercent(&this->fill_percent_te_id);
		this->CancelReservation(StationID::Invalid(), st);
		delete this->cargo_payment;
		assert(this->cargo_payment == nullptr); // cleared by ~CargoPayment
	}

	if (this->IsEngineCountable()) {
		GroupStatistics::CountEngine(this, -1);
		if (this->IsPrimaryVehicle()) GroupStatistics::CountVehicle(this, -1);
		GroupStatistics::UpdateAutoreplace(this->owner);

		if (this->owner == _local_company) InvalidateAutoreplaceWindow(this->engine_type, this->group_id);
		DeleteGroupHighlightOfVehicle(this);
	}

	Company::Get(this->owner)->freeunits[this->type].ReleaseID(this->unitnumber);

	if (this->type == VEH_AIRCRAFT && this->IsPrimaryVehicle()) {
		Aircraft *a = Aircraft::From(this);
		ReleaseAircraftAirportBlocks(a);
	}


	if (this->type == VEH_ROAD && this->IsPrimaryVehicle()) {
		RoadVehicle *v = RoadVehicle::From(this);
		if (!v->vehstatus.Test(VehState::Crashed) && IsInsideMM(v->GetState(), RVSB_IN_DT_ROAD_STOP, RVSB_IN_DT_ROAD_STOP_END)) {
			/* Leave the drive through roadstop, when you have not already left it. */
			openttd_rust_road_stop_leave(&GetRustSharedServices(), RustVehicle(v), v->GetRustState());
		}

		if (v->disaster_vehicle != VehicleID::Invalid()) ReleaseDisasterVehicle(v->disaster_vehicle);
	}

	if (this->Previous() == nullptr) {
		InvalidateWindowData(WC_VEHICLE_DEPOT, this->tile);
	}

	if (this->IsPrimaryVehicle()) {
		CloseWindowById(WC_VEHICLE_VIEW, this->index);
		CloseWindowById(WC_VEHICLE_ORDERS, this->index);
		CloseWindowById(WC_VEHICLE_REFIT, this->index);
		CloseWindowById(WC_VEHICLE_DETAILS, this->index);
		CloseWindowById(WC_VEHICLE_TIMETABLE, this->index);
		SetWindowDirty(WC_COMPANY, this->owner);
		OrderBackup::ClearVehicle(this);
	}
	InvalidateWindowClassesData(GetWindowClassForVehicleType(this->type), 0);

	this->cargo.Truncate();
	DeleteVehicleOrders(this);
	DeleteDepotHighlightOfVehicle(this);

	StopGlobalFollowVehicle(this);
}

Vehicle::~Vehicle()
{
	if (CleaningPool()) {
		this->cargo.OnCleanPool();
		return;
	}

	/* sometimes, eg. for disaster vehicles, when company bankrupts, when removing crashed/flooded vehicles,
	 * it may happen that vehicle chain is deleted when visible */
	if (!this->vehstatus.Test(VehState::Hidden)) this->MarkAllViewportsDirty();

	Vehicle *v = this->Next();
	this->SetNext(nullptr);

	delete v;

	openttd_rust_occupancy_update(&GetRustVehicleServices(), RustVehicle(this), true);
	UpdateVehicleViewportHash(this, INVALID_COORD, 0, this->sprite_cache.old_coord.left, this->sprite_cache.old_coord.top);
	if (this->type != VEH_EFFECT) {
		DeleteVehicleNews(this->index);
		DeleteNewGRFInspectWindow(GetGrfSpecFeature(this->type), this->index);
	}
}


#include "rust/vehicle_ffi.h"
#include "rust/station_service_ffi.h"
#include "rust/vehicle_refit_ffi.h"
OpenTTDVehicleSchedulerTables RustVehicleSchedulerTables(OpenTTDCommandCost *refit_cost, OpenTTDCommandCost *temporary) noexcept
{
	return {.scheduler = &GetRustVehicleSchedulerServices(), .cargo = CargoStorageServices(), .station = &GetRustStationServices(), .loading = &GetRustStationLoading(), .pending = &_fleet_pending_services, .transaction = &FleetTransactionServices(), .group = &FleetGroupServices(), .visits = &GetRustVehicleVisits(), .refit_cost = refit_cost, .refit_temporary = temporary};
}



/**
 * Add vehicle sprite for drawing to the screen.
 * @param v Vehicle to draw.
 */
static void DoDrawVehicle(const Vehicle *v)
{
	PaletteID pal = PAL_NONE;

	if (v->vehstatus.Test(VehState::DefaultPalette)) pal = v->vehstatus.Test(VehState::Crashed) ? PALETTE_CRASH : GetVehiclePalette(v);

	/* Check whether the vehicle shall be transparent due to the game state */
	bool shadowed = v->vehstatus.Test(VehState::Shadow);

	if (v->type == VEH_EFFECT) {
		/* Check whether the vehicle shall be transparent/invisible due to GUI settings.
		 * However, transparent smoke and bubbles look weird, so always hide them. */
		TransparencyOption to = EffectVehicle::From(v)->GetTransparencyOption();
		if (to != TO_INVALID && (IsTransparencySet(to) || IsInvisibilitySet(to))) return;
	}

	StartSpriteCombine();
	for (uint i = 0; i < v->sprite_cache.sprite_seq.count; ++i) {
		PaletteID pal2 = v->sprite_cache.sprite_seq.seq[i].pal;
		if (!pal2 || v->vehstatus.Test(VehState::Crashed)) pal2 = pal;
		AddSortableSpriteToDraw(v->sprite_cache.sprite_seq.seq[i].sprite, pal2, v->x_pos, v->y_pos, v->z_pos, v->bounds, shadowed);
	}
	EndSpriteCombine();
}

/**
 * Add the vehicle sprites that should be drawn at a part of the screen.
 * @param dpi Rectangle being drawn.
 */
void ViewportAddVehicles(DrawPixelInfo *dpi)
{
	/* The bounding rectangle */
	const int l = dpi->left;
	const int r = dpi->left + dpi->width;
	const int t = dpi->top;
	const int b = dpi->top + dpi->height;

	/* Border size of MAX_VEHICLE_PIXEL_xy */
	const int xb = MAX_VEHICLE_PIXEL_X * ZOOM_BASE;
	const int yb = MAX_VEHICLE_PIXEL_Y * ZOOM_BASE;

	/* The hash area to scan */
	uint xl, xu, yl, yu;

	if (static_cast<uint>(dpi->width + xb) < GEN_HASHX_SIZE) {
		xl = GetViewportHashX(l - xb);
		xu = GetViewportHashX(r);
	} else {
		/* scan whole hash row */
		xl = 0;
		xu = GEN_HASHX_MASK;
	}

	if (static_cast<uint>(dpi->height + yb) < GEN_HASHY_SIZE) {
		yl = GetViewportHashY(t - yb);
		yu = GetViewportHashY(b);
	} else {
		/* scan whole column */
		yl = 0;
		yu = GEN_HASHY_MASK;
	}

	for (uint y = yl;; y = (y + GEN_HASHY_INC) & GEN_HASHY_MASK) {
		for (uint x = xl;; x = (x + GEN_HASHX_INC) & GEN_HASHX_MASK) {
			const Vehicle *v = _vehicle_viewport_hash[x + y]; // already masked & 0xFFF

			while (v != nullptr) {

				if (!v->vehstatus.Test(VehState::Hidden) &&
					l <= v->coord.right + xb &&
					t <= v->coord.bottom + yb &&
					r >= v->coord.left - xb &&
					b >= v->coord.top - yb)
				{
					/*
					 * This vehicle can potentially be drawn as part of this viewport and
					 * needs to be revalidated, as the sprite may not be correct.
					 */
					if (v->sprite_cache.revalidate_before_draw) {
						VehicleSpriteSeq seq;
						v->GetImage(v->direction, EIT_ON_MAP, &seq);

						if (seq.IsValid() && v->sprite_cache.sprite_seq != seq) {
							v->sprite_cache.sprite_seq = seq;
							/*
							 * A sprite change may also result in a bounding box change,
							 * so we need to update the bounding box again before we
							 * check to see if the vehicle should be drawn. Note that
							 * we can't interfere with the viewport hash at this point,
							 * so we keep the original hash on the assumption there will
							 * not be a significant change in the top and left coordinates
							 * of the vehicle.
							 */
							v->UpdateBoundingBoxCoordinates(false);

						}

						v->sprite_cache.revalidate_before_draw = false;
					}

					if (l <= v->coord.right &&
						t <= v->coord.bottom &&
						r >= v->coord.left &&
						b >= v->coord.top) DoDrawVehicle(v);
				}

				v = v->hash_viewport_next;
			}

			if (x == xu) break;
		}

		if (y == yu) break;
	}
}

/**
 * Find the vehicle close to the clicked coordinates.
 * @param vp Viewport clicked in.
 * @param x  X coordinate in the viewport.
 * @param y  Y coordinate in the viewport.
 * @return Closest vehicle, or \c nullptr if none found.
 */
Vehicle *CheckClickOnVehicle(const Viewport &vp, int x, int y)
{
	Vehicle *found = nullptr;
	uint dist, best_dist = UINT_MAX;

	x -= vp.left;
	y -= vp.top;
	if (!IsInsideMM(x, 0, vp.width) || !IsInsideMM(y, 0, vp.height)) return nullptr;

	x = ScaleByZoom(x, vp.zoom) + vp.virtual_left;
	y = ScaleByZoom(y, vp.zoom) + vp.virtual_top;

	/* Border size of MAX_VEHICLE_PIXEL_xy */
	const int xb = MAX_VEHICLE_PIXEL_X * ZOOM_BASE;
	const int yb = MAX_VEHICLE_PIXEL_Y * ZOOM_BASE;

	/* The hash area to scan */
	uint xl = GetViewportHashX(x - xb);
	uint xu = GetViewportHashX(x);
	uint yl = GetViewportHashY(y - yb);
	uint yu = GetViewportHashY(y);

	for (uint hy = yl;; hy = (hy + GEN_HASHY_INC) & GEN_HASHY_MASK) {
		for (uint hx = xl;; hx = (hx + GEN_HASHX_INC) & GEN_HASHX_MASK) {
			Vehicle *v = _vehicle_viewport_hash[hx + hy]; // already masked & 0xFFF

			while (v != nullptr) {
				if (!v->vehstatus.Any({VehState::Hidden, VehState::Unclickable}) &&
					x >= v->coord.left && x <= v->coord.right &&
					y >= v->coord.top && y <= v->coord.bottom) {

					dist = std::max(
						abs(((v->coord.left + v->coord.right) >> 1) - x),
						abs(((v->coord.top + v->coord.bottom) >> 1) - y)
					);

					if (dist < best_dist) {
						found = v;
						best_dist = dist;
					}
				}
				v = v->hash_viewport_next;
			}
			if (hx == xu) break;
		}
		if (hy == yu) break;
	}

	return found;
}

/**
 * Decrease the value of a vehicle.
 * @param v %Vehicle to devaluate.
 */
void DecreaseVehicleValue(Vehicle *v)
{
	openttd_rust_vehicle_decrease_value(RustVehicle(v), &GetRustVehicleServices());
}


void CheckVehicleBreakdown(Vehicle *v)
{
	openttd_rust_vehicle_check_breakdown(RustVehicle(v), &GetRustVehicleServices());
}

/**
 * Handle all of the aspects of a vehicle breakdown
 * This includes adding smoke and sounds, and ending the breakdown when appropriate.
 * @return true iff the vehicle is stopped because of a breakdown
 * @note This function always returns false for aircraft, since these never stop for breakdowns
 */
bool Vehicle::HandleBreakdown()
{
	return openttd_rust_vehicle_handle_breakdown(RustVehicle(this), &GetRustVehicleServices());
}

/**
 * Update economy age of a vehicle.
 * @param v Vehicle to update.
 */
void EconomyAgeVehicle(Vehicle *v)
{
	openttd_rust_vehicle_economy_age(RustVehicle(v), &GetRustVehicleServices());
}

/**
 * Update age of a vehicle.
 * @param v Vehicle to update.
 */
void AgeVehicle(Vehicle *v)
{
	openttd_rust_vehicle_age(RustVehicle(v), &GetRustVehicleServices());
}

/**
 * Calculates how full a vehicle is.
 * @param front The front vehicle of the consist to check.
 * @param colour The string to show depending on if we are unloading or loading
 * @return A percentage of how full the Vehicle is.
 *         Percentages are rounded towards 50%, so that 0% and 100% are only returned
 *         if the vehicle is completely empty or full.
 *         This is useful for both display and conditional orders.
 */
uint8_t CalcPercentVehicleFilled(const Vehicle *front, StringID *colour)
{
	assert(colour == nullptr || (Station::GetIfValid(front->last_station_visited) != nullptr && front->current_order.IsType(OT_LOADING)));
	return openttd_rust_percent_filled(RustVehicle(front), colour, &GetRustVehicleVisits());
}

/**
 * Vehicle entirely entered the depot, update its status, orders, vehicle windows, service it, etc.
 * @param v Vehicle that entered a depot.
 */
void VehicleEnterDepot(Vehicle *v)
{
	/* Always work with the front of the vehicle */
	assert(v == v->First());

	openttd_rust_visit_enter_depot(RustVehicle(v), &GetRustVehicleVisits());
}


/**
 * Update the position of the vehicle. This will update the hash that tells
 *  which vehicles are on a tile.
 */
void Vehicle::UpdatePosition()
{
	openttd_rust_occupancy_update(&GetRustVehicleServices(), RustVehicle(this), false);
}

/**
 * Update the bounding box co-ordinates of the vehicle
 * @param update_cache Update the cached values for previous co-ordinate values
 */
void Vehicle::UpdateBoundingBoxCoordinates(bool update_cache) const
{
	Rect new_coord;
	this->sprite_cache.sprite_seq.GetBounds(&new_coord);

	/* z-bounds are not used. */
	Point pt = RemapCoords(this->x_pos + this->bounds.origin.x + this->bounds.offset.x, this->y_pos + this->bounds.origin.y + this->bounds.offset.y, this->z_pos);
	new_coord.left   += pt.x;
	new_coord.top    += pt.y;
	new_coord.right  += pt.x + 2 * ZOOM_BASE;
	new_coord.bottom += pt.y + 2 * ZOOM_BASE;

	extern bool _draw_bounding_boxes;
	if (_draw_bounding_boxes) {
		int x = this->x_pos + this->bounds.origin.x;
		int y = this->y_pos + this->bounds.origin.y;
		int z = this->z_pos + this->bounds.origin.z;
		new_coord.left   = std::min(new_coord.left, RemapCoords(x + bounds.extent.x, y, z).x);
		new_coord.right  = std::max(new_coord.right, RemapCoords(x, y + bounds.extent.y, z).x + 1);
		new_coord.top    = std::min(new_coord.top, RemapCoords(x, y, z + bounds.extent.z).y);
		new_coord.bottom = std::max(new_coord.bottom, RemapCoords(x + bounds.extent.x, y + bounds.extent.y, z).y + 1);
	}

	if (update_cache) {
		/*
		 * If the old coordinates are invalid, set the cache to the new coordinates for correct
		 * behaviour the next time the coordinate cache is checked.
		 */
		this->sprite_cache.old_coord = this->coord.left == INVALID_COORD ? new_coord : this->coord;
	}
	else {
		/* Extend the bounds of the existing cached bounding box so the next dirty window is correct */
		this->sprite_cache.old_coord.left   = std::min(this->sprite_cache.old_coord.left,   this->coord.left);
		this->sprite_cache.old_coord.top    = std::min(this->sprite_cache.old_coord.top,    this->coord.top);
		this->sprite_cache.old_coord.right  = std::max(this->sprite_cache.old_coord.right,  this->coord.right);
		this->sprite_cache.old_coord.bottom = std::max(this->sprite_cache.old_coord.bottom, this->coord.bottom);
	}

	this->coord = new_coord;
}

/**
 * Update the vehicle on the viewport, updating the right hash and setting the new coordinates.
 * @param dirty Mark the (new and old) coordinates of the vehicle as dirty.
 */
void Vehicle::UpdateViewport(bool dirty)
{
	/* If the existing cache is invalid we should ignore it, as it will be set to the current coords by UpdateBoundingBoxCoordinates */
	bool ignore_cached_coords = this->sprite_cache.old_coord.left == INVALID_COORD;

	this->UpdateBoundingBoxCoordinates(true);

	if (ignore_cached_coords) {
		UpdateVehicleViewportHash(this, this->coord.left, this->coord.top, INVALID_COORD, INVALID_COORD);
	} else {
		UpdateVehicleViewportHash(this, this->coord.left, this->coord.top, this->sprite_cache.old_coord.left, this->sprite_cache.old_coord.top);
	}

	if (dirty) {
		if (ignore_cached_coords) {
			this->sprite_cache.is_viewport_candidate = this->MarkAllViewportsDirty();
		} else {
			this->sprite_cache.is_viewport_candidate = ::MarkAllViewportsDirty(
				std::min(this->sprite_cache.old_coord.left, this->coord.left),
				std::min(this->sprite_cache.old_coord.top, this->coord.top),
				std::max(this->sprite_cache.old_coord.right, this->coord.right),
				std::max(this->sprite_cache.old_coord.bottom, this->coord.bottom));
		}
	}
}

/**
 * Update the position of the vehicle, and update the viewport.
 */
void Vehicle::UpdatePositionAndViewport()
{
	if (this->type != VEH_EFFECT) this->UpdatePosition();
	this->UpdateViewport(true);
}

/**
 * Marks viewports dirty where the vehicle's image is.
 * @return true if at least one viewport has a dirty block
 */
bool Vehicle::MarkAllViewportsDirty() const
{
	return ::MarkAllViewportsDirty(this->coord.left, this->coord.top, this->coord.right, this->coord.bottom);
}

/**
 * Get position information of a vehicle when moving one pixel in the direction it is facing
 * @param v Vehicle to move
 * @return Position information after the move
 */
GetNewVehiclePosResult GetNewVehiclePos(const Vehicle *v)
{
	static const int8_t _delta_coord[16] = {
		-1,-1,-1, 0, 1, 1, 1, 0, /* x */
		-1, 0, 1, 1, 1, 0,-1,-1, /* y */
	};

	int x = v->x_pos + _delta_coord[v->direction];
	int y = v->y_pos + _delta_coord[v->direction + 8];

	GetNewVehiclePosResult gp;
	gp.x = x;
	gp.y = y;
	gp.old_tile = v->tile;
	gp.new_tile = TileVirtXY(x, y);
	return gp;
}

static const Direction _new_direction_table[] = {
	DIR_N,  DIR_NW, DIR_W,
	DIR_NE, DIR_SE, DIR_SW,
	DIR_E,  DIR_SE, DIR_S
};

Direction GetDirectionTowards(const Vehicle *v, int x, int y)
{
	int i = 0;

	if (y >= v->y_pos) {
		if (y != v->y_pos) i += 3;
		i += 3;
	}

	if (x >= v->x_pos) {
		if (x != v->x_pos) i++;
		i++;
	}

	Direction dir = v->direction;

	DirDiff dirdiff = DirDifference(_new_direction_table[i], dir);
	if (dirdiff == DIRDIFF_SAME) return dir;
	return ChangeDir(dir, dirdiff > DIRDIFF_REVERSE ? DIRDIFF_45LEFT : DIRDIFF_45RIGHT);
}

/**
 * Call the tile callback function for a vehicle entering a tile
 * @param v    Vehicle entering the tile
 * @param tile Tile entered
 * @param x    X position
 * @param y    Y position
 * @return Some meta-data over the to be entered tile.
 * @see VehicleEnterTileStates to see what the bits in the return value mean.
 */
VehicleEnterTileStates VehicleEnterTile(Vehicle *v, TileIndex tile, int x, int y)
{
	return VehicleEnterTileStates{openttd_rust_movement_enter(&GetRustMovementServices(), RustVehicle(v), tile.base(), x, y)};
}

/**
 * Find first unused unit number.
 * This does not mark the unit number as used.
 * @returns First unused unit number.
 */
UnitID FreeUnitIDGenerator::NextID() const
{
	for (auto it = std::begin(this->used_bitmap); it != std::end(this->used_bitmap); ++it) {
		BitmapStorage available = ~(*it);
		if (available == 0) continue;
		return static_cast<UnitID>(std::distance(std::begin(this->used_bitmap), it) * BITMAP_SIZE + FindFirstBit(available) + 1);
	}
	return static_cast<UnitID>(this->used_bitmap.size() * BITMAP_SIZE + 1);
}

/**
 * Use a unit number. If the unit number is not valid it is ignored.
 * @param index Unit number to use.
 * @returns Unit number used.
 */
UnitID FreeUnitIDGenerator::UseID(UnitID index)
{
	if (index == 0 || index == UINT16_MAX) return index;

	index--;

	size_t slot = index / BITMAP_SIZE;
	if (slot >= this->used_bitmap.size()) this->used_bitmap.resize(slot + 1);
	SetBit(this->used_bitmap[index / BITMAP_SIZE], index % BITMAP_SIZE);

	return index + 1;
}

/**
 * Release a unit number. If the unit number is not valid it is ignored.
 * @param index Unit number to release.
 */
void FreeUnitIDGenerator::ReleaseID(UnitID index)
{
	if (index == 0 || index == UINT16_MAX) return;

	index--;

	assert(index / BITMAP_SIZE < this->used_bitmap.size());
	ClrBit(this->used_bitmap[index / BITMAP_SIZE], index % BITMAP_SIZE);
}

/**
 * Get an unused unit number for a vehicle (if allowed).
 * @param type Type of vehicle
 * @return A unused unit number for the given type of vehicle if it is allowed to build one, else \c UINT16_MAX.
 */
UnitID GetFreeUnitNumber(VehicleType type)
{
	/* Check whether it is allowed to build another vehicle. */
	uint max_veh;
	switch (type) {
		case VEH_TRAIN:    max_veh = _settings_game.vehicle.max_trains;   break;
		case VEH_ROAD:     max_veh = _settings_game.vehicle.max_roadveh;  break;
		case VEH_SHIP:     max_veh = _settings_game.vehicle.max_ships;    break;
		case VEH_AIRCRAFT: max_veh = _settings_game.vehicle.max_aircraft; break;
		default: NOT_REACHED();
	}

	const Company *c = Company::Get(_current_company);
	if (c->group_all[type].num_vehicle >= max_veh) return UINT16_MAX; // Currently already at the limit, no room to make a new one.

	return c->freeunits[type].NextID();
}


/**
 * Check whether we can build infrastructure for the given
 * vehicle type. This to disable building stations etc. when
 * you are not allowed/able to have the vehicle type yet.
 * @param type the vehicle type to check this for
 * @return true if there is any reason why you may build
 *         the infrastructure for the given vehicle type
 */
bool CanBuildVehicleInfrastructure(VehicleType type, uint8_t subtype)
{
	assert(IsCompanyBuildableVehicleType(type));

	if (!Company::IsValidID(_local_company)) return false;

	UnitID max;
	switch (type) {
		case VEH_TRAIN:
			if (!HasAnyRailTypesAvail(_local_company)) return false;
			max = _settings_game.vehicle.max_trains;
			break;
		case VEH_ROAD:
			if (!HasAnyRoadTypesAvail(_local_company, (RoadTramType)subtype)) return false;
			max = _settings_game.vehicle.max_roadveh;
			break;
		case VEH_SHIP:     max = _settings_game.vehicle.max_ships; break;
		case VEH_AIRCRAFT: max = _settings_game.vehicle.max_aircraft; break;
		default: NOT_REACHED();
	}

	/* We can build vehicle infrastructure when we may build the vehicle type */
	if (max > 0) {
		/* Can we actually build the vehicle type? */
		for (const Engine *e : Engine::IterateType(type)) {
			if (type == VEH_ROAD && GetRoadTramType(e->VehInfo<RoadVehicleInfo>().roadtype) != (RoadTramType)subtype) continue;
			if (e->company_avail.Test(_local_company)) return true;
		}
		return false;
	}

	/* We should be able to build infrastructure when we have the actual vehicle type */
	for (const Vehicle *v : Vehicle::Iterate()) {
		if (v->type == VEH_ROAD && GetRoadTramType(RoadVehicle::From(v)->roadtype) != (RoadTramType)subtype) continue;
		if (v->owner == _local_company && v->type == type) return true;
	}

	return false;
}


/**
 * Determines the #LiveryScheme for a vehicle.
 * @param engine_type Engine of the vehicle.
 * @param parent_engine_type Engine of the front vehicle, #EngineID::Invalid() if vehicle is at front itself.
 * @param v the vehicle, \c nullptr if in purchase list etc.
 * @return livery scheme to use.
 */
LiveryScheme GetEngineLiveryScheme(EngineID engine_type, EngineID parent_engine_type, const Vehicle *v)
{
	return openttd_rust_engine_livery_scheme(&GetRustCompanyIdentityServices(), engine_type.base(), parent_engine_type.base(), RustVehicle(const_cast<Vehicle *>(v)));
}

/**
 * Determines the livery for a vehicle.
 * @param engine_type EngineID of the vehicle
 * @param company Owner of the vehicle
 * @param parent_engine_type EngineID of the front vehicle. VehicleID::Invalid() if vehicle is at front itself.
 * @param v the vehicle. nullptr if in purchase list etc.
 * @param livery_setting The livery settings to use for acquiring the livery information.
 * @return livery to use
 */
const Livery *GetEngineLivery(EngineID engine_type, CompanyID company, EngineID parent_engine_type, const Vehicle *v, uint8_t livery_setting)
{
	const Company *c = Company::Get(company);
	LiveryScheme scheme = LS_DEFAULT;

	if (livery_setting == LIT_ALL || (livery_setting == LIT_COMPANY && company == _local_company)) {
		if (v != nullptr) {
			const Group *g = Group::GetIfValid(v->First()->group_id);
			if (g != nullptr) {
				/* Traverse parents until we find a livery or reach the top */
				while (!g->livery.in_use.Any({Livery::Flag::Primary, Livery::Flag::Secondary}) && g->parent != GroupID::Invalid()) {
					g = Group::Get(g->parent);
				}
				if (g->livery.in_use.Any({Livery::Flag::Primary, Livery::Flag::Secondary})) return &g->livery;
			}
		}

		/* The default livery is always available for use, but its in_use flag determines
		 * whether any _other_ liveries are in use. */
		if (c->livery[LS_DEFAULT].in_use.Any({Livery::Flag::Primary, Livery::Flag::Secondary})) {
			/* Determine the livery scheme to use */
			scheme = GetEngineLiveryScheme(engine_type, parent_engine_type, v);
		}
	}

	return &c->livery[scheme];
}


static PaletteID GetEngineColourMap(EngineID engine_type, CompanyID company, EngineID parent_engine_type, const Vehicle *v)
{
	PaletteID map = (v != nullptr) ? v->colourmap : PAL_NONE;

	/* Return cached value if any */
	if (map != PAL_NONE) return map;

	const Engine *e = Engine::Get(engine_type);

	/* Check if we should use the colour map callback */
	if (e->info.callback_mask.Test(VehicleCallbackMask::ColourRemap)) {
		uint16_t callback = GetVehicleCallback(CBID_VEHICLE_COLOUR_MAPPING, 0, 0, engine_type, v);
		/* Failure means "use the default two-colour" */
		if (callback != CALLBACK_FAILED) {
			static_assert(PAL_NONE == 0); // Returning 0x4000 (resp. 0xC000) coincidences with default value (PAL_NONE)
			map = GB(callback, 0, 14);
			/* If bit 14 is set, then the company colours are applied to the
			 * map else it's returned as-is. */
			if (!HasBit(callback, 14)) {
				/* Update cache */
				if (v != nullptr) const_cast<Vehicle *>(v)->colourmap = map;
				return map;
			}
		}
	}

	bool twocc = e->info.misc_flags.Test(EngineMiscFlag::Uses2CC);

	if (map == PAL_NONE) map = twocc ? (PaletteID)SPR_2CCMAP_BASE : (PaletteID)PALETTE_RECOLOUR_START;

	/* Spectator has news shown too, but has invalid company ID - as well as dedicated server */
	if (!Company::IsValidID(company)) return map;

	const Livery *livery = GetEngineLivery(engine_type, company, parent_engine_type, v, _settings_client.gui.liveries);

	map += livery->colour1;
	if (twocc) map += livery->colour2 * 16;

	/* Update cache */
	if (v != nullptr) const_cast<Vehicle *>(v)->colourmap = map;
	return map;
}

/**
 * Get the colour map for an engine. This used for unbuilt engines in the user interface.
 * @param engine_type ID of engine
 * @param company ID of company
 * @return A ready-to-use palette modifier
 */
PaletteID GetEnginePalette(EngineID engine_type, CompanyID company)
{
	return GetEngineColourMap(engine_type, company, EngineID::Invalid(), nullptr);
}

/**
 * Get the colour map for a vehicle.
 * @param v Vehicle to get colour map for
 * @return A ready-to-use palette modifier
 */
PaletteID GetVehiclePalette(const Vehicle *v)
{
	if (v->IsGroundVehicle()) {
		return GetEngineColourMap(v->engine_type, v->owner, v->GetGroundVehicleCache()->first_engine, v);
	}

	return GetEngineColourMap(v->engine_type, v->owner, EngineID::Invalid(), v);
}

/**
 * Delete all implicit orders which were not reached.
 */
void Vehicle::DeleteUnreachedImplicitOrders()
{
	openttd_rust_delete_implicit(this, &GetRustOrdersLeaves());
}

/**
 * Prepare everything to begin the loading when arriving at a station.
 * @pre IsTileType(this->tile, MP_STATION) || this->type == VEH_SHIP.
 */

/**
 * Return all reserved cargo packets to the station and reset all packets
 * staged for transfer.
 * @param st the station where the reserved packets should go.
 */
void Vehicle::CancelReservation(StationID next, Station *st)
{
	openttd_rust_visit_cancel_reservation(RustVehicle(this), next.base(), st, &GetRustVehicleVisits());
}

/**
 * Perform all actions when leaving a station.
 * @pre this->current_order.IsType(OT_LOADING)
 */
void Vehicle::LeaveStation()
{
	assert(this->current_order.IsType(OT_LOADING));

	openttd_rust_visit_leave_station(RustVehicle(this), &GetRustVehicleVisits());
}


/**
 * Release the vehicle's unit number.
 */
void Vehicle::ReleaseUnitNumber()
{
	Company::Get(this->owner)->freeunits[this->type].ReleaseID(this->unitnumber);
	this->unitnumber = 0;
}

/**
 * Handle the loading of the vehicle; when not it skips through dummy
 * orders and does nothing in all other cases.
 * @param mode is the non-first call for this vehicle in this tick?
 */

/**
 * Check if the current vehicle has a full load order.
 * @return true Iff this vehicle has a full load order.
 */
bool Vehicle::HasFullLoadOrder() const
{
	return openttd_rust_has_full_load(const_cast<Vehicle *>(this), &GetRustOrdersLeaves()) != 0;
}

/**
 * Check if the current vehicle has a conditional order.
 * @return true Iff this vehicle has a conditional order.
 */
bool Vehicle::HasConditionalOrder() const
{
	return openttd_rust_has_conditional(const_cast<Vehicle *>(this), &GetRustOrdersLeaves()) != 0;
}

/**
 * Check if the current vehicle has an unbunching order.
 * @return true Iff this vehicle has an unbunching order.
 */
bool Vehicle::HasUnbunchingOrder() const
{
	return openttd_rust_has_unbunching(const_cast<Vehicle *>(this), &GetRustOrdersLeaves()) != 0;
}

/**
 * Check if the previous order is a depot unbunching order.
 * @return true Iff the previous order is a depot order with the unbunch flag.
 */

/**
 * Leave an unbunching depot and calculate the next departure time for shared order vehicles.
 */
void Vehicle::LeaveUnbunchingDepot()
{
	openttd_rust_leave_unbunching(this, &GetRustOrdersLeaves());
}

/**
 * Check whether a vehicle inside a depot is waiting for unbunching.
 * @return True if the vehicle must continue waiting, or false if it may try to leave the depot.
 */
bool Vehicle::IsWaitingForUnbunching() const
{
	assert(this->IsInDepot()); return openttd_rust_wait_unbunching(const_cast<Vehicle *>(this), &GetRustOrdersLeaves()) != 0;
};

/**
 * Send this vehicle to the depot using the given command(s).
 * @param flags   the command flags (like execute and such).
 * @param command the command to execute.
 * @return the cost of the depot action.
 */
CommandCost Vehicle::SendToDepot(DoCommandFlags flags, DepotCommandFlags command)
{
	CommandCost ret = CheckOwnership(this->owner);
	if (ret.Failed()) return ret;

	if (this->vehstatus.Test(VehState::Crashed)) return CMD_ERROR;
	if (this->IsStoppedInDepot()) return CMD_ERROR;

	/* No matter why we're headed to the depot, unbunching data is no longer valid. */
	if (flags.Test(DoCommandFlag::Execute)) this->ResetDepotUnbunching();

	if (this->current_order.IsType(OT_GOTO_DEPOT)) {
		bool halt_in_depot = this->current_order.GetDepotActionType().Test(OrderDepotActionFlag::Halt);
		if (command.Test(DepotCommandFlag::Service) == halt_in_depot) {
			/* We called with a different DEPOT_SERVICE setting.
			 * Now we change the setting to apply the new one and let the vehicle head for the same depot.
			 * Note: the if is (true for requesting service == true for ordered to stop in depot)          */
			if (flags.Test(DoCommandFlag::Execute)) {
				this->current_order.SetDepotOrderType({});
				this->current_order.SetDepotActionType(halt_in_depot ? OrderDepotActionFlags{} : OrderDepotActionFlag::Halt);
				SetWindowWidgetDirty(WC_VEHICLE_VIEW, this->index, WID_VV_START_STOP);
			}
			return CommandCost();
		}

		if (command.Test(DepotCommandFlag::DontCancel)) return CMD_ERROR; // Requested no cancellation of depot orders
		if (flags.Test(DoCommandFlag::Execute)) {
			/* If the orders to 'goto depot' are in the orders list (forced servicing),
			 * then skip to the next order; effectively cancelling this forced service */
			if (this->current_order.GetDepotOrderType().Test(OrderDepotTypeFlag::PartOfOrders)) this->IncrementRealOrderIndex();

			if (this->IsGroundVehicle()) {
				uint16_t &gv_flags = this->GetGroundVehicleFlags();
				SetBit(gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS);
			}

			this->current_order.MakeDummy();
			SetWindowWidgetDirty(WC_VEHICLE_VIEW, this->index, WID_VV_START_STOP);
		}
		return CommandCost();
	}

	ClosestDepot closest_depot = this->FindClosestDepot();
	static const StringID no_depot[] = {STR_ERROR_UNABLE_TO_FIND_ROUTE_TO, STR_ERROR_UNABLE_TO_FIND_LOCAL_DEPOT, STR_ERROR_UNABLE_TO_FIND_LOCAL_DEPOT, STR_ERROR_CAN_T_SEND_AIRCRAFT_TO_HANGAR};
	if (!closest_depot.found) return CommandCost(no_depot[this->type]);

	if (flags.Test(DoCommandFlag::Execute)) {
		if (this->current_order.IsType(OT_LOADING)) this->LeaveStation();

		if (this->IsGroundVehicle() && this->GetNumManualOrders() > 0) {
			uint16_t &gv_flags = this->GetGroundVehicleFlags();
			SetBit(gv_flags, GVF_SUPPRESS_IMPLICIT_ORDERS);
		}

		this->SetDestTile(closest_depot.location);
		this->current_order.MakeGoToDepot(closest_depot.destination.ToDepotID(), {});
		if (!command.Test(DepotCommandFlag::Service)) this->current_order.SetDepotActionType(OrderDepotActionFlag::Halt);
		SetWindowWidgetDirty(WC_VEHICLE_VIEW, this->index, WID_VV_START_STOP);

		/* If there is no depot in front and the train is not already reversing, reverse automatically (trains only) */
		if (this->type == VEH_TRAIN && (closest_depot.reverse ^ Train::From(this)->GetTrainFlags().Test(VehicleRailFlag::Reversing))) {
			Command<CMD_REVERSE_TRAIN_DIRECTION>::Do(DoCommandFlag::Execute, this->index, false);
		}

		if (this->type == VEH_AIRCRAFT) {
			Aircraft *a = Aircraft::From(this);
			if (a->state == FLYING && a->targetairport != closest_depot.destination) {
				/* The aircraft is now heading for a different hangar than the next in the orders */
				AircraftNextAirportPos_and_Order(a);
			}
		}
	}

	return CommandCost();

}

/**
 * Update the cached visual effect.
 * @param allow_power_change true if the wagon-is-powered-state may change.
 */
void Vehicle::UpdateVisualEffect(bool allow_power_change)
{
	openttd_rust_visual_bind(&GetRustVisualServices());
	openttd_rust_visual_update(RustVehicle(this), allow_power_change);
}


/**
 * Draw visual effects (smoke and/or sparks) for a vehicle chain.
 * @pre this->IsPrimaryVehicle()
 */
void Vehicle::ShowVisualEffect() const
{
	if (!this->VisualEffectEligible()) return;
	/* Use the speed as limited by underground and orders. */
	this->ShowVisualEffectAtSpeed(this->GetCurrentMaxSpeed());
}

/** Shared assertion and eligibility gate, before any maximum-speed query. */
bool Vehicle::VisualEffectEligible() const
{
	openttd_rust_visual_bind(&GetRustVisualServices());
	return openttd_rust_visual_eligible(RustVehicle(this));
}

/** Draw effects after VisualEffectEligible(), using the owner's current speed limit. */
void Vehicle::ShowVisualEffectAtSpeed(uint max_speed) const
{
	openttd_rust_visual_bind(&GetRustVisualServices());
	openttd_rust_visual_show(RustVehicle(this), max_speed);
}

/**
 * Set the next vehicle of this vehicle.
 * @param next the next vehicle. nullptr removes the next vehicle.
 */
void Vehicle::SetNext(Vehicle *next)
{
	assert(this != next);

	if (this->next != nullptr) {
		/* We had an old next vehicle. Update the first and previous pointers */
		for (Vehicle *v = this->next; v != nullptr; v = v->Next()) {
			v->first = this->next;
		}
		this->next->previous = nullptr;
	}

	this->next = next;

	if (this->next != nullptr) {
		/* A new next vehicle. Update the first and previous pointers */
		if (this->next->previous != nullptr) this->next->previous->next = nullptr;
		this->next->previous = this;
		for (Vehicle *v = this->next; v != nullptr; v = v->Next()) {
			v->first = this->first;
		}
	}
}

/**
 * Adds this vehicle to a shared vehicle chain.
 * @param shared_chain a vehicle of the chain with shared vehicles.
 * @pre !this->IsOrderListShared()
 */
void Vehicle::AddToShared(Vehicle *shared_chain)
{
	openttd_rust_add_shared(this, shared_chain, &GetRustOrdersLeaves());
}

/**
 * Removes the vehicle from the shared order list.
 */
void Vehicle::RemoveFromShared()
{
	openttd_rust_remove_shared(this, &GetRustOrdersLeaves());
}

static const IntervalTimer<TimerGameEconomy> _economy_vehicles_yearly({TimerGameEconomy::YEAR, TimerGameEconomy::Priority::VEHICLE}, [](auto)
{
	for (Vehicle *v : Vehicle::Iterate()) {
		if (v->IsPrimaryVehicle()) {
			/* show warning if vehicle is not generating enough income last 2 years (corresponds to a red icon in the vehicle list) */
			Money profit = v->GetDisplayProfitThisYear();
			if (v->economy_age >= VEHICLE_PROFIT_MIN_AGE && profit < 0) {
				if (_settings_client.gui.vehicle_income_warn && v->owner == _local_company) {
					AddVehicleAdviceNewsItem(AdviceType::VehicleUnprofitable,
						GetEncodedString(TimerGameEconomy::UsingWallclockUnits() ? STR_NEWS_VEHICLE_UNPROFITABLE_PERIOD : STR_NEWS_VEHICLE_UNPROFITABLE_YEAR, v->index, profit),
						v->index);
				}
				AI::NewEvent(v->owner, new ScriptEventVehicleUnprofitable(v->index));
			}

			v->profit_last_year = v->profit_this_year;
			v->profit_this_year = 0;
			SetWindowDirty(WC_VEHICLE_DETAILS, v->index);
		}
	}
	GroupStatistics::UpdateProfits();
	SetWindowClassesDirty(WC_TRAINS_LIST);
	SetWindowClassesDirty(WC_SHIPS_LIST);
	SetWindowClassesDirty(WC_ROADVEH_LIST);
	SetWindowClassesDirty(WC_AIRCRAFT_LIST);
});

/**
 * Can this station be used by the given engine type?
 * @param engine_type the type of vehicles to test
 * @param st the station to test for
 * @return true if and only if the vehicle of the type can use this station.
 * @note For road vehicles the Vehicle is needed to determine whether it can
 *       use the station. This function will return true for road vehicles
 *       when at least one of the facilities is available.
 */
bool CanVehicleUseStation(EngineID engine_type, const Station *st)
{
	return openttd_rust_can_engine_use_station(engine_type.base(), st, &GetAirportResolverServices());
}

/**
 * Can this station be used by the given vehicle?
 * @param v the vehicle to test
 * @param st the station to test for
 * @return true if and only if the vehicle can use this station.
 */
bool CanVehicleUseStation(const Vehicle *v, const Station *st)
{
	return openttd_rust_can_vehicle_use_station(RustVehicle(v), st, &GetAirportResolverServices());
}

/**
 * Get reason string why this station can't be used by the given vehicle.
 * @param v The vehicle to test.
 * @param st The station to test for.
 * @return The string explaining why the vehicle cannot use the station.
 */
StringID GetVehicleCannotUseStationReason(const Vehicle *v, const Station *st)
{
	switch (v->type) {
		case VEH_TRAIN:
			return STR_ERROR_NO_RAIL_STATION;

		case VEH_ROAD: {
			const RoadVehicle *rv = RoadVehicle::From(v);
			RoadStop *rs = st->GetPrimaryRoadStop(rv->IsBus() ? RoadStopType::Bus : RoadStopType::Truck);

			StringID err = rv->IsBus() ? STR_ERROR_NO_BUS_STATION : STR_ERROR_NO_TRUCK_STATION;

			for (; rs != nullptr; rs = rs->next) {
				/* Articulated vehicles cannot use bay road stops, only drive-through. Make sure the vehicle can actually use this bay stop */
				if (HasTileAnyRoadType(rs->xy, rv->compatible_roadtypes) && IsBayRoadStopTile(rs->xy) && rv->HasArticulatedPart()) {
					err = STR_ERROR_NO_STOP_ARTICULATED_VEHICLE;
					continue;
				}

				/* Bay stop errors take precedence, but otherwise the vehicle may not be compatible with the roadtype/tramtype of this station tile.
				 * We give bay stop errors precedence because they are usually a bus sent to a tram station or vice versa. */
				if (!HasTileAnyRoadType(rs->xy, rv->compatible_roadtypes) && err != STR_ERROR_NO_STOP_ARTICULATED_VEHICLE) {
					err = RoadTypeIsRoad(rv->roadtype) ? STR_ERROR_NO_STOP_COMPATIBLE_ROAD_TYPE : STR_ERROR_NO_STOP_COMPATIBLE_TRAM_TYPE;
					continue;
				}
			}

			return err;
		}

		case VEH_SHIP:
			return STR_ERROR_NO_DOCK;

		case VEH_AIRCRAFT:
			if (!st->facilities.Test(StationFacility::Airport)) return STR_ERROR_NO_AIRPORT;
			if (v->GetEngine()->VehInfo<AircraftVehicleInfo>().subtype & AIR_CTOL) {
				return STR_ERROR_AIRPORT_NO_PLANES;
			} else {
				return STR_ERROR_AIRPORT_NO_HELICOPTERS;
			}

		default:
			return INVALID_STRING_ID;
	}
}

/**
 * Access the ground vehicle cache of the vehicle.
 * @pre The vehicle is a #GroundVehicle.
 * @return #GroundVehicleCache of the vehicle.
 */
GroundVehicleCache *Vehicle::GetGroundVehicleCache()
{
	assert(this->IsGroundVehicle());
	if (this->type == VEH_TRAIN) {
		return &Train::From(this)->gcache;
	} else {
		return &RoadVehicle::From(this)->gcache;
	}
}

/**
 * Access the ground vehicle cache of the vehicle.
 * @pre The vehicle is a #GroundVehicle.
 * @return #GroundVehicleCache of the vehicle.
 */
const GroundVehicleCache *Vehicle::GetGroundVehicleCache() const
{
	assert(this->IsGroundVehicle());
	if (this->type == VEH_TRAIN) {
		return &Train::From(this)->gcache;
	} else {
		return &RoadVehicle::From(this)->gcache;
	}
}

/**
 * Access the ground vehicle flags of the vehicle.
 * @pre The vehicle is a #GroundVehicle.
 * @return #GroundVehicleFlags of the vehicle.
 */
uint16_t &Vehicle::GetGroundVehicleFlags()
{
	assert(this->IsGroundVehicle());
	if (this->type == VEH_TRAIN) {
		return Train::From(this)->gv_flags;
	} else {
		return RoadVehicle::From(this)->gv_flags;
	}
}

/**
 * Access the ground vehicle flags of the vehicle.
 * @pre The vehicle is a #GroundVehicle.
 * @return #GroundVehicleFlags of the vehicle.
 */
const uint16_t &Vehicle::GetGroundVehicleFlags() const
{
	assert(this->IsGroundVehicle());
	if (this->type == VEH_TRAIN) {
		return Train::From(this)->gv_flags;
	} else {
		return RoadVehicle::From(this)->gv_flags;
	}
}

/**
 * Calculates the set of vehicles that will be affected by a given selection.
 * @param[in,out] set Set of affected vehicles.
 * @param v First vehicle of the selection.
 * @param num_vehicles Number of vehicles in the selection (not counting articulated parts).
 * @pre \a set must be empty.
 * @post \a set will contain the vehicles that will be refitted.
 */
void GetVehicleSet(VehicleSet &set, Vehicle *v, uint8_t num_vehicles)
{
	openttd_rust_vehicle_selection_set(&GetRustVehicleSelectionServices(), v, num_vehicles, &set, [](void *set, uint32_t id) { include(*static_cast<VehicleSet *>(set), VehicleID(id)); });
}

/**
 * Calculates the maximum weight of the ground vehicle when loaded.
 * @return Weight in tonnes
 */
uint32_t Vehicle::GetDisplayMaxWeight() const
{
	uint32_t max_weight = 0;

	for (const Vehicle *u = this; u != nullptr; u = u->Next()) {
		max_weight += u->GetMaxWeight();
	}

	return max_weight;
}

/**
 * Calculates the minimum power-to-weight ratio using the maximum weight of the ground vehicle
 * @return power-to-weight ratio in 10ths of hp(I) per tonne
 */
uint32_t Vehicle::GetDisplayMinPowerToWeight() const
{
	uint32_t max_weight = GetDisplayMaxWeight();
	if (max_weight == 0) return 0;
	return GetGroundVehicleCache()->cached_power * 10u / max_weight;
}

/**
 * Checks if two vehicle chains have the same list of engines.
 * @param v1 First vehicle chain.
 * @param v1 Second vehicle chain.
 * @return True if same, false if different.
 */
bool VehiclesHaveSameEngineList(const Vehicle *v1, const Vehicle *v2)
{
	while (true) {
		if (v1 == nullptr && v2 == nullptr) return true;
		if (v1 == nullptr || v2 == nullptr) return false;
		if (v1->GetEngine() != v2->GetEngine()) return false;
		v1 = v1->GetNextVehicle();
		v2 = v2->GetNextVehicle();
	}
}

/**
 * Checks if two vehicles have the same list of orders.
 * @param v1 First vehicles.
 * @param v1 Second vehicles.
 * @return True if same, false if different.
 */
bool VehiclesHaveSameOrderList(const Vehicle *v1, const Vehicle *v2)
{
	return std::ranges::equal(v1->Orders(), v2->Orders(), [](const Order &o1, const Order &o2) { return o1.Equals(o2); });
}

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDGroundServices.hpp"
#include "abi-visit-OpenTTDFleetPendingServices.hpp"
#include "abi-visit-OpenTTDVehicleVisits.hpp"
#include "abi-visit-OpenTTDVisualServices.hpp"
bool CheckAbi_vehicle()
{
	return AbiServicesComplete(GetRustGroundServices<Train>()) &&
		AbiServicesComplete(GetRustGroundServices<RoadVehicle>()) &&
		AbiServicesComplete(_fleet_pending_services) &&
		AbiServicesComplete(GetRustVehicleVisits()) && AbiServicesComplete(GetRustVisualServices());
}
