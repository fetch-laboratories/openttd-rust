/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_station.h Canonical station fields and opaque handles for Rust ports. */
#ifndef RUST_SERVICES_STATION_H
#define RUST_SERVICES_STATION_H
#include "../airport.h"
#include "../station_base.h"
#include "../roadveh.h"
#include "../station_func.h"
#include "../industry.h"
#include "../waypoint_base.h"
#include "../station_kdtree.h"
#include "../town.h"
#include "../window_func.h"
#include "../widgets/station_widget.h"
#include "../newgrf_debug.h"
#include "../newgrf_station.h"
#include "../newgrf_storage.h"
#include "../newgrf_roadstop.h"
#include "../newgrf_airporttiles.h"
#include "station_activity_ffi.h"
#include "vehicle_services.h"
#include "services_ffi.h"
#include "services_newgrf.h"
#include "environment_services.h"
#include "../base_station_base.h"
#include "../station_map.h"
#include "../pbs.h"
#include "roadstop_resolver_ffi.h"
#include "station_association_ffi.h"

/* Shared canonical road-stop association/container leaves. No element address
 * survives a call; evaluation may mutate storage but cannot reset these shells. */
inline CargoTypes RustStationCargoTriggers(const StationSpec *spec) noexcept { return spec->cargo_triggers; }
inline uint16_t RustStationAnimationTriggers(const StationSpec *spec) noexcept { return spec->animation.triggers.base(); }
inline OpenTTDStationSpecTriggers RustStationSpecTriggers(const StationSpec *spec) noexcept { return {RustStationAnimationTriggers(spec), RustStationCargoTriggers(spec)}; }
inline size_t RustStationSpecCount(const BaseStation *st) noexcept { return st->speclist.size(); }
inline const StationSpec *RustStationSlotSpec(const BaseStation *st, size_t index) noexcept { return st->speclist[index].spec; }
inline OpenTTDStationSpecVacancy RustStationSlotVacancy(const BaseStation *st, size_t index) noexcept { const auto &slot = st->speclist[index]; return {slot.spec, slot.grfid}; }
inline OpenTTDGRFClassIdentity RustStationSlotKey(const BaseStation *st, size_t index) noexcept { const auto &slot = st->speclist[index]; return {slot.grfid, slot.localidx}; }
inline void RustStationSpecResize(BaseStation *st, size_t size) noexcept { st->speclist.resize(size); }
inline void RustStationSpecClear(BaseStation *st) noexcept { st->speclist.clear(); }
inline void RustStationSetSpecSlot(BaseStation *st, uint8_t index, const StationSpec *spec, uint32_t grfid, uint16_t local_id) noexcept { auto &slot = st->speclist[index]; slot.spec = spec; slot.grfid = grfid; slot.localidx = local_id; }
inline void RustStationRebindSpec(BaseStation *st, size_t index, const StationSpec *spec) noexcept { st->speclist[index].spec = spec; }
inline uint32_t RustCustomStationIndex(uint32_t tile) noexcept { return GetCustomStationSpecIndex(TileIndex{tile}); }
inline void RustStationSetCachedTriggers(BaseStation *st, uint16_t animation, CargoTypes cargo) noexcept { st->cached_anim_triggers = StationAnimationTriggers(animation); st->cached_cargo_triggers = cargo; }
inline uint8_t RustRoadStopRandomBits(const BaseStation *st, uint32_t tile) noexcept { return st->GetRoadStopRandomBits(TileIndex(tile)); }
inline void RustRoadStopSetRandomBits(BaseStation *st, uint32_t tile, uint8_t bits) noexcept { st->SetRoadStopRandomBits(TileIndex(tile), bits); }
inline uint8_t RustRoadStopAnimationFrame(BaseStation *st, uint32_t tile) noexcept { return st->GetRoadStopAnimationFrame(TileIndex(tile)); }
inline size_t RustRoadStopSpecCount(const BaseStation *st) noexcept { return st->roadstop_speclist.size(); }
inline const RoadStopSpec *RustRoadStopSlotSpec(const BaseStation *st, size_t index) noexcept { return st->roadstop_speclist[index].spec; }
inline uint32_t RustRoadStopSlotGRFID(const BaseStation *st, size_t index) noexcept { return st->roadstop_speclist[index].grfid; }
inline OpenTTDRoadStopSpecKey RustRoadStopSlotKey(const BaseStation *st, size_t index) noexcept { return {RustRoadStopSlotGRFID(st, index), st->roadstop_speclist[index].localidx}; }
inline OpenTTDRoadStopSpecVacancy RustRoadStopSlotVacancy(const BaseStation *st, size_t index) noexcept { return {RustRoadStopSlotSpec(st, index), RustRoadStopSlotGRFID(st, index)}; }
inline void RustRoadStopSpecResize(BaseStation *st, size_t size) noexcept { st->roadstop_speclist.resize(size); }
inline void RustRoadStopSpecClear(BaseStation *st) noexcept { st->roadstop_speclist.clear(); }
inline void RustRoadStopSetSpecSlot(BaseStation *st, uint8_t index, const RoadStopSpec *spec, uint32_t grfid, uint16_t local_id) noexcept { auto &slot = st->roadstop_speclist[index]; slot.spec = spec; slot.grfid = grfid; slot.localidx = local_id; }
inline bool RustCustomRoadStopSpecIndex(uint32_t tile) noexcept { return IsCustomRoadStopSpecIndex(TileIndex(tile)); }
inline uint32_t RustCustomRoadStopIndex(uint32_t tile) noexcept { return GetCustomRoadStopSpecIndex(TileIndex(tile)); }
inline CargoTypes RustRoadStopCachedCargo(const BaseStation *st) noexcept { return st->cached_roadstop_cargo_triggers; }
inline void RustRoadStopSetCachedTriggers(BaseStation *st, uint16_t animation, CargoTypes cargo) noexcept { st->cached_roadstop_anim_triggers = StationAnimationTriggers(animation); st->cached_roadstop_cargo_triggers = cargo; }
inline CargoTypes RustRoadStopCargoTriggers(const RoadStopSpec *spec) noexcept { return spec->cargo_triggers; }
inline uint16_t RustRoadStopAnimationTriggers(const RoadStopSpec *spec) noexcept { return spec->animation.triggers.base(); }
inline OpenTTDRoadStopSpecTriggers RustRoadStopSpecTriggers(const RoadStopSpec *spec) noexcept { return {RustRoadStopAnimationTriggers(spec), RustRoadStopCargoTriggers(spec)}; }

