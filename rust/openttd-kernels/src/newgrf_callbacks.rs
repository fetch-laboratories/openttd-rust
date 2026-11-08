/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared constants and the native callback ID domain from `newgrf_callbacks.h`.

/// Native `CallbackID : uint16_t`, preserving unnamed values too.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallbackID(pub u16);

/// Native `CBID_VEHICLE_LENGTH`.
pub const CBID_VEHICLE_LENGTH: CallbackID = CallbackID::CBID_VEHICLE_LENGTH;
/// Native `CBID_VEHICLE_32DAY_CALLBACK`.
pub const CBID_VEHICLE_32DAY_CALLBACK: CallbackID = CallbackID::CBID_VEHICLE_32DAY_CALLBACK;

/// Native `CBID_INDUSTRY_SPECIAL_EFFECT`.
pub const CBID_INDUSTRY_SPECIAL_EFFECT: u16 = CallbackID::CBID_INDUSTRY_SPECIAL_EFFECT.0;
/// Native `CBID_INDTILE_DRAW_FOUNDATIONS`.
pub const CBID_INDTILE_DRAW_FOUNDATIONS: CallbackID = CallbackID::CBID_INDTILE_DRAW_FOUNDATIONS;
/// Native `CBID_INDTILE_ANIMATION_TRIGGER`.
pub const CBID_INDTILE_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_INDTILE_ANIMATION_TRIGGER;
/// Native `CBID_STATION_ANIMATION_TRIGGER`.
pub const CBID_STATION_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_STATION_ANIMATION_TRIGGER;
/// Native `CBID_AIRPTILE_ANIMATION_TRIGGER`.
pub const CBID_AIRPTILE_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_AIRPTILE_ANIMATION_TRIGGER;
/// Native `CBID_OBJECT_ANIMATION_TRIGGER`.
pub const CBID_OBJECT_ANIMATION_TRIGGER: CallbackID = CallbackID::CBID_OBJECT_ANIMATION_TRIGGER;

/// Native `CBID_HOUSE_ANIMATION_NEXT_FRAME`.
pub const CBID_HOUSE_ANIMATION_NEXT_FRAME: CallbackID = CallbackID::CBID_HOUSE_ANIMATION_NEXT_FRAME;
/// Native `CBID_HOUSE_ANIMATION_SPEED`.
pub const CBID_HOUSE_ANIMATION_SPEED: CallbackID = CallbackID::CBID_HOUSE_ANIMATION_SPEED;
/// Native `CBID_INDTILE_ANIMATION_NEXT_FRAME`.
pub const CBID_INDTILE_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_INDTILE_ANIMATION_NEXT_FRAME;
/// Native `CBID_INDTILE_ANIMATION_SPEED`.
pub const CBID_INDTILE_ANIMATION_SPEED: CallbackID = CallbackID::CBID_INDTILE_ANIMATION_SPEED;
/// Native `CBID_STATION_ANIMATION_NEXT_FRAME`.
pub const CBID_STATION_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_STATION_ANIMATION_NEXT_FRAME;
/// Native `CBID_STATION_ANIMATION_SPEED`.
pub const CBID_STATION_ANIMATION_SPEED: CallbackID = CallbackID::CBID_STATION_ANIMATION_SPEED;
/// Native `CBID_AIRPTILE_ANIMATION_NEXT_FRAME`.
pub const CBID_AIRPTILE_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_AIRPTILE_ANIMATION_NEXT_FRAME;
/// Native `CBID_AIRPTILE_ANIMATION_SPEED`.
pub const CBID_AIRPTILE_ANIMATION_SPEED: CallbackID = CallbackID::CBID_AIRPTILE_ANIMATION_SPEED;
/// Native `CBID_OBJECT_ANIMATION_NEXT_FRAME`.
pub const CBID_OBJECT_ANIMATION_NEXT_FRAME: CallbackID =
    CallbackID::CBID_OBJECT_ANIMATION_NEXT_FRAME;
