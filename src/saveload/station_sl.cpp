/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_sl.cpp Code handling saving and loading of stations. */

#include "../stdafx.h"
#include "../rust/station_association_ffi.h"

#include "saveload.h"
#include "compat/station_sl_compat.h"

#include "../station_base.h"
#include "../waypoint_base.h"
#include "../roadstop_base.h"
#include "../vehicle_base.h"
#include "../newgrf_station.h"
#include "../newgrf_roadstop.h"
#include "../timer/timer_game_calendar.h"
#include "../rust/station_queue_save.hpp"

#include "table/strings.h"

#include "../safeguards.h"

static SaveLoad StationLoadingVehicles(SaveLoadVersion from, SaveLoadVersion to)
{
	return {"loading_vehicles", SL_REFLIST, REF_VEHICLE, 0, from, to, [](void *object, size_t) -> void * {
		return RustStationLoadingQueueSaveScope::Address(&static_cast<Station *>(object)->Record());
	}, 0, nullptr};
}

/**
 * Update the buoy orders to be waypoint orders.
 * @param o the order 'list' to check.
 */
static void UpdateWaypointOrder(Order &o)
{
	if (!o.IsType(OT_GOTO_STATION)) return;

	const Station *st = Station::Get(o.GetDestination().ToStationID());
	if ((st->GetHadVehicleOfType() & HVOT_WAYPOINT) == 0) return;

	o.MakeGoToWaypoint(o.GetDestination().ToStationID());
}

/**
 * Perform all steps to upgrade from the old station buoys to the new version
 * that uses waypoints. This includes some old saveload mechanics.
 */
void MoveBuoysToWaypoints()
{
	/* Buoy orders become waypoint orders */
	for (OrderList *ol : OrderList::Iterate()) {
		VehicleType vt = ol->GetFirstSharedVehicle()->GetType();
		if (vt != VEH_SHIP && vt != VEH_TRAIN) continue;

		for (Order &o : ol->GetOrders()) UpdateWaypointOrder(o);
	}

	for (Vehicle *v : Vehicle::Iterate()) {
		VehicleType vt = v->GetType();
		if (vt != VEH_SHIP && vt != VEH_TRAIN) continue;

		UpdateWaypointOrder(v->current_order);
	}

	/* Now make the stations waypoints */
	for (Station *st : Station::Iterate()) {
		if ((st->GetHadVehicleOfType() & HVOT_WAYPOINT) == 0) continue;

		StationID index    = st->index;
		TileIndex xy       = st->GetXY();
		Town *town         = st->Record().town;
		StringID string_id = st->Record().string_id;
		std::string name   = st->name;
		TimerGameCalendar::Date build_date = st->GetBuildDate();
		/* TTDPatch could use "buoys with rail station" for rail waypoints */
		bool train         = st->GetTrainStation().tile != INVALID_TILE;
		TileArea train_st  = st->GetTrainStation();

		/* Delete the station, so we can make it a real waypoint. */
		delete st;

		/* Stations and waypoints are in the same pool, so if a station
		 * is deleted there must be place for a Waypoint. */
		assert(Waypoint::CanAllocateItem());
		Waypoint *wp   = new (index) Waypoint(xy);
		wp->Record().town       = town;
		wp->Record().string_id  = train ? STR_SV_STNAME_WAYPOINT : STR_SV_STNAME_BUOY;
		wp->name       = std::move(name);
		wp->Record().delete_ctr = 0; // Just reset delete counter for once.
		wp->SetBuildDate(build_date);
		wp->SetOwner(train ? GetTileOwner(xy) : OWNER_NONE);

		if (IsInsideBS(string_id, STR_SV_STNAME_BUOY, 9)) wp->Record().town_cn = string_id - STR_SV_STNAME_BUOY;

		if (train) {
			/* When we make a rail waypoint of the station, convert the map as well. */
			for (TileIndex t : train_st) {
				Tile tile(t);
				if (!IsTileType(tile, MP_STATION) || GetStationIndex(tile) != index) continue;

				SB(tile.m6(), 3, 3, to_underlying(StationType::RailWaypoint));
				wp->GetRect().BeforeAddTile(t, StationRect::ADD_FORCE);
			}

			wp->SetTrainStation(train_st);
			wp->SetFacilities(wp->GetFacilities().Set(StationFacility::Train));
		} else if (IsBuoyTile(xy) && GetStationIndex(xy) == index) {
			wp->GetRect().BeforeAddTile(xy, StationRect::ADD_FORCE);
			wp->SetFacilities(wp->GetFacilities().Set(StationFacility::Dock));
		}
	}
}

void AfterLoadStations()
{
	/* Update the speclists of all stations to point to the currently loaded custom stations. */
	for (BaseStation *st : BaseStation::Iterate()) {
		openttd_rust_station_spec_rebind(&GetStationAssociationServices(), st);

		if (Station::IsExpected(st)) {
			Station *sta = Station::From(st);
			for (const RoadStop *rs = sta->GetBusStops(); rs != nullptr; rs = rs->GetNext()) { TileArea area = sta->GetBusStation(); area.Add(rs->GetXY()); sta->SetBusStation(area); }
			for (const RoadStop *rs = sta->GetTruckStops(); rs != nullptr; rs = rs->GetNext()) { TileArea area = sta->GetTruckStation(); area.Add(rs->GetXY()); sta->SetTruckStation(area); }
		}

		StationUpdateCachedTriggers(st);
		RoadStopUpdateCachedTriggers(st);
	}

	/* Station blocked, wires and pylon flags need to be stored in the map. This is effectively cached data, so no
	 * version check is necessary. */
	for (const auto t : Map::Iterate()) {
		if (HasStationTileRail(t)) SetRailStationTileFlags(t, GetStationSpec(t));
	}
}

