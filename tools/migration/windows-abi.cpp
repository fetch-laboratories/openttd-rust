/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file windows-abi.cpp Bounded first-32-bit layout and real C ABI call checks. */
#include "stdafx.h"
#include "tests/fixture_map.hpp"
#include "ground_vehicle.hpp"
#include "misc/history_type.hpp"
#include "rust/abi_ffi.h"
#include "rust/auth_ffi.h"
#include "rust/crypto_ffi.h"
#include "rust/alternating_ffi.h"
#include "rust/builder_ffi.h"
#include "rust/byte_strings_ffi.h"
#include "rust/cargo_flow_ffi.h"
#include "rust/cargo_payment_ffi.h"
#include "rust/cargo_storage_ffi.h"
#include "rust/command_cost_ffi.h"
#include "rust/command_services_ffi.h"
#include "rust/consumer_ffi.h"
#include "rust/encoded_ffi.h"
#include "rust/ffi.h"
#include "rust/history_ffi.h"
#include "rust/generic_ffi.h"
#include "rust/math_ffi.h"
#include "rust/orders_ffi.h"
#include "rust/packet_ffi.h"
#include "rust/rail_command_ffi.h"
#include "rust/station_command_ffi.h"
#include "rust/water_command_ffi.h"
#include "rust/tunnelbridge_command_ffi.h"
#include "bridge_type.h"
#include "rust/rail_yapf_ffi.h"
#include "rust/road_command_ffi.h"
#include "rust/script_list_ffi.h"
#include "rust/spiral_ffi.h"
#include "rust/station_cargo_ffi.h"
#include "rust/station_service_ffi.h"
#include "rust/station_association_ffi.h"
#include "rust/station_definitions_ffi.h"
#include "newgrf_station.h"
#include "newgrf_roadstop.h"
#include "newgrf_airport.h"
#include "newgrf_object.h"
#include "rust/string_validation_ffi.h"
#include "rust/town_founding_ffi.h"
#include "rust/town_ffi.h"
#include "rust/townname_ffi.h"
#include "rust/train_ffi.h"
#include "rust/train_reservation_ffi.h"
#include "rust/utf8_ffi.h"
#include "rust/vehicle_ffi.h"
#include "rust/vehicle_transaction_ffi.h"
#include "rust/vehicle_refit_ffi.h"
#include "rust/vehicle_visits_ffi.h"
#include "rust/vehicle_maintenance_ffi.h"
#include "rust/vehicle_services.h"
/* The shared town, industry and subsidy records use these headers' field types. */
#include "town.h"
#include "industry.h"
#include "subsidy_base.h"
#include "abi-includes.inc"
#include "rust/tile_type_access.hpp"
#include "tests/abi_layout.hpp"

#include <algorithm>
#include <array>
#include <cctype>
#include <clocale>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <initializer_list>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <vector>

#define CHECK(condition) do { if (!(condition)) { std::fprintf(stderr, "%s:%d: %s\n", __FILE__, __LINE__, #condition); std::abort(); } } while (0)

/* The one table of Rust ABI layouts for FFI types; native game structure lists stay in their tests. */
static void Layout(const char *name, std::initializer_list<size_t> values)
{
	CHECK(AbiLayoutMismatch(name, values) == -1);
	std::printf("layout %s", name);
	for (size_t value : values) std::printf(" %zu", value);
	std::printf("\n");
}

