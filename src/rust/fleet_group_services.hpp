/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "vehicle_services.h"
#include "vehicle_transaction_ffi.h"

/* The group table: the Group and EngineRenew pool operations, GroupStatistics::Get and the
 * Vehicle members still stored in the C++ shell. All wrappers are noexcept. */
static OpenTTDFleetGroupRecord FleetGroup_record(Group *g) noexcept
{
	if (g == nullptr) return {nullptr, nullptr, nullptr, 0};
	return {g, g->state.state, g->statistics.state.state, g->index.base()};
}
static void FleetGroup_group_delete(void *shell) noexcept { delete static_cast<Group *>(shell); }
static OpenTTDFleetGroupRecord FleetGroup_group_get(uint16_t index) noexcept { return FleetGroup_record(Group::Get(GroupID(index))); }
static OpenTTDFleetGroupRecord FleetGroup_group_get_if_valid(uint16_t index) noexcept { return FleetGroup_record(Group::GetIfValid(GroupID(index))); }
static OpenTTDFleetGroupRecord FleetGroup_group_new(uint8_t owner, uint8_t vt) noexcept { return FleetGroup_record(new Group(CompanyID(owner), static_cast<VehicleType>(vt))); }
static OpenTTDFleetGroupRecord FleetGroup_next_group(uint32_t from) noexcept { for (Group *g : Group::Iterate(from)) { if (g->index.base() >= from) return FleetGroup_record(g); } return FleetGroup_record(nullptr); }
static void * FleetGroup_group_statistics_get(uint8_t company, uint16_t id_g, uint8_t type) noexcept { return GroupStatistics::Get(CompanyID(company), GroupID(id_g), static_cast<VehicleType>(type)).state.state; }
static OpenTTDFleetRenewRecord FleetGroup_next_renew(uint32_t from) noexcept { for (EngineRenew *er : EngineRenew::Iterate(from)) { if (er->index.base() >= from) return {er, er->index.base()}; } return {nullptr, 0}; }
static void FleetGroup_renew_delete(void *shell) noexcept { delete static_cast<EngineRenew *>(shell); }
static void * FleetGroup_renew_new() noexcept { return new EngineRenew(); }
static void * FleetGroup_renew_state(void *shell) noexcept { return static_cast<EngineRenew *>(shell)->state.state; }
static bool FleetGroup_is_engine_countable(OpenTTDVehicle *v) noexcept { return VehicleOf(v)->IsEngineCountable(); }
const OpenTTDFleetGroupServices &FleetGroupServices()
{
	static const OpenTTDFleetGroupServices services{
		.selection = []() noexcept { return &GetRustVehicleSelectionServices(); },
		.shared = &GetRustSharedServices(),
		.vehicles = &GetRustVehicleServices(),
		.group_can_allocate = []() noexcept { return Group::CanAllocateItem(); },
		.group_delete = FleetGroup_group_delete,
		.group_get = FleetGroup_group_get,
		.group_get_if_valid = FleetGroup_group_get_if_valid,
		.group_new = FleetGroup_group_new,
		.next_group = FleetGroup_next_group,
		.group_statistics_get = FleetGroup_group_statistics_get,
		.next_renew = FleetGroup_next_renew,
		.renew_can_allocate = []() noexcept { return EngineRenew::CanAllocateItem(); },
		.renew_delete = FleetGroup_renew_delete,
		.renew_new = FleetGroup_renew_new,
		.renew_state = FleetGroup_renew_state,
		.group_id = RustVehicleGroupID,
		.is_engine_countable = FleetGroup_is_engine_countable,
		.set_group_id = RustVehicleSetGroupID,
	};
	return services;
}