/* Borrowed native station shells; retain strict tile/type/index preconditions.
 * Const and mutable table receivers share each scalar's actual field read. */
inline BaseStation *RustBaseStationByTile(uint32_t tile) noexcept { return BaseStation::GetByTile(TileIndex(tile)); }
/* Bind directly: another service table may still be dynamically initializing. */
template <class Handle>
inline bool RustStationIsExpected(Handle handle) noexcept { return Station::IsExpected(static_cast<const BaseStation *>(handle)); }
template <typename TStation>
inline uint16_t RustBaseStationID(TStation *st) noexcept { return st->index.base(); }
inline uint8_t RustStationFacilities(const BaseStation *st) noexcept { return st->facilities.base(); }
template <class Handle> inline uint8_t RustStationFacilitiesHandle(Handle handle) noexcept { return RustStationFacilities(static_cast<const BaseStation *>(handle)); }
/** Read existing cargo data only; a missing GoodsEntryData stays unallocated. */
inline const OpenTTDCargoList *RustStationExistingCargoOwner(BaseStation *st, uint8_t cargo) noexcept { const auto &goods = Station::From(st)->goods[cargo]; return goods.HasData() ? goods.GetData().cargo.RustOwner() : nullptr; }

inline uint16_t RustAirportMovingFlags(const AirportMovingData *moving) noexcept { return moving->flags.base(); }

