/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Complete landscape-generation control. Native leaves own shared map writes.
//! Progress cancellation returns through native frames; no map reference is kept.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::similar_names
)]
use crate::clear_map::{CLEAR_DESERT, CLEAR_ROUGH};
use crate::clear_map::{CLEAR_GRASS, CLEAR_ROCKS};
use crate::command_type::DoCommandFlag as F;
use crate::direction_func::{change_diag_dir, diag_dir_to_axis, reverse_diag_dir};
use crate::direction_type::AXIS_Y;
use crate::direction_type::{
    DIAGDIRDIFF_90LEFT, DIAGDIRDIFF_90RIGHT, DIAGDIRDIFF_END, DIAGDIRDIFF_REVERSE, DIAGDIRDIFF_SAME,
};
use crate::genworld::{
    CUSTOM_SEA_LEVEL_NUMBER_DIFFICULTY, GWM_HEIGHTMAP, GWP_LANDSCAPE, GWP_RIVER, LG_ORIGINAL,
    LG_TERRAGENESIS, Progress,
};
use crate::ground::ClearServices;
use crate::heightmap::{HM_CLOCKWISE, HM_COUNTER_CLOCKWISE};
use crate::landscape_type::{ARCTIC, TROPIC};
use crate::node_hash::{NodeMap, NodeSet};
use crate::services::chance16_i;
use crate::services::{ClearGroundType, Map, Services};
use crate::slope_func::{
    SLOPE_ELEVATED, complement_slope, get_inclined_slope_direction, is_inclined_slope,
    is_slope_with_three_corners_raised, is_steep_slope,
};
use crate::slope_func::{SLOPE_FLAT, SLOPE_STEEP, is_slope_with_one_corner_raised};
use crate::tile_type::{INVALID_TILE, MP_WATER};
use crate::tile_type::{
    MAX_TILE_HEIGHT, TROPICZONE_DESERT, TROPICZONE_NORMAL, TROPICZONE_RAINFOREST,
};
use crate::tile_type::{MP_CLEAR, MP_VOID};
use crate::tree_cmd::TP_NONE;
use crate::water::WaterServices;
use crate::water_type::WATER_CLASS_SEA;
use crate::witness as w;
use std::ffi::c_void;
#[repr(C)]
pub struct Frame {
    _private: [u8; 0],
}
#[repr(C)]
pub struct FlatHeight {
    pub height: i32,
    pub flat: bool,
}
#[repr(C)]
pub struct Template {
    pub width: u32,
    pub height: u32,
    pub data: *const u8,
}
#[repr(C)]
pub struct Loop {
    pub settings: crate::trees::LoopSettings,
    pub services: *const crate::tile_loop::Services,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Settings {
    pub max_search_nodes: i32,
    pub climate: u8,
    pub generator: u8,
    pub rivers: u8,
    pub min_river_length: u8,
    pub route_random: u8,
    pub terrain: u8,
    pub sea: u8,
    pub snow_coverage: u8,
    pub desert_coverage: u8,
    pub tree_placer: u8,
    pub height_limit: u8,
    pub rotation: u8,
    pub heightmap_height: u8,
    pub freeform: bool,
}
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
    pub finish: extern "C" fn(*mut c_void),
}

