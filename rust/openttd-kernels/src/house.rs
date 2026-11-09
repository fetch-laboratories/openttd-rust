/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Definitions of classes and constants related to houses from `house.h`.
//! Native specification storage remains shared.
use crate::house_type::{HouseClassID, HouseID};
use crate::landscape_type::LandscapeType;

/// Simple value that indicates the house has reached the final stage of construction.
pub const TOWN_HOUSE_COMPLETED: u8 = 3;
/// No house class is assigned.
pub const HOUSE_NO_CLASS: HouseClassID = 0;
/// Offset for new houses.
pub const NEW_HOUSE_OFFSET: HouseID = 110;
/// Total number of houses.
pub const NUM_HOUSES: HouseID = 4096;
/// Invalid house ID sentinel.
pub const INVALID_HOUSE_ID: HouseID = u16::MAX;
/// Number of supported houses per `NewGRF`.
pub const NUM_HOUSES_PER_GRF: HouseID = NUM_HOUSES;
/// Max number of cargoes accepted by a tile.
pub const HOUSE_NUM_ACCEPTS: u8 = 16;
/// Original number of accepted cargo types.
pub const HOUSE_ORIGINAL_NUM_ACCEPTS: u8 = 3;
/// Raw adapter for `HouseZone::TownEdge`, retained for the `NewGRF` road-stop resolver's later pass.
pub const HOUSE_ZONE_TOWN_EDGE: u8 = HouseZone::TownEdge.0;

/// Get the `HouseZones` climate mask for the current landscape type; declared in `house.h`
/// (`GetClimateMaskForLandscape`), with the native unreachable landscape domain.
pub(crate) fn get_climate_mask_for_landscape(landscape: LandscapeType) -> HouseZones {
    match landscape {
        LandscapeType::Temperate => HouseZones::from_element(HouseZone::ClimateTemperate),
        LandscapeType::Arctic => HouseZones::from_element(HouseZone::ClimateSubarcticAboveSnow)
            .with(HouseZones::from_element(
                HouseZone::ClimateSubarcticBelowSnow,
            )),
        LandscapeType::Tropic => HouseZones::from_element(HouseZone::ClimateSubtropic),
        LandscapeType::Toyland => HouseZones::from_element(HouseZone::ClimateToyland),
        _ => unreachable!("landscape"),
    }
}

/// Opaque native house specification, retained by the GRF/spec owner.
#[repr(C)]
pub struct HouseSpec {
    _private: [u8; 0],
}

/// Native `BuildingFlag` bit positions, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildingFlag(pub u8);

#[allow(non_upper_case_globals)]
impl BuildingFlag {
    /// Native `Size1x1`.
    pub const Size1x1: Self = Self(0);
    /// Native `NotSloped`.
    pub const NotSloped: Self = Self(1);
    /// Native `Size2x1`.
    pub const Size2x1: Self = Self(2);
    /// Native `Size1x2`.
    pub const Size1x2: Self = Self(3);
    /// Native `Size2x2`.
    pub const Size2x2: Self = Self(4);
    /// Native `IsAnimated`.
    pub const IsAnimated: Self = Self(5);
    /// Native `IsChurch`.
    pub const IsChurch: Self = Self(6);
    /// Native `IsStadium`.
    pub const IsStadium: Self = Self(7);
}

/// Native `BuildingFlags`, retaining all size and property bits.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BuildingFlags(pub u8);

impl BuildingFlags {
    /// Native footprint masks, including all north-tile sizes.
    pub const BUILDING_HAS_1_TILE: Self =
        Self::from_element(BuildingFlag::Size1x1).with(Self::BUILDING_HAS_2_TILES);
    /// All multitile north-tile sizes.
    pub const BUILDING_HAS_2_TILES: Self = Self::from_element(BuildingFlag::Size2x1)
        .with(Self::from_element(BuildingFlag::Size1x2))
        .with(Self::BUILDING_HAS_4_TILES);
    /// North tile of a four-tile house.
    pub const BUILDING_HAS_4_TILES: Self = Self::from_element(BuildingFlag::Size2x2);
    /// Footprints extending along X.
    pub const BUILDING_2_TILES_X: Self =
        Self::from_element(BuildingFlag::Size2x1).with(Self::BUILDING_HAS_4_TILES);
    /// Footprints extending along Y.
    pub const BUILDING_2_TILES_Y: Self =
        Self::from_element(BuildingFlag::Size1x2).with(Self::BUILDING_HAS_4_TILES);

