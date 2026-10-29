/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file services_rail.h Canonical inline rail-map reads for native consumers. */
#ifndef RUST_SERVICES_RAIL_H
#define RUST_SERVICES_RAIL_H
#include <type_traits>
#include "signal_ffi.h"
#include "environment_services.h"
#include "../rail_map.h"
#include "../rail.h"
#include "../road_map.h"
#include "../station_map.h"
#include "../tunnelbridge_map.h"
#include "../tunnel_map.h"
#include "../landscape.h"
#include "../water_map.h"
#include "../depot_map.h"
#include "../tunnelbridge.h"
#include "../pbs.h"
#include "train_reservation_ffi.h"
#include "../pathfinder/follow_track.hpp"

inline OpenTTDSignalContext GetRustSignalContext() noexcept
{
	return {&GetRustSharedServices(), &GetRustRailMapServices(), &GetRustSignalServices(), OpenTTDMapLogX()};
}

/* Preserve native accessor types and assertion policy. The FFI tables adapt
 * only result widths; native consumers inline these same reads. */
template <typename EngineType>
inline bool RustRailCompatible(EngineType engine, RailType tile) noexcept { return IsCompatibleRail(engine, tile); }
template <typename EngineType>
inline bool RustRailPowered(EngineType engine, RailType tile) noexcept { return HasPowerOnRail(engine, tile); }
inline bool RustRailPoweredMask(uint64_t types, RailType type) noexcept { return RustRailPowered(RailTypes(types), type); }
inline bool RustRailCompatibleLegacy(uint8_t engine, uint8_t tile) noexcept { return RustRailCompatible(static_cast<RailType>(engine), static_cast<RailType>(tile)); }
inline bool RustRailPoweredLegacy(uint8_t engine, uint8_t tile) noexcept { return RustRailPowered(static_cast<RailType>(engine), static_cast<RailType>(tile)); }
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
inline Axis RustRailStationAxis(uint32_t tile) noexcept { return GetRailStationAxis(TileIndex(tile)); }
inline Axis RustCrossingRoadAxis(uint32_t tile) noexcept { return GetCrossingRoadAxis(TileIndex(tile)); }
inline TransportType RustTunnelBridgeTransport(uint32_t tile) noexcept { return GetTunnelBridgeTransportType(TileIndex(tile)); }
inline uint32_t RustOtherBridgeEnd(uint32_t tile) noexcept { return GetOtherBridgeEnd(TileIndex{tile}).base(); }
inline uint32_t RustOtherTunnelBridgeEnd(uint32_t tile) noexcept { return GetOtherTunnelBridgeEnd(TileIndex(tile)).base(); }

