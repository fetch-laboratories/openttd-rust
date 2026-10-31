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
	CargoPacket(uint16_t count, Money feeder, CargoPacket &original);
	explicit CargoPacket(const OpenTTDCargoPacketFields &fields);
	~CargoPacket();
	OpenTTDCargoPacket *RustOwner() const { return this->state; }
	OpenTTDCargoPacketFields Export() const;
	void Import(const OpenTTDCargoPacketFields &fields);
	CargoPacket *Split(uint size);
	void Merge(CargoPacket *cp);
	void Reduce(uint count);
	void SetNextHop(StationID next) { openttd_rust_cargo_packet_set(this->state, 0, next.base()); }
	void UpdateLoadingTile(TileIndex tile);
	void UpdateUnloadingTile(TileIndex tile);
	void AddFeederShare(Money share) { openttd_rust_cargo_packet_feeder(this->state, share.base()); }
	uint16_t Count() const { return this->Export().count; }
	Money GetFeederShare() const { return this->Export().feeder_share; }
	Money GetFeederShare(uint count) const { return openttd_rust_cargo_packet_share(this->state, count); }
	uint16_t GetPeriodsInTransit() const { return this->Export().periods_in_transit; }
	Source GetSource() const { auto f = this->Export(); return {f.source_id, static_cast<SourceType>(f.source_type)}; }
	StationID GetFirstStation() const { return StationID(this->Export().first_station); }
	StationID GetNextHop() const { return StationID(this->Export().next_hop); }
	uint GetDistance(TileIndex tile) const;
	static void InvalidateAllFrom(Source src);
	static void InvalidateAllFrom(StationID station);
	static void AfterLoad();
};

typedef std::list<CargoPacket *> CargoPacketList;
typedef MultiMap<StationID, CargoPacket *> StationCargoPacketMap;
typedef std::map<StationID, uint> StationCargoAmountMap;
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
	uint ActionCount(MoveToAction action) const { return openttd_rust_cargo_list_action_count(this->state, action); }
	uint StoredCount() const { return openttd_rust_cargo_list_stored(this->state); }
	uint TotalCount() const { return openttd_rust_cargo_list_count(this->state); }
	uint ReservedCount() const { return this->ActionCount(MTA_LOAD); }
	uint UnloadCount() const { return openttd_rust_cargo_list_unload(this->state); }
	uint RemainingCount() const { return openttd_rust_cargo_list_remaining(this->state); }
	void Append(CargoPacket *cp, MoveToAction action = MTA_KEEP);
	void AgeCargo();
	bool Stage(bool accepted, StationID station, std::span<const StationID> next, OrderUnloadType unload, const GoodsEntry *ge, CargoType cargo, CargoPayment *payment, TileIndex tile);
	void KeepAll();
	template <MoveToAction Tfrom, MoveToAction Tto> uint Reassign(uint amount);
	uint Return(uint amount, StationCargoList *dest, StationID next, TileIndex tile);
	uint Unload(uint amount, StationCargoList *dest, CargoType cargo, CargoPayment *payment, TileIndex tile);
	uint Shift(uint amount, VehicleCargoList *dest);
	uint Truncate(uint amount = UINT_MAX);
	uint Reroute(uint amount, VehicleCargoList *dest, StationID avoid, StationID avoid2, const GoodsEntry *ge);
};

class StationCargoList : public CargoList<StationCargoList, StationCargoPacketMap> {
public:
	static void InvalidateAllFrom(Source src);
	void Append(CargoPacket *cp, StationID next);
	bool HasCargoFor(std::span<const StationID> next) const;
	StationID GetFirstStation() const;
	size_t MapSize() const { return openttd_rust_cargo_list_key_count(this->state); }
	uint AvailableCount() const { return openttd_rust_cargo_list_count(this->state); }
	uint ReservedCount() const { return openttd_rust_cargo_list_reserved(this->state); }
	void SetReservedCount(uint count) { openttd_rust_cargo_list_set_reserved(this->state, count); }
	uint TotalCount() const { return openttd_rust_cargo_list_station_total(this->state); }
	uint Reserve(uint amount, VehicleCargoList *dest, std::span<const StationID> next, TileIndex tile);
	uint Load(uint amount, VehicleCargoList *dest, std::span<const StationID> next, TileIndex tile);
	uint Truncate(uint amount = UINT_MAX, StationCargoAmountMap *origins = nullptr);
	uint Reroute(uint amount, StationCargoList *dest, StationID avoid, StationID avoid2, const GoodsEntry *ge);
};

#endif /* CARGOPACKET_H */
