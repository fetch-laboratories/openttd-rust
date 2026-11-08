/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/**
 * @file core-boundary-comparison.cpp C++ calling-boundary observations kept from the batch-U
 * comparison probes retired by #736 (batch A owner). Rust tests carry every Rust-routed
 * observation; these sections observe only fork C++ glue that Rust cannot see. Exactly one
 * CORE_BOUNDARY_* section is compiled per build by tools/core-boundary-comparison.py, against
 * both the unchanged pinned original and the candidate. Expected literals are the pinned
 * original's output (14ec60f248, probes at 305ac20c28). Each section names the boundary whose
 * retirement removes it.
 */
#include "stdafx.h"

#if defined(CORE_BOUNDARY_INTEGER)
/*
 * From integer_probe.cpp; build -O0 -DSTRGEN with src/core/{string_consumer,string_builder,utf8}.cpp.
 * 1. PutIntegerBase overload resolution for char/char16/char32, unsigned short, long long,
 *    unscoped enums, implicit conversions and an overloaded unary plus (fork-modified
 *    StringBuilderIntegerAdapter, src/core/string_builder.hpp). Retires with the last C++
 *    PutIntegerBase caller.
 * 2. STRGEN fatal timing: the logger never returns, so the fork's ReadClamped/SkipClamped and
 *    ReadIntegerBase must log before committing the cursor (6 parser and 10 consumer cases).
 * 3. Peek/Read borrowed views start at the cursor. 2 and 3 retire with the C++ StringConsumer
 *    glue (src/core/string_consumer.{hpp,cpp}).
 */
#include "core/string_consumer.hpp"
#include "core/string_builder.hpp"
#include <bit>
#include <iostream>
#include <sstream>

namespace integer_boundary {

static StringConsumer *logging_consumer = nullptr;
struct Fatal { std::string line; };

static std::string Hex(std::string_view bytes)
{
	std::string result;
	for (unsigned char byte : bytes) {
		result += "0123456789abcdef"[byte >> 4];
		result += "0123456789abcdef"[byte & 15];
	}
	return result;
}

static std::string Unhex(std::string_view hex)
{
	std::string result;
	if (hex == "-") return result;
	for (size_t i = 0; i < hex.size(); i += 2) result += static_cast<char>(std::stoi(std::string(hex.substr(i, 2)), nullptr, 16));
	return result;
}

class RecordingBuilder final : public BaseStringBuilder {
public:
	std::vector<std::string> events;
	void PutBuffer(std::span<const char> bytes) override { this->events.emplace_back(bytes.data(), bytes.size()); }
};

/* The original's DumpBuilder for calls made inside one PutIntegerBase (phase 0). */
template <typename T>
static std::string Format(T value)
{
	RecordingBuilder builder;
	builder.PutIntegerBase(value, 36);
	std::string text = std::to_string(builder.events.size());
	for (const auto &event : builder.events) text += " 1 0 " + std::to_string(event.size()) + ' ' + (event.empty() ? "-" : Hex(event));
	return text;
}

template <typename T>
static std::string FormatBits(uint64_t bits, int base)
{
	using Unsigned = std::make_unsigned_t<T>;
	RecordingBuilder builder;
	builder.PutIntegerBase<T>(std::bit_cast<T>(static_cast<Unsigned>(bits)), base);
	std::string text = std::to_string(builder.events.size());
	for (const auto &event : builder.events) text += " 1 0 " + std::to_string(event.size()) + ' ' + (event.empty() ? "-" : Hex(event));
	return text;
}

static int Check(std::string_view name, const std::string &got, std::string_view want)
{
	if (got == want) return 0;
	std::cout << "integer boundary " << name << ": original " << want << " candidate " << got << '\n';
	return 1;
}

} // namespace integer_boundary

#if defined(STRGEN) || defined(SETTINGSGEN)
[[noreturn]] void FatalErrorI(const std::string &message)
{
	using namespace integer_boundary;
	throw Fatal{"fatal " + std::to_string(logging_consumer->GetBytesRead()) + ' ' + Hex(message)};
}
#endif

