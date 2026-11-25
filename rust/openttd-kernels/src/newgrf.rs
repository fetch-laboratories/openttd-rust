/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `newgrf.h`: base for the `NewGRF` implementation, with `GetGlobalVariable`
//! (`newgrf/newgrf_act0_globalvar.cpp`) and `GetActiveCargoLabel` (`newgrf.cpp`).
//!
//! `GRFFile` stays a C++ record in `_grf_files` until its port (#669). Rust reads its members
//! through a [`GRFFileRef`], whose methods call the shared table's member readers.
use crate::base_bitset_type;
use crate::bitmath_func::{gb, sb};
use crate::cargo_registry::get_cargo_type_by_label;
use crate::cargo_type::{CargoLabel, CargoType, MixedCargoType};
use crate::landscape_type::LandscapeType;
use crate::newgrf_config::GRFPalette;
use crate::newgrf_engine::VEHICLEINFO_FULL_VEHICLE_WIDTH;
use crate::rail::get_rail_type_info;
use crate::rail_type::RailType;
use crate::services::Services as Shared;
use crate::settings_type::SP_CUSTOM;
use crate::tile_type::TILE_HEIGHT;
use crate::timer_game_common::{
    DAYS_TILL_ORIGINAL_BASE_YEAR, ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR, calendar_date,
    calendar_date_to_ymd, calendar_fraction, calendar_year, calendar_ymd_to_date, is_leap_year,
};
use std::ffi::c_void;
use std::ptr::NonNull;

/// `CanalFeature`: list of different canal 'features'. Each feature gets an entry in the canal
/// spritegroup table.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanalFeature(pub u8);
impl CanalFeature {
    /// `CF_END`.
    pub const CF_END: Self = Self(9);
}

/// `GrfLoadingStage`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct GrfLoadingStage(pub u8);
impl GrfLoadingStage {
    /// `GLS_FILESCAN`.
    pub const GLS_FILESCAN: Self = Self(0);
    /// `GLS_SAFETYSCAN`.
    pub const GLS_SAFETYSCAN: Self = Self(1);
    /// `GLS_LABELSCAN`.
    pub const GLS_LABELSCAN: Self = Self(2);
    /// `GLS_INIT`.
    pub const GLS_INIT: Self = Self(3);
    /// `GLS_RESERVE`.
    pub const GLS_RESERVE: Self = Self(4);
    /// `GLS_ACTIVATION`.
    pub const GLS_ACTIVATION: Self = Self(5);
}

/// `GrfMiscBit`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrfMiscBit(pub u8);
#[allow(
    non_upper_case_globals,
    reason = "the enumerator names of the original GrfMiscBit (newgrf.h)"
)]
impl GrfMiscBit {
    /// Use 32 pixels per train vehicle in depot gui and vehicle details. Never set in the
    /// global variable; see `GRFFile::traininfo_vehicle_width`.
    pub const TrainWidth32Pixels: Self = Self(3);
    /// `AmbientSoundCallback`.
    pub const AmbientSoundCallback: Self = Self(4);
    /// `SecondRockyTileSet`.
    pub const SecondRockyTileSet: Self = Self(6);
}

/// `GrfMiscBits`, `EnumBitSet<GrfMiscBit, uint8_t>`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrfMiscBits(pub u8);
impl GrfMiscBits {
    /// `BaseBitSet::Test`: whether the bit is set.
    #[must_use]
    pub const fn test(self, bit: GrfMiscBit) -> bool {
        base_bitset_type::test(self.0 as u64, bit.0)
    }
    /// `BaseBitSet::Set`: set the bit.
    pub const fn set(&mut self, bit: GrfMiscBit) {
        self.0 = base_bitset_type::set_u8(self.0, bit.0);
    }
}

/// `GrfSpecFeature`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrfSpecFeature(pub u8);

