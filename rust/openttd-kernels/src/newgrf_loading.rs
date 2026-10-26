/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical `NewGRF` loading execution state. The game-state mutex serializes native aliases
//! and Rust entries. Raw field accesses end before native parser reentry; the carrier never
//! moves, and native aliases own no storage. Parsed objects remain native.
#![allow(unsafe_code, missing_docs, clippy::missing_safety_doc)]
use crate::game_state::GameState;
use crate::newgrf::GSF_END;
use std::{collections::BTreeMap, ffi::c_void, ptr};

#[repr(C)]
pub struct Execution {
    stage: u8,
    spriteid: u32,
    file: *mut c_void,
    grffile: *mut c_void,
    grfconfig: *mut c_void,
    nfo_line: u32,
    skip_sprites: i32,
    spritegroups: [*const c_void; 256],
}
static EXECUTION: GameState<Execution> = GameState::new(Execution {
    stage: 0,
    spriteid: 0,
    file: ptr::null_mut(),
    grffile: ptr::null_mut(),
    grfconfig: ptr::null_mut(),
    nfo_line: 0,
    skip_sprites: 0,
    spritegroups: [ptr::null(); 256],
});
#[derive(Clone, Copy)]
struct SpriteSet {
    sprite: u32,
    num_sprites: u32,
}
struct State {
    spritesets: [BTreeMap<u32, SpriteSet>; GSF_END as usize],
}
static STATE: GameState<State> = GameState::new(State {
    spritesets: [const { BTreeMap::new() }; GSF_END as usize],
});
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_grf_execution() -> *mut Execution {
    EXECUTION.get()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_clear_file() {
    unsafe {
        (*EXECUTION.get()).nfo_line = 0;
        (*EXECUTION.get()).skip_sprites = 0;
        (*EXECUTION.get()).spritegroups.fill(ptr::null());
        for sets in &mut (*STATE.get()).spritesets {
            sets.clear();
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_add_sets(
    feature: u8,
    first_sprite: u32,
    first_set: u32,
    numsets: u32,
    numents: u32,
) {
    let sets = unsafe { &mut (*STATE.get()).spritesets[usize::from(feature)] };
    for i in 0..numsets {
        sets.insert(
            first_set.wrapping_add(i),
            SpriteSet {
                sprite: first_sprite.wrapping_add(i.wrapping_mul(numents)),
                num_sprites: numents,
            },
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_has_sets(feature: u8) -> bool {
    unsafe { !(*STATE.get()).spritesets[usize::from(feature)].is_empty() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_has_set(feature: u8, set: u32) -> bool {
    unsafe { (*STATE.get()).spritesets[usize::from(feature)].contains_key(&set) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_set_sprite(feature: u8, set: u32) -> u32 {
    unsafe { (&(*STATE.get()).spritesets[usize::from(feature)])[&set].sprite }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_set_size(feature: u8, set: u32) -> u32 {
    unsafe { (&(*STATE.get()).spritesets[usize::from(feature)])[&set].num_sprites }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGrfExecution" ["probe"] => Execution {
        stage, spriteid, file, grffile, grfconfig, nfo_line, skip_sprites, spritegroups
    };
}
