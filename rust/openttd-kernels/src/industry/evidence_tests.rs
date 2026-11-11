/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Absent-NewGRF industry tile triggers, rare builtin animation states and tile acceptance
//! (#736 W).
//!
//! Moved from `tools/migration/industry-lifecycle-gap.cpp`, which ran the pinned original's
//! trigger, randomisation, `AnimateTile_Industry` and `AddAcceptedCargo_Industry` bodies
//! beside these production entries in one stub world and compared the two. The original's
//! side is frozen in `lifecycle-oracle.txt` (`python3 tools/oracle-capture.py
//! industry_lifecycle`); these tests rebuild the same stub world around the production
//! entries. Each stub records its call as `kind:a:b:c` in call order, with the kinds the
//! oracle header lists, and a draw as its kind alone: the draws are the fixture's linear
//! congruential generator, so a draw's value follows from the seed and the draw count.
//! The resolver, animation callback, cargo translation and map leaves stand in for the
//! native ones exactly as the probe's did; the tests prove the Rust decisions, the order
//! of their leaf calls and draws, and the values they write.
use super::lifecycle::{AnimationSpec, LifecycleServices, RandomisationSpec, Reseed};
use super::*;
use crate::animated_tile_map::AnimatedTileState;
use crate::cargo_type::{CargoArray, CargoTypes};
use crate::industry_type::{IndustryAnimationTrigger, IndustryGfx, IndustryRandomTrigger};
use crate::industrytype::get_industry_tile_spec;
use crate::newgrf_animation_base::{Binding, Entry, Info};
use crate::newgrf_animation_type::AnimationStatus;
use crate::newgrf_callbacks::{CALLBACK_FAILED, CBID_INDTILE_ANIMATION_TRIGGER, CallbackID};
use crate::oracle;
use crate::test_tables::{
    ANIMATION, INDUSTRIES, INDUSTRY_CONSTRUCTION, INDUSTRY_LIFECYCLE, INDUSTRY_TILES, SHARED,
    unreachable1, unreachable3,
};
use crate::tile_type::MP_INDUSTRY;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

const ORACLE: &str = include_str!("lifecycle-oracle.txt");

/// One tile of the fixture's 3x2 industry.
#[derive(Clone, Copy, Default)]
struct Tile {
    triggers: u8,
    bits: u8,
    belongs: bool,
}

/// The fixture's map, resolver and animation state.
#[derive(Default)]
struct World {
    /// A 3x2 industry with a 4-wide row stride; tile 3 lies outside the industry.
    tiles: [Tile; 7],
    enabled: [bool; 7],
    spec_triggers: [u8; 7],
    trace: Vec<[u32; 4]>,
    rng: u32,
    mask: u32,
    /// Builtin animation tile 2: whether it is animating, and its gfx, frame, loop and
    /// construction stage.
    builtin: bool,
    gfx: u16,
    frame: u8,
    animation_loop: u8,
    stage: u8,
    tick: u64,
    state: u8,
    toffee: Vec<u8>,
    /// Acceptance: the callback results, whether the amount callback rewrites the industry's
    /// first accepted cargo, and the owner it rewrites.
    accept_result: u16,
    amount_result: u16,
    mutate_accepted: bool,
    owner: usize,
}

/// Process-wide, as the probe's globals were; each test holds `GLOBAL_LOCK`.
static WORLD: LazyLock<Mutex<World>> = LazyLock::new(|| Mutex::new(World::default()));
/// `_settings_client.sound.ambient`.
static AMBIENT: AtomicBool = AtomicBool::new(false);

/// No stub calls another while the world is borrowed.
fn with<R>(f: impl FnOnce(&mut World) -> R) -> R {
    f(&mut WORLD.lock().unwrap_or_else(PoisonError::into_inner))
}
fn record(entry: [u32; 4]) {
    with(|w| w.trace.push(entry));
}

/// The native `Industry` shell the probe's stubs passed through.
static SHELL: u8 = 0;
/// The tile specs the animation callback receives, one per tile.
static SPECS: [u8; 7] = [0; 7];
/// The builtin path's spec, which the custom animation entry returns.
static BUILTIN_SPEC: u8 = 0;

