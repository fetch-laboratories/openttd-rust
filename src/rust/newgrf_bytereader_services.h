/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_bytereader_services.h Checked native reads for synchronous Rust property parsers. */
#ifndef RUST_NEWGRF_BYTEREADER_SERVICES_H
#define RUST_NEWGRF_BYTEREADER_SERVICES_H

#include "../newgrf/newgrf_bytereader.h"
#include "../newgrf/newgrf_internal.h"
#include "newgrf_bytereader_ffi.h"

/* Failure preserves the native cursor movement and partial effects, leaving its
 * output untouched. Callers return from Rust before rethrowing in the parser's
 * native exception frame; no ByteReader borrow survives another parser call. */
template <typename Action>
inline bool RustGRFReadChecked(Action &&action) noexcept
{
	try {
		action();
		return true;
	} catch (const OTTDByteReaderSignal &) {
		return false;
	}
}

inline bool RustGRFReadByte(ByteReader *reader, uint8_t *value) noexcept
{
	return RustGRFReadChecked([&] { *value = reader->ReadByte(); });
}

inline bool RustGRFReadWord(ByteReader *reader, uint16_t *value) noexcept
{
	return RustGRFReadChecked([&] { *value = reader->ReadWord(); });
}

inline bool RustGRFReadDWord(ByteReader *reader, uint32_t *value) noexcept
{
	return RustGRFReadChecked([&] { *value = reader->ReadDWord(); });
}

inline bool RustGRFSkipBytes(ByteReader *reader, size_t size) noexcept
{
	return RustGRFReadChecked([&] { reader->Skip(size); });
}

inline bool RustGRFReadBadgeList(ByteReader *reader, GrfSpecFeature feature, std::vector<BadgeID> *badges) noexcept
{
	return RustGRFReadChecked([&] { *badges = ReadBadgeList(*reader, feature); });
}

inline bool RustGRFSkipBadgeList(ByteReader *reader) noexcept
{
	return RustGRFReadChecked([&] { SkipBadgeList(*reader); });
}

inline const OpenTTDByteReaderServices &GetRustByteReaderServices() noexcept
{
	static const OpenTTDByteReaderServices services{
		.read_byte = RustGRFReadByte,
		.read_word = RustGRFReadWord,
		.read_dword = RustGRFReadDWord,
		.skip = RustGRFSkipBytes,
		.read_badges = RustGRFReadBadgeList,
		.skip_badges = RustGRFSkipBadgeList,
	};
	return services;
}

[[noreturn]] inline void RustGRFRethrowReadError()
{
	throw OTTDByteReaderSignal();
}

#endif /* RUST_NEWGRF_BYTEREADER_SERVICES_H */
