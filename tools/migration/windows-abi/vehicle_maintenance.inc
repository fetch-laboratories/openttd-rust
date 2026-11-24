#pragma once

#include "trees.inc"
#include "vehicle_tables.inc"
#include "rust/company_identity_ffi.h"
#include "rust/window_ffi.h"
#include "window_type.h"

/* The window services every vehicle probe dirties through; the first binding stays for the
 * process. */
static size_t _vehicle_maintenance_details_dirty;
static void BindVehicleMaintenanceWindows()
{
	static OpenTTDWindowServices windows{};
	windows.set_window_dirty = [](uint16_t cls, int32_t) noexcept {
		if (cls == WC_VEHICLE_DETAILS) ++_vehicle_maintenance_details_dirty;
	};
	openttd_rust_window_bind(&windows);
}

/* The member slots' calls: one value store and one details dirty, and the renew setting's
 * two results for the owner the company slot returns. */
static void VehicleMaintenanceBoundary()
{
	BindVehicleMaintenanceWindows();
	static size_t values;
	values = _vehicle_maintenance_details_dirty = 0;
	static int company_shell;
	auto &maintenance = _vehicle_maintenance_probe;
	static OpenTTDVehicleResolverServices resolver{};
	resolver.value = [](OpenTTDVehicle *) noexcept { return int64_t{-257}; };
	resolver.id = [](OpenTTDVehicle *) noexcept -> uint32_t { return 7; };
	resolver.age = [](OpenTTDVehicle *) noexcept -> int32_t { return 100; };
	resolver.max_age = [](OpenTTDVehicle *) noexcept -> int32_t { return 100; };
	maintenance.set_value = [](OpenTTDVehicle *, int64_t value) noexcept { CHECK(value == -255); values++; };
	static OpenTTDCompanyIdentityServices company{};
	company.company = [](uint8_t id) noexcept -> Company * { CHECK(id == 0); return reinterpret_cast<Company *>(&company_shell); };
	company.engine_renew = [](const Company *c) noexcept { CHECK(c == reinterpret_cast<const Company *>(&company_shell)); return false; };
	company.engine_renew_months = [](const Company *c) noexcept -> int16_t { CHECK(c == reinterpret_cast<const Company *>(&company_shell)); return -4; };
	maintenance.company = &company;
	const auto shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	RoadTestTables tables(&shared);
	auto &base = tables.vehicle;
	base.newgrf = []() noexcept -> const OpenTTDVehicleResolverServices * { return &resolver; };
	OpenTTDVehicle record{};
	record.type = VEH_ROAD;
	auto *shell = &record;
	openttd_rust_vehicle_decrease_value(shell, &base);
	const auto *owner = reinterpret_cast<const Company *>(&company_shell);
	CHECK(openttd_rust_vehicle_needs_renew(shell, &base, owner, false));
	CHECK(!openttd_rust_vehicle_needs_renew(shell, &base, owner, true));
	CHECK(_vehicle_maintenance_details_dirty == 1 && values == 1);
	maintenance.company = nullptr;
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
