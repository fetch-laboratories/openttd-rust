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

use crate::industrytype::NativeVec;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::{self, null_mut};

crate::abi::shared_records! {
    fn abi_record;
    /// Shell and record have independent construction/destruction observation points.
    #[derive(Clone, Copy)]
    "RustPoolSlot" ["pool"] => pub struct PoolSlot {
        pub shell: *mut c_void,
        pub record: *mut c_void,
    }
    /// Intrusive link in dead, natively allocated shell bytes.
    "RustPoolCacheLink" ["pool"] => pub struct PoolCacheLink {
        pub next: *mut PoolCacheLink,
    }
    /// Canonical storage; its address stays stable until the owner is dropped.
    "RustPoolDescriptor" ["pool"] => pub struct PoolDescriptor {
        pub slots: NativeVec<PoolSlot> => "RustVector<RustPoolSlot>",
        pub used_bitmap: NativeVec<usize> => "RustVector<std::size_t>",
        pub first_free: usize,
        pub first_unused: usize,
        pub items: usize,
        pub checked: usize,
        pub cleaning: bool,
        pub alloc_cache: *mut PoolCacheLink,
    }
}

/// Native leaves, copied before callbacks so no Rust owner borrow survives reentry.
#[derive(Clone, Copy)]
pub struct NativeOps {
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
pub struct PoolConfig {
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
    /// Allocate a stable descriptor. PoolBase registration remains native.
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

    unsafe fn resize_for(pool: *mut Self, index: usize) {
        unsafe {
            let old_size = (*pool).view.slots.len();
            Self::check(pool, index >= old_size);
            Self::check(pool, index < (*pool).config.max_size);
            let step = (*pool).config.growth_step;
            let new_size = (index + 1).wrapping_add(step - 1) & !(step - 1);
            let new_size = new_size.min((*pool).config.max_size);
            (*pool).view.slots.resize(
                new_size,
                PoolSlot {
                    shell: null_mut(),
                    record: null_mut(),
                },
            );
            let bits = usize::BITS as usize;
            (*pool).view.used_bitmap.resize(new_size.div_ceil(bits), 0);
            let bitmap = (*pool).view.used_bitmap.as_mut_ptr();
            if old_size % bits != 0 {
                *bitmap.add(old_size / bits) &= !(usize::MAX << (old_size % bits));
            }
            if new_size % bits != 0 {
                *bitmap.add(new_size / bits) |= usize::MAX << (new_size % bits);
            }
        }
    }

    unsafe fn find_first_free(pool: *mut Self) -> usize {
        unsafe {
            let bits = usize::BITS as usize;
            for word in ((*pool).view.first_free / bits)..(*pool).view.used_bitmap.len() {
                let available = !*(*pool).view.used_bitmap.as_mut_ptr().add(word);
                if available != 0 {
                    return word * bits + available.trailing_zeros() as usize;
                }
            }
            Self::check(pool, (*pool).view.first_unused == (*pool).view.slots.len());
            if (*pool).view.first_unused < (*pool).config.max_size {
                Self::resize_for(pool, (*pool).view.first_unused);
                return (*pool).view.first_unused;
            }
            Self::check(pool, (*pool).view.first_unused == (*pool).config.max_size);
            usize::MAX
        }
    }

    /// Ordinary new: search/grow precedes assertion-credit consumption and exhaustion.
    pub unsafe fn allocate(pool: *mut Self, size: usize) -> Result<*mut c_void, AllocationError> {
        unsafe {
            let index = Self::find_first_free(pool);
            if (*pool).config.assertions {
                Self::check(pool, (*pool).view.checked != 0);
                (*pool).view.checked = (*pool).view.checked.wrapping_sub(1);
            }
            if index == usize::MAX {
                return Err(AllocationError::Exhausted);
            }
            (*pool).view.first_free = index + 1;
            Ok(Self::allocate_item(pool, size, index))
        }
    }

    /// Indexed load allocation leaves ordinary cursor and assertion credit unchanged.
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
            *(*pool)
                .view
                .used_bitmap
                .as_mut_ptr()
                .add(index / usize::BITS as usize) |= 1 << (index % usize::BITS as usize);
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
                *(*pool)
                    .view
                    .used_bitmap
                    .as_mut_ptr()
                    .add(index / usize::BITS as usize) &= !(1 << (index % usize::BITS as usize));
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
    pub unsafe fn record(self, id: I) -> *mut R {
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
    pub unsafe fn shell(self, id: I) -> *mut S {
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
    pub unsafe fn next_index(self, mut index: usize, max_size: usize) -> usize {
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
