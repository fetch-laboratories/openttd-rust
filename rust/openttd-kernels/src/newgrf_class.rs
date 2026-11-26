/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `newgrf_class.h` and `newgrf_class_func.h`: `NewGRFClass`, the classes of the `NewGRF`
//! stations, road stops, airports and objects.
//!
//! Rust owns each instantiation's `classes` and the members of each class. C++ reads the
//! three shared members of a class in place through `Classes()` and `Get`, and a class's specs
//! through the spans its facade borrows; both alias the Rust buffers until the next `Reset`,
//! `Allocate` or `Insert`, as they alias the original's vectors. The game-state mutex
//! serializes every access, the specs are native records that outlive their membership, and
//! no registry borrow survives an availability check or a native spec leaf.
use crate::debug::grf_msg;
use crate::game_state::GameState;
use crate::newgrf_specs::{SpecServices, Specs};
use crate::strings_type::StringID;
use crate::table::strings::STR_EMPTY;
use std::marker::PhantomData;
use std::ptr;

/// Struct containing information relating to `NewGRF` classes for stations and airports.
/// Its three members are shared with the C++ `NewGRFClass`, which reads them in place.
#[repr(C)]
pub struct NewGRFClass<T> {
    /// Global ID for class, e.g. 'DFLT', 'WAYP', etc.
    pub global_id: u32,
    /// Name of this class.
    pub name: StringID,
    /// Index of class within the list of classes. The original's member is a `Tindex`; the
    /// shared record holds it as `u32` for every instantiation, so `index()` narrows it back.
    index: u32,
    spec_type: PhantomData<*mut T>,
}

/// The members of a class that only Rust reads, kept beside `classes` in its registry.
struct ClassMembers<T> {
    /// Number of specs in this class potentially available to the user.
    ui_count: u32,
    /// List of specifications.
    spec: Vec<*mut T>,
}

/// One instantiation's `NewGRFClass::classes`, with the Rust-only members of each class at
/// the class's index.
pub(crate) struct Registry<T> {
    /// The actual classes. This may be reallocated during initialization so pointers may be
    /// invalidated.
    classes: Vec<NewGRFClass<T>>,
    members: Vec<ClassMembers<T>>,
}
impl<T> Registry<T> {
    pub(crate) const fn new() -> Self {
        Self {
            classes: Vec::new(),
            members: Vec::new(),
        }
    }
}

/// A registry's `Tindex`: the class-ID wrapper its `NewGRFClass` instantiation indexes by.
/// The exports pass it as `u16`; `from_index` narrows as the original's
/// `static_cast<Tindex>` does.
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
    #[allow(
        clippy::cast_possible_truncation,
        reason = "static_cast<AirportClassID> narrows to the enum's uint8_t"
    )]
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

/// The spec type of one `NewGRFClass` instantiation: its `Tindex`, its registry, and the
/// specialisations the original writes per instantiation.
pub(crate) trait ClassSpec: Sized + 'static {
    /// `Tindex`.
    type Index: ClassIndex;
    /// The animation trigger type of the spec's leaves in `Specs`.
    type Triggers;
    /// This instantiation's registry.
    fn registry() -> *mut Registry<Self>;
    /// The slot of the class table C++ installs at each `Reset`.
    fn services_slot() -> &'static GameState<*const Services<Self>>;
    /// The class table C++ installed at the last `Reset`, which every entry but `Reset`
    /// runs after.
    #[allow(unsafe_code)]
    fn services() -> &'static Services<Self> {
        // SAFETY: Every entry runs after the reset that installs the table.
        unsafe { Self::services_slot().table() }
    }
    /// The spec's leaves in the shared spec tables.
    const SPEC_SERVICES: fn(&Specs) -> &SpecServices<Self, Self::Triggers>;
    /// `NewGRFClass::IsUIAvailable`: check whether the spec will be available to the user at
    /// some point in time.
    fn is_ui_available(class: &NewGRFClass<Self>, index: u32) -> bool;
    /// `NewGRFClass::InsertDefaults`: initialise the defaults.
    fn insert_defaults();
}