/* Rail command, depot and crossing leaves (rail_cmd.rs). */
inline RailType RustInvalidRailType() noexcept { return INVALID_RAILTYPE; }
inline VehicleAccelerationModel RustRailAccelerationType(RailType type) noexcept { return GetRailTypeInfo(type)->acceleration_type; }
inline uint16_t RustRailMaxSpeed(RailType type) noexcept { return GetRailTypeInfo(type)->max_speed; }
inline void RustSetRailCostMultiplier(RailTypeInfo *info, uint16_t value) noexcept { info->cost_multiplier = value; }
inline uint16_t RustRailCostMultiplier(RailType type) noexcept { return GetRailTypeInfo(type)->cost_multiplier; }
inline RailType RustRailType(uint32_t tile) noexcept { return GetRailType(TileIndex(tile)); }
inline void RustSetRailType(uint32_t tile, uint8_t rt) noexcept { SetRailType(TileIndex(tile), static_cast<RailType>(rt)); }
inline RailTileType RustRailTileType(uint32_t tile) noexcept { return GetRailTileType(TileIndex(tile)); }
inline RailGroundType RustRailGround(uint32_t tile) noexcept { return GetRailGroundType(TileIndex(tile)); }
inline void RustSetRailGroundTyped(uint32_t tile, RailGroundType ground) noexcept { SetRailGroundType(TileIndex(tile), ground); }
inline void RustSetTrackBits(uint32_t tile, uint8_t bits) noexcept { SetTrackBits(TileIndex(tile), static_cast<TrackBits>(bits)); }
inline void RustSetPresentSignals(uint32_t tile, uint8_t signals) noexcept { SetPresentSignals(TileIndex(tile), signals); }
inline uint RustSignalStates(uint32_t tile) noexcept { return GetSignalStates(TileIndex(tile)); }
inline void RustSetSignalStates(uint32_t tile, uint8_t states) noexcept { SetSignalStates(TileIndex(tile), states); }
inline void RustSetHasSignals(uint32_t tile, bool signals) noexcept { SetHasSignals(TileIndex(tile), signals); }
inline void RustSetSignalType(uint32_t tile, uint8_t track, uint8_t type) noexcept { SetSignalType(TileIndex(tile), static_cast<Track>(track), static_cast<SignalType>(type)); }
inline SignalVariant RustSignalVariant(uint32_t tile, uint8_t track) noexcept { return GetSignalVariant(TileIndex(tile), static_cast<Track>(track)); }
inline void RustSetSignalVariant(uint32_t tile, uint8_t track, uint8_t variant) noexcept { SetSignalVariant(TileIndex(tile), static_cast<Track>(track), static_cast<SignalVariant>(variant)); }
inline void RustCycleSignalSide(uint32_t tile, uint8_t track) noexcept { CycleSignalSide(TileIndex(tile), static_cast<Track>(track)); }
inline uint64_t RustAllCompatibleRailTypes(uint64_t types) noexcept { return GetAllCompatibleRailTypes(RailTypes{types}).base(); }
inline RailType RustTileRailType(uint32_t tile) noexcept { return GetTileRailType(TileIndex{tile}); }
inline bool RustRail90Disallowed(uint8_t a, uint8_t b, bool def) noexcept { return Rail90DegTurnDisallowed(static_cast<RailType>(a), static_cast<RailType>(b), def); }
inline bool RustDefaultRail90Disallowed(uint8_t a, uint8_t b) noexcept { return RustRail90Disallowed(a, b, _settings_game.pf.forbid_90_deg); }
inline bool RustTileRail90Disallowed(uint32_t a, uint32_t b) noexcept { return RustDefaultRail90Disallowed(RustTileRailType(a), RustTileRailType(b)); }
inline bool RustOnewaySignalBlocking(uint32_t tile, uint8_t td) noexcept { return HasOnewaySignalBlockingTrackdir(TileIndex{tile}, static_cast<Trackdir>(td)); }
inline bool RustHasPbsSignal(uint32_t tile, uint8_t td) noexcept { return HasPbsSignalOnTrackdir(TileIndex{tile}, static_cast<Trackdir>(td)); }
inline bool RustShowTrackReservation() noexcept { return _settings_client.gui.show_track_reservation; }
inline TrackBits RustRailReservation(uint32_t tile) noexcept { return GetRailReservationTrackBits(TileIndex(tile)); }
inline void RustSetTrackReservation(uint32_t tile, uint8_t bits) noexcept { SetTrackReservation(TileIndex(tile), static_cast<TrackBits>(bits)); }
inline bool RustDepotReserved(uint32_t tile) noexcept { return HasDepotReservation(TileIndex(tile)); }
inline void RustSetDepotReservation(uint32_t tile, bool reserved) noexcept { SetDepotReservation(TileIndex{tile}, reserved); }
inline void RustSetDepotExit(uint32_t tile, uint8_t dir) noexcept { SetRailDepotExitDirection(TileIndex(tile), static_cast<DiagDirection>(dir)); }
inline void RustMakeRailNormal(uint32_t tile, uint8_t owner, uint8_t bits, uint8_t rt) noexcept { MakeRailNormal(TileIndex(tile), Owner(owner), static_cast<TrackBits>(bits), static_cast<RailType>(rt)); }
inline void RustMakeRailDepot(uint32_t tile, uint8_t owner, uint16_t depot, uint8_t dir, uint8_t rt) noexcept { MakeRailDepot(TileIndex(tile), Owner(owner), DepotID(depot), static_cast<DiagDirection>(dir), static_cast<RailType>(rt)); }
inline TrackBits RustCrossingRailBits(uint32_t tile) noexcept { return GetCrossingRailBits(TileIndex(tile)); }
inline bool RustHasStationRail(uint32_t tile) noexcept { return HasStationRail(TileIndex(tile)); }
inline bool RustIsRailStation(uint32_t tile) noexcept { return IsRailStationTile(TileIndex(tile)); }
inline bool RustStationBlocked(uint32_t tile) noexcept { return IsStationTileBlocked(TileIndex(tile)); }
inline Track RustRailStationTrack(uint32_t tile) noexcept { return GetRailStationTrack(TileIndex(tile)); }
inline bool RustWormholeReserved(uint32_t tile) noexcept { return HasTunnelBridgeReservation(TileIndex(tile)); }
inline void RustSetTunnelBridgeReservation(uint32_t tile, bool reserved) noexcept { SetTunnelBridgeReservation(TileIndex{tile}, reserved); }
inline void RustSetTileOwner(uint32_t tile, uint8_t owner) noexcept { SetTileOwner(TileIndex(tile), Owner(owner)); }

/* Shared signature adapters preserve existing legacy callback ABIs. Query is
 * compile-time and inline: each entry makes the same native query. */
template <auto Query, typename First, typename Second>
auto OpenTTDEnumMapQuery(std::underlying_type_t<First> first, std::underlying_type_t<Second> second) noexcept
{
	return Query(static_cast<First>(first), static_cast<Second>(second));
}
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
inline bool RustIsTunnel(uint32_t tile) noexcept { return IsTunnel(TileIndex{tile}); }
inline bool RustMovementPlainRail(uint32_t tile) noexcept { return IsPlainRail(TileIndex{tile}); }
inline bool RustRailDepotTile(uint32_t tile) noexcept { return IsRailDepotTile(TileIndex{tile}); }
static_assert(TRACK_END == 6);
inline bool RustRailOnewaySignal(uint32_t tile, uint8_t track) noexcept { return IsOnewaySignal(TileIndex{tile}, static_cast<Track>(track)); }
inline bool RustCrossingBarred(uint32_t tile) noexcept { return IsCrossingBarred(TileIndex{tile}); }
#endif /* RUST_SERVICES_RAIL_H */
