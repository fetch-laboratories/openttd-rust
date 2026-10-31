/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file station_cmd.cpp Handling of station tiles. */

#include "stdafx.h"
#include "rust/animation_ffi.h"
#include "core/flatset_type.hpp"
#include "aircraft.h"
#include "bridge_map.h"
#include "vehiclelist_func.h"
#include "viewport_func.h"
#include "viewport_kdtree.h"
#include "command_func.h"
#include "town.h"
#include "news_func.h"
#include "train.h"
#include "ship.h"
#include "roadveh.h"
#include "industry.h"
#include "newgrf_cargo.h"
#include "newgrf_debug.h"
#include "newgrf_station.h"
#include "newgrf_canal.h" /* For the buoy */
#include "pathfinder/yapf/yapf_cache.h"
#include "road_internal.h" /* For drawing catenary/checking road removal */
#include "autoslope.h"
#include "water.h"
#include "tilehighlight_func.h"
#include "strings_func.h"
#include "clear_func.h"
#include "timer/timer_game_calendar.h"
#include "vehicle_func.h"
#include "string_func.h"
#include "animated_tile_func.h"
#include "elrail_func.h"
#include "station_base.h"
#include "station_func.h"
#include "station_kdtree.h"
#include "roadstop_base.h"
#include "newgrf_railtype.h"
#include "newgrf_roadtype.h"
#include "waypoint_base.h"
#include "waypoint_func.h"
#include "pbs.h"
#include "debug.h"
#include "core/random_func.hpp"
#include "core/container_func.hpp"
#include "company_base.h"
#include "table/airporttile_ids.h"
#include "newgrf_airporttiles.h"
#include "order_backup.h"
#include "newgrf_house.h"
#include "company_gui.h"
#include "linkgraph/linkgraph_base.h"
#include "linkgraph/refresh.h"
#include "tunnelbridge_map.h"
#include "station_cmd.h"
#include "waypoint_cmd.h"
#include "landscape_cmd.h"
#include "rail_cmd.h"
#include "newgrf_roadstop.h"
#include "timer/timer.h"
#include "timer/timer_game_calendar.h"
#include "timer/timer_game_economy.h"
#include "timer/timer_game_tick.h"
#include "cheat_type.h"
#include "road_func.h"
#include "station_layout_type.h"

#include "widgets/station_widget.h"
#include "widgets/misc_widget.h"

#include "table/strings.h"
#include "table/station_land.h"

#include <bitset>

#include "rust/vehicle_services.h"
#include "rust/services_map.h"
#include "rust/services_station.h"
#include "rust/services_cargo.h"
#include "rust/cargo_registry_ffi.h"
#include "rust/settings_services.h"
#include "rust/station_command_ffi.h"
#include "rust/tile_owner_services.h"
#include "rust/command_services.h"
#include "rust/presentation_services.h"
#include "rust/tile_animation_services.h"
#include "rust/settings_services.h"
#include "rust/environment_services.h"
#include "rust/services_water.h"
#include "rust/company_services.h"
#include "rust/industry_shell_services.h"
#include "rust/services_road.h"
#include "rust/vehicle_visits_ffi.h"
#include "rust/services_linkgraph.h"
#include "rust/station_lifecycle_ffi.h"
#include "rust/station_resolver_ffi.h"
#include "rust/newgrf_vehicle_ffi.h"

#include "rust/clock_ffi.h"

#include "safeguards.h"

/**
 * Static instance of FlowStat::SharesMap.
 * Note: This instance is created on task start.
 *       Lazy creation on first usage results in a data race between the CDist threads.
 */

/**
 * Check whether the given tile is a hangar.
 * @param t the tile to of whether it is a hangar.
 * @pre IsTileType(t, MP_STATION)
 * @return true if and only if the tile is a hangar.
 */
bool IsHangar(Tile t)
{
	return openttd_rust_is_hangar(TileIndex(t).base(), &GetAirportResolverServices());
}

/**
 * Look for a station owned by the given company around the given tile area.
 * @param ta the area to search over
 * @param closest_station the closest owned station found so far
 * @param company the company whose stations to look for
 * @param st to 'return' the found station
 * @param filter Filter function
 * @return Succeeded command (if zero or one station found) or failed command (for two or more stations found).
 */



/**
 * Find the closest deleted station of the current company
 * @param tile the tile to search from.
 * @return the closest station or nullptr if too far.
 */

TileArea Station::GetTileArea(StationType type) const
{
	switch (type) {
		case StationType::Rail: return this->train_station;
		case StationType::Airport: return this->airport;
		case StationType::Truck: return this->truck_station;
		case StationType::Bus: return this->bus_station;
		case StationType::Dock:
		case StationType::Oilrig: return this->docking_station;
		default: NOT_REACHED();
	}
}

/**
 * Update the virtual coords needed to draw the station sign.
 */
void Station::UpdateVirtCoord()
{
	Point pt = RemapCoords2(TileX(this->xy) * TILE_SIZE, TileY(this->xy) * TILE_SIZE);

	pt.y -= 32 * ZOOM_BASE;
	if (this->facilities.Test(StationFacility::Airport) && this->airport.type == AT_OILRIG) pt.y -= 16 * ZOOM_BASE;

	if (this->sign.kdtree_valid) _viewport_sign_kdtree.Remove(ViewportSignKdtreeItem::MakeStation(this->index));

	this->sign.UpdatePosition(pt.x, pt.y, GetString(STR_VIEWPORT_STATION, this->index, this->facilities), GetString(STR_STATION_NAME, this->index, this->facilities));

	_viewport_sign_kdtree.Insert(ViewportSignKdtreeItem::MakeStation(this->index));

	SetWindowDirty(WC_STATION_VIEW, this->index);
}

/** Update the virtual coords needed to draw the station sign for all stations. */
void UpdateAllStationVirtCoords()
{
	for (BaseStation *st : BaseStation::Iterate()) {
		st->UpdateVirtCoord();
	}
}

void BaseStation::FillCachedName() const
{
	auto tmp_params = MakeParameters(this->index);
	this->cached_name = GetStringWithArgs(Waypoint::IsExpected(this) ? STR_WAYPOINT_NAME : STR_STATION_NAME, tmp_params);
}

void ClearAllStationCachedNames()
{
	for (BaseStation *st : BaseStation::Iterate()) {
		st->cached_name.clear();
	}
}

/**
 * Get a mask of the cargo types that the station accepts.
 * @param st Station to query
 * @return the expected mask
 */
CargoTypes GetAcceptanceMask(const Station *st)
{
	return openttd_rust_station_acceptance_mask(st->rust_service.get());
}

/**
 * Get a mask of the cargo types that are empty at the station.
 * @param st Station to query
 * @return the empty mask
 */
CargoTypes GetEmptyMask(const Station *st)
{
	return openttd_rust_station_empty_mask(const_cast<Station *>(st));
}

/**
 * Add news item for when a station changes which cargoes it accepts.
 * @param st Station of cargo change.
 * @param cargoes Bit mask of cargo types to list.
 * @param reject True iff the station rejects the cargo types.
 */
static void ShowRejectOrAcceptNews(const Station *st, CargoTypes cargoes, bool reject)
{
	StringID msg = reject ? STR_NEWS_STATION_NO_LONGER_ACCEPTS_CARGO_LIST : STR_NEWS_STATION_NOW_ACCEPTS_CARGO_LIST;
	AddNewsItem(GetEncodedString(msg, st->index, cargoes), NewsType::Acceptance, NewsStyle::Small, NewsFlag::InColour, st->index);
}

/**
 * Get the cargo types being produced around the tile (in a rectangle).
 * @param north_tile Northern most tile of area
 * @param w X extent of the area
 * @param h Y extent of the area
 * @param rad Search radius in addition to the given area
 */
CargoArray GetProductionAroundTiles(TileIndex north_tile, int w, int h, int rad)
{
	CargoArray produced{};
	openttd_rust_station_production_around(north_tile.base(), w, h, rad, produced.data(), &GetRustStationCatchmentServices());
	return produced;
}

/**
 * Get the acceptance of cargoes around the tile in 1/8.
 * @param center_tile Center of the search area
 * @param w X extent of area
 * @param h Y extent of area
 * @param rad Search radius in addition to given area
 * @param always_accepted bitmask of cargo accepted by houses and headquarters; can be nullptr
 * @param ind Industry associated with neutral station (e.g. oil rig) or nullptr
 */
CargoArray GetAcceptanceAroundTiles(TileIndex center_tile, int w, int h, int rad, CargoTypes *always_accepted)
{
	CargoArray acceptance{};
	openttd_rust_station_acceptance_around(center_tile.base(), w, h, rad, acceptance.data(), always_accepted, &GetRustStationCatchmentServices());
	return acceptance;
}

