/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Action2 partial reads, publication and the callback interner (#736 W).
//!
//! Moved from `Action2PartialRead` of `tools/simulation/newgrf-loading-console.cpp` and the
//! Catch2 interner case of `src/tests/newgrf_loading.cpp`. The inputs and literal
//! expectations are that fixture's, which passed on the unchanged original's
//! `NewSpriteGroup` (the reference role runs the fixture). The real Rust parser drives
//! recording leaves in place of the native pool, so these tests prove its decisions and
//! leaf-call order, not the pool or `SpriteGroup` internals. The file-loaded graph and
//! resolver consumers stay native; `install_test_labels` stands in for the cargo table.
use super::*;
use crate::newgrf_bytereader::Cursor;
use crate::newgrf_loading::{TEST_TABLE as LOADING_TEST_TABLE, openttd_rust_grf_execution};
use crate::test_tables::{SHARED, unreachable1, unreachable2, unreachable3};
use std::sync::atomic::{AtomicU8, Ordering};

const INVALID_LAYOUT: u32 = 1001;
const INVALID_PRODUCTION: u32 = 1002;

#[derive(Default)]
struct Adjust {
    operation: u8,
    kind: u8,
    variable: u8,
    parameter: u8,
    shift_num: u8,
    and_mask: u32,
    add_val: u32,
    divmod_val: u32,
    subroutine: usize,
}
struct Deterministic {
    adjusts: Vec<Adjust>,
    ranges: Vec<Range>,
}
struct LayoutNode {
    sequence: u32,
    /// `ground.sprite` and `ground.pal`.
    ground: [u32; 2],
    consistent_max_offset: u32,
}
#[derive(Default)]
struct Production {
    version: u8,
    num_input: u8,
    subtract_input: [i16; 16],
    cargo_input: [u8; 16],
    num_output: u8,
    add_output: [u16; 16],
    cargo_output: [u8; 16],
    again: u8,
}
enum Node {
    Callback(u16),
    Deterministic(Box<Deterministic>),
    Layout(Box<LayoutNode>),
    Production(Box<Production>),
}
struct World {
    nodes: Vec<Node>,
    errors: Vec<u32>,
    error_data: Vec<Vec<u8>>,
}
static WORLD: GameState<World> = GameState::new(World {
    nodes: Vec::new(),
    errors: Vec::new(),
    error_data: Vec::new(),
});
static VERSION: AtomicU8 = AtomicU8::new(8);

fn world<'a>() -> &'a mut World {
    // SAFETY: Each test holds GLOBAL_LOCK, and no borrow outlives a leaf call.
    unsafe { &mut *WORLD.get() }
}
fn handle(index: usize) -> Handle {
    (index + 1) as Handle
}
fn index(group: Handle) -> usize {
    group as usize - 1
}
fn push(node: Node) -> Handle {
    world().nodes.push(node);
    handle(world().nodes.len() - 1)
}
fn deterministic_of(group: Handle) -> &'static mut Deterministic {
    match &mut world().nodes[index(group)] {
        Node::Deterministic(node) => node,
        _ => panic!("group is not deterministic"),
    }
}
fn layout_of(group: Handle) -> &'static mut LayoutNode {
    match &mut world().nodes[index(group)] {
        Node::Layout(node) => node,
        _ => panic!("group is not a layout"),
    }
}
fn callback_of(group: Handle) -> u16 {
    match world().nodes[index(group)] {
        Node::Callback(value) => value,
        _ => panic!("group is not a callback result"),
    }
}

