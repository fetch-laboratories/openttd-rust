/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

#include "vehicle_services.h"
#include "engine_services.h"
#include "company_services.h"
#include "company_identity_services.h"
#include "presentation_services.h"

struct FleetVehicleListContext { VehicleList list; const VehicleListIdentifier &vli; };
/* Fleet shared-world services. No selected group algorithm remains here.
 * All wrappers are noexcept; ordinary commands retain native stack frames.
 * Rust ends canonical field accesses before callbacks/reentry/destruction. */
static void * FleetGroup_group(uint16_t id) noexcept { auto *g = Group::GetIfValid(GroupID(id)); return g == nullptr ? nullptr : g->state.state; }
static uint32_t FleetGroup_next_group(uint32_t from) noexcept { for (Group *g : Group::Iterate(from)) { if (g->index.base() >= from) return g->index.base(); } return UINT32_MAX; }
static void * FleetGroup_vehicle(uint32_t id) noexcept { return Vehicle::GetIfValid(VehicleID(id)); }
static uint8_t FleetGroup_vehicle_type(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->type; }
static uint16_t FleetGroup_vehicle_group(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->group_id.base(); }
static uint16_t FleetGroup_vehicle_engine(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return RustVehicleEngineID(v); }
static bool FleetGroup_countable(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->IsEngineCountable(); }
static void * FleetGroup_first_shared(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->FirstShared(); }
static void * FleetGroup_next_shared(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); return v->NextShared(); }
static void FleetGroup_set_membership(void *shell, uint16_t id) noexcept { static_cast<Vehicle *>(shell)->group_id = GroupID(id); }
static void FleetGroup_invalidate_cache(void *shell) noexcept { auto *v = static_cast<Vehicle *>(shell); v->colourmap = PAL_NONE; RustVehicleInvalidateNewGRFCache(v); }
static void FleetGroup_viewport(void *shell) noexcept { static_cast<Vehicle *>(shell)->UpdateViewport(true); }
static void * FleetGroup_stats(uint8_t company, uint16_t id, uint8_t type) noexcept { return GroupStatistics::Get(CompanyID(company), GroupID(id), static_cast<VehicleType>(type)).state.state; }
static void * FleetGroup_head(uint8_t company) noexcept { return Company::Get(CompanyID(company))->renewal_head.state; }
static void * FleetGroup_renew_state(void *shell) noexcept { return static_cast<EngineRenew *>(shell)->state.state; }
static void * FleetGroup_next_renew(uint32_t from) noexcept { for (EngineRenew *er : EngineRenew::Iterate(from)) { if (er->index.base() >= from) return er; } return nullptr; }
static uint32_t FleetGroup_renew_id(void *shell) noexcept { return static_cast<EngineRenew *>(shell)->index.base(); }
static bool FleetGroup_buildable_type(uint8_t type) noexcept { return IsCompanyBuildableVehicleType(static_cast<VehicleType>(type)); }
static bool FleetGroup_can_allocate() noexcept { return Group::CanAllocateItem(); }
static uint16_t FleetGroup_allocate(uint8_t owner, uint8_t type) noexcept { return (new Group(CompanyID(owner), static_cast<VehicleType>(type)))->index.base(); }
static uint16_t FleetGroup_use_number(uint8_t owner) noexcept { auto *c = Company::Get(CompanyID(owner)); return c->freegroups.UseID(c->freegroups.NextID()); }
static void FleetGroup_release_number(uint8_t owner, uint16_t number) noexcept { Company::Get(CompanyID(owner))->freegroups.ReleaseID(number); }
static bool FleetGroup_keep_length(uint8_t owner) noexcept { return Company::Get(CompanyID(owner))->settings.renew_keep_length; }
static void FleetGroup_delete_group(uint16_t id) noexcept { delete Group::Get(GroupID(id)); }
static void FleetGroup_invalid_parent(uint16_t id, uint16_t parent) noexcept { Debug(misc, 2, "Group {} has invalid parent {}", GroupID(id), GroupID(parent)); }
static void FleetGroup_clear_backup(uint16_t id) noexcept { OrderBackup::ClearGroup(GroupID(id)); }
static void FleetGroup_remove_rule(uint8_t owner, uint16_t engine, uint16_t id, uint32_t flags) noexcept { RemoveEngineReplacementForCompany(Company::Get(CompanyID(owner)), EngineID(engine), GroupID(id), DoCommandFlags(static_cast<uint16_t>(flags))); }
static void FleetGroup_remove_vehicles(uint16_t id, uint32_t flags) noexcept { Command<CMD_REMOVE_ALL_VEHICLES_GROUP>::Do(DoCommandFlags(static_cast<uint16_t>(flags)), GroupID(id)); }
static void FleetGroup_delete_child(uint16_t id, uint32_t flags) noexcept { Command<CMD_DELETE_GROUP>::Do(DoCommandFlags(static_cast<uint16_t>(flags)), GroupID(id)); }
static void FleetGroup_add_to_group(uint16_t id, uint32_t vehicle, uint32_t flags) noexcept { Command<CMD_ADD_VEHICLE_GROUP>::Do(DoCommandFlags(static_cast<uint16_t>(flags)), GroupID(id), VehicleID(vehicle), false, VehicleListIdentifier{}); }
static void FleetGroup_list_dirty(uint8_t owner, uint8_t type) noexcept { InvalidateWindowData(GetWindowClassForVehicleType(static_cast<VehicleType>(type)), VehicleListIdentifier(VL_GROUP_LIST, static_cast<VehicleType>(type), CompanyID(owner)).ToWindowNumber()); }
static void FleetGroup_list_set_dirty(uint8_t owner, uint8_t type) noexcept { SetWindowDirty(GetWindowClassForVehicleType(static_cast<VehicleType>(type)), VehicleListIdentifier(VL_GROUP_LIST, static_cast<VehicleType>(type), CompanyID(owner)).ToWindowNumber()); }
static void FleetGroup_colour_dirty(uint8_t owner, uint8_t type) noexcept { InvalidateWindowData(WC_COMPANY_COLOUR, CompanyID(owner), static_cast<VehicleType>(type)); }
static void FleetGroup_replace_dirty(uint8_t type) noexcept { SetWindowDirty(WC_REPLACE_VEHICLE, type); }
static void FleetGroup_replace_invalidate(uint8_t type) noexcept { InvalidateWindowData(WC_REPLACE_VEHICLE, type); }
static void FleetGroup_alter_dirty(uint8_t type) noexcept { InvalidateWindowData(WC_REPLACE_VEHICLE, type, 1); RustInvalidateWindowClassesData(WC_VEHICLE_VIEW); RustInvalidateWindowClassesData(WC_VEHICLE_DETAILS); }
static void FleetGroup_vehicle_dirty(uint32_t id) noexcept { RustVehicleInvalidateView(VehicleID(id)); InvalidateWindowData(WC_VEHICLE_DETAILS, VehicleID(id)); }
static void FleetGroup_close_replace(uint8_t type) noexcept { CloseWindowById(WC_REPLACE_VEHICLE, type); }
static bool FleetGroup_list_generate(void *context) noexcept { auto *c = static_cast<FleetVehicleListContext *>(context); return GenerateVehicleSortList(&c->list, c->vli); }
static size_t FleetGroup_list_size(void *context) noexcept { return static_cast<FleetVehicleListContext *>(context)->list.size(); }
static void * FleetGroup_list_at(void *context, size_t index) noexcept { return const_cast<Vehicle *>(static_cast<FleetVehicleListContext *>(context)->list[index]); }
static void * FleetGroup_renew_allocate() noexcept { return new EngineRenew(); }
static bool FleetGroup_renew_can_allocate() noexcept { return EngineRenew::CanAllocateItem(); }
static void FleetGroup_renew_delete(void *shell) noexcept { delete static_cast<EngineRenew *>(shell); }
const OpenTTDFleetGroupServices &FleetGroupServices()
{
	static const OpenTTDFleetGroupServices services{
		.group = FleetGroup_group,
		.next_group = FleetGroup_next_group,
		.next_company = RustNextCompanyID,
		.next_vehicle = GetRustSharedServices().vehicle_next,
		.vehicle = FleetGroup_vehicle,
		.vehicle_id = OpenTTDMapQuery<uint32_t, RustVehicleIndexOpaque, void *>,
		.vehicle_type = FleetGroup_vehicle_type,
		.vehicle_owner = RustOpaqueHandleRead<uint8_t, RustVehicleOwnerOf>,
		.vehicle_group = FleetGroup_vehicle_group,
		.vehicle_engine = FleetGroup_vehicle_engine,
		.profit = RustOpaqueVehicleRead<int64_t, RustVehicleDisplayProfitLastYear>,
		.old_enough = RustOpaqueVehicleRead<bool, RustVehicleOldEnough>,
		.primary = RustOpaqueVehicleRead<bool, RustVehiclePrimary>,
		.countable = FleetGroup_countable,
		.ground = RustOpaqueVehicleRead<bool, RustVehicleIsGround>,
		.front = RustOpaqueVehicleRead<bool, RustVehicleFront>,
		.next_part = RustOpaqueVehicleRead<void *, RustVehicleNext>,
		.first_shared = FleetGroup_first_shared,
		.next_shared = FleetGroup_next_shared,
		.set_membership = FleetGroup_set_membership,
		.invalidate_cache = FleetGroup_invalidate_cache,
		.viewport = FleetGroup_viewport,
		.stats = FleetGroup_stats,
		.head = FleetGroup_head,
		.renew_state = FleetGroup_renew_state,
		.next_renew = FleetGroup_next_renew,
		.renew_id = FleetGroup_renew_id,
		.engine_type = RustIndexedEngineRead<uint8_t, RustEngineType>,
		.current_company = RustCurrentCompanyID,
		.buildable_type = FleetGroup_buildable_type,
		.can_allocate = FleetGroup_can_allocate,
		.allocate = FleetGroup_allocate,
		.use_number = FleetGroup_use_number,
		.release_number = FleetGroup_release_number,
		.company_livery = Identity_default_livery,
		.keep_length = FleetGroup_keep_length,
		.delete_group = FleetGroup_delete_group,
		.invalid_parent = FleetGroup_invalid_parent,
		.clear_backup = FleetGroup_clear_backup,
		.remove_rule = FleetGroup_remove_rule,
		.remove_vehicles = FleetGroup_remove_vehicles,
		.delete_child = FleetGroup_delete_child,
		.add_to_group = FleetGroup_add_to_group,
		.utf8_length = GetRustSharedServices().utf8_length,
		.list_dirty = FleetGroup_list_dirty,
		.list_set_dirty = FleetGroup_list_set_dirty,
		.colour_dirty = FleetGroup_colour_dirty,
		.replace_dirty = FleetGroup_replace_dirty,
		.replace_invalidate = FleetGroup_replace_invalidate,
		.alter_dirty = FleetGroup_alter_dirty,
		.vehicle_dirty = FleetGroup_vehicle_dirty,
		.depot_dirty = RustVehicleDirtyDepotOpaque,
		.close_replace = FleetGroup_close_replace,
		.screen_dirty = RustScreenDirty,
		.list_generate = FleetGroup_list_generate,
		.list_size = FleetGroup_list_size,
		.list_at = FleetGroup_list_at,
		.renew_allocate = FleetGroup_renew_allocate,
		.renew_can_allocate = FleetGroup_renew_can_allocate,
		.renew_delete = FleetGroup_renew_delete,
		.recursion_error = STR_ERROR_GROUP_CAN_T_SET_PARENT_RECURSION,
	};
	return services;
}
