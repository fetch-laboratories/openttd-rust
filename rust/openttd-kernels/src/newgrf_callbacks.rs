/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `newgrf_callbacks.h`: callbacks that `NewGRFs` could implement.

use crate::base_bitset_type;

/// List of implemented `NewGRF` callbacks. Most of these callbacks are only triggered when the corresponding bit is set in the callback flags/trigger for a vehicle, house, industry, etc. Names are formatted as `CBID_<CLASS>_<CALLBACK>`
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallbackID(pub u16);

impl CallbackID {
    /// Set when using the callback resolve system, but not to resolve a callback.
    pub const CBID_NO_CALLBACK: Self = Self(0x00);
    /// Set when calling a randomizing trigger (almost undocumented).
    pub const CBID_RANDOM_TRIGGER: Self = Self(0x01);
    // There are no callbacks 0x02 - 0x0F.

    /// Visual effects and wagon power.
    ///
    /// 8 bit callback
    pub const CBID_VEHICLE_VISUAL_EFFECT: Self = Self(0x10);
    /// Vehicle length, returns the amount of 1/8's the vehicle is shorter for trains and RVs.
    pub const CBID_VEHICLE_LENGTH: Self = Self(0x11);
    /// Determine the amount of cargo to load per unit of time when using gradual loading.
    ///
    /// 8 bit callback
    pub const CBID_VEHICLE_LOAD_AMOUNT: Self = Self(0x12);
    /// Determine whether a newstation should be made available to build.
    ///
    /// 8 bit callback
    pub const CBID_STATION_AVAILABILITY: Self = Self(0x13);
    /// Choose a tile layout to draw, instead of the standard range.
    pub const CBID_STATION_DRAW_TILE_LAYOUT: Self = Self(0x14);
    /// Refit capacity, the passed vehicle needs to have its `->cargo_type` set to the cargo we are refitting to, returns the new cargo capacity.
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_REFIT_CAPACITY: Self = Self(0x15);
    /// Builds articulated engines for trains and RVs.
    ///
    /// 8 bit callback for grf version < 8
    pub const CBID_VEHICLE_ARTIC_ENGINE: Self = Self(0x16);
    /// Determine whether the house can be built on the specified tile.
    ///
    /// 8 bit callback
    pub const CBID_HOUSE_ALLOW_CONSTRUCTION: Self = Self(0x17);
    /// AI construction/purchase selection
    ///
    /// 8 bit callback, implemented for stations only
    pub const CBID_GENERIC_AI_PURCHASE_SELECTION: Self = Self(0x18);
    /// Determine the cargo "suffixes" for each refit possibility of a cargo.
    pub const CBID_VEHICLE_CARGO_SUFFIX: Self = Self(0x19);
    /// Determine the next animation frame for a house.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_NEXT_FRAME: Self = Self(0x1A);
    /// Called for periodically starting or stopping the animation.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_TRIGGER_TILE_LOOP: Self = Self(0x1B);
    /// Called whenever the construction stage of a house changes.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_TRIGGER_CONSTRUCTION_STAGE_CHANGED: Self = Self(0x1C);
    /// Determine whether a wagon can be attached to an already existing train.
    pub const CBID_TRAIN_ALLOW_WAGON_ATTACH: Self = Self(0x1D);
    /// Called to determine the colour of a town building.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_COLOUR: Self = Self(0x1E);
    /// Called to decide how much cargo a town building can accept.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_CARGO_ACCEPTANCE: Self = Self(0x1F);
    /// Called to indicate how long the current animation frame should last.
    ///
    /// 8 bit callback
    pub const CBID_HOUSE_ANIMATION_SPEED: Self = Self(0x20);
    /// Called periodically to determine if a house should be destroyed.
    ///
    /// 8 bit callback
    pub const CBID_HOUSE_DESTRUCTION: Self = Self(0x21);
    /// Called to determine if the given industry type is available. For grf version >= 8 also a probability can be returned.
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_PROBABILITY: Self = Self(0x22);
    /// This callback is called from vehicle purchase lists. It returns a value to be used as a custom string ID in the 0xD000 range.
    pub const CBID_VEHICLE_ADDITIONAL_TEXT: Self = Self(0x23);
    /// Called when building a station to customize the tile layout
    ///
    /// 15 bit callback
    pub const CBID_STATION_BUILD_TILE_LAYOUT: Self = Self(0x24);
    /// Called for periodically starting or stopping the animation.
    ///
    /// 15 bit callback
    pub const CBID_INDTILE_ANIMATION_TRIGGER: Self = Self(0x25);
    /// Called to determine industry tile next animation frame.
    ///
    /// 15 bit callback
    pub const CBID_INDTILE_ANIMATION_NEXT_FRAME: Self = Self(0x26);
    /// Called to indicate how long the current animation frame should last.
    ///
    /// 8 bit callback
    pub const CBID_INDTILE_ANIMATION_SPEED: Self = Self(0x27);
    /// Called to determine if the given industry can be built on specific area.
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_LOCATION: Self = Self(0x28);
    /// Called on production changes, so it can be adjusted.
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_PRODUCTION_CHANGE: Self = Self(0x29);
    /// Called to determine which cargoes a town building should accept.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_ACCEPT_CARGO: Self = Self(0x2A);
    /// Called to query the cargo acceptance of the industry tile
    ///
    /// 15 bit callback
    pub const CBID_INDTILE_CARGO_ACCEPTANCE: Self = Self(0x2B);
    /// Called to determine which cargoes an industry should accept.
    ///
    /// 15 bit callback
    pub const CBID_INDTILE_ACCEPT_CARGO: Self = Self(0x2C);
    /// Called to determine if a specific colour map should be used for a vehicle instead of the default livery.
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_COLOUR_MAPPING: Self = Self(0x2D);
    /// Called to determine how much cargo a town building produces.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_PRODUCE_CARGO: Self = Self(0x2E);
    /// Called to determine if the given industry tile can be built on specific tile.
    ///
    /// 15 bit callback
    pub const CBID_INDTILE_SHAPE_CHECK: Self = Self(0x2F);
    /// Called to determine the type (if any) of foundation to draw for industry tile.
    ///
    /// 15 bit callback
    pub const CBID_INDTILE_DRAW_FOUNDATIONS: Self = Self(0x30);
    /// Called when the company (or AI) tries to start or stop a vehicle. Mainly used for preventing a vehicle from leaving the depot.
    ///
    /// 15 bit callback, but 0xFF test is done with 8 bit
    pub const CBID_VEHICLE_START_STOP_CHECK: Self = Self(0x31);
    /// Called for every vehicle every 32 days (not all on same date though).
    ///
    /// 2 bit callback
    pub const CBID_VEHICLE_32DAY_CALLBACK: Self = Self(0x32);
    /// Called to play a special sound effect
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_SOUND_EFFECT: Self = Self(0x33);
    /// Return the vehicles this given vehicle can be "upgraded" to.
    ///
    /// 15 bit callback, not implemented
    pub const CBID_VEHICLE_AUTOREPLACE_SELECTION: Self = Self(0x34);
    /// Called monthly on production changes, so it can be adjusted more frequently
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_MONTHLYPROD_CHANGE: Self = Self(0x35);
    /// Called to modify various vehicle properties. Callback parameter 1 specifies the property index, as used in Action 0, to change.
    ///
    /// 8/15 bit depends on queried property
    pub const CBID_VEHICLE_MODIFY_PROPERTY: Self = Self(0x36);
    /// Called to determine text to display after cargo name
    ///
    /// 15 bit callback, but 0xFF test is done with 8 bit
    pub const CBID_INDUSTRY_CARGO_SUFFIX: Self = Self(0x37);
    /// Called to determine more text in the fund industry window
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_FUND_MORE_TEXT: Self = Self(0x38);
    /// Called to calculate the income of delivered cargo
    ///
    /// 15 bit callback
    pub const CBID_CARGO_PROFIT_CALC: Self = Self(0x39);
    /// Called to determine more text in the industry window
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_WINDOW_MORE_TEXT: Self = Self(0x3A);
    /// Called to determine industry special effects
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_SPECIAL_EFFECT: Self = Self(0x3B);
    /// Called to determine if industry can alter the ground below industry tile
    ///
    /// 15 bit callback
    pub const CBID_INDTILE_AUTOSLOPE: Self = Self(0x3C);
    /// Called to determine if the industry can still accept or refuse more cargo arrival
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_REFUSE_CARGO: Self = Self(0x3D);
    // There are no callbacks 0x3E - 0x13F

