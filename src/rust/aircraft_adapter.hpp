/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file aircraft_adapter.hpp Canonical pool observations and direct shared leaves. */
#include "aircraft_ffi.h"
#include "vehicle_services.h"
#include "station_services.h"
#include "company_services.h"
static_assert(ORIGINAL_SAMPLE_COUNT == 73 && SND_18_TAKEOFF_HELICOPTER == 22 && SND_12_EXPLOSION == 16 && SND_17_SKID_PLANE == 21);
static_assert(NUM_CARGO == 64 && AT_OILRIG == 9);
static_assert(AIR_FAST == 2 && AIR_CTOL == 1 && ROTOR_Z_OFFSET == 5);
static_assert(HRS_ROTOR_STOPPED == 0 && HRS_ROTOR_MOVING_3 == 3);
static_assert(VehicleID::Invalid().base() == 1048575 && StationID::Invalid().base() == 65535);
static_assert(EV_BREAKDOWN_SMOKE_AIRCRAFT == 10 && EV_EXPLOSION_LARGE == 5 && EV_EXPLOSION_SMALL == 7);
static_assert(HVOT_AIRCRAFT == 16);
static_assert(OWNER_NONE.base() == 16);
static_assert(AirportBlocks{AirportBlock::Nothing}.base() == (uint64_t{1} << 30));
static_assert(AirportBlocks{AirportBlock::Zeppeliner}.base() == (uint64_t{1} << 62));
static_assert(AirportBlocks{AirportBlock::AirportClosed}.base() == (uint64_t{1} << 63));

/* Resolve each live shell/private owner once. Disaster helpers retain null State.
 * The record contains opaque pointers only; Rust never views C++ object layout. */
static OpenTTDAircraftVehicle AircraftVehicle(Vehicle *v) noexcept
{
	if (v == nullptr) return {nullptr, nullptr, VehicleID::Invalid().base()};
	return {RustVehicle(v), v->type == VEH_AIRCRAFT ? Aircraft::From(v)->rust_state.get() : nullptr, v->index.base()};
}

static OpenTTDAircraftVehicle OPENTTD_AIRCRAFT_CALL Aircraft_next(OpenTTDAircraftVehicle vehicle) noexcept
{
	return AircraftVehicle(RustVehicleNext(VehicleOf(vehicle.handle)));
}

static const OpenTTDStation *OPENTTD_AIRCRAFT_CALL Aircraft_station(uint16_t id) noexcept
{
	return RustStation(Station::GetIfValid(id));
}

static uint32_t OPENTTD_AIRCRAFT_CALL Aircraft_airport_tile(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationAirportTile(st);
}

