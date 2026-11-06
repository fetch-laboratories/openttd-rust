/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station_association_ffi.h Rail definition association policy over Rust-owned station reference lists. */
#ifndef RUST_STATION_ASSOCIATION_FFI_H
#define RUST_STATION_ASSOCIATION_FFI_H
#include "service_defaults.hpp"
#include "station_ffi.h"
#include "newgrf_class_ffi.h"
struct BaseStation;
struct OpenTTDSharedServices;
struct OpenTTDRoadStopResolverServices;
struct OpenTTDStationSpecTriggers { uint16_t animation; uint64_t cargo; };
struct OpenTTDStationAssociationServices {
	OpenTTDStationSpecTriggers (*triggers)(const StationSpec *) noexcept OPENTTD_SERVICE_DEFAULT(triggers);
	const OpenTTDSharedServices *shared;
};
const OpenTTDStationAssociationServices &GetStationAssociationServices() noexcept;
extern "C" {
OpenTTDStationSpecIndex openttd_rust_station_spec_allocate(const StationSpec *, BaseStation *);
void openttd_rust_station_spec_assign(const OpenTTDStationAssociationServices *, const StationSpec *, BaseStation *, uint8_t);
void openttd_rust_station_spec_cache(const OpenTTDStationAssociationServices *, BaseStation *);
void openttd_rust_station_spec_rebind(BaseStation *);
}
#endif
