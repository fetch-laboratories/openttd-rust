/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Station-owned custom road-stop tile data. Native views expire on mutation or
//! owner destruction. No borrow crosses a callback. Save loading reserves first
//! and publishes each default row before native field loading, preserving partial
//! rows when ordinary load errors return to C++.
#![allow(unsafe_code)]

use crate::industrytype::NativeVec;
use crate::station_record::{RoadStopTileData, StationRecord, record_of};
use crate::station_type::BaseStation;
use crate::tile_type::TileIndex;

fn edit<R>(record: *mut StationRecord, change: impl FnOnce(&mut Vec<RoadStopTileData>) -> R) -> R {
    // SAFETY: Serialized live owner; change is Rust-only and cannot reenter.
    let mut data = unsafe { std::mem::take(&mut (*record).custom_roadstop_tile_data) }.into_vec();
    let result = change(&mut data);
    unsafe { (*record).custom_roadstop_tile_data = NativeVec::from_vec(data) };
    result
}

/// Existing element count at the source loop entry.
pub(crate) fn roadstop_count(shell: *const BaseStation) -> usize {
    unsafe { (*record_of(shell)).custom_roadstop_tile_data.len() }
}

/// Reacquire an element after each callback; the caller preserves the source
/// live-index precondition rather than retaining a span across mutations.
pub(crate) fn roadstop_tile(shell: *const BaseStation, index: usize) -> u32 {
    unsafe {
        (*record_of(shell)).custom_roadstop_tile_data.as_slice()[index]
            .tile
            .0
    }
}

/// Source `GetRoadStopRandomBits` / `GetRoadStopAnimationFrame` default to zero.
pub(crate) fn roadstop_value(shell: *const BaseStation, tile: u32, animation: bool) -> u8 {
    unsafe { (*record_of(shell)).custom_roadstop_tile_data.as_slice() }
        .iter()
        .find(|entry| entry.tile.0 == tile)
        .map_or(0, |entry| {
            if animation {
                entry.animation_frame
            } else {
                entry.random_bits
            }
        })
}

/// Source `SetRoadStopTileData`: a missing tile is inserted even for zero;
/// unchanged values return false and retain the element order.
pub(crate) fn set(record: *mut StationRecord, tile: u32, value: u8, animation: bool) -> bool {
    edit(record, |data| {
        for entry in &mut *data {
            if entry.tile.0 == tile {
                let field = if animation {
                    &mut entry.animation_frame
                } else {
                    &mut entry.random_bits
                };
                if *field == value {
                    return false;
                }
                *field = value;
                return true;
            }
        }
        data.push(RoadStopTileData {
            tile: TileIndex(tile),
            random_bits: if animation { 0 } else { value },
            animation_frame: if animation { value } else { 0 },
        });
        value != 0
    })
}

pub(crate) fn roadstop_set(
    shell: *const BaseStation,
    tile: u32,
    value: u8,
    animation: bool,
) -> bool {
    set(unsafe { record_of(shell) }, tile, value, animation)
}

pub(crate) fn remove(record: *mut StationRecord, tile: u32) {
    edit(record, |data| {
        if let Some(index) = data.iter().position(|entry| entry.tile.0 == tile) {
            data.swap_remove(index);
        }
    });
}

pub(crate) fn roadstop_remove(shell: *const BaseStation, tile: u32) {
    remove(unsafe { record_of(shell) }, tile);
}

/// # Safety
/// Live serialized station record. Element aliases end before this operation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_roadstop_data_set(
    record: *mut StationRecord,
    tile: u32,
    value: u8,
    animation: u8,
) -> u8 {
    u8::from(set(record, tile, value, animation != 0))
}

/// # Safety
/// Same live-owner and invalidation contract as the setter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_roadstop_data_remove(
    record: *mut StationRecord,
    tile: u32,
) {
    remove(record, tile);
}

/// # Safety
/// Same live-owner and invalidation contract as the setter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_roadstop_data_clear(record: *mut StationRecord) {
    edit(record, Vec::clear);
}

/// Source vector.reserve(count), before publishing any newly loaded row.
/// # Safety
/// The live record is held across the native save scope; no element alias survives.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_roadstop_data_reserve(
    record: *mut StationRecord,
    count: usize,
) {
    edit(record, |data| {
        data.reserve(count.saturating_sub(data.len()));
    });
}

/// Publish a default row before native `SlObject`; an error leaves its partial fields.
/// # Safety
/// Same save-scope contract; the returned row expires on the next vector mutation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_station_roadstop_data_append(
    record: *mut StationRecord,
) -> *mut RoadStopTileData {
    edit(record, |data| {
        data.push(RoadStopTileData {
            tile: TileIndex(u32::MAX),
            random_bits: 0,
            animation_frame: 0,
        });
        std::ptr::from_mut(data.last_mut().unwrap())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_insertion_swap_remove_and_partial_load_publication() {
        let mut owner = StationRecord::new(std::ptr::null_mut(), TileIndex(1));
        let record = &raw mut owner;
        assert!(!set(record, 11, 0, true));
        assert_eq!(owner.custom_roadstop_tile_data.len(), 1);
        assert!(!set(record, 11, 0, false));
        assert!(set(record, 11, 7, false));
        assert!(set(record, 12, 3, true));
        assert!(set(record, 13, 8, false));
        remove(record, 12);
        remove(record, 99);
        assert_eq!(
            owner
                .custom_roadstop_tile_data
                .as_slice()
                .iter()
                .map(|r| r.tile.0)
                .collect::<Vec<_>>(),
            [11, 13]
        );
        unsafe {
            openttd_rust_station_roadstop_data_reserve(record, 5);
            let row = openttd_rust_station_roadstop_data_append(record);
            assert_eq!((*row).tile.0, u32::MAX);
            (*row).tile = TileIndex(14); // A native load can stop after just this field.
            assert_eq!(owner.custom_roadstop_tile_data.len(), 3);
            assert_eq!(owner.custom_roadstop_tile_data.as_slice()[2].random_bits, 0);
            openttd_rust_station_roadstop_data_clear(record);
        }
        assert!(owner.custom_roadstop_tile_data.is_empty());
    }
}
