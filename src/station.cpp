/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station.cpp Implementation of the station base class. */

#include "stdafx.h"
#include "core/flatset_type.hpp"
#include "company_func.h"
#include "company_base.h"
#include "roadveh.h"
#include "viewport_func.h"
#include "viewport_kdtree.h"
#include "command_func.h"
#include "news_func.h"
#include "aircraft.h"
#include "vehiclelist.h"
#include "core/pool_func.hpp"
#include "station_base.h"
#include "station_kdtree.h"
#include "roadstop_base.h"
#include "industry.h"
#include "town.h"
#include "core/random_func.hpp"
#include "linkgraph/linkgraph.h"
#include "linkgraph/linkgraphschedule.h"

#include "table/strings.h"
#include "rust/services_station.h"
#include "rust/station_lifecycle_ffi.h"
#include "rust/roadstop_resolver_ffi.h"
#include "rust/station_command_ffi.h"
#include "rust/services_assert.h"
#include "rust/services_cargo.h"
#include "rust/orders_ffi.h"
#include "rust/services_industry.h"
#include "rust/industry_ffi.h"
#include "rust/industry_shell_services.h"
#include "rust/settings_services.h"
#include "rust/airport_resolver_ffi.h"
#include "rust/object_resolver_ffi.h"
#include "rust/town_ffi.h"
#include "rust/vehicle_visits_ffi.h"

#include "safeguards.h"

/** The pool of stations. */
StationPool _station_pool("Station");
INSTANTIATE_POOL_METHODS(Station)


StationKdtree _station_kdtree{};

void RebuildStationKdtree()
{
	std::vector<StationID> stids;
	for (const Station *st : Station::Iterate()) {
		stids.push_back(st->index);
	}
	_station_kdtree.Build(stids.begin(), stids.end());
}


BaseStation::~BaseStation()
{
	if (CleaningPool()) return;

	CloseWindowById(WC_TRAINS_LIST,   VehicleListIdentifier(VL_STATION_LIST, VEH_TRAIN,    this->owner, this->index).ToWindowNumber());
	CloseWindowById(WC_ROADVEH_LIST,  VehicleListIdentifier(VL_STATION_LIST, VEH_ROAD,     this->owner, this->index).ToWindowNumber());
	CloseWindowById(WC_SHIPS_LIST,    VehicleListIdentifier(VL_STATION_LIST, VEH_SHIP,     this->owner, this->index).ToWindowNumber());
	CloseWindowById(WC_AIRCRAFT_LIST, VehicleListIdentifier(VL_STATION_LIST, VEH_AIRCRAFT, this->owner, this->index).ToWindowNumber());

	this->sign.MarkDirty();
}

Station::Station(TileIndex tile) :
	SpecializedStation<Station, false>(tile),
	bus_station(INVALID_TILE, 0, 0),
	truck_station(INVALID_TILE, 0, 0),
	ship_station(INVALID_TILE, 0, 0),
	indtype(IT_INVALID)
{
	/* this->random_bits is set in Station::AddFacility() */
}

/**
 * Clean up a station by clearing vehicle orders, invalidating windows and
 * removing link stats.
 * Aircraft-Hangar orders need special treatment here, as the hangars are
 * actually part of a station (tiletype is STATION), but the order type
 * is OT_GOTO_DEPOT.
 */
Station::~Station()
{
	openttd_rust_station_destruct(&GetRustStationLifecycleServices(), this);
}


/**
 * Invalidating of the JoinStation window has to be done
 * after removing item from the pool.
 */
void BaseStation::PostDestructor(size_t)
{
	InvalidateWindowData(WC_SELECT_STATION, 0, 0);
}

bool BaseStation::SetRoadStopTileData(TileIndex tile, uint8_t data, bool animation)
{
	for (RoadStopTileData &tile_data : this->custom_roadstop_tile_data) {
		if (tile_data.tile == tile) {
			uint8_t &v = animation ? tile_data.animation_frame : tile_data.random_bits;
			if (v == data) return false;
			v = data;
			return true;
		}
	}
	RoadStopTileData tile_data;
	tile_data.tile = tile;
	tile_data.animation_frame = animation ? data : 0;
	tile_data.random_bits = animation ? 0 : data;
	this->custom_roadstop_tile_data.push_back(tile_data);
	return data != 0;
}