/// The native leaves of a spec's `class_index` and `index` members, and the tables the
/// instantiation's specialisations read. C++ fills one per instantiation.
#[repr(C)]
pub struct Services<T> {
    /// `static_cast<uint16_t>(spec->class_index)`.
    pub class_index: unsafe extern "C" fn(*const T) -> u16,
    /// `spec->index = index`.
    pub set_index: unsafe extern "C" fn(*mut T, u16),
    /// The object table, which only the object instantiation's `IsUIAvailable` reads; null
    /// for the others.
    pub object: *const crate::objects::ObjectServices,
    shared: *const crate::services::Services,
}

impl<T: ClassSpec> NewGRFClass<T> {
    /// The members of the class at `class`. A class index from `get` or from a class's own
    /// `index` is in bounds, and the C++ class passes its own `Index()`.
    #[allow(unsafe_code)]
    fn members<'a>(class: u16) -> &'a ClassMembers<T> {
        // SAFETY: The module's serialization contract; the borrow ends before any callback.
        unsafe { &(&(*T::registry()).members)[usize::from(class)] }
    }
    /// `members`, to update them.
    #[allow(unsafe_code)]
    fn members_mut<'a>(class: u16) -> &'a mut ClassMembers<T> {
        // SAFETY: As for `members`; no other borrow of the members is live.
        unsafe { &mut (&mut (*T::registry()).members)[usize::from(class)] }
    }

    /// `NewGRFClass::classes`, whose records C++ reads in place.
    #[allow(unsafe_code)]
    fn classes() -> *mut Self {
        // SAFETY: The module's serialization contract.
        unsafe { (*T::registry()).classes.as_mut_ptr() }
    }
    /// The class at `class`, which the C++ class passes as its own `Index()`.
    #[allow(unsafe_code)]
    fn at<'a>(class: u16) -> &'a Self {
        // SAFETY: The module's serialization contract and the C++ caller's own index.
        unsafe { &*Self::classes().add(usize::from(class)) }
    }