/// Native `CBID_OBJECT_ANIMATION_SPEED`.
pub const CBID_OBJECT_ANIMATION_SPEED: CallbackID = CallbackID::CBID_OBJECT_ANIMATION_SPEED;

/// Native `CBID_HOUSE_ALLOW_CONSTRUCTION`.
pub const CBID_HOUSE_ALLOW_CONSTRUCTION: CallbackID = CallbackID::CBID_HOUSE_ALLOW_CONSTRUCTION;
/// Native `CBID_HOUSE_DRAW_FOUNDATIONS`.
pub const CBID_HOUSE_DRAW_FOUNDATIONS: CallbackID = CallbackID::CBID_HOUSE_DRAW_FOUNDATIONS;

/// `CALLBACK_FAILED`.
pub const CALLBACK_FAILED: u16 = 0xFFFF;
/// Sentinel indicating that the loop for `CBID_HOUSE_PRODUCE_CARGO` has ended.
pub const CALLBACK_HOUSEPRODCARGO_END: u16 = 0x20FF;

crate::abi::layouts! {
    fn abi_layout;
    "CallbackID" ["maintenance"] => CallbackID {
    };
}

/// Native `CBID_VEHICLE_MODIFY_PROPERTY`.
pub const CBID_VEHICLE_MODIFY_PROPERTY: CallbackID = CallbackID::CBID_VEHICLE_MODIFY_PROPERTY;
/// Native `CBID_VEHICLE_BUILD_PROBABILITY`.
pub const CBID_VEHICLE_BUILD_PROBABILITY: CallbackID = CallbackID::CBID_VEHICLE_BUILD_PROBABILITY;

/// Native `CBID_NO_CALLBACK`.
pub const CBID_NO_CALLBACK: CallbackID = CallbackID::CBID_NO_CALLBACK;
/// Native `CBID_RANDOM_TRIGGER`.
pub const CBID_RANDOM_TRIGGER: CallbackID = CallbackID::CBID_RANDOM_TRIGGER;
/// Native `CBID_TRAIN_ALLOW_WAGON_ATTACH`.
pub const CBID_TRAIN_ALLOW_WAGON_ATTACH: CallbackID = CallbackID::CBID_TRAIN_ALLOW_WAGON_ATTACH;
/// Native `CBID_VEHICLE_START_STOP_CHECK`.
pub const CBID_VEHICLE_START_STOP_CHECK: CallbackID = CallbackID::CBID_VEHICLE_START_STOP_CHECK;
/// Native `CBID_VEHICLE_COLOUR_MAPPING`.
pub const CBID_VEHICLE_COLOUR_MAPPING: CallbackID = CallbackID::CBID_VEHICLE_COLOUR_MAPPING;
/// Native `CBID_VEHICLE_SPAWN_VISUAL_EFFECT`.
pub const CBID_VEHICLE_SPAWN_VISUAL_EFFECT: CallbackID =
    CallbackID::CBID_VEHICLE_SPAWN_VISUAL_EFFECT;

/// Native `CBID_VEHICLE_REFIT_COST`.
pub const CBID_VEHICLE_REFIT_COST: CallbackID = CallbackID::CBID_VEHICLE_REFIT_COST;
/// Native `CBID_VEHICLE_LOAD_AMOUNT`.
pub const CBID_VEHICLE_LOAD_AMOUNT: CallbackID = CallbackID::CBID_VEHICLE_LOAD_AMOUNT;

/// Native `INDUSTRY_PRODUCTION_CARGO_ARRIVAL`.
pub const INDUSTRY_PRODUCTION_CARGO_ARRIVAL: u16 =
    IndustryCallbackMasks::from_element(IndustryCallbackMask::ProductionCargoArrival).0;
/// Native `INDUSTRY_PRODUCTION_256_TICKS`.
pub const INDUSTRY_PRODUCTION_256_TICKS: u16 =
    IndustryCallbackMasks::from_element(IndustryCallbackMask::Production256Ticks).0;

