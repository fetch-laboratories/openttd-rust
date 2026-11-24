/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The company identity bodies of `company_cmd.cpp`: colours, names, presidents, liveries and
//! their four commands, with `GetEngineLiveryScheme` of `vehicle.cpp`.
//!
//! `Company` stays a native record, reached through one slot per member. A company, livery,
//! group or vehicle pointer is a live native record, serialized by the game-state mutex for
//! the synchronous call; each field access is one field-sized load or store at its
//! observation point. A name span a slot returns ends before the next call that can change
//! that name.
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_manager_face::{is_valid_company_manager_face, set_company_manager_face_style};
use crate::company_type::{
    CompanyID, CompanyManagerFace, MAX_COMPANIES, MAX_LENGTH_COMPANY_NAME_CHARS,
    MAX_LENGTH_PRESIDENT_NAME_CHARS, OpenTTDCompany,
};
use crate::engine_type::EngineID;
use crate::gfx_type::Colours;
use crate::livery::{Livery, LiveryFlag, LiveryFlags, LiveryScheme};
use crate::news_func::{AddNewsItemArgs, add_news_item};
use crate::news_type::{CompanyNewsInformation, NewsFlags, NewsReference, NewsStyle, NewsType};
use crate::raw_field::{read_field, write_field};
use crate::script_event_types::ScriptTextEvent;
use crate::services::{ByteSpan, CommandCost, CommandServices, Cost, Services as Shared};
use crate::services_assert::{assert_with, not_reached};
use crate::strings_func::{get_encoded_string, get_string};
use crate::strings_type::StringParameterData::{self, Integer};
use crate::strings_type::{
    SPECSTR_ANDCO_NAME, SPECSTR_COMPANY_NAME_START, SPECSTR_PRESIDENT_NAME, SPECSTR_TOWNNAME_END,
    SPECSTR_TOWNNAME_START, StringID,
};
use crate::table::strings::{
    STR_COMPANY_NAME, STR_ERROR_NAME_MUST_BE_UNIQUE, STR_NEWS_COMPANY_LAUNCH_DESCRIPTION,
    STR_NEWS_COMPANY_LAUNCH_TITLE, STR_PRESIDENT_NAME, STR_SV_UNNAMED,
};
use crate::tile_type::TileIndex;
use crate::vehicle::VehicleRef;
use crate::vehicle_type::OpenTTDVehicle;
use crate::window_type::{WindowClass, WindowNumber};

/// The tables and the native `Company` members the identity bodies use.
#[repr(C)]
pub struct Services {
    /// The town table, for `Town::index`.
    pub authority: &'static crate::town::AuthorityServices,
    /// The engine capabilities table, for engines, cargo classes and the vehicle table.
    pub capabilities: *const crate::engine_capabilities::Services,
    /// The command table, for the nested `CMD_RENAME_COMPANY`.
    pub commands: &'static CommandServices,
    /// The face table.
    pub faces: &'static crate::company_manager_face::Services,
    /// The group table.
    pub groups: &'static crate::fleet::GroupServices,
    /// `_random`, the game state's randomizer.
    pub randomizer: *mut crate::random_func::Randomizer,
    /// The shared table.
    pub shared: &'static Shared,
    /// `CompanyAdminUpdate(c)`.
    pub admin_update: extern "C" fn(*mut OpenTTDCompany) -> (),
    /// `BuildOwnerLegend()`.
    pub build_owner_legend: extern "C" fn() -> (),
    /// `Company::colour`.
    pub colour: extern "C" fn(*mut OpenTTDCompany) -> Colours,
    /// `Company::Get(index)`.
    pub company: extern "C" fn(CompanyID) -> *mut OpenTTDCompany,
    /// `CompanySettings::engine_renew`.
    pub engine_renew: extern "C" fn(*const OpenTTDCompany) -> bool,
    /// `CompanySettings::engine_renew_money`.
    pub engine_renew_money: extern "C" fn(*const OpenTTDCompany) -> u32,
    /// `CompanySettings::engine_renew_months`.
    pub engine_renew_months: extern "C" fn(*const OpenTTDCompany) -> i16,
    /// `&Company::face`.
    pub face: extern "C" fn(*mut OpenTTDCompany) -> *mut CompanyManagerFace,
    /// `Company::index`.
    pub index: extern "C" fn(*mut OpenTTDCompany) -> CompanyID,
    /// `Company::is_ai`.
    pub is_ai: extern "C" fn(*mut OpenTTDCompany) -> bool,
    /// `CargoSpec::Get(cargo_type)->is_freight`.
    pub is_freight: extern "C" fn(crate::cargo_type::CargoType) -> bool,
    /// `Company::last_build_coordinate`.
    pub last_build_coordinate: extern "C" fn(*mut OpenTTDCompany) -> TileIndex,
    /// `Company::livery`, the array's first element.
    pub livery: extern "C" fn(*mut OpenTTDCompany) -> *mut Livery,
    /// `Company::name`.
    pub name: extern "C" fn(*mut OpenTTDCompany) -> ByteSpan,
    /// `Company::name_1`.
    pub name_1: extern "C" fn(*mut OpenTTDCompany) -> StringID,
    /// `Company::name_2`.
    pub name_2: extern "C" fn(*mut OpenTTDCompany) -> u32,
    /// The first company from an index on, as `Company::Iterate` advances; null at the end.
    pub next_company: extern "C" fn(u32) -> *mut OpenTTDCompany,
    /// The object table, resolved at the call: its initialiser reaches this table.
    pub objects: extern "C" fn() -> *const crate::objects::ObjectServices,
    /// `Company::president_name`.
    pub president_name: extern "C" fn(*mut OpenTTDCompany) -> ByteSpan,
    /// `Company::president_name_1`.
    pub president_name_1: extern "C" fn(*mut OpenTTDCompany) -> StringID,
    /// `Company::president_name_2`.
    pub president_name_2: extern "C" fn(*mut OpenTTDCompany) -> u32,
    /// `&c->RenewalList()`: the Rust-owned slot holding the company's `EngineRenewList`.
    pub renewal_head: extern "C" fn(*const OpenTTDCompany) -> *mut *mut (),
    /// `ResetVehicleColourMap()`.
    pub reset_colourmap: extern "C" fn() -> (),
    /// `Company::colour = value`.
    pub set_colour: extern "C" fn(*mut OpenTTDCompany, Colours) -> (),
    /// `Company::name = text`.
    pub set_name: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> (),
    /// `Company::name_1 = value`.
    pub set_name_1: extern "C" fn(*mut OpenTTDCompany, StringID) -> (),
    /// `Company::name_2 = value`.
    pub set_name_2: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
    /// `Company::president_name = text`.
    pub set_president_name: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> (),
    /// `Company::president_name_1 = value`.
    pub set_president_name_1: extern "C" fn(*mut OpenTTDCompany, StringID) -> (),
    /// `Company::president_name_2 = value`.
    pub set_president_name_2: extern "C" fn(*mut OpenTTDCompany, u32) -> (),
}

