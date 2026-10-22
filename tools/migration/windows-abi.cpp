/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file windows-abi.cpp Bounded first-32-bit layout and real C ABI call checks. */
#include "stdafx.h"
#include "rust/subsidy_ffi.h"
#include "3rdparty/monocypher/monocypher.h"
#include "aircraft-boundary.hpp"
#include "misc/history_type.hpp"
#include "rust/abi_ffi.h"
#include "rust/aircraft_ffi.h"
#include "rust/auth_ffi.h"
#include "rust/alternating_ffi.h"
#include "rust/blake2b_ffi.h"
#include "rust/builder_ffi.h"
#include "rust/byte_strings_ffi.h"
#include "rust/cargo_flow_ffi.h"
#include "rust/cargo_payment_ffi.h"
#include "rust/cargo_storage_ffi.h"
#include "rust/command_cost_ffi.h"
#include "rust/command_services_ffi.h"
#include "rust/consumer_ffi.h"
#include "rust/crypto_primitives_ffi.h"
#include "rust/disaster_ffi.h"
#include "rust/effect_ffi.h"
#include "rust/encoded_ffi.h"
#include "rust/ffi.h"
#include "rust/history_ffi.h"
#include "rust/linkgraph_ffi.h"
#include "rust/math_ffi.h"
#include "rust/orders_ffi.h"
#include "rust/packet_ffi.h"
#include "rust/rail_command_ffi.h"
#include "rust/station_command_ffi.h"
#include "rust/water_command_ffi.h"
#include "bridge_type.h"
#include "rust/rail_yapf_ffi.h"
#include "rust/road_command_ffi.h"
#include "rust/road_ffi.h"
#include "rust/road_yapf_ffi.h"
#include "rust/script_list_ffi.h"
#include "rust/ship_control_ffi.h"
#include "rust/ship_yapf_ffi.h"
#include "rust/spiral_ffi.h"
#include "rust/station_cargo_ffi.h"
#include "rust/station_service_ffi.h"
#include "rust/string_validation_ffi.h"
#include "rust/town_founding_ffi.h"
#include "rust/townname_ffi.h"
#include "rust/train_ffi.h"
#include "rust/train_reservation_ffi.h"
#include "rust/trees_ffi.h"
#include "rust/utf8_ffi.h"
#include "rust/vehicle_ffi.h"
#include "rust/vehicle_transaction_ffi.h"
#include "rust/vehicle_refit_ffi.h"
#include "rust/vehicle_maintenance_ffi.h"
#include "rust/vehicle_services.h"
#include "rust/water_regions_ffi.h"
#include "rust/x25519_ffi.h"
#include "tests/abi_layout.hpp"
#include "tests/effect_protocol.hpp"
#include "tests/ship_yapf_protocol.hpp"
#include "tests/town_protocol.hpp"
#include "tests/water_regions_protocol.hpp"
// Registry: house (#256).
#include "rust/house_ffi.h"

// Registry: landscape (#212).
#include "rust/ground_ffi.h"
#include "rust/landscape_loop_ffi.h"
#include "rust/tile_type_access.hpp"
#include "tests/ground_protocol.hpp"
#include "tests/landscape_protocol.hpp"

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
#include "abi-layout-scheduler.inc"
#include "abi-layout-maintenance.inc"
#include "abi-layout-landscape.inc"

	CHECK(openttd_rust_abi_layout_named("unknown", 0) == SIZE_MAX);
	CHECK(openttd_rust_abi_layout_named("OpenTTDTrainServices", SIZE_MAX) == SIZE_MAX);
	static_assert(std::is_same_v<decltype(OpenTTDSignalContext::log_x), unsigned int>);
	CHECK(static_cast<size_t>(PTRDIFF_MAX) == (SIZE_MAX >> 1));
	std::printf("pointer_bytes %zu sentinel %zu borrow_limit %zu\n", sizeof(void *), SIZE_MAX, static_cast<size_t>(PTRDIFF_MAX));

}