int IntegerBoundary()
{
	using namespace integer_boundary;
	int failures = 0;
	enum SignedEnum : int64_t { NegativeEnum = INT64_MIN };
	enum UnsignedEnum : uint64_t { MaximumEnum = UINT64_MAX };
	enum BoolEnum : bool { TrueEnum = true };
	struct ImplicitSigned { operator int() const { return -42; } };
	struct ImplicitUnsigned { operator uint64_t() const { return UINT64_MAX; } };
	struct OverloadedPlus {
		operator int() const { return 42; }
		int operator+() const { return 999; }
	};
	failures += Check("char", FormatBits<char>(0x80, 10), "1 1 0 4 2d313238");
	failures += Check("unsigned-short", FormatBits<unsigned short>(UINT16_MAX, 36), "1 1 0 4 31656b66");
	failures += Check("long-long", FormatBits<long long>(uint64_t{1} << 63, 36), "1 1 0 14 2d3179327030696a333265386538");
	failures += Check("unsigned-long-long", FormatBits<unsigned long long>(UINT64_MAX, 36), "1 1 0 13 33773565313132363473677366");
	failures += Check("char16", FormatBits<char16_t>(UINT16_MAX, 16), "1 1 0 4 66666666");
	failures += Check("char32", FormatBits<char32_t>(UINT32_MAX, 16), "1 1 0 8 6666666666666666");
	failures += Check("signed-enum", Format(NegativeEnum), "1 1 0 14 2d3179327030696a333265386538");
	failures += Check("unsigned-enum", Format(MaximumEnum), "1 1 0 13 33773565313132363473677366");
	failures += Check("bool-enum", Format(TrueEnum), "1 1 0 1 31");
	failures += Check("implicit-signed", Format(ImplicitSigned{}), "1 1 0 3 2d3136");
	failures += Check("implicit-unsigned", Format(ImplicitUnsigned{}), "1 1 0 13 33773565313132363473677366");
	failures += Check("overloaded-plus", Format(OverloadedPlus{}), "1 1 0 2 3136");

	/* Fatal timing: {record, original stdout line}. */
	static const std::pair<std::string_view, std::string_view> api_fatal[] = {
		{"8 1 0 0 2d307866665441494c", "fatal 0 496e7465676572206f7574206f662072616e67653a20272d3078666627"},
		{"8 1 0 0 2d30783130305441494c", "fatal 0 496e7465676572206f7574206f662072616e67653a2027313030272b275441494c27"},
		{"32 1 0 0 30782d", "fatal 0 43616e6e6f7420706172736520696e74656765723a2027272b272d27"},
		{"32 0 10 0 2d31", "fatal 0 43616e6e6f7420706172736520696e74656765723a2027272b272d3127"},
		{"16 1 0 0 307800ff0a", "fatal 0 43616e6e6f7420706172736520696e74656765723a2027272b2700ff0a27"},
		{"64 0 10 0 393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393900ff0a21", "fatal 0 496e7465676572206f7574206f662072616e67653a20273939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939393939272b2700ff0a2127"},
	};
	for (auto [record, want] : api_fatal) {
		std::istringstream input{std::string(record)};
		unsigned width, is_signed, base, clamp;
		std::string hex;
		input >> width >> is_signed >> base >> clamp >> hex;
		std::string bytes = Unhex(hex);
		StringConsumer read{std::string_view(bytes)};
		logging_consumer = &read;
		std::string got = "returned";
		try {
			switch (width * 2 + is_signed) {
				case 17: (void)read.ReadIntegerBase<int8_t>(base, 42, clamp); break;
				case 33: (void)read.ReadIntegerBase<int16_t>(base, 42, clamp); break;
				case 64: (void)read.ReadIntegerBase<uint32_t>(base, 42, clamp); break;
				case 65: (void)read.ReadIntegerBase<int32_t>(base, 42, clamp); break;
				case 128: (void)read.ReadIntegerBase<uint64_t>(base, 42, clamp); break;
				default: std::abort();
			}
		} catch (const Fatal &fatal) { got = fatal.line; }
		failures += Check(record, got, want);
	}
	static const std::pair<std::string_view, std::string_view> consumer_fatal[] = {
		{"1 1 8 4100ff", "fatal 1 536f757263652062756666657220746f6f2073686f72743a2038203e2032"},
		{"2 1 8 4100ff", "fatal 1 536f757263652062756666657220746f6f2073686f72743a2038203e2032"},
		{"3 3 0 414243", "fatal 3 536f757263652062756666657220746f6f2073686f72743a2031203e2030"},
		{"4 2 0 414243", "fatal 2 536f757263652062756666657220746f6f2073686f72743a2032203e2031"},
		{"5 1 0 414243", "fatal 1 536f757263652062756666657220746f6f2073686f72743a2034203e2032"},
		{"6 1 0 414243", "fatal 1 536f757263652062756666657220746f6f2073686f72743a2038203e2032"},
		{"13 0 0 -", "fatal 0 536f757263652062756666657220746f6f2073686f72743a2031203e2030"},
		{"14 0 0 -", "fatal 0 536f757263652062756666657220746f6f2073686f72743a2031203e2030"},
		{"13 3 0 414243", "fatal 3 536f757263652062756666657220746f6f2073686f72743a2031203e2030"},
		{"14 3 0 414243", "fatal 3 536f757263652062756666657220746f6f2073686f72743a2031203e2030"},
	};
	for (auto [record, want] : consumer_fatal) {
		std::istringstream input{std::string(record)};
		unsigned operation;
		size_t offset, requested;
		std::string hex;
		input >> operation >> offset >> requested >> hex;
		std::string bytes = Unhex(hex);
		StringConsumer consumer{bytes.empty() ? std::string_view{} : std::string_view(bytes)};
		logging_consumer = &consumer;
		std::string got = "returned";
		try {
			consumer.Skip(offset);
			switch (operation) {
				case 1: (void)consumer.Read(requested); break;
				case 2: consumer.Skip(requested); break;
				case 3: (void)consumer.ReadUint8(42); break;
				case 4: (void)consumer.ReadUint16LE(42); break;
				case 5: (void)consumer.ReadUint32LE(42); break;
				case 6: (void)consumer.ReadUint64LE(42); break;
				case 13: (void)consumer.ReadUtf8(42); break;
				case 14: consumer.SkipUtf8(); break;
				default: std::abort();
			}
		} catch (const Fatal &fatal) { got = fatal.line; }
		failures += Check(record, got, want);
	}

	/* Borrowed-view identity: every original Peek/Read case returned a view at the cursor. */
	for (unsigned operation : {0U, 1U}) {
		for (auto [offset, requested] : std::initializer_list<std::pair<size_t, size_t>>{{1, 0}, {3, 0}, {1, 1}, {1, 2}}) {
			std::string bytes("A\0\xff", 3);
			StringConsumer consumer{std::string_view(bytes)};
			logging_consumer = &consumer;
			consumer.Skip(offset);
			auto view = operation == 0 ? consumer.Peek(requested) : consumer.Read(requested);
			failures += Check("view at cursor", std::to_string(view.data() == bytes.data() + offset), "1");
		}
	}
	return failures;
}

