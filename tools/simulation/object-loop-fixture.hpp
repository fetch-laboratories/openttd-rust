/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file object-loop-fixture.hpp Canonical record backing for unchanged scheduler observations. */
#pragma once
#include "../../src/object_base.h"

/** The scheduler probe observes dispatch, not pool lifetime. Its object record
 * supplies the same nonanimated, non-HQ state as the displaced policy callback. */
struct ObjectLoopFixture {
	RustPoolCore *pool;
	RustPoolCore *previous;
	void *shell;
	OpenTTDObjectRecord *record;
	ObjectLoopFixture()
	{
		const RustPoolConfig config{.max_size = ObjectID::End().base(), .growth_step = 64, .cache = false, .item_size = 1, .assertions = false};
		const RustPoolNativeOps ops{
			.context = nullptr,
			.allocate = [](void *, size_t size) noexcept -> void * { return std::allocator<uint8_t>{}.allocate(size); },
			.deallocate = [](void *, void *shell, size_t size) noexcept { std::allocator<uint8_t>{}.deallocate(static_cast<uint8_t *>(shell), size); },
			.set_index = [](void *, void *, size_t) noexcept {},
			.destroy = [](void *, void *) noexcept { std::abort(); },
			.post_destructor = [](void *, size_t) noexcept {},
			.assertion = [](void *, bool valid) noexcept { if (!valid) std::abort(); },
		};
		this->pool = openttd_rust_pool_new(&config, &ops);
		this->previous = openttd_rust_object_pool_bind(this->pool);
		this->shell = openttd_rust_pool_allocate(this->pool, 1);
		this->record = openttd_rust_object_record_new(static_cast<Object *>(this->shell), 0);
		this->record->type = OBJECT_TRANSMITTER;
		openttd_rust_pool_bind_record(this->pool, 0, this->record);
	}
	~ObjectLoopFixture()
	{
		openttd_rust_pool_retire_record(this->pool, 0);
		openttd_rust_object_record_delete(this->record);
		openttd_rust_pool_free(this->pool, 1, 0);
		openttd_rust_object_pool_bind(this->previous);
		openttd_rust_pool_drop(this->pool);
	}
};
