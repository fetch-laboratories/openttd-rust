/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Constants shared from `station_base.h`.

/// `INITIAL_STATION_RATING`.
pub const INITIAL_STATION_RATING: u8 = 175;

/// `GoodsEntry::State` positions from the native header.
pub const GE_ACCEPTANCE: u8 = 0;
pub const GE_RATING: u8 = 1;
pub const GE_EVER_ACCEPTED: u8 = 2;
pub const GE_LAST_MONTH: u8 = 3;
pub const GE_CURRENT_MONTH: u8 = 4;
pub const GE_ACCEPTED_BIGTICK: u8 = 5;
/// `GoodsEntry::HasRating`.
pub(crate) fn has_rating(status: u8) -> bool {
    status & (1 << GE_RATING) != 0
}
/// `GoodsEntry::HasVehicleEverTriedLoading`.
pub(crate) fn has_vehicle_ever_tried_loading(last_speed: u8) -> bool {
    last_speed != 0
}
