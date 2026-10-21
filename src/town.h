/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file town.h Base of the town class. */

#ifndef TOWN_H
#define TOWN_H

#include "misc/history_type.hpp"
#include "viewport_type.h"
#include "timer/timer_game_tick.h"
#include "town_map.h"
#include "subsidy_type.h"
#include "newgrf_storage.h"
#include "cargotype.h"
#ifdef WITH_RUST
#include "rust/town_owner.hpp"
#endif

template <typename T>
struct BuildingCounts {
	std::vector<T> id_count{};
	std::vector<T> class_count{};

	auto operator<=>(const BuildingCounts &) const = default;
};

static const uint CUSTOM_TOWN_NUMBER_DIFFICULTY  = 4; ///< value for custom town number in difficulty settings
static const uint CUSTOM_TOWN_MAX_NUMBER = 5000;  ///< this is the maximum number of towns a user can specify in customisation

static const uint TOWN_GROWTH_WINTER = 0xFFFFFFFE; ///< The town only needs this cargo in the winter (any amount)
static const uint TOWN_GROWTH_DESERT = 0xFFFFFFFF; ///< The town needs the cargo for growth when on desert (any amount)
static const uint16_t TOWN_GROWTH_RATE_NONE = 0xFFFF; ///< Special value for Town::growth_rate to disable town growth.
static const uint16_t MAX_TOWN_GROWTH_TICKS = 930; ///< Max amount of original town ticks that still fit into uint16_t, about equal to UINT16_MAX / TOWN_GROWTH_TICKS but slightly less to simplify calculations

typedef Pool<Town, TownID, 64> TownPool;
extern TownPool _town_pool;

/** Flags controlling various town behaviours. */
enum class TownFlag : uint8_t {
	IsGrowing = 0, ///< Conditions for town growth are met. Grow according to Town::growth_rate.
	HasChurch = 1, ///< There can be only one church by town.
	HasStadium = 2, ///< There can be only one stadium by town.
	CustomGrowth = 3, ///< Growth rate is controlled by GS.
};

using TownFlags = EnumBitSet<TownFlag, uint8_t>;

#ifdef WITH_RUST
/** Trivial native object over the Rust allocation. Its fields have canonical
 * Rust storage; references below preserve native readers and save adapters. */
struct TownSimulationFields {
	ValidHistoryMask valid_history = 0;
	TileIndex xy = INVALID_TILE;
	uint32_t num_houses = 0, population = 0;
	std::array<uint32_t, NUM_HOUSE_ZONES> radius{};
	std::array<uint32_t, NUM_TAE> goal{};
	TypedIndexContainer<std::array<int16_t, MAX_COMPANIES>, CompanyID> ratings{};
	std::array<TransportedCargoStat<uint16_t>, NUM_TAE> received{};
	uint16_t noise_reached = 0, time_until_rebuild = 0;
	CompanyMask statues{}, have_ratings{};
	TypedIndexContainer<std::array<uint8_t, MAX_COMPANIES>, CompanyID> unwanted{};
	CompanyID exclusivity = CompanyID::Invalid();
	uint8_t exclusive_counter = 0;
	bool larger_town = false;
	TownLayout layout{};
	uint32_t townnamegrfid = 0;
	uint16_t townnametype = 0;
	uint32_t townnameparts = 0;
};
static_assert(sizeof(TownSimulationFields) == sizeof(OpenTTDTownSimulationFields));
static_assert(alignof(TownSimulationFields) == alignof(OpenTTDTownSimulationFields));
#define CHECK_TOWN_FIELD(field) static_assert(offsetof(TownSimulationFields, field) == offsetof(OpenTTDTownSimulationFields, field))
CHECK_TOWN_FIELD(valid_history); CHECK_TOWN_FIELD(xy); CHECK_TOWN_FIELD(num_houses); CHECK_TOWN_FIELD(population);
CHECK_TOWN_FIELD(radius); CHECK_TOWN_FIELD(goal); CHECK_TOWN_FIELD(ratings); CHECK_TOWN_FIELD(received);
CHECK_TOWN_FIELD(noise_reached); CHECK_TOWN_FIELD(time_until_rebuild); CHECK_TOWN_FIELD(statues); CHECK_TOWN_FIELD(have_ratings);
CHECK_TOWN_FIELD(unwanted); CHECK_TOWN_FIELD(exclusivity); CHECK_TOWN_FIELD(exclusive_counter); CHECK_TOWN_FIELD(larger_town); CHECK_TOWN_FIELD(layout);
CHECK_TOWN_FIELD(townnamegrfid); CHECK_TOWN_FIELD(townnametype); CHECK_TOWN_FIELD(townnameparts);
#undef CHECK_TOWN_FIELD
#endif

