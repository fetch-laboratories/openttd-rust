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
#include "industry_shell_services.h"
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
inline size_t RustStationTileFlagCount(const StationSpec *spec) noexcept { return spec->tileflags.size(); }
template <typename Spec>
inline size_t RustStationBridgeInfoCount(const Spec *spec) noexcept { return spec->bridgeable_info.size(); }
inline CargoTypes RustStationCargoTriggers(const StationSpec *spec) noexcept { return spec->cargo_triggers; }
inline uint16_t RustStationAnimationTriggers(const StationSpec *spec) noexcept { return spec->animation.triggers.base(); }
inline OpenTTDStationSpecTriggers RustStationSpecTriggers(const StationSpec *spec) noexcept { return {RustStationAnimationTriggers(spec), RustStationCargoTriggers(spec)}; }
inline size_t RustStationSpecCount(const BaseStation *st) noexcept { return st->speclist.size(); }
inline const StationSpec *RustStationSlotSpec(const BaseStation *st, size_t index) noexcept { return st->speclist[index].spec; }
inline uint32_t RustStationSlotGRFID(const BaseStation *st, size_t index) noexcept { return st->speclist[index].grfid; }
inline uint16_t RustStationSlotLocalID(const BaseStation *st, size_t index) noexcept { return st->speclist[index].localidx; }
inline OpenTTDStationSpecVacancy RustStationSlotVacancy(const BaseStation *st, size_t index) noexcept { return {RustStationSlotSpec(st, index), RustStationSlotGRFID(st, index)}; }
inline OpenTTDGRFClassIdentity RustStationSlotKey(const BaseStation *st, size_t index) noexcept { return {RustStationSlotGRFID(st, index), RustStationSlotLocalID(st, index)}; }
inline void RustStationSpecResize(BaseStation *st, size_t size) noexcept { st->speclist.resize(size); }
inline void RustStationSpecClear(BaseStation *st) noexcept { st->speclist.clear(); }
inline void RustStationRebindSpec(BaseStation *st, size_t index, const StationSpec *spec) noexcept { st->speclist[index].spec = spec; }
inline void RustStationSetSpecSlot(BaseStation *st, uint8_t index, const StationSpec *spec, uint32_t grfid, uint16_t local_id) noexcept { RustStationRebindSpec(st, index, spec); auto &slot = st->speclist[index]; slot.grfid = grfid; slot.localidx = local_id; }
inline uint32_t RustCustomStationIndex(uint32_t tile) noexcept { return GetCustomStationSpecIndex(TileIndex{tile}); }
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
inline void RustRoadStopRebindSpec(BaseStation *st, size_t index, const RoadStopSpec *spec) noexcept { st->roadstop_speclist[index].spec = spec; }
inline void RustRoadStopSetSpecSlot(BaseStation *st, uint8_t index, const RoadStopSpec *spec, uint32_t grfid, uint16_t local_id) noexcept { RustRoadStopRebindSpec(st, index, spec); auto &slot = st->roadstop_speclist[index]; slot.grfid = grfid; slot.localidx = local_id; }
inline bool RustCustomRoadStopSpecIndex(uint32_t tile) noexcept { return IsCustomRoadStopSpecIndex(TileIndex(tile)); }
inline uint32_t RustCustomRoadStopIndex(uint32_t tile) noexcept { return GetCustomRoadStopSpecIndex(TileIndex(tile)); }
inline CargoTypes RustRoadStopCargoTriggers(const RoadStopSpec *spec) noexcept { return spec->cargo_triggers; }
inline uint16_t RustRoadStopAnimationTriggers(const RoadStopSpec *spec) noexcept { return spec->animation.triggers.base(); }
inline OpenTTDRoadStopSpecTriggers RustRoadStopSpecTriggers(const RoadStopSpec *spec) noexcept { return {RustRoadStopAnimationTriggers(spec), RustRoadStopCargoTriggers(spec)}; }

/* Borrowed native station shells; retain strict tile/type/index preconditions.
 * Const and mutable table receivers share each scalar's actual field read. */
inline BaseStation *RustBaseStationByTile(uint32_t tile) noexcept { return BaseStation::GetByTile(TileIndex(tile)); }
/* Bind directly: another service table may still be dynamically initializing. */
template <typename TStation>
inline uint16_t RustBaseStationID(TStation *st) noexcept { return st->index.base(); }
/** Read existing cargo data only; a missing GoodsEntryData stays unallocated. */
inline const OpenTTDCargoList *RustStationExistingCargoOwner(BaseStation *st, uint8_t cargo) noexcept { const auto &goods = Station::From(st)->GetGoods(cargo); return goods.HasData() ? goods.GetData().cargo.RustOwner() : nullptr; }

inline uint16_t RustAirportMovingFlags(const AirportMovingData *moving) noexcept { return moving->flags.base(); }

