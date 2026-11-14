/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Types related to news (`news_type.h`).

use crate::engine_type::EngineID;
use crate::industry_type::IndustryID;
use crate::station_type::StationID;
use crate::tile_type::TileIndex;
use crate::town_type::TownId;
use crate::vehicle_type::VehicleID;

/// Type of news (`NewsType`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewsType(pub u8);
#[allow(non_upper_case_globals, reason = "NewsType enumerator spelling")]
impl NewsType {
    /// First vehicle arrived for company
    pub const ArrivalCompany: Self = Self(0);
    /// First vehicle arrived for competitor
    pub const ArrivalOther: Self = Self(1);
    /// An accident or disaster has occurred
    pub const Accident: Self = Self(2);
    /// An accident or disaster has occurred
    pub const AccidentOther: Self = Self(3);
    /// Company info (new companies, bankruptcy messages)
    pub const CompanyInfo: Self = Self(4);
    /// Opening of industries
    pub const IndustryOpen: Self = Self(5);
    /// Closing of industries
    pub const IndustryClose: Self = Self(6);
    /// Economic changes (recession, industry up/dowm)
    pub const Economy: Self = Self(7);
    /// Production changes of industry serviced by local company
    pub const IndustryCompany: Self = Self(8);
    /// Production changes of industry serviced by competitor(s)
    pub const IndustryOther: Self = Self(9);
    /// Other industry production changes
    pub const IndustryNobody: Self = Self(10);
    /// Bits of news about vehicles of the company
    pub const Advice: Self = Self(11);
    /// New vehicle has become available
    pub const NewVehicles: Self = Self(12);
    /// A type of cargo is (no longer) accepted
    pub const Acceptance: Self = Self(13);
    /// News about subsidies (announcements, expirations, acceptance)
    pub const Subsidies: Self = Self(14);
    /// General news (from towns)
    pub const General: Self = Self(15);
}

/// Sub type of the `NewsType::Advice` to be able to remove specific news items (`AdviceType`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdviceType(pub u8);
#[allow(non_upper_case_globals, reason = "AdviceType enumerator spelling")]
impl AdviceType {
    /// Next (order) destination is too far for the aircraft type.
    pub const AircraftDestinationTooFar: Self = Self(0);
    /// The vehicle is waiting in the depot.
    pub const VehicleWaiting: Self = Self(8);
    /// No advice type.
    pub const Invalid: Self = Self(9);
}

/// News Window Styles (`NewsStyle`).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewsStyle(pub u8);
#[allow(non_upper_case_globals, reason = "NewsStyle enumerator spelling")]
impl NewsStyle {
    /// Thin news item. (Newspaper with headline and viewport)
    pub const Thin: Self = Self(0);
    /// Small news item. (Information window with text and viewport)
    pub const Small: Self = Self(1);
    /// Normal news item. (Newspaper with text only)
    pub const Normal: Self = Self(2);
    /// Vehicle news item. (new engine available)
    pub const Vehicle: Self = Self(3);
    /// Company news item. (Newspaper with face)
    pub const Company: Self = Self(4);
}

/// Various OR-able news-item flags (`NewsFlag`).
///
/// `NewsFlag::InColour` is set automatically if needed.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewsFlag(pub u8);
#[allow(non_upper_case_globals, reason = "NewsFlag enumerator spelling")]
impl NewsFlag {
    /// News item is shown in colour (otherwise it is shown in black & white).
    pub const InColour: Self = Self(0);
    /// News item disables transparency in the viewport.
    pub const NoTransparency: Self = Self(1);
    /// News item uses shaded colours.
    pub const Shaded: Self = Self(2);
    /// String param 0 contains a vehicle ID. (special autoreplace behaviour)
    pub const VehicleParam0: Self = Self(3);
}