template <class Handle> inline Handle RustStationGet(uint16_t id) noexcept { return Station::Get(StationID{id}); }
inline Station *RustAnimationStation(BaseStation *st) noexcept { return Station::From(st); }
inline bool RustStationAirportMember(const Station *st, uint32_t tile) noexcept { return st->TileBelongsToAirport(TileIndex{tile}); }
inline bool RustStationHasAnimationTrigger(BaseStation *s, uint8_t trigger) noexcept { return s->cached_anim_triggers.Test(static_cast<StationAnimationTrigger>(trigger)); }
inline bool RustRoadStopHasAnimationTrigger(BaseStation *s, uint8_t trigger) noexcept { return s->cached_roadstop_anim_triggers.Test(static_cast<StationAnimationTrigger>(trigger)); }
inline size_t RustRoadStopTileDataCount(BaseStation *s) noexcept { return s->custom_roadstop_tile_data.size(); }
inline uint32_t RustRoadStopDataTile(BaseStation *s, size_t i) noexcept { return s->custom_roadstop_tile_data[i].tile.base(); }
inline const StationSpec *RustAnimationRailSpec(uint32_t tile) noexcept { return GetStationSpec(TileIndex{tile}); }
inline bool RustStationRectangleEmpty(const BaseStation *station) noexcept { return station->rect.IsEmpty(); }
/** `&st->goods[cargo].GetOrCreateData().cargo`'s Rust owner. */
inline OpenTTDCargoList *RustStationCargoOwner(OpenTTDStation *st, uint8_t cargo) noexcept { return static_cast<Station *>(st)->goods[cargo].GetOrCreateData().cargo.RustOwner(); }
template <class Handle> inline bool RustStationRectangleEmptyHandle(Handle handle) noexcept { return RustStationRectangleEmpty(static_cast<const BaseStation *>(handle)); }
inline void RustStationCatchment(const Station *station, void *context, void (*visit)(void *, uint32_t)) noexcept
{
	BitmapTileIterator it(station->catchment_tiles);
	for (TileIndex tile = it; tile != INVALID_TILE; tile = ++it) visit(context, tile.base());
}
/** Native entry-frame storage; bind at each original scan. Serialized callbacks
 * cannot rebuild catchments/remove industries/reset the world; no Rust layout view. */
