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
inline uint32_t RustCustomStationIndex(uint32_t tile) noexcept { return GetCustomStationSpecIndex(TileIndex{tile}); }
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
inline uint16_t RustBaseStationID(TStation *st) noexcept { return st->Record().id; }

inline uint16_t RustAirportMovingFlags(const AirportMovingData *moving) noexcept { return moving->flags.base(); }

template <class Handle> inline Handle RustStationGet(uint16_t id) noexcept { return Station::Get(StationID{id}); }
inline uint8_t RustAirportLayoutHeading(const AirportFTAClass *airport, uint32_t index) noexcept { return airport->layout[index].heading; }
inline uint8_t OpenTTDAirportLayoutHeading(const void *airport, uint8_t index) noexcept { return RustAirportLayoutHeading(static_cast<const AirportFTAClass *>(airport), index); }
inline uint8_t RustAirportFlags(const AirportFTAClass *airport) noexcept { return airport->flags.base(); }
inline uint8_t OpenTTDAirportFlags(const void *airport) noexcept { return RustAirportFlags(static_cast<const AirportFTAClass *>(airport)); }
inline bool RustStationConvert8bitBoolean(const StationSpec *spec, uint16_t id, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(RustSpecGRFFile(spec), static_cast<CallbackID>(id), value); }
inline bool RustRoadStopConvert8bitBoolean(const RoadStopSpec *spec, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(RustSpecGRFFile(spec), CBID_STATION_AVAILABILITY, value); }
inline void RustStationCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_STATIONS, tile); }
inline void RustRoadStopCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_ROADSTOPS, tile); }
inline void RustAirportTileCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTTILES, tile); }
inline void RustStationMarkTilesDirty(BaseStation *st, bool cargo) noexcept { Station::From(st)->MarkTilesDirty(cargo); }
inline const OpenTTDStation *RustStation(const Station *s) noexcept { return reinterpret_cast<const OpenTTDStation *>(s); }
inline const Station *StationOf(const OpenTTDStation *s) noexcept { return reinterpret_cast<const Station *>(s); }
/* GetSpec preserves the native dummy-airport fallback. Resolver-specific tile
 * gates remain with their callers; all handles are borrowed synchronously. */
template <typename Pointer> inline Pointer RustAirportSpec(uint16_t type) noexcept { return AirportSpec::RustRawSpec(type); }
template <typename Pointer> inline Pointer RustAirportTileSpec(uint8_t gfx) noexcept { return const_cast<AirportTileSpec *>(AirportTileSpec::Get(gfx)); }
inline const AirportFTAClass *RustAirportSpecFTA(const AirportSpec *spec) noexcept { return spec->fsm; }
inline TTDPAirportType RustAirportTTDType(const AirportSpec *spec) noexcept { return spec->ttd_airport_type; }
inline uint32_t RustStationAirportTile(const BaseStation *s) noexcept { return Station::From(s)->GetAirport().GetTile().base(); }
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
inline void RustStationStationViewInvalidate(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_VIEW, st->index, data); }
inline void RustStationDeleteAirportStorage(BaseStation *st) noexcept { delete Station::From(st)->Record().airport.psa; }
inline BaseStation *RustIndustryNeutralBaseStation(Industry *industry) noexcept { return RustIndustryNeutralStation(industry); }
inline void RustStationListInvalidation(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_LIST, st->GetOwner(), data); }
inline bool RustWaypointAvailable() noexcept { return Waypoint::CanAllocateItem(); }
inline void RustWaypointDefaultName(BaseStation *st) noexcept { MakeDefaultName(Waypoint::From(st)); }
inline void RustWaypointViewInvalidate(BaseStation *st) noexcept { InvalidateWindowData(WC_WAYPOINT_VIEW, st->index); }
inline void RustMakeBuoy(uint32_t tile, uint16_t id, WaterClass water) noexcept { MakeBuoy(TileIndex{tile}, StationID(id), water); }
inline void RustMakeRailWaypoint(uint32_t tile, uint8_t owner, uint16_t id, uint8_t axis, uint8_t gfx, RailType rt) noexcept { MakeRailWaypoint(TileIndex{tile}, Owner(owner), StationID(id), static_cast<Axis>(axis), gfx, rt); }
/** Width/handle adapters for older typed service tables. */
template <class Handle> inline uint64_t RustStationLegacyCompatible(Handle, uint32_t a, uint32_t b) noexcept { return RustCompatibleStationTiles(a, b); }




inline void RustStationClosedWidget(BaseStation *st) noexcept { SetWindowWidgetDirty(WC_STATION_VIEW, st->index, WID_SV_CLOSE_AIRPORT); }
inline void RustStationCloseAirportInspect(BaseStation *st) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTS, RustBaseStationID(st)); }

inline uint8_t RustAirportDeltaZ(const AirportFTAClass *airport) noexcept { return airport->delta_z; }

/* The existing aircraft table retains its opaque pointers and return widths. */
inline uint8_t OpenTTDAirportDeltaZ(const void *airport) noexcept { return RustAirportDeltaZ(static_cast<const AirportFTAClass *>(airport)); }
inline uint8_t RustStationTileRandomBits(uint32_t tile) noexcept { return GetStationTileRandomBits(TileIndex{tile}); }
inline bool RustStationSignValid(BaseStation *st) noexcept { return st->sign.kdtree_valid; }
inline void RustStationListClassesInvalidate() noexcept { InvalidateWindowClassesData(WC_STATION_LIST, 0); }
inline uint16_t RustAirportSpecMaintenance(const AirportSpec *spec) noexcept { return spec->maintenance_cost; }
#endif /* RUST_SERVICES_STATION_H */
