/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file disaster_adapter.hpp Copied canonical observations and returned shared services. */
#include "services_water.h"
#include "services_landscape.h"
#include "services_station.h"
#include "services_industry.h"
#include "services_map.h"
#include "services_rail.h"
#include "disaster_ffi.h"
#include "vehicle_services.h"
#include "disaster_counter.h"
#include "services_ffi.h"

static_assert(VEH_EFFECT == 4);
static_assert(VehicleID::Invalid().base() == 1048575);
static_assert(ST_BIG_SUBMARINE == 14 && ROTOR_Z_OFFSET == 5);
static_assert(SPR_BLIMP == 3905 && SPR_BLIMP_CRASHING == 3906 && SPR_BLIMP_CRASHED == 3907);
static_assert(SPR_UFO_SMALL_SCOUT == 3908 && SPR_UFO_SMALL_SCOUT_DARKER == 3909);
static_assert(SPR_SUB_SMALL_NE == 3910 && SPR_SUB_LARGE_NE == 3914 && SPR_F_15 == 3918 && SPR_F_15_FIRING == 3919);
static_assert(SPR_UFO_HARVESTER == 3920 && SPR_XCOM_SKYRANGER == 3921 && SPR_AH_64A == 3922 && SPR_AH_64A_FIRING == 3923);
static_assert(SPR_ROTOR_MOVING_1 == 3902 && SPR_ROTOR_MOVING_3 == 3904);
static_assert(MP_CLEAR == 0 && MP_RAILWAY == 1 && MP_HOUSE == 3 && MP_TREES == 4 && MP_WATER == 6 && MP_INDUSTRY == 8);
static_assert(TRACK_BIT_ALL == 63 && EV_CRASH_SMOKE == 4 && EV_EXPLOSION_LARGE == 5 && EV_EXPLOSION_SMALL == 7);

#include "company_services.h"
#include "settings_services.h"
#include "environment_services.h"

static_assert(SND_12_EXPLOSION == 16 && DIRDIFF_90LEFT == 6);

