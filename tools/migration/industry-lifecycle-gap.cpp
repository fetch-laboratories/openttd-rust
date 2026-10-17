/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry-lifecycle-gap.cpp Unavailable NewGRF triggers and rare builtin animation states. */
#include "stdafx.h"
#include "core/bitmath_func.hpp"
#include "core/enum_type.hpp"
#include "sound_type.h"
#include "industry-graphics-reference.inc"
#include "rust/industry_ffi.h"
using TileIndex = uint32_t;
using IndustryGfx = uint16_t;
enum class IndustryTileSpecialFlag : uint8_t { AcceptsAllCargo = 1 };
enum class IndustryTileCallbackMask : uint8_t { CargoAcceptance = 2, AcceptCargo = 3 };
enum class IndustryRandomTrigger : uint8_t { TileLoop, IndustryTick, CargoReceived };
enum class IndustryAnimationTrigger : uint8_t { ConstructionStageChanged, TileLoop, IndustryTick, CargoReceived, CargoDistributed };
struct Bits {
	uint8_t value = 0;
	void Set(IndustryRandomTrigger trigger) { this->value |= 1U << static_cast<uint8_t>(trigger); }
	void Reset(Bits used) { this->value &= ~used.value; }
	template <typename T> bool Test(T trigger) const { return (this->value & (1U << static_cast<uint8_t>(trigger))) != 0; }
};
using IndustryRandomTriggers = Bits;
constexpr int MP_INDUSTRY = 8, CBID_RANDOM_TRIGGER = 1, CBID_INDTILE_ANIMATION_TRIGGER = 0x25, VSG_SCOPE_SELF = 0, VSG_SCOPE_PARENT = 1;
struct Industry;
static Industry *active;
struct Tile { uint8_t triggers, bits; bool belongs; };
enum class AnimationStatus : uint8_t { NoAnimation, Animated };
struct BuiltinState { uint16_t gfx; uint8_t frame, loop, stage; bool operator==(const BuiltinState &) const = default; };
static BuiltinState builtin{};
static bool builtin_mode;
struct IndustryTileSpec {
	struct { bool enabled; void *grffile = nullptr; bool HasSpriteGroups() const { return enabled; } } grf_prop;
	struct { Bits triggers; AnimationStatus status = AnimationStatus::NoAnimation; } animation;
	std::array<uint8_t, 16> accepts_cargo{};
	std::array<int8_t, 16> acceptance{};
	Bits special_flags, callback_mask;
};
static std::array<Tile, 6> tiles;
static std::array<IndustryTileSpec, 6> specs;
static IndustryTileSpec builtin_spec;
static std::vector<std::array<uint32_t, 4>> trace;
static uint32_t rng, self_mask, parent_mask;
static bool IsValidTile(TileIndex tile) { return tile < tiles.size(); }
static bool IsTileType(TileIndex tile, int) { return tiles[tile].belongs; }
static IndustryGfx GetIndustryGfx(TileIndex tile) { return builtin_mode ? builtin.gfx : tile; }
static const IndustryTileSpec *GetIndustryTileSpec(IndustryGfx gfx) { return builtin_mode ? &builtin_spec : &specs[gfx]; }
static uint32_t Random() { rng = rng * 1664525U + 1013904223U; trace.push_back({0, rng, 0, 0}); return rng; }
static Bits GetIndustryRandomTriggers(TileIndex tile) { return {tiles[tile].triggers}; }
static void SetIndustryRandomTriggers(TileIndex tile, Bits bits) { tiles[tile].triggers = bits.value; trace.push_back({1, tile, bits.value, 0}); }
static uint8_t GetIndustryRandomBits(TileIndex tile) { return tiles[tile].bits; }
static void SetIndustryRandomBits(TileIndex tile, uint8_t bits) { tiles[tile].bits = bits; trace.push_back({2, tile, bits, 0}); }
static void MarkTileDirtyByTile(TileIndex tile) { trace.push_back({3, tile, 0, 0}); }
struct Industry {
	std::array<TileIndex, 6> location{0, 1, 2, 3, 4, 5};
	uint16_t random;
	struct Accepted { uint8_t cargo; };
	std::vector<Accepted> accepted;
	bool IsCargoAccepted(uint8_t cargo) const { return std::ranges::any_of(accepted, [cargo](const auto &a) { return a.cargo == cargo; }); }
	bool TileBelongsToIndustry(TileIndex tile) const { return tiles[tile].belongs; }
	static Industry *GetByTile(TileIndex) { return active; }
};
struct IndustryTileResolverObject {
	TileIndex tile;
	uint32_t reseed[2]{};
	Bits waiting;
	IndustryTileResolverObject(IndustryGfx, TileIndex tile, Industry *, int) : tile(tile) {}
	void SetWaitingRandomTriggers(Bits bits) { waiting = bits; }
	void ResolveRerandomisation() {
		trace.push_back({4, tile, waiting.value, tiles[tile].triggers});
		reseed[0] = self_mask ^ (tile * 0x0101U);
		reseed[1] = parent_mask;
		/* Resolver may affect live tile state before the source reads it. */
		tiles[tile].bits ^= 0x35;
	}
	Bits GetUsedRandomTriggers() const { return {static_cast<uint8_t>(waiting.value & (1U << (tile % 3)))}; }
};
struct IndustryAnimationBase {
	static void ChangeAnimationFrame(int, const IndustryTileSpec *, Industry *, TileIndex tile, uint32_t random, uint32_t parameter) { trace.push_back({5, tile, random, parameter}); }
};
#include "industry-lifecycle-reference.inc"
struct TimerGameTick { static inline uint64_t counter; };
static struct { struct { bool ambient; } sound; } _settings_client;
static uint8_t GetAnimationFrame(TileIndex) { return builtin.frame; }
static void SetAnimationFrame(TileIndex tile, uint8_t frame) { builtin.frame = frame; trace.push_back({6, tile, frame, 0}); }
static uint8_t GetIndustryAnimationLoop(TileIndex) { return builtin.loop; }
static void SetIndustryAnimationLoop(TileIndex tile, uint8_t loop) { builtin.loop = loop; trace.push_back({7, tile, loop, 0}); }
static void DeleteAnimatedTile(TileIndex tile) { trace.push_back({8, tile, 0, 0}); }
static void SetIndustryGfx(TileIndex tile, IndustryGfx gfx) { builtin.gfx = gfx; trace.push_back({9, tile, gfx, 0}); }
static void SetIndustryConstructionStage(TileIndex tile, uint8_t stage) { builtin.stage = stage; trace.push_back({10, tile, stage, 0}); }
static void SndPlayTileFx(uint16_t sound, TileIndex tile) { trace.push_back({11, tile, sound, 0}); }
static void AnimateNewIndustryTile(TileIndex tile) { trace.push_back({12, tile, 0, 0}); }
static bool Chance16(uint32_t a, uint32_t b) { return (((Random() & 65535) * b + b / 2) >> 16) < a; }
#include "industry-animation-reference.inc"

