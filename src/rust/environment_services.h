/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file environment_services.h Shared native world state for synchronous Rust callbacks. */
#ifndef RUST_ENVIRONMENT_SERVICES_H
#define RUST_ENVIRONMENT_SERVICES_H
#include "../genworld.h"
#include "../debug.h"
#include "../network/network.h"
#include "../town.h"
#include <utility>
#include "../map_func.h"
#include "../openttd.h"
#include "../progress.h"
#include "../settings_type.h"
#include "../landscape.h"

inline int32_t RustDesyncLevel() noexcept { return _debug_desync_level; }
inline bool RustNetworking() noexcept { return _networking; }
inline bool RustNetworkServer() noexcept { return _network_server; }
inline bool RustNetworkDedicated() noexcept { return _network_dedicated; }
inline bool RustNetworkAvailable() noexcept { return _network_available; }
inline bool RustExitGame() noexcept { return _exit_game; }
inline void RustSetExitGame() noexcept { _exit_game = true; }
inline bool RustHasModalProgress() noexcept { return HasModalProgress(); }
inline uint32_t OpenTTDMapLogY() noexcept { return Map::LogY(); }
inline uint32_t OpenTTDMapLogX() noexcept { return Map::LogX(); }
inline bool OpenTTDFreeformEdges() noexcept { return _settings_game.construction.freeform_edges; }
inline bool OpenTTDEditor() noexcept { return _game_mode == GM_EDITOR; }
inline bool OpenTTDGeneratingWorld() noexcept { return _generating_world; }
inline LandscapeType OpenTTDLandscape() noexcept { return _settings_game.game_creation.landscape; }
/* One byte conversion at the FFI boundary; the native service keeps its enum. */
inline uint8_t OpenTTDLandscapeValue() noexcept { return to_underlying(OpenTTDLandscape()); }
struct OpenTTDTownState;
struct OpenTTDTownRef;
struct OpenTTDTownHandle;
struct Town;
inline uint16_t RustTownAuthorityIndex(const OpenTTDTownHandle *town) noexcept { return reinterpret_cast<const Town *>(town)->index.base(); }
inline uint32_t OpenTTDTownTile(const OpenTTDTownHandle *town) noexcept { return reinterpret_cast<const Town *>(town)->xy.base(); }
Town *OpenTTDTownShellByID(uint32_t id) noexcept;
OpenTTDTownState *OpenTTDTownOwnerByID(uint32_t id) noexcept;
OpenTTDTownRef OpenTTDTownResolveByID(uint32_t id) noexcept;
inline uint OpenTTDMapScaleSize(uint n) noexcept { return Map::ScaleBySize(n); }
inline uint OpenTTDMapScaleLandProportion(uint n) noexcept { return Map::ScaleByLandProportion(n); }
enum ClearGround : uint8_t;
enum TreeGround : uint8_t;
inline bool RustIsTileFlat(uint32_t tile, int *height = nullptr) noexcept { return IsTileFlat(TileIndex{tile}, height); }
bool OpenTTDIsTileFlat(uint32_t tile) noexcept;
ClearGround OpenTTDClearGround(uint32_t tile) noexcept;
TreeGround OpenTTDTreeGround(uint32_t tile) noexcept;
inline bool OpenTTDExchangeGeneratingWorld(bool value) noexcept { return std::exchange(_generating_world, value); }
void OpenTTDAssertTownGenerationContext() noexcept;
inline uint32_t OpenTTDMapScaleSize1D(uint32_t n) noexcept { return Map::ScaleBySize1D(n); }
#endif /* RUST_ENVIRONMENT_SERVICES_H */
