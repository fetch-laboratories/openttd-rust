/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `LoadTownData`: ordered validation and partial import, coordinates, founding
//! attempts, failure signs and bounded expansion. File reading, JSON parsing and
//! each feature's typed extraction stay native; Rust asks for one feature at a
//! time, so a malformed later feature leaves earlier towns founded and unexpanded.
use super::founding::{FoundingServices, Tile, TownLayout, TownSize};
use super::{State, TownId};
use crate::heightmap::{HM_CLOCKWISE, HM_COUNTER_CLOCKWISE};
use crate::services::{ByteSpan, Services};
use crate::town_type::{TOWN_EXPAND_BUILDINGS, TOWN_EXPAND_ROADS, TSZ_SMALL};

#[repr(C)]
pub struct Frame {
    _private: [u8; 0],
}

/// One validated feature; `name` lives until the next `feature` call.
#[repr(C)]
pub struct Feature {
    pub name: ByteSpan,
    pub population: u32,
    pub x: f32,
    pub y: f32,
    pub city: bool,
}

/// The feature passed every type check.
pub const FEATURE_OK: u8 = 0;
/// Past the last array element; status 2 is a non-object or a missing or non-string name.
pub const FEATURE_END: u8 = 1;
/// Another field is missing or mistyped; `name` identifies the town.
pub const FEATURE_NAMED_MALFORMED: u8 = 3;

#[repr(C)]
pub struct Strings {
    pub load_failed: u32,
    pub json_format: u32,
    pub town_format: u32,
    pub bad_coordinate: u32,
    pub failed_towns: u32,
}

#[repr(C)]
pub struct ImportServices {
    pub shared: *const Services,
    pub founding: *const FoundingServices,
    pub strings: Strings,
    /// Read and parse `_file_to_saveload`; false for any file, parse or non-array failure.
    pub load: extern "C" fn(*mut Frame) -> bool,
    pub feature: extern "C" fn(*mut Frame, usize, *mut Feature) -> u8,
    pub named_error: extern "C" fn(u32, u32, ByteSpan),
    pub failed_warning: extern "C" fn(u32, u32),
    /// `Command<CMD_FOUND_TOWN>::Do(DoCommandFlag::Execute, ...)`, returning the town ID.
    pub found: extern "C" fn(Tile, TownSize, bool, TownLayout, bool, u32, ByteSpan) -> TownId,
    /// `Command<CMD_PLACE_SIGN>::Post`.
    pub place_sign: extern "C" fn(Tile, ByteSpan),
    /// `Command<CMD_EXPAND_TOWN>::Post`.
    pub expand: extern "C" fn(TownId, u32, u8),
}

impl ImportServices {
    fn format_error(&self) {
        (unsafe { &*self.shared }.show_error)(self.strings.load_failed, self.strings.json_format);
    }

