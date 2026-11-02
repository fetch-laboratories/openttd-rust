/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file grf_bindings.cpp Operations on native spec and engine binding payloads. */
#include "../stdafx.h"
#include "grf_bindings_ffi.h"
#include "vehicle_services.h"
#include "../engine_base.h"
#include "../newgrf_badge.h"
#include "../newgrf/newgrf_internal.h"
#include "../string_func.h"
#include "../safeguards.h"
static_assert(GSF_ORIGINAL_STRINGS == 0x48 && static_cast<uint8_t>(StandardSpriteGroup::Default) == 0 && static_cast<uint8_t>(StandardSpriteGroup::Purchase) == 1);
const OpenTTDGrfBindingServices &GetRustGrfBindingServices() noexcept
{
	static const OpenTTDGrfBindingServices services{
		.badge_name = [](GRFFile *file, uint16_t id) noexcept -> StringID * { auto it = file->badge_map.find(id); return it == file->badge_map.end() ? nullptr : &GetBadge(it->second)->name; },
		.engine_string = [](const OpenTTDEngine *engine) noexcept { return &const_cast<Engine *>(EngineOf(engine))->info.string_id; },
		.new_name = [](uint16_t id, const uint8_t *text, size_t size) noexcept { GrfMsg(8, "FeatureNewName: 0x{:04X} <- {}", id, StrMakeValid(std::string_view{reinterpret_cast<const char *>(text), size})); },
	};
	return services;
}
