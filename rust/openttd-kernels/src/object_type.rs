/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Consumed native header constants, asserted in services.cpp.
/// Native `OBJECT_STATUE`.
pub const OBJECT_STATUE: u16 = ObjectType::OBJECT_STATUE.0;

/// Native `ObjectID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectID(pub u32);

impl ObjectID {
    /// Native `PoolID::Begin()`.
    pub const BEGIN: Self = Self(0);
    /// Native `PoolID::End()`.
    pub const END: Self = Self(16_711_680);
    /// Native `PoolID::Invalid()`.
    pub const INVALID: Self = Self(4_294_967_295);
}

/// Native `ObjectAnimationTrigger`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectAnimationTrigger(pub u8);

#[allow(non_upper_case_globals)]
impl ObjectAnimationTrigger {
    /// Triggered when the object is built (for all tiles at the same time).
    pub const Built: Self = Self(0);
    /// Triggered in the periodic tile loop.
    pub const TileLoop: Self = Self(1);
    /// Triggered every 256 ticks (for all tiles at the same time).
    pub const TileLoopNorth: Self = Self(2);
}

impl ObjectAnimationTrigger {
    /// Compatibility spelling of `Built`.
    pub const BUILT: Self = Self::Built;
    /// Compatibility spelling of `TileLoop`.
    pub const TILE_LOOP: Self = Self::TileLoop;
    /// Compatibility spelling of `TileLoopNorth`.
    pub const TILE_LOOP_NORTH: Self = Self::TileLoopNorth;
}

/// Native `ObjectAnimationTriggers`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectAnimationTriggers(pub u16);

impl ObjectAnimationTriggers {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: ObjectAnimationTrigger) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}

impl ObjectAnimationTriggers {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: ObjectAnimationTrigger) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Native `ObjectType`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectType(pub u16);

crate::base_bitset_type::impl_bitset_ops!(ObjectAnimationTriggers);

impl ObjectType {
    /// The large antenna.
    pub const OBJECT_TRANSMITTER: Self = Self(0);
    /// The nice lighthouse.
    pub const OBJECT_LIGHTHOUSE: Self = Self(1);
    /// Statue in towns.
    pub const OBJECT_STATUE: Self = Self(2);
    /// Owned land flag.
    pub const OBJECT_OWNED_LAND: Self = Self(3);
    /// Headquarters of a player.
    pub const OBJECT_HQ: Self = Self(4);
    /// Offset for new objects.
    pub const NEW_OBJECT_OFFSET: Self = Self(5);
    /// Number of supported objects overall.
    pub const NUM_OBJECTS: Self = Self(64000);
    /// Number of supported objects per `NewGRF`.
    pub const NUM_OBJECTS_PER_GRF: Self = Self::NUM_OBJECTS;
    /// An invalid object type.
    pub const INVALID_OBJECT_TYPE: Self = Self(0xFFFF);
}
