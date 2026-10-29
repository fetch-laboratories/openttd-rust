/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file clock_native.hpp Native clock invocation and the scheduled-console exception relay. */
#ifndef RUST_CLOCK_NATIVE_HPP
#define RUST_CLOCK_NATIVE_HPP
#include "clock_ffi.h"
#include "../script/script_fatalerror.hpp"
#include <exception>
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
