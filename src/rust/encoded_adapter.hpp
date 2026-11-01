/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file encoded_adapter.hpp Ownership, tagged descriptors, and original diagnostic/assert adapters. */
#ifndef RUST_ENCODED_ADAPTER_HPP
#define RUST_ENCODED_ADAPTER_HPP

#include "encoded_ffi.h"
#include "../strings_type.h"
#include "../core/string_consumer.hpp"
#include "../table/control_codes.h"
#if defined(STRGEN) || defined(SETTINGSGEN)
#include "../error_func.h"
#else
#include "../debug.h"
#endif

static_assert(sizeof(StringID) == sizeof(uint32_t));
static_assert(SCC_RECORD_SEPARATOR == 0x1E && SCC_ENCODED == 0xE000 && SCC_ENCODED_INTERNAL == 0xE001 && SCC_ENCODED_NUMERIC == 0xE002 && SCC_ENCODED_STRING == 0xE003);

#ifdef WITH_ASSERT
static constexpr uint8_t RUST_ENCODED_STRING_ASSERTIONS = 1;
#else
static constexpr uint8_t RUST_ENCODED_STRING_ASSERTIONS = 0;
#endif
#if !defined(NDEBUG) || defined(WITH_ASSERT)
static constexpr uint8_t RUST_ENCODED_NUMERIC_ASSERTIONS = 1;
#else
static constexpr uint8_t RUST_ENCODED_NUMERIC_ASSERTIONS = 0;
#endif

/** RAII returns storage to Rust even if C++ allocation/diagnostic formatting throws. */
using RustEncodedOwner = std::unique_ptr<OpenTTDRustEncodedResult, decltype(&openttd_rust_encoded_destroy)>;

inline RustEncodedOwner OwnRustEncoded(OpenTTDRustEncodedResult *result)
{
	return {result, openttd_rust_encoded_destroy};
}

/** Borrow variant payloads for one synchronous Rust operation; no layouts cross FFI. */
inline OpenTTDRustEncodedParameter RustEncodedParameter(const StringParameter &parameter)
{
	struct visitor {
		OpenTTDRustEncodedParameter operator()(const std::monostate &) const { return {0, nullptr, 0, 0}; }
		OpenTTDRustEncodedParameter operator()(uint64_t value) const { return {value, nullptr, 0, 1}; }
		OpenTTDRustEncodedParameter operator()(const std::string &value) const
		{
			return {0, reinterpret_cast<const uint8_t *>(value.data()), value.size(), 2};
		}
	};
	return std::visit(visitor{}, parameter.data);
}

/** Use the same logger routing as StringConsumer without exposing its private helper. */
inline void LogRustEncodedError(std::string &&message)
{
#if defined(STRGEN) || defined(SETTINGSGEN)
	FatalErrorI(std::move(message));
#else
	DebugPrint("misc", 0, std::move(message));
#endif
}

/** Replay one integer diagnostic against its original input. */
inline void LogRustEncodedDiagnostic(const OpenTTDRustEncodedDiagnostic &diagnostic, std::string_view input)
{
	auto part = input.substr(diagnostic.offset, diagnostic.length);
	if (diagnostic.kind == 3) {
		LogRustEncodedError(fmt::format("Integer out of range: '{}'", part));
	} else {
		LogRustEncodedError(fmt::format("{}: '{}'+'{}'", diagnostic.kind == 2 ? "Integer out of range" : "Cannot parse integer",
			part, input.substr(diagnostic.offset + diagnostic.length, diagnostic.tail_length)));
	}
}

/** Preserve the native assertion policy for an encoded result. */
inline void AssertRustEncoded([[maybe_unused]] const OpenTTDRustEncodedView &view, [[maybe_unused]] std::string_view input)
{
#if !defined(NDEBUG) || defined(WITH_ASSERT)
	if (view.assertion_kind == 1) {
		StringConsumer record(input.substr(view.assertion_offset, view.assertion_length));
		assert(!record.AnyBytesLeft());
	}
#endif
#ifdef WITH_ASSERT
	if (view.assertion_kind == 2) {
		size_t len = view.assertion_length;
		char32_t c = view.assertion_codepoint;
		assert(len == 0 || (c != SCC_ENCODED && c != SCC_ENCODED_INTERNAL && c != SCC_RECORD_SEPARATOR));
	}
#endif
}

/** Replay offsets in original input, then copy immutable Rust bytes into a C++ owner. */
inline std::string CopyRustEncoded(const RustEncodedOwner &owner, std::string_view input = {})
{
	auto view = openttd_rust_encoded_view(owner.get());
	for (size_t i = 0; i < view.diagnostic_count; ++i) {
		LogRustEncodedDiagnostic(openttd_rust_encoded_diagnostic(owner.get(), i), input);
	}
	AssertRustEncoded(view, input);
	if (view.length == 0) return {};
	return {reinterpret_cast<const char *>(view.bytes), view.length};
}

#endif /* RUST_ENCODED_ADAPTER_HPP */
