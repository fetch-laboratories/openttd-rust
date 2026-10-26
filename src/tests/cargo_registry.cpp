/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file cargo_registry.cpp Cargo registry publication across serialized game threads. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../cargotype.h"
#include "../newgrf_cargo.h"
#include <thread>

TEST_CASE("Cargo registry - reset and lookup are process-wide")
{
	std::thread([] { SetupCargoForClimate(LandscapeType::Arctic); }).join();
	CHECK(GetCargoTypeByLabel(CT_FOOD) == 11);
	CHECK(GetClimateDependentCargoTranslationTable()[11] == CT_FOOD);
	CHECK(HasBit(_cargo_mask, 11));
	CHECK(IsDefaultCargo(11));
	std::thread([] { SetupCargoForClimate(LandscapeType::Temperate); }).join();
	CHECK(GetCargoTypeByLabel(CT_FOOD) == INVALID_CARGO);
	CHECK(GetClimateDependentCargoTranslationTable()[11] == CT_INVALID);
	CHECK_FALSE(HasBit(_cargo_mask, 11));
	CHECK_FALSE(IsDefaultCargo(11));
}