    /// `NewGRFClass::Reset`: reset the classes, i.e. clear everything, after installing the
    /// class table `services`.
    /// # Safety
    /// `services` is a process-lifetime class table.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn reset(services: *const Services<T>) {
        T::services_slot().exchange(services);
        // SAFETY: The module's serialization contract.
        unsafe { *T::registry() = Registry::new() };
        T::insert_defaults();
    }

    /// `NewGRFClass::Allocate`: allocate a class with a given global class ID. Returns the
    /// (non global!) class ID for the class. Upon allocating the same global class ID for a
    /// second time, this first allocation will be given.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the position narrows to Tindex, as the original's static_cast"
    )]
    #[allow(unsafe_code)]
    pub(crate) fn allocate(global_id: u32, tmax: T::Index) -> T::Index {
        // SAFETY: The module's serialization contract; no callback runs in this body.
        let registry = unsafe { &mut *T::registry() };
        let found = registry.classes.iter().find(|c| c.global_id == global_id);

        /* Id is already allocated, so reuse it. */
        if let Some(found) = found {
            return found.index();
        }

        /* More slots available, allocate a slot to the global id. */
        if registry.classes.len() < usize::from(tmax.index()) {
            let index = T::Index::from_index(registry.classes.len() as u16);
            registry.classes.push(NewGRFClass {
                global_id,
                name: STR_EMPTY,
                index: u32::from(index.index()),
                spec_type: PhantomData,
            });
            registry.members.push(ClassMembers {
                ui_count: 0,
                spec: Vec::new(),
            });
            return index;
        }

        grf_msg!(
            2,
            "ClassAllocate: already allocated {} classes, using default",
            tmax.index()
        );
        T::Index::from_index(0)
    }

    /// `NewGRFClass::Insert`: insert a spec into the class, and update its index.
    /// # Safety
    /// The class table is installed, and a non-null `spec` is a live native spec.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the position narrows to uint16_t, as the original's static_cast"
    )]
    #[allow(unsafe_code)]
    pub(crate) unsafe fn insert(&self, spec: *mut T) {
        let class = self.index().index();
        let members = Self::members_mut(class);
        members.spec.push(spec);
        let index = (members.spec.len() - 1) as u16;
        if !spec.is_null() {
            // SAFETY: The caller's spec contract.
            unsafe { (T::services().set_index)(spec, index) };
        }

        // The availability is checked at the narrowed index, even after it wraps.
        if T::is_ui_available(self, u32::from(index)) {
            let members = Self::members_mut(class);
            members.ui_count = members.ui_count.wrapping_add(1);
        }
    }

    /// `NewGRFClass::Assign`: assign a spec to one of the classes. The spec must have a valid
    /// class index set.
    /// # Safety
    /// The class table is installed and `spec` is a live native spec.
    #[allow(unsafe_code)]
    pub(crate) unsafe fn assign(spec: *mut T) {
        // SAFETY: The caller's contracts.
        let class_index = T::Index::from_index(unsafe { (T::services().class_index)(spec) });
        crate::services_assert::assert_with(|| {
            usize::from(class_index.index()) < Self::get_class_count() as usize
        });
        // SAFETY: `get` returns a class of the registry; the caller's contracts.
        unsafe { (*Self::get(class_index)).insert(spec) };
    }

    /// `NewGRFClass::Get`: get a particular class.
    #[allow(unsafe_code)]
    pub(crate) fn get(class_index: T::Index) -> *mut Self {
        // SAFETY: The module's serialization contract; no callback runs in this body.
        let classes = unsafe { &mut (*T::registry()).classes };
        crate::services_assert::assert_with(|| usize::from(class_index.index()) < classes.len());
        // A build without assertions indexes past the end, as the original's
        // `classes[class_index]`.
        classes
            .as_mut_ptr()
            .wrapping_add(usize::from(class_index.index()))
    }

    /// `NewGRFClass::GetClassCount`: get the number of allocated classes.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the size narrows to uint, as the original's static_cast"
    )]
    #[allow(unsafe_code)]
    pub(crate) fn get_class_count() -> u32 {
        // SAFETY: The module's serialization contract.
        unsafe { (*T::registry()).classes.len() as u32 }
    }

    /// `NewGRFClass::GetUIClassCount`: get the number of classes available to the user.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "count_if's difference narrows to the uint result"
    )]
    #[allow(unsafe_code)]
    pub(crate) fn get_ui_class_count() -> u32 {
        // SAFETY: The module's serialization contract.
        let classes = unsafe { &(*T::registry()).classes };
        classes
            .iter()
            .filter(|cls| cls.get_ui_spec_count() > 0)
            .count() as u32
    }

    /// `NewGRFClass::GetByGrf`: retrieve a spec by GRF location.
    #[allow(unsafe_code)]
    pub(crate) fn get_by_grf(grfid: u32, local_id: u16) -> *const T {
        // SAFETY: The class table names the process-lifetime shared table.
        let leaves = (T::SPEC_SERVICES)(unsafe { &*T::services().shared }.specs());
        // SAFETY: The module's serialization contract; the spec leaves are native field reads
        // that do not reenter the registry.
        let registry = unsafe { &*T::registry() };
        for cls in &registry.members {
            for &spec in &cls.spec {
                if spec.is_null() {
                    continue;
                }
                if (leaves.local_id)(spec) != local_id {
                    continue;
                }
                if (leaves.grfid)(spec) == grfid {
                    return spec;
                }
            }
        }

        ptr::null()
    }

    /// `NewGRFClass::Index`.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the shared u32 holds a Tindex, which the narrowing restores"
    )]
    pub(crate) fn index(&self) -> T::Index {
        T::Index::from_index(self.index as u16)
    }

    /// `NewGRFClass::Specs`: get read-only span of specs of this class.
    pub(crate) fn specs(&self) -> &[*mut T] {
        &Self::members(self.index().index()).spec
    }

    /// `NewGRFClass::GetSpecCount`: get the number of allocated specs within the class.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the size narrows to uint, as the original's static_cast"
    )]
    pub(crate) fn get_spec_count(&self) -> u32 {
        self.specs().len() as u32
    }

    /// `NewGRFClass::GetUISpecCount`: get the number of potentially user-available specs
    /// within the class.
    pub(crate) fn get_ui_spec_count(&self) -> u32 {
        Self::members(self.index().index()).ui_count
    }

    /// `NewGRFClass::GetSpec`: get a spec from the class at a given index.
    pub(crate) fn get_spec(&self, index: u32) -> *const T {
        /* If the custom spec isn't defined any more, then the GRF file probably was not loaded. */
        if index < self.get_spec_count() {
            self.specs()[index as usize]
        } else {
            ptr::null()
        }
    }
}