static void Calls()
{
	CheckTownProtocol([](bool condition) { CHECK(condition); });
	{
		static OpenTTDTownState *owner = openttd_rust_town_new();
		OpenTTDSharedServices shared{}; OpenTTDTownLeaves leaves{}; OpenTTDTownNameServices names{}; OpenTTDCommandCostServices costs{}; OpenTTDTownFoundingServices founding{};
		founding.shared = &shared; founding.growth = &leaves; founding.names = &names; founding.cost = &costs; founding.close_error = 0x87654321;
		costs.error = [](OpenTTDCommandCost *cost, uint32_t error) noexcept { *reinterpret_cast<uint32_t *>(cost) = error; };
		leaves.environment.editor = []() noexcept { return false; };
		leaves.environment.freeform_edges = []() noexcept { return false; };
		names.next = [](uint from) noexcept -> OpenTTDTownRef { CHECK(from == 0 || from == UINT16_MAX); return from == 0 ? OpenTTDTownRef{reinterpret_cast<Town *>(uintptr_t{1}), owner, UINT16_MAX - 1} : OpenTTDTownRef{nullptr, nullptr, UINT16_MAX}; };
		names.utf8_length = [](const uint8_t *data, size_t size) noexcept -> size_t { CHECK(size == 1 && *data == 'x'); return size; };
		founding.parameters = [](OpenTTDTownFoundingFrame *) noexcept -> const TownNameParams * { return nullptr; };
		shared.current_company = []() noexcept -> uint8_t { return UINT8_MAX; };
		founding.found_setting = []() noexcept { return TF_CUSTOM_LAYOUT; };
		founding.can_allocate = []() noexcept { return true; };
		shared.distance_from_edge = [](uint32_t tile) noexcept -> uint32_t { CHECK(tile == UINT32_MAX); return 13; };
		founding.close_to_town = [](uint32_t tile, uint distance) noexcept { CHECK(tile == UINT32_MAX && distance == 20); return true; };
		founding.resolve = [](uint16_t id) noexcept -> OpenTTDTownRef { CHECK(id == UINT16_MAX); return {nullptr, nullptr, id}; };
		uint32_t error = 0; auto *cost = reinterpret_cast<OpenTTDCommandCost *>(&error); OpenTTDTownFoundingFrame frame; const uint8_t text = 'x';
		openttd_rust_town_found(&founding, nullptr, &frame, &frame.names, cost, UINT32_MAX, TSZ_SMALL, false, TL_ORIGINAL, false, 0, {&text, 1}, false);
		CHECK(error == 0x87654321); openttd_rust_town_rename(&founding, cost, UINT16_MAX, {nullptr, 0}, false);
		CHECK(error == UINT16_MAX); openttd_rust_town_destroy(owner);
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

static void CryptoPrimitives()
{
	/* Actual public facades invoke all 15 Rust functions with native descriptors.
	 * Counter high bits, initialized-only Poly1305 storage, callback convention,
	 * in-place failure/retry and rekey are observed on the current native target. */
	std::array<uint8_t, 32> key{}, h{};
	std::array<uint8_t, 24> nonce{};
	std::array<uint8_t, 65> cipher{}, plain{}, decoded{};
	for (size_t i = 0; i < key.size(); ++i) key[i] = static_cast<uint8_t>(i * 13 + 9);
	for (size_t i = 0; i < nonce.size(); ++i) nonce[i] = static_cast<uint8_t>(i * 7 + 101);
	nonce[0] = 0x12; nonce[1] = 0x34; nonce[2] = 0x56; nonce[3] = 0xF8;
	crypto_chacha20_h(h.data(), key.data(), nonce.data());
	CHECK(crypto_chacha20_djb(cipher.data(), nullptr, 65, key.data(), nonce.data(), UINT64_MAX) == 1);
	CHECK(crypto_chacha20_ietf(cipher.data(), nullptr, 65, key.data(), nonce.data(), UINT32_MAX) == 1);
	CHECK(crypto_chacha20_x(cipher.data(), nullptr, 65, key.data(), nonce.data(), UINT64_MAX) == 1);
	plain.fill(0x42);
	CHECK(crypto_chacha20_djb(cipher.data(), plain.data(), plain.size(), key.data(), nonce.data(), UINT64_C(0xFEDCBA9876543210)) == UINT64_C(0xFEDCBA9876543212));
	crypto_chacha20_djb(decoded.data(), cipher.data(), cipher.size(), key.data(), nonce.data(), UINT64_C(0xFEDCBA9876543210));
	CHECK(decoded == plain);
	crypto_poly1305_ctx poly;
	std::fill_n(reinterpret_cast<uint8_t *>(&poly), sizeof(poly), 0xA5);
	crypto_poly1305_init(&poly, key.data());
	CHECK(std::all_of(std::begin(poly.c), std::end(poly.c), [](uint8_t c) { return c == 0xA5; }));
	crypto_poly1305_update(&poly, nullptr, 0);
	crypto_poly1305_update(&poly, plain.data(), 15);
	crypto_poly1305_update(&poly, plain.data() + 15, plain.size() - 15);
	std::array<uint8_t, 16> mac{}, split{};
	crypto_poly1305_final(&poly, split.data());
	CHECK(std::all_of(reinterpret_cast<const uint8_t *>(&poly), reinterpret_cast<const uint8_t *>(&poly) + sizeof(poly), [](uint8_t c) { return c == 0; }));
	crypto_poly1305(mac.data(), plain.data(), plain.size(), key.data());
	CHECK(mac == split);
	for (unsigned variant = 0; variant != 3; ++variant) {
		crypto_aead_ctx send, receive;
		if (variant == 0) { crypto_aead_init_x(&send, key.data(), nonce.data()); crypto_aead_init_x(&receive, key.data(), nonce.data()); }
		if (variant == 1) { crypto_aead_init_djb(&send, key.data(), nonce.data()); crypto_aead_init_djb(&receive, key.data(), nonce.data()); }
		if (variant == 2) { crypto_aead_init_ietf(&send, key.data(), nonce.data()); crypto_aead_init_ietf(&receive, key.data(), nonce.data()); }
		auto counter = send.counter;
		CHECK(counter == (variant == 2 ? UINT64_C(0xF856341200000000) : uint64_t{0}));
		for (unsigned chunk = 0; chunk != 2; ++chunk) {
			crypto_aead_write(&send, cipher.data(), mac.data(), nonce.data(), 17, plain.data(), plain.size());
			CHECK(send.counter == counter);
			auto bad = mac; bad[0] ^= 1;
			decoded.fill(0xA5);
			auto before = receive;
			CHECK(crypto_aead_read(&receive, decoded.data(), bad.data(), nonce.data(), 17, cipher.data(), cipher.size()) == -1);
			CHECK(std::all_of(decoded.begin(), decoded.end(), [](uint8_t c) { return c == 0xA5; }));
			CHECK(receive.counter == before.counter && std::equal(std::begin(receive.key), std::end(receive.key), std::begin(before.key)));
			CHECK(crypto_aead_read(&receive, cipher.data(), mac.data(), nonce.data(), 17, cipher.data(), cipher.size()) == 0);
			CHECK(cipher == plain && receive.counter == counter && std::equal(std::begin(send.key), std::end(send.key), std::begin(receive.key)));
		}
	}
	crypto_aead_lock(cipher.data(), mac.data(), key.data(), nonce.data(), nonce.data(), 17, plain.data(), plain.size());
	CHECK(crypto_aead_unlock(decoded.data(), mac.data(), key.data(), nonce.data(), nonce.data(), 17, cipher.data(), cipher.size()) == 0 && decoded == plain);
	std::printf("crypto 15 Rust algorithms, cdecl leaves, caller layouts, counter high bits, partial init, padding and failed retry passed; poly_size=%zu poly_align=%zu aead_size=%zu aead_align=%zu\n", sizeof(poly), alignof(crypto_poly1305_ctx), sizeof(crypto_aead_ctx), alignof(crypto_aead_ctx));
}

static void Blake2b()
{
	std::array<uint8_t, 129> message;
	std::array<uint8_t, 128> key;
	for (size_t i = 0; i < message.size(); ++i) message[i] = static_cast<uint8_t>(i * 37 + 11);
	for (size_t i = 0; i < key.size(); ++i) key[i] = static_cast<uint8_t>(i * 13 + 7);
	std::array<uint8_t, 64> expected, actual;
	crypto_blake2b_keyed(expected.data(), 32, key.data(), 65, message.data(), message.size());
	crypto_blake2b_ctx ctx;
	std::fill_n(reinterpret_cast<uint8_t *>(&ctx), sizeof(ctx), 0xA5);
	crypto_blake2b_keyed_init(&ctx, 32, key.data(), 65);
	CHECK(ctx.input_idx == 128 && ctx.hash_size == 32 && ctx.input_offset[0] == 0 && ctx.input_offset[1] == 0);
	crypto_blake2b_update(&ctx, nullptr, 0);
	crypto_blake2b_update(&ctx, message.data(), 127);
	auto copy = ctx;
	crypto_blake2b_update(&ctx, message.data() + 127, 2);
	crypto_blake2b_final(&ctx, actual.data());
	CHECK(std::equal(actual.begin(), actual.begin() + 32, expected.begin()));
	CHECK(std::all_of(reinterpret_cast<const uint8_t *>(&ctx), reinterpret_cast<const uint8_t *>(&ctx) + sizeof(ctx), [](uint8_t x) { return x == 0; }));
	crypto_blake2b_update(&copy, message.data() + 127, 1);
	crypto_blake2b_final(&copy, actual.data());
	crypto_blake2b_keyed(expected.data(), 32, key.data(), 65, message.data(), 128);
	CHECK(std::equal(actual.begin(), actual.begin() + 32, expected.begin()));
	crypto_blake2b(expected.data(), 64, message.data(), message.size());
	crypto_blake2b_init(&ctx, 64);
	crypto_blake2b_update(&ctx, message.data(), 128);
	CHECK(ctx.input_idx == 128 && ctx.input_offset[0] == 0);
	crypto_blake2b_update(&ctx, message.data() + 128, 1);
	crypto_blake2b_final(&ctx, actual.data()); CHECK(actual == expected);
	crypto_blake2b_init(&ctx, 64); crypto_blake2b_update(&ctx, message.data(), 128);
	ctx.input_offset[0] = UINT64_MAX - 127; ctx.input_offset[1] = UINT64_C(0x1122334455667788);
	crypto_blake2b_update(&ctx, message.data() + 128, 1);
	CHECK(ctx.input_offset[0] == 0 && ctx.input_offset[1] == UINT64_C(0x1122334455667789) && ctx.input_idx == 1);
	crypto_blake2b_final(&ctx, actual.data());
	crypto_blake2b_init(&ctx, 0); crypto_blake2b_final(&ctx, nullptr);
	CHECK(std::all_of(reinterpret_cast<const uint8_t *>(&ctx), reinterpret_cast<const uint8_t *>(&ctx) + sizeof(ctx), [](uint8_t x) { return x == 0; }));
	std::array<uint8_t, 72> sentinel; sentinel.fill(0xA5);
	crypto_blake2b_keyed(sentinel.data(), 65, key.data(), 128, message.data(), message.size());
	CHECK(std::all_of(sentinel.begin() + 64, sentinel.end(), [](uint8_t x) { return x == 0xA5; }));
	std::printf("blake2b six facades, context size %zu align %zu offsets %zu %zu %zu %zu %zu; copy, carry, pending block, wipe and source sizes passed\n", sizeof(ctx), alignof(crypto_blake2b_ctx), offsetof(crypto_blake2b_ctx, hash), offsetof(crypto_blake2b_ctx, input_offset), offsetof(crypto_blake2b_ctx, input), offsetof(crypto_blake2b_ctx, input_idx), offsetof(crypto_blake2b_ctx, hash_size));
}

static size_t x25519_wipes, x25519_last_wipe;
static unsigned x25519_verifies;
static bool x25519_wipes_zero;
static void OPENTTD_CRYPTO_CALL X25519Wipe(void *data, size_t size) noexcept
{
	crypto_wipe(data, size);
	x25519_wipes++; x25519_last_wipe = size;
	const auto *bytes = static_cast<const uint8_t *>(data);
	x25519_wipes_zero &= std::all_of(bytes, bytes + size, [](uint8_t x) { return x == 0; });
}
static int32_t OPENTTD_CRYPTO_CALL X25519Verify32(const uint8_t *a, const uint8_t *b) noexcept
{
	x25519_verifies++;
	return crypto_verify32(a, b);
}
static void X25519()
{
	const OpenTTDX25519Leaves leaves{X25519Wipe, X25519Verify32};
	std::array<uint8_t, 32> secret, point{}, expected, actual;
	for (size_t i = 0; i < secret.size(); ++i) secret[i] = static_cast<uint8_t>(i * 37 + 13);
	point[0] = 9;
	crypto_x25519_public_key(expected.data(), secret.data());
	x25519_wipes = x25519_verifies = 0; x25519_wipes_zero = true;
	openttd_rust_x25519_public_key(&leaves, actual.data(), secret.data());
	CHECK(actual == expected && x25519_verifies == 4 && x25519_wipes == 29 && x25519_last_wipe == 32 && x25519_wipes_zero);
	x25519_wipes = x25519_verifies = 0;
	openttd_rust_x25519(&leaves, actual.data(), secret.data(), point.data());
	CHECK(actual == expected && x25519_verifies == 4 && x25519_wipes == 29 && x25519_last_wipe == 32 && x25519_wipes_zero);
	std::array<uint8_t, 64> storage;
	storage.fill(13); storage[33] = 0xA5;
	openttd_rust_x25519_trim(storage.data() + 1, storage.data());
	CHECK(storage[1] == 8 && storage[32] == 0x4D && storage[33] == 0xA5);
	CHECK(std::all_of(storage.begin() + 2, storage.begin() + 32, [](uint8_t x) { return x == 13; }));
	secret.fill(0); secret[31] = 0x80;
	std::array<uint8_t, 32> short_result, full_result;
	x25519_wipes = x25519_verifies = 0;
	openttd_rust_x25519_ladder(&leaves, short_result.data(), secret.data(), point.data(), 255);
	CHECK(x25519_verifies == 4 && x25519_wipes == 28 && x25519_last_wipe == 40 && x25519_wipes_zero);
	openttd_rust_x25519_ladder(&leaves, full_result.data(), secret.data(), point.data(), 256);
	CHECK(short_result != full_result && x25519_verifies == 8 && x25519_wipes == 56 && x25519_last_wipe == 40 && x25519_wipes_zero);
	std::printf("x25519 four exports, cdecl wipe/verify32, physical wipe counts/final sizes, forward trim and 255/256 high-bit distinction passed\n");
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

static void PacketState()
{
	static_assert(sizeof(intptr_t) == sizeof(size_t));
	OpenTTDPacketState state;
	openttd_rust_packet_init(&state, SIZE_MAX);
	CHECK(state.limit == SIZE_MAX && state.position == 0);
	CHECK(openttd_rust_packet_can_write(&state, SIZE_MAX, 1) == 1);
	CHECK(openttd_rust_packet_boolean(128) == 1 && openttd_rust_packet_boolean(0) == 0);
	CHECK(openttd_rust_packet_buffer_size(SIZE_MAX) == 1 && openttd_rust_packet_prefix(65537) == 1);
	std::array<uint8_t, 8> bytes{8, 0, 1, 2, 3, 4, 5, 6};
	CHECK(openttd_rust_packet_parse_size(&state, bytes.data(), bytes.size()) == 8);
	openttd_rust_packet_read_start(&state);
	CHECK(openttd_rust_packet_has_size(&state) == 1 && openttd_rust_packet_can_read(&state, 8, 6) == 1);
	CHECK(openttd_rust_packet_recv(&state, bytes.data(), bytes.size(), 2) == 0x0201 && state.position == 4);
	uint16_t count = 1;
	uint8_t byte = 0;
	CHECK(openttd_rust_packet_buffer_next(&state, bytes.data(), bytes.size(), &count, &byte) == 1 && byte == 3 && state.position == 5);
	CHECK(openttd_rust_packet_buffer_next(&state, bytes.data(), bytes.size(), &count, &byte) == 0 && count == UINT16_MAX);
	CHECK(openttd_rust_packet_remaining(&state, 8) == 3 && openttd_rust_packet_transfer_amount(&state, 8, 2) == 2);
	openttd_rust_packet_transfer_commit(&state, -1);
	CHECK(state.position == 5);
	openttd_rust_packet_transfer_commit(&state, 65537);
	CHECK(state.position == 6);
	openttd_rust_packet_skip_mac(&state, 65537);
	CHECK(state.position == 7);
	OpenTTDPacketFrame frame;
	CHECK(openttd_rust_packet_frame(2, 8, 2, &frame) == 1 && frame.message == 4 && frame.payload == 4);
	CHECK(openttd_rust_packet_frame(2, 4, 2, &frame) == 0);
	CHECK(openttd_rust_packet_send_amount(&state, SIZE_MAX - 2, 4) == 2);
	openttd_rust_packet_write_header(bytes.data(), bytes.size());
	CHECK(bytes[0] == 8 && bytes[1] == 0);
	openttd_rust_packet_send_reset(&state);
	CHECK(state.position == 0);
	std::printf("packet scalar framing, binary reads and native cursor transitions passed\n");
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

static uint8_t OPENTTD_LINKGRAPH_CALL LinkGraphAbort(const void *context) noexcept
{
	auto *state = static_cast<const std::array<uint32_t, 2> *>(context);
	return (*state)[0];
}

static void LinkGraphJob()
{
	std::array<OpenTTDLinkGraphNode, 2> nodes{{{10, 0, 12, 1, 1, 0, 1}, {0, 1, 24, 4, 1, 1, 0}}};
	OpenTTDLinkGraphEdge edge{20, 0, 1};
	OpenTTDLinkGraphSettings settings{2, 100, 100, 80, 1, 1, 127, 127, 30};
	std::array<uint32_t, 2> context{0, 0};
	auto *owner = openttd_rust_linkgraph_run(nodes.data(), nodes.size(), &edge, 1, &settings, &context, LinkGraphAbort);
	size_t count = 0;
	const auto *shares = openttd_rust_linkgraph_shares(owner, &count);
	CHECK(count == 2 && shares[0].node == 0 && shares[1].node == 1);
	CHECK(shares[0].origin == 12 && shares[0].via == 24 && shares[0].cumulative == 10 && shares[0].unrestricted == 10);
	CHECK(shares[1].origin == 12 && shares[1].via == 24 && shares[1].cumulative == 10 && shares[1].unrestricted == 10);
	CHECK(openttd_rust_linkgraph_edges(owner)[0] == 10);
	openttd_rust_linkgraph_destroy(owner);
	context[0] = 1;
	owner = openttd_rust_linkgraph_run(nodes.data(), nodes.size(), &edge, 1, &settings, &context, LinkGraphAbort);
	openttd_rust_linkgraph_shares(owner, &count);
	CHECK(count == 0 && openttd_rust_linkgraph_edges(owner)[0] == 0);
	openttd_rust_linkgraph_destroy(owner);
	std::printf("linkgraph owned snapshot/result and cdecl abort callback passed\n");
}

// ABI/reentry probe, not simulation evidence. The full game harness compares map/DATE.
struct TreeProbe {
	OpenTTDTreePlantObservation plant{.tile_type = 7, .bridge = false, .ground = 0, .density = 0, .snow = false, .coast = false, .zone = 0};
	OpenTTDTreeObservation tree{.ground = 3, .density = 3, .species = 20, .count = 2, .growth = 3, .zone = 0};
	uint32_t writes = 0, waters = 0, clears = 0;
	int64_t nested_cost = 0;
	bool abort = false;
	const OpenTTDSharedServices *shared = nullptr;
};
static TreeProbe tree_probe;
template <typename Result, typename... Args>
static Result TreeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
static uint32_t TreeRandom(void *) noexcept { return UINT32_MAX; }
static const OpenTTDTreeServices tree_probe_services{
	.plant_observation = [](uint32_t) noexcept { return tree_probe.plant; },
	.tree_observation = [](uint32_t) noexcept { return tree_probe.tree; },
	.tree_count = [](uint32_t) noexcept { return uint32_t{tree_probe.tree.count}; },
	.clear_observation = [](uint32_t) noexcept { return OpenTTDTreeClearObservation{.count = tree_probe.tree.count, .species = tree_probe.tree.species}; },
	.snow_line = TreeUnused<uint8_t>,
	.sin = [](float value) noexcept { return value; },
	.cos = [](float value) noexcept { return value; },
	.make_tree = [](uint32_t, uint8_t type, uint32_t count, uint8_t growth, uint8_t ground, uint32_t density) noexcept {
		++tree_probe.writes;
		CHECK(type == 12 && count == 2 && growth == 6 && ground == 0 && density == 2);
	},
	.set_ground_density = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.add_count = TreeUnused<void, uint32_t, int32_t>,
	.add_growth = TreeUnused<void, uint32_t, int32_t>,
	.set_growth = TreeUnused<void, uint32_t, uint8_t>,
	.make_clear = TreeUnused<void, uint32_t, uint8_t, uint32_t>,
	.ambient = TreeUnused<bool, uint32_t>,
	.progress = []() noexcept { return tree_probe.abort; },
	.progress_total = TreeUnused<bool, uint32_t>,
	.town_rating = TreeUnused<void, OpenTTDTreeCommand *, uint32_t, bool>,
	.command_begin = [](OpenTTDTreeCommand *, uint32_t, uint32_t, bool) noexcept { return INT32_MAX; },
	.command_tile = [](OpenTTDTreeCommand *) noexcept { return uint32_t{1}; },
	.command_next = [](OpenTTDTreeCommand *) noexcept { return UINT32_MAX; },
	.command_debit = TreeUnused<void, OpenTTDTreeCommand *>,
	.landscape_clear = TreeUnused<OpenTTDTreeLandscapeClear, OpenTTDTreeCommand *, uint32_t>,
};
static void Trees()
{
	OpenTTDSharedServices shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	shared.make_shore = TreeUnused<void, uint32_t>;
	shared.make_snow = TreeUnused<void, uint32_t, uint32_t>;
	shared.play_sound = TreeUnused<void, uint32_t, uint16_t>;
	static const OpenTTDClearServices clear{.make_clear = [](uint32_t, ClearGround, uint32_t) noexcept { ++tree_probe.clears; }};
	static const OpenTTDWaterGroundServices water{
		.rail = nullptr,
		.clear = &clear,
		.vehicles = nullptr,
		.costs = nullptr,
		.company = nullptr,
		.non_flooding = TreeUnused<bool, uint32_t>,
		.set_non_flooding = TreeUnused<void, uint32_t, bool>,
		.coast = TreeUnused<bool, uint32_t>,
		.set_tree_ground = TreeUnused<void, uint32_t, TreeGround, uint32_t>,
		.make_sea = TreeUnused<void, uint32_t>,
		.buoy_or_dock = TreeUnused<bool, uint32_t>,
		.canal_or_river = TreeUnused<bool, uint32_t>,
		.region_context = TreeUnused<OpenTTDWaterRegionContext>,
		.remove_rail = TreeUnused<void, OpenTTDCommandCost *, uint32_t, Track>,
		.apply_foundation = TreeUnused<OpenTTDFoundationSlope, Foundation, Slope>,
		.airport_tile = TreeUnused<bool, uint32_t>,
		.airport = TreeUnused<OpenTTDAirportGround, uint32_t>,
		.airport_member = TreeUnused<bool, const OpenTTDStation *, uint32_t>,
		.airport_z = TreeUnused<int32_t, uint32_t>,
		.victim = TreeUnused<OpenTTDFloodVictim, OpenTTDVehicle *>,
		.aircraft_pose = TreeUnused<OpenTTDAircraftFloodPose, OpenTTDVehicle *>,
		.ground_pose = TreeUnused<OpenTTDGroundFloodPose, OpenTTDVehicle *>,
		.crash = TreeUnused<uint32_t, OpenTTDVehicle *>,
		.ai_event = TreeUnused<void, OpenTTDVehicle *, uint32_t>,
		.game_event = TreeUnused<void, OpenTTDVehicle *, uint32_t>,
		.news = TreeUnused<void, OpenTTDVehicle *, uint32_t>,
		.bridge_height = TreeUnused<int32_t, uint32_t>,
	};
	shared.water_services = []() noexcept { return &water; };
	shared.map_max_x = []() noexcept { return 63U; };
	shared.map_max_y = []() noexcept { return 63U; };
	shared.tile_type = [](uint32_t) noexcept { return uint8_t{4}; };
	shared.tree_ground = [](uint32_t tile) noexcept {
		/* A water observation reenters trees; no owner borrow spans the call. */
		++tree_probe.waters;
		tree_probe.nested_cost = openttd_rust_trees_clear_tile(nullptr, tile + 1, 5, true, false, &tree_probe_services, tree_probe.shared);
		return static_cast<TreeGround>(0);
	};
	shared.clear_square_services = {
		.may_animate = [](uint32_t) noexcept { return false; },
		.docking = [](uint32_t) noexcept { return false; },
		.generating_world = []() noexcept { return false; },
	};
	shared.mark_dirty = [](uint32_t) noexcept {};
	shared.tile_valid = [](uint32_t) noexcept { return false; };
	tree_probe.shared = &shared;
	uint8_t *counter = openttd_rust_tree_counter();
	openttd_rust_trees_initialize();
	CHECK(*counter == 0);
	*counter = 37; // Same boundary write made by DATE/TTD descriptors.
	openttd_rust_trees_tick({.tick_counter = 0, .size = 4096, .climate = 0, .extra = 2}, &tree_probe_services, &shared);
	CHECK(*counter == 36 && counter == openttd_rust_tree_counter());
	openttd_rust_trees_tick({.tick_counter = 0, .size = 4096 * 4096, .climate = 0, .extra = 2}, &tree_probe_services, &shared);
	CHECK(*counter == 36 && tree_probe.writes == 0);
	tree_probe.plant = {.tile_type = 0, .bridge = false, .ground = 0, .density = 2, .snow = false, .coast = false, .zone = 0};
	openttd_rust_trees_plant(1, 12, 2, 6, &tree_probe_services, &shared);
	CHECK(tree_probe.writes == 1 && openttd_rust_trees_can_plant(1, false, &tree_probe_services));
	tree_probe.plant.tile_type = 4;
	const auto command = openttd_rust_trees_cmd_plant(nullptr, 1, 1, 255, false,
		{.build_price = 15, .size = 4096, .climate = 0, .editor = false, .execute = false, .company_valid = false}, &tree_probe_services, &shared);
	CHECK(command.cost == 30 && command.status == 0);
	tree_probe.plant.tile_type = 0;
	openttd_rust_trees_tile_loop(1, {.tick_counter = 0, .size_x = 64, .climate = 0, .extra = 2, .ambient = false}, nullptr, &tree_probe_services, &shared);
	CHECK(tree_probe.waters == 1 && tree_probe.clears == 1 && tree_probe.nested_cost == 40 && tree_probe.writes == 1);
	openttd_rust_trees_initialize();
	CHECK(*counter == 0 && counter == openttd_rust_tree_counter());
	tree_probe.abort = true;
	const OpenTTDTreeGenerateSettings generate{.size = 4096, .size_x = 64, .size_y = 64, .climate = 0, .placer = 1, .height_limit = 15, .editor = false, .freeform_edges = true};
	CHECK(openttd_rust_trees_generate(generate, &tree_probe_services, &shared));
	CHECK(openttd_rust_trees_place_randomly(generate, &tree_probe_services, &shared) && tree_probe.writes == 1);
	std::printf("tree_direct_services_counter_abort_and_reentry passed\n");
}

/* Complete typed tables exercise road cdecl calls without importing game globals.
 * These are boundary/lifetime checks, not a second simulation oracle. */
template <typename Function> struct VehicleUnused;
template <typename Result, typename... Args>
struct VehicleUnused<Result (OPENTTD_VEHICLE_CALL *)(Args...) noexcept> {
	static Result OPENTTD_VEHICLE_CALL Call(Args...) noexcept
	{
		CHECK(false);
		if constexpr (!std::is_void_v<Result>) return {};
	}
};
#define VEHICLE_UNUSED(table, field) table.field = VehicleUnused<decltype(table.field)>::Call
static OpenTTDVehicleMaintenanceServices _vehicle_maintenance_probe;
struct RoadTestTables {
	OpenTTDVehicleServices vehicle;
	OpenTTDGroundServices ground;
	OpenTTDSharedServices shared;
	OpenTTDRoadLeaves leaves;
	explicit RoadTestTables(const OpenTTDSharedServices *world) : shared(*world)
	{
		vehicle.maintenance = &_vehicle_maintenance_probe;
		VEHICLE_UNUSED(vehicle, write_counters);

		VEHICLE_UNUSED(vehicle, altitude_inputs);

		VEHICLE_UNUSED(vehicle, check_orders_only);
		VEHICLE_UNUSED(vehicle, write_coordinates);
		VEHICLE_UNUSED(vehicle, subtype);
		VEHICLE_UNUSED(vehicle, x);
		VEHICLE_UNUSED(vehicle, y);
		VEHICLE_UNUSED(vehicle, z);
		VEHICLE_UNUSED(vehicle, tick);
		VEHICLE_UNUSED(vehicle, owner);
		VEHICLE_UNUSED(vehicle, speed);
		VEHICLE_UNUSED(vehicle, subspeed);
		VEHICLE_UNUSED(vehicle, running);
		VEHICLE_UNUSED(vehicle, order_time);
		VEHICLE_UNUSED(vehicle, vehicle_type);
		VEHICLE_UNUSED(vehicle, write_x);
		VEHICLE_UNUSED(vehicle, write_y);
		VEHICLE_UNUSED(vehicle, write_z);
		VEHICLE_UNUSED(vehicle, write_tile);
		VEHICLE_UNUSED(vehicle, write_status);
		VEHICLE_UNUSED(vehicle, write_subspeed);
		VEHICLE_UNUSED(vehicle, write_breakdown_counter);
		VEHICLE_UNUSED(vehicle, write_running);
		VEHICLE_UNUSED(vehicle, write_order_time);
		VEHICLE_UNUSED(vehicle, write_profit);
		VEHICLE_UNUSED(vehicle, write_economy_service);
		VEHICLE_UNUSED(vehicle, write_calendar_service);
		VEHICLE_UNUSED(vehicle, write_breakdowns);
		VEHICLE_UNUSED(vehicle, write_reliability);
		VEHICLE_UNUSED(vehicle, write_cargo_age);
		VEHICLE_UNUSED(vehicle, coordinates);
		VEHICLE_UNUSED(vehicle, xy);
		VEHICLE_UNUSED(vehicle, speed_inputs);
		VEHICLE_UNUSED(vehicle, direction_progress);
		VEHICLE_UNUSED(vehicle, tick_status);
		VEHICLE_UNUSED(vehicle, day_cost);
		VEHICLE_UNUSED(vehicle, make_dummy);
		VEHICLE_UNUSED(vehicle, loading);
		VEHICLE_UNUSED(vehicle, write_tick);
		VEHICLE_UNUSED(vehicle, direction);
		VEHICLE_UNUSED(vehicle, status);
		VEHICLE_UNUSED(vehicle, tile);
		VEHICLE_UNUSED(vehicle, progress);
		VEHICLE_UNUSED(vehicle, dest_tile);
		VEHICLE_UNUSED(vehicle, order_type);
		VEHICLE_UNUSED(vehicle, order_destination);
		VEHICLE_UNUSED(vehicle, order_nonstop);
		VEHICLE_UNUSED(ground, has_articulated);
		VEHICLE_UNUSED(leaves, max_depot_penalty);
		VEHICLE_UNUSED(leaves, process_orders_then_loading);
		VEHICLE_UNUSED(vehicle, waiting_for_unbunching);
		VEHICLE_UNUSED(vehicle, leave_unbunching_depot);
		VEHICLE_UNUSED(vehicle, reset_depot_unbunching);
		VEHICLE_UNUSED(vehicle, enter_depot);
		VEHICLE_UNUSED(vehicle, pathfinding_result);
		VEHICLE_UNUSED(vehicle, new_position);
		VEHICLE_UNUSED(vehicle, move_then_status);
		VEHICLE_UNUSED(vehicle, base_viewport);
		VEHICLE_UNUSED(vehicle, property);
		VEHICLE_UNUSED(vehicle, callback);
		VEHICLE_UNUSED(vehicle, unknown_callback_result);
		VEHICLE_UNUSED(vehicle, length_changed);
		VEHICLE_UNUSED(vehicle, play_vehicle_sound);
		VEHICLE_UNUSED(vehicle, play_sound);
		VEHICLE_UNUSED(vehicle, large_explosion);
		VEHICLE_UNUSED(vehicle, order_free);
		VEHICLE_UNUSED(vehicle, order_dummy);
		VEHICLE_UNUSED(vehicle, start_stop_dirty);
		VEHICLE_UNUSED(vehicle, destroy);
		VEHICLE_UNUSED(vehicle, check_orders);
		VEHICLE_UNUSED(vehicle, day);
		VEHICLE_UNUSED(vehicle, write_day);
		VEHICLE_UNUSED(vehicle, write_direction);
		VEHICLE_UNUSED(vehicle, write_speed);
		VEHICLE_UNUSED(vehicle, last_station);
		VEHICLE_UNUSED(vehicle, write_last_station);
		VEHICLE_UNUSED(vehicle, write_dest);
		VEHICLE_UNUSED(vehicle, write_progress);
		VEHICLE_UNUSED(ground, move_incline);
		VEHICLE_UNUSED(ground, update_viewport_then_next);
		VEHICLE_UNUSED(ground, turn);
		VEHICLE_UNUSED(ground, do_update_speed);
		VEHICLE_UNUSED(ground, set_last_speed);
		VEHICLE_UNUSED(ground, acceleration);
		VEHICLE_UNUSED(ground, crash);
		VEHICLE_UNUSED(ground, first_cargo_changed);
		VEHICLE_UNUSED(ground, go_to_depot_service);
		VEHICLE_UNUSED(ground, write_first_engine);
		VEHICLE_UNUSED(ground, write_length);
		VEHICLE_UNUSED(shared, tile_type);
		VEHICLE_UNUSED(shared, tile_owner);
		VEHICLE_UNUSED(shared, station_index);
		VEHICLE_UNUSED(shared, depot_index);
		VEHICLE_UNUSED(shared, is_level_crossing);
		VEHICLE_UNUSED(shared, tunnel_bridge_direction);
		VEHICLE_UNUSED(shared, bridge_speed);
		VEHICLE_UNUSED(leaves, speed_read);
		VEHICLE_UNUSED(leaves, max_speed_read);
		VEHICLE_UNUSED(leaves, speed_part);
		VEHICLE_UNUSED(leaves, step);
		VEHICLE_UNUSED(leaves, crash_part);
		VEHICLE_UNUSED(leaves, part);
		VEHICLE_UNUSED(leaves, link);
		VEHICLE_UNUSED(leaves, link_tile);
		VEHICLE_UNUSED(leaves, enter_tile);
		VEHICLE_UNUSED(leaves, visit_close);
		VEHICLE_UNUSED(leaves, visit_trains);
		VEHICLE_UNUSED(leaves, visit_tile);
		VEHICLE_UNUSED(leaves, has_road);
		VEHICLE_UNUSED(leaves, track_status);
		VEHICLE_UNUSED(leaves, any_road_bits);
		VEHICLE_UNUSED(leaves, is_road_depot_tile);
		VEHICLE_UNUSED(leaves, depot_direction);
		VEHICLE_UNUSED(leaves, is_normal_road);
		VEHICLE_UNUSED(leaves, disallowed_directions);
		VEHICLE_UNUSED(leaves, is_bay_stop);
		VEHICLE_UNUSED(leaves, bay_direction);
		VEHICLE_UNUSED(leaves, is_drive_through);
		VEHICLE_UNUSED(leaves, is_station_road_stop);
		VEHICLE_UNUSED(leaves, stop_type);
		VEHICLE_UNUSED(leaves, has_free_bay);
		VEHICLE_UNUSED(leaves, continuation);
		VEHICLE_UNUSED(leaves, move_tile);
		VEHICLE_UNUSED(leaves, path_read);
		VEHICLE_UNUSED(leaves, choose_track);
		VEHICLE_UNUSED(leaves, find_depot);
		VEHICLE_UNUSED(leaves, tram_build_read);
		VEHICLE_UNUSED(vehicle, previous_tile);
		VEHICLE_UNUSED(leaves, disconnect);
		VEHICLE_UNUSED(leaves, stop_read);
		VEHICLE_UNUSED(leaves, should_stop);
		VEHICLE_UNUSED(leaves, stop_leave);
		VEHICLE_UNUSED(leaves, stop_entrance);
		VEHICLE_UNUSED(leaves, stop_entrance_busy);
		VEHICLE_UNUSED(leaves, arrival_read);
		VEHICLE_UNUSED(leaves, arrival_news);
		VEHICLE_UNUSED(leaves, begin_loading_at);
		VEHICLE_UNUSED(leaves, overtake_read);
		VEHICLE_UNUSED(leaves, sound_read);
		VEHICLE_UNUSED(leaves, depot_exit);
		VEHICLE_UNUSED(leaves, crash_news);
		VEHICLE_UNUSED(vehicle, detach_last);
		VEHICLE_UNUSED(ground, cache_begin);
		VEHICLE_UNUSED(vehicle, next);
		VEHICLE_UNUSED(vehicle, engine_id);
		VEHICLE_UNUSED(vehicle, engine);
		VEHICLE_UNUSED(vehicle, engine_for_length);
		VEHICLE_UNUSED(ground, cached_length);
		VEHICLE_UNUSED(leaves, shorten_factor);
		VEHICLE_UNUSED(leaves, default_max_speed);
		VEHICLE_UNUSED(leaves, update_cargo_age);
		VEHICLE_UNUSED(vehicle, write_max_speed);
		VEHICLE_UNUSED(leaves, cost_read);
		VEHICLE_UNUSED(leaves, price);
		VEHICLE_UNUSED(leaves, pay_running);
		VEHICLE_UNUSED(leaves, service_read);
		VEHICLE_UNUSED(leaves, turn_read);
		VEHICLE_UNUSED(leaves, trackdir_read);
		leaves.vehicle = &vehicle;
		leaves.ground = &ground;
		leaves.shared = &shared;
	}
	RoadTestTables(const RoadTestTables &) = delete;
	RoadTestTables &operator=(const RoadTestTables &) = delete;
};
#undef VEHICLE_UNUSED
struct RoadAbiProbe {
	OpenTTDRoadState *state = openttd_rust_road_new();
	const OpenTTDRoadLeaves *leaves = nullptr;
	OpenTTDRoadEntry entry{.acceleration_model = 0, .road_side = 0, .queue = false, .map_log_x = 8};
	uint32_t dest = 0, nested = 0, destroyed = 0, departures = 0, progress = 255;
	uint8_t tick = 0, running = 0, status = 0;
	uint32_t cache_properties = 0, cargo_ages = 0, length_changes = 0, viewports = 0, turns = 0, next_reads = 0;
	uint32_t progress_writes = 0, visual_stage = 0;
	uint16_t post_effect_max = 80;
	bool visual_eligible = true, reset_status = false;
	bool fallback_cache = false, engine_replaced = false;
	bool hidden_viewport_tail = false;
	int32_t order_time = 0;
};
static RoadAbiProbe *road_abi_probe;
static int road_abi_second, road_abi_hidden, road_abi_engine, road_abi_new_engine;
static int road_abi_vehicle; ///< Its address is the opaque vehicle handle.
static OpenTTDVehicle *RoadAbiHandle() { return reinterpret_cast<OpenTTDVehicle *>(&road_abi_vehicle); }
static OpenTTDRoadMaxSpeedRead RoadAbiMaxSpeed()
{
	return {.next = nullptr, .next_state = nullptr, .tile = 100, .max_track_speed = 80, .order_max_speed = 65535, .direction = 1, .status = 0};
}
static void RoadBoundary()
{
	RoadAbiProbe probe;
	road_abi_probe = &probe;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	probe.leaves = &t.leaves;
	OpenTTDVehicle *v = RoadAbiHandle();

	/* An articulated part only stores its counters. */
	t.vehicle.write_counters = [](OpenTTDVehicle *u, uint8_t tick, uint8_t running, int32_t order_time) noexcept {
		CHECK(u == RoadAbiHandle());
		auto &p = *road_abi_probe;
		p.tick = tick;
		p.running = running;
		p.order_time = order_time;
		if (p.reset_status) p.status = 0;
	};
	t.vehicle.write_tick = [](OpenTTDVehicle *u, uint8_t tick) noexcept { CHECK(u == RoadAbiHandle()); road_abi_probe->tick = tick; };
	const OpenTTDRoadTickRead part{.tick = 255, .front = false};
	CHECK(openttd_rust_road_tick(v, probe.state, &part, &t.leaves, nullptr));
	CHECK(probe.tick == 0 && probe.running == 0 && probe.order_time == 0 && world.draws == 0);

	/* A direct callback changes canonical state before Rust reobserves it. */
	t.ground.crash = [](OpenTTDVehicle *u, bool flooded) noexcept -> OpenTTDGroundCrash {
		CHECK(u == RoadAbiHandle() && !flooded);
		openttd_rust_road_set(road_abi_probe->state, 0, 64);
		return {.victims = 7, .front = true};
	};
	t.leaves.stop_leave = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->departures; };
	CHECK(openttd_rust_road_crash(v, probe.state, false, &t.leaves) == 8);
	CHECK(probe.departures == 1 && openttd_rust_road_get(probe.state, 5) == 1);

	/* Entries nested in a NewGRF property callback see and change the same owner. */
	t.leaves.trackdir_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadTrackdirRead { return {.tile = 100, .direction = 1, .status = 0}; };
	t.leaves.is_bay_stop = [](uint32_t) noexcept { return false; };
	t.vehicle.write_dest = [](OpenTTDVehicle *, uint32_t tile) noexcept { road_abi_probe->dest = tile; };
	t.ground.cache_begin = [](OpenTTDVehicle *) noexcept {};
	t.vehicle.engine_for_length = [](OpenTTDVehicle *) noexcept -> OpenTTDEngineForLength {
		auto &p = *road_abi_probe;
		CHECK(p.cache_properties == p.cargo_ages);
		return {.engine = reinterpret_cast<const OpenTTDEngine *>(p.engine_replaced ? &road_abi_new_engine : &road_abi_engine), .grf_version = 8};
	};
	t.vehicle.next = [](OpenTTDVehicle *u) noexcept -> OpenTTDVehicle * {
		auto &p = *road_abi_probe;
		CHECK(p.cargo_ages == (u == RoadAbiHandle() ? 1U : 2U));
		return p.fallback_cache && u == RoadAbiHandle() ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) : nullptr;
	};
	t.vehicle.engine_id = [](OpenTTDVehicle *u) noexcept -> uint16_t {
		CHECK(u == RoadAbiHandle() && road_abi_probe->cargo_ages == 1);
		return 9; // The first part's callbacks changed the head engine.
	};
	t.leaves.shorten_factor = [](const OpenTTDEngine *e) noexcept -> uint8_t {
		CHECK(road_abi_probe->fallback_cache && e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine) && road_abi_probe->cache_properties == 1);
		return 2; // Read through the engine captured before the property callback.
	};
	t.ground.cached_length = [](OpenTTDVehicle *) noexcept -> uint8_t {
		CHECK(road_abi_probe->fallback_cache && road_abi_probe->cache_properties > road_abi_probe->cargo_ages);
		return 8;
	};
	t.vehicle.length_changed = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->length_changes; };
	t.vehicle.unknown_callback_result = [](const OpenTTDEngine *e, CallbackID callback, uint16_t result) noexcept {
		CHECK(e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_new_engine) && callback == 0x11 && result == 9);
	};
	t.leaves.default_max_speed = [](OpenTTDVehicle *) noexcept -> uint16_t {
		CHECK(road_abi_probe->fallback_cache && road_abi_probe->cache_properties == 3);
		return 99;
	};
	t.ground.write_first_engine = [](OpenTTDVehicle *, uint16_t engine) noexcept { CHECK(engine == 0xFFFF || (road_abi_probe->fallback_cache && engine == 9)); };
	t.vehicle.property = [](OpenTTDVehicle *u, uint8_t property, int32_t fallback) noexcept -> int32_t {
		auto &p = *road_abi_probe;
		++p.cache_properties;
		if (p.fallback_cache) {
			p.engine_replaced = true;
			return property == 0x23 ? (u == RoadAbiHandle() ? 0xFFFF : 9) : 0;
		}
		const OpenTTDRoadMaxSpeedRead read = RoadAbiMaxSpeed();
		CHECK(openttd_rust_road_max_speed(u, p.state, p.entry.acceleration_model, &read, p.leaves) == 80);
		CHECK(openttd_rust_road_trackdir(u, p.state, p.leaves) == 0);
		++p.nested;
		openttd_rust_road_set_dest(u, p.state, 142 + p.nested, p.dest, p.leaves);
		CHECK(openttd_rust_road_path_size(p.state) == 0);
		openttd_rust_road_path_push(p.state, {10, 10815});
		return property == 0x23 ? 0 : property == 0x15 ? 20 : fallback;
	};
	t.ground.write_length = [](OpenTTDVehicle *u, uint8_t length, OpenTTDVehicle *, uint16_t total) noexcept {
		CHECK(road_abi_probe->fallback_cache ? (u == RoadAbiHandle() ? length == 6 && total == 6 : length == 1 && total == 7) : length == 8 && total == 8);
	};
	t.leaves.update_cargo_age = [](OpenTTDVehicle *) noexcept { ++road_abi_probe->cargo_ages; };
	t.vehicle.write_max_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == (road_abi_probe->fallback_cache ? 99 : 80)); };
	openttd_rust_road_update_cache(v, probe.state, false, &t.leaves);
	CHECK(probe.nested == 2 && probe.dest == 144 && openttd_rust_road_path_get(probe.state, 0).tile == 10815);

	/* Failed/invalid properties consume late fallback reads and the post-callback link.
	 * The first, !same_length run rejects conditional reads; a changed engine
	 * still leaves fallback/error reads attached to the captured engine context. */
	probe.fallback_cache = true;
	probe.cache_properties = probe.cargo_ages = 0;
	openttd_rust_road_update_cache(v, probe.state, true, &t.leaves);
	CHECK(probe.cargo_ages == 2 && probe.length_changes == 2);

	/* Running-cost property callbacks can replace the vehicle's engine. Price retains the original engine. */
	probe.engine_replaced = false;
	t.leaves.cost_read = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadCostRead {
		CHECK(u == RoadAbiHandle() && !road_abi_probe->engine_replaced);
		return {.engine = reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine), .cost_class = 1, .cost_factor = 3};
	};
	t.vehicle.property = [](OpenTTDVehicle *u, uint8_t property, int32_t value) noexcept -> int32_t {
		CHECK(u == RoadAbiHandle() && property == 9 && value == 3);
		road_abi_probe->engine_replaced = true;
		return 4;
	};
	t.leaves.price = [](const OpenTTDEngine *e, uint32_t factor) noexcept -> int64_t {
		CHECK(road_abi_probe->engine_replaced && e == reinterpret_cast<const OpenTTDEngine *>(&road_abi_engine) && factor == 4);
		return 123;
	};
	CHECK(openttd_rust_road_running_cost(v, probe.state, &t.leaves) == 123);

	/* A non-station order must not read its destination at the bay stopping frame. */
	openttd_rust_road_set(probe.state, 0, 32);
	openttd_rust_road_set(probe.state, 1, 20);
	openttd_rust_road_set(probe.state, 6, 1); // Skip nearby-vehicle traversal.
	t.leaves.step = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadStep {
		uint16_t rd = openttd_rust_road_drive_entry(0, 32, 21);
		CHECK((rd & 0xC0) == 0 && (rd & 15) < 15);
		return {.tile = 100, .x = (rd & 15) + 1, .y = (rd >> 8) & 15, .z = 0, .cur_speed = 0, .direction = 1, .length = 8, .front = true, .tram = false};
	};
	t.leaves.link = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadLink { return {.next = nullptr, .next_state = nullptr}; };
	t.shared.station_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == 100); return 55; };
	t.leaves.is_drive_through = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	t.leaves.stop_entrance = [](OpenTTDVehicle *, bool busy) noexcept { CHECK(!busy); };
	t.vehicle.write_last_station = [](OpenTTDVehicle *, uint16_t station) noexcept { CHECK(station == 55); };
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.leaves.is_bay_stop = [](uint32_t tile) noexcept { CHECK(tile == 100); return false; };
	t.vehicle.play_vehicle_sound = [](OpenTTDVehicle *, VehicleSoundEvent event) noexcept { CHECK(event == 1); return true; };
	t.vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	t.leaves.enter_tile = [](OpenTTDVehicle *, uint32_t, int32_t, int32_t) noexcept -> OpenTTDRoadEntered { return {.tile = 100, .cur_speed = 0, .flags = 4, .direction = 1, .order_type = 0}; };
	t.vehicle.write_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == 0); };
	CHECK(!openttd_rust_road_individual(v, probe.state, nullptr, nullptr, &probe.entry, &t.leaves, nullptr));
	CHECK(openttd_rust_road_get(probe.state, 0) == 36);
	openttd_rust_road_set(probe.state, 0, 0);
	openttd_rust_road_set(probe.state, 6, 0);

	/* Visual eligibility precedes lookup; effect reentry changes the later speed inputs. */
	t.leaves.process_orders_then_loading = [](OpenTTDVehicle *u) noexcept -> uint8_t {
		auto &p = *road_abi_probe;
		openttd_rust_road_set_dest(u, p.state, 100, p.dest, p.leaves);
		return 0;
	};
	t.leaves.visual_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadVisualRead {
		auto &p = *road_abi_probe; p.visual_stage = 1;
		auto read = RoadAbiMaxSpeed();
		if (!p.visual_eligible) { read.next = reinterpret_cast<OpenTTDVehicle *>(&road_abi_second); read.next_state = p.state; } // A skipped lookup must not traverse.
		return {.max = read, .eligible = p.visual_eligible};
	};
	t.leaves.visual_effect_then_speed = [](OpenTTDVehicle *u, uint32_t max, bool realistic) noexcept -> OpenTTDRoadSpeedRead {
		auto &p = *road_abi_probe;
		CHECK(p.visual_stage == 1 && p.visual_eligible && max == 80); p.visual_stage = 3;
		openttd_rust_road_set(p.state, 3, 1); // Reentry changes overtaking before UpdateSpeed.
		openttd_rust_road_set_dest(u, p.state, 100, p.dest, p.leaves);
		p.post_effect_max = 40;
		++p.nested;
		return p.leaves->speed_read(u, realistic);
	};
	t.leaves.speed_read = [](OpenTTDVehicle *, bool realistic) noexcept -> OpenTTDRoadSpeedRead {
		auto &p = *road_abi_probe;
		CHECK(p.visual_stage == (p.visual_eligible ? 3U : 1U));
		p.visual_stage = 4;
		auto read = RoadAbiMaxSpeed(); read.max_track_speed = p.post_effect_max;
		return {.max = read, .x = 0, .y = 0, .z = 0, .acceleration = realistic ? 256 : 0, .length = 8, .tram = false};
	};
	t.ground.do_update_speed = [](OpenTTDVehicle *, uint32_t acceleration, int32_t minimum, int32_t maximum) noexcept -> OpenTTDGroundSpeed {
		CHECK(road_abi_probe->visual_stage == 4 && acceleration == 512 && minimum == (road_abi_probe->entry.acceleration_model == 1 ? 4 : 0) && maximum == 40);
		return {.distance = 0, .cur_speed = 0};
	};
	t.ground.set_last_speed = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundLastSpeed { road_abi_probe->progress = 0; return {.status = 0}; };
	t.ground.update_viewport_then_next = [](OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept -> OpenTTDGroundViewportNext {
		CHECK(part == RoadAbiHandle() && front == part);
		++road_abi_probe->viewports;
		road_abi_probe->progress = 7;
		++road_abi_probe->next_reads;
		return {.next = nullptr, .final_progress = static_cast<uint8_t>(road_abi_probe->progress)};
	};
	t.leaves.crash_part = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadCrashPart { return {.next = nullptr, .next_state = nullptr, .tile = 100, .z = 0, .crossing = false}; };
	OpenTTDRoadTickRead front{.tick = 255, .front = true, .movement = probe.entry, .order_time = &probe.order_time, .running = &probe.running, .status = &probe.status};
	// Fresh scalar loads and the post-write crashed-bit reset precede controller entry.
	probe.order_time = INT32_MAX;
	probe.running = UINT8_MAX;
	probe.status = 128;
	probe.reset_status = true;
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	probe.reset_status = false;
	CHECK(probe.tick == 0 && probe.status == 0);
	CHECK(probe.nested == 3 && probe.dest == 100 && probe.order_time == INT32_MIN && probe.running == 0 && probe.progress == 7 && probe.next_reads == 1 && world.draws == 0);

	/* Stopped parts preserve running ticks; later status resets must survive the entry. */
	const auto breakdown_probe = _vehicle_maintenance_probe.breakdown_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept {
		return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = road_abi_probe->status};
	};
	const auto last_speed = t.ground.set_last_speed;
	t.ground.set_last_speed = [](OpenTTDVehicle *) noexcept {
		road_abi_probe->status = 0;
		return OpenTTDGroundLastSpeed{};
	};
	probe.status = 2;
	probe.running = UINT8_MAX;
	probe.order_time = INT32_MAX;
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	CHECK(probe.running == UINT8_MAX && probe.order_time == INT32_MIN && probe.status == 0);
	_vehicle_maintenance_probe.breakdown_probe = breakdown_probe;
	t.ground.set_last_speed = last_speed;

	/* Skip effects with no lookup; realistic UpdateSpeed still observes post-effect inputs. */
	probe.visual_eligible = false; probe.entry.acceleration_model = 1;
	front.movement = probe.entry;
	/* Hidden vehicles skip viewport callbacks, then use the direct canonical link. */
	t.ground.set_last_speed = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundLastSpeed { return {.status = 1}; };
	t.vehicle.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { CHECK(road_abi_probe->viewports == 1); ++road_abi_probe->next_reads; return nullptr; };
	t.vehicle.progress = [](OpenTTDVehicle *) noexcept -> uint8_t { CHECK(road_abi_probe->viewports == 1 && road_abi_probe->next_reads == 2); return static_cast<uint8_t>(road_abi_probe->progress); };
	CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
	CHECK(probe.viewports == 1 && probe.next_reads == 2 && probe.progress == 7);
	probe.visual_eligible = true; probe.entry.acceleration_model = 0;
	front.movement = probe.entry;

	/* Viewport reentry can add a later visible part. Its progress mutation is
	 * observed only after that viewport and any hidden final part's Next read. */
	t.ground.set_last_speed = [](OpenTTDVehicle *) noexcept -> OpenTTDGroundLastSpeed { road_abi_probe->progress = 0; return {.status = 0}; };
	t.ground.update_viewport_then_next = [](OpenTTDVehicle *part, OpenTTDVehicle *front) noexcept -> OpenTTDGroundViewportNext {
		auto &p = *road_abi_probe;
		CHECK(front == RoadAbiHandle());
		CHECK((p.viewports == 0 && part == front) || (p.viewports == 1 && part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_second)));
		++p.viewports;
		p.progress = part == front ? 7 : 0;
		openttd_rust_road_set_dest(front, p.state, 200 + p.viewports, p.dest, p.leaves);
		++p.next_reads;
		OpenTTDVehicle *next = part == front ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) : p.hidden_viewport_tail ? reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden) : nullptr;
		return {.next = next, .final_progress = next == nullptr ? uint8_t(p.progress) : uint8_t(99)};
	};
	t.vehicle.status = [](OpenTTDVehicle *part) noexcept -> uint8_t {
		bool hidden = part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden);
		CHECK(hidden ? road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2 : part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_second) && road_abi_probe->viewports == 1);
		return hidden ? 1 : 0;
	};
	t.vehicle.next = [](OpenTTDVehicle *part) noexcept -> OpenTTDVehicle * {
		CHECK(part == reinterpret_cast<OpenTTDVehicle *>(&road_abi_hidden) && road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2);
		++road_abi_probe->next_reads;
		return nullptr;
	};
	t.vehicle.progress = [](OpenTTDVehicle *front) noexcept -> uint8_t {
		CHECK(front == RoadAbiHandle() && road_abi_probe->hidden_viewport_tail && road_abi_probe->viewports == 2 && road_abi_probe->next_reads == 3);
		return road_abi_probe->progress;
	};
	t.vehicle.write_progress = [](OpenTTDVehicle *front, uint8_t progress) noexcept {
		auto &p = *road_abi_probe;
		CHECK(front == RoadAbiHandle() && p.viewports == 2 && p.next_reads == (p.hidden_viewport_tail ? 3U : 2U) && progress == 0);
		++p.progress_writes;
		p.progress = progress;
	};
	for (bool hidden : {false, true}) {
		probe.hidden_viewport_tail = hidden;
		probe.viewports = probe.next_reads = probe.progress_writes = 0;
		CHECK(openttd_rust_road_tick(v, probe.state, &front, &t.leaves, nullptr));
		CHECK(probe.viewports == 2 && probe.next_reads == (hidden ? 3U : 2U) && probe.progress_writes == 1 && probe.progress == 0 && probe.dest == 202);
	}

	/* Crash animation reads the next link after the turn/viewport callback. */
	openttd_rust_road_set(probe.state, 5, 0);
	probe.status = 128;
	t.vehicle.direction = [](OpenTTDVehicle *u) noexcept -> uint8_t { CHECK(u == RoadAbiHandle() && road_abi_probe->turns == 0); return 1; };
	t.ground.turn = [](OpenTTDVehicle *u, uint8_t direction) noexcept { CHECK(u == RoadAbiHandle() && direction < 8); ++road_abi_probe->turns; };
	t.vehicle.next = [](OpenTTDVehicle *u) noexcept -> OpenTTDVehicle * { CHECK(u == RoadAbiHandle() && road_abi_probe->turns == 1); return nullptr; };
	const OpenTTDRoadTickRead animation{.tick = 7, .front = true, .movement = probe.entry, .order_time = &probe.order_time, .running = &probe.running, .status = &probe.status};
	CHECK(openttd_rust_road_tick(v, probe.state, &animation, &t.leaves, nullptr));
	CHECK(probe.turns == 1 && world.draws == 1);

	/* The final-part deletion returns its copied alive result only. */
	openttd_rust_road_set(probe.state, 0, 0);
	openttd_rust_road_set(probe.state, 5, 2219);
	t.leaves.link = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadLink { return {.next = nullptr, .next_state = nullptr}; };
	t.vehicle.detach_last = [](OpenTTDVehicle *previous, OpenTTDVehicle *last, OpenTTDVehicle *first) noexcept {
		CHECK(previous == RoadAbiHandle() && last == previous && first == previous);
	};
	t.vehicle.destroy = [](OpenTTDVehicle *u) noexcept {
		CHECK(u == RoadAbiHandle());
		openttd_rust_road_destroy(road_abi_probe->state);
		road_abi_probe->state = nullptr;
		++road_abi_probe->destroyed;
	};
	probe.status = 128;
	const OpenTTDRoadTickRead crashed{.tick = 31, .front = true, .movement = probe.entry, .order_time = &probe.order_time, .running = &probe.running, .status = &probe.status};
	CHECK(!openttd_rust_road_tick(v, probe.state, &crashed, &t.leaves, nullptr));
	CHECK(probe.state == nullptr && probe.destroyed == 1 && world.draws == 1);
	std::printf("road_native_typed_reentry_path_and_owner_destruction passed\n");
}

