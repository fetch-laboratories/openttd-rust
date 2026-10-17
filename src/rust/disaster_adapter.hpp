/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
/** @file disaster_adapter.hpp Copied canonical observations and returned shared services. */
#include "services_rail.h"
#include "disaster_ffi.h"
#include "vehicle_services.h"
#include "disaster_counter.h"
#include "services_ffi.h"

static_assert(VehicleID::Invalid().base() == 1048575);
static_assert(ST_BIG_SUBMARINE == 14 && ROTOR_Z_OFFSET == 5);
static_assert(SPR_BLIMP == 3905 && SPR_BLIMP_CRASHING == 3906 && SPR_BLIMP_CRASHED == 3907);
static_assert(SPR_UFO_SMALL_SCOUT == 3908 && SPR_UFO_SMALL_SCOUT_DARKER == 3909);
static_assert(SPR_SUB_SMALL_NE == 3910 && SPR_SUB_LARGE_NE == 3914 && SPR_F_15 == 3918 && SPR_F_15_FIRING == 3919);
static_assert(SPR_UFO_HARVESTER == 3920 && SPR_XCOM_SKYRANGER == 3921 && SPR_AH_64A == 3922 && SPR_AH_64A_FIRING == 3923);
static_assert(SPR_ROTOR_MOVING_1 == 3902 && SPR_ROTOR_MOVING_3 == 3904);
static_assert(MP_CLEAR == 0 && MP_RAILWAY == 1 && MP_HOUSE == 3 && MP_TREES == 4 && MP_WATER == 6 && MP_INDUSTRY == 8);
static_assert(TRACK_BIT_ALL == 63 && EV_CRASH_SMOKE == 4 && EV_EXPLOSION_LARGE == 5 && EV_EXPLOSION_SMALL == 7);

template <class T> static uint32_t DisasterNext(size_t from)
{
	for (const T *item : T::Iterate(from)) return item->index.base();
	return VehicleID::Invalid().base();
}

/* These leaves neither allocate nor invoke scripts/NewGRF/logging. Every record
 * is copied before returning. Original source preconditions govern ID access. */
static void OPENTTD_DISASTER_CALL DisasterRead(void *, uint32_t kind, uint32_t id, int64_t a, int64_t b, int64_t *out) noexcept
{
	switch (kind) {
		case 0:
			out[0] = Map::SizeX(); out[1] = Map::SizeY(); out[2] = Map::MaxX(); out[3] = Map::MaxY(); out[4] = Map::Size() - 1;
			out[5] = TimerGameCalendar::year.base(); out[6] = _settings_game.difficulty.disasters; out[7] = _settings_client.sound.disaster;
			out[8] = _settings_game.construction.freeform_edges; out[9] = a == 0 ? 0 : Vehicle::CanAllocateItem(a);
			break;
		case 1: {
			const Vehicle *v = Vehicle::Get(id);
			out[0] = v->subtype; out[1] = v->x_pos; out[2] = v->y_pos; out[3] = v->z_pos; out[4] = v->tile.base(); out[5] = v->dest_tile.base();
			out[6] = v->direction; out[7] = RustVehicleAge(v).base(); out[8] = v->tick_counter; out[9] = v->owner.base(); out[10] = v->vehstatus.base();
			out[11] = v->Next() == nullptr ? VehicleID::Invalid().base() : v->Next()->index.base(); out[12] = v->sprite_cache.sprite_seq.seq[0].sprite;
			out[13] = v->IsFrontEngine(); out[14] = v->IsGroundVehicle(); out[15] = v->type; out[16] = RustVehicleBreakdownCounter(v); out[17] = RustVehicleBreakdownDelay(v);
			if (v->type == VEH_ROAD) { const RoadVehicle *r = RoadVehicle::From(v); out[18] = r->GetCrashedCounter(); out[19] = r->disaster_vehicle.base(); }
			break;
		}
		case 2: {
			const Industry *i = Industry::Get(id); const auto &behaviour = GetIndustrySpec(i->type)->behaviour;
			out[0] = i->location.tile.base(); out[1] = i->type; out[2] = behaviour.Test(IndustryBehaviour::AirplaneAttacks);
			out[3] = behaviour.Test(IndustryBehaviour::ChopperAttacks); out[4] = behaviour.Test(IndustryBehaviour::CanSubsidence); out[5] = i->town->index.base();
			break;
		}
		case 3: { const Station *s = Station::Get(id); out[0] = s->airport.tile.base(); out[1] = s->airport.type; break; }
		case 4: out[0] = Company::Get(id)->group_all[VEH_ROAD].num_vehicle; break;
		case 5: {
			TileIndex tile{id}; out[0] = IsValidTile(tile); if (!out[0]) break;
			out[1] = GetTileType(tile);
			if (out[1] == MP_RAILWAY) out[2] = GetTileOwner(tile).base();
			out[4] = IsTileType(tile, MP_RAILWAY) && RustRailIsDepot(id); out[5] = IsAirportTile(tile);
			if (out[5]) out[6] = GetStationIndex(tile).base();
			if (IsTileType(tile, MP_INDUSTRY)) out[7] = GetIndustryIndex(tile).base();
			out[8] = IsPlainRailTile(tile); break;
		}
		case 6: {
			size_t from = static_cast<uint32_t>(a) == VehicleID::Invalid().base() ? 0 : a + 1;
			switch (id) {
				case 0: out[0] = DisasterNext<Company>(from); break;
				case 1: out[0] = DisasterNext<RoadVehicle>(from); break;
				case 2: out[0] = DisasterNext<Vehicle>(from); break;
				case 3: out[0] = DisasterNext<Train>(from); break;
				case 4: out[0] = DisasterNext<Station>(from); break;
				case 5: out[0] = DisasterNext<Industry>(from); break;
				case 6: out[0] = DisasterNext<DisasterVehicle>(from); break;
			}
			break;
		}
		case 7: out[0] = reinterpret_cast<intptr_t>(DisasterVehicle::Get(id)->rust_state.get()); break;
		case 8: out[0] = GetSlopePixelZ(a, b); break;
		case 9: out[0] = GetDirectionTowards(Vehicle::Get(id), a, b); break;
		case 10: { auto gp = GetNewVehiclePos(Vehicle::Get(id)); out[0] = gp.x; out[1] = gp.y; break; }
		case 11: out[0] = Industry::Get(id)->TileBelongsToIndustry(TileIndex{static_cast<uint32_t>(a)}); break;
		case 12: out[0] = TileAddWrap(TileIndex{id}, a, b).base(); break;
		case 13: out[0] = TileOffsByDiagDir(static_cast<DiagDirection>(a)); break;
		case 14: out[0] = DisasterVehicle::GetIfValid(id) != nullptr; break;
		case 15: out[0] = Company::IsHumanID(CompanyID{static_cast<uint8_t>(id)}); break;
	}
}