/// The closure Action 0 registers for a class name property: `Get(spec->class_index)->name =
/// str` once the string is mapped.
/// # Safety
/// The class table is installed and `spec` is a live native spec.
#[allow(unsafe_code)]
pub(crate) unsafe fn set_class_name<T: ClassSpec>(spec: *const T, name: StringID) {
    // SAFETY: The caller's contracts.
    let class_index = T::Index::from_index(unsafe { (T::services().class_index)(spec) });
    // SAFETY: `get` returns a class of the registry.
    unsafe { (*NewGRFClass::<T>::get(class_index)).name = name };
}

/// `NewGRFClass::Reset`.
/// # Safety
/// `s` is the process-lifetime class table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_class_reset(
    s: *const Services<crate::station_cmd::RailSpec>,
) {
    // SAFETY: The caller's table contract.
    unsafe { NewGRFClass::reset(s) };
}
/// `NewGRFClass::Allocate`, with the facade's `Tmax`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_allocate(global_id: u32, tmax: u16) -> u16 {
    NewGRFClass::<crate::station_cmd::RailSpec>::allocate(
        global_id,
        crate::newgrf_station::StationClassID::from_index(tmax),
    )
    .index()
}
/// `NewGRFClass::Assign`.
/// # Safety
/// `spec` is a live native spec.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_class_assign(
    spec: *mut crate::station_cmd::RailSpec,
) {
    // SAFETY: The caller's spec contract.
    unsafe { NewGRFClass::assign(spec) };
}
/// `set_class_name`, for the native class name closure.
/// # Safety
/// `spec` is a live native spec.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_class_set_name(
    spec: *const crate::station_cmd::RailSpec,
    name: u32,
) {
    // SAFETY: The caller's spec contract.
    unsafe { set_class_name(spec, StringID(name)) };
}
/// `NewGRFClass::classes`, which `Classes()` and `Get` read in place.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_classes()
-> *mut NewGRFClass<crate::station_cmd::RailSpec> {
    NewGRFClass::classes()
}
/// `NewGRFClass::GetClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_count() -> u32 {
    NewGRFClass::<crate::station_cmd::RailSpec>::get_class_count()
}
/// `NewGRFClass::GetUIClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_ui_count() -> u32 {
    NewGRFClass::<crate::station_cmd::RailSpec>::get_ui_class_count()
}
/// `NewGRFClass::Specs` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_specs(
    class: u16,
) -> *const *mut crate::station_cmd::RailSpec {
    NewGRFClass::at(class).specs().as_ptr()
}
/// `NewGRFClass::GetSpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::station_cmd::RailSpec>::at(class).get_spec_count()
}
/// `NewGRFClass::GetUISpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_ui_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::station_cmd::RailSpec>::at(class).get_ui_spec_count()
}
/// `NewGRFClass::GetSpec` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_spec(
    class: u16,
    index: u32,
) -> *const crate::station_cmd::RailSpec {
    NewGRFClass::at(class).get_spec(index)
}
/// `NewGRFClass::GetByGrf`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_station_class_by_grf(
    grfid: u32,
    local_id: u16,
) -> *const crate::station_cmd::RailSpec {
    NewGRFClass::get_by_grf(grfid, local_id)
}