    /// Called for periodically starting or stopping the animation.
    ///
    /// 15 bit callback
    pub const CBID_STATION_ANIMATION_TRIGGER: Self = Self(0x140);
    /// Called to determine station tile next animation frame.
    ///
    /// 15 bit callback
    pub const CBID_STATION_ANIMATION_NEXT_FRAME: Self = Self(0x141);
    /// Called to indicate how long the current animation frame should last.
    ///
    /// 8 bit callback
    pub const CBID_STATION_ANIMATION_SPEED: Self = Self(0x142);
    /// Called to determine whether a town building can be destroyed.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_DENY_DESTRUCTION: Self = Self(0x143);
    /// Select an ambient sound to play for a given type of tile.
    ///
    /// 15 bit callback
    pub const CBID_SOUNDS_AMBIENT_EFFECT: Self = Self(0x144);
    /// Called to calculate part of a station rating.
    ///
    /// 15 bit callback
    pub const CBID_CARGO_STATION_RATING_CALC: Self = Self(0x145);
    /// Allow signal sprites to be replaced dynamically.
    ///
    /// 15 bit callback, not implemented
    pub const CBID_NEW_SIGNALS_SPRITE_DRAW: Self = Self(0x146);
    /// Add an offset to the default sprite numbers to show another sprite.
    ///
    /// 15 bit callback
    pub const CBID_CANALS_SPRITE_OFFSET: Self = Self(0x147);
    /// Called when a cargo type specified in property 20 is accepted.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_TRIGGER_WATCHED_CARGO_ACCEPTED: Self = Self(0x148);
    /// Callback done for each tile of a station to check the slope.
    ///
    /// 15 bit callback
    pub const CBID_STATION_LAND_SLOPE_CHECK: Self = Self(0x149);
    /// Called to determine the colour of an industry.
    ///
    /// 4 bit callback
    pub const CBID_INDUSTRY_DECIDE_COLOUR: Self = Self(0x14A);
    /// Customize the input cargo types of a newly build industry.
    ///
    /// 8 bit callback
    pub const CBID_INDUSTRY_INPUT_CARGO_TYPES: Self = Self(0x14B);
    /// Customize the output cargo types of a newly build industry.
    ///
    /// 8 bit callback
    pub const CBID_INDUSTRY_OUTPUT_CARGO_TYPES: Self = Self(0x14C);
    /// Called on the Get Tile Description for an house tile.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_CUSTOM_NAME: Self = Self(0x14D);
    /// Called to determine the type (if any) of foundation to draw for house tile.
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_DRAW_FOUNDATIONS: Self = Self(0x14E);
    /// Called to determine if one can alter the ground below a house tile
    ///
    /// 15 bit callback
    pub const CBID_HOUSE_AUTOSLOPE: Self = Self(0x14F);
    /// Called to determine the type (if any) of foundation to draw for an airport tile.
    ///
    /// 15 bit callback
    pub const CBID_AIRPTILE_DRAW_FOUNDATIONS: Self = Self(0x150);
    /// Called for periodically starting or stopping the animation.
    ///
    /// 15 bit callback
    pub const CBID_AIRPTILE_ANIMATION_TRIGGER: Self = Self(0x152);
    /// Called to determine airport tile next animation frame.
    ///
    /// 15 bit callback
    pub const CBID_AIRPTILE_ANIMATION_NEXT_FRAME: Self = Self(0x153);
    /// Called to indicate how long the current animation frame should last.
    ///
    /// 8 bit callback
    pub const CBID_AIRPTILE_ANIMATION_SPEED: Self = Self(0x154);
    /// This callback is called from airport list. It returns a value to be used as a custom string ID in the 0xD000 range.
    ///
    /// 15 bit callback
    pub const CBID_AIRPORT_ADDITIONAL_TEXT: Self = Self(0x155);
    /// Called to determine text to show as airport layout name.
    ///
    /// 15 bit callback
    pub const CBID_AIRPORT_LAYOUT_NAME: Self = Self(0x156);
    /// Callback done for each tile of an object to check the slope.
    ///
    /// 15 bit callback
    pub const CBID_OBJECT_LAND_SLOPE_CHECK: Self = Self(0x157);
    /// Determine the next animation frame for a house.
    ///
    /// 15 bit callback
    pub const CBID_OBJECT_ANIMATION_NEXT_FRAME: Self = Self(0x158);
    /// Called for periodically starting or stopping the animation.
    ///
    /// 15 bit callback
    pub const CBID_OBJECT_ANIMATION_TRIGGER: Self = Self(0x159);
    /// Called to indicate how long the current animation frame should last.
    ///
    /// 8 bit callback
    pub const CBID_OBJECT_ANIMATION_SPEED: Self = Self(0x15A);
    /// Called to determine the colour of a town building.
    ///
    /// 15 bit callback
    pub const CBID_OBJECT_COLOUR: Self = Self(0x15B);
    /// Called to determine more text in the fund object window
    ///
    /// 15 bit callback
    pub const CBID_OBJECT_FUND_MORE_TEXT: Self = Self(0x15C);
    /// Called to determine if one can alter the ground below an object tile
    ///
    /// 15 bit callback
    pub const CBID_OBJECT_AUTOSLOPE: Self = Self(0x15D);
    /// Called to determine the cost factor for refitting a vehicle.
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_REFIT_COST: Self = Self(0x15E);
    /// Called when industry is built to set initial production level.
    ///
    /// 15 bit callback
    pub const CBID_INDUSTRY_PROD_CHANGE_BUILD: Self = Self(0x15F);
    /// Called to spawn visual effects for vehicles.
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_SPAWN_VISUAL_EFFECT: Self = Self(0x160);
    /// Called to determine the engine name to show.
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_NAME: Self = Self(0x161);
    /// Called to determine probability during build.
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_BUILD_PROBABILITY: Self = Self(0x162);
    /// Called to get custom engine refit mask. Called once for each defined cargo after all `NewGRFs` are loaded.
    ///
    /// 15 bit callback
    pub const CBID_VEHICLE_CUSTOM_REFIT: Self = Self(0x0163);
}

