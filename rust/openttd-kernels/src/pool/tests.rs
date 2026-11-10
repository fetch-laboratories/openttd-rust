/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Literal observations from the pinned original `core/pool_func.hpp`.
use super::*;
use std::alloc::{Layout, alloc, dealloc};
use std::ptr;

#[repr(C)]
struct Shell {
    index: usize,
    serial: usize,
    record: usize,
}
const SIZE: usize = size_of::<Shell>();
#[derive(Default)]
struct Context {
    pool: *mut PoolCore,
    events: Vec<(char, usize, usize)>,
    serial: usize,
    replace: bool,
    grow_on_destroy: bool,
}
#[allow(clippy::cast_ptr_alignment)] // Layout explicitly requests Shell alignment.
unsafe extern "C" fn allocate(context: *mut c_void, size: usize) -> *mut c_void {
    unsafe {
        let c = context.cast::<Context>();
        let shell =
            alloc(Layout::from_size_align(size, align_of::<Shell>()).unwrap()).cast::<Shell>();
        assert!(!shell.is_null());
        (*c).serial += 1;
        ptr::addr_of_mut!((*shell).serial).write((*c).serial);
        (*c).events.push(('A', (*c).serial, size));
        shell.cast()
    }
}
unsafe extern "C" fn deallocate(context: *mut c_void, shell: *mut c_void, size: usize) {
    unsafe {
        let c = context.cast::<Context>();
        let view = PoolCore::descriptor((*c).pool);
        (*c).events.push((
            'X',
            (*shell.cast::<Shell>()).serial,
            usize::from((*view).cleaning),
        ));
        dealloc(
            shell.cast(),
            Layout::from_size_align(size, align_of::<Shell>()).unwrap(),
        );
    }
}
unsafe extern "C" fn set_index(context: *mut c_void, shell: *mut c_void, index: usize) {
    unsafe {
        let c = context.cast::<Context>();
        let view = PoolCore::descriptor((*c).pool);
        let slot = (*view).slots.as_mut_ptr().add(index);
        assert_eq!((*slot).shell, shell);
        assert!((*slot).record.is_null());
        assert_eq!(
            (*view).used_bitmap.as_slice()[index / BITMAP_SIZE] >> (index % BITMAP_SIZE) & 1,
            1
        );
        ptr::addr_of_mut!((*shell.cast::<Shell>()).index).write(index);
        (*c).events.push(('I', index, (*view).items));
    }
}
unsafe extern "C" fn destroy(context: *mut c_void, shell: *mut c_void) {
    unsafe {
        let c = context.cast::<Context>();
        let pool = (*c).pool;
        let view = PoolCore::descriptor(pool);
        let index = (*shell.cast::<Shell>()).index;
        assert_eq!((*(*view).slots.as_mut_ptr().add(index)).shell, shell);
        (*c).events
            .push(('D', index, usize::from((*view).cleaning)));
        let record = (*(*view).slots.as_mut_ptr().add(index))
            .record
            .cast::<usize>();
        if !record.is_null() {
            (*c).events.push(('R', index, *record));
        }
        if (*c).grow_on_destroy {
            (*c).grow_on_destroy = false;
            PoolCore::allocate_at(pool, SIZE, 8).unwrap();
        }
        PoolCore::retire_record(pool, index);
        PoolCore::free_item(pool, SIZE, index);
        if (*view).cleaning {
            (*c).events.push((
                'F',
                index,
                (*view).used_bitmap.as_slice()[index / BITMAP_SIZE] >> (index % BITMAP_SIZE) & 1,
            ));
        }
    }
}
unsafe extern "C" fn post(context: *mut c_void, index: usize) {
    unsafe {
        let c = context.cast::<Context>();
        let pool = (*c).pool;
        let view = PoolCore::descriptor(pool);
        let slot = (*view).slots.as_mut_ptr().add(index);
        assert!((*slot).shell.is_null());
        assert!((*slot).record.is_null());
        (*c).events.push(('P', index, (*view).items));
        if (*c).replace {
            (*c).replace = false;
            PoolCore::allocate_at(pool, SIZE, index).unwrap();
        }
    }
}
unsafe extern "C" fn assertion(_: *mut c_void, condition: bool) {
    assert!(condition);
}
struct Fixture {
    context: Box<Context>,
    pool: Box<PoolCore>,
}
impl Fixture {
    fn new(max_size: usize, cache: bool, assertions: bool) -> Self {
        let mut context = Box::<Context>::default();
        let ops = NativeOps {
            context: (&raw mut *context).cast(),
            allocate,
            deallocate,
            set_index,
            destroy,
            post_destructor: post,
            assertion,
        };
        let mut pool = PoolCore::new(
            PoolConfig {
                max_size,
                growth_step: 4,
                cache,
                item_size: SIZE,
                assertions,
            },
            ops,
        );
        context.pool = &raw mut *pool;
        Self { context, pool }
    }
    fn raw(&mut self) -> *mut PoolCore {
        &raw mut *self.pool
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            PoolCore::clean(self.raw());
        }
    }
}
#[derive(Clone, Copy)]
struct Id(usize);
impl PoolId for Id {
    fn index(self) -> usize {
        self.0
    }
}

