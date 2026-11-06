/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file vehicle_properties_ffi.h Synchronous native storage leaves for vehicle properties. */
#ifndef RUST_VEHICLE_PROPERTIES_FFI_H
#define RUST_VEHICLE_PROPERTIES_FFI_H
#include "engine_loading_ffi.h"
#include "newgrf_bytereader_ffi.h"
struct OpenTTDVehicleProperties {
	const OpenTTDEngineLoadingServices *loading;
	uint16_t (*id)(const OpenTTDEngine *) noexcept OPENTTD_SERVICE_DEFAULT(id);
	SoundID (*sound)(const GRFFile *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(sound);
	bool (*train_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(train_image_valid);
	bool (*road_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(road_image_valid);
	bool (*ship_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(ship_image_valid);
	bool (*aircraft_image_valid)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(aircraft_image_valid);
	void (*uses_2cc)(bool) noexcept OPENTTD_SERVICE_DEFAULT(uses_2cc);
};
const OpenTTDVehicleProperties &GetRustVehicleProperties() noexcept;
extern "C" {
int32_t openttd_rust_vehicle_properties(const OpenTTDVehicleProperties *, const OpenTTDByteReaderServices *, const GRFFile *, VehicleType, uint32_t, uint32_t, int32_t, ByteReader *);
}
#endif
