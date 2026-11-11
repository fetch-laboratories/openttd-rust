/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry tile and parent triggers, builtin tile animation and tile acceptance over the
//! retired `tools/migration/industry-lifecycle-gap.cpp` domain (#736 W), against its
//! original-only capture in `industry/lifecycle-oracle.txt`. The service tables model the
//! fixture's synthetic 3x2 industry exactly as the probe's candidate bindings did, and each
//! case's fixture calls are traced in the oracle's `kind:a:b:c` form.
#![allow(unsafe_code)]

use super::{Industry, LifecycleServices, Services};
use crate::animated_tile_map::AnimatedTileState;
use crate::industry_type::IndustryGfx;
use crate::industry_type::{IndustryAnimationTrigger, IndustryID, IndustryRandomTrigger};
use crate::newgrf_animation_base::{Binding, Entry, Info};
use crate::oracle;
use crate::tile_type::TileIndex;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::fmt::Write as _;

const ORACLE: &str = include_str!("lifecycle-oracle.txt");
const SEEDS: [u32; 4] = [0, 1, 0x1234_5678, u32::MAX];
const MASKS: [u32; 7] = [0, 1, 0x55, 0xFFFF, 0x1_0000, 0x1234_5678, u32::MAX];
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
const GFX: [u16; 15] = [
    1, 10, 30, 31, 32, 48, 88, 143, 148, 151, 155, 162, 165, 174, 175,
];
const FRAMES: [u8; 26] = [
    0, 1, 3, 4, 5, 6, 7, 14, 23, 28, 39, 49, 50, 56, 69, 70, 96, 127, 128, 130, 191, 192, 194, 195,
    254, 255,
];
const ACCEPT_RESULTS: [u16; 4] = [0, 0x7FFF, 0x0401, 0xFFFF];
const AMOUNT_RESULTS: [u16; 4] = [0, 0x0FFF, 0x0312, 0xFFFF];
const AMOUNTS: [i8; 9] = [-128, -9, -8, -1, 0, 1, 119, 120, 127];
/// `GFX_TOFFEE_QUARRY`.
const TOFFEE_QUARRY: u16 = 165;
/// The fixture's builtin animation and acceptance tile.
const TILE: u32 = 2;
const CALLBACK_FAILED: u16 = 0xFFFF;
const CBID_INDTILE_ANIMATION_TRIGGER: u16 = 0x25;
const CBID_INDTILE_CARGO_ACCEPTANCE: u16 = 0x2B;
const CBID_INDTILE_ACCEPT_CARGO: u16 = 0x2C;

/// One fixture tile: random triggers, random bits and industry membership.
#[derive(Clone, Copy, Default)]
struct Tile {
    triggers: u8,
    bits: u8,
    belongs: bool,
}
/// One fixture tile spec: whether it has sprite groups, and its animation triggers.
#[derive(Clone, Copy, Default)]
struct Spec {
    enabled: bool,
    triggers: u8,
}
/// The builtin animation tile's gfx, frame, loop and construction stage.
#[derive(Clone, Copy, Default)]
struct Builtin {
    gfx: u16,
    frame: u8,
    animation_loop: u8,
    stage: u8,
}

thread_local! {
    static TILES: Cell<[Tile; 7]> = const { Cell::new([Tile { triggers: 0, bits: 0, belongs: false }; 7]) };
    static SPECS: RefCell<[Spec; 7]> = const { RefCell::new([Spec { enabled: false, triggers: 0 }; 7]) };
    static TRACE: RefCell<Vec<[u32; 4]>> = const { RefCell::new(Vec::new()) };
    static RNG: Cell<u32> = const { Cell::new(0) };
    static MASK: Cell<u32> = const { Cell::new(0) };
    static BUILTIN: Cell<Builtin> = const { Cell::new(Builtin { gfx: 0, frame: 0, animation_loop: 0, stage: 0 }) };
    static BUILTIN_MODE: Cell<bool> = const { Cell::new(false) };
    static STATE: Cell<AnimatedTileState> = const { Cell::new(AnimatedTileState::NONE) };
    static AMBIENT: Cell<bool> = const { Cell::new(false) };
    static TICK: Cell<u64> = const { Cell::new(0) };
    static ACCEPT: Cell<u16> = const { Cell::new(0) };
    static AMOUNT: Cell<u16> = const { Cell::new(0) };
    static MUTATE: Cell<bool> = const { Cell::new(false) };
    static OWNER: Cell<*mut Industry> = const { Cell::new(std::ptr::null_mut()) };
    static TOFFEE: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}
