/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file encoded_ffi.h Tagged inputs and an opaque Rust-owned immutable output. */
#ifndef RUST_ENCODED_FFI_H
#define RUST_ENCODED_FFI_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Allocated and destroyed only by Rust; C++ must never dereference/delete it. */
typedef struct OpenTTDRustEncodedResult OpenTTDRustEncodedResult;

/** 0 monostate, 1 uint64 bits, 2 borrowed arbitrary bytes; no C++ variant layout. */
typedef struct {
	uint64_t value;
	const uint8_t *bytes;
	size_t length;
	uint8_t kind;
} OpenTTDRustEncodedParameter;

/** Immutable borrowed output, valid until its Rust owner is destroyed. */
typedef struct {
	const uint8_t *bytes;
	size_t length;
	size_t diagnostic_count;
	size_t assertion_offset;
	size_t assertion_length;
	uint32_t assertion_codepoint;
	uint8_t apply; ///< Compatibility fixer: 0 leave original unchanged, 1 replace it.
	uint8_t assertion_kind; ///< 0 none, 1 numeric record remainder, 2 forbidden string prefix.
} OpenTTDRustEncodedView;

/** All offsets refer to the operation's complete original encoded input. */
typedef struct {
	size_t offset;
	size_t length;
	size_t tail_length; ///< Original following preview, <=4, bounded by its record/input.
	uint8_t kind; ///< Integer error: 1 invalid, 2 range, 3 negative hex.
} OpenTTDRustEncodedDiagnostic;

/** Read-only spans may overlap and empty spans allow NULL; parameter arrays may be under-aligned
 * (MSVC i686). Tags are 0/1/2. Rust owns intermediate and output storage; destroy the result once
 * with encoded_destroy, also when C++ copying or logging throws. Keep the encoded input live until
 * ordered diagnostics are replayed: replay them before the first enabled assertion and commit no
 * output after it. Nonempty spans and initialized parameter arrays occupy live immutable allocations
 * of byte size <= PTRDIFF_MAX; getters never mutate or reallocate the returned storage. */
OpenTTDRustEncodedResult *openttd_rust_encoded_serialize(uint32_t id, const OpenTTDRustEncodedParameter *parameters, size_t count, uint8_t string_assertions);
OpenTTDRustEncodedResult *openttd_rust_encoded_legacy(const uint8_t *data, size_t length, uint8_t fix_code);
OpenTTDRustEncodedResult *openttd_rust_encoded_negatives(const uint8_t *data, size_t length);
/** Assertions: numeric is !NDEBUG || WITH_ASSERT; string is WITH_ASSERT only. */
OpenTTDRustEncodedResult *openttd_rust_encoded_replace(const uint8_t *data, size_t length, size_t index, OpenTTDRustEncodedParameter replacement, uint8_t numeric_assertions, uint8_t string_assertions);
OpenTTDRustEncodedView openttd_rust_encoded_view(const OpenTTDRustEncodedResult *owner);
OpenTTDRustEncodedDiagnostic openttd_rust_encoded_diagnostic(const OpenTTDRustEncodedResult *owner, size_t index); ///< index <diagnostic_count.
void openttd_rust_encoded_destroy(OpenTTDRustEncodedResult *owner); ///< NULL is a no-op; no view survives this call.

#ifdef __cplusplus
}
#endif
#endif /* RUST_ENCODED_FFI_H */