impl Services {
    /// The engine capabilities table.
    #[allow(unsafe_code)] // One borrow of the process-lifetime table.
    fn capabilities(&self) -> &crate::engine_capabilities::Services {
        // SAFETY: The native table names the canonical, immutable process-lifetime table.
        unsafe { &*self.capabilities }
    }

    /// `_current_company`.
    fn current_company(&self) -> CompanyID {
        CompanyID((self.shared.current_company)())
    }
}

/// `Company::Iterate()`: call `f` with each company in pool order until it returns false;
/// returns whether the loop ran to the end.
fn companies(s: &Services, mut f: impl FnMut(*mut OpenTTDCompany) -> bool) -> bool {
    let mut from = 0;
    loop {
        let c = (s.next_company)(from);
        if c.is_null() {
            return true;
        }
        from = u32::from((s.index)(c).0) + 1;
        if !f(c) {
            return false;
        }
    }
}

/// `&c->livery[scheme]`, given the array `Services::livery` returns.
fn livery_at(livery: *mut Livery, scheme: LiveryScheme) -> *mut Livery {
    livery.wrapping_add(usize::from(scheme.0))
}

/// A span over `bytes`, live while they are, for a native string member's assignment or
/// comparison.
fn span(bytes: &[u8]) -> ByteSpan {
    ByteSpan {
        data: bytes.as_ptr(),
        size: bytes.len(),
    }
}

/// `Utf8StringLength(text)`.
fn utf8_string_length(s: &Services, text: &[u8]) -> usize {
    (s.shared.utf8_length)(text.as_ptr(), text.len())
}

/// `_company_colours`: the company colours, indexed by company.
static mut COMPANY_COLOURS: [Colours; MAX_COMPANIES as usize] =
    [Colours::COLOUR_BEGIN; MAX_COMPANIES as usize];

/// `_company_colours`, on which `company_cmd.cpp` builds its container.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_company_colours() -> *mut u8 {
    (&raw mut COMPANY_COLOURS).cast()
}

/// `_company_colours[company] = colour`.
#[allow(unsafe_code)] // One store to the process-lifetime array.
pub(crate) fn set_company_colour(company: CompanyID, colour: Colours) {
    // SAFETY: The game-state mutex serializes the array; one element-sized store.
    unsafe {
        (&raw mut COMPANY_COLOURS)
            .cast::<Colours>()
            .add(usize::from(company.0))
            .write(colour);
    }
}

/// Sorting weights for the company colours.
const COLOUR_SORT: [u8; Colours::COLOUR_END.0 as usize] =
    [2, 2, 3, 2, 3, 2, 3, 2, 3, 2, 2, 2, 3, 1, 1, 1];
