/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file utf8.cpp Handling of UTF-8 encoded data.
 */

#include "../stdafx.h"
#include "utf8.hpp"
#include "../rust/utf8_ffi.h"
#include "../safeguards.h"

/**
 * Encode a character to UTF-8.
 * @param c Character
 * @return Binary data and length. Length is zero, if "c" is out of range.
 */
[[nodiscard]] std::pair<char[4], size_t> EncodeUtf8(char32_t c)
{
	static_assert(sizeof(char32_t) == sizeof(uint32_t));
	auto encoded = openttd_rust_encode_utf8(static_cast<uint32_t>(c));
	std::pair<char[4], size_t> result{};
	std::copy(std::begin(encoded.bytes), std::end(encoded.bytes), std::begin(result.first));
	result.second = encoded.length;
	return result;
}

/**
 * Decode a character from UTF-8.
 * @param buf Binary data.
 * @return Length and character. Length is zero, if the input data is invalid.
 */

/**
 * Create iterator pointing at codepoint, which occupies the byte position "offset".
 * "offset" does not need to point at the first byte of the UTF-8 sequence,
 * the iterator will still address the correct position of the first byte.
 * @param offset Byte offset into view.
 * @return Iterator pointing at start of codepoint, of which "offset" is part of.
 */
Utf8View::iterator Utf8View::GetIterAtByte(size_t offset) const
{
	assert(offset <= this->src.size());
	return iterator(this->src, openttd_rust_utf8_at_byte(reinterpret_cast<const uint8_t *>(this->src.data()), this->src.size(), offset));
}

/* Check if the given character is part of a UTF8 sequence. */
bool IsUtf8Part(char c)
{
	return openttd_rust_is_utf8_part(static_cast<uint8_t>(c)) != 0;
}

Utf8View::iterator& Utf8View::iterator::operator++()
{
	auto size = this->src.size();
	assert(this->position < size);
	this->position = openttd_rust_utf8_next(reinterpret_cast<const uint8_t *>(this->src.data()), size, this->position);
	return *this;
}

Utf8View::iterator& Utf8View::iterator::operator--()
{
	assert(this->position > 0);
	this->position = openttd_rust_utf8_previous(reinterpret_cast<const uint8_t *>(this->src.data()), this->src.size(), this->position);
	return *this;
}