struct RustStationTruncation {
	StationCargoAmountMap amounts;
	StationCargoAmountMap::iterator iterator;
};
static const OpenTTDStationServices _rust_station_services = {
	.graphs = &GetRustLiveGraphServices(),
	.station = [](uint16_t id) noexcept -> OpenTTDLoadingStation {
		Station *st = RustStationGet<Station *>(id);
		return {.native = st, .base = st, .state = RustStationServiceOwner(st)};
	},
	.station_owner = RustStationOwnerByte<BaseStation>,
	.station_exclusivity = [](OpenTTDBaseStation *handle) noexcept -> OpenTTDStationExclusivity {
		const Town *town = OpenTTDBaseStationTown(static_cast<BaseStation *>(handle));
		return {
			town->exclusive_counter, town->exclusivity.base()
		};
	},
	.rating_counts = [](OpenTTDBaseStation *handle, uint8_t cargo) noexcept -> OpenTTDRatingCounts {
		GoodsEntry &ge = Station::From(static_cast<BaseStation *>(handle))->goods[cargo];
		return {
			ge.AvailableCount(), ge.HasData() ? static_cast<uint32_t>(ge.GetData().cargo.MapSize()) : 0
		};
	},
	.get = [](uint16_t id) noexcept -> OpenTTDBaseStation * {
		return BaseStation::GetIfValid(StationID(id));
	},
	.owner = RustStationServiceOwner,
	.pool_size = []() noexcept -> size_t {
		return BaseStation::GetPoolSize();
	},
	.environment = GetRustEnvironmentServices(),
	.facilities = RustStationFacilitiesHandle<OpenTTDBaseStation *>,
	.in_use = [](OpenTTDBaseStation *handle) noexcept -> bool {
		return static_cast<BaseStation *>(handle)->IsInUse();
	},
	.is_station = RustStationIsExpected<OpenTTDBaseStation *>,
	.local_owner = [](OpenTTDBaseStation *handle) noexcept -> bool {
		return static_cast<BaseStation *>(handle)->owner == _local_company;
	},
	.town_statue = [](OpenTTDBaseStation *handle) noexcept -> bool {
		return Company::IsValidID(static_cast<BaseStation *>(handle)->owner) && OpenTTDBaseStationTown(static_cast<BaseStation *>(handle))->statues.Test(static_cast<BaseStation *>(handle)->owner);
	},
	.select_goods = []() noexcept -> bool {
		return _settings_game.order.selectgoods;
	},
	.rating_cheat = []() noexcept -> bool {
		return _cheats.station_rating.value;
	},
	.available = [](OpenTTDBaseStation *handle, uint8_t cargo) noexcept -> uint32_t {
		return Station::From(static_cast<BaseStation *>(handle))->goods[cargo].AvailableCount();
	},
	.cargo_valid = OpenTTDCargoSpecValid,
	.rating_callback_enabled = [](uint8_t cargo) noexcept -> bool {
		return RustCargoSpec(cargo)->callback_mask.Test(CargoCallbackMask::StationRatingCalc);
	},
	.passenger_cargo = [](uint8_t cargo) noexcept -> bool {
		return IsCargoInClass(cargo, CargoClass::Passengers);
	},
	.packet_available = []() noexcept -> bool {
		return CargoPacket::CanAllocateItem();
	},
	.rating_redraw = [](OpenTTDBaseStation *handle, bool waiting_changed) noexcept -> void {
		BaseStation *st = static_cast<BaseStation *>(handle);
		if (waiting_changed) SetWindowDirty(WC_STATION_VIEW, st->index);
		else SetWindowWidgetDirty(WC_STATION_VIEW, st->index, WID_SV_ACCEPT_RATING_LIST);
	},
	.acceptance_news = [](OpenTTDBaseStation *handle, uint64_t cargoes, bool reject) noexcept -> void {
		ShowRejectOrAcceptNews(Station::From(static_cast<BaseStation *>(handle)), cargoes, reject);
	},
	.acceptance_redraw = [](OpenTTDBaseStation *handle) noexcept -> void {
		SetWindowWidgetDirty(WC_STATION_VIEW, static_cast<BaseStation *>(handle)->index, WID_SV_ACCEPT_RATING_LIST);
	},
	.delete_station = [](OpenTTDBaseStation *handle) noexcept -> void {
		delete static_cast<BaseStation *>(handle);
	},


	.append_packet = [](OpenTTDBaseStation *handle, uint8_t cargo, uint32_t amount, OpenTTDCargoSource source) noexcept -> void {
		Station *st = Station::From(static_cast<BaseStation *>(handle));
		GoodsEntry &ge = st->goods[cargo];
		StationID next = ge.GetVia(st->index);
		ge.GetOrCreateData().cargo.Append(new CargoPacket(st->index, amount, Source(source.id, source.type)), next);
	},
	.station_list_invalidation = RustStationListInvalidation,


	.waiting_redraw = [](OpenTTDBaseStation *handle) noexcept -> void {
		SetWindowDirty(WC_STATION_VIEW, static_cast<BaseStation *>(handle)->index);
		RustStationMarkTilesDirty(static_cast<BaseStation *>(handle), true);
	},
	.cargo_registry = &GetRustCargoRegistryServices(),
	.with_truncation = [](OpenTTDBaseStation *handle, uint8_t cargo, uint32_t amount, void *context, OpenTTDStationScope body) noexcept {
		RustStationTruncation result;
		GoodsEntry &ge = static_cast<Station *>(handle)->goods[cargo];
		if (ge.HasData()) ge.GetData().cargo.Truncate(amount, &result.amounts);
		result.iterator = result.amounts.begin();
		body(context, &result);
	},
	.truncate_next = [](void *handle) noexcept -> OpenTTDTruncatedSource {
		auto *result = static_cast<RustStationTruncation *>(handle);
		if (result->iterator == result->amounts.end()) return {0, 0, false};
		const auto &[source, amount] = *result->iterator++;
		return {amount, source.base(), true};
	},
	.shared = &GetRustSharedServices(),
};
static const OpenTTDStationLinks _rust_station_links = {
	.graphs = &GetRustLiveGraphServices(),
	.automatic_distribution = OpenTTDAutomaticCargoDistribution,
	.edge = [](OpenTTDBaseStation *from, uint16_t destination) noexcept -> OpenTTDStationEdge {
		Station *to = Station::Get(StationID(destination));
		return {to, OpenTTDDistanceManhattan(RustStationLocation(from), RustStationLocation(to))};
	},
	.delete_flows = [](OpenTTDBaseStation *handle, uint8_t cargo, uint16_t destination) noexcept -> void {
		GoodsEntry &ge = static_cast<Station *>(handle)->goods[cargo];
		if (ge.HasData()) ge.GetData().flows.DeleteFlows(StationID(destination));
	},
	.restrict_flows = [](OpenTTDBaseStation *handle, uint8_t cargo, uint16_t destination) noexcept -> void {
		GoodsEntry &ge = static_cast<Station *>(handle)->goods[cargo];
		if (ge.HasData()) ge.GetData().flows.RestrictFlows(StationID(destination));
	},
	.order_list = [](uint16_t id) noexcept -> OpenTTDOrderList * {
		return OrderList::GetIfValid(OrderListID(id));
	},
	.order_pool_size = []() noexcept -> size_t {
		return OrderList::GetPoolSize();
	},
	.order_count = [](OpenTTDOrderList *handle) noexcept -> size_t {
		return static_cast<OrderList *>(handle)->GetOrders().size();
	},
	.order_destination = [](OpenTTDOrderList *handle, size_t index) noexcept -> OpenTTDOrderDestination {
		const Order *order = &static_cast<OrderList *>(handle)->GetOrders()[index];
		return {order->GetDestination().base(), order->GetType()};
	},
	.order_vehicle = [](OpenTTDOrderList *handle) noexcept -> OpenTTDVehicle * {
		return RustVehicle(static_cast<OrderList *>(handle)->GetFirstSharedVehicle());
	},
	.next_vehicle = GetRustVehicleServices().next,
	.next_shared = [](OpenTTDVehicle *handle) noexcept -> OpenTTDVehicle * {
		return RustVehicle(VehicleOf(handle)->NextShared());
	},
	.stopped_in_depot = [](OpenTTDVehicle *handle) noexcept -> bool {
		return VehicleOf(handle)->IsStoppedInDepot();
	},
	.last_service = RustVehicleLastServiceOf,
	.cargo_type = RustVehicleCargoType,
	.goods_entry = RustStationGoodsEntry,
	.station_resolver = &GetStationResolverServices(),
	.vehicle_resolver = &GetRustVehicleResolverServices(),
};
const OpenTTDStationServices &GetRustStationServices() noexcept { return _rust_station_services; }
const OpenTTDStationLinks &GetRustStationLinks() noexcept { return _rust_station_links; }


/**
 * Update the acceptance for a station.
 * @param st Station to update
 * @param show_msg controls whether to display a message that acceptance was changed.
 */
void UpdateStationAcceptance(Station *st, bool show_msg)
{
	openttd_rust_station_acceptance(st, &_rust_station_services, show_msg);
}


/**
 * Common part of building various station parts and possibly attaching them to an existing one.
 * @param[in,out] st Station to attach to
 * @param flags Command flags
 * @param reuse Whether to try to reuse a deleted station (gray sign) if possible
 * @param area Area occupied by the new part
 * @param name_class Station naming class to use to generate the new station's name
 * @return Command error that occurred, if any
 */

/**
 * This is called right after a station was deleted.
 * It checks if the whole station is free of substations, and if so, the station will be
 * deleted after a little while.
 * @param st Station
 */

/**
 * After adding/removing tiles to station, update some station-related stuff.
 * @param adding True if adding tiles, false if removing them.
 * @param type StationType being modified.
 */
