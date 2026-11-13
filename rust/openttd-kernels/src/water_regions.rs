/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `water_regions.h` and `water_regions.cpp`: dividing the water in the map into
//! square regions to assist pathfinding.
//!
//! The C++ facade holds one [`Regions`] owner per map and replaces it at every
//! `Map::Allocate`. Each export receives that owner exclusively for the call and
//! the immutable shared services table; no Rust borrow of the owner survives a
//! return, so a C++ visitor between two [`openttd_rust_water_visit_next`] steps
//! may enter the owner again. Replacing the owner during a visit breaks the
//! original's region-reference lifetime and is outside its defined behaviour.
#![allow(unsafe_code)]
use crate::bitmath_func::{gb, has_bit};
use crate::debug::debug;
use crate::direction_func::{diag_dir_to_axis, reverse_diag_dir};
use crate::direction_type::{Axis, DiagDirection};
use crate::follow_track::CFollowTrackT;
use crate::map_func::{Map, World, tile_index_diff_c_by_diag_dir};
use crate::movement::CachedTable;
use crate::rail_type::RailTypes;
use crate::services::Services;
use crate::tile_type::{TileIndex, TileType};
use crate::tilearea::TileArea;
use crate::track_func::{
    set_trackdir_bit_iterator, track_bits_to_trackdir_bits, track_status_to_track_bits,
};
use crate::track_type::{TrackBits, TrackdirBits};
use crate::transport_type::TransportType;

/// `WaterRegionIndex`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WaterRegionIndex(pub u32);

/// `WaterRegionPatchLabel`.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WaterRegionPatchLabel(pub u8);
impl WaterRegionPatchLabel {
    /// `operator++`, wrapping as `uint8_t` does.
    fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

/// `WATER_REGION_EDGE_LENGTH`.
pub const WATER_REGION_EDGE_LENGTH: i32 = 16;
/// `WATER_REGION_NUMBER_OF_TILES`.
pub const WATER_REGION_NUMBER_OF_TILES: i32 = WATER_REGION_EDGE_LENGTH * WATER_REGION_EDGE_LENGTH;
/// `INVALID_WATER_REGION_PATCH`.
pub const INVALID_WATER_REGION_PATCH: WaterRegionPatchLabel = WaterRegionPatchLabel(0);
/// `FIRST_REGION_LABEL`.
const FIRST_REGION_LABEL: WaterRegionPatchLabel = WaterRegionPatchLabel(1);

/// `WaterRegionTraversabilityBits`: one bit per edge tile.
type WaterRegionTraversabilityBits = u16;
/// `WATER_REGION_EDGE_LENGTH` as an array length.
#[allow(
    clippy::cast_sign_loss,
    reason = "the positive int WATER_REGION_EDGE_LENGTH sizes the per-side label buffer"
)]
const EDGE_TILES: usize = WATER_REGION_EDGE_LENGTH as usize;
/// `WATER_REGION_NUMBER_OF_TILES` as an array length.
#[allow(
    clippy::cast_sign_loss,
    reason = "the positive int WATER_REGION_NUMBER_OF_TILES is std::array's extent"
)]
const NUMBER_OF_TILES: usize = WATER_REGION_NUMBER_OF_TILES as usize;
/// `WaterRegionPatchLabelArray`.
type WaterRegionPatchLabelArray = [WaterRegionPatchLabel; NUMBER_OF_TILES];

// The original's static_asserts, as array lengths that must agree at compile time:
// `sizeof(WaterRegionTraversabilityBits) * 8 == WATER_REGION_EDGE_LENGTH`, and
// `sizeof(WaterRegionPatchLabel) == sizeof(uint8_t)` (important for the hash calculation).
const _: [(); EDGE_TILES] = [(); WaterRegionTraversabilityBits::BITS as usize];
const _: [(); std::mem::size_of::<u8>()] = [(); std::mem::size_of::<WaterRegionPatchLabel>()];

/// `WaterRegionPatchDesc`: describes a single interconnected patch of water
/// within a particular water region. The C++ side declares it as
/// `OpenTTDWaterPatch` with a `uint8_t` label.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WaterRegionPatchDesc {
    /// The X coordinate of the water region, i.e. X=2 is the 3rd water region along the X-axis.
    pub x: i32,
    /// The Y coordinate of the water region, i.e. Y=2 is the 3rd water region along the Y-axis.
    pub y: i32,
    /// Unique label identifying the patch within the region.
    pub label: WaterRegionPatchLabel,
}

/// `GetWaterTracks`.
fn get_water_tracks(movement: &CachedTable<'_>, tile: TileIndex) -> TrackBits {
    TrackBits(track_status_to_track_bits(
        movement.get().get_tile_track_status(
            tile,
            TransportType::TRANSPORT_WATER,
            0,
            DiagDirection::INVALID_DIAGDIR,
        ),
    ))
}

/// `IsAqueductTile`: `IsBridgeTile` (the tile type before `IsBridge`, which
/// asserts it) and then the transport type.
fn is_aqueduct_tile(world: &World<'_>, tile: TileIndex) -> bool {
    world.kind(tile) == TileType::MP_TUNNELBRIDGE
        && world.bridge(tile.0)
        && TransportType(world.tunnel_bridge_transport(tile.0)) == TransportType::TRANSPORT_WATER
}