#ifdef WITH_RUST
struct TownGrowthFields {
	uint16_t counter = 0, rate = 0;
	uint8_t funding = 0, road = 0;
	TownFlags flags{};
};
static_assert(sizeof(TownGrowthFields) == sizeof(OpenTTDTownGrowthFields));
static_assert(alignof(TownGrowthFields) == alignof(OpenTTDTownGrowthFields));
#define CHECK_TOWN_GROWTH_FIELD(field) static_assert(offsetof(TownGrowthFields, field) == offsetof(OpenTTDTownGrowthFields, field))
CHECK_TOWN_GROWTH_FIELD(counter); CHECK_TOWN_GROWTH_FIELD(rate); CHECK_TOWN_GROWTH_FIELD(funding); CHECK_TOWN_GROWTH_FIELD(road); CHECK_TOWN_GROWTH_FIELD(flags);
#undef CHECK_TOWN_GROWTH_FIELD
#endif

/** Data structure with cached data of towns. */
struct TownCache {
	uint32_t num_houses = 0; ///< Amount of houses
	uint32_t population = 0; ///< Current population of people
	TrackedViewportSign sign{}; ///< Location of name sign, UpdateVirtCoord updates this
	PartsOfSubsidy part_of_subsidy{}; ///< Is this town a source/destination of a subsidy?
	std::array<uint32_t, NUM_HOUSE_ZONES> squared_town_zone_radius{}; ///< UpdateTownRadius updates this given the house count
	BuildingCounts<uint16_t> building_counts{}; ///< The number of each type of building in the town

	auto operator<=>(const TownCache &) const = default;
};

#ifdef WITH_RUST
/** Cache facade preserves canonical scalar addresses; only the presentation and
 * nearby-station caches remain native. Debug cache snapshots copy their values. */
struct RustTownCache {
	uint32_t &num_houses, &population;
	TrackedViewportSign sign{};
	PartsOfSubsidy part_of_subsidy{};
	std::array<uint32_t, NUM_HOUSE_ZONES> &squared_town_zone_radius;
	RustTownBuildingCounts<uint16_t> building_counts;
	explicit RustTownCache(TownSimulationFields &fields, OpenTTDTownState *owner) : num_houses(fields.num_houses), population(fields.population), squared_town_zone_radius(fields.radius), building_counts(owner) {}
	TownCache Snapshot() const
	{
		TownCache snapshot{num_houses, population, sign, part_of_subsidy, squared_town_zone_radius};
		snapshot.building_counts.id_count.assign(building_counts.id_count.begin(), building_counts.id_count.end());
		snapshot.building_counts.class_count.assign(building_counts.class_count.begin(), building_counts.class_count.end());
		return snapshot;
	}
	bool operator==(const TownCache &other) const { return this->Snapshot() == other; }
	friend bool operator==(const TownCache &other, const RustTownCache &cache) { return cache == other; }
};
#endif

/** Town data structure. */
struct Town : TownPool::PoolItem<&_town_pool> {
#ifdef WITH_RUST
	RustTownOwner growth_owner{openttd_rust_town_new()};
	TownGrowthFields &growth = *new (openttd_rust_town_growth(growth_owner.get())) TownGrowthFields();
	TownSimulationFields &simulation = *new (openttd_rust_town_simulation(growth_owner.get())) TownSimulationFields();
	TileIndex &xy = simulation.xy;
	RustTownCache cache{simulation, growth_owner.get()};
#else
	TileIndex xy = INVALID_TILE; ///< town center tile