fn shell() -> *mut c_void {
    (&raw const SHELL).cast_mut().cast()
}
fn spec_token(tile: u32) -> *const c_void {
    (&raw const SPECS[tile as usize]).cast()
}

extern "C" fn random(_: *mut c_void) -> u32 {
    let value = with(|w| {
        w.rng = w.rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        w.rng
    });
    record([0, value, 0, 0]);
    value
}
extern "C" fn mark_dirty(tile: u32) {
    record([3, tile, 0, 0]);
}
extern "C" fn set_frame(tile: u32, frame: u8) {
    with(|w| w.frame = frame);
    record([6, tile, frame.into(), 0]);
}
extern "C" fn tick_counter() -> u64 {
    with(|w| w.tick)
}
extern "C" fn tile_type(tile: u32) -> u8 {
    assert_eq!(tile, 2, "only the animated tile is read");
    MP_INDUSTRY
}
unsafe extern "C" fn play_sound(sound: u16, tile: u32) {
    record([11, tile, sound.into(), 0]);
}
unsafe extern "C" fn set_construction_stage(tile: u32, stage: u8) {
    with(|w| w.stage = stage);
    record([10, tile, stage.into(), 0]);
}

unsafe extern "C" fn belongs(tile: u32, _: u16) -> bool {
    with(|w| w.tiles[tile as usize].belongs)
}
#[allow(
    clippy::cast_possible_truncation,
    reason = "the fixture's gfx is its tile index"
)]
unsafe extern "C" fn randomisation_spec(tile: u32) -> RandomisationSpec {
    RandomisationSpec {
        gfx: tile as u16,
        enabled: with(|w| w.enabled[tile as usize]),
    }
}
unsafe extern "C" fn random_triggers(tile: u32) -> u8 {
    with(|w| w.tiles[tile as usize].triggers)
}
unsafe extern "C" fn set_random_triggers(tile: u32, bits: u8) {
    with(|w| w.tiles[tile as usize].triggers = bits);
    record([1, tile, bits.into(), 0]);
}
/// The probe's resolver: reseed masks from the case mask, and a live tile write before the
/// caller reads the bits back.
#[allow(
    clippy::cast_possible_truncation,
    reason = "the used triggers are the waiting byte's masked bits"
)]
unsafe extern "C" fn resolve_reseed(
    tile: u32,
    _: u16,
    industry: *mut c_void,
    waiting: u8,
) -> Reseed {
    assert_eq!(
        industry,
        shell(),
        "the resolver receives the industry shell"
    );
    let (triggers, mask) = with(|w| (w.tiles[tile as usize].triggers, w.mask));
    record([4, tile, waiting.into(), triggers.into()]);
    // Resolver may affect live tile state before the source reads it.
    with(|w| w.tiles[tile as usize].bits ^= 0x35);
    Reseed {
        own: mask ^ tile.wrapping_mul(0x0101),
        parent: mask,
        used: (u32::from(waiting) & (1 << (tile % 3))) as u8,
    }
}
unsafe extern "C" fn random_bits(tile: u32) -> u8 {
    with(|w| w.tiles[tile as usize].bits)
}
unsafe extern "C" fn set_random_bits(tile: u32, bits: u8) {
    with(|w| w.tiles[tile as usize].bits = bits);
    record([2, tile, bits.into(), 0]);
}
unsafe extern "C" fn animation_spec(tile: u32) -> AnimationSpec {
    AnimationSpec {
        spec: spec_token(tile),
        triggers: with(|w| w.spec_triggers[tile as usize]),
    }
}

