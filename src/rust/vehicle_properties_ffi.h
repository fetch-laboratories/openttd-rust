/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_properties_ffi.h Synchronous native storage leaves for vehicle properties. */
#ifndef RUST_VEHICLE_PROPERTIES_FFI_H
#define RUST_VEHICLE_PROPERTIES_FFI_H
#include "engine_loading_ffi.h"
#include "newgrf_bytereader_ffi.h"
struct OpenTTDVehicleProperties {
	const OpenTTDEngineLoadingServices *loading;
	void (*set_intro)(const OpenTTDEngine *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_intro);
	void (*set_decay)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_decay);
	void (*set_life)(const OpenTTDEngine *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_life);
	void (*set_model_life)(const OpenTTDEngine *, int32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_model_life);
	void (*set_climates)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_climates);
	void (*set_load_amount)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_load_amount);
	void (*set_cargo_label)(const OpenTTDEngine *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_label);
	void (*set_callbacks)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_callbacks);
	void (*set_refit_cost)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_refit_cost);
	void (*set_retire)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_retire);
	void (*set_misc)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_misc);
	void (*set_extra)(const OpenTTDEngine *, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(set_extra);
	void (*set_cargo_age)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cargo_age);
	void (*set_train_ai)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_ai);
	void (*set_train_speed)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_speed);
	void (*set_train_power)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_power);
	void (*set_train_kind)(const OpenTTDEngine *, RailVehicleTypes) noexcept OPENTTD_SERVICE_DEFAULT(set_train_kind);
	void (*set_train_running)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_running);
	void (*set_train_price)(const OpenTTDEngine *, Price) noexcept OPENTTD_SERVICE_DEFAULT(set_train_price);
	void (*set_train_image)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_image);
	void (*set_train_capacity)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_capacity);
	void (*set_train_weight)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_weight);
	void (*set_train_cost)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_cost);
	void (*set_train_class)(const OpenTTDEngine *, EngineClass) noexcept OPENTTD_SERVICE_DEFAULT(set_train_class);
	void (*set_train_wagon_power)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_wagon_power);
	void (*set_train_te)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_te);
	void (*set_train_drag)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_drag);
	void (*set_train_shorten)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_shorten);
	void (*set_train_visual)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_visual);
	void (*set_train_wagon_weight)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_wagon_weight);
	void (*set_train_user)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_user);
	void (*set_train_curve)(const OpenTTDEngine *, int16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_train_curve);
	void (*set_road_running)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_running);
	void (*set_road_price)(const OpenTTDEngine *, Price) noexcept OPENTTD_SERVICE_DEFAULT(set_road_price);
	void (*set_road_image)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_image);
	void (*set_road_capacity)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_capacity);
	void (*set_road_cost)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_cost);
	void (*set_road_sound)(const OpenTTDEngine *, SoundID) noexcept OPENTTD_SERVICE_DEFAULT(set_road_sound);
	void (*set_road_power)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_power);
	void (*set_road_weight)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_weight);
	void (*set_road_te)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_te);
	void (*set_road_drag)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_drag);
	void (*set_road_visual)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_visual);
	void (*set_road_shorten)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_road_shorten);
	void (*set_ship_image)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_image);
	void (*set_ship_cost)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_cost);
	void (*set_ship_speed)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_speed);
	void (*set_ship_capacity)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_capacity);
	void (*set_ship_running)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_running);
	void (*set_ship_sound)(const OpenTTDEngine *, SoundID) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_sound);
	void (*set_ship_ocean)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_ocean);
	void (*set_ship_canal)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_canal);
	void (*set_ship_visual)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_visual);
	void (*set_ship_acceleration)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_ship_acceleration);
	void (*set_aircraft_image)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_image);
	void (*set_aircraft_subtype)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_subtype);
	void (*set_aircraft_cost)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_cost);
	void (*set_aircraft_speed)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_speed);
	void (*set_aircraft_acceleration)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_acceleration);
	void (*set_aircraft_running)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_running);
	void (*set_aircraft_passengers)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_passengers);
	void (*set_aircraft_mail)(const OpenTTDEngine *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_mail);
	void (*set_aircraft_sound)(const OpenTTDEngine *, SoundID) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_sound);
	void (*set_aircraft_range)(const OpenTTDEngine *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_aircraft_range);
	uint16_t (*id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(id);
	uint16_t (*callbacks)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(callbacks);
	uint16_t (*train_power)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_power);
	RailVehicleTypes (*train_kind)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_kind);
	uint16_t (*train_weight)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_weight);
	EngineClass (*train_class)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(train_class);
	uint8_t (*aircraft_subtype)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_subtype);
	std::vector<BadgeID> * (*badges)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(badges);
	SoundID (*sound)(const GRFFile *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);
	bool (*train_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(train_image_valid);
	bool (*road_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_image_valid);
	bool (*ship_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(ship_image_valid);
	bool (*aircraft_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_image_valid);
	void (*uses_2cc)(bool) noexcept OPENTTD_SERVICE_DEFAULT(uses_2cc);
	void (*invalid_sprite)(VehicleType, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_sprite);
	void (*invalid_cargo)(VehicleType, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_cargo);
	void (*invalid_track)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_track);
	void (*invalid_weight)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_weight);
	void (*ai_rank)() noexcept OPENTTD_SERVICE_DEFAULT(ai_rank);
	void (*invalid_price)(const char *, size_t, uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(invalid_price);
};
const OpenTTDVehicleProperties &GetRustVehicleProperties() noexcept;
extern "C" {
int32_t openttd_rust_vehicle_properties(const OpenTTDVehicleProperties *, const OpenTTDByteReaderServices *, const GRFFile *, VehicleType, uint32_t, uint32_t, int32_t, ByteReader *);
}
#endif
