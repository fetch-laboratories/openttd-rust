/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_base.h Base classes/functions for stations. */

#ifndef STATION_BASE_H
#define STATION_BASE_H

#include "core/flatset_type.hpp"
#include "core/random_func.hpp"
#include "base_station_base.h"
#include "newgrf_airport.h"
#include "cargopacket.h"
#include "industry_type.h"
#include "linkgraph/linkgraph_type.h"
#include "newgrf_storage.h"
#include "bitmap_type.h"
#include "rust/station_queue.hpp"

static const uint8_t INITIAL_STATION_RATING = 175;
static const uint8_t MAX_STATION_RATING = 255;

#include "rust/cargo_flow.hpp"

/** Call-scoped presentation of the canonical Rust attachment; never owns it. */
struct GoodsEntryData {
	OpenTTDStationGoodsData *record;
	StationCargoList cargo; ///< Borrowed cargo waiting in this station.
	FlowStatMap flows; ///< Borrowed planned flows through this station.
	explicit GoodsEntryData(OpenTTDStationGoodsData *record) : record(record), cargo(StationCargoList::Borrow(record->cargo)), flows(FlowStatMap::Borrow(record->flows)) {}

	bool IsEmpty() const
	{
		return openttd_rust_station_goods_data_is_empty(this->record) != 0;
	}
};

/**
 * Stores station stats for a single cargo: a view over one goods slot of the shared station record.
 */
struct GoodsEntry {
	/** Status of this cargo for the station. */
	enum class State : uint8_t {
		/**
		 * Set when the station accepts the cargo currently for final deliveries.
		 * It is updated every STATION_ACCEPTANCE_TICKS ticks by checking surrounding tiles for acceptance >= 8/8.
		 */
		Acceptance = 0,

		/**
		 * This indicates whether a cargo has a rating at the station.
		 * Set when cargo was ever waiting at the station.
		 * It is set when cargo supplied by surrounding tiles is moved to the station, or when
		 * arriving vehicles unload/transfer cargo without it being a final delivery.
		 *
		 * This flag is cleared after 255 * STATION_RATING_TICKS of not having seen a pickup.
		 */
		Rating = 1,

		/**
		 * Set when a vehicle ever delivered cargo to the station for final delivery.
		 * This flag is never cleared.
		 */
		EverAccepted = 2,

		/**
		 * Set when cargo was delivered for final delivery last month.
		 * This flag is set to the value of State::CurrentMonth at the start of each month.
		 */
		LastMonth = 3,

		/**
		 * Set when cargo was delivered for final delivery this month.
		 * This flag is reset on the beginning of every month.
		 */
		CurrentMonth = 4,

		/**
		 * Set when cargo was delivered for final delivery during the current STATION_ACCEPTANCE_TICKS interval.
		 * This flag is reset every STATION_ACCEPTANCE_TICKS ticks.
		 */
		AcceptedBigtick = 5,
	};
	using States = EnumBitSet<State, uint8_t>;
	using GoodsEntryData = ::GoodsEntryData;

	/** @param record The goods slot in a live station record. */
	explicit GoodsEntry(OpenTTDGoodsEntry *record) : record(record) {}

	/* The record holds max_waiting_cargo, node, link_graph, status, time_since_pickup,
	 * rating, last_speed, last_age, amount_fract and the data pointer. */
	inline OpenTTDGoodsEntry &Record() const { return *this->record; }
	inline States GetStatus() const { return States{this->record->status}; } ///< Status of this cargo, see #State.
	inline void SetStatus(States status) { this->record->status = status.base(); }
	inline LinkGraphID GetLinkGraph() const { return LinkGraphID{this->record->link_graph}; } ///< Link graph this station belongs to.
	inline void SetLinkGraph(LinkGraphID link_graph) { this->record->link_graph = link_graph.base(); }

