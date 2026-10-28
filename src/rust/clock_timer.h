/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file clock_timer.h Specialized calendar/economy registration and callback binding. */
#ifndef RUST_CLOCK_TIMER_H
#define RUST_CLOCK_TIMER_H
class TimerGameCalendar;
class TimerGameEconomy;
template <typename T> class BaseTimer;
template <typename T> class IntervalTimer;
template <typename T> class TimeoutTimer;
template <typename T> inline constexpr bool IsRustGameClock = std::is_same_v<T, TimerGameCalendar> || std::is_same_v<T, TimerGameEconomy>;
struct RustClockCallback {
	bool (*invoke)(const void *, void *) noexcept;
	const void *context;
};
struct RustClockBinding {
	RustClockCallback callback{};
	bool *fired = nullptr;
};
struct RustEmptyClockBinding {};
void RustClockRegister(BaseTimer<TimerGameCalendar> &timer) noexcept;
void RustClockRegister(BaseTimer<TimerGameEconomy> &timer) noexcept;
void RustClockUnregister(BaseTimer<TimerGameCalendar> &timer) noexcept;
void RustClockUnregister(BaseTimer<TimerGameEconomy> &timer) noexcept;
void RustClockChangePeriod(BaseTimer<TimerGameCalendar> &timer, uint8_t trigger, uint8_t priority) noexcept;
void RustClockChangePeriod(BaseTimer<TimerGameEconomy> &timer, uint8_t trigger, uint8_t priority) noexcept;
void RustClockBind(BaseTimer<TimerGameCalendar> &timer, RustClockCallback callback, bool *fired) noexcept;
void RustClockBind(BaseTimer<TimerGameEconomy> &timer, RustClockCallback callback, bool *fired) noexcept;
#endif