/// `AnimateNewIndustryTile`: recorded, and its spec reports no animation.
extern "C" fn animation_entry(tile: u32) -> Entry<TileSpec, ShellHandle> {
    record([12, tile, 0, 0]);
    Entry {
        spec: (&raw const BUILTIN_SPEC).cast(),
        object: ptr::null_mut(),
    }
}
extern "C" fn animation_info(spec: *const TileSpec) -> Info {
    assert_eq!(spec.cast(), &raw const BUILTIN_SPEC, "builtin spec");
    Info {
        speed: 0,
        frames: 0,
        looping: false,
        speed_callback: false,
        next_callback: false,
        random: false,
        enabled: false,
    }
}
/// The animation trigger callback: recorded, and it fails as no `NewGRF` answers it.
extern "C" fn animation_callback(
    callback: u16,
    random: u32,
    parameter: u32,
    spec: *const TileSpec,
    industry: *mut ShellHandle,
    tile: u32,
    extra: i32,
) -> u16 {
    assert!(callback == CBID_INDTILE_ANIMATION_TRIGGER.0 && extra == 0);
    assert!(spec.cast() == spec_token(tile) && industry.cast() == shell());
    record([5, tile, random, parameter]);
    CALLBACK_FAILED
}
extern "C" fn animation_state(tile: u32) -> AnimatedTileState {
    assert_eq!(tile, 2);
    AnimatedTileState(with(|w| w.state))
}
extern "C" fn set_animation_state(tile: u32, state: AnimatedTileState) {
    assert!(state == AnimatedTileState::DELETED);
    with(|w| w.state = state.0);
    record([8, tile, 0, 0]);
}
extern "C" fn industry_object(_: u32) -> *mut ShellHandle {
    shell().cast()
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the fixture's gfx is its tile index"
)]
unsafe extern "C" fn gfx(tile: u32) -> u16 {
    // The builtin cases animate one gfx; elsewhere the gfx is the tile index.
    with(|w| if w.builtin { w.gfx } else { tile as u16 })
}
unsafe extern "C" fn set_gfx(tile: u32, gfx: u16) {
    with(|w| w.gfx = gfx);
    record([9, tile, gfx.into(), 0]);
}
unsafe extern "C" fn frame(_: u32) -> u8 {
    with(|w| w.frame)
}
unsafe extern "C" fn animation_loop(_: u32) -> u8 {
    with(|w| w.animation_loop)
}
unsafe extern "C" fn set_animation_loop(tile: u32, count: u8) {
    with(|w| w.animation_loop = count);
    record([7, tile, count.into(), 0]);
}
unsafe extern "C" fn toffee_offset(frame: u8) -> u8 {
    with(|w| w.toffee[usize::from(frame)])
}
unsafe extern "C" fn tile_industry(_: u32) -> TileIndustry {
    TileIndustry {
        shell: shell().cast(),
        owner: with(|w| w.owner) as *mut Industry,
    }
}
unsafe extern "C" fn tile_callback(tile: u32, _: u16, callback: u16, _: u32, _: u32) -> u16 {
    record([14, tile, callback.into(), 0]);
    let (accept, amount, mutate, owner) = with(|w| {
        (
            w.accept_result,
            w.amount_result,
            w.mutate_accepted,
            w.owner as *mut Industry,
        )
    });
    if callback == CallbackID::CBID_INDTILE_CARGO_ACCEPTANCE.0 && mutate {
        // SAFETY: The test's live owner; no view of its accepted list is held.
        unsafe { (*accepted(owner, 0)).cargo = 63 };
    }
    if callback == CallbackID::CBID_INDTILE_ACCEPT_CARGO.0 {
        accept
    } else {
        amount
    }
}

/// `LifecycleServices` with the fixture's trigger and randomisation leaves.
const LIFECYCLE: LifecycleServices = LifecycleServices {
    belongs,
    randomisation_spec,
    random_triggers,
    set_random_triggers,
    resolve_reseed,
    random_bits,
    set_random_bits,
    animation_spec,
    ..INDUSTRY_LIFECYCLE
};

