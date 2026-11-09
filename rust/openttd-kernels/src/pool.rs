/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `core/pool_func.hpp` policy. Native shells retain their allocator, constructors,
//! index layout and destructor chain. Records bind at owner initialization, not allocation.
//!
//! # Safety
//! All raw operations require a stable live owner and the game-state mutex/startup
//! contract. No pool, buffer, shell or record borrow may span a callback. Callbacks
//! cannot unwind; destroy must retire the record and call `free_item` through sized
//! delete. Allocation/deallocation/index leaves cannot mutate this pool. Post hooks
//! and destructors may reenter; buffer pointers expire on growth/cleaning. Dropping
//! the owner releases buffers only, without implicitly deleting native shells.
#![allow(unsafe_code)]
#![allow(clippy::missing_safety_doc)] // Raw entry points share the module's access contract.

use crate::bitmath_func::{clr_bit_size, find_first_bit, set_bit_size};
use crate::industrytype::NativeVec;
use crate::math::align;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::null_mut;

crate::abi::shared_records! {
    fn abi_record;
    /// Shell and record have independent construction/destruction observation points.
    #[derive(Clone, Copy)]
    "RustPoolSlot" ["pool"] => pub struct PoolSlot {
        /// Native shell, published before construction and retained through destruction.
        pub shell: *mut c_void,
        /// Rust record, published and retired by its actual owner.
        pub record: *mut c_void,
    }
    /// Intrusive link in dead, natively allocated shell bytes.
    "RustPoolCacheLink" ["pool"] => pub struct PoolCacheLink {
        /// Previous cache head; stored only after shell destruction.
        pub next: *mut PoolCacheLink,
    }
    /// Canonical storage; its address stays stable until the owner is dropped.
    "RustPoolDescriptor" ["pool"] => pub struct PoolDescriptor {
        /// Initialized slots, with canonical buffer pointer, length and capacity.
        pub slots: NativeVec<PoolSlot> => "RustVector<RustPoolSlot>",
        /// Native-size occupancy words, including occupied tail padding.
        pub used_bitmap: NativeVec<usize> => "RustVector<std::size_t>",
        /// Ordinary search starts at the word containing this coordinate.
        pub first_free: usize,
        /// Live extent; freeing a slot never decreases it.
        pub first_unused: usize,
        /// Number of occupied shell slots.
        pub items: usize,
        /// `WITH_ASSERT` allocation credit; untouched when assertions are disabled.
        pub checked: usize,
        /// Suppresses bitmap clearing and post hooks during cleanup.
        pub cleaning: bool,
        /// LIFO list of dead native shell allocations.
        pub alloc_cache: *mut PoolCacheLink,
    }
    /// Native leaves, copied before callbacks so no Rust owner borrow survives reentry.
    #[derive(Clone, Copy)]
    "RustPoolNativeOps" ["pool"] => pub struct NativeOps {
        /// Stable native binding context; not a pool metadata mirror.
        pub context: *mut c_void,
        /// `std::allocator<uint8_t>::allocate(size)`; does not zero bytes.
        pub allocate: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
        /// Matching native byte allocator, with the original size.
        pub deallocate: unsafe extern "C" fn(*mut c_void, *mut c_void, usize),
        /// Write `PoolItem::index` through its actual native layout.
        pub set_index: unsafe extern "C" fn(*mut c_void, *mut c_void, usize),
        /// `delete Titem *`, including record teardown and sized delete.
        pub destroy: unsafe extern "C" fn(*mut c_void, *mut c_void),
        /// `Titem::PostDestructor(index)`; may reuse this index immediately.
        pub post_destructor: unsafe extern "C" fn(*mut c_void, usize),
        /// Native assertion, called only on failure with native assertions enabled.
        pub assertion: unsafe extern "C" fn(*mut c_void, bool),
    }

    /// Immutable native instantiation parameters.
    #[derive(Clone, Copy)]
    "RustPoolConfig" ["pool"] => pub struct PoolConfig {
        /// `Tindex::End().base()`.
        pub max_size: usize,
        /// Power-of-two `Tgrowth_step` required by original `Align`.
        pub growth_step: usize,
        /// `Tcache`.
        pub cache: bool,
        /// `sizeof(Titem)` for cached reuse/drain, not a derived allocation size.
        pub item_size: usize,
        /// Native `WITH_ASSERT`, independent of Rust build profile.
        pub assertions: bool,
    }


}