	/**
	 * Reports whether a vehicle has ever tried to load the cargo at this station.
	 * This does not imply that there was cargo available for loading. Refer to GoodsEntry::State::Rating for that.
	 * @return true if vehicle tried to load.
	 */
	bool HasVehicleEverTriedLoading() const { return this->record->last_speed != 0; }

	/**
	 * Does this cargo have a rating at this station?
	 * @return true if the cargo has a rating, i.e. cargo has been moved to the station.
	 */
	inline bool HasRating() const
	{
		return this->GetStatus().Test(GoodsEntry::State::Rating);
	}

	/**
	 * Get the best next hop for a cargo packet from station source.
	 * @param source Source of the packet.
	 * @return The chosen next hop or StationID::Invalid() if none was found.
	 */
	inline StationID GetVia(StationID source) const
	{
		if (!this->HasData()) return StationID::Invalid();

		FlowStatMap::const_iterator flow_it(this->GetData().flows.find(source));
		return flow_it != this->GetData().flows.end() ? flow_it->second.GetVia() : StationID::Invalid();
	}

	/**
	 * Get the best next hop for a cargo packet from station source, optionally
	 * excluding one or two stations.
	 * @param source Source of the packet.
	 * @param excluded If this station would be chosen choose the second best one instead.
	 * @param excluded2 Second station to be excluded, if != StationID::Invalid().
	 * @return The chosen next hop or StationID::Invalid() if none was found.
	 */
	inline StationID GetVia(StationID source, StationID excluded, StationID excluded2 = StationID::Invalid()) const
	{
		if (!this->HasData()) return StationID::Invalid();

		FlowStatMap::const_iterator flow_it(this->GetData().flows.find(source));
		return flow_it != this->GetData().flows.end() ? flow_it->second.GetVia(excluded, excluded2) : StationID::Invalid();
	}

	/**
	 * Test if this goods entry has optional cargo packet/flow data.
	 * @returns true iff optional data is present.
	 */
	[[debug_inline]] inline bool HasData() const { return this->record->data != nullptr; }

	/**
	 * Clear optional cargo packet/flow data.
	 */
	void ClearData()
	{
		openttd_rust_station_goods_data_clear(this->record, CargoStorageServices());
	}

	/**
	 * Get optional cargo packet/flow data.
	 * @pre HasData()
	 * @returns cargo packet/flow data.
	 */
	[[debug_inline]] inline GoodsEntryData GetData() const
	{
		assert(this->HasData());
		return GoodsEntryData(this->record->data);
	}

	/**
	 * Get optional cargo packet/flow data. The data is create if it is not already present.
	 * @returns cargo packet/flow data.
	 */
	inline GoodsEntryData GetOrCreateData()
	{
		return GoodsEntryData(openttd_rust_station_goods_data_ensure(this->record));
	}

	uint8_t ConvertState() const;

	/**
	 * Returns sum of cargo still available for loading at the station.
	 * (i.e. not counting cargo which is already reserved for loading)
	 * @return Cargo on board the vehicle.
	 */
	inline uint AvailableCount() const
	{
		return this->HasData() ? this->GetData().cargo.AvailableCount() : 0;
	}

	/**
	 * Returns total count of cargo at the station, including
	 * cargo which is already reserved for loading.
	 * @return Total cargo count.
	 */
	inline uint TotalCount() const
	{
		return this->HasData() ? this->GetData().cargo.TotalCount() : 0;
	}

private:
	OpenTTDGoodsEntry *record; ///< The goods slot; its data pointer owns the optional cargo packet and flow data.
};

/** Iterable views over a station record's goods slots. */
struct GoodsRange {
	OpenTTDGoodsEntry *first;
	struct iterator {
		OpenTTDGoodsEntry *slot;
		GoodsEntry operator*() const { return GoodsEntry{this->slot}; }
		iterator &operator++() { ++this->slot; return *this; }
		bool operator==(const iterator &other) const = default;
	};
	iterator begin() const { return {this->first}; }
	iterator end() const { return {this->first + NUM_CARGO}; }
	static constexpr size_t size() { return NUM_CARGO; }
};

