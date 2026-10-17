/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "services_ffi.h"
#include "services_water.h"

/** @file ship_control_services.h Typed synchronous shared-world services. */
#include "settings_services.h"
#include "tile_animation_services.h"
#include "vehicle_services.h"
static_assert(MP_INDUSTRY == 8 && MP_TUNNELBRIDGE == 9 && TILE_HEIGHT == 8 && TILE_SIZE == 16);
static_assert(TRACK_BIT_DEPOT == 128 && TRACK_BIT_WORMHOLE == 64 && HVOT_SHIP == 32);
static_assert(to_underlying(WaterClass::Sea) == 0 && DIRDIFF_REVERSE == 4);
static_assert(static_cast<uint8_t>(VehicleEnterTileState::EnteredWormhole) == 1 && static_cast<uint8_t>(VehicleEnterTileState::CannotEnter) == 2);

static uint32_t Ship_depot_dir(uint32_t tile) noexcept { return GetShipDepotDirection(TileIndex(tile)); }

static uint32_t Ship_depot_axis(uint32_t tile) noexcept { return GetShipDepotAxis(TileIndex(tile)); }

static bool Ship_is_depot(uint32_t tile) noexcept { return IsShipDepotTile(TileIndex(tile)); }

static uint32_t Ship_depot_index(uint32_t tile) noexcept { return GetRustSharedServices().depot_index(tile); }

static bool Ship_wait_unbunch(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return v->IsWaitingForUnbunching(); }

static bool Ship_chain_depot(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return v->IsChainInDepot(); }

static uint32_t Ship_servint(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return Company::Get(v->owner)->settings.vehicle.servint_ships; }

static bool Ship_needs_service(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return v->NeedsAutomaticServicing(); }

static uint32_t Ship_max_distance() noexcept { return RustMaximumDepotPenalty() / YAPF_TILE_LENGTH; }

static bool Ship_tile_valid(uint32_t tile) noexcept { return IsValidTile(TileIndex(tile)); }

static uint32_t Ship_tile_type(uint32_t tile) noexcept { return GetRustSharedServices().tile_type(tile); }

static uint32_t Ship_water_class(uint32_t tile) noexcept { return to_underlying(GetEffectiveWaterClass(TileIndex(tile))); }

static bool Ship_lock_middle(uint32_t tile) noexcept
{
	return IsTileType(TileIndex(tile), MP_WATER) && IsLock(TileIndex(tile)) && GetLockPart(TileIndex(tile)) == LockPart::Middle;
}

static uint32_t Ship_lock_dir(uint32_t tile) noexcept { return GetInclinedSlopeDirection(static_cast<Slope>(GetRustSharedServices().tile_slope(tile))); }

static uint32_t Ship_tile_min_z(uint32_t tile) noexcept { return GetRustSharedServices().tile_z(tile); }

static uint32_t Ship_tile_max_z(uint32_t tile) noexcept { return OpenTTDTileMaxZ(TileIndex(tile)); }

static uint32_t Ship_track_status(uint32_t tile, uint32_t entry_dir) noexcept
{
	return TrackStatusToTrackBits(GetTileTrackStatus(TileIndex(tile), TRANSPORT_WATER, 0, static_cast<DiagDirection>(entry_dir)));
}

static uint32_t Ship_offset(uint32_t direction) noexcept { return static_cast<uint32_t>(TileOffsByDiagDir(static_cast<DiagDirection>(direction))); }

static uint32_t Ship_diag_between(uint32_t from, uint32_t to) noexcept { return DiagdirBetweenTiles(TileIndex(from), TileIndex(to)); }

static uint32_t Ship_dist_square(uint32_t from, uint32_t to) noexcept { return DistanceSquare(TileIndex(from), TileIndex(to)); }

static uint32_t Ship_dist_manhattan(uint32_t from, uint32_t to) noexcept { return DistanceManhattan(TileIndex(from), TileIndex(to)); }


static bool Ship_dock(uint32_t tile) noexcept { return IsDockTile(TileIndex(tile)); }

static bool Ship_dock_water(uint32_t tile) noexcept { return IsDockWaterPart(TileIndex(tile)); }

static uint32_t Ship_station(uint32_t tile) noexcept { return GetRustSharedServices().station_index(tile); }