void BaseStation::RemoveRoadStopTileData(TileIndex tile)
{
	for (RoadStopTileData &tile_data : this->custom_roadstop_tile_data) {
		if (tile_data.tile == tile) {
			tile_data = this->custom_roadstop_tile_data.back();
			this->custom_roadstop_tile_data.pop_back();
			return;
		}
	}
}

/**
 * Get the primary road stop (the first road stop) that the given vehicle can load/unload.
 * @param v the vehicle to get the first road stop for
 * @return the first roadstop that this vehicle can load at
 */
RoadStop *Station::GetPrimaryRoadStop(const RoadVehicle *v) const
{
	RoadStop *rs = this->GetPrimaryRoadStop(v->IsBus() ? RoadStopType::Bus : RoadStopType::Truck);

	for (; rs != nullptr; rs = rs->next) {
		/* The vehicle cannot go to this roadstop (different roadtype) */
		if (!HasTileAnyRoadType(rs->xy, v->compatible_roadtypes)) continue;
		/* The vehicle is articulated and can therefore not go to a standard road stop. */
		if (IsBayRoadStopTile(rs->xy) && v->HasArticulatedPart()) continue;

		/* The vehicle can actually go to this road stop. So, return it! */
		break;
	}

	return rs;
}

/**
 * Called when new facility is built on the station. If it is the first facility
 * it initializes also 'xy' and 'random_bits' members
 */

/**
 * Marks the tiles of the station as dirty.
 *
 * @ingroup dirty
 */
void Station::MarkTilesDirty(bool cargo_change) const
{
	if (this->train_station.tile == INVALID_TILE) return;

	/* cargo_change is set if we're refreshing the tiles due to cargo moving
	 * around. */
	if (cargo_change) {
		/* Don't waste time updating if there are no custom station graphics
		 * that might change. Even if there are custom graphics, they might
		 * not change. Unfortunately we have no way of telling. */
		if (this->speclist.empty()) return;
	}

	for (TileIndex tile : this->train_station) {
		if (this->TileBelongsToRailStation(tile)) {
			MarkTileDirtyByTile(tile);
		}
	}
}


uint Station::GetCatchmentRadius() const
{
	return openttd_rust_station_catchment_radius(const_cast<Station *>(this), &GetRustStationCatchmentServices());
}

void Station::RemoveFromAllNearbyLists()
{
	openttd_rust_station_remove_from_nearby_lists(this, &GetRustStationCatchmentServices());
}

/* static */ void Station::RecomputeCatchmentForAll()
{
	openttd_rust_station_recompute_catchment_for_all(&GetRustStationCatchmentServices());
}

bool StationRect::PtInExtendedRect(int x, int y, int distance) const
{
	return openttd_rust_station_rect_pt_in_extended_rect(reinterpret_cast<const OpenTTDStationRect *>(this), x, y, distance);
}

bool StationRect::IsEmpty() const
{
	return openttd_rust_station_rect_empty(reinterpret_cast<const OpenTTDStationRect *>(this));
}

CommandCost StationRect::BeforeAddTile(TileIndex tile, StationRectMode mode)
{
	if (openttd_rust_station_rect_before_add_tile(reinterpret_cast<OpenTTDStationRect *>(this), tile.base(), mode, &GetRustStationCatchmentServices())) return CommandCost();
	return CommandCost(STR_ERROR_STATION_TOO_SPREAD_OUT);
}

CommandCost StationRect::BeforeAddRect(TileIndex tile, int w, int h, StationRectMode mode)
{
	if (openttd_rust_station_rect_before_add_rect(reinterpret_cast<OpenTTDStationRect *>(this), tile.base(), w, h, mode, &GetRustStationCatchmentServices())) return CommandCost();
	return CommandCost(STR_ERROR_STATION_TOO_SPREAD_OUT);
}

