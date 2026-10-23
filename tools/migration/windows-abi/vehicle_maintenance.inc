#pragma once

#include "trees.inc"
#include "vehicle_tables.inc"

/* Explicit trivial Date and actual VehicleType carriers and aggregate return/write call ABI. Opaque
 * shells are never dereferenced; the game target checks real member stores. */
static void VehicleMaintenanceBoundary()
{
	static size_t dirty;
	static size_t resets;
	static size_t values;
	dirty = resets = values = 0;
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicleMaintenanceBreakdownProbe { return {.counter = 0, .status = 0}; };
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept { dirty++; };
	maintenance.reset = [](OpenTTDVehicle *) noexcept {
		return OpenTTDVehicleMaintenanceReset{.economy_date = OpenTTDEconomyDate{0x1020304}, .calendar_date = OpenTTDCalendarDate{-0x12345}, .reliability = 0x9876, .chance = 12, .breakdowns = 2};
	};
	maintenance.chance_write = [](OpenTTDVehicle *, uint8_t chance) noexcept { CHECK(chance == 3); resets++; };
	maintenance.value = [](OpenTTDVehicle *) noexcept { return int64_t{-257}; };
	maintenance.value_write = [](OpenTTDVehicle *, int64_t value) noexcept { CHECK(value == -255); values++; };
	maintenance.renewal = [](OpenTTDVehicle *, const Company *) noexcept {
		return OpenTTDVehicleMaintenanceRenewal{.age = OpenTTDCalendarDate{100}, .max_age = OpenTTDCalendarDate{100}, .months = -4, .enabled = false, .kind = static_cast<VehicleType>(254), .subtype = 0};
	};
	const auto shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	RoadTestTables tables(&shared);
	auto &base = tables.vehicle;
	base.maintenance = &maintenance;
	base.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	base.write_economy_service = [](OpenTTDVehicle *, int32_t date) noexcept { CHECK(date == 0x1020304); };
	base.write_calendar_service = [](OpenTTDVehicle *, int32_t date) noexcept { CHECK(date == -0x12345); };
	base.write_breakdowns = [](OpenTTDVehicle *, uint8_t since) noexcept { CHECK(since == 0); };
	base.write_reliability = [](OpenTTDVehicle *, uint16_t reliability) noexcept { CHECK(reliability == 0x9876); };
	uint8_t opaque;
	auto *shell = reinterpret_cast<OpenTTDVehicle *>(&opaque);
	openttd_rust_vehicle_service_depot(shell, &base);
	openttd_rust_vehicle_decrease_value(shell, &base);
	CHECK(openttd_rust_vehicle_needs_renew(shell, &base, nullptr, false));
	CHECK(!openttd_rust_vehicle_needs_renew(shell, &base, nullptr, true));
	CHECK(dirty == 2 && resets == 1 && values == 1);
}

/* Actual native storage type is checked by the game-side shell probe; this
 * standalone ABI target also exercises const BaseBitSet member identity and
 * native size_t/pointer cursor calls on every platform. */
static void VehicleSchedulerBoundary()
{
	using Status = decltype(std::declval<Vehicle>().vehstatus);
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
	static Vehicle *slots[5];
	static size_t live;
	static size_t size;
	std::array<uint8_t, 2> opaque{};
	slots[0] = reinterpret_cast<Vehicle *>(&opaque[0]);
	slots[1] = nullptr;
	slots[2] = reinterpret_cast<Vehicle *>(&opaque[1]);
	slots[3] = slots[4] = nullptr;
	live = size = 2;
	OpenTTDVehicleSchedulerServices services{};
	services.pool = []() noexcept { return OpenTTDVehiclePoolCursor{.slots = slots, .slots_len = size, .first_unused = &live}; };
	auto cursor = services.pool();
	size_t from = 0;
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == reinterpret_cast<OpenTTDVehicle *>(slots[0]));
	CHECK(from == 1);
	live = 3;
	size = 5;
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == reinterpret_cast<OpenTTDVehicle *>(slots[2]));
	CHECK(cursor.slots_len == 5);
	CHECK(cursor.first_unused == &live);
	CHECK(from == 3);
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == nullptr);
}

/* These initialize the maintenance table consumed by the road and aircraft probes. */
static void VehicleMaintenanceProbes()
{
	VehicleSchedulerBoundary();
	VehicleMaintenanceBoundary();
}
ABI_PROBE_FIRST(VehicleMaintenanceProbes);
