/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_station.h Canonical station fields and opaque handles for Rust ports. */
#ifndef RUST_SERVICES_STATION_H
#define RUST_SERVICES_STATION_H
#include "../station_base.h"
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
#include "../base_station_base.h"
#include "../station_map.h"
#include "../pbs.h"

inline const AirportFTAClass *RustAirportFTA(uint8_t type) noexcept { return GetAirport(type); }
inline const void *RustDummyAirportFTA() noexcept { return RustAirportFTA(AT_DUMMY); }
inline TileIndex RustStationHangarTile(const BaseStation *st, uint32_t index) noexcept { return Station::From(st)->airport.GetHangarTile(index); }
inline bool RustStationConvert8bitBoolean(const StationSpec *spec, uint16_t id, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(spec->grf_prop.grffile, static_cast<CallbackID>(id), value); }
inline bool RustRoadStopConvert8bitBoolean(const RoadStopSpec *spec, uint16_t value) noexcept { return OpenTTDConvert8bitBooleanCallback(spec->grf_prop.grffile, CBID_STATION_AVAILABILITY, value); }
inline void RustStationCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_STATIONS, tile); }
inline void RustRoadStopCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_ROADSTOPS, tile); }
inline void RustAirportTileCloseInspect(uint32_t tile) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTTILES, tile); }
inline uint32_t RustStationHangarTileValue(BaseStation *st, uint32_t index) noexcept { return RustStationHangarTile(st, index).base(); }
inline void RustStationRailCallbackError(const StationSpec *spec, uint16_t callback, uint16_t result) noexcept { RustUnknownCallbackResult(spec->grf_prop.grfid, callback, result); }
inline void RustStationMarkTilesDirty(BaseStation *st, bool cargo) noexcept { Station::From(st)->MarkTilesDirty(cargo); }
inline void RustStationAnimation(BaseStation *st, TileIndex tile, StationAnimationTrigger trigger, CargoType cargo = INVALID_CARGO) noexcept { TriggerStationAnimation(st, tile, trigger, cargo); }
inline void RustRoadStopAnimation(BaseStation *st, TileIndex tile, StationAnimationTrigger trigger, CargoType cargo = INVALID_CARGO) noexcept { TriggerRoadStopAnimation(st, tile, trigger, cargo); }
inline bool RustAirportTileAnimation(Station *st, TileIndex tile, AirportAnimationTrigger trigger) noexcept { return TriggerAirportTileAnimation(st, tile, trigger); }
inline const OpenTTDStation *RustStation(const Station *s) noexcept { return reinterpret_cast<const OpenTTDStation *>(s); }
inline const Station *StationOf(const OpenTTDStation *s) noexcept { return reinterpret_cast<const Station *>(s); }
inline uint32_t RustStationAirportTile(const BaseStation *s) noexcept { return Station::From(s)->airport.tile.base(); }
inline uint8_t RustStationAirportType(const BaseStation *s) noexcept { return Station::From(s)->airport.type; }
inline Direction RustStationAirportRotation(const BaseStation *s) noexcept { return Station::From(s)->airport.rotation; }
inline uint32_t RustStationLocation(const BaseStation *s) noexcept { return s->xy.base(); }
inline void RustStationRadius(uint32_t tile, uint32_t radius, void *context, void (*visit)(void *, BaseStation *)) noexcept
{
	ForAllStationsRadius(TileIndex{tile}, radius, [context, visit](Station *st) { visit(context, st); });
}
inline Owner OpenTTDStationOwner(const BaseStation *station) noexcept { return station->owner; }
const OpenTTDStationService *OpenTTDStationActivityOwner(const OpenTTDStation *station) noexcept;
inline bool RustStationHasAirport(const Station *s) noexcept { return s->facilities.Test(StationFacility::Airport); }
inline bool RustStationHasHangar(const Station *s) noexcept { return s->airport.HasHangar(); }
inline bool RustStationReserved(uint32_t tile) noexcept { return HasStationReservation(TileIndex{tile}); }
inline void RustStationPlatformReservation(uint32_t tile, uint8_t dir, bool reserved) noexcept { SetRailStationPlatformReservation(TileIndex{tile}, static_cast<DiagDirection>(dir), reserved); }
inline bool RustCompatibleStationTiles(uint32_t a, uint32_t b) noexcept { return IsCompatibleTrainStationTile(TileIndex{a}, TileIndex{b}); }
inline bool RustStationStationAvailable() noexcept { return Station::CanAllocateItem(); }
inline void RustStationSetTown(BaseStation *st, const OpenTTDTownHandle *town) noexcept { st->town = const_cast<Town *>(reinterpret_cast<const Town *>(town)); }
inline void RustStationHaveRating(BaseStation *st, uint8_t owner) noexcept { st->town->have_ratings.Set(CompanyID(owner)); }
inline void RustStationRailTriggers(BaseStation *st, const StationSpec *spec) noexcept { st->cached_anim_triggers.Set(spec->animation.triggers); }
inline void RustStationResetFacilities(BaseStation *st, uint8_t bits) noexcept { st->facilities.Reset(StationFacilities(bits)); }
inline void RustStationRoadTriggers(BaseStation *st, const RoadStopSpec *spec) noexcept { st->cached_roadstop_anim_triggers.Set(spec->animation.triggers); }
inline Town *OpenTTDBaseStationTown(const BaseStation *st) noexcept { return st->town; }
inline const OpenTTDTownHandle * RustStationStationTown(BaseStation *st) noexcept { return reinterpret_cast<const OpenTTDTownHandle *>(OpenTTDBaseStationTown(st)); }
inline void RustStationStationViewInvalidate(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_VIEW, st->index, data); }
inline void RustStationDeleteAirportStorage(BaseStation *st) noexcept { delete Station::From(st)->airport.psa; }
inline Industry *RustStationAssociatedIndustry(BaseStation *st) noexcept { return Station::From(st)->industry; }
inline void RustStationSetStationOwner(BaseStation *st, uint8_t owner) noexcept { st->owner = Owner(owner); }
inline void RustStationSetFacilities(BaseStation *st, uint8_t facilities) noexcept { st->facilities = StationFacilities(facilities); }
inline void RustStationSetBuildDate(BaseStation *st, int32_t date) noexcept { st->build_date = TimerGameCalendar::Date(date); }
inline void RustStationListInvalidation(BaseStation *st, int32_t data) noexcept { InvalidateWindowData(WC_STATION_LIST, st->owner, data); }
/** Width/handle adapters for older typed service tables. */
template <class T> inline uint8_t RustStationOwnerByte(T *st) noexcept { return OpenTTDStationOwner(st).base(); }
template <class Handle> inline void RustStationLegacyPlatformReservation(Handle, uint32_t tile, uint8_t dir, uint8_t reserved) noexcept { RustStationPlatformReservation(tile, dir, reserved != 0); }
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