extern "C" fn callback_new(value: u16) -> Handle {
    push(Node::Callback(value))
}
extern "C" fn group_index(group: Handle) -> u32 {
    index(group) as u32
}
extern "C" fn group_get(group: u32) -> Handle {
    handle(group as usize)
}
extern "C" fn deterministic_new(_: u32, _: u8, _: u8) -> Handle {
    push(Node::Deterministic(Box::new(Deterministic {
        adjusts: Vec::new(),
        ranges: Vec::new(),
    })))
}
extern "C" fn adjust_begin(group: Handle) -> AdjustStorage {
    let adjusts = &mut deterministic_of(group).adjusts;
    adjusts.push(Adjust::default());
    let a = adjusts.last_mut().unwrap();
    AdjustStorage {
        operation: &raw mut a.operation,
        kind: &raw mut a.kind,
        variable: &raw mut a.variable,
        parameter: &raw mut a.parameter,
        shift_num: &raw mut a.shift_num,
        and_mask: &raw mut a.and_mask,
        add_val: &raw mut a.add_val,
        divmod_val: &raw mut a.divmod_val,
        subroutine: (&raw mut a.subroutine).cast(),
    }
}
extern "C" fn deterministic_default(_: Handle, _: GroupResult) {}
extern "C" fn deterministic_error(_: Handle, _: Handle) {}
extern "C" fn range_append(group: Handle, range: Range) {
    deterministic_of(group).ranges.push(range);
}
extern "C" fn layout_new(_: u32) -> Handle {
    push(Node::Layout(Box::new(LayoutNode {
        sequence: 0,
        ground: [0; 2],
        consistent_max_offset: 0,
    })))
}
extern "C" fn group_layout(group: Handle) -> *mut Layout {
    std::ptr::from_mut(layout_of(group)).cast()
}
extern "C" fn layout_allocate(layout: *mut Layout, count: u32) {
    unsafe { (*layout.cast::<LayoutNode>()).sequence = count };
}
extern "C" fn layout_storage(layout: *mut Layout) -> LayoutStorage {
    let layout = layout.cast::<LayoutNode>();
    unsafe {
        LayoutStorage {
            ground: (&raw mut (*layout).ground).cast(),
            consistent_max_offset: &raw mut (*layout).consistent_max_offset,
        }
    }
}
extern "C" fn pal_sprite_storage(image: *mut PalSprite) -> PalSpriteStorage {
    let image = image.cast::<u32>();
    PalSpriteStorage {
        sprite: image,
        pal: unsafe { image.add(1) },
    }
}
extern "C" fn production_new(_: u32) -> Handle {
    push(Node::Production(Box::default()))
}
extern "C" fn production_storage(group: Handle) -> ProductionStorage {
    let Node::Production(f) = &mut world().nodes[index(group)] else {
        panic!("group is not a production");
    };
    ProductionStorage {
        version: &raw mut f.version,
        num_input: &raw mut f.num_input,
        subtract_input: f.subtract_input.as_mut_ptr(),
        cargo_input: f.cargo_input.as_mut_ptr(),
        num_output: &raw mut f.num_output,
        add_output: f.add_output.as_mut_ptr(),
        cargo_output: f.cargo_output.as_mut_ptr(),
        again: &raw mut f.again,
    }
}
extern "C" fn error_data(_: *mut c_void, data: *const u8, length: usize) {
    let text = unsafe { std::slice::from_raw_parts(data, length) };
    world().error_data.push(text.to_vec());
}
extern "C" fn set_status(_: *mut c_void, _: u8) {}
extern "C" fn error(_: *mut c_void, _: u32, message: u32, _: bool) -> *mut c_void {
    world().errors.push(message);
    std::ptr::dangling_mut()
}
extern "C" fn grf_file_version(_: *const c_void) -> u8 {
    VERSION.load(Ordering::SeqCst)
}

const SERVICES: Services = Services {
    pal_sprite_storage,
    invalid_layout: INVALID_LAYOUT,
    invalid_production: INVALID_PRODUCTION,
    callback_new,
    group_index,
    group_get,
    result_new: unreachable2,
    assert_loaded: unreachable2,
    deterministic_new,
    random_new: unreachable1,
    real_new: unreachable1,
    layout_new,
    production_new,
    adjust_begin,
    deterministic_default,
    deterministic_error,
    range_append,
    random_storage: unreachable1,
    random_reserve: unreachable2,
    random_append: unreachable2,
    loaded_reserve: unreachable2,
    loaded_append: unreachable2,
    loading_reserve: unreachable2,
    loading_append: unreachable2,
    production_storage,
    group_layout,
    layout_allocate,
    layout_storage,
    layout_sequence: unreachable2,
    sequence_storage: unreachable1,
    is_parent: unreachable1,
    has_registers: unreachable1,
    allocate_registers: unreachable1,
    registers: unreachable2,
    assert_offsets: unreachable3,
    error_data,
};