static void OPENTTD_DISASTER_CALL DisasterWrite(void *, uint32_t field, uint32_t id, int64_t value) noexcept
{
	Vehicle *v = Vehicle::Get(id);
	switch (field) {
		case 0: v->subtype = value; break;
		case 1: v->x_pos = value; break;
		case 2: v->y_pos = value; break;
		case 3: v->z_pos = value; break;
		case 4: v->tile = TileIndex{static_cast<uint32_t>(value)}; break;
		case 5: v->dest_tile = TileIndex{static_cast<uint32_t>(value)}; break;
		case 6: v->direction = static_cast<Direction>(value); break;
		case 7: v->age = TimerGameCalendar::Date{static_cast<int32_t>(value)}; break;
		case 8: v->tick_counter = value; break;
		case 9: v->owner = CompanyID{static_cast<uint8_t>(value)}; break;
		case 10: v->vehstatus = VehStates{static_cast<uint8_t>(value)}; break;
		case 11: v->SetNext(Vehicle::Get(static_cast<uint32_t>(value))); break;
		case 12: v->sprite_cache.sprite_seq.Set(value); break;
		case 16: v->breakdown_ctr = value; break;
		case 17: v->breakdown_delay = value; break;
		case 19: RoadVehicle::From(v)->disaster_vehicle = VehicleID{static_cast<uint32_t>(value)}; break;
	}
}

