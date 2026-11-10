/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Company identity, colours, names and livery selection. Native shells retain
//! unmoved fields. Every borrowed name span ends before its next native mutation.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
use crate::company_type::{CompanyID, CompanyManagerFace, OpenTTDCompany};
use crate::engine_type::EngineID;
use crate::livery::{Livery, LiveryScheme, PRIMARY, SECONDARY};
use crate::raw_field::{read_field, write_field};
use crate::services::{ByteSpan, Services as Shared};
use crate::strings_type::StringID;
use crate::tile_type::TileIndex;
use crate::vehicle_type::{OpenTTDVehicle, VehicleID};
use std::mem::MaybeUninit;
type Town = crate::town_type::TownHandle;
#[repr(C, align(16))]
pub struct NameStorage {
    bytes: [MaybeUninit<u8>; 128],
}
#[repr(C)]
pub struct Strings {
    pub unnamed: u32,
    pub president: u32,
    pub company: u32,
    pub president_generated: u32,
    pub andco: u32,
    pub town_begin: u32,
    pub town_end: u32,
    pub company_begin: u32,
}
#[repr(C)]
pub struct Services {
    pub shared: &'static Shared,
    pub randomizer: *mut crate::town::names::Randomizer,
    pub faces: &'static crate::company_manager_face::Services,
    pub groups: &'static crate::fleet::GroupServices,
    pub strings: Strings,
    pub company: extern "C" fn(CompanyID) -> *mut OpenTTDCompany,
    pub next_company: extern "C" fn(u32) -> *mut OpenTTDCompany,
    pub company_id: extern "C" fn(*mut OpenTTDCompany) -> CompanyID,
    pub renew_reserve: extern "C" fn(*const OpenTTDCompany) -> u32,
    pub name_id: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_name_id: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub name_parts: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_name_parts: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub president_id: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_president_id: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub president_parts: extern "C" fn(*mut OpenTTDCompany) -> u32,
    pub set_president_parts: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    pub colour: extern "C" fn(*mut OpenTTDCompany) -> u8,
    pub set_colour: extern "C" fn(*mut OpenTTDCompany, u8) -> (),
    pub last_build: extern "C" fn(*mut OpenTTDCompany) -> TileIndex,
    pub is_ai: extern "C" fn(*mut OpenTTDCompany) -> bool,
    pub name: extern "C" fn(*mut OpenTTDCompany) -> ByteSpan,
    pub set_name: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> (),
    pub president: extern "C" fn(*mut OpenTTDCompany) -> ByteSpan,
    pub set_president: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> (),
    pub livery: extern "C" fn(*mut OpenTTDCompany, u8) -> *mut Livery,
    pub face: extern "C" fn(*mut OpenTTDCompany) -> *mut CompanyManagerFace,
    pub name_frame_init: extern "C" fn(*mut NameStorage) -> (),
    pub name_frame_drop: extern "C" fn(*mut NameStorage) -> (),
    pub render: extern "C" fn(*mut NameStorage, u32, u32) -> ByteSpan,
    pub render_other: extern "C" fn(*mut NameStorage, u32, u32) -> ByteSpan,
    pub company_renamed: extern "C" fn(*mut OpenTTDCompany, *mut NameStorage) -> (),
    pub president_renamed: extern "C" fn(*mut OpenTTDCompany, *mut NameStorage) -> (),
    pub launch_news: extern "C" fn(*mut OpenTTDCompany, *mut Town) -> (),
    pub admin_update: extern "C" fn(CompanyID) -> (),
    pub rename_transport: extern "C" fn(ByteSpan) -> (),
    pub president_windows: extern "C" fn() -> (),
    pub colour_windows: extern "C" fn() -> (),
    pub object_colours: extern "C" fn(*mut OpenTTDCompany) -> (),
    pub reset_colourmap: extern "C" fn() -> (),
    /// The engine capabilities table, for engines, vehicle cargo and cargo classes.
    pub capabilities: *const crate::engine_capabilities::Services,
    pub wagon_override: extern "C" fn(*mut OpenTTDVehicle) -> bool,
    pub cargo_freight: extern "C" fn(u8) -> bool,
    pub vehicle_id: extern "C" fn(*mut OpenTTDVehicle) -> VehicleID,
    pub invalidate_vehicle: extern "C" fn(*mut OpenTTDVehicle) -> (),
}
impl Services {
    /// The engine capabilities table.
    fn capabilities(&self) -> &crate::engine_capabilities::Services {
        // SAFETY: Both tables are process-lifetime and immutable; the native table
        // names the canonical engine capabilities table.
        unsafe { &*self.capabilities }
    }
}
impl NameStorage {
    fn new() -> Self {
        Self {
            bytes: [MaybeUninit::uninit(); 128],
        }
    }
}
struct Names<'a> {
    s: &'a Services,
    slot: *mut NameStorage,
    _lifetime: std::marker::PhantomData<&'a mut NameStorage>,
}
impl<'a> Names<'a> {
    fn new(s: &'a Services, slot: &'a mut NameStorage) -> Self {
        (s.name_frame_init)(slot);
        Self {
            s,
            slot,
            _lifetime: std::marker::PhantomData,
        }
    }
    fn render(&mut self, id: u32, param: u32) -> ByteSpan {
        (self.s.render)(self.slot, id, param)
    }
}
impl Drop for Names<'_> {
    fn drop(&mut self) {
        (self.s.name_frame_drop)(self.slot);
    }
}
fn companies(s: &Services, mut f: impl FnMut(*mut OpenTTDCompany) -> bool) -> bool {
    let mut from = 0;
    loop {
        let c = (s.next_company)(from);
        if c.is_null() {
            return true;
        }
        from = u32::from((s.company_id)(c).0) + 1;
        if !f(c) {
            return false;
        }
    }
}
static mut COLOURS: [u8; crate::company_type::MAX_COMPANIES as usize] =
    [0; crate::company_type::MAX_COMPANIES as usize];
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_colours() -> *mut u8 {
    std::ptr::addr_of_mut!(COLOURS).cast()
}
fn cache_colour(id: CompanyID, colour: u8) {
    unsafe {
        openttd_rust_company_colours()
            .add(usize::from(id.0))
            .write(colour);
    }
}
/// Generate a company colour.
/// The original 100 draws precede weighting and pool-order colour elimination.
pub(crate) fn generate_company_colour(s: &Services) -> u8 {
    use crate::gfx_type::{
        COLOUR_BLUE, COLOUR_BROWN, COLOUR_CREAM, COLOUR_DARK_BLUE, COLOUR_DARK_GREEN, COLOUR_GREEN,
        COLOUR_GREY, COLOUR_LIGHT_BLUE, COLOUR_MAUVE, COLOUR_ORANGE, COLOUR_PALE_GREEN,
        COLOUR_PURPLE, COLOUR_WHITE, COLOUR_YELLOW, Colours,
    };
    // Sorting weights for the company colours (_colour_sort).
    const COLOUR_SORT: [u8; Colours::COLOUR_END.0 as usize] =
        [2, 2, 3, 2, 3, 2, 3, 2, 3, 2, 2, 2, 3, 1, 1, 1];
    // Similar colours, so we can try to prevent same coloured companies (_similar_colour).
    const SIMILAR_COLOUR: [[Colours; 2]; Colours::COLOUR_END.0 as usize] = [
        [Colours(COLOUR_BLUE), Colours(COLOUR_LIGHT_BLUE)], // COLOUR_DARK_BLUE
        [Colours(COLOUR_GREEN), Colours(COLOUR_DARK_GREEN)], // COLOUR_PALE_GREEN
        [Colours::INVALID_COLOUR, Colours::INVALID_COLOUR], // COLOUR_PINK
        [Colours(COLOUR_ORANGE), Colours::INVALID_COLOUR],  // COLOUR_YELLOW
        [Colours::INVALID_COLOUR, Colours::INVALID_COLOUR], // COLOUR_RED
        [Colours(COLOUR_DARK_BLUE), Colours(COLOUR_BLUE)],  // COLOUR_LIGHT_BLUE
        [Colours(COLOUR_PALE_GREEN), Colours(COLOUR_DARK_GREEN)], // COLOUR_GREEN
        [Colours(COLOUR_PALE_GREEN), Colours(COLOUR_GREEN)], // COLOUR_DARK_GREEN
        [Colours(COLOUR_DARK_BLUE), Colours(COLOUR_LIGHT_BLUE)], // COLOUR_BLUE
        [Colours(COLOUR_BROWN), Colours(COLOUR_ORANGE)],    // COLOUR_CREAM
        [Colours(COLOUR_PURPLE), Colours::INVALID_COLOUR],  // COLOUR_MAUVE
        [Colours(COLOUR_MAUVE), Colours::INVALID_COLOUR],   // COLOUR_PURPLE
        [Colours(COLOUR_YELLOW), Colours(COLOUR_CREAM)],    // COLOUR_ORANGE
        [Colours(COLOUR_CREAM), Colours::INVALID_COLOUR],   // COLOUR_BROWN
        [Colours(COLOUR_WHITE), Colours::INVALID_COLOUR],   // COLOUR_GREY
        [Colours(COLOUR_GREY), Colours::INVALID_COLOUR],    // COLOUR_WHITE
    ];
    // Initialize array.
    let mut colours =
        std::array::from_fn::<_, { Colours::COLOUR_END.0 as usize }, _>(|i| Colours(i as u8));
    // And randomize it.
    for _ in 0..100 {
        let r = s.shared.random();
        colours.swap(
            crate::bitmath_func::gb(r, 0, 4) as usize,
            crate::bitmath_func::gb(r, 4, 4) as usize,
        );
    }
    // Bubble sort it according to the values in table 1.
    for _ in 0..Colours::COLOUR_END.0 {
        for j in 1..usize::from(Colours::COLOUR_END.0) {
            if COLOUR_SORT[usize::from(colours[j - 1].0)] < COLOUR_SORT[usize::from(colours[j].0)] {
                colours.swap(j - 1, j);
            }
        }
    }
    // Move the colours that look similar to each company's colour to the side.
    companies(s, |c| {
        let pcolour = Colours((s.colour)(c));
        if let Some(i) = colours.iter().position(|&x| x == pcolour) {
            colours[i] = Colours::INVALID_COLOUR;
        }
        for similar in SIMILAR_COLOUR[usize::from(pcolour.0)] {
            if similar == Colours::INVALID_COLOUR {
                break;
            }
            for i in 1..usize::from(Colours::COLOUR_END.0) {
                if colours[i - 1] == similar {
                    colours.swap(i - 1, i);
                }
            }
        }
        true
    });
    // Return the first available colour.
    colours
        .into_iter()
        .find(|&x| x != Colours::INVALID_COLOUR)
        .expect("NOT_REACHED")
        .0
}
/// Generate the name of a company from the last build coordinate.
pub(crate) fn generate_company_name(s: &Services, c: *mut OpenTTDCompany) {
    if StringID((s.name_id)(c)) != StringID(s.strings.unnamed) || (s.last_build)(c).0 == 0 {
        return;
    }
    let t = crate::town::spatial::closest(s.shared, (s.last_build)(c).0, u32::MAX);
    let town = (s.shared.town_owner)(t);
    let simulation = unsafe { crate::town::openttd_rust_town_simulation(town) };
    let mut slot = NameStorage::new();
    let mut names = Names::new(s, &mut slot);
    let mut str = StringID(s.strings.andco);
    let mut strp;
    let town_name = unsafe { crate::town::names::openttd_rust_town_name(town).size == 0 }
        && crate::math::is_inside_mm(
            usize::from(read_field!(simulation, townnametype)),
            s.strings.town_begin as usize,
            s.strings.town_end as usize,
        );
    let mut try_town = town_name;
    loop {
        if try_town {
            str = StringID(
                u32::from(read_field!(simulation, townnametype)) - s.strings.town_begin
                    + s.strings.company_begin,
            );
            strp = read_field!(simulation, townnameparts);
        } else if StringID((s.president_id)(c)) == StringID(s.strings.president_generated) {
            strp = (s.president_parts)(c);
            names.render(str.0, strp);
            break;
        } else {
            strp = s.shared.random();
        }
        // No companies must have this name already.
        let unique = companies(s, |cc| {
            StringID((s.name_id)(cc)) != str || (s.name_parts)(cc) != strp
        });
        if unique {
            let name = names.render(str.0, strp);
            if (s.shared.utf8_length)(name.data, name.size)
                < crate::company_type::MAX_LENGTH_COMPANY_NAME_CHARS
            {
                break;
            }
        }
        str = StringID(s.strings.andco);
        try_town = false;
    }
    (s.set_name_id)(c, str.0);
    (s.set_name_parts)(c, strp);
    (s.shared.screen_dirty)();
    (s.company_renamed)(c, names.slot);
    if (s.is_ai)(c) {
        (s.launch_news)(c, t.cast_mut());
    }
}
/// Generate a random president name of a company.
pub(crate) fn generate_president_name(s: &Services, c: *mut OpenTTDCompany) {
    let mut slot = NameStorage::new();
    let mut names = Names::new(s, &mut slot);
    loop {
        (s.set_president_parts)(c, s.shared.random());
        (s.set_president_id)(c, s.strings.president_generated);
        // Reserve space for an extra unicode character so a too-long president
        // name can be detected.
        let name = names.render(s.strings.president, u32::from((s.company_id)(c).0));
        if (s.shared.utf8_length)(name.data, name.size)
            >= crate::company_type::MAX_LENGTH_PRESIDENT_NAME_CHARS
        {
            continue;
        }
        if companies(s, |cc| {
            cc == c
                || !name.same_bytes((s.render_other)(
                    names.slot,
                    s.strings.president,
                    u32::from((s.company_id)(cc).0),
                ))
        }) {
            return;
        }
    }
}
// Preserve the scalar entry names used by the existing company lifecycle.
pub(crate) use generate_company_colour as generate_colour;
pub(crate) use generate_company_name as generate_name;
pub(crate) use generate_president_name as generate_president;

