/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file saveload-codec-console.hpp Candidate codec setup for consoles that include saveload/saveload.cpp. */
#ifndef SAVELOAD_CODEC_CONSOLE_HPP
#define SAVELOAD_CODEC_CONSOLE_HPP
#ifdef WITH_RUST
/* Reset the Rust codec as the reference's `_sl = {}` resets its state. */
[[maybe_unused]] static void ConsoleResetCodec(uint8_t action)
{
	_sl = {};
	openttd_rust_saveload_dumper_close();
	openttd_rust_saveload_reader_close();
	_codec = {};
	_codec.version = reinterpret_cast<const uint16_t *>(&_sl_version);
	_codec.minor_version = &_sl_minor_version;
	_codec.action = action;
}
/* Load from @p filter, as the reference's `ReadBuffer` over it with `_next_offs = 0`. */
[[maybe_unused]] static void ConsoleOpenReader(std::shared_ptr<LoadFilter> filter)
{
	ConsoleResetCodec(SLA_LOAD);
	_sl.read_filter = std::move(filter);
	openttd_rust_saveload_reader_open(_sl.read_filter.get());
}
#endif
#endif /* SAVELOAD_CODEC_CONSOLE_HPP */