struct RustNearbyIndustryCursor { IndustryList::const_iterator current, end; bool started; };
inline void BeginNearbyIndustries(RustNearbyIndustryCursor *cursor, const Station *station) noexcept
{
	*cursor = {station->industries_near.begin(), station->industries_near.end(), false};
}
inline Industry *NextNearbyIndustry(RustNearbyIndustryCursor *cursor) noexcept
{
	if (cursor->started) ++cursor->current;
	cursor->started = true;
	return cursor->current == cursor->end ? nullptr : cursor->current->industry;
}
inline uint8_t RustAirportLayoutHeading(const AirportFTAClass *airport, uint32_t index) noexcept { return airport->layout[index].heading; }
inline uint8_t OpenTTDAirportLayoutHeading(const void *airport, uint8_t index) noexcept { return RustAirportLayoutHeading(static_cast<const AirportFTAClass *>(airport), index); }
inline uint8_t RustAirportFlags(const AirportFTAClass *airport) noexcept { return airport->flags.base(); }
inline uint8_t OpenTTDAirportFlags(const void *airport) noexcept { return RustAirportFlags(static_cast<const AirportFTAClass *>(airport)); }
inline bool RustSelectedAirportHasHangar(const BaseStation *st, const AirportSpec *spec) noexcept { return Station::From(st)->airport.HasHangar(spec); }
inline TileIndex RustStationHangarTile(const BaseStation *st, uint32_t index, const AirportSpec *spec) noexcept { return Station::From(st)->airport.GetHangarTile(index, spec); }
inline uint32_t RustSelectedHangarTile(const BaseStation *st, uint32_t index, const AirportSpec *spec) noexcept { return RustStationHangarTile(st, index, spec).base(); }
inline RoadStop *RustStationPrimaryRoadStop(const OpenTTDVehicle *vehicle, const Station *st) noexcept { return st->GetPrimaryRoadStop(RoadVehicle::From(VehicleOf(vehicle))); }
inline bool RustStationConvert8bitBoolean(const StationSpec *spec, uint16_t id, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(RustSpecGRFFile(spec), static_cast<CallbackID>(id), value); }
inline bool RustRoadStopConvert8bitBoolean(const RoadStopSpec *spec, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(RustSpecGRFFile(spec), CBID_STATION_AVAILABILITY, value); }
inline void RustStationCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_STATIONS, tile); }
inline void RustRoadStopCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_ROADSTOPS, tile); }
inline void RustAirportTileCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTTILES, tile); }
inline void RustStationMarkTilesDirty(BaseStation *st, bool cargo) noexcept { Station::From(st)->MarkTilesDirty(cargo); }
inline const OpenTTDStation *RustStation(const Station *s) noexcept { return reinterpret_cast<const OpenTTDStation *>(s); }
inline Station *RustStationByTile(uint32_t tile) noexcept { return Station::GetByTile(TileIndex{tile}); }
inline const OpenTTDStation *OpenTTDStationByTile(uint32_t tile) noexcept { return RustStation(RustStationByTile(tile)); }
inline const Station *StationOf(const OpenTTDStation *s) noexcept { return reinterpret_cast<const Station *>(s); }
/* GetSpec preserves the native dummy-airport fallback. Resolver-specific tile
 * gates remain with their callers; all handles are borrowed synchronously. */
