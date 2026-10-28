/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file history-comparison.cpp Concrete public history mask/phase/typed-execution gaps. */
#include "stdafx.h"
#include "misc/history_func.hpp"
#include "industry.h"
#include "town.h"
#include <iostream>
#include <stdexcept>

#ifndef WITH_RUST
TimerGameEconomy::Month TimerGameEconomy::month = {};
#endif

struct Fatal {};
[[noreturn]] void NOT_REACHED(const std::source_location) { throw Fatal{}; }
[[noreturn]] void AssertFailedError(std::string_view, const std::source_location) { throw Fatal{}; }

template <>
uint16_t SumHistory(std::span<const uint16_t> history)
{
	uint32_t total = std::accumulate(std::begin(history), std::end(history), 0, [](uint32_t r, const uint16_t &value) { return r + value; });
	return ClampTo<uint16_t>(total);
}

static uint next_id = 0;
static bool recording = false;
static uint throw_at = 0;
static uint operation = 0;
static bool changing_phase = false;
static std::vector<std::string> trace;

static void Record(const std::string &event)
{
	if (!recording) return;
	trace.push_back(event);
	++operation;
	if (throw_at != 0 && operation == throw_at) throw std::runtime_error("typed");
}

struct Tracked {
	uint id;
	uint value;
	Tracked() : id(++next_id), value(0)
	{
		Record("new:" + std::to_string(id));
		if (recording && changing_phase) TimerGameEconomy::month = (TimerGameEconomy::month + 1) % 12;
	}
	Tracked(const Tracked &other) : id(++next_id), value(other.value) { Record("ctorcopy:" + std::to_string(id) + ":" + std::to_string(other.id)); }
	Tracked(Tracked &&other) : id(++next_id), value(other.value) { Record("ctormove:" + std::to_string(id) + ":" + std::to_string(other.id)); }
	Tracked &operator=(const Tracked &other)
	{
		Record("copy:" + std::to_string(id) + ":" + std::to_string(other.id));
		value = other.value;
		return *this;
	}
	Tracked &operator=(Tracked &&other)
	{
		Record("move:" + std::to_string(id) + ":" + std::to_string(other.id));
		value = other.value;
		return *this;
	}
	~Tracked()
	{
		if (recording) trace.push_back("drop:" + std::to_string(id));
		if (recording && changing_phase) TimerGameEconomy::month = (TimerGameEconomy::month + 1) % 12;
	}
};

template <>
Tracked SumHistory(std::span<const Tracked> history)
{
	std::string group = "sum";
	for (const auto &value : history) group += ":" + std::to_string(value.id);
	Record(group);
	if (changing_phase) TimerGameEconomy::month = (TimerGameEconomy::month + 1) % 12;
	Tracked result;
	for (const auto &value : history) result.value += value.value;
	result.value /= history.size();
	return result;
}

static void PrintHistory(const auto &history)
{
	for (const auto &value : history) std::cout << ',' << value;
}