/* Moving and partly entered consists cannot use IsChainInDepot's service path,
 * but FindClosestRoadDepot still selects their current depot at distance zero. */
struct RoadServiceProbe {
	OpenTTDRoadState *head = openttd_rust_road_new();
	OpenTTDRoadState *tail = openttd_rust_road_new();
	uint32_t tile = 3091, dest = 0;
	uint16_t speed = 5;
	bool depot_tile = true;
	uint32_t depot_orders = 0, services = 0, searches = 0;
};
static RoadServiceProbe *road_service_probe;
static int road_service_vehicles[2]; ///< Their addresses are the opaque head and tail handles.
static OpenTTDVehicle *RoadServiceHandle(int part) { return reinterpret_cast<OpenTTDVehicle *>(&road_service_vehicles[part]); }
static void RoadServiceBoundary()
{
	RoadServiceProbe probe;
	road_service_probe = &probe;
	EffectTestWorld world;
	effect_test_world = &world;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&world, EffectTestRandom, EffectTestIndustry);
	RoadTestTables t(&services);
	t.leaves.max_depot_penalty = []() noexcept -> uint32_t { return 300; };
	/* Direct maintenance: aged out, no reliability decay, no breakdown, service due. */
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.max_economy_date = OpenTTDEconomyDate{0};
	maintenance.economy_age = [](OpenTTDVehicle *) noexcept { return OpenTTDEconomyDate{0}; };
	t.vehicle.day = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.vehicle.write_day = [](OpenTTDVehicle *, uint8_t day) noexcept { CHECK(day == 1); };
	maintenance.check_breakdown = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceCheckBreakdown r{};
		r.no_service = true;
		return r;
	};
	maintenance.breakdown_guard = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceBreakdownGuard r{};
		r.counter = 1;
		return r;
	};
	maintenance.automatic = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceAutomatic{}; };
	t.vehicle.status = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	maintenance.interval = [](OpenTTDVehicle *) noexcept {
		OpenTTDVehicleMaintenanceInterval r{};
		r.now = OpenTTDEconomyDate{1};
		return r;
	};
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept {};
	maintenance.reset = [](OpenTTDVehicle *) noexcept { ++road_service_probe->services; return OpenTTDVehicleMaintenanceReset{}; };
	t.vehicle.write_economy_service = t.vehicle.write_calendar_service = [](OpenTTDVehicle *, int32_t) noexcept {};
	t.vehicle.write_breakdowns = [](OpenTTDVehicle *, uint8_t) noexcept {};
	t.vehicle.write_reliability = [](OpenTTDVehicle *, uint16_t) noexcept {};
	maintenance.chance_write = [](OpenTTDVehicle *, uint8_t) noexcept {};
	t.vehicle.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	t.leaves.service_read = [](OpenTTDVehicle *) noexcept -> OpenTTDRoadServiceRead {
		auto &p = *road_service_probe;
		return {.tile = p.tile, .servint = 1, .cur_speed = p.speed};
	};
	t.leaves.is_road_depot_tile = [](uint32_t tile) noexcept { CHECK(tile == road_service_probe->tile); return road_service_probe->depot_tile; };
	t.leaves.link_tile = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadLinkTile {
		auto &p = *road_service_probe;
		bool head = u == RoadServiceHandle(0);
		return {.link = {.next = head ? RoadServiceHandle(1) : nullptr, .next_state = head ? p.tail : nullptr}, .tile = p.tile};
	};
	t.leaves.link = [](OpenTTDVehicle *u) noexcept -> OpenTTDRoadLink {
		bool head = u == RoadServiceHandle(0);
		return {.next = head ? RoadServiceHandle(1) : nullptr, .next_state = head ? road_service_probe->tail : nullptr};
	};
	t.leaves.find_depot = [](OpenTTDVehicle *, int32_t penalty) noexcept -> OpenTTDRoadDepotResult {
		CHECK(penalty == 300);
		++road_service_probe->searches;
		return {.tile = UINT32_MAX, .length = UINT32_MAX};
	};
	t.shared.depot_index = [](uint32_t tile) noexcept -> uint16_t { CHECK(tile == road_service_probe->tile); return 55; };
	t.vehicle.order_type = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	t.ground.go_to_depot_service = [](OpenTTDVehicle *, uint16_t depot) noexcept { CHECK(depot == 55); ++road_service_probe->depot_orders; };
	t.vehicle.dest_tile = [](OpenTTDVehicle *) noexcept -> uint32_t { return road_service_probe->dest; };
	t.vehicle.write_dest = [](OpenTTDVehicle *, uint32_t tile) noexcept { CHECK(tile == road_service_probe->tile); road_service_probe->dest = tile; };
	t.vehicle.start_stop_dirty = [](OpenTTDVehicle *) noexcept {};
	t.vehicle.check_orders = [](OpenTTDVehicle *) noexcept -> uint8_t { return 0; };
	auto day = [&]() { openttd_rust_road_economy_day(RoadServiceHandle(0), probe.head, true, &t.leaves); };
	for (bool moving : {true, false}) {
		probe.speed = moving ? 5 : 0;
		probe.dest = 42;
		openttd_rust_road_set(probe.head, 0, 254);
		openttd_rust_road_set(probe.tail, 0, moving ? 254 : 1);
		openttd_rust_road_path_push(probe.head, {10, 10815});
		day();
		CHECK(probe.dest == probe.tile && openttd_rust_road_path_size(probe.head) == 0);
	}
	CHECK(probe.depot_orders == 2 && probe.services == 0 && world.draws == 0);
	openttd_rust_road_set(probe.tail, 0, 254);
	day();
	CHECK(probe.services == 1 && probe.depot_orders == 2);
	probe.depot_tile = false;
	day();
	CHECK(probe.searches == 1);
	openttd_rust_road_destroy(probe.head);
	openttd_rust_road_destroy(probe.tail);
	std::printf("road_service_moving_partial_parked_and_search_branches passed\n");
}

