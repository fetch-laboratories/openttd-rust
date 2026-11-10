/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `LoadTownData`: ordered validation and partial import, coordinates, founding
//! attempts, failure signs and bounded expansion. File reading, JSON parsing and
//! each feature's typed extraction stay native; Rust asks for one feature at a
//! time, so a malformed later feature leaves earlier towns founded and unexpanded.
use super::State;
use super::founding::{FoundingServices, Tile, TownLayout, TownSize};
use crate::heightmap::HeightmapRotation;
use crate::services::Services;
use crate::tile_type::TileIndex;
use crate::town_type::TownExpandMode;
use crate::town_type::TownID;

#[repr(C)]
pub struct Frame {
    _private: [u8; 0],
}

/// One validated feature; the native frame keeps its name until the next `feature` call.
#[repr(C)]
pub struct Feature {
    /// Target population; original `population`.
    pub population: u32,
    /// Original `x_proportion`: proportion 0..1 of the maximum X coordinate.
    pub x: f32,
    /// Original `y_proportion`: proportion 0..1 of the maximum Y coordinate.
    pub y: f32,
    /// Original `is_city`: whether to create a city.
    pub city: bool,
}

/// The feature passed every type check.
pub const FEATURE_OK: u8 = 0;
/// Past the last array element; status 2 is a non-object or a missing or non-string name.
pub const FEATURE_END: u8 = 1;
/// Another field is missing or mistyped; the native frame retains the town name.
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
    pub named_error: extern "C" fn(u32, u32, *const Frame),
    pub failed_warning: extern "C" fn(u32, u32),
    /// `Command<CMD_FOUND_TOWN>::Do(DoCommandFlag::Execute, ...)`, returning the town ID.
    pub found: extern "C" fn(*const Frame, Tile, TownSize, bool, TownLayout, bool, u32) -> TownID,
    /// `Command<CMD_PLACE_SIGN>::Post`.
    pub place_sign: extern "C" fn(Tile, *const Frame),
    /// `Command<CMD_EXPAND_TOWN>::Post`.
    pub expand: extern "C" fn(TownID, u32, u8),
}

impl ImportServices {
    fn format_error(&self) {
        (unsafe { &*self.shared }.show_error)(self.strings.load_failed, self.strings.json_format);
    }

