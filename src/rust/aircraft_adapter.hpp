/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file aircraft_adapter.hpp Canonical pool observations and direct shared leaves. */
#include "aircraft_ffi.h"
#include "services_assert.h"
#include "vehicle_services.h"
#include "engine_services.h"
#include "services_map.h"
#include "services_station.h"
#include "company_services.h"
#include "vehicle_transaction_ffi.h"
/* The Rust shared definitions these values name: airport.rs, sound_type.rs, aircraft.rs. */
static_assert(ORIGINAL_SAMPLE_COUNT == 73 && MAX_TERMINALS == 8 && TERMGROUP == 255 && ROTOR_Z_OFFSET == 5);
static_assert(HRS_ROTOR_STOPPED == 0 && HRS_ROTOR_MOVING_1 == 1 && HRS_ROTOR_MOVING_3 == 3);
static_assert(to_underlying(AirportMovingDataFlag::NoSpeedClamp) == 0 && to_underlying(AirportMovingDataFlag::Takeoff) == 1 && to_underlying(AirportMovingDataFlag::Land) == 3);
static_assert(to_underlying(AirportBlock::RunwayIn) == 8 && to_underlying(AirportBlock::RunwayInOut) == 8 && to_underlying(AirportBlock::RunwayIn2) == 29);
static_assert(to_underlying(AirportBlock::Term7) == 22 && to_underlying(AirportBlock::Term8) == 23 && to_underlying(AirportBlock::Helipad3) == 24);
static_assert(to_underlying(AirportBlock::Nothing) == 30 && to_underlying(AirportBlock::Zeppeliner) == 62 && to_underlying(AirportBlock::AirportClosed) == 63);

/* Resolve each live shell/private owner once. Disaster helpers retain null State.
 * The record contains opaque pointers only; Rust never views C++ object layout. */

static uint64_t * OPENTTD_RUST_CALL Aircraft_airport_blocks(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return st->airport.rust_blocks.get();
}

static uint8_t OPENTTD_RUST_CALL Aircraft_had_vehicle(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return st->had_vehicle_of_type;
}

static uint8_t OPENTTD_RUST_CALL Aircraft_airport_elements(const void * airport) noexcept
{
	const auto *ap = static_cast<const AirportFTAClass *>(airport);
	return ap->nofelements;
}

static uint8_t OPENTTD_RUST_CALL Aircraft_helipads(const void * airport) noexcept
{
	const auto *ap = static_cast<const AirportFTAClass *>(airport);
	return ap->num_helipads;
}

static const void * OPENTTD_RUST_CALL Aircraft_node(const void * airport, uint8_t pos) noexcept
{
	return &static_cast<const AirportFTAClass *>(airport)->layout[pos];
}

static const void * OPENTTD_RUST_CALL Aircraft_fta_next(const void *node) noexcept { return static_cast<const AirportFTA *>(node)->next.get(); }
static uint64_t OPENTTD_RUST_CALL Aircraft_fta_blocks(const void *node) noexcept { return static_cast<const AirportFTA *>(node)->blocks.base(); }
static uint8_t OPENTTD_RUST_CALL Aircraft_fta_position(const void *node) noexcept { return static_cast<const AirportFTA *>(node)->position; }
static uint8_t OPENTTD_RUST_CALL Aircraft_fta_next_position(const void *node) noexcept { return static_cast<const AirportFTA *>(node)->next_position; }
static uint8_t OPENTTD_RUST_CALL Aircraft_fta_heading(const void *node) noexcept { return static_cast<const AirportFTA *>(node)->heading; }

static OpenTTDAircraftMoving OPENTTD_RUST_CALL Aircraft_moving(const void * airport, uint8_t pos) noexcept
{
	const auto *m = static_cast<const AirportFTAClass *>(airport)->MovingData(pos);
	return {.x = m->x, .y = m->y, .flags = RustAirportMovingFlags(m), .direction = m->direction};
}

static uint8_t OPENTTD_RUST_CALL Aircraft_airport_entry(const void * ap, uint8_t direction) noexcept
{
	return static_cast<const AirportFTAClass *>(ap)->entry_points[direction];
}

static uint8_t OPENTTD_RUST_CALL Aircraft_terminal_count(const void * ap, uint32_t index) noexcept
{
	return static_cast<const AirportFTAClass *>(ap)->terminals[index];
}

