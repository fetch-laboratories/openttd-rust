/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `rail.cpp` and the `rail.h` helpers over the rail type table.
//!
//! `RailTypeInfo` stays a C++ class in `_railtypes` until its storage port. Rust resolves
//! a rail type once with [`get_rail_type_info`] and reads the members it needs through the
//! returned [`RailTypeInfoRef`]. A `RailTypeInfoRef` is valid for the call that resolved it.
use crate::bitmath_func::SetBitIterator;
use crate::company_type::CompanyID;
use crate::engine_type::VehicleAccelerationModel;
use crate::map_func::World;
use crate::rail_type::{RailType, RailTypeLabel, RailTypes};
use crate::services::Services as Shared;
use crate::tile_type::{TileIndex, TileType};
use crate::timer_game_calendar::CalendarDate;
use crate::timer_game_common::MAX_DATE;
use crate::transport_type::TransportType;
use crate::type_availability::{Services, company_available, services};
use crate::vehicle::{RailVehicleTypes, VehicleType};

/// Native `RailTypeInfo`, opaque: Rust holds a `GetRailTypeInfo` result only through a
/// [`RailTypeInfoRef`].
#[repr(C)]
pub struct RailTypeInfo {
    _private: [u8; 0],
}

/// A resolved `const RailTypeInfo *` with the shared table whose readers it calls.
#[derive(Clone, Copy)]
pub(crate) struct RailTypeInfoRef<'a> {
    shared: &'a Shared,
    rti: *const RailTypeInfo,
}

/// `GetRailTypeInfo`: returns the rail type information for a given rail type, through the
/// native helper and its `assert(railtype < RAILTYPE_END)`.
#[inline]
pub(crate) fn get_rail_type_info(shared: &Shared, railtype: RailType) -> RailTypeInfoRef<'_> {
    RailTypeInfoRef {
        shared,
        rti: (shared.rail_type_info)(railtype),
    }
}

impl RailTypeInfoRef<'_> {
    /// `RailTypeInfo::acceleration_type`: acceleration type of this rail type.
    #[inline]
    pub(crate) fn acceleration_type(self) -> VehicleAccelerationModel {
        VehicleAccelerationModel((self.shared.rail_type_acceleration_type)(self.rti))
    }
    /// `RailTypeInfo::compatible_railtypes`: bit mask of rail type numbers that are
    /// compatible with this rail type.
    #[inline]
    pub(crate) fn compatible_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_compatible_railtypes)(self.rti))
    }
    /// `RailTypeInfo::curve_speed`: multiplier for curve maximum speed advantage.
    #[inline]
    pub(crate) fn curve_speed(self) -> u8 {
        (self.shared.rail_type_curve_speed)(self.rti)
    }
    /// `RailTypeInfo::introduces_railtypes`: bitmask of which other rail types are
    /// introduced when this rail type is introduced.
    #[inline]
    pub(crate) fn introduces_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_introduces_railtypes)(self.rti))
    }
    /// `RailTypeInfo::introduction_date`: introduction date. When `INVALID_DATE` or a
    /// vehicle using this rail type gets introduced earlier, the vehicle's introduction
    /// date will be used instead for this rail type.
    #[inline]
    pub(crate) fn introduction_date(self) -> CalendarDate {
        CalendarDate((self.shared.rail_type_introduction_date)(self.rti))
    }
    /// `RailTypeInfo::introduction_required_railtypes`: bitmask of rail types that are
    /// required for this rail type to be introduced at a given `introduction_date`.
    #[inline]
    pub(crate) fn introduction_required_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_introduction_required_railtypes)(
            self.rti,
        ))
    }
    /// `RailTypeInfo::label`: unique 32 bit rail type identifier.
    #[inline]
    pub(crate) fn label(self) -> RailTypeLabel {
        RailTypeLabel((self.shared.rail_type_label)(self.rti))
    }
    /// `RailTypeInfo::max_speed`: maximum speed for vehicles travelling on this rail type.
    #[inline]
    pub(crate) fn max_speed(self) -> u16 {
        (self.shared.rail_type_max_speed)(self.rti)
    }
    /// `RailTypeInfo::powered_railtypes`: bit mask of rail type numbers that are powered
    /// on this rail type.
    #[inline]
    pub(crate) fn powered_railtypes(self) -> RailTypes {
        RailTypes((self.shared.rail_type_powered_railtypes)(self.rti))
    }
}