void Station::AfterStationTileSetChange(bool adding, StationType type)
{
	openttd_rust_station_tiles_changed(&GetRustStationCommandServices(), this, adding, to_underlying(type));
}

CommandCost ClearTile_Station(TileIndex tile, DoCommandFlags flags);

/**
 * Checks if the given tile is buildable, flat and has a certain height.
 * @param tile TileIndex to check.
 * @param invalid_dirs Prohibited directions for slopes (set of #DiagDirection).
 * @param allowed_z Height allowed for the tile. If allowed_z is negative, it will be set to the height of this tile.
 * @param allow_steep Whether steep slopes are allowed.
 * @param check_bridge Check for the existence of a bridge.
 * @return The cost in case of success, or an error code if it failed.
 */
CommandCost CheckBuildableTile(TileIndex tile, DiagDirections invalid_dirs, int &allowed_z, bool allow_steep, bool check_bridge = true)
{
	CommandCost cost;
	openttd_rust_station_check_buildable(&GetRustStationCommandServices(), RustCommandCost(&cost), tile.base(), invalid_dirs.base(), &allowed_z, allow_steep, check_bridge);
	return cost;
}

/**
 * Checks if an airport can be built at the given location and clear the area.
 * @param tile_iter Airport tile iterator.
 * @param flags Operation to perform.
 * @return The cost in case of success, or an error code if it failed.
 */

/**
 * Get station-type-specific string for a bridge that is too low.
 * @param type Station type.
 * @return bridge too low string.
 */

/**
 * Get bridgeable tile information for a station type.
 * @param type Station type.
 * @return bridgeable tile information.
 */
static std::span<const BridgeableTileInfo> GetStationBridgeableTileInfo(StationType type)
{
	return _station_bridgeable_info[to_underlying(type)];
}

/**
 * Test if a rail station can be built below a bridge.
 * @param tile Tile to test.
 * @param spec Custom station spec to test.
 * @param type Type of rail station.
 * @param layout Layout piece of station to test.
 * @return Command result.
 */

/**
 * Test if a road station can be built below a bridge.
 * @param tile Tile to test.
 * @param spec Custom roadstop spec to test.
 * @param type Type of road station.
 * @param layout Layout piece of station to test.
 * @return Command result.
 */
CommandCost IsRoadStationBridgeAboveOk(TileIndex tile, const RoadStopSpec *spec, StationType type, StationGfx layout)
{
	CommandCost cost;
	openttd_rust_station_road_bridge_ok(&GetRustStationCommandServices(), RustCommandCost(&cost), tile.base(), spec, to_underlying(type), layout);
	return cost;
}

/**
 * Test if a dock can be built below a bridge.
 * @param tile Tile to test.
 * @param layout Layout piece of station to test.
 * @return Command result.
 */

/**
 * Test if a buoy can be built below a bridge.
 * @param tile Tile to test.
 * @return Command result.
 */

/**
 * Checks if a rail station can be built at the given tile.
 * @param tile_cur Tile to check.
 * @param north_tile North tile of the area being checked.
 * @param allowed_z Height allowed for the tile. If allowed_z is negative, it will be set to the height of this tile.
 * @param flags Operation to perform.
 * @param axis Rail station axis.
 * @param station StationID to be queried and returned if available.
 * @param rt The rail type to check for (overbuilding rail stations over rail).
 * @param affected_vehicles List of trains with PBS reservations on the tiles
 * @param spec_class Station class.
 * @param spec_index Index into the station class.
 * @param plat_len Platform length.
 * @param numtracks Number of platforms.
 * @return The cost in case of success, or an error code if it failed.
 */

/**
 * Checks if a road stop can be built at the given tile.
 * @param cur_tile Tile to check.
 * @param allowed_z Height allowed for the tile. If allowed_z is negative, it will be set to the height of this tile.
 * @param spec Spec of road stop to be built.
 * @param flags Operation to perform.
 * @param invalid_dirs Prohibited directions (set of DiagDirections).
 * @param is_drive_through True if trying to build a drive-through station.
 * @param station_type Station type (bus, truck or road waypoint).
 * @param axis Axis of a drive-through road stop.
 * @param station StationID to be queried and returned if available.
 * @param rt Road type to build, may be INVALID_ROADTYPE if an existing road is required.
 * @return The cost in case of success, or an error code if it failed.
 */

/**
 * Check whether we can expand the rail part of the given station.
 * @param st the station to expand
 * @param new_ta the current (and if all is fine new) tile area of the rail part of the station
 * @return Succeeded or failed command.
 */

RailStationTileLayout::RailStationTileLayout(const StationSpec *spec, uint8_t platforms, uint8_t length) : platforms(platforms), length(length)
{
	if (spec == nullptr) return;

	/* Look for a predefined layout for the required size. */
	auto found = spec->layouts.find(GetStationLayoutKey(platforms, length));
	if (found != std::end(spec->layouts)) this->layout = found->second;
}

StationGfx RailStationTileLayout::Iterator::operator*() const
{
	/* Use predefined layout if it exists. Mask bit zero which will indicate axis. */
	if (!stl.layout.empty()) return this->stl.layout[this->position] & ~1;

	if (this->stl.length == 1) {
		/* Special case for 1-long platforms, all bare platforms except one small building. */
		return this->position == ((this->stl.platforms - 1) / 2) ? 2 : 0;
	}

	if ((this->position < this->stl.length && (this->stl.platforms % 2 == 1))) {
		/* Number of tracks is odd, make the first platform bare with a small building. */
		return this->position == ((this->stl.length - 1) / 2) ? 2 : 0;
	}

	if (this->stl.length > 4 && ((this->position % this->stl.length) == 0 || (this->position % this->stl.length) == this->stl.length - 1)) {
		/* Station is longer than 4 tiles, place bare platforms at either end. */
		return 0;
	}

	/* None of the above so must be north or south part of larger station. */
	return (((this->position / this->stl.length) % 2) == (this->stl.platforms % 2)) ? 4 : 6;
}

/**
 * Find a nearby station that joins this station.
 * @tparam T the class to find a station for
 * @tparam error_message the error message when building a station on top of others
 * @tparam F the filter functor type
 * @param existing_station an existing station we build over
 * @param station_to_join the station to join to
 * @param adjacent whether adjacent stations are allowed
 * @param ta the area of the newly build station
 * @param st 'return' pointer for the found station
 * @return command cost with the error or 'okay'
 */

/**
 * Find a nearby station that joins this station.
 * @param existing_station an existing station we build over
 * @param station_to_join the station to join to
 * @param adjacent whether adjacent stations are allowed
 * @param ta the area of the newly build station
 * @param st 'return' pointer for the found station
 * @return command cost with the error or 'okay'
 */

/**
 * Find a nearby waypoint that joins this waypoint.
 * @param existing_waypoint an existing waypoint we build over
 * @param waypoint_to_join the waypoint to join to
 * @param adjacent whether adjacent waypoints are allowed
 * @param ta the area of the newly build waypoint
 * @param wp 'return' pointer for the found waypoint
 * @param is_road whether to find a road waypoint
 * @return command cost with the error or 'okay'
 */

/**
 * Clear platform reservation during station building/removing.
 * @param v vehicle which holds reservation
 */

/**
 * Get station tile flags for the given StationGfx.
 * @param gfx StationGfx of station tile.
 * @param statspec Station spec of station tile.
 * @return Tile flags to apply.
 */

/**
 * Set rail station tile flags for the given tile.
 * @param tile Tile to set flags on.
 * @param statspec Statspec of the tile.
 */
void SetRailStationTileFlags(TileIndex tile, const StationSpec *statspec)
{
	openttd_rust_station_set_rail_flags(&GetRustStationCommandServices(), tile.base(), statspec);
}

/**
 * Build rail station
 * @param flags operation to perform
 * @param tile_org northern most position of station dragging/placement
 * @param rt railtype
 * @param axis orientation (Axis)
 * @param numtracks number of tracks
 * @param plat_len platform length
 * @param spec_class custom station class
 * @param spec_index custom station id
 * @param station_to_join station ID to join (NEW_STATION if build new one)
 * @param adjacent allow stations directly adjacent to other stations.
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildRailStation(DoCommandFlags flags, TileIndex tile_org, RailType rt, Axis axis, uint8_t numtracks, uint8_t plat_len, StationClassID spec_class, uint16_t spec_index, StationID station_to_join, bool adjacent)
{
	CommandCost cost, ret;
	openttd_rust_station_build_rail(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile_org.base(), rt, axis, numtracks, plat_len, spec_class, spec_index, station_to_join.base(), adjacent);
	return cost;
}







/**
 * Remove a number of tiles from any rail station within the area.
 * @param ta the area to clear station tile from.
 * @param affected_stations the stations affected.
 * @param flags the command flags.
 * @param removal_cost the cost for removing the tile, including the rail.
 * @param keep_rail whether to keep the rail of the station.
 * @tparam T the type of station to remove.
 * @return the number of cleared tiles or an error.
 */

