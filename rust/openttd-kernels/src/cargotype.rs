/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Town cargo acceptance effects from `cargotype.h`.
/// Native `TAE_WATER`.
pub const TAE_WATER: u8 = TownAcceptanceEffect::TAE_WATER.0;
/// Native `TAE_FOOD`.
pub const TAE_FOOD: u8 = TownAcceptanceEffect::TAE_FOOD.0;
/// `CargoClass::Mail` mask.
pub const CC_MAIL: u16 = CargoClasses::from_element(CargoClass::Mail).0;
/// `CargoClass::Express` mask.
pub const CC_EXPRESS: u16 = CargoClasses::from_element(CargoClass::Express).0;
/// `CargoClass::Armoured` mask.
pub const CC_ARMOURED: u16 = CargoClasses::from_element(CargoClass::Armoured).0;
/// `CargoClass::Bulk` mask.
pub const CC_BULK: u16 = CargoClasses::from_element(CargoClass::Bulk).0;
/// `CargoClass::PieceGoods` mask.
pub const CC_PIECEGOODS: u16 = CargoClasses::from_element(CargoClass::PieceGoods).0;
/// `CargoClass::Liquid` mask.
pub const CC_LIQUID: u16 = CargoClasses::from_element(CargoClass::Liquid).0;
/// `CargoClass::Refrigerated` mask.
pub const CC_REFRIGERATED: u16 = CargoClasses::from_element(CargoClass::Refrigerated).0;

/// Invalid specification bit number.
pub const INVALID_CARGO_BITNUM: u8 = 255;
/// Town production effect from `cargotype.h`.
pub const TPE_NONE: u8 = TownProductionEffect::TPE_NONE.0;
/// Town production effect from `cargotype.h`.
pub const TPE_PASSENGERS: u8 = TownProductionEffect::TPE_PASSENGERS.0;
/// Town production effect from `cargotype.h`.
pub const TPE_MAIL: u8 = TownProductionEffect::TPE_MAIL.0;
/// Town production effect from `cargotype.h`.
pub const NUM_TPE: u8 = TownProductionEffect::NUM_TPE.0;
/// Town production effect from `cargotype.h`.
pub const INVALID_TPE: u8 = TownProductionEffect::INVALID_TPE.0;

/// Tests for validity of this cargospec (`CargoSpec::IsValid`).
/// `assert(cs->IsValid())` can be triggered when GRF config is modified.
#[must_use]
pub const fn is_cargo_spec_valid(bitnum: u8) -> bool {
    bitnum != INVALID_CARGO_BITNUM
}

/// Returns an iterable ensemble of all valid `CargoSpec` (`CargoSpec::Iterate`).
/// The default first slot is zero; each slot is observed when advancing, without a world borrow.
pub(crate) fn iterate<T>(
    mut load: impl FnMut(u8) -> T,
    mut valid: impl FnMut(&T) -> bool,
) -> impl Iterator<Item = (u8, T)> {
    (0..crate::cargo_type::NUM_CARGO)
        .map(move |cargo| {
            let cargo = crate::cargo_type::CargoType(cargo);
            (cargo.0, load(cargo.0))
        })
        .filter(move |(_, spec)| valid(spec))
}

/// `CargoClass::Passengers`.
pub const PASSENGERS: u16 = CargoClasses::from_element(CargoClass::Passengers).0;
/// Does cargo have cargo class `cc`? (`IsCargoInClass`).
/// Classes are read from the canonical native cargo spec; `requested` is a mask.
#[must_use]
pub const fn is_cargo_in_class(classes: u16, requested: u16) -> bool {
    let classes = CargoClasses(classes);
    let cc = CargoClasses(requested);
    classes.0 & cc.0 != CargoClasses::EMPTY.0
}

/// Native `TownAcceptanceEffect`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TownAcceptanceEffect(pub u8);

