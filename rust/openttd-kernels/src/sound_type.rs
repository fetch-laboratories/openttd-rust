/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Shared `sound_type.h` values.
pub const SND_05_TRAIN_THROUGH_TUNNEL: u16 = SoundFx::SND_05_TRAIN_THROUGH_TUNNEL.0 as u16;

/// `SND_12_EXPLOSION`.
pub const SND_12_EXPLOSION: u16 = SoundFx::SND_12_EXPLOSION.0 as u16;

/// `SND_14_CASHTILL`.
pub const SND_14_CASHTILL: u16 = SoundFx::SND_14_CASHTILL.0 as u16;

/// Baseset sound effects in TTD/`NewGRF` order. The first two samples in the baseset
/// occur at position 39 here; enum names identify baseset positions, so names 02
/// through 28 are off by two (see the original `_sound_idx` translation table).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoundFx(pub u8);
impl SoundFx {
    /// Native `SND_BEGIN`.
    pub const SND_BEGIN: Self = Self(0);
    /// 0 == 0x00  Construction: water infrastructure
    pub const SND_02_CONSTRUCTION_WATER: Self = Self(0);
    /// 1 == 0x01  Industry producing: factory: whistle
    pub const SND_03_FACTORY: Self = Self(1);
    /// 2 == 0x02  Station departure: steam engine
    pub const SND_04_DEPARTURE_STEAM: Self = Self(2);
    /// 3 == 0x03  Train enters tunnel: steam engine
    pub const SND_05_TRAIN_THROUGH_TUNNEL: Self = Self(3);
    /// 4 == 0x04  Station departure: cargo ships
    pub const SND_06_DEPARTURE_CARGO_SHIP: Self = Self(4);
    /// 5 == 0x05  Station departure: passenger ships
    pub const SND_07_DEPARTURE_FERRY: Self = Self(5);
    /// 6 == 0x06  Takeoff: propeller plane (non-toyland)
    pub const SND_08_TAKEOFF_PROPELLER: Self = Self(6);
    /// 7 == 0x07  Takeoff: regular jet plane
    pub const SND_09_TAKEOFF_JET: Self = Self(7);
    /// 8 == 0x08  Station departure: diesel and electric engine
    pub const SND_0A_DEPARTURE_TRAIN: Self = Self(8);
    /// 9 == 0x09  Industry animation: coal/copper/gold mine: headgear
    pub const SND_0B_MINE: Self = Self(9);
    /// 10 == 0x0A  Industry animation: power station: spark
    pub const SND_0C_POWER_STATION: Self = Self(10);
    /// 11 == 0x0B  unused (1)
    pub const SND_0D_UNUSED: Self = Self(11);
    /// 12 == 0x0C  Train passes through level crossing
    pub const SND_0E_LEVEL_CROSSING: Self = Self(12);
    /// 13 == 0x0D  Breakdown: road vehicle (non-toyland)
    pub const SND_0F_BREAKDOWN_ROADVEHICLE: Self = Self(13);
    /// 14 == 0x0E  Breakdown: train or ship (non-toyland)
    pub const SND_10_BREAKDOWN_TRAIN_SHIP: Self = Self(14);
    /// 15 == 0x0F  unused (2)
    pub const SND_11_UNUSED: Self = Self(15);
    /// 16 == 0x10  Destruction, crashes, disasters, ...
    pub const SND_12_EXPLOSION: Self = Self(16);
    /// 15 == 0x11  Train+train crash
    pub const SND_13_TRAIN_COLLISION: Self = Self(17);
    /// 18 == 0x12  Income from cargo delivery
    pub const SND_14_CASHTILL: Self = Self(18);
    /// 19 == 0x13  GUI button click
    pub const SND_15_BEEP: Self = Self(19);
    /// 20 == 0x14  News ticker
    pub const SND_16_NEWS_TICKER: Self = Self(20);
    /// 21 == 0x15  Plane landing / touching ground
    pub const SND_17_SKID_PLANE: Self = Self(21);
    /// 22 == 0x16  Takeoff: helicopter
    pub const SND_18_TAKEOFF_HELICOPTER: Self = Self(22);
    /// 23 == 0x17  Station departure: truck and old bus (1) (non-toyland)
    pub const SND_19_DEPARTURE_OLD_RV_1: Self = Self(23);
    /// 24 == 0x18  Station departure: truck and old bus (2) (random variation of `SND_19_DEPARTURE_OLD_RV_1`) (non-toyland)
    pub const SND_1A_DEPARTURE_OLD_RV_2: Self = Self(24);
    /// 25 == 0x19  Station departure: modern bus (non-toyland)
    pub const SND_1B_DEPARTURE_MODERN_BUS: Self = Self(25);
    /// 26 == 0x1A  Station departure: old bus (non-toyland)
    pub const SND_1C_DEPARTURE_OLD_BUS: Self = Self(26);
    /// 27 == 0x1B  News: first vehicle at station
    pub const SND_1D_APPLAUSE: Self = Self(27);
    /// 28 == 0x1C  News: new engine available
    pub const SND_1E_NEW_ENGINE: Self = Self(28);
    /// 29 == 0x1D  Construction: other (non-water, non-rail, non-bridge)
    pub const SND_1F_CONSTRUCTION_OTHER: Self = Self(29);
    /// 30 == 0x1E  Construction: rail infrastructure
    pub const SND_20_CONSTRUCTION_RAIL: Self = Self(30);
    /// 31 == 0x1F  Road reconstruction animation
    pub const SND_21_ROAD_WORKS: Self = Self(31);
    /// 32 == 0x20  unused (3)
    pub const SND_22_UNUSED: Self = Self(32);
    /// 33 == 0x21  unused (4)
    pub const SND_23_UNUSED: Self = Self(33);
    /// 34 == 0x22  Industry producing: farm (1): sheep
    pub const SND_24_FARM_1: Self = Self(34);
    /// 35 == 0x23  Industry producing: farm (2): cow
    pub const SND_25_FARM_2: Self = Self(35);
    /// 36 == 0x24  Industry producing: farm (3): horse
    pub const SND_26_FARM_3: Self = Self(36);
    /// 37 == 0x25  Construction: bridge
    pub const SND_27_CONSTRUCTION_BRIDGE: Self = Self(37);
    /// 38 == 0x26  Industry producing: sawmill
    pub const SND_28_SAWMILL: Self = Self(38);
    /// 39 == 0x27  New year: performance improved
    pub const SND_00_GOOD_YEAR: Self = Self(39);
    /// 40 == 0x28  New year: performance declined
    pub const SND_01_BAD_YEAR: Self = Self(40);
    /// 41 == 0x29  Industry animation: sugar mine (2): shaking sieve
    pub const SND_29_SUGAR_MINE_2: Self = Self(41);
    /// 42 == 0x2A  Industry animation: toy factory (3): eject product
    pub const SND_2A_TOY_FACTORY_3: Self = Self(42);
    /// 43 == 0x2B  Industry animation: toy factory (2): stamp product
    pub const SND_2B_TOY_FACTORY_2: Self = Self(43);
    /// 44 == 0x2C  Industry animation: toy factory (1): conveyor belt
    pub const SND_2C_TOY_FACTORY_1: Self = Self(44);
    /// 45 == 0x2D  Industry animation: sugar mine (1): shaking sieve
    pub const SND_2D_SUGAR_MINE_1: Self = Self(45);
    /// 46 == 0x2E  Industry animation: bubble generator (1): generate bubble
    pub const SND_2E_BUBBLE_GENERATOR: Self = Self(46);
    /// 47 == 0x2F  Industry animation: bubble generator (2a): bubble pop
    pub const SND_2F_BUBBLE_GENERATOR_FAIL: Self = Self(47);
    /// 48 == 0x30  Industry animation: toffee quarry: drill
    pub const SND_30_TOFFEE_QUARRY: Self = Self(48);
    /// 49 == 0x31  Industry animation: bubble generator (2b): bubble slurped
    pub const SND_31_BUBBLE_GENERATOR_SUCCESS: Self = Self(49);
    /// 50 == 0x32  unused (5)
    pub const SND_32_UNUSED: Self = Self(50);
    /// 51 == 0x33  Industry producing: plastic fountain
    pub const SND_33_PLASTIC_MINE: Self = Self(51);
    /// 52 == 0x34  Tree ambient: arctic snow (1): wind
    pub const SND_34_ARCTIC_SNOW_1: Self = Self(52);
    /// 53 == 0x35  Breakdown: road vehicle (toyland)
    pub const SND_35_BREAKDOWN_ROADVEHICLE_TOYLAND: Self = Self(53);
    /// 54 == 0x36  Industry animation: lumber mill (3): crashing tree
    pub const SND_36_LUMBER_MILL_3: Self = Self(54);
    /// 55 == 0x37  Industry animation: lumber mill (2): falling tree
    pub const SND_37_LUMBER_MILL_2: Self = Self(55);
    /// 56 == 0x38  Industry animation: lumber mill (1): chainsaw
    pub const SND_38_LUMBER_MILL_1: Self = Self(56);
    /// 57 == 0x39  Tree ambient: arctic snow (2): heavy wind
    pub const SND_39_ARCTIC_SNOW_2: Self = Self(57);
    /// 58 == 0x3A  Breakdown: train or ship (toyland)
    pub const SND_3A_BREAKDOWN_TRAIN_SHIP_TOYLAND: Self = Self(58);
    /// 59 == 0x3B  Takeoff: supersonic plane (fast)
    pub const SND_3B_TAKEOFF_JET_FAST: Self = Self(59);
    /// 60 == 0x3C  Station departure: bus (1) (toyland)
    pub const SND_3C_DEPARTURE_BUS_TOYLAND_1: Self = Self(60);
    /// 61 == 0x3D  Takeoff: huge jet plane (high capacity)
    pub const SND_3D_TAKEOFF_JET_BIG: Self = Self(61);
    /// 62 == 0x3E  Station departure: bus (2) (toyland)
    pub const SND_3E_DEPARTURE_BUS_TOYLAND_2: Self = Self(62);
    /// 63 == 0x3F  Station departure: truck (1) (toyland)
    pub const SND_3F_DEPARTURE_TRUCK_TOYLAND_1: Self = Self(63);
    /// 64 == 0x40  Station departure: truck (2) (toyland)
    pub const SND_40_DEPARTURE_TRUCK_TOYLAND_2: Self = Self(64);
    /// 65 == 0x41  Station departure: maglev engine
    pub const SND_41_DEPARTURE_MAGLEV: Self = Self(65);
    /// 66 == 0x42  Tree ambient: rainforest ambient (1): bird (1)
    pub const SND_42_RAINFOREST_1: Self = Self(66);
    /// 67 == 0x43  Tree ambient: rainforest ambient (2): lion
    pub const SND_43_RAINFOREST_2: Self = Self(67);
    /// 68 == 0x44  Tree ambient: rainforest ambient (3): monkeys
    pub const SND_44_RAINFOREST_3: Self = Self(68);
    /// 69 == 0x45  Takeoff: propeller plane (1) (toyland)
    pub const SND_45_TAKEOFF_PROPELLER_TOYLAND_1: Self = Self(69);
    /// 70 == 0x46  Takeoff: propeller plane (2) (toyland)
    pub const SND_46_TAKEOFF_PROPELLER_TOYLAND_2: Self = Self(70);
    /// 71 == 0x47  Station departure: monorail engine
    pub const SND_47_DEPARTURE_MONORAIL: Self = Self(71);
    /// 72 == 0x48  Tree ambient: rainforest ambient (4): bird (2)
    pub const SND_48_RAINFOREST_4: Self = Self(72);
    /// Native `SND_END`.
    pub const SND_END: Self = Self(73);
}
/// `ORIGINAL_SAMPLE_COUNT`: the number of sounds in the original sample.cat.
pub const ORIGINAL_SAMPLE_COUNT: u32 = 73;

/// Native `SoundID`, including all `NewGRF` indices and the invalid sentinel.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SoundID(pub u16);
impl SoundID {
    /// Native invalid sound.
    pub const INVALID_SOUND: Self = Self(0xffff);
}
impl From<SoundFx> for SoundID {
    fn from(value: SoundFx) -> Self {
        Self(u16::from(value.0))
    }
}