/// `GetOtherBridgeEnd`, through the service the track follower uses.
fn get_other_bridge_end(s: &Services, tile: TileIndex) -> TileIndex {
    TileIndex((crate::movement::table(s).tunnel_bridge().other_bridge_end)(tile.0))
}

/// `GetWaterRegionX`.
fn get_water_region_x(map: Map, tile: TileIndex) -> i32 {
    (map.tile_x(tile) / WATER_REGION_EDGE_LENGTH.cast_unsigned()).cast_signed()
}
/// `GetWaterRegionY`.
fn get_water_region_y(map: Map, tile: TileIndex) -> i32 {
    (map.tile_y(tile) / WATER_REGION_EDGE_LENGTH.cast_unsigned()).cast_signed()
}

/// `GetWaterRegionMapSizeX`.
fn get_water_region_map_size_x(map: Map) -> i32 {
    (map.size_x() / WATER_REGION_EDGE_LENGTH.cast_unsigned()).cast_signed()
}
/// `GetWaterRegionMapSizeY`.
fn get_water_region_map_size_y(size_y: u32) -> i32 {
    (size_y / WATER_REGION_EDGE_LENGTH.cast_unsigned()).cast_signed()
}

/// `GetWaterRegionIndex(int region_x, int region_y)`.
fn get_water_region_index(map: Map, region_x: i32, region_y: i32) -> WaterRegionIndex {
    WaterRegionIndex((get_water_region_map_size_x(map) * region_y + region_x).cast_unsigned())
}
/// `GetWaterRegionIndex(TileIndex tile)`.
fn get_water_region_index_for_tile(map: Map, tile: TileIndex) -> WaterRegionIndex {
    get_water_region_index(
        map,
        get_water_region_x(map, tile),
        get_water_region_y(map, tile),
    )
}

/// The original's implicit `int` to `uint16_t` conversion of a region
/// coordinate passed to `GetUpdatedWaterRegion`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "GetUpdatedWaterRegion's uint16_t parameters narrow its int arguments"
)]
fn to_region_coordinate(value: i32) -> u16 {
    value as u16
}

/// `WaterRegionData`: the data stored for each water region.
#[derive(Default)]
struct WaterRegionData {
    edge_traversability_bits:
        [WaterRegionTraversabilityBits; DiagDirection::DIAGDIR_END.0 as usize],
    /// Tile patch labels, this may be `None` in the following trivial cases:
    /// region is invalid, region is only land (0 patches), region is only water (1 patch).
    tile_patch_labels: Option<Box<WaterRegionPatchLabelArray>>,
    has_cross_region_aqueducts: bool,
    /// 0 = no water, 1 = one single patch of water, etc...
    number_of_patches: u8,
}

/// The map's water regions: the original's `_water_region_data` and
/// `_is_water_region_valid`, with the map dimensions they were allocated for.
pub struct Regions {
    map: Map,
    size_y: u32,
    water_region_data: Vec<WaterRegionData>,
    is_water_region_valid: Vec<bool>,
    /// `ForceUpdate`'s function static `tiles_to_check`, which keeps its capacity
    /// between updates.
    tiles_to_check: Vec<TileIndex>,
}
impl Regions {
    /// The two containers `AllocateWaterRegions` sizes for a map of `size_x` by
    /// `size_y` tiles, every region invalid.
    pub(crate) fn new(size_x: u32, size_y: u32) -> Self {
        let map = Map::from_log_x(size_x.trailing_zeros());
        let number_of_regions =
            get_water_region_map_size_x(map) * get_water_region_map_size_y(size_y);
        let number_of_regions = number_of_regions.cast_unsigned() as usize;
        Self {
            map,
            size_y,
            water_region_data: (0..number_of_regions)
                .map(|_| WaterRegionData::default())
                .collect(),
            is_water_region_valid: vec![false; number_of_regions],
            tiles_to_check: Vec::new(),
        }
    }
    /// `AllocateWaterRegions`' allocation and its closing assertion; the facade
    /// keeps its debug line.
    fn allocate(world: &World<'_>, size_x: u32, size_y: u32) -> Self {
        let regions = Self::new(size_x, size_y);
        world
            .assert_with(|| regions.is_water_region_valid.len() == regions.water_region_data.len());
        regions
    }
    fn data(&self, index: WaterRegionIndex) -> &WaterRegionData {
        &self.water_region_data[index.0 as usize]
    }
}

