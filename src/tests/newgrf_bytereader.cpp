/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file newgrf_bytereader.cpp Native checked-reader failure and partial-effect behavior. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../core/backup_type.hpp"
#include "../newgrf_badge.h"
#include "../rust/newgrf_bytereader_services.h"
#include "../safeguards.h"

TEST_CASE("Checked NewGRF reader preserves value and cursor failures", "[newgrf][reader]")
{
	const uint8_t data[] = {0x81, 0x23, 0x45, 0x67};
	for (size_t size = 0; size <= std::size(data); ++size) {
		ByteReader byte(data, size), word(data, size), dword(data, size), skip(data, size);
		uint8_t b = 0xFF;
		uint16_t w = 0xFFFF;
		uint32_t d = UINT32_MAX;
		CHECK(RustGRFReadByte(&byte, &b) == (size >= 1));
		CHECK(b == (size >= 1 ? 0x81 : 0xFF));
		CHECK(byte.Remaining() == (size >= 1 ? size - 1 : size));
		CHECK(RustGRFReadWord(&word, &w) == (size >= 2));
		CHECK(w == (size >= 2 ? 0x2381 : 0xFFFF));
		CHECK(word.Remaining() == (size >= 2 ? size - 2 : size));
		CHECK(RustGRFReadDWord(&dword, &d) == (size >= 4));
		CHECK(d == (size >= 4 ? 0x67452381U : UINT32_MAX));
		CHECK(dword.Remaining() == (size >= 4 ? size - 4 : size));
		CHECK(RustGRFSkipBytes(&skip, 3) == (size >= 3));
		CHECK(skip.Remaining() == (size >= 3 ? size - 3 : 0));
	}
	ByteReader property(data, 1);
	uint8_t first = 0, second = 0xFF;
	REQUIRE(RustGRFReadByte(&property, &first));
	CHECK(!RustGRFReadByte(&property, &second));
	CHECK(first == 0x81);
	CHECK(second == 0xFF);
	CHECK_THROWS_AS(RustGRFRethrowReadError(), OTTDByteReaderSignal);
}

TEST_CASE("Checked NewGRF badges retain partial native effects", "[newgrf][reader][badges]")
{
	const BadgeID id = GetOrCreateBadge("test/checked-reader").index;
	Badge *badge = GetBadge(id);
	Backup features(badge->features);
	GRFFile file;
	file.badge_list.push_back(id);
	Backup current(_cur_gps.grffile, &file);
	const std::vector<std::vector<uint8_t>> cases = {
		{}, {0, 0}, {2, 0, 0, 0, 0, 0}, {2, 0, 0, 0, 0}, {2, 0, 0xFF, 0xFF, 0, 0},
	};
	for (const auto &data : cases) {
		const GrfSpecFeatures before = badge->features;
		ByteReader original(data.data(), data.size()), checked(data.data(), data.size());
		const std::vector<BadgeID> sentinel{BadgeID{UINT32_MAX}};
		std::vector<BadgeID> expected = sentinel, actual = sentinel;
		bool success = true;
		try {
			expected = ReadBadgeList(original, GSF_INDUSTRIES);
		} catch (const OTTDByteReaderSignal &) {
			success = false;
		}
		const GrfSpecFeatures after = badge->features;
		badge->features = before;
		CHECK(RustGRFReadBadgeList(&checked, GSF_INDUSTRIES, &actual) == success);
		CHECK(actual == expected);
		CHECK(checked.Remaining() == original.Remaining());
		CHECK(badge->features == after);
		badge->features = before;
		ByteReader skipped(data.data(), data.size()), native_skip(data.data(), data.size());
		success = true;
		try {
			SkipBadgeList(native_skip);
		} catch (const OTTDByteReaderSignal &) {
			success = false;
		}
		CHECK(RustGRFSkipBadgeList(&skipped) == success);
		CHECK(skipped.Remaining() == native_skip.Remaining());
	}
	current.Restore();
	features.Restore();
}