/**
 * (Re)building of road stop caches after loading a savegame.
 */
void AfterLoadRoadStops()
{
	openttd_rust_road_stop_after_load(&GetRustSharedServices());
}

static const SaveLoad _roadstop_desc[] = {
	SLE_VARNAME(RoadStop, Record().xy, "xy", SLE_UINT32),
	SLE_VARNAME(RoadStop, Record().status, "status", SLE_UINT8),
	SLE_GENERAL_NAME(SL_REF, "next", RoadStop, Record().next, REF_ROADSTOPS, 0, SL_MIN_VERSION, SL_MAX_VERSION, 0),
};

static uint16_t _waiting_acceptance;
static uint32_t _old_num_flows;
static uint16_t _cargo_source;
static uint32_t _cargo_source_xy;
static uint8_t  _cargo_periods;
static Money  _cargo_feeder_share;

std::list<CargoPacket *> _packets;
uint32_t _old_num_dests;

struct FlowSaveLoad {
	StationID source;
	StationID via;
	uint32_t share;
	bool restricted;
};

typedef std::pair<const StationID, std::list<CargoPacket *> > StationCargoPair;

static OldPersistentStorage _old_st_persistent_storage;

/**
 * Swap the temporary packets with the packets without specific destination in
 * the given goods entry. Assert that at least one of those is empty.
 * @param ge Goods entry to swap with.
 */
static void SwapPackets(GoodsEntry *ge)
{
	auto snapshot = ge->GetOrCreateData().cargo.Packets();
	StationCargoPacketMap &ge_packets = const_cast<StationCargoPacketMap &>(*snapshot);

	if (_packets.empty()) {
		std::map<StationID, std::list<CargoPacket *> >::iterator it(ge_packets.find(StationID::Invalid()));
		if (it == ge_packets.end()) {
			return;
		} else {
			it->second.swap(_packets);
		}
	} else {
		assert(ge_packets[StationID::Invalid()].empty());
		ge_packets[StationID::Invalid()].swap(_packets);
	}
	ge->GetData().cargo.ImportPackets(ge_packets);
}

template <typename T>
class SlStationSpecList : public VectorSaveLoadHandler<SlStationSpecList<T>, BaseStation, SpecMapping<T>> {
public:
	static inline const SaveLoad description[] = {
		SLE_CONDVAR(SpecMapping<T>, grfid,    SLE_UINT32,                SLV_27,                    SL_MAX_VERSION),
		SLE_CONDVAR(SpecMapping<T>, localidx, SLE_FILE_U8 | SLE_VAR_U16, SLV_27,                    SLV_EXTEND_ENTITY_MAPPING),
		SLE_CONDVAR(SpecMapping<T>, localidx, SLE_UINT16,                SLV_EXTEND_ENTITY_MAPPING, SL_MAX_VERSION),
	};
	static inline const SaveLoadCompatTable compat_description = _station_spec_list_sl_compat;

	static inline uint8_t last_num_specs; ///< Number of specs of the last loaded station.

	std::vector<SpecMapping<T>> &GetVector(BaseStation *bst) const override { return GetStationSpecList<T>(bst); }

	size_t GetLength() const override
	{
		return IsSavegameVersionBefore(SLV_SAVELOAD_LIST_LENGTH) ? last_num_specs : SlGetStructListLength(UINT8_MAX);
	}
};

/* Instantiate SlStationSpecList classes. */
template class SlStationSpecList<StationSpec>;
template class SlStationSpecList<RoadStopSpec>;

class SlStationCargo : public DefaultSaveLoadHandler<SlStationCargo, OpenTTDGoodsEntry> {
public:
	static inline const SaveLoad description[] = {
		    SLE_VAR(StationCargoPair, first,  SLE_UINT16),
		SLE_REFLIST(StationCargoPair, second, REF_CARGO_PACKET),
	};
	static inline const SaveLoadCompatTable compat_description = _station_cargo_sl_compat;

	void Save(OpenTTDGoodsEntry *slot) const override
	{
		GoodsEntry view{slot};
		GoodsEntry *ge = &view;
		if (!ge->HasData()) {
			SlSetStructListLength(0);
			return;
		}

		const auto packets = ge->GetData().cargo.Packets();
		SlSetStructListLength(packets->MapSize());
		for (StationCargoPacketMap::ConstMapIterator it(packets->begin()); it != packets->end(); ++it) {
			SlObject(const_cast<StationCargoPacketMap::value_type *>(&(*it)), this->GetDescription());
		}
	}