static uint32_t Ship_industry_station(uint32_t tile) noexcept
{
	const Industry *i = Industry::GetByTile(TileIndex(tile));
	return i->neutral_station == nullptr ? UINT32_MAX : i->neutral_station->index.base();
}

static bool Ship_oilrig(uint32_t tile) noexcept { return IsTileType(TileIndex(tile), MP_STATION) && IsOilRig(TileIndex(tile)); }

static bool Ship_station_use(uint32_t id, uint32_t station) noexcept { Ship *v = Ship::Get(VehicleID(id)); return CanVehicleUseStation(v, Station::Get(StationID(station))); }

static uint32_t Ship_station_xy(uint32_t station) noexcept { return Station::Get(StationID(station))->xy.base(); }

static bool Ship_station_contains(uint32_t station, uint32_t tile) noexcept { return Station::Get(StationID(station))->docking_station.Contains(TileIndex(tile)); }

static bool Ship_station_dock(uint32_t station) noexcept { return Station::Get(StationID(station))->facilities.Test(StationFacility::Dock); }

static uint32_t Ship_station_visits(uint32_t station) noexcept { return Station::Get(StationID(station))->had_vehicle_of_type; }

static void Ship_visit_set(uint32_t station, uint32_t vehicle_type) noexcept
{
	Station::Get(StationID(station))->had_vehicle_of_type |= static_cast<StationHadVehicleOfType>(vehicle_type);
}

static void Ship_arrival(uint32_t id, uint32_t station) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	AddVehicleNewsItem(GetEncodedString(STR_NEWS_FIRST_SHIP_ARRIVAL, StationID(station)), v->owner == _local_company ? NewsType::ArrivalCompany : NewsType::ArrivalOther, v->index, StationID(station));
	AI::NewEvent(v->owner, new ScriptEventStationFirstVehicle(StationID(station), v->index));
	Game::NewEvent(new ScriptEventStationFirstVehicle(StationID(station), v->index));
}

static void Ship_service(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); VehicleServiceInDepot(v); }

static void Ship_leave_unbunch(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->LeaveUnbunchingDepot(); }

static void Ship_path_result(uint32_t id, bool found) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->HandlePathfindingResult(found); }

static void Ship_order_free(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->current_order.Free(); }

static void Ship_order_dummy(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->current_order.MakeDummy(); }

static void Ship_order_depot(uint32_t id, uint32_t depot) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	v->current_order.MakeGoToDepot(DepotID(depot), OrderDepotTypeFlag::Service);
}

static void Ship_order_leave(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->current_order.MakeLeaveStation(); }

static void Ship_order_increment(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->IncrementRealOrderIndex(); }

static void Ship_timetable(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); UpdateVehicleTimetable(v, true); }

static void Ship_position(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->UpdatePosition(); }

static void Ship_start_dirty(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); SetWindowWidgetDirty(WC_VEHICLE_VIEW, v->index, WID_VV_START_STOP); }

static void Ship_depot_dirty(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); SetWindowDirty(WC_VEHICLE_DEPOT, v->tile); }

static void Ship_depot_invalidate(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); InvalidateWindowData(WC_VEHICLE_DEPOT, v->tile); }

static void Ship_ships_dirty() noexcept { SetWindowClassesDirty(WC_SHIPS_LIST); }

static void Ship_details_dirty(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); SetWindowDirty(WC_VEHICLE_DETAILS, v->index); }

static void Ship_age(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); AgeVehicle(v); }

static void Ship_economy_age(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); EconomyAgeVehicle(v); }

static void Ship_decrease_value(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); DecreaseVehicleValue(v); }

static void Ship_check_breakdown(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); CheckVehicleBreakdown(v); }

static void Ship_check_orders(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); CheckOrders(v); }

static int64_t Ship_running_cost(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return static_cast<int64_t>(v->GetRunningCost()); }

static uint32_t Ship_cost_divisor() noexcept { return CalendarTime::DAYS_IN_YEAR * Ticks::DAY_TICKS; }

static void Ship_pay_running(uint32_t id, int64_t amount) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	CommandCost cost(EXPENSES_SHIP_RUN, Money(amount));
	v->profit_this_year -= cost.GetCost();
	v->running_ticks = 0;
	SubtractMoneyFromCompanyFract(v->owner, cost);
}