static void Layouts()
{
#include "abi-layout-probe.inc"
#include "abi-layout-packet.inc"
#include "abi-layout-record_test.inc"
#include "abi-layout-grf_loading.inc"
#include "abi-layout-act2.inc"
#include "abi-layout-linkgraph_schedule.inc"
#include "abi-layout-scheduler.inc"
#include "abi-layout-maintenance.inc"
#include "abi-layout-landscape.inc"
#include "abi-layout-genworld.inc"
#include "abi-layout-session.inc"
#include "abi-layout-settings.inc"
#include "abi-layout-script.inc"
#include "abi-layout-vehicle.inc"
#include "abi-layout-ground.inc"
#include "abi-layout-train_state.inc"
#include "abi-layout-content.inc"

	static_assert(std::is_same_v<decltype(OpenTTDAbiRecordFixture::tag), uint8_t>);
	static_assert(std::is_same_v<decltype(OpenTTDAbiRecordFixture::value), uint32_t>);
	static_assert(std::is_same_v<decltype(OpenTTDAbiRecordFixture::count), size_t>);
	static_assert(std::is_same_v<decltype(OpenTTDAbiRecordFixture::data), uint8_t *>);
	static_assert(std::is_same_v<decltype(OpenTTDAbiRecordFixture::next), const OpenTTDAbiRecordFixture *>);
	CHECK(openttd_rust_abi_layout_named("unknown", 0) == SIZE_MAX);
	CHECK(openttd_rust_abi_layout_named("OpenTTDTrainServices", SIZE_MAX) == SIZE_MAX);
	static_assert(std::is_same_v<decltype(OpenTTDSignalContext::log_x), unsigned int>);
	CHECK(static_cast<size_t>(PTRDIFF_MAX) == (SIZE_MAX >> 1));
	std::printf("pointer_bytes %zu sentinel %zu borrow_limit %zu\n", sizeof(void *), SIZE_MAX, static_cast<size_t>(PTRDIFF_MAX));

}