	void Load(OpenTTDGoodsEntry *slot) const override
	{
		GoodsEntry view{slot};
		GoodsEntry *ge = &view;
		size_t num_dests = IsSavegameVersionBefore(SLV_SAVELOAD_LIST_LENGTH) ? _old_num_dests : SlGetStructListLength(UINT32_MAX);
		if (num_dests == 0) return;

		GoodsEntry::GoodsEntryData &data = ge->GetOrCreateData();
		auto packets = std::make_unique<StationCargoPacketMap>(*data.cargo.Packets());
		StationCargoPair pair;
		for (uint j = 0; j < num_dests; ++j) {
			SlObject(&pair, this->GetLoadDescription());
			(*packets)[pair.first].swap(pair.second);
			assert(pair.second.empty());
		}
		data.cargo.ImportPackets(*packets);
	}

	void FixPointers(OpenTTDGoodsEntry *slot) const override
	{
		GoodsEntry view{slot};
		GoodsEntry *ge = &view;
		if (!ge->HasData()) return;

		auto packets = ge->GetData().cargo.Packets();
		for (StationCargoPacketMap::ConstMapIterator it = packets->begin(); it != packets->end(); ++it) {
			SlObject(const_cast<StationCargoPair *>(&(*it)), this->GetDescription());
		}
		ge->GetData().cargo.ImportPackets(*packets);
	}
};

class SlStationFlow : public DefaultSaveLoadHandler<SlStationFlow, OpenTTDGoodsEntry> {
public:
	static inline const SaveLoad description[] = {
		    SLE_VAR(FlowSaveLoad, source,     SLE_UINT16),
		    SLE_VAR(FlowSaveLoad, via,        SLE_UINT16),
		    SLE_VAR(FlowSaveLoad, share,      SLE_UINT32),
		SLE_CONDVAR(FlowSaveLoad, restricted, SLE_BOOL, SLV_187, SL_MAX_VERSION),
	};
	static inline const SaveLoadCompatTable compat_description = _station_flow_sl_compat;

	void Save(OpenTTDGoodsEntry *slot) const override
	{
		GoodsEntry view{slot};
		GoodsEntry *ge = &view;
		if (!ge->HasData()) {
			SlSetStructListLength(0);
			return;
		}

		size_t num_flows = 0;
		for (const auto &it : ge->GetData().flows) {
			num_flows += it.second.GetShares()->size();
		}
		SlSetStructListLength(num_flows);

		for (const auto &outer_it : ge->GetData().flows) {
			const auto *shares = outer_it.second.GetShares();
			uint32_t sum_shares = 0;
			FlowSaveLoad flow{};
			flow.source = outer_it.first;
			for (auto &inner_it : *shares) {
				flow.via = inner_it.second;
				flow.share = inner_it.first - sum_shares;
				flow.restricted = inner_it.first > outer_it.second.GetUnrestricted();
				sum_shares = inner_it.first;
				assert(flow.share > 0);
				SlObject(&flow, this->GetDescription());
			}
		}
	}

	void Load(OpenTTDGoodsEntry *slot) const override
	{
		GoodsEntry view{slot};
		GoodsEntry *ge = &view;
		size_t num_flows = IsSavegameVersionBefore(SLV_SAVELOAD_LIST_LENGTH) ? _old_num_flows : SlGetStructListLength(UINT32_MAX);
		if (num_flows == 0) return;

		GoodsEntry::GoodsEntryData &data = ge->GetOrCreateData();
		FlowSaveLoad flow{};
		FlowStat *fs = nullptr;
		FlowStatMap::iterator current_flow;
		StationID prev_source = StationID::Invalid();
		for (uint32_t j = 0; j < num_flows; ++j) {
			SlObject(&flow, this->GetLoadDescription());
			if (fs == nullptr || prev_source != flow.source) {
				current_flow = data.flows.emplace(flow.source, FlowStat(flow.via, flow.share, flow.restricted)).first;
				fs = &current_flow->second;
			} else {
				fs->AppendShare(flow.via, flow.share, flow.restricted);
			}
			prev_source = flow.source;
		}
	}
};

class SlStationGoods : public DefaultSaveLoadHandler<SlStationGoods, BaseStation> {
public:
	static inline uint cargo_reserved_count;