using CargoType = uint8_t;
using CargoArray = std::array<uint32_t, 64>;
using CargoTypes = uint64_t;
constexpr uint8_t INVALID_CARGO = 255;
constexpr uint16_t CALLBACK_FAILED = 0xFFFF, CBID_INDTILE_ACCEPT_CARGO = 0x2C, CBID_INDTILE_CARGO_ACCEPTANCE = 0x2B;
constexpr uint INDUSTRY_ORIGINAL_NUM_INPUTS = 3;
static bool IsValidCargoType(CargoType cargo) { return cargo != INVALID_CARGO; }
struct AcceptedSlot { uint8_t cargo; uint16_t waiting; uint32_t accumulated; int32_t date; void *history; };
static OpenTTDIndustry *acceptance_owner;
static uint16_t accept_result, amount_result;
static bool mutate_accepted;
static uint8_t GetCargoTranslation(uint8_t cargo, void *)
{
	uint8_t result = cargo == 31 ? 255 : (cargo * 3 + 2) % 64;
	trace.push_back({13, cargo, result, 0});
	return result;
}
static uint16_t GetIndustryTileCallback(uint16_t callback, uint32_t, uint32_t, IndustryGfx, Industry *, TileIndex tile)
{
	trace.push_back({14, tile, callback, 0});
	if (callback == CBID_INDTILE_CARGO_ACCEPTANCE && mutate_accepted) {
		active->accepted[0].cargo = 63;
		if (acceptance_owner != nullptr) static_cast<AcceptedSlot *>(openttd_rust_industry_accepted_view(acceptance_owner).data)[0].cargo = 63;
	}
	return callback == CBID_INDTILE_ACCEPT_CARGO ? accept_result : amount_result;
}
#include "industry-acceptance-reference.inc"
static void CheckAcceptance()
{
	OpenTTDIndustryTileServices service{};
	service.gfx = [](uint32_t tile) noexcept -> uint16_t { return GetIndustryGfx(tile); };
	service.industry = [](uint32_t tile) noexcept -> OpenTTDIndustryTileOwner { return {Industry::GetByTile(tile), acceptance_owner}; };
	service.spec = [](uint16_t gfx) noexcept -> const OpenTTDIndustryTileSpec * { return GetIndustryTileSpec(gfx); };
	service.acceptance = [](const OpenTTDIndustryTileSpec *spec) noexcept -> OpenTTDIndustryTileAcceptance {
		OpenTTDIndustryTileAcceptance result;
		std::ranges::copy(spec->accepts_cargo, result.cargo);
		std::ranges::copy(spec->acceptance, result.amount);
		result.accepts_all = spec->special_flags.Test(IndustryTileSpecialFlag::AcceptsAllCargo);
		result.accepts_callback = spec->callback_mask.Test(IndustryTileCallbackMask::AcceptCargo);
		result.amount_callback = spec->callback_mask.Test(IndustryTileCallbackMask::CargoAcceptance);
		return result;
	};
	service.callback = [](uint32_t tile, uint16_t gfx, uint16_t callback, uint32_t p1, uint32_t p2) noexcept -> uint16_t { return GetIndustryTileCallback(callback, p1, p2, gfx, Industry::GetByTile(tile), tile); };
	service.cargo_translation = [](const OpenTTDIndustryTileSpec *spec, uint8_t cargo) noexcept -> uint8_t { return GetCargoTranslation(cargo, spec->grf_prop.grffile); };
	Industry industry{}; active = &industry;
	OpenTTDIndustry *owner = openttd_rust_industry_new();
	uint cases = 0;
	for (uint8_t flags = 0; flags < 8; flags++) for (uint8_t pattern = 0; pattern < 4; pattern++) {
		for (uint16_t accepted : {0U, 0x7FFFU, 0x0401U, 0xFFFFU}) for (uint16_t amounts : {0U, 0x0FFFU, 0x0312U, 0xFFFFU}) {
			for (int8_t amount : {-128, -9, -8, -1, 0, 1, 119, 120, 127}) for (bool mutation : {false, true}) {
				auto setup = [&](bool candidate) {
					trace.clear(); acceptance_owner = candidate ? owner : nullptr;
					accept_result = accepted; amount_result = amounts; mutate_accepted = mutation;
					industry.accepted = {{2}, {5}, {255}, {2}};
					openttd_rust_industry_accepted_resize(owner, industry.accepted.size());
					auto *slots = static_cast<AcceptedSlot *>(openttd_rust_industry_accepted_view(owner).data);
					for (size_t n = 0; n < industry.accepted.size(); n++) slots[n].cargo = industry.accepted[n].cargo;
					auto &spec = specs[2]; spec.special_flags.value = (flags & 1) << 1; spec.callback_mask.value = ((flags & 2) << 2) | (flags & 4);
					for (uint n = 0; n < 16; n++) {
						spec.accepts_cargo[n] = pattern == 0 ? INVALID_CARGO : pattern == 1 ? n : pattern == 2 ? 2 : n % 3 == 0 ? INVALID_CARGO : n;
						spec.acceptance[n] = amount;
					}
				};
				CargoArray initial; initial.fill(UINT32_MAX - 4);
				setup(false); auto expected = initial; CargoTypes expected_always = 4;
				AddAcceptedCargo_Industry(2, expected, expected_always); const auto expected_trace = trace;
				setup(true); auto actual = initial; CargoTypes actual_always = 4;
				openttd_rust_industry_tile_acceptance(2, actual.data(), &actual_always, &service);
				if (actual != expected || actual_always != expected_always || trace != expected_trace) {
					std::fprintf(stderr, "acceptance flags %u pattern %u result %u/%u signed amount %d mutation %d\n", flags, pattern, accepted, amounts, amount, mutation);
					std::abort();
				}
				cases++;
			}
		}
	}
	openttd_rust_industry_destroy(owner); acceptance_owner = nullptr;
	std::printf("Industry acceptance absent-NewGRF reference cases: %u\n", cases);
}
static void CheckBuiltinAnimation(const OpenTTDIndustryServices &services)
{
	OpenTTDIndustryConstructionServices construction{};
	construction.animation_enabled = [](uint16_t) noexcept -> bool { return builtin_spec.animation.status != AnimationStatus::NoAnimation; };
	construction.set_construction_stage = [](uint32_t tile, uint8_t stage) noexcept { SetIndustryConstructionStage(tile, stage); };
	OpenTTDIndustryTileServices tileservice{};
	tileservice.gfx = [](uint32_t tile) noexcept -> uint16_t { return GetIndustryGfx(tile); };
	tileservice.set_gfx = [](uint32_t tile, uint16_t gfx) noexcept { SetIndustryGfx(tile, gfx); };
	tileservice.frame = [](uint32_t tile) noexcept -> uint8_t { return GetAnimationFrame(tile); };
	tileservice.set_frame = [](uint32_t tile, uint8_t frame) noexcept { SetAnimationFrame(tile, frame); };
	tileservice.animation_loop = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryAnimationLoop(tile); };
	tileservice.set_animation_loop = [](uint32_t tile, uint8_t loop) noexcept { SetIndustryAnimationLoop(tile, loop); };
	tileservice.ambient = []() noexcept -> bool { return _settings_client.sound.ambient; };
	tileservice.animate_new = [](uint32_t tile) noexcept { AnimateNewIndustryTile(tile); };
	tileservice.toffee_offset = [](uint8_t frame) noexcept -> uint8_t { return _industry_anim_offs_toffee[frame]; };
	builtin_mode = true;
	uint cases = 0;
	for (uint32_t seed : {0U, 1U, 0x12345678U, UINT32_MAX}) for (uint64_t tick : {0ULL, 1ULL, 2ULL, 3ULL, 7ULL, 8ULL, 1023ULL, 1024ULL, 1025ULL, 1056ULL, 1407ULL, 1408ULL, 1439ULL, 1440ULL, 1536ULL, 1568ULL, 1919ULL, 1920ULL, 1951ULL, 1952ULL, 2047ULL, static_cast<unsigned long long>(UINT64_MAX)}) {
		for (uint16_t gfx : {1, 10, 30, 31, 32, 48, 88, 143, 148, 151, 155, 162, 165, 174, 175}) for (uint8_t frame : {0, 1, 3, 4, 5, 6, 7, 14, 23, 28, 39, 49, 50, 56, 69, 70, 96, 127, 128, 130, 191, 192, 194, 195, 254, 255}) {
			if (gfx == GFX_TOFFEE_QUARRY && frame >= std::size(_industry_anim_offs_toffee)) continue; // Reference has no defined read outside its rendering table.
			for (uint8_t loop : {0, 7, 255}) for (bool ambient : {false, true}) for (bool custom : {false, true}) {
				auto setup = [&] { rng = seed; trace.clear(); builtin = {gfx, frame, loop, 0}; _settings_client.sound.ambient = ambient; TimerGameTick::counter = tick; builtin_spec.animation.status = custom ? AnimationStatus::Animated : AnimationStatus::NoAnimation; };
				setup(); AnimateTile_Industry(2);
				const auto expected_trace = trace; const auto expected_state = builtin; const auto expected_rng = rng;
				setup(); openttd_rust_industry_tile_animate(2, &services, &construction, &tileservice);
				if (trace != expected_trace || builtin != expected_state || rng != expected_rng) {
					std::fprintf(stderr, "builtin gfx %u tick %llu frame %u loop %u ambient %d custom %d: traces %zu/%zu\n", gfx, static_cast<unsigned long long>(tick), frame, loop, ambient, custom, trace.size(), expected_trace.size());
					std::abort();
				}
				cases++;
			}
		}
	}
	builtin_mode = false;
	std::printf("Industry builtin animation reference cases: %u\n", cases);
}

