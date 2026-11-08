/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Declaration of basic house types and enums from `house_type.h`.

/// `OpenTTD` ID of house types.
pub type HouseID = u16;
/// Classes of houses.
pub type HouseClassID = u16;

/// Randomisation triggers for houses from `house_type.h`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseRandomTrigger(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "Preserves HouseRandomTrigger enumerator spelling from house_type.h"
)]
impl HouseRandomTrigger {
    /// The tile of the house has been triggered during the tileloop.
    pub const TileLoop: Self = Self(0);
    /// The top tile of a multitile building was triggered; the other tiles
    /// of the same building get the same random value during their tileloop.
    pub const TileLoopNorth: Self = Self(1);
}

/// House randomisation trigger set, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseRandomTriggers(pub u8);

impl HouseRandomTriggers {
    /// Native element-to-set conversion.
    #[must_use]
    pub const fn from_element(element: HouseRandomTrigger) -> Self {
        Self(1 << element.0)
    }
    /// Native `Set` of one element.
    pub fn set(&mut self, element: HouseRandomTrigger) {
        self.0 |= Self::from_element(element).0;
    }
    /// Native `Reset` of the given set.
    pub fn reset(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}
