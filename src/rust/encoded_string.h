/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file encoded_string.h Native operations over canonical Rust-owned encoded bytes. */
#ifndef RUST_ENCODED_STRING_H
#define RUST_ENCODED_STRING_H
#include "owned_bytes.h"
#include "../strings_type.h"
/** Inline view of the existing vector layout; views expire on assignment/drop.
 * Assignment requires the serialized live Rust-owned destination, and copies aliased bytes first. */
struct RustEncodedString : RustVector<uint8_t> {
	RustEncodedString() = default;
	RustEncodedString(const RustEncodedString &) = delete;
	RustEncodedString &operator=(const EncodedString &text) { auto view = Span(text); openttd_rust_owned_bytes_assign(this, view); return *this; }
	RustEncodedString &operator=(const RustEncodedString &text) { RustBytesAssign(*this, RustBytesView(text)); return *this; }
	void clear() { RustBytesAssign(*this, {}); }
	std::string GetDecodedString() const { return EncodedString::GetDecodedString(RustBytesView(*this)); }
	operator EncodedString() const { return EncodedString(std::string(RustBytesView(*this))); }
	static OpenTTDByteSpan Span(const EncodedString &text) { return {reinterpret_cast<const uint8_t *>(text.string.data()), text.string.size()}; }
};
#endif /* RUST_ENCODED_STRING_H */