int main()
{
	int failures = IntegerBoundary();
	std::cout << "Integer boundary failures: " << failures << '\n';
	return failures != 0;
}
#endif /* CORE_BOUNDARY_INTEGER */

#if defined(CORE_BOUNDARY_UTF8)
/*
 * From utf8-comparison.cpp; build -O2 with src/core/{utf8,string_consumer,string_builder,
 * string_inplace}.cpp (src/string.cpp is included for its static StrMakeValid builder template).
 * 1. StrMakeValid's fork-modified loop (src/string.cpp) commits consumer progress before each
 *    C++ builder call, propagates a builder bad_alloc and tolerates builder reentry. Retires
 *    with the last C++ StrMakeValid builder.
 * 2. The fork's inline DecodeUtf8 ASCII shortcut (src/core/utf8.hpp) never calls Rust. Retires
 *    with the C++ DecodeUtf8 facade.
 * 3. InPlaceBuilder::PutBuffer's fork glue turns a Rust overtake refusal into NOT_REACHED with
 *    unchanged state. Retires with the C++ InPlaceBuilder.
 */
#include "core/utf8.hpp"
#include "core/string_consumer.hpp"
#include <cstdlib>
#include <iostream>
#include <stdexcept>

void DebugPrint(std::string_view, int, std::string &&) { std::abort(); }