static void Scalars()
{
	const HistoryRange wide{40};
	const HistoryRange small{2}, middle{small, 2, 4}, upper{middle, 2, 5}, fourth{upper, 1, 5};
	const HistoryRange one{4}, two{one, 1, 4}, three{two, 1, 4}, four{three, 1, 4}, five{four, 1, 4};
	const std::array<const HistoryRange *, 9> ranges{&HISTORY_MONTH, &HISTORY_QUARTER, &HISTORY_YEAR, &wide, &small, &middle, &fourth, &five, &upper};
	const std::array<ValidHistoryMask, 10> masks{0, 1, 2, 4, 8, 0xAAAAAAULL, (1ULL << 24), (1ULL << 41), (1ULL << 35) | (1ULL << 63), UINT64_MAX};
	for (size_t range = 0; range < ranges.size(); ++range) {
		const auto &hr = *ranges[range];
		for (uint month = 0; month < 12; ++month) {
			for (size_t mask_index = 0; mask_index < masks.size(); ++mask_index) {
				HistoryData<uint16_t> history;
				for (uint i = 0; i < history.size(); ++i) history[i] = i * 13 + 7;
				auto mask = masks[mask_index];
				TimerGameEconomy::month = (month + 5) % 12;
				UpdateValidHistory(mask, hr, month);
				std::cout << "update " << range << ' ' << month << ' ' << mask_index << ' ' << mask << '\n';
				/* Exercise both production update-before-rotate and the original no-prerequisite mask. */
				for (uint update_first = 0; update_first < 2; ++update_first) {
					auto rotated = history;
					RotateHistory(rotated, update_first ? mask : masks[mask_index], hr, month);
					std::cout << "rotate " << range << ' ' << month << ' ' << mask_index << ' ' << update_first;
					PrintHistory(rotated);
					std::cout << '\n';
				}
				const std::array<uint, 7> ages{0U, static_cast<uint>(hr.periods / 2), static_cast<uint>(hr.periods - 1), static_cast<uint>(hr.periods), static_cast<uint>(hr.periods + 1), UINT32_MAX, 0x40000000U};
				for (uint age : ages) {
					uint16_t result = 999;
					bool first_valid = IsValidHistory(masks[mask_index], hr, age);
					std::cout << "query " << range << ' ' << month << ' ' << mask_index << ' ' << age << ' ' << first_valid;
					try {
						bool valid = GetHistory(history, masks[mask_index], hr, age, result);
						std::cout << ' ' << valid << ' ' << result;
					} catch (const Fatal &) { std::cout << " fatal " << result; }
					std::cout << '\n';
				}
			}
		}
	}
	/* Multiplication wraps before the age guard, yielding a defined in-range query. */
	HistoryData<uint16_t> history{};
	for (uint i = 0; i < history.size(); ++i) history[i] = i;
	for (uint month = 0; month < 12; ++month) {
		TimerGameEconomy::month = month;
		uint16_t result = 999;
		bool valid = GetHistory(history, UINT64_MAX, HISTORY_QUARTER, 0x55555556U, result);
		std::cout << "wrapped-quarter " << month << ' ' << valid << ' ' << result << '\n';
	}
}

static void Typed()
{
	for (uint rotate = 0; rotate < 2; ++rotate) {
		for (uint phase_changes = 0; phase_changes < 2; ++phase_changes) {
			for (uint alias : {0U, 1U, 25U, 60U}) {
				for (uint throw_point : {0U, 1U, 2U, 4U, 5U, 8U, 16U, 24U, 32U, 50U, 80U}) {
					recording = false;
					changing_phase = false;
					next_id = 0;
					HistoryData<Tracked> history;
					Tracked separate;
					for (uint i = 0; i < history.size(); ++i) history[i].value = i * 5 + 3;
					trace.clear(); operation = 0; throw_at = throw_point; changing_phase = phase_changes; recording = true;
					TimerGameEconomy::month = 0;
					bool valid = false;
					std::string outcome = "ok";
					try {
						if (rotate) RotateHistory(history, UINT64_MAX, HISTORY_YEAR, 0);
						else valid = GetHistory(history, 4, HISTORY_YEAR, 0, alias == 0 ? separate : history[alias]);
					} catch (const std::runtime_error &) { outcome = "throw"; }
					recording = false;
					std::cout << "typed " << rotate << ' ' << phase_changes << ' ' << alias << ' ' << throw_point << ' ' << outcome << ' ' << valid << ' ' << static_cast<uint>(TimerGameEconomy::month) << ' ' << separate.value;
					for (const auto &value : history) std::cout << ',' << value.value;
					for (const auto &event : trace) std::cout << ' ' << event;
					std::cout << '\n';
				}
			}
		}
	}
}