inline const AirportFTAClass *RustAirportSpecFTA(const AirportSpec *spec) noexcept { return spec->fsm; }
inline TTDPAirportType RustAirportTTDType(const AirportSpec *spec) noexcept { return spec->ttd_airport_type; }
inline uint32_t RustStationAirportTile(const BaseStation *s) noexcept { return Station::From(s)->airport.tile.base(); }
inline uint8_t RustStationAirportLayout(const BaseStation *st) noexcept { return Station::From(st)->airport.layout; }
inline uint8_t RustStationAirportType(const BaseStation *s) noexcept { return Station::From(s)->airport.type; }
inline Direction RustStationAirportRotation(const BaseStation *s) noexcept { return Station::From(s)->airport.rotation; }
inline uint32_t RustStationLocation(const BaseStation *s) noexcept { return s->xy.base(); }
inline void RustStationRadius(uint32_t tile, uint32_t radius, void *context, void (*visit)(void *, BaseStation *)) noexcept
{
	ForAllStationsRadius(TileIndex{tile}, radius, [context, visit](Station *st) { visit(context, st); });
}
inline Owner OpenTTDStationOwner(const BaseStation *station) noexcept { return station->owner; }
const OpenTTDStationService *OpenTTDStationActivityOwner(const OpenTTDStation *station) noexcept;
inline Town *OpenTTDBaseStationTown(const BaseStation *st) noexcept { return st->town; }
inline uint16_t RustStationTownID(const Station *station) noexcept { return RustTownAuthorityIndex(reinterpret_cast<const OpenTTDTownHandle *>(OpenTTDBaseStationTown(station))); }
inline Station *OpenTTDNextStation(uint32_t from) noexcept { auto range = Station::Iterate(from); return range.empty() ? nullptr : *range.begin(); }
inline bool RustStationHasAirport(const Station *s) noexcept { return s->facilities.Test(StationFacility::Airport); }
inline bool RustStationReserved(uint32_t tile) noexcept { return HasStationReservation(TileIndex{tile}); }
inline void RustSetRailStationReservation(uint32_t tile, bool reserved) noexcept { SetRailStationReservation(TileIndex{tile}, reserved); }
inline bool RustCompatibleStationTiles(uint32_t a, uint32_t b) noexcept { return IsCompatibleTrainStationTile(TileIndex{a}, TileIndex{b}); }
inline bool RustStationStationAvailable() noexcept { return Station::CanAllocateItem(); }
inline void RustStationSetTown(BaseStation *st, const OpenTTDTownHandle *town) noexcept { st->town = const_cast<Town *>(reinterpret_cast<const Town *>(town)); }
inline void RustStationHaveRating(BaseStation *st, uint8_t owner) noexcept { OpenTTDBaseStationTown(st)->have_ratings.Set(CompanyID(owner)); }
inline void RustStationRailTriggers(BaseStation *st, const StationSpec *spec) noexcept { st->cached_anim_triggers.Set(spec->animation.triggers); }
inline void RustStationResetFacilities(BaseStation *st, uint8_t bits) noexcept { st->facilities.Reset(StationFacilities(bits)); }
inline void RustStationRoadTriggers(BaseStation *st, const RoadStopSpec *spec) noexcept { st->cached_roadstop_anim_triggers.Set(spec->animation.triggers); }
inline const OpenTTDTownHandle * RustStationStationTown(BaseStation *st) noexcept { return reinterpret_cast<const OpenTTDTownHandle *>(OpenTTDBaseStationTown(st)); }
inline void RustStationStationViewInvalidate(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_VIEW, st->index, data); }
inline void RustStationDeleteAirportStorage(BaseStation *st) noexcept { delete Station::From(st)->airport.psa; }
inline Industry *RustStationAssociatedIndustry(BaseStation *st) noexcept { return Station::From(st)->industry; }
inline void RustStationSetStationOwner(BaseStation *st, uint8_t owner) noexcept { st->owner = Owner(owner); }
inline void RustStationSetFacilities(BaseStation *st, uint8_t facilities) noexcept { st->facilities = StationFacilities(facilities); }
template <typename TStation>
inline int32_t RustStationBuildDate(TStation *st) noexcept { return st->build_date.base(); }
inline void RustStationSetBuildDate(BaseStation *st, int32_t date) noexcept { st->build_date = TimerGameCalendar::Date(date); }
inline void RustStationListInvalidation(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_LIST, st->owner, data); }
inline void RustStationClearName(BaseStation *st) noexcept { st->name.clear(); }
inline void RustStationSetFacilityBits(BaseStation *st, uint8_t bits) noexcept { st->facilities.Set(StationFacilities(bits)); }
inline void RustStationClearCachedName(BaseStation *st) noexcept { st->cached_name.clear(); }
inline void RustStationSetName(BaseStation *st, const uint8_t *text, size_t length) noexcept { st->name.assign(reinterpret_cast<const char *>(text), length); }
inline bool RustWaypointAvailable() noexcept { return Waypoint::CanAllocateItem(); }
inline uint32_t RustStationStringID(BaseStation *st) noexcept { return st->string_id; }
inline void RustStationSetStringID(BaseStation *st, uint32_t string) noexcept { st->string_id = string; }
inline void RustStationSetLocation(BaseStation *st, uint32_t tile) noexcept { st->xy = TileIndex{tile}; }
inline void RustWaypointSetRoad(BaseStation *st) noexcept { SetBit(Waypoint::From(st)->waypoint_flags, WPF_ROAD); }
inline void RustWaypointDefaultName(BaseStation *st) noexcept { MakeDefaultName(Waypoint::From(st)); }
inline void RustWaypointViewInvalidate(BaseStation *st) noexcept { InvalidateWindowData(WC_WAYPOINT_VIEW, st->index); }
inline void RustWaypointRoadAreaAdd(BaseStation *st, uint32_t tile) noexcept { Waypoint::From(st)->road_waypoint_area.Add(TileIndex{tile}); }
inline void RustMakeBuoy(uint32_t tile, uint16_t id, WaterClass water) noexcept { MakeBuoy(TileIndex{tile}, StationID(id), water); }
inline void RustMakeRailWaypoint(uint32_t tile, uint8_t owner, uint16_t id, uint8_t axis, uint8_t gfx, RailType rt) noexcept { MakeRailWaypoint(TileIndex{tile}, Owner(owner), StationID(id), static_cast<Axis>(axis), gfx, rt); }
/** Width/handle adapters for older typed service tables. */
template <class T> inline uint8_t RustStationOwnerByte(T *st) noexcept { return OpenTTDStationOwner(st).base(); }
template <class Handle> inline uint64_t RustStationLegacyCompatible(Handle, uint32_t a, uint32_t b) noexcept { return RustCompatibleStationTiles(a, b); }
inline bool RustStationRoadWaypoint(BaseStation *st) noexcept { return HasBit(Waypoint::From(st)->waypoint_flags, WPF_ROAD); }
inline void RustStationClearAirport(BaseStation *st) noexcept { Station::From(st)->airport.Clear(); }
inline uint32_t Aircraft_airport_tile(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationAirportTile(st);
}

