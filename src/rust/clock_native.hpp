/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file clock_native.hpp Native clock invocation and the scheduled-console exception relay. */
#ifndef RUST_CLOCK_NATIVE_HPP
#define RUST_CLOCK_NATIVE_HPP
#include "clock_ffi.h"
#include "../script/script_fatalerror.hpp"
#include <exception>
template <auto Elapsed>
inline bool RustClockElapsed(const OpenTTDClockServices &services, [[maybe_unused]] uint32_t delta)
{
	assert(delta == 1);
	std::exception_ptr scheduled_console_exception;
	bool progressed = Elapsed(&services, reinterpret_cast<OpenTTDConsoleException *>(&scheduled_console_exception));
	if (scheduled_console_exception) std::rethrow_exception(scheduled_console_exception);
	return progressed;
}
inline bool RustClockRelayScheduledConsole(void (*run)(), void *exception) noexcept
{
	try {
		run();
		return true;
	} catch (const Script_FatalError &) {
		*static_cast<std::exception_ptr *>(exception) = std::current_exception();
		return false;
	}
}
#endif