static int64_t OPENTTD_DISASTER_CALL DisasterService(void *, const OpenTTDDisasterAction *request) noexcept
{
	const auto &step = *request;
	int64_t response = 0;
	switch (step.kind) {
		case 4: Vehicle::Get(step.id)->UpdatePositionAndViewport(); break;
		case 5: response = GetAircraftFlightLevel(DisasterVehicle::Get(step.id)); break;
		case 6: { int z; GetAircraftFlightLevelBounds(DisasterVehicle::Get(step.id), &z, nullptr); response = z; break; }
		case 7: CreateEffectVehicleRel(Vehicle::Get(step.id), step.a, step.b, step.c, static_cast<EffectVehicleType>(step.other)); break;
		case 8: CreateEffectVehicleAbove(step.a, step.b, step.c, static_cast<EffectVehicleType>(step.other)); break;
		case 9: SndPlayVehicleFx(SND_12_EXPLOSION, Vehicle::Get(step.id)); break;
		case 10: SndPlayTileFx(SND_12_EXPLOSION, TileIndex{step.id}); break;
		case 11:
			switch (step.other) {
				case 0: AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_ZEPPELIN, step.a), NewsType::Accident, TileIndex{step.id}); break;
				case 1: AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_SMALL_UFO), NewsType::Accident, TileIndex{step.id}); break;
				case 2: AddIndustryNewsItem(GetEncodedString(STR_NEWS_DISASTER_AIRPLANE_OIL_REFINERY, step.a), NewsType::Accident, IndustryID{static_cast<uint16_t>(step.id)}); break;
				case 3: AddIndustryNewsItem(GetEncodedString(STR_NEWS_DISASTER_HELICOPTER_FACTORY, step.a), NewsType::Accident, IndustryID{static_cast<uint16_t>(step.id)}); break;
				case 4: AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_BIG_UFO, step.a), NewsType::Accident, TileIndex{step.id}); break;
				case 5: AddTileNewsItem(GetEncodedString(STR_NEWS_DISASTER_COAL_MINE_SUBSIDENCE, step.a), NewsType::Accident, TileIndex{step.id}); break;
			}
			break;
		case 12:
			if (step.other == 0) { TileIndex tile{step.id}; AI::NewEvent(GetTileOwner(tile), new ScriptEventDisasterZeppelinerCrashed(GetStationIndex(tile))); }
			else if (step.other == 1) { TileIndex tile{step.id}; AI::NewEvent(GetTileOwner(tile), new ScriptEventDisasterZeppelinerCleared(StationID{static_cast<uint16_t>(step.a)})); }
			else {
				RoadVehicle *target = RoadVehicle::Get(step.id);
				AI::NewEvent(target->owner, new ScriptEventVehicleCrashed(target->index, target->tile, ScriptEventVehicleCrashed::CRASH_RV_UFO, step.a, target->owner));
				Game::NewEvent(new ScriptEventVehicleCrashed(target->index, target->tile, ScriptEventVehicleCrashed::CRASH_RV_UFO, step.a, target->owner));
			}
			break;
		case 13: {
			Station *st = Station::Get(step.id);
			if (step.other) st->airport.blocks.Set({AirportBlock::Zeppeliner, AirportBlock::RunwayIn});
			else st->airport.blocks.Reset({AirportBlock::Zeppeliner, AirportBlock::RunwayIn});
			break;
		}
		case 15: response = (new DisasterVehicle(step.a, step.b, static_cast<Direction>(step.c), static_cast<DisasterSubType>(step.id), VehicleID{step.other}))->index.base(); break;
		case 16: response = EnsureNoVehicleOnGround(TileIndex{step.id}).Failed(); break;
		case 18: UpdateSignalsInBuffer(); break;
		case 19: DoClearSquare(TileIndex{step.id}); break;
		case 20: ResetIndustryConstructionStage(TileIndex{step.id}); MarkTileDirtyByTile(TileIndex{step.id}); break;
		case 21: response = ClosestTownFromTile(TileIndex{step.id}, UINT_MAX)->index.base(); break;
		case 23: response = TrackStatusToTrackBits(GetTileTrackStatus(TileIndex{step.id}, TRANSPORT_WATER, 0)); break;
		case 24: DisasterVehicle::Get(step.id)->UpdateDeltaXY(); break;
	}
	return response;
}

static int64_t RunDisaster(uint32_t operation, uint32_t id = 0, int64_t a = 0, int64_t b = 0, int64_t c = 0, int64_t d = 0)
{
	std::unique_ptr<OpenTTDDisasterRun, decltype(&openttd_rust_disaster_destroy)> run{
		openttd_rust_disaster_create(operation, id, a, b, c, d, nullptr, DisasterRead, DisasterWrite, &GetRustSharedServices(), DisasterService), openttd_rust_disaster_destroy};
	int64_t response = 0;
	for (;;) {
		auto step = openttd_rust_disaster_advance(run.get(), response);
		response = 0;
		switch (step.kind) {
			case 0: return step.a;
			case 2: delete Vehicle::Get(step.id); break;
			case 14: response = RoadVehicle::Get(step.id)->Crash(); break;
			case 17: {
				Backup<CompanyID> cur_company(_current_company, step.other == 0 ? OWNER_WATER : OWNER_NONE);
				Command<CMD_LANDSCAPE_CLEAR>::Do(DoCommandFlag::Execute, TileIndex{step.id});
				cur_company.Restore(); break;
			}
		}
	}
}

DisasterVehicle::DisasterVehicle(int x, int y, Direction direction, DisasterSubType subtype, VehicleID target) :
	SpecializedVehicleBase(), rust_state{openttd_rust_disaster_state_create(target.base())}
{
	RunDisaster(3, this->index.base(), x, y, direction, subtype);
}
void DisasterVehicle::UpdatePosition(int x, int y, int z) { RunDisaster(4, this->index.base(), x, y, z); }
void DisasterVehicle::UpdateImage() { RunDisaster(9, this->index.base()); }
bool DisasterVehicle::Tick() { return RunDisaster(0, this->index.base()) != 0; }
void StartupDisasters() { RunDisaster(2); }
void ReleaseDisastersTargetingIndustry(IndustryID industry) { RunDisaster(5, industry.base()); }
void ReleaseDisasterVehicle(VehicleID vehicle)
{
	DisasterVehicle *v = DisasterVehicle::GetIfValid(vehicle);
	if (v == nullptr) return;
	assert(v->subtype == ST_SMALL_UFO);
	assert(v->State() != 0);
	RunDisaster(6, vehicle.base());
}
static const IntervalTimer<TimerGameEconomy> _economy_disaster_daily({TimerGameEconomy::DAY, TimerGameEconomy::Priority::DISASTER}, [](auto) { RunDisaster(1); });
