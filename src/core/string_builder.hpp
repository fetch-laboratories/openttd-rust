/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file string_builder.hpp Compose strings from textual and binary data.
 */

#ifndef STRING_BUILDER_HPP
#define STRING_BUILDER_HPP

#include <charconv>
#include "../rust/builder_ffi.h"

/** Match std::to_chars integer overload resolution, including implicit conversions. */
class StringBuilderIntegerAdapter {
	template <class T>
	static OpenTTDRustFormattedInteger FormatValue(T value, int base)
	{
		static_assert(sizeof(T) <= sizeof(uint64_t));
		static_assert(sizeof(int) == sizeof(int32_t));
		bool negative = false;
		if constexpr (std::is_signed_v<T>) negative = value < 0;
		return openttd_rust_format_integer(static_cast<uint64_t>(value), negative, base);
	}

public:
	static OpenTTDRustFormattedInteger Format(char value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(signed char value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(unsigned char value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(short value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(unsigned short value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(int value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(unsigned int value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(long value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(unsigned long value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(long long value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(unsigned long long value, int base) { return FormatValue(value, base); }
	static OpenTTDRustFormattedInteger Format(bool, int) = delete;
};

/**
 * Compose data into a string / buffer.
 */
class BaseStringBuilder {
public:
	using size_type = std::string_view::size_type;

	virtual ~BaseStringBuilder() = default;

	/**
	 * Append buffer.
	 */
	virtual void PutBuffer(std::span<const char> str) = 0;

	/**
	 * Append string.
	 */
	void Put(std::string_view str) { this->PutBuffer(str); }

	void PutUint8(uint8_t value);
	void PutSint8(int8_t value);
	void PutUint16LE(uint16_t value);
	void PutSint16LE(int16_t value);
	void PutUint32LE(uint32_t value);
	void PutSint32LE(int32_t value);
	void PutUint64LE(uint64_t value);
	void PutSint64LE(int64_t value);

	void PutChar(char c);
	void PutUtf8(char32_t c);

	/**
	 * Append integer 'value' in given number 'base'.
	 */
	template <class T>
	void PutIntegerBase(T value, int base)
	{
		auto result = StringBuilderIntegerAdapter::Format(value, base);
		if (result.length == 0) return;
		this->PutBuffer({reinterpret_cast<const char *>(result.bytes), result.length});
	}
};

/**
 * Compose data into a growing std::string.
 */
class StringBuilder final : public BaseStringBuilder {
	std::string *dest;
public:
	/**
	 * Construct StringBuilder into destination string.
	 * @note The lifetime of the string must exceed the lifetime of the StringBuilder.
	 */
	StringBuilder(std::string &dest) : dest(&dest) {}

	/**
	 * Check whether any bytes have been written.
	 */
	[[nodiscard]] bool AnyBytesWritten() const noexcept { return !this->dest->empty(); }
	/**
	 * Get number of already written bytes.
	 */
	[[nodiscard]] size_type GetBytesWritten() const noexcept { return this->dest->size(); }
	/**
	 * Get already written data.
	 */
	[[nodiscard]] const std::string &GetWrittenData() const noexcept { return *dest; }
	/**
	 * Get mutable already written data.
	 */
	[[nodiscard]] std::string &GetString() noexcept { return *dest; }

	void PutBuffer(std::span<const char> str) override;

	/**
	 * Append string.
	 */
	StringBuilder& operator+=(std::string_view str)
	{
		this->Put(str);
		return *this;
	}

	using back_insert_iterator = std::back_insert_iterator<std::string>;
	/**
	 * Create a back-insert-iterator.
	 */
	back_insert_iterator back_inserter()
	{
		return back_insert_iterator(*this->dest);
	}
};

#endif /* STRING_BUILDER_HPP */
