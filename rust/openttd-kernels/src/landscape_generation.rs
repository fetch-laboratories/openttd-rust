/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Complete landscape-generation control. Native leaves own shared map writes.
//! Progress cancellation returns through native frames; no map reference is kept.
#![allow(unsafe_code, missing_docs, clippy::cast_possible_truncation)]
use crate::clear_map::{CLEAR_GRASS, CLEAR_ROCKS};
use crate::ground::ClearServices;
use crate::services::{ClearGroundType, Map, Services};
use crate::slope_func::{SLOPE_FLAT, SLOPE_STEEP, is_slope_with_one_corner_raised};
use crate::tile_type::MAX_TILE_HEIGHT;
use crate::tile_type::{MP_CLEAR, MP_VOID};
use crate::water::WaterServices;
use std::ffi::c_void;
#[repr(C)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub data: *const u8,
}
#[repr(C)]
pub struct Decoder {
    pub context: *mut c_void,
    pub read: extern "C" fn(*mut c_void, *mut Image) -> bool,
}

#[repr(C)]
pub struct GenerationServices {
    pub shared: *const Services,
    pub clear: *const ClearServices,
    pub water: *const WaterServices,
    pub set_height: extern "C" fn(u32, u32),
    pub inner_tile: extern "C" fn(u32) -> bool,
    pub make_void: extern "C" fn(u32),
    pub screen_dirty: extern "C" fn(),
    pub surrounded_by_water: extern "C" fn() -> bool,
    pub set_freeform: extern "C" fn(bool),
    pub clear_bridge_middle: extern "C" fn(u32),
}
struct World<'a> {
    a: &'a GenerationServices,
    s: &'a Services,
    clear: &'a ClearServices,
    map: Map,
    size_y: u32,
}
impl<'a> World<'a> {
    unsafe fn new(a: &'a GenerationServices) -> Self {
        let (s, clear) = unsafe { (&*a.shared, &*a.clear) };
        Self {
            a,
            s,
            clear,
            map: Map {
                log_x: (s.map_log_x)() as u8,
            },
            size_y: (s.map_max_y)() + 1,
        }
    }
    fn initialize(&self, freeform: bool) {
        for y in u32::from(freeform)..self.size_y - 1 {
            for x in u32::from(freeform)..self.map.max_x() {
                let tile = self.map.tile_xy(x, y);
                (self.clear.make_clear)(tile, ClearGroundType(CLEAR_GRASS), 3);
                (self.a.set_height)(tile, 0);
                (self.s.set_tropic_zone)(tile, 0);
                (self.a.clear_bridge_middle)(tile);
            }
        }
        for x in 0..self.map.size_x() {
            (self.a.make_void)(self.map.tile_xy(x, self.size_y - 1));
        }
        for y in 0..self.size_y {
            (self.a.make_void)(self.map.tile_xy(self.map.max_x(), y));
        }
    }
    fn fix_slopes(&self, max_height: u8) {
        // Two source-ordered passes. Heights are re-read at each source observation.
        for reverse in [false, true] {
            for row in 0..self.size_y {
                let y = if reverse { self.size_y - 1 - row } else { row };
                for col in 0..self.map.size_x() {
                    let x = if reverse { self.map.max_x() - col } else { col };
                    let mut current = MAX_TILE_HEIGHT;
                    if (!reverse && x != 0) || (reverse && x != self.map.max_x()) {
                        current = (self.s.tile_height)(
                            self.map.tile_xy(if reverse { x + 1 } else { x - 1 }, y),
                        );
                    }
                    if (!reverse && y != 0) || (reverse && y != self.size_y - 1) {
                        let h = (self.s.tile_height)(
                            self.map.tile_xy(x, if reverse { y + 1 } else { y - 1 }),
                        );
                        if h < current {
                            current = (self.s.tile_height)(
                                self.map.tile_xy(x, if reverse { y + 1 } else { y - 1 }),
                            );
                        }
                    }
                    let tile = self.map.tile_xy(x, y);
                    if (self.s.tile_height)(tile) >= current + 2 {
                        (self.a.set_height)(tile, current + 1);
                        if (self.a.inner_tile)(tile)
                            && self.s.random_range(u32::from(max_height)) <= current
                        {
                            (self.clear.make_clear)(tile, ClearGroundType(CLEAR_ROCKS), 3);
                        }
                    }
                }
            }
        }
    }
}
/// # Safety
/// Tables and callbacks are live for this serialized call; native asserts remain native.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_initialize_landscape(
    a: *const GenerationServices,
    freeform: bool,
) {
    unsafe { World::new(&*a) }.initialize(freeform);
}
/// # Safety
/// Same table contract as initialize; callbacks never unwind across Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_fix_slopes(a: *const GenerationServices, max_height: u8) {
    unsafe { World::new(&*a) }.fix_slopes(max_height);
}

