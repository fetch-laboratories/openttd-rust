/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Cargo registry and stack invocation policy. Native specifications/files remain canonical;
//! reset and rebuild exclude runtime calls. Native views alias the sole Rust buffers.
//! No registry reference survives a service call; nested Action2 invocations own stack state.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
use crate::bitmath_func::{clr_bit64, set_bit64};
use crate::cargo_type::{CT_INVALID, CargoLabel, CargoType, NUM_CARGO, NUM_ORIGINAL_CARGO};
use crate::cargotype::{NUM_TPE, TownAcceptanceEffect, TownProductionEffect, is_cargo_spec_valid};
use crate::game_state::GameState;
use crate::gfx_type::SpriteID;
use crate::newgrf::GrfSpecFeature;
use crate::newgrf_callbacks::{CBID_NO_CALLBACK, CallbackID};
use crate::newgrf_spritegroup::{self, State};
use std::{collections::BTreeMap, ffi::c_void, ptr};
type Handle = *const c_void;
#[repr(C)]
pub struct ProductionCargo {
    pub cargo: u8,
    pub multiplier: u16,
}
#[repr(C)]
pub struct Sprite {
    sprite: u32,
    count: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LabelSpec {
    label: u32,
    bitnum: u8,
}
#[repr(C)]
pub struct FinaliseSpec {
    label: u32,
    bitnum: u8,
    effect: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct OrderSpec {
    cargo: u8,
    effect: u8,
    special: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
/// Data stays live through pure searches; native vector limits fit a Rust slice.
pub struct Labels {
    pub(crate) data: *const u32,
    pub(crate) size: usize,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct File {
    labels: Labels,
    version: u8,
    fallback: bool,
}
#[repr(C)]
pub struct Invocation {
    label: u32,
    file: Handle,
    root: Handle,
    callback: u16,
    param1: u32,
    pub(crate) param2: u32,
}
#[repr(C)]
pub struct Services {
    shared: *const crate::services::Services,
    common: *const newgrf_spritegroup::Services,
    pub(crate) spec: extern "C" fn(u8) -> Handle,
    copy_default: extern "C" fn(u8, u8),
    clear_specs: extern "C" fn(u8),
    label_spec: extern "C" fn(Handle) -> LabelSpec,
    bitnum: extern "C" fn(Handle) -> u8,
    town_acceptance_effect: extern "C" fn(u8) -> u8,
    finalise_spec: extern "C" fn(Handle) -> FinaliseSpec,
    order_spec: extern "C" fn(Handle) -> OrderSpec,
    index: extern "C" fn(Handle) -> u8,
    production: extern "C" fn(Handle) -> ProductionCargo,
    sort: extern "C" fn(*mut Handle, usize),
    publish_sorted: extern "C" fn(*mut Handle, usize),
    publish_standard: extern "C" fn(*mut Handle, usize),
    publish_town: extern "C" fn(u8, *mut Handle, usize),
    set_file: extern "C" fn(Handle, Handle),
    set_effect: extern "C" fn(Handle, u8),
    invalidate_strings: extern "C" fn(Handle),
    file: extern "C" fn(Handle) -> File,
    assign_fallback: extern "C" fn(*mut c_void, Labels),
    resize_fallback: extern "C" fn(*mut c_void, usize),
    set_fallback: extern "C" fn(*mut c_void, u32, u32),
    clear_inverse: extern "C" fn(*mut c_void),
    set_inverse: extern "C" fn(*mut c_void, u8, u8),
    pub(crate) invocation: extern "C" fn(Handle, *mut Invocation),
    pub(crate) callback_frame: extern "C" fn(*const Invocation, *mut i32, usize) -> u16,
    sprite_frame: extern "C" fn(*const Invocation) -> u32,
    sprite: extern "C" fn(Handle) -> Sprite,
}
/// Test-only table for `crate::test_tables`; its private fields keep it in this module.
#[cfg(test)]
pub(crate) const TEST_TABLE: Services = Services {
    shared: std::ptr::null(),
    common: std::ptr::null(),
    spec: crate::test_tables::unreachable1,
    copy_default: crate::test_tables::unreachable2,
    clear_specs: crate::test_tables::unreachable1,
    label_spec: crate::test_tables::unreachable1,
    bitnum: crate::test_tables::unreachable1,
    town_acceptance_effect: crate::test_tables::unreachable1,
    finalise_spec: crate::test_tables::unreachable1,
    order_spec: crate::test_tables::unreachable1,
    index: crate::test_tables::unreachable1,
    production: crate::test_tables::unreachable1,
    sort: crate::test_tables::unreachable2,
    publish_sorted: crate::test_tables::unreachable2,
    publish_standard: crate::test_tables::unreachable2,
    publish_town: crate::test_tables::unreachable3,
    set_file: crate::test_tables::unreachable2,
    set_effect: crate::test_tables::unreachable2,
    invalidate_strings: crate::test_tables::unreachable1,
    file: crate::test_tables::unreachable1,
    assign_fallback: crate::test_tables::unreachable2,
    resize_fallback: crate::test_tables::unreachable2,
    set_fallback: crate::test_tables::unreachable3,
    clear_inverse: crate::test_tables::unreachable1,
    set_inverse: crate::test_tables::unreachable3,
    invocation: crate::test_tables::unreachable2,
    callback_frame: crate::test_tables::unreachable3,
    sprite_frame: crate::test_tables::unreachable1,
    sprite: crate::test_tables::unreachable1,
};
struct Registry {
    services: *const Services,
    /// Available cargoes, including phony cargoes such as regearing cargoes.
    cargo_mask: u64,
    /// Real cargoes, excluding phony cargoes such as regearing cargoes.
    standard_cargo_mask: u64,
    // Default vehicle cargoes are identified by label so a label can move to another slot.
    default_cargo_labels: Vec<CargoLabel>,
    // Original 12 climate-dependent slots map via label to cargo type; absent cargoes use CT_INVALID.
    climate_dependent_cargo_labels: [u32; NUM_ORIGINAL_CARGO as usize],
    // Version 8+ translation maps 32 original bitnum slots to labels, or CT_INVALID if absent.
    climate_independent_cargo_labels: [u32; 32],
    label_map: BTreeMap<CargoLabel, CargoType>,
    sorted_cargo_specs: Vec<Handle>,
    sorted_cargo_types: [u8; NUM_CARGO as usize],
    town_production_cargoes: [Vec<Handle>; NUM_TPE as usize],
}
static REGISTRY: GameState<Registry> = GameState::new(Registry {
    services: ptr::null(),
    cargo_mask: 0,
    standard_cargo_mask: 0,
    default_cargo_labels: Vec::new(),
    climate_dependent_cargo_labels: [0; NUM_ORIGINAL_CARGO as usize],
    climate_independent_cargo_labels: [0; 32],
    label_map: BTreeMap::new(),
    sorted_cargo_specs: Vec::new(),
    sorted_cargo_types: [0; NUM_CARGO as usize],
    town_production_cargoes: [const { Vec::new() }; NUM_TPE as usize],
});
fn services() -> &'static Services {
    unsafe { &*(*REGISTRY.get()).services }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).cargo_mask }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_standard_cargo_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).standard_cargo_mask }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_reverse() -> *mut u8 {
    unsafe { (&raw mut (*REGISTRY.get()).sorted_cargo_types).cast() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_setup(s: *const Services, climate: u8) {
    let l = crate::landscape_type::LandscapeType(climate);
    // SAFETY: serialized native loading excludes all runtime registry views.
    unsafe {
        (*REGISTRY.get()).services = s;
        crate::movement::native_assert(&*(*s).shared, l.0 < crate::landscape_type::NUM_LANDSCAPE);
        (*REGISTRY.get()).cargo_mask = 0;
        (*REGISTRY.get()).default_cargo_labels.clear();
        (*REGISTRY.get())
            .climate_dependent_cargo_labels
            .fill(CT_INVALID);
        (*REGISTRY.get())
            .climate_independent_cargo_labels
            .fill(CT_INVALID);
    }
    let s = unsafe { &*s };
    // Copy from default cargo by label or index.
    for slot in 0..NUM_ORIGINAL_CARGO {
        (s.copy_default)(l.0, slot);
        let cs = (s.spec)(slot);
        let spec = (s.label_spec)(cs);
        if is_cargo_spec_valid(spec.bitnum) {
            let cargo = CargoType((s.index)(cs));
            unsafe {
                set_bit64(&mut (*REGISTRY.get()).cargo_mask, cargo.0);
                (*REGISTRY.get())
                    .default_cargo_labels
                    .push(CargoLabel(spec.label));
                (*REGISTRY.get()).climate_dependent_cargo_labels[usize::from(cargo.0)] = spec.label;
                (*REGISTRY.get()).climate_independent_cargo_labels[usize::from(spec.bitnum)] =
                    spec.label;
            }
        }
    }
    // Reset and disable remaining cargo types.
    (s.clear_specs)(NUM_ORIGINAL_CARGO);
    openttd_rust_cargo_labels_rebuild();
}
/// `BuildCargoLabelMap`, called repeatedly as `NewGRF` cargoes are defined so initialization
/// callers of `TranslateRefitMask` and `GetCargoTranslation` see the correct information.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_labels_rebuild() {
    unsafe {
        (*REGISTRY.get()).label_map.clear();
    }
    let s = services();
    for cargo in 0..NUM_CARGO {
        let cs = (s.spec)(cargo);
        let spec = (s.label_spec)(cs);
        let label = CargoLabel(spec.label);
        // During initialization a CargoSpec can be marked valid before its label is set.
        if is_cargo_spec_valid(spec.bitnum)
            && label != CargoLabel(0)
            && label != CargoLabel::CT_INVALID
        {
            let index = CargoType((s.index)(cs));
            unsafe {
                // A label that already exists keeps its first cargo slot.
                (*REGISTRY.get()).label_map.entry(label).or_insert(index);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_bitnum_changed(spec: Handle, cargo: u8, file: Handle) {
    let s = services();
    if is_cargo_spec_valid((s.bitnum)(spec)) {
        (s.set_file)(spec, file);
        unsafe {
            set_bit64(&mut (*REGISTRY.get()).cargo_mask, cargo);
        }
    } else {
        unsafe {
            clr_bit64(&mut (*REGISTRY.get()).cargo_mask, cargo);
        }
    }
    openttd_rust_cargo_labels_rebuild();
}
/// `GetCargoTypeByLabel`: the cargo type of `label`, or `INVALID_CARGO`.
pub(crate) fn get_cargo_type_by_label(label: CargoLabel) -> CargoType {
    unsafe {
        (*REGISTRY.get())
            .label_map
            .get(&label)
            .copied()
            .unwrap_or(CargoType::INVALID_CARGO)
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_by_label(label: u32) -> u8 {
    get_cargo_type_by_label(CargoLabel(label)).0
}
/// Canonical `CargoSpec::label`, including duplicate labels in distinct slots.
pub(crate) fn cargo_label(cargo: u8) -> u32 {
    let cargo = CargoType(cargo);
    let s = services();
    CargoLabel((s.label_spec)((s.spec)(cargo.0)).label).0
}
/// `IsDefaultCargo`: test if a cargo is a default cargo type.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_is_default_abi(cargo: u8) -> u8 {
    let cargo_type = CargoType(cargo);
    let s = services();
    let cs = (s.label_spec)((s.spec)(cargo_type.0));
    u8::from(
        is_cargo_spec_valid(cs.bitnum)
            && unsafe {
                (*REGISTRY.get())
                    .default_cargo_labels
                    .contains(&CargoLabel(cs.label))
            },
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_climate_labels() -> Labels {
    Labels {
        data: unsafe { (&raw const (*REGISTRY.get()).climate_dependent_cargo_labels).cast() },
        size: usize::from(NUM_ORIGINAL_CARGO),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_bitnum_labels() -> Labels {
    Labels {
        data: unsafe { (&raw const (*REGISTRY.get()).climate_independent_cargo_labels).cast() },
        size: 32,
    }
}

/// Scalar count adapter for `CargoSpec::town_production_cargoes`.
pub fn town_count(effect: u8) -> usize {
    let effect = TownProductionEffect(effect);
    unsafe { (*REGISTRY.get()).town_production_cargoes[usize::from(effect.0)].len() }
}
/// Scalar cargo-index adapter for `CargoSpec::town_production_cargoes`.
pub fn town_cargo(effect: u8, index: usize) -> u8 {
    let effect = TownProductionEffect(effect);
    let spec = unsafe {
        let state = &*REGISTRY.get();
        *state.town_production_cargoes[usize::from(effect.0)].get_unchecked(index)
    };
    CargoType((services().index)(spec)).0
}
/// Scalar production adapter for `CargoSpec::town_production_cargoes`.
pub fn production(effect: u8, index: u32) -> ProductionCargo {
    let effect = TownProductionEffect(effect);
    let spec = unsafe {
        let state = &*REGISTRY.get();
        state.town_production_cargoes[usize::from(effect.0)]
            .get(index as usize)
            .copied()
    };
    spec.map_or(
        ProductionCargo {
            cargo: CargoType::INVALID_CARGO.0,
            multiplier: 0,
        },
        |spec| (services().production)(spec),
    )
}
/// Initialize the list of sorted cargo specifications.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_sort(native: *const Services) {
    // Language loading sorts the initially empty specification array before climate setup.
    unsafe {
        (*REGISTRY.get()).services = native;
    }
    let s = services();
    for effect in 0..TownProductionEffect::NUM_TPE.0 {
        let effect = TownProductionEffect(effect);
        let data = unsafe {
            let tpc = &mut (*REGISTRY.get()).town_production_cargoes[usize::from(effect.0)];
            tpc.clear();
            tpc.as_mut_ptr()
        };
        (s.publish_town)(effect.0, data, 0);
    }
    let data = unsafe {
        (*REGISTRY.get()).sorted_cargo_specs.clear();
        (*REGISTRY.get()).sorted_cargo_specs.as_mut_ptr()
    };
    (s.publish_sorted)(data, 0);
    // Add each cargo spec to the list, and determine the largest cargo icon size.
    for (_, cargo) in crate::cargotype::iterate(
        |cargo| (s.spec)(cargo),
        |spec| is_cargo_spec_valid((s.bitnum)(*spec)),
    ) {
        unsafe {
            (*REGISTRY.get()).sorted_cargo_specs.push(cargo);
        }
    }
    let (data, count) = unsafe {
        (
            (*REGISTRY.get()).sorted_cargo_specs.as_mut_ptr(),
            (*REGISTRY.get()).sorted_cargo_specs.len(),
        )
    };
    (s.publish_sorted)(data, count);
    // Sort cargo specifications by cargo class and name.
    (s.sort)(data, count);
    // Populate.
    for index in 0..count {
        let cargo = CargoType((s.index)(unsafe { data.add(index).read() }));
        unsafe {
            (*REGISTRY.get()).sorted_cargo_types[usize::from(cargo.0)] = index as u8;
        }
    }
    // Count the number of standard cargos and fill the mask.
    unsafe {
        (*REGISTRY.get()).standard_cargo_mask = 0;
    }
    let mut nb_standard_cargo = 0;
    for index in 0..count {
        let cargo = unsafe { data.add(index).read() };
        let order = (s.order_spec)(cargo);
        let town_production_effect = TownProductionEffect(order.effect);
        crate::movement::native_assert(
            unsafe { &*s.shared },
            town_production_effect != TownProductionEffect::INVALID_TPE,
        );
        let (town_data, town_count) = unsafe {
            let tpc = &mut (*REGISTRY.get()).town_production_cargoes
                [usize::from(town_production_effect.0)];
            tpc.push(cargo);
            (tpc.as_mut_ptr(), tpc.len())
        };
        (s.publish_town)(town_production_effect.0, town_data, town_count);
        if order.special {
            break;
        }
        nb_standard_cargo += 1;
        let cargo_type = CargoType(order.cargo);
        unsafe {
            set_bit64(&mut (*REGISTRY.get()).standard_cargo_mask, cargo_type.0);
        }
    }
    // _sorted_standard_cargo_specs is a subset of _sorted_cargo_specs.
    (s.publish_standard)(data, nb_standard_cargo);
}
/// `FinaliseCargoArray`: check for invalid cargoes.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_finalise() {
    let s = services();
    for cargo in 0..CargoType::NUM_CARGO.0 {
        let cargo = CargoType(cargo);
        let cs = (s.spec)(cargo.0);
        let info = (s.finalise_spec)(cs);
        let town_production_effect = TownProductionEffect(info.effect);
        if town_production_effect == TownProductionEffect::INVALID_TPE {
            // Set default town production effect by cargo label.
            let label = CargoLabel(info.label);
            (s.set_effect)(
                cs,
                match label {
                    CargoLabel::CT_PASSENGERS => TownProductionEffect::TPE_PASSENGERS,
                    CargoLabel::CT_MAIL => TownProductionEffect::TPE_MAIL,
                    _ => TownProductionEffect::TPE_NONE,
                }
                .0,
            );
        }
        if !is_cargo_spec_valid(info.bitnum) {
            (s.invalidate_strings)(cs);
        }
    }
}
/// `GetCargoTranslationTable`: get the cargo translation table to use for the given GRF file.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_translation_table(file: Handle) -> Labels {
    let grffile = (services().file)(file);
    // Always use the translation table if it's installed.
    if grffile.labels.size != 0 {
        grffile.labels
    } else if grffile.version < 7 {
        // Pre-v7 use climate-dependent "slot" table.
        openttd_rust_cargo_climate_labels()
    } else {
        // Otherwise use climate-independent "bitnum" table.
        openttd_rust_cargo_bitnum_labels()
    }
}
/// `GetCargoTranslation`: translate a GRF-local cargo slot/bitnum into a `CargoType`.
/// `usebit` defines whether pre-v7 cargo is a bitnum (true) or cargo slot (false).
/// For GRF version >= 7 cargo is always a translated cargo bit.
/// Returns `CargoType` or `INVALID_CARGO` if the cargo is not available.
pub(crate) fn get_cargo_translation(cargo: u8, file: Handle, usebit: bool) -> CargoType {
    let grffile = (services().file)(file);
    // We can't use GetCargoTranslationTable here as the usebit flag changes behaviour.
    // Pre-version 7 uses the bitnum lookup from (standard in v8) instead of climate dependent in some places..
    let cargo_list = if grffile.version < 7 && !usebit && !grffile.fallback {
        openttd_rust_cargo_climate_labels()
    } else if grffile.labels.size != 0 {
        grffile.labels
    } else {
        openttd_rust_cargo_bitnum_labels()
    };
    if usize::from(cargo) < cargo_list.size {
        let label = CargoLabel(unsafe { cargo_list.data.add(usize::from(cargo)).read() });
        get_cargo_type_by_label(label)
    } else {
        CargoType::INVALID_CARGO
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_translate(cargo: u8, file: Handle, usebit: bool) -> u8 {
    get_cargo_translation(cargo, file, usebit).0
}
/// `BuildCargoTranslationMap`: construct the cargo mapping, the reverse of a cargo translation table.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_build_inverse(file: *mut c_void) {
    let s = services();
    (s.clear_inverse)(file);
    let cargo_list = openttd_rust_cargo_translation_table(file);
    for (_, (cs, info)) in crate::cargotype::iterate(
        |cargo| {
            let cs = (s.spec)(cargo);
            (cs, (s.label_spec)(cs))
        },
        |(_, info)| is_cargo_spec_valid(info.bitnum),
    ) {
        // Check the translation table for this cargo's label.
        let label = CargoLabel(info.label);
        let idx = unsafe {
            crate::container_func::find_index(
                std::slice::from_raw_parts(cargo_list.data, cargo_list.size),
                &label.0,
            )
        };
        if idx >= 0 {
            let cargo = CargoType((s.index)(cs));
            (s.set_inverse)(file, cargo.0, idx as u8);
        }
    }
}
/// `MaybeInstallFallbackCargoLabel`: install a cargo label in a fallback cargo list,
/// if a NewGRF-defined cargo list is not present. This allows this `NewGRF` to use its
/// self-defined cargo types without needing an explicit translation table.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_fallback(file: *mut c_void, id: u32, last: u32, label: u32) {
    let label = CargoLabel(label);
    let s = services();
    if (s.file)(file).labels.size == 0 {
        // Cargo translation table isn't configured yet, assume it won't be and configure the cargo list as a fallback list.
        let default_cargo_list = openttd_rust_cargo_translation_table(file);
        (s.assign_fallback)(file, default_cargo_list);
    }
    let grffile = (s.file)(file);
    if grffile.fallback {
        // Automatically fill fallback cargo list with the defined label, resizing as needed.
        if grffile.labels.size < last as usize {
            (s.resize_fallback)(file, last as usize);
        }
        (s.set_fallback)(file, id, label.0);
    }
}
/// Constructor of the cargo resolver (`CargoResolverObject`).
/// The native frame uses this stack invocation for the synchronous resolution.
fn invocation(spec: Handle, callback: u16, param1: u32, param2: u32) -> Invocation {
    let callback = CallbackID(callback);
    let mut invocation = Invocation {
        label: 0,
        file: ptr::null(),
        root: ptr::null(),
        callback: callback.0,
        param1,
        param2,
    };
    (services().invocation)(spec, &raw mut invocation);
    invocation
}
/// `GetCargoCallback`, preserving the supplied register span through the native frame.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_callback(
    callback: u16,
    param1: u32,
    param2: u32,
    spec: Handle,
    regs: *mut i32,
    count: usize,
) -> u16 {
    let invocation = invocation(spec, callback, param1, param2);
    (services().callback_frame)(&raw const invocation, regs, count)
}
/// `GetCargoCallback` for runtime callers with an empty register span.
pub unsafe fn get_cargo_callback(
    s: &Services,
    cargo: u8,
    callback: u16,
    param1: u32,
    param2: u32,
) -> u16 {
    let cargo = CargoType(cargo);
    let callback = CallbackID(callback);
    let mut invocation = Invocation {
        label: 0,
        file: ptr::null(),
        root: ptr::null(),
        callback: callback.0,
        param1,
        param2,
    };
    (s.invocation)((s.spec)(cargo.0), &raw mut invocation);
    (s.callback_frame)(&raw const invocation, ptr::null_mut(), 0)
}
/// Get the custom sprite for the given cargo type, or zero if not available.
/// `GetCustomCargoSprite` constructs its cargo resolver with no callback.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_custom_sprite(spec: Handle) -> u32 {
    let invocation = invocation(spec, CBID_NO_CALLBACK.0, 0, 0);
    let sprite: SpriteID = (services().sprite_frame)(&raw const invocation);
    sprite
}
/// `CargoResolverObject::ResolveCallback` within the live native callback frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_evaluate_callback(
    invocation: *const Invocation,
    resolver: *mut c_void,
    state: *mut State,
    regs: *mut i32,
    count: usize,
) -> u16 {
    let s = services();
    let (root, file) = unsafe { ((*invocation).root, (*invocation).file) };
    unsafe {
        newgrf_spritegroup::openttd_rust_resolver_callback(
            &*s.common, root, resolver, state, file, regs, count,
        )
    }
}
/// `CargoResolverObject::Resolve<ResultSpriteGroup>` within the live native sprite frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_evaluate_sprite(
    invocation: *const Invocation,
    resolver: *mut c_void,
    state: *mut State,
) -> u32 {
    let s = services();
    let (root, file) = unsafe { ((*invocation).root, (*invocation).file) };
    let result = unsafe {
        newgrf_spritegroup::openttd_rust_resolver_resolve(&*s.common, root, resolver, state, file)
    };
    // ResolverResult alternative 2 is const ResultSpriteGroup*.
    if result.kind != 2 || result.group.is_null() {
        return 0;
    }
    let sprite = (s.sprite)(result.group);
    if sprite.count == 0 {
        0
    } else {
        let sprite: SpriteID = sprite.sprite;
        sprite
    }
}
/// `CargoResolverObject::GetFeature`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_feature() -> u8 {
    GrfSpecFeature::GSF_CARGOES.0
}
/// `CargoResolverObject::GetDebugID`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_debug_id(invocation: *const Invocation) -> u32 {
    unsafe { CargoLabel((*invocation).label).0 }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCargoProduction" ["probe"] => ProductionCargo {cargo,multiplier};
    "OpenTTDCargoSprite" ["probe"] => Sprite {sprite,count};
    "OpenTTDCargoLabelSpec" ["probe"] => LabelSpec {label,bitnum};
    "OpenTTDCargoFinaliseSpec" ["probe"] => FinaliseSpec {label,bitnum,effect};
    "OpenTTDCargoOrderSpec" ["probe"] => OrderSpec {cargo,effect,special};
    "OpenTTDCargoLabels" ["probe"] => Labels {data,size};
    "OpenTTDCargoFile" ["probe"] => File {labels,version,fallback};
    "OpenTTDCargoInvocation" ["probe"] => Invocation {label,file,root,callback,param1,param2};
    "OpenTTDCargoRegistryServices" ["probe"] => Services {shared,common,spec,copy_default,clear_specs,label_spec,bitnum,town_acceptance_effect,finalise_spec,order_spec,index,production,sort,publish_sorted,publish_standard,publish_town,set_file,set_effect,invalidate_strings,file,assign_fallback,resize_fallback,set_fallback,clear_inverse,set_inverse,invocation,callback_frame,sprite_frame,sprite};
}

pub(crate) fn file_cargo_count(file: Handle) -> usize {
    (services().file)(file).labels.size
}

/// Determines the first active cargo slot with a certain town effect.
/// `FindFirstCargoWithTownAcceptanceEffect`, in active cargo-slot order.
pub(crate) fn first_town_acceptance(effect: u8) -> Option<u8> {
    let effect = TownAcceptanceEffect(effect);
    let s = services();
    crate::cargotype::iterate(
        |cargo| (s.spec)(cargo),
        |spec| is_cargo_spec_valid((s.bitnum)(*spec)),
    )
    .find_map(|(cargo, _spec)| {
        (TownAcceptanceEffect((s.town_acceptance_effect)(cargo)) == effect).then_some(cargo)
    })
}

/// Scalar adapter for `CargoSpec::town_acceptance_effect`.
pub(crate) fn town_acceptance(cargo: u8) -> u8 {
    let cargo = CargoType(cargo);
    TownAcceptanceEffect((services().town_acceptance_effect)(cargo.0)).0
}

/// Test-only: the current file reports `labels` as its cargo list, and each maps to its cargo.
#[cfg(test)]
pub(crate) fn install_test_labels(labels: &'static [u32], cargos: &[u8]) {
    static TABLE: GameState<Services> = GameState::new(TEST_TABLE);
    static LABELS: GameState<&'static [u32]> = GameState::new(&[]);
    extern "C" fn file(_: Handle) -> File {
        // SAFETY: Serialized by the caller's test lock; no borrow outlives this read.
        let labels = unsafe { *LABELS.get() };
        File {
            labels: Labels {
                data: labels.as_ptr(),
                size: labels.len(),
            },
            version: 8,
            fallback: false,
        }
    }
    // SAFETY: Serialized by the caller's test lock.
    unsafe {
        LABELS.get().write(labels);
        (*TABLE.get()).file = file;
        let registry = &mut *REGISTRY.get();
        registry.services = TABLE.get();
        registry.label_map.clear();
        for (label, cargo) in labels.iter().zip(cargos) {
            registry
                .label_map
                .insert(CargoLabel(*label), CargoType(*cargo));
        }
    }
}
/// Test-only: forget [`install_test_labels`].
#[cfg(test)]
pub(crate) fn clear_test_labels() {
    // SAFETY: Serialized by the caller's test lock.
    unsafe {
        let registry = &mut *REGISTRY.get();
        registry.services = std::ptr::null();
        registry.label_map.clear();
    }
}
#[cfg(test)]
mod evidence_tests;
