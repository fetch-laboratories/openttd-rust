/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file string_consumer.cpp Implementation of string parsing. */

#include "../stdafx.h"
#include "string_consumer.hpp"

#include "utf8.hpp"
#include "string_builder.hpp"

#include "../string_func.h"

#if defined(STRGEN) || defined(SETTINGSGEN)
#include "../error_func.h"
#else
#include "../debug.h"
#endif

#include "../safeguards.h"

/* static */ const std::string_view StringConsumer::WHITESPACE_NO_NEWLINE = "\t\v\f\r ";
/* static */ const std::string_view StringConsumer::WHITESPACE_OR_NEWLINE = "\t\n\v\f\r ";

/* static */ void StringConsumer::LogError(std::string &&msg)
{
#if defined(STRGEN) || defined(SETTINGSGEN)
	FatalErrorI(std::move(msg));
#else
	DebugPrint("misc", 0, std::move(msg));
#endif
}

std::optional<uint8_t> StringConsumer::PeekUint8() const
{
	auto result = this->PeekBinary(sizeof(uint8_t));
	if (result.length == 0) return std::nullopt;
	return static_cast<uint8_t>(result.value_bits);
}

std::optional<uint16_t> StringConsumer::PeekUint16LE() const
{
	auto result = this->PeekBinary(sizeof(uint16_t));
	if (result.length == 0) return std::nullopt;
	return static_cast<uint16_t>(result.value_bits);
}

std::optional<uint32_t> StringConsumer::PeekUint32LE() const
{
	auto result = this->PeekBinary(sizeof(uint32_t));
	if (result.length == 0) return std::nullopt;
	return static_cast<uint32_t>(result.value_bits);
}

std::optional<uint64_t> StringConsumer::PeekUint64LE() const
{
	auto result = this->PeekBinary(sizeof(uint64_t));
	if (result.length == 0) return std::nullopt;
	return static_cast<uint64_t>(result.value_bits);
}

std::optional<char> StringConsumer::PeekChar() const
{
	auto result = this->PeekUint8();
	if (!result.has_value()) return {};
	return static_cast<char>(*result);
}


std::string_view StringConsumer::PeekClamped(size_type len) const
{
	auto bounds = openttd_rust_consumer_bound(this->src.size(), this->position, len);
	return this->src.substr(this->position, bounds.length);
}

std::string_view StringConsumer::ReadClamped(size_type len)
{
	auto bounds = openttd_rust_consumer_bound(this->src.size(), this->position, len);
	auto result = this->src.substr(this->position, bounds.length);
	if (bounds.shortfall) LogError(fmt::format("Source buffer too short: {} > {}", len, bounds.length));
	this->position = bounds.position; // Commit only after the original logger returns.
	return result;
}

void StringConsumer::SkipClamped(size_type len)
{
	auto bounds = openttd_rust_consumer_bound(this->src.size(), this->position, len);
	if (bounds.shortfall) LogError(fmt::format("Source buffer too short: {} > {}", len, bounds.length));
	this->position = bounds.position; // No cursor change before a fatal logger.
}

StringConsumer::size_type StringConsumer::Find(std::string_view str) const
{
	assert(!str.empty());
	return this->FindBytes(str, 0);
}

StringConsumer::size_type StringConsumer::FindUtf8(char32_t c) const
{
	auto [data, len] = EncodeUtf8(c);
	return this->Find({data, len});
}

StringConsumer::size_type StringConsumer::FindCharIn(std::string_view chars) const
{
	assert(!chars.empty());
	return this->FindBytes(chars, 1);
}

StringConsumer::size_type StringConsumer::FindCharNotIn(std::string_view chars) const
{
	assert(!chars.empty());
	return this->FindBytes(chars, 2);
}

std::string_view StringConsumer::PeekUntil(std::string_view str, SeparatorUsage sep) const
{
	assert(!str.empty());
	auto lengths = this->SeparatorResult(str, sep);
	return this->src.substr(this->position, lengths.result_length);
}

std::string_view StringConsumer::PeekUntilUtf8(char32_t c, SeparatorUsage sep) const
{
	auto [data, len] = EncodeUtf8(c);
	return PeekUntil({data, len}, sep);
}

std::string_view StringConsumer::ReadUntilUtf8(char32_t c, SeparatorUsage sep)
{
	auto [data, len] = EncodeUtf8(c);
	return ReadUntil({data, len}, sep);
}

void StringConsumer::SkipUntilUtf8(char32_t c, SeparatorUsage sep)
{
	auto [data, len] = EncodeUtf8(c);
	return SkipUntil({data, len}, sep);
}

void StringConsumer::SkipIntegerBase(int base)
{
	assert(base == 0 || base == 8 || base == 10 || base == 16);
	auto input = this->GetLeftData();
	this->Skip(openttd_rust_skip_integer(reinterpret_cast<const uint8_t *>(input.data()), input.size(), base));
}