	TownCache cache{}; ///< Container for all cacheable data.
#endif

	/* Town name */
#ifdef WITH_RUST
	uint32_t &townnamegrfid = simulation.townnamegrfid;
	uint16_t &townnametype = simulation.townnametype;
	uint32_t &townnameparts = simulation.townnameparts;
#else
	uint32_t townnamegrfid = 0;
	uint16_t townnametype = 0;
	uint32_t townnameparts = 0;
#endif
#ifdef WITH_RUST
	mutable std::string custom_name_cache{}; ///< Presentation only: preserves custom-name reference identity.
	RustTownName name{growth_owner.get(), custom_name_cache};
#else
	std::string name{}; ///< Custom town name. If empty, the town was not renamed and uses the generated name.
#endif
	mutable std::string cached_name{}; ///< NOSAVE: Cache of the resolved name of the town, if not using a custom town name

#ifdef WITH_RUST
	TownFlags &flags = growth.flags;
#else
	TownFlags flags{}; ///< See #TownFlags.
#endif

#ifdef WITH_RUST
	uint16_t &noise_reached = simulation.noise_reached;
#else
	uint16_t noise_reached = 0; ///< level of noise that all the airports are generating
#endif

#ifdef WITH_RUST
	CompanyMask &statues = simulation.statues;
#else
	CompanyMask statues{}; ///< which companies have a statue?
#endif

	/* Company ratings. */
#ifdef WITH_RUST
	CompanyMask &have_ratings = simulation.have_ratings;
#else
	CompanyMask have_ratings{}; ///< which companies have a rating
#endif
#ifdef WITH_RUST
	TypedIndexContainer<std::array<uint8_t, MAX_COMPANIES>, CompanyID> &unwanted = simulation.unwanted;
#else
	TypedIndexContainer<std::array<uint8_t, MAX_COMPANIES>, CompanyID> unwanted{}; ///< how many months companies aren't wanted by towns (bribe)
#endif
#ifdef WITH_RUST
	CompanyID &exclusivity = simulation.exclusivity;
#else
	CompanyID exclusivity = CompanyID::Invalid(); ///< which company has exclusivity
#endif
#ifdef WITH_RUST
	uint8_t &exclusive_counter = simulation.exclusive_counter;
#else
	uint8_t exclusive_counter = 0; ///< months till the exclusivity expires
#endif
#ifdef WITH_RUST
	TypedIndexContainer<std::array<int16_t, MAX_COMPANIES>, CompanyID> &ratings = simulation.ratings;
#else
	TypedIndexContainer<std::array<int16_t, MAX_COMPANIES>, CompanyID> ratings{};  ///< ratings of each company for this town
#endif

	struct SuppliedHistory {
		uint32_t production = 0; ///< Total produced
		uint32_t transported = 0; ///< Total transported

		uint8_t PctTransported() const
		{
#ifdef WITH_RUST
			return openttd_rust_town_percent_transported(reinterpret_cast<const OpenTTDTownSuppliedHistory *>(this));
#else
			if (this->production == 0) return 0;
			return ClampTo<uint8_t>(this->transported * 256 / this->production);
#endif
		}
	};

	struct SuppliedCargo {
		CargoType cargo = INVALID_CARGO;
		HistoryData<SuppliedHistory> history{};

