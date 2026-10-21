/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file industry-production-gap.cpp Unchanged production policy for absent NewGRF versions. */
#include "stdafx.h"
#include "core/math_func.hpp"
#include "core/bitmath_func.hpp"
#include "rust/industry_ffi.h"
#include <variant>
using CargoType = uint8_t;
static constexpr uint16_t CALLBACK_FAILED = 0xFFFF;
static constexpr int STR_NEWGRF_BUGGY = 0, STR_NEWGRF_BUGGY_ENDLESS_PRODUCTION_CALLBACK = 1, STR_NEWGRF_BUGGY_INVALID_CARGO_PRODUCTION_CALLBACK = 2, WL_WARNING = 0, WC_INDUSTRY_VIEW = 0;
enum class IndustryBehaviour { ProdMultiHandling = 14, ProdCallbackRandom = 15 };
struct IndustrySpec {
	struct { uint32_t bits; bool Test(IndustryBehaviour b) const { return HasBit(this->bits, static_cast<uint8_t>(b)); } } behaviour;
	struct { struct File { std::string filename; } *grffile; } grf_prop;
	uint32_t name = 0;
};
static IndustrySpec spec;
struct Slot { CargoType cargo; uint16_t waiting; };
struct Industry {
	OpenTTDIndustry *owner;
	std::vector<Slot> accepted, produced;
	uint8_t prod_level, type = 0;
	struct { uint32_t tile = 0; } location;
	uint16_t index = 0;
	auto GetCargoAccepted(CargoType c) { return c != 255 ? std::ranges::find(this->accepted, c, &Slot::cargo) : this->accepted.end(); }
	auto GetCargoProduced(CargoType c) { return c != 255 ? std::ranges::find(this->produced, c, &Slot::cargo) : this->produced.end(); }
};
static bool IsValidCargoType(CargoType c) { return c != 255; }
static const IndustrySpec *GetIndustrySpec(uint8_t) { return &spec; }
struct IndustryProductionSpriteGroup {
	uint8_t version, num_input = 3, num_output = 3;
	int32_t subtract_input[3] = {0, 1, 2}, add_output[3] = {3, 4, 5}, again = 6;
	uint8_t cargo_input[3] = {2, 255, 7}, cargo_output[3] = {5, 255, 9};
};
static IndustryProductionSpriteGroup group;
static uint32_t rng, resolve_limit, errors, dirty;
static int32_t values[7];
static std::vector<std::pair<uint32_t, uint32_t>> parameters;
static uint32_t Random() { return rng++; }
struct IndustriesResolverObject {
	uint32_t callback_param1 = 0, callback_param2 = 0;
	IndustriesResolverObject(uint32_t, Industry *, uint8_t) {}
	int32_t GetRegister(int field) const { return values[field]; }
	template <typename T> const T *Resolve() {
		parameters.emplace_back(this->callback_param1, this->callback_param2);
		if (parameters.size() > resolve_limit) return nullptr;
		return &group;
	}
};
template <typename... T> static int GetEncodedString(int, T &&...) { return 0; }
static void ShowErrorMessage(int, int, int) { errors++; }
static void SetWindowDirty(int, uint16_t) { dirty++; }
#include "industry-production-reference.inc"
struct Produced { uint8_t cargo; uint16_t waiting; uint8_t rate; uint16_t history[122]; };
struct Accepted { uint8_t cargo; uint16_t waiting; uint32_t accumulated; int32_t date; void *history; };
static void Resolve(void *, uint32_t random, uint32_t parameter, OpenTTDIndustryProductionResult *out)
{
	IndustriesResolverObject object(0, nullptr, 0);
	object.callback_param1 = random;
	object.callback_param2 = parameter;
	const auto *g = object.Resolve<IndustryProductionSpriteGroup>();
	out->present = g != nullptr;
	if (g == nullptr) return;
	out->version = g->version;
	if (g->version == 255) return;
	out->num_input = g->num_input;
	out->num_output = g->num_output;
	auto deref = [&](int field) { return g->version >= 1 ? object.GetRegister(field) : field; };
	for (size_t n = 0; n < 3; n++) {
		out->subtract[n] = deref(g->subtract_input[n]);
		out->add[n] = deref(g->add_output[n]);
		out->cargo_input[n] = g->cargo_input[n];
		out->cargo_output[n] = g->cargo_output[n];
	}
	out->again = deref(g->again);
}
int main()
{
	std::remove_pointer_t<decltype(spec.grf_prop.grffile)> file{};
	spec.grf_prop.grffile = &file;
	OpenTTDIndustryServices services{};
	auto shared = OpenTTDFixtureSharedServices(nullptr, [](void *) noexcept -> uint32_t { return Random(); });
	services.shared = &shared;
	services.set_dirty = [](void *) noexcept { dirty++; };
	services.callback_error = [](void *, bool) noexcept { errors++; };
	uint cases = 0;
	for (uint8_t version : {0, 1, 2, 255}) for (uint8_t level : {0, 4, 16, 128, 255}) for (uint32_t behaviour : {0u, 1u << 14, 1u << 15, 3u << 14}) {
		for (uint32_t limit : {0u, 1u, 3u, 65536u}) for (uint8_t reason : {0, 1}) {
			for (int32_t amount : {INT32_MIN, -65536, -1, 0, 1, 65536, INT32_MAX}) {
				group.version = version;
				uint8_t cargo = amount == INT32_MAX ? 254 : amount == INT32_MIN ? 64 : 2;
				group.cargo_input[0] = cargo;
				group.cargo_output[0] = cargo;
				resolve_limit = limit;
				spec.behaviour.bits = behaviour;
				std::fill(std::begin(values), std::end(values), amount);
				values[6] = limit == 1 ? 0 : 0x123;
				Industry reference{nullptr, {{cargo, 65535}, {255, 15}, {3, 1}}, {{cargo, 65535}, {255, 15}, {8, 1}}, level};
				rng = 0x12345678; errors = dirty = 0; parameters.clear();
				IndustryProductionCallback(&reference, reason);
				auto expected = parameters; auto expected_rng = rng; auto expected_errors = errors; auto expected_dirty = dirty;
				Industry candidate{openttd_rust_industry_new(), {}, {}, level};
				auto *fields = openttd_rust_industry_fields(candidate.owner); fields->prod_level = level;
				openttd_rust_industry_accepted_resize(candidate.owner, 3);
				openttd_rust_industry_produced_resize(candidate.owner, 3);
				auto a = openttd_rust_industry_accepted_view(candidate.owner);
				auto p = openttd_rust_industry_produced_view(candidate.owner);
				for (size_t n = 0; n < 3; n++) {
					auto *slot_a = static_cast<Accepted *>(a.data) + n; slot_a->cargo = n == 0 ? cargo : n == 1 ? 255 : 3; slot_a->waiting = n == 0 ? 65535 : n == 1 ? 15 : 1;
					auto *slot_p = static_cast<Produced *>(p.data) + n; slot_p->cargo = n == 0 ? cargo : n == 1 ? 255 : 8; slot_p->waiting = n == 0 ? 65535 : n == 1 ? 15 : 1;
				}
				rng = 0x12345678; errors = dirty = 0; parameters.clear();
				openttd_rust_industry_production_callback(&candidate, candidate.owner, nullptr, behaviour, reason, Resolve, &services);
				if (parameters != expected || rng != expected_rng || errors != expected_errors || dirty != expected_dirty) return 1;
				for (size_t n = 0; n < 3; n++) if (reference.accepted[n].waiting != static_cast<Accepted *>(a.data)[n].waiting || reference.produced[n].waiting != static_cast<Produced *>(p.data)[n].waiting) return 2;
				openttd_rust_industry_destroy(candidate.owner);
				cases++;
			}
		}
	}
	std::printf("industry NewGRF gap: %u unchanged-body version/repeat/arithmetic cases passed\n", cases);
}