static void Calls()
{
	{
		static OpenTTDTownState *owner = openttd_rust_town_new(nullptr);
		OpenTTDSharedServices shared{}; OpenTTDTownLeaves leaves{}; OpenTTDTownNameServices names{}; OpenTTDCommandCostServices costs{}; OpenTTDTownFoundingServices founding{};
		founding.shared = &shared; founding.growth = &leaves; founding.names = &names; founding.cost = &costs; founding.close_error = 0x87654321;
		costs.error = [](OpenTTDCommandCost *cost, uint32_t error) noexcept { *reinterpret_cast<uint32_t *>(cost) = error; };
		leaves.environment.editor = []() noexcept { return false; };
		leaves.environment.freeform_edges = []() noexcept { return false; };
		names.next = [](uint from) noexcept -> OpenTTDTownRef { CHECK(from == 0 || from == UINT16_MAX); return from == 0 ? OpenTTDTownRef{reinterpret_cast<Town *>(uintptr_t{1}), owner, UINT16_MAX - 1} : OpenTTDTownRef{nullptr, nullptr, UINT16_MAX}; };
		names.utf8_length = [](const uint8_t *data, size_t size) noexcept -> size_t { CHECK(size == 1 && *data == 'x'); return size; };
		founding.name_style = []() noexcept -> uint8_t { return 0; };
		shared.current_company = []() noexcept -> uint8_t { return UINT8_MAX; };
		founding.found_setting = []() noexcept { return TF_CUSTOM_LAYOUT; };
		founding.can_allocate = []() noexcept { return true; };
		/* TileXY(32, 32) on a 64x64 map is 32 tiles from every edge. */
		const OpenTTDFixtureMap map{6};
		shared.map = &map.statics;
		shared.map_max_y = []() noexcept -> uint32_t { return 63; };
		reinterpret_cast<OpenTTDTownRecord *>(owner)->xy = TileIndex{2080};
		/* Preserve the sparse boundary ID while coordinates read canonical slots.
		 * The borrowed descriptor points at the same isolated owner as the callbacks. */
		std::vector<RustPoolSlot> town_slots(UINT16_MAX);
		town_slots[UINT16_MAX - 1] = {.shell = reinterpret_cast<Town *>(uintptr_t{1}), .record = owner};
		RustPoolDescriptor town_descriptor{
			.slots = {town_slots.data(), town_slots.size(), 0}, .used_bitmap = {},
			.first_free = 0, .first_unused = UINT16_MAX, .items = 1, .checked = 0,
			.cleaning = false, .alloc_cache = nullptr,
		};
		const RustPoolDescriptor *town_descriptor_ptr = &town_descriptor;
		shared.town_pool = &town_descriptor_ptr;
		openttd_rust_town_rebuild_index(&shared);
		founding.resolve = [](uint16_t id) noexcept -> OpenTTDTownRef { CHECK(id == UINT16_MAX); return {nullptr, nullptr, id}; };
		uint32_t error = 0; auto *cost = reinterpret_cast<OpenTTDCommandCost *>(&error); OpenTTDTownFoundingFrame frame; const uint8_t text = 'x';
		openttd_rust_town_found(&founding, nullptr, &frame, &frame.names, cost, 2080, TSZ_SMALL, false, TL_ORIGINAL, false, 0, {&text, 1}, false);
		CHECK(error == 0x87654321); openttd_rust_town_rename(&founding, cost, UINT16_MAX, {nullptr, 0}, false);
		CHECK(error == UINT16_MAX);
		town_slots[UINT16_MAX - 1].shell = nullptr; town_descriptor.items = 0;
		openttd_rust_town_rebuild_index(&shared);
		openttd_rust_town_destroy(owner);
	}
	std::unique_ptr<OpenTTDRoadState, decltype(&openttd_rust_road_destroy)> road(openttd_rust_road_new(), openttd_rust_road_destroy);
	for (uint8_t field = 0; field < 7; ++field) {
		CHECK(openttd_rust_road_get(road.get(), field) == 0);
		openttd_rust_road_set(road.get(), field, 65535);
		CHECK(openttd_rust_road_get(road.get(), field) == (field == 2 || field == 5 ? 65535 : 255));
	}
	CHECK(openttd_rust_road_path_size(road.get()) == 0);
	const OpenTTDRoadPathElement road_path[] = {{255, UINT32_MAX}, {9, 123456}};
	openttd_rust_road_path_replace(road.get(), road_path, 2);
	CHECK(openttd_rust_road_path_get(road.get(), 1).tile == 123456);
	openttd_rust_road_path_push(road.get(), {6, 42});
	CHECK(openttd_rust_road_path_get(road.get(), 2).trackdir == 6);
	openttd_rust_road_path_pop(road.get());
	CHECK(openttd_rust_road_path_size(road.get()) == 2);
	openttd_rust_road_path_clear(road.get());
	openttd_rust_road_path_replace(road.get(), nullptr, 0);
	CHECK(openttd_rust_road_path_size(road.get()) == 0);
	road.reset();
	const std::array<uint8_t, 8> bytes{0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE};
	constexpr uint64_t bits = UINT64_C(0xFEDCBA9876543210);
	auto le = openttd_rust_encode_uint_le(bits);
	CHECK(std::memcmp(le.bytes, bytes.data(), bytes.size()) == 0);
	auto number = openttd_rust_consumer_little_endian(bytes.data(), bytes.size(), 8);
	CHECK(number.value_bits == bits && number.length == 8);
	const std::string maximum = "ffffffffffffffff";
	auto integer = openttd_rust_parse_integer(reinterpret_cast<const uint8_t *>(maximum.data()), maximum.size(), 16, 64, 0, 0);
	CHECK(integer.value_bits == UINT64_MAX && integer.length == 16 && integer.error_kind == 0);
	auto formatted = openttd_rust_format_integer(UINT64_MAX, 0, 16);
	CHECK(formatted.length == maximum.size() && std::memcmp(formatted.bytes, maximum.data(), maximum.size()) == 0);
	auto empty_integer = openttd_rust_parse_integer(nullptr, 0, 10, 64, 0, 0);
	CHECK(empty_integer.length == 0 && empty_integer.error_kind == 1);
	CHECK(openttd_rust_skip_integer(nullptr, 0, 10) == 0);
	auto utf8 = openttd_rust_encode_utf8(0x1F600);
	CHECK(utf8.length == 4 && utf8.bytes[0] == 0xF0 && utf8.bytes[3] == 0x80);
	auto decoded = openttd_rust_decode_utf8(utf8.bytes, utf8.length);
	CHECK(decoded.length == 4 && decoded.codepoint == 0x1F600);
	CHECK(openttd_rust_decode_utf8(nullptr, 0).length == 0);
	CHECK(openttd_rust_utf8_next(utf8.bytes, utf8.length, 0) == 4);
	CHECK(openttd_rust_utf8_previous(utf8.bytes, utf8.length, 4) == 0);
	CHECK(openttd_rust_utf8_at_byte(utf8.bytes, utf8.length, 2) == 0);
	auto state = openttd_rust_alternating_initialize(SIZE_MAX - 1, 1);
	CHECK(state.position == SIZE_MAX - 1 && state.current_after == 1);
	auto step = openttd_rust_alternating_advance(state, SIZE_MAX);
	CHECK(step.state.position == SIZE_MAX && step.movement == 0 && step.state.current_after == 1);
	CHECK(openttd_rust_alternating_compare(state, step.state) == -1);
	auto bound = openttd_rust_consumer_bound(31, 7, SIZE_MAX);
	CHECK(bound.length == 24 && bound.position == 31 && bound.shortfall == 0);
	auto match = openttd_rust_consumer_prefix(nullptr, 0, nullptr, 0);
	CHECK(match.length == 0 && match.matched == 1);
	CHECK(openttd_rust_consumer_find(bytes.data(), bytes.size(), utf8.bytes, 1, 0) == SIZE_MAX);
	auto trimmed = openttd_rust_bytes_trim(nullptr, 0, nullptr, 0);
	CHECK(trimmed.offset == 0 && trimmed.length == 0);
	CHECK(openttd_rust_bytes_case(nullptr, 0, nullptr, 0, 0, std::is_signed_v<char>, 0) == 0);
	auto spiral = openttd_rust_spiral_square(10, 10, 3, 32, 32);
	CHECK(openttd_rust_spiral_equal(spiral, spiral) == 1 && openttd_rust_spiral_end(spiral) == 0);
	spiral = openttd_rust_spiral_advance(spiral, 32, 32);
	CHECK(openttd_rust_spiral_end(spiral) == 0);
	CHECK(openttd_rust_get_partial_pixel_z(3, 12, 0) == 0);
	CHECK(openttd_rust_get_partial_pixel_z(-2, 0, 0x03) == UINT32_MAX);
	CHECK(openttd_rust_get_partial_pixel_z(0, 0, 0x10) == UINT64_MAX);
	std::printf("real_calls high_bits=%llx passed\n", static_cast<unsigned long long>(number.value_bits));
}

