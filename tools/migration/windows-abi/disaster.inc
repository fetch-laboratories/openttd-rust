#pragma once
#include "rust/disaster_ffi.h"

struct DisasterProbe { uint32_t reads = 0, writes = 0, draws = 0, random = 0; };
static void OPENTTD_DISASTER_CALL DisasterProbeRead(void *context, uint32_t kind, uint32_t, int64_t, int64_t, int64_t *out) noexcept
{
	auto *probe = static_cast<DisasterProbe *>(context); probe->reads++;
	if (kind == 0) { out[0] = 64; out[1] = 128; out[2] = 63; out[3] = 127; out[4] = 8191; out[5] = 1935; out[6] = 0; out[9] = 1; }
	if (kind == 1) { out[0] = 13; out[7] = 8880; }
}
static void OPENTTD_DISASTER_CALL DisasterProbeWrite(void *context, uint32_t, uint32_t, int64_t) noexcept
{
	static_cast<DisasterProbe *>(context)->writes++;
}
static uint32_t DisasterProbeRandom(void *context) noexcept { auto *probe = static_cast<DisasterProbe *>(context); probe->draws++; return probe->random; }
static int64_t OPENTTD_DISASTER_CALL DisasterProbeService(void *, const OpenTTDDisasterAction *) noexcept { return 0; }
static uint32_t DisasterProbeIndustry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static void Disasters()
{
	using Owner = std::unique_ptr<OpenTTDDisasterRun, decltype(&openttd_rust_disaster_destroy)>;
	using StateOwner = std::unique_ptr<OpenTTDDisasterState, decltype(&openttd_rust_disaster_state_destroy)>;
	DisasterProbe probe;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&probe, DisasterProbeRandom, DisasterProbeIndustry);
	auto make = [&](uint32_t operation, uint32_t id = 0) { return Owner(openttd_rust_disaster_create(operation, id, 0, 0, 0, 0, &probe, DisasterProbeRead, DisasterProbeWrite, &services, DisasterProbeService), openttd_rust_disaster_destroy); };
	StateOwner state(openttd_rust_disaster_state_create(1048575), openttd_rust_disaster_state_destroy);
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
	std::printf("disaster_private_counter_direct_rng_delete_cancel passed\n");
}
ABI_PROBE(Disasters);
