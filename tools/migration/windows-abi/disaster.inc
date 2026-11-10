#pragma once
#include "rust/disaster_ffi.h"
#include "rust/clock_ffi.h"
#include "industrytype.h"
#include "rust/industry_ffi.h"
#include "core/backup_type.hpp"

struct DisasterProbe {
	OpenTTDVehicle record{};
	DisasterProbe() { this->record.shell = reinterpret_cast<OpenTTDVehicleShell *>(this); this->record.tick_counter = 255; this->record.subtype = 13; this->record.type = VEH_DISASTER; }
	uint32_t draws = 0, random = 0, viewports = 0, deletes = 0, clears = 0, signals = 0, news = 0, tile_types = 0;
	int32_t age = 8880;
	uint8_t tile_type = 0, clear_owner = 0;
	std::vector<uint32_t> script{};
	size_t scripted = 0;
	OpenTTDVehicleTileLinks links{};
	const OpenTTDDisasterServices *services = nullptr;
	OpenTTDDisasterState *state = nullptr;
};
static DisasterProbe *_disaster_probe = nullptr;
static std::vector<int> _disaster_cost_calls;
static uint32_t DisasterProbeRandom(void *context) noexcept
{
	auto *p = static_cast<DisasterProbe *>(context);
	p->draws++;
	return p->scripted < p->script.size() ? p->script[p->scripted++] : p->random;
}
static uint32_t DisasterProbeIndustry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static void Disasters()
{
	using StateOwner = std::unique_ptr<OpenTTDDisasterState, decltype(&openttd_rust_disaster_state_destroy)>;
	DisasterProbe probe;
	_disaster_probe = &probe;
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(&probe, DisasterProbeRandom, DisasterProbeIndustry);
	OpenTTDVehicleServices vehicles{};

	vehicles.destroy = [](OpenTTDVehicle *v) noexcept { reinterpret_cast<DisasterProbe *>(v->shell)->deletes++; };
	OpenTTDDisasterServices services{};
	services.shared = &shared;
	services.vehicles = &vehicles;
	services.age = [](OpenTTDVehicle *v) noexcept { return reinterpret_cast<DisasterProbe *>(v->shell)->age; };
	services.write_age = [](OpenTTDVehicle *v, int32_t age) noexcept { reinterpret_cast<DisasterProbe *>(v->shell)->age = age; };
	services.disasters_enabled = []() noexcept { return false; };
	probe.services = &services;
	StateOwner state(openttd_rust_disaster_state_create(1048575), openttd_rust_disaster_state_destroy);
	probe.state = state.get();
	const OpenTTDDisasterVehicle vehicle{.handle = &probe.record, .state = state.get(), .id = 17};
	auto *address = state.get(); state->state = 65535; state->flags = 0xAB; state->image_override = UINT32_MAX;
	uint16_t *delay = openttd_rust_disaster_delay(); *delay = 0;
	openttd_rust_clock_disaster_daily(&services, nullptr); CHECK(*delay == 65535 && probe.draws == 0);
	*delay = 1; probe.random = 511;
	openttd_rust_clock_disaster_daily(&services, nullptr); CHECK(*delay == 1241 && probe.draws == 1);
	probe.random = 0;
	CHECK(delay == openttd_rust_disaster_delay() && state.get() == address && state->state == 65535 && state->flags == 0xAB && state->image_override == UINT32_MAX);
	CHECK(!openttd_rust_disaster_tick(&services, vehicle));
	CHECK(probe.age == 8881 && probe.record.tick_counter == 0 && probe.deletes == 1 && probe.draws == 1);

	vehicles.tile_links = [](OpenTTDVehicle *v) noexcept { return &reinterpret_cast<DisasterProbe *>(v->shell)->links; };
	vehicles.map_log_x = []() noexcept { return 6U; };

	vehicles.base_viewport = [](OpenTTDVehicle *v) noexcept { auto *p = reinterpret_cast<DisasterProbe *>(v->shell); CHECK(p->links.current != nullptr); p->viewports++; };
	vehicles.first_sprite = [](OpenTTDVehicle *) noexcept { return 3904U; };
	services.write_first_sprite = [](OpenTTDVehicle *, uint32_t image) noexcept { CHECK(image == 3902); };
	probe.record.tick_counter = 1; probe.record.subtype = 8;
	CHECK(openttd_rust_disaster_tick(&services, vehicle));
	CHECK(probe.record.tick_counter == 2 && probe.viewports == 1 && probe.draws == 1 && probe.deletes == 1);

	/* Coal mine subsidence reaches DisasterClearSquare through the daily entry. */
	OpenTTDCommandCostServices costs{};
	costs.construct = [](OpenTTDCommandCostStorage *p) noexcept { _disaster_cost_calls.push_back(1); return reinterpret_cast<OpenTTDCommandCost *>(p); };
	costs.destroy = [](OpenTTDCommandCost *) noexcept { _disaster_cost_calls.push_back(5); };
	costs.zero = [](OpenTTDCommandCost *p) noexcept { _disaster_cost_calls.push_back(3); *reinterpret_cast<uint32_t *>(p) = 0; };
	costs.error = [](OpenTTDCommandCost *p, uint32_t error) noexcept { CHECK(error == 0x87654321); _disaster_cost_calls.push_back(3); *reinterpret_cast<uint32_t *>(p) = error; };
	costs.success = [](const OpenTTDCommandCost *p) noexcept { _disaster_cost_calls.push_back(4); return *reinterpret_cast<const uint32_t *>(p) == 0; };
	const OpenTTDOccupancyCommands commands{.vehicles = &vehicles, .max_pixel_z = [](uint32_t tile) noexcept { CHECK(tile == 0); _disaster_cost_calls.push_back(2); return 0; }, .costs = &costs, .vehicle_error = 0x87654321};
	services.occupancy = &commands;
	services.disasters_enabled = []() noexcept { return true; };
	services.next_industry = [](uint32_t from) noexcept -> OpenTTDDisasterIndustryTarget {
		return from <= 7 ? OpenTTDDisasterIndustryTarget{.industry = _disaster_probe, .id = 7, .kind = 0} : OpenTTDDisasterIndustryTarget{.industry = nullptr, .id = 0xFFFF, .kind = IT_INVALID};
	};
	/* The target's type 0 can cause subsidence; Rust reads the spec record. */
	openttd_rust_industry_specs()[0].behaviour = IndustryBehaviours{IndustryBehaviour::CanSubsidence};
	services.industry_location = [](void *) noexcept { return OpenTTDDisasterIndustryLocation{.tile = 0, .town = 3}; };
	services.coal_news = [](uint32_t tile, uint16_t town) noexcept { CHECK(tile == 65 && town == 3); _disaster_probe->news++; };
	services.human = [](uint8_t company) noexcept { CHECK(company == 1); return true; };
	services.update_signals = []() noexcept { _disaster_probe->signals++; };
	services.next_disaster = [](uint32_t from) noexcept {
		auto *p = _disaster_probe;
		return from <= 17 ? OpenTTDDisasterVehicle{.handle = &p->record, .state = p->state, .id = 17} : OpenTTDDisasterVehicle{.handle = nullptr, .state = nullptr, .id = 1048575};
	};
	probe.record.dest_tile = 7;
	/* A nested entry from the clear command, as an industry destructor releases its attackers. */
	services.clear = [](uint32_t tile, uint8_t company) noexcept {
		auto *p = _disaster_probe;
		CHECK(tile == 0); p->clears++; p->clear_owner = company;
		openttd_rust_disaster_release_industry(p->services, 7);
	};
	/* Tile 0 is plain rail, not a depot, wherever the probe reports a railway. */
	static OpenTTDFixtureMap map{6, 4096};
	map.base[0].type = MP_RAILWAY << 4;
	shared.map = &map.statics;
	AutoRestoreBackup<decltype(GetRustClockState().calendar_year)> calendar_year_scope(GetRustClockState().calendar_year, decltype(GetRustClockState().calendar_year){1955});
	shared.tile_valid = [](uint32_t tile) noexcept { return tile == 0; };
	shared.tile_type = [](uint32_t tile) noexcept { CHECK(tile == 0); _disaster_probe->tile_types++; return _disaster_probe->tile_type; };
	shared.tile_owner = [](uint32_t tile) noexcept { CHECK(tile == 0); return uint8_t{1}; };
	/* Reset delay, choose the third of three disasters, the first subsidence industry and north-east. */
	auto subside = [&]() {
		probe.script = {0, UINT32_MAX, 0, 0}; probe.scripted = 0; probe.draws = 0; *delay = 1;
		_disaster_cost_calls.clear();
		openttd_rust_clock_disaster_daily(&services, nullptr);
		CHECK(probe.draws == 4 && probe.scripted == 4 && *delay == 730);
		CHECK(_disaster_cost_calls == std::vector<int>({1, 2, 3, 4, 5}));
	};
	probe.record.type = VEH_TRAIN;
	subside();
	CHECK(probe.news == 1 && probe.tile_types == 0 && probe.clears == 0);
	openttd_rust_occupancy_update(&vehicles, vehicle.handle, true);
	probe.record.subtype = 4; state->state = 1;
	probe.tile_type = MP_HOUSE;
	subside();
	CHECK(probe.news == 2 && probe.tile_types == 1 && probe.clears == 1 && probe.clear_owner == 0x10 && probe.signals == 0 && state->state == 3);
	probe.tile_type = MP_RAILWAY;
	subside();
	CHECK(probe.news == 3 && probe.tile_types == 2 && probe.clears == 2 && probe.clear_owner == 0x11 && probe.signals == 1);
	CHECK(probe.deletes == 1 && probe.viewports == 1);

	vehicles.can_allocate = [](size_t count) noexcept { CHECK(count >= 1 && count <= 3); return false; };
	struct CapacityCase { int32_t (*year)() noexcept; uint32_t random; };
	const CapacityCase cases[] = {
		{[]() noexcept { return 1930; }, 0},
		{[]() noexcept { return 1955; }, 0},
		{[]() noexcept { return 1965; }, 32768},
		{[]() noexcept { return 1990; }, 0},
		{[]() noexcept { return 2010; }, 0},
		{[]() noexcept { return 1940; }, 65535},
		{[]() noexcept { return 2000; }, 65535},
	};
	for (const auto &test : cases) {
		GetRustClockState().calendar_year = decltype(GetRustClockState().calendar_year){test.year()};
		probe.random = test.random; probe.draws = 0; *delay = 1;
		openttd_rust_clock_disaster_daily(&services, nullptr);
		CHECK(probe.draws == 2 && probe.deletes == 1 && probe.viewports == 1);
	}
	_disaster_probe = nullptr;
	std::printf("disaster_private_counter_direct_rng_delete_rotor_clear_reentry_capacity passed\n");
}
ABI_PROBE(Disasters);
