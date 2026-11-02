/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file auth-comparison.cpp Stateful endpoint using actual Packet and authentication implementations. */
#include "stdafx.h"
#include "3rdparty/monocypher/monocypher.h"
#include "debug.h"
#include "core/random_func.hpp"
#include <iostream>
#include <charconv>
#include <sstream>
#include <stdexcept>
#include <cstdlib>

/* Fail only ordinary C++ allocation at the selected application boundary.
 * Packet growth uses the Rust allocator and is outside this C++ exception probe. */
static bool fail_cpp_allocation;
void *operator new(size_t size)
{
	if (fail_cpp_allocation) throw std::bad_alloc{};
	if (void *result = std::malloc(size == 0 ? 1 : size)) return result;
	throw std::bad_alloc{};
}
void operator delete(void *pointer) noexcept { std::free(pointer); }
void operator delete(void *pointer, size_t) noexcept { std::free(pointer); }

/* Fixed-capacity observations cannot allocate/throw from a primitive callback.
 * Every wrapper calls the real unchanged vendor implementation. */
static std::array<uint8_t, 64> last_keys{};
static std::array<uint8_t, 32> last_stream_key{};
static uint64_t last_counter;
static bool hash_wiped = true, wipe_ok = true;
static std::array<size_t, 256> wipe_sizes{};
static size_t wipes;
static void ObserveWipe(void *data, size_t size) noexcept
{
	::crypto_wipe(data, size);
	const auto *bytes = static_cast<const uint8_t *>(data);
	for (size_t i = 0; i < size; ++i) wipe_ok &= bytes[i] == 0;
	if (wipes < wipe_sizes.size()) wipe_sizes[wipes++] = size;
}
static void ObserveHashFinal(crypto_blake2b_ctx *context, uint8_t *output) noexcept
{
	::crypto_blake2b_final(context, output);
	std::copy_n(output, 64, last_keys.begin());
	const auto *bytes = reinterpret_cast<const uint8_t *>(context);
	for (size_t i = 0; i < sizeof(*context); ++i) hash_wiped &= bytes[i] == 0;
}
static void ObserveStream(crypto_aead_ctx *context) noexcept
{
	std::copy_n(context->key, 32, last_stream_key.begin());
	last_counter = context->counter;
}
static void ObserveWrite(crypto_aead_ctx *context, uint8_t *cipher, uint8_t *mac, const uint8_t *ad, size_t ad_size, const uint8_t *message, size_t size) noexcept
{
	::crypto_aead_write(context, cipher, mac, ad, ad_size, message, size);
	ObserveStream(context);
}
static int ObserveRead(crypto_aead_ctx *context, uint8_t *message, const uint8_t *mac, const uint8_t *ad, size_t ad_size, const uint8_t *cipher, size_t size) noexcept
{
	int result = ::crypto_aead_read(context, message, mac, ad, ad_size, cipher, size);
	ObserveStream(context);
	return result;
}
/* Observe the actual selected implementation without replacing any algorithm. */
#define crypto_wipe ObserveWipe
#define crypto_blake2b_final ObserveHashFinal
#define crypto_aead_write ObserveWrite
#define crypto_aead_read ObserveRead
#include "network/network_crypto.cpp"
#undef crypto_wipe
#undef crypto_blake2b_final
#undef crypto_aead_write
#undef crypto_aead_read

int _debug_net_level = 9;
static std::vector<size_t> random_sizes;
static uint8_t random_seed;
static size_t random_throw_after = SIZE_MAX;
static size_t logs;
static std::string last_log;
static bool throw_log;
void RandomBytesWithFallback(std::span<uint8_t> output)
{
	if (random_sizes.size() >= random_throw_after) throw std::runtime_error("RNG fixture");
	for (size_t i = 0; i < output.size(); ++i) output[i] = static_cast<uint8_t>(random_seed + i + random_sizes.size() * 31);
	random_sizes.push_back(output.size());
}
void DebugPrint(std::string_view category, int level, std::string &&message) { if (throw_log) throw std::runtime_error("logger fixture"); ++logs; last_log = fmt::format("{}:{}:{}", category, level, message); }
[[noreturn]] void NOT_REACHED(const std::source_location) { throw std::runtime_error("NOT_REACHED"); }
[[noreturn]] void AssertFailedError(std::string_view, const std::source_location) { throw std::runtime_error("assertion"); }

