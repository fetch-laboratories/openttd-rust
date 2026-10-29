/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_lifecycle_ffi.h Station naming, tile evolution and destructor operations. */
#ifndef RUST_STATION_LIFECYCLE_FFI_H
#define RUST_STATION_LIFECYCLE_FFI_H
#include "service_defaults.hpp"
#include "station_ffi.h"
#include "services_ffi.h"
#include "station_catchment_ffi.h"
#include "vehicle_visits_ffi.h"
#include "aircraft_ffi.h"
struct OpenTTDStationCommandServices;
struct IndustrySpec;
struct AirportSpec;
struct OpenTTDOrdersLeaves;
struct OpenTTDRoadStopResolverServices;
struct OpenTTDStationNames {
	uint32_t base, central, airport, oilfield, docks, heliport, mines, lakeside, woods, forest;
	uint32_t valley, heights, north, south, east, west, transfer, halt, exchange, annexe, sidings;
	uint32_t branch, upper, lower, fallback, undefined, null;
};
struct OpenTTDStationLifecycleServices {
	const OpenTTDSharedServices *shared;
	const OpenTTDStationCatchmentServices *catchment;
	OpenTTDTileTypeAllocation (*allocation)() noexcept OPENTTD_SERVICE_DEFAULT(allocation);
	void (*check)(bool) noexcept OPENTTD_SERVICE_DEFAULT(check);
	uint8_t (*climate)() noexcept OPENTTD_SERVICE_DEFAULT(climate);
	OpenTTDStationNames names;
	const OpenTTDTownHandle *(*town)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(town);
	uint8_t (*industry_type)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(industry_type);
	void (*set_industry_type)(BaseStation *, uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(set_industry_type);
	const IndustrySpec *(*spec)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	uint32_t (*spec_name)(const IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_name);
	bool (*spec_enabled)(const IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_enabled);
	uint8_t (*spec_life)(const IndustrySpec *) noexcept OPENTTD_SERVICE_DEFAULT(spec_life);
	void (*set_random_bits)(BaseStation *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_random_bits);
	uint16_t (*cargo_classes)(uint8_t) noexcept OPENTTD_SERVICE_DEFAULT(cargo_classes);
	bool (*forest)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(forest);
	const OpenTTDRoadStopResolverServices *roadstop;
	Station *(*as_station)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(as_station);
	bool (*cleaning_pool)() noexcept OPENTTD_SERVICE_DEFAULT(cleaning_pool);
	void (*close_view)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(close_view);
	void (*delete_news)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(delete_news);
	void (*remove_index)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(remove_index);
	bool (*sign_valid)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(sign_valid);
	void (*remove_sign)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(remove_sign);
	uint32_t (*vehicle_id)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(vehicle_id);
	uint16_t (*last_loading)(OpenTTDVehicle *) noexcept OPENTTD_SERVICE_DEFAULT(last_loading);
	uint16_t (*airport_maintenance)(const AirportSpec *) noexcept OPENTTD_SERVICE_DEFAULT(airport_maintenance);
	size_t maintenance_price;
};
const OpenTTDStationLifecycleServices &GetRustStationLifecycleServices() noexcept;
inline const OpenTTDStationLifecycleServices *RustStationLifecycleServices() noexcept { return &GetRustStationLifecycleServices(); }
extern "C" {
void openttd_rust_station_destruct(const OpenTTDStationLifecycleServices *, BaseStation *);
int64_t openttd_rust_station_airport_maintenance(const OpenTTDStationLifecycleServices *, uint8_t);
}
#endif /* RUST_STATION_LIFECYCLE_FFI_H */
