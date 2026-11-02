/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file crypto-vectors.cpp Generate immutable Rust known answers from the pinned original. */
#include "3rdparty/monocypher/monocypher.h"
#include "3rdparty/monocypher/monocypher-ed25519.h"
#include <algorithm>
#include <array>
#include <cstdio>
#include <span>
#include <vector>

static void Hex(std::span<const uint8_t> bytes)
{
	std::printf(" ");
	if (bytes.empty()) std::printf("-");
	for (auto byte : bytes) std::printf("%02x", byte);
}

static std::vector<uint8_t> Pattern(size_t size, uint8_t seed)
{
	std::vector<uint8_t> bytes(size);
	for (size_t i = 0; i < size; ++i) bytes[i] = static_cast<uint8_t>(seed + i * 29);
	return bytes;
}

int main()
{
	std::puts("# Monocypher 4.0.2, baseline 14ec60f248547d4d062a1160f0fc26d742319888.");
	auto secret = Pattern(32, 7);
	auto peer_secret = Pattern(32, 99);
	std::array<uint8_t, 32> public_key, peer_public, shared;
	crypto_x25519_public_key(public_key.data(), secret.data());
	crypto_x25519_public_key(peer_public.data(), peer_secret.data());
	for (auto point : {peer_public, std::array<uint8_t, 32>{}, std::array<uint8_t, 32>{1}}) {
		crypto_x25519(shared.data(), secret.data(), point.data());
		std::printf("x"); Hex(secret); Hex(point); Hex(shared); std::puts("");
	}
	std::printf("public"); Hex(secret); Hex(public_key); std::puts("");
	for (size_t length : {0, 1, 127, 128, 129, 256, 1024}) {
		auto message = Pattern(length, 13);
		for (size_t size : {16, 32, 64}) {
			std::array<uint8_t, 64> digest;
			crypto_blake2b(digest.data(), size, message.data(), message.size());
			std::printf("hash %zu", size); Hex(message); Hex(std::span(digest).first(size)); std::puts("");
		}
	}
	crypto_x25519(shared.data(), secret.data(), peer_public.data());
	for (size_t length : {0, 1, 19, 128}) {
		auto extra = Pattern(length, 31);
		crypto_blake2b_ctx hash;
		std::array<uint8_t, 64> keys;
		crypto_blake2b_init(&hash, keys.size());
		crypto_blake2b_update(&hash, shared.data(), shared.size());
		crypto_blake2b_update(&hash, public_key.data(), public_key.size());
		crypto_blake2b_update(&hash, peer_public.data(), peer_public.size());
		crypto_blake2b_update(&hash, extra.data(), extra.size());
		crypto_blake2b_final(&hash, keys.data());
		std::printf("kdf"); Hex(secret); Hex(peer_secret); Hex(extra); Hex(keys); std::puts("");
	}
	for (uint8_t seed : {0, 255}) {
		auto nonce = Pattern(24, seed);
		crypto_aead_ctx stream;
		crypto_aead_init_x(&stream, secret.data(), nonce.data());
		std::printf("stream"); Hex(secret); Hex(nonce); std::puts("");
		for (size_t length : {0, 1, 8, 15, 16, 17, 63, 64, 65, 256, 1460}) {
			for (size_t ad_size : {0, 32}) {
				auto ad = Pattern(ad_size, 93);
				auto message = Pattern(length, 37);
				std::vector<uint8_t> cipher(length);
				std::array<uint8_t, 16> mac;
				crypto_aead_write(&stream, cipher.data(), mac.data(), ad.data(), ad.size(), message.data(), message.size());
				std::printf("message"); Hex(message); Hex(ad); Hex(cipher); Hex(mac); std::puts("");
				crypto_aead_lock(cipher.data(), mac.data(), secret.data(), nonce.data(), ad.data(), ad.size(), message.data(), message.size());
				std::printf("lock"); Hex(secret); Hex(nonce); Hex(message); Hex(ad); Hex(cipher); Hex(mac); std::puts("");
			}
		}
	}
	for (size_t length : {0, 8, 32, 127, 128, 129, 256}) {
		auto message = Pattern(length, 51);
		std::array<uint8_t, 64> digest, expanded, signature;
		std::array<uint8_t, 32> key;
		crypto_sha512(digest.data(), message.data(), message.size());
		std::printf("sha512"); Hex(message); Hex(digest); std::puts("");
		auto seed = secret;
		crypto_ed25519_key_pair(expanded.data(), key.data(), seed.data());
		crypto_ed25519_sign(signature.data(), expanded.data(), message.data(), message.size());
		for (int bit : {-1, 0, 31, 32, 63}) {
			auto changed = signature;
			if (bit >= 0) changed[bit] ^= 1;
			int valid = crypto_ed25519_check(changed.data(), key.data(), message.data(), message.size()) == 0;
			std::printf("ed %d", valid); Hex(changed); Hex(key); Hex(message); std::puts("");
		}
	}
	// Noncanonical encodings, sign bit on zero x, small-order points and S >= L.
	for (int first : {0, 1, 2, 0xec, 0xed, 0xee, 0xff}) {
		for (uint8_t high : {0, 0x7f, 0xff}) {
			std::array<uint8_t, 32> key{};
			if (first >= 0xec) key.fill(0xff);
			key[0] = first; key[31] = high;
			for (int scalar : {0, 1, 255}) {
				std::array<uint8_t, 64> signature{};
				std::copy(key.begin(), key.end(), signature.begin());
				if (scalar == 255) std::fill(signature.begin() + 32, signature.end(), 0xff);
				else signature[32] = scalar;
				int valid = crypto_ed25519_check(signature.data(), key.data(), nullptr, 0) == 0;
				std::printf("ed %d", valid); Hex(signature); Hex(key); Hex({}); std::puts("");
			}
		}
	}
}