/// The fixture's tables, leaked so the installed animation table outlives the test.
struct Tables {
    industries: &'static Services,
    tiles: &'static TileServices,
    animation: &'static crate::animated_tile::Services,
}
fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}
fn tables() -> Tables {
    let mut settings = crate::settings_type::SettingAddresses::NULL;
    settings.ambient = AMBIENT.as_ptr();
    let mut shared = SHARED;
    shared.random = random;
    shared.mark_dirty = mark_dirty;
    shared.tile_animation_set_frame = set_frame;
    shared.tick_counter = tick_counter;
    shared.tile_type = tile_type;
    shared.settings = leak(settings);
    let shared = leak(shared);
    let mut industries = INDUSTRIES;
    industries.shared = shared;
    industries.play_sound = play_sound;
    let industries = leak(industries);
    let mut tiles = INDUSTRY_TILES;
    tiles.shared = shared;
    tiles.gfx = gfx;
    tiles.set_gfx = set_gfx;
    tiles.frame = frame;
    tiles.animation_loop = animation_loop;
    tiles.set_animation_loop = set_animation_loop;
    tiles.toffee_offset = toffee_offset;
    tiles.industry = tile_industry;
    tiles.callback = tile_callback;
    let tiles = leak(tiles);
    let binding: &'static Binding<TileSpec, ShellHandle, i32> = leak(Binding {
        entry: animation_entry,
        info: animation_info,
        callback: animation_callback,
        sound: unreachable3,
        version8: unreachable1,
        speed_error: unreachable3,
    });
    let mut animation = ANIMATION;
    animation.shared = shared;
    animation.industries = industries;
    animation.construction = leak(ConstructionServices {
        set_construction_stage,
        ..INDUSTRY_CONSTRUCTION
    });
    animation.industry_tiles = tiles;
    animation.industry = binding;
    animation.state = animation_state;
    animation.set_state = set_animation_state;
    animation.industry_object = industry_object;
    Tables {
        industries,
        tiles,
        animation: leak(animation),
    }
}

/// The trace as the oracle stores it.
fn trace() -> String {
    with(|w| {
        let mut out = String::new();
        for entry in &w.trace {
            if entry[0] == 0 {
                out.push_str(" 0");
            } else {
                let _ = write!(out, " {}:{}:{}:{}", entry[0], entry[1], entry[2], entry[3]);
            }
        }
        out
    })
}
fn draws() -> usize {
    with(|w| w.trace.iter().filter(|entry| entry[0] == 0).count())
}

/// Swap the fixture's animation table in, and its queue holding only tile 2, for one test.
struct AnimationScope {
    previous: *const crate::animated_tile::Services,
    queue: Vec<u32>,
}
impl AnimationScope {
    fn new(table: &'static crate::animated_tile::Services) -> Self {
        let queue = (0..crate::animated_tile::openttd_rust_animation_count())
            .map(|i| crate::animated_tile::openttd_rust_animation_at(i))
            .collect();
        // SAFETY: The table is leaked; the caller holds the test lock.
        let previous = unsafe { crate::animated_tile::openttd_rust_animation_exchange(table) };
        Self { previous, queue }
    }
}
impl Drop for AnimationScope {
    fn drop(&mut self) {
        crate::animated_tile::openttd_rust_animation_initialize();
        for &tile in &self.queue {
            crate::animated_tile::openttd_rust_animation_append(tile);
        }
        // SAFETY: Restores the table that was installed before this scope.
        unsafe { crate::animated_tile::openttd_rust_animation_exchange(self.previous) };
    }
}

const SEEDS: [u32; 4] = [0, 1, 0x1234_5678, u32::MAX];
const MASKS: [u32; 7] = [0, 1, 0x55, 0xFFFF, 0x10000, 0x1234_5678, u32::MAX];