/// `Pool::BITMAP_SIZE`: the number of slots in one `BitmapStorage` (`size_t`) word.
const BITMAP_SIZE: usize = usize::BITS as usize;
/// `Pool::NO_FREE_ITEM`: constant to indicate we can't allocate any more items
const NO_FREE_ITEM: usize = usize::MAX;

/// Allocation failures returned before the native caller formats/throws an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AllocationError {
    /// `FatalError("{}: no more free items", name)`.
    Exhausted,
    /// `SlErrorCorruptFmt("{} index {} out of range ({})", name, index, max)`.
    OutOfRange,
    /// `SlErrorCorruptFmt("{} index {} already in use", name, index)`.
    AlreadyUsed,
}

/// Stable owner. There is no second `Vec` or descriptor metadata to synchronize.
pub struct PoolCore {
    view: PoolDescriptor,
    config: PoolConfig,
    ops: NativeOps,
}

impl PoolCore {
    /// Allocate a stable descriptor. `PoolBase` registration remains native.
    #[must_use]
    pub fn new(config: PoolConfig, ops: NativeOps) -> Box<Self> {
        Box::new(Self {
            view: PoolDescriptor {
                slots: NativeVec::new(),
                used_bitmap: NativeVec::new(),
                first_free: 0,
                first_unused: 0,
                items: 0,
                checked: 0,
                cleaning: false,
                alloc_cache: null_mut(),
            },
            config,
            ops,
        })
    }

    /// Inline native readers retain this pointer, refreshing buffer pointers on mutation.
    pub unsafe fn descriptor(pool: *mut Self) -> *mut PoolDescriptor {
        unsafe { &raw mut (*pool).view }
    }

    unsafe fn check(pool: *mut Self, condition: bool) {
        unsafe {
            if (*pool).config.assertions && !condition {
                let ops = (*pool).ops;
                (ops.assertion)(ops.context, condition);
            }
        }
    }

    /// Source unsigned subtraction, including `n > max_size` wraparound.
    pub unsafe fn can_allocate(pool: *mut Self, n: usize) -> bool {
        unsafe {
            let result = (*pool).view.items <= (*pool).config.max_size.wrapping_sub(n);
            if (*pool).config.assertions {
                (*pool).view.checked = if result { n } else { 0 };
            }
            result
        }
    }

    /// `ResizeFor`: resizes the pool so `index` can be addressed. Preconditions:
    /// `index >= size` and `index < MAX_SIZE`.
    unsafe fn resize_for(pool: *mut Self, index: usize) {
        unsafe {
            let old_size = (*pool).view.slots.len();
            Self::check(pool, index >= old_size);
            Self::check(pool, index < (*pool).config.max_size);
            let step = (*pool).config.growth_step;
            let new_size = align(index + 1, step);
            let new_size = new_size.min((*pool).config.max_size);
            (*pool).view.slots.resize(
                new_size,
                PoolSlot {
                    shell: null_mut(),
                    record: null_mut(),
                },
            );
            (*pool)
                .view
                .used_bitmap
                .resize(align(new_size, BITMAP_SIZE) / BITMAP_SIZE, 0);
            let bitmap = (*pool).view.used_bitmap.as_mut_ptr();
            if !old_size.is_multiple_of(BITMAP_SIZE) {
                // Already-allocated bits above old size are now unused.
                *bitmap.add(old_size / BITMAP_SIZE) &= !(usize::MAX << (old_size % BITMAP_SIZE));
            }
            if !new_size.is_multiple_of(BITMAP_SIZE) {
                // Bits above new size are considered used.
                *bitmap.add(new_size / BITMAP_SIZE) |= usize::MAX << (new_size % BITMAP_SIZE);
            }
        }
    }