impl TownAcceptanceEffect {
    /// Native `TAE_BEGIN`.
    pub const TAE_BEGIN: Self = Self(0);
    /// Cargo has no effect.
    pub const TAE_NONE: Self = Self(0);
    /// Cargo behaves passenger-like.
    pub const TAE_PASSENGERS: Self = Self(1);
    /// Cargo behaves mail-like.
    pub const TAE_MAIL: Self = Self(2);
    /// Cargo behaves goods/candy-like.
    pub const TAE_GOODS: Self = Self(3);
    /// Cargo behaves water-like.
    pub const TAE_WATER: Self = Self(4);
    /// Cargo behaves food/fizzy-drinks-like.
    pub const TAE_FOOD: Self = Self(5);
    /// End of town effects.
    pub const TAE_END: Self = Self(6);
    /// Amount of town effects.
    pub const NUM_TAE: Self = Self(6);
}

/// Native `TownProductionEffect`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TownProductionEffect(pub u8);

impl TownProductionEffect {
    /// Town will not produce this cargo type.
    pub const TPE_NONE: Self = Self(0);
    /// Cargo behaves passenger-like for production.
    pub const TPE_PASSENGERS: Self = Self(1);
    /// Cargo behaves mail-like for production.
    pub const TPE_MAIL: Self = Self(2);
    /// Native `NUM_TPE`.
    pub const NUM_TPE: Self = Self(3);
    /// Native `INVALID_TPE`.
    pub const INVALID_TPE: Self = Self(4);
}

/// Native `CargoClass`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoClass(pub u8);

#[allow(non_upper_case_globals)]
impl CargoClass {
    /// Passengers
    pub const Passengers: Self = Self(0);
    /// Mail
    pub const Mail: Self = Self(1);
    /// Express cargo (Goods, Food, Candy, but also possible for passengers)
    pub const Express: Self = Self(2);
    /// Armoured cargo (Valuables, Gold, Diamonds)
    pub const Armoured: Self = Self(3);
    /// Bulk cargo (Coal, Grain etc., Ores, Fruit)
    pub const Bulk: Self = Self(4);
    /// Piece goods (Livestock, Wood, Steel, Paper)
    pub const PieceGoods: Self = Self(5);
    /// Liquids (Oil, Water, Rubber)
    pub const Liquid: Self = Self(6);
    /// Refrigerated cargo (Food, Fruit)
    pub const Refrigerated: Self = Self(7);
    /// Hazardous cargo (Nuclear Fuel, Explosives, etc.)
    pub const Hazardous: Self = Self(8);
    /// Covered/Sheltered Freight (Transportation in Box Vans, Silo Wagons, etc.)
    pub const Covered: Self = Self(9);
    /// Oversized (stake/flatbed wagon)
    pub const Oversized: Self = Self(10);
    /// Powderized, moist protected (powder/silo wagon)
    pub const Powderized: Self = Self(11);
    /// Not Pourable (open wagon, but not hopper wagon)
    pub const NotPourable: Self = Self(12);
    /// Potable / food / clean.
    pub const Potable: Self = Self(13);
    /// Non-potable / non-food / dirty.
    pub const NonPotable: Self = Self(14);
    /// Special bit used for livery refit tricks instead of normal cargoes.
    pub const Special: Self = Self(15);
}

/// Native `CargoClasses`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoClasses(pub u16);

impl CargoClasses {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: CargoClass) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}

impl CargoClasses {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: CargoClass) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
    /// Native `BaseBitSet::Any` of a set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

crate::base_bitset_type::impl_bitset_ops!(CargoClasses);

/// Divisor of `CargoSpec::town_production_multiplier` (`cargotype.h`).
pub const TOWN_PRODUCTION_DIVISOR: u32 = 256;

/// `SetCargoBitIterator`: the cargo-typed specialization of `SetBitIterator`.
pub struct SetCargoBitIterator(crate::bitmath_func::SetBitIterator);
impl SetCargoBitIterator {
    /// Copy the cargo mask for iteration without retaining its owner.
    pub const fn new(bitset: crate::cargo_type::CargoTypes) -> Self {
        Self(crate::bitmath_func::SetBitIterator::new(bitset.0))
    }
}
impl Iterator for SetCargoBitIterator {
    type Item = crate::cargo_type::CargoType;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(crate::cargo_type::CargoType)
    }
}