/// `WaterRegion`: represents a square section of the map of a fixed size. Within
/// this square individual unconnected patches of water are identified using a
/// Connected Component Labeling (CCL) algorithm. Note that all information stored
/// in this class applies only to tiles within the square section, there is no
/// knowledge about the rest of the map. This makes it easy to invalidate and
/// update a water region if any changes are made to it, such as construction or
/// terraforming.
///
/// The original holds a reference to its `WaterRegionData`; this value holds its
/// index, so each method reads the live data from the owner without a borrow
/// that spans a visitor callback.
#[derive(Clone, Copy)]
struct WaterRegion {
    index: WaterRegionIndex,
    tile_area: TileArea,
}
impl WaterRegion {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "OrthogonalTileArea's uint16_t extents take the int WATER_REGION_EDGE_LENGTH"
    )]
    fn new(map: Map, region_x: u16, region_y: u16) -> Self {
        let edge = WATER_REGION_EDGE_LENGTH;
        Self {
            index: get_water_region_index(map, i32::from(region_x), i32::from(region_y)),
            tile_area: TileArea::new(
                map.tile_xy(
                    (i32::from(region_x) * edge).cast_unsigned(),
                    (i32::from(region_y) * edge).cast_unsigned(),
                )
                .0,
                edge as u16,
                edge as u16,
            ),
        }
    }

    fn tile(self) -> TileIndex {
        TileIndex(self.tile_area.tile)
    }

    fn contains(self, world: &World<'_>, map: Map, tile: TileIndex) -> bool {
        self.tile_area.contains(tile.0, map, |ok| world.check(ok))
    }

    /// Returns the local index of the tile within the region. The N corner represents 0,
    /// the x direction is positive in the SW direction, and Y is positive in the SE direction.
    fn get_local_index(self, world: &World<'_>, map: Map, tile: TileIndex) -> usize {
        world.assert_with(|| self.contains(world, map, tile));
        let x = map.tile_x(tile).wrapping_sub(map.tile_x(self.tile()));
        let y = map.tile_y(tile).wrapping_sub(map.tile_y(self.tile()));
        x.wrapping_add(WATER_REGION_EDGE_LENGTH.cast_unsigned().wrapping_mul(y)) as usize
    }

    /// Returns a set of bits indicating whether an edge tile on a particular side is
    /// traversable or not. These values can be used to determine whether a ship can
    /// enter/leave the region through a particular edge tile.
    /// See [`Self::get_local_index`] for a description of the coordinate system used.
    fn get_edge_traversability_bits(
        self,
        regions: &Regions,
        side: DiagDirection,
    ) -> WaterRegionTraversabilityBits {
        regions.data(self.index).edge_traversability_bits[usize::from(side.0)]
    }

    /// The amount of individual water patches present within the water region. A value
    /// of 0 means there is no water present in the water region at all.
    fn number_of_patches(self, regions: &Regions) -> i32 {
        i32::from(regions.data(self.index).number_of_patches)
    }

    /// Whether the water region contains aqueducts that cross the region boundaries.
    fn has_cross_region_aqueducts(self, regions: &Regions) -> bool {
        regions.data(self.index).has_cross_region_aqueducts
    }

    /// Returns the patch label that was assigned to the tile.
    fn get_label(
        self,
        regions: &Regions,
        world: &World<'_>,
        tile: TileIndex,
    ) -> WaterRegionPatchLabel {
        let map = regions.map;
        world.assert_with(|| self.contains(world, map, tile));
        match &regions.data(self.index).tile_patch_labels {
            None => {
                if self.number_of_patches(regions) == 0 {
                    INVALID_WATER_REGION_PATCH
                } else {
                    FIRST_REGION_LABEL
                }
            }
            Some(labels) => labels[self.get_local_index(world, map, tile)],
        }
    }

    /// Performs the connected component labeling and other data gathering.
    fn force_update(self, regions: &mut Regions, world: &World<'_>, s: &Services) {
        let map = regions.map;
        let movement = CachedTable::new(s);
        crate::witness::hit(crate::witness::WATER_REBUILDS);
        debug!(
            map,
            3,
            "Updating water region ({},{})",
            get_water_region_x(map, self.tile()),
            get_water_region_y(map, self.tile())
        );
        let Regions {
            water_region_data,
            tiles_to_check,
            ..
        } = regions;
        let data = &mut water_region_data[self.index.0 as usize];
        data.has_cross_region_aqueducts = false;

        // Acquire a tile patch label array if this region does not already have one
        let tile_patch_labels = data
            .tile_patch_labels
            .get_or_insert_with(|| Box::new([INVALID_WATER_REGION_PATCH; NUMBER_OF_TILES]));

        tile_patch_labels.fill(INVALID_WATER_REGION_PATCH);
        data.edge_traversability_bits.fill(0);

        let mut current_label = FIRST_REGION_LABEL;
        let mut highest_assigned_label = INVALID_WATER_REGION_PATCH;

        // Perform connected component labeling. This uses a flooding algorithm that expands until no
        // additional tiles can be added. Only tiles inside the water region are considered.
        for start_tile in self.tile_area.tiles(map).map(TileIndex) {
            tiles_to_check.clear();
            tiles_to_check.push(start_tile);

            let mut increase_label = false;
            while let Some(tile) = tiles_to_check.pop() {
                crate::witness::hit(crate::witness::WATER_TRACKS);
                let valid_dirs = TrackdirBits(track_bits_to_trackdir_bits(
                    get_water_tracks(&movement, tile).0,
                ));
                if valid_dirs == TrackdirBits::TRACKDIR_BIT_NONE {
                    continue;
                }

                let tile_patch = &mut tile_patch_labels[self.get_local_index(world, map, tile)];
                if *tile_patch != INVALID_WATER_REGION_PATCH {
                    continue;
                }

                *tile_patch = current_label;
                highest_assigned_label = current_label;
                increase_label = true;

                for dir in set_trackdir_bit_iterator(valid_dirs) {
                    // By using a TrackFollower we "play by the same rules" as the actual ship pathfinder
                    crate::witness::hit(crate::witness::WATER_FOLLOWS);
                    let mut ft = CFollowTrackT::vehicle(
                        s,
                        TransportType::TRANSPORT_WATER,
                        None,
                        RailTypes::INVALID_RAILTYPES,
                        true,
                    );
                    if ft.follow(s, tile, dir) {
                        if self.contains(world, map, ft.new_tile) {
                            tiles_to_check.push(ft.new_tile);
                        } else if !ft.is_bridge {
                            world.assert_with(|| map.distance_manhattan(ft.new_tile, tile) == 1);
                            let side = map.diagdir_between_tiles(tile, ft.new_tile);
                            let local_x_or_y =
                                if diag_dir_to_axis(side, |ok| world.check(ok)) == Axis::AXIS_X {
                                    map.tile_y(tile).wrapping_sub(map.tile_y(self.tile()))
                                } else {
                                    map.tile_x(tile).wrapping_sub(map.tile_x(self.tile()))
                                };
                            // SetBit
                            data.edge_traversability_bits[usize::from(side.0)] |= 1 << local_x_or_y;
                        } else {
                            data.has_cross_region_aqueducts = true;
                        }
                    }
                }
            }

            if increase_label {
                current_label.increment();
            }
        }

        data.number_of_patches = highest_assigned_label.0;

        // NumberOfPatches() over the count just stored.
        let number_of_patches = i32::from(data.number_of_patches);
        if number_of_patches == 0
            || (number_of_patches == 1
                && tile_patch_labels
                    .iter()
                    .all(|&label| label == FIRST_REGION_LABEL))
        {
            // No need for patch storage: trivial cases
            data.tile_patch_labels = None;
        }
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "GB takes the uint8_t bit position of the loop's int row"
    )]
    fn print_debug_info(self, regions: &Regions, world: &World<'_>) {
        let map = regions.map;
        debug!(
            map,
            9,
            "Water region {},{} labels and edge traversability = ...",
            get_water_region_x(map, self.tile()),
            get_water_region_y(map, self.tile())
        );

        let max_element_width = self.number_of_patches(regions).to_string().len();
        // fmt::join(traversability, " ") with each character padded to the element width.
        let spaced = |bits: WaterRegionTraversabilityBits| {
            let traversability = format!("{bits:0EDGE_TILES$b}");
            traversability
                .chars()
                .map(|c| format!("{c:max_element_width$}"))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let border = EDGE_TILES * (max_element_width + 1) + 1;

        let traversability =
            spaced(self.get_edge_traversability_bits(regions, DiagDirection::DIAGDIR_NW));
        debug!(map, 9, "    {}", traversability);
        debug!(map, 9, "  +{:->border$}+", "");

        for y in 0..WATER_REGION_EDGE_LENGTH {
            let mut line = String::new();
            for x in 0..WATER_REGION_EDGE_LENGTH {
                let label = self.get_label(
                    regions,
                    world,
                    world.dimensions.tile_add_xy(self.tile(), x, y),
                );
                if label == INVALID_WATER_REGION_PATCH {
                    line = format!("{:max_element_width$} {}", ".", line);
                } else {
                    line = format!("{:max_element_width$} {}", label.0, line);
                }
            }
            debug!(
                map,
                9,
                "{} | {}| {}",
                gb(
                    u32::from(
                        self.get_edge_traversability_bits(regions, DiagDirection::DIAGDIR_SW)
                    ),
                    y as u8,
                    1
                ),
                line,
                gb(
                    u32::from(
                        self.get_edge_traversability_bits(regions, DiagDirection::DIAGDIR_NE)
                    ),
                    y as u8,
                    1
                )
            );
        }

        debug!(map, 9, "  +{:->border$}+", "");
        let traversability =
            spaced(self.get_edge_traversability_bits(regions, DiagDirection::DIAGDIR_SE));
        debug!(map, 9, "    {}", traversability);
    }
}