/// `RailTypeSpriteGroup`: sprite groups for a rail type.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RailTypeSpriteGroup(pub u8);

impl RailTypeSpriteGroup {
    /// Cursor and toolbar icon images
    pub const RTSG_CURSORS: Self = Self(0);
    /// Images for overlaying track
    pub const RTSG_OVERLAY: Self = Self(1);
    /// Main group of ground images
    pub const RTSG_GROUND: Self = Self(2);
    /// Main group of ground images for snow or desert
    pub const RTSG_TUNNEL: Self = Self(3);
    /// Catenary wires
    pub const RTSG_WIRES: Self = Self(4);
    /// Catenary pylons
    pub const RTSG_PYLONS: Self = Self(5);
    /// Bridge surface images
    pub const RTSG_BRIDGE: Self = Self(6);
    /// Level crossing overlay images
    pub const RTSG_CROSSING: Self = Self(7);
    /// Depot images
    pub const RTSG_DEPOT: Self = Self(8);
    /// Fence images
    pub const RTSG_FENCES: Self = Self(9);
    /// Tunnel portal overlay
    pub const RTSG_TUNNEL_PORTAL: Self = Self(10);
    /// Signal images
    pub const RTSG_SIGNALS: Self = Self(11);
    /// Complete ground images
    pub const RTSG_GROUND_COMPLETE: Self = Self(12);
    /// End marker
    pub const RTSG_END: Self = Self(13);
}

/// `RailTypeSpriteGroup::RTSG_END`, for the files no pass has finished.
pub const RTSG_END: u8 = RailTypeSpriteGroup::RTSG_END.0;

/// `GetAllCompatibleRailTypes`: returns all compatible rail types for a set of rail types.
pub(crate) fn get_all_compatible_rail_types(shared: &Shared, railtypes: RailTypes) -> RailTypes {
    let mut compatible = RailTypes::EMPTY;
    for rt in SetBitIterator::new(railtypes.0).map(RailType) {
        compatible = compatible.with(get_rail_type_info(shared, rt).compatible_railtypes());
    }
    compatible
}

/// `GetAllPoweredRailTypes`: returns all powered rail types for a set of rail types.
pub(crate) fn get_all_powered_rail_types(shared: &Shared, railtypes: RailTypes) -> RailTypes {
    let mut powered = RailTypes::EMPTY;
    for rt in SetBitIterator::new(railtypes.0).map(RailType) {
        powered = powered.with(get_rail_type_info(shared, rt).powered_railtypes());
    }
    powered
}

/// `GetAllIntroducesRailTypes`: returns all introduced rail types for a set of rail types.
pub(crate) fn get_all_introduces_rail_types(shared: &Shared, railtypes: RailTypes) -> RailTypes {
    let mut introduces = RailTypes::EMPTY;
    for rt in SetBitIterator::new(railtypes.0).map(RailType) {
        introduces = introduces.with(get_rail_type_info(shared, rt).introduces_railtypes());
    }
    introduces
}

/// `IsCompatibleRail(RailType, RailType)`: checks if an engine of the given `RailType` can
/// drive on a tile with a given `RailType`. This would normally just be an equality check,
/// but for electric rails (which also support non-electric engines).
pub(crate) fn is_compatible_rail(
    shared: &Shared,
    enginetype: RailType,
    tiletype: RailType,
) -> bool {
    get_rail_type_info(shared, enginetype)
        .compatible_railtypes()
        .test(tiletype)
}

/// `IsCompatibleRail(RailTypes, RailType)`: checks if an engine of the given `RailTypes`
/// can drive on a tile with a given `RailType`.
pub(crate) fn is_compatible_rail_types(
    shared: &Shared,
    enginetype: RailTypes,
    tiletype: RailType,
) -> bool {
    get_all_compatible_rail_types(shared, enginetype).test(tiletype)
}

/// `HasPowerOnRail(RailType, RailType)`: checks if an engine of the given `RailType` got
/// power on a tile with a given `RailType`. This would normally just be an equality check,
/// but for electric rails (which also support non-electric engines).
pub(crate) fn has_power_on_rail(shared: &Shared, enginetype: RailType, tiletype: RailType) -> bool {
    get_rail_type_info(shared, enginetype)
        .powered_railtypes()
        .test(tiletype)
}

