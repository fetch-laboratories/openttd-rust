/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file clock_state.h Native subobjects in canonical Rust clock storage. */
#ifndef RUST_CLOCK_STATE_H
#define RUST_CLOCK_STATE_H
#include "../timer/timer_game_common.h"
struct Calendar;
struct Economy;
struct OpenTTDClockState {
	TimerGame<Calendar>::Year calendar_year;
	TimerGame<Calendar>::Date calendar_date;
	TimerGame<Economy>::Year economy_year;
	TimerGame<Economy>::Date economy_date;
	uint32_t days_since_last_month;
	uint16_t calendar_fraction;
	uint16_t calendar_subfraction;
	uint16_t economy_fraction;
	uint8_t calendar_month;
	uint8_t economy_month;
};
extern "C" OpenTTDClockState *openttd_rust_clock_storage();
inline OpenTTDClockState &GetRustClockState() noexcept
{
	/* Start native StrongType lifetimes once, before publishing any inline aliases. */
	static OpenTTDClockState *state = new (openttd_rust_clock_storage()) OpenTTDClockState{};
	return *state;
}
#endif
