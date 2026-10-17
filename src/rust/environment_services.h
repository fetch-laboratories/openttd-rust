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
#include <utility>
#include "../map_func.h"
#include "../openttd.h"
#include "../settings_type.h"
#include "../landscape.h"

inline uint32_t OpenTTDMapLogX() noexcept { return Map::LogX(); }
inline bool OpenTTDEditor() noexcept { return _game_mode == GM_EDITOR; }
inline bool OpenTTDGeneratingWorld() noexcept { return _generating_world; }
inline LandscapeType OpenTTDLandscape() noexcept { return _settings_game.game_creation.landscape; }
/* One byte conversion at the FFI boundary; the native service keeps its enum. */
inline uint8_t OpenTTDLandscapeValue() noexcept { return to_underlying(OpenTTDLandscape()); }
inline uint8_t OpenTTDHighestSnowLine() noexcept { return HighestSnowLine(); }
struct OpenTTDTownState;
struct OpenTTDTownRef;
OpenTTDTownRef OpenTTDTownResolveByID(uint32_t id) noexcept;
inline uint OpenTTDMapScaleSize(uint n) noexcept { return Map::ScaleBySize(n); }
inline uint OpenTTDMapScaleLandProportion(uint n) noexcept { return Map::ScaleByLandProportion(n); }
enum ClearGround : uint8_t;
enum TreeGround : uint8_t;
bool OpenTTDIsTileFlat(uint32_t tile) noexcept;
ClearGround OpenTTDClearGround(uint32_t tile) noexcept;
TreeGround OpenTTDTreeGround(uint32_t tile) noexcept;
inline bool OpenTTDExchangeGeneratingWorld(bool value) noexcept { return std::exchange(_generating_world, value); }
void OpenTTDAssertTownGenerationContext() noexcept;
#endif /* RUST_ENVIRONMENT_SERVICES_H */
