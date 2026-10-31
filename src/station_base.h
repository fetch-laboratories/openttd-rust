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

/**
 * Stores station stats for a single cargo.
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

	struct GoodsEntryData {
		StationCargoList cargo{}; ///< The cargo packets of cargo waiting in this station
		FlowStatMap flows{}; ///< Planned flows through this station.

		bool IsEmpty() const
		{
			return this->cargo.TotalCount() == 0 && this->flows.empty();
		}
	};

	uint &max_waiting_cargo;
	States &status;
	uint8_t &time_since_pickup, &rating, &last_speed, &last_age, &amount_fract;
	/* Native typed status aliases the Rust-owned byte; start its object lifetime. */
	GoodsEntry(OpenTTDStationCargo *cargo) : max_waiting_cargo(cargo->max_waiting_cargo),
		status(*::new (static_cast<void *>(&cargo->status)) States), time_since_pickup(cargo->time_since_pickup),
		rating(cargo->rating), last_speed(cargo->last_speed), last_age(cargo->last_age), amount_fract(cargo->amount_fract)
	{
		static_assert(sizeof(States) == sizeof(uint8_t));
		static_assert(alignof(States) == alignof(uint8_t));
		static_assert(std::is_trivially_copyable_v<States>);
	}

	NodeID node = INVALID_NODE; ///< ID of node in link graph referring to this goods entry.
	LinkGraphID link_graph = LinkGraphID::Invalid(); ///< Link graph this station belongs to.


	/**
	 * Reports whether a vehicle has ever tried to load the cargo at this station.
	 * This does not imply that there was cargo available for loading. Refer to GoodsEntry::State::Rating for that.
	 * @return true if vehicle tried to load.
	 */
	bool HasVehicleEverTriedLoading() const { return this->last_speed != 0; }

	/**
	 * Does this cargo have a rating at this station?
	 * @return true if the cargo has a rating, i.e. cargo has been moved to the station.
	 */
	inline bool HasRating() const
	{
		return this->status.Test(GoodsEntry::State::Rating);
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
	[[debug_inline]] inline bool HasData() const { return this->data != nullptr; }

	/**
	 * Clear optional cargo packet/flow data.
	 */
	void ClearData() { this->data.reset(); }

	/**
	 * Get optional cargo packet/flow data.
	 * @pre HasData()
	 * @returns cargo packet/flow data.
	 */
	[[debug_inline]] inline const GoodsEntryData &GetData() const
	{
		assert(this->HasData());
		return *this->data;
	}

	/**
	 * Get non-const optional cargo packet/flow data.
	 * @pre HasData()
	 * @returns non-const cargo packet/flow data.
	 */
	[[debug_inline]] inline GoodsEntryData &GetData()
	{
		assert(this->HasData());
		return *this->data;
	}

	/**
	 * Get optional cargo packet/flow data. The data is create if it is not already present.
	 * @returns cargo packet/flow data.
	 */
	inline GoodsEntryData &GetOrCreateData()
	{
		if (!this->HasData()) this->data = std::make_unique<GoodsEntryData>();
		return *this->data;
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
	std::unique_ptr<GoodsEntryData> data = nullptr; ///< Optional cargo packet and flow data.
};

/** All airport-related information. Only valid if tile != INVALID_TILE. */
#include "rust/aircraft_ffi.h"
static_assert(sizeof(AirportBlocks) == sizeof(uint64_t) && alignof(AirportBlocks) == alignof(uint64_t));
static_assert(std::is_trivially_destructible_v<AirportBlocks>);
struct Airport : public TileArea {
	Airport() : TileArea(INVALID_TILE, 0, 0) {}

	std::unique_ptr<uint64_t, decltype(&openttd_rust_airport_blocks_destroy)> rust_blocks{openttd_rust_airport_blocks_new(), openttd_rust_airport_blocks_destroy};
	/* Begin the trivial bitset's lifetime in its canonical allocation. Rust reads
	 * and writes only its raw uint64 representation; allocation release ends it. */
	AirportBlocks &blocks = *new (this->rust_blocks.get()) AirportBlocks{};
	uint8_t type = 0; ///< Type of this airport, @see AirportTypes
	uint8_t layout = 0; ///< Airport layout number.
	Direction rotation = INVALID_DIR; ///< How this airport is rotated.

	PersistentStorage *psa = nullptr; ///< Persistent storage for NewGRF airports.

	/**
	 * Get the AirportSpec that from the airport type of this airport. If there
	 * is no airport (\c tile == INVALID_TILE) then return the dummy AirportSpec.
	 * @return The AirportSpec for this airport.
	 */
	const AirportSpec *GetSpec() const
	{
		return openttd_rust_airport_get_spec(this->tile.base(), this->type, &GetAirportResolverServices());
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
		switch (this->rotation) {
			case DIR_N: return this->tile + ToTileIndexDiff(tidc);

			case DIR_E: return this->tile + TileDiffXY(tidc.y, as->size_x - 1 - tidc.x);

			case DIR_S: return this->tile + TileDiffXY(as->size_x - 1 - tidc.x, as->size_y - 1 - tidc.y);

			case DIR_W: return this->tile + TileDiffXY(as->size_y - 1 - tidc.y, tidc.x);

			default: NOT_REACHED();
		}
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
		for (const auto &depot : as->depots) {
			if (depot.hangar_num == hangar_num) {
				return this->GetRotatedTileFromOffset(depot.ti, as);
			}
		}
		NOT_REACHED();
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
		const HangarTileTable *htt = GetHangarDataByTile(tile, as);
		return ChangeDir(htt->dir, DirDifference(this->rotation, as->layouts[0].rotation));
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
		const HangarTileTable *htt = GetHangarDataByTile(tile, as);
		return htt->hangar_num;
	}

	/** Get the number of hangars on this airport. */
	inline uint GetNumHangars() const
	{
		return this->GetNumHangars(this->GetSpec());
	}

	inline uint GetNumHangars(const AirportSpec *as) const
	{
		uint num = 0;
		uint counted = 0;
		for (const auto &depot : as->depots) {
			if (!HasBit(counted, depot.hangar_num)) {
				num++;
				SetBit(counted, depot.hangar_num);
			}
		}
		return num;
	}

private:
	/**
	 * Retrieve hangar information of a hangar at a given tile.
	 * @param tile %Tile containing the hangar.
	 * @return The requested hangar information.
	 * @pre The \a tile must be at a hangar tile at an airport.
	 */
	inline const HangarTileTable *GetHangarDataByTile(TileIndex tile, const AirportSpec *as) const
	{
		for (const auto &depot : as->depots) {
			if (this->GetRotatedTileFromOffset(depot.ti, as) == tile) {
				return &depot;
			}
		}
		NOT_REACHED();
	}
};


/** Station data structure */
struct Station final : SpecializedStation<Station, false> {
public:
	RoadStop *GetPrimaryRoadStop(RoadStopType type) const
	{
		return type == RoadStopType::Bus ? bus_stops : truck_stops;
	}

	RoadStop *GetPrimaryRoadStop(const struct RoadVehicle *v) const;

	RoadStop *bus_stops = nullptr; ///< All the road stops
	TileArea bus_station{}; ///< Tile area the bus 'station' part covers
	RoadStop *truck_stops = nullptr; ///< All the truck stops
	TileArea truck_station{}; ///< Tile area the truck 'station' part covers

	Airport airport{}; ///< Tile area the airport covers
	TileArea ship_station{}; ///< Tile area the ship 'station' part covers
	TileArea docking_station{}; ///< Tile area the docking tiles cover

	IndustryType indtype = IT_INVALID; ///< Industry type to get the name from

	BitmapTileArea catchment_tiles{openttd_rust_station_catchment_tiles(this->rust_service.get())};

	StationHadVehicleOfType had_vehicle_of_type{};

	RustStationLoadingQueue loading_vehicles{this->rust_service.get()};
	uint8_t &time_since_load = openttd_rust_station_service_fields(this->rust_service.get())->time_since_load;
	uint8_t &time_since_unload = openttd_rust_station_service_fields(this->rust_service.get())->time_since_unload;
	uint8_t &last_vehicle_type = openttd_rust_station_service_fields(this->rust_service.get())->last_vehicle_type;
	CargoTypes &always_accepted = openttd_rust_station_service_fields(this->rust_service.get())->always_accepted;
	template <size_t... I> static std::array<GoodsEntry, NUM_CARGO> MakeGoods(OpenTTDStationService *state, std::index_sequence<I...>)
	{
		return {{GoodsEntry(openttd_rust_station_service_cargo(state, I))...}};
	}
	std::array<GoodsEntry, NUM_CARGO> goods = MakeGoods(this->rust_service.get(), std::make_index_sequence<NUM_CARGO>{});

	RustIndustryList industries_near{this->rust_service.get()};
	Industry *industry = nullptr; ///< NOSAVE: Associated industry for neutral stations. (Rebuilt on load from Industry->st)

	Station(TileIndex tile = INVALID_TILE);
	~Station();


	void MarkTilesDirty(bool cargo_change) const;

	void UpdateVirtCoord() override;

	void MoveSign(TileIndex new_xy) override;

	void AfterStationTileSetChange(bool adding, StationType type);

	static void RecomputeCatchmentForAll();

	uint GetCatchmentRadius() const;
	void RemoveFromAllNearbyLists();

	inline bool TileIsInCatchment(TileIndex tile) const
	{
		return this->catchment_tiles.HasTile(tile);
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
	AirportTileIterator(const Station *st) : OrthogonalTileIterator(st->airport), st(st)
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

void RebuildStationKdtree();


#endif /* STATION_BASE_H */
