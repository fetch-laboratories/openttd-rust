/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file pool-record-fixture.hpp Town, industry and subsidy accesses in fixtures shared with the pinned original. */
#pragma once
#ifdef WITH_RUST
#define FIXTURE_POOL_RECORD(item) ((item)->Record())
#define FIXTURE_TOWN_CACHE(town) ((town)->Record())
#else
#define FIXTURE_POOL_RECORD(item) (*(item))
#define FIXTURE_TOWN_CACHE(town) ((town)->cache)
#endif