		SuppliedCargo() = default;
		SuppliedCargo(CargoType cargo) : cargo(cargo) {}
	};

#ifdef WITH_RUST
	static_assert(sizeof(SuppliedHistory) == sizeof(OpenTTDTownSuppliedHistory));
	static_assert(alignof(SuppliedHistory) == alignof(OpenTTDTownSuppliedHistory));
	static_assert(offsetof(SuppliedHistory, production) == offsetof(OpenTTDTownSuppliedHistory, production));
	static_assert(offsetof(SuppliedHistory, transported) == offsetof(OpenTTDTownSuppliedHistory, transported));
	static_assert(HISTORY_RECORDS == 61 && sizeof(SuppliedCargo) == sizeof(OpenTTDTownSupplied));
	static_assert(alignof(SuppliedCargo) == alignof(OpenTTDTownSupplied));
	static_assert(offsetof(SuppliedCargo, cargo) == offsetof(OpenTTDTownSupplied, cargo));
	static_assert(offsetof(SuppliedCargo, history) == offsetof(OpenTTDTownSupplied, history));
	using SuppliedCargoes = RustTownSupplied<SuppliedCargo>;
#else
	using SuppliedCargoes = std::vector<SuppliedCargo>;
#endif

#ifdef WITH_RUST
	SuppliedCargoes supplied{growth_owner.get()};
#else
	SuppliedCargoes supplied{}; ///< Cargo statistics about supplied cargo.
#endif
#ifdef WITH_RUST
	std::array<TransportedCargoStat<uint16_t>, NUM_TAE> &received = simulation.received;
#else
	std::array<TransportedCargoStat<uint16_t>, NUM_TAE> received{}; ///< Cargo statistics about received cargotypes.
#endif
#ifdef WITH_RUST
	std::array<uint32_t, NUM_TAE> &goal = simulation.goal;
#else
	std::array<uint32_t, NUM_TAE> goal{}; ///< Amount of cargo required for the town to grow.
#endif
#ifdef WITH_RUST
	ValidHistoryMask &valid_history = simulation.valid_history;
#else
	ValidHistoryMask valid_history = 0; ///< Mask of valid history records.
#endif

	EncodedString text{}; ///< General text with additional information.

	inline SuppliedCargo &GetOrCreateCargoSupplied(CargoType cargo)
	{
#ifdef WITH_RUST
		assert(IsValidCargoType(cargo));
		auto result = openttd_rust_town_supplied_entry(this->growth_owner.get(), cargo);
		auto *entry = reinterpret_cast<SuppliedCargo *>(result.entry);
		if (result.created) new (entry) SuppliedCargo(cargo);
		return *entry;
#else
		assert(IsValidCargoType(cargo));
		auto it = std::ranges::lower_bound(this->supplied, cargo, std::less{}, &SuppliedCargo::cargo);
		if (it == std::end(this->supplied) || it->cargo != cargo) it = this->supplied.emplace(it, cargo);
		return *it;
#endif
	}

	inline SuppliedCargoes::const_iterator GetCargoSupplied(CargoType cargo) const
	{
#ifdef WITH_RUST
		if (!IsValidCargoType(cargo)) return std::end(this->supplied);
		return this->supplied.data() + openttd_rust_town_supplied_find(this->growth_owner.get(), cargo);
#else
		if (!IsValidCargoType(cargo)) return std::end(this->supplied);
		auto it = std::ranges::lower_bound(this->supplied, cargo, std::less{}, &SuppliedCargo::cargo);
		if (it == std::end(this->supplied) || it->cargo != cargo) return std::end(supplied);
		return it;
#endif
	}

	inline uint8_t GetPercentTransported(CargoType cargo_type) const
	{
		auto it = this->GetCargoSupplied(cargo_type);
		if (it == std::end(this->supplied)) return 0;

		return it->history[LAST_MONTH].PctTransported();
	}

	StationList stations_near{}; ///< NOSAVE: List of nearby stations.

#ifdef WITH_RUST
	uint16_t &time_until_rebuild = simulation.time_until_rebuild;
#else
	uint16_t time_until_rebuild = 0; ///< time until we rebuild a house
#endif

#ifdef WITH_RUST
	uint16_t &grow_counter = growth.counter;
	uint16_t &growth_rate = growth.rate;
	uint8_t &fund_buildings_months = growth.funding;
	uint8_t &road_build_months = growth.road;
#else
	uint16_t grow_counter = 0; ///< counter to count when to grow, value is smaller than or equal to growth_rate
	uint16_t growth_rate = 0; ///< town growth rate

	uint8_t fund_buildings_months = 0; ///< fund buildings program in action?
	uint8_t road_build_months = 0; ///< fund road reconstruction in action?

#endif

#ifdef WITH_RUST
	bool &larger_town = simulation.larger_town;
#else
	bool larger_town = false; ///< if this is a larger town and should grow more quickly
#endif
#ifdef WITH_RUST
	TownLayout &layout = simulation.layout;
#else
	TownLayout layout{}; ///< town specific road layout
#endif