#[test]
fn indexed_growth_holes_and_live_typed_lookup() {
    unsafe {
        let mut f = Fixture::new(10, false, true);
        let p = f.raw();
        let v = PoolCore::descriptor(p);
        let typed = TypedPool::<Id, usize, Shell>::from_descriptor(v);
        assert!(PoolCore::can_allocate(p, 3));
        let six = PoolCore::allocate_at(p, SIZE, 6).unwrap();
        assert_eq!(
            (
                (*v).slots.len(),
                (*v).first_unused,
                (*v).first_free,
                (*v).checked
            ),
            (8, 7, 0, 3)
        );
        assert!(typed.record(Id(6)).is_null());
        let record = &raw mut (*six.cast::<Shell>()).record;
        record.write(42);
        PoolCore::bind_record(p, 6, record.cast());
        assert_eq!(typed.record(Id(6)), record);
        assert_eq!(typed.shell(Id(6)), six.cast());
        for index in [0, 1] {
            let shell = PoolCore::allocate(p, SIZE).unwrap();
            assert_eq!((*shell.cast::<Shell>()).index, index);
        }
        assert_eq!(((*v).first_free, (*v).checked), (2, 1));
        assert_eq!(
            PoolCore::allocate_at(p, SIZE, 6),
            Err(AllocationError::AlreadyUsed)
        );
        assert_eq!(
            PoolCore::allocate_at(p, SIZE, 10),
            Err(AllocationError::OutOfRange)
        );
        assert_eq!(((*v).slots.len(), (*v).first_free, (*v).checked), (8, 2, 1));
        destroy((&raw mut *f.context).cast(), typed.shell(Id(1)).cast());
        assert_eq!(((*v).first_unused, (*v).first_free), (7, 1));
        let cursor = typed.next_index(1, 10);
        assert_eq!(cursor, 6);
        PoolCore::allocate(p, SIZE).unwrap();
        assert_eq!(typed.next_index(1, 10), 1);
        assert_eq!(typed.next_index(cursor + 1, 10), 10);
        let stable_record = typed.record(Id(6));
        PoolCore::allocate_at(p, SIZE, 9).unwrap();
        assert_eq!(((*v).slots.len(), (*v).first_unused), (10, 10));
        assert_eq!(typed.record(Id(6)), stable_record);
        assert_eq!(typed.next_index(7, 10), 9);
        assert_eq!(typed.next_index(10, 10), 10);
        assert!(typed.record(Id(10)).is_null());
    }
}

#[test]
fn maximum_tail_growth_capacity_and_credit() {
    unsafe {
        for maximum in [10, BITMAP_SIZE + 3] {
            let mut f = Fixture::new(maximum, false, true);
            let p = f.raw();
            let v = PoolCore::descriptor(p);
            assert!(PoolCore::can_allocate(p, 2));
            assert!(PoolCore::can_allocate(p, maximum + 1)); // Native unsigned subtraction wraps.
            for expected in 0..maximum {
                let shell = PoolCore::allocate(p, SIZE).unwrap();
                assert_eq!((*shell.cast::<Shell>()).index, expected);
                let len = (*v).slots.len();
                assert_eq!(len, ((expected + 4) & !3).min(maximum));
                if !len.is_multiple_of(BITMAP_SIZE) {
                    assert_eq!(
                        (*v).used_bitmap.as_slice().last().unwrap() >> (len % BITMAP_SIZE),
                        usize::MAX >> (len % BITMAP_SIZE)
                    );
                }
            }
            assert_eq!((*v).checked, 1);
            assert_eq!(PoolCore::allocate(p, SIZE), Err(AllocationError::Exhausted));
            assert_eq!((*v).checked, 0);
            assert!(!PoolCore::can_allocate(p, 1));
            assert!(PoolCore::can_allocate(p, 0));
            assert_eq!((*v).checked, 0);
        }
        let mut f = Fixture::new(10, false, false);
        let p = f.raw();
        let v = PoolCore::descriptor(p);
        (*v).checked = 19;
        assert!(PoolCore::can_allocate(p, 4));
        PoolCore::allocate(p, SIZE).unwrap();
        assert_eq!((*v).checked, 19); // Rust debug mode does not select WITH_ASSERT.
        // FindFirstFree inspects the entire first word, without masking below first_free.
        PoolCore::allocate_at(p, SIZE, 6).unwrap();
        (*v).first_free = 5;
        let shell = PoolCore::allocate(p, SIZE).unwrap();
        assert_eq!((*shell.cast::<Shell>()).index, 1);
    }
}