/**
 * Remove a single tile from a rail station.
 * This allows for custom-built station with holes and weird layouts
 * @param flags operation to perform
 * @param start tile of station piece to remove
 * @param end other edge of the rect to remove
 * @param keep_rail if set keep the rail
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveFromRailStation(DoCommandFlags flags, TileIndex start, TileIndex end, bool keep_rail)
{
	CommandCost cost, ret, error;
	openttd_rust_station_remove_rail_area(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&error), flags.base(), start.base(), end.base(), keep_rail, false);
	return cost;
}

/**
 * Remove a single tile from a waypoint.
 * This allows for custom-built waypoint with holes and weird layouts
 * @param flags operation to perform
 * @param start tile of waypoint piece to remove
 * @param end other edge of the rect to remove
 * @param keep_rail if set keep the rail
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveFromRailWaypoint(DoCommandFlags flags, TileIndex start, TileIndex end, bool keep_rail)
{
	CommandCost cost, ret, error;
	openttd_rust_station_remove_rail_area(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&error), flags.base(), start.base(), end.base(), keep_rail, true);
	return cost;
}


/**
 * Remove a rail station/waypoint
 * @param st The station/waypoint to remove the rail part from
 * @param flags operation to perform
 * @param removal_cost the cost for removing a tile
 * @tparam T the type of station to remove
 * @return cost or failure of operation
 */

/**
 * Remove a rail station
 * @param tile Tile of the station.
 * @param flags operation to perform
 * @return cost or failure of operation
 */

/**
 * Remove a rail waypoint
 * @param tile Tile of the waypoint.
 * @param flags operation to perform
 * @return cost or failure of operation
 */


/**
 * @param truck_station Determines whether a stop is #RoadStopType::Bus or #RoadStopType::Truck
 * @param st The Station to do the whole procedure for
 * @return a pointer to where to link a new RoadStop*
 */


/**
 * Find a nearby station that joins this road stop.
 * @param existing_stop an existing road stop we build over
 * @param station_to_join the station to join to
 * @param adjacent whether adjacent stations are allowed
 * @param ta the area of the newly build station
 * @param st 'return' pointer for the found station
 * @return command cost with the error or 'okay'
 */

/**
 * Calculates cost of new road stops within the area.
 * @param tile_area Area to check.
 * @param flags Operation to perform.
 * @param is_drive_through True if trying to build a drive-through station.
 * @param station_type Station type (bus, truck or road waypoint).
 * @param roadstopspec Spec of road stop being built.
 * @param axis Axis of a drive-through road stop.
 * @param ddir Entrance direction (#DiagDirection) for normal stops. Converted to the axis for drive-through stops.
 * @param station StationID to be queried and returned if available.
 * @param rt Road type to build, may be INVALID_ROADTYPE if an existing road is required.
 * @param unit_cost The cost to build one road stop of the current type.
 * @return The cost in case of success, or an error code if it failed.
 */

/**
 * Build a bus or truck stop.
 * @param flags Operation to perform.
 * @param tile Northernmost tile of the stop.
 * @param width Width of the road stop.
 * @param length Length of the road stop.
 * @param stop_type Type of road stop (bus/truck).
 * @param is_drive_through False for normal stops, true for drive-through.
 * @param ddir Entrance direction (#DiagDirection) for normal stops. Converted to the axis for drive-through stops.
 * @param rt The roadtype.
 * @param spec_class Road stop spec class.
 * @param spec_index Road stop spec index.
 * @param station_to_join Station ID to join (NEW_STATION if build new one).
 * @param adjacent Allow stations directly adjacent to other stations.
 * @return The cost of this operation or an error.
 */
CommandCost CmdBuildRoadStop(DoCommandFlags flags, TileIndex tile, uint8_t width, uint8_t length, RoadStopType stop_type, bool is_drive_through,
		DiagDirection ddir, RoadType rt, RoadStopClassID spec_class, uint16_t spec_index, StationID station_to_join, bool adjacent)
{
	CommandCost cost, ret;
	openttd_rust_station_build_road(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), width, length, to_underlying(stop_type), is_drive_through, ddir, rt, spec_class, spec_index, station_to_join.base(), adjacent);
	return cost;
}

/**
 * Remove a bus station/truck stop
 * @param tile TileIndex been queried
 * @param flags operation to perform
 * @param replacement_spec_index replacement spec index to avoid deallocating, if < 0, tile is not being replaced
 * @return cost or failure of operation
 */

/**
 * Remove a road waypoint
 * @param tile TileIndex been queried
 * @param flags operation to perform
 * @param replacement_spec_index replacement spec index to avoid deallocating, if < 0, tile is not being replaced
 * @return cost or failure of operation
 */

/**
 * Remove a tile area of road stop or road waypoints
 * @param flags operation to perform
 * @param roadstop_area tile area of road stop or road waypoint tiles to remove
 * @param road_waypoint Whether to remove road waypoints or road stops
 * @param remove_road Remove roads of drive-through stops?
 * @return the cost of this operation or an error
 */

/**
 * Remove bus or truck stops.
 * @param flags Operation to perform.
 * @param tile Northernmost tile of the removal area.
 * @param width Width of the removal area.
 * @param height Height of the removal area.
 * @param stop_type Type of stop (bus/truck).
 * @param remove_road Remove roads of drive-through stops?
 * @return The cost of this operation or an error.
 */
CommandCost CmdRemoveRoadStop(DoCommandFlags flags, TileIndex tile, uint8_t width, uint8_t height, RoadStopType stop_type, bool remove_road)
{
	CommandCost cost, ret, error;
	openttd_rust_station_remove_road_area(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&error), flags.base(), tile.base(), 0, width, height, to_underlying(stop_type), remove_road, false);
	return cost;
}

/**
 * Remove road waypoints.
 * @param flags operation to perform
 * @param start tile of road waypoint piece to remove
 * @param end other edge of the rect to remove
 * @return the cost of this operation or an error
 */
CommandCost CmdRemoveFromRoadWaypoint(DoCommandFlags flags, TileIndex start, TileIndex end)
{
	CommandCost cost, ret, error;
	openttd_rust_station_remove_road_area(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&error), flags.base(), start.base(), end.base(), 0, 0, 0, false, true);
	return cost;
}

/**
 * Get a possible noise reduction factor based on distance from town center.
 * The further you get, the less noise you generate.
 * So all those folks at city council can now happily slee...  work in their offices
 * @param as airport information
 * @param distance minimum distance between town and airport
 * @return the noise that will be generated, according to distance
 */
uint8_t GetAirportNoiseLevelForDistance(const AirportSpec *as, uint distance)
{
	return openttd_rust_station_airport_noise(&GetRustStationCommandServices(), as->noise_level, distance);
}

/**
 * Finds the town nearest to given airport. Based on minimal manhattan distance to any airport's tile.
 * If two towns have the same distance, town with lower index is returned.
 * @param as airport's description
 * @param rotation airport's rotation
 * @param tile origin tile (top corner of the airport)
 * @param it An iterator over all airport tiles (consumed)
 * @param[out] mindist Minimum distance to town
 * @return nearest town to airport
 */
Town *AirportGetNearestTown(const AirportSpec *as, Direction rotation, TileIndex tile, TileIterator &&it, uint &mindist)
{
	const auto *town = openttd_rust_station_airport_nearest(&GetRustStationCommandServices(), as, rotation, tile.base(), &it, &mindist);
	return const_cast<Town *>(reinterpret_cast<const Town *>(town));
}

/**
 * Finds the town nearest to given existing airport. Based on minimal manhattan distance to any airport's tile.
 * If two towns have the same distance, town with lower index is returned.
 * @param station existing station with airport
 * @param[out] mindist Minimum distance to town
 * @return nearest town to airport
 */

/** Recalculate the noise generated by the airports of each town */
void UpdateAirportsNoise()
{
	openttd_rust_station_update_airport_noise(&GetRustStationCommandServices());
}

/**
 * Place an Airport.
 * @param flags operation to perform
 * @param tile tile where airport will be built
 * @param airport_type airport type, @see airport.h
 * @param layout airport layout
 * @param station_to_join station ID to join (NEW_STATION if build new one)
 * @param allow_adjacent allow airports directly adjacent to other airports.
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildAirport(DoCommandFlags flags, TileIndex tile, uint8_t airport_type, uint8_t layout, StationID station_to_join, bool allow_adjacent)
{
	CommandCost cost, ret;
	openttd_rust_station_build_airport(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), airport_type, layout, station_to_join.base(), allow_adjacent);
	return cost;
}

/**
 * Remove an airport
 * @param tile TileIndex been queried
 * @param flags operation to perform
 * @return cost or failure of operation
 */

/**
 * Open/close an airport to incoming aircraft.
 * @param flags Operation to perform.
 * @param station_id Station ID of the airport.
 * @return the cost of this operation or an error
 */
CommandCost CmdOpenCloseAirport(DoCommandFlags flags, StationID station_id)
{
	CommandCost cost;
	openttd_rust_station_open_close_airport(&GetRustStationCommandServices(), RustCommandCost(&cost), flags.base(), station_id.base());
	return cost;
}

/**
 * Tests whether the company's vehicles have this station in orders
 * @param station station ID
 * @param include_company If true only check vehicles of \a company, if false only check vehicles of other companies
 * @param company company ID
 */
bool HasStationInUse(StationID station, bool include_company, CompanyID company)
{
	return openttd_rust_station_in_orders(&GetRustStationCommandServices(), station.base(), include_company, company.base());
}


