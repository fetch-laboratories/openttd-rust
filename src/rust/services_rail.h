/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_rail.h Canonical inline rail-map reads for native consumers. */
#ifndef RUST_SERVICES_RAIL_H
#define RUST_SERVICES_RAIL_H
#include "../rail_map.h"
#include "../road_map.h"
#include "../station_map.h"
#include "../tunnelbridge_map.h"
#include "../landscape.h"
#include "../water_map.h"
#include "../depot_map.h"
#include "../tunnelbridge.h"
#include "../pbs.h"

/* Preserve native accessor types and assertion policy. The FFI tables adapt
 * only result widths; native consumers inline these same reads without another
 * service-table call or a second definition of the map query. */
inline bool RustRailIsDepot(uint32_t tile) noexcept { return IsRailDepot(TileIndex(tile)); }
inline DiagDirection RustRailDepotDirection(uint32_t tile) noexcept { return GetRailDepotDirection(TileIndex(tile)); }
inline TrackBits RustRailTracks(uint32_t tile) noexcept { return GetTrackBits(TileIndex(tile)); }
inline bool RustRailHasSignals(uint32_t tile) noexcept { return HasSignals(TileIndex(tile)); }
inline uint RustRailPresentSignals(uint32_t tile) noexcept { return GetPresentSignals(TileIndex(tile)); }
inline bool RustRailHasSignalTrack(uint32_t tile, uint8_t track) noexcept { return HasSignalOnTrack(TileIndex(tile), static_cast<Track>(track)); }
inline SignalType RustRailSignalType(uint32_t tile, uint8_t track) noexcept { return GetSignalType(TileIndex(tile), static_cast<Track>(track)); }
inline bool RustRailHasSignalTrackdir(uint32_t tile, uint8_t td) noexcept { return HasSignalOnTrackdir(TileIndex(tile), static_cast<Trackdir>(td)); }
inline SignalState RustRailSignalState(uint32_t tile, uint8_t td) noexcept { return GetSignalStateByTrackdir(TileIndex(tile), static_cast<Trackdir>(td)); }
inline void RustRailSetSignalState(uint32_t tile, uint8_t td, uint8_t state) noexcept { SetSignalStateByTrackdir(TileIndex(tile), static_cast<Trackdir>(td), static_cast<SignalState>(state)); }
inline bool RustRailSignalIsGreen(uint32_t tile, uint8_t td) noexcept { return RustRailSignalState(tile, td) == SIGNAL_STATE_GREEN; }
inline void RustRailSetSignalGreen(uint32_t tile, uint8_t td, uint8_t green) noexcept { RustRailSetSignalState(tile, td, green != 0 ? SIGNAL_STATE_GREEN : SIGNAL_STATE_RED); }
inline TrackStatus RustRailTrackStatus(uint32_t tile, uint8_t dir) noexcept { return GetTileTrackStatus(TileIndex(tile), TRANSPORT_RAIL, 0, static_cast<DiagDirection>(dir)); }
inline Axis RustRailStationAxis(uint32_t tile) noexcept { return GetRailStationAxis(TileIndex(tile)); }
inline Axis RustCrossingRoadAxis(uint32_t tile) noexcept { return GetCrossingRoadAxis(TileIndex(tile)); }
inline TransportType RustTunnelBridgeTransport(uint32_t tile) noexcept { return GetTunnelBridgeTransportType(TileIndex(tile)); }
inline uint32_t RustOtherTunnelBridgeEnd(uint32_t tile) noexcept { return GetOtherTunnelBridgeEnd(TileIndex(tile)).base(); }