struct DisasterProbe { uint32_t reads = 0, writes = 0, draws = 0, random = 0; };
static void OPENTTD_DISASTER_CALL DisasterProbeRead(void *context, uint32_t kind, uint32_t, int64_t, int64_t, int64_t *out) noexcept
{
	auto *probe = static_cast<DisasterProbe *>(context); probe->reads++;
	if (kind == 0) { out[0] = 64; out[1] = 128; out[2] = 63; out[3] = 127; out[4] = 8191; out[5] = 1935; out[6] = 0; out[9] = 1; }
	if (kind == 1) { out[0] = 13; out[7] = 8880; }
}
static void OPENTTD_DISASTER_CALL DisasterProbeWrite(void *context, uint32_t, uint32_t, int64_t) noexcept
{
	static_cast<DisasterProbe *>(context)->writes++;
}
static uint32_t DisasterProbeRandom(void *context) noexcept { auto *probe = static_cast<DisasterProbe *>(context); probe->draws++; return probe->random; }
static int64_t OPENTTD_DISASTER_CALL DisasterProbeService(void *, const OpenTTDDisasterAction *) noexcept { return 0; }
static uint32_t DisasterProbeIndustry(int32_t, int32_t, uint32_t *) noexcept { return 0; }
static void Disasters()
{
	using Owner = std::unique_ptr<OpenTTDDisasterRun, decltype(&openttd_rust_disaster_destroy)>;
	using StateOwner = std::unique_ptr<OpenTTDDisasterState, decltype(&openttd_rust_disaster_state_destroy)>;
	DisasterProbe probe;
	const OpenTTDSharedServices services = OpenTTDFixtureSharedServices(&probe, DisasterProbeRandom, DisasterProbeIndustry);
	auto make = [&](uint32_t operation, uint32_t id = 0) { return Owner(openttd_rust_disaster_create(operation, id, 0, 0, 0, 0, &probe, DisasterProbeRead, DisasterProbeWrite, &services, DisasterProbeService), openttd_rust_disaster_destroy); };
	StateOwner state(openttd_rust_disaster_state_create(1048575), openttd_rust_disaster_state_destroy);
	auto *address = state.get(); state->state = 65535; state->flags = 0xAB; state->image_override = UINT32_MAX;
	uint16_t *delay = openttd_rust_disaster_delay(); *delay = 0;
	{ auto daily = make(1); CHECK(openttd_rust_disaster_advance(daily.get(), 0).kind == 0 && *delay == 65535 && probe.draws == 0); }
	*delay = 1; probe.random = 511;
	{ auto daily = make(1); CHECK(openttd_rust_disaster_advance(daily.get(), 0).kind == 0 && *delay == 1241 && probe.draws == 1); }
	probe.random = 0;
	{ auto startup = make(2); CHECK(openttd_rust_disaster_advance(startup.get(), 0).kind == 0 && *delay == 730 && probe.draws == 2); }
	CHECK(delay == openttd_rust_disaster_delay() && state.get() == address && state->state == 65535 && state->flags == 0xAB && state->image_override == UINT32_MAX);
	auto expired = make(0, 17);
	CHECK(openttd_rust_disaster_advance(expired.get(), 0).kind == 2);
	uint32_t reads = probe.reads, writes = probe.writes;
	expired.reset(); CHECK(probe.reads == reads && probe.writes == writes && probe.draws == 2);
	std::printf("disaster_private_counter_direct_rng_delete_cancel passed\n");
}