// Different values for Callback result evaluations.
//
// The header declares both `static const uint`. They stay `u16`: nearly every use compares
// them with or stores them as a `uint16_t` callback result, where the original's comparison
// converts implicitly, so a `u32` would add a cast at each such site that the original does
// not write. A use that compares a wider value converts with `u32::from`.

/// Result of a failed callback.
pub const CALLBACK_FAILED: u16 = 0xFFFF;
/// Sentinel indicating that the loop for `CBID_HOUSE_PRODUCE_CARGO` has ended.
pub const CALLBACK_HOUSEPRODCARGO_END: u16 = 0x20FF;

crate::abi::layouts! {
    fn abi_layout;
    "CallbackID" ["maintenance"] => CallbackID {
    };
}

/// Callback masks for vehicles, indicates which callbacks are used by a vehicle. Some callbacks are always used and don't have a mask.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl VehicleCallbackMask {
    /// Visual effects and wagon power (trains, road vehicles and ships)
    pub const VisualEffect: Self = Self(0);
    /// Vehicle length (trains and road vehicles)
    pub const Length: Self = Self(1);
    /// Load amount
    pub const LoadAmount: Self = Self(2);
    /// Cargo capacity after refit
    pub const RefitCapacity: Self = Self(3);
    /// Add articulated engines (trains and road vehicles)
    pub const ArticEngine: Self = Self(4);
    /// Show suffix after cargo name
    pub const CargoSuffix: Self = Self(5);
    /// Change colour mapping of vehicle
    pub const ColourRemap: Self = Self(6);
    /// Vehicle uses custom sound effects
    pub const SoundEffect: Self = Self(7);
    /// Engine name
    pub const Name: Self = Self(8);
    /// Custom refit mask
    pub const CustomRefit: Self = Self(9);
}