/// Similar colours, so we can try to prevent same coloured companies.
const SIMILAR_COLOUR: [[Colours; 2]; Colours::COLOUR_END.0 as usize] = [
    [Colours::COLOUR_BLUE, Colours::COLOUR_LIGHT_BLUE], // COLOUR_DARK_BLUE
    [Colours::COLOUR_GREEN, Colours::COLOUR_DARK_GREEN], // COLOUR_PALE_GREEN
    [Colours::INVALID_COLOUR, Colours::INVALID_COLOUR], // COLOUR_PINK
    [Colours::COLOUR_ORANGE, Colours::INVALID_COLOUR],  // COLOUR_YELLOW
    [Colours::INVALID_COLOUR, Colours::INVALID_COLOUR], // COLOUR_RED
    [Colours::COLOUR_DARK_BLUE, Colours::COLOUR_BLUE],  // COLOUR_LIGHT_BLUE
    [Colours::COLOUR_PALE_GREEN, Colours::COLOUR_DARK_GREEN], // COLOUR_GREEN
    [Colours::COLOUR_PALE_GREEN, Colours::COLOUR_GREEN], // COLOUR_DARK_GREEN
    [Colours::COLOUR_DARK_BLUE, Colours::COLOUR_LIGHT_BLUE], // COLOUR_BLUE
    [Colours::COLOUR_BROWN, Colours::COLOUR_ORANGE],    // COLOUR_CREAM
    [Colours::COLOUR_PURPLE, Colours::INVALID_COLOUR],  // COLOUR_MAUVE
    [Colours::COLOUR_MAUVE, Colours::INVALID_COLOUR],   // COLOUR_PURPLE
    [Colours::COLOUR_YELLOW, Colours::COLOUR_CREAM],    // COLOUR_ORANGE
    [Colours::COLOUR_CREAM, Colours::INVALID_COLOUR],   // COLOUR_BROWN
    [Colours::COLOUR_WHITE, Colours::INVALID_COLOUR],   // COLOUR_GREY
    [Colours::COLOUR_GREY, Colours::INVALID_COLOUR],    // COLOUR_WHITE
];

/// Generate a company colour. Returns the generated company colour.
#[expect(
    clippy::cast_possible_truncation,
    reason = "`static_cast<Colours>(i)` of an index below `COLOUR_END`."
)]
pub(crate) fn generate_company_colour(s: &Services) -> Colours {
    const END: usize = Colours::COLOUR_END.0 as usize;
    // Initialize array
    let mut colours = std::array::from_fn::<_, END, _>(|i| Colours(i as u8));

    // And randomize it
    for _ in 0..100 {
        let r = s.shared.random();
        colours.swap(
            crate::bitmath_func::gb(r, 0, 4) as usize,
            crate::bitmath_func::gb(r, 4, 4) as usize,
        );
    }

    // Bubble sort it according to the values in table 1
    for _ in 0..END {
        for j in 1..END {
            if COLOUR_SORT[usize::from(colours[j - 1].0)] < COLOUR_SORT[usize::from(colours[j].0)] {
                colours.swap(j - 1, j);
            }
        }
    }

    // Move the colours that look similar to each company's colour to the side
    companies(s, |c| {
        let pcolour = (s.colour)(c);

        for colour in &mut colours {
            if *colour == pcolour {
                *colour = Colours::INVALID_COLOUR;
                break;
            }
        }

        for similar in SIMILAR_COLOUR[usize::from(pcolour.0)] {
            if similar == Colours::INVALID_COLOUR {
                break;
            }

            for i in 1..END {
                if colours[i - 1] == similar {
                    colours.swap(i - 1, i);
                }
            }
        }
        true
    });

    // Return the first available colour
    for colour in colours {
        if colour != Colours::INVALID_COLOUR {
            return colour;
        }
    }

    not_reached()
}

/// Generate the name of a company from the last build coordinate. `c` is the company to
/// give a name.
#[allow(unsafe_code)] // The town's name and field-sized name-field loads.
pub(crate) fn generate_company_name(s: &Services, c: *mut OpenTTDCompany) {
    if (s.name_1)(c) != STR_SV_UNNAMED {
        return;
    }
    if (s.last_build_coordinate)(c) == TileIndex(0) {
        return;
    }

    let t = crate::town::spatial::closest_town_from_tile(
        s.shared,
        (s.last_build_coordinate)(c),
        u32::MAX,
    );
    let town = (s.shared.town_owner)(t);
    let simulation = crate::town::simulation(town);

    // The original's labels: the first candidate is the town's name when it has a
    // generated one; a failure at `verify_name` goes to `bad_town_name`.
    let townnametype = u32::from(read_field!(simulation, townnametype));
    // SAFETY: The module contract: the closest town's owner is live.
    let mut candidate = (unsafe { crate::town::names::town_name(town) }.size == 0
        && crate::math::is_inside_mm(
            townnametype as usize,
            SPECSTR_TOWNNAME_START.0 as usize,
            SPECSTR_TOWNNAME_END.0 as usize,
        ))
    .then(|| {
        (
            StringID(townnametype - SPECSTR_TOWNNAME_START.0 + SPECSTR_COMPANY_NAME_START.0),
            read_field!(simulation, townnameparts),
        )
    });
    let (str, strp, name) = 'set_name: loop {
        // verify_name:
        if let Some((str, strp)) = candidate {
            // No companies must have this name already
            let unique = companies(s, |cc| (s.name_1)(cc) != str || (s.name_2)(cc) != strp);
            if unique {
                let name = get_string(str, [Integer(u64::from(strp))]);
                if utf8_string_length(s, &name) < MAX_LENGTH_COMPANY_NAME_CHARS {
                    break 'set_name (str, strp, name);
                }
            }
        }
        // bad_town_name:
        if (s.president_name_1)(c) == SPECSTR_PRESIDENT_NAME {
            let str = SPECSTR_ANDCO_NAME;
            let strp = (s.president_name_2)(c);
            let name = get_string(str, [Integer(u64::from(strp))]);
            break 'set_name (str, strp, name);
        }
        candidate = Some((SPECSTR_ANDCO_NAME, s.shared.random()));
    };

    // set_name:
    (s.set_name_1)(c, str);
    (s.set_name_2)(c, strp);

    (s.shared.screen_dirty)();
    crate::ai::broadcast_new_text_event(
        &ScriptTextEvent::company_renamed((s.index)(c), &name),
        CompanyID::INVALID,
    );
    crate::game::new_text_event(&ScriptTextEvent::company_renamed((s.index)(c), &name));

    if (s.is_ai)(c) {
        // `CompanyNewsInformation` resolves the company by its index (#1048).
        let cni = CompanyNewsInformation::new(STR_NEWS_COMPANY_LAUNCH_TITLE, (s.index)(c), None);
        let company_name = cni.company_name();
        let headline = get_encoded_string(
            STR_NEWS_COMPANY_LAUNCH_DESCRIPTION,
            &[
                StringParameterData::String(&company_name),
                Integer(u64::from((s.authority.town_index)(t))),
            ],
        );
        add_news_item(&AddNewsItemArgs {
            ref1: NewsReference::Tile((s.last_build_coordinate)(c)).record(),
            data: cni.into_data(),
            ..AddNewsItemArgs::new(
                &headline,
                NewsType::CompanyInfo,
                NewsStyle::Company,
                NewsFlags::default(),
            )
        });
    }
}

