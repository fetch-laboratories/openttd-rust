/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file effectvehicle.cpp Effect owner lifetime and serialization-boundary evidence. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../effectvehicle_base.h"
#include "../effectvehicle_func.h"
#include "../vehicle_func.h"
#include "../map_func.h"
#include "../landscape.h"
#include "../core/random_func.hpp"
#include "mock_environment.h"
#ifdef WITH_RUST
#include "../../tools/simulation/session-fixture.hpp"
#include "effect_protocol.hpp"
#endif
#include "../safeguards.h"

#ifdef WITH_RUST
TEST_CASE("Effect vehicles - native direct services and ownership")
{
	CheckEffectProtocol([](bool result) { CHECK(result); });
}

TEST_CASE("Animated tiles - ordered queue and partial native load publication")
{
	std::vector<uint32_t> saved;
	for (size_t i = 0; i < openttd_rust_animation_count(); ++i) saved.push_back(openttd_rust_animation_at(i));
	static std::array<AnimatedTileState, 4> states{}; states.fill(AnimatedTileState::None);
	auto shared = OpenTTDSharedServices{};
	shared.tile_type = [](uint32_t t) noexcept -> uint8_t { return t == 2 ? 0 : 3; };
	shared.map_log_x = []() noexcept -> uint32_t { return 0; };
	shared.map_max_y = []() noexcept -> uint32_t { return 3; };
	OpenTTDAnimationServices services{};
	services.shared = &shared;
	services.state = [](uint32_t t) noexcept { return states[t]; };
	services.set_state = [](uint32_t t, AnimatedTileState s) noexcept { states[t] = s; };
	RustAnimationScope scope(services);
	openttd_rust_animation_initialize();
	for (uint32_t t : {1, 2, 3}) openttd_rust_animation_add(t, false);
	openttd_rust_animation_delete(1, false);
	openttd_rust_animation_add(1, false); // Reactivation keeps its original position.
	CHECK(openttd_rust_animation_count() == 3);
	openttd_rust_animation_delete(2, true);
	CHECK(openttd_rust_animation_at(0) == 1);
	CHECK(openttd_rust_animation_at(1) == 3);
	openttd_rust_animation_delete(1, false); states[3] = AnimatedTileState::None;
	{
		SessionFixture unrelated;
		CHECK(unrelated.Run(GM_NORMAL));
		CHECK(openttd_rust_animation_count() == 2);
		CHECK(states[1] == AnimatedTileState::Deleted);
		CHECK(states[3] == AnimatedTileState::None);
	}
	SessionFixture fixture;
	fixture.animate = true;
	auto clock = GetRustClockServices();
	clock.minutes_per_calendar_year = []() noexcept -> uint16_t { return 0; };
	fixture.services.clock = &clock;
	fixture.services.desync_level = []() noexcept -> int32_t { return 0; };
	fixture.services.check_caches = SessionFixture::Nothing;
	fixture.services.company_begin = SessionFixture::FrameNothing;
	fixture.services.console_exception = [](OpenTTDSessionFrame *) noexcept -> OpenTTDConsoleException * { return nullptr; };
	fixture.services.failed = [](OpenTTDSessionFrame *) noexcept { return false; };
	fixture.services.tick_timer = [](OpenTTDSessionFrame *) noexcept { return false; };
	// Stop at the real tick-timer exception boundary after processing animation.
	CHECK_FALSE(fixture.Run(GM_NORMAL));
	CHECK(openttd_rust_animation_count() == 0);
	{
		SessionFixture after_failure;
		CHECK(after_failure.Run()); // The prior injected failure leaves no storage mode behind.
	}
	try {
		RustAnimationSave loading(true);
		RustAnimationSave::Tiles().resize(3);
		RustAnimationSave::Tiles()[0] = TileIndex{3};
		throw 17; // SlCopy/SlGlobList/SlError unwind entirely on native frames.
	} catch (int) {}
	CHECK(openttd_rust_animation_count() == 3);
	CHECK(openttd_rust_animation_at(0) == 3);
	CHECK(openttd_rust_animation_at(1) == 0);
	CHECK(RustAnimationSave::active == nullptr);
	openttd_rust_animation_initialize();
	for (uint32_t t : {1, 2, 1, 3}) openttd_rust_animation_append(t);
	openttd_rust_animation_afterload(true, true);
	CHECK(openttd_rust_animation_count() == 2);
	CHECK(openttd_rust_animation_at(0) == 1);
	CHECK(openttd_rust_animation_at(1) == 3);
	CHECK((states[1] == AnimatedTileState::Animated && states[2] == AnimatedTileState::None && states[3] == AnimatedTileState::Animated));
	openttd_rust_animation_initialize();
	for (uint32_t t : saved) openttd_rust_animation_append(t);
}