/// Callback masks for stations.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl StationCallbackMask {
    /// Availability of station in construction window
    pub const Avail: Self = Self(0);
    /// Use callback to select a tile layout to use when drawing.
    pub const DrawTileLayout: Self = Self(1);
    /// Use a custom next frame callback
    pub const AnimationNextFrame: Self = Self(2);
    /// Customize the animation speed of the station
    pub const AnimationSpeed: Self = Self(3);
    /// Check slope of new station tiles
    pub const SlopeCheck: Self = Self(4);
}

/// Callback masks for road stops.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadStopCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl RoadStopCallbackMask {
    /// Availability of road stop in construction window
    pub const Avail: Self = Self(0);
    /// Use a custom next frame callback
    pub const AnimationNextFrame: Self = Self(1);
    /// Customize the animation speed of the road stop
    pub const AnimationSpeed: Self = Self(2);
}

/// Callback masks for houses.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HouseCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl HouseCallbackMask {
    /// decide whether the house can be built on a given tile
    pub const AllowConstruction: Self = Self(0);
    /// decides next animation frame
    pub const AnimationNextFrame: Self = Self(1);
    /// periodically start/stop the animation
    pub const AnimationTriggerTileLoop: Self = Self(2);
    /// change animation when construction stage changes
    pub const AnimationTriggerConstructionStageChanged: Self = Self(3);
    /// decide the colour of the building
    pub const Colour: Self = Self(4);
    /// decides amount of cargo acceptance
    pub const CargoAcceptance: Self = Self(5);
    /// decides animation speed
    pub const AnimationSpeed: Self = Self(6);
    /// trigger destruction of building
    pub const Destruction: Self = Self(7);
    /// decides accepted types
    pub const AcceptCargo: Self = Self(8);
    /// custom cargo production
    pub const ProduceCargo: Self = Self(9);
    /// conditional protection
    pub const DenyDestruction: Self = Self(10);
    /// decides if default foundations need to be drawn
    pub const DrawFoundations: Self = Self(11);
    /// decides allowance of autosloping
    pub const Autoslope: Self = Self(12);
}