static void Production()
{
	/* Each field uses the actual production specializations extracted verbatim. */
	HistoryData<Industry::ProducedHistory> produced{};
	HistoryData<Industry::AcceptedHistory> accepted{};
	HistoryData<Town::SuppliedHistory> supplied{};
	for (uint i = 0; i < HISTORY_RECORDS; ++i) {
		produced[i] = {.production = static_cast<uint16_t>(i * 37), .transported = static_cast<uint16_t>(i * 13)};
		accepted[i] = {.accepted = static_cast<uint16_t>(i * 11), .waiting = static_cast<uint16_t>(i * 17)};
		supplied[i] = {.production = UINT32_MAX - i * 97, .transported = 0x80000000U + i * 13};
	}
	for (uint month = 0; month < 12; ++month) {
		TimerGameEconomy::month = month;
		for (uint age : {0U, 1U, 4U, 5U, 23U}) {
			Industry::ProducedHistory p;
			Industry::AcceptedHistory a;
			Town::SuppliedHistory t;
			bool vp = GetHistory(produced, 4, HISTORY_YEAR, age, p);
			bool va = GetHistory(accepted, 4, HISTORY_YEAR, age, a);
			bool vt = GetHistory(supplied, 4, HISTORY_YEAR, age, t);
			std::cout << "production " << month << ' ' << age << ' ' << vp << ' ' << p.production << ' ' << p.transported << ' ' << va << ' ' << a.accepted << ' ' << a.waiting << ' ' << vt << ' ' << t.production << ' ' << t.transported << '\n';
		}
		auto p = produced; auto a = accepted; auto t = supplied;
		RotateHistory(p, UINT64_MAX, HISTORY_YEAR, month);
		RotateHistory(a, UINT64_MAX, HISTORY_YEAR, month);
		RotateHistory(t, UINT64_MAX, HISTORY_YEAR, month);
		std::cout << "production-rotate " << month;
		for (uint index : {0U, 1U, 24U, 25U, 41U, 42U, 60U}) std::cout << ' ' << p[index].production << ' ' << p[index].transported << ' ' << a[index].accepted << ' ' << a[index].waiting << ' ' << t[index].production << ' ' << t[index].transported;
		std::cout << '\n';
	}
	produced = {};
	const std::array<uint16_t, 12> values{0, 0, 2, 0, 0, 2, 0, 0, 2, 2, 2, 2};
	for (uint i = 0; i < values.size(); ++i) produced[i + 1].production = values[i];
	TimerGameEconomy::month = 0;
	Industry::ProducedHistory nested;
	GetHistory(produced, UINT64_MAX, HISTORY_YEAR, 0, nested);
	auto flattened = SumHistory<Industry::ProducedHistory>(std::span(produced).subspan(1, 12));
	std::cout << "grouping " << nested.production << ' ' << flattened.production << '\n';
}

struct Filler {
	uint id;
	void Fill(uint index, uint16_t value) { std::cout << ' ' << id << ':' << index << ":F:" << value; }
	void MakeInvalid(uint index) { std::cout << ' ' << id << ':' << index << ":I"; }
	void MakeZero(uint index) { std::cout << ' ' << id << ':' << index << ":Z"; }
};

static void Fillers()
{
	HistoryData<uint16_t> history;
	for (uint i = 0; i < history.size(); ++i) history[i] = i * 7;
	for (uint month = 0; month < 12; ++month) {
		TimerGameEconomy::month = month;
		for (ValidHistoryMask mask : std::array<ValidHistoryMask, 3>{0, 4, UINT64_MAX}) {
			for (uint present = 0; present < 2; ++present) {
				std::cout << "fillers " << month << ' ' << mask << ' ' << present;
				FillFromHistory<6>(present ? &history : nullptr, mask, HISTORY_QUARTER, Filler{1}, Filler{2});
				std::cout << '\n';
			}
		}
	}
}

int main()
{
	Scalars();
	Typed();
	Production();
	Fillers();
}
