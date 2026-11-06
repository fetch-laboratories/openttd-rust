/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Engine binding storage. GRF files and sprite groups are borrowed until reset; engines
//! are rebuilt before those files are freed. Serialized entries end all views before
//! logging, mutation, callbacks and drop. Nested engine lists retain source order.
#![allow(unsafe_code, missing_docs)]
use super::Lifecycle;
use crate::debug::grf_msg;
use crate::engine_type::EngineID;
use crate::industrytype::NativeVec;
use crate::raw_field::write_field;
use crate::services::Services;
use std::ffi::c_void;
crate::abi::shared_records! {
    fn abi_record;
    /// Sorted cargo key and borrowed sprite group.
    #[derive(Clone, Copy)]
    "OpenTTDEngineCargoBinding" ["engine"] => pub struct CargoBinding {
        pub cargo: u8 => "CargoType",
        pub group: *const c_void => "const SpriteGroup *",
    }
    /// Engine-local GRF identity and sorted bindings.
    #[derive(Clone, Default)]
    "OpenTTDEngineGRFProps" ["engine"] => pub struct GrfProps {
        pub local_id: u16,
        pub grfid: u32,
        pub grffile: *const c_void => "const GRFFile *",
        pub spritegroups: NativeVec<CargoBinding> => "RustVector<OpenTTDEngineCargoBinding>",
    }
    /// Ordered wagon match and its independently owned engine IDs.
    #[derive(Clone)]
    "OpenTTDEngineWagonOverride" ["engine"] => pub struct WagonOverride {
        pub engines: NativeVec<EngineID> => "RustVector<EngineID>",
        pub cargo: u8 => "CargoType",
        pub group: *const c_void => "const SpriteGroup *",
    }
    /// Engine-owned containers; native readers borrow the generated layout.
    #[derive(Clone, Default)]
    "OpenTTDEngineBindings" ["engine"] => pub struct Bindings {
        pub grf_prop: GrfProps,
        pub overrides: NativeVec<WagonOverride> => "RustVector<OpenTTDEngineWagonOverride>",
        pub badges: NativeVec<u32> => "RustVector<BadgeID>",
    }
}
impl Drop for Bindings {
    fn drop(&mut self) {
        // Engine's native members retire in reverse declaration order.
        drop(std::mem::take(&mut self.badges));
        drop(std::mem::take(&mut self.overrides));
        drop(std::mem::take(&mut self.grf_prop.spritegroups));
    }
}
pub(crate) unsafe fn sprite_group(engine: *const Lifecycle, cargo: u8) -> *const c_void {
    let groups = unsafe { &(*engine).bindings.grf_prop.spritegroups }.as_slice();
    let index = groups.partition_point(|entry| entry.cargo < cargo);
    groups
        .get(index)
        .filter(|entry| entry.cargo == cargo)
        .map_or(std::ptr::null(), |entry| entry.group)
}
pub(crate) unsafe fn set_custom_sprites(
    engine: *const Lifecycle,
    id: EngineID,
    cargo: u8,
    group: *const c_void,
) {
    if !unsafe { sprite_group(engine, cargo) }.is_null() {
        grf_msg!(
            6,
            "SetCustomEngineSprites: engine {} cargo {} already has group -- replacing",
            id.0,
            cargo
        );
    }
    unsafe {
        set_sprite_group(
            &raw mut (*engine.cast_mut()).bindings.grf_prop,
            cargo,
            group,
        );
    }
}
pub(crate) unsafe fn set_sprite_group(props: *mut GrfProps, cargo: u8, group: *const c_void) {
    let destination = unsafe { &mut (*props).spritegroups };
    let mut groups = std::mem::take(destination).into_vec();
    let index = groups.partition_point(|entry| entry.cargo < cargo);
    if index == groups.len() || groups[index].cargo != cargo {
        groups.insert(index, CargoBinding { cargo, group });
    } else {
        groups[index].group = group;
    }
    *destination = NativeVec::from_vec(groups);
}
pub(crate) unsafe fn set_wagon_override(
    shared: &Services,
    engine: *const Lifecycle,
    cargo: u8,
    group: *const c_void,
    engines: &[EngineID],
) {
    unsafe {
        append_override(
            shared,
            &raw mut (*engine.cast_mut()).bindings,
            cargo,
            group,
            engines,
        );
    }
}
pub(crate) unsafe fn append_override(
    shared: &Services,
    bindings: *mut Bindings,
    cargo: u8,
    group: *const c_void,
    engines: &[EngineID],
) {
    crate::movement::native_assert(shared, cargo < crate::cargo_type::NUM_CARGO + 2);
    let overrides = unsafe { &mut (*bindings).overrides };
    overrides.push(WagonOverride {
        engines: NativeVec::new(),
        cargo: 0,
        group: std::ptr::null(),
    });
    let entry = unsafe { overrides.as_mut_ptr().add(overrides.len() - 1) };
    write_field!(entry, group, group);
    write_field!(entry, cargo, cargo);
    unsafe {
        (*entry).engines = NativeVec::from_vec(engines.to_vec());
    }
}
pub(crate) unsafe fn wagon_override(
    engine: *const Lifecycle,
    cargo: u8,
    overriding: EngineID,
) -> *const c_void {
    for entry in unsafe { &(*engine).bindings.overrides }.as_slice() {
        if (entry.cargo == cargo || entry.cargo == crate::newgrf_commons::SG_DEFAULT)
            && entry.engines.as_slice().contains(&overriding)
        {
            return entry.group;
        }
    }
    std::ptr::null()
}
pub(crate) unsafe fn set_grf(shared: &Services, engine: *const Lifecycle, file: *const c_void) {
    write_field!(engine.cast_mut(), bindings.grf_prop.grffile, file);
    let id = if file.is_null() {
        0
    } else {
        (shared.grf_file_id)(file)
    };
    write_field!(engine.cast_mut(), bindings.grf_prop.grfid, id);
}

pub(crate) unsafe fn badges(engine: *const Lifecycle) -> crate::newgrf_specs::BadgeSpan {
    unsafe { &(*engine).bindings.badges }.badge_view()
}

pub(crate) unsafe fn append_copyable_badges(
    shared: &Services,
    engine: *const Lifecycle,
    source: crate::newgrf_specs::BadgeSpan,
    feature: u8,
) {
    let destination = unsafe { badges(engine) };
    let mut result = Vec::with_capacity(
        destination
            .len
            .checked_add(source.len)
            .expect("badge capacity"),
    );
    let count = (shared.copyable_badges)(destination, source, feature, result.as_mut_ptr());
    assert!(count <= result.capacity());
    unsafe {
        result.set_len(count);
        (*engine.cast_mut()).bindings.badges = NativeVec::from_vec(result);
    }
}
