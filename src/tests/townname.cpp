/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file townname.cpp Focused built-in rendering and builder append tests. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../strings_func.h"
#include "../strings_internal.h"
#include "../safeguards.h"

TEST_CASE("Town names - builder append and company formatting")
{
	/* townname::generator_tests carries every generator's names (#736 W). This keeps the C++
	 * GenerateTownNameString append and the company-name dispatch around them, at the first and
	 * last generator; the fixture's names come from the unchanged pinned generators. */
	static constexpr std::pair<size_t, std::array<std::string_view, 2>> expected[] = {
		{0, {"Invenville", "Fort Wenfingburg Springs"}},
		{20, {"Barcelona", "Vilafranca del Pened\u00e8s"}},
	};
	/* Existing bytes include curse/vowel text, UTF-8 and NUL: output starts after them. */
	const std::string prefix("Cunt aou \u00e4\0", 12);
	for (const auto &[lang, names] : expected) {
		for (size_t endpoint = 0; endpoint < 2; endpoint++) {
			const uint32_t seed = endpoint == 0 ? 0 : UINT32_MAX;
			std::string result = prefix;
			StringBuilder builder(result);
			GenerateTownNameString(builder, lang, seed);
			GenerateTownNameString(builder, lang, seed);
			CHECK(result == prefix + std::string(names[endpoint]) + std::string(names[endpoint]));
			result = prefix;
			auto params = MakeParameters(seed);
			GetStringWithArgs(builder, SPECSTR_COMPANY_NAME_START + static_cast<StringID>(lang), params);
			CHECK(result == prefix + std::string(names[endpoint]) + " Transport");
		}
	}
	/* Finnish scans only the newly appended stem; a prefix vowel must not force la. */
	std::string result = prefix;
	StringBuilder builder(result);
	GenerateTownNameString(builder, 8, 3797316664U);
	CHECK(result == prefix + "Hein\u00e4l\u00e4");
}