static unsigned Number(std::string_view input, int base)
{
	unsigned result;
	if (std::from_chars(input.data(), input.data() + input.size(), result, base).ec != std::errc{}) throw std::runtime_error("number");
	return result;
}
static std::vector<uint8_t> Unhex(std::string_view input)
{
	if (input == "-") return {};
	std::vector<uint8_t> bytes;
	for (size_t i = 0; i < input.size(); i += 2) bytes.push_back(static_cast<uint8_t>(Number(input.substr(i, 2), 16)));
	return bytes;
}
static std::string Hex(std::span<const uint8_t> input)
{
	static constexpr char alphabet[] = "0123456789abcdef";
	std::string result;
	for (auto byte : input) { result.push_back(alphabet[byte >> 4]); result.push_back(alphabet[byte & 15]); }
	return result.empty() ? "-" : result;
}
class FixtureSession : public X25519AuthenticationHandler {
public:
	FixtureSession(const X25519SecretKey &secret) : X25519AuthenticationHandler(secret) {}
	using X25519AuthenticationHandler::SendRequest;
	using X25519AuthenticationHandler::ReceiveRequest;
	using X25519AuthenticationHandler::SendResponse;
	using X25519AuthenticationHandler::ReceiveResponse;
	using X25519AuthenticationHandler::SendEnableEncryption;
	using X25519AuthenticationHandler::ReceiveEnableEncryption;
	using X25519AuthenticationHandler::CreateClientToServerEncryptionHandler;
	using X25519AuthenticationHandler::CreateServerToClientEncryptionHandler;
	using X25519AuthenticationHandler::GetPeerPublicKey;
};
static NetworkSocketHandler socket_handler;
static std::string Wire(Packet &packet)
{
	packet.PrepareToSend();
	std::vector<uint8_t> bytes;
	packet.TransferOut([&](std::span<const uint8_t> src) { bytes.assign(src.begin(), src.end()); return src.size(); });
	return Hex(bytes);
}
static Packet Read(std::string_view hex)
{
	auto bytes = Unhex(hex);
	Packet packet(&socket_handler, COMPAT_MTU, bytes.size());
	packet.TransferIn([&](std::span<uint8_t> dest) { std::ranges::copy(bytes, dest.begin()); return bytes.size(); });
	if (!packet.PrepareToRead()) throw std::runtime_error("invalid wire");
	packet.Recv_uint8();
	return packet;
}

/* Direct primitive gaps complement the unchanged mixed endpoint corpus.
 * Every compared context is prefilled so padding/chunk bytes are defined. */
static void PrimitiveRequire(bool condition) { if (!condition) throw std::runtime_error("primitive invariant"); }
static std::vector<uint8_t> PrimitiveBytes(size_t size, unsigned seed)
{
	std::vector<uint8_t> bytes(size);
	for (size_t i = 0; i < size; ++i) bytes[i] = static_cast<uint8_t>(i * 37 + seed);
	return bytes;
}
template <typename T> static std::string ContextBytes(const T &context)
{
	return Hex({reinterpret_cast<const uint8_t *>(&context), sizeof(context)});
}
static void PrimitiveInit(crypto_aead_ctx &ctx, unsigned variant, const uint8_t *key, const uint8_t *nonce)
{
	if (variant == 0) crypto_aead_init_x(&ctx, key, nonce);
	if (variant == 1) crypto_aead_init_djb(&ctx, key, nonce);
	if (variant == 2) crypto_aead_init_ietf(&ctx, key, nonce);
}
/* Bounded BLAKE2b gaps. Source-defined sizes 0/65 and keys 65/128 are
 * reproduction probes, not recommended cryptographic parameters. */