/**
 * Build a dock/haven.
 * @param flags operation to perform
 * @param tile tile where dock will be built
 * @param station_to_join station ID to join (NEW_STATION if build new one)
 * @param adjacent allow docks directly adjacent to other docks.
 * @return the cost of this operation or an error
 */
CommandCost CmdBuildDock(DoCommandFlags flags, TileIndex tile, StationID station_to_join, bool adjacent)
{
	CommandCost cost, ret;
	openttd_rust_station_build_dock(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), flags.base(), tile.base(), station_to_join.base(), adjacent);
	return cost;
}

void RemoveDockingTile(TileIndex t)
{
	openttd_rust_station_remove_docking_tile(&GetRustStationCommandServices(), t.base());
}

/**
 * Clear docking tile status from tiles around a removed dock, if the tile has
 * no neighbours which would keep it as a docking tile.
 * @param tile Ex-dock tile to check.
 */
void ClearDockingTilesCheckingNeighbours(TileIndex tile)
{
	openttd_rust_station_clear_docking_neighbours(&GetRustStationCommandServices(), tile.base());
}

/**
 * Find the part of a dock that is land-based
 * @param t Dock tile to find land part of
 * @return tile of land part of dock
 */

/**
 * Remove a dock
 * @param tile TileIndex been queried
 * @param flags operation to perform
 * @return cost or failure of operation
 */

/**
 * Get station tile layout for a station type and its station gfx.
 * @param st Station type to draw.
 * @param gfx StationGfx of tile to draw.
 * @return Tile layout to draw.
 */
const DrawTileSprites *GetStationTileLayout(StationType st, uint8_t gfx)
{
	const auto &layouts = _station_display_datas[to_underlying(st)];
	if (gfx >= layouts.size()) gfx &= 1;
	return layouts.data() + gfx;
}

/**
 * Check whether a sprite is a track sprite, which can be replaced by a non-track ground sprite and a rail overlay.
 * If the ground sprite is suitable, \a ground is replaced with the new non-track ground sprite, and \a overlay_offset
 * is set to the overlay to draw.
 * @param         ti             Positional info for the tile to decide snowyness etc. May be nullptr.
 * @param[in,out] ground         Groundsprite to draw.
 * @param[out]    overlay_offset Overlay to draw.
 * @return true if overlay can be drawn.
 */
bool SplitGroundSpriteForOverlay(const TileInfo *ti, SpriteID *ground, RailTrackOffset *overlay_offset)
{
	bool snow_desert;
	switch (*ground) {
		case SPR_RAIL_TRACK_X:
		case SPR_MONO_TRACK_X:
		case SPR_MGLV_TRACK_X:
			snow_desert = false;
			*overlay_offset = RTO_X;
			break;

		case SPR_RAIL_TRACK_Y:
		case SPR_MONO_TRACK_Y:
		case SPR_MGLV_TRACK_Y:
			snow_desert = false;
			*overlay_offset = RTO_Y;
			break;

		case SPR_RAIL_TRACK_X_SNOW:
		case SPR_MONO_TRACK_X_SNOW:
		case SPR_MGLV_TRACK_X_SNOW:
			snow_desert = true;
			*overlay_offset = RTO_X;
			break;

		case SPR_RAIL_TRACK_Y_SNOW:
		case SPR_MONO_TRACK_Y_SNOW:
		case SPR_MGLV_TRACK_Y_SNOW:
			snow_desert = true;
			*overlay_offset = RTO_Y;
			break;

		default:
			return false;
	}

	if (ti != nullptr) {
		/* Decide snow/desert from tile */
		switch (_settings_game.game_creation.landscape) {
			case LandscapeType::Arctic:
				snow_desert = (uint)ti->z > GetSnowLine() * TILE_HEIGHT;
				break;

			case LandscapeType::Tropic:
				snow_desert = GetTropicZone(ti->tile) == TROPICZONE_DESERT;
				break;

			default:
				break;
		}
	}

	*ground = snow_desert ? SPR_FLAT_SNOW_DESERT_TILE : SPR_FLAT_GRASS_TILE;
	return true;
}

/**
 * Get blocked pillar information for a station tile.
 * @param bridgeable_info
 * @param layout Tile layout of rail station.
 * @return blocked pillar information.
 */
static BridgePillarFlags GetStationBlockedPillars(std::span<const BridgeableTileInfo> bridgeable_info, uint8_t layout)
{
	if (layout < std::size(bridgeable_info)) return bridgeable_info[layout].disallowed_pillars;
	return BRIDGEPILLARFLAGS_ALL;
}

/**
 * Draw custom station foundations for a NewGRF station if provided.
 * @param statspec Custom NewGRF station.
 * @param st Station being drawn.
 * @param ti Tile being drawn.
 * @param gfx Graphics layout for tile.
 * @return true iff a custom foundation was drawn.
 */
static bool DrawCustomStationFoundations(const StationSpec *statspec, BaseStation *st, TileInfo *ti, StationGfx gfx)
{
	if (statspec == nullptr || !statspec->flags.Test(StationSpecFlag::CustomFoundations)) return false;

	/* Station has custom foundations.
	 * Check whether the foundation continues beyond the tile's upper sides. */
	uint edge_info = 0;
	auto [slope, z] = GetFoundationPixelSlope(ti->tile);
	if (!HasFoundationNW(ti->tile, slope, z)) SetBit(edge_info, 0);
	if (!HasFoundationNE(ti->tile, slope, z)) SetBit(edge_info, 1);

	SpriteID image = GetCustomStationFoundationRelocation(statspec, st, ti->tile, gfx, edge_info);
	if (image == 0) return false;

	if (statspec->flags.Test(StationSpecFlag::ExtendedFoundations)) {
		/* Station provides extended foundations. */
		static constexpr uint8_t foundation_parts[] = {
			UINT8_MAX, UINT8_MAX, UINT8_MAX,         0, // Invalid,  Invalid,   Invalid,   SLOPE_SW
			UINT8_MAX,         1,         2,         3, // Invalid,  SLOPE_EW,  SLOPE_SE,  SLOPE_WSE
			UINT8_MAX,         4,         5,         6, // Invalid,  SLOPE_NW,  SLOPE_NS,  SLOPE_NWS
			        7,         8,         9,            // SLOPE_NE, SLOPE_ENW, SLOPE_SEN
		};
		assert(ti->tileh < std::size(foundation_parts));
		if (foundation_parts[ti->tileh] == UINT8_MAX) return false;

		AddSortableSpriteToDraw(image + foundation_parts[ti->tileh], PAL_NONE, *ti, {{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT - 1}, {}});
	} else {
		/* Draw simple foundations, built up from 8 possible foundation sprites.
		 * Each set bit represents one of the eight composite sprites to be drawn.
		 * 'Invalid' entries will not drawn but are included for completeness. */
		static constexpr uint8_t composite_foundation_parts[] = {
			0b0000'0000, 0b1101'0001, 0b1110'0100, 0b1110'0000, // Invalid,  Invalid,   Invalid,   SLOPE_SW
			0b1100'1010, 0b1100'1001, 0b1100'0100, 0b1100'0000, // Invalid,  SLOPE_EW,  SLOPE_SE,  SLOPE_WSE
			0b1101'0010, 0b1001'0001, 0b1110'0100, 0b1010'0000, // Invalid,  SLOPE_NW,  SLOPE_NS,  SLOPE_NWS
			0b0100'1010, 0b0000'1001, 0b0100'0100,              // SLOPE_NE, SLOPE_ENW, SLOPE_SEN
		};
		assert(ti->tileh < std::size(composite_foundation_parts));

		uint8_t parts = composite_foundation_parts[ti->tileh];

		/* If foundations continue beyond the tile's upper sides then mask out the last two pieces. */
		if (HasBit(edge_info, 0)) ClrBit(parts, 6);
		if (HasBit(edge_info, 1)) ClrBit(parts, 7);

		/* We always have to draw at least one sprite, so if we have no parts to draw fall back to default foundation. */
		if (parts == 0) return false;

		StartSpriteCombine();
		for (uint i : SetBitIterator(parts)) {
			AddSortableSpriteToDraw(image + i, PAL_NONE, *ti, {{}, {TILE_SIZE, TILE_SIZE, TILE_HEIGHT - 1}, {}});
		}
		EndSpriteCombine();
	}

	OffsetGroundSprite(0, -static_cast<int>(TILE_HEIGHT));
	ti->z += ApplyPixelFoundationToSlope(FOUNDATION_LEVELED, ti->tileh);

	return true;
}

