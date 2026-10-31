/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file witness.cpp The one branch-witness switch and writer; counters live in Rust. */
#include "../stdafx.h"
#include <cstdio>
#include <cstdlib>
#include "../3rdparty/fmt/format.h"
#include "../safeguards.h"

extern "C" {
void openttd_rust_witness_enable();
const char *openttd_rust_witness_name(size_t, size_t *);
uint64_t openttd_rust_witness_count(size_t);
}

/* OPENTTD_WITNESS enables every port's counters; the harness requires named
 * branches from $HOME/branch-witnesses.json. Hits never cross into C++. */
static struct WitnessWriter {
	bool enabled = std::getenv("OPENTTD_WITNESS") != nullptr;
	WitnessWriter() { if (this->enabled) openttd_rust_witness_enable(); }
	~WitnessWriter()
	{
		const char *home = std::getenv("HOME");
		if (!this->enabled || home == nullptr) return;
		FILE *file = std::fopen(fmt::format("{}/branch-witnesses.json", home).c_str(), "w");
		if (file == nullptr) return;
		size_t length = 0;
		for (size_t i = 0; const char *name = openttd_rust_witness_name(i, &length); ++i) {
			fmt::print(file, "{}\"{}\":{}", i == 0 ? "{" : ",", std::string_view(name, length), openttd_rust_witness_count(i));
		}
		fmt::print(file, "}}\n");
		std::fclose(file);
	}
} _witness_writer;
