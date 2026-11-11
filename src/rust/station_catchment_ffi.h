/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_catchment_ffi.h Rust-owned station catchment, rectangles and nearby sets. */
#ifndef RUST_STATION_CATCHMENT_FFI_H
#define RUST_STATION_CATCHMENT_FFI_H
#include "service_defaults.hpp"
#include "linkgraph_ffi.h"
#include <cstdint>
#include <cstddef>
struct BaseStation;
struct Industry;
struct OpenTTDSharedServices;
struct OpenTTDAirportResolverServices;
struct OpenTTDIndustryTileServices;
struct OpenTTDObjectServices;
struct OpenTTDTownLeaves;
struct OpenTTDStationServices;
struct OpenTTDVehicleVisits;
struct OpenTTDStationService;
struct OpenTTDIndustry;
struct OpenTTDTownState;

/** Layout of the canonical rectangle aliased by the native StationRect facade. */
struct OpenTTDStationRect { int32_t left, top, right, bottom; };
/** One IndustryList member, ordered by distance and industry ID. */
struct OpenTTDIndustryListEntry { uint32_t distance; Industry *industry; uint16_t id; };
/** Borrowed ordered members, valid until the list next changes. */
struct OpenTTDIndustryListView { const OpenTTDIndustryListEntry *data; size_t size; };
/** Facility fields GetCatchmentRadius observes. */
struct OpenTTDStationRadiusFacilities { bool bus, truck; uint32_t train, ship, airport; };

/* Serialized by the game-state mutex. Every leaf is a direct synchronous noexcept
 * call; none unwinds during ordinary play. Rust ends container borrows before each. */
struct OpenTTDStationCatchmentServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDAirportResolverServices *airport;
	const OpenTTDIndustryTileServices *industry_tile;
	const OpenTTDObjectServices *object;
	const OpenTTDTownLeaves *town;
	const OpenTTDStationServices *station;
	const OpenTTDVehicleVisits *visits;
	size_t (*station_count)() noexcept OPENTTD_SERVICE_DEFAULT(station_count);
	BaseStation *(*station_if_valid)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station_if_valid);
	BaseStation *(*next_station)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_station);
	OpenTTDStationService *(*owner)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(owner);
	OpenTTDStationService *(*station_owner)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(station_owner);
	uint16_t (*id)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(id);
	uint32_t (*location)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(location);
	Industry *(*associated_industry)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(associated_industry);
	OpenTTDStationRadiusFacilities (*radius_facilities)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(radius_facilities);
	bool (*modified_catchment)() noexcept OPENTTD_SERVICE_DEFAULT(modified_catchment);
	uint8_t (*spread)() noexcept OPENTTD_SERVICE_DEFAULT(spread);
	Industry *(*industry_by_id)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(industry_by_id);
	uint16_t (*industry_id)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(industry_id);
	OpenTTDIndustry *(*industry_owner)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(industry_owner);
	BaseStation *(*industry_neutral)(Industry *) noexcept OPENTTD_SERVICE_DEFAULT(industry_neutral);
	Industry *(*next_industry)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(next_industry);
};
const OpenTTDStationCatchmentServices &GetRustStationCatchmentServices() noexcept;

