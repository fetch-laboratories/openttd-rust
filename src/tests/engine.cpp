/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

/** @file engine.cpp Native engine owner lifetime and ABI evidence. */
#include "../stdafx.h"
#include "../engine_base.h"
#include "../engine_func.h"
#include "../newgrf.h"
#include <thread>
#ifdef WITH_RUST
#include "../rust/engine_ffi.h"
#include "../rust/engine_loading_ffi.h"
#include "../newgrf/newgrf_internal_vehicle.h"
#include "../rust/type_availability_ffi.h"
#include "abi_layout.hpp"
#endif
#include "../3rdparty/catch2/catch.hpp"
#include "../safeguards.h"

TEST_CASE("Engine lifecycle canonical ABI", "[rust][engine][abi]")
{
#ifdef WITH_RUST
	ABI_LAYOUT_CHECK;
#include "abi-layout-engine.inc"
#include "abi-layout-engine_loading.inc"
#endif
}

TEST_CASE("Engine temporary owners survive relocation and replacement", "[rust][engine][lifetime]")
{
#ifdef WITH_RUST
	std::vector<Engine> staged;
	staged.emplace_back();
	staged[0].age = -123;
	staged[0].preview_company = CompanyID{3};
	staged[0].name = std::string_view("name\0bytes", 10);
	auto *allocation = staged[0].rust_lifecycle.state;
	auto *age = &staged[0].age;
	for (size_t i = 1; i < 1024; ++i) staged.emplace_back();
	REQUIRE(staged[0].rust_lifecycle.state == allocation);
	REQUIRE(&staged[0].age == age);
	REQUIRE(staged[0].age == -123);
	REQUIRE(staged[0].preview_company == CompanyID{3});
	REQUIRE(staged[0].name.View() == std::string_view("name\0bytes", 10));
	/* Placement replacement is used by FixTTOEngines after native staging. */
	staged[0].~Engine();
	::new (&staged[0]) Engine();
	REQUIRE(staged[0].age == 0);
	REQUIRE(staged[0].preview_company == CompanyID::Invalid());
	REQUIRE(staged[0].name.empty());
	/* Canonical aliases immediately see partial decoded writes. */
	staged[0].rust_lifecycle.state->age = INT32_MIN;
	REQUIRE(staged[0].age == INT32_MIN);
	staged.clear();
#endif
}

TEST_CASE("Engine mappings preserve reservation, sorted identity and replacement metadata", "[engine][loading]")
{
	std::array<std::vector<EngineIDMapping>, VEH_COMPANY_END> saved;
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		auto view = _engine_mngr.Mappings(type);
		saved[type].assign(view.begin(), view.end());
	}
	_engine_mngr.ResetToDefaultMapping();
	for (VehicleType type = VEH_BEGIN; type != VEH_COMPANY_END; type++) {
		REQUIRE(_engine_mngr.Mappings(type).size() == _engine_counts[type]);
		REQUIRE(_engine_mngr.GetID(type, 0, INVALID_GRFID) == EngineID{_engine_offsets[type]});
		const auto original = _engine_mngr.GetID(type, 1, INVALID_GRFID);
		REQUIRE(_engine_mngr.UseUnreservedID(type, 1, 17, true) == original);
		REQUIRE(_engine_mngr.GetID(type, 1, 17) == EngineID::Invalid());
		REQUIRE(_engine_mngr.UseUnreservedID(type, 1, 17, false) == original);
		REQUIRE(_engine_mngr.GetID(type, 1, INVALID_GRFID) == EngineID::Invalid());
		REQUIRE(_engine_mngr.GetID(type, 1, 17) == original);
		_engine_mngr.SetID(type, 1, 17, 99, EngineID{500});
		auto map = _engine_mngr.Mappings(type);
		REQUIRE(map.front().grfid == 17);
		REQUIRE(map.front().engine == EngineID{500});
		REQUIRE(map.front().substitute_id == 1);
		_engine_mngr.SetID(type, 300, 12, 44, EngineID{501});
		map = _engine_mngr.Mappings(type);
		REQUIRE(map.front().grfid == 12);
		REQUIRE(map.front().internal_id == 300);
		REQUIRE(map.front().substitute_id == 44);
		REQUIRE(std::ranges::is_sorted(map, std::less{}, EngineIDMappingKeyProjection{}));
	}
	_engine_mngr.ClearMapping();
	for (const auto &map : saved) for (const auto &entry : map) _engine_mngr.SetID(entry.type, entry.internal_id, entry.grfid, entry.substitute_id, entry.engine);
}

#ifdef WITH_RUST
TEST_CASE("Engine loading state crosses serialized game threads and retains partial fields", "[rust][engine][loading]")
{
	_gted.resize(3);
	_gted[EngineID{1}].cargo_allowed = CargoClasses{123};
	_gted[EngineID{1}].railtypelabels.push_back(RAILTYPE_LABEL_RAIL);
	_gted[EngineID{1}].UpdateRefittability(false);
	std::thread game([] {
		REQUIRE(_gted[EngineID{1}].cargo_allowed.base() == 123);
		REQUIRE(_gted[EngineID{1}].refittability == GRFTempEngineData::EMPTY);
		_gted[EngineID{1}].UpdateRefittability(true);
		_gted[EngineID{1}].ctt_include_mask = 7;
	});
	game.join();
	_gted.resize(70);
	REQUIRE(_gted[EngineID{1}].ctt_include_mask == 7);
	REQUIRE(_gted[EngineID{1}].railtypelabels[0] == RAILTYPE_LABEL_RAIL);
	REQUIRE(_gted[EngineID{1}].refittability == GRFTempEngineData::NONEMPTY);
	_gted[EngineID{1}].UpdateRefittability(false);
	REQUIRE(_gted[EngineID{1}].refittability == GRFTempEngineData::NONEMPTY);
	REQUIRE(_gted[EngineID{69}].refittability == GRFTempEngineData::UNSET);
	_gted.clear();
	_gted.resize(3);
	REQUIRE(_gted[EngineID{1}].cargo_allowed.None());
	REQUIRE(_gted[EngineID{1}].railtypelabels.empty());
	REQUIRE(_gted[EngineID{1}].defaultcargo_grf == nullptr);
	_gted.clear();
}
#endif
