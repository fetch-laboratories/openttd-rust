/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `road_func.h` helpers.
/// `DiagDirToRoadBits`: create the road-part which belongs to the given `DiagDirection`.
pub fn diag_dir_to_road_bits(
    d: crate::direction_type::DiagDirection,
    check: impl Fn(bool),
) -> crate::road_type::RoadBits {
    if !crate::direction_func::is_valid_diag_direction(d) {
        check(false);
    }
    crate::road_type::RoadBits(
        (u32::from(crate::road_type::RoadBits::ROAD_NW.0).wrapping_shl(u32::from(3 ^ d.0)) & 0xff)
            as u8,
    )
}
/// Raw adapter: `DiagDirToRoadBits` over base values, for files no pass has finished yet.
pub fn diag_dir_to_road_bits_raw(dir: u8, check: impl Fn(bool)) -> u8 {
    diag_dir_to_road_bits(crate::direction_type::DiagDirection(dir), check).0
}

/// `MirrorRoadBits`, retaining `IsValidRoadBits`'s precondition.
pub fn mirror_road_bits(bits: u8, check: impl Fn(bool) + Copy) -> u8 {
    if bits >= crate::road_type::ROAD_END {
        check(false);
    }
    (bits & 3) << 2 | (bits >> 2) & 3
}

/// `AxisToRoadBits`: create the road-part which belongs to the given `Axis`.
pub(crate) fn axis_to_road_bits(
    a: crate::direction_type::Axis,
    check: impl Fn(bool) + Copy,
) -> crate::road_type::RoadBits {
    if !crate::direction_func::is_valid_axis(a) {
        check(false);
    }
    if a == crate::direction_type::Axis::AXIS_X {
        crate::road_type::RoadBits::ROAD_X
    } else {
        crate::road_type::RoadBits::ROAD_Y
    }
}