/** All airport-related information. Only valid if tile != INVALID_TILE.
 * A view over the airport of a live station record. */
#include "rust/aircraft_ffi.h"
static_assert(sizeof(AirportBlocks) == sizeof(uint64_t) && alignof(AirportBlocks) == alignof(uint64_t));
struct Airport {
	/** @param record The airport of a live station record. */
	explicit Airport(OpenTTDAirport *record) : record(record) {}

	/* The record holds the area, blocks, type, layout, rotation and psa link. */
	inline OpenTTDAirport &Record() const { return *this->record; }
	inline TileArea GetArea() const { return TileAreaOf(this->record->area); } ///< Tile area the airport covers.
	inline void SetArea(const TileArea &area) { StoreTileArea(this->record->area, area); }
	inline TileIndex GetTile() const { return TileIndex{this->record->area.tile}; } ///< The northern tile of the airport, or INVALID_TILE.
	inline AirportBlocks GetBlocks() const { return AirportBlocks{this->record->blocks}; } ///< Stores which blocks on the airport are taken.
	inline void SetBlocks(AirportBlocks blocks) { this->record->blocks = blocks.base(); }
	inline Direction GetRotation() const { return static_cast<Direction>(this->record->rotation); } ///< How this airport is rotated.
	inline void SetRotation(Direction rotation) { this->record->rotation = rotation; }
	inline operator TileArea() const { return this->GetArea(); }
	inline OrthogonalTileIterator begin() const { return this->GetArea().begin(); }
	inline OrthogonalTileIterator end() const { return this->GetArea().end(); }
	inline void Clear() { TileArea area = this->GetArea(); area.Clear(); this->SetArea(area); } ///< Clear the airport's tile area.

	/**
	 * Get the AirportSpec that from the airport type of this airport. If there
	 * is no airport (\c tile == INVALID_TILE) then return the dummy AirportSpec.
	 * @return The AirportSpec for this airport.
	 */
	const AirportSpec *GetSpec() const
	{
		return openttd_rust_airport_get_spec(this->record->area.tile, this->record->type, &GetAirportResolverServices());
	}

	/**
	 * Get the finite-state machine for this airport or the finite-state machine
	 * for the dummy airport in case this isn't an airport.
	 * @pre this->type < NEW_AIRPORT_OFFSET.
	 * @return The state machine for this airport.
	 */
	const AirportFTAClass *GetFTA() const
	{
		return this->GetFTA(this->GetSpec());
	}

	const AirportFTAClass *GetFTA(const AirportSpec *as) const
	{
		return as->fsm;
	}

	/** Check if this airport has at least one hangar. */
	inline bool HasHangar() const
	{
		return this->HasHangar(this->GetSpec());
	}

	inline bool HasHangar(const AirportSpec *as) const
	{
		return !as->depots.empty();
	}

	/**
	 * Add the tileoffset to the base tile of this airport but rotate it first.
	 * The base tile is the northernmost tile of this airport. This function
	 * helps to make sure that getting the tile of a hangar works even for
	 * rotated airport layouts without requiring a rotated array of hangar tiles.
	 * @param tidc The tilediff to add to the airport tile.
	 * @return The tile of this airport plus the rotated offset.
	 */
	inline TileIndex GetRotatedTileFromOffset(TileIndexDiffC tidc) const
	{
		return this->GetRotatedTileFromOffset(tidc, this->GetSpec());
	}

	inline TileIndex GetRotatedTileFromOffset(TileIndexDiffC tidc, const AirportSpec *as) const
	{
		return TileIndex{openttd_rust_airport_rotated_tile(this->record, tidc, as, &GetAirportResolverServices())};
	}