	static inline const SaveLoad description[] = {
		SLEG_CONDVAR("waiting_acceptance", _waiting_acceptance, SLE_UINT16,        SL_MIN_VERSION, SLV_68),
		 SLE_CONDVAR(OpenTTDGoodsEntry, status,               SLE_UINT8,                  SLV_68, SL_MAX_VERSION),
		     SLE_VAR(OpenTTDGoodsEntry, time_since_pickup,    SLE_UINT8),
		     SLE_VAR(OpenTTDGoodsEntry, rating,               SLE_UINT8),
		SLEG_CONDVAR("cargo_source", _cargo_source,    SLE_FILE_U8 | SLE_VAR_U16,   SL_MIN_VERSION, SLV_7),
		SLEG_CONDVAR("cargo_source", _cargo_source,    SLE_UINT16,                  SLV_7, SLV_68),
		SLEG_CONDVAR("cargo_source_xy", _cargo_source_xy, SLE_UINT32,               SLV_44, SLV_68),
		SLEG_CONDVAR("cargo_days", _cargo_periods,     SLE_UINT8,                   SL_MIN_VERSION, SLV_68),
		     SLE_VAR(OpenTTDGoodsEntry, last_speed,           SLE_UINT8),
		     SLE_VAR(OpenTTDGoodsEntry, last_age,             SLE_UINT8),
		SLEG_CONDVAR("cargo_feeder_share", _cargo_feeder_share,  SLE_FILE_U32 | SLE_VAR_I64, SLV_14, SLV_65),
		SLEG_CONDVAR("cargo_feeder_share", _cargo_feeder_share,  SLE_INT64,                  SLV_65, SLV_68),
		 SLE_CONDVAR(OpenTTDGoodsEntry, amount_fract,         SLE_UINT8,                 SLV_150, SL_MAX_VERSION),
		SLEG_CONDREFLIST("packets", _packets,          REF_CARGO_PACKET,           SLV_68, SLV_183),
		SLEG_CONDVAR("old_num_dests", _old_num_dests,  SLE_UINT32,                SLV_183, SLV_SAVELOAD_LIST_LENGTH),
		SLEG_CONDVAR("cargo.reserved_count", SlStationGoods::cargo_reserved_count, SLE_UINT,                  SLV_181, SL_MAX_VERSION),
		 SLE_CONDVAR(OpenTTDGoodsEntry, link_graph,           SLE_UINT16,                SLV_183, SL_MAX_VERSION),
		 SLE_CONDVAR(OpenTTDGoodsEntry, node,                 SLE_UINT16,                SLV_183, SL_MAX_VERSION),
		SLEG_CONDVAR("old_num_flows", _old_num_flows,  SLE_UINT32,                SLV_183, SLV_SAVELOAD_LIST_LENGTH),
		 SLE_CONDVAR(OpenTTDGoodsEntry, max_waiting_cargo,    SLE_UINT32,                SLV_183, SL_MAX_VERSION),
		SLEG_CONDSTRUCTLIST("flow", SlStationFlow,                                SLV_183, SL_MAX_VERSION),
		SLEG_CONDSTRUCTLIST("cargo", SlStationCargo,                              SLV_183, SL_MAX_VERSION),
	};

	static inline const SaveLoadCompatTable compat_description = _station_goods_sl_compat;

	/**
	 * Get the number of cargoes used by this savegame version.
	 * @return The number of cargoes used by this savegame version.
	 */
	size_t GetNumCargo() const
	{
		if (IsSavegameVersionBefore(SLV_55)) return 12;
		if (IsSavegameVersionBefore(SLV_EXTEND_CARGOTYPES)) return 32;
		if (IsSavegameVersionBefore(SLV_SAVELOAD_LIST_LENGTH)) return NUM_CARGO;
		/* Read from the savegame how long the list is. */
		return SlGetStructListLength(NUM_CARGO);
	}

	void Save(BaseStation *bst) const override
	{
		Station *st = Station::From(bst);

		SlSetStructListLength(NUM_CARGO);

		for (GoodsEntry ge : st->Goods()) {
			SlStationGoods::cargo_reserved_count = ge.HasData() ? ge.GetData().cargo.ReservedCount() : 0;
			SlObject(&ge.Record(), this->GetDescription());
		}
	}

	void Load(BaseStation *bst) const override
	{
		Station *st = Station::From(bst);

		/* Before savegame version 161, persistent storages were not stored in a pool. */
		if (IsSavegameVersionBefore(SLV_161) && !IsSavegameVersionBefore(SLV_145) && st->GetFacilities().Test(StationFacility::Airport)) {
			/* Store the old persistent storage. The GRFID will be added later. */
			assert(PersistentStorage::CanAllocateItem());
			st->Record().airport.psa = new PersistentStorage(0, GSF_INVALID, TileIndex{});
			std::copy(std::begin(_old_st_persistent_storage.storage), std::end(_old_st_persistent_storage.storage), std::begin(st->Record().airport.psa->storage));
		}

		const size_t num_cargo = std::min<size_t>(this->GetNumCargo(), NUM_CARGO);
		for (CargoType cargo = 0; cargo < num_cargo; ++cargo) {
			GoodsEntry ge = st->GetGoods(cargo);
			SlObject(&ge.Record(), this->GetLoadDescription());
			if (!IsSavegameVersionBefore(SLV_181) && SlStationGoods::cargo_reserved_count != 0) {
				ge.GetOrCreateData().cargo.SetReservedCount(SlStationGoods::cargo_reserved_count);
			}
			if (IsSavegameVersionBefore(SLV_183)) {
				SwapPackets(&ge);
			}
			if (IsSavegameVersionBefore(SLV_68)) {
				ge.SetStatus(ge.GetStatus().Set(GoodsEntry::State::Acceptance, HasBit(_waiting_acceptance, 15)));
				if (GB(_waiting_acceptance, 0, 12) != 0) {
					/* In old versions, enroute_from used 0xFF as StationID::Invalid() */
					StationID source = (IsSavegameVersionBefore(SLV_7) && _cargo_source == 0xFF) ? StationID::Invalid() : static_cast<StationID>(_cargo_source);

					/* Make sure we can allocate the CargoPacket. This is safe
					 * as there can only be ~64k stations and 32 cargoes in these
					 * savegame versions. As the CargoPacketPool has more than
					 * 16 million entries; it fits by an order of magnitude. */
					assert(CargoPacket::CanAllocateItem());

					/* Don't construct the packet with station here, because that'll fail with old savegames */
					CargoPacket *cp = new CargoPacket(GB(_waiting_acceptance, 0, 12), _cargo_periods, source, TileIndex{_cargo_source_xy}, _cargo_feeder_share);
					ge.GetOrCreateData().cargo.Append(cp, StationID::Invalid());
					ge.SetStatus(ge.GetStatus().Set(GoodsEntry::State::Rating));
				}
			}
		}
	}