/// `GetTileIndexFromLocalCoordinate`.
fn get_tile_index_from_local_coordinate(
    world: &World<'_>,
    map: Map,
    region_x: i32,
    region_y: i32,
    local_x: i32,
    local_y: i32,
) -> TileIndex {
    world.assert_with(|| (0..WATER_REGION_EDGE_LENGTH).contains(&local_x));
    world.assert_with(|| (0..WATER_REGION_EDGE_LENGTH).contains(&local_y));
    map.tile_xy(
        (WATER_REGION_EDGE_LENGTH * region_x + local_x).cast_unsigned(),
        (WATER_REGION_EDGE_LENGTH * region_y + local_y).cast_unsigned(),
    )
}

/// `GetEdgeTileCoordinate`.
fn get_edge_tile_coordinate(
    world: &World<'_>,
    map: Map,
    region_x: i32,
    region_y: i32,
    side: DiagDirection,
    x_or_y: i32,
) -> TileIndex {
    world.assert_with(|| (0..WATER_REGION_EDGE_LENGTH).contains(&x_or_y));
    let (local_x, local_y) = match side {
        DiagDirection::DIAGDIR_NE => (0, x_or_y),
        DiagDirection::DIAGDIR_SW => (WATER_REGION_EDGE_LENGTH - 1, x_or_y),
        DiagDirection::DIAGDIR_NW => (x_or_y, 0),
        DiagDirection::DIAGDIR_SE => (x_or_y, WATER_REGION_EDGE_LENGTH - 1),
        _ => crate::services_assert::not_reached(),
    };
    get_tile_index_from_local_coordinate(world, map, region_x, region_y, local_x, local_y)
}

