/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file newgrf_bytereader_ffi.h Typed checked operations for Rust feature interpreters. */
#ifndef RUST_NEWGRF_BYTEREADER_FFI_H
#define RUST_NEWGRF_BYTEREADER_FFI_H
#include "service_defaults.hpp"
#include "../newgrf_badge_type.h"
#include "../newgrf.h"
#include "../newgrf/newgrf_bytereader.h"
struct OpenTTDByteReaderServices {
	bool (*has_data)(ByteReader *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(has_data);
	bool (*peek_dword)(ByteReader *, uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(peek_dword);
	bool (*read_bytes)(ByteReader *, size_t, const uint8_t **) noexcept OPENTTD_SERVICE_DEFAULT(read_bytes);
	bool (*read_byte)(ByteReader *, uint8_t *) noexcept OPENTTD_SERVICE_DEFAULT(read_byte);
	bool (*read_word)(ByteReader *, uint16_t *) noexcept OPENTTD_SERVICE_DEFAULT(read_word);
	bool (*read_dword)(ByteReader *, uint32_t *) noexcept OPENTTD_SERVICE_DEFAULT(read_dword);
	bool (*skip)(ByteReader *, size_t) noexcept OPENTTD_SERVICE_DEFAULT(skip);
	bool (*read_badges)(ByteReader *, GrfSpecFeature, std::vector<BadgeID> *) noexcept OPENTTD_SERVICE_DEFAULT(read_badges);
	bool (*skip_badges)(ByteReader *) noexcept OPENTTD_SERVICE_DEFAULT(skip_badges);
};
#endif
