/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargopacket.h Base class for cargo packets. */

#ifndef CARGOPACKET_H
#define CARGOPACKET_H

#include "core/pool_type.hpp"
#include "economy_type.h"
#include "station_type.h"
#include "order_type.h"
#include "cargo_type.h"
#include "source_type.h"
#include "vehicle_type.h"
#include "core/multimap.hpp"
#include "saveload/saveload.h"

/** Unique identifier for a single cargo packet. */
using CargoPacketID = PoolID<uint32_t, struct CargoPacketIDTag, 0xFFF000, 0xFFFFFF>;
struct CargoPacket;

/** Type of the pool for cargo packets for a little over 16 million packets. */
using CargoPacketPool = Pool<CargoPacket, CargoPacketID, 1024, PoolType::Normal, true>;
/** The actual pool with cargo packets. */
extern CargoPacketPool _cargopacket_pool;

struct GoodsEntry; // forward-declare for Stage() and RerouteStalePackets()

template <class Tinst, class Tcont> class CargoList;
class StationCargoList; // forward-declare, so we can use it in VehicleCargoList.
extern SaveLoadTable GetCargoPacketDesc();

#include "rust/cargo_storage_ffi.h"
const OpenTTDCargoStorageServices *CargoStorageServices();

/** Pool shell: all packet fields have one canonical Rust owner. */
struct CargoPacket : CargoPacketPool::PoolItem<&_cargopacket_pool> {
private:
	OpenTTDCargoPacket *state;
public:
	static const uint16_t MAX_COUNT = UINT16_MAX;
	CargoPacket();
	CargoPacket(StationID first_station, uint16_t count, Source source);
	CargoPacket(uint16_t count, uint16_t periods, StationID first, TileIndex tile, Money feeder);
	explicit CargoPacket(const OpenTTDCargoPacket &fields);
	~CargoPacket();
	OpenTTDCargoPacket *RustOwner() const { return this->state; }
	void UpdateLoadingTile(TileIndex tile);
	uint16_t Count() const { return this->state->count; }
	uint16_t GetPeriodsInTransit() const { return this->state->periods_in_transit; }
	Source GetSource() const { return {this->state->source_id, static_cast<SourceType>(this->state->source_type)}; }
	StationID GetFirstStation() const { return StationID(this->state->first_station); }
	static void InvalidateAllFrom(Source src);
	static void AfterLoad();
};

typedef std::list<CargoPacket *> CargoPacketList;
typedef MultiMap<StationID, CargoPacket *> StationCargoPacketMap;
class VehicleCargoList;

/** Read exports are call-local; unresolved load references are imported outside Rust. */
template <class Tinst, class Tcont>
class CargoList {
protected:
	OpenTTDCargoList *state;
public:
	typedef typename Tcont::iterator Iterator;
	typedef typename Tcont::reverse_iterator ReverseIterator;
	typedef typename Tcont::const_iterator ConstIterator;
	typedef typename Tcont::const_reverse_iterator ConstReverseIterator;
	enum MoveToAction : uint8_t { MTA_BEGIN = 0, MTA_TRANSFER = 0, MTA_DELIVER, MTA_KEEP, MTA_LOAD, MTA_END, NUM_MOVE_TO_ACTION = MTA_END };
	CargoList();
	~CargoList();
	CargoList(const CargoList &) = delete;
	CargoList &operator=(const CargoList &) = delete;
	OpenTTDCargoList *RustOwner() const { return this->state; }
	void OnCleanPool();
	std::unique_ptr<const Tcont> Packets() const;
	void ImportPackets(const Tcont &packets);
	uint PeriodsInTransit() const { return openttd_rust_cargo_list_periods(this->state); }
	OpenTTDCargoListFields Export() const;
	void ImportMeta(const OpenTTDCargoListFields &fields);
	void InvalidateCache();
};

class VehicleCargoList : public CargoList<VehicleCargoList, CargoPacketList> {
public:
	StationID GetFirstStation() const;
	Money GetFeederShare() const { return openttd_rust_cargo_list_feeder(this->state); }
	uint StoredCount() const { return openttd_rust_cargo_list_stored(this->state); }
	uint TotalCount() const { return openttd_rust_cargo_list_count(this->state); }
	void Append(CargoPacket *cp, MoveToAction action = MTA_KEEP);
	void KeepAll();
	uint Truncate(uint amount = UINT_MAX);
};

class StationCargoList : public CargoList<StationCargoList, StationCargoPacketMap> {
public:
	void Append(CargoPacket *cp, StationID next);
	StationID GetFirstStation() const;
	size_t MapSize() const { return openttd_rust_cargo_list_key_count(this->state); }
	uint AvailableCount() const { return openttd_rust_cargo_list_count(this->state); }
	uint ReservedCount() const { return openttd_rust_cargo_list_reserved(this->state); }
	void SetReservedCount(uint count) { openttd_rust_cargo_list_set_reserved(this->state, count); }
	uint TotalCount() const { return openttd_rust_cargo_list_station_total(this->state); }
	uint Truncate(uint amount = UINT_MAX);
};

#endif /* CARGOPACKET_H */
