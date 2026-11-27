/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared typed `NewGRF` specification leaves (`spec_services_ffi.h`), one native
//! table per spec type, reached through `Services::newgrf_specs`. Specs stay
//! native and live for each synchronous call; every table is process-lifetime.
use crate::house::HouseSpec;
use crate::newgrf_bytereader::BadgeList;
use crate::station_cmd::{AirportSpec, RailSpec, RoadSpec};
use crate::{animated_tile::AirportSpec as AirportTileSpec, objects::Spec as ObjectSpec};
use std::ffi::c_void;

crate::abi::shared_records! {
    fn abi_record;
    /// Borrowed badge IDs; native storage remains live for one synchronous leaf.
    #[derive(Clone, Copy)]
    "OpenTTDBadgeSpan" ["probe"] => pub struct BadgeSpan {
        pub data: *const u32 => "const BadgeID *",
        pub len: usize,
    }
}
impl BadgeSpan {
    /// `GetBadgeVariableResult(*ro.grffile, badges, parameter)`, which reads the
    /// resolver's file.
    pub(crate) fn get_badge_variable_result(
        self,
        shared: &crate::services::Services,
        resolver: *const c_void,
        parameter: u32,
    ) -> u32 {
        (shared.resolver_badges)(resolver, self.data, self.len, parameter)
    }
}

/// Leaves of specs carrying `StandardGRFFileProps`.
#[repr(C)]
pub struct StandardServices<S> {
    /// `grf_prop.HasSpriteGroups()`.
    pub has_sprite_groups: extern "C" fn(*const S) -> bool,
    /// `grf_prop.SetSpriteGroup` for default/purchase bindings.
    pub set_sprite_group: extern "C" fn(*mut S, u8, *const c_void),
    /// `grf_prop.GetSpriteGroup(entity_exists)`.
    pub get_sprite_group: extern "C" fn(*const S, bool) -> *const c_void,
}

/// Leaves of specs carrying `SubstituteGRFFileProps` and an `enabled` flag.
#[repr(C)]
pub struct SubstituteServices<S> {
    /// `enabled`.
    pub enabled: extern "C" fn(*const S) -> bool,
    /// `grf_prop.override_id`.
    pub override_id: extern "C" fn(*const S) -> u16,
    /// Store `enabled`.
    pub set_enabled: extern "C" fn(*mut S, bool),
    /// Store `grf_prop.override_id`.
    pub set_override_id: extern "C" fn(*mut S, u16),
    /// `grf_prop.subst_id`.
    pub subst_id: extern "C" fn(*const S) -> u16,
}

/// `AnimationInfo` stores of specs carrying one; `T` is the trigger storage type.
#[repr(C)]
#[allow(
    clippy::struct_field_names,
    reason = "each store takes its member's name: `set_frames` for `animation.frames`"
)]
pub struct AnimationServices<S, T> {
    /// Store `animation.frames`.
    pub set_frames: extern "C" fn(*mut S, u8),
    /// Store `animation.speed`.
    pub set_speed: extern "C" fn(*mut S, u8),
    /// Store `animation.status`.
    pub set_status: extern "C" fn(*mut S, u8),
    /// Store `animation.triggers`.
    pub set_triggers: extern "C" fn(*mut S, T),
}

/// Leaves of every spec. A capability table (`animation`, `standard`, `substitute`) is
/// null for a spec type without that capability, and a caller names only spec types
/// that have it; every table is process-lifetime.
#[repr(C)]
pub struct SpecServices<S, T> {
    animation: *const AnimationServices<S, T>,
    /// `&badges`, the spec's native badge list, which `assign_badge_list` assigns.
    pub badge_list: extern "C" fn(*mut S) -> *mut BadgeList,
    /// `badges`: borrow the native badge storage without retaining the spec.
    pub badges: extern "C" fn(*const S) -> BadgeSpan,
    /// `grf_prop.grffile`.
    pub grffile: extern "C" fn(*const S) -> *const crate::newgrf::GRFFile,
    /// `grf_prop.grfid`.
    pub grfid: extern "C" fn(*const S) -> u32,
    /// `grf_prop.HasGrfFile()`.
    pub has_grf_file: extern "C" fn(*const S) -> bool,
    /// `grf_prop.local_id`.
    pub local_id: extern "C" fn(*const S) -> u16,
    /// Writable display name, or null for specs with no display name.
    pub name: extern "C" fn(*mut S) -> *mut u32,
    /// `grf_prop.SetGRFFile`.
    pub set_grf_file: extern "C" fn(*mut S, *const crate::newgrf::GRFFile),
    /// Store `grf_prop.local_id`.
    pub set_local_id: extern "C" fn(*mut S, u16),
    standard: *const StandardServices<S>,
    substitute: *const SubstituteServices<S>,
}