impl GrfSpecFeature {
    /// `GSF_TRAINS`.
    pub const GSF_TRAINS: Self = Self(0);
    /// `GSF_ROADVEHICLES`.
    pub const GSF_ROADVEHICLES: Self = Self(1);
    /// `GSF_SHIPS`.
    pub const GSF_SHIPS: Self = Self(2);
    /// `GSF_AIRCRAFT`.
    pub const GSF_AIRCRAFT: Self = Self(3);
    /// `GSF_STATIONS`.
    pub const GSF_STATIONS: Self = Self(4);
    /// `GSF_CANALS`.
    pub const GSF_CANALS: Self = Self(5);
    /// `GSF_BRIDGES`.
    pub const GSF_BRIDGES: Self = Self(6);
    /// `GSF_HOUSES`.
    pub const GSF_HOUSES: Self = Self(7);
    /// `GSF_GLOBALVAR`.
    pub const GSF_GLOBALVAR: Self = Self(8);
    /// `GSF_INDUSTRYTILES`.
    pub const GSF_INDUSTRYTILES: Self = Self(9);
    /// `GSF_INDUSTRIES`.
    pub const GSF_INDUSTRIES: Self = Self(10);
    /// `GSF_CARGOES`.
    pub const GSF_CARGOES: Self = Self(11);
    /// `GSF_SOUNDFX`.
    pub const GSF_SOUNDFX: Self = Self(12);
    /// `GSF_AIRPORTS`.
    pub const GSF_AIRPORTS: Self = Self(13);
    /// `GSF_SIGNALS`.
    pub const GSF_SIGNALS: Self = Self(14);
    /// `GSF_OBJECTS`.
    pub const GSF_OBJECTS: Self = Self(15);
    /// `GSF_RAILTYPES`.
    pub const GSF_RAILTYPES: Self = Self(16);
    /// `GSF_AIRPORTTILES`.
    pub const GSF_AIRPORTTILES: Self = Self(17);
    /// `GSF_ROADTYPES`.
    pub const GSF_ROADTYPES: Self = Self(18);
    /// `GSF_TRAMTYPES`.
    pub const GSF_TRAMTYPES: Self = Self(19);
    /// `GSF_ROADSTOPS`.
    pub const GSF_ROADSTOPS: Self = Self(20);
    /// `GSF_BADGES`.
    pub const GSF_BADGES: Self = Self(21);
    /// `GSF_END`.
    pub const GSF_END: Self = Self(22);

    /// Unspecified feature, default badge.
    pub const GSF_DEFAULT: Self = Self::GSF_END;
    /// Fake town `GrfSpecFeature` for `NewGRF` debugging (parent scope).
    pub const GSF_FAKE_TOWNS: Self = Self::GSF_END;
    /// End of the fake features.
    pub const GSF_FAKE_END: Self = Self(23);

    /// `GSF_ORIGINAL_STRINGS`.
    pub const GSF_ORIGINAL_STRINGS: Self = Self(0x48);

    /// An invalid spec feature.
    pub const GSF_INVALID: Self = Self(0xFF);
}

/// `GrfSpecFeatures`, `EnumBitSet<GrfSpecFeature, uint32_t, GrfSpecFeature::GSF_END>`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrfSpecFeatures(pub u32);

impl GrfSpecFeatures {
    /// Empty bit set.
    pub const EMPTY: Self = Self(0);
    /// `BaseBitSet::Set`: set the feature.
    pub const fn set(&mut self, element: GrfSpecFeature) {
        self.0 = base_bitset_type::set_u32(self.0, element.0);
    }
}

/// `INVALID_GRFID`.
pub const INVALID_GRFID: u32 = 0xFFFF_FFFF;

/// `GRFFile`: dynamic data of a loaded `NewGRF`; opaque, and read through a [`GRFFileRef`].
#[repr(C)]
pub struct GRFFile {
    _private: [u8; 0],
}

