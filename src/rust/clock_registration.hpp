/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file clock_registration.hpp Typed clock registration after complete timer construction. */
#ifndef RUST_CLOCK_REGISTRATION_HPP
#define RUST_CLOCK_REGISTRATION_HPP
#include "clock_ffi.h"
inline void RustClockRegister(BaseTimer<TimerGameCalendar> &timer) noexcept
{
	assert(openttd_rust_calendar_valid(timer.period));
	openttd_rust_calendar_register(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), &timer.period);
}
inline void RustClockRegister(BaseTimer<TimerGameEconomy> &timer) noexcept
{
	assert(openttd_rust_economy_valid(timer.period));
	openttd_rust_economy_register(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), &timer.period);
}
inline void RustClockUnregister(BaseTimer<TimerGameCalendar> &timer) noexcept
{
	openttd_rust_calendar_unregister(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), timer.period);
}
inline void RustClockUnregister(BaseTimer<TimerGameEconomy> &timer) noexcept
{
	openttd_rust_economy_unregister(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), timer.period);
}
inline void AssertClockPeriod([[maybe_unused]] bool valid) noexcept { assert(valid); }
inline void RustClockChangePeriod(BaseTimer<TimerGameCalendar> &timer, uint8_t trigger, uint8_t priority) noexcept
{
	openttd_rust_calendar_change(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), &timer.period, {static_cast<TimerGameCalendar::Trigger>(trigger), static_cast<TimerGameCalendar::Priority>(priority)}, AssertClockPeriod);
}
inline void RustClockChangePeriod(BaseTimer<TimerGameEconomy> &timer, uint8_t trigger, uint8_t priority) noexcept
{
	openttd_rust_economy_change(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), &timer.period, {static_cast<TimerGameEconomy::Trigger>(trigger), static_cast<TimerGameEconomy::Priority>(priority)}, AssertClockPeriod);
}
inline void RustClockBind(BaseTimer<TimerGameCalendar> &timer, RustClockCallback callback, bool *fired) noexcept
{
	openttd_rust_calendar_bind(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), timer.period, callback, fired);
}
inline void RustClockBind(BaseTimer<TimerGameEconomy> &timer, RustClockCallback callback, bool *fired) noexcept
{
	openttd_rust_economy_bind(reinterpret_cast<OpenTTDNativeClockTimer *>(&timer), timer.period, callback, fired);
}
#endif