template <class Handle> inline Handle RustStationGet(uint16_t id) noexcept { return Station::Get(StationID{id}); }
inline Station *RustAnimationStation(BaseStation *st) noexcept { return Station::From(st); }
inline bool RustStationAirportMember(const Station *st, uint32_t tile) noexcept { return st->TileBelongsToAirport(TileIndex{tile}); }
inline size_t RustRoadStopTileDataCount(BaseStation *s) noexcept { return s->custom_roadstop_tile_data.size(); }
inline uint32_t RustRoadStopDataTile(BaseStation *s, size_t i) noexcept { return s->custom_roadstop_tile_data[i].tile.base(); }
inline const StationSpec *RustAnimationRailSpec(uint32_t tile) noexcept { return GetStationSpec(TileIndex{tile}); }
/** `&st->GetGoods(cargo).GetOrCreateData().cargo`'s Rust owner. */
inline OpenTTDCargoList *RustStationCargoOwner(OpenTTDStation *st, uint8_t cargo) noexcept { return static_cast<Station *>(st)->GetGoods(cargo).GetOrCreateData().cargo.RustOwner(); }
inline uint8_t RustAirportLayoutHeading(const AirportFTAClass *airport, uint32_t index) noexcept { return airport->layout[index].heading; }
inline uint8_t OpenTTDAirportLayoutHeading(const void *airport, uint8_t index) noexcept { return RustAirportLayoutHeading(static_cast<const AirportFTAClass *>(airport), index); }
inline uint8_t RustAirportFlags(const AirportFTAClass *airport) noexcept { return airport->flags.base(); }
inline uint8_t OpenTTDAirportFlags(const void *airport) noexcept { return RustAirportFlags(static_cast<const AirportFTAClass *>(airport)); }
inline bool RustSelectedAirportHasHangar(const BaseStation *st, const AirportSpec *spec) noexcept { return Station::From(st)->GetAirport().HasHangar(spec); }
inline TileIndex RustStationHangarTile(const BaseStation *st, uint32_t index, const AirportSpec *spec) noexcept { return Station::From(st)->GetAirport().GetHangarTile(index, spec); }
inline uint32_t RustSelectedHangarTile(const BaseStation *st, uint32_t index, const AirportSpec *spec) noexcept { return RustStationHangarTile(st, index, spec).base(); }
inline bool RustStationConvert8bitBoolean(const StationSpec *spec, uint16_t id, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(RustSpecGRFFile(spec), static_cast<CallbackID>(id), value); }
inline bool RustRoadStopConvert8bitBoolean(const RoadStopSpec *spec, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(RustSpecGRFFile(spec), CBID_STATION_AVAILABILITY, value); }
inline void RustStationCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_STATIONS, tile); }
inline void RustRoadStopCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_ROADSTOPS, tile); }
inline void RustAirportTileCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTTILES, tile); }
inline void RustStationMarkTilesDirty(BaseStation *st, bool cargo) noexcept { Station::From(st)->MarkTilesDirty(cargo); }
inline const OpenTTDStation *RustStation(const Station *s) noexcept { return reinterpret_cast<const OpenTTDStation *>(s); }
inline Station *RustStationByTile(uint32_t tile) noexcept { return Station::GetByTile(TileIndex{tile}); }
inline bool RustWaypointSingleTile(uint16_t id) noexcept { return Waypoint::Get(StationID{id})->IsSingleTile(); }
inline const OpenTTDStation *OpenTTDStationByTile(uint32_t tile) noexcept { return RustStation(RustStationByTile(tile)); }
inline const Station *StationOf(const OpenTTDStation *s) noexcept { return reinterpret_cast<const Station *>(s); }
/* GetSpec preserves the native dummy-airport fallback. Resolver-specific tile
 * gates remain with their callers; all handles are borrowed synchronously. */
