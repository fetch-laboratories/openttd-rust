/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Complete landscape-generation control. Native leaves own shared map writes.
//! Progress cancellation returns through native frames; no map reference is kept.
#![allow(unsafe_code, missing_docs, clippy::cast_possible_truncation)]
use crate::ground::ClearServices;
use crate::services::{ClearGroundType, Map, Services};
use crate::water::WaterServices;

#[repr(C)]
pub struct GenerationServices {
    pub shared: *const Services,
    pub clear: *const ClearServices,
    pub water: *const WaterServices,
    pub set_height: extern "C" fn(u32, u32),
    pub inner_tile: extern "C" fn(u32) -> bool,
    pub make_void: extern "C" fn(u32),
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
                (self.clear.make_clear)(tile, ClearGroundType(0), 3);
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
                    let mut current = 255;
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
                            (self.clear.make_clear)(tile, ClearGroundType(2), 3);
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
