/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file engine_loading_services.hpp Engine specifications, pool and loading diagnostics. */
#include "engine_loading_ffi.h"
#include "engine_services.h"
#include "settings_services.h"
#include "services_newgrf.h"
#include "services_cargo.h"
#include "../window_func.h"

const OpenTTDEngineLoadingServices &GetRustEngineLoadingServices() noexcept
{
	static const OpenTTDEngineLoadingServices services{
		.shared = &GetRustSharedServices(),
		.resolver = &GetRustVehicleResolverServices(),
		.next = Engine_next,
		.resolve = Engine_resolve_strict,
		.dynamic = RustDynamicEngines,
		.file_id = RustGRFFileID,
		.find_file = [](uint32_t id) noexcept -> const GRFFile * { return GetFileByGRFID(id); },
		.engine_file = RustEngineGRF,
		.has_file = RustEngineHasGRF,
		.set_file = [](const OpenTTDEngine *shell, const GRFFile *file) noexcept { const_cast<Engine *>(EngineOf(shell))->grf_prop.SetGRFFile(file); },
		.mapping_log = [](uint32_t source, uint32_t target, bool loaded) noexcept {
			if (loaded) GrfMsg(5, "Mapping from GRFID {:x} to {:x}", std::byteswap(source), std::byteswap(target));
			else GrfMsg(5, "Tried mapping from GRFID {:x} to {:x} but target is not loaded", std::byteswap(source), std::byteswap(target));
		},
		.replaced_log = [](uint16_t id, uint32_t file, VehicleType type, uint16_t local) noexcept { GrfMsg(5, "Replaced engine at index {} for GRFID {:x}, type {}, index {}", id, std::byteswap(file), type, local); },
		.created_log = [](uint16_t id, uint32_t file, VehicleType type, uint16_t local) noexcept { GrfMsg(5, "Created new engine at index {} for GRFID {:x}, type {}, index {}", id, std::byteswap(file), type, local); },
		.full_log = []() noexcept { GrfMsg(0, "Can't allocate any more engines"); },
		.can_allocate = []() noexcept { return Engine::CanAllocateItem(); },
		.pool_size = []() noexcept -> size_t { return Engine::GetPoolSize(); },
		.construct = [](VehicleType type, uint16_t local) noexcept { return EngineRef(new Engine(type, local)); },
		.construct_at = [](VehicleType type, uint16_t local, uint16_t id) noexcept { new (EngineID{id}) Engine(type, local); },
		.close_previews = []() noexcept { CloseWindowByClass(WC_ENGINE_PREVIEW); },
		.clean_pool = []() noexcept { _engine_pool.CleanPool(); },
		.railtypes = RustEngineRailTypes,
		.rail_label = [](RailType rt) noexcept { return GetRailTypeInfo(rt)->label; },
		.refit_inputs = [](const OpenTTDEngine *shell) noexcept { const auto &ei = EngineOf(shell)->info; return OpenTTDEngineRefitInputs{.mask = RustEngineCargoRefitMask(shell), .cargo = ei.cargo_type, .custom = VehicleCallbackMasks{RustEngineCallbackMask(shell)}.Test(VehicleCallbackMask::CustomRefit)}; },
		.cargo_label = RustEngineCargoLabel,
		.train_cargo = [](const OpenTTDEngine *shell) noexcept { return OpenTTDEngineTrainCargo{.capacity = RustEngineRailCapacity(shell), .kind = Engine_rail_kind(shell)}; },
		.set_cargo = RustSetEngineCargo,
		.set_refit = RustSetEngineRefitMask,
		.disable = RustDisableEngine,
		.old_refittable = RustEngineOldRefittable,
		.set_old_refittable = RustSetEngineOldRefittable,
		.default_cargo = [](CargoType cargo) noexcept { return IsDefaultCargo(cargo); },
		.cargo_by_label = OpenTTDCargoByLabel,
		.cargo_mask = []() noexcept { return _cargo_mask; },
		.standard_cargo_mask = []() noexcept { return _standard_cargo_mask; },
		.cargo_next = [](uint32_t from) noexcept { for (const CargoSpec *cs : CargoSpec::Iterate(from)) return OpenTTDEngineCargoClass{.classes = RustCargoSpecClasses(cs), .cargo = cs->Index()}; return OpenTTDEngineCargoClass{.classes = 0, .cargo = INVALID_CARGO}; },
		.cargo_map = RustGRFCargoMap,
		.file_version = RustGRFFileVersion,
		.cargo_list_size = [](const GRFFile *file) noexcept { return file->cargo_list.size(); },
		.cargo_translation = RustCargoTranslation,
		.unknown_refit = RustUnknownCallbackResult,
		.climate = OpenTTDLandscapeValue,

		.variant = RustEngineVariant,
		.set_variant = RustSetEngineVariant,
		.invalid_string = RustSetEngineInvalidString,
		.engine_climate = static_cast<bool (*)(const OpenTTDEngine *) noexcept>(RustEngineClimate),
		.rail_badges = [](const OpenTTDEngine *shell, RailType rt) noexcept { AppendCopyableBadgeList(const_cast<Engine *>(EngineOf(shell))->badges, GetRailTypeInfo(rt)->badges, GSF_TRAINS); },
		.road_badges = [](const OpenTTDEngine *shell) noexcept { auto *e = const_cast<Engine *>(EngineOf(shell)); AppendCopyableBadgeList(e->badges, GetRoadTypeInfo(RustEngineRoadType(e))->badges, GSF_ROADVEHICLES); },
		.rail_kind = Engine_rail_kind,
		.livery = [](uint16_t id) noexcept { return GetEngineLiveryScheme(EngineID{id}, EngineID::Invalid(), nullptr); },
		.use_livery = [](LiveryScheme scheme) noexcept { SetBit(_loaded_newgrf_features.used_liveries, scheme); },
		.variant_flags = [](const OpenTTDEngine *shell) noexcept { const_cast<Engine *>(EngineOf(shell))->display_flags.Set({EngineDisplayFlag::HasVariants, EngineDisplayFlag::IsFolded}); },
		.variant_cycle_log = [](const OpenTTDEngine *shell) noexcept { const Engine *e = EngineOf(shell); GrfMsg(1, "FinaliseEngineArray: Variant of engine {:x} in '{}' loops back on itself", e->grf_prop.local_id, RustEngineGRF(shell)->filename); },
		.is_tram = RustEngineIsTram,
		.set_road_speed = RustSetEngineRoadSpeed,
		.set_roadtype = RustSetEngineRoadType,
		.road_list_size = [](const GRFFile *file, bool tram) noexcept { return (tram ? file->tramtype_list : file->roadtype_list).size(); },
		.road_label = [](const GRFFile *file, bool tram, size_t index) noexcept { return (tram ? file->tramtype_list : file->roadtype_list)[index]; },
		.road_by_label = [](RoadTypeLabel label) noexcept { return GetRoadTypeByLabel(label); },
		.rail_by_label = [](RailTypeLabel label) noexcept { return GetRailTypeByLabel(label); },
		.set_railtypes = [](const OpenTTDEngine *shell, uint64_t types) noexcept { auto &rvi = const_cast<Engine *>(EngineOf(shell))->VehInfo<RailVehicleInfo>(); rvi.railtypes = RailTypes{types}; rvi.intended_railtypes = RailTypes{types}; },
		.override_log = [](uint32_t source, uint32_t target) noexcept {
			if (target == 0) GrfMsg(5, "SetNewGRFOverride: Removed override of 0x{:X}", std::byteswap(source));
			else GrfMsg(5, "SetNewGRFOverride: Added override of 0x{:X} to 0x{:X}", std::byteswap(source), std::byteswap(target));
		},

		.vehicle_next = [](uint32_t from) noexcept { const Vehicle *v = VehicleOf(RustNextVehicle(from)); return v == nullptr ? OpenTTDEngineLoadingVehicle{UINT32_MAX, VEH_INVALID} : OpenTTDEngineLoadingVehicle{RustVehicleIndexOpaque(v), RustVehicleVehicleType(v)}; },
		.reload = []() noexcept { ReloadNewGRFData(); },
		.default_counts = []() noexcept { return _engine_counts; },
	};
	return services;
}