/// `GetUpdatedWaterRegion(uint16_t region_x, uint16_t region_y)`.
fn get_updated_water_region(
    regions: &mut Regions,
    world: &World<'_>,
    s: &Services,
    region_x: u16,
    region_y: u16,
) -> WaterRegion {
    let water_region = WaterRegion::new(regions.map, region_x, region_y);
    let index = water_region.index.0 as usize;
    if !regions.is_water_region_valid[index] {
        water_region.force_update(regions, world, s);
        regions.is_water_region_valid[index] = true;
    }
    water_region
}

/// `GetUpdatedWaterRegion(TileIndex tile)`.
fn get_updated_water_region_for_tile(
    regions: &mut Regions,
    world: &World<'_>,
    s: &Services,
    tile: TileIndex,
) -> WaterRegion {
    let map = regions.map;
    get_updated_water_region(
        regions,
        world,
        s,
        to_region_coordinate(get_water_region_x(map, tile)),
        to_region_coordinate(get_water_region_y(map, tile)),
    )
}

/// `GetWaterRegionPatchInfo`: returns basic water region patch information for
/// the provided tile.
fn get_water_region_patch_info(
    regions: &mut Regions,
    world: &World<'_>,
    s: &Services,
    tile: TileIndex,
) -> WaterRegionPatchDesc {
    let region = get_updated_water_region_for_tile(regions, world, s, tile);
    let map = regions.map;
    WaterRegionPatchDesc {
        x: get_water_region_x(map, tile),
        y: get_water_region_y(map, tile),
        label: region.get_label(regions, world, tile),
    }
}

/// `InvalidateWaterRegion`: marks the water region that tile is part of as invalid.
pub(crate) fn invalidate_water_region(regions: &mut Regions, s: &Services, tile: TileIndex) {
    let world = World::new(s);
    if !world.is_valid_tile(tile) {
        return;
    }

    let map = regions.map;
    let invalidate_region = |regions: &mut Regions, tile: TileIndex| {
        let water_region_index = get_water_region_index_for_tile(map, tile).0 as usize;
        if !regions.is_water_region_valid[water_region_index] {
            debug!(
                map,
                3,
                "Invalidated water region ({},{})",
                get_water_region_x(map, tile),
                get_water_region_y(map, tile)
            );
        }
        regions.is_water_region_valid[water_region_index] = false;
    };

    invalidate_region(regions, tile);

    // When updating the water region we look into the first tile of adjacent water regions to determine edge
    // traversability. This means that if we invalidate any region edge tiles we might also change the traversability
    // of the adjacent region. This code ensures the adjacent regions also get invalidated in such a case.
    for side in DiagDirection::DIAGDIR_BEGIN.0..DiagDirection::DIAGDIR_END.0 {
        let adjacent_tile = map.add_tile_index_diff_c_wrap(
            tile,
            tile_index_diff_c_by_diag_dir(DiagDirection(side), |ok| world.check(ok)),
            regions.size_y,
        );
        if adjacent_tile == TileIndex::INVALID {
            continue;
        }
        if get_water_region_index_for_tile(map, adjacent_tile)
            != get_water_region_index_for_tile(map, tile)
        {
            invalidate_region(regions, adjacent_tile);
        }
    }
}

/// The progress of one `VisitWaterRegionPatchNeighbours` call, held by its caller
/// between steps; a zeroed record starts a visit. The C++ side declares it as
/// `OpenTTDWaterVisit`.
///
/// Each step returns the next neighbour the original would pass to its callback.
/// The pending labels stand for `VisitAdjacentWaterRegionPatchNeighbours`'s
/// function static `unique_labels`, here one buffer per visit.
#[repr(C)]
pub struct WaterVisit {
    /// The descriptor at the first step, whose region is the original's `current_region`.
    origin: WaterRegionPatchDesc,
    /// Region coordinates of the pending neighbour labels.
    neighbour_x: i32,
    neighbour_y: i32,
    /// Position of the cross-region aqueduct scan in the origin region's tile order.
    aqueduct_tile: u32,
    /// Neighbour labels found on the side last visited, read in order.
    labels: [WaterRegionPatchLabel; EDGE_TILES],
    /// Nonzero once the first step has run.
    started: u8,
    /// The next side to visit; `DIAGDIR_END` once every side is done.
    side: DiagDirection,
    count: u8,
    read: u8,
    /// Nonzero once `HasCrossRegionAqueducts` has been read.
    aqueducts_read: u8,
    has_cross_region_aqueducts: u8,
}
impl WaterVisit {
    /// The zeroed record that starts a visit.
    pub(crate) const fn new() -> Self {
        Self {
            origin: WaterRegionPatchDesc {
                x: 0,
                y: 0,
                label: INVALID_WATER_REGION_PATCH,
            },
            neighbour_x: 0,
            neighbour_y: 0,
            aqueduct_tile: 0,
            labels: [INVALID_WATER_REGION_PATCH; EDGE_TILES],
            started: 0,
            side: DiagDirection::DIAGDIR_BEGIN,
            count: 0,
            read: 0,
            aqueducts_read: 0,
            has_cross_region_aqueducts: 0,
        }
    }
}