TEST_CASE("Vehicle visual cache - canonical alias and independent value copies")
{
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *a = new EffectVehicle();
	auto *b = new EffectVehicle();
	CHECK(&a->vcache.cached_vis_effect == openttd_rust_vehicle_visual_cache(a->rust_grf_cache.state));
	CHECK(a->vcache.cached_vis_effect == 0);
	a->vcache.cached_vis_effect = 0x8F;
	VehicleCache copy = a->vcache;
	a->vcache.cached_vis_effect = 0x42;
	CHECK(copy.cached_vis_effect == 0x8F);
	copy.cached_vis_effect = 0xA3;
	CHECK(a->vcache.cached_vis_effect == 0x42);
	b->vcache = copy;
	CHECK(&b->vcache.cached_vis_effect == openttd_rust_vehicle_visual_cache(b->rust_grf_cache.state));
	CHECK(b->vcache.cached_vis_effect == 0xA3);
	copy.cached_vis_effect = 0;
	CHECK(b->vcache.cached_vis_effect == 0xA3);
	const VehicleID index = a->index;
	_vehicle_pool.CleanPool();
	REQUIRE(Vehicle::CanAllocateItem());
	auto *reused = new (index) EffectVehicle();
	CHECK(reused->vcache.cached_vis_effect == 0);
	_vehicle_pool.CleanPool();
}

TEST_CASE("Effect vehicles - zero construction, nested save staging and pool cleanup")
{
	REQUIRE(Vehicle::CanAllocateItem(2));
	auto *a = new EffectVehicle();
	auto *b = new EffectVehicle();
	CHECK(a->GetAnimationState() == 0);
	CHECK(a->GetAnimationSubstate() == 0);
	a->SetAnimationState(65535);
	a->SetAnimationSubstate(255);
	{
		EffectVehicleAnimationScope saving(a, false);
		CHECK(EffectVehicleAnimationScope::State() == 65535);
		CHECK(EffectVehicleAnimationScope::Substate() == 255);
		{
			EffectVehicleAnimationScope loading(b, true);
			EffectVehicleAnimationScope::State() = 17;
			EffectVehicleAnimationScope::Substate() = 19;
			CHECK(b->GetAnimationState() == 0); // No persistent C++ mirror.
		}
		CHECK((b->GetAnimationState() == 17 && b->GetAnimationSubstate() == 19));
		CHECK(EffectVehicleAnimationScope::State() == 65535); // Parent restored.
	}
	try {
		EffectVehicleAnimationScope loading(a, true);
		EffectVehicleAnimationScope::State() = 31;
		throw 1; // Partial historical load writes survive failure, as in C++.
	} catch (int) {}
	CHECK((a->GetAnimationState() == 31 && a->GetAnimationSubstate() == 255));
	/* The derived owner's deleter still runs when Vehicle's base destructor takes
	 * the Pool::Cleaning early return; no world/map initialization is needed. */
	_vehicle_pool.CleanPool();
}
#endif

