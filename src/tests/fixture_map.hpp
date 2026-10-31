/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file fixture_map.hpp Canonical map statics for native protocol fixtures. */
#ifndef TESTS_FIXTURE_MAP_HPP
#define TESTS_FIXTURE_MAP_HPP
#include "../rust/services_ffi.h"

/* Fixture stand-in for the canonical map statics; a zero size selects the
 * callback backend. Shared tables hold its address, so it neither moves nor copies. */
struct OpenTTDFixtureMap {
	std::unique_ptr<TileBaseStorage[]> base;
	std::unique_ptr<TileExtendedStorage[]> extended;
	uint32_t size = 0;
	uint32_t log_x = 0;
	OpenTTDMapStatics statics{&this->base, &this->extended, &this->size, &this->log_x, true};
	explicit OpenTTDFixtureMap(uint32_t log_x, uint32_t size = 0) : base(std::make_unique<TileBaseStorage[]>(size)), extended(std::make_unique<TileExtendedStorage[]>(size)), size(size), log_x(log_x) {}
	void Resize(uint32_t log_x, uint32_t size)
	{
		this->base = std::make_unique<TileBaseStorage[]>(size);
		this->extended = std::make_unique<TileExtendedStorage[]>(size);
		this->size = size;
		this->log_x = log_x;
	}
	OpenTTDFixtureMap(const OpenTTDFixtureMap &) = delete;
	OpenTTDFixtureMap &operator=(const OpenTTDFixtureMap &) = delete;
};

#endif /* TESTS_FIXTURE_MAP_HPP */