static OpenTTDDisasterVehicle DisasterHandle(DisasterVehicle *v) noexcept
{
	return {.handle = RustVehicle(v), .state = v == nullptr ? nullptr : v->rust_state.get(), .id = v == nullptr ? VehicleID::Invalid().base() : v->index.base()};
}
static OpenTTDDisasterVehicle DisasterBaseHandle(Vehicle *v) noexcept
{
	return {.handle = RustVehicle(v), .state = nullptr, .id = v == nullptr ? VehicleID::Invalid().base() : v->index.base()};
}
static OpenTTDDisasterRoadTarget DisasterRoadHandle(RoadVehicle *v) noexcept
{
	return {.handle = RustVehicle(v), .state = v == nullptr ? nullptr : v->GetRustState(), .id = v == nullptr ? VehicleID::Invalid().base() : v->index.base()};
}
static OpenTTDDisasterVehicle OPENTTD_RUST_CALL DisasterNextPart(OpenTTDVehicle *v) noexcept
{
	Vehicle *next = RustVehicleNext(VehicleOf(v));
	return DisasterHandle(next == nullptr ? nullptr : DisasterVehicle::From(next));
}
static OpenTTDDisasterVehicle OPENTTD_RUST_CALL DisasterResolve(uint32_t id) noexcept { return DisasterHandle(DisasterVehicle::Get(id)); }
static OpenTTDDisasterRoadTarget OPENTTD_RUST_CALL DisasterRoadTarget(uint32_t id) noexcept { return DisasterRoadHandle(RoadVehicle::Get(id)); }
static OpenTTDDisasterVehicle OPENTTD_RUST_CALL DisasterNext(uint32_t from) noexcept
{
	for (DisasterVehicle *v : DisasterVehicle::Iterate(from)) return DisasterHandle(v);
	return DisasterHandle(nullptr);
}
static OpenTTDDisasterRoadTarget OPENTTD_RUST_CALL DisasterNextRoad(uint32_t from) noexcept
{
	for (RoadVehicle *v : RoadVehicle::Iterate(from)) return DisasterRoadHandle(v);
	return DisasterRoadHandle(nullptr);
}
static OpenTTDDisasterVehicle OPENTTD_RUST_CALL DisasterNextVehicle(uint32_t from) noexcept { return DisasterBaseHandle(VehicleOf(RustNextVehicle(from))); }
static OpenTTDDisasterVehicle OPENTTD_RUST_CALL DisasterNextTrain(uint32_t from) noexcept
{
	for (Train *v : Train::Iterate(from)) return DisasterBaseHandle(v);
	return DisasterBaseHandle(nullptr);
}
static OpenTTDDisasterRoadSelection OPENTTD_RUST_CALL DisasterRoadSelection(OpenTTDVehicle *shell) noexcept
{
	const RoadVehicle *v = RoadVehicle::From(VehicleOf(shell));
	return {.front = RustVehicleFront(v), .disaster = v->disaster_vehicle.base()};
}
static OpenTTDDisasterRoadApproach OPENTTD_RUST_CALL DisasterRoadApproach(OpenTTDVehicle *shell) noexcept
{
	const Vehicle *v = VehicleOf(shell);
	return {.x = RustVehicleX(v), .y = RustVehicleY(v), .status = RustVehicleStatus(v), .breakdown = RustVehicleBreakdownCounter(v)};
}
static void OPENTTD_RUST_CALL DisasterWriteRoadTarget(OpenTTDVehicle *v, uint32_t disaster) noexcept { RoadVehicle::From(VehicleOf(v))->disaster_vehicle = VehicleID{disaster}; }
static OpenTTDDisasterTrainSelection OPENTTD_RUST_CALL DisasterTrainSelection(OpenTTDVehicle *shell) noexcept
{
	const Vehicle *v = VehicleOf(shell);
	return {.front = RustVehicleFront(v), .owner = RustVehicleOwner(v), .tile = RustVehicleTile(v), .status = RustVehicleStatus(v)};
}
static uint16_t OPENTTD_RUST_CALL DisasterRoadCount(uint8_t company) noexcept { return Company::Get(CompanyID{company})->group_all[VEH_ROAD].num_vehicle; }
static OpenTTDDisasterStationTarget OPENTTD_RUST_CALL DisasterNextStation(uint32_t from) noexcept
{
	const Station *st = OpenTTDNextStation(from);
	return st == nullptr ? OpenTTDDisasterStationTarget{.next = UINT32_MAX, .tile = INVALID_TILE.base(), .airport_type = 0} :
		OpenTTDDisasterStationTarget{.next = uint32_t{RustBaseStationID(st)} + 1, .tile = RustStationAirportTile(st), .airport_type = st->airport.type};
}
static OpenTTDDisasterIndustryTarget DisasterIndustryHandle(Industry *i) noexcept
{
	if (i == nullptr) return {.industry = nullptr, .id = IndustryID::Invalid().base(), .airplane = false, .helicopter = false, .subsidence = false};
	const auto &behaviour = GetIndustrySpec(i->type)->behaviour;
	return {.industry = i, .id = i->index.base(), .airplane = behaviour.Test(IndustryBehaviour::AirplaneAttacks), .helicopter = behaviour.Test(IndustryBehaviour::ChopperAttacks), .subsidence = behaviour.Test(IndustryBehaviour::CanSubsidence)};
}
static OpenTTDDisasterIndustryTarget OPENTTD_RUST_CALL DisasterNextIndustry(uint32_t from) noexcept { return DisasterIndustryHandle(OpenTTDNextIndustry(from)); }
static OpenTTDDisasterIndustryTarget OPENTTD_RUST_CALL DisasterIndustry(uint16_t id) noexcept { return DisasterIndustryHandle(RustIndustryByID(id)); }
static OpenTTDDisasterIndustryLocation OPENTTD_RUST_CALL DisasterIndustryLocation(void *shell) noexcept
{
	const Industry *i = static_cast<Industry *>(shell);
	return {.tile = i->location.tile.base(), .town = OpenTTDIndustryTown(i)->index.base()};
}
static uint32_t OPENTTD_RUST_CALL DisasterIndustryTile(void *shell) noexcept { return static_cast<Industry *>(shell)->location.tile.base(); }
static uint16_t OPENTTD_RUST_CALL DisasterIndustryIndex(uint32_t tile) noexcept { return GetIndustryIndex(TileIndex{tile}).base(); }
static bool OPENTTD_RUST_CALL DisasterIndustryBelongs(void *shell, uint32_t tile) noexcept { return static_cast<Industry *>(shell)->TileBelongsToIndustry(TileIndex{tile}); }
static void OPENTTD_RUST_CALL DisasterAirportBlocks(uint16_t id, bool blocked) noexcept
{
	Station *st = Station::Get(id);
	if (blocked) st->airport.blocks.Set({AirportBlock::Zeppeliner, AirportBlock::RunwayIn});
	else st->airport.blocks.Reset({AirportBlock::Zeppeliner, AirportBlock::RunwayIn});
}
static void OPENTTD_RUST_CALL DisasterZeppelinCrashed(uint32_t tile) noexcept
{
	AI::NewEvent(CompanyID{RustTileOwnerValue(tile)}, new ScriptEventDisasterZeppelinerCrashed(StationID(MapSvcStationIndex(tile))));
}
static void OPENTTD_RUST_CALL DisasterZeppelinCleared(uint32_t tile, uint16_t station) noexcept
{
	AI::NewEvent(CompanyID{RustTileOwnerValue(tile)}, new ScriptEventDisasterZeppelinerCleared(StationID{station}));
}
static void OPENTTD_RUST_CALL DisasterRoadCrashed(OpenTTDVehicle *shell, uint32_t victims) noexcept
{
	const Vehicle *target = VehicleOf(shell);
	AI::NewEvent(target->owner, new ScriptEventVehicleCrashed(target->index, target->tile, ScriptEventVehicleCrashed::CRASH_RV_UFO, victims, target->owner));
	Game::NewEvent(new ScriptEventVehicleCrashed(target->index, target->tile, ScriptEventVehicleCrashed::CRASH_RV_UFO, victims, target->owner));
}
static void OPENTTD_RUST_CALL DisasterZeppelinNews(uint32_t tile, uint16_t station) noexcept { AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_ZEPPELIN, station), NewsType::Accident, TileIndex{tile}); }
static void OPENTTD_RUST_CALL DisasterSmallUfoNews(uint32_t tile) noexcept { AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_SMALL_UFO), NewsType::Accident, TileIndex{tile}); }
static void OPENTTD_RUST_CALL DisasterAirplaneNews(uint16_t industry, uint16_t town) noexcept { AddIndustryNewsItem(GetEncodedString(STR_NEWS_DISASTER_AIRPLANE_OIL_REFINERY, town), NewsType::Accident, IndustryID{industry}); }
static void OPENTTD_RUST_CALL DisasterHelicopterNews(uint16_t industry, uint16_t town) noexcept { AddIndustryNewsItem(GetEncodedString(STR_NEWS_DISASTER_HELICOPTER_FACTORY, town), NewsType::Accident, IndustryID{industry}); }
static void OPENTTD_RUST_CALL DisasterBigUfoNews(uint32_t tile, uint16_t town) noexcept { AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_BIG_UFO, town), NewsType::Accident, TileIndex{tile}); }
static void OPENTTD_RUST_CALL DisasterCoalNews(uint32_t tile, uint16_t town) noexcept { AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_COAL_MINE_SUBSIDENCE, town), NewsType::Accident, TileIndex{tile}); }
static void OPENTTD_RUST_CALL DisasterClear(uint32_t tile, uint8_t company) noexcept
{
	Backup<CompanyID> cur_company(_current_company, CompanyID{company});
	Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, TileIndex{tile});
	cur_company.Restore();
}
static OpenTTDDisasterVehicle OPENTTD_RUST_CALL DisasterCreate(int32_t x, int32_t y, uint8_t direction, uint8_t subtype, uint32_t target) noexcept
{
	return DisasterHandle(new DisasterVehicle(x, y, static_cast<Direction>(direction), static_cast<DisasterSubType>(subtype), VehicleID{target}));
}
static const OpenTTDDisasterServices &DisasterServices() noexcept
{
	static const OpenTTDDisasterServices services{
		.shared = &GetRustSharedServices(),
		.vehicles = &GetRustVehicleServices(),
		.occupancy = GetRustOccupancyCommands(),
		.environment = &GetRustEnvironmentServices(),
		.rail_map = &GetRustRailMapServices(),
		.aircraft = &GetRustAircraftLeaves(),
		.road = &GetRustRoadLeaves(),
		.age = RustVehicleRead<int32_t, RustVehicleAgeValue>,
		.write_age = RustVehicleInvoke<RustWriteVehicleAge, int32_t>,
		.write_sprite = RustVehicleInvoke<RustWriteVehicleSpriteSequence, SpriteID>,
		.first_sprite = RustVehicleRead<SpriteID, RustVehicleFirstSprite>,
		.write_first_sprite = RustVehicleInvoke<RustWriteVehicleFirstSprite, SpriteID>,
		.write_subtype = RustVehicleInvoke<RustVehicleSetSubtype, uint8_t>,
		.write_owner = RustVehicleSetOwner,
		.set_next = RustTypedVehicleSetNext,
		.next_part = DisasterNextPart,
		.update_delta = RustVehicleUpdateDelta<OpenTTDVehicle *>,
		.toward = RustVehicleDirectionTowards,
		.create = DisasterCreate,
		.resolve = DisasterResolve,
		.next_disaster = DisasterNext,
		.next_road = DisasterNextRoad,
		.road_target = DisasterRoadTarget,
		.road_selection = DisasterRoadSelection,
		.road_approach = DisasterRoadApproach,
		.write_road_disaster = DisasterWriteRoadTarget,
		.write_breakdown = RustVehicleInvoke<RustVehicleWriteBreakdown, uint8_t, uint8_t>,
		.next_vehicle = DisasterNextVehicle,
		.ground = RustVehicleRead<bool, RustVehicleIsGround>,
		.next_train = DisasterNextTrain,
		.train_selection = DisasterTrainSelection,
		.road_count = DisasterRoadCount,
		.human = RustCompanyIsHuman,
		.next_station = DisasterNextStation,
		.next_industry = DisasterNextIndustry,
		.industry = DisasterIndustry,
		.industry_location = DisasterIndustryLocation,
		.industry_tile = DisasterIndustryTile,
		.industry_index = DisasterIndustryIndex,
		.industry_belongs = DisasterIndustryBelongs,
		.reset_industry_tile = [](uint32_t tile) noexcept { ResetIndustryConstructionStage(TileIndex{tile}); },
		.airport_tile = RustAirportTile,
		.airport_blocks = DisasterAirportBlocks,
		.zeppelin_crashed = DisasterZeppelinCrashed,
		.zeppelin_cleared = DisasterZeppelinCleared,
		.road_crashed = DisasterRoadCrashed,
		.zeppelin_news = DisasterZeppelinNews,
		.small_ufo_news = DisasterSmallUfoNews,
		.airplane_news = DisasterAirplaneNews,
		.helicopter_news = DisasterHelicopterNews,
		.big_ufo_news = DisasterBigUfoNews,
		.coal_news = DisasterCoalNews,
		.clear = DisasterClear,
		.update_signals = RustUpdateSignals,
		.disasters_enabled = RustDisastersEnabled,
		.can_allocate = RustVehicleCanAllocate,
		.town_index = RustTownAuthorityIndex,
	};
	return services;
}