static uint32_t Ship_speed_default(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return ShipVehInfo(v->engine_type)->max_speed; }

static uint32_t Ship_age_default(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return EngInfo(v->engine_type)->cargo_age_period; }

static uint32_t Ship_speed_frac(uint32_t id, bool sea) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	return sea ? ShipVehInfo(v->engine_type)->ocean_speed_frac : ShipVehInfo(v->engine_type)->canal_speed_frac;
}

static uint32_t Ship_speed_property(uint32_t id, uint32_t default_speed) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	return GetVehicleProperty(v, PROP_SHIP_SPEED, default_speed);
}
static uint32_t Ship_age_property(uint32_t id, uint32_t default_period) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	return GetVehicleProperty(v, PROP_SHIP_CARGO_AGE_PERIOD, default_period);
}

static void Ship_update_visual(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->UpdateVisualEffect(); }

static void Ship_cache_invalidate(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->InvalidateNewGRFCacheOfChain(); }

static uint32_t Ship_capacity(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return v->GetEngine()->DetermineCapacity(v); }

static void Ship_sprite_direction(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->sprite_cache.last_direction = INVALID_DIR; }

static uint32_t Ship_tile_x(uint32_t tile) noexcept { return TileX(TileIndex(tile)); }

static uint32_t Ship_tile_y(uint32_t tile) noexcept { return TileY(TileIndex(tile)); }

static bool Ship_build_flag(uint32_t engine) noexcept { return Engine::Get(EngineID(engine))->flags.Test(EngineFlag::ExclusivePreview); }

static void Ship_build_random(uint32_t id, uint16_t random) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->random_bits = random; }

static OpenTTDShipPosition Ship_new_position(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); auto gp = GetNewVehiclePos(v); return {gp.x, gp.y, gp.old_tile.base(), gp.new_tile.base()}; }

static uint32_t Ship_exit_dir(uint32_t direction, uint32_t state) noexcept { return VehicleExitDir(static_cast<Direction>(direction), static_cast<TrackBits>(state)); }

static uint32_t Ship_track_direction(uint32_t track, uint32_t direction) noexcept
{
	return TrackDirectionToTrackdir(static_cast<Track>(track), static_cast<Direction>(direction));
}

static uint32_t Ship_tracks_reach(uint32_t direction) noexcept { return DiagdirReachesTracks(static_cast<DiagDirection>(direction)); }

static bool Ship_busy_tile(uint32_t id) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	return HasVehicleOnTile(v->tile, [](const Vehicle *u) { return u->type == VEH_SHIP && u->cur_speed != 0; });
}

static size_t Ship_path_size(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return v->path.size(); }

static uint32_t Ship_path_back(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return v->path.back().trackdir; }

static void Ship_path_pop(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->path.pop_back(); }

static void Ship_path_clear(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->path.clear(); }

static uint32_t Ship_enter_tile(uint32_t id, uint32_t tile, uint32_t x, uint32_t y) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	return VehicleEnterTile(v, TileIndex(tile), static_cast<int32_t>(x), static_cast<int32_t>(y)).base();
}

static void Ship_enter_depot(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); VehicleEnterDepot(v); }

static bool Ship_process_orders(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return ProcessOrders(v); }

static void Ship_loading(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->HandleLoading(); }

static void Ship_begin_loading(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->BeginLoading(); }

static bool Ship_breakdown(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); return v->HandleBreakdown(); }

static void Ship_viewport(uint32_t id, bool update_delta, bool force) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->UpdateViewport(update_delta, force); }

static void Ship_base_viewport(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->Vehicle::UpdateViewport(true); }

static void Ship_visual(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->ShowVisualEffect(); }

static void Ship_cache(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->UpdateCache(); }

static void Ship_play_sound(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->PlayLeaveStationSound(); }

static OpenTTDShipReverseResult Ship_yapf_reverse(uint32_t id, bool return_trackdir) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	Trackdir dir = INVALID_TRACKDIR;
	bool reverse = YapfShipCheckReverse(v, return_trackdir ? &dir : nullptr);
	return {reverse, static_cast<uint8_t>(dir)};
}

static OpenTTDShipTrackResult Ship_yapf_choose(uint32_t id, uint32_t tile) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	bool found = true;
	Track track = YapfShipChooseTrack(v, TileIndex(tile), found, v->path);
	return {static_cast<uint8_t>(track), found};
}

