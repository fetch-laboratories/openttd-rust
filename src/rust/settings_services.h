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
#include "byte_span_ffi.h"
#include "../settings_type.h"
#include "../road.h"
#include "../autoslope.h"
#include "../openttd.h"

inline bool RustDynamicEngines() noexcept { return _settings_game.vehicle.dynamic_engines; }
inline uint8_t RustRoadAccelerationModel() noexcept { return _settings_game.vehicle.roadveh_acceleration_model; }
inline uint8_t RustRoadSide() noexcept { return _settings_game.vehicle.road_side; }
inline bool RustRoadQueue() noexcept { return _settings_game.pf.roadveh_queue; }
inline uint32_t RustMaximumDepotPenalty() noexcept { return _settings_game.pf.yapf.maximum_go_to_depot_penalty; }
inline uint8_t RustPlaneSpeed() noexcept { return _settings_game.vehicle.plane_speed; }
inline uint8_t RustPlaneCrashes() noexcept { return _settings_game.vehicle.plane_crashes; }
inline bool RustServiceAtHelipad() noexcept { return _settings_game.order.serviceathelipad; }
inline bool RustDisastersEnabled() noexcept { return _settings_game.difficulty.disasters; }
inline bool RustDisasterSound() noexcept { return _settings_client.sound.disaster; }
inline uint16_t RustCommandsPerFrame() noexcept { return _settings_client.network.commands_per_frame; }
inline uint16_t RustCommandsPerFrameServer() noexcept { return _settings_client.network.commands_per_frame_server; }
inline uint16_t RustNetworkBytesPerFrame() noexcept { return _settings_client.network.bytes_per_frame; }
inline uint16_t RustNetworkBytesPerFrameBurst() noexcept { return _settings_client.network.bytes_per_frame_burst; }
inline OpenTTDByteSpan RustNetworkClientName() noexcept
{
	const auto &name = _settings_client.network.client_name;
	return {reinterpret_cast<const uint8_t *>(name.data()), name.size()};
}
inline uint8_t RustNetworkMaxClients() noexcept { return _settings_client.network.max_clients; }
inline uint8_t RustNetworkMaxCompanies() noexcept { return _settings_client.network.max_companies; }
inline uint8_t RustNetworkMinActiveClients() noexcept { return _settings_client.network.min_active_clients; }
inline bool RustNetworkPauseOnJoin() noexcept { return _settings_client.network.pause_on_join; }
inline uint8_t RustNetworkFrameFreq() noexcept { return _settings_client.network.frame_freq; }
inline uint16_t RustNetworkMaxDownloadTime() noexcept { return _settings_client.network.max_download_time; }
inline uint16_t RustNetworkMaxInitTime() noexcept { return _settings_client.network.max_init_time; }
inline uint16_t RustNetworkMaxJoinTime() noexcept { return _settings_client.network.max_join_time; }
inline uint16_t RustNetworkMaxLagTime() noexcept { return _settings_client.network.max_lag_time; }
inline uint16_t RustNetworkMaxPasswordTime() noexcept { return _settings_client.network.max_password_time; }
inline uint16_t RustNetworkSyncFreq() noexcept { return _settings_client.network.sync_freq; }
inline bool RustNeverExpireAirports() noexcept { return _settings_game.station.never_expire_airports; }
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
inline uint8_t OpenTTDTownCargoGenMode() noexcept { return _settings_game.economy.town_cargogen_mode; }
inline uint8_t OpenTTDPlaceHouses() noexcept { return _settings_game.economy.place_houses; }
inline bool RustServeNeutralIndustries() noexcept { return _settings_game.station.serve_neutral_industries; }
inline uint8_t RustStationSpread() noexcept { return _settings_game.station.station_spread; }
inline bool RustModifiedCatchment() noexcept { return _settings_game.station.modified_catchment; }
inline bool RustStationStopOnTownRoad() noexcept { return _settings_game.construction.road_stop_on_town_road; }
inline bool RustStationStopOnCompetitorRoad() noexcept { return _settings_game.construction.road_stop_on_competitor_road; }
inline bool RustStationNoiseEnabled() noexcept { return _settings_game.economy.station_noise_level; }
inline bool RustStationDistantJoin() noexcept { return _settings_game.station.distant_join_stations; }
inline bool RustNormalGame() noexcept { return _game_mode == GM_NORMAL; }
inline bool RustExtraDynamite() noexcept { return _settings_game.construction.extra_dynamite; }
inline void RustSetMapHeightLimit(uint8_t height) noexcept { _settings_game.construction.map_height_limit = height; }
inline void RustSetGenerationSeed(uint32_t seed) noexcept { _settings_game.game_creation.generation_seed = seed; }
inline uint8_t RustMapHeightLimit() noexcept { return _settings_game.construction.map_height_limit; }
inline uint16_t RustMaxBridgeLength() noexcept { return _settings_game.construction.max_bridge_length; }
inline uint8_t RustMaxBridgeHeight() noexcept { return _settings_game.construction.max_bridge_height; }
inline uint16_t RustMaxTunnelLength() noexcept { return _settings_game.construction.max_tunnel_length; }
inline uint16_t OpenTTDSubsidyDuration() noexcept { return _settings_game.difficulty.subsidy_duration; }
inline uint8_t RustSubsidyMultiplier() noexcept { return _settings_game.difficulty.subsidy_multiplier; }
inline uint8_t RustFeederPaymentShare() noexcept { return _settings_game.economy.feeder_payment_share; }
inline bool OpenTTDManualCargoDistribution(CargoType cargo) noexcept { return _settings_game.linkgraph.GetDistributionType(cargo) == DT_MANUAL; }
inline bool OpenTTDAutomaticCargoDistribution(CargoType cargo) noexcept { return !OpenTTDManualCargoDistribution(cargo); }
inline bool OpenTTDManualPassengerDistribution() noexcept { return _settings_game.linkgraph.distribution_pax == DT_MANUAL; }
inline bool OpenTTDFundRoads() noexcept { return _settings_game.economy.fund_roads; }
inline bool OpenTTDFundBuildings() noexcept { return _settings_game.economy.fund_buildings; }
inline bool OpenTTDExclusiveRights() noexcept { return _settings_game.economy.exclusive_rights; }
inline bool OpenTTDBribe() noexcept { return _settings_game.economy.bribe; }
inline void RustSetSnowLineHeight(uint8_t height) noexcept { _settings_game.game_creation.snow_line_height = height; }
inline void RustSetFreeformEdges(bool value) noexcept { _settings_game.construction.freeform_edges = value; }
inline bool RustGradualLoading() noexcept { return _settings_game.order.gradual_loading; }
inline uint8_t RustTrainSignalSide() noexcept { return _settings_game.construction.train_signal_side; }
inline uint8_t RustFreightTrains() noexcept { return _settings_game.vehicle.freight_trains; }
inline bool RustWagonSpeedLimits() noexcept { return _settings_game.vehicle.wagon_speed_limits; }
inline bool RustAllowTownRoads() noexcept { return _settings_game.economy.allow_town_roads; }
inline bool RustImprovedLoad() noexcept { return _settings_game.order.improved_load; }
inline bool RustInflation() noexcept { return _settings_game.economy.inflation; }
inline int32_t RustStartingYear() noexcept { return _settings_game.game_creation.starting_year.base(); }
inline uint32_t RustGenerationSeed() noexcept { return _settings_game.game_creation.generation_seed; }
/** Adapt legacy callback widths and unused context arguments to a canonical setting read. */
template <typename Result, auto Read, typename... Args>
inline Result RustSettingRead(Args...) noexcept { return Read(); }
inline uint8_t RustVehicleSmokeAmount() noexcept { return _settings_game.vehicle.smoke_amount; }
inline uint8_t RustTrainPathBackoffInterval() noexcept { return _settings_game.pf.path_backoff_interval; }
inline bool RustTrainReservePaths() noexcept { return _settings_game.pf.reserve_paths; }
inline uint16_t RustMinutesPerCalendarYear() noexcept { return _settings_game.economy.minutes_per_calendar_year; }
inline bool RustUsingWallclockUnits(bool newgame) noexcept { return (newgame ? _settings_newgame : _settings_game).economy.timekeeping_units == TKU_WALLCLOCK; }
inline VehicleDefaultSettings *RustClientVehicleDefaults() noexcept { return &_settings_client.company.vehicle; }
inline uint16_t RustNewgameMinutesPerCalendarYear() noexcept { return _settings_newgame.economy.minutes_per_calendar_year; }
inline void RustSetMinutesPerCalendarYear(bool newgame, uint16_t minutes) noexcept { (newgame ? _settings_newgame : _settings_game).economy.minutes_per_calendar_year = minutes; }
inline void RustSetNewgameWallclockUnits() noexcept { _settings_newgame.economy.timekeeping_units = TKU_WALLCLOCK; }
inline int32_t RustVehicleLifeExtension() noexcept { return _settings_game.vehicle.extend_vehicle_life; }
#endif /* RUST_SETTINGS_SERVICES_H */
