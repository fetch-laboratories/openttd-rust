/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Types used by YAPF (`pathfinder/yapf/yapf_type.hpp`).

/// Enum used in `PfCalcCost()` to see why was the segment closed.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct EndSegmentReason(pub u8);

impl EndSegmentReason {
    /* The following reasons can be saved into cached segment */
    /// Track ends here.
    pub(crate) const DEAD_END: Self = Self(0);
    /// The next tile has a different rail type than our tiles.
    pub(crate) const RAIL_TYPE: Self = Self(1);
    /// Infinite loop detected.
    pub(crate) const INFINITE_LOOP: Self = Self(2);
    /// The segment is too long (possible infinite loop).
    pub(crate) const SEGMENT_TOO_LONG: Self = Self(3);
    /// The next tile contains a choice (the track splits to more than one segments).
    pub(crate) const CHOICE_FOLLOWS: Self = Self(4);
    /// Stop in the depot (could be a target next time).
    pub(crate) const DEPOT: Self = Self(5);
    /// Waypoint encountered (could be a target next time).
    pub(crate) const WAYPOINT: Self = Self(6);
    /// Station encountered (could be a target next time).
    pub(crate) const STATION: Self = Self(7);
    /// Safe waiting position found (could be a target).
    pub(crate) const SAFE_TILE: Self = Self(8);

    /* The following reasons are used only internally by PfCalcCost().
     *  They should not be found in the cached segment. */
    /// The path is too long (searching for the nearest depot in the given radius).
    pub(crate) const PATH_TOO_LONG: Self = Self(9);
    /// First signal was 2-way and it was red.
    pub(crate) const FIRST_TWO_WAY_RED: Self = Self(10);
    // `LookAheadEnd` (11) and `TargetReached` (12) follow; the original sets
    // neither and no mask names them.
}

/// `EndSegmentReasons`: `EnumBitSet<EndSegmentReason, uint16_t>`.
#[repr(transparent)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct EndSegmentReasons(pub u16);

impl EndSegmentReasons {
    /// The set of the given reasons.
    #[must_use]
    pub(crate) const fn of(reasons: &[EndSegmentReason]) -> Self {
        let mut bits = 0;
        let mut i = 0;
        while i < reasons.len() {
            bits |= 1 << reasons[i].0;
            i += 1;
        }
        Self(bits)
    }
    /// `Set`: add a reason.
    #[inline]
    pub(crate) fn set(&mut self, reason: EndSegmentReason) {
        self.0 |= 1 << reason.0;
    }
    /// `Set`: add every reason of a set.
    #[inline]
    pub(crate) fn set_all(&mut self, reasons: Self) {
        self.0 |= reasons.0;
    }
    /// `Test`: whether a reason is set.
    #[inline]
    #[must_use]
    pub(crate) const fn test(self, reason: EndSegmentReason) -> bool {
        self.0 & (1 << reason.0) != 0
    }
    /// `Any()`: whether any reason is set.
    #[inline]
    #[must_use]
    pub(crate) const fn any(self) -> bool {
        self.0 != 0
    }
    /// `Any(other)`: whether any reason of `other` is set.
    #[inline]
    #[must_use]
    pub(crate) const fn any_of(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl std::ops::BitAnd for EndSegmentReasons {
    type Output = Self;
    #[inline]
    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

/* What reasons mean that the target can be found and needs to be detected. */
pub(crate) const ESRF_POSSIBLE_TARGET: EndSegmentReasons = EndSegmentReasons::of(&[
    EndSegmentReason::DEPOT,
    EndSegmentReason::WAYPOINT,
    EndSegmentReason::STATION,
    EndSegmentReason::SAFE_TILE,
]);

/* What reasons can be stored back into cached segment. */
pub(crate) const ESRF_CACHED_MASK: EndSegmentReasons = EndSegmentReasons::of(&[
    EndSegmentReason::DEAD_END,
    EndSegmentReason::RAIL_TYPE,
    EndSegmentReason::INFINITE_LOOP,
    EndSegmentReason::SEGMENT_TOO_LONG,
    EndSegmentReason::CHOICE_FOLLOWS,
    EndSegmentReason::DEPOT,
    EndSegmentReason::WAYPOINT,
    EndSegmentReason::STATION,
    EndSegmentReason::SAFE_TILE,
]);

/* Reasons to abort pathfinding in this direction. */
pub(crate) const ESRF_ABORT_PF_MASK: EndSegmentReasons = EndSegmentReasons::of(&[
    EndSegmentReason::DEAD_END,
    EndSegmentReason::PATH_TOO_LONG,
    EndSegmentReason::INFINITE_LOOP,
    EndSegmentReason::FIRST_TWO_WAY_RED,
]);

#[cfg(test)]
mod tests {
    use super::{ESRF_ABORT_PF_MASK, ESRF_CACHED_MASK, ESRF_POSSIBLE_TARGET};

    #[test]
    fn masks_hold_the_bits_of_yapf_type_hpp() {
        // Bit positions follow the declaration order of `EndSegmentReason` in
        // the original `yapf_type.hpp`: DeadEnd 0 ... SafeTile 8, PathTooLong 9,
        // FirstTwoWayRed 10.
        assert_eq!(ESRF_POSSIBLE_TARGET.0, 0b1_1110_0000);
        assert_eq!(ESRF_CACHED_MASK.0, 0b1_1111_1111);
        assert_eq!(ESRF_ABORT_PF_MASK.0, 0b110_0000_0101);
    }
}