static void BlakePrimitives()
{
	std::cout << "blake-layout " << sizeof(crypto_blake2b_ctx) << ' ' << alignof(crypto_blake2b_ctx)
			<< ' ' << offsetof(crypto_blake2b_ctx, hash) << ' ' << offsetof(crypto_blake2b_ctx, input_offset)
			<< ' ' << offsetof(crypto_blake2b_ctx, input) << ' ' << offsetof(crypto_blake2b_ctx, input_idx)
			<< ' ' << offsetof(crypto_blake2b_ctx, hash_size) << '\n';
	struct HashCase { size_t digest, key, message; };
	std::vector<HashCase> cases;
	const std::array<size_t, 10> sizes{0, 1, 7, 8, 9, 16, 32, 63, 64, 65};
	const std::array<size_t, 6> keys{0, 1, 32, 64, 65, 128};
	const std::array<size_t, 13> messages{0, 1, 7, 8, 9, 127, 128, 129, 255, 256, 257, 1024, 1041};
	for (size_t i = 0; i < messages.size(); ++i) cases.push_back({sizes[i % sizes.size()], keys[i % keys.size()], messages[i]});
	for (size_t size : sizes) cases.push_back({size, 32, 129});
	for (size_t size : keys) cases.push_back({64, size, 128});
	for (size_t case_id = 0; case_id < cases.size(); ++case_id) {
		const auto [digest, key_size, length] = cases[case_id];
		auto key = PrimitiveBytes(key_size, 17), message = PrimitiveBytes(length, 53);
		std::array<uint8_t, 80> expected;
		expected.fill(0xA5);
		crypto_blake2b_keyed(expected.data() + 8, digest, key_size ? key.data() : nullptr, key_size, length ? message.data() : nullptr, length);
		PrimitiveRequire(std::all_of(expected.begin(), expected.begin() + 8, [](uint8_t x) { return x == 0xA5; }));
		PrimitiveRequire(std::all_of(expected.begin() + 8 + std::min(digest, size_t{64}), expected.end(), [](uint8_t x) { return x == 0xA5; }));
		std::cout << "blake-oneshot " << case_id << ' ' << digest << ' ' << key_size << ' ' << length << ' ' << Hex(expected) << '\n';
		if (!key_size) {
			std::array<uint8_t, 80> direct; direct.fill(0xA5);
			crypto_blake2b(direct.data() + 8, digest, length ? message.data() : nullptr, length);
			PrimitiveRequire(direct == expected);
			std::cout << "blake-unkeyed " << case_id << ' ' << Hex(direct) << '\n';
		}
		// Word and block partitions, plus real 1024-byte file chunk shapes.
		for (size_t boundary : {size_t{7}, size_t{8}, size_t{9}, size_t{127}, size_t{128}, size_t{1024}}) {
			crypto_blake2b_ctx ctx;
			std::fill_n(reinterpret_cast<uint8_t *>(&ctx), sizeof(ctx), 0xA5);
			if (key_size) crypto_blake2b_keyed_init(&ctx, digest, key.data(), key_size);
			else crypto_blake2b_init(&ctx, digest);
			std::cout << "blake-init " << case_id << ' ' << boundary << ' ' << ContextBytes(ctx) << '\n';
			crypto_blake2b_update(&ctx, nullptr, 0);
			const auto split = std::min(boundary, length);
			crypto_blake2b_update(&ctx, length ? message.data() : nullptr, split);
			std::cout << "blake-update " << case_id << ' ' << boundary << ' ' << ContextBytes(ctx) << '\n';
			crypto_blake2b_update(&ctx, nullptr, 0);
			crypto_blake2b_update(&ctx, length ? message.data() + split : nullptr, length - split);
			crypto_blake2b_update(&ctx, nullptr, 0);
			std::array<uint8_t, 80> result; result.fill(0xA5);
			crypto_blake2b_final(&ctx, result.data() + 8);
			PrimitiveRequire(result == expected && std::ranges::all_of(ContextBytes(ctx), [](char c) { return c == '0'; }));
			std::cout << "blake-final " << case_id << ' ' << boundary << ' ' << Hex(result) << ' ' << ContextBytes(ctx) << '\n';
		}
	}
	// Defined zero-output finalization: a null output is never dereferenced.
	{
		auto message = PrimitiveBytes(129, 19), key = PrimitiveBytes(32, 23);
		crypto_blake2b_ctx ctx;
		std::fill_n(reinterpret_cast<uint8_t *>(&ctx), sizeof(ctx), 0xA5);
		crypto_blake2b_init(&ctx, 0); crypto_blake2b_update(&ctx, message.data(), message.size());
		crypto_blake2b_final(&ctx, nullptr);
		PrimitiveRequire(std::ranges::all_of(ContextBytes(ctx), [](char c) { return c == '0'; }));
		crypto_blake2b(nullptr, 0, message.data(), message.size());
		crypto_blake2b_keyed(nullptr, 0, key.data(), key.size(), message.data(), message.size());
		std::cout << "blake-null-final " << ContextBytes(ctx) << '\n';
	}
	// Snapshot fields before divergent continuations; no incidental uninitialized padding.
	{
		auto message = PrimitiveBytes(129, 67), key = PrimitiveBytes(32, 13);
		crypto_blake2b_ctx left;
		std::fill_n(reinterpret_cast<uint8_t *>(&left), sizeof(left), 0xA5);
		crypto_blake2b_keyed_init(&left, 32, key.data(), key.size());
		crypto_blake2b_update(&left, message.data(), 127);
		auto right = left;
		crypto_blake2b_update(&left, message.data() + 127, 1);
		crypto_blake2b_update(&right, message.data() + 127, 2);
		std::cout << "blake-copy-state " << ContextBytes(left) << ' ' << ContextBytes(right) << '\n';
		std::array<uint8_t, 32> a, b, expected;
		crypto_blake2b_final(&left, a.data()); crypto_blake2b_final(&right, b.data());
		crypto_blake2b_keyed(expected.data(), 32, key.data(), key.size(), message.data(), 128); PrimitiveRequire(a == expected);
		crypto_blake2b_keyed(expected.data(), 32, key.data(), key.size(), message.data(), 129); PrimitiveRequire(b == expected && a != b);
		std::cout << "blake-copy-final " << Hex(a) << ' ' << Hex(b) << ' ' << ContextBytes(left) << ' ' << ContextBytes(right) << '\n';
	}
	{
		auto message = PrimitiveBytes(129, 89);
		crypto_blake2b_ctx ctx;
		std::fill_n(reinterpret_cast<uint8_t *>(&ctx), sizeof(ctx), 0xA5);
		crypto_blake2b_init(&ctx, 64); crypto_blake2b_update(&ctx, message.data(), 128);
		ctx.input_offset[0] = UINT64_MAX - 127; ctx.input_offset[1] = UINT64_C(0x1122334455667788);
		crypto_blake2b_update(&ctx, message.data() + 128, 1);
		PrimitiveRequire(ctx.input_offset[0] == 0 && ctx.input_offset[1] == UINT64_C(0x1122334455667789) && ctx.input_idx == 1);
		std::cout << "blake-counter " << ContextBytes(ctx) << '\n';
		std::array<uint8_t, 64> digest; crypto_blake2b_final(&ctx, digest.data());
		std::cout << "blake-counter-final " << Hex(digest) << ' ' << ContextBytes(ctx) << '\n';
	}
	// Equal and partial overlap in both directions, and key/message/output sharing.
	for (unsigned style = 0; style != 6; ++style) {
		auto storage = PrimitiveBytes(320, 73), original = storage;
		const size_t message_at = 80, output_at = style == 0 ? 80 : style == 1 ? 96 : style == 2 ? 64 : style == 3 ? 8 : style == 4 ? 32 : 112;
		const size_t key_at = style == 5 ? 96 : 8, key_size = style == 5 ? 64 : 32;
		std::array<uint8_t, 64> expected;
		crypto_blake2b_keyed(expected.data(), 64, original.data() + key_at, key_size, original.data() + message_at, 129);
		crypto_blake2b_keyed(storage.data() + output_at, 64, storage.data() + key_at, key_size, storage.data() + message_at, 129);
		PrimitiveRequire(std::equal(expected.begin(), expected.end(), storage.begin() + output_at));
		PrimitiveRequire(std::equal(storage.begin(), storage.begin() + output_at, original.begin()));
		PrimitiveRequire(std::equal(storage.begin() + output_at + 64, storage.end(), original.begin() + output_at + 64));
		std::cout << "blake-overlap " << style << ' ' << Hex(storage) << '\n';
	}
	// Exact unchanged extended_hash and EdDSA buffer shapes.
	{
		auto storage = PrimitiveBytes(96, 41), original = storage;
		std::array<uint8_t, 64> expected;
		crypto_blake2b(expected.data(), 64, original.data(), 64);
		crypto_blake2b(storage.data() + 32, 64, storage.data(), 64);
		PrimitiveRequire(std::equal(expected.begin(), expected.end(), storage.begin() + 32));
		std::cout << "blake-extended-overlap " << Hex(storage) << '\n';
		auto seed = PrimitiveBytes(64, 23); original = seed;
		crypto_blake2b(expected.data(), 64, original.data(), 32); crypto_blake2b(seed.data(), 64, seed.data(), 32);
		PrimitiveRequire(std::equal(expected.begin(), expected.end(), seed.begin()));
		std::cout << "blake-eddsa-overlap " << Hex(seed) << '\n';
	}
	{
		auto password = PrimitiveBytes(19, 43), salt = PrimitiveBytes(16, 29);
		std::vector<uint64_t> work(8 * 128);
		std::array<uint8_t, 32> hash;
		crypto_argon2_config config{CRYPTO_ARGON2_ID, 8, 1, 1};
		crypto_argon2_inputs inputs{password.data(), salt.data(), static_cast<uint32_t>(password.size()), static_cast<uint32_t>(salt.size())};
		crypto_argon2(hash.data(), hash.size(), work.data(), config, inputs, crypto_argon2_no_extras);
		std::cout << "blake-argon2-caller " << Hex(hash) << ' ' << Hex({reinterpret_cast<const uint8_t *>(work.data()), work.size() * sizeof(uint64_t)}) << '\n';
	}
	{
		auto seed = PrimitiveBytes(32, 37), message = PrimitiveBytes(129, 49);
		std::array<uint8_t, 64> secret, signature;
		std::array<uint8_t, 32> public_key;
		crypto_eddsa_key_pair(secret.data(), public_key.data(), seed.data());
		crypto_eddsa_sign(signature.data(), secret.data(), message.data(), message.size());
		PrimitiveRequire(crypto_eddsa_check(signature.data(), public_key.data(), message.data(), message.size()) == 0);
		auto bad = signature; bad[31] ^= 1;
		const auto failure = crypto_eddsa_check(bad.data(), public_key.data(), message.data(), message.size()); PrimitiveRequire(failure == -1);
		PrimitiveRequire(std::ranges::all_of(seed, [](uint8_t x) { return x == 0; }));
		std::cout << "blake-eddsa-caller " << Hex(secret) << ' ' << Hex(public_key) << ' ' << Hex(signature) << ' ' << failure << ' ' << Hex(seed) << '\n';
	}
}