	bool show_zone = false; ///< NOSAVE: mark town to show the local authority zone in the viewports

	std::vector<PersistentStorage *> psa_list{};

	/**
	 * Creates a new town.
	 * @param tile center tile of the town
	 */
	Town(TileIndex tile = INVALID_TILE)
#ifdef WITH_RUST
	{ this->xy = tile; }
#else
	: xy(tile) { }
#endif

	TownCache CopyTownCache() const
	{
#ifdef WITH_RUST
		return this->cache.Snapshot();
#else
		return this->cache;
#endif
	}

	/** Destroy the town. */
	~Town();

#ifndef WITH_RUST
	void InitializeLayout(TownLayout layout);
#endif

	/**
	 * Calculate the max town noise.
	 * The value is counted using the population divided by the content of the
	 * entry in town_noise_population corresponding to the town's tolerance.
	 * @return the maximum noise level the town will tolerate.
	 */
	inline uint16_t MaxTownNoise() const
	{
		if (this->cache.population == 0) return 0; // no population? no noise

		/* 3 is added (the noise of the lowest airport), so the  user can at least build a small airfield. */
		return ClampTo<uint16_t>((this->cache.population / _settings_game.economy.town_noise_population[_settings_game.difficulty.town_council_tolerance]) + 3);
	}

	void UpdateVirtCoord();

	inline const std::string &GetCachedName() const
	{
#ifdef WITH_RUST
		const auto custom_name = this->name.View();
		if (!custom_name.empty()) {
			return this->custom_name_cache;
		}
#else
		if (!this->name.empty()) return this->name;
#endif
		if (this->cached_name.empty()) this->FillCachedName();
		return this->cached_name;
	}

	static inline Town *GetByTile(TileIndex tile)
	{
		return Town::Get(GetTownIndex(tile));
	}

	static Town *GetRandom();
	static void PostDestructor(size_t index);

private:
	void FillCachedName() const;
};

uint32_t GetWorldPopulation();

void UpdateAllTownVirtCoords();
void ClearAllTownCachedNames();
void ShowTownViewWindow(TownID town);
void ExpandTown(Town *t);

void RebuildTownKdtree();

/** Settings for town council attitudes. */
enum TownCouncilAttitudes {
	TOWN_COUNCIL_LENIENT    = 0,
	TOWN_COUNCIL_TOLERANT   = 1,
	TOWN_COUNCIL_HOSTILE    = 2,
	TOWN_COUNCIL_PERMISSIVE = 3,
};

/**
 * Action types that a company must ask permission for to a town authority.
 * @see CheckforTownRating
 */
enum class TownRatingCheckType : uint8_t {
	RoadRemove, ///< Removal of a road owned by the town.
	TunnelBridgeRemove, ///< Removal of a tunnel or bridge owned by the town.
	End,
};

/** Special values for town list window for the data parameter of #InvalidateWindowData. */
enum TownDirectoryInvalidateWindowData {
	TDIWD_FORCE_REBUILD,
	TDIWD_POPULATION_CHANGE,
	TDIWD_FORCE_RESORT,
};

CommandCost CheckforTownRating(DoCommandFlags flags, Town *t, TownRatingCheckType type);


TileIndexDiff GetHouseNorthPart(HouseID &house);

Town *CalcClosestTownFromTile(TileIndex tile, uint threshold = UINT_MAX);

void ResetHouses();

/** Town actions of a company. */
enum class TownAction : uint8_t {
	AdvertiseSmall, ///< Small advertising campaign.
	AdvertiseMedium, ///< Medium advertising campaign.
	AdvertiseLarge, ///< Large advertising campaign.
	RoadRebuild, ///< Rebuild the roads.
	BuildStatue, ///< Build a statue.
	FundBuildings, ///< Fund new buildings.
	BuyRights, ///< Buy exclusive transport rights.
	Bribe, ///< Try to bribe the council.
	End,
};
using TownActions = EnumBitSet<TownAction, uint8_t>;