struct Fixture {
    shared: crate::services::Services,
    global: crate::newgrf::Services,
    loading: LoadingServices,
    file: Box<u8>,
}
impl Fixture {
    fn new() -> Box<Self> {
        let mut shared = SHARED;
        shared.grf_file_version = grf_file_version;
        let mut f = Box::new(Self {
            shared,
            global: crate::newgrf::TEST_TABLE,
            loading: LOADING_TEST_TABLE,
            file: Box::new(0),
        });
        f.global.shared = &raw const f.shared;
        f.loading.global = &raw const f.global;
        f.loading.set_status = set_status;
        f.loading.error = error;
        world().nodes.clear();
        world().errors.clear();
        world().error_data.clear();
        VERSION.store(8, Ordering::SeqCst);
        unsafe {
            let state = openttd_rust_grf_execution();
            (*state).grffile = (&raw mut *f.file).cast();
            (*state).nfo_line = 485;
            (*state).skip_sprites = 0;
            (*state).spritegroups.fill(ptr::null());
            openttd_rust_action2_reset(false);
        }
        f
    }
    fn parse(&self, bytes: &[u8]) -> (bool, usize) {
        let mut reader = Cursor::new(bytes.as_ptr(), bytes.len());
        let done = unsafe { parse_action(&SERVICES, &self.loading, &mut reader) };
        (done, reader.remaining())
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            openttd_rust_action2_reset(true);
            let state = openttd_rust_grf_execution();
            (*state).grffile = ptr::null_mut();
            (*state).nfo_line = 0;
            (*state).skip_sprites = 0;
            (*state).spritegroups.fill(ptr::null());
        }
        world().nodes.clear();
    }
}
fn slot(id: usize) -> Handle {
    unsafe { (*openttd_rust_grf_execution()).spritegroups[id] }
}
fn count() -> usize {
    world().nodes.len()
}

#[test]
fn partial_reads_publication_and_interner() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let f = Fixture::new();

    // First publication of a deterministic group with one callback range.
    assert!(
        f.parse(&[1, 200, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127])
            .0
    );
    let published = slot(200);
    let first_group = deterministic_of(published);
    assert_eq!(first_group.ranges.len(), 1);
    let first_callback = first_group.ranges[0].result.group;
    assert_eq!(callback_of(first_callback), 3);

    // A truncated group allocates its node and interned callback, yet never publishes.
    let before = count();
    let (done, left) = f.parse(&[1, 200, 0x81, 0x0C, 0, 255, 1, 4, 128]);
    assert!(!done);
    assert_eq!(left, 0);
    assert_eq!(slot(200), published);
    assert_eq!(count(), before + 2);
    let partial = deterministic_of(handle(before));
    assert_eq!(partial.adjusts.len(), 1);
    assert_eq!(partial.adjusts[0].and_mask, 255);
    assert!(partial.ranges.is_empty());
    assert_eq!(callback_of(handle(before + 1)), 4);

    // A repeated callback result is interned, within one load.
    let before = count();
    assert!(
        f.parse(&[1, 201, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127])
            .0
    );
    assert_eq!(
        deterministic_of(slot(201)).ranges[0].result.group,
        first_callback
    );
    assert_eq!(count(), before + 1);

    // The final reset forgets the interned results.
    unsafe { openttd_rust_action2_reset(true) };
    let before = count();
    assert!(
        f.parse(&[1, 202, 0x81, 0x0C, 0, 255, 1, 3, 128, 0, 255, 255, 127])
            .0
    );
    assert_eq!(count(), before + 2);

    // An unsupported production version publishes null over the old slot.
    assert!(f.parse(&[10, 200, 3]).0);
    assert!(slot(200).is_null());

    // Before GRF version 8 the callback high byte FF is masked off; afterwards only bit 15.
    VERSION.store(7, Ordering::SeqCst);
    assert!(
        f.parse(&[1, 203, 0x81, 0x0C, 0, 255, 1, 3, 255, 0, 255, 255, 127])
            .0
    );
    assert_eq!(
        callback_of(deterministic_of(slot(203)).ranges[0].result.group),
        3
    );
    VERSION.store(8, Ordering::SeqCst);
    assert!(
        f.parse(&[1, 204, 0x81, 0x0C, 0, 255, 1, 3, 255, 0, 255, 255, 127])
            .0
    );
    assert_eq!(
        callback_of(deterministic_of(slot(204)).ranges[0].result.group),
        0x7F03
    );

    // A layout truncated inside its ground sprite stays allocated and unpublished.
    let before = count();
    let (done, left) = f.parse(&[9, 205, 1, 1, 0, 0]);
    assert!(!done);
    assert_eq!(left, 1);
    assert_eq!(count(), before + 1);
    let layout = layout_of(handle(before));
    assert_eq!(layout.sequence, 1);
    assert_eq!(layout.ground, [1, 0]);
    assert!(slot(205).is_null());

    // Too many production inputs disables the file with the original's error data.
    assert!(f.parse(&[10, 206, 2, 17]).0);
    assert_eq!(
        world().error_data.last().unwrap(),
        b"too many inputs (max 16)"
    );
    assert_eq!(world().errors.last(), Some(&INVALID_PRODUCTION));
    assert!(slot(206).is_null());
    assert_eq!(unsafe { (*openttd_rust_grf_execution()).skip_sprites }, -1);

    // A ground sprite with an invalid flag disables the file as an invalid layout.
    unsafe { (*openttd_rust_grf_execution()).skip_sprites = 0 };
    assert!(f.parse(&[9, 209, 0x41, 1, 0, 0, 0, 16, 0]).0);
    assert_eq!(world().errors.last(), Some(&INVALID_LAYOUT));
    assert!(slot(209).is_null());
}