static void DrawTile_Station(TileInfo *ti)
{
	const NewGRFSpriteLayout *layout = nullptr;
	SpriteLayoutProcessor processor; // owns heap, borrowed by tmp_layout and t
	DrawTileSpriteSpan tmp_layout;
	const DrawTileSprites *t = nullptr;
	int32_t total_offset;
	const RailTypeInfo *rti = nullptr;
	uint32_t relocation = 0;
	uint32_t ground_relocation = 0;
	BaseStation *st = nullptr;
	const StationSpec *statspec = nullptr;
	uint tile_layout = 0;
	auto bridgeable_info = GetStationBridgeableTileInfo(GetStationType(ti->tile));

	if (HasStationRail(ti->tile)) {
		rti = GetRailTypeInfo(GetRailType(ti->tile));
		total_offset = rti->GetRailtypeSpriteOffset();

		if (IsCustomStationSpecIndex(ti->tile)) {
			/* look for customization */
			st = BaseStation::GetByTile(ti->tile);
			statspec = st->speclist[GetCustomStationSpecIndex(ti->tile)].spec;

			if (statspec != nullptr) {
				tile_layout = GetStationGfx(ti->tile);

				if (statspec->callback_mask.Test(StationCallbackMask::DrawTileLayout)) {
					uint16_t callback = GetStationCallback(CBID_STATION_DRAW_TILE_LAYOUT, 0, 0, statspec, st, ti->tile);
					if (callback != CALLBACK_FAILED) tile_layout = (callback & ~1) + GetRailStationAxis(ti->tile);
				}

				/* Ensure the chosen tile layout is valid for this custom station */
				if (!statspec->renderdata.empty()) {
					layout = &statspec->renderdata[tile_layout < statspec->renderdata.size() ? tile_layout : (uint)GetRailStationAxis(ti->tile)];
					if (!layout->NeedsPreprocessing()) {
						t = layout;
						layout = nullptr;
					}
				}
			}
		}
		if (statspec != nullptr) bridgeable_info = statspec->bridgeable_info;
	} else {
		total_offset = 0;
	}

	StationGfx gfx = GetStationGfx(ti->tile);
	if (IsAirport(ti->tile)) {
		gfx = GetAirportGfx(ti->tile);
		if (gfx >= NEW_AIRPORTTILE_OFFSET) {
			const AirportTileSpec *ats = AirportTileSpec::Get(gfx);
			if (ats->grf_prop.HasSpriteGroups() && DrawNewAirportTile(ti, Station::GetByTile(ti->tile), ats)) {
				return;
			}
			/* No sprite group (or no valid one) found, meaning no graphics associated.
			 * Use the substitute one instead */
			assert(ats->grf_prop.subst_id != INVALID_AIRPORTTILE);
			gfx = ats->grf_prop.subst_id;
		}
		switch (gfx) {
			case APT_RADAR_GRASS_FENCE_SW:
				t = &_station_display_datas_airport_radar_grass_fence_sw[GetAnimationFrame(ti->tile)];
				break;
			case APT_GRASS_FENCE_NE_FLAG:
				t = &_station_display_datas_airport_flag_grass_fence_ne[GetAnimationFrame(ti->tile)];
				break;
			case APT_RADAR_FENCE_SW:
				t = &_station_display_datas_airport_radar_fence_sw[GetAnimationFrame(ti->tile)];
				break;
			case APT_RADAR_FENCE_NE:
				t = &_station_display_datas_airport_radar_fence_ne[GetAnimationFrame(ti->tile)];
				break;
			case APT_GRASS_FENCE_NE_FLAG_2:
				t = &_station_display_datas_airport_flag_grass_fence_ne_2[GetAnimationFrame(ti->tile)];
				break;
		}
	}

	Owner owner = GetTileOwner(ti->tile);

	PaletteID palette;
	if (Company::IsValidID(owner)) {
		palette = GetCompanyPalette(owner);
	} else {
		/* Some stations are not owner by a company, namely oil rigs */
		palette = PALETTE_TO_GREY;
	}

	if (layout == nullptr && t == nullptr) t = GetStationTileLayout(GetStationType(ti->tile), gfx);

	/* don't show foundation for docks */
	if (ti->tileh != SLOPE_FLAT && !IsDock(ti->tile)) {
		if (!DrawCustomStationFoundations(statspec, st, ti, tile_layout)) {
			DrawFoundation(ti, FOUNDATION_LEVELED);
		}
	}

	bool draw_ground = false;

	if (IsBuoy(ti->tile)) {
		DrawWaterClassGround(ti);
		SpriteID sprite = GetCanalSprite(CF_BUOY, ti->tile);
		if (sprite != 0) total_offset = sprite - SPR_IMG_BUOY;
	} else if (IsDock(ti->tile) || (IsOilRig(ti->tile) && IsTileOnWater(ti->tile))) {
		if (ti->tileh == SLOPE_FLAT) {
			DrawWaterClassGround(ti);
		} else {
			assert(IsDock(ti->tile));
			TileIndex water_tile = ti->tile + TileOffsByDiagDir(GetDockDirection(ti->tile));
			WaterClass wc = HasTileWaterClass(water_tile) ? GetWaterClass(water_tile) : WaterClass::Invalid;
			if (wc == WaterClass::Sea) {
				DrawShoreTile(ti->tileh);
			} else {
				DrawClearLandTile(ti, 3);
			}
		}
	} else if (IsRoadWaypointTile(ti->tile)) {
		RoadBits bits = AxisToRoadBits(GetDriveThroughStopAxis(ti->tile));
		RoadType road_rt = GetRoadTypeRoad(ti->tile);
		RoadType tram_rt = GetRoadTypeTram(ti->tile);
		RoadBits road = (road_rt != INVALID_ROADTYPE) ? bits : ROAD_NONE;
		RoadBits tram = (tram_rt != INVALID_ROADTYPE) ? bits : ROAD_NONE;
		const RoadTypeInfo *road_rti = (road_rt != INVALID_ROADTYPE) ? GetRoadTypeInfo(road_rt) : nullptr;
		const RoadTypeInfo *tram_rti = (tram_rt != INVALID_ROADTYPE) ? GetRoadTypeInfo(tram_rt) : nullptr;

		if (ti->tileh != SLOPE_FLAT) {
			DrawFoundation(ti, FOUNDATION_LEVELED);
		}

		DrawRoadGroundSprites(ti, road, tram, road_rti, tram_rti, GetRoadWaypointRoadside(ti->tile), IsRoadWaypointOnSnowOrDesert(ti->tile));
	} else {
		if (layout != nullptr) {
			/* Sprite layout which needs preprocessing */
			bool separate_ground = statspec->flags.Test(StationSpecFlag::SeparateGround);
			processor = SpriteLayoutProcessor(*layout, total_offset, rti->fallback_railtype, 0, 0, separate_ground);
			GetCustomStationRelocation(processor, statspec, st, ti->tile);
			tmp_layout = processor.GetLayout();
			t = &tmp_layout;
			total_offset = 0;
		} else if (statspec != nullptr) {
			/* Simple sprite layout */
			ground_relocation = relocation = GetCustomStationRelocation(statspec, st, ti->tile, 0);
			if (statspec->flags.Test(StationSpecFlag::SeparateGround)) {
				ground_relocation = GetCustomStationRelocation(statspec, st, ti->tile, 1);
			}
			ground_relocation += rti->fallback_railtype;
		}

		draw_ground = true;
	}

	if (draw_ground && !IsAnyRoadStop(ti->tile)) {
		SpriteID image = t->ground.sprite;
		PaletteID pal  = t->ground.pal;
		RailTrackOffset overlay_offset;
		if (rti != nullptr && rti->UsesOverlay() && SplitGroundSpriteForOverlay(ti, &image, &overlay_offset)) {
			SpriteID ground = GetCustomRailSprite(rti, ti->tile, RTSG_GROUND);
			DrawGroundSprite(image, PAL_NONE);
			DrawGroundSprite(ground + overlay_offset, PAL_NONE);

			if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation && HasStationReservation(ti->tile)) {
				SpriteID overlay = GetCustomRailSprite(rti, ti->tile, RTSG_OVERLAY);
				DrawGroundSprite(overlay + overlay_offset, PALETTE_CRASH);
			}
		} else {
			image += HasBit(image, SPRITE_MODIFIER_CUSTOM_SPRITE) ? ground_relocation : total_offset;
			if (HasBit(pal, SPRITE_MODIFIER_CUSTOM_SPRITE)) pal += ground_relocation;
			DrawGroundSprite(image, GroundSpritePaletteTransform(image, pal, palette));

			/* PBS debugging, draw reserved tracks darker */
			if (_game_mode != GM_MENU && _settings_client.gui.show_track_reservation && HasStationRail(ti->tile) && HasStationReservation(ti->tile)) {
				DrawGroundSprite(GetRailStationAxis(ti->tile) == AXIS_X ? rti->base_sprites.single_x : rti->base_sprites.single_y, PALETTE_CRASH);
			}
		}
	}

	if (HasStationRail(ti->tile) && HasRailCatenaryDrawn(GetRailType(ti->tile))) DrawRailCatenary(ti);

	if (IsAnyRoadStop(ti->tile)) {
		RoadType road_rt = GetRoadTypeRoad(ti->tile);
		RoadType tram_rt = GetRoadTypeTram(ti->tile);
		const RoadTypeInfo *road_rti = road_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(road_rt);
		const RoadTypeInfo *tram_rti = tram_rt == INVALID_ROADTYPE ? nullptr : GetRoadTypeInfo(tram_rt);

		StationGfx view = GetStationGfx(ti->tile);
		StationType type = GetStationType(ti->tile);

		const RoadStopSpec *stopspec = GetRoadStopSpec(ti->tile);
		RoadStopDrawModes stop_draw_mode{};
		if (stopspec != nullptr) {
			stop_draw_mode = stopspec->draw_mode;
			st = BaseStation::GetByTile(ti->tile);
			std::array<int32_t, 1> regs100;
			auto result = GetRoadStopLayout(ti, stopspec, st, type, view, regs100);
			if (result.has_value()) {
				if (stopspec->flags.Test(RoadStopSpecFlag::DrawModeRegister)) {
					stop_draw_mode = static_cast<RoadStopDrawModes>(regs100[0]);
				}
				if (type == StationType::RoadWaypoint && stop_draw_mode.Test(RoadStopDrawMode::WaypGround)) {
					draw_ground = true;
				}
				processor = std::move(*result);
				tmp_layout = processor.GetLayout();
				t = &tmp_layout;
			}
		}

		/* Draw ground sprite */
		if (draw_ground) {
			SpriteID image = t->ground.sprite;
			PaletteID pal  = t->ground.pal;
			image += HasBit(image, SPRITE_MODIFIER_CUSTOM_SPRITE) ? ground_relocation : total_offset;
			if (GB(image, 0, SPRITE_WIDTH) != 0) {
				if (HasBit(pal, SPRITE_MODIFIER_CUSTOM_SPRITE)) pal += ground_relocation;
				DrawGroundSprite(image, GroundSpritePaletteTransform(image, pal, palette));
			}
		}

		if (IsDriveThroughStopTile(ti->tile)) {
			if (type != StationType::RoadWaypoint && (stopspec == nullptr || stop_draw_mode.Test(RoadStopDrawMode::Overlay))) {
				uint sprite_offset = GetDriveThroughStopAxis(ti->tile) == AXIS_X ? 1 : 0;
				DrawRoadOverlays(ti, PAL_NONE, road_rti, tram_rti, sprite_offset, sprite_offset);
			}
		} else {
			/* Non-drivethrough road stops are only valid for roads. */
			assert(road_rt != INVALID_ROADTYPE && tram_rt == INVALID_ROADTYPE);

			if ((stopspec == nullptr || stop_draw_mode.Test(RoadStopDrawMode::Road)) && road_rti->UsesOverlay()) {
				SpriteID ground = GetCustomRoadSprite(road_rti, ti->tile, ROTSG_ROADSTOP);
				DrawGroundSprite(ground + view, PAL_NONE);
			}
		}
		if (stopspec != nullptr) bridgeable_info = stopspec->bridgeable_info;

		if (stopspec == nullptr || !stopspec->flags.Test(RoadStopSpecFlag::NoCatenary)) {
			/* Draw road, tram catenary */
			DrawRoadCatenary(ti);
		}
	}

	if (IsRailWaypoint(ti->tile)) {
		/* Don't offset the waypoint graphics; they're always the same. */
		total_offset = 0;
	}

	DrawRailTileSeq(ti, t, TO_BUILDINGS, total_offset, relocation, palette);
	DrawBridgeMiddle(ti, GetStationBlockedPillars(bridgeable_info, GetStationGfx(ti->tile)));
}

