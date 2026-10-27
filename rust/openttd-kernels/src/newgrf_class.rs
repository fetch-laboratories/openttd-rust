/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical shared `NewGRF` class registry. Native presentation spans alias the Rust
//! buffers until allocation/reset or insertion, just as the original native vectors.
//! The game-state mutex serializes access; native spec shells outlive membership.
//! No registry borrow survives an eligibility callback or a native field service.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::game_state::GameState;
use std::ptr;

#[repr(C)]
pub struct Class {
    pub global_id: u32,
    pub name: u32,
    pub rust_index: u32,
}
#[repr(C)]
pub struct Identity {
    pub grfid: u32,
    pub local_id: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DefaultClass {
    label: u32,
    name: u32,
}
#[repr(C)]
pub struct Services<T> {
    pub class_index: unsafe extern "C" fn(*const T) -> u16,
    pub set_index: unsafe extern "C" fn(*mut T, u16),
    pub identity: unsafe extern "C" fn(*const T) -> Identity,
    pub exhausted: unsafe extern "C" fn(u16),
    pub object: *const crate::objects::ObjectServices,
    defaults: *const DefaultClass,
    default_count: usize,
    maximum: u16,
    empty: u32,
    insert_empty: bool,
}
struct Members<T> {
    specs: Vec<*mut T>,
    ui_count: u32,
}
struct Registry<T> {
    classes: Vec<Class>,
    members: Vec<Members<T>>,
}
impl<T> Registry<T> {
    const fn new() -> Self {
        Self {
            classes: Vec::new(),
            members: Vec::new(),
        }
    }
}
unsafe fn allocate<T>(
    state: *mut Registry<T>,
    global_id: u32,
    max: u16,
    empty: u32,
    s: &Services<T>,
) -> u16 {
    if let Some(i) = unsafe {
        (*state)
            .classes
            .iter()
            .position(|c| c.global_id == global_id)
    } {
        return i as u16;
    }
    let index = unsafe { (*state).classes.len() };
    if index < usize::from(max) {
        unsafe {
            (*state).classes.push(Class {
                global_id,
                name: empty,
                rust_index: index as u32,
            });
            (*state).members.push(Members {
                specs: Vec::new(),
                ui_count: 0,
            });
        }
        return index as u16;
    }
    unsafe {
        (s.exhausted)(max);
    }
    0
}
unsafe fn get_spec<T>(state: *mut Registry<T>, class: u16, index: u32) -> *mut T {
    unsafe {
        (&(*state).members)[usize::from(class)]
            .specs
            .get(index as usize)
            .copied()
            .unwrap_or(ptr::null_mut())
    }
}
unsafe fn available<T>(state: *mut Registry<T>, class: u16, index: u32, s: &Services<T>) -> bool {
    if s.object.is_null() {
        return true;
    }
    let spec = unsafe { get_spec(state, class, index) };
    unsafe { crate::objects::openttd_rust_object_ever_available(s.object, spec.cast()) }
}
unsafe fn insert<T>(state: *mut Registry<T>, class: u16, spec: *mut T, s: &Services<T>) {
    let index = unsafe {
        let members = &mut (&mut (*state).members)[usize::from(class)];
        let index = members.specs.len() as u16;
        members.specs.push(spec);
        index
    };
    if !spec.is_null() {
        unsafe {
            (s.set_index)(spec, index);
        }
    }
    // NewGRFClass::Insert checks availability at the narrowed index, even after wrap.
    if unsafe { available(state, class, u32::from(index), s) } {
        unsafe {
            (&mut (*state).members)[usize::from(class)].ui_count = (&(*state).members)
                [usize::from(class)]
            .ui_count
            .wrapping_add(1);
        }
    }
}
unsafe fn by_grf<T>(
    state: *mut Registry<T>,
    grfid: u32,
    local_id: u16,
    s: &Services<T>,
) -> *const T {
    let classes = unsafe { (*state).members.len() };
    for class in 0..classes {
        let count = unsafe { (&(*state).members)[class].specs.len() };
        for index in 0..count {
            let spec = unsafe { (&(*state).members)[class].specs[index] };
            if spec.is_null() {
                continue;
            }
            let identity = unsafe { (s.identity)(spec) };
            if identity.local_id == local_id && identity.grfid == grfid {
                return spec;
            }
        }
    }
    ptr::null()
}
macro_rules! registry {
    ($module:ident, $spec:ty, $prefix:literal, [$($assign_export:meta),*] $(, $set_name:ident)?) => {
        pub mod $module {
            use super::*;
            type Spec = $spec;
            static STATE: GameState<Registry<Spec>> = GameState::new(Registry::new());
            static SERVICES: GameState<*const Services<Spec>> = GameState::new(ptr::null());
            #[unsafe(export_name = concat!($prefix, "_reset"))]
            pub unsafe extern "C" fn reset(s: *const Services<Spec>) {
                SERVICES.exchange(s);
                unsafe {
                    *STATE.get() = Registry::new();
                    let s = &*s;
                    for i in 0..s.default_count {
                        let default = s.defaults.add(i).read();
                        let index = allocate(default.label, s.maximum, s.empty);
                        (*classes().add(usize::from(index))).name = default.name;
                        if s.insert_empty {
                            let index = allocate(default.label, s.maximum, s.empty);
                            insert(index, ptr::null_mut());
                        }
                    }
                }
            }
            #[unsafe(export_name = concat!($prefix, "_allocate"))]
            pub unsafe extern "C" fn allocate(id: u32, max: u16, empty: u32) -> u16 {
                unsafe { super::allocate(STATE.get(), id, max, empty, SERVICES.table()) }
            }
            pub unsafe extern "C" fn insert(class: u16, spec: *mut Spec) {
                unsafe {
                    super::insert(STATE.get(), class, spec, SERVICES.table());
                }
            }
            $(
                #[unsafe(no_mangle)]
                pub unsafe extern "C" fn $set_name(spec: *const Spec, name: u32) {
                    let class = unsafe { (SERVICES.table().class_index)(spec) };
                    unsafe { (*classes().add(usize::from(class))).name = name; }
                }
            )?
            $(#[$assign_export])*
            pub unsafe extern "C" fn assign(spec: *mut Spec) {
                let s = unsafe { SERVICES.table() };
                let class = unsafe { (s.class_index)(spec) };
                unsafe {
                    insert(class, spec);
                }
            }
            #[unsafe(export_name = concat!($prefix, "_classes"))]
            pub extern "C" fn classes() -> *mut Class {
                unsafe { (*STATE.get()).classes.as_mut_ptr() }
            }
            #[unsafe(export_name = concat!($prefix, "_count"))]
            pub extern "C" fn count() -> u32 {
                unsafe { (*STATE.get()).classes.len() as u32 }
            }
            #[unsafe(export_name = concat!($prefix, "_ui_count"))]
            pub extern "C" fn ui_count() -> u32 {
                unsafe {
                    (*STATE.get())
                        .members
                        .iter()
                        .filter(|m| m.ui_count > 0)
                        .count() as u32
                }
            }
            #[unsafe(export_name = concat!($prefix, "_specs"))]
            pub unsafe extern "C" fn specs(class: u16) -> *const *mut Spec {
                unsafe { (&(*STATE.get()).members)[usize::from(class)].specs.as_ptr() }
            }
            #[unsafe(export_name = concat!($prefix, "_spec_count"))]
            pub extern "C" fn spec_count(class: u16) -> u32 {
                unsafe { (&(*STATE.get()).members)[usize::from(class)].specs.len() as u32 }
            }
            #[unsafe(export_name = concat!($prefix, "_ui_spec_count"))]
            pub unsafe extern "C" fn ui_spec_count(class: u16) -> u32 {
                unsafe { (&(*STATE.get()).members)[usize::from(class)].ui_count }
            }
            #[unsafe(export_name = concat!($prefix, "_spec"))]
            pub extern "C" fn spec(class: u16, index: u32) -> *mut Spec {
                unsafe { get_spec(STATE.get(), class, index) }
            }
            #[unsafe(export_name = concat!($prefix, "_by_grf"))]
            pub unsafe extern "C" fn by_grf(grfid: u32, local_id: u16) -> *const Spec {
                unsafe { super::by_grf(STATE.get(), grfid, local_id, SERVICES.table()) }
            }
        }
    };
}
registry!(
    station,
    crate::station_cmd::RailSpec,
    "openttd_rust_station_class",
    [unsafe(export_name = "openttd_rust_station_class_assign")],
    openttd_rust_station_class_set_name
);
registry!(
    roadstop,
    crate::station_cmd::RoadSpec,
    "openttd_rust_roadstop_class",
    [unsafe(export_name = "openttd_rust_roadstop_class_assign")],
    openttd_rust_roadstop_class_set_name
);
registry!(
    airport,
    crate::station_cmd::AirportSpec,
    "openttd_rust_airport_class",
    []
);
registry!(
    object,
    crate::objects::Spec,
    "openttd_rust_object_class",
    [unsafe(export_name = "openttd_rust_object_class_assign")]
);

crate::abi::layouts! {
    fn abi_layout;
    "StationClass" ["station_service"] => Class { global_id, name, rust_index };
    "RoadStopClass" ["station_service"] => Class { global_id, name, rust_index };
    "AirportClass" ["station_service"] => Class { global_id, name, rust_index };
    "ObjectClass" ["station_service"] => Class { global_id, name, rust_index };
    "OpenTTDGRFClassDefault" ["station_service"] => DefaultClass { label, name };
    "OpenTTDGRFClassIdentity" ["station_service"] => Identity { grfid, local_id };
    "OpenTTDStationClassServices" ["station_service"] => Services<crate::station_cmd::RailSpec> { class_index, set_index, identity, exhausted, object, defaults, default_count, maximum, empty, insert_empty };
    "OpenTTDRoadStopClassServices" ["station_service"] => Services<crate::station_cmd::RoadSpec> { class_index, set_index, identity, exhausted, object, defaults, default_count, maximum, empty, insert_empty };
    "OpenTTDAirportClassServices" ["station_service"] => Services<crate::station_cmd::AirportSpec> { class_index, set_index, identity, exhausted, object, defaults, default_count, maximum, empty, insert_empty };
    "OpenTTDObjectClassServices" ["station_service"] => Services<crate::objects::Spec> { class_index, set_index, identity, exhausted, object, defaults, default_count, maximum, empty, insert_empty };
}

pub(crate) fn station_label(class: u16) -> u32 {
    unsafe { (*station::classes().add(usize::from(class))).global_id }
}
pub(crate) fn roadstop_label(class: u16) -> u32 {
    unsafe { (*roadstop::classes().add(usize::from(class))).global_id }
}