    /// `LoadTownData`: find the target tile with the original float products and unsigned conversions.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub(crate) fn target(
        &self,
        rotation: HeightmapRotation,
        x_proportion: f32,
        y_proportion: f32,
    ) -> TileIndex {
        let s = unsafe { &*self.shared };
        let map = s.map();
        let max_x = map.max_x() as f32;
        let max_y = (s.map_max_y)() as f32;
        match rotation {
            HeightmapRotation::HM_CLOCKWISE => {
                // Tile coordinates align with what we expect.
                TileIndex(map.tile_xy((x_proportion * max_x) as u32, (y_proportion * max_y) as u32))
            }
            HeightmapRotation::HM_COUNTER_CLOCKWISE => {
                // Tile coordinates are rotated and must be adjusted.
                // AArch64 compilers contract the original subtraction and multiplication.
                #[cfg(target_arch = "aarch64")]
                let rotated_x = (-y_proportion).mul_add(max_x, 1.0);
                #[cfg(not(target_arch = "aarch64"))]
                let rotated_x = 1.0 - y_proportion * max_x;
                // Emscripten and AArch64 clamp the original negative float-to-uint conversion to zero.
                #[cfg(any(target_os = "emscripten", target_arch = "aarch64"))]
                let tile_x = rotated_x as u32;
                // x86 compilers truncate through a signed integer, retaining the low 32 bits.
                #[cfg(not(any(target_os = "emscripten", target_arch = "aarch64")))]
                let tile_x = (rotated_x as i64) as u32;
                TileIndex(map.tile_xy(tile_x, (x_proportion * max_y) as u32))
            }
            _ => {
                (s.not_reached)();
                TileIndex(0)
            }
        }
    }

    /// `LoadTownData` founding loop: seek a suitable tile up to 16 tiles away.
    /// The target may be water, blocked, or a steep slope the founding command cannot terraform.
    /// `SpiralTileSequence(target_tile, 16, 0, 0)` excludes the target itself.
    fn found(&self, frame: *mut Frame, target_tile: TileIndex, feature: &Feature) -> TownID {
        let s = unsafe { &*self.shared };
        let layout = (unsafe { &*self.founding }.layout_setting)();
        let map = s.map();
        let size_y = crate::map_func::World::new(s).size_y();
        let mut spiral = crate::spiral::hole(
            map.tile_x(target_tile.0),
            map.tile_y(target_tile.0),
            16,
            0,
            0,
            map.size_x(),
            size_y,
        );
        let mut town_id = TownID::INVALID;
        while !spiral.is_end() {
            let tile = TileIndex(map.tile_xy(spiral.x, spiral.y));
            town_id = (self.found)(
                frame,
                tile,
                TownSize::TSZ_SMALL,
                feature.city,
                layout,
                false,
                0,
            );
            // Check if the command succeeded.
            if town_id != TownID::INVALID {
                break;
            }
            spiral = spiral.advance(map.size_x(), size_y);
        }
        town_id
    }

    /// `genworld.cpp::LoadTownData`: found towns, then expand to target populations.
    fn load_town_data(&self, frame: *mut Frame, rotation: HeightmapRotation) {
        // Load the JSON file as a string, parse it, and check that the result is an array.
        if !(self.load)(frame) {
            self.format_error();
            return;
        }
        let shared = unsafe { &*self.shared };
        let mut towns: Vec<(TownID, *mut State, u32)> = Vec::new();
        let mut failed_towns: u32 = 0;
        let mut index = 0;
        // Iterate through towns and attempt to found them.
        loop {
            let mut feature = Feature {
                population: 0,
                x: 0.0,
                y: 0.0,
                city: false,
            };
            // Ensure the feature is an object and its fields exist with the correct types.
            match (self.feature)(frame, index, &raw mut feature) {
                FEATURE_OK => {}
                FEATURE_END => break,
                FEATURE_NAMED_MALFORMED => {
                    // Other malformed fields let us tell the player which town is the problem.
                    (self.named_error)(self.strings.load_failed, self.strings.town_format, frame);
                    return;
                }
                _ => {
                    // A malformed town name permits only a general warning.
                    self.format_error();
                    return;
                }
            }
            index += 1;
            // Check for improper coordinates and warn the player.
            if feature.x <= 0.0 || feature.y <= 0.0 || feature.x >= 1.0 || feature.y >= 1.0 {
                (self.named_error)(self.strings.load_failed, self.strings.bad_coordinate, frame);
                return;
            }
            // Find the target tile for the town.
            let target_tile = self.target(rotation, feature.x, feature.y);
            let town_id = self.found(frame, target_tile, &feature);
            if town_id == TownID::INVALID {
                // Leave a sign at the intended location and count the failure, so the
                // player can diagnose heightmap misalignment or place rough-terrain towns manually.
                (self.place_sign)(target_tile, frame);
                failed_towns = failed_towns.wrapping_add(1);
                continue;
            }
            towns.push((
                town_id,
                (shared.town_owner_by_id)(u32::from(town_id.0)),
                feature.population,
            ));
        }
        // Tell the player how many towns could not be founded.
        if failed_towns > 0 {
            (self.failed_warning)(self.strings.failed_towns, failed_towns);
        }
        // Now that the towns have been created, grow them to their target populations.
        for (town_id, state, population) in towns {
            // Grow a constant number of houses, rather than basing growth on town size,
            // so the try limit applies predictably regardless of road layout and geography.
            const HOUSES_TO_GROW: u32 = 10;
            // Grid towns can grow almost forever, but wandering roads randomly gets
            // less efficient. An arbitrary limit yields about 4900 houses on a flat
            // 3x3 grid, or 2800 houses with "Better roads".
            let mut try_limit: i32 = 1000;
            // Repeated failed growth attempts only waste time.
            let mut fail_limit: i32 = 10;
            loop {
                // SAFETY: The founded town's owner lives through expansion; each read ends before Post.
                let before = unsafe { super::num_houses(state) };
                (self.expand)(
                    town_id,
                    HOUSES_TO_GROW,
                    (TownExpandMode::Buildings | TownExpandMode::Roads).0,
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
    unsafe { &*services }.load_town_data(frame, HeightmapRotation(rotation));
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDTownDataFeature" ["genworld"] => Feature { population, x, y, city, };
    "OpenTTDTownDataStrings" ["genworld"] => Strings { load_failed, json_format, town_format, bad_coordinate, failed_towns, };
    "OpenTTDTownDataServices" ["genworld"] => ImportServices { shared, founding, strings, load, feature, named_error, failed_warning, found, place_sign, expand, };
}