/// Callback masks for canals.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanalCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl CanalCallbackMask {
    /// Enable add sprite offset callback
    pub const SpriteOffset: Self = Self(0);
}

/// Callback masks for cargoes.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl CargoCallbackMask {
    /// custom profit calculation
    pub const ProfitCalc: Self = Self(0);
    /// custom station rating for this cargo type
    pub const StationRatingCalc: Self = Self(1);
}

/// Callback masks for Industries
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryCallbackMask {
    /// industry availability/probability callback
    pub const Probability: Self = Self(0);
    /// call production callback when cargo arrives at the industry
    pub const ProductionCargoArrival: Self = Self(1);
    /// call production callback every 256 ticks
    pub const Production256Ticks: Self = Self(2);
    /// check industry construction on given area
    pub const Location: Self = Self(3);
    /// controls random production change
    pub const ProductionChange: Self = Self(4);
    /// controls monthly random production change
    pub const MonthlyProdChange: Self = Self(5);
    /// cargo sub-type display
    pub const CargoSuffix: Self = Self(6);
    /// additional text in fund window
    pub const FundMoreText: Self = Self(7);
    /// additional text in industry window
    pub const WindowMoreText: Self = Self(8);
    /// control special effects
    pub const SpecialEffect: Self = Self(9);
    /// option out of accepting cargo
    pub const RefuseCargo: Self = Self(10);
    /// give a custom colour to newly build industries
    pub const DecideColour: Self = Self(11);
    /// customize the cargoes the industry requires
    pub const InputCargoTypes: Self = Self(12);
    /// customize the cargoes the industry produces
    pub const OutputCargoTypes: Self = Self(13);
    /// initialise production level on construction
    pub const ProdChangeBuild: Self = Self(14);
}

/// Callback masks for industry tiles
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryTileCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl IndustryTileCallbackMask {
    /// decides next animation frame
    pub const AnimationNextFrame: Self = Self(0);
    /// decides animation speed
    pub const AnimationSpeed: Self = Self(1);
    /// decides amount of cargo acceptance
    pub const CargoAcceptance: Self = Self(2);
    /// decides accepted types
    pub const AcceptCargo: Self = Self(3);
    /// decides slope suitability
    pub const ShapeCheck: Self = Self(4);
    /// decides if default foundations need to be drawn
    pub const DrawFoundations: Self = Self(5);
    /// decides allowance of autosloping
    pub const Autoslope: Self = Self(6);
}

/// Callback masks for objects
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl ObjectCallbackMask {
    /// decides slope suitability
    pub const SlopeCheck: Self = Self(0);
    /// decides next animation frame
    pub const AnimationNextFrame: Self = Self(1);
    /// decides animation speed
    pub const AnimationSpeed: Self = Self(2);
    /// decide the colour of the building
    pub const Colour: Self = Self(3);
    /// additional text in fund window
    pub const FundMoreText: Self = Self(4);
    /// decides allowance of autosloping
    pub const Autoslope: Self = Self(5);
}