/* Only direct curve/overlap/count gaps; the shim includes actual vendor source
 * unchanged, exposing its private coarse helper without a rewritten oracle. */
extern "C" void FixtureLadder(uint8_t *, const uint8_t *, const uint8_t *, int32_t);
static void X25519Primitives()
{
	std::vector<std::array<uint8_t, 32>> scalars(6);
	scalars[0].fill(0); scalars[1].fill(0xff);
	for (size_t i = 0; i < 32; ++i) scalars[2][i] = static_cast<uint8_t>(i * 37 + 13);
	scalars[3] = scalars[2]; scalars[3][0] ^= 7; scalars[3][31] ^= 0xc0;
	scalars[4] = scalars[2]; scalars[4][0] &= 0xf8; scalars[4][31] &= 0x3f;
	scalars[5] = scalars[2]; scalars[5][0] |= 7; scalars[5][31] |= 0xc0;
	std::vector<std::array<uint8_t, 32>> points(9);
	points[1][0] = 1; points[2][0] = 9; points[3] = points[2]; points[3][31] = 0x80;
	for (size_t i : {4, 5, 6, 7}) { points[i].fill(0xff); points[i][31] = 0x7f; }
	points[4][0] = 0xec; points[5][0] = 0xed; points[6][0] = 0xee;
	points[8] = points[7]; points[8][31] = 0xff;
	std::array<uint8_t, 32> clamped_public{};
	for (size_t i = 0; i < scalars.size(); ++i) {
		std::array<uint8_t, 32> public_key, trimmed;
		crypto_x25519_public_key(public_key.data(), scalars[i].data());
		crypto_eddsa_trim_scalar(trimmed.data(), scalars[i].data());
		if (i == 2) clamped_public = public_key;
		if (i > 2) PrimitiveRequire(public_key == clamped_public);
		std::cout << "x25519-public " << i << ' ' << Hex(public_key) << ' ' << Hex(trimmed) << '\n';
		std::array<uint8_t, 32> low_base, low_max;
		for (size_t j = 0; j < points.size(); ++j) {
			std::array<uint8_t, 32> shared;
			crypto_x25519(shared.data(), scalars[i].data(), points[j].data());
			if (j == 2) low_base = shared;
			if (j == 3) PrimitiveRequire(shared == low_base);
			if (j == 7) low_max = shared;
			if (j == 8) PrimitiveRequire(shared == low_max);
			std::cout << "x25519-shared " << i << ' ' << j << ' ' << Hex(shared) << '\n';
		}
	}
	for (unsigned style = 0; style != 7; ++style) {
		auto storage = PrimitiveBytes(160, 31), original = storage;
		const size_t output_at = style == 0 ? 32 : style == 1 ? 37 : style == 2 ? 27 : style == 3 ? 80 : style == 4 ? 87 : style == 5 ? 73 : 56;
		std::array<uint8_t, 32> expected;
		crypto_x25519(expected.data(), original.data() + 32, original.data() + 80);
		crypto_x25519(storage.data() + output_at, storage.data() + 32, storage.data() + 80);
		PrimitiveRequire(std::equal(expected.begin(), expected.end(), storage.begin() + output_at));
		PrimitiveRequire(std::equal(storage.begin(), storage.begin() + output_at, original.begin()));
		PrimitiveRequire(std::equal(storage.begin() + output_at + 32, storage.end(), original.begin() + output_at + 32));
		std::cout << "x25519-overlap " << style << ' ' << Hex(storage) << '\n';
	}
	for (int delta : {-7, -1, 0, 1, 7}) {
		auto storage = PrimitiveBytes(96, 19), original = storage;
		const auto output_at = static_cast<size_t>(32 + delta);
		crypto_eddsa_trim_scalar(storage.data() + output_at, storage.data() + 32);
		PrimitiveRequire(std::equal(storage.begin(), storage.begin() + output_at, original.begin()));
		PrimitiveRequire(std::equal(storage.begin() + output_at + 32, storage.end(), original.begin() + output_at + 32));
		std::cout << "x25519-trim-forward " << delta << ' ' << Hex(storage) << '\n';
		storage = original;
		std::array<uint8_t, 32> expected;
		crypto_x25519_public_key(expected.data(), original.data() + 32);
		crypto_x25519_public_key(storage.data() + output_at, storage.data() + 32);
		PrimitiveRequire(std::equal(expected.begin(), expected.end(), storage.begin() + output_at));
		std::cout << "x25519-public-overlap " << delta << ' ' << Hex(storage) << '\n';
	}
	for (unsigned style = 0; style != 2; ++style) {
		std::array<uint8_t, 32> scalar{}, point{}, short_result, full_result;
		scalar[31] = 0x80; scalar[0] = style == 0 ? 0 : 19; point[0] = 9;
		FixtureLadder(short_result.data(), scalar.data(), point.data(), 255);
		FixtureLadder(full_result.data(), scalar.data(), point.data(), 256);
		PrimitiveRequire(short_result != full_result);
		std::cout << "x25519-ladder-bits " << style << ' ' << Hex(short_result) << ' ' << Hex(full_result) << '\n';
		// Coarse helper itself must consume all scalar/point bytes before output.
		std::array<uint8_t, 64> storage{};
		std::copy(scalar.begin(), scalar.end(), storage.begin()); std::copy(point.begin(), point.end(), storage.begin() + 32);
		FixtureLadder(storage.data() + 17, storage.data(), storage.data() + 32, 256);
		PrimitiveRequire(std::equal(full_result.begin(), full_result.end(), storage.begin() + 17));
		std::cout << "x25519-ladder-overlap " << style << ' ' << Hex(storage) << '\n';
	}
	for (unsigned seed : {0, 1, 7}) {
		auto secret = PrimitiveBytes(32, seed), other = PrimitiveBytes(32, seed + 53);
		std::array<uint8_t, 32> small, fast, base, point, inverse;
		crypto_x25519_dirty_small(small.data(), secret.data()); crypto_x25519_dirty_fast(fast.data(), secret.data());
		PrimitiveRequire(small == fast);
		std::cout << "x25519-dirty-caller " << seed << ' ' << Hex(small) << ' ' << Hex(fast) << '\n';
		crypto_x25519_public_key(base.data(), other.data()); crypto_x25519(point.data(), secret.data(), base.data());
		crypto_x25519_inverse(inverse.data(), secret.data(), point.data()); PrimitiveRequire(inverse == base);
		std::cout << "x25519-inverse-caller " << seed << ' ' << Hex(base) << ' ' << Hex(point) << ' ' << Hex(inverse) << '\n';
	}
}

