/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_properties_services.hpp Native property storage and validation leaves. */
#include "vehicle_properties_ffi.h"
#include "engine_services.h"
#include "../newgrf_sound.h"
#include "../newgrf/newgrf_internal_vehicle.h"
static std::string_view RustPropertyLocation(VehicleType type) noexcept
{
	static constexpr std::string_view names[]{"RailVehicleChangeInfo", "RoadVehicleChangeInfo", "ShipVehicleChangeInfo", "AircraftVehicleChangeInfo"};
	return names[type];
}
const OpenTTDVehicleProperties &GetRustVehicleProperties() noexcept
{
	static const OpenTTDVehicleProperties services{
		.loading = &GetRustEngineLoadingServices(),
		.set_intro = RustSetEngineIntro,
		.set_decay = RustSetEngineDecay,
		.set_life = RustSetEngineLife,
		.set_model_life = RustSetEngineModelLife,
		.set_climates = RustSetEngineClimates,
		.set_load_amount = RustSetEngineLoadAmount,
		.set_cargo_label = RustSetEngineCargoLabel,
		.set_callbacks = RustSetEngineCallbacks,
		.set_refit_cost = RustSetEngineRefitCost,
		.set_retire = RustSetEngineRetire,
		.set_misc = RustSetEngineMisc,
		.set_extra = RustSetEngineExtra,
		.set_cargo_age = RustSetEngineCargoAge,
		.set_train_ai = RustSetEngineTrainAi,
		.set_train_speed = RustSetEngineTrainSpeed,
		.set_train_power = RustSetEngineTrainPower,
		.set_train_kind = RustSetEngineTrainKind,
		.set_train_running = RustSetEngineTrainRunning,
		.set_train_price = RustSetEngineTrainPrice,
		.set_train_image = RustSetEngineTrainImage,
		.set_train_capacity = RustSetEngineTrainCapacity,
		.set_train_weight = RustSetEngineTrainWeight,
		.set_train_cost = RustSetEngineTrainCost,
		.set_train_class = RustSetEngineTrainClass,
		.set_train_wagon_power = RustSetEngineTrainWagonPower,
		.set_train_te = RustSetEngineTrainTe,
		.set_train_drag = RustSetEngineTrainDrag,
		.set_train_shorten = RustSetEngineTrainShorten,
		.set_train_visual = RustSetEngineTrainVisual,
		.set_train_wagon_weight = RustSetEngineTrainWagonWeight,
		.set_train_user = RustSetEngineTrainUser,
		.set_train_curve = RustSetEngineTrainCurve,
		.set_road_running = RustSetEngineRoadRunning,
		.set_road_price = RustSetEngineRoadPrice,
		.set_road_image = RustSetEngineRoadImage,
		.set_road_capacity = RustSetEngineRoadCapacity,
		.set_road_cost = RustSetEngineRoadCost,
		.set_road_sound = RustSetEngineRoadSound,
		.set_road_power = RustSetEngineRoadPower,
		.set_road_weight = RustSetEngineRoadWeight,
		.set_road_te = RustSetEngineRoadTe,
		.set_road_drag = RustSetEngineRoadDrag,
		.set_road_visual = RustSetEngineRoadVisual,
		.set_road_shorten = RustSetEngineRoadShorten,
		.set_ship_image = RustSetEngineShipImage,
		.set_ship_cost = RustSetEngineShipCost,
		.set_ship_speed = RustSetEngineShipSpeed,
		.set_ship_capacity = RustSetEngineShipCapacity,
		.set_ship_running = RustSetEngineShipRunning,
		.set_ship_sound = RustSetEngineShipSound,
		.set_ship_ocean = RustSetEngineShipOcean,
		.set_ship_canal = RustSetEngineShipCanal,
		.set_ship_visual = RustSetEngineShipVisual,
		.set_ship_acceleration = RustSetEngineShipAcceleration,
		.set_aircraft_image = RustSetEngineAircraftImage,
		.set_aircraft_subtype = RustSetEngineAircraftSubtype,
		.set_aircraft_cost = RustSetEngineAircraftCost,
		.set_aircraft_speed = RustSetEngineAircraftSpeed,
		.set_aircraft_acceleration = RustSetEngineAircraftAcceleration,
		.set_aircraft_running = RustSetEngineAircraftRunning,
		.set_aircraft_passengers = RustSetEngineAircraftPassengers,
		.set_aircraft_mail = RustSetEngineAircraftMail,
		.set_aircraft_sound = RustSetEngineAircraftSound,
		.set_aircraft_range = RustSetEngineAircraftRange,
		.id = RustEngineID,
		.callbacks = RustEngineCallbackMask,
		.train_power = RustEngineTrainPower,
		.train_kind = Engine_rail_kind,
		.train_weight = RustEngineTrainWeight,
		.train_class = RustEngineTrainClass,
		.aircraft_subtype = RustEngineAircraftSubtype,
		.badges = RustEngineBadges,
		.sound = [](const GRFFile *file, uint16_t sound) noexcept { return GetNewGRFSoundID(file, sound); },
		.train_image_valid = IsValidNewGRFImageIndex<VEH_TRAIN>,
		.road_image_valid = IsValidNewGRFImageIndex<VEH_ROAD>,
		.ship_image_valid = IsValidNewGRFImageIndex<VEH_SHIP>,
		.aircraft_image_valid = IsValidNewGRFImageIndex<VEH_AIRCRAFT>,
		.uses_2cc = [](bool value) noexcept { _loaded_newgrf_features.has_2CC |= value; },
		.invalid_sprite = [](VehicleType type, uint8_t value) noexcept { GrfMsg(1, "{}: Invalid Sprite {} specified, ignoring", RustPropertyLocation(type), value); },
		.invalid_cargo = [](VehicleType type, uint8_t value) noexcept { GrfMsg(2, "{}: Invalid cargo type {}, using first refittable", RustPropertyLocation(type), value); },
		.invalid_track = [](uint8_t value) noexcept { GrfMsg(1, "RailVehicleChangeInfo: Invalid track type {} specified, ignoring", value); },
		.invalid_weight = [](uint8_t value) noexcept { GrfMsg(2, "RailVehicleChangeInfo: Nonsensical weight of {} tons, ignoring", value << 8); },
		.ai_rank = []() noexcept { GrfMsg(2, "RailVehicleChangeInfo: Property 0x18 'AI rank' not used by NoAI, ignored."); },
		.invalid_price = [](const char *location, size_t size, uint32_t value) noexcept { GrfMsg(1, "{}: Unsupported running cost base 0x{:04X}, ignoring", std::string_view(location, size), value); },
	};
	return services;
}

static_assert(AIR_HELI == 0 && EC_DIESEL == 1 && EC_ELECTRIC == 2 && EC_MAGLEV == 4);
static_assert(RAILVEH_SINGLEHEAD == 0 && RAILVEH_MULTIHEAD == 1 && RAILVEH_WAGON == 2);
static_assert(RAILTYPE_LABEL_RAIL == 0x5241494c && RAILTYPE_LABEL_ELECTRIC == 0x454c524c && RAILTYPE_LABEL_MONO == 0x4d4f4e4f && RAILTYPE_LABEL_MAGLEV == 0x4d474c56);
static_assert(to_underlying(EngineMiscFlag::Uses2CC) == 1);