/// Callback masks for airport tiles
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportTileCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl AirportTileCallbackMask {
    /// decides next animation frame
    pub const AnimationNextFrame: Self = Self(0);
    /// decides animation speed
    pub const AnimationSpeed: Self = Self(1);
    /// decides slope suitability
    pub const ShapeCheck: Self = Self(4);
    /// decides if default foundations need to be drawn
    pub const DrawFoundations: Self = Self(5);
    /// decides allowance of autosloping
    pub const Autoslope: Self = Self(6);
}

/// The `EnumBitSet` of a callback mask: its storage, `BaseBitSet::Set` of one element
/// (the single-element constructor), `BaseBitSet::Test` and the union `Set(const Timpl &)`.
macro_rules! callback_mask_set {
    ($set:ident, $element:ident, $storage:ty, $doc:literal) => {
        #[doc = $doc]
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $set(pub $storage);

        impl $set {
            /// Empty bit set.
            pub const EMPTY: Self = Self(0);
            /// `EnumBitSet(Tenum value)`: the set holding one element, through `BaseBitSet::Set`.
            #[must_use]
            #[allow(
                clippy::cast_possible_truncation,
                reason = "BaseBitSet::Set stores to the set's Tstorage (core/base_bitset_type.hpp)"
            )]
            pub const fn from_element(element: $element) -> Self {
                Self(base_bitset_type::set(0, element.0) as $storage)
            }
            /// `BaseBitSet::Test`: whether the element is set.
            #[must_use]
            pub const fn test(self, element: $element) -> bool {
                base_bitset_type::test(self.0 as u64, element.0)
            }
            /// `BaseBitSet::Set(const Timpl &other)`: the union with another set.
            #[must_use]
            pub const fn with(self, other: Self) -> Self {
                Self(self.0 | other.0)
            }
        }

        crate::base_bitset_type::impl_bitset_ops!($set);
    };
}

callback_mask_set!(
    VehicleCallbackMasks,
    VehicleCallbackMask,
    u16,
    "`VehicleCallbackMasks`, `EnumBitSet<VehicleCallbackMask, uint16_t>`."
);
callback_mask_set!(
    StationCallbackMasks,
    StationCallbackMask,
    u16,
    "`StationCallbackMasks`, `EnumBitSet<StationCallbackMask, uint16_t>`."
);
callback_mask_set!(
    RoadStopCallbackMasks,
    RoadStopCallbackMask,
    u8,
    "`RoadStopCallbackMasks`, `EnumBitSet<RoadStopCallbackMask, uint8_t>`."
);
callback_mask_set!(
    HouseCallbackMasks,
    HouseCallbackMask,
    u16,
    "`HouseCallbackMasks`, `EnumBitSet<HouseCallbackMask, uint16_t>`."
);
callback_mask_set!(
    CanalCallbackMasks,
    CanalCallbackMask,
    u8,
    "`CanalCallbackMasks`, `EnumBitSet<CanalCallbackMask, uint8_t>`."
);
callback_mask_set!(
    CargoCallbackMasks,
    CargoCallbackMask,
    u8,
    "`CargoCallbackMasks`, `EnumBitSet<CargoCallbackMask, uint8_t>`."
);
callback_mask_set!(
    IndustryCallbackMasks,
    IndustryCallbackMask,
    u16,
    "`IndustryCallbackMasks`, `EnumBitSet<IndustryCallbackMask, uint16_t>`."
);
callback_mask_set!(
    IndustryTileCallbackMasks,
    IndustryTileCallbackMask,
    u8,
    "`IndustryTileCallbackMasks`, `EnumBitSet<IndustryTileCallbackMask, uint8_t>`."
);
callback_mask_set!(
    ObjectCallbackMasks,
    ObjectCallbackMask,
    u8,
    "`ObjectCallbackMasks`, `EnumBitSet<ObjectCallbackMask, uint8_t>`."
);
callback_mask_set!(
    AirportTileCallbackMasks,
    AirportTileCallbackMask,
    u8,
    "`AirportTileCallbackMasks`, `EnumBitSet<AirportTileCallbackMask, uint8_t>`."
);