inline uint32_t Aircraft_station_tile(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationLocation(st);
}

inline uint8_t Aircraft_rotation(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationAirportRotation(st);
}

inline uint8_t Aircraft_airport_type(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationAirportType(st);
}

inline uint8_t Aircraft_station_owner(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return OpenTTDStationOwner(st).base();
}

inline void RustStationIndexInsert(BaseStation *st) noexcept { _station_kdtree.Insert(st->index); }
inline void RustStationClosedWidget(BaseStation *st) noexcept { SetWindowWidgetDirty(WC_STATION_VIEW, st->index, WID_SV_CLOSE_AIRPORT); }
inline void RustStationCloseAirportInspect(BaseStation *st) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTS, RustBaseStationID(st)); }

inline uint16_t RustStationAirportWidth(const Station *station) noexcept { return station->airport.w; }
inline uint16_t RustStationAirportHeight(const Station *station) noexcept { return station->airport.h; }
inline uint8_t RustAirportDeltaZ(const AirportFTAClass *airport) noexcept { return airport->delta_z; }

/* The existing aircraft table retains its opaque pointers and return widths. */
inline uint8_t OpenTTDAirportDeltaZ(const void *airport) noexcept { return RustAirportDeltaZ(static_cast<const AirportFTAClass *>(airport)); }
inline void OpenTTDModifyStationRatingAround(uint32_t tile, uint8_t owner, int amount, uint radius) noexcept { ModifyStationRatingAround(TileIndex(tile), CompanyID(owner), amount, radius); }
inline bool RustMovementRailStation(uint32_t tile) noexcept { return IsRailStation(TileIndex{tile}); }
inline bool RustStationBuoy(uint32_t tile) noexcept { return IsBuoy(TileIndex{tile}); }
inline uint8_t RustStationTileRandomBits(uint32_t tile) noexcept { return GetStationTileRandomBits(TileIndex{tile}); }
template <typename TStation>
inline uint16_t RustStationRandomBits(TStation *st) noexcept { return st->random_bits; }
template <typename TStation>
inline uint8_t RustStationWaitingTriggers(TStation *st) noexcept { return st->waiting_random_triggers.base(); }
inline void RustStationSetWaitingTriggers(BaseStation *station, uint8_t triggers) noexcept { station->waiting_random_triggers = StationRandomTriggers(triggers); }
inline void RustStationSetRandomBits(BaseStation *station, uint16_t bits) noexcept { station->random_bits = bits; }
inline uint64_t RustStationCachedCargoTriggers(BaseStation *station) noexcept { return station->cached_cargo_triggers; }
#endif /* RUST_SERVICES_STATION_H */