/// A `const GRFFile *` the original dereferences, with the shared table whose member readers
/// its methods call.
///
/// The records live in C++'s `_grf_files` and are destroyed only by the loader's reset
/// (`ResetNewGRFData`'s `_grf_files.clear()`), which runs during `NewGRF` loading, outside
/// the game loop and command execution. A handle is a call-time value: it is formed in the
/// current entry from the loader's current file, a spec's `grf_prop.grffile`, a resolver's
/// `grffile` or a registry the same reset clears (`_gcl`), is never stored, and neither it
/// nor anything borrowed from it is used after a call that can run that reset. A record or
/// registry that keeps a file holds `*const GRFFile`.
#[derive(Clone, Copy)]
pub(crate) struct GRFFileRef<'a> {
    shared: &'a Shared,
    grffile: NonNull<GRFFile>,
}

impl<'a> GRFFileRef<'a> {
    /// The handle of a file the original dereferences without a test.
    ///
    /// # Safety
    /// `grffile` is a live record, as the type states.
    #[allow(unsafe_code, reason = "declares the caller's liveness contract")]
    pub(crate) unsafe fn of(shared: &'a Shared, grffile: *const GRFFile) -> Self {
        // A null file is a broken caller contract, as the native dereference would be.
        Self {
            shared,
            grffile: NonNull::new(grffile.cast_mut()).unwrap(),
        }
    }

    /// The handle of a file the original tests for null, or `None` for a null file.
    ///
    /// # Safety
    /// A non-null `grffile` is a live record, as the type states.
    #[allow(unsafe_code, reason = "declares the caller's liveness contract")]
    pub(crate) unsafe fn new(shared: &'a Shared, grffile: *const GRFFile) -> Option<Self> {
        NonNull::new(grffile.cast_mut()).map(|grffile| Self { shared, grffile })
    }

    /// The record itself, for a service that takes the file.
    pub(crate) fn as_ptr(self) -> *const GRFFile {
        self.grffile.as_ptr()
    }

    fn file(self) -> *const c_void {
        self.grffile.as_ptr().cast()
    }

    /// `GRFFile::grfid`.
    pub(crate) fn grfid(self) -> u32 {
        (self.shared.grf_file_id)(self.file())
    }

    /// `GRFFile::grf_version`.
    pub(crate) fn grf_version(self) -> u8 {
        (self.shared.grf_file_version)(self.file())
    }

    /// `GRFFile::filename`, borrowed from this handle.
    #[allow(unsafe_code, reason = "the record's live std::string bytes")]
    pub(crate) fn filename(&self) -> &[u8] {
        // SAFETY: The record is live for the handle's interval (the type's contract), and the
        // returned slice borrows the handle.
        unsafe { (self.shared.grf_file_name)(self.file()).bytes() }
    }

    /// `GRFFile::cargo_map`: inverse cargo translation table (`CargoType` -> local ID).
    pub(crate) fn cargo_map(self, cargo: CargoType) -> u8 {
        (self.shared.grf_cargo_map)(self.file(), cargo.0)
    }

    /// `GRFFile::GetParam`: get GRF Parameter with range checking.
    pub(crate) fn get_param(self, number: u32) -> u32 {
        // Note: We implicitly test for number < this->param.size() and return 0 for invalid
        // parameters. In fact this is the more important test, as param is zeroed anyway.
        (self.shared.grf_parameter)(self.file(), number)
    }

    /// `GRFFile::traininfo_vehicle_pitch`: vertical offset for drawing train images in depot
    /// GUI and vehicle details.
    pub(crate) fn traininfo_vehicle_pitch(self) -> i32 {
        (self.shared.grf_file_traininfo_vehicle_pitch)(self.file())
    }

    /// `GRFFile::traininfo_vehicle_width`: width (in pixels) of a 8/8 train vehicle in depot
    /// GUI and vehicle details.
    pub(crate) fn traininfo_vehicle_width(self) -> u32 {
        (self.shared.grf_file_traininfo_vehicle_width)(self.file())
    }
}

