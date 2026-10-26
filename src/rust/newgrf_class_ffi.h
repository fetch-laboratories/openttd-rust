/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_class_ffi.h Canonical shared class registry and native presentation aliases. */
#ifndef RUST_NEWGRF_CLASS_FFI_H
#define RUST_NEWGRF_CLASS_FFI_H
#include "service_defaults.hpp"
#include "../strings_type.h"
struct StationSpec;
struct RoadStopSpec;
struct AirportSpec;
struct ObjectSpec;
struct OpenTTDObjectServices;
struct OpenTTDGRFClassIdentity { uint32_t grfid; uint16_t local_id; };
template <typename T> struct OpenTTDGRFClassServices {
	uint16_t (*class_index)(const T *) noexcept OPENTTD_SERVICE_DEFAULT(class_index);
	void (*set_index)(T *, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_index);
	OpenTTDGRFClassIdentity (*identity)(const T *) noexcept OPENTTD_SERVICE_DEFAULT(identity);
	void (*exhausted)(uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(exhausted);
	const OpenTTDObjectServices *object;
};
extern "C" uint16_t openttd_rust_station_layout_key(uint8_t, uint8_t);
extern "C" bool openttd_rust_is_waypoint_class(uint32_t);
extern "C" void openttd_rust_station_class_defaults(StringID, StringID, StringID);
extern "C" void openttd_rust_roadstop_class_defaults(StringID, StringID, StringID);
using OpenTTDStationClassServices = OpenTTDGRFClassServices<StationSpec>;
using OpenTTDRoadStopClassServices = OpenTTDGRFClassServices<RoadStopSpec>;
using OpenTTDAirportClassServices = OpenTTDGRFClassServices<AirportSpec>;
using OpenTTDObjectClassServices = OpenTTDGRFClassServices<ObjectSpec>;
template <typename T> struct RustGRFClassOps;
extern "C" {
void openttd_rust_station_class_reset(const OpenTTDGRFClassServices<StationSpec> *);
uint16_t openttd_rust_station_class_allocate(uint32_t, uint16_t, uint32_t);
void openttd_rust_station_class_assign(StationSpec *);
void * openttd_rust_station_class_classes();
uint32_t openttd_rust_station_class_count();
uint32_t openttd_rust_station_class_ui_count();
StationSpec * const * openttd_rust_station_class_specs(uint16_t);
uint32_t openttd_rust_station_class_spec_count(uint16_t);
uint32_t openttd_rust_station_class_ui_spec_count(uint16_t);
const StationSpec * openttd_rust_station_class_spec(uint16_t, uint32_t);
const StationSpec * openttd_rust_station_class_by_grf(uint32_t, uint16_t);
}
template <> struct RustGRFClassOps<StationSpec> {
	static constexpr auto reset = openttd_rust_station_class_reset;
	static constexpr auto allocate = openttd_rust_station_class_allocate;
	static constexpr auto assign = openttd_rust_station_class_assign;
	static constexpr auto classes = openttd_rust_station_class_classes;
	static constexpr auto count = openttd_rust_station_class_count;
	static constexpr auto ui_count = openttd_rust_station_class_ui_count;
	static constexpr auto specs = openttd_rust_station_class_specs;
	static constexpr auto spec_count = openttd_rust_station_class_spec_count;
	static constexpr auto ui_spec_count = openttd_rust_station_class_ui_spec_count;
	static constexpr auto spec = openttd_rust_station_class_spec;
	static constexpr auto by_grf = openttd_rust_station_class_by_grf;
};
extern "C" {
void openttd_rust_roadstop_class_reset(const OpenTTDGRFClassServices<RoadStopSpec> *);
uint16_t openttd_rust_roadstop_class_allocate(uint32_t, uint16_t, uint32_t);
void openttd_rust_roadstop_class_assign(RoadStopSpec *);
void * openttd_rust_roadstop_class_classes();
uint32_t openttd_rust_roadstop_class_count();
uint32_t openttd_rust_roadstop_class_ui_count();
RoadStopSpec * const * openttd_rust_roadstop_class_specs(uint16_t);
uint32_t openttd_rust_roadstop_class_spec_count(uint16_t);
uint32_t openttd_rust_roadstop_class_ui_spec_count(uint16_t);
const RoadStopSpec * openttd_rust_roadstop_class_spec(uint16_t, uint32_t);
const RoadStopSpec * openttd_rust_roadstop_class_by_grf(uint32_t, uint16_t);
}
template <> struct RustGRFClassOps<RoadStopSpec> {
	static constexpr auto reset = openttd_rust_roadstop_class_reset;
	static constexpr auto allocate = openttd_rust_roadstop_class_allocate;
	static constexpr auto assign = openttd_rust_roadstop_class_assign;
	static constexpr auto classes = openttd_rust_roadstop_class_classes;
	static constexpr auto count = openttd_rust_roadstop_class_count;
	static constexpr auto ui_count = openttd_rust_roadstop_class_ui_count;
	static constexpr auto specs = openttd_rust_roadstop_class_specs;
	static constexpr auto spec_count = openttd_rust_roadstop_class_spec_count;
	static constexpr auto ui_spec_count = openttd_rust_roadstop_class_ui_spec_count;
	static constexpr auto spec = openttd_rust_roadstop_class_spec;
	static constexpr auto by_grf = openttd_rust_roadstop_class_by_grf;
};
extern "C" {
void openttd_rust_airport_class_reset(const OpenTTDGRFClassServices<AirportSpec> *);
uint16_t openttd_rust_airport_class_allocate(uint32_t, uint16_t, uint32_t);
void openttd_rust_airport_class_assign(AirportSpec *);
void * openttd_rust_airport_class_classes();
uint32_t openttd_rust_airport_class_count();
uint32_t openttd_rust_airport_class_ui_count();
AirportSpec * const * openttd_rust_airport_class_specs(uint16_t);
uint32_t openttd_rust_airport_class_spec_count(uint16_t);
uint32_t openttd_rust_airport_class_ui_spec_count(uint16_t);
const AirportSpec * openttd_rust_airport_class_spec(uint16_t, uint32_t);
const AirportSpec * openttd_rust_airport_class_by_grf(uint32_t, uint16_t);
}
template <> struct RustGRFClassOps<AirportSpec> {
	static constexpr auto reset = openttd_rust_airport_class_reset;
	static constexpr auto allocate = openttd_rust_airport_class_allocate;
	static constexpr auto assign = openttd_rust_airport_class_assign;
	static constexpr auto classes = openttd_rust_airport_class_classes;
	static constexpr auto count = openttd_rust_airport_class_count;
	static constexpr auto ui_count = openttd_rust_airport_class_ui_count;
	static constexpr auto specs = openttd_rust_airport_class_specs;
	static constexpr auto spec_count = openttd_rust_airport_class_spec_count;
	static constexpr auto ui_spec_count = openttd_rust_airport_class_ui_spec_count;
	static constexpr auto spec = openttd_rust_airport_class_spec;
	static constexpr auto by_grf = openttd_rust_airport_class_by_grf;
};
extern "C" {
void openttd_rust_object_class_reset(const OpenTTDGRFClassServices<ObjectSpec> *);
uint16_t openttd_rust_object_class_allocate(uint32_t, uint16_t, uint32_t);
void openttd_rust_object_class_assign(ObjectSpec *);
void * openttd_rust_object_class_classes();
uint32_t openttd_rust_object_class_count();
uint32_t openttd_rust_object_class_ui_count();
ObjectSpec * const * openttd_rust_object_class_specs(uint16_t);
uint32_t openttd_rust_object_class_spec_count(uint16_t);
uint32_t openttd_rust_object_class_ui_spec_count(uint16_t);
const ObjectSpec * openttd_rust_object_class_spec(uint16_t, uint32_t);
const ObjectSpec * openttd_rust_object_class_by_grf(uint32_t, uint16_t);
}
template <> struct RustGRFClassOps<ObjectSpec> {
	static constexpr auto reset = openttd_rust_object_class_reset;
	static constexpr auto allocate = openttd_rust_object_class_allocate;
	static constexpr auto assign = openttd_rust_object_class_assign;
	static constexpr auto classes = openttd_rust_object_class_classes;
	static constexpr auto count = openttd_rust_object_class_count;
	static constexpr auto ui_count = openttd_rust_object_class_ui_count;
	static constexpr auto specs = openttd_rust_object_class_specs;
	static constexpr auto spec_count = openttd_rust_object_class_spec_count;
	static constexpr auto ui_spec_count = openttd_rust_object_class_ui_spec_count;
	static constexpr auto spec = openttd_rust_object_class_spec;
	static constexpr auto by_grf = openttd_rust_object_class_by_grf;
};
#endif