/// One lifecycle case on the fixture's industry: `[seed, mask, variant]`, the trigger and
/// the operation (tile and industry randomisation, industry and tile animation).
#[allow(
    clippy::cast_possible_truncation,
    reason = "the fixture's per-tile bytes and the industry's random word wrap as its uint8_t and uint16_t fields do"
)]
fn lifecycle_case(
    industries: &Services,
    owner: *mut Industry,
    [seed, mask, variant]: [u32; 3],
    trigger: u8,
    operation: u8,
) -> String {
    with(|w| {
        (w.rng, w.mask) = (seed, mask);
        w.trace.clear();
        for tile in 0..7_u32 {
            w.tiles[tile as usize] = Tile {
                triggers: (tile * 7 + variant) as u8,
                bits: (tile * 23 + variant) as u8,
                belongs: tile != 1 && (variant != 7 || tile != 4),
            };
            w.enabled[tile as usize] = (variant >> (tile % 3)) & 1 != 0;
            w.spec_triggers[tile as usize] = (tile * 13 + variant) as u8;
        }
    });
    // SAFETY: The caller's live owner.
    let metadata = unsafe { &mut *openttd_rust_industry_metadata(owner) };
    metadata.random = seed as u16;
    let mut result = true;
    // SAFETY: The live owner and the fixture's tables; serialized by the lock.
    unsafe {
        match operation {
            0 => trigger_industry_randomisation(
                shell(),
                owner,
                IndustryID(0),
                IndustryRandomTrigger(trigger),
                4,
                industries,
                &LIFECYCLE,
            ),
            1 => trigger_industry_tile_randomisation(
                TileIndex(2),
                IndustryRandomTrigger(trigger),
                shell(),
                owner,
                industries,
                &LIFECYCLE,
            ),
            // `TriggerIndustryTileAnimation_ConstructionStageChanged`
            // passes 0x100 for a first shape.
            3 => {
                let extra = if trigger == 0 && variant & 1 != 0 {
                    0x100
                } else {
                    0
                };
                result = trigger_industry_tile_animation(
                    TileIndex(2),
                    IndustryAnimationTrigger(trigger),
                    extra,
                    industries,
                    &LIFECYCLE,
                );
            }
            _ => {
                result = trigger_industry_animation(
                    owner,
                    IndustryID(0),
                    IndustryAnimationTrigger(trigger),
                    4,
                    industries,
                    &LIFECYCLE,
                );
            }
        }
    }
    // Tiles the call changed, as index:triggers:bits.
    let mut out = format!("{} {} {}", u8::from(result), draws(), metadata.random);
    with(|w| {
        for tile in 0..7_u32 {
            let now = w.tiles[tile as usize];
            if now.triggers != (tile * 7 + variant) as u8 || now.bits != (tile * 23 + variant) as u8
            {
                let _ = write!(out, " {tile}:{}:{}", now.triggers, now.bits);
            }
        }
    });
    out.push_str(" |");
    out.push_str(&trace());
    out
}

#[test]
fn lifecycle_triggers_match_original() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let t = tables();
    let _scope = AnimationScope::new(t.animation);
    let sections = oracle::parse(ORACLE);
    let owner = openttd_rust_industry_new();
    // SAFETY: The live owner allocated above.
    unsafe {
        let metadata = &mut *openttd_rust_industry_metadata(owner);
        (metadata.tile, metadata.width, metadata.height) = (0, 3, 2);
    }
    let mut cases = Vec::new();
    for seed in SEEDS {
        for mask in MASKS {
            for variant in 0..8_u32 {
                for trigger in 0..5_u8 {
                    for operation in 0..4 {
                        if operation != 2 && trigger >= 3 {
                            continue;
                        }
                        let output = lifecycle_case(
                            t.industries,
                            owner,
                            [seed, mask, variant],
                            trigger,
                            operation,
                        );
                        cases.push((
                            format!(
                                "seed {seed} mask {mask} variant {variant} trigger {trigger} operation {operation}"
                            ),
                            output,
                        ));
                    }
                }
            }
        }
    }
    // SAFETY: The owner allocated above; no view of it remains.
    unsafe { openttd_rust_industry_destroy(owner) };
    assert_eq!(oracle::check(&sections, "lifecycle", cases), 3136);
}

/// One builtin animation case on tile 2.
fn builtin_case(
    gfx: u16,
    frame: u8,
    animation_loop: u8,
    ambient: bool,
    seed: u32,
    tick: u64,
) -> String {
    crate::animated_tile::openttd_rust_animation_initialize();
    crate::animated_tile::openttd_rust_animation_append(2);
    with(|w| {
        w.state = AnimatedTileState::ANIMATED.0;
        w.rng = seed;
        w.trace.clear();
        (w.gfx, w.frame, w.animation_loop, w.stage) = (gfx, frame, animation_loop, 0);
        w.tick = tick;
    });
    AMBIENT.store(ambient, Ordering::SeqCst);
    crate::animated_tile::openttd_rust_animate_tile(2);
    assert!(
        crate::animated_tile::openttd_rust_animation_count() == 1
            && crate::animated_tile::openttd_rust_animation_at(0) == 2,
        "the queue keeps its one tile"
    );
    let mut fields: Vec<String> = with(|w| {
        [
            (w.gfx != gfx).then(|| format!("g={}", w.gfx)),
            (w.frame != frame).then(|| format!("f={}", w.frame)),
            (w.animation_loop != animation_loop).then(|| format!("l={}", w.animation_loop)),
            (w.stage != 0).then(|| format!("s={}", w.stage)),
        ]
        .into_iter()
        .flatten()
        .collect()
    });
    fields.push(draws().to_string());
    fields.push(with(|w| w.state).to_string());
    format!("{} |{}", fields.join(" "), trace())
}