/// `CBID_STATION_LAND_SLOPE_CHECK`.
pub const CBID_STATION_LAND_SLOPE_CHECK: CallbackID = CallbackID::CBID_STATION_LAND_SLOPE_CHECK;
/// `CBID_STATION_AVAILABILITY`.
pub const CBID_STATION_AVAILABILITY: u16 = CallbackID::CBID_STATION_AVAILABILITY.0;

/// Native `CBID_GENERIC_AI_PURCHASE_SELECTION`.
pub const CBID_GENERIC_AI_PURCHASE_SELECTION: CallbackID =
    CallbackID::CBID_GENERIC_AI_PURCHASE_SELECTION;
/// Native `CBID_SOUNDS_AMBIENT_EFFECT`.
pub const CBID_SOUNDS_AMBIENT_EFFECT: CallbackID = CallbackID::CBID_SOUNDS_AMBIENT_EFFECT;
/// Object land slope check callback.
pub const CBID_OBJECT_LAND_SLOPE_CHECK: CallbackID = CallbackID::CBID_OBJECT_LAND_SLOPE_CHECK;
/// Object colour callback.
pub const CBID_OBJECT_COLOUR: CallbackID = CallbackID::CBID_OBJECT_COLOUR;

/// Engine loading custom refit callback.
pub const CBID_VEHICLE_CUSTOM_REFIT: CallbackID = CallbackID::CBID_VEHICLE_CUSTOM_REFIT;
/// `CBID_VEHICLE_VISUAL_EFFECT`.
pub const CBID_VEHICLE_VISUAL_EFFECT: CallbackID = CallbackID::CBID_VEHICLE_VISUAL_EFFECT;
/// Cargo payment callback from `newgrf_callbacks.h`.
pub const CBID_CARGO_PROFIT_CALC: CallbackID = CallbackID::CBID_CARGO_PROFIT_CALC;

/// Station cargo rating callback from `newgrf_callbacks.h`.
pub const CBID_CARGO_STATION_RATING_CALC: CallbackID = CallbackID::CBID_CARGO_STATION_RATING_CALC;

/// Native `CBID_VEHICLE_REFIT_CAPACITY`.
pub const CBID_VEHICLE_REFIT_CAPACITY: CallbackID = CallbackID::CBID_VEHICLE_REFIT_CAPACITY;
/// Native `CBID_VEHICLE_ARTIC_ENGINE`.
pub const CBID_VEHICLE_ARTIC_ENGINE: CallbackID = CallbackID::CBID_VEHICLE_ARTIC_ENGINE;
/// Native `VEHICLE_REFIT_CAPACITY`.
pub const VEHICLE_REFIT_CAPACITY: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::RefitCapacity).0;
/// Native `VEHICLE_ARTIC_ENGINE`.
pub const VEHICLE_ARTIC_ENGINE: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::ArticEngine).0;
/// Native `VEHICLE_CARGO_SUFFIX`.
pub const VEHICLE_CARGO_SUFFIX: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::CargoSuffix).0;

/// `VehicleCallbackMask::CustomRefit`.
pub const VEHICLE_CUSTOM_REFIT: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::CustomRefit).0;

/// `VehicleCallbackMask::VisualEffect`.
pub const VEHICLE_VISUAL_EFFECT: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::VisualEffect).0;

/// `VehicleCallbackMask::Length`.
pub const VEHICLE_LENGTH: u16 = VehicleCallbackMasks::from_element(VehicleCallbackMask::Length).0;

/// `VehicleCallbackMask::LoadAmount`.
pub const VEHICLE_LOAD_AMOUNT: u16 =
    VehicleCallbackMasks::from_element(VehicleCallbackMask::LoadAmount).0;

