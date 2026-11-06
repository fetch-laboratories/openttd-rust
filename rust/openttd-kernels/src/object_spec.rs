/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Canonical object definitions. File-owned boxes stay stable until publication; global
//! elements move on growth/reset, before class binding. All entries hold the game-state
//! mutex. Native read views and deferred targets never outlive their actual owners.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]
use crate::{game_state::GameState, industrytype::NativeVec};
use std::{ffi::c_void, ptr};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Props {
    pub local_id: u16,
    pub grfid: u32,
    pub grffile: *const c_void,
    pub spritegroups: [*const c_void; 2],
}
impl Props {
    const fn new() -> Self {
        Self {
            local_id: 0,
            grfid: 0,
            grffile: ptr::null(),
            spritegroups: [ptr::null(); 2],
        }
    }
    pub fn sprite_group(&self, exists: bool) -> *const c_void {
        if !exists && !self.spritegroups[1].is_null() {
            self.spritegroups[1]
        } else {
            self.spritegroups[0]
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Animation {
    pub frames: u8,
    pub status: u8,
    pub speed: u8,
    pub triggers: u16,
}
crate::abi::shared_records! {
    fn abi_record;
    #[derive(Clone)]
    "OpenTTDObjectSpecRecord" ["object_spec"] => pub struct Spec {
        pub class_index: u16 => "ObjectClassID",
        pub index: u16,
        pub grf_prop: Props => "StandardGRFFileProps",
        pub animation: Animation => "AnimationInfo<ObjectAnimationTriggers>",
        pub name: u32 => "StringID",
        pub climate: u8 => "LandscapeTypes",
        pub size: u8,
        pub build_cost_multiplier: u8,
        pub clear_cost_multiplier: u8,
        pub introduction_date: i32 => "TimerGameCalendar::Date",
        pub end_of_life_date: i32 => "TimerGameCalendar::Date",
        pub flags: u16 => "ObjectFlags",
        pub callback_mask: u8 => "ObjectCallbackMasks",
        pub height: u8,
        pub views: u8,
        pub generate_amount: u8,
        pub badges: NativeVec<u32> => "RustVector<BadgeID>",
    }
}
impl Spec {
    pub const fn new() -> Self {
        Self {
            class_index: 0,
            index: 0,
            grf_prop: Props::new(),
            animation: Animation {
                frames: 0,
                status: 255,
                speed: 0,
                triggers: 0,
            },
            name: 0,
            climate: 0,
            size: 0,
            build_cost_multiplier: 0,
            clear_cost_multiplier: 0,
            introduction_date: 0,
            end_of_life_date: 0,
            flags: 0,
            callback_mask: 0,
            height: 0,
            views: 0,
            generate_amount: 0,
            badges: NativeVec::new(),
        }
    }
    pub fn enabled(&self) -> bool {
        self.views > 0
    }
}
impl Default for Spec {
    fn default() -> Self {
        Self::new()
    }
}
pub(crate) static SPECS: GameState<NativeVec<Spec>> = GameState::new(NativeVec::new());
static EMPTY: GameState<Spec> = GameState::new(Spec::new());
pub fn count() -> usize {
    unsafe { (*SPECS.get()).len() }
}
pub unsafe fn get(id: u16) -> *const Spec {
    assert!(id < 64000);
    unsafe {
        (&*SPECS.get())
            .get(usize::from(id))
            .map_or(EMPTY.get(), ptr::from_ref)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_spec_init(out: *mut Spec) {
    unsafe { out.write(Spec::new()) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_spec_drop(spec: *mut Spec) {
    unsafe { ptr::drop_in_place(spec) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_spec_clone(out: *mut Spec, source: *const Spec) {
    let copy = unsafe { (*source).clone() };
    unsafe { out.write(copy) };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_spec_assign(out: *mut Spec, source: *const Spec) {
    let copy = unsafe { (*source).clone() };
    unsafe { *out = copy };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_specs() -> *mut NativeVec<Spec> {
    SPECS.get()
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_object_spec_count() -> usize {
    count()
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_spec_get(id: u16) -> *const Spec {
    unsafe { get(id) }
}
crate::abi::layouts! {
    fn abi_layout;
    "StandardGRFFileProps" ["object_spec"] => Props { local_id, grfid, grffile, spritegroups };
    "AnimationInfo<ObjectAnimationTriggers>" ["object_spec"] => Animation { frames, status, speed, triggers };
}
const fn original(size: u8, cost: u8, height: u8, climate: u8, amount: u8, flags: u16) -> Spec {
    let mut spec = Spec::new();
    spec.class_index = u16::MAX;
    spec.animation.speed = 2;
    spec.climate = climate;
    spec.size = size;
    spec.build_cost_multiplier = cost;
    spec.clear_cost_multiplier = cost;
    spec.height = height;
    spec.views = 1;
    spec.generate_amount = amount;
    spec.flags = flags;
    spec.end_of_life_date = crate::timer_game_common::MAX_DATE + 1;
    spec
}
static DEFAULTS: GameState<[Spec; 5]> = GameState::new([
    original(0x11, 0, 10, 7, 15, 3),
    original(0x11, 0, 8, 3, 8, 3 | (1 << 13)),
    original(0x11, 0, 5, 15, 0, 3 | (1 << 7)),
    original(
        0x11,
        10,
        0,
        15,
        0,
        (1 << 2) | (1 << 7) | (1 << 4) | (1 << 5),
    ),
    original(0x22, 0, 7, 15, 0, (1 << 1) | (1 << 7)),
]);
pub unsafe fn reset(s: &crate::objects::ObjectServices) {
    let names = [
        s.strings.transmitter,
        s.strings.lighthouse,
        s.strings.statue,
        s.strings.land,
        s.strings.headquarters,
    ];
    unsafe {
        (*SPECS.get()).edit(Vec::clear);
        for (i, &name) in names.iter().enumerate() {
            (*DEFAULTS.get())[i].name = name;
        }
        (*SPECS.get()).edit(|v| {
            v.reserve(5);
            for (i, default) in (*DEFAULTS.get()).iter().enumerate() {
                let mut spec = default.clone();
                spec.grf_prop.local_id = i as u16;
                v.push(spec);
            }
        });
        let lighthouse =
            crate::newgrf_class::object::allocate(u32::from_be_bytes(*b"LTHS"), u16::MAX, 0);
        (&mut *SPECS.get())[1].class_index = lighthouse;
        let transmitter =
            crate::newgrf_class::object::allocate(u32::from_be_bytes(*b"TRNS"), u16::MAX, 0);
        (&mut *SPECS.get())[0].class_index = transmitter;
    }
}
/// Same `NativeVec` representation; null slots are the native unique-owner nulls.
pub type FileSpecs = NativeVec<Option<Box<Spec>>>;
pub unsafe fn file_spec(file: *mut FileSpecs, id: u16) -> *mut Spec {
    if usize::from(id) >= unsafe { (*file).len() } {
        return ptr::null_mut();
    }
    unsafe {
        (&mut *file)
            .get_mut(usize::from(id))
            .and_then(Option::as_deref_mut)
            .map_or(ptr::null_mut(), ptr::from_mut)
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_file_clear(file: *mut FileSpecs) {
    unsafe { *file = NativeVec::new() };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_file_badge(file: *mut FileSpecs, badge: u32) -> u8 {
    if unsafe { (*file).is_empty() } {
        return 0;
    }
    let mut found = false;
    for spec in unsafe { &mut *file }.iter_mut().flatten() {
        spec.badges.push(badge);
        found = true;
    }
    u8::from(found)
}
use crate::debug::grf_msg;
use crate::newgrf_bytereader::{ByteReader, ReadError, Reader, Services as ReaderServices};
use crate::newgrf_internal::{CIR_INVALID_ID, CIR_SUCCESS, CIR_UNKNOWN};
fn ignore(prop: i32, r: &mut Reader<'_>) -> Result<i32, ReadError> {
    match prop {
        0x0b..=0x0d | 0x12 | 0x14 | 0x16..=0x18 => {
            r.byte()?;
        }
        0x09 | 0x0a | 0x10 | 0x11 | 0x13 | 0x15 => {
            r.word()?;
        }
        0x08 | 0x0e | 0x0f => {
            r.dword()?;
        }
        0x19 => r.skip_badges()?,
        _ => return Ok(CIR_UNKNOWN),
    }
    Ok(CIR_SUCCESS)
}
unsafe fn properties(
    file: *mut FileSpecs,
    first: u32,
    last: u32,
    prop: i32,
    r: &mut Reader<'_>,
    map_string: unsafe extern "C" fn(u16, *mut Spec, bool),
    has_2cc: *mut bool,
) -> Result<i32, ReadError> {
    if last > 64000 {
        grf_msg!(
            1,
            "ObjectChangeInfo: Too many objects loaded ({}), max ({}). Ignoring.",
            last,
            64000
        );
        return Ok(CIR_INVALID_ID);
    }
    if unsafe { (*file).len() } < last as usize {
        unsafe { (*file).resize_with(last as usize, || None) };
    }
    let mut result = CIR_SUCCESS;
    for id in first..last {
        let mut spec = unsafe { file_spec(file, id as u16) };
        if prop != 0x08 && spec.is_null() {
            result = result.max(ignore(prop, r)?);
            continue;
        }
        match prop {
            0x08 => {
                if spec.is_null() {
                    let mut new = Box::new(Spec::new());
                    new.views = 1;
                    new.size = 0x11;
                    spec = &raw mut *new;
                    unsafe { (&mut *file)[id as usize] = Some(new) };
                }
                let class = r.dword()?.swap_bytes();
                let index = unsafe { crate::newgrf_class::object::allocate(class, u16::MAX, 0) };
                unsafe { (*spec).class_index = index };
            }
            0x09 | 0x0a => {
                let source = r.word()?;
                unsafe { map_string(source, spec, prop == 0x09) };
            }
            0x0b => unsafe { (*spec).climate = r.byte()? },
            0x0c => {
                unsafe { (*spec).size = r.byte()? };
                let size = unsafe { (*spec).size };
                if size.trailing_zeros() >= 4 || size >> 4 == 0 {
                    grf_msg!(
                        0,
                        "ObjectChangeInfo: Invalid object size requested (0x{:X}) for object id {}. Ignoring.",
                        size,
                        id
                    );
                    unsafe { (*spec).size = 0x11 };
                }
            }
            0x0d => unsafe {
                (*spec).build_cost_multiplier = r.byte()?;
                (*spec).clear_cost_multiplier = (*spec).build_cost_multiplier;
            },
            0x0e => unsafe { (*spec).introduction_date = r.dword()? as i32 },
            0x0f => unsafe { (*spec).end_of_life_date = r.dword()? as i32 },
            0x10 => unsafe {
                (*spec).flags = r.word()?;
                *has_2cc |= (*spec).flags & (1 << 8) != 0;
            },
            0x11 => unsafe {
                (*spec).animation.frames = r.byte()?;
                (*spec).animation.status = r.byte()?;
            },
            0x12 => unsafe { (*spec).animation.speed = r.byte()? },
            0x13 => unsafe { (*spec).animation.triggers = r.word()? },
            0x14 => unsafe { (*spec).clear_cost_multiplier = r.byte()? },
            0x15 => unsafe { (*spec).callback_mask = r.word()? as u8 },
            0x16 => unsafe { (*spec).height = r.byte()? },
            0x17 => {
                unsafe { (*spec).views = r.byte()? };
                let views = unsafe { (*spec).views };
                if !matches!(views, 1 | 2 | 4) {
                    grf_msg!(
                        2,
                        "ObjectChangeInfo: Invalid number of views ({}) for object id {}. Ignoring.",
                        views,
                        id
                    );
                    unsafe { (*spec).views = 1 };
                }
            }
            0x18 => unsafe { (*spec).generate_amount = r.byte()? },
            0x19 => {
                let ids = r.badge_ids(0x0f)?;
                unsafe { (*spec).badges = NativeVec::from_vec(ids) };
            }
            _ => result = CIR_UNKNOWN,
        }
    }
    Ok(result)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_properties(
    file: *mut FileSpecs,
    first: u32,
    last: u32,
    prop: i32,
    reader: *mut ByteReader,
    services: &ReaderServices,
    map_string: unsafe extern "C" fn(u16, *mut Spec, bool),
    has_2cc: *mut bool,
    result: *mut i32,
) -> u8 {
    let mut reader = unsafe { Reader::new(reader, services) };
    match unsafe { properties(file, first, last, prop, &mut reader, map_string, has_2cc) } {
        Ok(value) => {
            unsafe { *result = value };
            1
        }
        Err(_) => 0,
    }
}
static MANAGER: GameState<*mut crate::override_manager::Manager> = GameState::new(ptr::null_mut());
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_object_register_manager(
    manager: *mut crate::override_manager::Manager,
) {
    unsafe { *MANAGER.get() = manager };
}
pub unsafe fn definition_id(local: u16, grfid: u32) -> u16 {
    unsafe { crate::override_manager::get(*MANAGER.get(), local, grfid) }
}
unsafe extern "C" fn valid_id(_: *mut c_void, id: u16) -> u8 {
    u8::from(id != 0xff)
}
unsafe fn set_entity(spec: *mut Spec) {
    unsafe {
        let manager = *MANAGER.get();
        let local = (*spec).grf_prop.local_id;
        let grfid = (*spec).grf_prop.grfid;
        let mut id = crate::override_manager::get(manager, local, grfid);
        if id == (*manager).invalid {
            id = crate::override_manager::openttd_rust_override_allocate(
                manager,
                local,
                grfid,
                0,
                ptr::null_mut(),
                id,
                valid_id,
            );
        }
        if id == (*manager).invalid {
            grf_msg!(
                1,
                "Object.SetEntitySpec: Too many objects allocated. Ignoring."
            );
            return;
        }
        if usize::from(id) >= (*SPECS.get()).len() {
            (*SPECS.get()).resize_with(usize::from(id) + 1, Spec::new);
        }
        // Native move assignment retains scalar fields in the source and transfers
        // only the owned vector; the file clears the moved-from definition later.
        let badges = std::mem::take(&mut (*spec).badges);
        let mut moved = (*spec).clone();
        moved.badges = badges;
        (&mut *SPECS.get())[usize::from(id)] = moved;
    }
}
pub unsafe fn bind() {
    for id in 0..count() {
        let spec = unsafe { (*SPECS.get()).as_mut_ptr().add(id) };
        if unsafe { (*spec).enabled() && (*spec).class_index != u16::MAX } {
            unsafe { crate::newgrf_class::object::assign(spec) };
        }
    }
}
pub unsafe fn finalise(s: &crate::newgrf_loading::Services) {
    let mut index = 0;
    loop {
        let file = (s.shared().grf_file_at_or_null)(index);
        if file.is_null() {
            break;
        }
        index += 1;
        let specs = unsafe { ((*s.bindings).object_specs)(file) };
        let count = unsafe { (*specs).len() };
        for id in 0..count {
            let spec = unsafe { file_spec(specs, id as u16) };
            if !spec.is_null()
                && unsafe { !(*spec).grf_prop.grffile.is_null() && (*spec).enabled() }
            {
                unsafe { set_entity(spec) };
            }
        }
        unsafe {
            (*specs).edit(|v| {
                v.clear();
                v.shrink_to_fit();
            });
        };
    }
    unsafe { bind() };
}
