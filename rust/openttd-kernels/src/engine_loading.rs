/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Engine definition identity and temporary loading storage. The game-state mutex
//! serializes entries. Mapping views live until the next mapping mutation; temporary
//! field views live until resize/clear. No borrow survives native callbacks.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]
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
static MAPPINGS: GameState<[Vec<Mapping>; VEH_COMPANY_END as usize]> =
    GameState::new([const { Vec::new() }; VEH_COMPANY_END as usize]);
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
unsafe fn set_override(source: u32, target: u32) {
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
impl TempFields {
    const UNSET: u8 = 0;
    const EMPTY: u8 = 1;
    const NONEMPTY: u8 = 2;
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
            refittability: Self::UNSET,
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
fn temp(engine: u16) -> *mut Temp {
    // Vec::as_mut_ptr does not borrow elements; native field aliases stay valid.
    unsafe { (*TEMP.get()).as_mut_ptr().add(usize::from(engine)) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_fields(engine: u16) -> *mut TempFields {
    unsafe { ptr::addr_of_mut!((*temp(engine)).fields) }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_refittability(
    fields: *mut TempFields,
    nonempty: bool,
) {
    let fields = unsafe { &mut *fields };
    if nonempty {
        fields.refittability = TempFields::NONEMPTY;
    } else if fields.refittability == TempFields::UNSET {
        fields.refittability = TempFields::EMPTY;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_labels_clear(engine: u16) {
    unsafe { &mut (*temp(engine)).labels }.clear();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_labels_push(engine: u16, label: u32) {
    unsafe { &mut (*temp(engine)).labels }.push(label);
}
#[repr(C)]
pub struct RailLabelView {
    data: *mut u32,
    size: usize,
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_labels(engine: u16) -> RailLabelView {
    let labels = unsafe { &mut (*temp(engine)).labels };
    RailLabelView {
        data: labels.as_mut_ptr(),
        size: labels.len(),
    }
}

use crate::vehicle_type::{VEH_COMPANY_END, VEH_TRAIN};
use crate::{engine::EngineRef, services::Services as Shared, vehicle::OpenTTDEngine};
type E = *const OpenTTDEngine;
type File = *const c_void;
#[repr(C)]
pub struct Services {
    shared: *const Shared,
    resolver: *const crate::newgrf_vehicle::Services,
    next: extern "C" fn(u32) -> EngineRef,
    resolve: extern "C" fn(u16) -> EngineRef,
    dynamic: extern "C" fn() -> bool,
    file_id: extern "C" fn(File) -> u32,
    find_file: extern "C" fn(u32) -> File,
    engine_file: extern "C" fn(E) -> File,
    has_file: extern "C" fn(E) -> bool,
    set_file: extern "C" fn(E, File),
    mapping_log: extern "C" fn(u32, u32, bool),
    replaced_log: extern "C" fn(u16, u32, u8, u16),
    created_log: extern "C" fn(u16, u32, u8, u16),
    full_log: extern "C" fn(),
    can_allocate: extern "C" fn() -> bool,
    pool_size: extern "C" fn() -> usize,
    construct: extern "C" fn(u8, u16) -> EngineRef,
    construct_at: extern "C" fn(u8, u16, u16),
    close_previews: extern "C" fn(),
    clean_pool: extern "C" fn(),
    railtypes: extern "C" fn(E) -> u64,
    rail_label: extern "C" fn(u8) -> u32,
    refit_inputs: extern "C" fn(E) -> RefitInputs,
    cargo_label: extern "C" fn(E) -> CargoLabel,
    train_cargo: extern "C" fn(E) -> TrainCargo,
    set_cargo: extern "C" fn(E, u8),
    set_refit: extern "C" fn(E, u64),
    disable: extern "C" fn(E),
    old_refittable: extern "C" fn(E) -> bool,
    set_old_refittable: extern "C" fn(E, bool),
    default_cargo: extern "C" fn(u8) -> bool,
    cargo_by_label: extern "C" fn(u32) -> u8,
    cargo_mask: extern "C" fn() -> u64,
    standard_cargo_mask: extern "C" fn() -> u64,
    cargo_next: extern "C" fn(u32) -> CargoClass,
    cargo_map: extern "C" fn(File, u8) -> u8,
    file_version: extern "C" fn(File) -> u8,
    cargo_list_size: extern "C" fn(File) -> usize,
    cargo_translation: extern "C" fn(u8, File, bool) -> u8,
    unknown_refit: extern "C" fn(u32, u16, u16),
    climate: extern "C" fn() -> u8,
    variant: extern "C" fn(E) -> u16,
    set_variant: extern "C" fn(E, u16),
    invalid_string: extern "C" fn(E),
    engine_climate: extern "C" fn(E) -> bool,
    rail_badges: extern "C" fn(E, u8),
    road_badges: extern "C" fn(E),
    rail_kind: extern "C" fn(E) -> RailVehicleTypes,
    livery: extern "C" fn(u16) -> u8,
    use_livery: extern "C" fn(u8),
    variant_flags: extern "C" fn(E),
    variant_cycle_log: extern "C" fn(E),
    is_tram: extern "C" fn(E) -> bool,
    set_road_speed: extern "C" fn(E, u16),
    set_roadtype: extern "C" fn(E, u8),
    road_list_size: extern "C" fn(File, bool) -> usize,
    road_label: extern "C" fn(File, bool, usize) -> u32,
    road_by_label: extern "C" fn(u32) -> u8,
    rail_by_label: extern "C" fn(u32) -> u8,
    set_railtypes: extern "C" fn(E, u64),
    override_log: extern "C" fn(u32, u32),
    vehicle_next: extern "C" fn(u32) -> LoadingVehicle,
    reload: extern "C" fn(),
    default_counts: extern "C" fn() -> *const u8,
}
fn default_count(s: &Services, kind: u8) -> u8 {
    unsafe { *(s.default_counts)().add(usize::from(kind)) }
}
fn scope(s: &Services, file: File, dynamic: bool) -> u32 {
    if !dynamic {
        return INVALID_GRFID;
    }
    let id = (s.file_id)(file);
    unsafe { &*OVERRIDES.get() }.get(&id).copied().unwrap_or(id)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_new_id(
    s: &Services,
    file: File,
    kind: u8,
    local: u16,
) -> u16 {
    unsafe { openttd_rust_engine_mapping_get(kind, local, scope(s, file, (s.dynamic)())) }
}
unsafe fn initialize_rail_labels(s: &Services, engine: EngineRef) {
    let railtypes = (s.railtypes)(engine.shell);
    unsafe {
        openttd_rust_engine_temp_labels_clear(engine.id);
    }
    for rt in 0_u8..64 {
        if railtypes & (1_u64 << rt) == 0 {
            continue;
        }
        let label = (s.rail_label)(rt);
        unsafe {
            openttd_rust_engine_temp_labels_push(engine.id, label);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_definition(
    s: &Services,
    file: File,
    kind: u8,
    local: u16,
    static_access: bool,
) -> E {
    let dynamic = (s.dynamic)();
    let scope = scope(s, file, dynamic);
    if dynamic {
        let file_id = (s.file_id)(file);
        if unsafe { &*OVERRIDES.get() }.contains_key(&file_id) {
            (s.mapping_log)(file_id, scope, !(s.find_file)(scope).is_null());
        }
        let id = unsafe { openttd_rust_engine_mapping_get(kind, local, scope) };
        if id != INVALID_ENGINE {
            let engine = (s.resolve)(id);
            if !(s.has_file)(engine.shell) {
                (s.set_file)(engine.shell, file);
            }
            return engine.shell;
        }
    }
    let id = unsafe { openttd_rust_engine_mapping_use(kind, local, scope, static_access) };
    if id != INVALID_ENGINE {
        let engine = (s.resolve)(id);
        if !(s.has_file)(engine.shell) {
            (s.set_file)(engine.shell, file);
            (s.replaced_log)(id, (s.file_id)(file), kind, local);
        }
        return engine.shell;
    }
    if static_access {
        return ptr::null();
    }
    if !(s.can_allocate)() {
        (s.full_log)();
        return ptr::null();
    }
    let size = (s.pool_size)();
    let engine = (s.construct)(kind, local);
    (s.set_file)(engine.shell, file);
    unsafe {
        openttd_rust_engine_mapping_set(
            kind,
            local,
            scope,
            (local as u8).min(default_count(s, kind)),
            engine.id,
        );
    }
    if size != (s.pool_size)() {
        unsafe {
            openttd_rust_engine_temp_resize((s.pool_size)());
        }
    }
    if kind == VEH_TRAIN {
        unsafe {
            initialize_rail_labels(s, engine);
        }
    }
    (s.created_log)(engine.id, (s.file_id)(file), kind, local);
    engine.shell
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_setup(s: &Services) {
    (s.close_previews)();
    (s.clean_pool)();
    for kind in VEH_TRAIN..VEH_COMPANY_END {
        let count = unsafe { (&*MAPPINGS.get())[usize::from(kind)].len() };
        crate::movement::native_assert(
            unsafe { &*s.shared },
            count >= usize::from(default_count(s, kind)),
        );
        for index in 0..count {
            let mapping = unsafe { (&*MAPPINGS.get())[usize::from(kind)][index] };
            (s.construct_at)(kind, mapping.internal_id, mapping.engine);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_temp_initialize(s: &Services) {
    unsafe {
        openttd_rust_engine_temp_resize((s.pool_size)());
    }
    let mut from = 0;
    loop {
        let engine = (s.next)(from);
        if engine.shell.is_null() {
            break;
        }
        from = u32::from(engine.id) + 1;
        if engine.kind.0 == VEH_TRAIN {
            unsafe {
                initialize_rail_labels(s, engine);
            }
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEngineLoadingVehicle" ["engine_loading"] => LoadingVehicle { id, kind => type };
    "OpenTTDEngineLoadingServices" ["engine_loading"] => Services {
        shared,
        resolver,
        next,
        resolve,
        dynamic,
        file_id,
        find_file,
        engine_file,
        has_file,
        set_file,
        mapping_log,
        replaced_log,
        created_log,
        full_log,
        can_allocate,
        pool_size,
        construct,
        construct_at,
        close_previews,
        clean_pool,
        railtypes,
        rail_label,
        refit_inputs,
        cargo_label,
        train_cargo,
        set_cargo,
        set_refit,
        disable,
        old_refittable,
        set_old_refittable,
        default_cargo,
        cargo_by_label,
        cargo_mask,
        standard_cargo_mask,
        cargo_next,
        cargo_map,
        file_version,
        cargo_list_size,
        cargo_translation,
        unknown_refit,
        climate,
        variant,
        set_variant,
        invalid_string,
        engine_climate,
        rail_badges,
        road_badges,
        rail_kind,
        livery,
        use_livery,
        variant_flags,
        variant_cycle_log,
        is_tram,
        set_road_speed,
        set_roadtype,
        road_list_size,
        road_label,
        road_by_label,
        rail_by_label,
        set_railtypes,
        override_log,
        vehicle_next,
        reload,
        default_counts,


    };
    "EngineIDMapping" ["engine_loading"] => Mapping { grfid, internal_id, kind => type, substitute_id, engine };
    "OpenTTDEngineMappingView" ["engine_loading"] => MappingView { data, size };
    "OpenTTDEngineCargoLabel" ["engine_loading"] => CargoLabel { value, mixed };
    "OpenTTDEngineRefitInputs" ["engine_loading"] => RefitInputs { mask, cargo, custom };
    "OpenTTDEngineTrainCargo" ["engine_loading"] => TrainCargo { capacity, kind };
    "OpenTTDEngineCargoClass" ["engine_loading"] => CargoClass { classes, cargo };
    "GRFTempEngineData" ["engine_loading"] => TempFields { cargo_allowed, cargo_allowed_required, cargo_disallowed, railtypelabels, roadtramtype, defaultcargo_grf, refittability, rv_max_speed, ctt_include_mask, ctt_exclude_mask };
    "RustRailLabelView" ["engine_loading"] => RailLabelView { data, size };

}
use crate::newgrf::{CargoLabel, get_active_cargo_label};
#[repr(C)]
pub struct RefitInputs {
    mask: u64,
    cargo: u8,
    custom: bool,
}
#[repr(C)]
pub struct TrainCargo {
    capacity: u8,
    kind: crate::vehicle::RailVehicleTypes,
}
#[repr(C)]
pub struct CargoClass {
    classes: u16,
    cargo: u8,
}
use crate::cargo_type::{
    CT_CANDY, CT_COAL, CT_COLA, CT_COPPER_ORE, CT_FOOD, CT_GOODS, CT_INVALID, CT_MAIL, CT_OIL,
    CT_PASSENGERS, CT_SUGAR, CT_VALUABLES, INVALID_CARGO, NUM_CARGO, is_valid_cargo_type,
};
use crate::cargotype::{
    CC_ARMOURED, CC_BULK, CC_EXPRESS, CC_LIQUID, CC_MAIL, CC_PIECEGOODS, CC_REFRIGERATED,
    PASSENGERS,
};
use crate::newgrf_callbacks::{CALLBACK_FAILED, CBID_VEHICLE_CUSTOM_REFIT};
use crate::vehicle::RailVehicleTypes;
use crate::vehicle_type::{VEH_AIRCRAFT, VEH_SHIP};
#[unsafe(no_mangle)]
pub(crate) extern "C" fn openttd_rust_engine_active_label(s: &Services, label: CargoLabel) -> u32 {
    get_active_cargo_label(label, |label| (s.cargo_by_label)(label))
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_engine_translate_refit(s: &Services, mask: u32, file: File) -> u64 {
    let mut result = 0;
    for bit in 0_u8..32 {
        if mask & (1 << bit) == 0 {
            continue;
        }
        let cargo = (s.cargo_translation)(bit, file, true);
        if is_valid_cargo_type(cargo) {
            result |= 1_u64 << cargo;
        }
    }
    result
}
fn engine_label(s: &Services, shell: E) -> u32 {
    get_active_cargo_label((s.cargo_label)(shell), |label| (s.cargo_by_label)(label))
}
fn default_masks(s: &Services, kind: u8, shell: E, id: u16, label: u32) -> (u16, u16) {
    if kind == VEH_AIRCRAFT {
        (PASSENGERS | CC_MAIL | CC_ARMOURED | CC_EXPRESS, CC_LIQUID)
    } else if kind == VEH_SHIP {
        match label {
            CT_PASSENGERS => (PASSENGERS, 0),
            CT_OIL => (CC_LIQUID, 0),
            _ if (s.climate)() == crate::services::LandscapeType::TOYLAND.0 => (
                CC_MAIL | CC_ARMOURED | CC_EXPRESS | CC_BULK | CC_PIECEGOODS | CC_LIQUID,
                PASSENGERS,
            ),
            _ => (
                CC_MAIL | CC_ARMOURED | CC_EXPRESS | CC_BULK | CC_PIECEGOODS,
                CC_LIQUID | PASSENGERS,
            ),
        }
    } else if kind == VEH_TRAIN && (s.train_cargo)(shell).kind != RailVehicleTypes::WAGON {
        (
            PASSENGERS | CC_MAIL | CC_ARMOURED | CC_EXPRESS | CC_BULK | CC_PIECEGOODS | CC_LIQUID,
            0,
        )
    } else {
        const DEFAULTS: &[(u8, u32, u16, u16)] = &[
            (0b1111, CT_PASSENGERS, PASSENGERS, 0),
            (0b0111, CT_MAIL, CC_MAIL, 0),
            (0b0111, CT_VALUABLES, CC_ARMOURED, CC_LIQUID),
            (0b1000, CT_MAIL, CC_MAIL | CC_ARMOURED, CC_LIQUID),
            (0b0011, CT_COAL, CC_BULK, 0),
            (0b0100, CT_COPPER_ORE, CC_BULK, 0),
            (0b1000, CT_SUGAR, CC_BULK, 0),
            (0b0111, CT_OIL, CC_LIQUID, 0),
            (0b1000, CT_COLA, CC_LIQUID, 0),
            (
                0b0001,
                CT_GOODS,
                CC_PIECEGOODS | CC_EXPRESS,
                CC_LIQUID | PASSENGERS,
            ),
            (
                0b0110,
                CT_GOODS,
                CC_PIECEGOODS | CC_EXPRESS,
                CC_LIQUID | PASSENGERS | CC_REFRIGERATED,
            ),
            (0b0110, CT_FOOD, CC_REFRIGERATED, 0),
            (
                0b1000,
                CT_CANDY,
                CC_PIECEGOODS | CC_EXPRESS,
                CC_LIQUID | PASSENGERS,
            ),
        ];
        let climate = (s.climate)();
        DEFAULTS
            .iter()
            .find(|&&(climates, cargo, _, _)| climates & (1 << climate) != 0 && cargo == label)
            .map_or_else(
                || unsafe {
                    let f = &(&*TEMP.get())[usize::from(id)].fields;
                    (f.cargo_allowed, f.cargo_disallowed)
                },
                |&(_, _, allowed, disallowed)| (allowed, disallowed),
            )
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_calculate_refits(s: &Services) {
    let mut original_known_cargoes = 0;
    for cargo in 0..NUM_CARGO {
        if (s.default_cargo)(cargo) {
            original_known_cargoes |= 1_u64 << cargo;
        }
    }
    let mut from = 0;
    loop {
        let e = (s.next)(from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        let mut input = (s.refit_inputs)(e.shell);
        if !is_valid_cargo_type(input.cargo) {
            input.cargo = (s.cargo_by_label)(engine_label(s, e.shell));
            (s.set_cargo)(e.shell, input.cargo);
        }
        let fields = unsafe { openttd_rust_engine_temp_fields(e.id) };
        if unsafe { (*fields).defaultcargo_grf.is_null() } {
            if e.kind.0 != VEH_TRAIN || (s.train_cargo)(e.shell).capacity != 0 {
                let label = if e.kind.0 == VEH_AIRCRAFT
                    || (e.kind.0 == VEH_TRAIN
                        && (s.train_cargo)(e.shell).kind != RailVehicleTypes::WAGON)
                {
                    CT_INVALID
                } else {
                    engine_label(s, e.shell)
                };
                let (allowed, disallowed) = default_masks(s, e.kind.0, e.shell, e.id, label);
                unsafe {
                    (*fields).cargo_allowed = allowed;
                    (*fields).cargo_disallowed = disallowed;
                }
                if e.kind.0 == VEH_SHIP {
                    (s.set_old_refittable)(e.shell, true);
                } else if e.kind.0 != VEH_AIRCRAFT
                    && (e.kind.0 != VEH_TRAIN
                        || (s.train_cargo)(e.shell).kind == RailVehicleTypes::WAGON)
                {
                    unsafe {
                        (*fields).ctt_exclude_mask = original_known_cargoes;
                    }
                }
            }
            unsafe {
                openttd_rust_engine_temp_refittability(fields, (*fields).cargo_allowed != 0);
                if is_valid_cargo_type(input.cargo) {
                    (*fields).ctt_exclude_mask &= !(1_u64 << input.cargo);
                }
            }
        }
        let (allowed, required, disallowed, only_default, include, exclude, default_file) = unsafe {
            (
                (*fields).cargo_allowed,
                (*fields).cargo_allowed_required,
                (*fields).cargo_disallowed,
                (*fields).refittability != TempFields::NONEMPTY,
                (*fields).ctt_include_mask,
                (*fields).ctt_exclude_mask,
                (*fields).defaultcargo_grf,
            )
        };
        let mut mask = 0;
        let mut not_mask = 0;
        if allowed != 0 {
            let mut cargo_from = 0;
            loop {
                let cs = (s.cargo_next)(cargo_from);
                if cs.cargo == INVALID_CARGO {
                    break;
                }
                cargo_from = u32::from(cs.cargo) + 1;
                if cs.classes & allowed != 0 && cs.classes & required == required {
                    mask |= 1_u64 << cs.cargo;
                }
                if cs.classes & disallowed != 0 {
                    not_mask |= 1_u64 << cs.cargo;
                }
            }
        }
        input.mask = (((mask & !not_mask) ^ input.mask) & (s.cargo_mask)() | include) & !exclude;
        (s.set_refit)(e.shell, input.mask);
        let file = if default_file.is_null() {
            (s.engine_file)(e.shell)
        } else {
            default_file
        };
        if !file.is_null() && input.custom {
            let mut cargo_from = 0;
            loop {
                let cs = (s.cargo_next)(cargo_from);
                if cs.cargo == INVALID_CARGO {
                    break;
                }
                cargo_from = u32::from(cs.cargo) + 1;
                let local_slot = (s.cargo_map)(file, cs.cargo);
                let callback = unsafe {
                    crate::newgrf_vehicle::openttd_rust_vehicle_callback(
                        &*s.resolver,
                        CBID_VEHICLE_CUSTOM_REFIT.0,
                        u32::from(cs.classes),
                        u32::from(local_slot),
                        e.id,
                        ptr::null_mut(),
                        ptr::null_mut(),
                        false,
                        ptr::null_mut(),
                        0,
                    )
                };
                match callback {
                    CALLBACK_FAILED | 0 => (),
                    1 => {
                        input.mask |= 1_u64 << cs.cargo;
                        (s.set_refit)(e.shell, input.mask);
                    }
                    2 => {
                        input.mask &= !(1_u64 << cs.cargo);
                        (s.set_refit)(e.shell, input.mask);
                    }
                    _ => {
                        (s.unknown_refit)((s.file_id)(file), CBID_VEHICLE_CUSTOM_REFIT.0, callback);
                    }
                }
            }
        }
        if is_valid_cargo_type(input.cargo) && (s.cargo_mask)() & (1_u64 << input.cargo) == 0 {
            input.cargo = INVALID_CARGO;
            (s.set_cargo)(e.shell, input.cargo);
        }
        if !only_default
            && (e.kind.0 != VEH_SHIP || (s.old_refittable)(e.shell))
            && is_valid_cargo_type(input.cargo)
            && input.mask & (1_u64 << input.cargo) == 0
        {
            input.cargo = INVALID_CARGO;
            (s.set_cargo)(e.shell, input.cargo);
        }
        if !is_valid_cargo_type(input.cargo) && input.mask != 0 {
            let file = if default_file.is_null() {
                (s.engine_file)(e.shell)
            } else {
                default_file
            };
            if !file.is_null() && (s.file_version)(file) >= 8 && (s.cargo_list_size)(file) != 0 {
                let mut best_local_slot = u8::MAX;
                for cargo in 0..NUM_CARGO {
                    if input.mask & (1_u64 << cargo) == 0 {
                        continue;
                    }
                    let local = (s.cargo_map)(file, cargo);
                    if local < best_local_slot {
                        best_local_slot = local;
                        input.cargo = cargo;
                        (s.set_cargo)(e.shell, input.cargo);
                    }
                }
            }
            if !is_valid_cargo_type(input.cargo) {
                input.cargo = crate::bitmath_func::find_first_bit(input.mask);
                (s.set_cargo)(e.shell, input.cargo);
            }
        }
        if !is_valid_cargo_type(input.cargo) && e.kind.0 == VEH_TRAIN {
            let train = (s.train_cargo)(e.shell);
            if train.kind != RailVehicleTypes::WAGON
                && train.capacity == 0
                && engine_label(s, e.shell) != CT_INVALID
            {
                input.cargo = crate::bitmath_func::find_first_bit((s.standard_cargo_mask)());
                (s.set_cargo)(e.shell, input.cargo);
            }
        }
        if !is_valid_cargo_type(input.cargo) {
            (s.disable)(e.shell);
        }
        if e.kind.0 == VEH_SHIP && !(s.old_refittable)(e.shell) {
            (s.set_refit)(e.shell, 0);
        }
    }
}
use crate::livery::{
    LS_DMU, LS_EMU, LS_FREIGHT_WAGON, LS_MAGLEV, LS_PASSENGER_WAGON_DIESEL,
    LS_PASSENGER_WAGON_STEAM, LS_STEAM,
};
use crate::rail_type::INVALID_RAILTYPE;
use crate::road_type::{INVALID_ROADTYPE, ROADTYPE_ROAD, ROADTYPE_TRAM};
use crate::vehicle_type::VEH_ROAD;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_set_override(s: &Services, source: u32, target: u32) {
    unsafe {
        set_override(source, target);
    }
    (s.override_log)(source, target);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_current_override(s: &Services, file: File) -> File {
    let target = unsafe { &*OVERRIDES.get() }
        .get(&(s.file_id)(file))
        .copied();
    target.map_or(ptr::null(), |target| (s.find_file)(target))
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_finalise(s: &Services) {
    let mut from = 0;
    loop {
        let e = (s.next)(from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if (s.engine_file)(e.shell).is_null() {
            let invalid = unsafe {
                (&*MAPPINGS.get())[usize::from(e.kind.0)]
                    .iter()
                    .find(|m| m.engine == e.id)
                    .is_none_or(|m| {
                        m.grfid != INVALID_GRFID || m.internal_id != u16::from(m.substitute_id)
                    })
            };
            if invalid {
                (s.invalid_string)(e.shell);
            }
        }
        let variant = (s.variant)(e.shell);
        if variant != INVALID_ENGINE {
            let id = unsafe {
                openttd_rust_engine_new_id(s, (s.engine_file)(e.shell), e.kind.0, variant)
            };
            (s.set_variant)(e.shell, id);
        }
        if !(s.engine_climate)(e.shell) {
            continue;
        }
        match e.kind.0 {
            VEH_TRAIN => {
                let railtypes = (s.railtypes)(e.shell);
                for rt in 0_u8..64 {
                    if railtypes & (1_u64 << rt) != 0 {
                        (s.rail_badges)(e.shell, rt);
                    }
                }
            }
            VEH_ROAD => (s.road_badges)(e.shell),
            _ => (),
        }
        if e.kind.0 != VEH_TRAIN || (s.rail_kind)(e.shell) != RailVehicleTypes::WAGON {
            let ls = (s.livery)(e.id);
            (s.use_livery)(ls);
            if e.kind.0 == VEH_TRAIN {
                (s.use_livery)(LS_FREIGHT_WAGON);
                match ls {
                    LS_STEAM..=LS_MAGLEV => {
                        (s.use_livery)(LS_PASSENGER_WAGON_STEAM + ls - LS_STEAM);
                    }
                    LS_DMU..=LS_EMU => (s.use_livery)(LS_PASSENGER_WAGON_DIESEL + ls - LS_DMU),
                    _ => unreachable!(),
                }
            }
        }
    }
    let mut from = 0;
    loop {
        let e = (s.next)(from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        let mut parent = (s.variant)(e.shell);
        let mut parent_slow = parent;
        let mut update_slow = false;
        while parent != INVALID_ENGINE {
            parent = (s.variant)((s.resolve)(parent).shell);
            if update_slow {
                parent_slow = (s.variant)((s.resolve)(parent_slow).shell);
            }
            update_slow = !update_slow;
            if parent != e.id && parent != parent_slow {
                continue;
            }
            (s.set_variant)(e.shell, INVALID_ENGINE);
            (s.variant_cycle_log)(e.shell);
            break;
        }
        let parent = (s.variant)(e.shell);
        if parent != INVALID_ENGINE {
            (s.variant_flags)((s.resolve)(parent).shell);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_bind_types(s: &Services) {
    let shared = unsafe { &*s.shared };
    let mut from = 0;
    loop {
        let e = (s.next)(from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if e.kind.0 != VEH_ROAD {
            continue;
        }
        let fields = unsafe { openttd_rust_engine_temp_fields(e.id) };
        let speed = unsafe { (*fields).rv_max_speed };
        if speed != 0 {
            (s.set_road_speed)(e.shell, u16::from(speed) * 4);
        }
        let tram = (s.is_tram)(e.shell);
        let file = (s.engine_file)(e.shell);
        if file.is_null() || unsafe { (*fields).roadtramtype == 0 } {
            (s.set_roadtype)(e.shell, if tram { ROADTYPE_TRAM } else { ROADTYPE_ROAD });
            continue;
        }
        let slot = unsafe {
            (*fields).roadtramtype = (*fields).roadtramtype.wrapping_sub(1);
            (*fields).roadtramtype
        };
        if usize::from(slot) < (s.road_list_size)(file, tram) {
            let label = (s.road_label)(file, tram, usize::from(slot));
            let rt = (s.road_by_label)(label);
            if rt != INVALID_ROADTYPE && (shared.road_tram_type)(rt).0 == tram {
                (s.set_roadtype)(e.shell, rt);
                continue;
            }
        }
        (s.disable)(e.shell);
    }
    let mut from = 0;
    loop {
        let e = (s.next)(from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if e.kind.0 != VEH_TRAIN {
            continue;
        }
        let mut types = 0;
        let count = unsafe { (&*TEMP.get())[usize::from(e.id)].labels.len() };
        for index in 0..count {
            let label = unsafe { (&*TEMP.get())[usize::from(e.id)].labels[index] };
            let rt = (s.rail_by_label)(label);
            if rt != INVALID_RAILTYPE {
                types |= 1_u64 << rt;
            }
        }
        if types != 0 {
            (s.set_railtypes)(e.shell, types);
        } else {
            (s.disable)(e.shell);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_engine_reset_config(s: &Services) -> bool {
    let mut from = 0;
    loop {
        let vehicle = (s.vehicle_next)(from);
        if vehicle.id == u32::MAX {
            break;
        }
        from = vehicle.id + 1;
        if crate::vehicle_func::is_company_buildable_vehicle_type(vehicle.kind) {
            return false;
        }
    }
    unsafe {
        openttd_rust_engine_mapping_defaults((s.default_counts)());
    }
    (s.reload)();
    true
}

#[repr(C)]
pub struct LoadingVehicle {
    id: u32,
    kind: u8,
}