impl CallbackID {
    /// Set when using the callback resolve system, but not to resolve a callback.
    /// Native `CBID_NO_CALLBACK`.
    pub const CBID_NO_CALLBACK: Self = Self(0);
    /// Set when calling a randomizing trigger (almost undocumented).
    /// Native `CBID_RANDOM_TRIGGER`.
    pub const CBID_RANDOM_TRIGGER: Self = Self(1);
    /// Visual effects and wagon power.
    /// 8 bit callback
    pub const CBID_VEHICLE_VISUAL_EFFECT: Self = Self(16);
    /// Vehicle length, returns the amount of 1/8's the vehicle is shorter for trains and RVs.
    /// Native `CBID_VEHICLE_LENGTH`.
    pub const CBID_VEHICLE_LENGTH: Self = Self(17);
    /// Determine the amount of cargo to load per unit of time when using gradual loading.
    /// 8 bit callback
    pub const CBID_VEHICLE_LOAD_AMOUNT: Self = Self(18);
    /// Determine whether a newstation should be made available to build.
    /// 8 bit callback
    pub const CBID_STATION_AVAILABILITY: Self = Self(19);
    /// Choose a tile layout to draw, instead of the standard range.
    /// Native `CBID_STATION_DRAW_TILE_LAYOUT`.
    pub const CBID_STATION_DRAW_TILE_LAYOUT: Self = Self(20);
    /// Refit capacity, the passed vehicle needs to have its `cargo_type` set to the cargo we are refitting to, returns the new cargo capacity.
    /// 15 bit callback
    pub const CBID_VEHICLE_REFIT_CAPACITY: Self = Self(21);
    /// Builds articulated engines for trains and RVs.
    /// 8 bit callback for grf version < 8
    pub const CBID_VEHICLE_ARTIC_ENGINE: Self = Self(22);
    /// Determine whether the house can be built on the specified tile.
    /// 8 bit callback
    pub const CBID_HOUSE_ALLOW_CONSTRUCTION: Self = Self(23);
    /// AI construction/purchase selection
    /// 8 bit callback, implemented for stations only
    pub const CBID_GENERIC_AI_PURCHASE_SELECTION: Self = Self(24);
    /// Determine the cargo "suffixes" for each refit possibility of a cargo.
    /// Native `CBID_VEHICLE_CARGO_SUFFIX`.
    pub const CBID_VEHICLE_CARGO_SUFFIX: Self = Self(25);
    /// Determine the next animation frame for a house.
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_NEXT_FRAME: Self = Self(26);
    /// Called for periodically starting or stopping the animation.
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_TRIGGER_TILE_LOOP: Self = Self(27);
    /// Called whenever the construction stage of a house changes.
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_TRIGGER_CONSTRUCTION_STAGE_CHANGED: Self = Self(28);
    /// Determine whether a wagon can be attached to an already existing train.
    /// Native `CBID_TRAIN_ALLOW_WAGON_ATTACH`.
    pub const CBID_TRAIN_ALLOW_WAGON_ATTACH: Self = Self(29);
    /// Called to determine the colour of a town building.
    /// 15 bit callback
    pub const CBID_HOUSE_COLOUR: Self = Self(30);
    /// Called to decide how much cargo a town building can accept.
    /// 15 bit callback
    pub const CBID_HOUSE_CARGO_ACCEPTANCE: Self = Self(31);
    /// Called to indicate how long the current animation frame should last.
    /// 8 bit callback
    pub const CBID_HOUSE_ANIMATION_SPEED: Self = Self(32);
    /// Called periodically to determine if a house should be destroyed.
    /// 8 bit callback
    pub const CBID_HOUSE_DESTRUCTION: Self = Self(33);
    /// Called to determine if the given industry type is available. For grf version >= 8 also a probability can be returned.
    /// 15 bit callback
    pub const CBID_INDUSTRY_PROBABILITY: Self = Self(34);
    /// This callback is called from vehicle purchase lists. It returns a value to be used as a custom string ID in the 0xD000 range.
    /// Native `CBID_VEHICLE_ADDITIONAL_TEXT`.
    pub const CBID_VEHICLE_ADDITIONAL_TEXT: Self = Self(35);
    /// Called when building a station to customize the tile layout
    /// 15 bit callback
    pub const CBID_STATION_BUILD_TILE_LAYOUT: Self = Self(36);
    /// Called for periodically starting or stopping the animation.
    /// 15 bit callback
    pub const CBID_INDTILE_ANIMATION_TRIGGER: Self = Self(37);
    /// Called to determine industry tile next animation frame.
    /// 15 bit callback
    pub const CBID_INDTILE_ANIMATION_NEXT_FRAME: Self = Self(38);
    /// Called to indicate how long the current animation frame should last.
    /// 8 bit callback
    pub const CBID_INDTILE_ANIMATION_SPEED: Self = Self(39);
    /// Called to determine if the given industry can be built on specific area.
    /// 15 bit callback
    pub const CBID_INDUSTRY_LOCATION: Self = Self(40);
    /// Called on production changes, so it can be adjusted.
    /// 15 bit callback
    pub const CBID_INDUSTRY_PRODUCTION_CHANGE: Self = Self(41);
    /// Called to determine which cargoes a town building should accept.
    /// 15 bit callback
    pub const CBID_HOUSE_ACCEPT_CARGO: Self = Self(42);
    /// Called to query the cargo acceptance of the industry tile
    /// 15 bit callback
    pub const CBID_INDTILE_CARGO_ACCEPTANCE: Self = Self(43);
    /// Called to determine which cargoes an industry should accept.
    /// 15 bit callback
    pub const CBID_INDTILE_ACCEPT_CARGO: Self = Self(44);
    /// Called to determine if a specific colour map should be used for a vehicle instead of the default livery.
    /// 15 bit callback
    pub const CBID_VEHICLE_COLOUR_MAPPING: Self = Self(45);
    /// Called to determine how much cargo a town building produces.
    /// 15 bit callback
    pub const CBID_HOUSE_PRODUCE_CARGO: Self = Self(46);
    /// Called to determine if the given industry tile can be built on specific tile.
    /// 15 bit callback
    pub const CBID_INDTILE_SHAPE_CHECK: Self = Self(47);
    /// Called to determine the type (if any) of foundation to draw for industry tile.
    /// 15 bit callback
    pub const CBID_INDTILE_DRAW_FOUNDATIONS: Self = Self(48);
    /// Called when the company (or AI) tries to start or stop a vehicle. Mainly used for preventing a vehicle from leaving the depot.
    /// 15 bit callback, but 0xFF test is done with 8 bit
    pub const CBID_VEHICLE_START_STOP_CHECK: Self = Self(49);
    /// Called for every vehicle every 32 days (not all on same date though).
    /// 2 bit callback
    pub const CBID_VEHICLE_32DAY_CALLBACK: Self = Self(50);
    /// Called to play a special sound effect
    /// 15 bit callback
    pub const CBID_VEHICLE_SOUND_EFFECT: Self = Self(51);
    /// Return the vehicles this given vehicle can be "upgraded" to.
    /// 15 bit callback, not implemented
    pub const CBID_VEHICLE_AUTOREPLACE_SELECTION: Self = Self(52);
    /// Called monthly on production changes, so it can be adjusted more frequently
    /// 15 bit callback
    pub const CBID_INDUSTRY_MONTHLYPROD_CHANGE: Self = Self(53);
    /// Called to modify various vehicle properties. Callback parameter 1 specifies the property index, as used in Action 0, to change.
    /// 8/15 bit depends on queried property
    pub const CBID_VEHICLE_MODIFY_PROPERTY: Self = Self(54);
    /// Called to determine text to display after cargo name
    /// 15 bit callback, but 0xFF test is done with 8 bit
    pub const CBID_INDUSTRY_CARGO_SUFFIX: Self = Self(55);
    /// Called to determine more text in the fund industry window
    /// 15 bit callback
    pub const CBID_INDUSTRY_FUND_MORE_TEXT: Self = Self(56);
    /// Called to calculate the income of delivered cargo
    /// 15 bit callback
    pub const CBID_CARGO_PROFIT_CALC: Self = Self(57);
    /// Called to determine more text in the industry window
    /// 15 bit callback
    pub const CBID_INDUSTRY_WINDOW_MORE_TEXT: Self = Self(58);
    /// Called to determine industry special effects
    /// 15 bit callback
    pub const CBID_INDUSTRY_SPECIAL_EFFECT: Self = Self(59);
    /// Called to determine if industry can alter the ground below industry tile
    /// 15 bit callback
    pub const CBID_INDTILE_AUTOSLOPE: Self = Self(60);
    /// Called to determine if the industry can still accept or refuse more cargo arrival
    /// 15 bit callback
    pub const CBID_INDUSTRY_REFUSE_CARGO: Self = Self(61);
    /// Called for periodically starting or stopping the animation.
    /// 15 bit callback
    pub const CBID_STATION_ANIMATION_TRIGGER: Self = Self(320);
    /// Called to determine station tile next animation frame.
    /// 15 bit callback
    pub const CBID_STATION_ANIMATION_NEXT_FRAME: Self = Self(321);
    /// Called to indicate how long the current animation frame should last.
    /// 8 bit callback
    pub const CBID_STATION_ANIMATION_SPEED: Self = Self(322);
    /// Called to determine whether a town building can be destroyed.
    /// 15 bit callback
    pub const CBID_HOUSE_DENY_DESTRUCTION: Self = Self(323);
    /// Select an ambient sound to play for a given type of tile.
    /// 15 bit callback
    pub const CBID_SOUNDS_AMBIENT_EFFECT: Self = Self(324);
    /// Called to calculate part of a station rating.
    /// 15 bit callback
    pub const CBID_CARGO_STATION_RATING_CALC: Self = Self(325);
    /// Allow signal sprites to be replaced dynamically.
    /// 15 bit callback, not implemented
    pub const CBID_NEW_SIGNALS_SPRITE_DRAW: Self = Self(326);
    /// Add an offset to the default sprite numbers to show another sprite.
    /// 15 bit callback
    pub const CBID_CANALS_SPRITE_OFFSET: Self = Self(327);
    /// Called when a cargo type specified in property 20 is accepted.
    /// 15 bit callback
    pub const CBID_HOUSE_ANIMATION_TRIGGER_WATCHED_CARGO_ACCEPTED: Self = Self(328);
    /// Callback done for each tile of a station to check the slope.
    /// 15 bit callback
    pub const CBID_STATION_LAND_SLOPE_CHECK: Self = Self(329);
    /// Called to determine the colour of an industry.
    /// 4 bit callback
    pub const CBID_INDUSTRY_DECIDE_COLOUR: Self = Self(330);
    /// Customize the input cargo types of a newly build industry.
    /// 8 bit callback
    pub const CBID_INDUSTRY_INPUT_CARGO_TYPES: Self = Self(331);
    /// Customize the output cargo types of a newly build industry.
    /// 8 bit callback
    pub const CBID_INDUSTRY_OUTPUT_CARGO_TYPES: Self = Self(332);
    /// Called on the Get Tile Description for an house tile.
    /// 15 bit callback
    pub const CBID_HOUSE_CUSTOM_NAME: Self = Self(333);
    /// Called to determine the type (if any) of foundation to draw for house tile.
    /// 15 bit callback
    pub const CBID_HOUSE_DRAW_FOUNDATIONS: Self = Self(334);
    /// Called to determine if one can alter the ground below a house tile
    /// 15 bit callback
    pub const CBID_HOUSE_AUTOSLOPE: Self = Self(335);
    /// Called to determine the type (if any) of foundation to draw for an airport tile.
    /// 15 bit callback
    pub const CBID_AIRPTILE_DRAW_FOUNDATIONS: Self = Self(336);
    /// Called for periodically starting or stopping the animation.
    /// 15 bit callback
    pub const CBID_AIRPTILE_ANIMATION_TRIGGER: Self = Self(338);
    /// Called to determine airport tile next animation frame.
    /// 15 bit callback
    pub const CBID_AIRPTILE_ANIMATION_NEXT_FRAME: Self = Self(339);
    /// Called to indicate how long the current animation frame should last.
    /// 8 bit callback
    pub const CBID_AIRPTILE_ANIMATION_SPEED: Self = Self(340);
    /// This callback is called from airport list. It returns a value to be used as a custom string ID in the 0xD000 range.
    /// 15 bit callback
    pub const CBID_AIRPORT_ADDITIONAL_TEXT: Self = Self(341);
    /// Called to determine text to show as airport layout name.
    /// 15 bit callback
    pub const CBID_AIRPORT_LAYOUT_NAME: Self = Self(342);
    /// Callback done for each tile of an object to check the slope.
    /// 15 bit callback
    pub const CBID_OBJECT_LAND_SLOPE_CHECK: Self = Self(343);
    /// Determine the next animation frame for a house.
    /// 15 bit callback
    pub const CBID_OBJECT_ANIMATION_NEXT_FRAME: Self = Self(344);
    /// Called for periodically starting or stopping the animation.
    /// 15 bit callback
    pub const CBID_OBJECT_ANIMATION_TRIGGER: Self = Self(345);
    /// Called to indicate how long the current animation frame should last.
    /// 8 bit callback
    pub const CBID_OBJECT_ANIMATION_SPEED: Self = Self(346);
    /// Called to determine the colour of a town building.
    /// 15 bit callback
    pub const CBID_OBJECT_COLOUR: Self = Self(347);
    /// Called to determine more text in the fund object window
    /// 15 bit callback
    pub const CBID_OBJECT_FUND_MORE_TEXT: Self = Self(348);
    /// Called to determine if one can alter the ground below an object tile
    /// 15 bit callback
    pub const CBID_OBJECT_AUTOSLOPE: Self = Self(349);
    /// Called to determine the cost factor for refitting a vehicle.
    /// 15 bit callback
    pub const CBID_VEHICLE_REFIT_COST: Self = Self(350);
    /// Called when industry is built to set initial production level.
    /// 15 bit callback
    pub const CBID_INDUSTRY_PROD_CHANGE_BUILD: Self = Self(351);
    /// Called to spawn visual effects for vehicles.
    /// 15 bit callback
    pub const CBID_VEHICLE_SPAWN_VISUAL_EFFECT: Self = Self(352);
    /// Called to determine the engine name to show.
    /// 15 bit callback
    pub const CBID_VEHICLE_NAME: Self = Self(353);
    /// Called to determine probability during build.
    /// 15 bit callback
    pub const CBID_VEHICLE_BUILD_PROBABILITY: Self = Self(354);
    /// Called to get custom engine refit mask. Called once for each defined cargo after all `NewGRFs` are loaded.
    /// 15 bit callback
    pub const CBID_VEHICLE_CUSTOM_REFIT: Self = Self(355);
}

