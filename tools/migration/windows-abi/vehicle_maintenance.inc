#pragma once

#include "trees.inc"
#include "vehicle_tables.inc"
#include "company_base.h"
#include "rust/company_ffi.h"
#include "rust/company_identity_ffi.h"
#include "rust/engine_capabilities_ffi.h"
#include "rust/fleet_ffi.h"

/* Explicit Date/VehicleType carriers and aggregate call ABI. Engine bindings use a live Rust
 * record; opaque company shells are never dereferenced. The game target checks member stores. */
static void VehicleMaintenanceBoundary()
{
	static size_t dirty;
	static size_t values;
	dirty = values = 0;
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicleMaintenanceBreakdownProbe { return {.counter = 0, .status = 0}; };
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept { dirty++; };
	maintenance.value = [](OpenTTDVehicle *) noexcept { return int64_t{-257}; };
	maintenance.value_write = [](OpenTTDVehicle *, int64_t value) noexcept { CHECK(value == -255); values++; };
	maintenance.renewal = [](OpenTTDVehicle *, const Company *) noexcept {
		return OpenTTDVehicleMaintenanceRenewal{.age = OpenTTDCalendarDate{100}, .max_age = OpenTTDCalendarDate{100}, .months = -4, .enabled = false, .kind = static_cast<VehicleType>(254), .subtype = 0};
	};
	const auto shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	RoadTestTables tables(&shared);
	static OpenTTDEngine *engine_record;
	engine_record = openttd_rust_engine_new();
	const OpenTTDEngineDefaults engine_defaults{};
	openttd_rust_engine_construct(engine_record, &engine_defaults, VEH_ROAD, UINT16_MAX);
	engine_record->vehicle_info.road.cost_factor = 53;
	engine_record->company_avail.Set(CompanyID{0});
	auto &base = tables.vehicle;
	base.engine = [](OpenTTDVehicle *) noexcept -> const OpenTTDEngine * { return engine_record; };
	base.maintenance = &maintenance;
	OpenTTDVehicle record{};
	record.type = VEH_ROAD;
	auto *shell = &record;
	openttd_rust_vehicle_decrease_value(shell, &base);
	CHECK(openttd_rust_vehicle_needs_renew(shell, &base, nullptr, false));
	CHECK(!openttd_rust_vehicle_needs_renew(shell, &base, nullptr, true));
	CHECK(dirty == 1 && values == 1);
	/* Replacement budgets use the Rust engine price before observing available money. */
	static OpenTTDEngineCapabilitiesServices capabilities{};
	static OpenTTDVehicleResolverServices resolver{};
	static uint32_t stage;
	static uint16_t factor;
	static int64_t available;
	static int company_shell;
	static CompanyFinances *active_finances, *after_finances;
	static bool infinite, after_infinite;
	static uint8_t supplied_id;
	CompanyFinanceOwner initial, changed;
	after_finances = changed.state;
	OpenTTDCompanyIdentityServices company{};
	company.renew_reserve = [](const Company *c) noexcept -> uint32_t { CHECK(c == reinterpret_cast<const Company *>(&company_shell) && stage++ == 0); return 100; };
	company.company_id = [](const Company *c) noexcept -> uint8_t { CHECK(c == reinterpret_cast<const Company *>(&company_shell) && (stage == 1 || stage == 6)); ++stage; return supplied_id; };
	OpenTTDCompanyFinanceServices finance{};
	finance.company_finances = [](uint8_t id) noexcept -> CompanyFinances * { CHECK(id == supplied_id && (stage == 2 || stage == 7)); ++stage; return id == COMPANY_SPECTATOR.base() ? nullptr : active_finances; };
	OpenTTDCommandServices commands{}; commands.finance = &finance;
	commands.infinite_money = []() noexcept { CHECK(stage == 3 || stage == 8); ++stage; return infinite; };
	maintenance.company = &company;
	resolver.engine_by_id = [](uint16_t id) noexcept -> const OpenTTDEngine * { CHECK(id == 123); return engine_record; };
	resolver.resolve_callback = [](OpenTTDVehicleResolverState *, uint16_t id, uint16_t callback, uint32_t property, uint32_t, int32_t *, size_t) noexcept -> uint16_t {
		CHECK(stage++ == 4 && id == 123 && callback == CBID_VEHICLE_MODIFY_PROPERTY && property == 0x11);
		active_finances = after_finances;
		active_finances->money = Money{available};
		infinite = after_infinite;
		return factor;
	};
	static OpenTTDVehicleServices vehicles{}; vehicles.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; }; capabilities.vehicles = &vehicles;
	capabilities.engine = [](uint16_t) noexcept -> const OpenTTDEngine * { return engine_record; };
	capabilities.id = [](const OpenTTDEngine *) noexcept -> uint16_t { return 123; };
	capabilities.shared = &tables.shared;
	tables.shared.grf_price_shift = [](const GRFFile *, uint32_t index) noexcept -> int32_t { CHECK(stage++ == 5 && index == PR_BUILD_VEHICLE_ROAD); return 0; };
	OpenTTDFleetTransactionServices transactions{}; transactions.capabilities = &capabilities; transactions.commands = &commands;
	maintenance.transaction = &transactions;

	maintenance.interval = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicleMaintenanceInterval {
		return {.company = reinterpret_cast<const Company *>(&company_shell), .last_service = {0}, .now = {100}, .interval = 1, .reliability = 0, .percent = false, .wallclock = false, .no_service = true, .breakdowns = 0};
	};
	maintenance.replacement = [](OpenTTDVehicle *, const Company *) noexcept -> OpenTTDVehicleMaintenanceReplacement { return {.engine = 1, .when_old = false}; };
	const int64_t original_price = openttd_rust_economy_prices()[PR_BUILD_VEHICLE_ROAD];
	openttd_rust_economy_prices()[PR_BUILD_VEHICLE_ROAD] = 25600;
	for (uint16_t value : {uint16_t{CALLBACK_FAILED}, uint16_t{17}}) {
		factor = value;
		const int64_t needed = value == CALLBACK_FAILED ? 10700 : 3500;
		for (bool funded : {false, true}) {
			available = needed - !funded; stage = 0;
			active_finances = initial.state; initial.state->money = Money{1000000};
			supplied_id = 7; infinite = after_infinite = false;
			CHECK(openttd_rust_vehicle_needs_service(shell, &base) == funded);
			CHECK(stage == 9);
		}
	}
	/* The initial reserve check precedes property evaluation at the same boundary. */
	for (int64_t money : {INT64_C(99), INT64_C(100)}) {
		active_finances = initial.state; initial.state->money = Money{money};
		supplied_id = 7; infinite = after_infinite = false; available = 3500; factor = 17; stage = 0;
		CHECK(openttd_rust_vehicle_needs_service(shell, &base) == (money == 100));
		CHECK(stage == (money == 100 ? 9 : 4));
	}
	for (bool invalid : {false, true}) {
		active_finances = initial.state; initial.state->money = Money{1000000};
		supplied_id = invalid ? COMPANY_SPECTATOR.base() : 7; infinite = false; after_infinite = !invalid;
		available = -1; stage = 0;
		CHECK(openttd_rust_vehicle_needs_service(shell, &base));
		CHECK(stage == 9);
	}
	openttd_rust_economy_prices()[PR_BUILD_VEHICLE_ROAD] = original_price;
	maintenance.company = nullptr; maintenance.transaction = nullptr;
	base.engine = nullptr;
	auto *retired = std::exchange(engine_record, nullptr);
	openttd_rust_engine_destroy(retired);
}