/// The global-variable table: the C++ globals and `GRFConfig` readers `GetGlobalVariable`
/// and `HasGrfMiscBit` read.
#[repr(C)]
pub struct Services {
    /// `GetGRFConfig(grfid, mask)`.
    pub(crate) get_grf_config: extern "C" fn(u32, u32) -> *mut c_void,
    /// `_misc_grf_features`.
    pub(crate) misc_grf_features: extern "C" fn() -> u8,
    /// `_openttd_newgrf_version`.
    openttd_newgrf_version: extern "C" fn() -> u32,
    /// `GRFConfig::palette` of a non-null config.
    pub(crate) palette: extern "C" fn(*const c_void) -> u8,
    /// The shared table.
    pub(crate) shared: *const Shared,
}
impl Services {
    /// The shared table.
    #[allow(unsafe_code, reason = "one borrow of the process-lifetime table")]
    pub(crate) fn shared(&self) -> &Shared {
        // SAFETY: Both tables are process-lifetime.
        unsafe { &*self.shared }
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGlobalVariableServices" ["storage"] => Services { get_grf_config, misc_grf_features, openttd_newgrf_version, palette, shared };
}

/// `HasGrfMiscBit`: check for grf miscellaneous bits.
pub(crate) fn has_grf_misc_bit(s: &Services, bit: GrfMiscBit) -> bool {
    GrfMiscBits((s.misc_grf_features)()).test(bit)
}

/// `GetGlobalVariable`: the value of global variable `param`, or `None` where the original
/// returns false and leaves `*value` unwritten. Variables `0x0D`, `0x0E`, `0x1E` and `0x20`
/// dereference `grffile`, as the original does.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the uint32_t stores and GB(TimerGameTick::counter, 0, 16) of GetGlobalVariable (newgrf/newgrf_act0_globalvar.cpp)"
)]
pub(crate) fn get_global_variable(
    s: &Services,
    param: u8,
    grffile: Option<GRFFileRef<'_>>,
) -> Option<u32> {
    let shared = s.shared();
    // The cases that dereference the file take it as the original does.
    let file = || grffile.unwrap();
    let value = match param {
        // current date
        0x00 => calendar_date()
            .0
            .wrapping_sub(DAYS_TILL_ORIGINAL_BASE_YEAR)
            .max(0) as u32,

        // current year
        0x01 => {
            (calendar_year()
                .0
                .clamp(ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR)
                - ORIGINAL_BASE_YEAR) as u32
        }

        // detailed date information: month of year (bit 0-7), day of month (bit 8-12), leap year (bit 15), day of year (bit 16-24)
        0x02 => {
            let ymd = calendar_date_to_ymd(calendar_date().0);
            let start_of_year = calendar_ymd_to_date(ymd.year, 0, 1);
            u32::from(ymd.month)
                | (u32::from(ymd.day) - 1) << 8
                | if is_leap_year(ymd.year) { 1 << 15 } else { 0 }
                | (calendar_date().0.wrapping_sub(start_of_year) as u32) << 16
        }

        // current climate, 0=temp, 1=arctic, 2=trop, 3=toyland
        0x03 => u32::from(shared.settings().landscape().0),

        // road traffic side, bit 4 clear=left, set=right
        0x06 => u32::from(shared.settings().road_side()) << 4,

        // date fraction
        0x09 => u32::from(calendar_fraction()) * 885,

        // animation counter
        0x0A => gb((shared.tick_counter)() as u32, 0, 16),

        // TTDPatch version
        0x0B => {
            let major: u32 = 2;
            let minor: u32 = 6;
            let revision: u32 = 1; // special case: 2.0.1 is 2.0.10
            let build: u32 = 1382;
            (major << 24) | (minor << 20) | (revision << 16) | build
        }

        // TTD Version, 00=DOS, 01=Windows
        0x0D => {
            let config = (s.get_grf_config)(file().grfid(), u32::MAX);
            u32::from((s.palette)(config) & GRFPalette::GRFP_USE_MASK.0)
        }

        // Y-offset for train sprites
        0x0E => file().traininfo_vehicle_pitch() as u32,

        // Rail track type cost factors
        0x0F => {
            let mut value = 0;
            let cost_multiplier = |rt| u32::from(get_rail_type_info(shared, rt).cost_multiplier());
            sb(&mut value, 0, 8, cost_multiplier(RailType::RAILTYPE_RAIL)); // normal rail
            if shared.settings().disable_elrails() {
                // skip elrail multiplier - disabled
                sb(&mut value, 8, 8, cost_multiplier(RailType::RAILTYPE_MONO)); // monorail
            } else {
                sb(
                    &mut value,
                    8,
                    8,
                    cost_multiplier(RailType::RAILTYPE_ELECTRIC),
                ); // electrified railway
                // Skip monorail multiplier - no space in result
            }
            sb(
                &mut value,
                16,
                8,
                cost_multiplier(RailType::RAILTYPE_MAGLEV),
            ); // maglev
            value
        }

        // current rail tool type
        0x11 => 0, // constant fake value to avoid desync

        // Game mode
        0x12 => u32::from(crate::session::mode().0),

        // case 0x13: // Tile refresh offset to left    not implemented
        // case 0x14: // Tile refresh offset to right   not implemented
        // case 0x15: // Tile refresh offset upwards    not implemented
        // case 0x16: // Tile refresh offset downwards  not implemented
        // case 0x17: // temperate snow line            not implemented

        // Always -1
        0x1A => u32::MAX,

        // Display options
        0x1B => 0x3F, // constant fake value to avoid desync

        // TTD Platform, 00=TTDPatch, 01=OpenTTD
        0x1D => 1,

        // Miscellaneous GRF features
        0x1E => {
            let mut bits = GrfMiscBits((s.misc_grf_features)());

            // Add the local flags
            crate::services_assert::assert_with(|| !bits.test(GrfMiscBit::TrainWidth32Pixels));
            if file().traininfo_vehicle_width() == VEHICLEINFO_FULL_VEHICLE_WIDTH {
                bits.set(GrfMiscBit::TrainWidth32Pixels);
            }

            u32::from(bits.0)
        }

        // case 0x1F: // locale dependent settings not implemented to avoid desync

        // snow line height
        0x20 => {
            let snowline = crate::snow_line::get(shared);
            if shared.settings().landscape() == LandscapeType::Arctic
                && snowline <= shared.settings().map_height_limit()
            {
                (u32::from(snowline)
                    * if file().grf_version() >= 8 {
                        1
                    } else {
                        TILE_HEIGHT
                    })
                .clamp(0, 0xFE)
            } else {
                // No snow
                0xFF
            }
        }

        // OpenTTD version
        0x21 => (s.openttd_newgrf_version)(),

        // difficulty level
        0x22 => SP_CUSTOM,

        // long format date
        0x23 => calendar_date().0 as u32,

        // long format year
        0x24 => calendar_year().0 as u32,

        _ => return None,
    };
    Some(value)
}