static void Ship_update_delta(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->UpdateDeltaXY(); }

static void Ship_build_owner(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->owner = _current_company; }

static void Ship_build_z(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->z_pos = GetSlopePixelZ(v->x_pos, v->y_pos); }

static void Ship_build_properties(uint32_t id, uint32_t engine) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	const Engine *e = Engine::Get(EngineID(engine));
	const auto &svi = e->VehInfo<ShipVehicleInfo>();
	v->vehstatus = {VehState::Hidden, VehState::Stopped, VehState::DefaultPalette};
	v->spritenum = svi.image_index;
	v->cargo_type = e->GetDefaultCargoType();
	assert(IsValidCargoType(v->cargo_type));
	v->cargo_cap = svi.capacity;
	v->refit_cap = 0;
	v->last_station_visited = StationID::Invalid();
	v->last_loading_station = StationID::Invalid();
	v->engine_type = e->index;
	v->reliability = e->reliability;
	v->reliability_spd_dec = e->reliability_spd_dec;
	v->max_age = e->GetLifeLengthInDays();
}

static void Ship_build_dates(uint32_t id) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	v->SetServiceInterval(Company::Get(_current_company)->settings.vehicle.servint_ships);
	v->date_of_last_service = TimerGameEconomy::date;
	v->date_of_last_service_newgrf = TimerGameCalendar::date;
	v->build_year = TimerGameCalendar::year;
	v->sprite_cache.sprite_seq.Set(SPR_IMG_QUERY);
}

static void Ship_build_acceleration(uint32_t id, uint32_t engine) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	const auto &svi = Engine::Get(EngineID(engine))->VehInfo<ShipVehicleInfo>();
	v->acceleration = svi.acceleration;
}

static void Ship_build_prototype(uint32_t id) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->vehicle_flags.Set(VehicleFlag::BuiltAsPrototype); }

static void Ship_build_interval_percent(uint32_t id) noexcept
{
	Ship *v = Ship::Get(VehicleID(id));
	v->SetServiceIntervalIsPercent(Company::Get(_current_company)->settings.vehicle.servint_ispercent);
}

static void Ship_build_capacity(uint32_t id, uint32_t capacity) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->cargo_cap = static_cast<uint16_t>(capacity); }

static void Ship_set_tile(uint32_t id, uint32_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->tile = TileIndex(static_cast<uint32_t>(value)); }

static void Ship_set_x(uint32_t id, int32_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->x_pos = static_cast<int32_t>(value); }

static void Ship_set_y(uint32_t id, int32_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->y_pos = static_cast<int32_t>(value); }

static void Ship_set_z(uint32_t id, int32_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->z_pos = static_cast<int32_t>(value); }

static void Ship_set_direction(uint32_t id, uint8_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->direction = static_cast<Direction>(value); }

static void Ship_set_speed(uint32_t id, uint16_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->cur_speed = static_cast<uint16_t>(value); }

static void Ship_set_tick(uint32_t id, uint8_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); RustWriteVehicleTick(v, static_cast<uint8_t>(value)); }

static void Ship_set_running(uint32_t id, uint8_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->running_ticks = static_cast<uint8_t>(value); }

static void Ship_set_day(uint32_t id, uint8_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->day_counter = static_cast<uint8_t>(value); }

static void Ship_set_order_time(uint32_t id, int32_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->current_order_time = static_cast<int32_t>(value); }

static void Ship_set_progress(uint32_t id, uint8_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); RustWriteVehicleProgress(v, static_cast<uint8_t>(value)); }

static void Ship_set_last_station(uint32_t id, uint16_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->last_station_visited = StationID(static_cast<uint16_t>(value)); }

static void Ship_set_hidden(uint32_t id, bool value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->vehstatus.Set(VehState::Hidden, value); }

static void Ship_set_max_speed(uint32_t id, uint16_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->vcache.cached_max_speed = static_cast<uint16_t>(value); }

static void Ship_set_cargo_age(uint32_t id, uint16_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->vcache.cached_cargo_age_period = static_cast<uint16_t>(value); }