/// Native `VehicleCallbackMask`, retaining every underlying `u8` value.
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

/// Native `StationCallbackMask`, retaining every underlying `u8` value.
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

/// Native `RoadStopCallbackMask`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadStopCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl RoadStopCallbackMask {
    /// Availability of road stop in construction window.
    pub const Avail: Self = Self(0);
    /// Use a custom next frame callback.
    pub const AnimationNextFrame: Self = Self(1);
    /// Customize the animation speed of the road stop.
    pub const AnimationSpeed: Self = Self(2);
}

/// Native `HouseCallbackMask`, retaining every underlying `u8` value.
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

/// Native `CanalCallbackMask`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanalCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl CanalCallbackMask {
    /// Enable add sprite offset callback
    pub const SpriteOffset: Self = Self(0);
}

/// Native `CargoCallbackMask`, retaining every underlying `u8` value.
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

/// Native `IndustryCallbackMask`, retaining every underlying `u8` value.
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

/// Native `IndustryTileCallbackMask`, retaining every underlying `u8` value.
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

/// Native `ObjectCallbackMask`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl ObjectCallbackMask {
    /// Decides slope suitability.
    pub const SlopeCheck: Self = Self(0);
    /// Decides next animation frame.
    pub const AnimationNextFrame: Self = Self(1);
    /// Decides animation speed.
    pub const AnimationSpeed: Self = Self(2);
    /// Decide the colour of the building.
    pub const Colour: Self = Self(3);
    /// Additional text in fund window.
    pub const FundMoreText: Self = Self(4);
    /// Decides allowance of autosloping.
    pub const Autoslope: Self = Self(5);
}