/* Rail command, depot and crossing leaves (#265): one native definition each. */
inline RailType RustRailType(uint32_t tile) noexcept { return GetRailType(TileIndex(tile)); }
inline void RustSetRailType(uint32_t tile, uint8_t rt) noexcept { SetRailType(TileIndex(tile), static_cast<RailType>(rt)); }
inline RailTileType RustRailTileType(uint32_t tile) noexcept { return GetRailTileType(TileIndex(tile)); }
inline RailGroundType RustRailGround(uint32_t tile) noexcept { return GetRailGroundType(TileIndex(tile)); }
inline void RustSetRailGround(uint32_t tile, uint8_t ground) noexcept { SetRailGroundType(TileIndex(tile), static_cast<RailGroundType>(ground)); }
inline void RustSetTrackBits(uint32_t tile, uint8_t bits) noexcept { SetTrackBits(TileIndex(tile), static_cast<TrackBits>(bits)); }
inline void RustSetPresentSignals(uint32_t tile, uint8_t signals) noexcept { SetPresentSignals(TileIndex(tile), signals); }
inline uint RustSignalStates(uint32_t tile) noexcept { return GetSignalStates(TileIndex(tile)); }
inline void RustSetSignalStates(uint32_t tile, uint8_t states) noexcept { SetSignalStates(TileIndex(tile), states); }
inline void RustSetHasSignals(uint32_t tile, bool signals) noexcept { SetHasSignals(TileIndex(tile), signals); }
inline void RustSetSignalType(uint32_t tile, uint8_t track, uint8_t type) noexcept { SetSignalType(TileIndex(tile), static_cast<Track>(track), static_cast<SignalType>(type)); }
inline SignalVariant RustSignalVariant(uint32_t tile, uint8_t track) noexcept { return GetSignalVariant(TileIndex(tile), static_cast<Track>(track)); }
inline void RustSetSignalVariant(uint32_t tile, uint8_t track, uint8_t variant) noexcept { SetSignalVariant(TileIndex(tile), static_cast<Track>(track), static_cast<SignalVariant>(variant)); }
inline void RustCycleSignalSide(uint32_t tile, uint8_t track) noexcept { CycleSignalSide(TileIndex(tile), static_cast<Track>(track)); }
inline TrackBits RustReservedTracks(uint32_t tile) noexcept { return GetReservedTrackbits(TileIndex(tile)); }
inline TrackBits RustRailReservation(uint32_t tile) noexcept { return GetRailReservationTrackBits(TileIndex(tile)); }
inline void RustSetTrackReservation(uint32_t tile, uint8_t bits) noexcept { SetTrackReservation(TileIndex(tile), static_cast<TrackBits>(bits)); }
inline bool RustDepotReserved(uint32_t tile) noexcept { return HasDepotReservation(TileIndex(tile)); }
inline void RustSetDepotExit(uint32_t tile, uint8_t dir) noexcept { SetRailDepotExitDirection(TileIndex(tile), static_cast<DiagDirection>(dir)); }
inline void RustMakeRailNormal(uint32_t tile, uint8_t owner, uint8_t bits, uint8_t rt) noexcept { MakeRailNormal(TileIndex(tile), Owner(owner), static_cast<TrackBits>(bits), static_cast<RailType>(rt)); }
inline void RustMakeRailDepot(uint32_t tile, uint8_t owner, uint16_t depot, uint8_t dir, uint8_t rt) noexcept { MakeRailDepot(TileIndex(tile), Owner(owner), DepotID(depot), static_cast<DiagDirection>(dir), static_cast<RailType>(rt)); }
inline TrackBits RustCrossingRailBits(uint32_t tile) noexcept { return GetCrossingRailBits(TileIndex(tile)); }
inline bool RustHasStationRail(uint32_t tile) noexcept { return HasStationRail(TileIndex(tile)); }
inline bool RustIsRailStation(uint32_t tile) noexcept { return IsRailStationTile(TileIndex(tile)); }
inline bool RustStationBlocked(uint32_t tile) noexcept { return IsStationTileBlocked(TileIndex(tile)); }
inline Track RustRailStationTrack(uint32_t tile) noexcept { return GetRailStationTrack(TileIndex(tile)); }
inline bool RustWormholeReserved(uint32_t tile) noexcept { return HasTunnelBridgeReservation(TileIndex(tile)); }
inline void RustSetTileOwner(uint32_t tile, uint8_t owner) noexcept { SetTileOwner(TileIndex(tile), Owner(owner)); }

/* Shared signature adapters preserve existing legacy callback ABIs. Query is
 * compile-time and inline: each entry makes the same native query. */
template <typename Result, auto Query, typename... Args>
Result OpenTTDMapQuery(Args... args) noexcept
{
	return static_cast<Result>(Query(args...));
}
template <typename Result, auto Query, typename Handle, typename... Args>
Result OpenTTDLegacyMapQuery(Handle, Args... args) noexcept
{
	return static_cast<Result>(Query(args...));
}
template <typename Result, auto Query, typename Handle, typename... Args>
Result OpenTTDLegacyContextMapQuery(void *, Handle, Args... args) noexcept
{
	return static_cast<Result>(Query(args...));
}
#endif /* RUST_SERVICES_RAIL_H */