/* Saves cannot observe NOSAVE rotation coordinates. Compare their full bit domain
 * with native C++ narrowing, and exercise real nested owner mutation in a callback. */
struct ShipProbe {
	uint32_t x = 32768, y = static_cast<uint32_t>(-32769), dest = 0;
	uint32_t order_time = UINT32_MAX;
	uint8_t tick = 255, running = 255, direction = 1;
	uint16_t speed = 33;
	uint32_t stage = 0, paths = 0, positions = 0;
	OpenTTDShipState *owner = nullptr;
	const OpenTTDShipLeaves *leaves = nullptr;
	OpenTTDSharedServices services{};
};
static ShipProbe ship_probe;
template <typename Result, typename... Args>
static Result ShipProbeUnused(Args...) noexcept
{
	if constexpr (!std::is_void_v<Result>) return {};
}
static void ShipBoundary()
{
	using Owner = std::unique_ptr<OpenTTDShipState, decltype(&openttd_rust_ship_state_destroy)>;
	Owner state(openttd_rust_ship_state_new(), openttd_rust_ship_state_destroy);
	CHECK(openttd_rust_ship_get_state(state.get()) == 0 && openttd_rust_ship_get_rotation(state.get()) == 255);
	for (uint32_t value = 0; value <= UINT16_MAX; ++value) {
		openttd_rust_ship_set_state(state.get(), static_cast<uint8_t>(value));
		openttd_rust_ship_set_rotation(state.get(), static_cast<uint8_t>(value));
		openttd_rust_ship_set_rotation_x(state.get(), static_cast<int16_t>(value));
		openttd_rust_ship_set_rotation_y(state.get(), static_cast<int16_t>(value));
		CHECK(openttd_rust_ship_get_state(state.get()) == static_cast<uint8_t>(value));
		CHECK(openttd_rust_ship_get_rotation(state.get()) == static_cast<uint8_t>(value));
		CHECK(openttd_rust_ship_get_rotation_x(state.get()) == static_cast<int16_t>(value));
		CHECK(openttd_rust_ship_get_rotation_y(state.get()) == static_cast<int16_t>(value));
	}
	static OpenTTDVehicleMaintenanceServices ship_maintenance;
	static OpenTTDVehicleServices ship_vehicle;
	ship_vehicle.maintenance = &ship_maintenance;
	static int ship_shell; ///< Its address is the opaque vehicle handle.
	auto *shell = reinterpret_cast<OpenTTDVehicle *>(&ship_shell);
	/* Initialize every function pointer, including paths unused in this concrete gap. */
	OpenTTDShipLeaves leaves{
		.depot_dir = ShipProbeUnused<uint32_t, uint32_t>, .depot_axis = ShipProbeUnused<uint32_t, uint32_t>,
		.is_depot = ShipProbeUnused<bool, uint32_t>, .depot_index = ShipProbeUnused<uint32_t, uint32_t>,
		.wait_unbunch = ShipProbeUnused<bool, uint32_t>, .chain_depot = ShipProbeUnused<bool, uint32_t>,
		.servint = ShipProbeUnused<uint32_t, uint32_t>,
		.max_distance = ShipProbeUnused<uint32_t>, .tile_valid = ShipProbeUnused<bool, uint32_t>,
		.tile_type = ShipProbeUnused<uint32_t, uint32_t>, .water_class = ShipProbeUnused<uint32_t, uint32_t>,
		.lock_middle = ShipProbeUnused<bool, uint32_t>, .lock_dir = ShipProbeUnused<uint32_t, uint32_t>,
		.tile_min_z = ShipProbeUnused<uint32_t, uint32_t>, .tile_max_z = ShipProbeUnused<uint32_t, uint32_t>,
		.track_status = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .offset = ShipProbeUnused<uint32_t, uint32_t>,
		.diag_between = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .dist_square = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.dist_manhattan = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .docking = ShipProbeUnused<bool, uint32_t>,
		.dock = ShipProbeUnused<bool, uint32_t>, .dock_water = ShipProbeUnused<bool, uint32_t>,
		.station = ShipProbeUnused<uint32_t, uint32_t>, .industry_station = ShipProbeUnused<uint32_t, uint32_t>,
		.oilrig = ShipProbeUnused<bool, uint32_t>, .station_use = ShipProbeUnused<bool, uint32_t, uint32_t>,
		.station_xy = ShipProbeUnused<uint32_t, uint32_t>, .station_contains = ShipProbeUnused<bool, uint32_t, uint32_t>,
		.station_dock = ShipProbeUnused<bool, uint32_t>, .station_visits = ShipProbeUnused<uint32_t, uint32_t>,
		.visit_set = ShipProbeUnused<void, uint32_t, uint32_t>, .arrival = ShipProbeUnused<void, uint32_t, uint32_t>,
		.leave_unbunch = ShipProbeUnused<void, uint32_t>,
		.path_result = ShipProbeUnused<void, uint32_t, bool>, .order_free = ShipProbeUnused<void, uint32_t>,
		.order_dummy = ShipProbeUnused<void, uint32_t>, .order_depot = ShipProbeUnused<void, uint32_t, uint32_t>,
		.order_leave = ShipProbeUnused<void, uint32_t>, .order_increment = ShipProbeUnused<void, uint32_t>,
		.timetable = ShipProbeUnused<void, uint32_t>, .position = ShipProbeUnused<void, uint32_t>,
		.start_dirty = ShipProbeUnused<void, uint32_t>, .depot_dirty = ShipProbeUnused<void, uint32_t>,
		.depot_invalidate = ShipProbeUnused<void, uint32_t>, .ships_dirty = ShipProbeUnused<void>,
		.check_orders = ShipProbeUnused<void, uint32_t>,
		.running_cost = ShipProbeUnused<int64_t, uint32_t>, .cost_divisor = ShipProbeUnused<uint32_t>,
		.pay_running = ShipProbeUnused<void, uint32_t, int64_t>, .speed_default = ShipProbeUnused<uint32_t, uint32_t>,
		.age_default = ShipProbeUnused<uint32_t, uint32_t>, .speed_frac = ShipProbeUnused<uint32_t, uint32_t, bool>,
		.speed_property = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .age_property = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.update_visual = ShipProbeUnused<void, uint32_t>, .cache_invalidate = ShipProbeUnused<void, uint32_t>,
		.capacity = ShipProbeUnused<uint32_t, uint32_t>, .sprite_direction = ShipProbeUnused<void, uint32_t>,
		.tile_x = ShipProbeUnused<uint32_t, uint32_t>, .tile_y = ShipProbeUnused<uint32_t, uint32_t>,
		.build_flag = ShipProbeUnused<bool, uint32_t>, .build_random = ShipProbeUnused<void, uint32_t, uint16_t>,
		.new_position = ShipProbeUnused<OpenTTDShipPosition, uint32_t>, .exit_dir = ShipProbeUnused<uint32_t, uint32_t, uint32_t>,
		.track_direction = ShipProbeUnused<uint32_t, uint32_t, uint32_t>, .tracks_reach = ShipProbeUnused<uint32_t, uint32_t>,
		.busy_tile = ShipProbeUnused<bool, uint32_t>, .path_size = ShipProbeUnused<size_t, uint32_t>,
		.path_back = ShipProbeUnused<uint32_t, uint32_t>, .path_pop = ShipProbeUnused<void, uint32_t>,
		.path_clear = ShipProbeUnused<void, uint32_t>, .enter_tile = ShipProbeUnused<uint32_t, uint32_t, uint32_t, uint32_t, uint32_t>,
		.enter_depot = ShipProbeUnused<void, uint32_t>, .process_orders = ShipProbeUnused<bool, uint32_t>,
		.loading = ShipProbeUnused<void, uint32_t>, .begin_loading = ShipProbeUnused<void, uint32_t>,
		.viewport = ShipProbeUnused<void, uint32_t, bool, bool>,
		.base_viewport = ShipProbeUnused<void, uint32_t>, .visual = ShipProbeUnused<void, uint32_t>,
		.cache = ShipProbeUnused<void, uint32_t>, .play_sound = ShipProbeUnused<void, uint32_t>,
		.yapf_reverse = ShipProbeUnused<OpenTTDShipReverseResult, uint32_t, bool>, .yapf_choose = ShipProbeUnused<OpenTTDShipTrackResult, uint32_t, uint32_t>,
		.update_delta = ShipProbeUnused<void, uint32_t>, .build_owner = ShipProbeUnused<void, uint32_t>,
		.build_z = ShipProbeUnused<void, uint32_t>, .build_properties = ShipProbeUnused<void, uint32_t, uint32_t>,
		.build_dates = ShipProbeUnused<void, uint32_t>, .build_acceleration = ShipProbeUnused<void, uint32_t, uint32_t>,
		.build_prototype = ShipProbeUnused<void, uint32_t>, .build_interval_percent = ShipProbeUnused<void, uint32_t>,
		.build_capacity = ShipProbeUnused<void, uint32_t, uint32_t>, .set_tile = ShipProbeUnused<void, uint32_t, uint32_t>,
		.set_x = ShipProbeUnused<void, uint32_t, int32_t>, .set_y = ShipProbeUnused<void, uint32_t, int32_t>,
		.set_z = ShipProbeUnused<void, uint32_t, int32_t>, .set_direction = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_speed = ShipProbeUnused<void, uint32_t, uint16_t>, .set_tick = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_running = ShipProbeUnused<void, uint32_t, uint8_t>, .set_day = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_order_time = ShipProbeUnused<void, uint32_t, int32_t>, .set_progress = ShipProbeUnused<void, uint32_t, uint8_t>,
		.set_last_station = ShipProbeUnused<void, uint32_t, uint16_t>, .set_hidden = ShipProbeUnused<void, uint32_t, bool>,
		.set_max_speed = ShipProbeUnused<void, uint32_t, uint16_t>, .set_cargo_age = ShipProbeUnused<void, uint32_t, uint16_t>,
		.set_dest = ShipProbeUnused<void, uint32_t, uint32_t>, .tile = ShipProbeUnused<uint32_t, uint32_t>,
		.dest = ShipProbeUnused<uint32_t, uint32_t>, .x = ShipProbeUnused<uint32_t, uint32_t>,
		.y = ShipProbeUnused<uint32_t, uint32_t>, .z = ShipProbeUnused<uint32_t, uint32_t>,
		.direction = ShipProbeUnused<uint32_t, uint32_t>, .speed = ShipProbeUnused<uint32_t, uint32_t>,
		.tick = ShipProbeUnused<uint32_t, uint32_t>, .running = ShipProbeUnused<uint32_t, uint32_t>,
		.day = ShipProbeUnused<uint32_t, uint32_t>, .order_time = ShipProbeUnused<uint32_t, uint32_t>,
		.progress = ShipProbeUnused<uint32_t, uint32_t>, .status = ShipProbeUnused<uint32_t, uint32_t>,
		.owner = ShipProbeUnused<uint32_t, uint32_t>, .last_station = ShipProbeUnused<uint32_t, uint32_t>,
		.order_destination = ShipProbeUnused<uint32_t, uint32_t>, .order_type = ShipProbeUnused<uint32_t, uint32_t>,
		.order_max_speed = ShipProbeUnused<uint32_t, uint32_t>, .acceleration = ShipProbeUnused<uint32_t, uint32_t>,
		.max_speed = ShipProbeUnused<uint32_t, uint32_t>, .state_owner = ShipProbeUnused<OpenTTDShipState *, uint32_t>,
		.patch = ShipProbeUnused<OpenTTDWaterPatch, uint32_t>, .neighbours = ShipProbeUnused<size_t, OpenTTDWaterPatch, OpenTTDWaterPatch *>,
		.next_depot = ShipProbeUnused<bool, uint32_t, OpenTTDShipDepot *>,
		.vehicle = &ship_vehicle
	};
	ship_probe = {};
	ship_probe.owner = state.get();
	ship_probe.leaves = &leaves;
	ship_probe.services = OpenTTDFixtureSharedServices(nullptr, ShipProbeUnused<uint32_t, void *>, ShipProbeUnused<uint32_t, int32_t, int32_t, uint32_t *>);
	leaves.state_owner = [](uint32_t) noexcept { return ship_probe.owner; };
	leaves.x = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.x); };
	leaves.y = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.y); };
	leaves.dest = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.dest); };
	leaves.order_time = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.order_time); };
	leaves.tick = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.tick); };
	leaves.running = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.running); };
	leaves.direction = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.direction); };
	leaves.speed = [](uint32_t) noexcept { return static_cast<uint32_t>(ship_probe.speed); };
	leaves.set_tick = [](uint32_t, uint8_t value) noexcept { ship_probe.tick = value; };
	leaves.set_running = [](uint32_t, uint8_t value) noexcept { ship_probe.running = value; };
	leaves.set_order_time = [](uint32_t, int32_t value) noexcept { ship_probe.order_time = static_cast<uint32_t>(value); };
	leaves.set_direction = [](uint32_t, uint8_t value) noexcept { ship_probe.direction = value; };
	leaves.set_speed = [](uint32_t, uint16_t value) noexcept { ship_probe.speed = value; };
	leaves.set_dest = [](uint32_t, uint32_t value) noexcept { ship_probe.dest = value; };
	leaves.path_clear = [](uint32_t) noexcept { ++ship_probe.paths; };
	leaves.position = [](uint32_t) noexcept { ++ship_probe.positions; };
	ship_maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept {
		CHECK(ship_probe.stage++ == 0);
		CHECK(ship_probe.tick == 0 && ship_probe.running == 0 && ship_probe.order_time == 0);
		return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0};
	};
	leaves.process_orders = [](uint32_t) noexcept { CHECK(ship_probe.stage++ == 1); return true; };
	leaves.yapf_reverse = [](uint32_t, bool trackdir) noexcept {
		CHECK(ship_probe.stage++ == 2 && !trackdir);
		return OpenTTDShipReverseResult{true, 255};
	};
	leaves.viewport = [](uint32_t id, bool update_delta, bool force) noexcept {
		CHECK(ship_probe.stage++ == 3 && update_delta && force);
		CHECK(ship_probe.direction == 5 && ship_probe.speed == 0 && ship_probe.paths == 1 && ship_probe.positions == 1);
		CHECK(openttd_rust_ship_get_rotation_x(ship_probe.owner) == static_cast<int16_t>(ship_probe.x));
		CHECK(openttd_rust_ship_get_rotation_y(ship_probe.owner) == static_cast<int16_t>(ship_probe.y));
		/* The outer call holds no Rust reference to this owner during nested mutation. */
		openttd_rust_ship_destination(id, 17, ship_probe.leaves, &ship_probe.services);
		CHECK(ship_probe.dest == 17 && ship_probe.paths == 2);
		openttd_rust_ship_set_rotation(ship_probe.owner, 3);
	};
	CHECK(openttd_rust_ship_tick(4, shell, &leaves, &ship_probe.services));
	CHECK(ship_probe.stage == 4 && openttd_rust_ship_get_rotation(state.get()) == 3);
	CHECK(!openttd_rust_ship_find_depot(4, 80, &leaves, &ship_probe.services).valid);
	leaves.owner = [](uint32_t) noexcept { return uint32_t{3}; };
	leaves.next_depot = [](uint32_t first, OpenTTDShipDepot *out) noexcept {
		if (first != 0) return false;
		*out = {0x1234, 0x80000017, 3, 1};
		return true;
	};
	leaves.dist_square = [](uint32_t, uint32_t) noexcept { return uint32_t{49}; };
	auto depot = openttd_rust_ship_find_depot(4, 80, &leaves, &ship_probe.services);
	CHECK(depot.valid && depot.tile == 0x80000017 && depot.id == 0x1234);
	state.reset(openttd_rust_ship_state_new());
	CHECK(openttd_rust_ship_get_rotation_x(state.get()) == 0 && openttd_rust_ship_get_rotation(state.get()) == 255);
	std::printf("ship scalar widths, native transient narrowing, direct order and nested owner mutation passed\n");
}
/* Direct callbacks reenter with the same live scalar allocation. Deletion models
 * Aircraft::PreDestructor releasing airport blocks before freeing that owner. */