/// Generate a random president name of a company. `c` is the company that needs a new
/// president name.
pub(crate) fn generate_president_name(s: &Services, c: *mut OpenTTDCompany) {
    // restart:
    'restart: loop {
        (s.set_president_name_2)(c, s.shared.random());
        (s.set_president_name_1)(c, SPECSTR_PRESIDENT_NAME);

        // Reserve space for extra unicode character. We need to do this to be able
        // to detect too long president name.
        let name = get_string(STR_PRESIDENT_NAME, [Integer(u64::from((s.index)(c).0))]);
        if utf8_string_length(s, &name) >= MAX_LENGTH_PRESIDENT_NAME_CHARS {
            continue;
        }

        let unique = companies(s, |cc| {
            if c != cc {
                let other_name =
                    get_string(STR_PRESIDENT_NAME, [Integer(u64::from((s.index)(cc).0))]);
                if name == other_name {
                    return false;
                }
            }
            true
        });
        if !unique {
            continue 'restart;
        }
        return;
    }
}

/// Reset the livery schemes to the company's primary colour.
/// This is used on loading games without livery information and on new company start up.
#[allow(unsafe_code)] // The group table's iteration and the field-sized livery stores.
pub(crate) fn reset_company_livery(s: &Services, c: *mut OpenTTDCompany) {
    let livery = (s.livery)(c);
    for scheme in LiveryScheme::LS_BEGIN.0..LiveryScheme::LS_END.0 {
        let l = livery_at(livery, LiveryScheme(scheme));
        // `in_use.Reset()`, through one field-sized load and store.
        let mut in_use = read_field!(l, in_use);
        in_use.reset_all();
        write_field!(l, in_use, in_use);
        write_field!(l, colour1, (s.colour)(c));
        write_field!(l, colour2, (s.colour)(c));
    }

    let mut from = 0;
    loop {
        // SAFETY: The module contract: the group table's records are live.
        let crate::fleet::NextGroup { group: g, index } = unsafe { (s.groups.next_group)(from) };
        if index == u32::MAX {
            break;
        }
        from = index + 1;
        if read_field!(g, owner) == (s.index)(c).0 {
            // `g->livery.in_use.Reset()`, through one field-sized load and store.
            let mut in_use = read_field!(g, livery.in_use);
            in_use.reset_all();
            write_field!(g, livery.in_use, in_use);
            write_field!(g, livery.colour1, (s.colour)(c));
            write_field!(g, livery.colour2, (s.colour)(c));
        }
    }
}

/// Update liveries for a company. This is called when the `LS_DEFAULT` scheme is changed, to
/// update schemes with colours set to default.
#[allow(unsafe_code)] // The field-sized livery accesses and the group table's update.
pub(crate) fn update_company_liveries(s: &Services, c: *mut OpenTTDCompany) {
    let livery = (s.livery)(c);
    let default = livery_at(livery, LiveryScheme::LS_DEFAULT);
    for i in 1..LiveryScheme::LS_END.0 {
        let l = livery_at(livery, LiveryScheme(i));
        if !read_field!(l, in_use).test(LiveryFlag::Primary) {
            write_field!(l, colour1, read_field!(default, colour1));
        }
        if !read_field!(l, in_use).test(LiveryFlag::Secondary) {
            write_field!(l, colour2, read_field!(default, colour2));
        }
    }
    // SAFETY: The module contract: the group table's records are live.
    unsafe { crate::fleet::company_liveries(s.groups, (s.index)(c).0, default) };
}

/// Arguments of `CmdSetCompanyManagerFace`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetCompanyManagerFace {
    /// The style of the company manager face.
    pub style: u32,
    /// The bits of company manager face.
    pub bits: u32,
}

