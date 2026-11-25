/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `cargotype.h`: the cargo vocabulary, and `CargoSpec` read through [`CargoSpecRef`].
//!
//! `CargoSpec` stays a C++ record in the static `CargoSpec::array` until its port (#1058).
//! Each member is read or written through its own slot of the cargo registry table.
use crate::cargo_registry::Services;
use crate::cargo_type::{CargoLabel, CargoType};
use crate::newgrf::GRFFile;
use crate::strings_type::StringID;

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
/// `CargoClass::Passengers` mask.
pub const PASSENGERS: u16 = CargoClasses::from_element(CargoClass::Passengers).0;

/// Constant representing invalid cargo.
pub const INVALID_CARGO_BITNUM: u8 = 0xFF;

/// `CargoSpec`: specification of a cargo type; opaque, and read through a [`CargoSpecRef`].
#[repr(C)]
pub struct CargoSpec {
    _private: [u8; 0],
}

/// A `const CargoSpec *`, with the registry table whose member slots its methods call.
///
/// Every spec is an element of the static `CargoSpec::array`, which lives for the process;
/// a handle may be kept across any call. What a member holds is read again at each call.
#[derive(Clone, Copy)]
pub(crate) struct CargoSpecRef<'a> {
    s: &'a Services,
    cs: *const CargoSpec,
}

/// `CargoSpec::Get`: retrieve the cargo specification for `cargo_type`, through the
/// table's `spec` slot, which keeps the native `assert(index < lengthof(CargoSpec::array))`.
pub(crate) fn cargo_spec_get(s: &Services, cargo_type: CargoType) -> CargoSpecRef<'_> {
    CargoSpecRef {
        s,
        cs: (s.spec)(cargo_type.0),
    }
}

/// `CargoSpec::Iterate`: each valid spec in cargo order. Each slot is looked up and tested
/// with `IsValid` as the iteration advances, as `Iterator::ValidateIndex` does.
pub(crate) fn iterate(s: &Services) -> impl Iterator<Item = CargoSpecRef<'_>> {
    (0..CargoType::NUM_CARGO.0)
        .map(move |index| cargo_spec_get(s, CargoType(index)))
        .filter(|cs| cs.is_valid())
}

impl<'a> CargoSpecRef<'a> {
    /// The handle of a spec the C++ caller passes, which is an element of `CargoSpec::array`.
    pub(crate) fn of(s: &'a Services, cs: *const CargoSpec) -> Self {
        Self { s, cs }
    }

    /// The record itself, for a service that takes the spec.
    pub(crate) fn as_ptr(self) -> *const CargoSpec {
        self.cs
    }

    /// The registry table this handle reads through.
    pub(crate) fn services(self) -> &'a Services {
        self.s
    }

    fn spec_mut(self) -> *mut CargoSpec {
        self.cs.cast_mut()
    }

    /// `CargoSpec::label`: unique label of the cargo type.
    pub(crate) fn label(self) -> CargoLabel {
        CargoLabel((self.s.label)(self.cs))
    }

    /// `CargoSpec::bitnum`: cargo bit number, is `INVALID_CARGO_BITNUM` for a non-used spec.
    pub(crate) fn bitnum(self) -> u8 {
        (self.s.bitnum)(self.cs)
    }

    /// `CargoSpec::classes`: classes of this cargo type.
    pub(crate) fn classes(self) -> CargoClasses {
        CargoClasses((self.s.classes)(self.cs))
    }

    /// `CargoSpec::town_acceptance_effect`: the effect that delivering this cargo type has
    /// on towns. Also affects destination of subsidies.
    pub(crate) fn town_acceptance_effect(self) -> TownAcceptanceEffect {
        TownAcceptanceEffect((self.s.town_acceptance_effect)(self.cs))
    }

    /// `CargoSpec::town_production_effect`: the effect on town cargo production.
    pub(crate) fn town_production_effect(self) -> TownProductionEffect {
        TownProductionEffect((self.s.town_production_effect)(self.cs))
    }

    /// `CargoSpec::town_production_multiplier`: town production multiplier, if commanded by
    /// `TownProductionEffect`.
    pub(crate) fn town_production_multiplier(self) -> u16 {
        (self.s.town_production_multiplier)(self.cs)
    }

    /// `CargoSpec::Index`: determines index of this cargospec (in the `CargoSpec::array`
    /// array).
    pub(crate) fn index(self) -> CargoType {
        CargoType((self.s.index)(self.cs))
    }

    /// `CargoSpec::IsValid`: tests for validity of this cargospec.
    /// `assert(cs->IsValid())` can be triggered when GRF config is modified.
    pub(crate) fn is_valid(self) -> bool {
        self.bitnum() != INVALID_CARGO_BITNUM
    }

    /// Store `CargoSpec::grffile`: `NewGRF` where `group` belongs to.
    pub(crate) fn set_grffile(self, grffile: *const GRFFile) {
        (self.s.set_grffile)(self.spec_mut(), grffile);
    }

    /// Store `CargoSpec::town_production_effect`: the effect on town cargo production.
    pub(crate) fn set_town_production_effect(self, effect: TownProductionEffect) {
        (self.s.set_town_production_effect)(self.spec_mut(), effect.0);
    }

    /// Store `CargoSpec::name`: name of this type of cargo.
    pub(crate) fn set_name(self, name: StringID) {
        (self.s.set_name)(self.spec_mut(), name.0);
    }

    /// Store `CargoSpec::name_single`: name of a single entity of this type of cargo.
    pub(crate) fn set_name_single(self, name_single: StringID) {
        (self.s.set_name_single)(self.spec_mut(), name_single.0);
    }

    /// Store `CargoSpec::units_volume`: name of a single unit of cargo of this type.
    pub(crate) fn set_units_volume(self, units_volume: StringID) {
        (self.s.set_units_volume)(self.spec_mut(), units_volume.0);
    }

    /// Store `CargoSpec::quantifier`: text for multiple units of cargo of this type.
    pub(crate) fn set_quantifier(self, quantifier: StringID) {
        (self.s.set_quantifier)(self.spec_mut(), quantifier.0);
    }

    /// Store `CargoSpec::abbrev`: two letter abbreviation for this cargo type.
    pub(crate) fn set_abbrev(self, abbrev: StringID) {
        (self.s.set_abbrev)(self.spec_mut(), abbrev.0);
    }
}

/// `IsCargoInClass`: does cargo `cargo` have cargo class `cc`?
#[must_use]
pub(crate) fn is_cargo_in_class(cargo: CargoType, cc: CargoClasses) -> bool {
    cargo_spec_get(crate::cargo_registry::services(), cargo)
        .classes()
        .any(cc)
}

/// [`is_cargo_in_class`] over classes the caller has read and a class mask, for files no
/// pass has finished.
#[must_use]
pub const fn is_cargo_in_class_raw(classes: u16, requested: u16) -> bool {
    CargoClasses(classes).any(CargoClasses(requested))
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
    /// Native `EnumBitSet` construction from one element: `BaseBitSet::Set` on an empty set.
    #[must_use]
    pub const fn from_element(element: CargoClass) -> Self {
        Self(crate::base_bitset_type::set_u16(0, element.0))
    }
}

impl CargoClasses {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: CargoClass) -> bool {
        crate::base_bitset_type::test(self.0 as u64, element.0)
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
    #[must_use]
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
