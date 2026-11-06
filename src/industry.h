/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry.h Base of all industries. */

#ifndef INDUSTRY_H
#define INDUSTRY_H

#include "core/flatset_type.hpp"
#include "misc/history_type.hpp"
#include "newgrf_storage.h"
#include "subsidy_type.h"
#include "industry_map.h"
#include "industrytype.h"
#include "tilearea_type.h"
#include "station_base.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#	include "rust/industry_adapter.hpp"
#include "rust/pool.hpp"
#	include "rust/station_catchment_ffi.h"


using IndustryPool = RustPool<Industry, IndustryID, 64>;
extern IndustryPool _industry_pool;

static const TimerGameEconomy::Year PROCESSING_INDUSTRY_ABANDONMENT_YEARS{5}; ///< If a processing industry doesn't produce for this many consecutive economy years, it may close.

/*
 * Production level maximum, minimum and default values.
 * It is not a value been really used in order to change, but rather an indicator
 * of how the industry is behaving.
 */
static constexpr uint8_t PRODLEVEL_CLOSURE = 0x00; ///< signal set to actually close the industry
static constexpr uint8_t PRODLEVEL_MINIMUM = 0x04; ///< below this level, the industry is set to be closing
static constexpr uint8_t PRODLEVEL_DEFAULT = 0x10; ///< default level set when the industry is created
static constexpr uint8_t PRODLEVEL_MAXIMUM = 0x80; ///< the industry is running at full speed

/**
 * Flags to control/override the behaviour of an industry.
 * These flags are controlled by game scripts.
 */
enum class IndustryControlFlag : uint8_t {
	/** When industry production change is evaluated, rolls to decrease are ignored. */
	NoProductionDecrease = 0,
	/** When industry production change is evaluated, rolls to increase are ignored. */
	NoProductionIncrease = 1,
	/**
	 * Industry can not close regardless of production level or time since last delivery.
	 * This does not prevent a closure already announced. */
	NoClosure = 2,
	/** Indicates that the production level of the industry is externally controlled. */
	ExternalProdLevel = 3,
	End,
};
using IndustryControlFlags = EnumBitSet<IndustryControlFlag, uint8_t, IndustryControlFlag::End>;

struct OpenTTDIndustryRecord;

/**
 * Defines the internal data of a functional industry.
 */
struct Industry : IndustryPool::PoolItem<&_industry_pool> {
	struct ProducedHistory {
		uint16_t production = 0; ///< Total produced
		uint16_t transported = 0; ///< Total transported

		uint8_t PctTransported() const
		{
			if (this->production == 0) return 0;
			return ClampTo<uint8_t>(this->transported * 256 / this->production);
		}
	};
	struct ProducedCargo {
		CargoType cargo = 0; ///< Cargo type
		uint16_t waiting = 0; ///< Amount of cargo produced
		uint8_t rate = 0; ///< Production rate
		HistoryData<ProducedHistory> history{}; ///< History of cargo produced and transported for this month and 24 previous months
	};

	struct AcceptedHistory {
		uint16_t accepted = 0; /// Total accepted.
		uint16_t waiting = 0; /// Average waiting.
	};

	struct AcceptedCargo {
		CargoType cargo = 0; ///< Cargo type
		uint16_t waiting = 0; ///< Amount of cargo waiting to processed
		uint32_t accumulated_waiting = 0; ///< Accumulated waiting total over the last month, used to calculate average.
		TimerGameEconomy::Date last_accepted{}; ///< Last day cargo was accepted by this industry
		RustIndustryHistory<HistoryData<AcceptedHistory>> history{};
		///< History of accepted and waiting cargo.

		/**
		 * Get history data, creating it if necessary.
		 * @return Accepted history data.
		 */
		inline HistoryData<AcceptedHistory> &GetOrCreateHistory()
		{
			openttd_rust_industry_history(this);
			return *this->history;
		}
	};

	using ProducedCargoes = RustIndustryVector<ProducedCargo, true>;
	using AcceptedCargoes = RustIndustryVector<AcceptedCargo, false>;
	RustIndustryOwner production_owner{[this] {
		auto *owner = openttd_rust_industry_new(this);
		/* The owning production constructor follows indexed pool publication. */
		_industry_pool.BindRecord(this->index.base(), this, owner);
		return owner;
	}(), RustIndustryDeleteOwned};
	/** The shared record: the canonical owner's first member, at its address. */
	inline OpenTTDIndustryRecord &Record();
	inline const OpenTTDIndustryRecord &Record() const;

	ProducedCargoes &Produced();
	const ProducedCargoes &Produced() const;
	AcceptedCargoes &Accepted();
	const AcceptedCargoes &Accepted() const;

	RustStationList<Station> StationsNear() const;
	mutable std::string cached_name{}; ///< NOSAVE: Cache of the resolved name of the industry