/// Change the company manager's face.
pub(crate) fn cmd_set_company_manager_face(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetCompanyManagerFace,
) {
    let CmdSetCompanyManagerFace { style, bits } = args;
    // `CompanyManagerFace tmp_face{style, bits, {}}`: the check reads its style and bits.
    if !is_valid_company_manager_face(s.faces, style, bits) {
        crate::command_func::cmd_error(out);
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        let cmf = (s.face)((s.company)(s.current_company()));
        set_company_manager_face_style(s.faces, cmf, style);
        crate::company_manager_face::set_face_bits(s.faces, cmf, bits);

        (s.shared.screen_dirty)();
    }
    out.zero();
}

/// Arguments of `CmdSetCompanyColour`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetCompanyColour {
    /// Scheme to set.
    pub scheme: LiveryScheme,
    /// Set first/second colour.
    pub primary: bool,
    /// New colour for vehicles, property, etc.
    pub colour: Colours,
}

/// Change the company's company-colour.
#[allow(unsafe_code)] // The field-sized livery accesses and the vehicle cache stores.
#[allow(
    clippy::too_many_lines,
    reason = "`CmdSetCompanyColour` is one body: its checks, livery updates and invalidations."
)]
pub(crate) fn cmd_set_company_colour(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetCompanyColour,
) {
    let CmdSetCompanyColour {
        mut scheme,
        primary,
        mut colour,
    } = args;
    if scheme >= LiveryScheme::LS_END
        || (colour >= Colours::COLOUR_END && colour != Colours::INVALID_COLOUR)
    {
        crate::command_func::cmd_error(out);
        return;
    }

    // Default scheme can't be reset to invalid.
    if scheme == LiveryScheme::LS_DEFAULT && colour == Colours::INVALID_COLOUR {
        crate::command_func::cmd_error(out);
        return;
    }

    let c = (s.company)(s.current_company());

    // Ensure no two companies have the same primary colour
    if scheme == LiveryScheme::LS_DEFAULT
        && primary
        && !companies(s, |cc| cc == c || (s.colour)(cc) != colour)
    {
        crate::command_func::cmd_error(out);
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        let livery = (s.livery)(c);
        let l = livery_at(livery, scheme);
        let default = livery_at(livery, LiveryScheme::LS_DEFAULT);
        if primary {
            if scheme != LiveryScheme::LS_DEFAULT {
                let mut in_use = read_field!(l, in_use);
                in_use.set_to(LiveryFlag::Primary, colour != Colours::INVALID_COLOUR);
                write_field!(l, in_use, in_use);
            }
            if colour == Colours::INVALID_COLOUR {
                colour = read_field!(default, colour1);
            }
            write_field!(l, colour1, colour);

            // If setting the first colour of the default scheme, adjust the
            // original and cached company colours too.
            if scheme == LiveryScheme::LS_DEFAULT {
                update_company_liveries(s, c);
                set_company_colour(s.current_company(), colour);
                (s.set_colour)(c, colour);
                (s.admin_update)(c);
            }
        } else {
            if scheme != LiveryScheme::LS_DEFAULT {
                let mut in_use = read_field!(l, in_use);
                in_use.set_to(LiveryFlag::Secondary, colour != Colours::INVALID_COLOUR);
                write_field!(l, in_use, in_use);
            }
            if colour == Colours::INVALID_COLOUR {
                colour = read_field!(default, colour2);
            }
            write_field!(l, colour2, colour);

            if scheme == LiveryScheme::LS_DEFAULT {
                update_company_liveries(s, c);
            }
        }

        let both = LiveryFlags::from_element(LiveryFlag::Primary)
            | LiveryFlags::from_element(LiveryFlag::Secondary);
        if read_field!(l, in_use).any(both) {
            // If enabling a scheme, set the default scheme to be in use too
            let mut in_use = read_field!(default, in_use);
            in_use.set(LiveryFlag::Primary);
            write_field!(default, in_use, in_use);
        } else {
            // Else loop through all schemes to see if any are left enabled.
            // If not, disable the default scheme too.
            let mut in_use = read_field!(default, in_use);
            in_use.reset_set(both);
            write_field!(default, in_use, in_use);
            scheme = LiveryScheme::LS_DEFAULT;
            while scheme < LiveryScheme::LS_END {
                if read_field!(livery_at(livery, scheme), in_use).any(both) {
                    let mut in_use = read_field!(default, in_use);
                    in_use.set(LiveryFlag::Primary);
                    write_field!(default, in_use, in_use);
                    break;
                }
                scheme = LiveryScheme(scheme.0 + 1);
            }
        }

        (s.reset_colourmap)();
        (s.shared.screen_dirty)();

        // All graph related to companies use the company colour.
        for cls in [
            WindowClass::WC_INCOME_GRAPH,
            WindowClass::WC_OPERATING_PROFIT,
            WindowClass::WC_DELIVERED_CARGO,
            WindowClass::WC_PERFORMANCE_HISTORY,
            WindowClass::WC_COMPANY_VALUE,
            WindowClass::WC_LINKGRAPH_LEGEND,
        ] {
            crate::window_func::invalidate_window_data(cls, WindowNumber(0), 0, false);
        }
        // The smallmap owner view also stores the company colours.
        (s.build_owner_legend)();
        crate::window_func::invalidate_window_data(
            WindowClass::WC_SMALLMAP,
            WindowNumber(0),
            1,
            false,
        );

        // Company colour data is indirectly cached.
        let resolver = s.capabilities().vehicles.newgrf();
        let mut from = 0;
        while let Some(v) = (s.shared.vehicle_next)(from) {
            from = v.index(resolver).0 + 1;
            if v.owner() == s.current_company() {
                // SAFETY: The module contract: the iterated vehicle is live.
                unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache(resolver, v) };
            }
        }

        // SAFETY: The object table is the canonical process-lifetime table.
        crate::objects::update_object_colours(unsafe { &*(s.objects)() }, c, (s.index)(c));
    }
    out.zero();
}