impl World<'_> {
    fn convert_ground_water(&self) {
        let water = unsafe { &*self.a.water };
        for tile in 0..self.map.size_x() * self.size_y {
            let slope_z = (self.s.tile_slope_z)(tile);
            let slope = slope_z.slope.0;
            if (self.s.tile_type)(tile) != MP_CLEAR || slope_z.z != 0 {
                continue;
            }
            if slope == SLOPE_FLAT {
                (water.make_sea)(tile);
            } else if is_slope_with_one_corner_raised(slope) {
                (self.s.make_shore)(tile);
            } else {
                let mask = crate::water::FLOOD_FROM[usize::from(slope & !SLOPE_STEEP)];
                for dir in 0..8 {
                    if mask & (1 << dir) == 0 {
                        continue;
                    }
                    let dest = self.map.add_dir(tile, dir);
                    let other = (self.s.tile_slope)(dest) & !SLOPE_STEEP;
                    if other == SLOPE_FLAT
                        || is_slope_with_one_corner_raised(other)
                        || (self.s.tile_type)(dest) == MP_VOID
                    {
                        (self.s.make_shore)(tile);
                        break;
                    }
                }
            }
        }
    }
    fn flat_empty_world(&self, height: u8, freeform: bool, max_height: u8) {
        let edge = if freeform { 0 } else { 2 };
        for y in edge..self.size_y - edge {
            for x in edge..self.map.size_x() - edge {
                (self.a.set_height)(self.map.tile_xy(x, y), u32::from(height));
            }
        }
        self.fix_slopes(max_height);
        (self.a.screen_dirty)();
    }
    fn greyscale(&self, image: &Image, rotation: u8, freeform: bool, heightmap_height: u8) {
        const DIV: u32 = 16384;
        let (width, height) = if rotation == 0 {
            (self.map.size_x(), self.size_y)
        } else {
            (self.size_y, self.map.size_x())
        };
        let (scale, row_pad, col_pad) = if image.width * DIV / image.height > width * DIV / height {
            let scale = width * DIV / image.width;
            (scale, (1 + height - image.height * scale / DIV) / 2, 0)
        } else {
            let scale = height * DIV / image.height;
            (scale, 0, (1 + width - image.width * scale / DIV) / 2)
        };
        if freeform {
            self.void_top_edges();
        }
        for row in 0..height {
            for col in 0..width {
                let tile = if rotation == 0 {
                    self.map.tile_xy(col, row)
                } else {
                    self.map.tile_xy(row, col)
                };
                if (!freeform && (self.s.distance_from_edge)(tile) <= 1)
                    || row < row_pad
                    || row >= height - row_pad - u32::from(!freeform)
                    || col < col_pad
                    || col >= width - col_pad - u32::from(!freeform)
                {
                    (self.a.set_height)(tile, 0);
                } else {
                    let img_row = (row - row_pad) * DIV / scale;
                    let img_col = if rotation == 0 {
                        (width - 1 - col - col_pad) * DIV / scale
                    } else {
                        (col - col_pad) * DIV / scale
                    };
                    // Native decoder's dimensions and this source resizing establish bounds.
                    let mut level = u32::from(unsafe {
                        *image.data.add((img_row * image.width + img_col) as usize)
                    });
                    if level > 0 {
                        level = 1 + (level - 1) * u32::from(heightmap_height) / 255;
                    }
                    (self.a.set_height)(tile, level);
                }
                if (self.a.inner_tile)(tile) {
                    (self.clear.make_clear)(tile, ClearGroundType(CLEAR_GRASS), 3);
                }
            }
        }
    }
    fn void_top_edges(&self) {
        for x in 0..self.map.size_x() {
            (self.a.make_void)(self.map.tile_xy(x, 0));
        }
        for y in 0..self.size_y {
            (self.a.make_void)(self.map.tile_xy(0, y));
        }
    }
    fn load_heightmap(
        &self,
        decoder: &Decoder,
        rotation: u8,
        freeform: bool,
        heightmap_height: u8,
        max_height: u8,
    ) -> bool {
        let mut image = Image {
            width: 0,
            height: 0,
            data: std::ptr::null(),
        };
        if !(decoder.read)(decoder.context, &raw mut image) {
            return false;
        }
        self.greyscale(&image, rotation, freeform, heightmap_height);
        self.fix_slopes(max_height);
        (self.a.set_freeform)(!(self.a.surrounded_by_water)());
        (self.a.screen_dirty)();
        true
    }
}
/// # Safety
/// Live serialized native map and immutable typed service tables.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_convert_ground_water(a: *const GenerationServices) {
    unsafe { World::new(&*a) }.convert_ground_water();
}
/// # Safety
/// Same native map/service contract as initialize.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_flat_empty_world(
    a: *const GenerationServices,
    height: u8,
    freeform: bool,
    max_height: u8,
) {
    unsafe { World::new(&*a) }.flat_empty_world(height, freeform, max_height);
}
/// # Safety
/// Decoder output is live and initialized until this call returns, and cannot
/// alias map storage. Decode failures retain the source's early false return.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_load_heightmap(
    a: *const GenerationServices,
    decoder: *const Decoder,
    rotation: u8,
    freeform: bool,
    heightmap_height: u8,
    max_height: u8,
) -> bool {
    unsafe { World::new(&*a) }.load_heightmap(
        unsafe { &*decoder },
        rotation,
        freeform,
        heightmap_height,
        max_height,
    )
}