static uint8_t OPENTTD_RUST_CALL Aircraft_hangar_exit(const OpenTTDStation *station, uint32_t tile, const AirportSpec *spec) noexcept
{
	return StationOf(station)->airport.GetHangarExitDirection(TileIndex{tile}, spec);
}

static uint16_t OPENTTD_RUST_CALL Aircraft_service_interval(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return RustCompanyAircraftServiceInterval(v->GetOwner());
}

static OpenTTDAircraftVehicle OPENTTD_RUST_CALL Aircraft_next_aircraft(uint32_t last) noexcept
{
	for (Aircraft *v : Aircraft::Iterate(last == VehicleID::Invalid().base() ? 0 : last + 1)) return RustAircraftVehicle(v);
	return RustAircraftVehicle(nullptr);
}

static void OPENTTD_RUST_CALL Aircraft_viewport(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); RustVehicleUpdateViewport(v, true, false);
}

static void OPENTTD_RUST_CALL Aircraft_rotor_image(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); GetRotorImage(v, EIT_ON_MAP, &v->Next()->Next()->sprite_cache.sprite_seq);
}

static void OPENTTD_RUST_CALL Aircraft_copy_sprite(OpenTTDVehicle *vehicle, OpenTTDVehicle *source) noexcept
{
	VehicleOf(vehicle)->sprite_cache.sprite_seq.CopyWithoutPalette(VehicleOf(source)->sprite_cache.sprite_seq);
}

static void OPENTTD_RUST_CALL Aircraft_truncate_cargo(OpenTTDVehicle *vehicle) noexcept
{
	VehicleOf(vehicle)->cargo.Truncate();
}

static void OPENTTD_RUST_CALL Aircraft_crash_news(OpenTTDAircraftVehicle vehicle, uint16_t station_id, uint32_t victims) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); StationID station{station_id};
	TileIndex vt = TileVirtXYClampedToMap(RustVehicleX(v), RustVehicleY(v)); bool missing = station == StationID::Invalid();
	EncodedString headline = missing ? GetEncodedString(STR_NEWS_PLANE_CRASH_OUT_OF_FUEL, victims) : GetEncodedString(STR_NEWS_AIRCRAFT_CRASH, victims, station);
	AI::NewEvent(v->GetOwner(), new ScriptEventVehicleCrashed(v->index, vt, missing ? ScriptEventVehicleCrashed::CRASH_AIRCRAFT_NO_AIRPORT : ScriptEventVehicleCrashed::CRASH_PLANE_LANDING, victims, v->GetOwner()));
	Game::NewEvent(new ScriptEventVehicleCrashed(v->index, vt, missing ? ScriptEventVehicleCrashed::CRASH_AIRCRAFT_NO_AIRPORT : ScriptEventVehicleCrashed::CRASH_PLANE_LANDING, victims, v->GetOwner()));
	AddTileNewsItem(std::move(headline), v->GetOwner() == _local_company ? NewsType::Accident : NewsType::AccidentOther, vt, station);
}

static void OPENTTD_RUST_CALL Aircraft_landing_rating(uint16_t station_id, uint8_t cargo) noexcept
{
	GoodsEntry &ge = Station::Get(station_id)->goods[cargo]; ge.rating = 1; if (ge.HasData()) ge.GetData().cargo.Truncate();
}

static void OPENTTD_RUST_CALL Aircraft_dirty_depot(OpenTTDAircraftVehicle vehicle) noexcept
{
	RustVehicleInvalidateDepot(VehicleOf(vehicle.handle)); SetWindowClassesDirty(WC_AIRCRAFT_LIST);
}

static void OPENTTD_RUST_CALL Aircraft_first_arrival(OpenTTDAircraftVehicle vehicle, uint16_t station) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); Station *st = Station::Get(station);
	/* show newsitem of celebrating citizens */
	AddVehicleNewsItem(GetEncodedString(STR_NEWS_FIRST_AIRCRAFT_ARRIVAL, st->index), v->GetOwner() == _local_company ? NewsType::ArrivalCompany : NewsType::ArrivalOther, v->index, st->index);
	AI::NewEvent(v->GetOwner(), new ScriptEventStationFirstVehicle(st->index, v->index)); Game::NewEvent(new ScriptEventStationFirstVehicle(st->index, v->index));
}

static void OPENTTD_RUST_CALL Aircraft_destination_too_far(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); AI::NewEvent(v->GetOwner(), new ScriptEventAircraftDestTooFar(v->index));
	if (v->GetOwner() == _local_company) AddVehicleAdviceNewsItem(AdviceType::AircraftDestinationTooFar, GetEncodedString(STR_NEWS_AIRCRAFT_DEST_TOO_FAR, v->index), v->index);
}