#[repr(C)]
pub struct GenerationServices {
    pub shared: *const Services,
    pub clear: *const ClearServices,
    pub water: *const WaterServices,
    pub trees: *const crate::trees::TreeServices,
    pub landscape_commands: *const crate::landscape_cmd::LandscapeServices,
    pub water_commands: *const crate::water_cmd::WaterCommandServices,
    pub set_progress: extern "C" fn(*mut Frame, Progress, u32) -> bool,
    pub increase_progress: extern "C" fn(*mut Frame, Progress) -> bool,
    pub begin_loop: extern "C" fn(*mut Frame) -> *const Loop,
    pub end_loop: extern "C" fn(*mut Frame),
    pub set_abort_callback: extern "C" fn(Option<extern "C" fn()>),
    pub tgp_settings: extern "C" fn() -> crate::tgp::Input,
    pub terrain_template: extern "C" fn(u32) -> Template,
    pub flat_height: extern "C" fn(u32) -> FlatHeight,
    pub river: extern "C" fn(u32) -> bool,
    pub set_snow_line: extern "C" fn(u8),
    pub not_reached: extern "C" fn(),
    pub river_debug: extern "C" fn(bool, u32, u32, u32, i32, i32),
    pub assert_condition: extern "C" fn(bool),
    pub sea_construct: extern "C" fn(*mut Frame),
    pub sea_destroy: extern "C" fn(*mut Frame),
    pub sea_insert: extern "C" fn(*mut Frame, u32),
    pub sea_contains: extern "C" fn(*mut Frame, u32) -> bool,
    pub sea_size: extern "C" fn(*mut Frame) -> usize,
    pub sea_first: extern "C" fn(*mut Frame) -> u32,
    pub sea_next: extern "C" fn(*mut Frame) -> u32,
    pub set_height: extern "C" fn(u32, u32),
    pub inner_tile: extern "C" fn(u32) -> bool,
    pub make_void: extern "C" fn(u32),
    pub screen_dirty: extern "C" fn(),
    pub surrounded_by_water: extern "C" fn() -> bool,
    pub set_freeform: extern "C" fn(bool),
    pub clear_bridge_middle: extern "C" fn(u32),
}
pub(crate) struct World<'a> {
    pub(crate) a: &'a GenerationServices,
    pub(crate) s: &'a Services,
    pub(crate) clear: &'a ClearServices,
    pub(crate) map: Map,
    pub(crate) size_y: u32,
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
        w::hit(w::GENERATION_INITIALIZE);
        for y in u32::from(freeform)..self.size_y - 1 {
            for x in u32::from(freeform)..self.map.max_x() {
                let tile = self.map.tile_xy(x, y);
                (self.clear.make_clear)(tile, ClearGroundType(CLEAR_GRASS), 3);
                (self.a.set_height)(tile, 0);
                (self.s.set_tropic_zone)(tile, TROPICZONE_NORMAL);
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
        w::hit(w::GENERATION_FIX_SLOPES);
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
        w::hit(w::GENERATION_WATERING);
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
        w::hit(w::GENERATION_FLAT);
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
        if rotation != HM_COUNTER_CLOCKWISE && rotation != HM_CLOCKWISE {
            (self.a.not_reached)();
        }
        let (width, height) = if rotation == HM_COUNTER_CLOCKWISE {
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
                let tile = if rotation == HM_COUNTER_CLOCKWISE {
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
                    let img_col = if rotation == HM_COUNTER_CLOCKWISE {
                        (width - 1 - col - col_pad) * DIV / scale
                    } else {
                        (col - col_pad) * DIV / scale
                    };
                    (self.a.assert_condition)(img_row < image.height);
                    (self.a.assert_condition)(img_col < image.width);
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
    pub(crate) fn void_top_edges(&self) {
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
            (decoder.finish)(decoder.context);
            w::hit(w::GENERATION_LOAD_FAILURE);
            return false;
        }
        w::hit(w::GENERATION_HEIGHTMAP);
        w::hit(if rotation == HM_COUNTER_CLOCKWISE {
            w::GENERATION_HEIGHTMAP_CCW
        } else {
            w::GENERATION_HEIGHTMAP_CW
        });
        self.greyscale(&image, rotation, freeform, heightmap_height);
        self.fix_slopes(max_height);
        (self.a.set_freeform)(!(self.a.surrounded_by_water)());
        (self.a.screen_dirty)();
        // End the source's local decoded buffer before returning to progress.
        // Every read through image.data ended in greyscale above.
        (decoder.finish)(decoder.context);
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
/// Decoder output is live and initialized until finish, and cannot
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

impl World<'_> {
    fn size(&self) -> u32 {
        self.map.size_x() * self.size_y
    }
    fn flat(&self, tile: u32) -> bool {
        (self.a.flat_height)(tile).flat
    }
    fn water(&self, tile: u32) -> bool {
        (self.s.is_water_tile)(tile)
    }
    fn valid(&self, tile: u32) -> bool {
        (self.s.tile_valid)(tile)
    }
    fn chance(&self, a: u32, b: u32) -> bool {
        chance16_i(a, b, self.s.random())
    }
    fn zone_desert(&self, tile: u32, g: &Settings) -> bool {
        g.climate == TROPIC && (self.s.tropic_zone)(tile) == TROPICZONE_DESERT
    }
    fn river_tile(&self, tile: u32) {
        // Direct call to the existing water owner, no native simulation bounce.
        unsafe {
            crate::water_cmd::openttd_rust_water_make_river(self.a.water_commands, tile);
        }
    }
    fn terraform(&self, tile: u32, slope: u8, up: bool, flags: u16) {
        let a = unsafe { &*self.a.landscape_commands };
        let costs = unsafe { &*a.costs };
        let commands = unsafe { &*a.commands };
        // Independent local native CommandCost; the shared command protocol keeps
        // top-level test/execute, money/rating and recursion behavior unchanged.
        unsafe {
            crate::command::with_cost(costs, |out| {
                crate::command::nested(commands, self.s, out, tile, flags, |f| {
                    crate::landscape_cmd::openttd_rust_landscape_terraform(
                        a, out.0, tile, slope, up, f,
                    )
                });
            });
        }
    }
    fn run_tile_loop(&self, frame: *mut Frame) {
        let input = unsafe { &*(self.a.begin_loop)(frame) };
        // The scheduler acquires its own audited descriptor at this original
        // entry. Generation itself neither reads nor retains that descriptor.
        unsafe {
            crate::tile_loop::openttd_rust_run_tile_loop(
                u32::from(self.map.log_x),
                self.size_y.ilog2(),
                input.settings,
                input.services,
            );
        }
        (self.a.end_loop)(frame);
    }
    fn generate_terrain(&self, terrain: usize, flag: u32, freeform: bool) {
        const COUNTS: [u32; 5] = [10, 22, 33, 37, 4];
        const STARTS: [u32; 5] = [0, 0, 0, 0, 33];
        let r = self.s.random();
        let template = (self.a.terrain_template)(
            (((r >> 24) * COUNTS[terrain]) >> 8)
                + STARTS[terrain]
                + crate::sprites::SPR_MAPGEN_BEGIN,
        );
        let x = r & self.map.max_x();
        let y = (r >> self.map.log_x) & (self.size_y - 1);
        let edge = 1 + u32::from(freeform);
        if x <= edge || y <= edge {
            return;
        }
        let direction = ((r >> 22) & 3) as u8;
        let (mut width, mut height) = (template.width, template.height);
        if diag_dir_to_axis(direction, |ok| (self.a.assert_condition)(ok)) == AXIS_Y {
            std::mem::swap(&mut width, &mut height);
        }
        if flag & 4 != 0 {
            let xw = x * self.size_y;
            let yw = y * self.map.size_x();
            let bias = (self.map.size_x() + self.size_y) * 16;
            let skip = match flag & 3 {
                0 => xw + yw > self.size().wrapping_sub(bias),
                1 => yw < xw + bias,
                2 => xw + yw < self.size() + bias,
                _ => xw < yw + bias,
            };
            if skip {
                return;
            }
        }
        if x + width >= self.map.max_x() || y + height >= self.size_y - 1 {
            return;
        }
        w::hit(
            [
                w::GENERATION_TEMPLATE_NE,
                w::GENERATION_TEMPLATE_SE,
                w::GENERATION_TEMPLATE_SW,
                w::GENERATION_TEMPLATE_NW,
            ][usize::from(direction)],
        );
        let mut tile = self.map.tile_xy(x, y);
        let (outer, inner, step, advance) = match direction {
            0 => (height, width, self.map.diff(1, 0), self.map.diff(0, 1)),
            1 => (width, height, self.map.diff(0, 1), self.map.diff(1, 0)),
            2 => {
                tile = tile.wrapping_add(self.map.diff(width.wrapping_sub(1) as i32, 0));
                (height, width, self.map.diff(-1, 0), self.map.diff(0, 1))
            }
            _ => {
                tile = tile.wrapping_add(self.map.diff(0, height.wrapping_sub(1) as i32));
                (width, height, self.map.diff(0, -1), self.map.diff(1, 0))
            }
        };
        let mut position = 0;
        let mut remaining = outer;
        loop {
            let mut current = tile;
            for _ in 0..inner {
                let level = u32::from(unsafe { *template.data.add(position) } & 15);
                if level >= (self.s.tile_height)(current) {
                    (self.a.set_height)(current, level);
                }
                position += 1;
                current = current.wrapping_add(step);
            }
            tile = tile.wrapping_add(advance);
            // Source uses do/while (--h/--w != 0), including unsigned zero.
            remaining = remaining.wrapping_sub(1);
            if remaining == 0 {
                break;
            }
        }
    }
    /// `CalculateSnowLine`: coastal tiles have no snow sprites.
    fn calculate_snow_line(&self, coverage: u8) {
        (self.a.set_snow_line)(self.coverage_line(u32::from(coverage), 0).max(2) as u8);
    }
    /// `CalculateDesertLine`: coverage is scanned from top to bottom.
    fn calculate_desert_line(&self, coverage: u8) -> u8 {
        self.coverage_line(100_u32.wrapping_sub(u32::from(coverage)), 4) as u8
    }
    fn coverage_line(&self, coverage: u32, edge_multiplier: u32) -> u32 {
        if edge_multiplier != 0 {
            w::hit(w::GENERATION_COVERAGE_EDGE);
        }
        let mut histogram = [0_i32; MAX_TILE_HEIGHT as usize + 1];
        let mut edges = [0_i32; MAX_TILE_HEIGHT as usize + 1];
        for tile in 0..self.size() {
            let height = (self.s.tile_height)(tile);
            histogram[height as usize] += 1;
            if edge_multiplier != 0 {
                for dir in 0..4 {
                    let neighbour = self.map.add_diff_c_wrap(tile, dir, self.size_y);
                    if self.valid(neighbour) && (self.s.tile_height)(neighbour) < height {
                        edges[height as usize] += 1;
                    }
                }
            }
        }
        let land = self.size().wrapping_sub(histogram[0] as u32);
        let mut best_score = land as i32;
        let goal = land.wrapping_mul(coverage).wrapping_div(100) as i32;
        let mut best_h = MAX_TILE_HEIGHT;
        let mut current = 0_i32;
        for height in (1..=MAX_TILE_HEIGHT).rev() {
            current = current.wrapping_add(histogram[height as usize]);
            let mut score = goal.wrapping_sub(current);
            if edge_multiplier != 0 && height > 1 {
                score = score.wrapping_sub(edges[1].wrapping_mul(edge_multiplier as i32));
                score =
                    score.wrapping_sub(edges[height as usize].wrapping_mul(edge_multiplier as i32));
            }
            if score.wrapping_abs() < best_score.wrapping_abs() {
                best_score = score;
                best_h = height;
            }
        }
        best_h
    }
    fn desert_rainforest(&self, frame: *mut Frame, line: u32) -> bool {
        let frequency = self.size() / 4;
        for tile in 0..self.size() {
            if tile % frequency == 0 && !(self.a.increase_progress)(frame, GWP_LANDSCAPE) {
                return false;
            }
            if !self.valid(tile) {
                continue;
            }
            if crate::genland::DESERT_RAINFOREST.iter().all(|&(dx, dy)| {
                let other = self.map.add_offset_c_wrap(tile, dx, dy, self.size_y);
                other == INVALID_TILE
                    || ((self.s.tile_height)(other) < line && (self.s.tile_type)(other) != MP_WATER)
            }) {
                (self.s.set_tropic_zone)(tile, TROPICZONE_DESERT);
                w::hit(w::GENERATION_DESERT);
            }
        }
        for i in 0..crate::landscape::TILE_UPDATE_FREQUENCY {
            if i % 64 == 0 && !(self.a.increase_progress)(frame, GWP_LANDSCAPE) {
                return false;
            }
            self.run_tile_loop(frame);
        }
        for tile in 0..self.size() {
            if tile % frequency == 0 && !(self.a.increase_progress)(frame, GWP_LANDSCAPE) {
                return false;
            }
            if !self.valid(tile) {
                continue;
            }
            if crate::genland::DESERT_RAINFOREST.iter().all(|&(dx, dy)| {
                let other = self.map.add_offset_c_wrap(tile, dx, dy, self.size_y);
                other == INVALID_TILE
                    || (self.s.tile_type)(other) != MP_CLEAR
                    || (self.s.clear_ground)(other).0 != CLEAR_DESERT
            }) {
                (self.s.set_tropic_zone)(tile, TROPICZONE_RAINFOREST);
                w::hit(w::GENERATION_RAINFOREST);
            }
        }
        true
    }
    fn find_spring(&self, tile: u32, g: &Settings) -> bool {
        let flat = (self.a.flat_height)(tile);
        if !flat.flat || self.water(tile) {
            return false;
        }
        if g.climate == TROPIC && (self.s.tropic_zone)(tile) != TROPICZONE_RAINFOREST {
            return false;
        }
        let mut count = 0;
        for dx in -1..=1 {
            for dy in -1..=1 {
                let other = (self.s.tile_add_wrap)(tile, dx, dy);
                if other != INVALID_TILE && (self.s.tile_max_z)(other) > flat.height {
                    count += 1;
                }
            }
        }
        if count < 4 {
            return false;
        }
        for dx in -16..=16 {
            for dy in -16..=16 {
                let other = (self.s.tile_add_wrap)(tile, dx, dy);
                if other != INVALID_TILE && (self.s.tile_max_z)(other) > flat.height + 2 {
                    return false;
                }
            }
        }
        true
    }
    fn valid_terminus_tile(&self, tile: u32, height: u32, g: &Settings) -> bool {
        self.valid(tile)
            && (self.s.tile_height)(tile) == height
            && self.flat(tile)
            && !self.zone_desert(tile, g)
    }
    fn for_spiral(&self, center: u32, diameter: u32, mut visit: impl FnMut(u32)) {
        let mut spiral = crate::spiral::square(
            self.map.tile_x(center),
            self.map.tile_y(center),
            diameter,
            self.map.size_x(),
            self.size_y,
        );
        while !spiral.is_end() {
            visit(self.map.tile_xy(spiral.x, spiral.y));
            spiral = spiral.advance(self.map.size_x(), self.size_y);
        }
    }
    fn lake(&self, center: u32, height: u32, g: &Settings) {
        w::hit(w::GENERATION_LAKE);
        self.river_tile(center);
        let diameter = self.s.random_range(8) + 3;
        for _ in 0..2 {
            self.for_spiral(center, diameter, |tile| {
                if !self.valid_terminus_tile(tile, height, g) {
                    return;
                }
                for dir in 0..4 {
                    if self.water(self.map.add_diag(tile, dir)) {
                        self.river_tile(tile);
                        break;
                    }
                }
            });
        }
    }
    fn wetlands(&self, center: u32, height: u32, length: u32, g: &Settings) {
        w::hit(w::GENERATION_WETLAND);
        self.river_tile(center);
        let diameter = length.max(16);
        let trees = self.chance(1, 2);
        self.for_spiral(center, diameter, |tile| {
            if !self.valid_terminus_tile(tile, height, g) {
                return;
            }
            let radius = diameter / 2;
            if self.map.distance_square(tile, center) > radius * radius && self.chance(3, 4) {
                return;
            }
            if self.chance(1, 3) {
                self.river_tile(tile);
            } else if (self.s.tile_type)(tile) == MP_CLEAR {
                (self.clear.set_ground)(tile, ClearGroundType(CLEAR_ROUGH), 3);
                if trees && g.tree_placer != TP_NONE {
                    w::hit(w::GENERATION_WETLAND_TREES);
                    unsafe {
                        crate::trees::openttd_rust_trees_place(
                            tile,
                            self.s.random(),
                            true,
                            g.climate,
                            self.a.trees,
                            self.a.shared,
                        );
                    }
                }
            }
        });
    }
    fn try_terminus(&self, tile: u32, begin: u32, g: &Settings) -> bool {
        if !self.valid(tile) || tile == begin || self.zone_desert(tile, g) {
            return false;
        }
        let flat = (self.a.flat_height)(tile);
        if !flat.flat {
            return false;
        }
        let begin_height = (self.s.tile_height)(begin) as i32;
        if flat.height != begin_height {
            return false;
        }
        if self.chance(1, 3) {
            self.lake(tile, flat.height as u32, g);
        } else {
            self.wetlands(
                tile,
                flat.height as u32,
                self.map.distance_manhattan(tile, begin),
                g,
            );
        }
        true
    }
    fn flows_down(&self, begin: u32, end: u32) -> bool {
        (self.a.assert_condition)(self.map.distance_manhattan(begin, end) == 1);
        let last = (self.s.tile_slope_z)(end);
        if last.slope.0 != SLOPE_FLAT && !is_inclined_slope(last.slope.0) {
            return false;
        }
        let first = (self.s.tile_slope_z)(begin);
        if last.z > first.z {
            return false;
        }
        if last.slope.0 == first.slope.0 && last.z < first.z {
            return true;
        }
        last.slope.0 == SLOPE_FLAT || first.slope.0 == SLOPE_FLAT
    }
    fn connected_sea(&self, frame: *mut Frame, tile: u32, limit: usize) -> bool {
        if !self.water(tile)
            || (self.clear.water_class)(tile) != WATER_CLASS_SEA
            || !self.flat(tile)
        {
            return false;
        }
        if (self.s.distance_from_edge)(tile) <= 1 {
            w::hit(w::GENERATION_SEA_EDGE);
            return true;
        }
        (self.a.sea_insert)(frame, tile);
        if (self.a.sea_size)(frame) > limit {
            w::hit(w::GENERATION_SEA_CUTOFF);
            return false;
        }
        for dir in 0..4 {
            let other = self.map.add_diag(tile, dir);
            if self.valid(other)
                && !(self.a.sea_contains)(frame, other)
                && self.connected_sea(frame, other, limit)
            {
                return true;
            }
        }
        false
    }
    fn flow_river(
        &self,
        frame: *mut Frame,
        spring: u32,
        begin: u32,
        min_length: u32,
        g: &Settings,
    ) -> (bool, bool) {
        if self.water(begin) {
            return (
                self.map.distance_manhattan(spring, begin) > min_length,
                (self.s.tile_z)(begin) == 0,
            );
        }
        let begin_height = (self.s.tile_height)(begin) as i32;
        let mut marks = NodeSet::default();
        marks.insert(begin);
        let mut queue = vec![begin];
        let mut found = false;
        let mut end = begin;
        let mut index = 0;
        while index != queue.len() {
            end = queue[index];
            let flat = (self.a.flat_height)(end);
            if flat.flat
                && (flat.height < begin_height || (flat.height == begin_height && self.water(end)))
            {
                if self.water(end) && (self.clear.water_class)(end) == WATER_CLASS_SEA {
                    const MAX_SEA_SIZE_THRESHOLD: usize = 1024;
                    let threshold = (2.0 * f64::from(self.size()).sqrt()) as usize;
                    let threshold = threshold.min(MAX_SEA_SIZE_THRESHOLD);
                    (self.a.sea_construct)(frame);
                    let edge = self.connected_sea(frame, end, threshold);
                    if edge || (self.a.sea_size)(frame) > threshold {
                        found = true;
                    } else {
                        w::hit(w::GENERATION_SEA_FLATTEN);
                        let mut tile = (self.a.sea_first)(frame);
                        while tile != INVALID_TILE {
                            self.terraform(tile, SLOPE_ELEVATED, false, F::EXECUTE);
                            let slope = complement_slope((self.s.tile_slope)(tile));
                            self.terraform(tile, slope, true, F::EXECUTE);
                            tile = (self.a.sea_next)(frame);
                        }
                    }
                    (self.a.sea_destroy)(frame);
                    if found {
                        break;
                    }
                } else {
                    found = true;
                    break;
                }
            }
            for dir in 0..4 {
                let other = self.map.add_diag(end, dir);
                if self.valid(other) && !marks.contains(&other) && self.flows_down(end, other) {
                    marks.insert(other);
                    queue.push(other);
                }
            }
            index += 1;
        }
        let mut main_river = false;
        if found {
            w::hit(w::GENERATION_FLOW_RECURSE);
            (found, main_river) = self.flow_river(frame, spring, end, min_length, g);
        } else if queue.len() > 32 {
            let center = queue[self.s.random_range(queue.len() as u32) as usize];
            if self.map.distance_manhattan(spring, center) > min_length
                && self.try_terminus(center, begin, g)
            {
                end = center;
                found = true;
            }
        }
        if found {
            self.build_river(begin, end, spring, main_river, g);
        }
        (found, main_river)
    }
    fn create_rivers(&self, frame: *mut Frame, g: &Settings) -> bool {
        if g.rivers == 0 {
            return true;
        }
        let mut wells = self
            .map
            .scale_by_size(self.size_y, 4_u32.wrapping_shl(u32::from(g.rivers)));
        let short = wells.wrapping_sub((wells / 10).max(1));
        if !(self.a.set_progress)(
            frame,
            GWP_RIVER,
            wells + crate::landscape::TILE_UPDATE_FREQUENCY / 64,
        ) {
            return false;
        }
        while wells != 0 {
            let long = wells > short;
            w::hit(if long {
                w::GENERATION_LONG_ATTEMPT
            } else {
                w::GENERATION_SHORT_ATTEMPT
            });
            if !(self.a.increase_progress)(frame, GWP_RIVER) {
                return false;
            }
            for _ in 0..if long { 512 } else { 128 } {
                let random_tile = self.s.random() & (self.size() - 1);
                let mut spiral = crate::spiral::square(
                    self.map.tile_x(random_tile),
                    self.map.tile_y(random_tile),
                    8,
                    self.map.size_x(),
                    self.size_y,
                );
                let mut done = false;
                while !spiral.is_end() {
                    let tile = self.map.tile_xy(spiral.x, spiral.y);
                    if self.find_spring(tile, g) {
                        w::hit(w::GENERATION_SPRING);
                        done = self
                            .flow_river(
                                frame,
                                tile,
                                tile,
                                u32::from(g.min_river_length) * if long { 4 } else { 1 },
                                g,
                            )
                            .0;
                        break;
                    }
                    spiral = spiral.advance(self.map.size_x(), self.size_y);
                }
                if done {
                    break;
                }
            }
            wells -= 1;
        }
        self.convert_ground_water();
        for i in 0..crate::landscape::TILE_UPDATE_FREQUENCY {
            if i % 64 == 0 && !(self.a.increase_progress)(frame, GWP_RIVER) {
                return false;
            }
            self.run_tile_loop(frame);
        }
        true
    }
    fn generate(&self, frame: *mut Frame, decoder: &Decoder, mode: u8, g: &Settings) -> bool {
        const GLS_HEIGHTMAP: u32 = 3;
        const GLS_TERRAGENESIS: u32 = 4;
        const GLS_ORIGINAL: u32 = 2;
        const GLS_TROPIC: u32 = 12;
        const GLS_OTHER: u32 = 0;
        let steps = if g.climate == TROPIC {
            GLS_TROPIC
        } else {
            GLS_OTHER
        };
        if mode == GWM_HEIGHTMAP {
            if !(self.a.set_progress)(frame, GWP_LANDSCAPE, steps + GLS_HEIGHTMAP) {
                return false;
            }
            if !self.load_heightmap(
                decoder,
                g.rotation,
                g.freeform,
                g.heightmap_height,
                g.height_limit,
            ) {
                return false;
            }
            if !(self.a.increase_progress)(frame, GWP_LANDSCAPE) {
                return false;
            }
        } else if g.generator == LG_TERRAGENESIS {
            w::hit(w::GENERATION_TGP);
            if !(self.a.set_progress)(frame, GWP_LANDSCAPE, steps + GLS_TERRAGENESIS) {
                return false;
            }
            if !crate::tgp::run(self, frame, (self.a.tgp_settings)()) {
                return false;
            }
        } else {
            w::hit(w::GENERATION_ORIGINAL);
            if !(self.a.set_progress)(frame, GWP_LANDSCAPE, steps + GLS_ORIGINAL) {
                return false;
            }
            if g.freeform {
                self.void_top_edges();
            }
            let r = self.s.random();
            match g.climate {
                ARCTIC => {
                    for _ in 0..self.map.scale_by_size(self.size_y, (r & 127) + 950) {
                        self.generate_terrain(2, 0, g.freeform);
                    }
                    let flag = ((r >> 7) & 3) | 4;
                    for _ in 0..self.map.scale_by_size(self.size_y, ((r >> 9) & 127) + 450) {
                        self.generate_terrain(4, flag, g.freeform);
                    }
                }
                TROPIC => {
                    for _ in 0..self.map.scale_by_size(self.size_y, (r & 127) + 170) {
                        self.generate_terrain(0, 0, g.freeform);
                    }
                    let mut flag = ((r >> 7) & 3) | 4;
                    for _ in 0..self.map.scale_by_size(self.size_y, ((r >> 9) & 255) + 1700) {
                        self.generate_terrain(0, flag, g.freeform);
                    }
                    flag ^= 2;
                    for _ in 0..self.map.scale_by_size(self.size_y, ((r >> 17) & 127) + 410) {
                        self.generate_terrain(3, flag, g.freeform);
                    }
                }
                _ => {
                    (self.a.assert_condition)(g.sea != CUSTOM_SEA_LEVEL_NUMBER_DIFFICULTY);
                    let amount =
                        (r & 127) + 3_u32.wrapping_sub(u32::from(g.sea)).wrapping_mul(256) + 100;
                    for _ in 0..self.map.scale_by_size(self.size_y, amount) {
                        self.generate_terrain(usize::from(g.terrain.min(3)), 0, g.freeform);
                    }
                }
            }
        }
        // No progress/redraw may observe the still-invalid source slopes.
        self.fix_slopes(g.height_limit);
        (self.a.screen_dirty)();
        if !(self.a.increase_progress)(frame, GWP_LANDSCAPE) {
            return false;
        }
        self.convert_ground_water();
        (self.a.screen_dirty)();
        if !(self.a.increase_progress)(frame, GWP_LANDSCAPE) {
            return false;
        }
        match g.climate {
            ARCTIC => {
                w::hit(w::GENERATION_SNOW);
                self.calculate_snow_line(g.snow_coverage);
            }
            TROPIC => {
                let line = u32::from(self.calculate_desert_line(g.desert_coverage));
                if !self.desert_rainforest(frame, line) {
                    return false;
                }
            }
            _ => {}
        }
        self.create_rivers(frame, g)
    }
}
/// # Safety
/// All native metadata/decoder/frame storage lives through the synchronous call.
/// Map operations are serialized; progress catches native abortion and returns
/// false. No native exception or Rust panic may unwind across this boundary.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_generate_landscape(
    a: *const GenerationServices,
    frame: *mut Frame,
    decoder: *const Decoder,
    g: Settings,
    mode: u8,
) -> bool {
    unsafe { World::new(&*a) }.generate(frame, unsafe { &*decoder }, mode, &g)
}

impl World<'_> {
    fn make_wider(&self, tile: u32, origin: u32) {
        if !self.valid(tile) || self.water(tile) || (self.s.tile_max_z)(tile) == 0 {
            return;
        }
        let mut current = (self.s.tile_slope)(tile);
        let mut desired = (self.s.tile_slope)(origin);
        if (self.s.tile_max_z)(tile) > (self.s.tile_max_z)(origin) {
            return;
        }
        if !self.flat(tile) && !is_inclined_slope(current) {
            if is_steep_slope(current) {
                return;
            }
            let mut flat_river = false;
            let mut sloped_river = false;
            for dir in 0..4 {
                let other = self.map.add_diag(tile, dir);
                let slope = (self.s.tile_slope)(other);
                if self.water(other) && (self.a.river)(other) {
                    if is_inclined_slope(slope)
                        && (self.s.tile_max_z)(tile) == (self.s.tile_max_z)(other)
                        && (get_inclined_slope_direction(slope)
                            == change_diag_dir(dir, DIAGDIRDIFF_90RIGHT)
                            || get_inclined_slope_direction(slope)
                                == change_diag_dir(dir, DIAGDIRDIFF_90LEFT))
                    {
                        desired = slope;
                        sloped_river = true;
                        break;
                    }
                    if self.flat(other) {
                        flat_river = true;
                    }
                }
            }
            if !sloped_river && !flat_river {
                return;
            }
            if !sloped_river && flat_river {
                desired = SLOPE_FLAT;
            }
            if desired == SLOPE_FLAT && is_slope_with_three_corners_raised(current) {
                for dir in 0..4 {
                    let other = self.map.add_diag(tile, dir);
                    if is_inclined_slope((self.s.tile_slope)(other)) && self.water(other) {
                        return;
                    }
                }
                w::hit(w::GENERATION_WIDEN_TERRAFORM);
                self.terraform(tile, complement_slope(current), true, F::EXECUTE | F::AUTO);
            } else if is_inclined_slope(desired) {
                let direction = reverse_diag_dir(get_inclined_slope_direction(desired), |ok| {
                    (self.a.assert_condition)(ok);
                });
                for diff in DIAGDIRDIFF_SAME..DIAGDIRDIFF_END {
                    if diff == DIAGDIRDIFF_SAME || diff == DIAGDIRDIFF_REVERSE {
                        continue;
                    }
                    let other = self.map.add_diag(tile, change_diag_dir(direction, diff));
                    if self.water(other) && (self.a.river)(other) && self.flat(other) {
                        return;
                    }
                }
                let mut change = current ^ desired;
                if !is_slope_with_one_corner_raised(current) {
                    change &= complement_slope(desired);
                    self.terraform(tile, change, false, F::EXECUTE | F::AUTO);
                }
                current = (self.s.tile_slope)(tile);
                if current != desired && is_slope_with_one_corner_raised(current) {
                    change = current ^ desired;
                    self.terraform(tile, change, true, F::EXECUTE | F::AUTO);
                }
            }
            current = (self.s.tile_slope)(tile);
        }
        if is_inclined_slope(current) {
            w::hit(w::GENERATION_WIDEN_SLOPE);
            let direction = get_inclined_slope_direction(current);
            let upstream = self.map.add_diag(tile, direction);
            let downstream = self.map.add_diag(
                tile,
                reverse_diag_dir(direction, |ok| (self.a.assert_condition)(ok)),
            );
            if !self.valid(upstream) || !self.valid(downstream) {
                return;
            }
            let ocean = (self.s.tile_z)(downstream) == 0
                && ((self.s.tile_slope)(downstream) == SLOPE_FLAT
                    || is_slope_with_one_corner_raised((self.s.tile_slope)(downstream)));
            if !self.water(downstream) && !ocean {
                if (self.s.tile_slope)(downstream) != SLOPE_FLAT {
                    return;
                }
                self.river_tile(downstream);
            }
            if !self.water(upstream) {
                if (self.s.tile_slope)(upstream) != SLOPE_FLAT {
                    return;
                }
                self.river_tile(upstream);
            }
        }
        if current == desired {
            w::hit(w::GENERATION_WIDEN);
            self.river_tile(tile);
        }
    }
    fn build_river(&self, start: u32, end: u32, spring: u32, main: bool, g: &Settings) {
        let mut pf = RiverBuilder::new(self, g, start, end);
        if !pf.find_path() {
            return;
        }
        let destination = pf.get_best_node().unwrap();
        w::hit(w::GENERATION_YAPF_FOUND);
        let mut current = Some(destination);
        while let Some(id) = current {
            let tile = pf.nodes[id].get_tile();
            if !self.water(tile) {
                self.river_tile(tile);
            }
            current = pf.nodes[id].parent;
        }
        if g.generator != LG_ORIGINAL && main {
            let long = u32::from(g.min_river_length) * 4;
            let mut current = Some(destination);
            while let Some(id) = current {
                let center = pf.nodes[id].get_tile();
                let length = self.map.distance_manhattan(spring, center);
                let diameter = (length / (long / 3) + 1).min(3);
                if diameter > 1 {
                    self.for_spiral(center, diameter, |tile| self.make_wider(tile, center));
                }
                current = pf.nodes[id].parent;
            }
        }
    }
}

type RiverNode = crate::yapf_node::Node<u32>;
/// Complete local CYapfBaseT/NodeList state. The no-cache component retains no
/// segments; keys are tiles because every source trackdir is `INVALID_TRACKDIR`.
struct RiverBuilder<'a, 'w> {
    world: &'a World<'w>,
    settings: &'a Settings,
    end: u32,
    nodes: Vec<RiverNode>,
    open: NodeMap<u32, usize>,
    closed: NodeMap<u32, usize>,
    heap: crate::binaryheap::BinaryHeap,
    best_dest: Option<usize>,
    best_intermediate: Option<usize>,
    rounds: u32,
}
impl<'a, 'w> RiverBuilder<'a, 'w> {
    fn new(world: &'a World<'w>, settings: &'a Settings, start: u32, end: u32) -> Self {
        let mut pf = Self {
            world,
            settings,
            end,
            nodes: Vec::new(),
            open: NodeMap::default(),
            closed: NodeMap::default(),
            heap: crate::binaryheap::BinaryHeap::new(2048),
            best_dest: None,
            best_intermediate: None,
            rounds: 0,
        };
        pf.add_startup_node(RiverNode::set(None, start));
        pf
    }
    fn node_cache_fetch(_node: &mut RiverNode) -> bool {
        false
    }
    fn include(&mut self, id: usize) {
        self.heap
            .include(id, |a, b| self.nodes[a].less(&self.nodes[b]));
    }
    fn pop_open_node(&mut self, id: usize) {
        self.open.remove(&self.nodes[id].get_tile());
        self.heap
            .remove(id, |a, b| self.nodes[a].less(&self.nodes[b]));
    }
    fn add_startup_node(&mut self, mut node: RiverNode) {
        Self::node_cache_fetch(&mut node);
        if !self.open.contains_key(&node.get_tile()) {
            let id = self.nodes.len();
            self.open.insert(node.get_tile(), id);
            self.nodes.push(node);
            self.include(id);
        }
    }
    fn detect_destination(&self, node: RiverNode) -> bool {
        node.get_tile() == self.end
    }
    fn calc_cost(&self, node: &mut RiverNode) {
        node.cost = self.nodes[node.parent.unwrap()]
            .cost
            .wrapping_add(1)
            .wrapping_add(
                self.world
                    .s
                    .random_range(u32::from(self.settings.route_random)) as i32,
            );
    }
    fn calc_estimate(&self, node: &mut RiverNode) {
        node.estimate = node
            .cost
            .wrapping_add(self.world.map.distance_manhattan(self.end, node.get_tile()) as i32);
        (self.world.a.assert_condition)(node.estimate >= self.nodes[node.parent.unwrap()].estimate);
    }
    fn add_new_node(&mut self, mut node: RiverNode) {
        Self::node_cache_fetch(&mut node);
        // Source calculates cost/estimate before either duplicate admission
        // check. Rejected candidates consume the same shared random word.
        self.calc_cost(&mut node);
        self.calc_estimate(&mut node);
        let set_intermediate = self.settings.max_search_nodes > 0
            && self.best_intermediate.is_none_or(|id| {
                self.nodes[id].estimate.wrapping_sub(self.nodes[id].cost)
                    > node.estimate.wrapping_sub(node.cost)
            });
        if let Some(&old) = self.open.get(&node.get_tile()) {
            if node.estimate < self.nodes[old].estimate {
                w::hit(w::GENERATION_YAPF_OPEN_UPDATE);
                self.pop_open_node(old);
                self.nodes[old] = node;
                self.open.insert(node.get_tile(), old);
                self.include(old);
                if set_intermediate {
                    self.best_intermediate = Some(old);
                }
            }
            return;
        }
        if let Some(&old) = self.closed.get(&node.get_tile()) {
            if node.estimate < self.nodes[old].estimate {
                (self.world.a.not_reached)();
            }
            return;
        }
        let id = self.nodes.len();
        self.nodes.push(node);
        self.open.insert(node.get_tile(), id);
        self.include(id);
        if set_intermediate {
            self.best_intermediate = Some(id);
        }
    }
    fn follow_node(&mut self, old: usize) {
        for dir in 0..4 {
            let tile = self.world.map.add_diag(self.nodes[old].get_tile(), dir);
            if self.world.valid(tile) && self.world.flows_down(self.nodes[old].get_tile(), tile) {
                self.add_new_node(RiverNode::set(Some(old), tile));
            }
        }
    }
    fn get_best_node(&self) -> Option<usize> {
        self.best_dest.or(self.best_intermediate)
    }
    fn find_path(&mut self) -> bool {
        #[allow(clippy::while_let_loop)] // Source counts the empty-open final round.
        loop {
            self.rounds = self.rounds.wrapping_add(1);
            let Some(best) = self.heap.first() else {
                break;
            };
            if self.detect_destination(self.nodes[best]) {
                self.best_dest = Some(best);
                break;
            }
            self.follow_node(best);
            if self.settings.max_search_nodes != 0
                && self.closed.len() as i32 >= self.settings.max_search_nodes
            {
                w::hit(w::GENERATION_YAPF_LIMIT);
                break;
            }
            self.pop_open_node(best);
            self.closed.insert(self.nodes[best].get_tile(), best);
        }
        let (cost, distance) = self.best_dest.map_or((-1, -1), |id| {
            (
                self.nodes[id].cost,
                self.nodes[id].estimate.wrapping_sub(self.nodes[id].cost),
            )
        });
        (self.world.a.river_debug)(
            self.best_dest.is_some(),
            self.rounds,
            self.open.len() as u32,
            self.closed.len() as u32,
            cost,
            distance,
        );
        self.best_dest.is_some()
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTerrainTemplate" ["landscape"] => crate::landscape_generation::Template {width,height,data,};
    "OpenTTDFlatTileHeight" ["landscape"] => crate::landscape_generation::FlatHeight {height,flat,};
    "OpenTTDGenerationLoop" ["landscape"] => crate::landscape_generation::Loop {settings,services,};
    "OpenTTDLandscapeGenerationSettings" ["landscape"] => crate::landscape_generation::Settings {max_search_nodes,climate,generator,rivers,min_river_length,route_random,terrain,sea,snow_coverage,desert_coverage,tree_placer,height_limit,rotation,heightmap_height,freeform,};
    "OpenTTDHeightmapImage" ["landscape"] => crate::landscape_generation::Image { width, height, data, };
    "OpenTTDHeightmapDecoder" ["landscape"] => crate::landscape_generation::Decoder { context, read, finish, };
    "OpenTTDLandscapeGenerationServices" ["landscape"] => crate::landscape_generation::GenerationServices {
        shared, clear, water, trees, landscape_commands, water_commands, set_progress, increase_progress, begin_loop, end_loop, set_abort_callback, tgp_settings, terrain_template, flat_height, river, set_snow_line, not_reached, river_debug, assert_condition, sea_construct, sea_destroy, sea_insert, sea_contains, sea_size, sea_first, sea_next, set_height, inner_tile, make_void, screen_dirty, surrounded_by_water, set_freeform, clear_bridge_middle,
    };
}