void StationPickerDrawSprite(int x, int y, StationType st, RailType railtype, RoadType roadtype, int image)
{
	int32_t total_offset = 0;
	PaletteID pal = GetCompanyPalette(_local_company);
	const DrawTileSprites *t = GetStationTileLayout(st, image);
	const RailTypeInfo *railtype_info = nullptr;

	if (railtype != INVALID_RAILTYPE) {
		railtype_info = GetRailTypeInfo(railtype);
		total_offset = railtype_info->GetRailtypeSpriteOffset();
	}

	SpriteID img = t->ground.sprite;
	RailTrackOffset overlay_offset;
	if (railtype_info != nullptr && railtype_info->UsesOverlay() && SplitGroundSpriteForOverlay(nullptr, &img, &overlay_offset)) {
		SpriteID ground = GetCustomRailSprite(railtype_info, INVALID_TILE, RTSG_GROUND);
		DrawSprite(img, PAL_NONE, x, y);
		DrawSprite(ground + overlay_offset, PAL_NONE, x, y);
	} else {
		DrawSprite(img + total_offset, HasBit(img, PALETTE_MODIFIER_COLOUR) ? pal : PAL_NONE, x, y);
	}

	if (roadtype != INVALID_ROADTYPE) {
		const RoadTypeInfo *roadtype_info = GetRoadTypeInfo(roadtype);
		if (image >= 4) {
			/* Drive-through stop */
			uint sprite_offset = 5 - image;

			/* Road underlay takes precedence over tram */
			if (roadtype_info->UsesOverlay()) {
				SpriteID ground = GetCustomRoadSprite(roadtype_info, INVALID_TILE, ROTSG_GROUND);
				DrawSprite(ground + sprite_offset, PAL_NONE, x, y);

				SpriteID overlay = GetCustomRoadSprite(roadtype_info, INVALID_TILE, ROTSG_OVERLAY);
				if (overlay) DrawSprite(overlay + sprite_offset, PAL_NONE, x, y);
			} else if (RoadTypeIsTram(roadtype)) {
				DrawSprite(SPR_TRAMWAY_TRAM + sprite_offset, PAL_NONE, x, y);
			}
		} else {
			/* Bay stop */
			if (RoadTypeIsRoad(roadtype) && roadtype_info->UsesOverlay()) {
				SpriteID ground = GetCustomRoadSprite(roadtype_info, INVALID_TILE, ROTSG_ROADSTOP);
				DrawSprite(ground + image, PAL_NONE, x, y);
			}
		}
	}

	/* Default waypoint has no railtype specific sprites */
	DrawRailTileSeqInGUI(x, y, t, (st == StationType::RailWaypoint || st == StationType::RoadWaypoint) ? 0 : total_offset, 0, pal);
}



static void FillTileDescRoadStop(TileIndex tile, TileDesc &td)
{
	RoadType road_rt = GetRoadTypeRoad(tile);
	RoadType tram_rt = GetRoadTypeTram(tile);
	Owner road_owner = INVALID_OWNER;
	Owner tram_owner = INVALID_OWNER;
	if (road_rt != INVALID_ROADTYPE) {
		const RoadTypeInfo *rti = GetRoadTypeInfo(road_rt);
		td.roadtype = rti->strings.name;
		td.road_speed = rti->max_speed / 2;
		road_owner = GetRoadOwner(tile, RTT_ROAD);
	}

	if (tram_rt != INVALID_ROADTYPE) {
		const RoadTypeInfo *rti = GetRoadTypeInfo(tram_rt);
		td.tramtype = rti->strings.name;
		td.tram_speed = rti->max_speed / 2;
		tram_owner = GetRoadOwner(tile, RTT_TRAM);
	}

	if (IsDriveThroughStopTile(tile)) {
		/* Is there a mix of owners? */
		if ((tram_owner != INVALID_OWNER && tram_owner != td.owner[0]) ||
				(road_owner != INVALID_OWNER && road_owner != td.owner[0])) {
			uint i = 1;
			if (road_owner != INVALID_OWNER) {
				td.owner_type[i] = STR_LAND_AREA_INFORMATION_ROAD_OWNER;
				td.owner[i] = road_owner;
				i++;
			}
			if (tram_owner != INVALID_OWNER) {
				td.owner_type[i] = STR_LAND_AREA_INFORMATION_TRAM_OWNER;
				td.owner[i] = tram_owner;
			}
		}
	}
}

void FillTileDescRailStation(TileIndex tile, TileDesc &td)
{
	const StationSpec *spec = GetStationSpec(tile);

	if (spec != nullptr) {
		td.station_class = StationClass::Get(spec->class_index)->name;
		td.station_name  = spec->name;

		if (spec->grf_prop.HasGrfFile()) {
			const GRFConfig *gc = GetGRFConfig(spec->grf_prop.grfid);
			td.grf = gc->GetName();
		}
	}

	const RailTypeInfo *rti = GetRailTypeInfo(GetRailType(tile));
	td.rail_speed = rti->max_speed;
	td.railtype = rti->strings.name;
}

void FillTileDescAirport(TileIndex tile, TileDesc &td)
{
	const AirportSpec *as = Station::GetByTile(tile)->airport.GetSpec();
	td.airport_class = AirportClass::Get(as->class_index)->name;
	td.airport_name = as->name;

	const AirportTileSpec *ats = AirportTileSpec::GetByTile(tile);
	td.airport_tile_name = ats->name;

	if (as->grf_prop.HasGrfFile()) {
		const GRFConfig *gc = GetGRFConfig(as->grf_prop.grfid);
		td.grf = gc->GetName();
	} else if (ats->grf_prop.HasGrfFile()) {
		const GRFConfig *gc = GetGRFConfig(ats->grf_prop.grfid);
		td.grf = gc->GetName();
	}
}