/// `NewsFlags`, an `EnumBitSet<NewsFlag, uint8_t>`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NewsFlags(pub u8);
impl NewsFlags {
    /// The set `{flags...}`.
    #[must_use]
    pub const fn new(flags: &[NewsFlag]) -> Self {
        let mut bits = 0;
        let mut i = 0;
        while i < flags.len() {
            bits |= 1 << flags[i].0;
            i += 1;
        }
        Self(bits)
    }
}
impl From<NewsFlag> for NewsFlags {
    fn from(flag: NewsFlag) -> Self {
        Self::new(&[flag])
    }
}

/// References to objects in news (`NewsReference`).
///
/// Be careful! Vehicles are a special case, as news are kept when vehicles are
/// autoreplaced/renewed. You have to make sure, `ChangeVehicleNews` catches the DParams of
/// your message. This is NOT ensured by the references.
///
/// The original is a `std::variant`; it crosses as a [`NewsReferenceRecord`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NewsReference {
    /// `std::monostate`.
    #[default]
    None,
    /// A tile.
    Tile(TileIndex),
    /// A vehicle.
    Vehicle(VehicleID),
    /// A station.
    Station(StationID),
    /// An industry.
    Industry(IndustryID),
    /// A town.
    Town(TownId),
    /// An engine.
    Engine(EngineID),
}

crate::abi::shared_records! {
    fn abi_record;
    /// A `NewsReference`: the variant's index and its value as the base integer.
    "OpenTTDNewsReference" ["news"] => pub struct NewsReferenceRecord {
        /// The alternative's base integer; zero for `std::monostate`.
        pub value: u32,
        /// `std::variant::index()`, in the original's alternative order.
        pub index: u8,
    }
}

impl NewsReference {
    /// The record that crosses to the native `AddNewsItem`.
    #[must_use]
    pub fn record(self) -> NewsReferenceRecord {
        let (index, value) = match self {
            Self::None => (0, 0),
            Self::Tile(tile) => (1, tile.0),
            Self::Vehicle(vehicle) => (2, vehicle.0),
            Self::Station(station) => (3, u32::from(station.0)),
            Self::Industry(industry) => (4, u32::from(industry.0)),
            Self::Town(town) => (5, u32::from(town.0)),
            Self::Engine(engine) => (6, u32::from(engine.0)),
        };
        NewsReferenceRecord { value, index }
    }
}

/// Container for any custom data that must be deleted after the news item has reached
/// end-of-life (`NewsAllocatedData`); the native object is only owned through a pointer.
#[repr(C)]
pub struct NewsAllocatedData {
    _private: [u8; 0],
}

/// Data that needs to be stored for company news messages (`CompanyNewsInformation`).
/// The problem with company news messages are the custom name of the companies and the fact
/// that the company data is reset, resulting in wrong names and such.
///
/// The native object is owned through this pointer until [`Self::into_data`] passes it to a
/// news item.
pub(crate) struct CompanyNewsInformation(*mut NewsAllocatedData);

impl CompanyNewsInformation {
    /// `std::make_unique<CompanyNewsInformation>(title, Company::Get(c), other)`; `None` is
    /// the original's default `other = nullptr`.
    pub(crate) fn new(
        title: crate::strings_type::StringID,
        c: crate::company_type::CompanyID,
        other: Option<crate::company_type::CompanyID>,
    ) -> Self {
        let other = other.unwrap_or(crate::company_type::CompanyID::INVALID);
        Self((crate::news_func::services().company_news_information)(
            title.0, c.0, other.0,
        ))
    }

    /// `company_name`: the name of the company.
    pub(crate) fn company_name(&self) -> Vec<u8> {
        let mut name = Vec::new();
        let (sink, target) = crate::strings_func::sink_into(&mut name);
        (crate::news_func::services().company_news_company_name)(self.0, sink, target);
        name
    }

    /// The owning pointer, for `AddNewsItem`.
    pub(crate) fn into_data(self) -> *mut NewsAllocatedData {
        self.0
    }
}