/// Native `AirportTileCallbackMask`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportTileCallbackMask(pub u8);

#[allow(non_upper_case_globals)]
impl AirportTileCallbackMask {
    /// Decides next animation frame.
    pub const AnimationNextFrame: Self = Self(0);
    /// Decides animation speed.
    pub const AnimationSpeed: Self = Self(1);
    /// Decides slope suitability.
    pub const ShapeCheck: Self = Self(4);
    /// Decides if default foundations need to be drawn.
    pub const DrawFoundations: Self = Self(5);
    /// Decides allowance of autosloping.
    pub const Autoslope: Self = Self(6);
}

/// Native `VehicleCallbackMasks`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VehicleCallbackMasks(pub u16);

impl VehicleCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: VehicleCallbackMask) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}

/// Native `StationCallbackMasks`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StationCallbackMasks(pub u16);

impl StationCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: StationCallbackMask) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}

/// Native `RoadStopCallbackMasks`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadStopCallbackMasks(pub u8);

impl RoadStopCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: RoadStopCallbackMask) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `HouseCallbackMasks`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HouseCallbackMasks(pub u16);

impl HouseCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: HouseCallbackMask) -> Self {
        Self((1_u64 << element.0) as u16)
    }
}

/// Native `CanalCallbackMasks`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanalCallbackMasks(pub u8);