template <typename Pointer> inline Pointer RustAirportSpec(uint16_t type) noexcept { return AirportSpec::RustRawSpec(type); }
template <typename Pointer> inline Pointer RustAirportTileSpec(uint8_t gfx) noexcept { return const_cast<AirportTileSpec *>(AirportTileSpec::Get(gfx)); }
inline const AirportFTAClass *RustAirportSpecFTA(const AirportSpec *spec) noexcept { return spec->fsm; }
inline TTDPAirportType RustAirportTTDType(const AirportSpec *spec) noexcept { return spec->ttd_airport_type; }
inline uint32_t RustStationTrainTile(const BaseStation *s) noexcept { return s->GetTrainStation().tile.base(); }
inline uint32_t RustStationShipTile(const BaseStation *s) noexcept { return Station::From(s)->GetShipStation().tile.base(); }
inline uint32_t RustStationAirportTile(const BaseStation *s) noexcept { return Station::From(s)->GetAirport().GetTile().base(); }
inline uint32_t RustStationLocation(const BaseStation *s) noexcept { return s->GetXY().base(); }
inline Town *OpenTTDBaseStationTown(const BaseStation *st) noexcept { return st->Record().town; }
inline Station *OpenTTDNextStation(uint32_t from) noexcept { auto range = Station::Iterate(from); return range.empty() ? nullptr : *range.begin(); }
template <class Handle> inline Handle RustStationIfValid(uint16_t id) noexcept { return Station::GetIfValid(StationID{id}); }
/** The canonical Rust service owner of a station or waypoint. */
template <class TStation> inline OpenTTDStationRecord *RustStationServiceOwner(TStation *st) noexcept { return const_cast<OpenTTDStationRecord *>(&st->Record()); }
inline bool RustStationReserved(uint32_t tile) noexcept { return HasStationReservation(TileIndex{tile}); }
inline void RustSetRailStationReservation(uint32_t tile, bool reserved) noexcept { SetRailStationReservation(TileIndex{tile}, reserved); }
inline bool RustCompatibleStationTiles(uint32_t a, uint32_t b) noexcept { return IsCompatibleTrainStationTile(TileIndex{a}, TileIndex{b}); }
inline bool RustStationStationAvailable() noexcept { return Station::CanAllocateItem(); }
inline void RustStationHaveRating(BaseStation *st, uint8_t owner) noexcept { OpenTTDBaseStationTown(st)->have_ratings.Set(CompanyID(owner)); }
inline void RustStationRailTriggers(BaseStation *st, const StationSpec *spec) noexcept { st->SetCachedAnimTriggers(st->GetCachedAnimTriggers().Set(spec->animation.triggers)); }
inline void RustStationRoadTriggers(BaseStation *st, const RoadStopSpec *spec) noexcept { st->SetCachedRoadStopAnimTriggers(st->GetCachedRoadStopAnimTriggers().Set(spec->animation.triggers)); }
inline void RustStationStationViewInvalidate(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_VIEW, st->index, data); }
inline void RustStationDeleteAirportStorage(BaseStation *st) noexcept { delete Station::From(st)->Record().airport.psa; }
inline BaseStation *RustIndustryNeutralBaseStation(Industry *industry) noexcept { return RustIndustryNeutralStation(industry); }
inline void RustStationListInvalidation(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_LIST, st->GetOwner(), data); }
inline void RustStationClearName(BaseStation *st) noexcept { st->name.clear(); }
inline void RustStationClearCachedName(BaseStation *st) noexcept { st->cached_name.clear(); }
inline void RustStationSetName(BaseStation *st, const uint8_t *text, size_t length) noexcept { st->name.assign(reinterpret_cast<const char *>(text), length); }
inline bool RustWaypointAvailable() noexcept { return Waypoint::CanAllocateItem(); }
inline void RustWaypointDefaultName(BaseStation *st) noexcept { MakeDefaultName(Waypoint::From(st)); }
inline void RustWaypointViewInvalidate(BaseStation *st) noexcept { InvalidateWindowData(WC_WAYPOINT_VIEW, st->index); }
inline void RustMakeBuoy(uint32_t tile, uint16_t id, WaterClass water) noexcept { MakeBuoy(TileIndex{tile}, StationID(id), water); }
inline void RustMakeRailWaypoint(uint32_t tile, uint8_t owner, uint16_t id, uint8_t axis, uint8_t gfx, RailType rt) noexcept { MakeRailWaypoint(TileIndex{tile}, Owner(owner), StationID(id), static_cast<Axis>(axis), gfx, rt); }
/** Width/handle adapters for older typed service tables. */
template <class Handle> inline uint64_t RustStationLegacyCompatible(Handle, uint32_t a, uint32_t b) noexcept { return RustCompatibleStationTiles(a, b); }




inline void RustStationClosedWidget(BaseStation *st) noexcept { SetWindowWidgetDirty(WC_STATION_VIEW, st->index, WID_SV_CLOSE_AIRPORT); }
inline void RustStationCloseAirportInspect(BaseStation *st) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTS, RustBaseStationID(st)); }

inline uint16_t RustStationAirportWidth(const Station *station) noexcept { return station->GetAirport().Record().area.w; }
inline uint16_t RustStationAirportHeight(const Station *station) noexcept { return station->GetAirport().Record().area.h; }
inline uint8_t RustAirportDeltaZ(const AirportFTAClass *airport) noexcept { return airport->delta_z; }

/* The existing aircraft table retains its opaque pointers and return widths. */
inline uint8_t OpenTTDAirportDeltaZ(const void *airport) noexcept { return RustAirportDeltaZ(static_cast<const AirportFTAClass *>(airport)); }
inline bool RustMovementRailStation(uint32_t tile) noexcept { return IsRailStation(TileIndex{tile}); }
inline bool RustStationBuoy(uint32_t tile) noexcept { return IsBuoy(TileIndex{tile}); }
inline uint8_t RustStationTileRandomBits(uint32_t tile) noexcept { return GetStationTileRandomBits(TileIndex{tile}); }
inline bool RustStationSignValid(BaseStation *st) noexcept { return st->sign.kdtree_valid; }
inline void RustStationListClassesInvalidate() noexcept { InvalidateWindowClassesData(WC_STATION_LIST, 0); }
inline void RustStationSetIndustryType(BaseStation *st, uint8_t type) noexcept { Station::From(st)->Record().indtype = type; }
inline uint16_t RustAirportSpecMaintenance(const AirportSpec *spec) noexcept { return spec->maintenance_cost; }
inline const OpenTTDGoodsEntry *RustStationGoodsEntry(BaseStation *st, uint8_t cargo) noexcept { return &Station::From(st)->Record().goods[cargo]; }
#endif /* RUST_SERVICES_STATION_H */