    /// `FindFirstFree`: searches for first free index. Returns the first free index,
    /// `NO_FREE_ITEM` on failure.
    unsafe fn find_first_free(pool: *mut Self) -> usize {
        unsafe {
            let bits = BITMAP_SIZE;
            for word in ((*pool).view.first_free / bits)..(*pool).view.used_bitmap.len() {
                let available = !*(*pool).view.used_bitmap.as_mut_ptr().add(word);
                if available != 0 {
                    return word * bits + usize::from(find_first_bit(available as u64));
                }
            }
            Self::check(pool, (*pool).view.first_unused == (*pool).view.slots.len());
            if (*pool).view.first_unused < (*pool).config.max_size {
                Self::resize_for(pool, (*pool).view.first_unused);
                return (*pool).view.first_unused;
            }
            Self::check(pool, (*pool).view.first_unused == (*pool).config.max_size);
            NO_FREE_ITEM
        }
    }

    /// Ordinary new: search/grow precedes assertion-credit consumption and exhaustion.
    ///
    /// # Errors
    /// Returns `Exhausted` after consuming ordinary assertion credit.
    pub unsafe fn allocate(pool: *mut Self, size: usize) -> Result<*mut c_void, AllocationError> {
        unsafe {
            let index = Self::find_first_free(pool);
            if (*pool).config.assertions {
                Self::check(pool, (*pool).view.checked != 0);
                (*pool).view.checked = (*pool).view.checked.wrapping_sub(1);
            }
            if index == NO_FREE_ITEM {
                return Err(AllocationError::Exhausted);
            }
            (*pool).view.first_free = index + 1;
            Ok(Self::allocate_item(pool, size, index))
        }
    }

    /// Indexed load allocation leaves ordinary cursor and assertion credit unchanged.
    ///
    /// # Errors
    /// Returns `OutOfRange` before growth, or `AlreadyUsed` after growth if occupied.
    pub unsafe fn allocate_at(
        pool: *mut Self,
        size: usize,
        index: usize,
    ) -> Result<*mut c_void, AllocationError> {
        unsafe {
            if index >= (*pool).config.max_size {
                return Err(AllocationError::OutOfRange);
            }
            if index >= (*pool).view.slots.len() {
                Self::resize_for(pool, index);
            }
            if !(*(*pool).view.slots.as_mut_ptr().add(index))
                .shell
                .is_null()
            {
                return Err(AllocationError::AlreadyUsed);
            }
            Ok(Self::allocate_item(pool, size, index))
        }
    }