	bool HasText() const { return openttd_rust_industry_text(this->production_owner.get()).size != 0; }
	EncodedString GetText() const
	{
		auto view = openttd_rust_industry_text(this->production_owner.get());
		return EncodedString(std::string(reinterpret_cast<const char *>(view.data), view.size));
	}
	static OpenTTDByteSpan TextView(const EncodedString &value) { return {reinterpret_cast<const uint8_t *>(value.string.data()), value.string.size()}; }
	void SetText(const EncodedString &value) { auto text = TextView(value); openttd_rust_industry_set_text(this->production_owner.get(), text.data, text.size); }


	Industry(TileIndex tile = INVALID_TILE);
	/** Synchronous resolver shell; its Rust stack owner outlives this shell.
	 * It neither publishes nor retires that borrowed caller-owned record. */
	explicit Industry(OpenTTDIndustry *borrowed);
	~Industry();

	void RecomputeProductionMultipliers();

	/**
	 * Check if a given tile belongs to this industry.
	 * @param tile The tile to check.
	 * @return True if the tile is part of this industry.
	 */
	inline bool TileBelongsToIndustry(TileIndex tile) const
	{
		return IsTileType(tile, MP_INDUSTRY) && GetIndustryIndex(tile) == this->index;
	}

	/**
	 * Safely get a produced cargo slot, or an empty data if the slot does not exist.
	 * @param slot produced cargo slot to retrieve.
	 * @return the real slot, or an empty slot.
	 */
	inline const ProducedCargo &GetProduced(size_t slot) const
	{
		static const ProducedCargo empty{INVALID_CARGO, 0, 0, {}};
		return slot < this->Produced().size() ? this->Produced()[slot] : empty;
	}

	/**
	 * Safely get an accepted cargo slot, or an empty data if the slot does not exist.
	 * @param slot accepted cargo slot to retrieve.
	 * @return the real slot, or an empty slot.
	 */
	inline const AcceptedCargo &GetAccepted(size_t slot) const
	{
		static const AcceptedCargo empty{INVALID_CARGO, 0, 0, {}, {}};
		return slot < this->Accepted().size() ? this->Accepted()[slot] : empty;
	}

	/**
	 * Get produced cargo slot for a specific cargo type.
	 * @param cargo CargoType to find.
	 * @return Iterator pointing to produced cargo slot if it exists, or the end iterator.
	 */
	inline ProducedCargoes::iterator GetCargoProduced(CargoType cargo)
	{
		if (!IsValidCargoType(cargo)) return std::end(this->Produced());
		return std::ranges::find(this->Produced(), cargo, &ProducedCargo::cargo);
	}

	/**
	 * Get produced cargo slot for a specific cargo type (const-variant).
	 * @param cargo CargoType to find.
	 * @return Iterator pointing to produced cargo slot if it exists, or the end iterator.
	 */
	inline ProducedCargoes::const_iterator GetCargoProduced(CargoType cargo) const
	{
		if (!IsValidCargoType(cargo)) return std::end(this->Produced());
		return std::ranges::find(this->Produced(), cargo, &ProducedCargo::cargo);
	}

	/**
	 * Get accepted cargo slot for a specific cargo type.
	 * @param cargo CargoType to find.
	 * @return Iterator pointing to accepted cargo slot if it exists, or the end iterator.
	 */
	inline AcceptedCargoes::iterator GetCargoAccepted(CargoType cargo)
	{
		if (!IsValidCargoType(cargo)) return std::end(this->Accepted());
		return std::ranges::find(this->Accepted(), cargo, &AcceptedCargo::cargo);
	}

	/**
	 * Get accepted cargo slot for a specific cargo type (const-variant).
	 * @param cargo CargoType to find.
	 * @return Iterator pointing to accepted cargo slot if it exists, or the end iterator.
	 */
	inline AcceptedCargoes::const_iterator GetCargoAccepted(CargoType cargo) const
	{
		if (!IsValidCargoType(cargo)) return std::end(this->Accepted());
		return std::ranges::find(this->Accepted(), cargo, &AcceptedCargo::cargo);
	}

	/**
	 * Test if this industry accepts any cargo.
	 * @return true iff the industry accepts any cargo.
	 */
	bool IsCargoAccepted() const { return std::any_of(std::begin(this->Accepted()), std::end(this->Accepted()), [](const auto &a) { return IsValidCargoType(a.cargo); }); }

	/**
	 * Test if this industry produces any cargo.
	 * @return true iff the industry produces any cargo.
	 */
	bool IsCargoProduced() const { return std::any_of(std::begin(this->Produced()), std::end(this->Produced()), [](const auto &p) { return IsValidCargoType(p.cargo); }); }

	/**
	 * Test if this industry accepts a specific cargo.
	 * @param cargo Cargo type to test.
	 * @return true iff the industry accepts the given cargo type.
	 */
	bool IsCargoAccepted(CargoType cargo) const { return std::any_of(std::begin(this->Accepted()), std::end(this->Accepted()), [&cargo](const auto &a) { return a.cargo == cargo; }); }