fn production_of(group: Handle) -> &'static Production {
    match &world().nodes[index(group)] {
        Node::Production(node) => node,
        _ => panic!("group is not a production"),
    }
}

#[test]
fn production_duplicate_and_invalid_cargo() {
    static LABELS: [u32; 1] = [0x4C49_414D];
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    crate::cargo_registry::install_test_labels(&LABELS, &[0]);
    let f = Fixture::new();

    // A repeated input cargo disables the file; the group is never published.
    assert!(f.parse(&[10, 207, 2, 2, 0, 9, 0]).0);
    assert_eq!(world().error_data.last().unwrap(), b"duplicate input cargo");
    assert_eq!(world().errors.last(), Some(&INVALID_PRODUCTION));
    assert!(slot(207).is_null());
    assert_eq!(unsafe { (*openttd_rust_grf_execution()).skip_sprites }, -1);

    // A cargo outside the file's list marks the group's version invalid and publishes it.
    unsafe { (*openttd_rust_grf_execution()).skip_sprites = 0 };
    assert!(f.parse(&[10, 208, 2, 1, 255, 9, 0, 0]).0);
    let invalid = production_of(slot(208));
    assert_eq!(invalid.version, 255);
    assert_eq!(invalid.cargo_input[0], 0xFF);
    assert_eq!(invalid.subtract_input[0], 9);
    assert_eq!(unsafe { (*openttd_rust_grf_execution()).skip_sprites }, 0);

    crate::cargo_registry::clear_test_labels();
}

#[test]
fn callback_interner_crosses_serialized_threads() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let f = Fixture::new();
    let address = (&raw const *f).expose_provenance();
    let parse = |id: u8| {
        std::thread::spawn(move || {
            // SAFETY: `f` outlives the join below, and this thread is the only one using it.
            let f = unsafe { &*std::ptr::with_exposed_provenance::<Fixture>(address) };
            f.parse(&[1, id, 0x81, 0x0C, 0, 255, 1, 55, 128, 0, 255, 255, 127])
                .0
        })
        .join()
        .unwrap()
    };
    assert!(parse(251));
    let first = deterministic_of(slot(251)).ranges[0].result.group;
    let before = count();
    assert!(parse(252));
    let second = deterministic_of(slot(252)).ranges[0].result.group;
    assert_eq!(first, second);
    assert_eq!(count(), before + 1);
}
