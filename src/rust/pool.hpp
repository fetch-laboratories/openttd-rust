/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
/** @file pool.hpp Native item/lifetime operations over canonical Rust pool policy. */
#ifndef RUST_POOL_HPP
#define RUST_POOL_HPP
#include "pool_ffi.h"
#include "../core/pool_type.hpp"
#include "../error_func.h"
#include "../saveload/saveload_error.hpp"

/** PoolBase keeps registration and cleanup-mask order; all mutable metadata is Rust-owned. */
template <class Titem, typename Tindex, size_t Tgrowth_step, PoolType Tpool_type = PoolType::Normal, bool Tcache = false>
requires std::is_base_of_v<PoolIDBase, Tindex>
struct RustPool : PoolBase {
	static constexpr size_t MAX_SIZE = Tindex::End().base();
	const std::string_view name;
	RustPoolCore *const owner;
	const RustPoolDescriptor *const descriptor;

	explicit RustPool(std::string_view name) : PoolBase(Tpool_type), name(name), owner(this->Create()), descriptor(openttd_rust_pool_descriptor(this->owner)) {}
	~RustPool() override { openttd_rust_pool_drop(this->owner); }
	RustPool(const RustPool &) = delete;
	RustPool &operator=(const RustPool &) = delete;

	template <RustPool *Tpool>
	using PoolItem = PoolItemBase<Titem, Tindex, Tpool>;
	template <class T>
	using IterateWrapper = typename Pool<Titem, Tindex, Tgrowth_step, Tpool_type, Tcache>::template IterateWrapper<T>;
	template <class T, class F>
	using IterateWrapperFiltered = typename Pool<Titem, Tindex, Tgrowth_step, Tpool_type, Tcache>::template IterateWrapperFiltered<T, F>;

	size_t GetPoolSize() const { return this->descriptor->first_unused; }
	size_t GetNumItems() const { return this->descriptor->items; }
	bool CleaningPool() const { return this->descriptor->cleaning; }
	Titem *Get(size_t index) const
	{
		assert(index < this->GetPoolSize());
		return static_cast<Titem *>(this->descriptor->slots[index].shell);
	}
	bool IsValidID(size_t index) const { return index < this->GetPoolSize() && this->Get(index) != nullptr; }
	bool CanAllocate(size_t count = 1) { return openttd_rust_pool_can_allocate(this->owner, count) != 0; }
	void CleanPool() override { openttd_rust_pool_clean(this->owner); }

	/** Called by the real record owner; an unpooled shell cannot publish a record. */
	void BindRecord(size_t index, Titem *shell, void *record)
	{
		if (index < this->GetPoolSize() && this->Get(index) == shell) openttd_rust_pool_bind_record(this->owner, index, record);
	}
	/** Index-free binding for pooled and stack/copy shells. No mutation/callback spans this scan.
	 * O(first_unused), including holes; repeated binding/retirement can be quadratic. */
	void BindRecordIfOwned(Titem *shell, void *record)
	{
		for (size_t index = 0; index < this->GetPoolSize(); ++index) {
			if (this->descriptor->slots[index].shell == shell) {
				this->BindRecord(index, shell, record);
				return;
			}
		}
	}
	void RetireRecord(size_t index, Titem *shell, const void *record)
	{
		if (index < this->GetPoolSize() && this->Get(index) == shell && this->descriptor->slots[index].record == record) openttd_rust_pool_retire_record(this->owner, index);
	}
	/** A base constructor can unwind before its record's shell field was initialized. */
	void RetireUnboundRecord(const void *record)
	{
		for (size_t index = 0; index < this->GetPoolSize(); ++index) {
			if (this->descriptor->slots[index].record == record) {
				openttd_rust_pool_retire_record(this->owner, index);
				return;
			}
		}
	}

	void *GetNew(size_t size)
	{
		void *shell = openttd_rust_pool_allocate(this->owner, size);
		if (shell == nullptr) FatalError("{}: no more free items", this->name);
		return shell;
	}
	void *GetNew(size_t size, size_t index)
	{
		void *shell = openttd_rust_pool_allocate_at(this->owner, size, index);
		if (shell == nullptr) {
			if (index >= MAX_SIZE) SlErrorCorruptFmt("{} index {} out of range ({})", this->name, index, MAX_SIZE);
			SlErrorCorruptFmt("{} index {} already in use", this->name, index);
		}
		return shell;
	}
	void FreeItem(size_t size, size_t index) { openttd_rust_pool_free(this->owner, size, index); }

private:
	RustPoolCore *Create()
	{
		const RustPoolConfig config{
			.max_size = MAX_SIZE, .growth_step = Tgrowth_step, .cache = Tcache, .item_size = sizeof(Titem),
#ifdef WITH_ASSERT
			.assertions = true,
#else
			.assertions = false,
#endif
		};
		const RustPoolNativeOps ops{
			.context = this,
			.allocate = [](void *, size_t size) noexcept -> void * { return std::allocator<uint8_t>{}.allocate(size); },
			.deallocate = [](void *, void *shell, size_t size) noexcept { std::allocator<uint8_t>{}.deallocate(static_cast<uint8_t *>(shell), size); },
			.set_index = [](void *, void *shell, size_t index) noexcept { static_cast<Titem *>(shell)->index = Tindex{static_cast<typename Tindex::BaseType>(index)}; },
			.destroy = [](void *, void *shell) noexcept { delete static_cast<Titem *>(shell); },
			.post_destructor = [](void *, size_t index) noexcept { Titem::PostDestructor(index); },
			.assertion = [](void *, [[maybe_unused]] bool condition) noexcept { assert(condition); },
		};
		return openttd_rust_pool_new(&config, &ops);
	}
};
#endif /* RUST_POOL_HPP */