	void FixPointers(BaseStation *bst) const override
	{
		Station *st = Station::From(bst);

		size_t num_cargo = IsSavegameVersionBefore(SLV_55) ? 12 : IsSavegameVersionBefore(SLV_EXTEND_CARGOTYPES) ? 32 : NUM_CARGO;
		for (CargoType cargo = 0; cargo < std::min<size_t>(num_cargo, NUM_CARGO); ++cargo) {
			GoodsEntry ge = st->GetGoods(cargo);
			if (IsSavegameVersionBefore(SLV_183)) {
				SwapPackets(&ge); // We have to swap back again to be in the format pre-183 expects.
				SlObject(&ge.Record(), this->GetDescription());
				SwapPackets(&ge);
			} else {
				SlObject(&ge.Record(), this->GetDescription());
			}
		}
	}
};

static const SaveLoad _old_station_desc[] = {
	SLE_CONDVARNAME(Station, Record().xy, "xy", SLE_FILE_U16 | SLE_VAR_U32, SL_MIN_VERSION, SLV_6),
	SLE_CONDVARNAME(Station, Record().xy, "xy", SLE_UINT32, SLV_6, SL_MAX_VERSION),
	SLE_CONDVARNAME(Station, Record().train_station.tile, "train_station.tile", SLE_FILE_U16 | SLE_VAR_U32, SL_MIN_VERSION, SLV_6),
	SLE_CONDVARNAME(Station, Record().train_station.tile, "train_station.tile", SLE_UINT32, SLV_6, SL_MAX_VERSION),
	SLE_CONDVARNAME(Station, Record().airport.area.tile, "airport.tile", SLE_FILE_U16 | SLE_VAR_U32, SL_MIN_VERSION, SLV_6),
	SLE_CONDVARNAME(Station, Record().airport.area.tile, "airport.tile", SLE_UINT32, SLV_6, SL_MAX_VERSION),
	    SLE_GENERAL_NAME(SL_REF, "town", Station, Record().town, REF_TOWN, 0, SL_MIN_VERSION, SL_MAX_VERSION, 0),
	    SLE_VARNAME(Station, Record().train_station.w, "train_station.w", SLE_FILE_U8 | SLE_VAR_U16),
	SLE_CONDVARNAME(Station, Record().train_station.h, "train_station.h", SLE_FILE_U8 | SLE_VAR_U16, SLV_2, SL_MAX_VERSION),

	    SLE_VARNAME(Station, Record().string_id, "string_id", SLE_STRINGID),
	SLE_CONDSSTR(Station, name,                      SLE_STR | SLF_ALLOW_CONTROL, SLV_84, SL_MAX_VERSION),
	SLE_CONDVARNAME(Station, Record().indtype, "indtype", SLE_UINT8, SLV_103, SL_MAX_VERSION),
	SLE_CONDVARNAME(Station, Record().had_vehicle_of_type, "had_vehicle_of_type", SLE_FILE_U16 | SLE_VAR_U8, SL_MIN_VERSION, SLV_122),
	SLE_CONDVARNAME(Station, Record().had_vehicle_of_type, "had_vehicle_of_type", SLE_UINT8, SLV_122, SL_MAX_VERSION),

	    SLE_VARNAME(Station, Record().time_since_load, "time_since_load", SLE_UINT8),
	    SLE_VARNAME(Station, Record().time_since_unload, "time_since_unload", SLE_UINT8),
	    SLE_VARNAME(Station, Record().delete_ctr, "delete_ctr", SLE_UINT8),
	    SLE_VARNAME(Station, Record().owner, "owner", SLE_UINT8),
	    SLE_VARNAME(Station, Record().facilities, "facilities", SLE_UINT8),
	    SLE_VARNAME(Station, Record().airport.type, "airport.type", SLE_UINT8),
	SLE_CONDVARNAME(Station, Record().airport.blocks, "airport.flags", SLE_VAR_U64 | SLE_FILE_U16, SL_MIN_VERSION, SLV_3),
	SLE_CONDVARNAME(Station, Record().airport.blocks, "airport.flags", SLE_VAR_U64 | SLE_FILE_U32, SLV_3, SLV_46),
	SLE_CONDVARNAME(Station, Record().airport.blocks, "airport.flags", SLE_UINT64, SLV_46, SL_MAX_VERSION),

	SLE_CONDVARNAME(Station, Record().last_vehicle_type, "last_vehicle_type", SLE_UINT8, SLV_26, SL_MAX_VERSION),

	SLE_CONDVARNAME(Station, Record().build_date, "build_date", SLE_FILE_U16 | SLE_VAR_I32, SLV_3, SLV_31),
	SLE_CONDVARNAME(Station, Record().build_date, "build_date", SLE_INT32, SLV_31, SL_MAX_VERSION),

	SLE_GENERAL_NAME(SL_REF, "bus_stops", Station, Record().bus_stops, REF_ROADSTOPS, 0, SLV_6, SL_MAX_VERSION, 0),
	SLE_GENERAL_NAME(SL_REF, "truck_stops", Station, Record().truck_stops, REF_ROADSTOPS, 0, SLV_6, SL_MAX_VERSION, 0),

	/* Used by newstations for graphic variations */
	SLE_CONDVARNAME(Station, Record().random_bits, "random_bits", SLE_UINT16, SLV_27, SL_MAX_VERSION),
	SLE_CONDVARNAME(Station, Record().waiting_random_triggers, "waiting_triggers", SLE_UINT8, SLV_27, SL_MAX_VERSION),
	SLEG_CONDVAR("num_specs", SlStationSpecList<StationSpec>::last_num_specs, SLE_UINT8, SLV_27, SL_MAX_VERSION),

	StationLoadingVehicles(SLV_57, SL_MAX_VERSION),

	SLEG_STRUCTLIST("goods", SlStationGoods),
	SLEG_CONDSTRUCTLIST("speclist", SlStationSpecList<StationSpec>, SLV_27, SL_MAX_VERSION),
};

