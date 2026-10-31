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
use crate::cargo_type::{
    CT_INVALID, CT_MAIL, CT_PASSENGERS, INVALID_CARGO, NUM_CARGO, NUM_ORIGINAL_CARGO,
};
use crate::cargotype::{
    INVALID_TPE, NUM_TPE, TPE_MAIL, TPE_NONE, TPE_PASSENGERS, is_cargo_spec_valid,
};
use crate::game_state::GameState;
use crate::newgrf_callbacks::CBID_NO_CALLBACK;
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
    param2: u32,
}
#[repr(C)]
pub struct Services {
    shared: *const crate::services::Services,
    common: *const newgrf_spritegroup::Services,
    spec: extern "C" fn(u8) -> Handle,
    copy_default: extern "C" fn(u8, u8),
    clear_specs: extern "C" fn(u8),
    label_spec: extern "C" fn(Handle) -> LabelSpec,
    bitnum: extern "C" fn(Handle) -> u8,
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
    invocation: extern "C" fn(Handle, *mut Invocation),
    callback_frame: extern "C" fn(*const Invocation, *mut i32, usize) -> u16,
    sprite_frame: extern "C" fn(*const Invocation) -> u32,
    sprite: extern "C" fn(Handle) -> Sprite,
}
struct Registry {
    services: *const Services,
    mask: u64,
    standard: u64,
    defaults: Vec<u32>,
    climate: [u32; NUM_ORIGINAL_CARGO as usize],
    bitnum: [u32; 32],
    labels: BTreeMap<u32, u8>,
    sorted: Vec<Handle>,
    reverse: [u8; NUM_CARGO as usize],
    town: [Vec<Handle>; NUM_TPE as usize],
}
static REGISTRY: GameState<Registry> = GameState::new(Registry {
    services: ptr::null(),
    mask: 0,
    standard: 0,
    defaults: Vec::new(),
    climate: [0; NUM_ORIGINAL_CARGO as usize],
    bitnum: [0; 32],
    labels: BTreeMap::new(),
    sorted: Vec::new(),
    reverse: [0; NUM_CARGO as usize],
    town: [const { Vec::new() }; NUM_TPE as usize],
});
fn services() -> &'static Services {
    unsafe { &*(*REGISTRY.get()).services }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).mask }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_standard_cargo_mask() -> *mut u64 {
    unsafe { &raw mut (*REGISTRY.get()).standard }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_reverse() -> *mut u8 {
    unsafe { (&raw mut (*REGISTRY.get()).reverse).cast() }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_setup(s: *const Services, climate: u8) {
    // SAFETY: serialized native loading excludes all runtime registry views.
    unsafe {
        (*REGISTRY.get()).services = s;
        crate::movement::native_assert(
            &*(*s).shared,
            climate < crate::landscape_type::NUM_LANDSCAPE,
        );
        (*REGISTRY.get()).mask = 0;
        (*REGISTRY.get()).defaults.clear();
        (*REGISTRY.get()).climate.fill(CT_INVALID);
        (*REGISTRY.get()).bitnum.fill(CT_INVALID);
    }
    let s = unsafe { &*s };
    for slot in 0..NUM_ORIGINAL_CARGO {
        (s.copy_default)(climate, slot);
        let cs = (s.spec)(slot);
        let spec = (s.label_spec)(cs);
        if is_cargo_spec_valid(spec.bitnum) {
            let cargo = (s.index)(cs);
            unsafe {
                set_bit64(&mut (*REGISTRY.get()).mask, cargo);
                (*REGISTRY.get()).defaults.push(spec.label);
                (*REGISTRY.get()).climate[usize::from(cargo)] = spec.label;
                (*REGISTRY.get()).bitnum[usize::from(spec.bitnum)] = spec.label;
            }
        }
    }
    (s.clear_specs)(NUM_ORIGINAL_CARGO);
    openttd_rust_cargo_labels_rebuild();
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_labels_rebuild() {
    unsafe {
        (*REGISTRY.get()).labels.clear();
    }
    let s = services();
    for cargo in 0..NUM_CARGO {
        let cs = (s.spec)(cargo);
        let spec = (s.label_spec)(cs);
        if is_cargo_spec_valid(spec.bitnum) && spec.label != 0 && spec.label != CT_INVALID {
            let index = (s.index)(cs);
            unsafe {
                (*REGISTRY.get()).labels.entry(spec.label).or_insert(index);
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
            set_bit64(&mut (*REGISTRY.get()).mask, cargo);
        }
    } else {
        unsafe {
            clr_bit64(&mut (*REGISTRY.get()).mask, cargo);
        }
    }
    openttd_rust_cargo_labels_rebuild();
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_by_label(label: u32) -> u8 {
    unsafe {
        (*REGISTRY.get())
            .labels
            .get(&label)
            .copied()
            .unwrap_or(INVALID_CARGO)
    }
}
/// Canonical `CargoSpec::label`, including duplicate labels in distinct slots.
pub(crate) fn cargo_label(cargo: u8) -> u32 {
    let s = services();
    (s.label_spec)((s.spec)(cargo)).label
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_is_default(cargo: u8) -> bool {
    let s = services();
    let spec = (s.label_spec)((s.spec)(cargo));
    is_cargo_spec_valid(spec.bitnum) && unsafe { (*REGISTRY.get()).defaults.contains(&spec.label) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_climate_labels() -> Labels {
    Labels {
        data: unsafe { (&raw const (*REGISTRY.get()).climate).cast() },
        size: usize::from(NUM_ORIGINAL_CARGO),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_bitnum_labels() -> Labels {
    Labels {
        data: unsafe { (&raw const (*REGISTRY.get()).bitnum).cast() },
        size: 32,
    }
}

pub fn town_count(effect: u8) -> usize {
    unsafe { (*REGISTRY.get()).town[usize::from(effect)].len() }
}
pub fn town_cargo(effect: u8, index: usize) -> u8 {
    let spec = unsafe {
        let state = &*REGISTRY.get();
        *state.town[usize::from(effect)].get_unchecked(index)
    };
    (services().index)(spec)
}
pub fn production(effect: u8, index: u32) -> ProductionCargo {
    let spec = unsafe {
        let state = &*REGISTRY.get();
        state.town[usize::from(effect)].get(index as usize).copied()
    };
    spec.map_or(
        ProductionCargo {
            cargo: INVALID_CARGO,
            multiplier: 0,
        },
        |spec| (services().production)(spec),
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_sort(native: *const Services) {
    // Language loading sorts the initially empty specification array before climate setup.
    unsafe {
        (*REGISTRY.get()).services = native;
    }
    let s = services();
    for effect in 0..NUM_TPE {
        let data = unsafe {
            let town = &mut (*REGISTRY.get()).town[usize::from(effect)];
            town.clear();
            town.as_mut_ptr()
        };
        (s.publish_town)(effect, data, 0);
    }
    let data = unsafe {
        (*REGISTRY.get()).sorted.clear();
        (*REGISTRY.get()).sorted.as_mut_ptr()
    };
    (s.publish_sorted)(data, 0);
    for (_, spec) in crate::cargotype::iterate(
        |cargo| (s.spec)(cargo),
        |spec| is_cargo_spec_valid((s.bitnum)(*spec)),
    ) {
        unsafe {
            (*REGISTRY.get()).sorted.push(spec);
        }
    }
    let (data, count) = unsafe {
        (
            (*REGISTRY.get()).sorted.as_mut_ptr(),
            (*REGISTRY.get()).sorted.len(),
        )
    };
    (s.publish_sorted)(data, count);
    (s.sort)(data, count);
    for index in 0..count {
        let cargo = (s.index)(unsafe { data.add(index).read() });
        unsafe {
            (*REGISTRY.get()).reverse[usize::from(cargo)] = index as u8;
        }
    }
    unsafe {
        (*REGISTRY.get()).standard = 0;
    }
    let mut standard = 0;
    for index in 0..count {
        let spec = unsafe { data.add(index).read() };
        let order = (s.order_spec)(spec);
        crate::movement::native_assert(unsafe { &*s.shared }, order.effect != INVALID_TPE);
        let (town_data, town_count) = unsafe {
            let town = &mut (*REGISTRY.get()).town[usize::from(order.effect)];
            town.push(spec);
            (town.as_mut_ptr(), town.len())
        };
        (s.publish_town)(order.effect, town_data, town_count);
        if order.special {
            break;
        }
        standard += 1;
        unsafe {
            set_bit64(&mut (*REGISTRY.get()).standard, order.cargo);
        }
    }
    (s.publish_standard)(data, standard);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_finalise() {
    let s = services();
    for cargo in 0..NUM_CARGO {
        let cs = (s.spec)(cargo);
        let info = (s.finalise_spec)(cs);
        if info.effect == INVALID_TPE {
            let label = info.label;
            (s.set_effect)(
                cs,
                match label {
                    CT_PASSENGERS => TPE_PASSENGERS,
                    CT_MAIL => TPE_MAIL,
                    _ => TPE_NONE,
                },
            );
        }
        if !is_cargo_spec_valid(info.bitnum) {
            (s.invalidate_strings)(cs);
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_translation_table(file: Handle) -> Labels {
    let info = (services().file)(file);
    if info.labels.size != 0 {
        info.labels
    } else if info.version < 7 {
        openttd_rust_cargo_climate_labels()
    } else {
        openttd_rust_cargo_bitnum_labels()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_translate(cargo: u8, file: Handle, usebit: bool) -> u8 {
    let info = (services().file)(file);
    let labels = if info.version < 7 && !usebit && !info.fallback {
        openttd_rust_cargo_climate_labels()
    } else if info.labels.size != 0 {
        info.labels
    } else {
        openttd_rust_cargo_bitnum_labels()
    };
    if usize::from(cargo) < labels.size {
        openttd_rust_cargo_by_label(unsafe { labels.data.add(usize::from(cargo)).read() })
    } else {
        INVALID_CARGO
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_build_inverse(file: *mut c_void) {
    let s = services();
    (s.clear_inverse)(file);
    let labels = openttd_rust_cargo_translation_table(file);
    for (_, (cs, info)) in crate::cargotype::iterate(
        |cargo| {
            let cs = (s.spec)(cargo);
            (cs, (s.label_spec)(cs))
        },
        |(_, info)| is_cargo_spec_valid(info.bitnum),
    ) {
        let index = unsafe {
            crate::container_func::find_index(
                std::slice::from_raw_parts(labels.data, labels.size),
                &info.label,
            )
        };
        if index >= 0 {
            (s.set_inverse)(file, (s.index)(cs), index as u8);
        }
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_fallback(file: *mut c_void, id: u32, last: u32, label: u32) {
    let s = services();
    if (s.file)(file).labels.size == 0 {
        let labels = openttd_rust_cargo_translation_table(file);
        (s.assign_fallback)(file, labels);
    }
    let info = (s.file)(file);
    if info.fallback {
        if info.labels.size < last as usize {
            (s.resize_fallback)(file, last as usize);
        }
        (s.set_fallback)(file, id, label);
    }
}
fn invocation(spec: Handle, callback: u16, param1: u32, param2: u32) -> Invocation {
    let mut invocation = Invocation {
        label: 0,
        file: ptr::null(),
        root: ptr::null(),
        callback,
        param1,
        param2,
    };
    (services().invocation)(spec, &raw mut invocation);
    invocation
}
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
pub unsafe fn callback(s: &Services, cargo: u8, callback: u16, param1: u32, param2: u32) -> u16 {
    let mut invocation = Invocation {
        label: 0,
        file: ptr::null(),
        root: ptr::null(),
        callback,
        param1,
        param2,
    };
    (s.invocation)((s.spec)(cargo), &raw mut invocation);
    (s.callback_frame)(&raw const invocation, ptr::null_mut(), 0)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_custom_sprite(spec: Handle) -> u32 {
    let invocation = invocation(spec, CBID_NO_CALLBACK.0, 0, 0);
    (services().sprite_frame)(&raw const invocation)
}
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
    if result.kind != 2 || result.group.is_null() {
        return 0;
    }
    let sprite = (s.sprite)(result.group);
    if sprite.count == 0 { 0 } else { sprite.sprite }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_cargo_feature() -> u8 {
    crate::newgrf::GSF_CARGOES
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_cargo_debug_id(invocation: *const Invocation) -> u32 {
    unsafe { (*invocation).label }
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
    "OpenTTDCargoRegistryServices" ["probe"] => Services {shared,common,spec,copy_default,clear_specs,label_spec,bitnum,finalise_spec,order_spec,index,production,sort,publish_sorted,publish_standard,publish_town,set_file,set_effect,invalidate_strings,file,assign_fallback,resize_fallback,set_fallback,clear_inverse,set_inverse,invocation,callback_frame,sprite_frame,sprite};
}

pub(crate) fn file_cargo_count(file: Handle) -> usize {
    (services().file)(file).labels.size
}
