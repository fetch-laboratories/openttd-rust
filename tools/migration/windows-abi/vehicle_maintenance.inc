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
	const auto shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom);
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

/* These initialize the maintenance table consumed by the road and aircraft probes. */
static void VehicleMaintenanceProbes()
{
	VehicleMaintenanceBoundary();
}
ABI_PROBE_FIRST(VehicleMaintenanceProbes);