static void OPENTTD_RUST_CALL Aircraft_delete_range_news(OpenTTDAircraftVehicle vehicle) noexcept
{
	DeleteVehicleNews(VehicleID{vehicle.id}, AdviceType::AircraftDestinationTooFar);
}

static void OPENTTD_RUST_CALL Aircraft_service_order(OpenTTDAircraftVehicle vehicle, uint16_t station) noexcept
{
	Aircraft::From(VehicleOf(vehicle.handle))->current_order.MakeGoToDepot(StationID{static_cast<uint16_t>(station)}, OrderDepotTypeFlag::Service);
}

static void OPENTTD_RUST_CALL Aircraft_subtract_cost(OpenTTDAircraftVehicle vehicle, int64_t cost) noexcept
{
	SubtractMoneyFromCompanyFract(VehicleOf(vehicle.handle)->GetOwner(), CommandCost(EXPENSES_AIRCRAFT_RUN, Money{cost}));
}

static void OPENTTD_RUST_CALL Aircraft_dirty_lists(OpenTTDAircraftVehicle vehicle) noexcept
{
	RustVehicleDetailsDirty(vehicle.handle); SetWindowClassesDirty(WC_AIRCRAFT_LIST);
}

static bool OPENTTD_RUST_CALL Aircraft_rotor_image_if_changed(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); Aircraft *rotor = v->Next()->Next(); VehicleSpriteSeq seq;
	GetRotorImage(v, EIT_ON_MAP, &seq); if (rotor->sprite_cache.sprite_seq == seq) return true; rotor->sprite_cache.sprite_seq = seq; return false;
}

static void OPENTTD_RUST_CALL Aircraft_update_rotor_image(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); Aircraft *rotor = v->Next()->Next(); VehicleSpriteSeq seq;
	GetRotorImage(v, EIT_ON_MAP, &seq); rotor->sprite_cache.sprite_seq = seq;
}

static const OpenTTDAircraftLeaves _aircraft_leaves{
	.airport_blocks = Aircraft_airport_blocks,
	.airport_delta_z = OpenTTDAirportDeltaZ,
	.airport_elements = Aircraft_airport_elements,
	.airport_entry = Aircraft_airport_entry,
	.airport_flags = OpenTTDAirportFlags,
	.airport_height = Aircraft_airport_height,
	.airport_tile = Aircraft_airport_tile,
	.airport_type = Aircraft_airport_type,
	.airport_width = Aircraft_airport_width,
	.copy_sprite = Aircraft_copy_sprite,
	.crash_news = Aircraft_crash_news,
	.delete_range_news = Aircraft_delete_range_news,
	.destination_too_far = Aircraft_destination_too_far,
	.direction_towards = RustVehicleHandleInvoke<RustVehicleDirectionTowards, OpenTTDAircraftVehicle, int32_t, int32_t>,
	.dirty_depot = Aircraft_dirty_depot,
	.dirty_lists = Aircraft_dirty_lists,
	.first_arrival = Aircraft_first_arrival,
	.fta_blocks = Aircraft_fta_blocks,
	.fta_heading = Aircraft_fta_heading,
	.fta_next = Aircraft_fta_next,
	.fta_next_position = Aircraft_fta_next_position,
	.fta_position = Aircraft_fta_position,
	.had_vehicle = Aircraft_had_vehicle,
	.hangar_exit = Aircraft_hangar_exit,
	.helipads = Aircraft_helipads,
	.landing_rating = Aircraft_landing_rating,
	.mass = &GetVehicleMassServices(),
	.moving = Aircraft_moving,
	.next_aircraft = Aircraft_next_aircraft,
	.node = Aircraft_node,
	.rotation = Aircraft_rotation,
	.rotor_image = Aircraft_rotor_image,
	.rotor_image_if_changed = Aircraft_rotor_image_if_changed,
	.service_interval = Aircraft_service_interval,
	.service_order = Aircraft_service_order,
	.shared = &GetRustSharedServices(),
	.station = Aircraft_station,
	.station_owner = Aircraft_station_owner,
	.station_tile = Aircraft_station_tile,
	.subtract_cost = Aircraft_subtract_cost,
	.terminal_count = Aircraft_terminal_count,
	.truncate_cargo = Aircraft_truncate_cargo,
	.update_rotor_image = Aircraft_update_rotor_image,
	.vehicle = &GetRustVehicleServices(),
	.vehicle_crash = RustVehicleBaseCrashHandle<OpenTTDAircraftVehicle>,
	.viewport = Aircraft_viewport,
};
const OpenTTDAircraftLeaves &GetRustAircraftLeaves() noexcept { return _aircraft_leaves; }