/// Reset the livery schemes to the company's primary colour.
/// Used on loading games without livery information and on new company start up.
pub(crate) fn reset_company_livery(s: &Services, c: *mut OpenTTDCompany) {
    for scheme in LiveryScheme::LS_BEGIN.0..LiveryScheme::LS_END.0 {
        let l = (s.livery)(c, scheme);
        write_field!(l, in_use, 0);
        write_field!(l, colour1, (s.colour)(c));
        write_field!(l, colour2, (s.colour)(c));
    }
    let owner = (s.company_id)(c).0;
    let mut from = 0;
    loop {
        let id = unsafe { (s.groups.next_group)(from) };
        if id == u32::MAX {
            break;
        }
        from = id + 1;
        let g = unsafe { (s.groups.group)(id as u16) }.cast::<crate::fleet::GroupFields>();
        if read_field!(g, owner) == owner {
            let l = unsafe { std::ptr::addr_of_mut!((*g).livery) };
            write_field!(l, in_use, 0);
            write_field!(l, colour1, (s.colour)(c));
            write_field!(l, colour2, (s.colour)(c));
        }
    }
}
pub(crate) fn set_startup_colour(s: &Services, c: *mut OpenTTDCompany, colour: u8) {
    (s.set_colour)(c, colour);
    reset_company_livery(s, c);
    cache_colour((s.company_id)(c), (s.colour)(c));
}
/// Update liveries when the default scheme changes, for schemes with colours set to default.
pub(crate) fn update_company_liveries(s: &Services, c: *mut OpenTTDCompany) {
    let default = (s.livery)(c, LiveryScheme::LS_DEFAULT.0);
    for scheme in 1..LiveryScheme::LS_END.0 {
        let l = (s.livery)(c, scheme);
        if read_field!(l, in_use) & PRIMARY == 0 {
            write_field!(l, colour1, read_field!(default, colour1));
        }
        if read_field!(l, in_use) & SECONDARY == 0 {
            write_field!(l, colour2, read_field!(default, colour2));
        }
    }
    unsafe {
        crate::fleet::company_liveries(s.groups, (s.company_id)(c).0);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_reset_livery(s: &Services, c: *mut OpenTTDCompany) {
    reset_company_livery(s, c);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_update_liveries(s: &Services, c: *mut OpenTTDCompany) {
    update_company_liveries(s, c);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_manager_face_abi(
    s: &Services,
    style: u32,
    bits: u32,
    execute: bool,
) -> u8 {
    if !crate::company_manager_face::is_valid_company_manager_face(s.faces, style, bits) {
        return u8::from(false);
    }
    if execute {
        let c = (s.company)(CompanyID((s.shared.current_company)()));
        let face = (s.face)(c);
        crate::company_manager_face::set_company_manager_face_style(s.faces, face, style);
        unsafe {
            (s.faces.bits)(face).write(bits);
        }
        (s.shared.screen_dirty)();
    }
    u8::from(true)
}
/// Change the company colour for the selected livery scheme.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_colour_abi(
    s: &Services,
    scheme: u8,
    primary: bool,
    mut colour: u8,
    execute: bool,
) -> u8 {
    use crate::gfx_type::{COLOUR_END, INVALID_COLOUR};
    let scheme = LiveryScheme(scheme);
    if scheme >= LiveryScheme::LS_END || (colour >= COLOUR_END && colour != INVALID_COLOUR) {
        return u8::from(false);
    }
    // Default scheme can't be reset to invalid.
    if scheme == LiveryScheme::LS_DEFAULT && colour == INVALID_COLOUR {
        return u8::from(false);
    }
    let id = CompanyID((s.shared.current_company)());
    let c = (s.company)(id);
    // Ensure no two companies have the same primary colour.
    if scheme == LiveryScheme::LS_DEFAULT
        && primary
        && !companies(s, |cc| cc == c || (s.colour)(cc) != colour)
    {
        return u8::from(false);
    }
    if execute {
        let l = (s.livery)(c, scheme.0);
        let default = (s.livery)(c, LiveryScheme::LS_DEFAULT.0);
        let flag = if primary { PRIMARY } else { SECONDARY };
        if scheme != LiveryScheme::LS_DEFAULT {
            let flags = read_field!(l, in_use);
            write_field!(
                l,
                in_use,
                if colour == INVALID_COLOUR {
                    flags & !flag
                } else {
                    flags | flag
                }
            );
        }
        if primary {
            if colour == INVALID_COLOUR {
                colour = read_field!(default, colour1);
            }
            write_field!(l, colour1, colour);
            // If setting the first colour of the default scheme, adjust the
            // original and cached company colours too.
            if scheme == LiveryScheme::LS_DEFAULT {
                update_company_liveries(s, c);
                cache_colour(id, colour);
                (s.set_colour)(c, colour);
                (s.admin_update)(id);
            }
        } else {
            if colour == INVALID_COLOUR {
                colour = read_field!(default, colour2);
            }
            write_field!(l, colour2, colour);
            if scheme == LiveryScheme::LS_DEFAULT {
                update_company_liveries(s, c);
            }
        }
        if read_field!(l, in_use) & (PRIMARY | SECONDARY) != 0 {
            // If enabling a scheme, set the default scheme to be in use too.
            write_field!(default, in_use, read_field!(default, in_use) | PRIMARY);
        } else {
            // Else loop through all schemes to see if any are left enabled.
            // If not, disable the default scheme too.
            write_field!(
                default,
                in_use,
                read_field!(default, in_use) & !(PRIMARY | SECONDARY)
            );
            for i in LiveryScheme::LS_DEFAULT.0..LiveryScheme::LS_END.0 {
                if read_field!((s.livery)(c, i), in_use) & (PRIMARY | SECONDARY) != 0 {
                    write_field!(default, in_use, read_field!(default, in_use) | PRIMARY);
                    break;
                }
            }
        }
        (s.reset_colourmap)();
        (s.shared.screen_dirty)();
        // All graphs related to companies use the company colour.
        // The smallmap owner view also stores the company colours.
        (s.colour_windows)();
        // Company colour data is indirectly cached.
        let mut from = 0;
        while let Some(v) = (s.shared.vehicle_next)(from) {
            let shell = v.0.as_ptr();
            from = (s.vehicle_id)(shell).0 + 1;
            if crate::company_type::CompanyID(v.owner()) == id {
                (s.invalidate_vehicle)(shell);
            }
        }
        (s.object_colours)(c);
    }
    u8::from(true)
}
/// Shared implementation of `CmdRenameCompany` and `CmdRenamePresident`.
fn rename_company_or_president(s: &Services, text: ByteSpan, execute: bool, president: bool) -> u8 {
    use crate::company_type::{MAX_LENGTH_COMPANY_NAME_CHARS, MAX_LENGTH_PRESIDENT_NAME_CHARS};
    let reset = text.size == 0;
    if !reset {
        let limit = if president {
            MAX_LENGTH_PRESIDENT_NAME_CHARS
        } else {
            MAX_LENGTH_COMPANY_NAME_CHARS
        };
        if (s.shared.utf8_length)(text.data, text.size) >= limit {
            return 1;
        }
        // IsUniqueCompanyName / IsUniquePresidentName: ignore empty names in the pool.
        if !companies(s, |c| {
            let name = if president {
                (s.president)(c)
            } else {
                (s.name)(c)
            };
            name.size == 0 || !name.same_bytes(text)
        }) {
            return 2;
        }
    }
    if execute {
        let id = CompanyID((s.shared.current_company)());
        let c = (s.company)(id);
        // An empty text clears the selected name, resetting it to the default.
        if president {
            (s.set_president)(c, text);
            if !reset
                && StringID((s.name_id)(c)) == StringID(s.strings.unnamed)
                && (s.name)(c).size == 0
            {
                (s.rename_transport)(text);
            }
            (s.president_windows)();
        } else {
            (s.set_name)(c, text);
        }
        (s.shared.screen_dirty)();
        (s.admin_update)(id);
        let mut slot = NameStorage::new();
        let mut names = Names::new(s, &mut slot);
        names.render(
            StringID(if president {
                s.strings.president
            } else {
                s.strings.company
            })
            .0,
            u32::from(id.0),
        );
        if president {
            (s.president_renamed)(c, names.slot);
        } else {
            (s.company_renamed)(c, names.slot);
        }
    }
    0
}
/// `CmdRenameCompany`: change the name of the company.
/// `execute` selects the operation; `text` is the new name or empty to reset to the default.
/// Returns the raw command result: success, error, or name-must-be-unique error.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_rename(s: &Services, text: ByteSpan, execute: bool) -> u8 {
    rename_company_or_president(s, text, execute, false)
}
/// `CmdRenamePresident`: change the name of the president.
/// `execute` selects the operation; `text` is the new name or empty to reset to the default.
/// Returns the raw command result: success, error, or name-must-be-unique error.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_president_rename(s: &Services, text: ByteSpan, execute: bool) -> u8 {
    rename_company_or_president(s, text, execute, true)
}
/// Determines the livery scheme for a vehicle.
/// `engine_type` is the vehicle engine; `parent_engine_type` is the front vehicle's
/// engine, or `EngineID::INVALID` if the vehicle is itself at the front.
/// `v` is null in the purchase list etc. Returns the livery scheme to use.
pub(crate) fn get_engine_livery_scheme(
    s: &Services,
    mut engine_type: EngineID,
    parent_engine_type: EngineID,
    v: *mut OpenTTDVehicle,
) -> u8 {
    use crate::cargo_registry::openttd_rust_cargo_by_label as get_cargo_type_by_label;
    use crate::cargo_type::{CargoLabel, CargoType, is_valid_cargo_type};
    use crate::cargotype::{CargoClass, CargoClasses};
    use crate::engine_type::{
        AIR_CTOL, AIR_FAST, AIR_HELI, EngineClass, EngineMiscFlag, EngineMiscFlags, RAIL_IS_MU,
        RailVehicleTypes,
    };
    use crate::livery::{
        LS_BUS, LS_DIESEL, LS_DMU, LS_ELECTRIC, LS_EMU, LS_FREIGHT_SHIP, LS_FREIGHT_TRAM,
        LS_FREIGHT_WAGON, LS_HELICOPTER, LS_LARGE_PLANE, LS_MAGLEV, LS_MONORAIL, LS_PASSENGER_SHIP,
        LS_PASSENGER_TRAM, LS_PASSENGER_WAGON_DIESEL, LS_PASSENGER_WAGON_ELECTRIC,
        LS_PASSENGER_WAGON_MAGLEV, LS_PASSENGER_WAGON_MONORAIL, LS_PASSENGER_WAGON_STEAM,
        LS_SMALL_PLANE, LS_STEAM, LS_TRUCK,
    };
    use crate::vehicle::{EngineRef, VehicleRef};
    use crate::vehicle_type::VehicleType;
    let mut cargo_type = if v.is_null() {
        CargoType::INVALID_CARGO
    } else {
        CargoType((s.capabilities().vehicle_cargo)(v))
    };
    let mut e = (s.capabilities().engine)(engine_type.0);
    match EngineRef::of(e).kind() {
        VehicleType::VEH_TRAIN => {
            if !v.is_null()
                && parent_engine_type != EngineID::INVALID
                && ((s.wagon_override)(v)
                    || (VehicleRef::of(v).is_articulated_part()
                        && EngineRef::of(e).rail_kind() != RailVehicleTypes::RAILVEH_WAGON))
            {
                // Wagonoverrides use the colour scheme of the front engine.
                // Articulated parts use the colour scheme of the first part. (Not supported for articulated wagons)
                engine_type = parent_engine_type;
                e = (s.capabilities().engine)(engine_type.0);
                // Note: Luckily cargo_type is not needed for engines
            }
            if !is_valid_cargo_type(cargo_type.0) {
                cargo_type = CargoType(EngineRef::of(e).default_cargo());
            }
            if !is_valid_cargo_type(cargo_type.0) {
                // The vehicle does not carry anything, let's pick some freight cargo
                cargo_type = CargoType(get_cargo_type_by_label(CargoLabel::CT_GOODS.0));
            }
            assert!(is_valid_cargo_type(cargo_type.0));
            if EngineRef::of(e).rail_kind() == RailVehicleTypes::RAILVEH_WAGON {
                if (s.cargo_freight)(cargo_type.0) {
                    return LiveryScheme(LS_FREIGHT_WAGON).0;
                }
                if parent_engine_type == EngineID::INVALID {
                    return LiveryScheme(LS_PASSENGER_WAGON_STEAM).0;
                }
                let p = (s.capabilities().engine)(parent_engine_type.0);
                // RAIL_IS_MU is the retained raw adapter for EngineMiscFlag::RailIsMU.
                let is_mu = EngineRef::of(p).misc() & RAIL_IS_MU != 0;
                match EngineClass(EngineRef::of(p).train_class()) {
                    EngineClass::EC_STEAM => LiveryScheme(LS_PASSENGER_WAGON_STEAM),
                    EngineClass::EC_DIESEL => {
                        if is_mu {
                            LiveryScheme(LS_DMU)
                        } else {
                            LiveryScheme(LS_PASSENGER_WAGON_DIESEL)
                        }
                    }
                    EngineClass::EC_ELECTRIC => {
                        if is_mu {
                            LiveryScheme(LS_EMU)
                        } else {
                            LiveryScheme(LS_PASSENGER_WAGON_ELECTRIC)
                        }
                    }
                    EngineClass::EC_MONORAIL => LiveryScheme(LS_PASSENGER_WAGON_MONORAIL),
                    EngineClass::EC_MAGLEV => LiveryScheme(LS_PASSENGER_WAGON_MAGLEV),
                    _ => unreachable!("NOT_REACHED"),
                }
            } else {
                let is_mu = EngineRef::of(e).misc() & RAIL_IS_MU != 0;
                match EngineClass(EngineRef::of(e).train_class()) {
                    EngineClass::EC_STEAM => LiveryScheme(LS_STEAM),
                    EngineClass::EC_DIESEL => {
                        if is_mu {
                            LiveryScheme(LS_DMU)
                        } else {
                            LiveryScheme(LS_DIESEL)
                        }
                    }
                    EngineClass::EC_ELECTRIC => {
                        if is_mu {
                            LiveryScheme(LS_EMU)
                        } else {
                            LiveryScheme(LS_ELECTRIC)
                        }
                    }
                    EngineClass::EC_MONORAIL => LiveryScheme(LS_MONORAIL),
                    EngineClass::EC_MAGLEV => LiveryScheme(LS_MAGLEV),
                    _ => unreachable!("NOT_REACHED"),
                }
            }
        }
        VehicleType::VEH_ROAD => {
            // Always use the livery of the front
            if !v.is_null() && parent_engine_type != EngineID::INVALID {
                engine_type = parent_engine_type;
                e = (s.capabilities().engine)(engine_type.0);
                cargo_type = CargoType((s.capabilities().vehicle_cargo)(
                    VehicleRef::of(v).first().0.as_ptr(),
                ));
            }
            if !is_valid_cargo_type(cargo_type.0) {
                cargo_type = CargoType(EngineRef::of(e).default_cargo());
            }
            if !is_valid_cargo_type(cargo_type.0) {
                // The vehicle does not carry anything, let's pick some freight cargo
                cargo_type = CargoType(get_cargo_type_by_label(CargoLabel::CT_GOODS.0));
            }
            assert!(is_valid_cargo_type(cargo_type.0));
            // Important: Use Tram Flag of front part. Luckily engine_type refers to the front part here.
            if EngineMiscFlags(EngineRef::of(e).misc()).test(EngineMiscFlag::RoadIsTram) {
                // Tram
                if CargoClasses((s.capabilities().cargo_classes)(cargo_type.0))
                    .test(CargoClass::Passengers)
                {
                    LiveryScheme(LS_PASSENGER_TRAM)
                } else {
                    LiveryScheme(LS_FREIGHT_TRAM)
                }
            } else if CargoClasses((s.capabilities().cargo_classes)(cargo_type.0))
                .test(CargoClass::Passengers)
            {
                // Bus or truck
                LiveryScheme(LS_BUS)
            } else {
                LiveryScheme(LS_TRUCK)
            }
        }
        VehicleType::VEH_SHIP => {
            if !is_valid_cargo_type(cargo_type.0) {
                cargo_type = CargoType(EngineRef::of(e).default_cargo());
            }
            if !is_valid_cargo_type(cargo_type.0) {
                // The vehicle does not carry anything, let's pick some freight cargo
                cargo_type = CargoType(get_cargo_type_by_label(CargoLabel::CT_GOODS.0));
            }
            assert!(is_valid_cargo_type(cargo_type.0));
            if CargoClasses((s.capabilities().cargo_classes)(cargo_type.0))
                .test(CargoClass::Passengers)
            {
                LiveryScheme(LS_PASSENGER_SHIP)
            } else {
                LiveryScheme(LS_FREIGHT_SHIP)
            }
        }
        VehicleType::VEH_AIRCRAFT => match EngineRef::of(e).aircraft_subtype() {
            AIR_HELI => LiveryScheme(LS_HELICOPTER),
            AIR_CTOL => LiveryScheme(LS_SMALL_PLANE),
            x if x == AIR_CTOL | AIR_FAST => LiveryScheme(LS_LARGE_PLANE),
            _ => unreachable!("NOT_REACHED"),
        },
        _ => unreachable!("NOT_REACHED"),
    }
    .0
}
// Compatibility name for the existing engine-loading caller; the helper has one body.
pub(crate) use get_engine_livery_scheme as engine_livery;

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_engine_livery_scheme(
    s: &Services,
    engine_type: EngineID,
    parent_engine_type: EngineID,
    v: *mut OpenTTDVehicle,
) -> u8 {
    get_engine_livery_scheme(s, engine_type, parent_engine_type, v)
}

crate::abi::layouts! {
    fn abi_layout;
 "OpenTTDCompanyNameStorage" ["company_economy"] => NameStorage { bytes, };
 "OpenTTDCompanyIdentityStrings" ["company_economy"] => Strings { unnamed, president, company, president_generated, andco, town_begin, town_end, company_begin, };
 "OpenTTDCompanyIdentityServices" ["company_economy"] => Services { shared, randomizer, faces, groups, strings, company, next_company, company_id, renew_reserve, name_id, set_name_id, name_parts, set_name_parts, president_id, set_president_id, president_parts, set_president_parts, colour, set_colour, last_build, is_ai, name, set_name, president, set_president, livery, face, name_frame_init, name_frame_drop, render, render_other, company_renamed, president_renamed, launch_news, admin_update, rename_transport, president_windows, colour_windows, object_colours, reset_colourmap, capabilities, wagon_override, cargo_freight, vehicle_id, invalidate_vehicle, };
}