#[test]
fn builtin_animation_matches_original() {
    const GFXS: [u16; 15] = [
        1, 10, 30, 31, 32, 48, 88, 143, 148, 151, 155, 162, 165, 174, 175,
    ];
    const FRAMES: [u8; 26] = [
        0, 1, 3, 4, 5, 6, 7, 14, 23, 28, 39, 49, 50, 56, 69, 70, 96, 127, 128, 130, 191, 192, 194,
        195, 254, 255,
    ];
    const TICKS: [u64; 22] = [
        0,
        1,
        2,
        3,
        7,
        8,
        1023,
        1024,
        1025,
        1056,
        1407,
        1408,
        1439,
        1440,
        1536,
        1568,
        1919,
        1920,
        1951,
        1952,
        2047,
        u64::MAX,
    ];
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let t = tables();
    let _scope = AnimationScope::new(t.animation);
    let sections = oracle::parse(ORACLE);
    let toffee_section = &sections["toffee"];
    let toffee: Vec<u8> = (0..toffee_section.len())
        .map(|n| toffee_section.case(n).unwrap().parse().unwrap())
        .collect();
    let saved: Vec<u8> = GFXS
        .iter()
        // SAFETY: Serialized by the lock; each spec is restored below.
        .map(|&gfx| unsafe { (*get_industry_tile_spec(IndustryGfx(gfx))).animation.status })
        .collect();
    with(|w| (w.builtin, w.toffee) = (true, toffee.clone()));
    let mut cases = Vec::new();
    // Stored custom, gfx, tick, frame, loop, ambient, seed outermost-first.
    for custom in [false, true] {
        for gfx in GFXS {
            let status = if custom {
                AnimationStatus::Looping
            } else {
                AnimationStatus::NoAnimation
            };
            // SAFETY: Serialized by the lock; restored below.
            unsafe { (*get_industry_tile_spec(IndustryGfx(gfx))).animation.status = status.0 };
            for tick in TICKS {
                for frame in FRAMES {
                    // The original has no defined read outside its rendering table.
                    if IndustryGfx(gfx) == IndustryGfx::GFX_TOFFEE_QUARRY
                        && usize::from(frame) >= toffee.len()
                    {
                        continue;
                    }
                    for animation_loop in [0, 7, 255] {
                        for ambient in [false, true] {
                            for seed in SEEDS {
                                let output =
                                    builtin_case(gfx, frame, animation_loop, ambient, seed, tick);
                                cases.push((
                                    format!(
                                        "custom {custom} gfx {gfx} tick {tick} frame {frame} loop {animation_loop} ambient {ambient} seed {seed}"
                                    ),
                                    output,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    for (&gfx, &status) in GFXS.iter().zip(&saved) {
        // SAFETY: Serialized by the lock.
        unsafe { (*get_industry_tile_spec(IndustryGfx(gfx))).animation.status = status };
    }
    with(|w| w.builtin = false);
    assert_eq!(oracle::check(&sections, "builtin", cases), 400_224);
}

/// One acceptance case on tile 2, spec 2: `[flags, pattern]` select the spec's flags,
/// masks and accepted cargoes, `[accept, amounts]` the two callback results.
fn acceptance_case(
    tiles: &TileServices,
    owner: *mut Industry,
    [flags, pattern]: [u8; 2],
    [accept, amounts]: [u16; 2],
    amount: i8,
    mutation: bool,
) -> String {
    let spec = get_industry_tile_spec(IndustryGfx(2));
    with(|w| {
        w.trace.clear();
        (w.accept_result, w.amount_result, w.mutate_accepted) = (accept, amounts, mutation);
    });
    // SAFETY: The live owner and spec; serialized by the lock.
    unsafe {
        openttd_rust_industry_accepted_resize(owner, 4);
        for (slot, cargo) in [2, 5, 255, 2].into_iter().enumerate() {
            (*accepted(owner, slot)).cargo = cargo;
        }
        (*spec).special_flags = (flags & 1) << 1;
        (*spec).callback_mask = ((flags & 2) << 2) | (flags & 4);
        for n in 0..16_u8 {
            (*spec).accepts_cargo[usize::from(n)] = match pattern {
                0 => CargoType::INVALID_CARGO.0,
                1 => n,
                2 => 2,
                _ if n % 3 == 0 => CargoType::INVALID_CARGO.0,
                _ => n,
            };
            (*spec).acceptance[usize::from(n)] = amount;
        }
        (*spec).grf_prop.grffile = ptr::null();
    }
    let initial = u32::MAX - 4;
    let mut acceptance: CargoArray = [initial; 64];
    let mut always = CargoTypes(4);
    // SAFETY: The fixture's tile, owner and tables.
    unsafe {
        add_accepted_cargo_industry(TileIndex(2), &mut acceptance, &mut always, tiles);
    }
    // The always-accepted mask, then the cargoes the call changed as cargo=amount.
    let mut out = always.0.to_string();
    for (cargo, value) in acceptance.iter().enumerate() {
        if *value != initial {
            let _ = write!(out, " {cargo}={value}");
        }
    }
    out.push_str(" |");
    out.push_str(&trace());
    out
}

#[test]
fn tile_acceptance_matches_original() {
    /// The fixture's cargo translation: slot `n` is label `(n * 3 + 2) % 64 + 1`, which
    /// is cargo `(n * 3 + 2) % 64`; slot 31 is the invalid label.
    static LABELS: [u32; 32] = {
        let mut labels = [u32::MAX; 32];
        let mut slot = 0_u32;
        while slot < 31 {
            labels[slot as usize] = (slot * 3 + 2) % 64 + 1;
            slot += 1;
        }
        labels
    };
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let t = tables();
    let sections = oracle::parse(ORACLE);
    let cargos: Vec<u8> = LABELS[..31]
        .iter()
        .map(|&label| u8::try_from(label - 1).unwrap())
        .collect();
    crate::cargo_registry::install_test_labels(&LABELS, &cargos);
    let owner = openttd_rust_industry_new();
    with(|w| w.owner = owner as usize);
    let spec = get_industry_tile_spec(IndustryGfx(2));
    // SAFETY: Serialized by the lock; restored below.
    let saved = unsafe {
        (
            (*spec).accepts_cargo,
            (*spec).acceptance,
            (*spec).special_flags,
            (*spec).callback_mask,
            (*spec).grf_prop.grffile,
        )
    };
    let mut cases = Vec::new();
    for flags in 0..8_u8 {
        for pattern in 0..4_u8 {
            for accept in [0, 0x7FFF, 0x0401, 0xFFFF] {
                for amounts in [0, 0x0FFF, 0x0312, 0xFFFF] {
                    for amount in [-128, -9, -8, -1, 0, 1, 119, 120, 127_i8] {
                        for mutation in [false, true] {
                            let output = acceptance_case(
                                t.tiles,
                                owner,
                                [flags, pattern],
                                [accept, amounts],
                                amount,
                                mutation,
                            );
                            cases.push((
                                format!(
                                    "flags {flags} pattern {pattern} result {accept}/{amounts} amount {amount} mutation {mutation}"
                                ),
                                output,
                            ));
                        }
                    }
                }
            }
        }
    }
    // SAFETY: Serialized by the lock; no view of the owner or spec remains.
    unsafe {
        (
            (*spec).accepts_cargo,
            (*spec).acceptance,
            (*spec).special_flags,
            (*spec).callback_mask,
            (*spec).grf_prop.grffile,
        ) = saved;
        openttd_rust_industry_destroy(owner);
    }
    with(|w| w.owner = 0);
    crate::cargo_registry::clear_test_labels();
    assert_eq!(oracle::check(&sections, "acceptance", cases), 9216);
}