impl CanalCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: CanalCallbackMask) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `CargoCallbackMasks`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CargoCallbackMasks(pub u8);

impl CargoCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: CargoCallbackMask) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `IndustryCallbackMasks`, retaining every underlying `u16` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryCallbackMasks(pub u16);

impl IndustryCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: IndustryCallbackMask) -> Self {
        Self((1_u64 << element.0) as u16)
    }
    /// Native `BaseBitSet::Any`: whether any of the given values are set.
    #[must_use]
    pub const fn any(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}

/// Native `IndustryTileCallbackMasks`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndustryTileCallbackMasks(pub u8);

impl IndustryTileCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: IndustryTileCallbackMask) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `ObjectCallbackMasks`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectCallbackMasks(pub u8);

impl ObjectCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: ObjectCallbackMask) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

/// Native `AirportTileCallbackMasks`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AirportTileCallbackMasks(pub u8);

impl AirportTileCallbackMasks {
    /// Empty native bit set.
    pub const EMPTY: Self = Self(0);
    /// Native `EnumBitSet` construction from one element (a bit position).
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn from_element(element: AirportTileCallbackMask) -> Self {
        Self((1_u64 << element.0) as u8)
    }
}

impl VehicleCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: VehicleCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl StationCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: StationCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl RoadStopCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: RoadStopCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl HouseCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: HouseCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl CanalCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: CanalCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl CargoCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: CargoCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl IndustryCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: IndustryCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl IndustryTileCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: IndustryTileCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl ObjectCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: ObjectCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl AirportTileCallbackMasks {
    /// Native `BaseBitSet::Test`, with an element rather than a set mask.
    #[must_use]
    pub const fn test(self, element: AirportTileCallbackMask) -> bool {
        (self.0 as u64) & (1_u64 << element.0) != 0
    }
    /// Native bit-set union.
    #[must_use]
    pub const fn with(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

crate::base_bitset_type::impl_bitset_ops!(VehicleCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(StationCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(RoadStopCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(HouseCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(CanalCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(CargoCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(IndustryCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(IndustryTileCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(ObjectCallbackMasks);
crate::base_bitset_type::impl_bitset_ops!(AirportTileCallbackMasks);