using EncodedOwner = std::unique_ptr<OpenTTDRustEncodedResult, decltype(&openttd_rust_encoded_destroy)>;

static void Encoded()
{
	const std::array<uint8_t, 3> bytes{'A', 0, 'B'};
	const std::array<OpenTTDRustEncodedParameter, 3> parameters{{{UINT64_MAX, nullptr, 0, 1}, {0, bytes.data(), bytes.size(), 2}, {0, nullptr, 0, 0}}};
	std::printf("descriptor_array_alignment %zu address_mod %zu\n", alignof(OpenTTDRustEncodedParameter), reinterpret_cast<uintptr_t>(parameters.data()) % alignof(OpenTTDRustEncodedParameter));
	EncodedOwner owner(openttd_rust_encoded_serialize(42, parameters.data(), parameters.size(), 0), openttd_rust_encoded_destroy);
	auto view = openttd_rust_encoded_view(owner.get());
	std::string expected = "\xEE\x80\x81" "2a\x1E\xEE\x80\x82" "ffffffffffffffff\x1E\xEE\x80\x83";
	expected.append("A\0B", 3);
	expected.push_back('\x1E');
	CHECK(view.length == expected.size() && std::memcmp(view.bytes, expected.data(), view.length) == 0);
	std::string copied(reinterpret_cast<const char *>(view.bytes), view.length);
	owner.reset();
	CHECK(copied == expected);
	/* Byte storage is deliberately not aligned as a Rust descriptor. The boundary
	 * copies initialized field representations without any typed C++-storage borrow. */
	alignas(8) std::array<uint8_t, sizeof(parameters) + 8> storage{};
	std::memcpy(storage.data() + 1, parameters.data(), sizeof(parameters));
	auto raw = reinterpret_cast<const OpenTTDRustEncodedParameter *>(storage.data() + 1);
	CHECK(reinterpret_cast<uintptr_t>(raw) % alignof(OpenTTDRustEncodedParameter) != 0);
	owner.reset(openttd_rust_encoded_serialize(42, raw, parameters.size(), 0));
	view = openttd_rust_encoded_view(owner.get());
	CHECK(view.length == expected.size() && std::memcmp(view.bytes, expected.data(), view.length) == 0);
	owner.reset(openttd_rust_encoded_serialize(42, nullptr, 0, 0));
	view = openttd_rust_encoded_view(owner.get());
	CHECK(view.length == 5);
	owner.reset(openttd_rust_encoded_replace(reinterpret_cast<const uint8_t *>(expected.data()), expected.size(), 0, parameters[0], 0, 0));
	view = openttd_rust_encoded_view(owner.get());
	/* Original replacement does not parse a final empty record after its last separator. */
	CHECK(view.length == expected.size() - 1 && std::memcmp(view.bytes, expected.data(), view.length) == 0);
	const std::string invalid = "\xEE\x80\x80" "0\x1E\xEE\x80\x82" "10000000000000000";
	owner.reset(openttd_rust_encoded_negatives(reinterpret_cast<const uint8_t *>(invalid.data()), invalid.size()));
	view = openttd_rust_encoded_view(owner.get());
	CHECK(view.diagnostic_count == 1);
	auto diagnostic = openttd_rust_encoded_diagnostic(owner.get(), 0);
	CHECK(diagnostic.kind == 2 && diagnostic.offset == 8 && diagnostic.length == 17 && diagnostic.tail_length == 0);
	openttd_rust_encoded_destroy(nullptr);
	std::printf("encoded owners, by-value replacement/view/diagnostic and raw unaligned array passed\n");
}