	/**
	 * Get the first tile of the given hangar.
	 * @param hangar_num The hangar to get the location of.
	 * @pre hangar_num < GetNumHangars().
	 * @return A tile with the given hangar.
	 */
	inline TileIndex GetHangarTile(uint hangar_num) const
	{
		return this->GetHangarTile(hangar_num, this->GetSpec());
	}

	inline TileIndex GetHangarTile(uint hangar_num, const AirportSpec *as) const
	{
		return TileIndex{openttd_rust_airport_hangar_tile(this->record, hangar_num, as, &GetAirportResolverServices())};
	}

	/**
	 * Get the exit direction of the hangar at a specific tile.
	 * @param tile The tile to query.
	 * @pre IsHangarTile(tile).
	 * @return The exit direction of the hangar, taking airport rotation into account.
	 */
	inline Direction GetHangarExitDirection(TileIndex tile) const
	{
		return this->GetHangarExitDirection(tile, this->GetSpec());
	}

	inline Direction GetHangarExitDirection(TileIndex tile, const AirportSpec *as) const
	{
		return static_cast<Direction>(openttd_rust_airport_hangar_exit(this->record, tile.base(), as, &GetAirportResolverServices()));
	}

	/**
	 * Get the hangar number of the hangar at a specific tile.
	 * @param tile The tile to query.
	 * @pre IsHangarTile(tile).
	 * @return The hangar number of the hangar at the given tile.
	 */
	inline uint GetHangarNum(TileIndex tile) const
	{
		return this->GetHangarNum(tile, this->GetSpec());
	}

	inline uint GetHangarNum(TileIndex tile, const AirportSpec *as) const
	{
		return openttd_rust_airport_hangar_num(this->record, tile.base(), as, &GetAirportResolverServices());
	}

	/** Get the number of hangars on this airport. */
	inline uint GetNumHangars() const
	{
		return this->GetNumHangars(this->GetSpec());
	}

	inline uint GetNumHangars(const AirportSpec *as) const
	{
		return openttd_rust_airport_num_hangars(as, &GetAirportResolverServices());
	}

private:
	OpenTTDAirport *record; ///< The airport of the station record.
};


/** Station data structure */
struct Station final : SpecializedStation<Station, false> {
public:
	RoadStop *GetPrimaryRoadStop(RoadStopType type) const
	{
		return type == RoadStopType::Bus ? this->GetBusStops() : this->GetTruckStops();
	}

	/* The shared record holds the road-stop list heads, the bus, truck, ship and docking
	 * areas, the airport, indtype, had_vehicle_of_type, industry, the loading activity
	 * fields, always_accepted and the goods slots. */
	inline RoadStop *GetBusStops() const; ///< All the road stops.
	inline void SetBusStops(RoadStop *stop);
	inline RoadStop *GetTruckStops() const; ///< All the truck stops.
	inline void SetTruckStops(RoadStop *stop);
	inline TileArea GetBusStation() const { return TileAreaOf(this->Record().bus_station); } ///< Tile area the bus 'station' part covers.
	inline void SetBusStation(const TileArea &area) { StoreTileArea(this->Record().bus_station, area); }
	inline TileArea GetTruckStation() const { return TileAreaOf(this->Record().truck_station); } ///< Tile area the truck 'station' part covers.
	inline void SetTruckStation(const TileArea &area) { StoreTileArea(this->Record().truck_station, area); }
	inline TileArea GetShipStation() const { return TileAreaOf(this->Record().ship_station); } ///< Tile area the ship 'station' part covers.
	inline void SetShipStation(const TileArea &area) { StoreTileArea(this->Record().ship_station, area); }
	inline TileArea GetDockingStation() const { return TileAreaOf(this->Record().docking_station); } ///< Tile area the docking tiles cover.
	inline void SetDockingStation(const TileArea &area) { StoreTileArea(this->Record().docking_station, area); }
	/** @return The airport of this station. */
	inline Airport GetAirport() const { return Airport{&const_cast<OpenTTDStationRecord &>(this->Record()).airport}; }
	inline StationHadVehicleOfType GetHadVehicleOfType() const { return static_cast<StationHadVehicleOfType>(this->Record().had_vehicle_of_type); }
	inline void SetHadVehicleOfType(StationHadVehicleOfType types) { this->Record().had_vehicle_of_type = types; }
	/** @return The goods entry of cargo \a cargo. */
	inline GoodsEntry GetGoods(CargoType cargo) const { return GoodsEntry{&const_cast<OpenTTDStationRecord &>(this->Record()).goods[cargo]}; }
	/** @return The goods entries of every cargo. */
	inline GoodsRange Goods() const { return GoodsRange{const_cast<OpenTTDStationRecord &>(this->Record()).goods}; }

