#pragma once
#include "rust/clock_ffi.h"
static void ClockLayouts()
{
#include "abi-layout-clocks.inc"
	static_assert(std::is_standard_layout_v<OpenTTDClockState> && std::is_trivially_destructible_v<OpenTTDClockState>);
	static_assert(sizeof(TimerGameCalendar::Year) == sizeof(int32_t) && alignof(TimerGameCalendar::Year) == alignof(int32_t));
	static_assert(sizeof(TimerGameCalendar::Date) == sizeof(int32_t) && alignof(TimerGameCalendar::Date) == alignof(int32_t));
	static_assert(sizeof(TimerGameEconomy::Year) == sizeof(int32_t) && alignof(TimerGameEconomy::Year) == alignof(int32_t));
	static_assert(sizeof(TimerGameEconomy::Date) == sizeof(int32_t) && alignof(TimerGameEconomy::Date) == alignof(int32_t));
}
ABI_PROBE(ClockLayouts);