/// Is the given name in use as name of a company? Returns true if the name is unique (that
/// is, not in use), else false.
fn is_unique_company_name(s: &Services, name: &[u8]) -> bool {
    companies(s, |c| {
        let n = (s.name)(c);
        n.size == 0 || !n.same_bytes(span(name))
    })
}

/// Arguments of `CmdRenameCompany`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdRenameCompany<'a> {
    /// The new name or an empty string when resetting to the default.
    pub text: &'a [u8],
}

/// Change the name of the company.
pub(crate) fn cmd_rename_company(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdRenameCompany<'_>,
) {
    let CmdRenameCompany { text } = args;
    let reset = text.is_empty();

    if !reset {
        if utf8_string_length(s, text) >= MAX_LENGTH_COMPANY_NAME_CHARS {
            crate::command_func::cmd_error(out);
            return;
        }
        if !is_unique_company_name(s, text) {
            out.error(STR_ERROR_NAME_MUST_BE_UNIQUE.0);
            return;
        }
    }

    if flags.test(DoCommandFlag::Execute) {
        let c = (s.company)(s.current_company());
        // An empty text clears the name (`c->name.clear()`), resetting it to the default.
        (s.set_name)(c, span(text));
        (s.shared.screen_dirty)();
        (s.admin_update)(c);

        let new_name = get_string(STR_COMPANY_NAME, [Integer(u64::from((s.index)(c).0))]);
        crate::ai::broadcast_new_text_event(
            &ScriptTextEvent::company_renamed((s.index)(c), &new_name),
            CompanyID::INVALID,
        );
        crate::game::new_text_event(&ScriptTextEvent::company_renamed((s.index)(c), &new_name));
    }

    out.zero();
}

/// Is the given name in use as president name of a company? Returns true if the name is
/// unique (that is, not in use), else false.
fn is_unique_president_name(s: &Services, name: &[u8]) -> bool {
    companies(s, |c| {
        let n = (s.president_name)(c);
        n.size == 0 || !n.same_bytes(span(name))
    })
}

/// Arguments of `CmdRenamePresident`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdRenamePresident<'a> {
    /// The new name or an empty string when resetting to the default.
    pub text: &'a [u8],
}

/// Change the name of the president.
#[allow(unsafe_code)] // The nested command's result storage.
pub(crate) fn cmd_rename_president(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdRenamePresident<'_>,
) {
    let CmdRenamePresident { text } = args;
    let reset = text.is_empty();

    if !reset {
        if utf8_string_length(s, text) >= MAX_LENGTH_PRESIDENT_NAME_CHARS {
            crate::command_func::cmd_error(out);
            return;
        }
        if !is_unique_president_name(s, text) {
            out.error(STR_ERROR_NAME_MUST_BE_UNIQUE.0);
            return;
        }
    }

    if flags.test(DoCommandFlag::Execute) {
        let c = (s.company)(s.current_company());

        // An empty text clears the name (`c->president_name.clear()`).
        (s.set_president_name)(c, span(text));
        if !reset && (s.name_1)(c) == STR_SV_UNNAMED && (s.name)(c).size == 0 {
            // `Command<CMD_RENAME_COMPANY>::Do(DoCommandFlag::Execute, text + " Transport")`;
            // its result is dropped.
            let mut transport = text.to_vec();
            transport.extend_from_slice(b" Transport");
            // SAFETY: The nested result lives on this stack for the synchronous command.
            unsafe {
                crate::command::with_cost(s.commands.cost, |result| {
                    crate::command::nested(
                        s.commands,
                        s.shared,
                        result,
                        0,
                        DoCommandFlag::EXECUTE,
                        |flags| {
                            cmd_rename_company(
                                s,
                                result,
                                DoCommandFlags(flags),
                                CmdRenameCompany { text: &transport },
                            );
                        },
                    );
                });
            }
        }

        crate::window_func::invalidate_window_classes_data(WindowClass::WC_COMPANY, 1, false);
        (s.shared.screen_dirty)();
        (s.admin_update)(c);

        let new_name = get_string(STR_PRESIDENT_NAME, [Integer(u64::from((s.index)(c).0))]);
        crate::ai::broadcast_new_text_event(
            &ScriptTextEvent::president_renamed((s.index)(c), &new_name),
            CompanyID::INVALID,
        );
        crate::game::new_text_event(&ScriptTextEvent::president_renamed((s.index)(c), &new_name));
    }

    out.zero();
}

