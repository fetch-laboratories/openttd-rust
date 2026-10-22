/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `economy_type.h` `Price` indices. `services.cpp` asserts every value.

/// `PR_BUILD_SIGNALS`.
pub const PR_BUILD_SIGNALS: usize = 3;
/// `PR_BUILD_DEPOT_TRAIN`.
pub const PR_BUILD_DEPOT_TRAIN: usize = 5;
/// `PR_CLEAR_ROUGH`.
pub const PR_CLEAR_ROUGH: usize = 23;
/// `PR_CLEAR_SIGNALS`.
pub const PR_CLEAR_SIGNALS: usize = 28;
/// `PR_CLEAR_DEPOT_TRAIN`.
pub const PR_CLEAR_DEPOT_TRAIN: usize = 30;
/// `PR_CLEAR_WATER`.
pub const PR_CLEAR_WATER: usize = 34;
/// `PR_BUILD_FOUNDATION`.
pub const PR_BUILD_FOUNDATION: usize = 57;