/// `GetGlobalVariable` for C++ callers: writes `*value` only for a known variable, since
/// callers pass uninitialised storage and an unknown variable leaves it as it was.
///
/// # Safety
/// `s` is the process-lifetime table, `value` is writable and a non-null `grffile` is live.
#[allow(unsafe_code, reason = "export and the caller's output pointer")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_global_variable_abi(
    s: &Services,
    variable: u8,
    value: *mut u32,
    grffile: *const GRFFile,
) -> u8 {
    // SAFETY: The caller's file is live for this call.
    let grffile = unsafe { GRFFileRef::new(s.shared(), grffile) };
    match get_global_variable(s, variable, grffile) {
        Some(result) => {
            // SAFETY: The caller passes writable storage, possibly uninitialised.
            unsafe { value.write(result) };
            1
        }
        None => 0,
    }
}

/// The fork's `repr(C)` form of `std::variant<CargoLabel, MixedCargoType>`, which has no name
/// in the original: `value` is the label, or the `MixedCargoType` where `mixed` is set.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CargoLabelChoice {
    /// The label, or the widened `MixedCargoType`.
    pub value: u32,
    /// Whether `value` holds a `MixedCargoType`.
    pub mixed: bool,
}

/// `GetActiveCargoLabel(const std::initializer_list<CargoLabel> &)`: the first label of
/// `labels` that has a cargo type, or `CT_INVALID`.
pub(crate) fn get_active_cargo_label_of(labels: &[CargoLabel]) -> CargoLabel {
    for &label in labels {
        let cargo_type = get_cargo_type_by_label(label);
        if cargo_type != CargoType::INVALID_CARGO {
            return label;
        }
    }
    CargoLabel::CT_INVALID
}