static struct AircraftBoundaryProbe {
	OpenTTDVehicleServices vehicle_services;
	OpenTTDSharedServices shared_services;
	OpenTTDAircraftLeaves leaves;
	OpenTTDAircraftVehicle vehicle;
	uint64_t blocks = UINT64_MAX;
	unsigned orders = 0, deleted = 0, ticks = 0, rotor_reads = 0, moves = 0;
	int32_t order_time = 0, x = 0, y = 0, z = 1;
	uint8_t order_kind = 0, progress = 0, subtype = 2;
	Direction direction = DIR_N;
	uint16_t maximum_speed = 0, cargo_age = 0;
	bool live = true;
} aircraft_probe;
static void AircraftDirectBoundary()
{
	auto &p = aircraft_probe;
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	CHECK(p.shared_services.is_water_tile != nullptr);
	auto &vehicle = p.vehicle_services;
	auto &shared = p.shared_services;
	auto &leaves = p.leaves;
	vehicle.status = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live); return uint8_t{0}; };
	vehicle.order_type = [](OpenTTDVehicle *) noexcept { return uint8_t{3}; };
	vehicle.maintenance = &_vehicle_maintenance_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0}; };
	vehicle.loading = [](OpenTTDVehicle *, bool) noexcept {};
	leaves.process_orders = [](OpenTTDAircraftVehicle v) noexcept {
		CHECK(aircraft_probe.live);
		aircraft_probe.orders++;
		openttd_rust_aircraft_cache(&aircraft_probe.leaves, v, true);
		v.state->pos = 19;
	};
	vehicle.property = [](OpenTTDVehicle *, uint8_t property, int32_t fallback) noexcept { CHECK(property == 0x0C && fallback == 0); return int32_t{200}; };
	leaves.cargo_age_property = [](OpenTTDAircraftVehicle) noexcept { return uint32_t{71}; };
	leaves.range_property = [](OpenTTDAircraftVehicle) noexcept { return uint32_t{41}; };
	vehicle.write_max_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { aircraft_probe.maximum_speed = speed; };
	vehicle.write_cargo_age = [](OpenTTDVehicle *, uint16_t age) noexcept { aircraft_probe.cargo_age = age; };
	leaves.next = [](OpenTTDAircraftVehicle v) noexcept { return v; };
	p.vehicle = {reinterpret_cast<OpenTTDVehicle *>(&p), openttd_rust_aircraft_state_new(), 17};
	CHECK(openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.orders == 1 && p.vehicle.state->pos == 19 && p.vehicle.state->cached_max_range == 41 && p.vehicle.state->cached_max_range_sqr == 1681);
	CHECK(p.maximum_speed == 2560 && p.cargo_age == 71);
	vehicle.status = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live); return uint8_t{128}; };
	leaves.station = [](uint16_t) noexcept -> const OpenTTDStation * { CHECK(aircraft_probe.live); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.airport_tile = [](const OpenTTDStation *) noexcept { return uint32_t{7}; };
	leaves.airport_fta = [](const OpenTTDStation *s) noexcept -> const void * { return s; };
	leaves.airport_blocks = [](const OpenTTDStation *) noexcept { CHECK(aircraft_probe.live); return &aircraft_probe.blocks; };
	leaves.node = [](const void *ap, uint8_t) noexcept -> const void * { return ap; };
	leaves.fta = [](const void *) noexcept { return OpenTTDAircraftNode{nullptr, uint64_t{1} << 9, 0, 0, 0}; };
	leaves.layout_blocks = [](const void *, uint8_t) noexcept { return uint64_t{1} << 9; };
	vehicle.destroy = [](OpenTTDVehicle * handle) noexcept {
		CHECK(handle == aircraft_probe.vehicle.handle);
		auto v = aircraft_probe.vehicle;
		CHECK(aircraft_probe.live && v.state->crashed_counter == 10002);
		openttd_rust_aircraft_release_blocks(&aircraft_probe.leaves, v);
		CHECK((aircraft_probe.blocks & ((uint64_t{1} << 8) | (uint64_t{1} << 9) | (uint64_t{1} << 29))) == 0);
		openttd_rust_aircraft_state_destroy(v.state);
		aircraft_probe.live = false;
		aircraft_probe.deleted++;
	};
	p.vehicle.state->targetairport = 7;
	p.vehicle.state->crashed_counter = 9999;
	CHECK(!openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(!p.live && p.deleted == 1);
	/* A disaster has its own flags and no Aircraft state, even with subtype zero. */
	shared.map_log_x = []() noexcept { return uint32_t{6}; };
	shared.map_max_x = shared.map_max_y = []() noexcept { return uint32_t{63}; };
	vehicle.x = vehicle.y = vehicle.z = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.live == false); return int32_t{0}; };
	shared.tile_height = [](uint32_t) noexcept { return uint32_t{0}; };
	vehicle.xy = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleXY{0, 0}; };
	vehicle.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	vehicle.vehicle_type = [](OpenTTDVehicle *) noexcept { return uint8_t{4}; };
	vehicle.direction = [](OpenTTDVehicle *) noexcept { return uint8_t{4}; };
	vehicle.altitude_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleAltitudeInputs{1, 4}; };
	uint8_t disaster_flags = 0;
	int32_t min = 0, max = 0;
	OpenTTDAircraftVehicle disaster{reinterpret_cast<OpenTTDVehicle *>(&p), nullptr, 81};
	openttd_rust_aircraft_flight_bounds(&leaves, disaster, &min, &max);
	CHECK(min == 30 && max == 270);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, false) == 1 && disaster_flags == 4);
	CHECK(openttd_rust_aircraft_flight_level(&leaves, disaster, &disaster_flags, true) == 2 && disaster_flags == 4);
	std::printf("aircraft direct nested mutation, live block release/deletion and disaster flight identity passed\n");
}
/* Concrete gaps: native callbacks can replace order fields/rotor counters and
 * direction/progress after the pre-speed observation. This probe forbids stale
 * snapshots at those original observation phases, using the production Rust entry. */
