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
use crate::debug::grf_msg;
use crate::game_state::GameState;
use crate::table::strings::STR_EMPTY;
use std::ptr;

#[repr(C)]
pub struct Class {
    pub global_id: u32,
    pub name: u32,
    pub rust_index: u32,
}
/// A registry's `Tindex`: the class-ID wrapper its `NewGRFClass` instantiation indexes by.
/// The registry's adapters and exports pass indices as `u16` (the shared record keeps
/// `rust_index` as `u32`); `from_index` narrows as the original's `static_cast<Tindex>` does.
pub trait ClassIndex: Copy {
    fn from_index(index: u16) -> Self;
    fn index(self) -> u16;
}
impl ClassIndex for crate::newgrf_station::StationClassID {
    fn from_index(index: u16) -> Self {
        Self(index)
    }
    fn index(self) -> u16 {
        self.0
    }
}
impl ClassIndex for crate::newgrf_roadstop::RoadStopClassID {
    fn from_index(index: u16) -> Self {
        Self(index)
    }
    fn index(self) -> u16 {
        self.0
    }
}
impl ClassIndex for crate::newgrf_airport::AirportClassID {
    fn from_index(index: u16) -> Self {
        Self(index as u8)
    }
    fn index(self) -> u16 {
        u16::from(self.0)
    }
}
impl ClassIndex for crate::newgrf_object::ObjectClassID {
    fn from_index(index: u16) -> Self {
        Self(index)
    }
    fn index(self) -> u16 {
        self.0
    }
}
#[repr(C)]
pub struct Identity {
    pub grfid: u32,
    pub local_id: u16,
}
#[repr(C)]
pub struct Services<T> {
    pub class_index: unsafe extern "C" fn(*const T) -> u16,
    pub set_index: unsafe extern "C" fn(*mut T, u16),
    pub identity: unsafe extern "C" fn(*const T) -> Identity,
    pub object: *const crate::objects::ObjectServices,
    shared: *const crate::services::Services,
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
/// `NewGRFClass::Index`, a field-sized load of the class's own index.
pub(crate) unsafe fn index<I: ClassIndex>(class: *const Class) -> I {
    I::from_index(unsafe { (*class).rust_index } as u16)
}
/// `NewGRFClass::Allocate`: allocate a class with a given global class ID.
unsafe fn allocate<T>(state: *mut Registry<T>, global_id: u32, max: u16) -> u16 {
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
                name: STR_EMPTY.0,
                rust_index: index as u32,
            });
            (*state).members.push(Members {
                specs: Vec::new(),
                ui_count: 0,
            });
        }
        return index as u16;
    }
    grf_msg!(
        2,
        "ClassAllocate: already allocated {} classes, using default",
        max
    );
    0
}
/// `NewGRFClass::Get`: the class at `class_index`, after the original's bound assertion.
unsafe fn get<T>(state: *mut Registry<T>, class_index: u16, s: &Services<T>) -> *mut Class {
    let count = unsafe { (*state).classes.len() };
    (crate::movement::table(unsafe { &*s.shared }).native_assert)(usize::from(class_index) < count);
    // An assertion-free build indexes past the end, as the original's `classes[class_index]`.
    unsafe {
        (*state)
            .classes
            .as_mut_ptr()
            .wrapping_add(usize::from(class_index))
    }
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
    unsafe { crate::objects::openttd_rust_object_ever_available_abi(s.object, spec.cast()) != 0 }
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
    ($module:ident, $spec:ty, $index:ty, $prefix:literal, $insert_defaults:path, [$($assign_export:meta),*] $(, $set_name:ident)?) => {
        pub mod $module {
            use super::*;
            type Spec = $spec;
            static STATE: GameState<Registry<Spec>> = GameState::new(Registry::new());
            pub(super) static SERVICES: GameState<*const Services<Spec>> = GameState::new(ptr::null());
            #[unsafe(export_name = concat!($prefix, "_reset"))]
            pub unsafe extern "C" fn reset(s: *const Services<Spec>) {
                SERVICES.exchange(s);
                unsafe {
                    *STATE.get() = Registry::new();
                    $insert_defaults();
                }
            }
            /// `NewGRFClass::Allocate`.
            pub unsafe fn allocate(id: u32, max: $index) -> $index {
                <$index>::from_index(unsafe { super::allocate(STATE.get(), id, max.index()) })
            }
            #[unsafe(export_name = concat!($prefix, "_allocate"))]
            pub unsafe extern "C" fn allocate_export(id: u32, max: u16) -> u16 {
                unsafe { allocate(id, <$index>::from_index(max)) }.index()
            }
            /// `NewGRFClass::Get`.
            pub unsafe fn get(class_index: $index) -> *mut Class {
                unsafe { super::get(STATE.get(), class_index.index(), SERVICES.table()) }
            }
            /// `NewGRFClass::Insert`.
            pub unsafe fn insert(class: $index, spec: *mut Spec) {
                unsafe {
                    super::insert(STATE.get(), class.index(), spec, SERVICES.table());
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
                let class = <$index>::from_index(unsafe { (s.class_index)(spec) });
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
    crate::newgrf_station::StationClassID,
    "openttd_rust_station_class",
    crate::newgrf_station::insert_defaults,
    [unsafe(export_name = "openttd_rust_station_class_assign")],
    openttd_rust_station_class_set_name
);
registry!(
    roadstop,
    crate::station_cmd::RoadSpec,
    crate::newgrf_roadstop::RoadStopClassID,
    "openttd_rust_roadstop_class",
    crate::newgrf_roadstop::insert_defaults,
    [unsafe(export_name = "openttd_rust_roadstop_class_assign")],
    openttd_rust_roadstop_class_set_name
);
registry!(
    airport,
    crate::station_cmd::AirportSpec,
    crate::newgrf_airport::AirportClassID,
    "openttd_rust_airport_class",
    crate::newgrf_airport::insert_defaults,
    []
);
registry!(
    object,
    crate::objects::Spec,
    crate::newgrf_object::ObjectClassID,
    "openttd_rust_object_class",
    crate::newgrf_object::insert_defaults,
    [unsafe(export_name = "openttd_rust_object_class_assign")]
);

crate::abi::layouts! {
    fn abi_layout;
    "StationClass" ["station_service"] => Class { global_id, name, rust_index };
    "RoadStopClass" ["station_service"] => Class { global_id, name, rust_index };
    "AirportClass" ["station_service"] => Class { global_id, name, rust_index };
    "ObjectClass" ["station_service"] => Class { global_id, name, rust_index };
    "OpenTTDGRFClassIdentity" ["station_service"] => Identity { grfid, local_id };
    "OpenTTDStationClassServices" ["station_service"] => Services<crate::station_cmd::RailSpec> { class_index, set_index, identity, object, shared };
    "OpenTTDRoadStopClassServices" ["station_service"] => Services<crate::station_cmd::RoadSpec> { class_index, set_index, identity, object, shared };
    "OpenTTDAirportClassServices" ["station_service"] => Services<crate::station_cmd::AirportSpec> { class_index, set_index, identity, object, shared };
    "OpenTTDObjectClassServices" ["station_service"] => Services<crate::objects::Spec> { class_index, set_index, identity, object, shared };
}

pub(crate) unsafe fn station_identity(spec: *const crate::station_cmd::RailSpec) -> Identity {
    unsafe { (station::SERVICES.table().identity)(spec) }
}

pub(crate) fn station_label(class: u16) -> u32 {
    unsafe { (*station::classes().add(usize::from(class))).global_id }
}
pub(crate) fn roadstop_label(class: u16) -> u32 {
    unsafe { (*roadstop::classes().add(usize::from(class))).global_id }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static ASSERTIONS: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn record_assertion(condition: bool) {
        ASSERTIONS.with_borrow_mut(|a| a.push(condition));
    }

    /// `NewGRFClass::Get` asserts `class_index < classes.size()` through the native
    /// assertion, for the default classes and for any other lookup. The kept C++ class test
    /// sees only lookups that pass. The object registry is the one no other test resets.
    #[test]
    fn get_makes_the_original_bound_assertion() {
        let mut movement = crate::test_tables::MOVEMENT;
        movement.native_assert = record_assertion;
        let mut shared = crate::test_tables::SHARED;
        shared.movement = Box::leak(Box::new(movement));
        let services: &'static Services<crate::objects::Spec> = Box::leak(Box::new(Services {
            class_index: crate::test_tables::unreachable1,
            set_index: crate::test_tables::unreachable2,
            identity: crate::test_tables::unreachable1,
            object: ptr::null(),
            shared: Box::leak(Box::new(shared)),
        }));
        unsafe { object::reset(services) };
        // ObjectClass::InsertDefaults looks up both default classes.
        assert_eq!(ASSERTIONS.take(), [true, true]);
        let count = object::count() as u16;
        assert_eq!(count, 2);
        unsafe {
            object::get(crate::newgrf_object::ObjectClassID(count - 1));
            object::get(crate::newgrf_object::ObjectClassID(count));
        }
        assert_eq!(ASSERTIONS.take(), [true, false]);
    }
}