/// `VisitAdjacentWaterRegionPatchNeighbours`: collects into the visit all water
/// region patches accessible from one particular side of the starting patch.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "HasBit takes the uint8_t bit position of the loop's int coordinate"
)]
fn visit_adjacent_water_region_patch_neighbours(
    regions: &mut Regions,
    world: &World<'_>,
    s: &Services,
    water_region_patch: WaterRegionPatchDesc,
    side: DiagDirection,
    visit: &mut WaterVisit,
) {
    if water_region_patch.label == INVALID_WATER_REGION_PATCH {
        return;
    }

    let current_region = get_updated_water_region(
        regions,
        world,
        s,
        to_region_coordinate(water_region_patch.x),
        to_region_coordinate(water_region_patch.y),
    );

    let map = regions.map;
    let offset = tile_index_diff_c_by_diag_dir(side, |ok| world.check(ok));
    let nx = water_region_patch.x + i32::from(offset.x);
    let ny = water_region_patch.y + i32::from(offset.y);

    if nx < 0
        || ny < 0
        || nx >= get_water_region_map_size_x(map)
        || ny >= get_water_region_map_size_y(regions.size_y)
    {
        return;
    }

    let neighbouring_region = get_updated_water_region(
        regions,
        world,
        s,
        to_region_coordinate(nx),
        to_region_coordinate(ny),
    );
    let opposite_side = reverse_diag_dir(side, |ok| world.check(ok));

    // Indicates via which local x or y coordinates (depends on the "side" parameter) we can cross over into the adjacent region.
    let traversability_bits = current_region.get_edge_traversability_bits(regions, side)
        & neighbouring_region.get_edge_traversability_bits(regions, opposite_side);
    if traversability_bits == 0 {
        return;
    }

    visit.neighbour_x = nx;
    visit.neighbour_y = ny;
    if current_region.number_of_patches(regions) == 1
        && neighbouring_region.number_of_patches(regions) == 1
    {
        // No further checks needed because we know there is just one patch for both adjacent regions
        visit.labels[0] = FIRST_REGION_LABEL;
        visit.count = 1;
        return;
    }

    // Multiple water patches can be reached from the current patch. Check each edge tile individually.
    // The visit's label buffer is the original's unique_labels: static and vector-instead-of-map for
    // performance reasons, emptied above for each side.
    for x_or_y in 0..WATER_REGION_EDGE_LENGTH {
        if !has_bit(traversability_bits, x_or_y as u8) {
            continue;
        }

        let current_edge_tile = get_edge_tile_coordinate(
            world,
            map,
            water_region_patch.x,
            water_region_patch.y,
            side,
            x_or_y,
        );
        let current_label = current_region.get_label(regions, world, current_edge_tile);
        if current_label != water_region_patch.label {
            continue;
        }

        let neighbour_edge_tile =
            get_edge_tile_coordinate(world, map, nx, ny, opposite_side, x_or_y);
        let neighbour_label = neighbouring_region.get_label(regions, world, neighbour_edge_tile);
        world.assert_with(|| neighbour_label != INVALID_WATER_REGION_PATCH);
        let found = &visit.labels[..usize::from(visit.count)];
        if !found.contains(&neighbour_label) {
            visit.labels[usize::from(visit.count)] = neighbour_label;
            visit.count += 1;
        }
    }
}