bool Aircraft::Tick()
{
	if (!this->IsNormalAircraft()) return true;
	PerformanceAccumulator framerate(PFE_GL_AIRCRAFT);
	return openttd_rust_aircraft_tick(&_aircraft_leaves, RustAircraftVehicle(this));
}
void Aircraft::OnNewCalendarDay() { openttd_rust_aircraft_calendar_day(&_aircraft_leaves, RustAircraftVehicle(this)); }
void Aircraft::OnNewEconomyDay() { openttd_rust_aircraft_economy_day(&_aircraft_leaves, RustAircraftVehicle(this)); }
void SetAircraftPosition(Aircraft *v, int x, int y, int z) { openttd_rust_aircraft_position(&_aircraft_leaves, RustAircraftVehicle(v), x, y, z); }
void UpdateAircraftCache(Aircraft *v, bool update_range) { openttd_rust_aircraft_cache(&_aircraft_leaves, RustAircraftVehicle(v), update_range); }
void GetAircraftFlightLevelBounds(const Vehicle *v, int *min, int *max) { openttd_rust_aircraft_flight_bounds(&_aircraft_leaves, RustAircraftVehicle(const_cast<Vehicle *>(v)), min, max); }
static uint8_t &GetFlightFlags(Aircraft *v) { return v->flags; }
static uint8_t &GetFlightFlags(DisasterVehicle *v) { return v->Flags(); }
template <class T> int GetAircraftFlightLevel(T *v, bool takeoff) { return openttd_rust_aircraft_flight_level(&_aircraft_leaves, RustAircraftVehicle(v), &GetFlightFlags(v), takeoff); }
template int GetAircraftFlightLevel(DisasterVehicle *v, bool takeoff);
template int GetAircraftFlightLevel(Aircraft *v, bool takeoff);
int GetTileHeightBelowAircraft(const Vehicle *v) { return openttd_rust_aircraft_height(&_aircraft_leaves, RustAircraftVehicle(const_cast<Vehicle *>(v))); }
int GetAircraftHoldMaxAltitude(const Aircraft *v) { return openttd_rust_aircraft_hold_altitude(&_aircraft_leaves, RustAircraftVehicle(const_cast<Aircraft *>(v))); }
void HandleMissingAircraftOrders(Aircraft *v) { openttd_rust_aircraft_missing_orders(&_aircraft_leaves, RustAircraftVehicle(v)); }
uint Aircraft::Crash(bool flooded) { return openttd_rust_aircraft_crash(&_aircraft_leaves, RustAircraftVehicle(this), flooded); }
void AircraftNextAirportPos_and_Order(Aircraft *v) { openttd_rust_aircraft_next_airport(&_aircraft_leaves, RustAircraftVehicle(v)); }
void AircraftLeaveHangar(Aircraft *v, Direction exit_dir) { openttd_rust_aircraft_leave_hangar(&_aircraft_leaves, RustAircraftVehicle(v), exit_dir); }
TileIndex Aircraft::GetOrderStationLocation(StationID) { openttd_rust_aircraft_order_location(&_aircraft_leaves, RustAircraftVehicle(this)); return TileIndex{}; }
void UpdateAirplanesOnNewStation(const Station *st) { openttd_rust_aircraft_replacement(&_aircraft_leaves, RustBaseStationID(st)); }
ClosestDepot Aircraft::FindClosestDepot()
{
	StationID station{openttd_rust_aircraft_closest_depot(&_aircraft_leaves, RustAircraftVehicle(this))};
	return station == StationID::Invalid() ? ClosestDepot() : ClosestDepot(Station::Get(station)->xy, station);
}
Station *GetTargetAirportIfValid(const Aircraft *v)
{
	assert(v->GetType() == VEH_AIRCRAFT);
	Station *st = RustStationIfValid<Station *>(v->targetairport.base());
	return st == nullptr || RustStationAirportTile(st) == INVALID_TILE.base() ? nullptr : st;
}
void ReleaseAircraftAirportBlocks(const Aircraft *v) { openttd_rust_aircraft_release_blocks(&_aircraft_leaves, RustAircraftVehicle(const_cast<Aircraft *>(v))); }