static void Ship_set_dest(uint32_t id, uint32_t value) noexcept { Ship *v = Ship::Get(VehicleID(id)); v->dest_tile = TileIndex(static_cast<uint32_t>(value)); }

static uint32_t Ship_tile(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleTile(v)); }

static uint32_t Ship_dest(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleDestTile(v)); }

static uint32_t Ship_x(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->x_pos); }

static uint32_t Ship_y(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->y_pos); }

static uint32_t Ship_z(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->z_pos); }

static uint32_t Ship_direction(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleDirection(v)); }

static uint32_t Ship_speed(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->cur_speed); }

static uint32_t Ship_tick(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleTick(v)); }

static uint32_t Ship_running(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->running_ticks); }

static uint32_t Ship_day(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->day_counter); }

static uint32_t Ship_order_time(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->current_order_time); }

static uint32_t Ship_progress(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleProgress(v)); }

static uint32_t Ship_status(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleStatus(v)); }

static uint32_t Ship_owner(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleOwner(v)); }

static uint32_t Ship_last_station(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->last_station_visited.base()); }

static uint32_t Ship_order_destination(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleOrderDestination(v)); }

static uint32_t Ship_order_type(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(RustVehicleOrderType(v)); }

static uint32_t Ship_order_max_speed(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->current_order.GetMaxSpeed()); }

static uint32_t Ship_acceleration(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->acceleration); }

static uint32_t Ship_max_speed(uint32_t id) noexcept { const Ship *v = Ship::Get(VehicleID(id)); return static_cast<uint32_t>(v->vcache.cached_max_speed); }