static bool fail_cpp_allocation;
void *operator new(size_t size)
{
	if (fail_cpp_allocation) throw std::bad_alloc{};
	if (void *result = std::malloc(size == 0 ? 1 : size)) return result;
	throw std::bad_alloc{};
}
void operator delete(void *pointer) noexcept { std::free(pointer); }
void operator delete(void *pointer, size_t) noexcept { std::free(pointer); }
[[noreturn]] void NOT_REACHED(const std::source_location) { throw std::runtime_error("NOT_REACHED"); }
#include "string.cpp" // The static StrMakeValid builder template.

static int failures = 0, checks = 0;
static void Check(bool ok, std::string_view what)
{
	++checks;
	if (!ok) { ++failures; std::cout << "FAIL " << what << '\n'; }
}

struct ObservedBuilder : BaseStringBuilder {
	StringConsumer &consumer;
	std::string output;
	std::array<size_t, 100> progress{};
	size_t calls = 0;
	bool reenter = false;
	explicit ObservedBuilder(StringConsumer &consumer) : consumer(consumer) {}
	void PutBuffer(std::span<const char> bytes) override
	{
		progress[calls++] = this->consumer.GetBytesRead();
		if (reenter) { reenter = false; auto nested = StrMakeValid("inner"sv); if (nested != "inner") std::abort(); this->consumer.Skip(1); }
		output.append(bytes.data(), bytes.size());
	}
};

static void AppendProgress()
{
	/* Original rows: fail reenter threw read output calls, then the first-call progress values. */
	struct Row { bool fail, reenter, threw; size_t read, output, calls; size_t first, second, last; };
	static constexpr Row rows[] = {
		{false, false, false, 40, 40, 40, 1, 2, 40},
		{false, true, false, 40, 39, 39, 1, 3, 40},
		{true, false, true, 16, 15, 16, 1, 2, 16},
		{true, true, true, 17, 15, 16, 1, 3, 17},
	};
	for (const auto &row : rows) {
		std::string input(40, 'A');
		StringConsumer consumer(input);
		ObservedBuilder builder(consumer);
		builder.reenter = row.reenter;
		bool threw = false;
		fail_cpp_allocation = row.fail;
		try { StrMakeValid(builder, consumer, StringValidationSetting::ReplaceWithQuestionMark); } catch (const std::bad_alloc &) { threw = true; }
		fail_cpp_allocation = false;
		bool ok = threw == row.threw && consumer.GetBytesRead() == row.read && builder.output.size() == row.output &&
			builder.calls == row.calls && builder.progress[0] == row.first && builder.progress[1] == row.second &&
			builder.progress[row.calls - 1] == row.last;
		/* Every call between the second and the last advances progress by exactly one byte. */
		for (size_t call = 2; call < builder.calls; ++call) ok &= builder.progress[call] == builder.progress[call - 1] + 1;
		Check(ok, "append-progress");
	}
}

