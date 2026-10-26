/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_loading.cpp Canonical loading aliases across serialized game threads. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../newgrf/newgrf_internal.h"
#include "../rust/newgrf_loading_ffi.h"
#include "abi_layout.hpp"
#include <thread>

TEST_CASE("Rust NewGRF loading - native ABI")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-grf_loading.inc"
}

TEST_CASE("Rust NewGRF loading - canonical aliases cross serialized threads")
{
	auto *state = openttd_rust_grf_execution();
	auto saved = *state;
	bool aliases = false;
	std::thread writer([&] {
		GrfProcessingState view;
		aliases = &view.stage == &state->stage && &view.spriteid == &state->spriteid && &view.file == &state->file && &view.grffile == &state->grffile && &view.grfconfig == &state->grfconfig && &view.nfo_line == &state->nfo_line && &view.skip_sprites == &state->skip_sprites && &view.spritegroups == &state->spritegroups;
		view.stage = GLS_RESERVE;
		view.nfo_line = 41;
		view.skip_sprites = -1;
		view.AddSpriteSets(GSF_SOUNDFX, 100, UINT32_MAX - 1, 2, 3);
	});
	writer.join();
	bool observed = false;
	std::thread reader([&] {
		GrfProcessingState view;
		observed = view.stage == GLS_RESERVE && view.nfo_line == 41 && view.skip_sprites == -1 && view.HasValidSpriteSets(GSF_SOUNDFX) && view.IsValidSpriteSet(GSF_SOUNDFX, UINT32_MAX) && view.GetSprite(GSF_SOUNDFX, UINT32_MAX) == 103 && view.GetNumEnts(GSF_SOUNDFX, UINT32_MAX) == 3;
	});
	reader.join();
	*state = saved;
	CHECK(aliases);
	CHECK(observed);
}
#endif