static void GetTileDesc_Station(TileIndex tile, TileDesc &td)
{
	td.owner[0] = GetTileOwner(tile);
	td.build_date = BaseStation::GetByTile(tile)->build_date;

	if (IsAnyRoadStop(tile)) FillTileDescRoadStop(tile, td);
	if (HasStationRail(tile)) FillTileDescRailStation(tile, td);
	if (IsAirport(tile)) FillTileDescAirport(tile, td);

	StringID str;
	switch (GetStationType(tile)) {
		default: NOT_REACHED();
		case StationType::Rail:     str = STR_LAI_STATION_DESCRIPTION_RAILROAD_STATION; break;
		case StationType::Airport:
			str = (IsHangar(tile) ? STR_LAI_STATION_DESCRIPTION_AIRCRAFT_HANGAR : STR_LAI_STATION_DESCRIPTION_AIRPORT);
			break;
		case StationType::Truck:    str = STR_LAI_STATION_DESCRIPTION_TRUCK_LOADING_AREA; break;
		case StationType::Bus:      str = STR_LAI_STATION_DESCRIPTION_BUS_STATION; break;
		case StationType::Oilrig: {
			const Industry *i = Station::GetByTile(tile)->industry;
			const IndustrySpec *is = GetIndustrySpec(i->type);
			td.owner[0] = i->owner;
			str = is->name;
			if (is->grf_prop.HasGrfFile()) td.grf = GetGRFConfig(is->grf_prop.grfid)->GetName();
			break;
		}
		case StationType::Dock:     str = STR_LAI_STATION_DESCRIPTION_SHIP_DOCK; break;
		case StationType::Buoy:     str = STR_LAI_STATION_DESCRIPTION_BUOY; break;
		case StationType::RailWaypoint: str = STR_LAI_STATION_DESCRIPTION_WAYPOINT; break;
		case StationType::RoadWaypoint: str = STR_LAI_STATION_DESCRIPTION_WAYPOINT; break;
	}
	td.str = str;
}








static bool ClickTile_Station(TileIndex tile)
{
	const BaseStation *bst = BaseStation::GetByTile(tile);

	if (bst->facilities.Test(StationFacility::Waypoint)) {
		ShowWaypointWindow(Waypoint::From(bst));
	} else if (IsHangar(tile)) {
		const Station *st = Station::From(bst);
		ShowDepotWindow(st->airport.GetHangarTile(st->airport.GetHangarNum(tile)), VEH_AIRCRAFT);
	} else {
		ShowStationViewWindow(bst->index);
	}
	return true;
}


/**
 * Run the watched cargo callback for all houses in the catchment area.
 * @param st Station.
 */
void TriggerWatchedCargoCallbacks(Station *st)
{
	openttd_rust_station_watched(st, &_rust_station_services);
}



/**
 * Reroute cargo of type c at station st or in any vehicles unloading there.
 * Make sure the cargo's new next hop is neither "avoid" nor "avoid2".
 * @param st Station to be rerouted at.
 * @param cargo Type of cargo.
 * @param avoid Original next hop of cargo, avoid this.
 * @param avoid2 Another station to be avoided when rerouting.
 */
void RerouteCargo(Station *st, CargoType cargo, StationID avoid, StationID avoid2)
{
	openttd_rust_station_reroute(st, cargo, avoid.base(), avoid2.base(), &_rust_station_services, &_rust_station_links);
}

/**
 * Check all next hops of cargo packets in this station for existence of a
 * a valid link they may use to travel on. Reroute any cargo not having a valid
 * link and remove timed out links found like this from the linkgraph. We're
 * not all links here as that is expensive and useless. A link no one is using
 * doesn't hurt either.
 * @param from Station to check.
 */
void DeleteStaleLinks(Station *from)
{
	openttd_rust_station_stale(from, &_rust_station_services, &_rust_station_links, &GetRustVehicleVisits());
}




/** Economy monthly loop for stations. */
static const IntervalTimer<TimerGameEconomy> _economy_stations_monthly({TimerGameEconomy::MONTH, TimerGameEconomy::Priority::STATION}, RustClockCallback{openttd_rust_clock_station_monthly, &_rust_station_services});




/**
 * Rename a station
 * @param flags operation to perform
 * @param station_id station ID that is to be renamed
 * @param text the new name or an empty string when resetting to the default
 * @return the cost of this operation or an error
 */
CommandCost CmdRenameStation(DoCommandFlags flags, StationID station_id, const std::string &text)
{
	CommandCost cost;
	openttd_rust_station_rename(&GetRustStationCommandServices(), RustCommandCost(&cost), flags.base(), station_id.base(), reinterpret_cast<const uint8_t *>(text.data()), text.size());
	return cost;
}

/**
 * Move a station name.
 * @param flags type of operation
 * @param station_id id of the station
 * @param tile to move the station name to
 * @return the cost of this operation or an error and the station ID
 */
std::tuple<CommandCost, StationID> CmdMoveStationName(DoCommandFlags flags, StationID station_id, TileIndex tile)
{
	CommandCost cost;
	auto id = openttd_rust_station_move_name(&GetRustStationCommandServices(), RustCommandCost(&cost), flags.base(), station_id.base(), tile.base());
	return {cost, StationID(id)};
}

/**
* Callback function that is called after a name is moved
* @param result of the operation
* @param station_id ID of the changed station
*/
void CcMoveStationName(Commands, const CommandCost &result, StationID station_id)
	{
		if (result.Failed()) return;

		ResetObjectToPlace();
		Station *st = Station::Get(station_id);
		SetViewportStationRect(st, false);
	}





void UpdateStationDockingTiles(Station *st)
{
	openttd_rust_station_update_docking(&GetRustStationCommandServices(), st);
}

void BuildOilRig(TileIndex tile)
{
	CommandCost ignored;
	openttd_rust_station_build_oilrig(&GetRustStationCommandServices(), RustCommandCost(&ignored), tile.base());
}

void DeleteOilRig(TileIndex tile)
{
	openttd_rust_station_delete_oilrig(&GetRustStationCommandServices(), tile.base());
}


/**
 * Check if a drive-through road stop tile can be cleared.
 * Road stops built on town-owned roads check the conditions
 * that would allow clearing of the original road.
 * @param tile The road stop tile to check.
 * @param flags Command flags.
 * @return A succeeded command if the road can be removed, a failed command with the relevant error message otherwise.
 */

/**
 * Clear a single tile of a station.
 * @param tile The tile to clear.
 * @param flags The DoCommand flags related to the "command".
 * @return The cost, or error of clearing.
 */
CommandCost ClearTile_Station(TileIndex tile, DoCommandFlags flags)
{
	CommandCost cost, ret, error;
	openttd_rust_station_clear(&GetRustStationCommandServices(), RustCommandCost(&cost), RustCommandCost(&ret), RustCommandCost(&error), tile.base(), flags.base());
	return cost;
}

static CommandCost TerraformTile_Station(TileIndex tile, DoCommandFlags flags, int z_new, Slope tileh_new)
{
	CommandCost cost;
	openttd_rust_station_terraform(&GetRustStationCommandServices(), RustCommandCost(&cost), tile.base(), flags.base(), z_new, tileh_new);
	return cost;
}


static CommandCost CheckBuildAbove_Station(TileIndex tile, DoCommandFlags, Axis, int height)
{
	CommandCost cost;
	openttd_rust_station_build_above(&GetRustStationCommandServices(), RustCommandCost(&cost), tile.base(), height);
	return cost;
}

#include "rust/station_command_services.hpp"

extern const TileTypeProcs _tile_type_station_procs = {
	DrawTile_Station,           // draw_tile_proc
	nullptr, // Rust movement owner dispatches directly.
	ClearTile_Station,          // clear_tile_proc
	nullptr,                       // add_accepted_cargo_proc
	GetTileDesc_Station,        // get_tile_desc_proc
	nullptr, // Rust movement owner dispatches directly.
	ClickTile_Station,          // click_tile_proc
	nullptr, // Rust tile scheduler calls the station owner directly.
	nullptr,                 // Rust ownership dispatcher calls the owner directly.
	nullptr,                       // add_produced_cargo_proc
	nullptr, // Rust movement owner dispatches directly.
	nullptr, // Rust movement owner dispatches directly.
	TerraformTile_Station,      // terraform_tile_proc
	CheckBuildAbove_Station, // check_build_above_proc
};

#include "rust/service_table_check.hpp"
#include "abi-visit-OpenTTDEnvironmentServices.hpp"
#include "abi-visit-OpenTTDStationServices.hpp"
#include "abi-visit-OpenTTDStationLinks.hpp"
#include "abi-visit-OpenTTDStationCommandServices.hpp"
#include "rust/station_definitions_ffi.h"
#include "rust/newgrf_act3_ffi.h"
#include "abi-visit-OpenTTDStationAssociationServices.hpp"
#include "abi-visit-OpenTTDStationDefinitionServices.hpp"
#include "abi-visit-OpenTTDAct3CargoServices.hpp"
#include "abi-visit-OpenTTDStationCatchmentServices.hpp"
#include "abi-visit-OpenTTDStationLifecycleServices.hpp"
bool CheckAbi_station_cmd()
{
	return AbiServicesComplete(GetRustStationLifecycleServices()) &&
		AbiServicesComplete(GetRustStationCatchmentServices()) &&
		AbiServicesComplete(_rust_station_services.environment) &&
		AbiServicesComplete(_rust_station_services) &&
		AbiServicesComplete(_rust_station_links) &&
		AbiServicesComplete(GetRustStationCommandServices()) &&
		AbiServicesComplete(GetStationAssociationServices()) &&
		AbiServicesComplete(GetStationDefinitionServices()) &&
		AbiServicesComplete(GetAct3CargoServices());
}