    /// `AllocateItem`: makes given index valid. Preconditions: `index < size` and the slot
    /// is empty.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "SetBit takes the bit position as uint8_t; index % BITMAP_SIZE narrows to it"
    )]
    unsafe fn allocate_item(pool: *mut Self, size: usize, index: usize) -> *mut c_void {
        unsafe {
            Self::check(
                pool,
                (*(*pool).view.slots.as_mut_ptr().add(index))
                    .shell
                    .is_null(),
            );
            (*pool).view.first_unused = (*pool).view.first_unused.max(index + 1);
            (*pool).view.items += 1;
            let ops = (*pool).ops;
            let shell = if (*pool).config.cache && !(*pool).view.alloc_cache.is_null() {
                Self::check(pool, (*pool).config.item_size == size);
                let cached = (*pool).view.alloc_cache;
                (*pool).view.alloc_cache = (*cached).next;
                cached.cast()
            } else {
                (ops.allocate)(ops.context, size)
            };
            (*(*pool).view.slots.as_mut_ptr().add(index)).shell = shell;
            set_bit_size(
                &mut *(*pool)
                    .view
                    .used_bitmap
                    .as_mut_ptr()
                    .add(index / BITMAP_SIZE),
                (index % BITMAP_SIZE) as u8,
            );
            (ops.set_index)(ops.context, shell, index);
            shell
        }
    }

    /// Publish only at real record-owner initialization; borrowed shells never bind.
    pub unsafe fn bind_record(pool: *mut Self, index: usize, record: *mut c_void) {
        unsafe {
            (*(*pool).view.slots.as_mut_ptr().add(index)).record = record;
        }
    }

    /// Called at record-owner destruction, before its Rust allocation becomes invalid.
    pub unsafe fn retire_record(pool: *mut Self, index: usize) {
        unsafe {
            (*(*pool).view.slots.as_mut_ptr().add(index)).record = null_mut();
        }
    }

    /// Sized delete calls this after the destructor chain, while shell is still published.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "ClrBit takes the bit position as uint8_t; index % BITMAP_SIZE narrows to it"
    )]
    pub unsafe fn free_item(pool: *mut Self, size: usize, index: usize) {
        unsafe {
            Self::check(pool, index < (*pool).view.slots.len());
            let shell = (*(*pool).view.slots.as_mut_ptr().add(index)).shell;
            Self::check(pool, !shell.is_null());
            let ops = (*pool).ops;
            if (*pool).config.cache {
                let cached = shell.cast::<PoolCacheLink>();
                (*cached).next = (*pool).view.alloc_cache;
                (*pool).view.alloc_cache = cached;
            } else {
                (ops.deallocate)(ops.context, shell, size);
            }
            (*(*pool).view.slots.as_mut_ptr().add(index)).shell = null_mut();
            (*pool).view.first_free = (*pool).view.first_free.min(index);
            (*pool).view.items -= 1;
            if !(*pool).view.cleaning {
                clr_bit_size(
                    &mut *(*pool)
                        .view
                        .used_bitmap
                        .as_mut_ptr()
                        .add(index / BITMAP_SIZE),
                    (index % BITMAP_SIZE) as u8,
                );
                (ops.post_destructor)(ops.context, index);
            }
        }
    }

    /// Ascending live traversal. Cleanup preserves assertion credit and suppresses post hooks.
    pub unsafe fn clean(pool: *mut Self) {
        unsafe {
            (*pool).view.cleaning = true;
            let ops = (*pool).ops;
            let mut index = 0;
            while index < (*pool).view.first_unused {
                let shell = (*(*pool).view.slots.as_mut_ptr().add(index)).shell;
                if !shell.is_null() {
                    (ops.destroy)(ops.context, shell);
                }
                index += 1;
            }
            Self::check(pool, (*pool).view.items == 0);
            (*pool).view.slots = NativeVec::new();
            (*pool).view.used_bitmap = NativeVec::new();
            (*pool).view.first_unused = 0;
            (*pool).view.first_free = 0;
            (*pool).view.cleaning = false;
            if (*pool).config.cache {
                while !(*pool).view.alloc_cache.is_null() {
                    let cached = (*pool).view.alloc_cache;
                    (*pool).view.alloc_cache = (*cached).next;
                    (ops.deallocate)(ops.context, cached.cast(), (*pool).config.item_size);
                }
            }
        }
    }
}

/// An owner's existing transparent pool-ID type supplies its native integer coordinate.
pub trait PoolId: Copy {
    /// Convert without generation checks or changing the native integer domain.
    fn index(self) -> usize;
}

impl PoolId for crate::vehicle_type::VehicleID {
    fn index(self) -> usize {
        self.0 as usize
    }
}

