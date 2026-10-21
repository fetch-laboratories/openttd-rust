/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file settings_services.h Shared native setting definitions consumed by game ports. */
#ifndef RUST_SETTINGS_SERVICES_H
#define RUST_SETTINGS_SERVICES_H
#include "services_road.h"
#include "../settings_type.h"
#include "../road.h"
#include "../autoslope.h"

inline uint8_t RustRoadAccelerationModel() noexcept { return _settings_game.vehicle.roadveh_acceleration_model; }
inline uint8_t RustRoadSide() noexcept { return _settings_game.vehicle.road_side; }
inline bool RustRoadQueue() noexcept { return _settings_game.pf.roadveh_queue; }
inline uint32_t RustMaximumDepotPenalty() noexcept { return _settings_game.pf.yapf.maximum_go_to_depot_penalty; }
inline uint8_t RustPlaneSpeed() noexcept { return _settings_game.vehicle.plane_speed; }
inline uint8_t RustPlaneCrashes() noexcept { return _settings_game.vehicle.plane_crashes; }
inline bool RustServiceAtHelipad() noexcept { return _settings_game.order.serviceathelipad; }
inline bool RustDisasterSound() noexcept { return _settings_client.sound.disaster; }
inline bool RustNeverExpireVehicles() noexcept { return _settings_game.vehicle.never_expire_vehicles; }
inline uint16_t RustTrainLimit() noexcept { return _settings_game.vehicle.max_trains; }
inline uint16_t RustRoadVehicleLimit() noexcept { return _settings_game.vehicle.max_roadveh; }
inline uint16_t RustShipLimit() noexcept { return _settings_game.vehicle.max_ships; }
inline uint16_t RustAircraftLimit() noexcept { return _settings_game.vehicle.max_aircraft; }
inline bool RustAITrainDisabled() noexcept { return _settings_game.ai.ai_disable_veh_train; }
inline bool RustAIRoadDisabled() noexcept { return _settings_game.ai.ai_disable_veh_roadveh; }
inline bool RustAIShipDisabled() noexcept { return _settings_game.ai.ai_disable_veh_ship; }
inline bool RustAIAircraftDisabled() noexcept { return _settings_game.ai.ai_disable_veh_aircraft; }
inline TownFounding OpenTTDTownFoundingSetting() noexcept { return _settings_game.economy.found_town; }
inline TownLayout OpenTTDTownLayoutSetting() noexcept { return _settings_game.economy.town_layout; }
inline uint8_t OpenTTDInitialCitySize() noexcept { return _settings_game.economy.initial_city_size; }
inline uint8_t OpenTTDTownDensity() noexcept { return _settings_game.difficulty.number_towns; }
inline uint16_t OpenTTDCustomTownNumber() noexcept { return _settings_newgame.game_creation.custom_town_number; }
inline uint8_t OpenTTDLargerTowns() noexcept { return _settings_game.economy.larger_towns; }
inline uint8_t OpenTTDTownNameStyle() noexcept { return _settings_game.game_creation.town_name; }
inline bool RustBuildOnSlopes() noexcept { return _settings_game.construction.build_on_slopes; }
inline bool RustAutoslopeEnabled() noexcept { return AutoslopeEnabled(); }
inline bool RustCrossingWithCompetitor() noexcept { return _settings_game.construction.crossing_with_competitor; }
inline bool RustDisableElrails() noexcept { return _settings_game.vehicle.disable_elrails; }
inline bool RustForbid90Deg() noexcept { return _settings_game.pf.forbid_90_deg; }
inline bool RustServeNeutralIndustries() noexcept { return _settings_game.station.serve_neutral_industries; }
inline uint8_t RustStationSpread() noexcept { return _settings_game.station.station_spread; }
inline bool RustStationStopOnTownRoad() noexcept { return _settings_game.construction.road_stop_on_town_road; }
inline bool RustStationStopOnCompetitorRoad() noexcept { return _settings_game.construction.road_stop_on_competitor_road; }
inline bool RustStationNoiseEnabled() noexcept { return _settings_game.economy.station_noise_level; }
inline bool RustStationDistantJoin() noexcept { return _settings_game.station.distant_join_stations; }
#endif /* RUST_SETTINGS_SERVICES_H */
