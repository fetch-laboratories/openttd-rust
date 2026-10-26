/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Typed native storage operations for the industry property interpreter.
use super::super::{SpecHandle, TileSpec};
use std::ffi::c_void;
#[repr(C)]
#[derive(Clone, Copy)]
pub struct LayoutTile {
    pub ti: crate::map_type::TileIndexDiffC,
    pub gfx: u16,
}
#[repr(C)]
pub struct LayoutView {
    pub data: *const LayoutTile,
    pub size: usize,
}
/// Rust owns each local layout allocation; native installation borrows data only until return.
#[repr(C)]
pub struct LayoutBuffer {
    pub data: *mut LayoutTile,
    pub size: usize,
    pub capacity: usize,
}
impl LayoutBuffer {
    pub fn new(tiles: Vec<LayoutTile>) -> Self {
        let mut tiles = std::mem::ManuallyDrop::new(tiles);
        Self {
            data: tiles.as_mut_ptr(),
            size: tiles.len(),
            capacity: tiles.capacity(),
        }
    }
}
impl Drop for LayoutBuffer {
    fn drop(&mut self) {
        unsafe {
            drop(Vec::from_raw_parts(self.data, self.size, self.capacity));
        }
    }
}

#[repr(C)]
pub struct Services {
    pub resize_industries: unsafe extern "C" fn(*mut c_void, usize),
    pub resize_tiles: unsafe extern "C" fn(*mut c_void, usize),
    pub create_industry: unsafe extern "C" fn(*mut c_void, u16, u8) -> *mut SpecHandle,
    pub create_tile: unsafe extern "C" fn(*mut c_void, u16, u8) -> *mut TileSpec,
    pub industry_identity: unsafe extern "C" fn(*mut SpecHandle, *mut c_void, u16, u8),
    pub tile_identity: unsafe extern "C" fn(*mut TileSpec, *mut c_void, u16, u8),
    pub clear_placement_check: unsafe extern "C" fn(*mut SpecHandle),
    pub clear_animation: unsafe extern "C" fn(*mut TileSpec),
    pub callbacks: unsafe extern "C" fn(*const SpecHandle) -> u16,
    pub set_rate: unsafe extern "C" fn(*mut SpecHandle, usize, u8),
    pub set_multiplier: unsafe extern "C" fn(*mut SpecHandle, usize, usize, u16),
    pub set_conflicting: unsafe extern "C" fn(*mut SpecHandle, usize, u8),
    pub set_creation: unsafe extern "C" fn(*mut SpecHandle, u8, u8),
    pub set_ingame: unsafe extern "C" fn(*mut SpecHandle, u8, u8),
    pub invalidate_produced: unsafe extern "C" fn(*mut SpecHandle, usize),
    pub invalidate_accepted: unsafe extern "C" fn(*mut SpecHandle, usize),
    pub invalidate_tile_accepted: unsafe extern "C" fn(*mut TileSpec, usize),
    pub set_acceptance: unsafe extern "C" fn(*mut TileSpec, usize, i8),
    pub set_sounds: unsafe extern "C" fn(*mut SpecHandle, *const u8, usize),
    pub clear_station_name: unsafe extern "C" fn(*mut SpecHandle),
    pub set_life: unsafe extern "C" fn(*mut SpecHandle, u8),
    pub set_cost: unsafe extern "C" fn(*mut SpecHandle, u8),
    pub set_minimal_cargo: unsafe extern "C" fn(*mut SpecHandle, u8),
    pub set_map_colour: unsafe extern "C" fn(*mut SpecHandle, u8),
    pub set_behaviour: unsafe extern "C" fn(*mut SpecHandle, u32),
    pub set_prospecting: unsafe extern "C" fn(*mut SpecHandle, u32),
    pub set_removal_cost: unsafe extern "C" fn(*mut SpecHandle, u32),
    pub set_callbacks: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub set_tile_slopes: unsafe extern "C" fn(*mut TileSpec, u8),
    pub set_tile_callbacks: unsafe extern "C" fn(*mut TileSpec, u8),
    pub set_animation_frames: unsafe extern "C" fn(*mut TileSpec, u8),
    pub set_animation_status: unsafe extern "C" fn(*mut TileSpec, u8),
    pub set_animation_speed: unsafe extern "C" fn(*mut TileSpec, u8),
    pub set_animation_triggers: unsafe extern "C" fn(*mut TileSpec, u8),
    pub set_tile_flags: unsafe extern "C" fn(*mut TileSpec, u8),
    pub map_closure_text: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub map_production_up_text: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub map_production_down_text: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub map_new_industry_text: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub map_name: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub map_station_name: unsafe extern "C" fn(*mut SpecHandle, u16),
    pub original_layout_count: unsafe extern "C" fn(u8) -> usize,
    pub original_layout: unsafe extern "C" fn(u8, u8) -> LayoutView,
    pub install_layouts: unsafe extern "C" fn(*mut SpecHandle, *const LayoutBuffer, usize),
    pub too_many_tiles: unsafe extern "C" fn(u32),
    pub too_many_industries: unsafe extern "C" fn(u32),
    pub tile_substitute_invalid: unsafe extern "C" fn(u8, u32),
    pub tile_override_invalid: unsafe extern "C" fn(u8, u32),
    pub industry_substitute_invalid: unsafe extern "C" fn(u8, u32),
    pub industry_override_invalid: unsafe extern "C" fn(u8, u32),
    pub layout_size_mismatch: unsafe extern "C" fn(u32),
    pub layout_import_type: unsafe extern "C" fn(u32),
    pub layout_import_index: unsafe extern "C" fn(u32),
    pub layout_tile_undefined: unsafe extern "C" fn(u16, u32),
    pub layout_invalid: unsafe extern "C" fn(u32),
    pub list_too_long: unsafe extern "C" fn(i32),
    pub invalid_import: unsafe extern "C" fn(),
    pub cargo_translation: unsafe extern "C" fn(u8, *const c_void) -> u8,
    pub industry_badges:
        unsafe extern "C" fn(*mut SpecHandle) -> *mut crate::newgrf_bytereader::BadgeList,
    pub tile_badges:
        unsafe extern "C" fn(*mut TileSpec) -> *mut crate::newgrf_bytereader::BadgeList,
}
crate::abi::layouts! {
    fn abi_layout;
    "IndustryTileLayoutTile" ["industry"] => LayoutTile { ti, gfx };
    "OpenTTDIndustryDefinitionLayout" ["industry"] => LayoutView { data, size };
    "OpenTTDIndustryDefinitionLayoutBuffer" ["industry"] => LayoutBuffer { data, size, capacity };
    "OpenTTDIndustryPropertyServices" ["industry"] => Services {
        resize_industries, resize_tiles, create_industry, create_tile, industry_identity, tile_identity, clear_placement_check, clear_animation, callbacks, set_rate, set_multiplier, set_conflicting, set_creation, set_ingame, invalidate_produced, invalidate_accepted, invalidate_tile_accepted, set_acceptance, set_sounds, clear_station_name, set_life, set_cost, set_minimal_cargo, set_map_colour, set_behaviour, set_prospecting, set_removal_cost, set_callbacks, set_tile_slopes, set_tile_callbacks, set_animation_frames, set_animation_status, set_animation_speed, set_animation_triggers, set_tile_flags, map_closure_text, map_production_up_text, map_production_down_text, map_new_industry_text, map_name, map_station_name, original_layout_count, original_layout, install_layouts,
        too_many_tiles, too_many_industries, tile_substitute_invalid, tile_override_invalid, industry_substitute_invalid, industry_override_invalid, layout_size_mismatch, layout_import_type, layout_import_index, layout_tile_undefined, layout_invalid, list_too_long, invalid_import, cargo_translation, industry_badges, tile_badges
    };
}
