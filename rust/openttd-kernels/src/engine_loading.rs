/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Engine definition identity and temporary loading storage. The game-state mutex
//! serializes entries. Mapping views live until the next mapping mutation; temporary
//! field views live until resize/clear. No borrow survives native callbacks.
#![allow(unsafe_code, missing_docs, clippy::cast_possible_truncation)]
use crate::{engine_type::INVALID_ENGINE, game_state::GameState, newgrf::INVALID_GRFID};
use std::{collections::BTreeMap, ffi::c_void, ptr};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Mapping {
    grfid: u32,
    internal_id: u16,
    kind: u8,
    substitute_id: u8,
    engine: u16,
}
impl Mapping {
    fn key(self) -> u64 {
        key(self.grfid, self.internal_id)
    }
}
fn key(grfid: u32, local: u16) -> u64 {
    (u64::from(grfid) << 32) | u64::from(local)
}
static MAPPINGS: GameState<[Vec<Mapping>; 4]> = GameState::new([const { Vec::new() }; 4]);
static OVERRIDES: GameState<BTreeMap<u32, u32>> = GameState::new(BTreeMap::new());
#[repr(C)]
pub struct MappingView {
    data: *const Mapping,
    size: usize,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_mappings(kind: u8) -> MappingView {
    let map = unsafe { &(&*MAPPINGS.get())[usize::from(kind)] };
    MappingView {
        data: map.as_ptr(),
        size: map.len(),
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_mapping_clear() {
    for map in unsafe { &mut *MAPPINGS.get() } {
        *map = Vec::new();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_mapping_defaults(counts: *const u8) {
    let mut engine = 0;
    for (kind, map) in unsafe { &mut *MAPPINGS.get() }.iter_mut().enumerate() {
        map.clear();
        for local in 0..unsafe { *counts.add(kind) } {
            map.push(Mapping {
                grfid: INVALID_GRFID,
                internal_id: u16::from(local),
                kind: kind as u8,
                substitute_id: local,
                engine,
            });
            engine += 1;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_mapping_get(kind: u8, local: u16, grfid: u32) -> u16 {
    let map = unsafe { &(&*MAPPINGS.get())[usize::from(kind)] };
    let index = map.partition_point(|m| m.key() < key(grfid, local));
    map.get(index)
        .filter(|m| m.key() == key(grfid, local))
        .map_or(INVALID_ENGINE, |m| m.engine)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_mapping_use(
    kind: u8,
    local: u16,
    grfid: u32,
    static_access: bool,
) -> u16 {
    let map = unsafe { &mut (&mut *MAPPINGS.get())[usize::from(kind)] };
    let index = map.partition_point(|m| m.key() < key(INVALID_GRFID, local));
    if index == map.len() || map[index].key() != key(INVALID_GRFID, local) {
        return INVALID_ENGINE;
    }
    let engine = map[index].engine;
    if !static_access && grfid != INVALID_GRFID {
        map[index].grfid = grfid;
        let position = map.partition_point(|m| m.key() < key(grfid, local));
        // Slide relocates after computing lower_bound on the modified list.
        if position > index {
            map[index..position].rotate_left(1);
        } else {
            map[position..=index].rotate_right(1);
        }
    }
    engine
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_mapping_set(
    kind: u8,
    local: u16,
    grfid: u32,
    substitute: u8,
    engine: u16,
) {
    let map = unsafe { &mut (&mut *MAPPINGS.get())[usize::from(kind)] };
    let index = map.partition_point(|m| m.key() < key(grfid, local));
    if index == map.len() || map[index].key() != key(grfid, local) {
        map.insert(
            index,
            Mapping {
                grfid,
                internal_id: local,
                kind,
                substitute_id: substitute,
                engine,
            },
        );
    } else {
        map[index].engine = engine;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_override_set(source: u32, target: u32) {
    let map = unsafe { &mut *OVERRIDES.get() };
    if target == 0 {
        map.remove(&source);
    } else {
        map.insert(source, target);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_override_get(source: u32, target: *mut u32) -> bool {
    if let Some(value) = unsafe { &*OVERRIDES.get() }.get(&source) {
        unsafe {
            *target = *value;
        }
        true
    } else {
        false
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_override_clear() {
    unsafe { &mut *OVERRIDES.get() }.clear();
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TempFields {
    cargo_allowed: u16,
    cargo_allowed_required: u16,
    cargo_disallowed: u16,
    railtypelabels: u16,
    roadtramtype: u8,
    defaultcargo_grf: *const c_void,
    refittability: u8,
    rv_max_speed: u8,
    ctt_include_mask: u64,
    ctt_exclude_mask: u64,
}
impl Default for TempFields {
    fn default() -> Self {
        Self {
            cargo_allowed: 0,
            cargo_allowed_required: 0,
            cargo_disallowed: 0,
            railtypelabels: 0,
            roadtramtype: 0,
            defaultcargo_grf: ptr::null(),
            refittability: 0,
            rv_max_speed: 0,
            ctt_include_mask: 0,
            ctt_exclude_mask: 0,
        }
    }
}
#[derive(Default)]
struct Temp {
    fields: TempFields,
    labels: Vec<u32>,
}
static TEMP: GameState<Vec<Temp>> = GameState::new(Vec::new());
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_resize(size: usize) {
    let temp = unsafe { &mut *TEMP.get() };
    let old = temp.len();
    temp.resize_with(size, Temp::default);
    for (id, entry) in temp.iter_mut().enumerate().skip(old) {
        entry.fields.railtypelabels = id as u16;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_clear() {
    unsafe { &mut *TEMP.get() }.clear();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_fields(engine: u16) -> *mut TempFields {
    &raw mut (unsafe { &mut *TEMP.get() })[usize::from(engine)].fields
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_refittability(
    fields: *mut TempFields,
    nonempty: bool,
) {
    let fields = unsafe { &mut *fields };
    if nonempty {
        fields.refittability = 2;
    } else if fields.refittability == 0 {
        fields.refittability = 1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_labels_clear(engine: u16) {
    (unsafe { &mut *TEMP.get() })[usize::from(engine)]
        .labels
        .clear();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_labels_push(engine: u16, label: u32) {
    (unsafe { &mut *TEMP.get() })[usize::from(engine)]
        .labels
        .push(label);
}
#[repr(C)]
pub struct RailLabelView {
    data: *mut u32,
    size: usize,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_labels(engine: u16) -> RailLabelView {
    let labels = &mut (unsafe { &mut *TEMP.get() })[usize::from(engine)].labels;
    RailLabelView {
        data: labels.as_mut_ptr(),
        size: labels.len(),
    }
}