/// One step of `VisitWaterRegionPatchNeighbours`: the next of all accessible water
/// region patches in each cardinal direction, plus any others that are reachable via
/// aqueducts, or `None` once there are no more.
///
/// `water_region_patch` is the caller's descriptor as it is now: the original takes
/// it by const reference, and a callback may change it between steps.
fn visit_water_region_patch_neighbours(
    regions: &mut Regions,
    s: &Services,
    visit: &mut WaterVisit,
    water_region_patch: WaterRegionPatchDesc,
) -> Option<WaterRegionPatchDesc> {
    let world = World::new(s);
    if visit.started == 0 {
        visit.started = 1;
        visit.origin = water_region_patch;
        if water_region_patch.label != INVALID_WATER_REGION_PATCH {
            get_updated_water_region(
                regions,
                &world,
                s,
                to_region_coordinate(water_region_patch.x),
                to_region_coordinate(water_region_patch.y),
            );
        }
    }
    if visit.origin.label == INVALID_WATER_REGION_PATCH {
        return None;
    }

    // Visit adjacent water region patches in each cardinal direction
    loop {
        if visit.read < visit.count {
            let label = visit.labels[usize::from(visit.read)];
            visit.read += 1;
            return Some(WaterRegionPatchDesc {
                x: visit.neighbour_x,
                y: visit.neighbour_y,
                label,
            });
        }
        if visit.side == DiagDirection::DIAGDIR_END {
            break;
        }
        let side = visit.side;
        visit.side = DiagDirection(side.0 + 1);
        visit.count = 0;
        visit.read = 0;
        visit_adjacent_water_region_patch_neighbours(
            regions,
            &world,
            s,
            water_region_patch,
            side,
            visit,
        );
    }

    // Visit neighbouring water patches accessible via cross-region aqueducts.
    // The original's current_region refers to the live data: read its flag after
    // the side visits, without forcing another update.
    let map = regions.map;
    let current_region = WaterRegion::new(
        map,
        to_region_coordinate(visit.origin.x),
        to_region_coordinate(visit.origin.y),
    );
    if visit.aqueducts_read == 0 {
        visit.aqueducts_read = 1;
        visit.has_cross_region_aqueducts =
            u8::from(current_region.has_cross_region_aqueducts(regions));
    }
    if visit.has_cross_region_aqueducts != 0 {
        let edge = WATER_REGION_EDGE_LENGTH.cast_unsigned();
        while visit.aqueduct_tile < WATER_REGION_NUMBER_OF_TILES.cast_unsigned() {
            // The OrthogonalTileIterator position over the region's tile area.
            let tile = map.tile_xy(
                map.tile_x(current_region.tile()) + visit.aqueduct_tile % edge,
                map.tile_y(current_region.tile()) + visit.aqueduct_tile / edge,
            );
            visit.aqueduct_tile += 1;
            if get_water_region_patch_info(regions, &world, s, tile) != water_region_patch {
                continue;
            }
            crate::witness::hit(crate::witness::WATER_AQUEDUCTS);
            if !is_aqueduct_tile(&world, tile) {
                continue;
            }
            let other_end_tile = get_other_bridge_end(s, tile);
            if get_water_region_index_for_tile(map, tile)
                != get_water_region_index_for_tile(map, other_end_tile)
            {
                return Some(get_water_region_patch_info(
                    regions,
                    &world,
                    s,
                    other_end_tile,
                ));
            }
        }
    }
    None
}

/// `PrintWaterRegionDebugInfo`.
fn print_water_region_debug_info(regions: &mut Regions, s: &Services, tile: TileIndex) {
    let world = World::new(s);
    get_updated_water_region_for_tile(regions, &world, s, tile).print_debug_info(regions, &world);
}

/// The owner for a newly allocated map of `size_x` by `size_y` tiles.
/// # Safety
/// `shared` is the live shared services table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_new(
    shared: *const Services,
    size_x: u32,
    size_y: u32,
) -> *mut Regions {
    let world = World::new(unsafe { &*shared });
    Box::into_raw(Box::new(Regions::allocate(&world, size_x, size_y)))
}

/// Destroy an owner from [`openttd_rust_water_new`].
/// # Safety
/// `owner` came from [`openttd_rust_water_new`] and is destroyed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_destroy(owner: *mut Regions) {
    drop(unsafe { Box::from_raw(owner) });
}

/// The label of `GetWaterRegionPatchInfo(tile)`.
/// # Safety
/// As the module contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_label(
    owner: *mut Regions,
    shared: *const Services,
    tile: u32,
) -> u8 {
    let (regions, s) = unsafe { (&mut *owner, &*shared) };
    let world = World::new(s);
    get_water_region_patch_info(regions, &world, s, TileIndex(tile))
        .label
        .0
}

/// `InvalidateWaterRegion`.
/// # Safety
/// As the module contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_invalidate(
    owner: *mut Regions,
    shared: *const Services,
    tile: u32,
) {
    let (regions, s) = unsafe { (&mut *owner, &*shared) };
    invalidate_water_region(regions, s, TileIndex(tile));
}

/// The next neighbour of a `VisitWaterRegionPatchNeighbours` visit, written to
/// `out`; returns 0 once the visit is done.
/// # Safety
/// As the module contract; `visit` and `out` are exclusive for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_visit_next(
    owner: *mut Regions,
    shared: *const Services,
    visit: *mut WaterVisit,
    water_region_patch: WaterRegionPatchDesc,
    out: *mut WaterRegionPatchDesc,
) -> u8 {
    let (regions, s, visit) = unsafe { (&mut *owner, &*shared, &mut *visit) };
    match visit_water_region_patch_neighbours(regions, s, visit, water_region_patch) {
        Some(patch) => {
            unsafe { out.write(patch) };
            1
        }
        None => 0,
    }
}