static void Locale()
{
	CHECK(std::setlocale(LC_CTYPE, "C") != nullptr);
#ifdef _WIN32
	int previous = _configthreadlocale(_ENABLE_PER_THREAD_LOCALE);
	CHECK(previous != -1);
	CHECK(std::setlocale(LC_CTYPE, ".1252") != nullptr);
#endif
	uint8_t byte = 0xC9;
	uint8_t expected = static_cast<uint8_t>(std::tolower(static_cast<unsigned char>(byte)));
#ifdef _WIN32
	CHECK(expected == 0xE9); // A real non-C mapping is required to distinguish locale state.
#endif
	openttd_rust_bytes_lower(&byte, 1, 0);
	CHECK(byte == expected);
	std::printf("shared_locale %s lower_C9 %u\n", std::setlocale(LC_CTYPE, nullptr), static_cast<unsigned>(byte));
	CHECK(std::setlocale(LC_CTYPE, "C") != nullptr);
#ifdef _WIN32
	CHECK(_configthreadlocale(previous) != -1);
#endif
}

using HistoryOwner = std::unique_ptr<OpenTTDRustHistoryEngine, decltype(&openttd_rust_history_destroy)>;

static void HistoryDescribe(OpenTTDRustHistoryEngine *owner, const HistoryRange &range)
{
	auto step = openttd_rust_history_next(owner);
	CHECK(step.kind == 1 && step.token == reinterpret_cast<uintptr_t>(&range));
	openttd_rust_history_describe(owner, {reinterpret_cast<uintptr_t>(range.hr), range.periods, range.records, range.first, range.last, range.division, range.total_division, range.hr != nullptr ? range.hr->periods : uint8_t{0}, range.hr != nullptr ? range.hr->division : uint8_t{0}});
}

static void History()
{
	const HistoryRange leaf{2};
	const HistoryRange parent{leaf, 1, 2};
	constexpr uint64_t bits = UINT64_C(0xFEDCBA9876543210);
	HistoryOwner owner(openttd_rust_history_create(0, reinterpret_cast<uintptr_t>(&leaf), bits, 0, 0), openttd_rust_history_destroy);
	HistoryDescribe(owner.get(), leaf);
	auto step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 0 && step.value == (bits | 2));
	owner.reset(openttd_rust_history_create(1, reinterpret_cast<uintptr_t>(&leaf), bits | 2, 0, 0));
	HistoryDescribe(owner.get(), leaf);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 0 && step.value == 1);
	owner.reset(openttd_rust_history_create(2, reinterpret_cast<uintptr_t>(&leaf), bits | 2, 0, 1));
	HistoryDescribe(owner.get(), leaf);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 2 && step.first == 1 && step.last == 3);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 3 && step.first == 0 && step.target == 1);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 4 && step.target == 0);
	CHECK(openttd_rust_history_next(owner.get()).kind == 0);
	owner.reset(openttd_rust_history_create(3, reinterpret_cast<uintptr_t>(&parent), bits | 2, 0, 0));
	HistoryDescribe(owner.get(), parent);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 6 && step.count == 1);
	openttd_rust_history_phase(owner.get(), 0);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 7 && step.target == 0);
	HistoryDescribe(owner.get(), leaf);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 9 && step.first == 1);
	CHECK(openttd_rust_history_complete(owner.get()) == 1);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 8 && step.count == 1);
	CHECK(openttd_rust_history_complete(owner.get()) == 1);
	step = openttd_rust_history_next(owner.get());
	CHECK(step.kind == 0 && step.value == 1);
	std::printf("history descriptor identities, high-bit mask and staged by-value calls passed\n");
}