#[allow(
    unsafe_code,
    reason = "the capability tables, under the type's contract"
)]
impl<S, T> SpecServices<S, T> {
    /// Animation stores, of a spec with `AnimationInfo`.
    pub fn animation(&self) -> &AnimationServices<S, T> {
        // SAFETY: the type's contract: the caller names a spec type with animation.
        unsafe { &*self.animation }
    }
    /// Sprite-group leaves, of a spec with `StandardGRFFileProps`.
    pub fn standard(&self) -> &StandardServices<S> {
        // SAFETY: the type's contract: the caller names a spec type with them.
        unsafe { &*self.standard }
    }
    /// Substitute leaves, of a spec with `SubstituteGRFFileProps`.
    pub fn substitute(&self) -> &SubstituteServices<S> {
        // SAFETY: the type's contract: the caller names a spec type with them.
        unsafe { &*self.substitute }
    }
}

/// One table per native spec type.
#[repr(C)]
pub struct Specs {
    airport: *const SpecServices<AirportSpec, u8>,
    airport_tile: *const SpecServices<AirportTileSpec, u8>,
    house: *const SpecServices<HouseSpec, u8>,
    object: *const SpecServices<ObjectSpec, u16>,
    road_stop: *const SpecServices<RoadSpec, u16>,
    station: *const SpecServices<RailSpec, u16>,
}

macro_rules! spec_tables {
    ($($field:ident: $spec:ty, $triggers:ty;)*) => {
        #[allow(unsafe_code, reason = "the process-lifetime tables the native table names")]
        impl Specs {
            $(
                #[doc = concat!("`", stringify!($field), "` spec leaves.")]
                pub fn $field(&self) -> &SpecServices<$spec, $triggers> {
                    // SAFETY: `_rust_newgrf_specs` fills every entry with a process-lifetime table.
                    unsafe { &*self.$field }
                }
            )*
        }
    };
}
spec_tables! {
    airport: AirportSpec, u8;
    airport_tile: AirportTileSpec, u8;
    house: HouseSpec, u8;
    object: ObjectSpec, u16;
    road_stop: RoadSpec, u16;
    station: RailSpec, u16;
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNewGRFSpecs" ["probe"] => Specs { airport, airport_tile, house, object, road_stop, station };
    "OpenTTDAirportSpecServices" ["probe"] => SpecServices<AirportSpec, u8> { animation, badge_list, badges, grffile, grfid, has_grf_file, local_id, name, set_grf_file, set_local_id, standard, substitute };
    "OpenTTDAirportTileSpecServices" ["probe"] => SpecServices<AirportTileSpec, u8> { animation, badge_list, badges, grffile, grfid, has_grf_file, local_id, name, set_grf_file, set_local_id, standard, substitute };
    "OpenTTDHouseSpecServices" ["probe"] => SpecServices<HouseSpec, u8> { animation, badge_list, badges, grffile, grfid, has_grf_file, local_id, name, set_grf_file, set_local_id, standard, substitute };
    "OpenTTDObjectSpecServices" ["probe"] => SpecServices<ObjectSpec, u16> { animation, badge_list, badges, grffile, grfid, has_grf_file, local_id, name, set_grf_file, set_local_id, standard, substitute };
    "OpenTTDRoadStopSpecServices" ["probe"] => SpecServices<RoadSpec, u16> { animation, badge_list, badges, grffile, grfid, has_grf_file, local_id, name, set_grf_file, set_local_id, standard, substitute };
    "OpenTTDStationSpecServices" ["probe"] => SpecServices<RailSpec, u16> { animation, badge_list, badges, grffile, grfid, has_grf_file, local_id, name, set_grf_file, set_local_id, standard, substitute };
    "OpenTTDAirportTileSpecAnimation" ["probe"] => AnimationServices<AirportTileSpec, u8> { set_frames, set_speed, set_status, set_triggers };
    "OpenTTDRoadStopSpecAnimation" ["probe"] => AnimationServices<RoadSpec, u16> { set_frames, set_speed, set_status, set_triggers };
    "OpenTTDStationSpecAnimation" ["probe"] => AnimationServices<RailSpec, u16> { set_frames, set_speed, set_status, set_triggers };
    "OpenTTDAirportSpecStandard" ["probe"] => StandardServices<AirportSpec> { has_sprite_groups, set_sprite_group, get_sprite_group };
    "OpenTTDAirportTileSpecStandard" ["probe"] => StandardServices<AirportTileSpec> { has_sprite_groups, set_sprite_group, get_sprite_group };
    "OpenTTDHouseSpecStandard" ["probe"] => StandardServices<HouseSpec> { has_sprite_groups, set_sprite_group, get_sprite_group };
    "OpenTTDObjectSpecStandard" ["probe"] => StandardServices<ObjectSpec> { has_sprite_groups, set_sprite_group, get_sprite_group };
    "OpenTTDAirportSpecSubstitute" ["probe"] => SubstituteServices<AirportSpec> { enabled, override_id, set_enabled, set_override_id, subst_id };
    "OpenTTDAirportTileSpecSubstitute" ["probe"] => SubstituteServices<AirportTileSpec> { enabled, override_id, set_enabled, set_override_id, subst_id };
    "OpenTTDHouseSpecSubstitute" ["probe"] => SubstituteServices<HouseSpec> { enabled, override_id, set_enabled, set_override_id, subst_id };
}
