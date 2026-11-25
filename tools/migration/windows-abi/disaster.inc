#pragma once
#include "rust/disaster_ffi.h"
#include "rust/clock_ffi.h"
#include "industrytype.h"
#include "rust/industry_ffi.h"
#include "rust/industry_resolver_ffi.h"
#include "rust/company_identity_ffi.h"
#include "rust/news_ffi.h"
#include "rust/settings_table_ffi.h"
#include "core/backup_type.hpp"

struct DisasterProbe {
	OpenTTDVehicle record{};
	DisasterProbe() { this->record.shell = reinterpret_cast<OpenTTDVehicleShell *>(this); this->record.tick_counter = 255; this->record.subtype = 13; this->record.type = VEH_DISASTER; }
	uint32_t draws = 0, random = 0, viewports = 0, deletes = 0, news = 0;
	int32_t age = 8880;
	std::vector<uint32_t> script{};
	size_t scripted = 0;
};
static DisasterProbe *_disaster_probe = nullptr;
static int _disaster_town = 0;
static uint32_t DisasterProbeRandom(void *context) noexcept
{
	auto *p = static_cast<DisasterProbe *>(context);
	p->draws++;
	return p->scripted < p->script.size() ? p->script[p->scripted++] : p->random;
}
/** The encoded headline `GetEncodedString(STR_NEWS_DISASTER_COAL_MINE_SUBSIDENCE, 3)`. */
static std::string DisasterCoalHeadline()
{
	char id[16];
	std::snprintf(id, sizeof(id), "%x", static_cast<unsigned>(STR_NEWS_DISASTER_COAL_MINE_SUBSIDENCE));
	return std::string("\xEE\x80\x81") + id + "\x1E" + "\xEE\x80\x82" + "3";
}
static void Disasters()
{
	using StateOwner = std::unique_ptr<OpenTTDDisasterState, decltype(&openttd_rust_disaster_state_destroy)>;
	DisasterProbe probe;
	_disaster_probe = &probe;
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(&probe, DisasterProbeRandom);
	bool disasters = false;
	OpenTTDSettingAddresses settings{};
	settings.disasters = &disasters;
	shared.settings = &settings;
	static OpenTTDVehicleResolverServices resolver{};
	resolver.age = [](OpenTTDVehicle *v) noexcept { return reinterpret_cast<DisasterProbe *>(v->shell)->age; };
	static OpenTTDVehicleMaintenanceServices maintenance{};
	maintenance.set_age = [](OpenTTDVehicle *v, int32_t age) noexcept { reinterpret_cast<DisasterProbe *>(v->shell)->age = age; };
	OpenTTDVehicleServices vehicles{};
	vehicles.destroy = [](OpenTTDVehicle *v) noexcept { reinterpret_cast<DisasterProbe *>(v->shell)->deletes++; };
	vehicles.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	vehicles.maintenance = &maintenance;
	OpenTTDDisasterServices services{};
	services.shared = &shared;
	services.vehicles = &vehicles;
	StateOwner state(openttd_rust_disaster_state_create(), openttd_rust_disaster_state_destroy);
	probe.record.disaster = state.get();
	auto *address = state.get(); state->state = 65535; state->flags = 0xAB; state->image_override = UINT32_MAX;
	uint16_t *delay = openttd_rust_disaster_delay(); *delay = 0;
	openttd_rust_clock_disaster_daily(&services, nullptr); CHECK(*delay == 65535 && probe.draws == 0);
	*delay = 1; probe.random = 511;
	openttd_rust_clock_disaster_daily(&services, nullptr); CHECK(*delay == 1241 && probe.draws == 1);
	probe.random = 0;
	CHECK(delay == openttd_rust_disaster_delay() && state.get() == address && state->state == 65535 && state->flags == 0xAB && state->image_override == UINT32_MAX);
	CHECK(!openttd_rust_disaster_tick(&services, &probe.record));
	CHECK(probe.age == 8881 && probe.record.tick_counter == 0 && probe.deletes == 1 && probe.draws == 1);

	/* The tile index hashes the vehicle's tile with the 64-wide map's dimensions. */
	static OpenTTDFixtureMap map{6, 4096};
	shared.map = &map.statics;

	vehicles.base_viewport = [](OpenTTDVehicle *v) noexcept { auto *p = reinterpret_cast<DisasterProbe *>(v->shell); CHECK(p->record.hash_tile_current != nullptr); p->viewports++; };
	vehicles.first_sprite = [](OpenTTDVehicle *) noexcept { return 3904U; };
	vehicles.write_first_sprite = [](OpenTTDVehicle *, uint32_t image) noexcept { CHECK(image == 3902); };
	probe.record.tick_counter = 1; probe.record.subtype = 8;
	CHECK(openttd_rust_disaster_tick(&services, &probe.record));
	CHECK(probe.record.tick_counter == 2 && probe.viewports == 1 && probe.draws == 1 && probe.deletes == 1);

	/* Coal mine subsidence reaches DisasterClearSquare through the daily entry. The
	 * industry is one Rust owner of type 0 at tile 0, whose town is town 3. */
	disasters = true;
	static OpenTTDIndustry *const coal_mine = openttd_rust_industry_new();
	openttd_rust_industry_metadata(coal_mine)->tile = 0;
	openttd_rust_industry_metadata(coal_mine)->type = 0;
	static OpenTTDIndustryTransactionServices transaction{};
	transaction.town = [](OpenTTDIndustryShell *) noexcept { return reinterpret_cast<const OpenTTDTownHandle *>(&_disaster_town); };
	static OpenTTDIndustryResolverServices industry_resolver{};
	industry_resolver.town_id = [](const OpenTTDTownHandle *town) noexcept { CHECK(town == reinterpret_cast<const OpenTTDTownHandle *>(&_disaster_town)); return uint16_t{3}; };
	OpenTTDIndustryServices industries{};
	industries.resolver = &industry_resolver;
	industries.transaction = &transaction;
	industries.next_tick = [](uint32_t from) noexcept {
		return from <= 7 ? OpenTTDIndustryEntry{.industry = _disaster_probe, .owner = coal_mine, .id = 7} : OpenTTDIndustryEntry{.industry = nullptr, .owner = nullptr, .id = 0xFFFF};
	};
	services.industry = &industries;
	/* The target's type 0 can cause subsidence; Rust reads the spec record. */
	openttd_rust_industry_specs()[0].behaviour = IndustryBehaviours{IndustryBehaviour::CanSubsidence};
	/* The news table is process-wide; no other case of this probe binds one. */
	static const OpenTTDNewsServices news{
		.add_news_item = [](const OpenTTDAddNewsItemArgs *args) noexcept {
			CHECK(args->ref1.index == 1 && args->ref1.value == 65);
			CHECK(std::string(reinterpret_cast<const char *>(args->headline), args->headline_length) == DisasterCoalHeadline());
			_disaster_probe->news++;
		},
	};
	openttd_rust_news_bind(&news);
	/* Tile 0 is plain rail of company 1, and the probe's train stays in the tile hash on it,
	 * where the rotor tick's position update put it. The ground check fails before the tile
	 * is classified, so the owner's company is never resolved. */
	static OpenTTDCompanyIdentityServices identity{};
	identity.company = [](uint8_t) noexcept -> Company * { CHECK(false); return nullptr; };
	services.identity = &identity;
	map.base[0].type = MP_RAILWAY << 4;
	map.base[0].m1 = 1;
	AutoRestoreBackup<decltype(GetRustClockState().calendar_year)> calendar_year_scope(GetRustClockState().calendar_year, decltype(GetRustClockState().calendar_year){1955});
	/* Reset delay, choose the third of three disasters, the first subsidence industry and north-east. */
	probe.record.type = VEH_TRAIN;
	probe.script = {0, UINT32_MAX, 0, 0}; probe.scripted = 0; probe.draws = 0; *delay = 1;
	openttd_rust_clock_disaster_daily(&services, nullptr);
	CHECK(probe.draws == 4 && probe.scripted == 4 && *delay == 730);
	CHECK(probe.news == 1 && probe.deletes == 1 && probe.viewports == 1);

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
	std::printf("disaster_private_counter_direct_rng_delete_rotor_subsidence_capacity passed\n");
}
ABI_PROBE(Disasters);