/// `GetActiveCargoLabel(const std::variant<CargoLabel, MixedCargoType> &)`.
pub(crate) fn get_active_cargo_label(label: CargoLabelChoice) -> CargoLabel {
    if !label.mixed {
        return CargoLabel(label.value);
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the record widens the uint8_t enum MixedCargoType (cargo_type.h)"
    )]
    let mixed = MixedCargoType(label.value as u8);
    match mixed {
        MixedCargoType::MCT_LIVESTOCK_FRUIT => {
            get_active_cargo_label_of(&[CargoLabel::CT_LIVESTOCK, CargoLabel::CT_FRUIT])
        }
        MixedCargoType::MCT_GRAIN_WHEAT_MAIZE => get_active_cargo_label_of(&[
            CargoLabel::CT_GRAIN,
            CargoLabel::CT_WHEAT,
            CargoLabel::CT_MAIZE,
        ]),
        MixedCargoType::MCT_VALUABLES_GOLD_DIAMONDS => get_active_cargo_label_of(&[
            CargoLabel::CT_VALUABLES,
            CargoLabel::CT_GOLD,
            CargoLabel::CT_DIAMONDS,
        ]),
        _ => crate::services_assert::not_reached(),
    }
}

// Raw values of the typed constants for files no pass has finished; each goes with its last
// user's pass (#734's residue).