static void AircraftPhaseBoundary()
{
	auto &p = aircraft_probe;
	p = {};
	p.leaves = AircraftTestLeaves(p.vehicle_services, p.shared_services);
	auto &v = p.vehicle_services;
	auto &s = p.shared_services;
	auto &leaves = p.leaves;
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	p.vehicle = {reinterpret_cast<OpenTTDVehicle *>(&p), state.get(), 17};
	v.subtype = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	v.tick_status = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleTickStatus{7, 0}; };
	v.write_tick = [](OpenTTDVehicle *, uint8_t tick) noexcept { CHECK(tick == 8); aircraft_probe.ticks++; };
	v.running = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	v.write_running = [](OpenTTDVehicle *, uint8_t running) noexcept { CHECK(running == 1); };
	leaves.next = [](OpenTTDAircraftVehicle u) noexcept {
		if (u.id == 18) { aircraft_probe.order_time = 40; aircraft_probe.rotor_reads++; }
		u.id++; u.handle = reinterpret_cast<OpenTTDVehicle *>(uintptr_t{u.id}); return u;
	};
	v.status = [](OpenTTDVehicle *u) noexcept { return static_cast<uint8_t>(u == aircraft_probe.vehicle.handle ? 0 : 1); };
	v.order_time = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.rotor_reads == 1); return aircraft_probe.order_time; };
	v.write_order_time = [](OpenTTDVehicle *, int32_t time) noexcept { CHECK(time == 41); aircraft_probe.order_time = time; };
	v.maintenance = &_vehicle_maintenance_probe;
	_vehicle_maintenance_probe.breakdown_probe = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleMaintenanceBreakdownProbe{.counter = 0, .status = 0}; };
	leaves.process_orders = [](OpenTTDAircraftVehicle) noexcept { aircraft_probe.orders++; aircraft_probe.order_kind = 3; };
	v.loading = [](OpenTTDVehicle *, bool) noexcept { aircraft_probe.order_kind = 4; };
	v.order_type = [](OpenTTDVehicle *) noexcept { return aircraft_probe.order_kind; };
	CHECK(openttd_rust_aircraft_tick(&leaves, p.vehicle));
	CHECK(p.ticks == 1 && p.orders == 2 && p.rotor_reads == 1 && p.order_time == 41);

	/* A speed change dirties the native widget before direction/progress are
	 * read. Their mutation yields exactly one move; a stale snapshot yields none. */
	v.subtype = [](OpenTTDVehicle *) noexcept { return aircraft_probe.subtype; };
	v.movement_observations = [](OpenTTDVehicle *shell) noexcept {
		auto &p = aircraft_probe;
		CHECK(shell == p.vehicle.handle);
		return OpenTTDVehicleMovementObservations{.position = {.subtype = &p.subtype, .z = &p.z},
			.x = &p.x, .y = &p.y, .speed = &p.maximum_speed, .direction = &p.direction};
	};
	v.status = [](OpenTTDVehicle *) noexcept { return uint8_t{0}; };
	leaves.process_orders = [](OpenTTDAircraftVehicle) noexcept {};
	v.loading = [](OpenTTDVehicle *, bool) noexcept {};
	v.order_type = [](OpenTTDVehicle *) noexcept { return uint8_t{1}; };
	p.vehicle.state->state = 14; p.vehicle.state->targetairport = 7;
	leaves.station = [](uint16_t station) noexcept { CHECK(station == 7); return reinterpret_cast<const OpenTTDStation *>(&aircraft_probe); };
	leaves.geometry = [](const OpenTTDStation *) noexcept { return OpenTTDAircraftGeometry{0, 0, 8, 8, true}; };
	leaves.airport_fta = [](const OpenTTDStation *) noexcept -> const void * { return &aircraft_probe; };
	leaves.moving = [](const void *, uint8_t) noexcept { return OpenTTDAircraftMoving{16, 0, 16, 0}; };
	v.xy = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleXY{aircraft_probe.x, aircraft_probe.y}; };
	s.map_log_x = []() noexcept { return uint32_t{6}; };
	s.map_max_x = s.map_max_y = []() noexcept { return uint32_t{63}; };
	s.plane_speed = []() noexcept { return uint8_t{1}; };
	v.speed_inputs = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleSpeedInputs{0, 100, 8, 0, 0}; };
	v.write_subspeed = [](OpenTTDVehicle *, uint8_t sub) noexcept { CHECK(sub == 104); };
	v.write_speed = [](OpenTTDVehicle *, uint16_t speed) noexcept { CHECK(speed == 2); aircraft_probe.maximum_speed = speed; };
	v.start_stop_dirty = [](OpenTTDVehicle *) noexcept { aircraft_probe.direction = DIR_NE; aircraft_probe.progress = 255; };
	v.direction_progress = [](OpenTTDVehicle *) noexcept { CHECK(aircraft_probe.progress == 255); return OpenTTDVehicleDirectionProgress{aircraft_probe.direction, aircraft_probe.progress}; };
	v.write_progress = [](OpenTTDVehicle *, uint8_t progress) noexcept { CHECK(progress == 1); aircraft_probe.progress = progress; };
	leaves.direction_towards = [](OpenTTDAircraftVehicle, int32_t, int32_t) noexcept { return uint8_t{1}; };
	v.direction = [](OpenTTDVehicle *) noexcept { return static_cast<uint8_t>(aircraft_probe.direction); };
	v.new_position = [](OpenTTDVehicle *) noexcept { return OpenTTDVehicleNewPosition{aircraft_probe.x + 1, aircraft_probe.y, 0}; };
	v.write_tile = [](OpenTTDVehicle *, uint32_t) noexcept {};
	v.z = [](OpenTTDVehicle *) noexcept { return int32_t{1}; };
	v.write_coordinates = [](OpenTTDVehicle *, int32_t x, int32_t y, int32_t z) noexcept { CHECK(z == 1); aircraft_probe.x = x; aircraft_probe.y = y; };
	/* Both callbacks mutate canonical inputs. Subtype must be loaded after
	 * UpdatePosition, then z after the rotor callback, rather than on acquisition. */
	leaves.update_position = [](OpenTTDAircraftVehicle) noexcept { aircraft_probe.moves++; aircraft_probe.subtype = 0; aircraft_probe.z = 17; };
	leaves.rotor_image = [](OpenTTDAircraftVehicle) noexcept { CHECK(aircraft_probe.z == 17); aircraft_probe.z = 25; };
	leaves.next = [](OpenTTDAircraftVehicle u) noexcept { if (u.id != 17) return OpenTTDAircraftVehicle{nullptr, nullptr, 1048575}; u.id = 18; u.handle = reinterpret_cast<OpenTTDVehicle *>(uintptr_t{18}); return u; };
	v.write_x = [](OpenTTDVehicle *, int32_t x) noexcept { CHECK(x == 1); };
	v.write_y = [](OpenTTDVehicle *, int32_t y) noexcept { CHECK(y == -3); };
	v.y = [](OpenTTDVehicle *) noexcept { return int32_t{-3}; };
	v.write_z = [](OpenTTDVehicle *, int32_t z) noexcept { CHECK(z == 0); };
	s.slope = [](int32_t, int32_t) noexcept { return int32_t{0}; };
	leaves.copy_sprite = [](OpenTTDAircraftVehicle, OpenTTDAircraftVehicle) noexcept {};
	leaves.position_viewport = [](OpenTTDAircraftVehicle) noexcept {};
	CHECK(openttd_rust_aircraft_event(&leaves, p.vehicle, false));
	CHECK(p.moves == 1 && p.x == 1 && p.y == 0 && p.progress == 1);
	p.subtype = 2; p.z = 1;
	v.position_observations = [](OpenTTDVehicle *shell) noexcept {
		CHECK(shell == aircraft_probe.vehicle.handle);
		return OpenTTDVehiclePositionObservations{.subtype = &aircraft_probe.subtype, .z = &aircraft_probe.z};
	};
	openttd_rust_aircraft_position(&leaves, p.vehicle, 1, 0, 1);
	CHECK(p.moves == 2 && p.z == 25);
	std::printf("aircraft rotor/order/loading phases and post-dirty motion mutation passed\n");
}

