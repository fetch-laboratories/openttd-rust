/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared typed NewGRF specification leaves (`spec_services_ffi.h`), one native
//! table per spec type, reached through `Services::newgrf_specs`. Specs stay
//! native and live for each synchronous call; every table is process-lifetime.
#![allow(unsafe_code)]
use crate::newgrf_bytereader::BadgeList;
use crate::newgrf_commons::SubstituteSpecIdentity;
use crate::station_cmd::{AirportSpec, RailSpec, RoadSpec};
use crate::{animated_tile::AirportSpec as AirportTileSpec, objects::Spec as ObjectSpec};
use crate::{industry::SpecHandle as IndustrySpec, industry::TileSpec as IndustryTileSpec};
use std::ffi::c_void;

/// Leaves of specs carrying `StandardGRFFileProps`.
#[repr(C)]
pub struct StandardServices<S> {
    /// `grf_prop.HasSpriteGroups()`.
    pub has_sprite_groups: extern "C" fn(*const S) -> bool,
    /// `grf_prop.GetSpriteGroup(entity_exists)`.
    pub sprite_group: extern "C" fn(*const S, bool) -> *const c_void,
}

/// Leaves of specs carrying `SubstituteGRFFileProps` and an `enabled` flag.
#[repr(C)]
pub struct SubstituteServices<S> {
    /// `enabled`.
    pub enabled: extern "C" fn(*const S) -> bool,
    /// `grf_prop` grfid, local ID and substitute ID.
    pub identity: extern "C" fn(*const S) -> SubstituteSpecIdentity,
    /// `grf_prop.override_id`.
    pub override_id: extern "C" fn(*const S) -> u16,
    /// Store `enabled`.
    pub set_enabled: extern "C" fn(*mut S, bool),
    /// Store `grf_prop.override_id`.
    pub set_override: extern "C" fn(*mut S, u16),
    /// `grf_prop.subst_id`.
    pub substitute: extern "C" fn(*const S) -> u16,
}

/// `AnimationInfo` stores of specs carrying one; `T` is the trigger storage type.
#[repr(C)]
pub struct AnimationServices<S, T> {
    /// Store `animation.frames`.
    pub frames: extern "C" fn(*mut S, u8),
    /// Store `animation.speed`.
    pub speed: extern "C" fn(*mut S, u8),
    /// Store `animation.status`.
    pub status: extern "C" fn(*mut S, u8),
    /// Store `animation.triggers`.
    pub triggers: extern "C" fn(*mut S, T),
}

/// Leaves of every spec; a capability table is null where the spec lacks it.
#[repr(C)]
pub struct SpecServices<S, T> {
    animation: *const AnimationServices<S, T>,
    /// The spec's native badge list.
    pub badge_list: extern "C" fn(*mut S) -> *mut BadgeList,
    /// `GetBadgeVariableResult` for the resolver's GRF file.
    pub badges: extern "C" fn(*const c_void, *const S, u32) -> u32,
    /// `ConvertBooleanCallback` with the spec's GRF file.
    pub boolean_callback: extern "C" fn(*const S, u16, u16) -> bool,
    /// The spec's GRF file's `cargo_map` entry.
    pub cargo_map: extern "C" fn(*const S, u8) -> u8,
    /// `grf_prop.grffile`.
    pub file: extern "C" fn(*const S) -> *const c_void,
    /// `grf_prop.grfid`.
    pub grfid: extern "C" fn(*const S) -> u32,
    /// `grf_prop.HasGrfFile()`.
    pub has_file: extern "C" fn(*const S) -> bool,
    /// `grf_prop.local_id`.
    pub local_id: extern "C" fn(*const S) -> u16,
    /// `grf_prop.SetGRFFile`.
    pub set_file: extern "C" fn(*mut S, *const c_void),
    /// Store `grf_prop.local_id`.
    pub set_local_id: extern "C" fn(*mut S, u16),
    standard: *const StandardServices<S>,
    substitute: *const SubstituteServices<S>,
}

impl<S, T> SpecServices<S, T> {
    /// Animation stores; only specs with `AnimationInfo` reach this.
    pub fn animation(&self) -> &AnimationServices<S, T> {
        debug_assert!(!self.animation.is_null());
        // SAFETY: the native table is process-lifetime; callers name animated specs only.
        unsafe { &*self.animation }
    }
    /// Sprite-group leaves; only specs with `StandardGRFFileProps` reach this.
    pub fn standard(&self) -> &StandardServices<S> {
        debug_assert!(!self.standard.is_null());
        // SAFETY: as for `animation`, for standard specs.
        unsafe { &*self.standard }
    }
    /// Substitute leaves; only specs with `SubstituteGRFFileProps` reach this.
    pub fn substitute(&self) -> &SubstituteServices<S> {
        debug_assert!(!self.substitute.is_null());
        // SAFETY: as for `animation`, for substitute specs.
        unsafe { &*self.substitute }
    }
}