	/**
	 * Test if this industry produces a specific cargo.
	 * @param cargo Cargo type to test.
	 * @return true iff the industry produces the given cargo types.
	 */
	bool IsCargoProduced(CargoType cargo) const { return std::any_of(std::begin(this->Produced()), std::end(this->Produced()), [&cargo](const auto &p) { return p.cargo == cargo; }); }

	/**
	 * Get the industry of the given tile
	 * @param tile the tile to get the industry from
	 * @pre IsTileType(t, MP_INDUSTRY)
	 * @return the industry
	 */
	static inline Industry *GetByTile(TileIndex tile)
	{
		return Industry::Get(GetIndustryIndex(tile));
	}

	static void PostDestructor(size_t index);

	/**
	 * Get the count of industries for this type.
	 * @param type IndustryType to query
	 * @pre type < NUM_INDUSTRYTYPES
	 */
	static inline uint16_t GetIndustryTypeCount(IndustryType type)
	{
		assert(type < NUM_INDUSTRYTYPES);
		return static_cast<uint16_t>(std::size(industries[type]));
	}

	inline const std::string &GetCachedName() const
	{
		if (this->cached_name.empty()) this->FillCachedName();
		return this->cached_name;
	}

	static std::array<RustIndustryRegistry, NUM_INDUSTRYTYPES> industries;

private:
	void FillCachedName() const;
};

/* Generated after Industry's nested cargo types are complete. */
#include "abi-record-industry_record.h"
inline OpenTTDIndustryRecord &Industry::Record() { return *reinterpret_cast<OpenTTDIndustryRecord *>(this->production_owner.get()); }
inline const OpenTTDIndustryRecord &Industry::Record() const { return *reinterpret_cast<const OpenTTDIndustryRecord *>(this->production_owner.get()); }
inline Industry::ProducedCargoes &Industry::Produced() { return this->Record().produced; }
inline const Industry::ProducedCargoes &Industry::Produced() const { return this->Record().produced; }
inline Industry::AcceptedCargoes &Industry::Accepted() { return this->Record().accepted; }
inline const Industry::AcceptedCargoes &Industry::Accepted() const { return this->Record().accepted; }
inline RustStationList<Station> Industry::StationsNear() const { return RustStationList<Station>(const_cast<OpenTTDStationList *>(&this->Record().stations_near)); }
inline Industry::Industry(TileIndex tile) { this->Record().location = TileArea(tile, 0, 0); }
inline Industry::Industry(OpenTTDIndustry *borrowed) : production_owner(borrowed, [](OpenTTDIndustry *) {}) { this->Record().location = TileArea(INVALID_TILE, 0, 0); }

void ClearAllIndustryCachedNames();

void PlantRandomFarmField(const Industry *i);

void ReleaseDisastersTargetingIndustry(IndustryID);

bool IsTileForestIndustry(TileIndex tile);

/** Data for managing the number of industries of a single industry type. */
#include "abi-record-industry_build_fields.h"
struct IndustryTypeBuildData : OpenTTDIndustryBuildFields {
	void Reset();

	bool GetIndustryTypeData(IndustryType it);
};

#include "abi-record-industry_builder_fields.h"

/**
 * Data for managing the number and type of industries in the game.
 */
struct IndustryBuildData {
	RustIndustryBuilderOwner owner{openttd_rust_industry_builder_new(), openttd_rust_industry_builder_destroy};
	OpenTTDIndustryBuilderFields &Fields() const { return *reinterpret_cast<OpenTTDIndustryBuilderFields *>(this->owner.get()); }

	void Reset();

	void SetupTargetCount();
	void TryBuildNewIndustry();

	void EconomyMonthlyLoop();
};

extern IndustryBuildData _industry_builder;
inline const RustVector<uint16_t> &RustIndustryRegistry::View() const { return _industry_builder.Fields().registry[this->type]; }


/** Special values for the industry list window for the data parameter of #InvalidateWindowData. */
enum IndustryDirectoryInvalidateWindowData : uint8_t {
	IDIWD_FORCE_REBUILD,
	IDIWD_PRODUCTION_CHANGE,
	IDIWD_FORCE_RESORT,
};

void TrimIndustryAcceptedProduced(Industry *ind);
const OpenTTDIndustryServices &GetIndustryRustServices();
const OpenTTDIndustryLifecycleServices &GetIndustryLifecycleRustServices();
OpenTTDIndustryRandomisationSpec IndustryRandomisationSpec(uint32_t tile) noexcept;
OpenTTDIndustryReseed IndustryResolveReseed(uint32_t tile, uint16_t gfx, void *industry, uint8_t waiting) noexcept;
OpenTTDIndustryAnimationSpec IndustryAnimationSpec(uint32_t tile) noexcept;

#endif /* INDUSTRY_H */