static void AircraftOwnership()
{
	std::unique_ptr<OpenTTDAircraftState, decltype(&openttd_rust_aircraft_state_destroy)> state{openttd_rust_aircraft_state_new(), openttd_rust_aircraft_state_destroy};
	std::unique_ptr<uint64_t, decltype(&openttd_rust_airport_blocks_destroy)> airport{openttd_rust_airport_blocks_new(), openttd_rust_airport_blocks_destroy};
	CHECK(state->targetairport == 65535 && state->last_direction == 255 && state->cached_max_range_sqr == 0 && *airport == 0);
	uint8_t &position = state->pos; uint16_t &counter = state->crashed_counter; uint64_t &blocks = *airport;
	position = 255; counter = 65535; blocks = UINT64_MAX; state->cached_max_range = 65535; state->cached_max_range_sqr = UINT32_MAX;
	CHECK(state->pos == 255 && state->crashed_counter == 65535 && *airport == UINT64_MAX && state->cached_max_range == 65535 && state->cached_max_range_sqr == UINT32_MAX);
	std::printf("aircraft_airport_canonical_scalar_lifetime passed\n");
}

/* Explicit trivial Date and actual VehicleType carriers and aggregate return/write call ABI. Opaque
 * shells are never dereferenced; the game target checks real member stores. */
static void VehicleMaintenanceBoundary()
{
	static size_t dirty;
	static size_t resets;
	static size_t values;
	dirty = resets = values = 0;
	auto &maintenance = _vehicle_maintenance_probe;
	maintenance.breakdown_probe = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicleMaintenanceBreakdownProbe { return {.counter = 0, .status = 0}; };
	maintenance.details_dirty = [](OpenTTDVehicle *) noexcept { dirty++; };
	maintenance.reset = [](OpenTTDVehicle *) noexcept {
		return OpenTTDVehicleMaintenanceReset{.economy_date = OpenTTDEconomyDate{0x1020304}, .calendar_date = OpenTTDCalendarDate{-0x12345}, .reliability = 0x9876, .chance = 12, .breakdowns = 2};
	};
	maintenance.chance_write = [](OpenTTDVehicle *, uint8_t chance) noexcept { CHECK(chance == 3); resets++; };
	maintenance.value = [](OpenTTDVehicle *) noexcept { return int64_t{-257}; };
	maintenance.value_write = [](OpenTTDVehicle *, int64_t value) noexcept { CHECK(value == -255); values++; };
	maintenance.renewal = [](OpenTTDVehicle *, const Company *) noexcept {
		return OpenTTDVehicleMaintenanceRenewal{.age = OpenTTDCalendarDate{100}, .max_age = OpenTTDCalendarDate{100}, .months = -4, .enabled = false, .kind = static_cast<VehicleType>(254), .subtype = 0};
	};
	const auto shared = OpenTTDFixtureSharedServices(nullptr, TreeRandom, EffectTestIndustry);
	RoadTestTables tables(&shared);
	auto &base = tables.vehicle;
	base.maintenance = &maintenance;
	base.next = [](OpenTTDVehicle *) noexcept -> OpenTTDVehicle * { return nullptr; };
	base.write_economy_service = [](OpenTTDVehicle *, int32_t date) noexcept { CHECK(date == 0x1020304); };
	base.write_calendar_service = [](OpenTTDVehicle *, int32_t date) noexcept { CHECK(date == -0x12345); };
	base.write_breakdowns = [](OpenTTDVehicle *, uint8_t since) noexcept { CHECK(since == 0); };
	base.write_reliability = [](OpenTTDVehicle *, uint16_t reliability) noexcept { CHECK(reliability == 0x9876); };
	uint8_t opaque;
	auto *shell = reinterpret_cast<OpenTTDVehicle *>(&opaque);
	openttd_rust_vehicle_service_depot(shell, &base);
	openttd_rust_vehicle_decrease_value(shell, &base);
	CHECK(openttd_rust_vehicle_needs_renew(shell, &base, nullptr, false));
	CHECK(!openttd_rust_vehicle_needs_renew(shell, &base, nullptr, true));
	CHECK(dirty == 2 && resets == 1 && values == 1);
}

/* Actual native storage type is checked by the game-side shell probe; this
 * standalone ABI target also exercises const BaseBitSet member identity and
 * native size_t/pointer cursor calls on every platform. */
static void VehicleSchedulerBoundary()
{
	using Status = decltype(std::declval<Vehicle>().vehstatus);
	static_assert(std::is_same_v<Status::BaseType, uint8_t>);
	static_assert(std::is_same_v<decltype(RustVehicleStatusAddress(nullptr)), const uint8_t *>);
	Status status;
	const uint8_t *address = status.base_address();
	static_assert(std::is_same_v<decltype(status.base_address()), const uint8_t *>);
	CHECK(address == status.base_address());
	status.Set(VehState::Hidden);
	CHECK(*address == status.base());
	status.Set(VehState::Crashed);
	CHECK(*address == 129);
	status.Reset(VehState::Hidden);
	CHECK(*address == 128);
	/* Opaque shells are never dereferenced: this target tests the pointer/size_t
	 * call ABI; the game target compares real pool mutations to PoolIterator. */
	static Vehicle *slots[5];
	static size_t live;
	static size_t size;
	std::array<uint8_t, 2> opaque{};
	slots[0] = reinterpret_cast<Vehicle *>(&opaque[0]);
	slots[1] = nullptr;
	slots[2] = reinterpret_cast<Vehicle *>(&opaque[1]);
	slots[3] = slots[4] = nullptr;
	live = size = 2;
	OpenTTDVehicleSchedulerServices services{};
	services.pool = []() noexcept { return OpenTTDVehiclePoolCursor{.slots = slots, .slots_len = size, .first_unused = &live}; };
	auto cursor = services.pool();
	size_t from = 0;
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == reinterpret_cast<OpenTTDVehicle *>(slots[0]));
	CHECK(from == 1);
	live = 3;
	size = 5;
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == reinterpret_cast<OpenTTDVehicle *>(slots[2]));
	CHECK(cursor.slots_len == 5);
	CHECK(cursor.first_unused == &live);
	CHECK(from == 3);
	CHECK(openttd_rust_vehicle_cursor_next(&services, &cursor, &from) == nullptr);
}
int main()
{
	Layouts();
	/* These initialize the shared maintenance table consumed by road probes. */
	VehicleSchedulerBoundary();
	VehicleMaintenanceBoundary();
	/* Independent component probes are alphabetical. */
	AircraftDirectBoundary();
	AircraftOwnership();
	AircraftPhaseBoundary();
	CheckEffectProtocol([](bool result) { CHECK(result); });
	Disasters();
	LinkGraphJob();
	RoadBoundary();
	RoadServiceBoundary();
	ShipBoundary();
	ShipYapfProbe::Run([](bool result) { CHECK(result); });
	Trees();
	WaterProbe::Run([](bool result) { CHECK(result); });
	// Registry: house (#256).

	// Registry: landscape (#212).
	CHECK(GroundProbe::Run());
	LandscapeProbe::Run([](bool result) { CHECK(result); });

	/* Opaque owner, native size_t, immutable byte borrow and complete UTF-8 output. */
	auto *townname = openttd_rust_townname_generate(1, UINT32_MAX);
	size_t name_length;
	const char *name = openttd_rust_townname_data(townname, &name_length);
	CHECK(name_length == 8 && std::memcmp(name, "Alen\xc3\xa7on", name_length) == 0);
	openttd_rust_townname_destroy(townname);
	Blake2b();
	Calls();
	CryptoPrimitives();
	Encoded();
	History();
	Locale();
	Math();
	PacketState();
	ScriptListControl();
	StationCargo();
	StringValidation();
	X25519();
	std::printf("ABI audit passed\n");
}