/// One table per native spec type.
#[repr(C)]
pub struct Specs {
    airport: *const SpecServices<AirportSpec, u8>,
    airport_tile: *const SpecServices<AirportTileSpec, u8>,
    industry: *const SpecServices<IndustrySpec, u8>,
    industry_tile: *const SpecServices<IndustryTileSpec, u8>,
    object: *const SpecServices<ObjectSpec, u16>,
    road_stop: *const SpecServices<RoadSpec, u16>,
    station: *const SpecServices<RailSpec, u16>,
}

macro_rules! spec_tables {
    ($($field:ident: $spec:ty, $triggers:ty;)*) => {
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
    industry: IndustrySpec, u8;
    industry_tile: IndustryTileSpec, u8;
    object: ObjectSpec, u16;
    road_stop: RoadSpec, u16;
    station: RailSpec, u16;
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDNewGRFSpecs" ["probe"] => Specs { airport, airport_tile, industry, industry_tile, object, road_stop, station };
    "OpenTTDAirportSpecServices" ["probe"] => SpecServices<AirportSpec, u8> { animation, badge_list, badges, boolean_callback, cargo_map, file, grfid, has_file, local_id, set_file, set_local_id, standard, substitute };
    "OpenTTDAirportTileSpecServices" ["probe"] => SpecServices<AirportTileSpec, u8> { animation, badge_list, badges, boolean_callback, cargo_map, file, grfid, has_file, local_id, set_file, set_local_id, standard, substitute };
    "OpenTTDIndustrySpecServices" ["probe"] => SpecServices<IndustrySpec, u8> { animation, badge_list, badges, boolean_callback, cargo_map, file, grfid, has_file, local_id, set_file, set_local_id, standard, substitute };
    "OpenTTDIndustryTileSpecServices" ["probe"] => SpecServices<IndustryTileSpec, u8> { animation, badge_list, badges, boolean_callback, cargo_map, file, grfid, has_file, local_id, set_file, set_local_id, standard, substitute };
    "OpenTTDObjectSpecServices" ["probe"] => SpecServices<ObjectSpec, u16> { animation, badge_list, badges, boolean_callback, cargo_map, file, grfid, has_file, local_id, set_file, set_local_id, standard, substitute };
    "OpenTTDRoadStopSpecServices" ["probe"] => SpecServices<RoadSpec, u16> { animation, badge_list, badges, boolean_callback, cargo_map, file, grfid, has_file, local_id, set_file, set_local_id, standard, substitute };
    "OpenTTDStationSpecServices" ["probe"] => SpecServices<RailSpec, u16> { animation, badge_list, badges, boolean_callback, cargo_map, file, grfid, has_file, local_id, set_file, set_local_id, standard, substitute };
    "OpenTTDAirportTileSpecAnimation" ["probe"] => AnimationServices<AirportTileSpec, u8> { frames, speed, status, triggers };
    "OpenTTDIndustryTileSpecAnimation" ["probe"] => AnimationServices<IndustryTileSpec, u8> { frames, speed, status, triggers };
    "OpenTTDRoadStopSpecAnimation" ["probe"] => AnimationServices<RoadSpec, u16> { frames, speed, status, triggers };
    "OpenTTDStationSpecAnimation" ["probe"] => AnimationServices<RailSpec, u16> { frames, speed, status, triggers };
    "OpenTTDAirportSpecStandard" ["probe"] => StandardServices<AirportSpec> { has_sprite_groups, sprite_group };
    "OpenTTDAirportTileSpecStandard" ["probe"] => StandardServices<AirportTileSpec> { has_sprite_groups, sprite_group };
    "OpenTTDIndustrySpecStandard" ["probe"] => StandardServices<IndustrySpec> { has_sprite_groups, sprite_group };
    "OpenTTDIndustryTileSpecStandard" ["probe"] => StandardServices<IndustryTileSpec> { has_sprite_groups, sprite_group };
    "OpenTTDObjectSpecStandard" ["probe"] => StandardServices<ObjectSpec> { has_sprite_groups, sprite_group };
    "OpenTTDAirportSpecSubstitute" ["probe"] => SubstituteServices<AirportSpec> { enabled, identity, override_id, set_enabled, set_override, substitute };
    "OpenTTDAirportTileSpecSubstitute" ["probe"] => SubstituteServices<AirportTileSpec> { enabled, identity, override_id, set_enabled, set_override, substitute };
    "OpenTTDIndustrySpecSubstitute" ["probe"] => SubstituteServices<IndustrySpec> { enabled, identity, override_id, set_enabled, set_override, substitute };
    "OpenTTDIndustryTileSpecSubstitute" ["probe"] => SubstituteServices<IndustryTileSpec> { enabled, identity, override_id, set_enabled, set_override, substitute };
}