static OpenTTDIndustryLifecycleServices Lifecycle()
{
	OpenTTDIndustryLifecycleServices result{};
	result.belongs = [](uint32_t tile, uint16_t) noexcept -> bool { return tiles[tile].belongs; };
	result.randomisation_spec = [](uint32_t tile) noexcept -> OpenTTDIndustryRandomisationSpec { return {static_cast<uint16_t>(tile), specs[tile].grf_prop.enabled}; };
	result.random_triggers = [](uint32_t tile) noexcept -> uint8_t { return tiles[tile].triggers; };
	result.set_random_triggers = [](uint32_t tile, uint8_t bits) noexcept { SetIndustryRandomTriggers(tile, {bits}); };
	result.resolve_reseed = [](uint32_t tile, uint16_t gfx, void *industry, uint8_t waiting) noexcept -> OpenTTDIndustryReseed {
		IndustryTileResolverObject object(gfx, tile, static_cast<Industry *>(industry), 1);
		object.SetWaitingRandomTriggers({waiting});
		object.ResolveRerandomisation();
		return {object.reseed[0], object.reseed[1], object.GetUsedRandomTriggers().value};
	};
	result.random_bits = [](uint32_t tile) noexcept -> uint8_t { return GetIndustryRandomBits(tile); };
	result.set_random_bits = [](uint32_t tile, uint8_t bits) noexcept { SetIndustryRandomBits(tile, bits); };
	result.animation_spec = [](uint32_t tile) noexcept -> OpenTTDIndustryAnimationSpec { return {&specs[tile], specs[tile].animation.triggers.value}; };
	result.change_frame = [](const void *spec, uint32_t tile, uint32_t random, uint32_t parameter) noexcept { IndustryAnimationBase::ChangeAnimationFrame(0, static_cast<const IndustryTileSpec *>(spec), active, tile, random, parameter); };
	return result;
}
static void Require(bool condition) { if (!condition) std::abort(); }
int main()
{
	const auto lifecycle = Lifecycle();
	OpenTTDIndustryServices services{};
	auto shared = OpenTTDFixtureSharedServices(nullptr, [](void *) noexcept -> uint32_t { return Random(); }, nullptr);
	shared.mark_dirty = [](uint32_t tile) noexcept { MarkTileDirtyByTile(tile); };
	shared.delete_animated = [](uint32_t tile) noexcept { DeleteAnimatedTile(tile); };
	shared.tick_counter = []() noexcept -> uint64_t { return TimerGameTick::counter; };
	services.shared = &shared;
	services.play_sound = [](uint16_t sound, uint32_t tile) noexcept { SndPlayTileFx(sound, tile); };
	uint cases = 0;
	for (uint32_t seed : {0U, 1U, 0x12345678U, UINT32_MAX}) for (uint32_t mask : {0U, 1U, 0x55U, 0xFFFFU, 0x10000U, 0x12345678U, UINT32_MAX}) {
		for (uint8_t variant = 0; variant < 8; variant++) for (uint8_t trigger = 0; trigger < 5; trigger++) {
			auto setup = [&] {
				rng = seed; self_mask = mask; parent_mask = mask; trace.clear();
				for (uint tile = 0; tile < tiles.size(); tile++) {
					tiles[tile] = {static_cast<uint8_t>(tile * 7 + variant), static_cast<uint8_t>(tile * 23 + variant), tile != 1 && (variant != 7 || tile != 3)};
					specs[tile] = {{((variant >> (tile % 3)) & 1) != 0}, {{static_cast<uint8_t>(tile * 13 + variant)}}};
				}
			};
			Industry industry{}; active = &industry;
			OpenTTDIndustry *owner = openttd_rust_industry_new();
			auto *metadata = openttd_rust_industry_metadata(owner);
			metadata->tile = 0; metadata->width = 3; metadata->height = 2;
			for (uint8_t operation = 0; operation < 4; operation++) {
				if (operation != 2 && trigger >= 3) continue;
				setup(); industry.random = static_cast<uint16_t>(seed); metadata->random = industry.random;
				bool reference_result = true;
				if (operation == 0) TriggerIndustryRandomisation(&industry, static_cast<IndustryRandomTrigger>(trigger));
				if (operation == 1) TriggerIndustryTileRandomisation(2, static_cast<IndustryRandomTrigger>(trigger));
				if (operation == 3) reference_result = trigger == 0 ? TriggerIndustryTileAnimation_ConstructionStageChanged(2, variant & 1) : TriggerIndustryTileAnimation(2, static_cast<IndustryAnimationTrigger>(trigger));
				if (operation == 2) reference_result = TriggerIndustryAnimation(&industry, static_cast<IndustryAnimationTrigger>(trigger));
				const auto reference_trace = trace;
				const auto reference_tiles = tiles;
				const auto reference_rng = rng;
				const auto reference_random = industry.random;
				setup(); metadata->random = static_cast<uint16_t>(seed);
				bool candidate_result = true;
				if (operation == 0) openttd_rust_industry_randomise(&industry, owner, 0, trigger, 3, &services, &lifecycle);
				if (operation == 1) openttd_rust_industry_randomise_tile(2, trigger, &industry, owner, &services, &lifecycle);
				if (operation == 3) candidate_result = openttd_rust_industry_animate_tile(2, trigger, trigger == 0 && (variant & 1) ? 0x100 : 0, &services, &lifecycle);
				if (operation == 2) candidate_result = openttd_rust_industry_animate(owner, 0, trigger, 3, &services, &lifecycle);
				if (trace != reference_trace || rng != reference_rng || metadata->random != reference_random || candidate_result != reference_result) {
					std::fprintf(stderr, "seed %u mask %u variant %u trigger %u operation %u: RNG %u/%u, random %u/%u, result %d/%d, traces %zu/%zu\n", seed, mask, variant, trigger, operation, rng, reference_rng, metadata->random, reference_random, candidate_result, reference_result, trace.size(), reference_trace.size());
					std::abort();
				}
				for (uint tile = 0; tile < tiles.size(); tile++) Require(tiles[tile].triggers == reference_tiles[tile].triggers && tiles[tile].bits == reference_tiles[tile].bits);
				cases++;
			}
			openttd_rust_industry_destroy(owner);
		}
	}
	std::printf("Industry lifecycle absent-NewGRF reference cases: %u\n", cases);
	CheckBuiltinAnimation(services);
	CheckAcceptance();
}