/// `GrfMiscBit::TrainWidth32Pixels`, for `newgrf_loading.rs`.
pub const TRAIN_WIDTH_32_PIXELS: u8 = GrfMiscBit::TrainWidth32Pixels.0;
/// `GrfMiscBit::SecondRockyTileSet`, for `newgrf_loading.rs`.
pub const SECOND_ROCKY_TILE_SET: u8 = GrfMiscBit::SecondRockyTileSet.0;
/// `CanalFeature::CF_END`, for `newgrf_act3/loading.rs`.
pub const CF_END: u16 = CanalFeature::CF_END.0 as u16;
/// `GrfLoadingStage::GLS_FILESCAN`, for `newgrf_loading.rs`.
pub const GLS_FILESCAN: u8 = GrfLoadingStage::GLS_FILESCAN.0;
/// `GrfLoadingStage::GLS_SAFETYSCAN`, for `newgrf_loading.rs`.
pub const GLS_SAFETYSCAN: u8 = GrfLoadingStage::GLS_SAFETYSCAN.0;
/// `GrfLoadingStage::GLS_LABELSCAN`, for `newgrf_loading.rs`.
pub const GLS_LABELSCAN: u8 = GrfLoadingStage::GLS_LABELSCAN.0;
/// `GrfLoadingStage::GLS_INIT`, for `newgrf_loading.rs`.
pub const GLS_INIT: u8 = GrfLoadingStage::GLS_INIT.0;
/// `GrfLoadingStage::GLS_RESERVE`, for `newgrf_loading.rs`.
pub const GLS_RESERVE: u8 = GrfLoadingStage::GLS_RESERVE.0;
/// `GrfLoadingStage::GLS_ACTIVATION`, for `newgrf_loading.rs`.
pub const GLS_ACTIVATION: u8 = GrfLoadingStage::GLS_ACTIVATION.0;
/// `GrfSpecFeature::GSF_TRAINS`, for `engine_loading.rs`, `newgrf_act2.rs` and
/// `newgrf_act3/loading.rs`.
pub const GSF_TRAINS: u8 = GrfSpecFeature::GSF_TRAINS.0;
/// `GrfSpecFeature::GSF_ROADVEHICLES`, for `engine_loading.rs`, `newgrf_act2.rs` and
/// `newgrf_act3/loading.rs`.
pub const GSF_ROADVEHICLES: u8 = GrfSpecFeature::GSF_ROADVEHICLES.0;
/// `GrfSpecFeature::GSF_SHIPS`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_SHIPS: u8 = GrfSpecFeature::GSF_SHIPS.0;
/// `GrfSpecFeature::GSF_AIRCRAFT`, for `newgrf_act2.rs`, `newgrf_act3/loading.rs` and
/// `newgrf_text/actions.rs`.
pub const GSF_AIRCRAFT: u8 = GrfSpecFeature::GSF_AIRCRAFT.0;
/// `GrfSpecFeature::GSF_STATIONS`, for `newgrf_act2.rs`, `newgrf_act3.rs` and
/// `newgrf_act3/loading.rs`.
pub const GSF_STATIONS: u8 = GrfSpecFeature::GSF_STATIONS.0;
/// `GrfSpecFeature::GSF_CANALS`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_CANALS: u8 = GrfSpecFeature::GSF_CANALS.0;
/// `GrfSpecFeature::GSF_HOUSES`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_HOUSES: u8 = GrfSpecFeature::GSF_HOUSES.0;
/// `GrfSpecFeature::GSF_GLOBALVAR`, for `newgrf_loading.rs`.
pub const GSF_GLOBALVAR: u8 = GrfSpecFeature::GSF_GLOBALVAR.0;
/// `GrfSpecFeature::GSF_INDUSTRYTILES`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_INDUSTRYTILES: u8 = GrfSpecFeature::GSF_INDUSTRYTILES.0;
/// `GrfSpecFeature::GSF_INDUSTRIES`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_INDUSTRIES: u8 = GrfSpecFeature::GSF_INDUSTRIES.0;
/// `GrfSpecFeature::GSF_CARGOES`, for `cargo_registry.rs`, `newgrf_act2.rs`,
/// `newgrf_act3/loading.rs` and `newgrf_loading.rs`.
pub const GSF_CARGOES: u8 = GrfSpecFeature::GSF_CARGOES.0;
/// `GrfSpecFeature::GSF_AIRPORTS`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_AIRPORTS: u8 = GrfSpecFeature::GSF_AIRPORTS.0;
/// `GrfSpecFeature::GSF_OBJECTS`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_OBJECTS: u8 = GrfSpecFeature::GSF_OBJECTS.0;
/// `GrfSpecFeature::GSF_RAILTYPES`, for `newgrf_act2.rs`, `newgrf_act3/loading.rs` and
/// `type_definitions.rs`.
pub const GSF_RAILTYPES: u8 = GrfSpecFeature::GSF_RAILTYPES.0;
/// `GrfSpecFeature::GSF_AIRPORTTILES`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_AIRPORTTILES: u8 = GrfSpecFeature::GSF_AIRPORTTILES.0;
/// `GrfSpecFeature::GSF_ROADTYPES`, for `newgrf_act2.rs`, `newgrf_act3/loading.rs` and
/// `type_definitions.rs`.
pub const GSF_ROADTYPES: u8 = GrfSpecFeature::GSF_ROADTYPES.0;
/// `GrfSpecFeature::GSF_TRAMTYPES`, for `newgrf_act2.rs` and `newgrf_act3/loading.rs`.
pub const GSF_TRAMTYPES: u8 = GrfSpecFeature::GSF_TRAMTYPES.0;
/// `GrfSpecFeature::GSF_ROADSTOPS`, for `newgrf_act2.rs`, `newgrf_act3.rs` and
/// `newgrf_act3/loading.rs`.
pub const GSF_ROADSTOPS: u8 = GrfSpecFeature::GSF_ROADSTOPS.0;
/// `GrfSpecFeature::GSF_BADGES`, for `newgrf_act2.rs`, `newgrf_act3/loading.rs` and
/// `newgrf_text/actions.rs`.
pub const GSF_BADGES: u8 = GrfSpecFeature::GSF_BADGES.0;
/// `GrfSpecFeature::GSF_END`, for `newgrf_act2.rs`, `newgrf_act3/loading.rs`,
/// `newgrf_loading.rs` and `newgrf_text/actions.rs`.
pub const GSF_END: u8 = GrfSpecFeature::GSF_END.0;
/// `GrfSpecFeature::GSF_DEFAULT`, for `newgrf_act3/loading.rs`.
pub const GSF_DEFAULT: u8 = GrfSpecFeature::GSF_DEFAULT.0;
/// `GrfSpecFeature::GSF_ORIGINAL_STRINGS`, for `newgrf_text/actions.rs`.
pub const GSF_ORIGINAL_STRINGS: u8 = GrfSpecFeature::GSF_ORIGINAL_STRINGS.0;

