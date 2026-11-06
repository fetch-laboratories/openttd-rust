#pragma once

#include "trees.inc"
#include "vehicle_tables.inc"
#include "company_base.h"
#include "rust/company_ffi.h"
#include "rust/company_identity_ffi.h"
#include "rust/engine_capabilities_ffi.h"
#include "rust/fleet_ffi.h"

/* Explicit trivial Date and actual VehicleType carriers and aggregate return/write call ABI. Opaque
 * shells are never dereferenced; the game target checks real member stores. */
static void VehicleMaintenanceBoundary()
{
	/* Actual native Company-member facade copy/move and Rust bitmap call ABI. */
	FreeUnitIDGenerator numbers;
	CHECK(numbers.NextID() == 1 && numbers.UseID(65) == 65);
	CHECK(numbers.UseID(1) == 1 && numbers.NextID() == 2);
	FreeUnitIDGenerator copied = numbers;
	CHECK(copied.UseID(2) == 2 && numbers.NextID() == 2);
	numbers = copied;
	CHECK(numbers.NextID() == 3);
	numbers.ReleaseID(1);
	CHECK(numbers.NextID() == 1 && copied.NextID() == 3);
	FreeUnitIDGenerator moved = std::move(copied);
	CHECK(moved.NextID() == 3 && copied.NextID() == 1);

	static size_t dirty;
	dirty = 0;
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept { dirty++; };
	maintenance.reset = [](OpenTTDVehicle *) noexcept {
		return OpenTTDVehicleMaintenanceReset{.economy_date = OpenTTDEconomyDate{0x1020304}, .calendar_date = OpenTTDCalendarDate{-0x12345}, .reliability = 0x9876, .chance = 12, .breakdowns = 2};
	};
	maintenance.renewal = [](OpenTTDVehicle *, const Company *) noexcept {
		return OpenTTDVehicleMaintenanceRenewal{.age = OpenTTDCalendarDate{100}, .max_age = OpenTTDCalendarDate{100}, .months = -4, .enabled = false, .kind = static_cast<VehicleType>(254), .subtype = 0};
	};
	const auto shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	RoadTestTables tables(&shared);
	auto &base = tables.vehicle;
	base.maintenance = &maintenance;
	OpenTTDVehicle record{};
	record.type = VEH_ROAD;
	record.value = -257;
	auto *shell = &record;
	openttd_rust_vehicle_service_depot(shell, &base);
	CHECK(record.date_of_last_service == 0x1020304 && record.date_of_last_service_newgrf == -0x12345);
	CHECK(record.breakdowns_since_last_service == 0 && record.reliability == 0x9876 && record.breakdown_chance == 3);
	openttd_rust_vehicle_decrease_value(shell, &base);
	CHECK(record.value == -255);
	CHECK(openttd_rust_vehicle_needs_renew(shell, &base, nullptr, false));
	CHECK(!openttd_rust_vehicle_needs_renew(shell, &base, nullptr, true));
	CHECK(dirty == 2);
	/* Replacement budgets use the Rust engine price before observing available money. */
	static OpenTTDEngineCapabilitiesServices capabilities{};
	static OpenTTDVehicleResolverServices resolver{};
	static uint32_t stage;
	static uint16_t factor;
	static int64_t available;
	static int engine_shell, company_shell;
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
	resolver.spritegroup = [](uint16_t, uint8_t) noexcept -> const SpriteGroup * { return nullptr; };
	resolver.resolve_callback = [](OpenTTDVehicleResolverState *, uint16_t id, uint16_t callback, uint32_t property, uint32_t, int32_t *, size_t) noexcept -> uint16_t {
		CHECK(stage++ == 4 && id == 123 && callback == CBID_VEHICLE_MODIFY_PROPERTY && property == 0x11);
		active_finances = after_finances;
		active_finances->money = Money{available};
		infinite = after_infinite;
		return factor;
	};
	capabilities.resolver = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	capabilities.engine = [](uint16_t) noexcept { return reinterpret_cast<const OpenTTDEngine *>(&engine_shell); };
	capabilities.kind = [](const OpenTTDEngine *) noexcept { return VEH_ROAD; };
	capabilities.id = [](const OpenTTDEngine *) noexcept -> uint16_t { return 123; };
	capabilities.ground = [](const OpenTTDEngine *) noexcept { return false; };
	capabilities.carrying = [](const OpenTTDEngine *) noexcept -> OpenTTDEngineCarrying { return {.kind = VEH_ROAD, .ground_capacity = 0, .default_cargo = INVALID_CARGO}; };
	capabilities.road_cost = [](const OpenTTDEngine *) noexcept -> uint8_t { return 53; };
	capabilities.price_shift = [](const OpenTTDEngine *, uint32_t index) noexcept -> int32_t { CHECK(stage++ == 5 && index == PR_BUILD_VEHICLE_ROAD); return 0; };
	OpenTTDFleetTransactionServices transactions{}; transactions.capabilities = &capabilities; transactions.commands = &commands;
	maintenance.transaction = &transactions;

	maintenance.interval = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicleMaintenanceInterval {
		return {.company = reinterpret_cast<const Company *>(&company_shell), .last_service = {0}, .now = {100}, .interval = 1, .reliability = 0, .percent = false, .wallclock = false, .no_service = true, .breakdowns = 0};
	};
	maintenance.replacement = [](OpenTTDVehicle *, const Company *) noexcept -> OpenTTDVehicleMaintenanceReplacement { return {.engine = 1, .when_old = false}; };
	maintenance.replacement_available = [](OpenTTDVehicle *, uint16_t) noexcept { return true; };
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
}

/* Actual native storage type is checked by the game-side shell probe; this
 * standalone ABI target also exercises const BaseBitSet member identity and
 * native size_t/pointer cursor calls on every platform. */
static void VehicleSchedulerBoundary()
{
	using Status = decltype(std::declval<Vehicle>().GetVehStatus());
	static_assert(std::is_same_v<Status::BaseType, uint8_t>);
	static_assert(std::is_same_v<decltype(RustVehicleStatusAddress(nullptr)), const uint8_t *>);
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
	 * call ABI; the game target compares real pool mutations to PoolIterator. */
	static RustPoolSlot slots[5];
	std::array<uint8_t, 2> opaque{};
	slots[0].shell = &opaque[0];
	slots[1] = {};
	slots[2].shell = &opaque[1];
	slots[3] = slots[4] = {};
	static OpenTTDVehicle records[2]{};
	slots[0].record = &records[0];
	slots[2].record = &records[1];
	RustPoolDescriptor descriptor{};
	descriptor.slots = {slots, 2, 0};
	descriptor.first_unused = 2;
	const RustPoolDescriptor *const owner = &descriptor;
	OpenTTDVehicleServices base{};
	base.pool = &owner;
	size_t from = 0;
	CHECK(openttd_rust_vehicle_cursor_next(&base, &from) == &records[0]);
	CHECK(from == 1);
	descriptor.first_unused = 3;
	descriptor.slots.count = 5;
	CHECK(openttd_rust_vehicle_cursor_next(&base, &from) == &records[1]);
	CHECK(from == 3);
	CHECK(openttd_rust_vehicle_cursor_next(&base, &from) == nullptr);
}

/* These initialize the maintenance table consumed by the road and aircraft probes. */
static void VehicleMaintenanceProbes()
{
	VehicleSchedulerBoundary();
	VehicleMaintenanceBoundary();
}
ABI_PROBE_FIRST(VehicleMaintenanceProbes);