static void AsciiDecode()
{
	for (unsigned byte = 0; byte < 0x80; ++byte) {
		const char bytes[2] = {static_cast<char>(byte), static_cast<char>(0xFF)};
		auto [length, codepoint] = DecodeUtf8({bytes, 2});
		Check(length == 1 && codepoint == byte, "ascii-decode");
	}
}

static void Overtake()
{
	std::array<char, 4> buffer{'a', 'b', 'c', 'd'};
	InPlaceReplacement pair(buffer);
	bool threw = false;
	try { pair.builder.PutChar('X'); } catch (const std::runtime_error &) { threw = true; }
	Check(threw && pair.consumer.GetBytesRead() == 0 && pair.builder.GetBytesWritten() == 0 &&
		std::string_view(buffer.data(), buffer.size()) == "abcd", "overtake");
}

int main()
{
	AppendProgress();
	AsciiDecode();
	Overtake();
	std::cout << "UTF-8 boundary checks: " << checks << ", failures: " << failures << '\n';
	return failures != 0;
}
#endif /* CORE_BOUNDARY_UTF8 */

#if defined(CORE_BOUNDARY_ENCODED)
/*
 * From encoded-comparison.cpp; build -O0 with the scoped original GetEncodedStringWithArgs,
 * EncodedString::ReplaceParam, FixSCCEncoded and FixSCCEncodedNegative bodies and
 * src/core/{string_builder,string_consumer,utf8}.cpp, once per assertion mode (none,
 * -DWITH_ASSERT, -DNDEBUG, -DNDEBUG -DWITH_ASSERT). The driver compares each mode's exit,
 * stdout and normalized stderr between the original and the candidate.
 * 1. The fork adapter (src/rust/encoded_adapter.hpp) replays Rust diagnostics through the
 *    native logger, and a throwing logger leaves FixSCCEncodedNegative's input uncommitted
 *    (--logs).
 * 2. Assertion dispatch per compile mode through AssertRustEncoded (--assert-numeric,
 *    --assert-string).
 * 3. Candidate only: a C++ bad_alloc while copying Rust output still destroys the Rust owner
 *    (--copy-throw, LeakSanitizer build).
 * All retire with the C++ EncodedString facades and native logger/assert handlers.
 */
#include "strings_func.h"
#include "misc/endian_buffer.hpp"
#ifdef WITH_RUST
#include "rust/encoded_adapter.hpp"
#endif
#include <iostream>
#include <stdexcept>
#include <utility>

extern void FixSCCEncodedNegative(std::string &str);
static bool throw_on_log = false;
static bool fail_next_allocation = false;
static std::string logged;

void *operator new(size_t size)
{
	if (std::exchange(fail_next_allocation, false)) throw std::bad_alloc();
	if (void *result = std::malloc(size == 0 ? 1 : size)) return result;
	throw std::bad_alloc();
}
void operator delete(void *pointer) noexcept { std::free(pointer); }
void operator delete(void *pointer, size_t) noexcept { std::free(pointer); }

static std::string Hex(std::string_view value)
{
	std::string result;
	for (unsigned char byte : value) {
		result += "0123456789abcdef"[byte >> 4];
		result += "0123456789abcdef"[byte & 15];
	}
	return result.empty() ? "-" : result;
}

void DebugPrint(std::string_view, int, std::string &&message)
{
	logged += message + '\n';
	if (throw_on_log) throw std::runtime_error("injected logger exception");
}

[[noreturn]] void AssertFailedError(std::string_view expression, const std::source_location)
{
	std::cout << "assert " << Hex(expression) << '\n' << std::flush;
	std::exit(3);
}

/** The probe's EncodedString round trip; the writer's final terminator is dropped. */
static std::string Bytes(const EncodedString &value)
{
	auto bytes = EndianBufferWriter<>::FromValue(value);
	bytes.pop_back();
	return {reinterpret_cast<const char *>(bytes.data()), bytes.size()};
}

static EncodedString FromBytes(std::string_view value)
{
	std::vector<uint8_t> buffer(value.begin(), value.end());
	buffer.push_back(0);
	return EndianBufferReader::ToValue<EncodedString>(buffer);
}

