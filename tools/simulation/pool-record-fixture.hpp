/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file pool-record-fixture.hpp Town, industry and subsidy accesses in fixtures shared with the pinned original. */
#pragma once
#ifdef WITH_RUST
#define FIXTURE_POOL_RECORD(item) ((item)->Record())
#define FIXTURE_TOWN_CACHE(town) ((town)->Record())
#define FIXTURE_STATIONS_NEAR(item) ((item)->StationsNear())
#define FIXTURE_TOWN_SUPPLIED(item) ((item)->Supplied())
#define FIXTURE_TOWN_ID_COUNTS(item) ((item)->Record().id_counts)
#define FIXTURE_TOWN_CLASS_COUNTS(item) ((item)->Record().class_counts)
#else
#define FIXTURE_POOL_RECORD(item) (*(item))
#define FIXTURE_TOWN_CACHE(town) ((town)->cache)
#define FIXTURE_STATIONS_NEAR(item) ((item)->stations_near)
#define FIXTURE_TOWN_SUPPLIED(item) ((item)->supplied)
#define FIXTURE_TOWN_ID_COUNTS(item) ((item)->cache.building_counts.id_count)
#define FIXTURE_TOWN_CLASS_COUNTS(item) ((item)->cache.building_counts.class_count)
#endif
