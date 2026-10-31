/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file effect_protocol.hpp Native direct-service checks for transient effects absent from saves. */
#ifndef EFFECT_PROTOCOL_HPP
#define EFFECT_PROTOCOL_HPP
#include "../rust/effect_ffi.h"
#include <array>
#include <memory>

#include "../rust/animation_save.hpp"
struct EffectTestWorld {
	OpenTTDEffectView view{};
	uint32_t random = UINT32_MAX, draws = 0, industry = 0, queries = 0, writes = 0, viewports = 0, animated = 0;
	uint8_t sound = 0; ///< Last sound played: 1 + success, or 0 for none.
};
static inline EffectTestWorld *effect_test_world;
static inline void OPENTTD_RUST_CALL EffectTestObserve(void *context, OpenTTDEffectView *view) noexcept
{
	*view = static_cast<EffectTestWorld *>(context)->view;
}
static inline void OPENTTD_RUST_CALL EffectTestWrite(void *context, uint8_t field, uint32_t value) noexcept
{
	auto &w = *static_cast<EffectTestWorld *>(context);
	w.writes++;
	switch (field) {
		case 0: w.view.x = static_cast<int32_t>(value); break;
		case 1: w.view.y = static_cast<int32_t>(value); break;
		case 2: w.view.z = static_cast<int32_t>(value); break;
		case 3: case 4: w.view.sprite = value; break;
		case 5: w.view.progress = static_cast<uint8_t>(value); break;
		case 6: w.view.spritenum = static_cast<uint8_t>(value); break;
	}
}
static inline OpenTTDEffectLeaves EffectTestLeaves()
{
	return {
		.observe = EffectTestObserve,
		.write_x = [](void *c, int32_t v) noexcept { EffectTestWrite(c, 0, v); },
		.write_y = [](void *c, int32_t v) noexcept { EffectTestWrite(c, 1, v); },
		.write_z = [](void *c, int32_t v) noexcept { EffectTestWrite(c, 2, v); },
		.sprite_sequence = [](void *c, uint32_t v) noexcept { EffectTestWrite(c, 3, v); },
		.sprite = [](void *c, uint32_t v) noexcept { EffectTestWrite(c, 4, v); },
		.progress = [](void *c, uint8_t v) noexcept { EffectTestWrite(c, 5, v); },
		.spritenum = [](void *c, uint8_t v) noexcept { EffectTestWrite(c, 6, v); },
		.viewport = [](void *c) noexcept { static_cast<EffectTestWorld *>(c)->viewports++; },
		.sound = [](void *c, uint8_t v) noexcept { static_cast<EffectTestWorld *>(c)->sound = 1 + v; },
	};
}
static inline void OPENTTD_RUST_CALL EffectTestViewport(void *context) noexcept { static_cast<EffectTestWorld *>(context)->viewports++; }
static inline void OPENTTD_RUST_CALL EffectTestSound(void *context, uint8_t success) noexcept { static_cast<EffectTestWorld *>(context)->sound = 1 + success; }
static inline uint32_t OPENTTD_RUST_CALL EffectTestIndustry(int32_t, int32_t, uint32_t *tile) noexcept
{
	auto &w = *effect_test_world;
	w.queries++;
	*tile = 123;
	return w.industry;
}

static inline uint32_t EffectTestRandom(void *context) noexcept
{
	auto &w = *static_cast<EffectTestWorld *>(context);
	w.draws++;
	return w.random;
}