/// Determines the `LiveryScheme` for a vehicle.
/// `engine_type` is the engine of the vehicle, `parent_engine_type` the engine of the front
/// vehicle, `EngineID::INVALID` if the vehicle is at front itself, and `v` the vehicle,
/// `None` if in purchase list etc. Returns the livery scheme to use.
#[allow(
    clippy::too_many_lines,
    reason = "`GetEngineLiveryScheme` is one switch over the vehicle type."
)]
pub(crate) fn get_engine_livery_scheme(
    s: &Services,
    mut engine_type: EngineID,
    parent_engine_type: EngineID,
    v: Option<VehicleRef>,
) -> LiveryScheme {
    use crate::cargo_registry::get_cargo_type_by_label;
    use crate::cargo_type::{CargoLabel, CargoType, is_valid_cargo_type};
    use crate::cargotype::{CargoClass, CargoClasses, is_cargo_in_class};
    use crate::engine_type::{
        AIR_CTOL, AIR_FAST, AIR_HELI, EngineClass, EngineMiscFlag, EngineMiscFlags,
        RailVehicleTypes,
    };
    use crate::vehicle::EngineRef;
    use crate::vehicle_type::VehicleType;
    let resolver = s.capabilities().vehicles.newgrf();
    let passengers = CargoClasses::from_element(CargoClass::Passengers).0;
    // `IsCargoInClass(cargo_type, CargoClass::Passengers)`.
    let is_passengers =
        |cargo: CargoType| is_cargo_in_class((s.capabilities().cargo_classes)(cargo.0), passengers);

    let mut cargo_type = v.map_or(CargoType::INVALID_CARGO, |v| v.cargo_type(resolver));
    let mut e = (s.capabilities().engine)(engine_type.0);
    match EngineRef::of(e).kind() {
        VehicleType::VEH_TRAIN => {
            if let Some(v) = v
                && parent_engine_type != EngineID::INVALID
                && (crate::newgrf_vehicle::uses_wagon_override(resolver, v)
                    || (v.is_articulated_part()
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
                cargo_type = get_cargo_type_by_label(CargoLabel::CT_GOODS);
            }
            assert_with(|| is_valid_cargo_type(cargo_type.0));
            if EngineRef::of(e).rail_kind() == RailVehicleTypes::RAILVEH_WAGON {
                if (s.is_freight)(cargo_type) {
                    LiveryScheme::LS_FREIGHT_WAGON
                } else if parent_engine_type == EngineID::INVALID {
                    LiveryScheme::LS_PASSENGER_WAGON_STEAM
                } else {
                    let p = EngineRef::of((s.capabilities().engine)(parent_engine_type.0));
                    let is_mu = EngineMiscFlags(p.misc()).test(EngineMiscFlag::RailIsMU);
                    match EngineClass(p.train_class()) {
                        EngineClass::EC_STEAM => LiveryScheme::LS_PASSENGER_WAGON_STEAM,
                        EngineClass::EC_DIESEL => {
                            if is_mu {
                                LiveryScheme::LS_DMU
                            } else {
                                LiveryScheme::LS_PASSENGER_WAGON_DIESEL
                            }
                        }
                        EngineClass::EC_ELECTRIC => {
                            if is_mu {
                                LiveryScheme::LS_EMU
                            } else {
                                LiveryScheme::LS_PASSENGER_WAGON_ELECTRIC
                            }
                        }
                        EngineClass::EC_MONORAIL => LiveryScheme::LS_PASSENGER_WAGON_MONORAIL,
                        EngineClass::EC_MAGLEV => LiveryScheme::LS_PASSENGER_WAGON_MAGLEV,
                        _ => not_reached(),
                    }
                }
            } else {
                let is_mu = EngineMiscFlags(EngineRef::of(e).misc()).test(EngineMiscFlag::RailIsMU);

                match EngineClass(EngineRef::of(e).train_class()) {
                    EngineClass::EC_STEAM => LiveryScheme::LS_STEAM,
                    EngineClass::EC_DIESEL => {
                        if is_mu {
                            LiveryScheme::LS_DMU
                        } else {
                            LiveryScheme::LS_DIESEL
                        }
                    }
                    EngineClass::EC_ELECTRIC => {
                        if is_mu {
                            LiveryScheme::LS_EMU
                        } else {
                            LiveryScheme::LS_ELECTRIC
                        }
                    }
                    EngineClass::EC_MONORAIL => LiveryScheme::LS_MONORAIL,
                    EngineClass::EC_MAGLEV => LiveryScheme::LS_MAGLEV,
                    _ => not_reached(),
                }
            }
        }

        VehicleType::VEH_ROAD => {
            // Always use the livery of the front
            if let Some(v) = v
                && parent_engine_type != EngineID::INVALID
            {
                engine_type = parent_engine_type;
                e = (s.capabilities().engine)(engine_type.0);
                cargo_type = v.first().cargo_type(resolver);
            }
            if !is_valid_cargo_type(cargo_type.0) {
                cargo_type = CargoType(EngineRef::of(e).default_cargo());
            }
            if !is_valid_cargo_type(cargo_type.0) {
                // The vehicle does not carry anything, let's pick some freight cargo
                cargo_type = get_cargo_type_by_label(CargoLabel::CT_GOODS);
            }
            assert_with(|| is_valid_cargo_type(cargo_type.0));

            // Important: Use Tram Flag of front part. Luckily engine_type refers to the front part here.
            if EngineMiscFlags(EngineRef::of(e).misc()).test(EngineMiscFlag::RoadIsTram) {
                // Tram
                if is_passengers(cargo_type) {
                    LiveryScheme::LS_PASSENGER_TRAM
                } else {
                    LiveryScheme::LS_FREIGHT_TRAM
                }
            } else {
                // Bus or truck
                if is_passengers(cargo_type) {
                    LiveryScheme::LS_BUS
                } else {
                    LiveryScheme::LS_TRUCK
                }
            }
        }

        VehicleType::VEH_SHIP => {
            if !is_valid_cargo_type(cargo_type.0) {
                cargo_type = CargoType(EngineRef::of(e).default_cargo());
            }
            if !is_valid_cargo_type(cargo_type.0) {
                // The vehicle does not carry anything, let's pick some freight cargo
                cargo_type = get_cargo_type_by_label(CargoLabel::CT_GOODS);
            }
            assert_with(|| is_valid_cargo_type(cargo_type.0));
            if is_passengers(cargo_type) {
                LiveryScheme::LS_PASSENGER_SHIP
            } else {
                LiveryScheme::LS_FREIGHT_SHIP
            }
        }

        VehicleType::VEH_AIRCRAFT => match EngineRef::of(e).aircraft_subtype() {
            AIR_HELI => LiveryScheme::LS_HELICOPTER,
            AIR_CTOL => LiveryScheme::LS_SMALL_PLANE,
            x if x == AIR_CTOL | AIR_FAST => LiveryScheme::LS_LARGE_PLANE,
            _ => not_reached(),
        },

        _ => not_reached(),
    }
}

/// `ResetCompanyLivery`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_company_reset_livery(s: &Services, c: *mut OpenTTDCompany) {
    reset_company_livery(s, c);
}