extern "C" {
OpenTTDIndustryListView openttd_rust_station_industries_near(OpenTTDStationService *);
OpenTTDStationRect *openttd_rust_station_rect(OpenTTDStationService *);
struct OpenTTDBitmapTileArea *openttd_rust_station_catchment_tiles(OpenTTDStationService *);
OpenTTDStationList *openttd_rust_town_stations_near(OpenTTDTownState *);
OpenTTDStationList *openttd_rust_industry_stations_near(OpenTTDIndustry *);
uint8_t openttd_rust_station_rect_empty_abi(const OpenTTDStationRect *);
extern "C++" inline bool openttd_rust_station_rect_empty(const OpenTTDStationRect *a0) { return openttd_rust_station_rect_empty_abi(a0) != 0; }
uint8_t openttd_rust_station_rect_pt_in_extended_rect_abi(const OpenTTDStationRect *, int32_t, int32_t, int32_t);
extern "C++" inline bool openttd_rust_station_rect_pt_in_extended_rect(const OpenTTDStationRect *a0, int32_t a1, int32_t a2, int32_t a3) { return openttd_rust_station_rect_pt_in_extended_rect_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_station_rect_before_add_tile_abi(OpenTTDStationRect *, uint32_t, uint8_t, const OpenTTDStationCatchmentServices *);
extern "C++" inline bool openttd_rust_station_rect_before_add_tile(OpenTTDStationRect *a0, uint32_t a1, uint8_t a2, const OpenTTDStationCatchmentServices *a3) { return openttd_rust_station_rect_before_add_tile_abi(a0, a1, a2, a3) != 0; }
uint8_t openttd_rust_station_rect_before_add_rect_abi(OpenTTDStationRect *, uint32_t, int32_t, int32_t, uint8_t, const OpenTTDStationCatchmentServices *);
extern "C++" inline bool openttd_rust_station_rect_before_add_rect(OpenTTDStationRect *a0, uint32_t a1, int32_t a2, int32_t a3, uint8_t a4, const OpenTTDStationCatchmentServices *a5) { return openttd_rust_station_rect_before_add_rect_abi(a0, a1, a2, a3, a4, a5) != 0; }
void openttd_rust_station_recompute_catchment_for_all(const OpenTTDStationCatchmentServices *);
uint32_t openttd_rust_station_catchment_radius(BaseStation *, const OpenTTDStationCatchmentServices *);
void openttd_rust_station_production_around(uint32_t, int32_t, int32_t, int32_t, uint32_t *, const OpenTTDStationCatchmentServices *);
void openttd_rust_station_acceptance_around(uint32_t, int32_t, int32_t, int32_t, uint32_t *, uint64_t *, const OpenTTDStationCatchmentServices *);
}

/**
 * Borrowed facade over a Rust-owned StationList, preserving the native ordered
 * set's reader interface. It never outlives the owner and is not copied.
 */
template <class TStation>
class RustStationList {
	const OpenTTDStationList *list;
	const RustVector<OpenTTDStationListEntry> &View() const { return this->list->entries; }
public:
	class const_iterator {
		const OpenTTDStationListEntry *entry;
	public:
		using iterator_category = std::forward_iterator_tag;
		using value_type = TStation *;
		using difference_type = std::ptrdiff_t;
		using pointer = TStation *const *;
		using reference = TStation *;
		explicit const_iterator(const OpenTTDStationListEntry *entry = nullptr) : entry(entry) {}
		TStation *operator*() const { return TStation::From(this->entry->station); }
		const_iterator &operator++() { ++this->entry; return *this; }
		const_iterator operator++(int) { const_iterator old = *this; ++this->entry; return old; }
		bool operator==(const const_iterator &other) const = default;
		uint16_t Id() const { return this->entry->id; }
	};
	explicit RustStationList(const OpenTTDStationList *list) : list(list) {}
	RustStationList(const RustStationList &) = delete;
	RustStationList &operator=(const RustStationList &) = delete;
	const_iterator begin() const { return const_iterator(this->View().begin()); }
	const_iterator end() const { return const_iterator(this->View().end()); }
	size_t size() const { return this->View().size(); }
	bool empty() const { return this->size() == 0; }
	/** Ordered-set lookup by the StationCompare key. */
	const_iterator find(const TStation *st) const
	{
		const auto &view = this->View();
		const OpenTTDStationListEntry *last = view.end();
		const OpenTTDStationListEntry *at = std::lower_bound(view.begin(), last, st->index.base(), [](const OpenTTDStationListEntry &e, uint16_t id) { return e.id < id; });
		return (at != last && at->id == st->index.base()) ? const_iterator(at) : const_iterator(last);
	}
};

/** Borrowed facade over a station's Rust-owned delivery list. */
class RustIndustryList {
	OpenTTDStationService *owner;
public:
	explicit RustIndustryList(OpenTTDStationService *owner) : owner(owner) {}
	RustIndustryList(const RustIndustryList &) = delete;
	RustIndustryList &operator=(const RustIndustryList &) = delete;
	const OpenTTDIndustryListEntry *begin() const { return openttd_rust_station_industries_near(this->owner).data; }
	const OpenTTDIndustryListEntry *end() const { auto view = openttd_rust_station_industries_near(this->owner); return view.data + view.size; }
};
#endif /* RUST_STATION_CATCHMENT_FFI_H */