template <typename Check> void CheckEffectProtocol(Check check)
{
	/* Persistent state is covered by simulation. These pinned-source fixtures
	 * observe immediate writes, transient services and RNG short circuit. */
	EffectTestWorld w;
	effect_test_world = &w;
	const OpenTTDEffectLeaves leaves = EffectTestLeaves();
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&w, EffectTestRandom, EffectTestIndustry);
	auto animation_shared = services;
	animation_shared.mark_dirty = [](uint32_t) noexcept {};
	auto animation = OpenTTDAnimationServices{};
	animation.shared = &animation_shared;
	animation.state = [](uint32_t) noexcept -> AnimatedTileState { return AnimatedTileState::Deleted; };
	animation.set_state = [](uint32_t tile, AnimatedTileState) noexcept { effect_test_world->animated = tile; };
	RustAnimationScope animation_scope(animation);

	auto owner = std::unique_ptr<OpenTTDEffectState, decltype(&openttd_rust_effect_destroy)>(openttd_rust_effect_new(), openttd_rust_effect_destroy);
	check(openttd_rust_effect_get_animation(owner.get()) == 0 && openttd_rust_effect_get_substate(owner.get()) == 0);
	openttd_rust_effect_set_animation(owner.get(), 65535);
	openttd_rust_effect_set_substate(owner.get(), static_cast<uint8_t>(511));
	check(openttd_rust_effect_get_substate(owner.get()) == 255);
	const std::array<uint32_t, 12> sprites{3708,3079,3073,3084,2040,3709,3737,3725,1416,4751,2040,2040};
	const std::array<uint8_t, 12> progress{7,12,0,1,12,0,0,0,0,0,12,12};
	auto run = [&](uint8_t init = 0) -> uint8_t {
		w.writes = w.viewports = w.animated = w.sound = 0;
		if (init != 0) { openttd_rust_effect_initialize(owner.get(), &w, w.view.subtype, &leaves, &services); return 1; }
		return openttd_rust_effect_run(owner.get(), &w, &leaves, &services);
	};
	for (uint8_t subtype = 0; subtype < 12; subtype++) {
		w.view = {13,-9,21,0,0,99,subtype,1};
		check(run(1) == 1 && w.view.sprite == sprites[subtype] && w.view.progress == progress[subtype]);
		check(w.view.x == 13 && w.view.y == -9 && w.view.z == 21);
		check(openttd_rust_effect_get_animation(owner.get()) == (subtype < 8 ? 65535 : 0));
		check(openttd_rust_effect_get_substate(owner.get()) == (subtype < 8 ? 255 : 0));
		check(w.view.spritenum == (subtype == 9 ? 0 : 99));
	}
	check(w.draws == 1);
	for (uint16_t duration : {uint16_t{0}, uint16_t{1}, uint16_t{65535}}) {
		w.view = {0,0,0,3740,255,0,6,0};
		openttd_rust_effect_set_animation(owner.get(), duration);
		check(run() == (duration == 1 ? 0 : 1));
		check(w.view.progress == 0 && w.view.sprite == 3737 && w.writes == 2 && w.viewports == 1);
		check(openttd_rust_effect_get_animation(owner.get()) == static_cast<uint16_t>(duration - 1));
	}
	w.view = {0,0,0,UINT32_MAX,3,0,5,0};
	check(run() == 1 && w.view.sprite == 0);
	w.view = {0,0,0,3724,3,0,5,0};
	check(run() == 0 && w.writes == 1); // Deletion occurs only after return.
	for (uint32_t random : {0U, 682U, 683U, 65536U}) {
		w.view = {10,20,180,4748,3,1,9,1}; w.random = random; w.draws = 0;
		openttd_rust_effect_set_animation(owner.get(), 3);
		check(run() == 1 && w.draws == 1);
		const bool burst = random != 683;
		check(w.view.spritenum == (burst ? 5 : 1));
		check(w.sound == (burst ? 1 : 0));
		check(w.view.z == 181 && openttd_rust_effect_get_animation(owner.get()) == 0);
	}
	w.view = {10,20,181,4748,3,1,9,1}; w.draws = 0;
	openttd_rust_effect_set_animation(owner.get(), 3);
	check(run() == 1 && w.draws == 0 && w.view.spritenum == 5 && w.sound == 1 && w.view.z == 182);
	for (uint32_t industry : {0U, 1U, 2U}) {
		w.view = {10,20,100,4748,3,6,9,1}; w.industry = industry; w.queries = 0;
		openttd_rust_effect_set_animation(owner.get(), 77);
		check(run() == 1 && w.queries == 1 && w.sound == 2 && w.animated == (industry == 2 ? 123 : 0));
		check(openttd_rust_effect_get_animation(owner.get()) == 79 && w.view.sprite == 4758 && w.view.x == 10 && w.view.y == 20 && w.view.z == 100);
	}
	w.view = {0,0,0,4757,3,5,9,0};
	openttd_rust_effect_set_animation(owner.get(), 3);
	check(run() == 0 && w.writes == 1);
	owner.reset();
	effect_test_world = nullptr;
}
#endif /* EFFECT_PROTOCOL_HPP */