#[test]
fn destructor_publication_post_reentrant_reuse() {
    unsafe {
        let mut f = Fixture::new(10, false, false);
        let p = f.raw();
        let v = PoolCore::descriptor(p);
        let shell = PoolCore::allocate_at(p, SIZE, 0).unwrap();
        let record = &raw mut (*shell.cast::<Shell>()).record;
        record.write(7);
        PoolCore::bind_record(p, 0, record.cast());
        f.context.replace = true;
        destroy((&raw mut *f.context).cast(), shell);
        assert_eq!(
            f.context.events,
            [
                ('A', 1, SIZE),
                ('I', 0, 1),
                ('D', 0, 0),
                ('R', 0, 7),
                ('X', 1, 0),
                ('P', 0, 0),
                ('A', 2, SIZE),
                ('I', 0, 1)
            ]
        );
        assert!(!(*(*v).slots.as_mut_ptr()).shell.is_null());
        assert!((*(*v).slots.as_mut_ptr()).record.is_null());
        assert_eq!(((*v).items, (*v).first_unused), (1, 1));
    }
}

#[test]
fn cleanup_live_extent_bitmap_suppression_and_cache_lifo() {
    unsafe {
        let mut f = Fixture::new(10, true, true);
        let p = f.raw();
        let v = PoolCore::descriptor(p);
        let first = PoolCore::allocate_at(p, SIZE, 0).unwrap();
        let second = PoolCore::allocate_at(p, SIZE, 2).unwrap();
        destroy((&raw mut *f.context).cast(), first);
        destroy((&raw mut *f.context).cast(), second);
        assert!(PoolCore::can_allocate(p, 5));
        assert_eq!(PoolCore::allocate(p, SIZE).unwrap(), second);
        assert_eq!(PoolCore::allocate(p, SIZE).unwrap(), first);
        assert_eq!((*v).checked, 3);
        f.context.grow_on_destroy = true;
        f.context.events.clear();
        PoolCore::clean(p);
        assert_eq!(
            f.context.events,
            [
                ('D', 0, 1),
                ('A', 3, SIZE),
                ('I', 8, 3),
                ('F', 0, 1),
                ('D', 1, 1),
                ('F', 1, 1),
                ('D', 8, 1),
                ('F', 8, 1),
                ('X', 3, 0),
                ('X', 1, 0),
                ('X', 2, 0)
            ]
        );
        assert_eq!(
            (
                (*v).slots.len(),
                (*v).used_bitmap.len(),
                (*v).first_free,
                (*v).first_unused,
                (*v).items,
                (*v).checked
            ),
            (0, 0, 0, 0, 0, 3)
        );
        assert!(!(*v).cleaning);
        assert!((*v).alloc_cache.is_null());
        assert_eq!(PoolCore::descriptor(p), v);
    }
}

#[test]
fn two_threads_share_serialized_owner() {
    use crate::game_state::GameState;
    use std::sync::Mutex;
    static OWNER: GameState<*mut PoolCore> = GameState::new(null_mut());
    static LOCK: Mutex<()> = Mutex::new(());
    let mut f = Fixture::new(10, false, false);
    unsafe {
        *OWNER.get() = f.raw();
    }
    std::thread::spawn(|| {
        let _lock = LOCK.lock().unwrap();
        unsafe {
            PoolCore::allocate_at(*OWNER.get(), SIZE, 6).unwrap();
        }
    })
    .join()
    .unwrap();
    std::thread::spawn(|| {
        let _lock = LOCK.lock().unwrap();
        unsafe {
            let p = *OWNER.get();
            let v = PoolCore::descriptor(p);
            assert_eq!(((*v).items, (*v).first_unused), (1, 7));
            PoolCore::allocate(p, SIZE).unwrap();
            assert_eq!(((*v).items, (*v).first_free), (2, 1));
            PoolCore::clean(p);
        }
    })
    .join()
    .unwrap();
    assert_eq!(f.pool.view.items, 0);
    unsafe {
        *OWNER.get() = null_mut();
    }
}