static int PrimitiveMain()
{
	auto key = PrimitiveBytes(32, 9), nonce = PrimitiveBytes(24, 101);
	std::cout << "layout poly " << sizeof(crypto_poly1305_ctx) << ' ' << alignof(crypto_poly1305_ctx)
			<< ' ' << offsetof(crypto_poly1305_ctx, c) << ' ' << offsetof(crypto_poly1305_ctx, c_idx)
			<< ' ' << offsetof(crypto_poly1305_ctx, r) << ' ' << offsetof(crypto_poly1305_ctx, pad) << ' ' << offsetof(crypto_poly1305_ctx, h) << '\n';
	std::cout << "layout aead " << sizeof(crypto_aead_ctx) << ' ' << alignof(crypto_aead_ctx)
			<< ' ' << offsetof(crypto_aead_ctx, counter) << ' ' << offsetof(crypto_aead_ctx, key) << ' ' << offsetof(crypto_aead_ctx, nonce) << '\n';
	for (unsigned overlap = 0; overlap != 3; ++overlap) {
		auto storage = PrimitiveBytes(80, 19);
		std::array<uint8_t, 32> out;
		uint8_t *target = overlap == 0 ? out.data() : storage.data() + (overlap == 1 ? 0 : 32);
		crypto_chacha20_h(target, storage.data(), storage.data() + 16);
		std::cout << "h " << overlap << ' ' << Hex({target, 32}) << '\n';
	}
	for (unsigned variant = 0; variant != 3; ++variant) for (size_t length : {0, 1, 63, 64, 65, 129}) {
		for (uint64_t counter : {uint64_t{0}, uint64_t{UINT32_MAX - 1}, uint64_t{UINT64_MAX}}) for (unsigned style = 0; style != 3; ++style) {
			auto plain = PrimitiveBytes(length, 27);
			std::vector<uint8_t> output(length + 8, 0xA5);
			if (style == 1) std::copy(plain.begin(), plain.end(), output.begin());
			const uint8_t *input = style == 2 ? nullptr : style == 1 ? output.data() : plain.data();
			uint64_t next = 0;
			if (variant == 0) next = crypto_chacha20_x(output.data(), input, length, key.data(), nonce.data(), counter);
			if (variant == 1) next = crypto_chacha20_djb(output.data(), input, length, key.data(), nonce.data(), counter);
			if (variant == 2) next = crypto_chacha20_ietf(output.data(), input, length, key.data(), nonce.data(), static_cast<uint32_t>(counter));
			std::cout << "cipher " << variant << ' ' << length << ' ' << counter << ' ' << style << ' ' << next << ' ' << Hex(output) << '\n';
		}
	}
	for (unsigned variant = 0; variant != 3; ++variant) {
		auto storage = PrimitiveBytes(96, 33);
		std::array<uint8_t, 65> out;
		uint64_t next = 0;
		if (variant == 0) next = crypto_chacha20_x(out.data(), nullptr, out.size(), storage.data(), storage.data() + 8, UINT32_MAX);
		if (variant == 1) next = crypto_chacha20_djb(out.data(), nullptr, out.size(), storage.data(), storage.data() + 8, UINT32_MAX);
		if (variant == 2) next = crypto_chacha20_ietf(out.data(), nullptr, out.size(), storage.data(), storage.data() + 8, UINT32_MAX);
		std::cout << "key-nonce-overlap " << variant << ' ' << next << ' ' << Hex(out) << '\n';
	}
	{
		auto buf = PrimitiveBytes(64, 41);
		uint8_t zero_nonce[8]{};
		auto next = crypto_chacha20_djb(buf.data(), nullptr, 64, buf.data() + 32, zero_nonce, 0);
		std::cout << "elligator-key-output " << next << ' ' << Hex(buf) << '\n';
	}
	for (unsigned seed : {0, 1, 93}) {
		auto input = PrimitiveBytes(32, seed);
		std::array<uint8_t, 32> hidden, secret;
		crypto_elligator_key_pair(hidden.data(), secret.data(), input.data());
		PrimitiveRequire(std::ranges::all_of(input, [](uint8_t x) { return x == 0; }));
		std::cout << "elligator-pair " << seed << ' ' << Hex(hidden) << ' ' << Hex(secret) << ' ' << Hex(input) << '\n';
	}
	for (size_t length : {0, 15, 16, 17, 31, 32, 33}) {
		auto message = PrimitiveBytes(length, 59);
		std::array<uint8_t, 16> expected;
		crypto_poly1305(expected.data(), message.data(), length, key.data());
		std::cout << "poly-oneshot " << length << ' ' << Hex(expected) << '\n';
		for (size_t boundary : {0, 1, 15, 16, 17}) {
			const auto split = std::min(boundary, length);
			crypto_poly1305_ctx ctx;
			std::fill_n(reinterpret_cast<uint8_t *>(&ctx), sizeof(ctx), 0xA5);
			crypto_poly1305_init(&ctx, key.data());
			std::cout << "poly-init " << length << ' ' << split << ' ' << ContextBytes(ctx) << '\n';
			crypto_poly1305_update(&ctx, nullptr, 0);
			crypto_poly1305_update(&ctx, message.data(), split);
			std::cout << "poly-update " << length << ' ' << split << ' ' << ContextBytes(ctx) << '\n';
			crypto_poly1305_update(&ctx, nullptr, 0);
			crypto_poly1305_update(&ctx, length == 0 ? nullptr : message.data() + split, length - split);
			crypto_poly1305_update(&ctx, nullptr, 0);
			std::array<uint8_t, 16> mac;
			crypto_poly1305_final(&ctx, mac.data());
			PrimitiveRequire(mac == expected);
			PrimitiveRequire(std::ranges::all_of(ContextBytes(ctx), [](char c) { return c == '0'; }));
			std::cout << "poly-final " << length << ' ' << split << ' ' << Hex(mac) << ' ' << ContextBytes(ctx) << '\n';
		}
	}
	for (unsigned variant = 0; variant != 3; ++variant) for (size_t ad_size : {0, 1, 15, 16, 17, 33}) {
		auto ad = PrimitiveBytes(ad_size, 71);
		crypto_aead_ctx send, receive;
		std::fill_n(reinterpret_cast<uint8_t *>(&send), sizeof(send), 0xA5); std::fill_n(reinterpret_cast<uint8_t *>(&receive), sizeof(receive), 0xA5);
		PrimitiveInit(send, variant, key.data(), nonce.data()); PrimitiveInit(receive, variant, key.data(), nonce.data());
		std::cout << "aead-init " << variant << ' ' << ad_size << ' ' << ContextBytes(send) << '\n';
		for (size_t length : {0, 17, 65}) {
			auto plain = PrimitiveBytes(length, 83), cipher = plain;
			std::array<uint8_t, 16> mac;
			crypto_aead_write(&send, cipher.data(), mac.data(), ad.data(), ad_size, cipher.data(), length);
			std::cout << "aead-write " << variant << ' ' << ad_size << ' ' << length << ' ' << Hex(cipher) << ' ' << Hex(mac) << ' ' << ContextBytes(send) << '\n';
			std::vector<uint8_t> output(length + 8, 0xA5);
			auto bad = mac; bad[0] ^= 1;
			const auto before = ContextBytes(receive);
			int failed = crypto_aead_read(&receive, output.data(), bad.data(), ad.data(), ad_size, cipher.data(), length);
			PrimitiveRequire(failed == -1 && ContextBytes(receive) == before && std::ranges::all_of(output, [](uint8_t x) { return x == 0xA5; }));
			std::cout << "aead-failure " << failed << ' ' << Hex(output) << ' ' << ContextBytes(receive) << '\n';
			crypto_aead_ctx in_place = receive;
			int ok = crypto_aead_read(&receive, output.data(), mac.data(), ad.data(), ad_size, cipher.data(), length);
			PrimitiveRequire(ok == 0 && std::equal(plain.begin(), plain.end(), output.begin()) && ContextBytes(receive) == ContextBytes(send));
			ok = crypto_aead_read(&in_place, cipher.data(), mac.data(), ad.data(), ad_size, cipher.data(), length);
			PrimitiveRequire(ok == 0 && cipher == plain && ContextBytes(in_place) == ContextBytes(send));
			std::cout << "aead-retry " << ok << ' ' << Hex(output) << ' ' << Hex(cipher) << ' ' << ContextBytes(receive) << '\n';
		}
	}
	for (size_t ad_size : {0, 1, 15, 16, 17, 33}) for (size_t length : {0, 17, 65}) {
		auto ad = PrimitiveBytes(ad_size, 11), plain = PrimitiveBytes(length, 21);
		std::vector<uint8_t> cipher(length + 8, 0xA5), output(length + 8, 0xA5);
		std::array<uint8_t, 16> mac;
		crypto_aead_lock(cipher.data(), mac.data(), key.data(), nonce.data(), ad.data(), ad_size, plain.data(), length);
		auto bad = mac; bad[15] ^= 1;
		PrimitiveRequire(crypto_aead_unlock(output.data(), bad.data(), key.data(), nonce.data(), ad.data(), ad_size, cipher.data(), length) == -1);
		PrimitiveRequire(std::ranges::all_of(output, [](uint8_t x) { return x == 0xA5; }));
		PrimitiveRequire(crypto_aead_unlock(output.data(), mac.data(), key.data(), nonce.data(), ad.data(), ad_size, cipher.data(), length) == 0);
		PrimitiveRequire(std::equal(plain.begin(), plain.end(), output.begin()));
		std::cout << "aead-oneshot " << ad_size << ' ' << length << ' ' << Hex(cipher) << ' ' << Hex(mac) << ' ' << Hex(output) << '\n';
	}
	BlakePrimitives();
	X25519Primitives();
	return 0;
}

