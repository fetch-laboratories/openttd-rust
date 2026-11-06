/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file owned_bytes.h Inline views and assignment of Rust-owned string bytes. */
#ifndef RUST_OWNED_BYTES_H
#define RUST_OWNED_BYTES_H
#include "call_ffi.h"
#include "byte_span_ffi.h"
#include "rust_vector.h"
#include <string_view>
inline std::string_view RustBytesView(const RustVector<uint8_t> &bytes) { return bytes.empty() ? std::string_view{} : std::string_view{reinterpret_cast<const char *>(bytes.items), bytes.count}; }
extern "C" void OPENTTD_RUST_CALL openttd_rust_owned_bytes_assign(RustVector<uint8_t> *, OpenTTDByteSpan);
inline void RustBytesAssign(RustVector<uint8_t> &bytes, std::string_view text) { openttd_rust_owned_bytes_assign(&bytes, {reinterpret_cast<const uint8_t *>(text.data()), text.size()}); }
#endif /* RUST_OWNED_BYTES_H */