struct STNSChunkHandler : ChunkHandler {
	STNSChunkHandler() : ChunkHandler('STNS', CH_READONLY) {}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_old_station_desc, _old_station_sl_compat);

		_cargo_source_xy = 0;
		_cargo_periods = 0;
		_cargo_feeder_share = 0;

		int index;
		while ((index = SlIterateArray()) != -1) {
			Station *st = new (StationID(index)) Station();

			_waiting_acceptance = 0;
			RustStationLoadingQueueSaveScope loading(&st->Record(), true);
			SlObject(st, slt);
		}
	}

	void FixPointers() const override
	{
		/* From SLV_123 we store stations in STNN; before that in STNS. So do not
		 * fix pointers when the version is SLV_123 or up, as that would fix
		 * pointers twice: once in STNN chunk and once here. */
		if (!IsSavegameVersionBefore(SLV_123)) return;

		for (Station *st : Station::Iterate()) {
			RustStationLoadingQueueSaveScope loading(&st->Record(), true);
			SlObject(st, _old_station_desc);
		}
	}
};

class SlRoadStopTileData : public VectorSaveLoadHandler<SlRoadStopTileData, BaseStation, RoadStopTileData> {
public:
	static inline const SaveLoad description[] = {
	    SLE_VAR(RoadStopTileData, tile,            SLE_UINT32),
	    SLE_VAR(RoadStopTileData, random_bits,     SLE_UINT8),
	    SLE_VAR(RoadStopTileData, animation_frame, SLE_UINT8),
	};
	static inline const SaveLoadCompatTable compat_description = {};

	std::vector<RoadStopTileData> &GetVector(BaseStation *bst) const override { return bst->custom_roadstop_tile_data; }
};

/**
 * SaveLoad handler for the BaseStation, which all other stations / waypoints
 * make use of.
 */
class SlStationBase : public DefaultSaveLoadHandler<SlStationBase, BaseStation> {
public:
	static inline const SaveLoad description[] = {
		    SLE_VARNAME(BaseStation, Record().xy, "xy", SLE_UINT32),
		    SLE_GENERAL_NAME(SL_REF, "town", BaseStation, Record().town, REF_TOWN, 0, SL_MIN_VERSION, SL_MAX_VERSION, 0),
		    SLE_VARNAME(BaseStation, Record().string_id, "string_id", SLE_STRINGID),
		   SLE_SSTR(BaseStation, name,                   SLE_STR | SLF_ALLOW_CONTROL),
		    SLE_VARNAME(BaseStation, Record().delete_ctr, "delete_ctr", SLE_UINT8),
		    SLE_VARNAME(BaseStation, Record().owner, "owner", SLE_UINT8),
		    SLE_VARNAME(BaseStation, Record().facilities, "facilities", SLE_UINT8),
		    SLE_VARNAME(BaseStation, Record().build_date, "build_date", SLE_INT32),

		/* Used by newstations for graphic variations */
		    SLE_VARNAME(BaseStation, Record().random_bits, "random_bits", SLE_UINT16),
		    SLE_VARNAME(BaseStation, Record().waiting_random_triggers, "waiting_triggers", SLE_UINT8),
	   SLEG_CONDVAR("num_specs", SlStationSpecList<StationSpec>::last_num_specs, SLE_UINT8, SL_MIN_VERSION, SLV_SAVELOAD_LIST_LENGTH),
	};
	static inline const SaveLoadCompatTable compat_description = _station_base_sl_compat;

	void Save(BaseStation *bst) const override
	{
		SlObject(bst, this->GetDescription());
	}

	void Load(BaseStation *bst) const override
	{
		SlObject(bst, this->GetLoadDescription());
	}

	void FixPointers(BaseStation *bst) const override
	{
		SlObject(bst, this->GetDescription());
	}
};

/**
 * SaveLoad handler for a normal station (read: not a waypoint).
 */