/// Nonowning typed access, invalidated when the owner is destroyed.
#[derive(Clone, Copy)]
pub struct TypedPool<I, R, S> {
    view: *const PoolDescriptor,
    marker: PhantomData<fn(I) -> (R, S)>,
}
impl<I: PoolId, R, S> TypedPool<I, R, S> {
    /// The descriptor must outlive this view; raw reads follow the module's access contract.
    #[must_use]
    pub const unsafe fn from_descriptor(view: *const PoolDescriptor) -> Self {
        Self {
            view,
            marker: PhantomData,
        }
    }
    /// Resolve the current record directly, with one slot pointer load; a hole returns null.
    pub unsafe fn record(&self, id: I) -> *mut R {
        unsafe {
            let index = id.index();
            if index < (*self.view).first_unused {
                (*(*self.view).slots.as_mut_ptr().add(index)).record.cast()
            } else {
                null_mut()
            }
        }
    }
    /// Native shell identity remains distinct from the typed Rust record.
    pub unsafe fn shell(&self, id: I) -> *mut S {
        unsafe {
            let index = id.index();
            if index < (*self.view).first_unused {
                (*(*self.view).slots.as_mut_ptr().add(index)).shell.cast()
            } else {
                null_mut()
            }
        }
    }
    /// `PoolIterator::ValidateIndex`: reread live extent/slots, using the native end sentinel.
    #[must_use]
    pub unsafe fn next_index(&self, mut index: usize, max_size: usize) -> usize {
        unsafe {
            while index < (*self.view).first_unused {
                if !(*(*self.view).slots.as_mut_ptr().add(index))
                    .shell
                    .is_null()
                {
                    return index;
                }
                index += 1;
            }
            max_size
        }
    }
}

#[cfg(test)]
mod tests;

/// Native pool facade construction; parameters and operation table are copied.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_new(
    config: *const PoolConfig,
    ops: *const NativeOps,
) -> *mut PoolCore {
    unsafe { Box::into_raw(PoolCore::new(*config, *ops)) }
}
/// Release buffers after the native facade's final use; shells are cleaned explicitly.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_drop(pool: *mut PoolCore) {
    unsafe {
        drop(Box::from_raw(pool));
    }
}
/// Stable canonical metadata, invalidated only when the owner is dropped.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_descriptor(pool: *mut PoolCore) -> *mut PoolDescriptor {
    unsafe { PoolCore::descriptor(pool) }
}
/// Source `CanAllocate` with native assertion-credit behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_can_allocate(pool: *mut PoolCore, count: usize) -> u8 {
    u8::from(unsafe { PoolCore::can_allocate(pool, count) })
}
/// Null means `Exhausted`; the native caller formats `FatalError` after return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_allocate(
    pool: *mut PoolCore,
    size: usize,
) -> *mut c_void {
    unsafe { PoolCore::allocate(pool, size).unwrap_or(null_mut()) }
}
/// Null means `OutOfRange` or `AlreadyUsed`; index distinguishes the native error text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_allocate_at(
    pool: *mut PoolCore,
    size: usize,
    index: usize,
) -> *mut c_void {
    unsafe { PoolCore::allocate_at(pool, size, index).unwrap_or(null_mut()) }
}
/// Publish at record-owner initialization, after native shell publication.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_bind_record(
    pool: *mut PoolCore,
    index: usize,
    record: *mut c_void,
) {
    unsafe {
        PoolCore::bind_record(pool, index, record);
    }
}
/// Retire before dropping the owning record allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_retire_record(pool: *mut PoolCore, index: usize) {
    unsafe {
        PoolCore::retire_record(pool, index);
    }
}
/// Native sized delete preserves free-before-PostDestructor ordering.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_free(pool: *mut PoolCore, size: usize, index: usize) {
    unsafe {
        PoolCore::free_item(pool, size, index);
    }
}
/// Native `PoolBase` keeps registration/cleanup-mask order.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_pool_clean(pool: *mut PoolCore) {
    unsafe {
        PoolCore::clean(pool);
    }
}