/// The fixture's `Industry *active`: an identity only, never dereferenced.
static ACTIVE: u8 = 0;
/// The fixture's `builtin_spec`, which the animation entry reports: an identity only.
static BUILTIN_SPEC: u8 = 0;
fn builtin_spec() -> *const super::TileSpec {
    (&raw const BUILTIN_SPEC).cast()
}
fn active() -> *mut c_void {
    (&raw const ACTIVE).cast_mut().cast()
}
/// The opaque spec identity the animation spec service hands to the frame callback.
fn spec_address(tile: u32) -> *const c_void {
    SPECS.with_borrow(|specs| std::ptr::from_ref(&specs[tile as usize]).cast())
}
fn push(entry: [u32; 4]) {
    TRACE.with_borrow_mut(|trace| trace.push(entry));
}
fn tile(tile: u32) -> Tile {
    TILES.get()[tile as usize]
}
fn update(tile: u32, f: impl FnOnce(&mut Tile)) {
    let mut tiles = TILES.get();
    f(&mut tiles[tile as usize]);
    TILES.set(tiles);
}
/// The trace as the capture prints it, a draw as `R`.
fn trace() -> String {
    TRACE.with_borrow(|trace| {
        let mut out = String::new();
        for e in trace {
            if !out.is_empty() {
                out.push(' ');
            }
            if e[0] == 0 {
                out.push('R');
            } else {
                let _ = write!(out, "{}:{}:{}:{}", e[0], e[1], e[2], e[3]);
            }
        }
        out
    })
}

// Shared services: the fixture's Random(), MarkTileDirtyByTile, SetAnimationFrame and tick.
extern "C" fn random(_: *mut c_void) -> u32 {
    let rng = RNG
        .get()
        .wrapping_mul(1_664_525)
        .wrapping_add(1_013_904_223);
    RNG.set(rng);
    push([0, rng, 0, 0]);
    rng
}
extern "C" fn no_industry(_: i32, _: i32, _: *mut u32) -> u32 {
    unreachable!("unexpected industry lookup")
}
extern "C" fn mark_dirty(tile: u32) {
    push([3, tile, 0, 0]);
}
extern "C" fn set_frame(tile: u32, frame: u8) {
    let mut builtin = BUILTIN.get();
    builtin.frame = frame;
    BUILTIN.set(builtin);
    push([6, tile, u32::from(frame), 0]);
}
extern "C" fn tick_counter() -> u64 {
    TICK.get()
}
extern "C" fn tile_type(tile: u32) -> u8 {
    assert_eq!(tile, TILE, "builtin animation reads only its tile's type");
    crate::tile_type::MP_INDUSTRY
}
extern "C" fn native_assert(condition: bool) {
    assert!(condition, "native assertion");
}

// Industry services.
extern "C" fn play_sound(sound: u16, tile: u32) {
    push([11, tile, u32::from(sound), 0]);
}