/* Actual native storage type is checked by the game-side shell probe; this
 * standalone ABI target also exercises const BaseBitSet member identity and
 * native size_t/pointer cursor calls on every platform. */
static void VehicleSchedulerBoundary()
{
	using Status = decltype(std::declval<Vehicle>().GetVehStatus());
	static_assert(std::is_same_v<Status::BaseType, uint8_t>);
	Status status;
	const uint8_t *address = status.base_address();
	static_assert(std::is_same_v<decltype(status.base_address()), const uint8_t *>);
	CHECK(address == status.base_address());
	status.Set(VehState::Hidden);
	CHECK(*address == status.base());
	status.Set(VehState::Crashed);
	CHECK(*address == 129);
	status.Reset(VehState::Hidden);
	CHECK(*address == 128);
	/* Opaque shells are never dereferenced: this target tests the pointer/size_t
	 * call ABI of the pool service through the production occupancy reset, which
	 * clears the current position of each live record below first_unused. */
	static RustPoolSlot slots[5];
	static size_t live;
	static size_t size;
	std::array<uint8_t, 2> opaque{};
	slots[0].shell = &opaque[0];
	slots[1] = {};
	slots[2].shell = &opaque[1];
	slots[3] = slots[4] = {};
	live = 3;
	size = 5;
	static OpenTTDVehicle records[2]{};
	static OpenTTDVehicleTileLinks links[2]{};
	static OpenTTDVehicle *position = nullptr;
	links[0].current = links[1].current = &position;
	OpenTTDVehicleServices base{};
	base.tile_links = [](OpenTTDVehicle *v) noexcept { return &links[v - records]; };
	slots[0].record = &records[0];
	slots[2].record = &records[1];
	OpenTTDVehicleSchedulerServices services{};
	services.vehicle = &base;
	services.pool = []() noexcept { return OpenTTDVehiclePoolCursor{.slots = slots, .slots_len = size, .first_unused = &live}; };
	openttd_rust_occupancy_reset(&services);
	CHECK(links[0].current == nullptr);
	CHECK(links[1].current == nullptr);
}

/* These initialize the maintenance table consumed by the road and aircraft probes. */
static void VehicleMaintenanceProbes()
{
	VehicleSchedulerBoundary();
	VehicleMaintenanceBoundary();
}
ABI_PROBE_FIRST(VehicleMaintenanceProbes);