/// Test-only global-variable table with every leaf unreachable; its private fields keep it here.
#[cfg(test)]
pub(crate) const TEST_TABLE: Services = Services {
    get_grf_config: crate::test_tables::unreachable2,
    misc_grf_features: crate::test_tables::unreachable0,
    openttd_newgrf_version: crate::test_tables::unreachable0,
    palette: crate::test_tables::unreachable1,
    shared: std::ptr::null(),
};

#[cfg(test)]
mod tests {
    use super::{GRFFileRef, GrfMiscBit, GrfMiscBits, TEST_TABLE, get_global_variable};
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

    static MISC_GRF_FEATURES: AtomicU8 = AtomicU8::new(0);
    static TRAININFO_VEHICLE_WIDTH: AtomicU32 = AtomicU32::new(0);
    static FAILED_ASSERTIONS: AtomicU32 = AtomicU32::new(0);

    extern "C" fn misc_grf_features() -> u8 {
        MISC_GRF_FEATURES.load(Ordering::Relaxed)
    }
    extern "C" fn traininfo_vehicle_width(_: *const c_void) -> u32 {
        TRAININFO_VEHICLE_WIDTH.load(Ordering::Relaxed)
    }
    extern "C" fn record_assertion(condition: bool) {
        if !condition {
            FAILED_ASSERTIONS.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// `newgrf/newgrf_act0_globalvar.cpp`'s variable `0x1E` asserts that the stored bits lack
    /// `TrainWidth32Pixels` and then sets it only for a file of the full width, 32.
    #[test]
    #[allow(unsafe_code)] // A dangling file record that only the width stub receives.
    fn misc_grf_features_assert_the_local_bit_and_add_the_file_width() {
        if crate::services_assert::isolate_test_check(
            "newgrf::tests::misc_grf_features_assert_the_local_bit_and_add_the_file_width",
            record_assertion,
        ) {
            return;
        }
        let mut shared = crate::test_tables::SHARED;
        shared.grf_file_traininfo_vehicle_width = traininfo_vehicle_width;
        let mut global = TEST_TABLE;
        global.misc_grf_features = misc_grf_features;
        global.shared = &raw const shared;
        // SAFETY: Only the width stub receives the record, and it does not read it.
        let file = unsafe { GRFFileRef::of(&shared, std::ptr::NonNull::dangling().as_ptr()) };
        let local = GrfMiscBits(1 << GrfMiscBit::TrainWidth32Pixels.0).0;
        let ambient = GrfMiscBits(1 << GrfMiscBit::AmbientSoundCallback.0).0;

        // A stored local bit fails the assertion, and the body goes on as the original's does.
        MISC_GRF_FEATURES.store(ambient | local, Ordering::Relaxed);
        TRAININFO_VEHICLE_WIDTH.store(29, Ordering::Relaxed);
        let value = get_global_variable(&global, 0x1E, Some(file));
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 1);
        assert_eq!(value, Some(u32::from(ambient | local)));

        MISC_GRF_FEATURES.store(ambient, Ordering::Relaxed);
        assert_eq!(
            get_global_variable(&global, 0x1E, Some(file)),
            Some(u32::from(ambient))
        );
        TRAININFO_VEHICLE_WIDTH.store(32, Ordering::Relaxed);
        assert_eq!(
            get_global_variable(&global, 0x1E, Some(file)),
            Some(u32::from(ambient | local))
        );
        assert_eq!(FAILED_ASSERTIONS.load(Ordering::Relaxed), 1);
    }
}