static void Math()
{
	/* Full uint32 input/output, including the rounded result beyond uint16. */
	CHECK(openttd_rust_int_sqrt(0) == 0);
	CHECK(openttd_rust_int_sqrt(6) == 2);
	CHECK(openttd_rust_int_sqrt(7) == 3);
	CHECK(openttd_rust_int_sqrt(UINT32_C(0x80000000)) == 46341);
	CHECK(openttd_rust_int_sqrt(UINT32_C(4294836225)) == 65535);
	CHECK(openttd_rust_int_sqrt(UINT32_MAX) == 65536);
	constexpr uint64_t bits = UINT64_C(0xFEDCBA9876543210);
	constexpr uint64_t high = UINT64_C(0x8000000000000000);
	/* The narrow signed reversed intervals intentionally retain C++ promotions. */
	CHECK(openttd_rust_soft_clamp(bits, high, UINT64_MAX, 64, 0) == bits);
	CHECK(openttd_rust_soft_clamp(0, high, UINT64_MAX, 64, 0) == high);
	CHECK(openttd_rust_soft_clamp(UINT64_MAX, 0, bits, 64, 0) == bits);
	CHECK(openttd_rust_soft_clamp(0, UINT64_MAX, 0, 64, 0) == high);
	CHECK(openttd_rust_soft_clamp(0, UINT64_MAX, UINT64_MAX - 2, 8, 1) == 126);
	CHECK(openttd_rust_soft_clamp(0, UINT64_MAX, UINT64_MAX - 2, 16, 1) == 32766);
	CHECK(openttd_rust_soft_clamp(0, 1500000000, static_cast<uint64_t>(-1500000000), 32, 1) == 0);
	CHECK(openttd_rust_soft_clamp(high, high, INT64_MAX, 64, 1) == high);
	std::printf("math int_sqrt and soft_clamp scalar high-bit, width and boundary calls passed\n");
}

static void Crypto()
{
	/* Monocypher's cofactored equation accepts the identity with zero S. Check
	 * both native uint8_t success values across the signature-check boundary. */
	std::array<uint8_t, 64> signature{};
	std::array<uint8_t, 32> public_key{};
	signature[0] = public_key[0] = 1;
	CHECK(openttd_rust_crypto_check(signature.data(), public_key.data(), nullptr, 0) == 1);
	signature[32] = 255;
	CHECK(openttd_rust_crypto_check(signature.data(), public_key.data(), nullptr, 0) == 0);
}

static void StationCargo()
{
	auto list = std::unique_ptr<OpenTTDScriptList, decltype(&openttd_rust_list_destroy)>(openttd_rust_list_new(), openttd_rust_list_destroy);
	OpenTTDCargoCollector state;
	CHECK(openttd_rust_cargo_plan(0, 1) == 1 && openttd_rust_cargo_plan(1, 3) == 3);
	CHECK(openttd_rust_cargo_plan(9, 0) == UINT8_MAX);
	openttd_rust_cargo_init(&state, 0, UINT16_MAX);
	openttd_rust_cargo_packet(&state, list.get(), UINT16_MAX, 2, UINT32_MAX);
	openttd_rust_cargo_packet(&state, list.get(), UINT16_MAX, 2, 2);
	openttd_rust_cargo_packet(&state, list.get(), 3, 2, 7);
	int64_t value = 0;
	CHECK(openttd_rust_list_get(list.get(), UINT16_MAX, &value) == 1 && value == 1);
	openttd_rust_cargo_finish(&state, list.get());
	CHECK(openttd_rust_list_get(list.get(), 3, &value) == 1 && value == 7);
	auto token = openttd_rust_list_token(list.get());
	openttd_rust_cargo_finish(&state, list.get());
	CHECK(openttd_rust_list_token(list.get()) == token);
	openttd_rust_cargo_init(&state, 2, 0);
	openttd_rust_cargo_origin(&state, 10);
	openttd_rust_cargo_share(&state, list.get(), 4, UINT32_MAX);
	openttd_rust_cargo_origin(&state, 11);
	openttd_rust_cargo_share(&state, list.get(), 4, 1);
	openttd_rust_cargo_finish(&state, list.get());
	CHECK(openttd_rust_list_get(list.get(), 4, &value) == 0);
	std::printf("station_cargo scalar layout, uint32 wrapping, origin reset and finalization passed\n");
}