static uint32_t OPENTTD_AIRCRAFT_CALL Aircraft_station_tile(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationLocation(st);
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_rotation(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return st->airport.rotation;
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_airport_type(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return st->airport.type;
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_station_owner(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return OpenTTDStationOwner(st).base();
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_has_hangar(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationHasHangar(st);
}

static const void * OPENTTD_AIRCRAFT_CALL Aircraft_airport_fta(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return st->airport.GetFTA();
}

static uint64_t * OPENTTD_AIRCRAFT_CALL Aircraft_airport_blocks(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return st->airport.rust_blocks.get();
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_had_vehicle(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return st->had_vehicle_of_type;
}

static const void * OPENTTD_AIRCRAFT_CALL Aircraft_dummy_airport() noexcept
{
	return GetAirport(AT_DUMMY);
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_airport_elements(const void * airport) noexcept
{
	const auto *ap = static_cast<const AirportFTAClass *>(airport);
	return ap->nofelements;
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_helipads(const void * airport) noexcept
{
	const auto *ap = static_cast<const AirportFTAClass *>(airport);
	return ap->num_helipads;
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_airport_flags(const void * airport) noexcept
{
	const auto *ap = static_cast<const AirportFTAClass *>(airport);
	return ap->flags.base();
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_airport_delta_z(const void * airport) noexcept
{
	const auto *ap = static_cast<const AirportFTAClass *>(airport);
	return ap->delta_z;
}

static const void * OPENTTD_AIRCRAFT_CALL Aircraft_node(const void * airport, uint8_t pos) noexcept
{
	return &static_cast<const AirportFTAClass *>(airport)->layout[pos];
}

static OpenTTDAircraftNode OPENTTD_AIRCRAFT_CALL Aircraft_fta(const void * node) noexcept
{
	const auto *n = static_cast<const AirportFTA *>(node);
	return {.next = n->next.get(), .blocks = n->blocks.base(), .position = n->position, .next_position = n->next_position, .heading = n->heading};
}

static OpenTTDAircraftMoving OPENTTD_AIRCRAFT_CALL Aircraft_moving(const void * airport, uint8_t pos) noexcept
{
	const auto *m = static_cast<const AirportFTAClass *>(airport)->MovingData(pos);
	return {.x = m->x, .y = m->y, .flags = m->flags.base(), .direction = m->direction};
}

static uint16_t OPENTTD_AIRCRAFT_CALL Aircraft_engine_speed(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return AircraftVehInfo(v->engine_type)->max_speed;
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_engine_subtype(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return AircraftVehInfo(v->engine_type)->subtype;
}

static uint16_t OPENTTD_AIRCRAFT_CALL Aircraft_engine_sound(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return AircraftVehInfo(v->engine_type)->sfx;
}

static uint16_t OPENTTD_AIRCRAFT_CALL Aircraft_engine_reliability(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return RustEngineReliability(v->GetEngine());
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_airport_entry(const void * ap, uint8_t direction) noexcept
{
	return static_cast<const AirportFTAClass *>(ap)->entry_points[direction];
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_direction_towards(OpenTTDAircraftVehicle vehicle, int32_t x, int32_t y) noexcept
{
	return GetDirectionTowards(VehicleOf(vehicle.handle), x, y);
}

static int32_t OPENTTD_AIRCRAFT_CALL Aircraft_hangar_height(uint16_t station) noexcept
{
	return GetTileMaxPixelZ(Station::Get(station)->airport.GetHangarTile(0));
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_terminal_count(const void * ap, uint32_t index) noexcept
{
	return static_cast<const AirportFTAClass *>(ap)->terminals[index];
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_hangar_exit(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return Station::GetByTile(v->tile)->airport.GetHangarExitDirection(v->tile);
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_can_use_station(OpenTTDAircraftVehicle vehicle, const OpenTTDStation *station) noexcept
{
	return CanVehicleUseStation(VehicleOf(vehicle.handle), StationOf(station));
}

static uint16_t OPENTTD_AIRCRAFT_CALL Aircraft_service_interval(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return RustCompanyAircraftServiceInterval(v->owner);
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_chain_in_depot(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return Aircraft::From(v)->IsChainInDepot();
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_nearest_depot_order(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return v->current_order.GetDepotActionType().Test(OrderDepotActionFlag::NearestDepot);
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_part_of_orders(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return v->current_order.GetDepotOrderType().Test(OrderDepotTypeFlag::PartOfOrders);
}

static uint16_t OPENTTD_AIRCRAFT_CALL Aircraft_next_station(uint16_t last) noexcept
{
	for (const Station *s : Station::Iterate(last == StationID::Invalid().base() ? 0 : last + 1)) return s->index.base();
	return StationID::Invalid().base();
}

static OpenTTDAircraftVehicle OPENTTD_AIRCRAFT_CALL Aircraft_next_aircraft(uint32_t last) noexcept
{
	for (Aircraft *v : Aircraft::Iterate(last == VehicleID::Invalid().base() ? 0 : last + 1)) return AircraftVehicle(v);
	return AircraftVehicle(nullptr);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_update_position(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); v->UpdatePosition(); v->UpdateViewport(true, false);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_rotor_image(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); GetRotorImage(v, EIT_ON_MAP, &v->Next()->Next()->sprite_cache.sprite_seq);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_copy_sprite(OpenTTDAircraftVehicle vehicle, OpenTTDAircraftVehicle source) noexcept
{
	VehicleOf(vehicle.handle)->sprite_cache.sprite_seq.CopyWithoutPalette(VehicleOf(source.handle)->sprite_cache.sprite_seq);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_position_viewport(OpenTTDAircraftVehicle vehicle) noexcept
{
	VehicleOf(vehicle.handle)->UpdatePositionAndViewport();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_create_effect(OpenTTDAircraftVehicle vehicle, uint8_t kind, int32_t x, int32_t y, int32_t z) noexcept
{
	CreateEffectVehicleRel(VehicleOf(vehicle.handle), x, y, z, static_cast<EffectVehicleType>(kind));
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_truncate_cargo(OpenTTDAircraftVehicle vehicle) noexcept
{
	VehicleOf(vehicle.handle)->cargo.Truncate();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_crash_news(OpenTTDAircraftVehicle vehicle, uint16_t station_id, uint32_t victims) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); StationID station{station_id};
	TileIndex vt = TileVirtXYClampedToMap(v->x_pos, v->y_pos); bool missing = station == StationID::Invalid();
	EncodedString headline = missing ? GetEncodedString(STR_NEWS_PLANE_CRASH_OUT_OF_FUEL, victims) : GetEncodedString(STR_NEWS_AIRCRAFT_CRASH, victims, station);
	AI::NewEvent(v->owner, new ScriptEventVehicleCrashed(v->index, vt, missing ? ScriptEventVehicleCrashed::CRASH_AIRCRAFT_NO_AIRPORT : ScriptEventVehicleCrashed::CRASH_PLANE_LANDING, victims, v->owner));
	Game::NewEvent(new ScriptEventVehicleCrashed(v->index, vt, missing ? ScriptEventVehicleCrashed::CRASH_AIRCRAFT_NO_AIRPORT : ScriptEventVehicleCrashed::CRASH_PLANE_LANDING, victims, v->owner));
	AddTileNewsItem(std::move(headline), v->owner == _local_company ? NewsType::Accident : NewsType::AccidentOther, vt, station);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_station_rating(uint32_t tile, uint8_t owner, int32_t amount, uint32_t radius) noexcept
{
	ModifyStationRatingAround(TileIndex{tile}, CompanyID{static_cast<uint8_t>(owner)}, amount, radius);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_landing_rating(uint16_t station_id, uint8_t cargo) noexcept
{
	GoodsEntry &ge = Station::Get(station_id)->goods[cargo]; ge.rating = 1; if (ge.HasData()) ge.GetData().cargo.Truncate();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_dirty_depot(OpenTTDAircraftVehicle vehicle) noexcept
{
	InvalidateWindowData(WC_VEHICLE_DEPOT, VehicleOf(vehicle.handle)->tile); SetWindowClassesDirty(WC_AIRCRAFT_LIST);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_first_arrival(OpenTTDAircraftVehicle vehicle, uint16_t station) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); Station *st = Station::Get(station); st->had_vehicle_of_type |= HVOT_AIRCRAFT;
	AddVehicleNewsItem(GetEncodedString(STR_NEWS_FIRST_AIRCRAFT_ARRIVAL, st->index), v->owner == _local_company ? NewsType::ArrivalCompany : NewsType::ArrivalOther, v->index, st->index);
	AI::NewEvent(v->owner, new ScriptEventStationFirstVehicle(st->index, v->index)); Game::NewEvent(new ScriptEventStationFirstVehicle(st->index, v->index));
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_begin_loading(OpenTTDAircraftVehicle vehicle) noexcept
{
	VehicleOf(vehicle.handle)->BeginLoading();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_dirty_details(OpenTTDAircraftVehicle vehicle) noexcept
{
	SetWindowDirty(WC_VEHICLE_DETAILS, vehicle.id);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_update_delta(OpenTTDAircraftVehicle vehicle) noexcept
{
	VehicleOf(vehicle.handle)->UpdateDeltaXY();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_touchdown_animation(OpenTTDAircraftVehicle vehicle, uint16_t station) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); TriggerAirportTileAnimation(Station::Get(station), TileVirtXY(v->x_pos, v->y_pos), AirportAnimationTrigger::AirplaneTouchdown);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_destination_too_far(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); AI::NewEvent(v->owner, new ScriptEventAircraftDestTooFar(v->index));
	if (v->owner == _local_company) AddVehicleAdviceNewsItem(AdviceType::AircraftDestinationTooFar, GetEncodedString(STR_NEWS_AIRCRAFT_DEST_TOO_FAR, v->index), v->index);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_delete_range_news(OpenTTDAircraftVehicle vehicle) noexcept
{
	DeleteVehicleNews(VehicleID{vehicle.id}, AdviceType::AircraftDestinationTooFar);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_service_order(OpenTTDAircraftVehicle vehicle, uint16_t station) noexcept
{
	Aircraft::From(VehicleOf(vehicle.handle))->current_order.MakeGoToDepot(StationID{static_cast<uint16_t>(station)}, OrderDepotTypeFlag::Service);
}

static int64_t OPENTTD_AIRCRAFT_CALL Aircraft_running_cost(OpenTTDAircraftVehicle vehicle) noexcept
{
	return Aircraft::From(VehicleOf(vehicle.handle))->GetRunningCost();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_subtract_cost(OpenTTDAircraftVehicle vehicle, int64_t cost) noexcept
{
	SubtractMoneyFromCompanyFract(VehicleOf(vehicle.handle)->owner, CommandCost(EXPENSES_AIRCRAFT_RUN, Money{cost}));
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_dirty_lists(OpenTTDAircraftVehicle vehicle) noexcept
{
	SetWindowDirty(WC_VEHICLE_DETAILS, vehicle.id); SetWindowClassesDirty(WC_AIRCRAFT_LIST);
}

static uint16_t OPENTTD_AIRCRAFT_CALL Aircraft_next_stopping_station(OpenTTDAircraftVehicle vehicle) noexcept
{
	std::vector<StationID> next_station; Aircraft::From(VehicleOf(vehicle.handle))->GetNextStoppingStation(next_station); return next_station.empty() ? StationID::Invalid().base() : next_station.back().base();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_remove_depot_orders(uint16_t station_id) noexcept
{
	RemoveOrderFromAllVehicles(OT_GOTO_DEPOT, StationID{static_cast<uint16_t>(station_id)}, true);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_assert_flying([[maybe_unused]] OpenTTDAircraftVehicle vehicle) noexcept
{
	assert(Aircraft::From(VehicleOf(vehicle.handle))->state == FLYING);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_invalid_movement(OpenTTDAircraftVehicle vehicle) noexcept
{
	Debug(misc, 0, "[Ap] cannot move further on Airport! (pos {} state {}) for vehicle {}", Aircraft::From(VehicleOf(vehicle.handle))->pos, Aircraft::From(VehicleOf(vehicle.handle))->state, vehicle.id); NOT_REACHED();
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_invalid_position(OpenTTDAircraftVehicle vehicle, const void * ap) noexcept
{
	Debug(misc, 0, "[Ap] position {} is not valid for current airport. Max position is {}", Aircraft::From(VehicleOf(vehicle.handle))->pos, static_cast<const AirportFTAClass *>(ap)->nofelements - 1); assert(Aircraft::From(VehicleOf(vehicle.handle))->pos < static_cast<const AirportFTAClass *>(ap)->nofelements);
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_invalid_scheme(OpenTTDAircraftVehicle) noexcept
{
	FatalError("OK, you shouldn't be here, check your Airport Scheme!");
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_unreachable(OpenTTDAircraftVehicle) noexcept
{
	NOT_REACHED();
}

static uint32_t OPENTTD_AIRCRAFT_CALL Aircraft_cargo_age_property(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return GetVehicleProperty(v, PROP_AIRCRAFT_CARGO_AGE_PERIOD, EngInfo(v->engine_type)->cargo_age_period);
}

static uint32_t OPENTTD_AIRCRAFT_CALL Aircraft_range_property(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return GetVehicleProperty(v, PROP_AIRCRAFT_RANGE, AircraftVehInfo(v->engine_type)->max_range);
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_touchdown_sound(OpenTTDAircraftVehicle vehicle) noexcept
{
	const Vehicle *v = VehicleOf(vehicle.handle);
	return PlayVehicleSound(v, VSE_TOUCHDOWN);
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_rotor_image_if_changed(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); Aircraft *rotor = v->Next()->Next(); VehicleSpriteSeq seq;
	GetRotorImage(v, EIT_ON_MAP, &seq); if (rotor->sprite_cache.sprite_seq == seq) return true; rotor->sprite_cache.sprite_seq = seq; return false;
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_update_rotor_image(OpenTTDAircraftVehicle vehicle) noexcept
{
	Aircraft *v = Aircraft::From(VehicleOf(vehicle.handle)); Aircraft *rotor = v->Next()->Next(); VehicleSpriteSeq seq;
	GetRotorImage(v, EIT_ON_MAP, &seq); rotor->sprite_cache.sprite_seq = seq;
}

static void OPENTTD_AIRCRAFT_CALL Aircraft_process_orders(OpenTTDAircraftVehicle vehicle) noexcept
{
	ProcessOrders(Aircraft::From(VehicleOf(vehicle.handle)));
}

static uint32_t OPENTTD_AIRCRAFT_CALL Aircraft_vehicle_crash(OpenTTDAircraftVehicle vehicle, bool flooded) noexcept
{
	return VehicleOf(vehicle.handle)->Vehicle::Crash(flooded);
}

static bool OPENTTD_AIRCRAFT_CALL Aircraft_send_to_depot(OpenTTDAircraftVehicle vehicle, bool service) noexcept
{
	const CompanyID old = OpenTTDCurrentCompany();
	OpenTTDSetCurrentCompany(CompanyID{RustVehicleOwner(VehicleOf(vehicle.handle))});
	bool failed = Command<CMD_SEND_VEHICLE_TO_DEPOT>::Do(DoCommandFlag::Execute, VehicleID{vehicle.id}, service ? DepotCommandFlag::Service : DepotCommandFlags{}, {}).Failed();
	OpenTTDSetCurrentCompany(old);
	return failed;
}

static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_fta_next_position(const void *node) noexcept
{
	return static_cast<const AirportFTA *>(node)->next_position;
}

static OpenTTDAircraftBlockNode OPENTTD_AIRCRAFT_CALL Aircraft_block_node(const void *node) noexcept
{
	const auto *n = static_cast<const AirportFTA *>(node);
	return {.blocks = n->blocks.base(), .position = n->position, .next_position = n->next_position};
}

static OpenTTDAircraftRouteNode OPENTTD_AIRCRAFT_CALL Aircraft_route_node(const void *node) noexcept
{
	const auto *n = static_cast<const AirportFTA *>(node);
	return {.next = n->next.get(), .next_position = n->next_position, .heading = n->heading};
}

static OpenTTDAircraftBlockChoice OPENTTD_AIRCRAFT_CALL Aircraft_block_choice(const void *node) noexcept
{
	const auto *n = static_cast<const AirportFTA *>(node);
	return {.next = n->next.get(), .blocks = n->blocks.base(), .heading = n->heading};
}

static OpenTTDAircraftGeometry OPENTTD_AIRCRAFT_CALL Aircraft_geometry(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	const uint32_t tile = RustStationAirportTile(st);
	if (tile == INVALID_TILE.base()) return {.tile = RustStationLocation(st), .rotation = 0, .width = 1, .height = 1, .airport = false};
	return {.tile = tile, .rotation = st->airport.rotation, .width = st->airport.w, .height = st->airport.h, .airport = true};
}
static uint64_t OPENTTD_AIRCRAFT_CALL Aircraft_layout_blocks(const void *ap, uint8_t pos) noexcept { return static_cast<const AirportFTAClass *>(ap)->layout[pos].blocks.base(); }
static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_layout_heading(const void *ap, uint8_t pos) noexcept { return static_cast<const AirportFTAClass *>(ap)->layout[pos].heading; }
static uint8_t OPENTTD_AIRCRAFT_CALL Aircraft_layout_next_position(const void *ap, uint8_t pos) noexcept { return static_cast<const AirportFTAClass *>(ap)->layout[pos].next_position; }
static const void *OPENTTD_AIRCRAFT_CALL Aircraft_layout_next(const void *ap, uint8_t pos) noexcept { return static_cast<const AirportFTAClass *>(ap)->layout[pos].next.get(); }
static OpenTTDAircraftLayoutRoute OPENTTD_AIRCRAFT_CALL Aircraft_layout_route(const void *ap, uint8_t pos) noexcept
{
	const AirportFTA *n = &static_cast<const AirportFTAClass *>(ap)->layout[pos];
	return {.node = n, .next = n->next.get(), .next_position = n->next_position, .heading = n->heading};
}
static OpenTTDAircraftHangarCandidate OPENTTD_AIRCRAFT_CALL Aircraft_hangar_candidate(const OpenTTDStation *station, uint8_t vehicle_owner) noexcept
{
	const Station *st = StationOf(station);
	uint8_t owner = OpenTTDStationOwner(st).base();
	bool airport = owner == vehicle_owner && RustStationHasAirport(st);
	return {.owner = owner, .airport = airport, .hangar = airport && RustStationHasHangar(st)};
}
static const OpenTTDAircraftLeaves _aircraft_leaves{
	.hangar_candidate = Aircraft_hangar_candidate,
	.geometry = Aircraft_geometry,
	.layout_blocks = Aircraft_layout_blocks,
	.layout_heading = Aircraft_layout_heading,
	.layout_next_position = Aircraft_layout_next_position,
	.layout_next = Aircraft_layout_next,
	.layout_route = Aircraft_layout_route,
	.vehicle = &GetRustVehicleServices(),
	.shared = &GetRustSharedServices(),
	.next = Aircraft_next,
	.station = Aircraft_station,
	.airport_tile = Aircraft_airport_tile,
	.station_tile = Aircraft_station_tile,
	.rotation = Aircraft_rotation,
	.airport_type = Aircraft_airport_type,
	.station_owner = Aircraft_station_owner,
	.has_hangar = Aircraft_has_hangar,
	.airport_fta = Aircraft_airport_fta,
	.airport_blocks = Aircraft_airport_blocks,
	.had_vehicle = Aircraft_had_vehicle,
	.dummy_airport = Aircraft_dummy_airport,
	.airport_elements = Aircraft_airport_elements,
	.helipads = Aircraft_helipads,
	.airport_flags = Aircraft_airport_flags,
	.airport_delta_z = Aircraft_airport_delta_z,
	.node = Aircraft_node,
	.fta = Aircraft_fta,
	.moving = Aircraft_moving,
	.engine_speed = Aircraft_engine_speed,
	.engine_subtype = Aircraft_engine_subtype,
	.engine_sound = Aircraft_engine_sound,
	.engine_reliability = Aircraft_engine_reliability,
	.airport_entry = Aircraft_airport_entry,
	.direction_towards = Aircraft_direction_towards,
	.hangar_height = Aircraft_hangar_height,
	.terminal_count = Aircraft_terminal_count,
	.hangar_exit = Aircraft_hangar_exit,
	.can_use_station = Aircraft_can_use_station,
	.service_interval = Aircraft_service_interval,
	.chain_in_depot = Aircraft_chain_in_depot,
	.nearest_depot_order = Aircraft_nearest_depot_order,
	.part_of_orders = Aircraft_part_of_orders,
	.next_station = Aircraft_next_station,
	.next_aircraft = Aircraft_next_aircraft,
	.update_position = Aircraft_update_position,
	.rotor_image = Aircraft_rotor_image,
	.copy_sprite = Aircraft_copy_sprite,
	.position_viewport = Aircraft_position_viewport,
	.create_effect = Aircraft_create_effect,
	.truncate_cargo = Aircraft_truncate_cargo,
	.crash_news = Aircraft_crash_news,
	.station_rating = Aircraft_station_rating,
	.landing_rating = Aircraft_landing_rating,
	.dirty_depot = Aircraft_dirty_depot,
	.first_arrival = Aircraft_first_arrival,
	.begin_loading = Aircraft_begin_loading,
	.dirty_details = Aircraft_dirty_details,
	.update_delta = Aircraft_update_delta,
	.touchdown_animation = Aircraft_touchdown_animation,
	.destination_too_far = Aircraft_destination_too_far,
	.delete_range_news = Aircraft_delete_range_news,
	.service_order = Aircraft_service_order,
	.running_cost = Aircraft_running_cost,
	.subtract_cost = Aircraft_subtract_cost,
	.dirty_lists = Aircraft_dirty_lists,
	.next_stopping_station = Aircraft_next_stopping_station,
	.remove_depot_orders = Aircraft_remove_depot_orders,
	.assert_flying = Aircraft_assert_flying,
	.invalid_movement = Aircraft_invalid_movement,
	.invalid_position = Aircraft_invalid_position,
	.invalid_scheme = Aircraft_invalid_scheme,
	.unreachable = Aircraft_unreachable,
	.cargo_age_property = Aircraft_cargo_age_property,
	.range_property = Aircraft_range_property,
	.touchdown_sound = Aircraft_touchdown_sound,
	.rotor_image_if_changed = Aircraft_rotor_image_if_changed,
	.update_rotor_image = Aircraft_update_rotor_image,
	.process_orders = Aircraft_process_orders,
	.vehicle_crash = Aircraft_vehicle_crash,
	.send_to_depot = Aircraft_send_to_depot,
	.fta_next_position = Aircraft_fta_next_position,
	.block_node = Aircraft_block_node,
	.route_node = Aircraft_route_node,
	.block_choice = Aircraft_block_choice,
};
bool Aircraft::Tick()
{
	if (!this->IsNormalAircraft()) return true;
	PerformanceAccumulator framerate(PFE_GL_AIRCRAFT);
	return openttd_rust_aircraft_tick(&_aircraft_leaves, AircraftVehicle(this));
}
void Aircraft::OnNewCalendarDay() { openttd_rust_aircraft_calendar_day(&_aircraft_leaves, AircraftVehicle(this)); }
void Aircraft::OnNewEconomyDay() { openttd_rust_aircraft_economy_day(&_aircraft_leaves, AircraftVehicle(this)); }
void SetAircraftPosition(Aircraft *v, int x, int y, int z) { openttd_rust_aircraft_position(&_aircraft_leaves, AircraftVehicle(v), x, y, z); }
void HandleAircraftEnterHangar(Aircraft *v) { openttd_rust_aircraft_enter_hangar(&_aircraft_leaves, AircraftVehicle(v)); }
void UpdateAircraftCache(Aircraft *v, bool update_range) { openttd_rust_aircraft_cache(&_aircraft_leaves, AircraftVehicle(v), update_range); }
void GetAircraftFlightLevelBounds(const Vehicle *v, int *min, int *max) { openttd_rust_aircraft_flight_bounds(&_aircraft_leaves, AircraftVehicle(const_cast<Vehicle *>(v)), min, max); }
static uint8_t &GetFlightFlags(Aircraft *v) { return v->flags; }
static uint8_t &GetFlightFlags(DisasterVehicle *v) { return v->Flags(); }
template <class T> int GetAircraftFlightLevel(T *v, bool takeoff) { return openttd_rust_aircraft_flight_level(&_aircraft_leaves, AircraftVehicle(v), &GetFlightFlags(v), takeoff); }
template int GetAircraftFlightLevel(DisasterVehicle *v, bool takeoff);
template int GetAircraftFlightLevel(Aircraft *v, bool takeoff);
int GetTileHeightBelowAircraft(const Vehicle *v) { return openttd_rust_aircraft_height(&_aircraft_leaves, AircraftVehicle(const_cast<Vehicle *>(v))); }
int GetAircraftHoldMaxAltitude(const Aircraft *v) { return openttd_rust_aircraft_hold_altitude(&_aircraft_leaves, AircraftVehicle(const_cast<Aircraft *>(v))); }
void HandleMissingAircraftOrders(Aircraft *v) { openttd_rust_aircraft_missing_orders(&_aircraft_leaves, AircraftVehicle(v)); }
uint Aircraft::Crash(bool flooded) { return openttd_rust_aircraft_crash(&_aircraft_leaves, AircraftVehicle(this), flooded); }
void AircraftNextAirportPos_and_Order(Aircraft *v) { openttd_rust_aircraft_next_airport(&_aircraft_leaves, AircraftVehicle(v)); }
void AircraftLeaveHangar(Aircraft *v, Direction exit_dir) { openttd_rust_aircraft_leave_hangar(&_aircraft_leaves, AircraftVehicle(v), exit_dir); }
TileIndex Aircraft::GetOrderStationLocation(StationID) { openttd_rust_aircraft_order_location(&_aircraft_leaves, AircraftVehicle(this)); return TileIndex{}; }
void UpdateAirplanesOnNewStation(const Station *st) { openttd_rust_aircraft_replacement(&_aircraft_leaves, st->index.base()); }
ClosestDepot Aircraft::FindClosestDepot()
{
	StationID station{openttd_rust_aircraft_closest_depot(&_aircraft_leaves, AircraftVehicle(this))};
	return station == StationID::Invalid() ? ClosestDepot() : ClosestDepot(Station::Get(station)->xy, station);
}
Station *GetTargetAirportIfValid(const Aircraft *v)
{
	assert(v->type == VEH_AIRCRAFT);
	Station *st = Station::GetIfValid(v->targetairport);
	return st == nullptr || st->airport.tile == INVALID_TILE ? nullptr : st;
}
void ReleaseAircraftAirportBlocks(const Aircraft *v) { openttd_rust_aircraft_release_blocks(&_aircraft_leaves, AircraftVehicle(const_cast<Aircraft *>(v))); }
void InvalidateAircraftTargetStation(StationID station) { openttd_rust_aircraft_invalidate_target(&_aircraft_leaves, station.base()); }