/// `NewGRFClass::Reset`.
/// # Safety
/// `s` is the process-lifetime class table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_class_reset(
    s: *const Services<crate::station_cmd::RoadSpec>,
) {
    // SAFETY: The caller's table contract.
    unsafe { NewGRFClass::reset(s) };
}
/// `NewGRFClass::Allocate`, with the facade's `Tmax`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_allocate(global_id: u32, tmax: u16) -> u16 {
    NewGRFClass::<crate::station_cmd::RoadSpec>::allocate(
        global_id,
        crate::newgrf_roadstop::RoadStopClassID::from_index(tmax),
    )
    .index()
}
/// `NewGRFClass::Assign`.
/// # Safety
/// `spec` is a live native spec.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_class_assign(
    spec: *mut crate::station_cmd::RoadSpec,
) {
    // SAFETY: The caller's spec contract.
    unsafe { NewGRFClass::assign(spec) };
}
/// `set_class_name`, for the native class name closure.
/// # Safety
/// `spec` is a live native spec.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_roadstop_class_set_name(
    spec: *const crate::station_cmd::RoadSpec,
    name: u32,
) {
    // SAFETY: The caller's spec contract.
    unsafe { set_class_name(spec, StringID(name)) };
}
/// `NewGRFClass::classes`, which `Classes()` and `Get` read in place.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_classes()
-> *mut NewGRFClass<crate::station_cmd::RoadSpec> {
    NewGRFClass::classes()
}
/// `NewGRFClass::GetClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_count() -> u32 {
    NewGRFClass::<crate::station_cmd::RoadSpec>::get_class_count()
}
/// `NewGRFClass::GetUIClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_ui_count() -> u32 {
    NewGRFClass::<crate::station_cmd::RoadSpec>::get_ui_class_count()
}
/// `NewGRFClass::Specs` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_specs(
    class: u16,
) -> *const *mut crate::station_cmd::RoadSpec {
    NewGRFClass::at(class).specs().as_ptr()
}
/// `NewGRFClass::GetSpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::station_cmd::RoadSpec>::at(class).get_spec_count()
}
/// `NewGRFClass::GetUISpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_ui_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::station_cmd::RoadSpec>::at(class).get_ui_spec_count()
}
/// `NewGRFClass::GetSpec` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_spec(
    class: u16,
    index: u32,
) -> *const crate::station_cmd::RoadSpec {
    NewGRFClass::at(class).get_spec(index)
}
/// `NewGRFClass::GetByGrf`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_roadstop_class_by_grf(
    grfid: u32,
    local_id: u16,
) -> *const crate::station_cmd::RoadSpec {
    NewGRFClass::get_by_grf(grfid, local_id)
}

/// `NewGRFClass::Reset`.
/// # Safety
/// `s` is the process-lifetime class table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_airport_class_reset(
    s: *const Services<crate::station_cmd::AirportSpec>,
) {
    // SAFETY: The caller's table contract.
    unsafe { NewGRFClass::reset(s) };
}
/// `NewGRFClass::Allocate`, with the facade's `Tmax`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_allocate(global_id: u32, tmax: u16) -> u16 {
    NewGRFClass::<crate::station_cmd::AirportSpec>::allocate(
        global_id,
        crate::newgrf_airport::AirportClassID::from_index(tmax),
    )
    .index()
}
/// `NewGRFClass::classes`, which `Classes()` and `Get` read in place.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_classes()
-> *mut NewGRFClass<crate::station_cmd::AirportSpec> {
    NewGRFClass::classes()
}
/// `NewGRFClass::GetClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_count() -> u32 {
    NewGRFClass::<crate::station_cmd::AirportSpec>::get_class_count()
}
/// `NewGRFClass::GetUIClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_ui_count() -> u32 {
    NewGRFClass::<crate::station_cmd::AirportSpec>::get_ui_class_count()
}
/// `NewGRFClass::Specs` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_specs(
    class: u16,
) -> *const *mut crate::station_cmd::AirportSpec {
    NewGRFClass::at(class).specs().as_ptr()
}
/// `NewGRFClass::GetSpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::station_cmd::AirportSpec>::at(class).get_spec_count()
}
/// `NewGRFClass::GetUISpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_ui_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::station_cmd::AirportSpec>::at(class).get_ui_spec_count()
}
/// `NewGRFClass::GetSpec` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_spec(
    class: u16,
    index: u32,
) -> *const crate::station_cmd::AirportSpec {
    NewGRFClass::at(class).get_spec(index)
}
/// `NewGRFClass::GetByGrf`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_airport_class_by_grf(
    grfid: u32,
    local_id: u16,
) -> *const crate::station_cmd::AirportSpec {
    NewGRFClass::get_by_grf(grfid, local_id)
}

