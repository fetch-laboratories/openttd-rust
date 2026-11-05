/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file grf_bindings.cpp Operations on native spec and engine binding payloads. */
#include "../stdafx.h"
#include "grf_bindings_ffi.h"
#include "../company_type.h"
#include "engine_services.h"
#include "../engine_base.h"
#include "../newgrf_badge.h"
#include "../newgrf_canal.h"
#include "../newgrf_object.h"
#include "../newgrf_engine.h"
#include "../cargotype.h"
#include "../rail.h"
#include "../road.h"
#include "../newgrf/newgrf_internal.h"
#include "../string_func.h"
#include "../safeguards.h"
static_assert(CF_END == 9 && RTSG_END == 13 && ROTSG_END == 12 && GSF_DEFAULT == 22);
static_assert(GSF_ORIGINAL_STRINGS == 0x48 && static_cast<uint8_t>(StandardSpriteGroup::Default) == 0 && static_cast<uint8_t>(StandardSpriteGroup::Purchase) == 1);
const OpenTTDGrfBindingServices &GetRustGrfBindingServices() noexcept
{
	static const OpenTTDGrfBindingServices services{
		.badge_bind = [](GRFFile *file, uint16_t id, uint8_t feature, const SpriteGroup *group) noexcept { auto it = file->badge_map.find(id); if (it == file->badge_map.end()) return false; auto &badge = *GetBadge(it->second); badge.grf_prop.SetSpriteGroup(static_cast<GrfSpecFeature>(feature), group); if (feature == GSF_DEFAULT) { badge.grf_prop.SetGRFFile(file); badge.grf_prop.local_id = id; } return true; },
		.badge_name = [](GRFFile *file, uint16_t id) noexcept -> StringID * { auto it = file->badge_map.find(id); return it == file->badge_map.end() ? nullptr : &GetBadge(it->second)->name; },
		.canal_bind = [](uint8_t id, const GRFFile *file, const SpriteGroup *group) noexcept { _water_feature[id].grffile = file; _water_feature[id].group = group; },
		.cargo_bind = [](uint8_t id, const GRFFile *file, const SpriteGroup *group) noexcept { auto *cargo = CargoSpec::Get(id); cargo->grffile = file; cargo->group = group; },
		.custom_engine_sprites = [](uint16_t engine, uint8_t cargo, const SpriteGroup *group) noexcept { SetCustomEngineSprites(EngineID{engine}, cargo, group); },
		.engine_grf = [](uint16_t engine, const GRFFile *file) noexcept { SetEngineGRF(EngineID{engine}, file); },
		.engine_string = RustEngineString,
		.mark_feature = [](GRFFile *file, GrfSpecFeature feature) noexcept { file->grf_features.Set(feature); },
		.new_name = [](uint16_t id, const uint8_t *text, size_t size) noexcept { GrfMsg(8, "FeatureNewName: 0x{:04X} <- {}", id, StrMakeValid(std::string_view{reinterpret_cast<const char *>(text), size})); },
		.object_spec = [](GRFFile *file, uint16_t id) noexcept -> ObjectSpec * { return id < file->objectspec.size() ? file->objectspec[id].get() : nullptr; },
		.rail_bind = [](RailType type, uint8_t cid, const GRFFile *file, const SpriteGroup *group) noexcept { extern RailTypeInfo _railtypes[RAILTYPE_END]; _railtypes[type].grffile[cid] = file; _railtypes[type].group[cid] = group; },
		.road_bind = [](RoadType type, uint8_t cid, const GRFFile *file, const SpriteGroup *group) noexcept { extern RoadTypeInfo _roadtypes[ROADTYPE_END]; _roadtypes[type].grffile[cid] = file; _roadtypes[type].group[cid] = group; },
		.wagon_override = [](uint16_t engine, uint8_t cargo, const SpriteGroup *group, EngineID *engines, size_t count) noexcept { SetWagonOverrideSprites(EngineID{engine}, cargo, group, {engines, count}); },
	};
	return services;
}