DECLARE_INCREMENT_DECREMENT_OPERATORS(TownAction);

#ifndef WITH_RUST
void ClearTownHouse(Town *t, TileIndex tile);
#endif
void UpdateTownMaxPass(Town *t);
void UpdateTownRadius(Town *t);
CommandCost CheckIfAuthorityAllowsNewStation(TileIndex tile, DoCommandFlags flags);
Town *ClosestTownFromTile(TileIndex tile, uint threshold);
void ChangeTownRating(Town *t, int add, int max, DoCommandFlags flags);
HouseZone GetTownRadiusGroup(const Town *t, TileIndex tile);
void SetTownRatingTestMode(bool mode);
TownActions GetMaskOfTownActions(CompanyID cid, const Town *t);
uint GetDefaultTownsForMapSize();
bool GenerateTowns(TownLayout layout, std::optional<uint> number = std::nullopt);
const CargoSpec *FindFirstCargoWithTownAcceptanceEffect(TownAcceptanceEffect effect);
CargoArray GetAcceptedCargoOfHouse(const HouseSpec *hs);

uint8_t GetTownActionCost(TownAction action);

/**
 * Set the default name for a depot/waypoint
 * @tparam T The type/class to make a default name for
 * @param obj The object/instance we want to find the name for
 */
template <class T>
void MakeDefaultName(T *obj)
{
	/* We only want to set names if it hasn't been set before, or when we're calling from afterload. */
	assert(obj->name.empty() || obj->town_cn == UINT16_MAX);

	obj->town = ClosestTownFromTile(obj->xy, UINT_MAX);

	/* Find first unused number belonging to this town. This can never fail,
	 * as long as there can be at most 65535 waypoints/depots in total.
	 *
	 * This does 'n * m' search, but with 32bit 'used' bitmap, it needs at
	 * most 'n * (1 + ceil(m / 32))' steps (n - number of waypoints in pool,
	 * m - number of waypoints near this town).
	 * Usually, it needs only 'n' steps.
	 *
	 * If it wasn't using 'used' and 'idx', it would just search for increasing 'next',
	 * but this way it is faster */

	uint32_t used = 0; // bitmap of used waypoint numbers, sliding window with 'next' as base
	uint32_t next = 0; // first number in the bitmap
	uint32_t idx  = 0; // index where we will stop
	uint32_t cid  = 0; // current index, goes to T::GetPoolSize()-1, then wraps to 0

	do {
		T *lobj = T::GetIfValid(cid);

		/* check only valid waypoints... */
		if (lobj != nullptr && obj != lobj) {
			/* only objects within the same city and with the same type */
			if (lobj->town == obj->town && lobj->IsOfType(obj)) {
				/* if lobj->town_cn < next, uint will overflow to '+inf' */
				uint i = (uint)lobj->town_cn - next;

				if (i < 32) {
					SetBit(used, i); // update bitmap
					if (i == 0) {
						/* shift bitmap while the lowest bit is '1';
						 * increase the base of the bitmap too */
						do {
							used >>= 1;
							next++;
						} while (HasBit(used, 0));
						/* when we are at 'idx' again at end of the loop and
						 * 'next' hasn't changed, then no object had town_cn == next,
						 * so we can safely use it */
						idx = cid;
					}
				}
			}
		}

		cid++;
		if (cid == T::GetPoolSize()) cid = 0; // wrap to zero...
	} while (cid != idx);

	obj->town_cn = (uint16_t)next; // set index...
}

/*
 * Converts original town ticks counters to plain game ticks. Note that
 * tick 0 is a valid tick so actual amount is one more than the counter value.
 */
inline uint16_t TownTicksToGameTicks(uint16_t ticks)
{
	return (std::min(ticks, MAX_TOWN_GROWTH_TICKS) + 1) * Ticks::TOWN_GROWTH_TICKS - 1;
}


RoadType GetTownRoadType();
bool CheckTownRoadTypes();
std::span<const DrawBuildingsTileStruct> GetTownDrawTileData();

#endif /* TOWN_H */