/// `HasPowerOnRail(RailTypes, RailType)`: checks if an engine of the given `RailTypes` got
/// power on a tile with a given `RailType`.
pub(crate) fn has_power_on_rail_types(
    shared: &Shared,
    enginetype: RailTypes,
    tiletype: RailType,
) -> bool {
    get_all_powered_rail_types(shared, enginetype).test(tiletype)
}

impl World<'_> {
    /// `GetTileRailType`: return the rail type of tile, or `INVALID_RAILTYPE` if this is no
    /// rail tile.
    pub(crate) fn get_tile_rail_type(&self, tile: TileIndex) -> RailType {
        match self.kind(tile) {
            TileType::MP_RAILWAY => return self.get_rail_type(tile),
            TileType::MP_ROAD => {
                /* rail/road crossing */
                if self.level_crossing(tile.0) {
                    return self.get_rail_type(tile);
                }
            }
            TileType::MP_STATION => {
                if self.has_station_rail(tile) {
                    return self.get_rail_type(tile);
                }
            }
            TileType::MP_TUNNELBRIDGE => {
                if TransportType(self.tunnel_bridge_transport(tile.0))
                    == TransportType::TRANSPORT_RAIL
                {
                    return self.get_rail_type(tile);
                }
            }
            _ => {}
        }
        RailType::INVALID_RAILTYPE
    }
}

/// `HasRailTypeAvail`: finds out if a company has a certain buildable rail type available.
pub(crate) fn has_rail_type_avail(s: &Services, company: CompanyID, railtype: RailType) -> bool {
    !RailTypes(crate::type_registry::rail_hidden()).test(railtype)
        && RailTypes((s.company_railtypes)(company.0)).test(railtype)
}

/// `HasAnyRailTypesAvail`: test if any buildable rail type is available for a company.
pub(crate) fn has_any_rail_types_avail(s: &Services, company: CompanyID) -> bool {
    let avail = (s.company_railtypes)(company.0) & !crate::type_registry::rail_hidden();
    RailTypes(avail).any()
}

/// `ValParamRailType`: validate functions for rail building.
pub(crate) fn val_param_rail_type(shared: &Shared, rail: RailType) -> bool {
    rail < RailType::RAILTYPE_END
        && has_rail_type_avail(
            services(shared),
            CompanyID((shared.current_company)()),
            rail,
        )
}

/// `AddDateIntroducedRailTypes`: add the rail types that are to be introduced at the given
/// date.
#[allow(unsafe_code)]
pub(crate) fn add_date_introduced_rail_types(
    s: &Services,
    current: RailTypes,
    date: CalendarDate,
) -> RailTypes {
    // SAFETY: The availability table names the process-lifetime shared table.
    let shared = unsafe { &*s.shared };
    let mut rts = current;

    for rt in RailType::RAILTYPE_BEGIN.0..RailType::RAILTYPE_END.0 {
        let rti = get_rail_type_info(shared, RailType(rt));
        /* Unused rail type. */
        if rti.label() == RailTypeLabel(0) {
            continue;
        }

        /* Not date introduced. */
        let introduction_date = rti.introduction_date();
        if !(0..MAX_DATE).contains(&introduction_date.0) {
            continue;
        }

        /* Not yet introduced at this date. */
        if introduction_date > date {
            continue;
        }

        /* Have we introduced all required railtypes? */
        let required = rti.introduction_required_railtypes();
        if rts.0 & required.0 != required.0 {
            continue;
        }

        rts = rts.with(rti.introduces_railtypes());
    }

    /* When we added railtypes we need to run this method again; the added
     * railtypes might enable more rail types to become introduced. */
    if rts == current {
        rts
    } else {
        add_date_introduced_rail_types(s, rts, date)
    }
}

/// The `RailVehicleInfo::railtypes` of a train engine the caller's gates admitted, or its
/// introduced rail types, after the original's `assert(rvi->railtypes.Any())`.
#[allow(unsafe_code)]
fn engine_rail_types(s: &Services, railtypes: RailTypes, introduces: bool) -> RailTypes {
    crate::services_assert::assert_with(|| railtypes.any());
    if introduces {
        // SAFETY: The availability table names the process-lifetime shared table.
        get_all_introduces_rail_types(unsafe { &*s.shared }, railtypes)
    } else {
        railtypes
    }
}

