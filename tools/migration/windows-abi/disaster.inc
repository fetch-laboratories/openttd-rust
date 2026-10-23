#pragma once
#include "rust/disaster_ffi.h"

struct DisasterProbe {
	uint32_t reads = 0, writes = 0, draws = 0, random = 0, resolves = 0, viewports = 0;
	OpenTTDDisasterState *state = nullptr;
	OpenTTDVehicleTileLinks links{};
	VehicleType type = VEH_DISASTER;
};
static void OPENTTD_DISASTER_CALL DisasterProbeRead(void *context, uint32_t kind, uint32_t, int64_t, int64_t, int64_t *out) noexcept
{
	auto *probe = static_cast<DisasterProbe *>(context); probe->reads++;
	if (kind == 0) { out[0] = 64; out[1] = 128; out[2] = 63; out[3] = 127; out[4] = 8191; out[5] = 1935; out[6] = 0; out[9] = 1; }
	if (kind == 1) { out[0] = 13; out[7] = 8880; out[11] = 1048575; }
	if (kind == 7) out[0] = reinterpret_cast<intptr_t>(probe->state);
	if (kind == 5) { out[1] = 1; out[2] = 0; }
	if (kind == 15) out[0] = 1;
}
static void OPENTTD_DISASTER_CALL DisasterProbeWrite(void *context, uint32_t, uint32_t, int64_t) noexcept
{
	static_cast<DisasterProbe *>(context)->writes++;
}
static uint32_t DisasterProbeRandom(void *context) noexcept { auto *probe = static_cast<DisasterProbe *>(context); probe->draws++; return probe->random; }
static int64_t OPENTTD_DISASTER_CALL DisasterProbeService(void *, const OpenTTDDisasterAction *) noexcept { return 0; }
static uint32_t DisasterProbeIndustry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static OpenTTDVehicle *OPENTTD_DISASTER_CALL DisasterProbeShell(void *context, uint32_t id) noexcept
{
	auto *p = static_cast<DisasterProbe *>(context); CHECK(id == 17); p->resolves++;
	return reinterpret_cast<OpenTTDVehicle *>(p);
}
static std::vector<int> _disaster_cost_calls;
static void Disasters()
{
	using Owner = std::unique_ptr<OpenTTDDisasterRun, decltype(&openttd_rust_disaster_destroy)>;
	using StateOwner = std::unique_ptr<OpenTTDDisasterState, decltype(&openttd_rust_disaster_state_destroy)>;
	DisasterProbe probe;
	OpenTTDVehicleServices vehicles{};
	vehicles.tile_links = [](OpenTTDVehicle *v) noexcept { return &reinterpret_cast<DisasterProbe *>(v)->links; };
	vehicles.map_log_x = []() noexcept { return 6U; };
	vehicles.tile = [](OpenTTDVehicle *) noexcept { return 0U; };
	vehicles.vehicle_type = [](OpenTTDVehicle *v) noexcept { return static_cast<uint8_t>(reinterpret_cast<DisasterProbe *>(v)->type); };
	vehicles.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	vehicles.z = [](OpenTTDVehicle *) noexcept { return int32_t{0}; };
	vehicles.base_viewport = [](OpenTTDVehicle *v) noexcept {
		auto *p = reinterpret_cast<DisasterProbe *>(v); CHECK(p->links.current != nullptr); p->viewports++;
	};
	OpenTTDCommandCostServices costs{};
	costs.construct = [](OpenTTDCommandCostStorage *p) noexcept { _disaster_cost_calls.push_back(1); return reinterpret_cast<OpenTTDCommandCost *>(p); };
	costs.destroy = [](OpenTTDCommandCost *) noexcept { _disaster_cost_calls.push_back(5); };
	costs.zero = [](OpenTTDCommandCost *p) noexcept { _disaster_cost_calls.push_back(3); *reinterpret_cast<uint32_t *>(p) = 0; };
	costs.error = [](OpenTTDCommandCost *p, uint32_t error) noexcept { CHECK(error == 0x87654321); _disaster_cost_calls.push_back(3); *reinterpret_cast<uint32_t *>(p) = error; };
	costs.success = [](const OpenTTDCommandCost *p) noexcept { _disaster_cost_calls.push_back(4); return *reinterpret_cast<const uint32_t *>(p) == 0; };
	const OpenTTDOccupancyCommands commands{.vehicles = &vehicles, .max_pixel_z = [](uint32_t tile) noexcept { CHECK(tile == 0); _disaster_cost_calls.push_back(2); return 0; }, .costs = &costs, .vehicle_error = 0x87654321};
	const OpenTTDDisasterOccupancy occupancy{.commands = &commands, .vehicle = DisasterProbeShell};
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&probe, DisasterProbeRandom, DisasterProbeIndustry);
	auto make = [&](uint32_t operation, uint32_t id = 0) { return Owner(openttd_rust_disaster_create(operation, id, 0, 0, 0, 0, &probe, DisasterProbeRead, DisasterProbeWrite, &services, &occupancy, DisasterProbeService), openttd_rust_disaster_destroy); };
	StateOwner state(openttd_rust_disaster_state_create(1048575), openttd_rust_disaster_state_destroy);
	probe.state = state.get();
	auto *address = state.get(); state->state = 65535; state->flags = 0xAB; state->image_override = UINT32_MAX;
	uint16_t *delay = openttd_rust_disaster_delay(); *delay = 0;
	{ auto daily = make(1); CHECK(openttd_rust_disaster_advance(daily.get(), 0).kind == 0 && *delay == 65535 && probe.draws == 0); }
	*delay = 1; probe.random = 511;
	{ auto daily = make(1); CHECK(openttd_rust_disaster_advance(daily.get(), 0).kind == 0 && *delay == 1241 && probe.draws == 1); }
	probe.random = 0;
	{ auto startup = make(2); CHECK(openttd_rust_disaster_advance(startup.get(), 0).kind == 0 && *delay == 730 && probe.draws == 2); }
	CHECK(delay == openttd_rust_disaster_delay() && state.get() == address && state->state == 65535 && state->flags == 0xAB && state->image_override == UINT32_MAX);
	auto expired = make(0, 17);
	CHECK(openttd_rust_disaster_advance(expired.get(), 0).kind == 2);
	uint32_t reads = probe.reads, writes = probe.writes;
	expired.reset(); CHECK(probe.reads == reads && probe.writes == writes && probe.draws == 2);
	state->image_override = 3905;
	{ auto motion = make(4, 17); CHECK(openttd_rust_disaster_advance(motion.get(), 0).kind == 0); }
	CHECK(probe.resolves == 1 && probe.viewports == 1 && probe.links.current != nullptr && probe.draws == 2);
	probe.type = VEH_TRAIN;
	reads = probe.reads;
	{ auto blocked = make(7, 0); CHECK(openttd_rust_disaster_advance(blocked.get(), 0).kind == 0); }
	CHECK(probe.reads == reads && _disaster_cost_calls == std::vector<int>({1, 2, 3, 4, 5}));
	openttd_rust_occupancy_update(&vehicles, reinterpret_cast<OpenTTDVehicle *>(&probe), true);
	_disaster_cost_calls.clear();
	{ auto clear = make(7, 0); CHECK(openttd_rust_disaster_advance(clear.get(), 0).kind == 17); CHECK(_disaster_cost_calls == std::vector<int>({1, 2, 3, 4, 5})); CHECK(openttd_rust_disaster_advance(clear.get(), 0).kind == 0); }
	CHECK(probe.resolves == 1 && probe.viewports == 1 && probe.draws == 2);
	std::printf("disaster_private_counter_direct_rng_delete_cancel_occupancy passed\n");
}
ABI_PROBE(Disasters);