static void PacketAssert(bool condition) noexcept { CHECK(condition); }

static void PacketState()
{
	static_assert(sizeof(intptr_t) == sizeof(size_t));
	bool quit = false;
	OpenTTDPacketServices services{
		.has_quit = [](const NetworkSocketHandler *socket) noexcept { return *reinterpret_cast<const bool *>(socket); },
		.mark_closed = [](NetworkSocketHandler *socket) noexcept { *reinterpret_cast<bool *>(socket) = true; },
	};
	OpenTTDPacketState state;
	openttd_rust_packet_init_send(&state, PacketAssert, reinterpret_cast<NetworkSocketHandler *>(&quit), 1, SIZE_MAX, 0);
	openttd_rust_packet_send_uint16(&state, PacketAssert, 0x0201);
	openttd_rust_packet_send_uint32(&state, PacketAssert, 0x06050403);
	openttd_rust_packet_begin_send(&state, PacketAssert);
	CHECK(state.data[0] == 9 && state.data[1] == 0 && state.limit == SIZE_MAX);
	CHECK(openttd_rust_packet_parse_size(&state) && state.position == 2);
	CHECK(openttd_rust_packet_recv_uint8(&state, &services) == 1);
	CHECK(openttd_rust_packet_recv_uint16(&state, &services) == 0x0201);
	CHECK(openttd_rust_packet_transfer_amount(&state, 2) == 2);
	openttd_rust_packet_transfer_commit(&state, -1);
	CHECK(state.position == 5);
	openttd_rust_packet_transfer_commit(&state, 65537);
	CHECK(state.position == 6);
	openttd_rust_packet_finish_read(&state, 65537);
	CHECK(state.position == 7);
	OpenTTDPacketFrame frame;
	CHECK(openttd_rust_packet_frame(2, 8, 2, &frame) && frame.message == 4 && frame.payload == 4);
	CHECK(!openttd_rust_packet_frame(2, 4, 2, &frame));
	CHECK(!openttd_rust_packet_can_read(&state, &services, 3, true) && quit);
	openttd_rust_packet_finish_send(&state);
	CHECK(state.position == 0);
	openttd_rust_packet_destroy(&state);
	std::printf("packet owned storage, typed binary reads and native cursor transitions passed\n");
}

static void StringValidation()
{
	static_assert(sizeof(char32_t) == sizeof(uint32_t));
	std::array<uint8_t, 4> input{13,10,0,0xFF};
	auto step = openttd_rust_validation_step(input.data(), input.size(), 15);
	CHECK(step.consumed == 1 && step.output.length == 0 && step.stopped == 0);
	step = openttd_rust_validation_step(input.data() + 1, input.size() - 1, 15);
	CHECK(step.consumed == 1 && step.output.length == 1 && step.output.bytes[0] == 10);
	step = openttd_rust_validation_step(input.data() + 2, 2, 15);
	CHECK(step.consumed == 1 && step.stopped == 1 && step.output.length == 0);
	step = openttd_rust_validation_step(input.data() + 3, 1, 255);
	CHECK(step.consumed == 1 && step.stopped == 0 && step.output.length == 0);
	std::array<uint8_t, 4> surrogate{0xED,0xA0,0x80,0};
	CHECK(openttd_rust_validation_valid(surrogate.data(), surrogate.size()) == 1);
	CHECK(openttd_rust_validation_valid(surrogate.data(), surrogate.size() - 1) == 0);
	std::array<uint8_t, 8> bytes{'a','b','c','d','e','f','g','h'};
	auto write = openttd_rust_inplace_write(bytes.data(), 0, 6, bytes.data() + 2, 4);
	CHECK(write.accepted == 1 && write.position == 4 && bytes[0] == 'c' && bytes[3] == 'f' && bytes[4] == 'e');
	write = openttd_rust_inplace_write(bytes.data(), 4, 4, input.data(), 1);
	CHECK(write.accepted == 0 && write.position == 4 && bytes[4] == 'e');
	write = openttd_rust_inplace_write(bytes.data(), 4, 1, input.data(), 1);
	CHECK(write.accepted == 1 && write.position == 5 && bytes[4] == 13);
	std::printf("string_validation historical policy, NUL, surrogate and live in-place copy passed\n");
}