	BitmapTileArea GetCatchmentTiles() const { return BitmapTileArea{openttd_rust_station_catchment_tiles(const_cast<OpenTTDStationRecord *>(&this->Record()))}; }

	RustStationLoadingQueue LoadingVehicles() const { return RustStationLoadingQueue{const_cast<OpenTTDStationRecord *>(&this->Record())}; }

	RustIndustryList IndustriesNear() const { return RustIndustryList{const_cast<OpenTTDStationRecord *>(&this->Record())}; }

	Station(TileIndex tile = INVALID_TILE);
	~Station();


	void MarkTilesDirty(bool cargo_change) const;

	void UpdateVirtCoord() override;


	void AfterStationTileSetChange(bool adding, StationType type);

	static void RecomputeCatchmentForAll();

	uint GetCatchmentRadius() const;
	void RemoveFromAllNearbyLists();

	inline bool TileIsInCatchment(TileIndex tile) const
	{
		return this->GetCatchmentTiles().HasTile(tile);
	}

	inline bool TileBelongsToRailStation(TileIndex tile) const override
	{
		return IsRailStationTile(tile) && GetStationIndex(tile) == this->index;
	}

	inline bool TileBelongsToRoadStop(TileIndex tile) const
	{
		return IsStationRoadStopTile(tile) && GetStationIndex(tile) == this->index;
	}

	inline bool TileBelongsToAirport(TileIndex tile) const
	{
		return IsAirportTile(tile) && GetStationIndex(tile) == this->index;
	}

	uint32_t GetNewGRFVariable(const ResolverObject &object, uint8_t variable, uint8_t parameter, bool &available) const override;

	TileArea GetTileArea(StationType type) const override;
};

/** Iterator to iterate over all tiles belonging to an airport. */
class AirportTileIterator : public OrthogonalTileIterator {
private:
	const Station *st = nullptr; ///< The station the airport is a part of.

public:
	/**
	 * Construct the iterator.
	 * @param st Station the airport is part of.
	 */
	AirportTileIterator(const Station *st) : OrthogonalTileIterator(st->GetAirport().GetArea()), st(st)
	{
		if (!st->TileBelongsToAirport(this->tile)) ++(*this);
	}

	inline TileIterator& operator ++() override
	{
		this->OrthogonalTileIterator::operator++();
		while (this->tile != INVALID_TILE && !st->TileBelongsToAirport(this->tile)) {
			this->OrthogonalTileIterator::operator++();
		}
		return *this;
	}

	std::unique_ptr<TileIterator> Clone() const override
	{
		return std::make_unique<AirportTileIterator>(*this);
	}
};


#include "roadstop_base.h"

inline RoadStop *Station::GetBusStops() const { return RoadStop::FromRecord(this->Record().bus_stops); }
inline void Station::SetBusStops(RoadStop *stop) { this->Record().bus_stops = stop == nullptr ? nullptr : stop->record; }
inline RoadStop *Station::GetTruckStops() const { return RoadStop::FromRecord(this->Record().truck_stops); }
inline void Station::SetTruckStops(RoadStop *stop) { this->Record().truck_stops = stop == nullptr ? nullptr : stop->record; }

#endif /* STATION_BASE_H */