const OpenTTDStationCatchmentServices &GetRustStationCatchmentServices() noexcept
{
	static const OpenTTDStationCatchmentServices services = {
		.shared = &GetRustSharedServices(),
		.airport = &GetAirportResolverServices(),
		.industry_tile = &GetIndustryTileServices(),
		.object = &GetRustObjectServices(),
		.town = &GetRustTownServices(),
		.station = &GetRustStationServices(),
		.visits = &GetRustVehicleVisits(),
		.station_count = []() noexcept -> size_t { return Station::GetNumItems(); },
		.station_if_valid = RustStationIfValid<BaseStation *>,
		.next_station = RustNextStation<BaseStation *>,
		.owner = RustStationServiceOwner,
		.station_owner = RustStationServiceByID,
		.id = RustBaseStationID<BaseStation>,
		.location = RustStationLocation,
		.associated_industry = RustStationAssociatedIndustry,
		.radius_facilities = [](BaseStation *handle) noexcept -> OpenTTDStationRadiusFacilities {
			const Station *st = Station::From(handle);
			return {st->bus_stops != nullptr, st->truck_stops != nullptr, RustStationTrainTile(st), RustStationShipTile(st), RustStationAirportTile(st)};
		},
		.modified_catchment = RustModifiedCatchment,
		.spread = RustStationSpread,
		.industry_by_id = RustIndustryByID<uint16_t>,
		.industry_id = RustIndustryID<Industry *>,
		.industry_owner = RustIndustryCanonicalOwner<Industry *>,
		.industry_neutral = RustIndustryNeutralBaseStation,
		.next_industry = OpenTTDNextIndustry,
	};
	return services;
}

/**
 * Calculates the maintenance cost of all airports of a company.
 * @param owner Company.
 * @return Total cost.
 */
Money AirportMaintenanceCost(Owner owner)
{
	return Money(openttd_rust_station_airport_maintenance(&GetRustStationLifecycleServices(), owner.base()));
}


const OpenTTDStationLifecycleServices &GetRustStationLifecycleServices() noexcept
{
	static_assert(STR_SV_STNAME_FALLBACK - STR_SV_STNAME <= 256);
	static const OpenTTDStationLifecycleServices services{
		.shared = &GetRustSharedServices(),
		.catchment = &GetRustStationCatchmentServices(),
		.names = {
			.base = STR_SV_STNAME, .central = STR_SV_STNAME_CENTRAL, .airport = STR_SV_STNAME_AIRPORT,
			.oilfield = STR_SV_STNAME_OILFIELD, .docks = STR_SV_STNAME_DOCKS, .heliport = STR_SV_STNAME_HELIPORT,
			.mines = STR_SV_STNAME_MINES, .lakeside = STR_SV_STNAME_LAKESIDE, .woods = STR_SV_STNAME_WOODS,
			.forest = STR_SV_STNAME_FOREST, .valley = STR_SV_STNAME_VALLEY, .heights = STR_SV_STNAME_HEIGHTS,
			.north = STR_SV_STNAME_NORTH, .south = STR_SV_STNAME_SOUTH, .east = STR_SV_STNAME_EAST, .west = STR_SV_STNAME_WEST,
			.transfer = STR_SV_STNAME_TRANSFER, .halt = STR_SV_STNAME_HALT, .exchange = STR_SV_STNAME_EXCHANGE,
			.annexe = STR_SV_STNAME_ANNEXE, .sidings = STR_SV_STNAME_SIDINGS, .branch = STR_SV_STNAME_BRANCH,
			.upper = STR_SV_STNAME_UPPER, .lower = STR_SV_STNAME_LOWER, .fallback = STR_SV_STNAME_FALLBACK,
			.undefined = STR_UNDEFINED, .null = STR_NULL,
		},
		.industry_type = RustStationIndustryType,
		.set_industry_type = RustStationSetIndustryType,
		.industry_generation = &GetIndustryGenerationServices(),
		.spec_name = RustIndustrySpecStationName,
		.spec_life = RustIndustrySpecLife,
		.forest = [](uint32_t tile) noexcept { return IsTileForestIndustry(TileIndex{tile}); },
		.roadstop = &GetRoadStopResolverServices(),
		.cleaning_pool = []() noexcept { return Station::CleaningPool(); },
		.close_view = [](uint16_t id) noexcept { CloseWindowById(WC_STATION_VIEW, StationID{id}); },
		.delete_news = [](uint16_t id) noexcept { DeleteStationNews(StationID{id}); },
		.remove_index = [](uint16_t id) noexcept { _station_kdtree.Remove(StationID{id}); },
		.sign_valid = RustStationSignValid,
		.remove_sign = [](uint16_t id) noexcept { _viewport_sign_kdtree.Remove(ViewportSignKdtreeItem::MakeStation(StationID{id})); },
		.last_loading = RustVehicleLastLoadingStation,
		.airport_maintenance = RustAirportSpecMaintenance,
		.maintenance_price = PR_INFRASTRUCTURE_AIRPORT,
	};
	return services;
}
