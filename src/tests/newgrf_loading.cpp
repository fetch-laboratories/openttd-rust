/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_loading.cpp Canonical loading aliases across serialized game threads. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#ifdef WITH_RUST
#include "../newgrf/newgrf_internal.h"
#include "../rust/newgrf_loading_ffi.h"
#include "../rust/newgrf_act2_ffi.h"
#include "abi_layout.hpp"
#include <thread>

TEST_CASE("Rust NewGRF loading - native ABI")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-grf_loading.inc"
#include "abi-layout-act2.inc"
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
#ifdef WITH_RUST
#include "../rust/newgrf_bytereader_services.h"

TEST_CASE("Rust Action2 callback interner crosses serialized threads")
{
	GRFFile file;
	file.grf_version = 8;
	auto *state = openttd_rust_grf_execution();
	auto saved = *state;
	state->grffile = &file;
	openttd_rust_action2_reset(true);
	const SpriteGroup *first = nullptr;
	const SpriteGroup *second = nullptr;
	auto parse = [](uint8_t slot) {
		const uint8_t bytes[]{GSF_ROADVEHICLES, slot, 0x81, 0x0C, 0, 255, 1, 55, 128, 0, 255, 255, 127};
		OpenTTDGrfCursor reader{bytes, std::size(bytes), 0};
		return openttd_rust_action2_parse(&GetRustAction2Services(), &GetRustGrfLoadingServices(), &reader);
	};
	bool parsed_first = false;
	std::thread writer([&] {
		parsed_first = parse(251);
		if (parsed_first) first = static_cast<const DeterministicSpriteGroup *>(state->spritegroups[251])->ranges[0].result.group;
	});
	writer.join();
	size_t before = SpriteGroup::GetNumItems();
	bool parsed_second = false;
	std::thread reader([&] {
		parsed_second = parse(252);
		if (parsed_second) second = static_cast<const DeterministicSpriteGroup *>(state->spritegroups[252])->ranges[0].result.group;
	});
	reader.join();
	CHECK(parsed_first);
	CHECK(parsed_second);
	CHECK(first == second);
	CHECK(SpriteGroup::GetNumItems() == before + 1);
	openttd_rust_action2_reset(true);
	delete state->spritegroups[251];
	delete state->spritegroups[252];
	delete first;
	if (second != first) delete second;
	*state = saved;
}
#endif