static void ScriptListControl()
{
	auto *list = openttd_rust_list_new();
	auto *control = openttd_rust_list_control_new(5);
	OpenTTDListControlInput input{};
	OpenTTDListControlAction action{};
	auto step = [&] { openttd_rust_list_control_step(control, list, &input, &action); input = {}; };
	step(); CHECK(action.kind == LC_TYPE && action.a == 2);
	input.kind = 0x05000002; step(); CHECK(action.kind == LC_GET_INT);
	input.a = INT64_MIN; step(); CHECK(action.kind == LC_TYPE && action.a == 3);
	input.kind = 0x01000008; step(); CHECK(action.kind == LC_GET_BOOL);
	input.flag = UINT64_MAX; step(); CHECK(action.kind == LC_RETURN && action.a == 0);
	int64_t value;
	CHECK(openttd_rust_list_get(list, INT64_MIN, &value) == 1 && value == 1);
	openttd_rust_list_control_destroy(control);
	control = openttd_rust_list_control_new(4);
	step(); CHECK(action.kind == LC_TYPE);
	input.kind = 0x05000002; step(); CHECK(action.kind == LC_GET_INT);
	input.a = INT64_MIN; step(); CHECK(action.kind == LC_PUSH_INT && action.a == 1);
	step(); CHECK(action.kind == LC_RETURN && action.a == 1);
	openttd_rust_list_control_destroy(control);
	control = openttd_rust_list_control_new(1);
	step(); CHECK(action.kind == LC_TOP);
	input.a = 1; step(); CHECK(action.kind == LC_DISABLE);
	step(); CHECK(action.kind == LC_ITEM);
	input.flag = 1; step(); CHECK(action.kind == LC_ITEM);
	input.flag = 2; step(); CHECK(action.kind == LC_INDEX);
	input.a = INT64_MAX; step(); CHECK(action.kind == LC_ITEM);
	step(); CHECK(action.kind == LC_RETURN && action.a == 0);
	CHECK(openttd_rust_list_get(list, INT64_MAX, &value) == 1 && value == 0);
	openttd_rust_list_control_destroy(control);
	control = openttd_rust_list_control_new(3);
	step(); CHECK(action.kind == LC_TYPE && action.a == -1);
	input.kind = 0x01000001; step(); CHECK(action.kind == LC_RETURN && action.a == 0);
	openttd_rust_list_control_destroy(control);
	openttd_rust_list_destroy(list);
	std::printf("script_list control high-bit bool/key, typed filtering and load rejection passed\n");
}

/* Each component keeps its probes in windows-abi/NAME.inc; the generator lists every
 * fragment in abi-probes.inc, so adding a component edits no shared file here. */
static std::vector<void (*)()> &AbiProbes()
{
	static std::vector<void (*)()> probes;
	return probes;
}
#define ABI_PROBE(function) [[maybe_unused]] static const bool function##_registered = (AbiProbes().push_back(function), true)
#define ABI_PROBE_FIRST(function) [[maybe_unused]] static const bool function##_registered = (AbiProbes().insert(AbiProbes().begin(), function), true)
#include "abi-probes.inc"

int main()
{
	Layouts();
	for (void (*probe)() : AbiProbes()) probe();

	/* Opaque owner, native size_t, immutable byte borrow and complete UTF-8 output. */
	auto *townname = openttd_rust_townname_generate(1, UINT32_MAX);
	size_t name_length;
	const char *name = openttd_rust_townname_data(townname, &name_length);
	CHECK(name_length == 8 && std::memcmp(name, "Alen\xc3\xa7on", name_length) == 0);
	openttd_rust_townname_destroy(townname);
	Calls();
	Crypto();
	Encoded();
	History();
	Locale();
	Math();
	PacketState();
	ScriptListControl();
	StationCargo();
	StringValidation();
	std::printf("ABI audit passed\n");
}