    /// The target tile, with the original float products and unsigned conversions.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub(crate) fn target(&self, rotation: u8, x: f32, y: f32) -> Tile {
        let s = unsafe { &*self.shared };
        let map = s.map();
        let max_x = map.max_x() as f32;
        let max_y = (s.map_max_y)() as f32;
        match rotation {
            HM_CLOCKWISE => Tile(map.tile_xy((x * max_x) as u32, (y * max_y) as u32)),
            HM_COUNTER_CLOCKWISE => {
                // Apple Clang contracts the original subtraction and multiplication on AArch64.
                #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
                let rotated_x = (-y).mul_add(max_x, 1.0);
                #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
                let rotated_x = 1.0 - y * max_x;
                // Emscripten and AArch64 clamp the original negative float-to-uint conversion to zero.
                #[cfg(any(target_os = "emscripten", target_arch = "aarch64"))]
                let tile_x = rotated_x as u32;
                // x86 compilers truncate through a signed integer, retaining the low 32 bits.
                #[cfg(not(any(target_os = "emscripten", target_arch = "aarch64")))]
                let tile_x = (rotated_x as i64) as u32;
                Tile(map.tile_xy(tile_x, (x * max_y) as u32))
            }
            _ => {
                (s.not_reached)();
                Tile(0)
            }
        }
    }

    /// Spiral founding attempts around the target, excluding the target itself.
    fn found(&self, target: Tile, feature: &Feature) -> TownId {
        let s = unsafe { &*self.shared };
        let layout = (unsafe { &*self.founding }.layout_setting)();
        let map = s.map();
        let size_y = (s.map_max_y)().wrapping_add(1);
        let mut spiral = crate::spiral::hole(
            map.tile_x(target.0),
            map.tile_y(target.0),
            16,
            0,
            0,
            map.size_x(),
            size_y,
        );
        let mut town = TownId::INVALID;
        while !spiral.is_end() {
            let tile = Tile(map.tile_xy(spiral.x, spiral.y));
            town = (self.found)(
                tile,
                TownSize(TSZ_SMALL),
                feature.city,
                layout,
                false,
                0,
                feature.name,
            );
            if town != TownId::INVALID {
                break;
            }
            spiral = spiral.advance(map.size_x(), size_y);
        }
        town
    }

    fn import(&self, frame: *mut Frame, rotation: u8) {
        // Grid towns can grow almost forever, so growth is capped; ten houses a
        // step keeps the try limit predictable whatever the road layout.
        const HOUSES_TO_GROW: u32 = 10;
        if !(self.load)(frame) {
            self.format_error();
            return;
        }
        let shared = unsafe { &*self.shared };
        let mut towns: Vec<(TownId, *mut State, u32)> = Vec::new();
        let mut failed_towns: u32 = 0;
        let mut index = 0;
        loop {
            let mut feature = Feature {
                name: ByteSpan {
                    data: std::ptr::null(),
                    size: 0,
                },
                population: 0,
                x: 0.0,
                y: 0.0,
                city: false,
            };
            match (self.feature)(frame, index, &raw mut feature) {
                FEATURE_OK => {}
                FEATURE_END => break,
                FEATURE_NAMED_MALFORMED => {
                    (self.named_error)(
                        self.strings.load_failed,
                        self.strings.town_format,
                        feature.name,
                    );
                    return;
                }
                _ => {
                    self.format_error();
                    return;
                }
            }
            index += 1;
            if feature.x <= 0.0 || feature.y <= 0.0 || feature.x >= 1.0 || feature.y >= 1.0 {
                (self.named_error)(
                    self.strings.load_failed,
                    self.strings.bad_coordinate,
                    feature.name,
                );
                return;
            }
            let target = self.target(rotation, feature.x, feature.y);
            let town = self.found(target, &feature);
            if town == TownId::INVALID {
                (self.place_sign)(target, feature.name);
                failed_towns = failed_towns.wrapping_add(1);
                continue;
            }
            towns.push((
                town,
                (shared.town_owner_by_id)(u32::from(town.0)),
                feature.population,
            ));
        }
        if failed_towns > 0 {
            (self.failed_warning)(self.strings.failed_towns, failed_towns);
        }
        for (town, state, population) in towns {
            let mut try_limit: i32 = 1000;
            let mut fail_limit: i32 = 10;
            loop {
                // SAFETY: The founded town's owner lives through expansion; each read ends before Post.
                let before = unsafe { super::num_houses(state) };
                (self.expand)(
                    town,
                    HOUSES_TO_GROW,
                    TOWN_EXPAND_BUILDINGS | TOWN_EXPAND_ROADS,
                );
                if unsafe { super::num_houses(state) } <= before {
                    fail_limit -= 1;
                }
                let tries = try_limit;
                try_limit -= 1;
                if !(fail_limit > 0
                    && tries > 0
                    && unsafe { super::population(state) } < population)
                {
                    break;
                }
            }
        }
    }
}

/// `LoadTownData`.
/// # Safety
/// Live services and a native frame for this synchronous call; no exception
/// leaves a service.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_load_town_data(
    services: *const ImportServices,
    frame: *mut Frame,
    rotation: u8,
) {
    unsafe { &*services }.import(frame, rotation);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTownDataFeature" ["genworld"] => Feature { name, population, x, y, city, };
    "OpenTTDTownDataStrings" ["genworld"] => Strings { load_failed, json_format, town_format, bad_coordinate, failed_towns, };
    "OpenTTDTownDataServices" ["genworld"] => ImportServices { shared, founding, strings, load, feature, named_error, failed_warning, found, place_sign, expand, };
}
