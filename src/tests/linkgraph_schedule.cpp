/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file linkgraph_schedule.cpp Native LGRS partial-state and recursive cleanup lifetime. */
#include "../stdafx.h"
#include "../3rdparty/catch2/catch.hpp"
#include "../rust/linkgraph_schedule_staging.hpp"
#include "../saveload/saveload.h"
#include "../saveload/saveload_filter.h"
#include "../linkgraph/linkgraphschedule.h"
#include "../rust/service_table_check.hpp"
#include "abi-visit-OpenTTDScheduleServices.hpp"
#include "abi_layout.hpp"
TEST_CASE("Link graph schedule - ABI and staging unwind")
{
	ABI_LAYOUT_CHECK;
#include "abi-layout-linkgraph_schedule.inc"
	CHECK(AbiServicesComplete(LinkGraphSchedule::services));
	struct FutureSave : LoadFilter {
		FutureSave() : LoadFilter(nullptr) {}
		size_t Read(uint8_t *buf, size_t len) override {
			const uint8_t header[] = {'O', 'T', 'T', 'N', 0xFF, 0xFF, 0, 0};
			std::copy_n(header, std::min(len, sizeof(header)), buf); return std::min(len, sizeof(header));
		}
	};
	for (bool recursive_null : {false, true}) {
		try {
			ScheduleStaging staging(true);
			staging.schedule = {reinterpret_cast<LinkGraph *>(uintptr_t{1})};
			staging.running = {reinterpret_cast<LinkGraphJob *>(uintptr_t{2})};
			if (recursive_null) {
				// A real SLA_LOAD SlError invokes SlNullPointers -> LGRS::FixPointers
				// before LoadWithFilter catches it; the outer staging is still alive.
				CHECK(LoadWithFilter(std::make_shared<FutureSave>()) == SL_REINIT);
				OpenTTDScheduleQueues q; openttd_rust_schedule_export(&q);
				CHECK((q.schedule_len == 0 && q.running_len == 0));
			}
			throw std::exception();
		} catch (const std::exception &) {}
		OpenTTDScheduleQueues q; openttd_rust_schedule_export(&q);
		REQUIRE((q.schedule_len == (recursive_null ? 0 : 1) && q.running_len == q.schedule_len));
		if (!recursive_null) CHECK((q.schedule[0] == reinterpret_cast<LinkGraph *>(uintptr_t{1}) && q.running[0] == reinterpret_cast<LinkGraphJob *>(uintptr_t{2})));
		CHECK(ScheduleStaging::active == nullptr);
	}
	OpenTTDScheduleQueues empty{}; openttd_rust_schedule_import(&empty);
}