class SlStationNormal : public DefaultSaveLoadHandler<SlStationNormal, BaseStation> {
public:
	static inline const SaveLoad description[] = {
		SLEG_STRUCT("base", SlStationBase),
		    SLE_VARNAME(Station, Record().train_station.tile, "train_station.tile", SLE_UINT32),
		    SLE_VARNAME(Station, Record().train_station.w, "train_station.w", SLE_FILE_U8 | SLE_VAR_U16),
		    SLE_VARNAME(Station, Record().train_station.h, "train_station.h", SLE_FILE_U8 | SLE_VAR_U16),

		    SLE_GENERAL_NAME(SL_REF, "bus_stops", Station, Record().bus_stops, REF_ROADSTOPS, 0, SL_MIN_VERSION, SL_MAX_VERSION, 0),
		    SLE_GENERAL_NAME(SL_REF, "truck_stops", Station, Record().truck_stops, REF_ROADSTOPS, 0, SL_MIN_VERSION, SL_MAX_VERSION, 0),
		SLE_CONDVARNAME(Station, Record().ship_station.tile, "ship_station.tile", SLE_UINT32, SLV_MULTITILE_DOCKS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Station, Record().ship_station.w, "ship_station.w", SLE_FILE_U8 | SLE_VAR_U16, SLV_MULTITILE_DOCKS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Station, Record().ship_station.h, "ship_station.h", SLE_FILE_U8 | SLE_VAR_U16, SLV_MULTITILE_DOCKS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Station, Record().docking_station.tile, "docking_station.tile", SLE_UINT32, SLV_MULTITILE_DOCKS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Station, Record().docking_station.w, "docking_station.w", SLE_FILE_U8 | SLE_VAR_U16, SLV_MULTITILE_DOCKS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Station, Record().docking_station.h, "docking_station.h", SLE_FILE_U8 | SLE_VAR_U16, SLV_MULTITILE_DOCKS, SL_MAX_VERSION),
		    SLE_VARNAME(Station, Record().airport.area.tile, "airport.tile", SLE_UINT32),
		SLE_CONDVARNAME(Station, Record().airport.area.w, "airport.w", SLE_FILE_U8 | SLE_VAR_U16, SLV_140, SL_MAX_VERSION),
		SLE_CONDVARNAME(Station, Record().airport.area.h, "airport.h", SLE_FILE_U8 | SLE_VAR_U16, SLV_140, SL_MAX_VERSION),
		    SLE_VARNAME(Station, Record().airport.type, "airport.type", SLE_UINT8),
		SLE_CONDVARNAME(Station, Record().airport.layout, "airport.layout", SLE_UINT8, SLV_145, SL_MAX_VERSION),
		SLE_VARNAME(Station, Record().airport.blocks, "airport.flags", SLE_UINT64),
		SLE_CONDVARNAME(Station, Record().airport.rotation, "airport.rotation", SLE_UINT8, SLV_145, SL_MAX_VERSION),
		SLEG_CONDARR("storage", _old_st_persistent_storage.storage,  SLE_UINT32, 16, SLV_145, SLV_161),
		SLE_GENERAL_NAME(SL_REF, "airport.psa", Station, Record().airport.psa, REF_STORAGE, 0, SLV_161, SL_MAX_VERSION, 0),

		    SLE_VARNAME(Station, Record().indtype, "indtype", SLE_UINT8),

		    SLE_VARNAME(Station, Record().time_since_load, "time_since_load", SLE_UINT8),
		    SLE_VARNAME(Station, Record().time_since_unload, "time_since_unload", SLE_UINT8),
		    SLE_VARNAME(Station, Record().last_vehicle_type, "last_vehicle_type", SLE_UINT8),
		    SLE_VARNAME(Station, Record().had_vehicle_of_type, "had_vehicle_of_type", SLE_UINT8),
		StationLoadingVehicles(SL_MIN_VERSION, SL_MAX_VERSION),
		SLE_CONDVARNAME(Station, Record().always_accepted, "always_accepted", SLE_FILE_U32 | SLE_VAR_U64, SLV_127, SLV_EXTEND_CARGOTYPES),
		SLE_CONDVARNAME(Station, Record().always_accepted, "always_accepted", SLE_UINT64, SLV_EXTEND_CARGOTYPES, SL_MAX_VERSION),
		SLEG_CONDSTRUCTLIST("speclist", SlRoadStopTileData,                          SLV_NEWGRF_ROAD_STOPS, SLV_ROAD_STOP_TILE_DATA),
		SLEG_STRUCTLIST("goods", SlStationGoods),
	};
	static inline const SaveLoadCompatTable compat_description = _station_normal_sl_compat;

	void Save(BaseStation *bst) const override
	{
		if (bst->GetFacilities().Test(StationFacility::Waypoint)) return;
		SlObject(bst, this->GetDescription());
	}

	void Load(BaseStation *bst) const override
	{
		if (bst->GetFacilities().Test(StationFacility::Waypoint)) return;
		SlObject(bst, this->GetLoadDescription());
	}

	void FixPointers(BaseStation *bst) const override
	{
		if (bst->GetFacilities().Test(StationFacility::Waypoint)) return;
		SlObject(bst, this->GetDescription());
	}
};

