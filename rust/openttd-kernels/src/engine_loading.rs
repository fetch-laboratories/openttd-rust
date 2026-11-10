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
use crate::debug::{Bytes, grf_msg};
use crate::raw_field::{read_field, write_field};
use crate::{
    cargo_registry::{
        openttd_rust_cargo_by_label, openttd_rust_cargo_is_default_abi, openttd_rust_cargo_mask,
        openttd_rust_cargo_translate, openttd_rust_standard_cargo_mask,
    },
    engine_type::INVALID_ENGINE,
    game_state::GameState,
    newgrf::{CargoLabel, INVALID_GRFID, get_active_cargo_label},
};
use std::{collections::BTreeMap, ffi::c_void, ptr};
pub(crate) mod properties;

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
/// `EngineOverrideManager::UseUnreservedID`: look for an unreserved `EngineID` matching the
/// local id, and reserve it if found. `grfid` is the scope of `local`: the ID of the
/// overridden `NewGRF` when one overrides another's engines, or `INVALID_GRFID` when dynamic
/// engines are disabled. `static_access` false actually reserves the `EngineID`. Returns the
/// engine ID if present and now reserved, or `INVALID_ENGINE` if not.
/// # Safety
/// The serialized engine-loading thread owns `MAPPINGS` for the call.
pub(crate) unsafe fn use_unreserved_id(
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
        crate::container_func::slide(map, index, index + 1, position);
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
pub unsafe extern "C" fn openttd_rust_engine_override_get_abi(source: u32, target: *mut u32) -> u8 {
    u8::from(
        if let Some(value) = unsafe { &*OVERRIDES.get() }.get(&source) {
            unsafe {
                *target = *value;
            }
            true
        } else {
            false
        },
    )
}
pub(crate) unsafe fn openttd_rust_engine_override_clear() {
    unsafe { &mut *OVERRIDES.get() }.clear();
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TempFields {
    cargo_allowed: u16,
    cargo_allowed_required: u16,
    cargo_disallowed: u16,
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
pub(crate) unsafe fn openttd_rust_engine_temp_resize(size: usize) {
    let temp = unsafe { &mut *TEMP.get() };
    temp.resize_with(size, Temp::default);
}
pub(crate) unsafe fn openttd_rust_engine_temp_clear() {
    unsafe { &mut *TEMP.get() }.clear();
}
fn temp(engine: u16) -> *mut Temp {
    // Vec::as_mut_ptr does not borrow elements; native field aliases stay valid.
    unsafe { (*TEMP.get()).as_mut_ptr().add(usize::from(engine)) }
}
pub(crate) unsafe fn openttd_rust_engine_temp_fields(engine: u16) -> *mut TempFields {
    unsafe { ptr::addr_of_mut!((*temp(engine)).fields) }
}
pub(crate) unsafe fn openttd_rust_engine_temp_refittability(
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
pub(crate) unsafe fn openttd_rust_engine_temp_labels_clear(engine: u16) {
    unsafe { &mut (*temp(engine)).labels }.clear();
}
pub(crate) unsafe fn openttd_rust_engine_temp_labels_push(engine: u16, label: u32) {
    unsafe { &mut (*temp(engine)).labels }.push(label);
}

use crate::vehicle_type::{VEH_COMPANY_END, VEH_TRAIN};
use crate::{engine::EngineRef, services::Services as Shared, vehicle::OpenTTDEngine};
type E = *const OpenTTDEngine;
type File = *const c_void;
#[repr(C)]
pub struct Services {
    identity: &'static crate::company_identity::Services,
    pub(crate) shared: *const Shared,
    pub(crate) resolver: *const crate::newgrf_vehicle::Services,
    types: *const crate::type_availability::Services,
    resolve: extern "C" fn(u16) -> EngineRef,
    pub(crate) dynamic: extern "C" fn() -> bool,
    pub(crate) find_file: extern "C" fn(u32) -> *mut c_void,
    mapping_log: extern "C" fn(u32, u32, bool),
    can_allocate: extern "C" fn() -> bool,
    pool_size: extern "C" fn() -> usize,
    construct: extern "C" fn(u8, u16) -> EngineRef,
    construct_at: extern "C" fn(u8, u16, u16),
    close_previews: extern "C" fn(),
    clean_pool: extern "C" fn(),
    cargo_next: extern "C" fn(u32) -> CargoClass,
    cargo_list_size: extern "C" fn(File) -> usize,
    pub(crate) unknown_refit: extern "C" fn(u32, u16, u16),
    use_livery: extern "C" fn(u8),
    variant_flags: extern "C" fn(E),
    override_log: extern "C" fn(u32, u32),
    vehicle_next: extern "C" fn(u32) -> LoadingVehicle,
    reload: extern "C" fn(),
    default_counts: extern "C" fn() -> *const u8,
    order_sort: extern "C" fn(*mut u16, usize),
    order_count: extern "C" fn() -> usize,
}
/// `Engine::Iterate(from)`, through the engine table's type-availability owner.
fn next(s: &Services, from: u32) -> EngineRef {
    // SAFETY: The immutable table names the process-lifetime type-availability table.
    unsafe { ((*s.types).next)(from) }
}
/// `_settings_game.game_creation.landscape`, loaded in place from the shared catalog.
fn climate(s: &Services) -> crate::landscape_type::LandscapeType {
    // SAFETY: The immutable shared table names the process-wide settings catalog.
    crate::landscape_type::LandscapeType(unsafe { (*s.shared).settings().climate.read() })
}
fn default_count(s: &Services, kind: u8) -> u8 {
    unsafe { *(s.default_counts)().add(usize::from(kind)) }
}
fn scope(s: &Services, file: File, dynamic: bool) -> u32 {
    if !dynamic {
        return INVALID_GRFID;
    }
    let id = (unsafe { &*s.shared }.grf_file_id)(file);
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
unsafe fn initialize_rail_labels(engine: EngineRef) {
    let railtypes = read_field!(OpenTTDEngine::rail(engine.shell), railtypes);
    unsafe {
        openttd_rust_engine_temp_labels_clear(engine.id);
    }
    for rt in 0_u8..64 {
        if railtypes & (1_u64 << rt) == 0 {
            continue;
        }
        let label = crate::type_registry::rail_label(rt);
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
        let file_id = (unsafe { &*s.shared }.grf_file_id)(file);
        if unsafe { &*OVERRIDES.get() }.contains_key(&file_id) {
            (s.mapping_log)(file_id, scope, !(s.find_file)(scope).is_null());
        }
        let id = unsafe { openttd_rust_engine_mapping_get(kind, local, scope) };
        if id != INVALID_ENGINE {
            let engine = (s.resolve)(id);
            if crate::raw_field::read_field!(engine.shell, bindings.grf_prop.grffile).is_null() {
                unsafe {
                    crate::engine::bindings::set_grf(&*s.shared, engine.shell, file);
                }
            }
            return engine.shell;
        }
    }
    let id = unsafe { use_unreserved_id(kind, local, scope, static_access) };
    if id != INVALID_ENGINE {
        let engine = (s.resolve)(id);
        if crate::raw_field::read_field!(engine.shell, bindings.grf_prop.grffile).is_null() {
            unsafe {
                crate::engine::bindings::set_grf(&*s.shared, engine.shell, file);
            }
            grf_msg!(
                5,
                "Replaced engine at index {} for GRFID {:x}, type {}, index {}",
                id,
                ((unsafe { &*s.shared }.grf_file_id)(file)).swap_bytes(),
                kind,
                local
            );
        }
        return engine.shell;
    }
    if static_access {
        return ptr::null();
    }
    if !(s.can_allocate)() {
        grf_msg!(0, "Can't allocate any more engines");
        return ptr::null();
    }
    let size = (s.pool_size)();
    let engine = (s.construct)(kind, local);
    unsafe {
        crate::engine::bindings::set_grf(&*s.shared, engine.shell, file);
    }
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
            initialize_rail_labels(engine);
        }
    }
    grf_msg!(
        5,
        "Created new engine at index {} for GRFID {:x}, type {}, index {}",
        engine.id,
        ((unsafe { &*s.shared }.grf_file_id)(file)).swap_bytes(),
        kind,
        local
    );
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
pub(crate) unsafe fn openttd_rust_engine_temp_initialize(s: &Services) {
    unsafe {
        openttd_rust_engine_temp_resize((s.pool_size)());
    }
    let mut from = 0;
    loop {
        let engine = next(s, from);
        if engine.shell.is_null() {
            break;
        }
        from = u32::from(engine.id) + 1;
        if engine.kind.0 == VEH_TRAIN {
            unsafe {
                initialize_rail_labels(engine);
            }
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDEngineLoadingVehicle" ["engine_loading"] => LoadingVehicle { id, kind => type };
    "OpenTTDEngineLoadingServices" ["engine_loading"] => Services {
        identity,
        shared,
        resolver,
        types,
        resolve,
        dynamic,
        find_file,
        mapping_log,
        can_allocate,
        pool_size,
        construct,
        construct_at,
        close_previews,
        clean_pool,
        cargo_next,
        cargo_list_size,
        unknown_refit,
        use_livery,
        variant_flags,
        override_log,
        vehicle_next,
        reload,
        default_counts,
        order_sort,
        order_count,
    };
    "EngineIDMapping" ["engine_loading"] => Mapping { grfid, internal_id, kind => type, substitute_id, engine };
    "OpenTTDEngineMappingView" ["engine_loading"] => MappingView { data, size };
    "OpenTTDNewGRFCargoLabel" ["engine_loading"] => CargoLabel { value, mixed };
    "OpenTTDEngineCargoClass" ["engine_loading"] => CargoClass { classes, cargo };

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
pub extern "C" fn openttd_rust_engine_translate_refit(mask: u32, file: File) -> u64 {
    let mut result = 0;
    for bit in 0_u8..32 {
        if mask & (1 << bit) == 0 {
            continue;
        }
        let cargo = openttd_rust_cargo_translate(bit, file, true);
        if is_valid_cargo_type(cargo) {
            result |= 1_u64 << cargo;
        }
    }
    result
}
fn engine_label(shell: E) -> u32 {
    get_active_cargo_label(read_field!(shell, info.cargo_label), |label| {
        openttd_rust_cargo_by_label(label)
    })
}
fn default_masks(s: &Services, kind: u8, shell: E, id: u16, label: u32) -> (u16, u16) {
    if kind == VEH_AIRCRAFT {
        (PASSENGERS | CC_MAIL | CC_ARMOURED | CC_EXPRESS, CC_LIQUID)
    } else if kind == VEH_SHIP {
        match label {
            // Ferries
            CT_PASSENGERS => (PASSENGERS, 0),
            // Tankers
            CT_OIL => (CC_LIQUID, 0),
            // Cargo ships
            // No tanker in toyland :(
            _ if climate(s) == crate::landscape_type::LandscapeType::Toyland => (
                CC_MAIL | CC_ARMOURED | CC_EXPRESS | CC_BULK | CC_PIECEGOODS | CC_LIQUID,
                PASSENGERS,
            ),
            _ => (
                CC_MAIL | CC_ARMOURED | CC_EXPRESS | CC_BULK | CC_PIECEGOODS,
                CC_LIQUID | PASSENGERS,
            ),
        }
    } else if kind == VEH_TRAIN
        && crate::vehicle::EngineRef::of(shell).rail_kind() != RailVehicleTypes::WAGON
    {
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
        let climate = climate(s).0;
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
pub(crate) unsafe fn openttd_rust_engine_calculate_refits(s: &Services) {
    let mut original_known_cargoes = 0;
    for cargo in 0..NUM_CARGO {
        if openttd_rust_cargo_is_default_abi(cargo) != 0 {
            original_known_cargoes |= 1_u64 << cargo;
        }
    }
    let mut from = 0;
    loop {
        let e = next(s, from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        let mut cargo = read_field!(e.shell, info.cargo_type);
        if !is_valid_cargo_type(cargo) {
            cargo = openttd_rust_cargo_by_label(engine_label(e.shell));
            write_field!(e.shell.cast_mut(), info.cargo_type, cargo);
        }
        let fields = unsafe { openttd_rust_engine_temp_fields(e.id) };
        if unsafe { (*fields).defaultcargo_grf.is_null() } {
            if e.kind.0 != VEH_TRAIN || read_field!(OpenTTDEngine::rail(e.shell), capacity) != 0 {
                let label = if e.kind.0 == VEH_AIRCRAFT
                    || (e.kind.0 == VEH_TRAIN
                        && crate::vehicle::EngineRef::of(e.shell).rail_kind()
                            != RailVehicleTypes::WAGON)
                {
                    CT_INVALID
                } else {
                    engine_label(e.shell)
                };
                let (allowed, disallowed) = default_masks(s, e.kind.0, e.shell, e.id, label);
                unsafe {
                    (*fields).cargo_allowed = allowed;
                    (*fields).cargo_disallowed = disallowed;
                }
                if e.kind.0 == VEH_SHIP {
                    write_field!(OpenTTDEngine::ship(e.shell), old_refittable, true);
                } else if e.kind.0 != VEH_AIRCRAFT
                    && (e.kind.0 != VEH_TRAIN
                        || crate::vehicle::EngineRef::of(e.shell).rail_kind()
                            == RailVehicleTypes::WAGON)
                {
                    unsafe {
                        (*fields).ctt_exclude_mask = original_known_cargoes;
                    }
                }
            }
            unsafe {
                openttd_rust_engine_temp_refittability(fields, (*fields).cargo_allowed != 0);
                if is_valid_cargo_type(cargo) {
                    (*fields).ctt_exclude_mask &= !(1_u64 << cargo);
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
        let original_mask = read_field!(e.shell, info.refit_mask);
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
        let mut refit_mask = (((mask & !not_mask) ^ original_mask)
            & unsafe { openttd_rust_cargo_mask().read() }
            | include)
            & !exclude;
        write_field!(e.shell.cast_mut(), info.refit_mask, refit_mask);
        let file = if default_file.is_null() {
            unsafe { (*e.shell).bindings.grf_prop.grffile }
        } else {
            default_file
        };
        if !file.is_null()
            && read_field!(e.shell, info.callback_mask)
                & crate::newgrf_callbacks::VEHICLE_CUSTOM_REFIT
                != 0
        {
            let mut cargo_from = 0;
            loop {
                let cs = (s.cargo_next)(cargo_from);
                if cs.cargo == INVALID_CARGO {
                    break;
                }
                cargo_from = u32::from(cs.cargo) + 1;
                let local_slot = (unsafe { &*s.shared }.grf_cargo_map)(file, cs.cargo);
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
                        refit_mask |= 1_u64 << cs.cargo;
                        write_field!(e.shell.cast_mut(), info.refit_mask, refit_mask);
                    }
                    2 => {
                        refit_mask &= !(1_u64 << cs.cargo);
                        write_field!(e.shell.cast_mut(), info.refit_mask, refit_mask);
                    }
                    _ => {
                        (s.unknown_refit)(
                            (unsafe { &*s.shared }.grf_file_id)(file),
                            CBID_VEHICLE_CUSTOM_REFIT.0,
                            callback,
                        );
                    }
                }
            }
        }
        if is_valid_cargo_type(cargo)
            && unsafe { openttd_rust_cargo_mask().read() } & (1_u64 << cargo) == 0
        {
            cargo = INVALID_CARGO;
            write_field!(e.shell.cast_mut(), info.cargo_type, cargo);
        }
        if !only_default
            && (e.kind.0 != VEH_SHIP || read_field!(OpenTTDEngine::ship(e.shell), old_refittable))
            && is_valid_cargo_type(cargo)
            && refit_mask & (1_u64 << cargo) == 0
        {
            cargo = INVALID_CARGO;
            write_field!(e.shell.cast_mut(), info.cargo_type, cargo);
        }
        if !is_valid_cargo_type(cargo) && refit_mask != 0 {
            let file = if default_file.is_null() {
                unsafe { (*e.shell).bindings.grf_prop.grffile }
            } else {
                default_file
            };
            if !file.is_null()
                && (unsafe { &*s.shared }.grf_file_version)(file) >= 8
                && (s.cargo_list_size)(file) != 0
            {
                let mut best_local_slot = u8::MAX;
                for candidate_cargo in 0..NUM_CARGO {
                    if refit_mask & (1_u64 << candidate_cargo) == 0 {
                        continue;
                    }
                    let local = (unsafe { &*s.shared }.grf_cargo_map)(file, candidate_cargo);
                    if local < best_local_slot {
                        best_local_slot = local;
                        cargo = candidate_cargo;
                        write_field!(e.shell.cast_mut(), info.cargo_type, cargo);
                    }
                }
            }
            if !is_valid_cargo_type(cargo) {
                cargo = crate::bitmath_func::find_first_bit(refit_mask);
                write_field!(e.shell.cast_mut(), info.cargo_type, cargo);
            }
        }
        if !is_valid_cargo_type(cargo)
            && e.kind.0 == VEH_TRAIN
            && crate::vehicle::EngineRef::of(e.shell).rail_kind() != RailVehicleTypes::WAGON
            && read_field!(OpenTTDEngine::rail(e.shell), capacity) == 0
            && engine_label(e.shell) != CT_INVALID
        {
            cargo = crate::bitmath_func::find_first_bit(unsafe {
                openttd_rust_standard_cargo_mask().read()
            });
            write_field!(e.shell.cast_mut(), info.cargo_type, cargo);
        }
        if !is_valid_cargo_type(cargo) {
            write_field!(e.shell.cast_mut(), info.climates, 0);
        }
        if e.kind.0 == VEH_SHIP && !read_field!(OpenTTDEngine::ship(e.shell), old_refittable) {
            write_field!(e.shell.cast_mut(), info.refit_mask, 0);
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
        .get(&(unsafe { &*s.shared }.grf_file_id)(file))
        .copied();
    target.map_or(ptr::null(), |target| (s.find_file)(target).cast_const())
}
pub(crate) unsafe fn openttd_rust_engine_finalise(s: &Services) {
    let mut from = 0;
    loop {
        let e = next(s, from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if unsafe { (*e.shell).bindings.grf_prop.grffile }.is_null() {
            let invalid = unsafe {
                (&*MAPPINGS.get())[usize::from(e.kind.0)]
                    .iter()
                    .find(|m| m.engine == e.id)
                    .is_none_or(|m| {
                        m.grfid != INVALID_GRFID || m.internal_id != u16::from(m.substitute_id)
                    })
            };
            if invalid {
                write_field!(e.shell.cast_mut(), info.string_id, unsafe {
                    (*s.resolver).capabilities.invalid_string_id
                });
            }
        }
        let variant = read_field!(e.shell, info.variant_id).0;
        if variant != INVALID_ENGINE {
            let id = unsafe {
                openttd_rust_engine_new_id(
                    s,
                    (*e.shell).bindings.grf_prop.grffile,
                    e.kind.0,
                    variant,
                )
            };
            write_field!(
                e.shell.cast_mut(),
                info.variant_id,
                crate::engine_type::EngineID(id)
            );
        }
        if !crate::vehicle::EngineRef::of(e.shell).climate(climate(s).0) {
            continue;
        }
        match e.kind.0 {
            VEH_TRAIN => {
                let railtypes = read_field!(OpenTTDEngine::rail(e.shell), railtypes);
                for rt in 0_u8..64 {
                    if railtypes & (1_u64 << rt) != 0 {
                        unsafe {
                            crate::engine::bindings::append_copyable_badges(
                                &*s.shared,
                                e.shell,
                                ((*s.resolver).rail_badge)(crate::services::RailType(rt)),
                                crate::newgrf::GSF_TRAINS,
                            );
                        }
                    }
                }
            }
            VEH_ROAD => unsafe {
                let rt = (*OpenTTDEngine::road(e.shell)).roadtype;
                crate::engine::bindings::append_copyable_badges(
                    &*s.shared,
                    e.shell,
                    ((*s.resolver).road_badge)(crate::services::RoadType(rt)),
                    crate::newgrf::GSF_ROADVEHICLES,
                );
            },
            _ => (),
        }
        if e.kind.0 != VEH_TRAIN
            || crate::vehicle::EngineRef::of(e.shell).rail_kind() != RailVehicleTypes::WAGON
        {
            let ls = crate::company_identity::engine_livery(
                s.identity,
                crate::engine_type::EngineID(e.id),
                crate::engine_type::EngineID::INVALID,
                std::ptr::null_mut(),
            );
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
        let e = next(s, from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        let mut parent = read_field!(e.shell, info.variant_id).0;
        let mut parent_slow = parent;
        let mut update_slow = false;
        while parent != INVALID_ENGINE {
            parent = read_field!((s.resolve)(parent).shell, info.variant_id).0;
            if update_slow {
                parent_slow = read_field!((s.resolve)(parent_slow).shell, info.variant_id).0;
            }
            update_slow = !update_slow;
            if parent != e.id && parent != parent_slow {
                continue;
            }
            write_field!(
                e.shell.cast_mut(),
                info.variant_id,
                crate::engine_type::EngineID(INVALID_ENGINE)
            );
            let shared = unsafe { &*s.shared };
            grf_msg!(
                1,
                "FinaliseEngineArray: Variant of engine {:x} in '{}' loops back on itself",
                crate::raw_field::read_field!(e.shell, bindings.grf_prop.local_id),
                Bytes(unsafe {
                    (shared.grf_file_name)((*e.shell).bindings.grf_prop.grffile.cast()).bytes()
                })
            );
            break;
        }
        let parent = read_field!(e.shell, info.variant_id).0;
        if parent != INVALID_ENGINE {
            (s.variant_flags)((s.resolve)(parent).shell);
        }
    }
}
pub(crate) unsafe fn openttd_rust_engine_bind_types(s: &Services) {
    let mut from = 0;
    loop {
        let e = next(s, from);
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
            write_field!(
                OpenTTDEngine::road(e.shell),
                max_speed,
                u16::from(speed) * 4
            );
        }
        let tram = (read_field!(e.shell, info.misc_flags) & crate::engine_type::ROAD_IS_TRAM != 0);
        let file = unsafe { (*e.shell).bindings.grf_prop.grffile };
        if file.is_null() || unsafe { (*fields).roadtramtype == 0 } {
            write_field!(
                OpenTTDEngine::road(e.shell),
                roadtype,
                if tram { ROADTYPE_TRAM } else { ROADTYPE_ROAD }
            );
            continue;
        }
        let slot = unsafe {
            (*fields).roadtramtype = (*fields).roadtramtype.wrapping_sub(1);
            (*fields).roadtramtype
        };
        let rt = crate::type_registry::openttd_rust_road_translate(
            crate::services::RoadTramType(tram),
            slot,
            file,
        );
        if rt != INVALID_ROADTYPE {
            write_field!(OpenTTDEngine::road(e.shell), roadtype, rt);
            continue;
        }
        write_field!(e.shell.cast_mut(), info.climates, 0);
    }
    let mut from = 0;
    loop {
        let e = next(s, from);
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
            let rt = crate::type_registry::openttd_rust_rail_by_label(label, true);
            if rt != INVALID_RAILTYPE {
                types |= 1_u64 << rt;
            }
        }
        if types != 0 {
            {
                let rail = unsafe { OpenTTDEngine::rail(e.shell) };
                write_field!(rail, railtypes, types);
                write_field!(rail, intended_railtypes, types);
            };
        } else {
            write_field!(e.shell.cast_mut(), info.climates, 0);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_engine_reset_config_abi(s: &Services) -> u8 {
    let mut from = 0;
    loop {
        let vehicle = (s.vehicle_next)(from);
        if vehicle.id == u32::MAX {
            break;
        }
        from = vehicle.id + 1;
        if crate::vehicle_func::is_company_buildable_vehicle_type(vehicle.kind) {
            return u8::from(false);
        }
    }
    unsafe {
        openttd_rust_engine_mapping_defaults((s.default_counts)());
    }
    (s.reload)();
    u8::from(true)
}

#[repr(C)]
pub struct LoadingVehicle {
    id: u32,
    kind: u8,
}

#[derive(Clone, Copy)]
struct ListOrderChange {
    engine: u16,
    target: u16,
}
static LIST_ORDER_CHANGES: GameState<Vec<ListOrderChange>> = GameState::new(Vec::new());
pub(crate) unsafe fn openttd_rust_vehicle_order(engine: u16, target: u16) {
    unsafe { &mut *LIST_ORDER_CHANGES.get() }.push(ListOrderChange { engine, target });
}
pub(crate) unsafe fn openttd_rust_vehicle_order_commit(s: &Services) {
    let mut ordering: Vec<u16> = (0..(s.order_count)()).map(|id| id as u16).collect();
    (s.order_sort)(ordering.as_mut_ptr(), ordering.len());
    let count = unsafe { &*LIST_ORDER_CHANGES.get() }.len();
    for index in 0..count {
        let change = unsafe { (&*LIST_ORDER_CHANGES.get())[index] };
        let engine = (s.resolve)(change.engine);
        if read_field!(engine.shell, bindings.grf_prop.local_id) == change.target {
            continue;
        }
        let kind = read_field!(engine.shell, r#type).0;
        let grfid = read_field!(engine.shell, bindings.grf_prop.grfid);
        let target = unsafe { openttd_rust_engine_mapping_get(kind, change.target, grfid) };
        if target == INVALID_ENGINE {
            continue;
        }
        let from = ordering.iter().position(|id| *id == change.engine);
        let to = ordering.iter().position(|id| *id == target);
        crate::movement::native_assert(unsafe { &*s.shared }, from.is_some() && to.is_some());
        crate::movement::native_assert(unsafe { &*s.shared }, from != to);
        // The native assertions abort before these indices are used when enabled.
        let from = from.unwrap_or(ordering.len());
        let to = to.unwrap_or(ordering.len());
        crate::container_func::slide(&mut ordering, from, from + 1, to);
    }
    for (index, id) in ordering.into_iter().enumerate() {
        write_field!(
            (s.resolve)(id).shell.cast_mut(),
            list_position,
            index as u16
        );
    }
    unsafe { &mut *LIST_ORDER_CHANGES.get() }.clear();
    unsafe { &mut *LIST_ORDER_CHANGES.get() }.shrink_to_fit();
}

/// Native badge owner consumes the feature mask immediately; only pure pool lookup
/// occurs between ordered appends, and no record view survives that lookup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_engine_file_badge(
    s: &Services,
    file: File,
    badge: u32,
) -> u32 {
    finalise_file_badges(s, file, crate::newgrf_badge_type::BadgeID(badge)).0
}
/// `FinaliseBadges`' engine loop for one GRF file's badge: append the badge to every engine
/// from `file`, returning the features of those engines' types.
fn finalise_file_badges(
    s: &Services,
    file: File,
    badge: crate::newgrf_badge_type::BadgeID,
) -> crate::newgrf::GrfSpecFeatures {
    use crate::newgrf::{GrfSpecFeature, GrfSpecFeatures};
    let mut features = GrfSpecFeatures::EMPTY;
    let mut from = 0;
    loop {
        let e = next(s, from);
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if crate::raw_field::read_field!(e.shell, bindings.grf_prop.grffile) != file {
            continue;
        }
        unsafe {
            (*e.shell.cast_mut()).bindings.badges.push(badge.0);
        }
        features.set(GrfSpecFeature(GrfSpecFeature::GSF_TRAINS.0 + e.kind.0));
    }
    features
}

#[cfg(test)]
mod tests {
    use super::*;
    mod mappings;
    #[test]
    fn loading_state_and_order_queue_cross_serialized_threads() {
        unsafe {
            openttd_rust_engine_temp_resize(3);
            (*temp(1)).fields.cargo_allowed = 123;
            openttd_rust_engine_temp_labels_push(1, crate::rail_type::RAILTYPE_LABEL_RAIL);
            openttd_rust_engine_temp_refittability(&raw mut (*temp(1)).fields, false);
            openttd_rust_vehicle_order(1, 7);
        }
        std::thread::spawn(|| unsafe {
            assert_eq!((*temp(1)).fields.cargo_allowed, 123);
            assert_eq!((*temp(1)).fields.refittability, TempFields::EMPTY);
            assert_eq!((&*LIST_ORDER_CHANGES.get())[0].target, 7);
            openttd_rust_engine_temp_refittability(&raw mut (*temp(1)).fields, true);
            (*temp(1)).fields.ctt_include_mask = 7;
            openttd_rust_vehicle_order(2, 8);
        })
        .join()
        .unwrap();
        unsafe {
            openttd_rust_engine_temp_resize(70);
            assert_eq!((*temp(1)).fields.ctt_include_mask, 7);
            assert_eq!(
                (&(*temp(1)).labels)[0],
                crate::rail_type::RAILTYPE_LABEL_RAIL
            );
            assert_eq!((*temp(1)).fields.refittability, TempFields::NONEMPTY);
            openttd_rust_engine_temp_refittability(&raw mut (*temp(1)).fields, false);
            assert_eq!((*temp(1)).fields.refittability, TempFields::NONEMPTY);
            assert_eq!((*temp(69)).fields.refittability, TempFields::UNSET);
            assert_eq!((&*LIST_ORDER_CHANGES.get())[1].engine, 2);
            assert_eq!((&*LIST_ORDER_CHANGES.get())[1].target, 8);
            openttd_rust_engine_temp_clear();
            openttd_rust_engine_temp_resize(3);
            assert_eq!((*temp(1)).fields.cargo_allowed, 0);
            assert_eq!((*temp(1)).labels, [] as [u32; 0]);
            assert!((*temp(1)).fields.defaultcargo_grf.is_null());
            properties::partial_read_tests();
            openttd_rust_engine_temp_clear();
            (*LIST_ORDER_CHANGES.get()).clear();
        }
    }
}