/// `PrintWaterRegionDebugInfo`.
/// # Safety
/// As the module contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_water_print_debug_info(
    owner: *mut Regions,
    shared: *const Services,
    tile: u32,
) {
    let (regions, s) = unsafe { (&mut *owner, &*shared) };
    print_water_region_debug_info(regions, s, TileIndex(tile));
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDWaterPatch" ["probe"] => WaterRegionPatchDesc {
        x,
        y,
        label,
    };
    "OpenTTDWaterVisit" ["probe"] => WaterVisit {
        origin,
        neighbour_x,
        neighbour_y,
        aqueduct_tile,
        labels,
        started,
        side,
        count,
        read,
        aqueducts_read,
        has_cross_region_aqueducts,
    };
}

#[cfg(test)]
mod tests {
    use super::{Regions, WaterRegionPatchLabel, get_water_region_patch_info};
    use crate::map_func::{BaseTileLayout, ExtendedTileLayout, TileTypeAllocation, World};
    use crate::tile_type::{TileIndex, TileType};
    use std::cell::RefCell;

    const SIZE_X: u32 = 32;
    /// The owner's 32 by 32 map, with 32 void rows below it: the follower reads the
    /// tile past the southern edge, which a generated map holds as void.
    const TILES: u32 = 32 * 64;

    thread_local! {
        static BASE: RefCell<Vec<BaseTileLayout>> = const { RefCell::new(Vec::new()) };
        static EXTENDED: RefCell<Vec<ExtendedTileLayout>> = const { RefCell::new(Vec::new()) };
    }

    extern "C" fn native_assert(condition: bool) {
        assert!(condition, "native assertion");
    }
    extern "C" fn random(_: *mut std::ffi::c_void) -> u32 {
        unreachable!("no draw")
    }
    extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
        unreachable!("no industry")
    }
    extern "C" fn tile_type(_: u32) -> u8 {
        unreachable!("the raw map backend serves every tile")
    }

    /// Install a map where `water` tiles are clear sea, the rest of the owner's map
    /// is clear land and its `Map::MaxX` column and the guard rows are void.
    fn install(water: impl Fn(u32) -> bool) {
        let tile = |t: u32| {
            let kind = if t % SIZE_X == SIZE_X - 1 || t >= SIZE_X * SIZE_X {
                TileType::MP_VOID
            } else if water(t) {
                TileType::MP_WATER
            } else {
                TileType::MP_CLEAR
            };
            BaseTileLayout {
                kind: kind.0 << 4,
                height: 0,
                m2: 0,
                m1: 0,
                m3: 0,
                m4: 0,
                m5: 0,
            }
        };
        BASE.with_borrow_mut(|b| *b = (0..TILES).map(tile).collect());
        EXTENDED.with_borrow_mut(|e| {
            *e = (0..TILES)
                .map(|_| ExtendedTileLayout {
                    m6: 0,
                    m7: 0,
                    m8: 0,
                })
                .collect();
        });
        let allocation = TileTypeAllocation {
            base: BASE.with_borrow(Vec::as_ptr),
            extended: EXTENDED.with_borrow(Vec::as_ptr),
            count: TILES,
            assertions: true,
        };
        crate::map_func::fixture::set(allocation, SIZE_X.trailing_zeros());
    }

    /// The shared table and the movement registry that points back at it, leaked for the test.
    fn services() -> &'static crate::services::Services {
        let s = Box::leak(Box::new(crate::services::fixture(
            std::ptr::null_mut(),
            random,
            industry,
        )));
        s.map = crate::map_func::fixture::statics();
        s.tile_type = tile_type;
        let mut movement = crate::train_reservation::tests::movement_table();
        movement.native_assert = native_assert;
        movement.shared = std::ptr::from_ref(s);
        s.movement = Box::leak(Box::new(movement));
        s
    }

    /// The stored edge masks and patch counts, which no export reads, after the
    /// production update of the north-west region.
    #[test]
    fn force_update_stores_the_fixture_masks_and_patch_counts() {
        let s = services();
        let mut regions = Regions::new(SIZE_X, SIZE_X);

        // All water: one patch, traversable along both southern edges, and the
        // north-eastern and north-western edges lie on the map border.
        install(|_| true);
        let world = World::new(s);
        let patch = get_water_region_patch_info(&mut regions, &world, s, TileIndex(0));
        assert!(patch.label == WaterRegionPatchLabel(1));
        let data = &regions.water_region_data[0];
        assert_eq!(data.number_of_patches, 1);
        assert_eq!(data.edge_traversability_bits, [0, u16::MAX, u16::MAX, 0]);
        assert!(data.tile_patch_labels.is_none() && !data.has_cross_region_aqueducts);

        // Two separate water tiles: two patches, no edge reaches another region.
        install(|t| t == 0 || t == 2);
        regions.is_water_region_valid[0] = false;
        let world = World::new(s);
        let patch = get_water_region_patch_info(&mut regions, &world, s, TileIndex(2));
        assert!(patch.label == WaterRegionPatchLabel(2));
        let data = &regions.water_region_data[0];
        assert_eq!(data.number_of_patches, 2);
        assert_eq!(data.edge_traversability_bits, [0; 4]);
        assert!(data.tile_patch_labels.is_some());
    }
}
