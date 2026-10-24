/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Generic callback registry and invocation policy. The game-state mutex serializes
//! all entries. Load/reset cannot run during resolution; files/graphs stay live.
//! Only registration allocates. Each candidate copies one pointer pair, ending the
//! registry borrow before Action2; invocations are independent stack values. No
//! context/register borrow spans evaluation, sound, or other reentrant leaves.
#![allow(unsafe_code, missing_docs, clippy::cast_possible_truncation)]
use crate::game_state::GameState;
use crate::industry_type::IT_INVALID;
use crate::newgrf::{GSF_END, GSF_SOUNDFX};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CBID_GENERIC_AI_PURCHASE_SELECTION, CBID_SOUNDS_AMBIENT_EFFECT,
};
use crate::newgrf_spritegroup::VSG_SCOPE_SELF;
use crate::random_func::chance16_i;
use crate::services::{Services as Shared, WaterClass};
use crate::tile_type::{MP_CLEAR, MP_TREES, MP_WATER};
use std::{ffi::c_void, ptr};
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Scope {
    cargo: u8,
    selection: u8,
    source: u8,
    destination: u8,
    distance: u8,
    event: u8,
    count: u8,
    station_size: u8,
    feature: u8,
    ai: bool,
}
#[repr(C)]
pub struct Invocation {
    scope: Scope,
    callback: u16,
    param1: u32,
    file: *const c_void,
    root: *const c_void,
}
#[repr(C)]
pub struct Result {
    file: *const c_void,
    result: u16,
}
#[repr(C)]
pub struct Services {
    shared: *const Shared,
    version: extern "C" fn(*const c_void) -> u8,
    cargo_map: extern "C" fn(*const c_void, u8) -> u8,
    cargo_bitnum: extern "C" fn(u8) -> u8,
    industry_substitute: extern "C" fn(u8) -> u16,
    evaluate: extern "C" fn(*const Invocation, *mut i32, usize) -> u16,
    common: *const crate::newgrf_spritegroup::Services,
    enabled: extern "C" fn() -> bool,
    ambient: extern "C" fn() -> bool,
    terrain: extern "C" fn(u32) -> u32,
    water_class: extern "C" fn(u32) -> WaterClass,
    sound: extern "C" fn(*const c_void, u16, u32),
    unsupported: extern "C" fn(u8),
    unhandled: extern "C" fn(u8),
}
#[derive(Clone, Copy)]
struct Callback {
    file: *const c_void,
    group: *const c_void,
}
struct Registry {
    native: *const Services,
    groups: [Vec<Callback>; GSF_END as usize],
}
static STATE: GameState<Registry> = GameState::new(Registry {
    native: ptr::null(),
    groups: [const { Vec::new() }; GSF_END as usize],
});
fn services() -> &'static Services {
    unsafe { &*(*STATE.get()).native }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_generic_reset(native: *const Services) {
    // SAFETY: load/reset is outside every runtime invocation, before graph destruction.
    let state = unsafe { &mut *STATE.get() };
    state.native = native;
    for groups in &mut state.groups {
        groups.clear();
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_generic_add(feature: u8, file: *const c_void, group: *const c_void) {
    if feature >= GSF_END {
        (services().unsupported)(feature);
        return;
    }
    // SAFETY: registration is confined to native load phases; preserve duplicates.
    unsafe {
        (*STATE.get()).groups[usize::from(feature)].push(Callback { file, group });
    }
}
unsafe fn result(
    invocation: *mut Invocation,
    v7: u32,
    v8: u32,
    regs: *mut i32,
    count: usize,
) -> Result {
    let s = services();
    let feature = unsafe { (*invocation).scope.feature };
    crate::movement::native_assert(unsafe { &*s.shared }, feature < GSF_END);
    let mut index = unsafe { (&(*STATE.get()).groups)[usize::from(feature)].len() };
    while index != 0 {
        index -= 1;
        // SAFETY: loader lifetime excludes mutation. No slice/iterator survives evaluation.
        let entry = unsafe { (&(*STATE.get()).groups)[usize::from(feature)][index] };
        let param1 = if (s.version)(entry.file) >= 8 { v8 } else { v7 };
        unsafe {
            (*invocation).file = entry.file;
            (*invocation).root = entry.group;
            (*invocation).param1 = param1;
        }
        // Ordinary ResolveCallback clears temporary registers/reset state per attempt.
        let result = (s.evaluate)(invocation, regs, count);
        if result != CALLBACK_FAILED {
            return Result {
                file: entry.file,
                result,
            };
        }
    }
    Result {
        file: ptr::null(),
        result: CALLBACK_FAILED,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_generic_variable(
    invocation: *const Invocation,
    variable: u8,
    available: *mut bool,
) -> u32 {
    let scope = unsafe { (*invocation).scope };
    let file = unsafe { (*invocation).file };
    let s = services();
    if scope.ai {
        match variable {
            0x40 => return u32::from((s.cargo_map)(file, scope.cargo)),
            0x80 => return u32::from(scope.cargo),
            0x81 => return u32::from((s.cargo_bitnum)(scope.cargo)),
            0x82 => return u32::from(scope.selection),
            0x83 => return u32::from(scope.source),
            0x84 => return u32::from(scope.destination),
            0x85 => return u32::from(scope.distance),
            0x86 => return u32::from(scope.event),
            0x87 => return u32::from(scope.count),
            0x88 => return u32::from(scope.station_size),
            _ => (),
        }
    }
    (s.unhandled)(variable);
    unsafe {
        *available = false;
    }
    u32::MAX
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_generic_scope(
    scope: *mut c_void,
    default_scope: *mut c_void,
    which: u8,
) -> *mut c_void {
    if which == VSG_SCOPE_SELF {
        scope
    } else {
        default_scope
    }
}
/// The native leaf supplies only the live polymorphic frame and its canonical
/// common carrier. Callback reset/register behavior stays in the common owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_generic_evaluate(
    invocation: *const Invocation,
    resolver: *mut c_void,
    state: *mut crate::newgrf_spritegroup::State,
    regs: *mut i32,
    count: usize,
) -> u16 {
    // SAFETY: the frame and invocation outlive this synchronous call. Copy only
    // handles before evaluation; no owner/registry borrow crosses reentry.
    let (root, file) = unsafe { ((*invocation).root, (*invocation).file) };
    unsafe {
        crate::newgrf_spritegroup::openttd_rust_resolver_callback(
            &*services().common,
            root,
            resolver,
            state,
            file,
            regs,
            count,
        )
    }
}
fn substitute(industry: u8) -> u8 {
    if industry == IT_AI_UNKNOWN || industry == IT_AI_TOWN {
        return industry;
    }
    let substitute = (services().industry_substitute)(industry);
    if substitute == u16::from(IT_INVALID) {
        industry
    } else {
        substitute as u8
    }
}
pub const IT_AI_UNKNOWN: u8 = 0xfe;
pub const IT_AI_TOWN: u8 = 0xff;
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_generic_ai(
    feature: u8,
    cargo: u8,
    selection: u8,
    source: u8,
    destination: u8,
    distance: u8,
    event: u8,
    count: u8,
    station_size: u8,
) -> Result {
    let source = substitute(source);
    let destination = substitute(destination);
    let mut invocation = Invocation {
        scope: Scope {
            cargo,
            selection,
            source,
            destination,
            distance,
            event,
            count,
            station_size,
            feature,
            ai: true,
        },
        callback: CBID_GENERIC_AI_PURCHASE_SELECTION.0,
        param1: 0,
        file: ptr::null(),
        root: ptr::null(),
    };
    let mut answer = unsafe { result(&raw mut invocation, 0, 0, ptr::null_mut(), 0) };
    if answer.result != CALLBACK_FAILED && (services().version)(answer.file) < 8 {
        answer.result &= 255;
    }
    answer
}
/// The original outer feature gate; tree callers observe it again AFTER return.
pub fn ambient_enabled() -> bool {
    #[cfg(test)]
    if unsafe { (*STATE.get()).native.is_null() } {
        return false;
    }
    (services().enabled)()
}
pub fn ambient(tile: u32) {
    if ambient_enabled() {
        ambient_callback(tile);
    }
}
fn ambient_callback(tile: u32) {
    let s = services();
    let shared = unsafe { &*s.shared };
    crate::movement::native_assert(
        shared,
        matches!((shared.tile_type)(tile), MP_CLEAR | MP_TREES | MP_WATER),
    );
    let random = shared.random();
    if !chance16_i(1, 200, random) || !(s.ambient)() {
        return;
    }
    let v7 = u32::from((shared.tile_type)(tile)) << 28
        | (shared.tile_height)(tile).min(15) << 24
        | ((random >> 16) & 255) << 16
        | (s.terrain)(tile);
    let v8 = u32::from((shared.tile_type)(tile)) << 24
        | (shared.tile_z)(tile).cast_unsigned() << 16
        | ((random >> 16) & 255) << 8
        | (if (shared.has_water_class)(tile) {
            u32::from((s.water_class)(tile).0)
        } else {
            0
        }) << 3
        | (s.terrain)(tile);
    let mut invocation = Invocation {
        scope: Scope {
            feature: GSF_SOUNDFX,
            ..Scope::default()
        },
        callback: CBID_SOUNDS_AMBIENT_EFFECT.0,
        param1: 0,
        file: ptr::null(),
        root: ptr::null(),
    };
    let answer = unsafe { result(&raw mut invocation, v7, v8, ptr::null_mut(), 0) };
    if answer.result != CALLBACK_FAILED {
        (s.sound)(answer.file, answer.result, tile);
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGenericScope" ["probe"] => Scope { cargo, selection, source, destination, distance, event, count, station_size, feature, ai };
    "OpenTTDGenericInvocation" ["probe"] => Invocation { scope, callback, param1, file, root };
    "OpenTTDGenericResult" ["probe"] => Result { file, result };
    "OpenTTDGenericServices" ["probe"] => Services { shared, version, cargo_map, cargo_bitnum, industry_substitute, evaluate, common, enabled, ambient, terrain, water_class, sound, unsupported, unhandled };
}