/* SCC_ENCODED, SCC_ENCODED_INTERNAL, SCC_ENCODED_NUMERIC and SCC_ENCODED_STRING. */
static const std::string ENCODED = "\xEE\x80\x80", INTERNAL = "\xEE\x80\x81", NUMERIC = "\xEE\x80\x82", STRING = "\xEE\x80\x83";

static int Logs()
{
	int failures = 0;
	auto check = [&](std::string input, std::string_view log, std::string_view output) {
		logged.clear();
		FixSCCEncodedNegative(input);
		if (logged != log || input != output) {
			std::cout << "FAIL log " << Hex(log) << '\n';
			++failures;
		}
	};
	check(ENCODED + "0\x1e" + NUMERIC + "-8000000000000001", "Integer out of range: '-8000000000000001'+''\n", ENCODED + "0\x1e" + NUMERIC + "0");
	check(ENCODED + "0\x1e" + NUMERIC, "Cannot parse integer: ''+''\n", ENCODED + "0\x1e" + NUMERIC + "0");
	/* A throwing logger propagates before the facade commits: the input stays unchanged. */
	std::string input = ENCODED + "0\x1e" + NUMERIC + "-xyz";
	const std::string original = input;
	throw_on_log = true;
	bool threw = false;
	logged.clear();
	try { FixSCCEncodedNegative(input); } catch (const std::runtime_error &) { threw = true; }
	throw_on_log = false;
	if (!threw || input != original || logged != "Cannot parse integer: ''+'-xyz'\n") {
		std::cout << "FAIL logger-throw\n";
		++failures;
	}
	std::cout << "Encoded boundary log checks: 3, failures: " << failures << '\n';
	return failures != 0;
}

int main(int argc, char **argv)
{
	std::string_view mode = argc == 2 ? argv[1] : "";
	if (mode == "--logs") return Logs();
	if (mode == "--assert-numeric") {
		/* An internal record whose numeric parameter has trailing text. */
		auto encoded = FromBytes(INTERNAL + "000A\x1e" + NUMERIC + "2fTAIL\x1e" + STRING + "last");
		auto output = Bytes(encoded.ReplaceParam(0, std::monostate{})); // Before any stdout.
		std::cout << "out " << Hex(output) << '\n';
		return 0;
	}
	if (mode == "--assert-string") {
		/* A string parameter beginning with SCC_ENCODED. */
		std::vector<StringParameter> parameters{ENCODED};
		auto output = Bytes(GetEncodedStringWithArgs(0, parameters)); // Before any stdout.
		std::cout << "out " << Hex(output) << '\n';
		return 0;
	}
#ifdef WITH_RUST
	if (mode == "--copy-throw") {
		std::string input = ENCODED + std::string(200, 'x');
		try {
			auto owner = OwnRustEncoded(openttd_rust_encoded_legacy(reinterpret_cast<const uint8_t *>(input.data()), input.size(), 0));
			fail_next_allocation = true;
			auto output = CopyRustEncoded(owner, input);
			return 1; // A successful copy would miss the intended exceptional path.
		} catch (const std::bad_alloc &) {
			std::cout << "copy-exception " << Hex(input) << '\n';
			return 0; // LeakSanitizer checks the destroyed Rust owner on process exit.
		}
	}
#endif
	return 2;
}
#endif /* CORE_BOUNDARY_ENCODED */

#if defined(CORE_BOUNDARY_MATH)
/*
 * From math-comparison.cpp; build -O2 with src/core/math_func.cpp; the candidate links with
 * -Wl,--wrap=openttd_rust_int_sqrt -Wl,--wrap=openttd_rust_soft_clamp (GNU ld only).
 * 1. Four SoftClamp static_asserts: constant evaluation in the fork-modified SoftClamp keeps
 *    the original body. The other eight probe static_asserts tested the unchanged ClampTo
 *    template and were deleted (#736 U).
 * 2. Runtime IntSqrt/SoftClamp cross the Rust ABI exactly once each under -O2.
 * Both retire with the C++ SoftClamp/IntSqrt facades in src/core/math_func.{hpp,cpp}.
 */