class SlStationWaypoint : public DefaultSaveLoadHandler<SlStationWaypoint, BaseStation> {
public:
	static inline const SaveLoad description[] = {
		SLEG_STRUCT("base", SlStationBase),
		    SLE_VARNAME(Waypoint, Record().town_cn, "town_cn", SLE_UINT16),

		SLE_CONDVARNAME(Waypoint, Record().train_station.tile, "train_station.tile", SLE_UINT32, SLV_124, SL_MAX_VERSION),
		SLE_CONDVARNAME(Waypoint, Record().train_station.w, "train_station.w", SLE_FILE_U8 | SLE_VAR_U16, SLV_124, SL_MAX_VERSION),
		SLE_CONDVARNAME(Waypoint, Record().train_station.h, "train_station.h", SLE_FILE_U8 | SLE_VAR_U16, SLV_124, SL_MAX_VERSION),
		SLE_CONDVARNAME(Waypoint, Record().waypoint_flags, "waypoint_flags", SLE_UINT16, SLV_ROAD_WAYPOINTS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Waypoint, Record().road_waypoint_area.tile, "road_waypoint_area.tile", SLE_UINT32, SLV_ROAD_WAYPOINTS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Waypoint, Record().road_waypoint_area.w, "road_waypoint_area.w", SLE_FILE_U8 | SLE_VAR_U16, SLV_ROAD_WAYPOINTS, SL_MAX_VERSION),
		SLE_CONDVARNAME(Waypoint, Record().road_waypoint_area.h, "road_waypoint_area.h", SLE_FILE_U8 | SLE_VAR_U16, SLV_ROAD_WAYPOINTS, SL_MAX_VERSION),
	};
	static inline const SaveLoadCompatTable compat_description = _station_waypoint_sl_compat;

	void Save(BaseStation *bst) const override
	{
		if (!bst->GetFacilities().Test(StationFacility::Waypoint)) return;
		SlObject(bst, this->GetDescription());
	}

	void Load(BaseStation *bst) const override
	{
		if (!bst->GetFacilities().Test(StationFacility::Waypoint)) return;
		SlObject(bst, this->GetLoadDescription());
	}

	void FixPointers(BaseStation *bst) const override
	{
		if (!bst->GetFacilities().Test(StationFacility::Waypoint)) return;
		SlObject(bst, this->GetDescription());
	}
};

static const SaveLoad _station_desc[] = {
	SLE_GENERAL_NAME(SL_SAVEBYTE, "facilities", BaseStation, Record().facilities, 0, 0, SL_MIN_VERSION, SL_MAX_VERSION, 0),
	SLEG_STRUCT("normal", SlStationNormal),
	SLEG_STRUCT("waypoint", SlStationWaypoint),
	SLEG_CONDSTRUCTLIST("speclist", SlStationSpecList<StationSpec>, SLV_27, SL_MAX_VERSION),
	SLEG_CONDSTRUCTLIST("roadstopspeclist", SlStationSpecList<RoadStopSpec>, SLV_NEWGRF_ROAD_STOPS, SL_MAX_VERSION),
	SLEG_CONDSTRUCTLIST("roadstoptiledata", SlRoadStopTileData, SLV_ROAD_STOP_TILE_DATA, SL_MAX_VERSION),
};

struct STNNChunkHandler : ChunkHandler {
	STNNChunkHandler() : ChunkHandler('STNN', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_station_desc);

		/* Write the stations */
		for (BaseStation *st : BaseStation::Iterate()) {
			SlSetArrayIndex(st->index);
			RustStationLoadingQueueSaveScope loading(st->GetFacilities().Test(StationFacility::Waypoint) ? nullptr : &static_cast<Station *>(st)->Record(), false);
			SlObject(st, _station_desc);
		}
	}


	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_station_desc, _station_sl_compat);

		_old_num_flows = 0;

		int index;
		while ((index = SlIterateArray()) != -1) {
			bool waypoint = static_cast<StationFacilities>(SlReadByte()).Test(StationFacility::Waypoint);

			BaseStation *bst = waypoint ? (BaseStation *)new (StationID(index)) Waypoint() : new (StationID(index)) Station();
			RustStationLoadingQueueSaveScope loading(waypoint ? nullptr : &static_cast<Station *>(bst)->Record(), true);
			SlObject(bst, slt);
		}
	}

	void FixPointers() const override
	{
		/* From SLV_123 we store stations in STNN; before that in STNS. So do not
		 * fix pointers when the version is below SLV_123, as that would fix
		 * pointers twice: once in STNS chunk and once here. */
		if (IsSavegameVersionBefore(SLV_123)) return;

		for (BaseStation *bst : BaseStation::Iterate()) {
			RustStationLoadingQueueSaveScope loading(bst->GetFacilities().Test(StationFacility::Waypoint) ? nullptr : &static_cast<Station *>(bst)->Record(), true);
			SlObject(bst, _station_desc);
		}
	}
};

struct ROADChunkHandler : ChunkHandler {
	ROADChunkHandler() : ChunkHandler('ROAD', CH_TABLE) {}

	void Save() const override
	{
		SlTableHeader(_roadstop_desc);

		for (RoadStop *rs : RoadStop::Iterate()) {
			SlSetArrayIndex(rs->index);
			SlObject(rs, _roadstop_desc);
		}
	}

	void Load() const override
	{
		const std::vector<SaveLoad> slt = SlCompatTableHeader(_roadstop_desc, _roadstop_sl_compat);

		int index;

		while ((index = SlIterateArray()) != -1) {
			RoadStop *rs = new (RoadStopID(index)) RoadStop(INVALID_TILE);

			SlObject(rs, slt);
		}
	}

	void FixPointers() const override
	{
		for (RoadStop *rs : RoadStop::Iterate()) {
			SlObject(rs, _roadstop_desc);
		}
	}
};

static const STNSChunkHandler STNS;
static const STNNChunkHandler STNN;
static const ROADChunkHandler ROAD;
static const ChunkHandlerRef station_chunk_handlers[] = {
	STNS,
	STNN,
	ROAD,
};

extern const ChunkHandlerTable _station_chunk_handlers(station_chunk_handlers);