/// `UpdateCompanyLiveries`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_company_update_liveries(s: &Services, c: *mut OpenTTDCompany) {
    update_company_liveries(s, c);
}

/// `CmdSetCompanyManagerFace`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_company_manager_face(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    style: u32,
    bits: u32,
) {
    cmd_set_company_manager_face(
        s,
        Cost(out, s.commands.cost),
        DoCommandFlags(flags),
        CmdSetCompanyManagerFace { style, bits },
    );
}

/// `CmdSetCompanyColour`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_company_colour(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    scheme: u8,
    primary: bool,
    colour: u8,
) {
    cmd_set_company_colour(
        s,
        Cost(out, s.commands.cost),
        DoCommandFlags(flags),
        CmdSetCompanyColour {
            scheme: LiveryScheme(scheme),
            primary,
            colour: Colours(colour),
        },
    );
}

/// `CmdRenameCompany`; `text` is the command's `std::string`, live for the call.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute and the command's text bytes.
pub extern "C" fn openttd_rust_company_rename(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    text: ByteSpan,
) {
    cmd_rename_company(
        s,
        Cost(out, s.commands.cost),
        DoCommandFlags(flags),
        CmdRenameCompany {
            // SAFETY: The facade's text is live for the synchronous command.
            text: unsafe { text.bytes() },
        },
    );
}

/// `CmdRenamePresident`; `text` is the command's `std::string`, live for the call.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute and the command's text bytes.
pub extern "C" fn openttd_rust_president_rename(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    text: ByteSpan,
) {
    cmd_rename_president(
        s,
        Cost(out, s.commands.cost),
        DoCommandFlags(flags),
        CmdRenamePresident {
            // SAFETY: The facade's text is live for the synchronous command.
            text: unsafe { text.bytes() },
        },
    );
}

/// `GetEngineLiveryScheme`, with a null `v` for none.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // Export attribute only.
pub extern "C" fn openttd_rust_engine_livery_scheme(
    s: &Services,
    engine_type: u16,
    parent_engine_type: u16,
    v: *mut OpenTTDVehicle,
) -> u8 {
    get_engine_livery_scheme(
        s,
        EngineID(engine_type),
        EngineID(parent_engine_type),
        std::ptr::NonNull::new(v).map(VehicleRef),
    )
    .0
}

#[allow(unsafe_code)] // The tests' fixture records and recording services.
#[cfg(test)]
mod tests;

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDCompanyIdentityServices" ["company_economy"] => Services {
        authority,
        capabilities,
        commands,
        faces,
        groups,
        randomizer,
        shared,
        admin_update,
        build_owner_legend,
        colour,
        company,
        engine_renew,
        engine_renew_money,
        engine_renew_months,
        face,
        index,
        is_ai,
        is_freight,
        last_build_coordinate,
        livery,
        name,
        name_1,
        name_2,
        next_company,
        objects,
        president_name,
        president_name_1,
        president_name_2,
        renewal_head,
        reset_colourmap,
        set_colour,
        set_name,
        set_name_1,
        set_name_2,
        set_president_name,
        set_president_name_1,
        set_president_name_2,
    };
}