inline bool Aircraft_has_hangar(const OpenTTDStation *s) noexcept
{
	const Station *st = StationOf(s);
	return RustStationHasHangar(st);
}

inline void RustStationIndexInsert(BaseStation *st) noexcept { _station_kdtree.Insert(st->index); }
inline void RustStationClosedWidget(BaseStation *st) noexcept { SetWindowWidgetDirty(WC_STATION_VIEW, st->index, WID_SV_CLOSE_AIRPORT); }
inline void RustStationCloseAirportInspect(BaseStation *st) noexcept { RustCloseNewGRFInspect(GSF_AIRPORTS, st->index.base()); }

inline uint16_t RustStationAirportWidth(const Station *station) noexcept { return station->airport.w; }
inline uint16_t RustStationAirportHeight(const Station *station) noexcept { return station->airport.h; }
inline const AirportFTAClass *RustStationAirportFTA(const Station *station) noexcept { return station->airport.GetFTA(); }
inline uint8_t RustAirportDeltaZ(const AirportFTAClass *airport) noexcept { return airport->delta_z; }

/* The existing aircraft table retains its opaque pointers and return widths. */
inline const void *OpenTTDStationAirportFTA(const OpenTTDStation *station) noexcept { return RustStationAirportFTA(StationOf(station)); }
inline uint8_t OpenTTDAirportDeltaZ(const void *airport) noexcept { return RustAirportDeltaZ(static_cast<const AirportFTAClass *>(airport)); }
inline void OpenTTDModifyStationRatingAround(uint32_t tile, uint8_t owner, int amount, uint radius) noexcept { ModifyStationRatingAround(TileIndex(tile), CompanyID(owner), amount, radius); }
#endif /* RUST_SERVICES_STATION_H */
