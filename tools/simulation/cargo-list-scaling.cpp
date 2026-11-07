/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file cargo-list-scaling.cpp Bounded witness for lists larger than the play corpus. */
#include "stdafx.h"
#include "rust/cargo_storage_ffi.h"
#include "rust/services_ffi.h"
#include <algorithm>
#include <array>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <list>
#include <vector>

using Clock = std::chrono::steady_clock;
static unsigned reads;
static OpenTTDCargoPacket *Owner(void *p) noexcept { ++reads; return static_cast<OpenTTDCargoPacket *>(p); }
static void Destroy(void *p) noexcept { openttd_rust_cargo_packet_destroy(static_cast<OpenTTDCargoPacket *>(p)); }
static void Require(bool condition) { if (!condition) std::abort(); }
static void Snapshot(void *context, uint16_t, void *shell) noexcept
{
	if (shell != nullptr) static_cast<std::vector<void *> *>(context)->push_back(shell);
}
static std::vector<void *> Snapshot(OpenTTDCargoList *list)
{
	std::vector<void *> result;
	openttd_rust_cargo_list_snapshot(list, &result, Snapshot);
	return result;
}
static OpenTTDCargoPacket *Packet(unsigned age)
{
	auto *packet = openttd_rust_cargo_packet_new(nullptr);
	packet->count = 1; packet->periods_in_transit = age;
	return packet;
}
static double Milliseconds(Clock::time_point start) { return std::chrono::duration<double, std::milli>(Clock::now() - start).count(); }

int main()
{
	OpenTTDCargoStorageServices services{};
	services.packet = Owner;
	services.destroy = Destroy;
	static const std::unique_ptr<TileBaseStorage[]> base;
	static const std::unique_ptr<TileExtendedStorage[]> extended;
	static const uint32_t size = 65536, log = 8;
	static const OpenTTDMapStatics map{&base, &extended, &size, &log, &log, false};
	static const OpenTTDSharedServices shared{.map = &map};
	services.shared = &shared;
	for (unsigned size : {1000U, 4000U, 16000U, 64000U}) {
		auto *vehicle = openttd_rust_cargo_list_new(1);
		for (unsigned i = 0; i < size; ++i) openttd_rust_cargo_list_insert(vehicle, 0, Packet(i));
		OpenTTDCargoListFields fields{};
		fields.count = size;
		fields.cargo_periods_in_transit = uint64_t(size) * (size - 1) / 2;
		fields.action_counts[2] = size;
		openttd_rust_cargo_list_import(vehicle, &fields);
		auto expected = Snapshot(vehicle);
		std::list<void *> original(expected.begin(), expected.end());
		std::array<double, 7> stages, lists;
		for (unsigned round = 0; round < stages.size(); ++round) {
			auto start = Clock::now();
			openttd_rust_cargo_list_stage(vehicle, &services, 0, 0, nullptr, 0, 4, nullptr, 0, nullptr, 0);
			stages[round] = Milliseconds(start);
			Require(Snapshot(vehicle) == expected);
			// The original NoUnload list operations, without packet/service work.
			auto it = original.begin();
			auto deliver = original.end();
			start = Clock::now();
			for (unsigned sum = 0; sum < size; ++sum) {
				auto *packet = *it;
				original.erase(it++);
				original.push_back(packet);
				if (deliver == original.end()) --deliver;
			}
			lists[round] = Milliseconds(start);
		}
		std::sort(stages.begin(), stages.end());
		std::sort(lists.begin(), lists.end());
		reads = 0;
		auto start = Clock::now();
		for (unsigned i = 0; i < 1024; ++i) openttd_rust_cargo_list_append(vehicle, &services, Packet(size - 1), 2);
		auto append = Milliseconds(start);
		Require(reads == 4096 && Snapshot(vehicle) == expected);
		openttd_rust_cargo_list_export(vehicle, &fields);
		Require(fields.count == size + 1024 && fields.action_counts[2] == size + 1024);
		start = Clock::now();
		openttd_rust_cargo_list_move(vehicle, nullptr, &services, 4, size + 1024, 0, 0, nullptr, 0, nullptr, 0, nullptr, 0);
		auto remove = Milliseconds(start);
		Require(Snapshot(vehicle).empty());
		openttd_rust_cargo_list_export(vehicle, &fields);
		Require(fields.count == 0 && fields.action_counts[2] == 0 && fields.cargo_periods_in_transit == 0);
		openttd_rust_cargo_list_destroy(vehicle, &services);

		auto *station = openttd_rust_cargo_list_new(0);
		openttd_rust_cargo_list_insert(station, 0, Packet(size));
		for (unsigned i = 0; i < size; ++i) openttd_rust_cargo_list_insert(station, i + 2, Packet(i));
		fields = {};
		fields.count = size + 1;
		fields.cargo_periods_in_transit = uint64_t(size) * (size - 1) / 2 + size;
		openttd_rust_cargo_list_import(station, &fields);
		expected = Snapshot(station);
		uint16_t last_key = size + 1;
		uint16_t missing_key = 1;
		start = Clock::now();
		for (unsigned i = 0; i < 1024; ++i) {
			Require(openttd_rust_cargo_list_has(station, &last_key, 1) == 1);
			Require(openttd_rust_cargo_list_has(station, &missing_key, 1) == 0);
		}
		auto has = Milliseconds(start);
		openttd_rust_cargo_list_insert(station, UINT16_MAX, nullptr);
		Require(openttd_rust_cargo_list_has(station, nullptr, 0) == 1);
		reads = 0;
		start = Clock::now();
		for (unsigned i = 0; i < 1024; ++i) openttd_rust_cargo_list_append(station, &services, Packet(size), 0);
		auto station_append = Milliseconds(start);
		Require(reads == 4096 && Snapshot(station) == expected);
		vehicle = openttd_rust_cargo_list_new(1);
		uint16_t next = 0;
		reads = 0;
		start = Clock::now();
		Require(openttd_rust_cargo_list_move(station, vehicle, &services, 7, 1025, 0, 0, &next, 1, nullptr, 0, nullptr, 0) == 1025);
		auto load = Milliseconds(start);
		Require(reads == 4);
		Require(Snapshot(vehicle) == std::vector<void *>{expected.front()});
		expected.erase(expected.begin());
		Require(Snapshot(station) == expected);
		openttd_rust_cargo_list_export(station, &fields);
		Require(fields.count == size);
		openttd_rust_cargo_list_destroy(vehicle, &services);
		openttd_rust_cargo_list_destroy(station, &services);
		std::printf("%u stage_ms=%.6f cpp_list_ms=%.6f reverse_remove_ms=%.6f first_hit_1024_ms=%.6f station_first_hit_1024_ms=%.6f keyed_load_ms=%.6f has_2048_ms=%.6f\n", size, stages[3], lists[3], remove, append, station_append, load, has);
	}
}