    /// Native element-to-set conversion.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: BuildingFlag) -> Self {
        Self((1_u64 << element.0) as u8)
    }
    /// Native set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Native `Reset` of the given bits.
    #[must_use]
    pub const fn without(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
    /// Native `Any` test.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: BuildingFlag) -> bool {
        self.any(Self::from_element(element))
    }
}

crate::base_bitset_type::impl_bitset_ops!(BuildingFlags);

/// Native `HouseZone` bit positions, retaining the full byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HouseZone(pub u8);

#[allow(non_upper_case_globals)]
impl HouseZone {
    /// Town edge.
    pub const TownEdge: Self = Self(0);
    /// Town outskirts.
    pub const TownOutskirt: Self = Self(1);
    /// Outer suburbs.
    pub const TownOuterSuburb: Self = Self(2);
    /// Inner suburbs.
    pub const TownInnerSuburb: Self = Self(3);
    /// Town centre.
    pub const TownCentre: Self = Self(4);
    /// End of town zones.
    pub const TownEnd: Self = Self(5);
    /// Building can appear in sub-arctic climate above the snow line.
    pub const ClimateSubarcticAboveSnow: Self = Self(11);
    /// Building can appear in temperate climate.
    pub const ClimateTemperate: Self = Self(12);
    /// Building can appear in sub-arctic climate below the snow line.
    pub const ClimateSubarcticBelowSnow: Self = Self(13);
    /// Building can appear in subtropical climate.
    pub const ClimateSubtropic: Self = Self(14);
    /// Building can appear in toyland climate.
    pub const ClimateToyland: Self = Self(15);
}

/// Native `HouseZones`, retaining every underlying availability bit.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HouseZones(pub u16);

impl HouseZones {
    /// All town zones.
    pub const HZ_ZONE_ALL: Self = Self::from_element(HouseZone::TownEdge)
        .with(Self::from_element(HouseZone::TownOutskirt))
        .with(Self::from_element(HouseZone::TownOuterSuburb))
        .with(Self::from_element(HouseZone::TownInnerSuburb))
        .with(Self::from_element(HouseZone::TownCentre));
    /// All climates, including both sides of the sub-arctic snow line.
    pub const HZ_CLIMATE_ALL: Self = Self::from_element(HouseZone::ClimateSubarcticAboveSnow)
        .with(Self::from_element(HouseZone::ClimateTemperate))
        .with(Self::from_element(HouseZone::ClimateSubarcticBelowSnow))
        .with(Self::from_element(HouseZone::ClimateSubtropic))
        .with(Self::from_element(HouseZone::ClimateToyland));

    /// Native element-to-set conversion.
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: HouseZone) -> Self {
        Self((1_u64 << element.0) as u16)
    }
    /// Native set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Native `Any` test.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
    /// Native `All` test.
    #[must_use]
    pub const fn all(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: HouseZone) -> bool {
        self.any(Self::from_element(element))
    }
    /// Native set iteration, visiting set elements in ascending bit order.
    pub fn iter(self) -> impl Iterator<Item = HouseZone> {
        // `HouseZones` stores 16 bits.
        (0..16_u8)
            .map(HouseZone)
            .filter(move |&zone| self.test(zone))
    }
}
crate::base_bitset_type::impl_bitset_ops!(HouseZones);

/// Number of town zones in `house.h`.
pub const NUM_HOUSE_ZONES: u32 = HouseZone::TownEnd.0 as u32;

/// Native `HouseExtraFlag` bit positions, retaining the full byte domain.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HouseExtraFlag(pub u8);

#[allow(non_upper_case_globals)]
impl HouseExtraFlag {
    /// This house only appears during town generation in random games, hence historical.
    pub const BuildingIsHistorical: Self = Self(0);
    /// Towns and AI will not remove this house, while human players can.
    pub const BuildingIsProtected: Self = Self(1);
    /// Synchronized callback 1B is performed on multitile houses.
    pub const SynchronisedCallback1B: Self = Self(2);
    /// Callback 1A needs random bits.
    pub const Callback1ARandomBits: Self = Self(3);
}

/// Native `HouseExtraFlags`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HouseExtraFlags(pub u8);

impl HouseExtraFlags {
    /// Native element-to-set conversion.
    #[must_use]
    pub const fn from_element(element: HouseExtraFlag) -> Self {
        Self(1 << element.0)
    }
    /// Native `Test` of one element.
    #[must_use]
    pub const fn test(self, element: HouseExtraFlag) -> bool {
        self.0 & Self::from_element(element).0 != 0
    }
}