/// `NewGRFClass::Reset`.
/// # Safety
/// `s` is the process-lifetime class table.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_class_reset(s: *const Services<crate::objects::Spec>) {
    // SAFETY: The caller's table contract.
    unsafe { NewGRFClass::reset(s) };
}
/// `NewGRFClass::Allocate`, with the facade's `Tmax`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_allocate(global_id: u32, tmax: u16) -> u16 {
    NewGRFClass::<crate::objects::Spec>::allocate(
        global_id,
        crate::newgrf_object::ObjectClassID::from_index(tmax),
    )
    .index()
}
/// `NewGRFClass::Assign`.
/// # Safety
/// `spec` is a live native spec.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_class_assign(spec: *mut crate::objects::Spec) {
    // SAFETY: The caller's spec contract.
    unsafe { NewGRFClass::assign(spec) };
}
/// `NewGRFClass::classes`, which `Classes()` and `Get` read in place.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_classes() -> *mut NewGRFClass<crate::objects::Spec> {
    NewGRFClass::classes()
}
/// `NewGRFClass::GetClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_count() -> u32 {
    NewGRFClass::<crate::objects::Spec>::get_class_count()
}
/// `NewGRFClass::GetUIClassCount`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_ui_count() -> u32 {
    NewGRFClass::<crate::objects::Spec>::get_ui_class_count()
}
/// `NewGRFClass::Specs` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_specs(class: u16) -> *const *mut crate::objects::Spec {
    NewGRFClass::at(class).specs().as_ptr()
}
/// `NewGRFClass::GetSpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::objects::Spec>::at(class).get_spec_count()
}
/// `NewGRFClass::GetUISpecCount` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_ui_spec_count(class: u16) -> u32 {
    NewGRFClass::<crate::objects::Spec>::at(class).get_ui_spec_count()
}
/// `NewGRFClass::GetSpec` of the C++ class whose `Index()` is `class`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_spec(
    class: u16,
    index: u32,
) -> *const crate::objects::Spec {
    NewGRFClass::at(class).get_spec(index)
}
/// `NewGRFClass::GetByGrf`.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_class_by_grf(
    grfid: u32,
    local_id: u16,
) -> *const crate::objects::Spec {
    NewGRFClass::get_by_grf(grfid, local_id)
}