impl IndustryCallbackMasks {
    /// `BaseBitSet::Any`: whether any of the given values are set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

// Adapters of the typed constants for files no pass has finished; each goes with its
// last user's pass (#734's residue).

/// `CallbackID::CBID_VEHICLE_32DAY_CALLBACK`, for `newgrf_vehicle.rs`.
pub const CBID_VEHICLE_32DAY_CALLBACK: CallbackID = CallbackID::CBID_VEHICLE_32DAY_CALLBACK;
/// `CallbackID::CBID_INDTILE_ANIMATION_TRIGGER`, for `animated_tile.rs`, `industry/evidence_tests.rs`.
pub const CBID_INDTILE_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_INDTILE_ANIMATION_TRIGGER;
/// `CallbackID::CBID_STATION_ANIMATION_TRIGGER`, for `animated_tile.rs`.
pub const CBID_STATION_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_STATION_ANIMATION_TRIGGER;
/// `CallbackID::CBID_AIRPTILE_ANIMATION_TRIGGER`, for `animated_tile.rs`.
pub const CBID_AIRPTILE_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_AIRPTILE_ANIMATION_TRIGGER;
/// `CallbackID::CBID_OBJECT_ANIMATION_TRIGGER`, for `animated_tile.rs`.
pub const CBID_OBJECT_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_OBJECT_ANIMATION_TRIGGER;
/// `CallbackID::CBID_HOUSE_ANIMATION_NEXT_FRAME`, for `newgrf_animation_base.rs`.
pub const CBID_HOUSE_ANIMATION_NEXT_FRAME: CallbackID = CallbackID::CBID_HOUSE_ANIMATION_NEXT_FRAME;
/// `CallbackID::CBID_HOUSE_ANIMATION_SPEED`, for `newgrf_animation_base.rs`.
pub const CBID_HOUSE_ANIMATION_SPEED: CallbackID = CallbackID::CBID_HOUSE_ANIMATION_SPEED;
/// `CallbackID::CBID_INDTILE_ANIMATION_NEXT_FRAME`, for `newgrf_animation_base.rs`.
pub const CBID_INDTILE_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_INDTILE_ANIMATION_NEXT_FRAME;
/// `CallbackID::CBID_INDTILE_ANIMATION_SPEED`, for `newgrf_animation_base.rs`.
pub const CBID_INDTILE_ANIMATION_SPEED: CallbackID = CallbackID::CBID_INDTILE_ANIMATION_SPEED;
/// `CallbackID::CBID_STATION_ANIMATION_NEXT_FRAME`, for `newgrf_animation_base.rs`.
pub const CBID_STATION_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_STATION_ANIMATION_NEXT_FRAME;
/// `CallbackID::CBID_STATION_ANIMATION_SPEED`, for `newgrf_animation_base.rs`.
pub const CBID_STATION_ANIMATION_SPEED: CallbackID = CallbackID::CBID_STATION_ANIMATION_SPEED;
/// `CallbackID::CBID_AIRPTILE_ANIMATION_NEXT_FRAME`, for `newgrf_animation_base.rs`.
pub const CBID_AIRPTILE_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_AIRPTILE_ANIMATION_NEXT_FRAME;
/// `CallbackID::CBID_AIRPTILE_ANIMATION_SPEED`, for `newgrf_animation_base.rs`.
pub const CBID_AIRPTILE_ANIMATION_SPEED: CallbackID = CallbackID::CBID_AIRPTILE_ANIMATION_SPEED;
/// `CallbackID::CBID_OBJECT_ANIMATION_NEXT_FRAME`, for `newgrf_animation_base.rs`.
pub const CBID_OBJECT_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_OBJECT_ANIMATION_NEXT_FRAME;
/// `CallbackID::CBID_OBJECT_ANIMATION_SPEED`, for `newgrf_animation_base.rs`.
pub const CBID_OBJECT_ANIMATION_SPEED: CallbackID = CallbackID::CBID_OBJECT_ANIMATION_SPEED;
/// `CallbackID::CBID_VEHICLE_MODIFY_PROPERTY`, for `newgrf_vehicle.rs`.
pub const CBID_VEHICLE_MODIFY_PROPERTY: CallbackID = CallbackID::CBID_VEHICLE_MODIFY_PROPERTY;
/// `CallbackID::CBID_VEHICLE_BUILD_PROBABILITY`, for `newgrf_vehicle.rs`.
pub const CBID_VEHICLE_BUILD_PROBABILITY: CallbackID = CallbackID::CBID_VEHICLE_BUILD_PROBABILITY;
/// `CallbackID::CBID_NO_CALLBACK`, for `cargo_registry.rs`, `newgrf_vehicle.rs`.
pub const CBID_NO_CALLBACK: CallbackID = CallbackID::CBID_NO_CALLBACK;
/// `CallbackID::CBID_RANDOM_TRIGGER`, for `newgrf_spritegroup.rs`, `newgrf_vehicle.rs`.
pub const CBID_RANDOM_TRIGGER: CallbackID = CallbackID::CBID_RANDOM_TRIGGER;
/// `CallbackID::CBID_TRAIN_ALLOW_WAGON_ATTACH`, for `newgrf_vehicle.rs`.
pub const CBID_TRAIN_ALLOW_WAGON_ATTACH: CallbackID = CallbackID::CBID_TRAIN_ALLOW_WAGON_ATTACH;
/// `CallbackID::CBID_VEHICLE_START_STOP_CHECK`, for `newgrf_vehicle.rs`.
pub const CBID_VEHICLE_START_STOP_CHECK: CallbackID = CallbackID::CBID_VEHICLE_START_STOP_CHECK;
/// `CallbackID::CBID_VEHICLE_COLOUR_MAPPING`, for `newgrf_vehicle.rs`.
pub const CBID_VEHICLE_COLOUR_MAPPING: CallbackID = CallbackID::CBID_VEHICLE_COLOUR_MAPPING;
/// `CallbackID::CBID_VEHICLE_SPAWN_VISUAL_EFFECT`, for `newgrf_vehicle.rs`.
pub const CBID_VEHICLE_SPAWN_VISUAL_EFFECT: CallbackID =
    CallbackID::CBID_VEHICLE_SPAWN_VISUAL_EFFECT;
/// `CallbackID::CBID_STATION_LAND_SLOPE_CHECK`, for `newgrf_station.rs`.
pub const CBID_STATION_LAND_SLOPE_CHECK: CallbackID = CallbackID::CBID_STATION_LAND_SLOPE_CHECK;
/// `CallbackID::CBID_OBJECT_LAND_SLOPE_CHECK`, for `objects.rs`.
pub const CBID_OBJECT_LAND_SLOPE_CHECK: CallbackID = CallbackID::CBID_OBJECT_LAND_SLOPE_CHECK;
/// `CallbackID::CBID_OBJECT_COLOUR`, for `objects.rs`.
pub const CBID_OBJECT_COLOUR: CallbackID = CallbackID::CBID_OBJECT_COLOUR;
/// `CallbackID::CBID_VEHICLE_CUSTOM_REFIT`, for `engine_loading.rs`.
pub const CBID_VEHICLE_CUSTOM_REFIT: CallbackID = CallbackID::CBID_VEHICLE_CUSTOM_REFIT;
/// `CallbackID::CBID_CARGO_PROFIT_CALC`, for `cargo_payment.rs`.
pub const CBID_CARGO_PROFIT_CALC: CallbackID = CallbackID::CBID_CARGO_PROFIT_CALC;
/// `CallbackID::CBID_VEHICLE_REFIT_CAPACITY`, for `engine_capabilities.rs`.
pub const CBID_VEHICLE_REFIT_CAPACITY: CallbackID = CallbackID::CBID_VEHICLE_REFIT_CAPACITY;
/// `CallbackID::CBID_VEHICLE_ARTIC_ENGINE`, for `engine_capabilities.rs`.
pub const CBID_VEHICLE_ARTIC_ENGINE: CallbackID = CallbackID::CBID_VEHICLE_ARTIC_ENGINE;
/// The raw `VehicleCallbackMask::RefitCapacity` bit, for `engine_capabilities.rs`.
pub const VEHICLE_REFIT_CAPACITY: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::RefitCapacity).0;
/// The raw `VehicleCallbackMask::ArticEngine` bit, for `engine_capabilities.rs`.
pub const VEHICLE_ARTIC_ENGINE: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::ArticEngine).0;
/// The raw `VehicleCallbackMask::CargoSuffix` bit, for `engine_capabilities.rs`.
pub const VEHICLE_CARGO_SUFFIX: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::CargoSuffix).0;
/// The raw `VehicleCallbackMask::CustomRefit` bit, for `engine_loading.rs`.
pub const VEHICLE_CUSTOM_REFIT: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::CustomRefit).0;
