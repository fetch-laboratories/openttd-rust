#pragma once
#include "rust/disaster_ffi.h"

struct DisasterProbe {
	uint32_t draws = 0, random = 0, viewports = 0, deletes = 0;
	int32_t age = 8880;
	uint8_t tick = 255;
	OpenTTDVehicleTileLinks links{};
};
static uint32_t DisasterProbeRandom(void *context) noexcept { auto *p = static_cast<DisasterProbe *>(context); p->draws++; return p->random; }
static uint32_t DisasterProbeIndustry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static void Disasters()
{
	using StateOwner = std::unique_ptr<OpenTTDDisasterState, decltype(&openttd_rust_disaster_state_destroy)>;
	DisasterProbe probe;
	const OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(&probe, DisasterProbeRandom, DisasterProbeIndustry);
	OpenTTDVehicleServices vehicles{};
	vehicles.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{13}; };
	vehicles.tick = [](OpenTTDVehicle *v) noexcept { return reinterpret_cast<DisasterProbe *>(v)->tick; };
	vehicles.write_tick = [](OpenTTDVehicle *v, uint8_t tick) noexcept { reinterpret_cast<DisasterProbe *>(v)->tick = tick; };
	vehicles.destroy = [](OpenTTDVehicle *v) noexcept { reinterpret_cast<DisasterProbe *>(v)->deletes++; };
	OpenTTDDisasterServices services{
		.shared = &shared,
		.vehicles = &vehicles,
		.age = [](OpenTTDVehicle *v) noexcept { return reinterpret_cast<DisasterProbe *>(v)->age; },
		.write_age = [](OpenTTDVehicle *v, int32_t age) noexcept { reinterpret_cast<DisasterProbe *>(v)->age = age; },
		.disasters_enabled = []() noexcept { return false; },
	};
	StateOwner state(openttd_rust_disaster_state_create(1048575), openttd_rust_disaster_state_destroy);
	const OpenTTDDisasterVehicle vehicle{.handle = reinterpret_cast<OpenTTDVehicle *>(&probe), .state = state.get(), .id = 17};
	auto *address = state.get(); state->state = 65535; state->flags = 0xAB; state->image_override = UINT32_MAX;
	uint16_t *delay = openttd_rust_disaster_delay(); *delay = 0;
	openttd_rust_disaster_daily(&services); CHECK(*delay == 65535 && probe.draws == 0);
	*delay = 1; probe.random = 511;
	openttd_rust_disaster_daily(&services); CHECK(*delay == 1241 && probe.draws == 1);
	probe.random = 0;
	openttd_rust_disaster_startup(&services); CHECK(*delay == 730 && probe.draws == 2);
	CHECK(delay == openttd_rust_disaster_delay() && state.get() == address && state->state == 65535 && state->flags == 0xAB && state->image_override == UINT32_MAX);
	CHECK(!openttd_rust_disaster_tick(&services, vehicle));
	CHECK(probe.age == 8881 && probe.tick == 0 && probe.deletes == 1 && probe.draws == 2);

	vehicles.tile_links = [](OpenTTDVehicle *v) noexcept { return &reinterpret_cast<DisasterProbe *>(v)->links; };
	vehicles.occupancy_update = [](OpenTTDVehicle *v) noexcept -> OpenTTDOccupancyUpdateObservation { return {.links = &reinterpret_cast<DisasterProbe *>(v)->links, .tile = 0}; };
	vehicles.map_log_x = []() noexcept { return 6U; };
	vehicles.vehicle_type = [](OpenTTDVehicle *) noexcept { return uint8_t{5}; };
	vehicles.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{8}; };
	vehicles.z = [](OpenTTDVehicle *) noexcept { return int32_t{0}; };
	vehicles.tile = [](OpenTTDVehicle *) noexcept { return 0U; };
	vehicles.base_viewport = [](OpenTTDVehicle *v) noexcept { auto *p = reinterpret_cast<DisasterProbe *>(v); CHECK(p->links.current != nullptr); p->viewports++; };
	services.first_sprite = [](OpenTTDVehicle *) noexcept { return 3904U; };
	services.write_first_sprite = [](OpenTTDVehicle *, uint32_t image) noexcept { CHECK(image == 3902); };
	probe.tick = 1;
	CHECK(openttd_rust_disaster_tick(&services, vehicle));
	CHECK(probe.tick == 2 && probe.viewports == 1 && probe.draws == 2 && probe.deletes == 1);
	openttd_rust_occupancy_update(&vehicles, vehicle.handle, true);
	std::printf("disaster_private_counter_direct_rng_delete_rotor_occupancy passed\n");
}
ABI_PROBE(Disasters);