crate::abi::layouts! {
    fn abi_layout;
    "StationClass" ["station_service"] => NewGRFClass<crate::station_cmd::RailSpec> { global_id, name, index };
    "RoadStopClass" ["station_service"] => NewGRFClass<crate::station_cmd::RoadSpec> { global_id, name, index };
    "AirportClass" ["station_service"] => NewGRFClass<crate::station_cmd::AirportSpec> { global_id, name, index };
    "ObjectClass" ["station_service"] => NewGRFClass<crate::objects::Spec> { global_id, name, index };
    "OpenTTDStationClassServices" ["station_service"] => Services<crate::station_cmd::RailSpec> { class_index, set_index, object, shared };
    "OpenTTDRoadStopClassServices" ["station_service"] => Services<crate::station_cmd::RoadSpec> { class_index, set_index, object, shared };
    "OpenTTDAirportClassServices" ["station_service"] => Services<crate::station_cmd::AirportSpec> { class_index, set_index, object, shared };
    "OpenTTDObjectClassServices" ["station_service"] => Services<crate::objects::Spec> { class_index, set_index, object, shared };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::newgrf_airport::{AirportClass, AirportClassID};
    use crate::newgrf_object::{ObjectClass, ObjectClassID};
    use crate::station_cmd::AirportSpec;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

    static FAILED_ASSERTIONS: AtomicU32 = AtomicU32::new(0);
    extern "C" fn record_assertion(condition: bool) {
        if !condition {
            FAILED_ASSERTIONS.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The indices `set_index` stores, in order.
    static SET_INDEX: Mutex<Vec<u16>> = Mutex::new(Vec::new());
    /// A stand-in spec: the `class_index` member the class leaves read, and the views the
    /// object eligibility leaf reports.
    struct StandIn {
        class_index: u16,
        views: u8,
    }
    #[allow(unsafe_code)]
    unsafe extern "C" fn class_index<T>(spec: *const T) -> u16 {
        // SAFETY: Every spec the tests pass is a live `StandIn`.
        unsafe { (*spec.cast::<StandIn>()).class_index }
    }
    extern "C" fn set_index<T>(_: *mut T, index: u16) {
        SET_INDEX.lock().unwrap().push(index);
    }
    fn services<T>(object: *const crate::objects::ObjectServices) -> &'static Services<T> {
        Box::leak(Box::new(Services {
            class_index: class_index::<T>,
            set_index: set_index::<T>,
            object,
            shared: ptr::null(),
        }))
    }

    /// `NewGRFClass::Get` asserts `class_index < classes.size()`, for the default classes and
    /// for any other lookup, and an assertion-free build goes on to index past the end. The
    /// kept C++ class test sees only lookups that pass.
    #[test]
    #[allow(unsafe_code)] // The object registry's reset, with a table of only its leaves.
    fn get_makes_the_original_bound_assertion() {
        if crate::services_assert::isolate_test_check(
            "newgrf_class::tests::get_makes_the_original_bound_assertion",
            record_assertion,
        ) {
            return;
        }
        // SAFETY: The table lives for the process; `ObjectClass::InsertDefaults` reads none
        // of its leaves.
        unsafe { ObjectClass::reset(services(ptr::null())) };
        // ObjectClass::InsertDefaults looks up both default classes.
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 0);
        let count = u16::try_from(ObjectClass::get_class_count()).unwrap();
        assert_eq!(count, 2);
        ObjectClass::get(ObjectClassID(count - 1));
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 0);
        ObjectClass::get(ObjectClassID(count));
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 1);
    }

    /// Exit code and marker of `two_failures`, which ends the process at the second failed
    /// assertion.
    const SECOND_FAILURE_CODE: i32 = 87;
    const SECOND_FAILURE_MARKER: &str = "newgrf_class test: second failed assertion";
    extern "C" fn two_failures(condition: bool) {
        if !condition && FAILED_ASSERTIONS.fetch_add(1, Ordering::Relaxed) == 1 {
            eprintln!("{SECOND_FAILURE_MARKER}");
            std::process::exit(SECOND_FAILURE_CODE);
        }
    }

    /// `NewGRFClass::Assign` asserts the spec's class index against the class count, and then
    /// `Get` asserts it again. Under a policy that returns from the first failure and ends the
    /// process at the second, a spec whose class index is the count ends at `Get`'s
    /// assertion, before the class is indexed or anything is stored. Without `Assign`'s own
    /// assertion, `Get`'s is the first, and the body goes on to index the class.
    #[test]
    #[allow(unsafe_code)] // A live stand-in spec that only the class leaves receive.
    fn assign_of_a_class_index_past_the_classes_fails_both_assertions() {
        if crate::services_assert::isolate_test_ends(
            "newgrf_class::tests::assign_of_a_class_index_past_the_classes_fails_both_assertions",
            SECOND_FAILURE_CODE,
            SECOND_FAILURE_MARKER,
        ) {
            return;
        }
        crate::services_assert::bind_test_check(two_failures);
        // SAFETY: The table lives for the process; `AirportClass::InsertDefaults` reads none
        // of its leaves.
        unsafe { AirportClass::reset(services(ptr::null())) };
        let mut spec = StandIn {
            class_index: u16::try_from(AirportClass::get_class_count()).unwrap(),
            views: 0,
        };
        // SAFETY: `spec` is live; the second failed assertion ends the process.
        unsafe { AirportClass::assign((&raw mut spec).cast::<AirportSpec>()) };
        unreachable!("Assign returned past its failed assertions");
    }

    /// `NewGRFClass::Assign` of a spec with a valid class index inserts it into that class:
    /// the spec's index is its position, and `IsUIAvailable`, always true for an airport,
    /// counts it for the user.
    #[test]
    #[allow(unsafe_code)] // Live stand-in specs that only the class leaves receive.
    fn assign_inserts_into_the_spec_class_and_counts_it_for_the_ui() {
        if crate::services_assert::isolate_test_check(
            "newgrf_class::tests::assign_inserts_into_the_spec_class_and_counts_it_for_the_ui",
            record_assertion,
        ) {
            return;
        }
        // SAFETY: The table lives for the process; `AirportClass::InsertDefaults` reads none
        // of its leaves.
        unsafe { AirportClass::reset(services(ptr::null())) };
        // AirportClass::InsertDefaults allocates four classes and inserts no spec.
        assert_eq!(AirportClass::get_class_count(), 4);
        assert_eq!(AirportClass::get_ui_class_count(), 0);
        let mut first = StandIn {
            class_index: 1,
            views: 0,
        };
        let mut second = StandIn {
            class_index: 1,
            views: 0,
        };
        // SAFETY: Both specs are live through the test.
        unsafe {
            AirportClass::assign((&raw mut first).cast());
            AirportClass::assign((&raw mut second).cast());
        }
        assert_eq!(*SET_INDEX.lock().unwrap(), [0, 1]);
        // SAFETY: `Get` returns a class of the registry.
        let class = unsafe { &*AirportClass::get(AirportClassID(1)) };
        assert_eq!(class.get_spec_count(), 2);
        assert_eq!(class.get_ui_spec_count(), 2);
        assert_eq!(class.get_spec(1), (&raw const second).cast());
        assert_eq!(class.get_spec(2), ptr::null());
        assert_eq!(AirportClass::get_ui_class_count(), 1);
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 0);
    }

    /// The object eligibility of a stand-in: its views, every climate, and no flags.
    #[allow(unsafe_code)]
    unsafe extern "C" fn eligibility(
        spec: *const crate::objects::Spec,
    ) -> crate::objects::Eligibility {
        crate::objects::Eligibility {
            flags: 0,
            climate: u8::MAX,
            // SAFETY: Every spec the test passes is a live `StandIn`.
            views: unsafe { (*spec.cast::<StandIn>()).views },
        }
    }
    extern "C" fn no() -> bool {
        false
    }
    extern "C" fn temperate() -> u8 {
        0
    }

    /// `ObjectClass::IsUIAvailable` is `GetSpec(index)->IsEverAvailable()`: `Assign` inserts
    /// both specs into their class, and counts for the user only the one that will be
    /// available, here the one with views in a game that is not being generated.
    #[test]
    #[allow(unsafe_code)] // Live stand-in specs that only the class and eligibility leaves receive.
    fn object_assign_counts_only_an_ever_available_spec_for_the_ui() {
        if crate::services_assert::isolate_test_check(
            "newgrf_class::tests::object_assign_counts_only_an_ever_available_spec_for_the_ui",
            record_assertion,
        ) {
            return;
        }
        let mut shared = crate::test_tables::SHARED;
        shared.clear_square_services.generating_world = no;
        let shared: &'static crate::services::Services = Box::leak(Box::new(shared));
        let mut town = crate::test_tables::TOWN_LEAVES;
        town.shared = shared;
        town.climate = temperate;
        town.environment.editor = no;
        let resolver = crate::objects::ResolverServices {
            shared: ptr::null(),
            views: crate::test_tables::unreachable1,
            eligibility,
            introduction: crate::test_tables::unreachable1,
            lifetime: crate::test_tables::unreachable1,
            town_scope: crate::test_tables::unreachable3,
            base_scope: crate::test_tables::unreachable3,
            evaluate: crate::test_tables::unreachable6,
        };
        let mut objects = crate::test_tables::OBJECTS;
        objects.town = Box::leak(Box::new(town));
        objects.resolver = Box::leak(Box::new(resolver));
        let objects: &'static crate::objects::ObjectServices = Box::leak(Box::new(objects));
        // SAFETY: The table lives for the process; `ObjectClass::InsertDefaults` reads none
        // of its leaves.
        unsafe { ObjectClass::reset(services(objects)) };
        let mut shown = StandIn {
            class_index: 1,
            views: 1,
        };
        let mut hidden = StandIn {
            class_index: 1,
            views: 0,
        };
        // SAFETY: Both specs are live through the test.
        unsafe {
            ObjectClass::assign((&raw mut hidden).cast());
            ObjectClass::assign((&raw mut shown).cast());
        }
        assert_eq!(*SET_INDEX.lock().unwrap(), [0, 1]);
        // SAFETY: `Get` returns a class of the registry.
        let class = unsafe { &*ObjectClass::get(ObjectClassID(1)) };
        assert_eq!(class.get_spec_count(), 2);
        assert_eq!(class.get_ui_spec_count(), 1);
        assert_eq!(class.get_spec(1), (&raw const shown).cast());
        assert_eq!(ObjectClass::get_ui_class_count(), 1);
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 0);
    }
}