/// `GetCompanyRailTypes`: get the rail types the given company can build.
#[allow(unsafe_code)]
pub(crate) fn get_company_rail_types(
    s: &Services,
    company: CompanyID,
    introduces: bool,
) -> RailTypes {
    let mut rts = RailTypes::EMPTY;
    let mut from = 0;
    loop {
        // SAFETY: Native pool leaves resolve one live shell; no borrow crosses calls.
        let e = unsafe { (s.next)(from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if e.kind != VehicleType::TRAIN {
            continue;
        }
        let engine = crate::vehicle::EngineRef::of(e.shell);
        // SAFETY: The availability table names the process-lifetime shared table.
        let climate = unsafe { (&*s.shared).settings().climate.read() };
        // SAFETY: `next` resolved this live engine shell; the reads end before any leaf.
        if engine.climate(climate) && unsafe { company_available(e, company.0) } {
            if engine.rail_kind() != RailVehicleTypes::WAGON {
                rts = rts.with(engine_rail_types(
                    s,
                    RailTypes(engine.railtypes()),
                    introduces,
                ));
            }
        }
    }

    if introduces {
        return add_date_introduced_rail_types(s, rts, crate::timer_game_common::calendar_date());
    }
    rts
}

/// `GetRailTypes`: get list of rail types, regardless of company availability.
#[allow(unsafe_code)]
pub(crate) fn get_rail_types(s: &Services, introduces: bool) -> RailTypes {
    let mut rts = RailTypes::EMPTY;
    let mut from = 0;
    loop {
        // SAFETY: Native pool leaves resolve one live shell; no borrow crosses calls.
        let e = unsafe { (s.next)(from) };
        if e.shell.is_null() {
            break;
        }
        from = u32::from(e.id) + 1;
        if e.kind != VehicleType::TRAIN {
            continue;
        }
        let engine = crate::vehicle::EngineRef::of(e.shell);
        // SAFETY: The availability table names the process-lifetime shared table.
        let climate = unsafe { (&*s.shared).settings().climate.read() };
        if !engine.climate(climate) {
            continue;
        }

        if engine.rail_kind() != RailVehicleTypes::WAGON {
            rts = rts.with(engine_rail_types(
                s,
                RailTypes(engine.railtypes()),
                introduces,
            ));
        }
    }

    if introduces {
        return add_date_introduced_rail_types(s, rts, CalendarDate(MAX_DATE));
    }
    rts
}

/* Shims for `src/rail.cpp`'s facades and the files no pass has finished. */

#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_rail_type(s: &Services, company: u8, rt: u8) -> u8 {
    u8::from(has_rail_type_avail(s, CompanyID(company), RailType(rt)))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_has_any_rail_types_abi(s: &Services, company: u8) -> u8 {
    u8::from(has_any_rail_types_avail(s, CompanyID(company)))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_val_rail_type(shared: &Shared, rt: u8) -> u8 {
    u8::from(val_param_rail_type(shared, RailType(rt)))
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_add_date_rail_types(s: &Services, current: u64, date: i32) -> u64 {
    add_date_introduced_rail_types(s, RailTypes(current), CalendarDate(date)).0
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_rail_types(
    s: &Services,
    company: u8,
    introduces: bool,
) -> u64 {
    get_company_rail_types(s, CompanyID(company), introduces).0
}
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_rail_types(s: &Services, introduces: bool) -> u64 {
    get_rail_types(s, introduces).0
}

/// Raw adapter: `HasRailTypeAvail`.
pub fn has_rail_type_avail_raw(s: &Services, company: u8, rt: u8) -> bool {
    has_rail_type_avail(s, CompanyID(company), RailType(rt))
}
/// Raw adapter: `ValParamRailType`.
pub fn val_param_rail_type_raw(shared: &Shared, rt: u8) -> bool {
    val_param_rail_type(shared, RailType(rt))
}
/// Raw adapter: `IsCompatibleRail(RailType, RailType)`.
pub(crate) fn is_compatible_rail_raw(shared: &Shared, engine: u8, tile: u8) -> bool {
    is_compatible_rail(shared, RailType(engine), RailType(tile))
}
/// Raw adapter: `GetAllIntroducesRailTypes`.
pub fn get_all_introduces_rail_types_raw(shared: &Shared, types: u64) -> u64 {
    get_all_introduces_rail_types(shared, RailTypes(types)).0
}