/* Use the real controller's expiry route for ordinary owner destruction. This
 * also avoids invoking the pool's post-destructor index read directly from a
 * test translation unit, which GCC 15 diagnoses when devirtualizing delete. */
static void ExpireEffect(EffectVehicle *v)
{
	v->subtype = EV_BREAKDOWN_SMOKE;
	v->SetAnimationState(1);
	CHECK_FALSE(v->Tick());
}

TEST_CASE("Effect vehicles - first sprite mutation and sequence replacement")
{
	MockEnvironment::Instance();
	Map::Allocate(64, 64);
	ResetVehicleHash();
	auto *v = CreateEffectVehicle(40, 50, 60, EV_BREAKDOWN_SMOKE);
	REQUIRE(v != nullptr);
	v->SetAnimationState(3);
	auto &sequence = v->sprite_cache.sprite_seq;
	sequence.count = 2;
	sequence.seq[0] = {3738, 17};
	sequence.seq[1] = {3740, 19};
	v->progress = 7;
	REQUIRE(v->Tick());
	CHECK(sequence.seq[0].sprite == 3739);
	CHECK(sequence.count == 2);
	CHECK(sequence.seq[0].pal == 17);
	sequence.seq[0].sprite = 3740;
	v->progress = 15;
	REQUIRE(v->Tick());
	CHECK(sequence.seq[0].sprite == 3737);
	CHECK(sequence.count == 1);
	CHECK(sequence.seq[0].pal == 0);
	ExpireEffect(v);
}

TEST_CASE("Effect vehicles - all factory variants and relative terrain coordinates")
{
	MockEnvironment::Instance();
	Map::Allocate(64, 64);
	ResetVehicleHash();
	SavedRandomSeeds saved;
	SaveRandomSeeds(&saved);
	SetRandomSeed(12345);
	auto expected_random = _random;
	const uint32_t chimney_random = expected_random.Next();
	static constexpr uint32_t sprites[]{3701,3079,3073,3084,2040,3709,3737,3725,1416,4751,2040,2040};
	static constexpr uint8_t progress[]{0,12,0,1,12,0,0,0,0,0,12,12};
	for (uint8_t subtype = 0; subtype < EV_END; subtype++) {
		auto *v = CreateEffectVehicle(40, 50, 60, static_cast<EffectVehicleType>(subtype));
		REQUIRE(v != nullptr);
		CHECK(v->sprite_cache.sprite_seq.seq[0].sprite == sprites[subtype] + (subtype == 0 ? chimney_random & 7 : 0));
		CHECK(v->progress == (subtype == 0 ? (chimney_random >> 16) & 7 : progress[subtype]));
		CHECK((v->x_pos == 40 && v->y_pos == 50 && v->z_pos == 60));
		CHECK(v->GetAnimationState() == 0);
		CHECK(v->GetAnimationSubstate() == 0);
		CHECK(v->tile == TileIndex{});
		CHECK(v->spritenum == 0);
		auto *relative = CreateEffectVehicleRel(v, -4, 5, -6, EV_ELECTRIC_SPARK);
		REQUIRE(relative != nullptr);
		CHECK((relative->x_pos == 36 && relative->y_pos == 55 && relative->z_pos == 54));
		ExpireEffect(relative);
		ExpireEffect(v);
	}
	CHECK((_random.state[0] == expected_random.state[0] && _random.state[1] == expected_random.state[1]));
	/* Above clamps only the terrain query. Original x/y remain outside the map. */
	auto *above = CreateEffectVehicleAbove(-9, 2000, 17, EV_ELECTRIC_SPARK);
	REQUIRE(above != nullptr);
	CHECK((above->x_pos == -9 && above->y_pos == 2000));
	CHECK(above->z_pos == GetSlopePixelZ(0, Map::MaxY() * TILE_SIZE) + 17);
	ExpireEffect(above);
	RestoreRandomSeeds(saved);
}