DisasterVehicle::DisasterVehicle(int x, int y, Direction direction, DisasterSubType subtype, VehicleID target) :
	SpecializedVehicleBase(), rust_state{openttd_rust_disaster_state_create(target.base())}
{
	openttd_rust_disaster_construct(&DisasterServices(), DisasterHandle(this), x, y, direction, subtype);
}
void DisasterVehicle::UpdatePosition(int x, int y, int z) { openttd_rust_disaster_position(&DisasterServices(), DisasterHandle(this), x, y, z); }
void DisasterVehicle::UpdateImage() { openttd_rust_disaster_image(&DisasterServices(), DisasterHandle(this)); }
bool DisasterVehicle::Tick() { return openttd_rust_disaster_tick(&DisasterServices(), DisasterHandle(this)); }
void StartupDisasters() { openttd_rust_disaster_startup(&DisasterServices()); }
void ReleaseDisastersTargetingIndustry(IndustryID industry) { openttd_rust_disaster_release_industry(&DisasterServices(), industry.base()); }
void ReleaseDisasterVehicle(VehicleID vehicle)
{
	DisasterVehicle *v = DisasterVehicle::GetIfValid(vehicle);
	if (v == nullptr) return;
	assert(v->subtype == ST_SMALL_UFO);
	assert(v->State() != 0);
	openttd_rust_disaster_release_vehicle(&DisasterServices(), DisasterHandle(v));
}
static const IntervalTimer<TimerGameEconomy> _economy_disaster_daily({TimerGameEconomy::DAY, TimerGameEconomy::Priority::DISASTER}, [](auto) { openttd_rust_disaster_daily(&DisasterServices()); });

#include "service_table_check.hpp"
#include "abi-visit-OpenTTDDisasterServices.hpp"
bool CheckAbi_disaster_vehicle() { return AbiServicesComplete(DisasterServices()); }
