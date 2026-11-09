/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_loading_services.hpp Engine specifications, pool and loading diagnostics. */
#include "engine_loading_ffi.h"
#include "company_identity_ffi.h"
#include "engine_services.h"
#include "type_availability_ffi.h"
#include "settings_services.h"
#include "services_newgrf.h"
#include "services_cargo.h"
#include "../window_func.h"

const OpenTTDEngineLoadingServices &GetRustEngineLoadingServices() noexcept
{
	static const OpenTTDEngineLoadingServices services{
		.identity = &GetRustCompanyIdentityServices(),
		.shared = &GetRustSharedServices(),
		.resolver = &GetRustVehicleResolverServices(),
		.types = &GetRustTypeAvailabilityServices(),
		.resolve = Engine_resolve_strict,
		.dynamic = RustDynamicEngines,
		.find_file = RustGRFFileByID,
		.mapping_log = [](uint32_t source, uint32_t target, bool loaded) noexcept {
			if (loaded) GrfMsg(5, "Mapping from GRFID {:x} to {:x}", std::byteswap(source), std::byteswap(target));
			else GrfMsg(5, "Tried mapping from GRFID {:x} to {:x} but target is not loaded", std::byteswap(source), std::byteswap(target));
		},
		.can_allocate = []() noexcept { return Engine::CanAllocateItem(); },
		.pool_size = []() noexcept -> size_t { return Engine::GetPoolSize(); },
		.construct = [](VehicleType type, uint16_t local) noexcept { return EngineRef(new Engine(type, local)); },
		.construct_at = [](VehicleType type, uint16_t local, uint16_t id) noexcept { new (EngineID{id}) Engine(type, local); },
		.close_previews = []() noexcept { CloseWindowByClass(WC_ENGINE_PREVIEW); },
		.clean_pool = []() noexcept { _engine_pool.CleanPool(); },
		.cargo_next = [](uint32_t from) noexcept { for (const CargoSpec *cs : CargoSpec::Iterate(from)) return OpenTTDEngineCargoClass{.classes = RustCargoSpecClasses(cs), .cargo = cs->Index()}; return OpenTTDEngineCargoClass{.classes = 0, .cargo = INVALID_CARGO}; },
		.cargo_list_size = [](const GRFFile *file) noexcept { return file->cargo_list.size(); },
		.unknown_refit = RustUnknownCallbackResult,

		.use_livery = [](LiveryScheme scheme) noexcept { SetBit(_loaded_newgrf_features.used_liveries, scheme); },
		.variant_flags = [](const OpenTTDEngine *shell) noexcept { const_cast<Engine *>(EngineOf(shell))->display_flags.Set({EngineDisplayFlag::HasVariants, EngineDisplayFlag::IsFolded}); },
		.override_log = [](uint32_t source, uint32_t target) noexcept {
			if (target == 0) GrfMsg(5, "SetNewGRFOverride: Removed override of 0x{:X}", std::byteswap(source));
			else GrfMsg(5, "SetNewGRFOverride: Added override of 0x{:X} to 0x{:X}", std::byteswap(source), std::byteswap(target));
		},

		.vehicle_next = [](uint32_t from) noexcept { const Vehicle *v = VehicleOf(RustNextVehicle(from)); return v == nullptr ? OpenTTDEngineLoadingVehicle{UINT32_MAX, VEH_INVALID} : OpenTTDEngineLoadingVehicle{RustVehicleIndexOpaque(v), RustVehicleVehicleType(v)}; },
		.reload = []() noexcept { ReloadNewGRFData(); },
		.default_counts = []() noexcept { return _engine_counts; },
		.order_sort = RustVehicleOrderSort,
		.order_count = []() noexcept -> size_t { return Engine::GetNumItems(); },
	};
	return services;
}

static_assert(to_underlying(VehicleCallbackMask::CustomRefit) == 9);