#include "core/math_func.hpp"

/* Fork-modified SoftClamp: constant evaluation keeps the original body (values from the
 * pinned original's static_asserts in math-comparison.cpp). */
static_assert(SoftClamp(0, 1500000000, -1500000000) == 0);
static_assert(SoftClamp<int8_t>(0, -1, -3) == 126);
static_assert(SoftClamp<int16_t>(0, -1, -3) == 32766);
static_assert(SoftClamp<uint64_t>(0, UINT64_MAX, 0) == (uint64_t(1) << 63));

#ifdef WITH_RUST
/* Runtime calls must cross the Rust ABI even under -O2: one call each. */
static uint64_t sqrt_calls = 0, soft_calls = 0;
extern "C" uint32_t __real_openttd_rust_int_sqrt(uint32_t);
extern "C" uint64_t __real_openttd_rust_soft_clamp(uint64_t, uint64_t, uint64_t, uint8_t, uint8_t);
extern "C" uint32_t __wrap_openttd_rust_int_sqrt(uint32_t value)
{
	sqrt_calls++;
	return __real_openttd_rust_int_sqrt(value);
}
extern "C" uint64_t __wrap_openttd_rust_soft_clamp(uint64_t value, uint64_t min, uint64_t max, uint8_t width, uint8_t sign)
{
	soft_calls++;
	return __real_openttd_rust_soft_clamp(value, min, max, width, sign);
}

#endif /* WITH_RUST */

int main()
{
	/* Original results: 5 and 1250 (math-comparison stream cases 0 and 2). */
	volatile uint32_t square = 25;
	volatile int a = 0, low = 1500, high = 1000;
	if (IntSqrt(square) != 5 || SoftClamp<int>(a, low, high) != 1250) return 1;
#ifdef WITH_RUST
	return sqrt_calls == 1 && soft_calls == 1 ? 0 : 2;
#else
	return 0;
#endif
}
#endif /* CORE_BOUNDARY_MATH */

#if defined(CORE_BOUNDARY_BYTES)
/*
 * From byte-strings-comparison.cpp; build -O2 with src/string.cpp and
 * src/core/string_consumer.cpp (64-bit size_t).
 * The fork's RustByteCase glue in src/string.cpp computes StrCompareIgnoreCase's length order
 * with string_view::compare, clamping a size difference beyond int. Retires with that C++
 * glue (the last C++ StrCompareIgnoreCase caller).
 */
#include "string_func.h"
#include <climits>
#include <sys/mman.h>

int ByteStringsBoundary()
{
	if constexpr (sizeof(size_t) > sizeof(int)) {
		size_t length = static_cast<size_t>(INT_MAX) + 17;
		/* Real anonymous readable zero-filled storage; comparisons fault in no pages. */
		void *allocation = mmap(nullptr, length, PROT_READ, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
		if (allocation == MAP_FAILED) return 0; // Unavailable, as the original probe printed.
		std::string_view view(static_cast<const char *>(allocation), length), empty, one = view.substr(0, 1);
		/* Original: wide 2147483647 -2147483648 2147483647 -2147483648 0 */
		bool same = StrCompareIgnoreCase(view, empty) == INT_MAX && StrCompareIgnoreCase(empty, view) == INT_MIN
			&& StrCompareIgnoreCase(view, one) == INT_MAX && StrCompareIgnoreCase(one, view) == INT_MIN
			&& StrCompareIgnoreCase(empty, empty) == 0;
		munmap(allocation, length);
		return same ? 0 : 1;
	}
	return 0;
}

int main()
{
	return ByteStringsBoundary();
}
#endif /* CORE_BOUNDARY_BYTES */