static OpenTTDShipState *Ship_state_owner(uint32_t id) noexcept { return Ship::Get(VehicleID(id))->GetRustState(); }
static OpenTTDWaterPatch Ship_patch(uint32_t tile) noexcept { auto p = GetWaterRegionPatchInfo(TileIndex(tile)); return {p.x, p.y, p.label.base()}; }
static size_t Ship_neighbours(OpenTTDWaterPatch patch, OpenTTDWaterPatch *out) noexcept
{
	size_t count = 0;
	VisitWaterRegionPatchCallback visitor = [&](const WaterRegionPatchDesc &p) { out[count++] = {p.x, p.y, p.label.base()}; };
	VisitWaterRegionPatchNeighbours({patch.x, patch.y, WaterRegionPatchLabel(patch.label)}, visitor);
	return count;
}
static bool Ship_next_depot(uint32_t first, OpenTTDShipDepot *out) noexcept
{
	for (const Depot *d : Depot::Iterate(first)) {
		*out = {d->index.base(), d->xy.base(), IsShipDepotTile(d->xy) ? GetRustSharedServices().tile_owner(d->xy.base()) : UINT32_MAX, static_cast<uint32_t>(IsShipDepotTile(d->xy))};
		return true;
	}
	return false;
}
static const OpenTTDShipLeaves &GetShipServices()
{
	static const OpenTTDShipLeaves leaves{
		.depot_dir = Ship_depot_dir, .depot_axis = Ship_depot_axis,
		.is_depot = Ship_is_depot, .depot_index = Ship_depot_index,
		.wait_unbunch = Ship_wait_unbunch, .chain_depot = Ship_chain_depot,
		.servint = Ship_servint, .needs_service = Ship_needs_service,
		.max_distance = Ship_max_distance, .tile_valid = Ship_tile_valid,
		.tile_type = Ship_tile_type, .water_class = Ship_water_class,
		.lock_middle = Ship_lock_middle, .lock_dir = Ship_lock_dir,
		.tile_min_z = Ship_tile_min_z, .tile_max_z = Ship_tile_max_z,
		.track_status = Ship_track_status, .offset = Ship_offset,
		.diag_between = Ship_diag_between, .dist_square = Ship_dist_square,
		.dist_manhattan = Ship_dist_manhattan, .docking = OpenTTDIsDockingTile,
		.dock = Ship_dock, .dock_water = Ship_dock_water,
		.station = Ship_station, .industry_station = Ship_industry_station,
		.oilrig = Ship_oilrig, .station_use = Ship_station_use,
		.station_xy = Ship_station_xy, .station_contains = Ship_station_contains,
		.station_dock = Ship_station_dock, .station_visits = Ship_station_visits,
		.visit_set = Ship_visit_set, .arrival = Ship_arrival,
		.service = Ship_service, .leave_unbunch = Ship_leave_unbunch,
		.path_result = Ship_path_result, .order_free = Ship_order_free,
		.order_dummy = Ship_order_dummy, .order_depot = Ship_order_depot,
		.order_leave = Ship_order_leave, .order_increment = Ship_order_increment,
		.timetable = Ship_timetable, .position = Ship_position,
		.start_dirty = Ship_start_dirty, .depot_dirty = Ship_depot_dirty,
		.depot_invalidate = Ship_depot_invalidate, .ships_dirty = Ship_ships_dirty,
		.details_dirty = Ship_details_dirty, .age = Ship_age,
		.economy_age = Ship_economy_age, .decrease_value = Ship_decrease_value,
		.check_breakdown = Ship_check_breakdown, .check_orders = Ship_check_orders,
		.running_cost = Ship_running_cost, .cost_divisor = Ship_cost_divisor,
		.pay_running = Ship_pay_running, .speed_default = Ship_speed_default,
		.age_default = Ship_age_default, .speed_frac = Ship_speed_frac,
		.speed_property = Ship_speed_property, .age_property = Ship_age_property,
		.update_visual = Ship_update_visual, .cache_invalidate = Ship_cache_invalidate,
		.capacity = Ship_capacity, .sprite_direction = Ship_sprite_direction,
		.tile_x = Ship_tile_x, .tile_y = Ship_tile_y,
		.build_flag = Ship_build_flag, .build_random = Ship_build_random,
		.new_position = Ship_new_position, .exit_dir = Ship_exit_dir,
		.track_direction = Ship_track_direction, .tracks_reach = Ship_tracks_reach,
		.busy_tile = Ship_busy_tile, .path_size = Ship_path_size,
		.path_back = Ship_path_back, .path_pop = Ship_path_pop,
		.path_clear = Ship_path_clear, .enter_tile = Ship_enter_tile,
		.enter_depot = Ship_enter_depot, .process_orders = Ship_process_orders,
		.loading = Ship_loading, .begin_loading = Ship_begin_loading,
		.breakdown = Ship_breakdown, .viewport = Ship_viewport,
		.base_viewport = Ship_base_viewport, .visual = Ship_visual,
		.cache = Ship_cache, .play_sound = Ship_play_sound,
		.yapf_reverse = Ship_yapf_reverse, .yapf_choose = Ship_yapf_choose,
		.update_delta = Ship_update_delta, .build_owner = Ship_build_owner,
		.build_z = Ship_build_z, .build_properties = Ship_build_properties,
		.build_dates = Ship_build_dates, .build_acceleration = Ship_build_acceleration,
		.build_prototype = Ship_build_prototype, .build_interval_percent = Ship_build_interval_percent,
		.build_capacity = Ship_build_capacity, .set_tile = Ship_set_tile,
		.set_x = Ship_set_x, .set_y = Ship_set_y,
		.set_z = Ship_set_z, .set_direction = Ship_set_direction,
		.set_speed = Ship_set_speed, .set_tick = Ship_set_tick,
		.set_running = Ship_set_running, .set_day = Ship_set_day,
		.set_order_time = Ship_set_order_time, .set_progress = Ship_set_progress,
		.set_last_station = Ship_set_last_station, .set_hidden = Ship_set_hidden,
		.set_max_speed = Ship_set_max_speed, .set_cargo_age = Ship_set_cargo_age,
		.set_dest = Ship_set_dest, .tile = Ship_tile,
		.dest = Ship_dest, .x = Ship_x,
		.y = Ship_y, .z = Ship_z,
		.direction = Ship_direction, .speed = Ship_speed,
		.tick = Ship_tick, .running = Ship_running,
		.day = Ship_day, .order_time = Ship_order_time,
		.progress = Ship_progress, .status = Ship_status,
		.owner = Ship_owner, .last_station = Ship_last_station,
		.order_destination = Ship_order_destination, .order_type = Ship_order_type,
		.order_max_speed = Ship_order_max_speed, .acceleration = Ship_acceleration,
		.max_speed = Ship_max_speed, .state_owner = Ship_state_owner,
		.patch = Ship_patch, .neighbours = Ship_neighbours,
		.next_depot = Ship_next_depot
	};
	return leaves;
}
