/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file station-spec-fixture.hpp Borrowed spec rows and silent setup for both games. */
#pragma once
#include "base_station_base.h"
#ifdef WITH_RUST
extern "C" void openttd_test_station_specs_resize(OpenTTDStationRecord *, uint8_t road, size_t count);
#endif
template <typename T>
static auto &FixtureSpecList(BaseStation *station) { return GetStationSpecList<T>(station); }
template <typename T>
static SpecMapping<T> &FixtureSpecRow(BaseStation *station, size_t index)
{
	/* The fixture holds the game-state mutex and the underlying allocation is mutable. */
	return const_cast<SpecMapping<T> &>(FixtureSpecList<T>(station)[index]);
}
template <typename T>
static void FixtureSpecResize(BaseStation *station, size_t count)
{
#ifdef WITH_RUST
	openttd_test_station_specs_resize(&station->Record(), std::is_same_v<T, RoadStopSpec>, count);
#else
	FixtureSpecList<T>(station).resize(count);
#endif
}
template <typename T>
struct FixtureSpecBackup {
	BaseStation *station;
	std::vector<SpecMapping<T>> saved;
	explicit FixtureSpecBackup(BaseStation *station) : station(station), saved(FixtureSpecList<T>(station).begin(), FixtureSpecList<T>(station).end()) {}
	~FixtureSpecBackup()
	{
#ifdef WITH_RUST
		FixtureSpecResize<T>(this->station, this->saved.size());
		for (size_t i = 0; i < this->saved.size(); ++i) FixtureSpecRow<T>(this->station, i) = this->saved[i];
#else
		FixtureSpecList<T>(this->station) = this->saved;
#endif
	}
};