// Lifecycle services: the fixture's tile model and resolver.
extern "C" fn belongs(tile: u32, _: u16) -> bool {
    self::tile(tile).belongs
}
extern "C" fn randomisation_spec(tile: u32) -> super::RandomisationSpec {
    super::RandomisationSpec {
        gfx: tile as u16,
        enabled: SPECS.with_borrow(|specs| specs[tile as usize].enabled),
    }
}
extern "C" fn random_triggers(tile: u32) -> u8 {
    self::tile(tile).triggers
}
extern "C" fn set_random_triggers(tile: u32, bits: u8) {
    update(tile, |t| t.triggers = bits);
    push([1, tile, u32::from(bits), 0]);
}
/// The fixture's `IndustryTileResolverObject::ResolveRerandomisation`: it traces the waiting
/// and stored triggers, reseeds by a mask and the tile, flips live bits, and reports the
/// trigger of the tile's column as used.
extern "C" fn resolve_reseed(
    tile: u32,
    _: u16,
    industry: *mut c_void,
    waiting: u8,
) -> super::Reseed {
    assert_eq!(industry, active());
    push([
        4,
        tile,
        u32::from(waiting),
        u32::from(self::tile(tile).triggers),
    ]);
    // Resolver may affect live tile state before the source reads it.
    update(tile, |t| t.bits ^= 0x35);
    super::Reseed {
        own: MASK.get() ^ tile.wrapping_mul(0x0101),
        parent: MASK.get(),
        used: waiting & (1 << (tile % 3)),
    }
}
extern "C" fn random_bits(tile: u32) -> u8 {
    self::tile(tile).bits
}
extern "C" fn set_random_bits(tile: u32, bits: u8) {
    update(tile, |t| t.bits = bits);
    push([2, tile, u32::from(bits), 0]);
}
extern "C" fn animation_spec(tile: u32) -> super::AnimationSpec {
    super::AnimationSpec {
        spec: spec_address(tile),
        triggers: SPECS.with_borrow(|specs| specs[tile as usize].triggers),
    }
}