int main(int argc, char **argv)
{
	if (argc == 2 && std::string_view(argv[1]) == "--primitives") return PrimitiveMain();
	std::unique_ptr<FixtureSession> session, copy;
	std::unique_ptr<NetworkEncryptionHandler> stream, stream_copy;
	std::unique_ptr<X25519DerivedKeys> keys;
	std::string line;
	while (std::getline(std::cin, line)) {
		std::istringstream input(line);
		std::string command, a, b;
		input >> command >> a >> b;
		try {
			if (command == "new") {
				session.reset(); copy.reset(); stream.reset(); stream_copy.reset();
				last_keys.fill(0); random_sizes.clear(); logs = 0; last_log.clear(); wipes = 0; wipe_ok = hash_wiped = true;
				auto secret_bytes = Unhex(a);
				X25519SecretKey secret;
				std::ranges::copy(secret_bytes, secret.begin());
				random_seed = static_cast<uint8_t>(Number(b, 10));
				session = std::make_unique<FixtureSession>(secret);
				std::cout << "ok";
			} else if (command == "request" || command == "response" || command == "nonce") {
				Packet packet(&socket_handler, PacketType{});
				bool ok = true;
				if (command == "request") session->SendRequest(packet);
				if (command == "response") { auto extra = Unhex(a); ok = session->SendResponse(packet, std::string_view(reinterpret_cast<const char *>(extra.data()), extra.size())); }
				if (command == "nonce") session->SendEnableEncryption(packet);
				std::cout << ok << ' ' << Wire(packet);
			} else if (command == "recv-request" || command == "recv-response" || command == "recv-nonce") {
				auto packet = Read(a);
				int result;
				if (command == "recv-request") result = session->ReceiveRequest(packet);
				else if (command == "recv-nonce") result = session->ReceiveEnableEncryption(packet);
				else { auto extra = Unhex(b); result = static_cast<int>(session->ReceiveResponse(packet, std::string_view(reinterpret_cast<const char *>(extra.data()), extra.size()))); }
				std::cout << result << ' ' << packet.RemainingBytesToTransfer();
			} else if (command == "state") {
				std::cout << session->GetPeerPublicKey() << ' ' << Hex(last_keys) << ' ' << logs << ' ' << hash_wiped << ' ' << Hex(std::span(reinterpret_cast<const uint8_t *>(last_log.data()), last_log.size())) << ' ';
				for (auto size : random_sizes) std::cout << size << ',';
			} else if (command == "init-stream") {
				stream = a == "0" ? session->CreateClientToServerEncryptionHandler() : session->CreateServerToClientEncryptionHandler();
				std::cout << stream->MACSize();
			} else if (command == "encrypt" || command == "decrypt") {
				auto message = Unhex(a);
				std::array<uint8_t, 16> mac{};
				bool ok = true;
				if (command == "encrypt") stream->Encrypt(mac, message);
				else { auto raw_mac = Unhex(b); std::ranges::copy(raw_mac, mac.begin()); ok = stream->Decrypt(mac, message); }
				std::cout << ok << ' ' << Hex(mac) << ' ' << Hex(message) << ' ' << Hex(last_stream_key) << ' ' << last_counter;
			} else if (command == "copy-stream") {
				stream_copy = std::make_unique<X25519EncryptionHandler>(*static_cast<X25519EncryptionHandler *>(stream.get()));
				std::cout << stream_copy->MACSize();
			} else if (command == "assign-stream") {
				*static_cast<X25519EncryptionHandler *>(stream_copy.get()) = *static_cast<X25519EncryptionHandler *>(stream.get());
				*static_cast<X25519EncryptionHandler *>(stream_copy.get()) = *static_cast<X25519EncryptionHandler *>(stream_copy.get());
				std::cout << stream_copy->MACSize();
			} else if (command == "swap-stream") {
				stream.swap(stream_copy); std::cout << stream->MACSize();
			} else if (command == "derive") {
				std::string side, payload; input >> side >> payload;
				auto secret_bytes = Unhex(a), peer_bytes = Unhex(b), extra = Unhex(payload);
				X25519SecretKey secret; X25519PublicKey peer;
				std::ranges::copy(secret_bytes, secret.begin()); std::ranges::copy(peer_bytes, peer.begin());
				if (!keys) keys = std::make_unique<X25519DerivedKeys>();
				bool ok = keys->Exchange(peer, static_cast<X25519KeyExchangeSide>(Number(side, 10)), secret, secret.CreatePublicKey(),
						std::string_view(reinterpret_cast<const char *>(extra.data()), extra.size()));
				std::cout << ok << ' ' << Hex(keys->ClientToServer()) << Hex(keys->ServerToClient());
			} else if (command == "alias-keys") {
				auto secret_bytes = Unhex(a), peer_bytes = Unhex(b);
				X25519SecretKey secret; X25519PublicKey peer;
				std::ranges::copy(secret_bytes, secret.begin()); std::ranges::copy(peer_bytes, peer.begin());
				auto before = keys->ClientToServer();
				bool ok = keys->Exchange(peer, X25519KeyExchangeSide::CLIENT, secret, secret.CreatePublicKey(),
						std::string_view(reinterpret_cast<const char *>(before.data()), before.size()));
				bool stable_exchange = before.data() == keys->ClientToServer().data();
				std::cout << ok << ' ' << stable_exchange << ' ' << Hex(keys->ClientToServer()) << Hex(keys->ServerToClient());
				X25519DerivedKeys replacement(*keys);
				replacement.Exchange(peer, X25519KeyExchangeSide::CLIENT, secret, secret.CreatePublicKey(), "assignment");
				*keys = replacement;
				std::cout << ' ' << (before.data() == keys->ClientToServer().data()) << ' ' << Hex(before) << Hex(keys->ServerToClient());
			} else if (command == "copy-keys") {
				X25519DerivedKeys copied(*keys); copied = *keys; copied = copied;
				X25519DerivedKeys moved(std::move(*keys));
				std::cout << Hex(copied.ClientToServer()) << Hex(copied.ServerToClient()) << ' ' << Hex(moved.ClientToServer()) << Hex(moved.ServerToClient());
			} else if (command == "copy") {
				copy = std::make_unique<FixtureSession>(*session);
				std::cout << copy->GetPeerPublicKey();
			} else if (command == "move-copy") {
				copy = std::make_unique<FixtureSession>(std::move(*session));
				std::cout << (copy->GetPeerPublicKey() == session->GetPeerPublicKey());
			} else if (command == "assign") {
				*copy = *session; *copy = *copy;
				session.swap(copy);
				std::cout << session->GetPeerPublicKey();
			} else if (command == "drop") {
				wipes = 0; session.reset(); copy.reset(); stream.reset(); stream_copy.reset(); keys.reset();
				std::cout << wipe_ok << ' ';
				for (size_t i = 0; i < wipes; ++i) std::cout << wipe_sizes[i] << ',';
			} else if (command == "throw-operations") {
				wipes = 0; bool caught = false;
				try {
					FixtureSession owned(*session);
					if (a == "log") {
						auto empty = Read("030000"); throw_log = true;
						owned.ReceiveRequest(empty);
					} else {
						fail_cpp_allocation = true;
						(void)owned.GetPeerPublicKey();
					}
				} catch (const std::exception &) { caught = true; }
				fail_cpp_allocation = false; throw_log = false;
				std::cout << caught << ' ' << wipe_ok << ' ' << (wipes != 0);
			} else if (command == "throw-construction") {
				auto secret_bytes = Unhex(a); X25519SecretKey secret;
				std::ranges::copy(secret_bytes, secret.begin());
				wipes = 0; random_throw_after = random_sizes.size() + Number(b, 10);
				try { FixtureSession thrown(secret); } catch (const std::runtime_error &) {}
				random_throw_after = SIZE_MAX;
				std::cout << wipe_ok << ' ' << (wipes != 0);
			} else throw std::runtime_error("unknown command");
			std::cout << '\n' << std::flush;
		} catch (const std::exception &error) { std::cout << "error " << error.what() << '\n' << std::flush; }
	}
}
