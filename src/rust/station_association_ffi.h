/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_association_ffi.h Rail definition associations in canonical native station shells. */
#ifndef RUST_STATION_ASSOCIATION_FFI_H
#define RUST_STATION_ASSOCIATION_FFI_H
#include "service_defaults.hpp"
#include "station_ffi.h"
#include "newgrf_class_ffi.h"
struct BaseStation;
struct OpenTTDSharedServices;
struct OpenTTDRoadStopResolverServices;
struct OpenTTDStationSpecVacancy { const StationSpec *spec; uint32_t grfid; };
struct OpenTTDStationSpecTriggers { uint16_t animation; uint64_t cargo; };
struct OpenTTDStationAssociationServices {
	OpenTTDStationSpecTriggers (*triggers)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(triggers);
	const OpenTTDSharedServices *shared;
	const OpenTTDRoadStopResolverServices *roadstop;
	size_t (*count)(const BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(count);
	OpenTTDStationSpecVacancy (*vacancy)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(vacancy);
	const StationSpec *(*spec)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(spec);
	OpenTTDGRFClassIdentity (*key)(const BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(key);
	void (*resize)(BaseStation *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(resize);
	void (*clear)(BaseStation *) noexcept OPENTTD_SERVICE_DEFAULT(clear);
	void (*set_slot)(BaseStation *, uint8_t, const StationSpec *, uint32_t, uint16_t) noexcept OPENTTD_SERVICE_DEFAULT(set_slot);
	void (*rebind)(BaseStation *, size_t, const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(rebind);
	OpenTTDGRFClassIdentity (*identity)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(identity);
	uint32_t (*custom_index)(uint32_t) noexcept OPENTTD_SERVICE_DEFAULT(custom_index);
	void (*set_cached)(BaseStation *, uint16_t, uint64_t) noexcept OPENTTD_SERVICE_DEFAULT(set_cached);
};
const OpenTTDStationAssociationServices &GetStationAssociationServices() noexcept;
extern "C" {
OpenTTDStationSpecIndex openttd_rust_station_spec_allocate(const OpenTTDStationAssociationServices *, const StationSpec *, BaseStation *);
void openttd_rust_station_spec_assign(const OpenTTDStationAssociationServices *, const StationSpec *, BaseStation *, uint8_t);
void openttd_rust_station_spec_cache(const OpenTTDStationAssociationServices *, BaseStation *);
void openttd_rust_station_spec_rebind(const OpenTTDStationAssociationServices *, BaseStation *);
}
#endif