// Animation binding: the fixture's no-op resolver, observed through its frame changes.
extern "C" fn animation_entry(tile: u32) -> Entry<super::TileSpec, crate::industry_type::Industry> {
    // AnimateNewIndustryTile.
    push([12, tile, 0, 0]);
    Entry {
        spec: builtin_spec(),
        object: std::ptr::null_mut(),
    }
}
extern "C" fn animation_info(spec: *const super::TileSpec) -> Info {
    assert_eq!(spec, builtin_spec());
    // The former AnimateNewIndustryTile observation made no frame changes.
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
#[allow(clippy::too_many_arguments)]
extern "C" fn animation_callback(
    callback: u16,
    random: u32,
    parameter: u32,
    spec: *const super::TileSpec,
    industry: *mut crate::industry_type::Industry,
    tile: u32,
    extra: i32,
) -> u16 {
    assert!(callback == CBID_INDTILE_ANIMATION_TRIGGER && extra == 0);
    assert!(spec.cast() == spec_address(tile) && industry.cast() == active());
    // IndustryAnimationBase::ChangeAnimationFrame.
    push([5, tile, random, parameter]);
    CALLBACK_FAILED
}
extern "C" fn no_sound(_: *const super::TileSpec, _: u8, _: u32) {
    unreachable!("unexpected animation sound")
}
extern "C" fn no_version8(_: *const super::TileSpec) -> bool {
    unreachable!("unexpected animation version")
}
extern "C" fn no_speed_error(_: *const super::TileSpec, _: u16, _: u16) {
    unreachable!("unexpected animation speed error")
}
extern "C" fn animation_state(tile: u32) -> AnimatedTileState {
    assert_eq!(tile, TILE);
    STATE.get()
}
extern "C" fn set_animation_state(tile: u32, state: AnimatedTileState) {
    assert!(state == AnimatedTileState::DELETED);
    // DeleteAnimatedTile.
    STATE.set(AnimatedTileState::DELETED);
    push([8, tile, 0, 0]);
}
extern "C" fn industry_object(_: u32) -> *mut crate::industry_type::Industry {
    active().cast()
}

// Industry tile and construction services of the builtin animations and acceptance.
extern "C" fn gfx(tile: u32) -> u16 {
    if BUILTIN_MODE.get() {
        BUILTIN.get().gfx
    } else {
        tile as u16
    }
}
extern "C" fn set_gfx(tile: u32, gfx: u16) {
    let mut builtin = BUILTIN.get();
    builtin.gfx = gfx;
    BUILTIN.set(builtin);
    push([9, tile, u32::from(gfx), 0]);
}
extern "C" fn frame(_: u32) -> u8 {
    BUILTIN.get().frame
}
extern "C" fn animation_loop(_: u32) -> u8 {
    BUILTIN.get().animation_loop
}
extern "C" fn set_animation_loop(tile: u32, value: u8) {
    let mut builtin = BUILTIN.get();
    builtin.animation_loop = value;
    BUILTIN.set(builtin);
    push([7, tile, u32::from(value), 0]);
}
extern "C" fn toffee_offset(frame: u8) -> u8 {
    TOFFEE.with_borrow(|table| table[usize::from(frame)])
}
extern "C" fn set_construction_stage(tile: u32, stage: u8) {
    let mut builtin = BUILTIN.get();
    builtin.stage = stage;
    BUILTIN.set(builtin);
    push([10, tile, u32::from(stage), 0]);
}
extern "C" fn tile_industry(_: u32) -> super::TileIndustry {
    super::TileIndustry {
        shell: active().cast(),
        owner: OWNER.get(),
    }
}
/// The fixture's `GetIndustryTileCallback`: an acceptance callback may first change the
/// industry's first accepted cargo.
extern "C" fn tile_callback(tile: u32, _: u16, callback: u16, _: u32, _: u32) -> u16 {
    push([14, tile, u32::from(callback), 0]);
    if callback == CBID_INDTILE_CARGO_ACCEPTANCE && MUTATE.get() {
        // SAFETY: The test's owner is live; no view of its accepted list is outstanding.
        unsafe { (&mut (*OWNER.get()).accepted)[0].cargo = 63 };
    }
    if callback == CBID_INDTILE_ACCEPT_CARGO {
        ACCEPT.get()
    } else {
        AMOUNT.get()
    }
}

/// The tables of one test run, leaked for the process-wide registries that hold them.
struct Tables {
    services: &'static Services,
    lifecycle: &'static LifecycleServices,
    construction: &'static super::ConstructionServices,
    tiles: &'static super::TileServices,
}
fn tables() -> Tables {
    let movement = Box::leak(Box::new(crate::movement::MovementServices {
        native_assert,
        ..crate::test_tables::MOVEMENT
    }));
    let settings = Box::leak(Box::new(crate::settings_type::SettingAddresses {
        ambient: AMBIENT.with(Cell::as_ptr),
        ..crate::settings_type::SettingAddresses::NULL
    }));
    let shared = Box::leak(Box::new(crate::services::Services {
        mark_dirty,
        tile_animation_set_frame: set_frame,
        tick_counter,
        tile_type,
        settings,
        movement,
        ..crate::services::fixture(std::ptr::null_mut(), random, no_industry)
    }));
    let services = Box::leak(Box::new(Services {
        shared,
        play_sound,
        ..crate::test_tables::INDUSTRIES
    }));
    let lifecycle = Box::leak(Box::new(LifecycleServices {
        belongs,
        randomisation_spec,
        random_triggers,
        set_random_triggers,
        resolve_reseed,
        random_bits,
        set_random_bits,
        animation_spec,
        ..crate::test_tables::INDUSTRY_LIFECYCLE
    }));
    let construction = Box::leak(Box::new(super::ConstructionServices {
        set_construction_stage,
        ..crate::test_tables::INDUSTRY_CONSTRUCTION
    }));
    let tiles = Box::leak(Box::new(super::TileServices {
        shared,
        gfx,
        set_gfx,
        frame,
        animation_loop,
        set_animation_loop,
        toffee_offset,
        industry: tile_industry,
        callback: tile_callback,
        ..crate::test_tables::INDUSTRY_TILES
    }));
    let binding: Binding<super::TileSpec, crate::industry_type::Industry, i32> = Binding {
        entry: animation_entry,
        info: animation_info,
        callback: animation_callback,
        sound: no_sound,
        version8: no_version8,
        speed_error: no_speed_error,
    };
    let binding = Box::leak(Box::new(binding));
    let animation = Box::leak(Box::new(crate::animated_tile::Services {
        shared,
        industries: services,
        construction,
        industry_tiles: tiles,
        industry: binding,
        state: animation_state,
        set_state: set_animation_state,
        industry_object,
        ..crate::test_tables::ANIMATION
    }));
    // SAFETY: The isolated test process owns the registry; the table is leaked.
    unsafe { crate::animated_tile::openttd_rust_animation_exchange(animation) };
    Tables {
        services,
        lifecycle,
        construction,
        tiles,
    }
}

/// The fixture's tiles and specs for one lifecycle case: tile 1 lies outside the
/// industry, as does tile 4 in variant 7.
fn lifecycle_setup(seed: u32, mask: u32, variant: u8) {
    RNG.set(seed);
    MASK.set(mask);
    TRACE.with_borrow_mut(Vec::clear);
    let mut tiles = [Tile::default(); 7];
    SPECS.with_borrow_mut(|specs| {
        for (index, (tile, spec)) in tiles.iter_mut().zip(specs.iter_mut()).enumerate() {
            let n = index as u8;
            *tile = Tile {
                triggers: n.wrapping_mul(7).wrapping_add(variant),
                bits: n.wrapping_mul(23).wrapping_add(variant),
                belongs: index != 1 && (variant != 7 || index != 4),
            };
            *spec = Spec {
                enabled: (variant >> (index % 3)) & 1 != 0,
                triggers: n.wrapping_mul(13).wrapping_add(variant),
            };
        }
    });
    TILES.set(tiles);
}

fn check_lifecycle(t: &Tables, section: &oracle::Section<'_>) -> usize {
    let mut case = 0;
    for seed in SEEDS {
        for mask in MASKS {
            for variant in 0..8_u8 {
                for trigger in 0..5_u8 {
                    let mut industry = Industry::default();
                    industry.metadata.tile = 0;
                    industry.metadata.width = 3;
                    industry.metadata.height = 2;
                    let owner = Box::into_raw(Box::new(industry));
                    for operation in 0..4_u8 {
                        if operation != 2 && trigger >= 3 {
                            continue;
                        }
                        lifecycle_setup(seed, mask, variant);
                        // SAFETY: `owner` is this loop's live box; tables are leaked.
                        unsafe { (*owner).metadata.random = seed as u16 };
                        let result = unsafe {
                            match operation {
                                0 => {
                                    super::trigger_industry_randomisation(
                                        active(),
                                        owner,
                                        IndustryID(0),
                                        IndustryRandomTrigger(trigger),
                                        4,
                                        t.services,
                                        t.lifecycle,
                                    );
                                    true
                                }
                                1 => {
                                    super::trigger_industry_tile_randomisation(
                                        TileIndex(TILE),
                                        IndustryRandomTrigger(trigger),
                                        active(),
                                        owner,
                                        t.services,
                                        t.lifecycle,
                                    );
                                    true
                                }
                                2 => super::trigger_industry_animation(
                                    owner,
                                    IndustryID(0),
                                    IndustryAnimationTrigger(trigger),
                                    4,
                                    t.services,
                                    t.lifecycle,
                                ),
                                _ => super::trigger_industry_tile_animation(
                                    TileIndex(TILE),
                                    IndustryAnimationTrigger(trigger),
                                    if trigger == 0 && variant & 1 != 0 {
                                        0x100
                                    } else {
                                        0
                                    },
                                    t.services,
                                    t.lifecycle,
                                ),
                            }
                        };
                        let mut actual = format!("{} {} ", u8::from(result), unsafe {
                            (*owner).metadata.random
                        });
                        for tile in TILES.get() {
                            let _ = write!(actual, "{}.{} ", tile.triggers, tile.bits);
                        }
                        let _ = write!(actual, "| {}", trace());
                        assert_eq!(
                            Some(actual.as_str()),
                            section.case(case),
                            "lifecycle seed {seed} mask {mask} variant {variant} trigger {trigger} operation {operation}"
                        );
                        case += 1;
                    }
                    // SAFETY: Reclaim this loop's box.
                    drop(unsafe { Box::from_raw(owner) });
                }
            }
        }
    }
    case
}

#[allow(clippy::too_many_lines)]
fn check_builtin_animation(t: &Tables, section: &oracle::Section<'_>) -> usize {
    let _ = t;
    BUILTIN_MODE.set(true);
    let mut case = 0;
    // The oracle's nesting: gfx, frame, loop, custom, tick, ambient, seed.
    for gfx in GFX {
        for frame in FRAMES {
            if gfx == TOFFEE_QUARRY && usize::from(frame) >= TOFFEE.with_borrow(Vec::len) {
                continue; // Reference has no defined read outside its rendering table.
            }
            for animation_loop in [0, 7, 255] {
                for custom in [false, true] {
                    for tick in TICKS {
                        for ambient in [false, true] {
                            for seed in SEEDS {
                                crate::animated_tile::openttd_rust_animation_initialize();
                                crate::animated_tile::openttd_rust_animation_append(TILE);
                                STATE.set(AnimatedTileState::ANIMATED);
                                RNG.set(seed);
                                TRACE.with_borrow_mut(Vec::clear);
                                BUILTIN.set(Builtin {
                                    gfx,
                                    frame,
                                    animation_loop,
                                    stage: 0,
                                });
                                AMBIENT.set(ambient);
                                TICK.set(tick);
                                // SAFETY: The isolated process owns the canonical specs.
                                unsafe {
                                    (*crate::industrytype::get_industry_tile_spec(IndustryGfx(
                                        gfx,
                                    )))
                                    .animation
                                    .status = if custom {
                                        crate::newgrf_animation_type::AnimationStatus::Looping.0
                                    } else {
                                        crate::newgrf_animation_type::AnimationStatus::NoAnimation.0
                                    };
                                }
                                crate::animated_tile::openttd_rust_animate_tile(TILE);
                                let b = BUILTIN.get();
                                let actual = format!(
                                    "{} {} {} {} {} | {}",
                                    b.gfx,
                                    b.frame,
                                    b.animation_loop,
                                    b.stage,
                                    STATE.get().0,
                                    trace()
                                );
                                assert_eq!(
                                    Some(actual.as_str()),
                                    section.case(case),
                                    "builtin gfx {gfx} tick {tick} frame {frame} loop {animation_loop} ambient {ambient} custom {custom} seed {seed}"
                                );
                                assert_eq!(crate::animated_tile::openttd_rust_animation_count(), 1);
                                assert_eq!(
                                    crate::animated_tile::openttd_rust_animation_at(0),
                                    TILE
                                );
                                case += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    BUILTIN_MODE.set(false);
    case
}

fn check_acceptance(t: &Tables, section: &oracle::Section<'_>) -> usize {
    // The fixture's cargo translation: c -> c == 31 ? INVALID_CARGO : (c * 3 + 2) % 64.
    static LABELS: [u32; 32] = {
        let mut labels = [u32::MAX; 32];
        let mut cargo = 0;
        while cargo < 31 {
            labels[cargo] = ((cargo * 3 + 2) % 64 + 1) as u32;
            cargo += 1;
        }
        labels
    };
    let cargos: Vec<u8> = (0..31_u32)
        .map(|cargo| ((cargo * 3 + 2) % 64) as u8)
        .collect();
    crate::cargo_registry::install_test_labels(&LABELS, &cargos);
    let owner = Box::into_raw(Box::new(Industry::default()));
    OWNER.set(owner);
    let mut case = 0;
    for flags in 0..8_u8 {
        for pattern in 0..4_u8 {
            for accepted in ACCEPT_RESULTS {
                for amounts in AMOUNT_RESULTS {
                    for amount in AMOUNTS {
                        for mutation in [false, true] {
                            TRACE.with_borrow_mut(Vec::clear);
                            ACCEPT.set(accepted);
                            AMOUNT.set(amounts);
                            MUTATE.set(mutation);
                            {
                                // SAFETY: The test's owner is live; this borrow ends here.
                                let list = unsafe { &mut (*owner).accepted };
                                list.clear();
                                for cargo in [2, 5, 255, 2] {
                                    list.push(super::Accepted {
                                        cargo,
                                        ..super::Accepted::default()
                                    });
                                }
                            }
                            // SAFETY: The isolated process owns the canonical specs.
                            unsafe {
                                let spec = &mut *crate::industrytype::get_industry_tile_spec(
                                    IndustryGfx(TILE as u16),
                                );
                                spec.special_flags = (flags & 1) << 1;
                                spec.callback_mask = ((flags & 2) << 2) | (flags & 4);
                                for n in 0..16_u8 {
                                    spec.accepts_cargo[usize::from(n)] = match pattern {
                                        0 => 255,
                                        1 => n,
                                        2 => 2,
                                        _ if n % 3 == 0 => 255,
                                        _ => n,
                                    };
                                    spec.acceptance[usize::from(n)] = amount;
                                }
                                spec.grf_prop.grffile = std::ptr::null();
                            }
                            let mut acceptance: crate::cargo_type::CargoArray =
                                [u32::MAX - 4; crate::cargo_type::NUM_CARGO as usize];
                            let mut always = crate::cargo_type::CargoTypes(4);
                            // SAFETY: Live owner and leaked tables; serial call.
                            unsafe {
                                super::add_accepted_cargo_industry(
                                    TileIndex(TILE),
                                    &mut acceptance,
                                    &mut always,
                                    t.tiles,
                                );
                            }
                            let mut actual = String::new();
                            for (cargo, value) in acceptance.iter().enumerate() {
                                if *value != u32::MAX - 4 {
                                    let _ = write!(actual, "{cargo}={value} ");
                                }
                            }
                            let _ = write!(actual, "{} | {}", always.0, trace());
                            assert_eq!(
                                Some(actual.as_str()),
                                section.case(case),
                                "acceptance flags {flags} pattern {pattern} result {accepted}/{amounts} amount {amount} mutation {mutation}"
                            );
                            case += 1;
                        }
                    }
                }
            }
        }
    }
    OWNER.set(std::ptr::null_mut());
    // SAFETY: Reclaim the owner; nothing retains it.
    drop(unsafe { Box::from_raw(owner) });
    crate::cargo_registry::clear_test_labels();
    case
}

/// Every retired probe case, in an isolated process: the animation registry, cargo
/// registry and canonical tile specs are process-wide.
#[test]
fn lifecycle_animation_and_acceptance_match_the_original_capture() {
    extern "C" fn check(condition: bool) {
        assert!(condition, "native assertion");
    }
    if crate::services_assert::isolate_test_check(
        "industry::lifecycle_oracle::lifecycle_animation_and_acceptance_match_the_original_capture",
        check,
    ) {
        return;
    }
    let sections = oracle::parse(ORACLE);
    let table = sections["T"].case(0).expect("toffee table");
    TOFFEE.set(
        table
            .split(' ')
            .map(|v| v.parse().expect("toffee offset"))
            .collect(),
    );
    let t = tables();
    let _ = t.construction;
    assert_eq!(check_lifecycle(&t, &sections["L"]), sections["L"].len());
    assert_eq!(
        check_builtin_animation(&t, &sections["A"]),
        sections["A"].len()
    );
    assert_eq!(check_acceptance(&t, &sections["C"]), sections["C"].len());
    assert_eq!(
        sections["L"].len() + sections["A"].len() + sections["C"].len(),
        3_136 + 400_224 + 9_216
    );
}
