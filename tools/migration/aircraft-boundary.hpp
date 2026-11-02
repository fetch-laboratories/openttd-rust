/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file aircraft-boundary.hpp Strict native fixture defaults for the direct aircraft ABI. */
#ifndef OPENTTD_AIRCRAFT_BOUNDARY_HPP
#define OPENTTD_AIRCRAFT_BOUNDARY_HPP
#include "rust/aircraft_ffi.h"
#include "rust/airport_resolver_ffi.h"
#include "rust/spec_services_ffi.h"
#include "rust/station_command_ffi.h"
#include "rust/station_service_ffi.h"
#include <cstdlib>
#include <cstdio>
/* Every function pointer is valid. A fixture explicitly supplies every service
 * it expects to use, so an unexpected crossing fails instead of silently passing. */
static OpenTTDAirportResolverServices &AircraftTestAirportServices() { static OpenTTDAirportResolverServices services; return services; }
static OpenTTDStationCommandServices &AircraftTestAirportCommands() { static OpenTTDStationCommandServices commands; return commands; }
static OpenTTDStationServices &AircraftTestAirportWorld() { static OpenTTDStationServices world; return world; }
static OpenTTDAircraftLeaves AircraftTestLeaves(OpenTTDVehicleServices &vehicle, OpenTTDSharedServices &shared)
{
	OpenTTDAircraftLeaves leaves{};
	leaves.vehicle = &vehicle;
	leaves.shared = &shared;
	shared.context = nullptr;
	/* Pure airport selection runs in Rust. These explicit leaves stand in
	 * for immutable native specs; unexpected services retain abort defaults. */
	static int airport_tag;
	auto &runtime = AircraftTestAirportServices(); runtime = OpenTTDAirportResolverServices{};
	runtime.shared = &shared; runtime.vehicles = &vehicle;
	runtime.dummy_spec = reinterpret_cast<const AirportSpec *>(&airport_tag);
	runtime.dummy_fta = reinterpret_cast<const AirportFTAClass *>(&airport_tag);
	runtime.base_spec = [](uint16_t) noexcept { return reinterpret_cast<const AirportSpec *>(&airport_tag); };
	static OpenTTDAirportSpecSubstitute substitute{};
	substitute.enabled = [](const AirportSpec *) noexcept { return true; };
	substitute.override_id = [](const AirportSpec *) noexcept { return uint16_t{254}; };
	static OpenTTDAirportSpecServices airport{};
	airport.substitute = &substitute;
	static OpenTTDNewGRFSpecs specs{};
	specs.airport = &airport;
	shared.newgrf_specs = &specs;
	runtime.fta = [](const AirportSpec *spec) noexcept { return reinterpret_cast<const AirportFTAClass *>(spec); };
	runtime.check = [](bool condition) noexcept { if (!condition) std::abort(); };
	runtime.engine_if_valid = [](uint16_t) noexcept { return reinterpret_cast<const OpenTTDEngine *>(&airport_tag); };
	runtime.engine_kind = [](const OpenTTDEngine *) noexcept { return static_cast<VehicleType>(3); };
	runtime.engine_subtype = [](const OpenTTDEngine *) noexcept { return uint8_t{1}; };
	runtime.fta_flags = [](const AirportFTAClass *) noexcept { return uint8_t{3}; };
	auto &commands = AircraftTestAirportCommands(); commands = OpenTTDStationCommandServices{};
	auto &world = AircraftTestAirportWorld(); world = OpenTTDStationServices{};
	world.facilities = [](OpenTTDBaseStation *) noexcept { return uint8_t{8}; };
	commands.station = &world;
	commands.airport_tile = [](const BaseStation *) noexcept { return uint32_t{7}; };
	commands.airport_type = [](const BaseStation *) noexcept { return uint8_t{0}; };
	shared.station_commands = []() noexcept -> const OpenTTDStationCommandServices * { return &AircraftTestAirportCommands(); };
	vehicle.vehicle_type = [](OpenTTDVehicle *) noexcept { return uint8_t{3}; };
	vehicle.engine_id = [](OpenTTDVehicle *) noexcept { return uint16_t{0}; };
	leaves.airport_tile = [](const OpenTTDStation *) noexcept { return uint32_t{7}; };
	leaves.airport_type = [](const OpenTTDStation *) noexcept { return uint8_t{0}; };
	openttd_rust_airport_exchange(&runtime);
	return leaves;
}
#endif /* OPENTTD_AIRCRAFT_BOUNDARY_HPP */
